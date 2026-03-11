// Claude Code agent implementations
// Uses the claude CLI for agent interactions

mod agent_config;
pub mod agent_names;
mod claude_code_client;
mod stream_processor;

#[allow(unused_imports)]
pub use agent_config::team_config::{
    env_variant_override, get_team_constraints, validate_child_team_config, validate_team_plan,
    ApprovedTeamPlan, ApprovedTeammate, ProcessMapping, ProcessSlot, TeamConstraintError,
    TeamConstraints, TeamConstraintsConfig, TeamMode, TeammateSpawnRequest,
};
pub use agent_config::{
    agent_configs, claude_runtime_config, defer_merge_enabled, file_logging_enabled,
    get_agent_config, get_allowed_tools, get_effective_settings, get_preapproved_tools,
    git_runtime_config, limits_config, process_mapping, reconciliation_config,
    resolve_file_logging_early, scheduler_config, stream_timeouts, supervisor_runtime_config,
    team_constraints_config, verification_config, AgentConfig, AllRuntimeConfig, GitRuntimeConfig,
    LimitsConfig, ReconciliationConfig, SchedulerConfig, StreamTimeoutsConfig,
    SupervisorRuntimeConfig, VerificationConfig,
};
pub use claude_code_client::kill_all_tracked_processes;
pub use claude_code_client::ClaudeCodeClient;
pub use claude_code_client::{
    StreamEvent as ClientStreamEvent, StreamingSpawnResult, TeammateContext, TeammateSpawnConfig,
    TeammateSpawnResult,
};

// Re-export stream processor types for use by services
pub use stream_processor::{
    AssistantContent, AssistantMessage, ContentBlock, ContentBlockItem, ContentDelta, DiffContext,
    ParsedLine, StreamEvent, StreamMessage, StreamProcessor, StreamResult, ToolCall, ToolCallStats,
};

use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{info, warn};

/// Apply common Claude CLI environment flags for RalphX-managed spawns.
pub fn apply_common_spawn_env(cmd: &mut Command) {
    cmd.env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1");
    cmd.env("CLAUDE_CODE_ENABLE_TASKS", "1");
    cmd.env("DEBUG", "true");
}

/// Qualify a short agent name with the `ralphx:` plugin prefix.
/// If the name already contains `:`, it's assumed to be fully qualified.
pub fn qualify_agent_name(name: &str) -> String {
    if name.contains(':') {
        name.to_string()
    } else {
        format!("ralphx:{}", name)
    }
}

/// Strip the `ralphx:` plugin prefix from an agent name.
/// Used when passing agent type to the MCP server or looking up agent configs
/// (both use short/unprefixed names).
pub fn mcp_agent_type(name: &str) -> &str {
    name.strip_prefix("ralphx:").unwrap_or(name)
}

/// Build base CLI arguments for Claude Code
/// These are the common args needed for all Claude CLI invocations with streaming output
///
/// When `agent_type` is provided, creates a dynamic MCP config that passes
/// the agent type as a CLI argument to the MCP server. This is necessary because
/// Claude CLI does NOT pass its environment variables to MCP servers it spawns.
fn is_test_environment() -> bool {
    if cfg!(test) {
        return true;
    }

    if std::env::var("RUST_TEST_THREADS").is_ok() {
        return true;
    }

    if let Ok(value) = std::env::var("RALPHX_TEST_MODE") {
        return value == "1" || value.eq_ignore_ascii_case("true");
    }

    false
}

pub fn ensure_claude_spawn_allowed() -> Result<(), String> {
    if is_test_environment() {
        return Err("Claude spawn disabled in tests".to_string());
    }

    if let Ok(value) = std::env::var("RALPHX_DISABLE_CLAUDE_SPAWN") {
        if value == "1" || value.eq_ignore_ascii_case("true") {
            return Err("Claude spawn disabled by RALPHX_DISABLE_CLAUDE_SPAWN".to_string());
        }
    }

    Ok(())
}

pub fn build_base_cli_command(
    cli_path: &Path,
    plugin_dir: &Path,
    agent_type: Option<&str>,
) -> Result<Command, String> {
    ensure_claude_spawn_allowed()?;
    sanitize_claude_user_state();
    let mut cmd = Command::new(cli_path);

    // Apply common environment hardening and debug flags for CLI spawns.
    apply_common_spawn_env(&mut cmd);
    cmd.env("CLAUDE_PLUGIN_ROOT", plugin_dir);

    // Optional setting-sources override from ralphx.yaml.
    if let Some(sources) = &claude_runtime_config().setting_sources {
        if !sources.is_empty() {
            cmd.args(["--setting-sources", &sources.join(",")]);
        }
    }

    // Temporary hardening: disable slash-command skill loading to avoid
    // startup JSON parse crashes in Claude's skill initialization path.
    cmd.arg("--disable-slash-commands");

    // Plugin directory for agent/skill discovery
    cmd.args([
        "--plugin-dir",
        plugin_dir.to_str().unwrap_or("./ralphx-plugin"),
    ]);

    // Output format for streaming JSON
    cmd.args(["--output-format", "stream-json"]);

    // Required for stream-json with -p (print mode)
    cmd.arg("--verbose");

    // Capture Claude's internal debug log per spawn for post-mortem analysis.
    // This is critical when the process exits 0 with no stdout/stderr.
    let debug_path = std::env::temp_dir().join(format!(
        "ralphx-claude-debug-{}-{}.log",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    if let Some(path_str) = debug_path.to_str() {
        cmd.args(["--debug-file", path_str]);
        tracing::debug!(path = %debug_path.display(), "Enabled Claude debug file");
    }

    // Configure permission handling from ralphx.yaml.
    let runtime = claude_runtime_config();
    cmd.args(["--permission-prompt-tool", &runtime.permission_prompt_tool]);
    cmd.args(["--permission-mode", &runtime.permission_mode]);
    if runtime.dangerously_skip_permissions {
        cmd.arg("--dangerously-skip-permissions");
    }
    // Optional settings JSON passed to claude CLI via --settings.
    // Agent-specific profile overrides global profile when configured.
    if let Some(s) = get_effective_settings(agent_type) {
        if let Ok(json) = serde_json::to_string(s) {
            cmd.args(["--settings", &json]);
        }
    }

    // If agent_type is provided, create a dynamic MCP config that passes it
    // to the MCP server via CLI args (since env vars don't propagate to MCP servers).
    // Always enforce strict MCP isolation from user/global servers.
    if let Some(agent) = agent_type {
        if let Some(temp_path) = create_mcp_config(plugin_dir, agent) {
            cmd.args([
                "--mcp-config",
                temp_path.to_str().unwrap_or(""),
                "--strict-mcp-config",
            ]);
            tracing::debug!(
                path = %temp_path.display(),
                agent_type = agent,
                "Dynamic MCP config written (strict)"
            );
        }
    }

    Ok(cmd)
}

fn resolve_agent_system_prompt_path(plugin_dir: &Path, agent_name: &str) -> Option<PathBuf> {
    let short = mcp_agent_type(agent_name);
    let project_root = plugin_dir.parent().unwrap_or(plugin_dir);
    let agents_dir = plugin_dir.join("agents");
    let configured = get_agent_config(short)
        .map(|cfg| {
            let configured_path = PathBuf::from(&cfg.system_prompt_file);
            if configured_path.is_absolute() {
                configured_path
            } else {
                project_root.join(configured_path)
            }
        })
        .filter(|p| p.exists());
    let direct = agents_dir.join(format!("{short}.md"));
    let fallback = short
        .strip_prefix("ralphx-")
        .map(|s| agents_dir.join(format!("{s}.md")));

    configured
        .or_else(|| if direct.exists() { Some(direct) } else { None })
        .or_else(|| fallback.filter(|p| p.exists()))
}

fn load_agent_system_prompt(plugin_dir: &Path, agent_name: &str) -> Option<String> {
    let path = resolve_agent_system_prompt_path(plugin_dir, agent_name)?;
    let raw = std::fs::read_to_string(path).ok()?;
    if let Some(after_first) = raw.strip_prefix("---") {
        if let Some(end_idx) = after_first.find("\n---") {
            let body = &after_first[end_idx + "\n---".len()..];
            return Some(body.trim().to_string());
        }
    }
    Some(raw.trim().to_string())
}

/// Best-effort cleanup for `~/.claude.json` to avoid startup instability from
/// corrupted or stale project metadata accumulated across many worktrees.
///
/// - If JSON is malformed, back it up and write an empty object.
/// - If `projects` is present, remove entries whose filesystem path no longer exists.
pub fn sanitize_claude_user_state() {
    let home_dir = match std::env::var("HOME") {
        Ok(home) if !home.is_empty() => PathBuf::from(home),
        _ => return,
    };
    let path = home_dir.join(".claude.json");
    if !path.exists() {
        return;
    }

    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            warn!(path = %path.display(), error = %e, "Failed to read ~/.claude.json");
            return;
        }
    };

    let mut json: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            warn!(path = %path.display(), error = %e, "Malformed ~/.claude.json; rotating and recreating");
            let backup = home_dir.join(format!(
                ".claude.json.corrupt-{}-{}",
                std::process::id(),
                uuid::Uuid::new_v4().simple()
            ));
            let _ = std::fs::rename(&path, &backup);
            let _ = std::fs::write(&path, "{}");
            return;
        }
    };

    let Some(root) = json.as_object_mut() else {
        return;
    };
    let (removed, remaining, mcp_overrides_cleared) = {
        let Some(projects) = root.get_mut("projects").and_then(|v| v.as_object_mut()) else {
            return;
        };

        let before = projects.len();
        projects.retain(|project_path, _| Path::new(project_path).exists());
        let removed = before.saturating_sub(projects.len());

        // Remove per-project MCP overrides so agent runs don't inherit stale
        // config from previously visited worktrees/repositories.
        let mut cleared = 0usize;
        for entry in projects.values_mut() {
            let Some(project_obj) = entry.as_object_mut() else {
                continue;
            };
            for key in [
                "mcpServers",
                "enabledMcpjsonServers",
                "disabledMcpjsonServers",
                "disabledMcpServers",
                "mcpContextUris",
            ] {
                if project_obj.remove(key).is_some() {
                    cleared += 1;
                }
            }
        }

        (removed, projects.len(), cleared)
    };

    if removed == 0 && mcp_overrides_cleared == 0 {
        return;
    }

    let serialized = match serde_json::to_string_pretty(&json) {
        Ok(s) => s,
        Err(e) => {
            warn!(path = %path.display(), error = %e, "Failed to serialize sanitized ~/.claude.json");
            return;
        }
    };

    let temp = home_dir.join(format!(
        ".claude.json.tmp-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));

    if let Err(e) = std::fs::write(&temp, serialized) {
        warn!(path = %temp.display(), error = %e, "Failed to write temp ~/.claude.json");
        return;
    }
    if let Err(e) = std::fs::rename(&temp, &path) {
        warn!(from = %temp.display(), to = %path.display(), error = %e, "Failed to replace ~/.claude.json");
        let _ = std::fs::remove_file(&temp);
        return;
    }

    info!(
        removed = removed,
        remaining = remaining,
        mcp_overrides_cleared = mcp_overrides_cleared,
        "Sanitized ~/.claude.json project metadata"
    );
}

/// Create a dynamic MCP config temp file for an agent.
///
/// Writes a JSON config that starts the configured MCP server with the agent's type
/// passed via `--agent-type` CLI arg (for tool filtering). Returns the temp file path.
/// Uses UUID in filename to avoid race conditions between parallel agent spawns.
pub fn create_mcp_config(plugin_dir: &Path, agent_type: &str) -> Option<PathBuf> {
    // ${CLAUDE_PLUGIN_ROOT} in .mcp.json means the plugin_dir itself (e.g. ralphx-plugin/).
    // spawn_teammate_interactive sets CLAUDE_PLUGIN_ROOT=plugin_dir, so expansion must match.
    let mcp_server_path = plugin_dir.join("ralphx-mcp-server/build/index.js");
    let mcp_server_path_str = mcp_server_path.to_string_lossy().to_string();
    // Resolve node path robustly — macOS GUI apps (Dock/Finder) have stripped PATH
    // (/usr/bin:/bin only), so which::which may fail. Fall back to common install paths.
    let node_command = which::which("node")
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()))
        .unwrap_or_else(|| {
            for candidate in ["/opt/homebrew/bin/node", "/usr/local/bin/node"] {
                if std::path::Path::new(candidate).exists() {
                    return candidate.to_string();
                }
            }
            "node".to_string()
        });

    // Strip plugin prefix for MCP server's --agent-type param
    let short_name = mcp_agent_type(agent_type);
    let mcp_server_name = &claude_runtime_config().mcp_server_name;

    // Start from ralphx-plugin/.mcp.json when available, then inject agent scoping args.
    // This preserves server fields (env, headers, etc.) while still enforcing per-agent tools.
    let mcp_json_path = plugin_dir.join(".mcp.json");
    let mut server_cfg = std::fs::read_to_string(&mcp_json_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|v| {
            v.get("mcpServers")
                .and_then(|servers| servers.get(mcp_server_name))
                .cloned()
        })
        .unwrap_or_else(|| {
            serde_json::json!({
                "type": "stdio",
                "command": node_command,
                "args": [mcp_server_path_str],
            })
        });

    // Ensure server config is an object; otherwise fall back to sane defaults.
    if !server_cfg.is_object() {
        server_cfg = serde_json::json!({
            "type": "stdio",
            "command": node_command,
            "args": [mcp_server_path_str]
        });
    }

    if let Some(server_obj) = server_cfg.as_object_mut() {
        if !server_obj.contains_key("type") {
            server_obj.insert(
                "type".to_string(),
                serde_json::Value::String("stdio".to_string()),
            );
        }
        // Always resolve bare "node" to its full path so macOS GUI apps (which have
        // stripped PATH) can find node even when it's installed via nvm/Homebrew.
        let current_command = server_obj
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("node");
        let resolved_command = if current_command == "node" {
            node_command.clone()
        } else {
            current_command.to_string()
        };
        server_obj.insert(
            "command".to_string(),
            serde_json::Value::String(resolved_command),
        );

        let mut args_vec: Vec<String> = server_obj
            .get("args")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        if args_vec.is_empty() {
            args_vec.push(mcp_server_path_str);
        } else {
            // Expand ${CLAUDE_PLUGIN_ROOT} templates to the actual plugin_dir path.
            // .mcp.json uses ${CLAUDE_PLUGIN_ROOT} as the plugin dir (e.g. ralphx-plugin/).
            // Must match what spawn_teammate_interactive sets: CLAUDE_PLUGIN_ROOT=plugin_dir.
            let plugin_dir_str = plugin_dir.to_string_lossy();
            args_vec = args_vec
                .into_iter()
                .map(|a| a.replace("${CLAUDE_PLUGIN_ROOT}", &plugin_dir_str))
                .collect();
        }

        // Always pass/override --agent-type for MCP-side tool filtering.
        if let Some(idx) = args_vec.iter().position(|a| a == "--agent-type") {
            if idx + 1 < args_vec.len() {
                args_vec[idx + 1] = short_name.to_string();
            } else {
                args_vec.push(short_name.to_string());
            }
        } else {
            args_vec.push("--agent-type".to_string());
            args_vec.push(short_name.to_string());
        }

        // Inject --allowed-tools from agent's mcp_tools config (Wave 3).
        // - Agent not in config (None) → skip arg entirely (MCP server falls back to TOOL_ALLOWLIST)
        // - Agent found, empty mcp_tools → inject __NONE__ sentinel (intentional zero tools)
        // - Agent found, non-empty mcp_tools → validate names, join with commas, inject arg
        let validated_tools: Option<Vec<String>> = get_agent_config(agent_type).map(|cfg| {
            cfg.allowed_mcp_tools
                .iter()
                .filter(|name| {
                    if validate_mcp_tool_name(name) {
                        true
                    } else {
                        tracing::error!(
                            "[RalphX] Invalid MCP tool name {:?} for agent {:?} (skipped from --allowed-tools)",
                            name,
                            agent_type
                        );
                        false
                    }
                })
                .cloned()
                .collect()
        });
        if let Some(arg_value) = format_allowed_tools_arg_value(validated_tools.as_deref()) {
            args_vec.push(format!("--allowed-tools={}", arg_value));
        }

        server_obj.insert(
            "args".to_string(),
            serde_json::Value::Array(
                args_vec
                    .into_iter()
                    .map(serde_json::Value::String)
                    .collect(),
            ),
        );
    }

    let mcp_config = serde_json::json!({
        "mcpServers": {
            mcp_server_name: server_cfg
        }
    });

    let config_json = serde_json::to_string(&mcp_config).ok()?;
    let temp_path = std::env::temp_dir().join(format!(
        "ralphx-mcp-{}-{}.json",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::write(&temp_path, &config_json).ok()?;
    Some(temp_path)
}

/// A ready-to-spawn CLI command that handles stdin piping automatically.
///
/// **CLI bug workaround (2.1.38):** `--agent` + `-p "text"` causes the CLI to
/// hang silently. Piping via stdin with `-p -` works correctly. `SpawnableCommand`
/// encapsulates this so callers just call `spawn()`.
pub struct SpawnableCommand {
    cmd: Command,
    stdin_prompt: Option<String>,
}

impl std::fmt::Debug for SpawnableCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let prompt_len = self.stdin_prompt.as_ref().map(|s| s.len());
        let prompt_preview = self.stdin_prompt.as_ref().map(|s| {
            let mut out = s.chars().take(200).collect::<String>();
            if s.chars().count() > 200 {
                out.push_str("...");
            }
            out.replace('\n', "\\n")
        });
        f.debug_struct("SpawnableCommand")
            .field("cmd", &self.cmd)
            .field("uses_stdin", &self.stdin_prompt.is_some())
            .field("stdin_prompt_len", &prompt_len)
            .field("stdin_prompt_preview", &prompt_preview)
            .finish()
    }
}

impl SpawnableCommand {
    /// Set an environment variable on the underlying command.
    pub fn env(&mut self, key: &str, val: &str) -> &mut Self {
        self.cmd.env(key, val);
        self
    }

    /// Append a CLI argument to the underlying command.
    pub fn arg(&mut self, val: &str) -> &mut Self {
        self.cmd.arg(val);
        self
    }

    /// Spawn in interactive mode: writes the stored prompt to stdin, then returns
    /// the stdin handle open for future multi-turn messages.
    ///
    /// Unlike `spawn()` (which drops stdin after writing, signaling EOF), this
    /// keeps stdin alive so the caller can write additional messages later.
    ///
    /// The command uses `-p - --input-format stream-json`, so each message
    /// (including the initial prompt) is a single-line JSON object. The CLI
    /// stays in print mode (required for `--output-format stream-json`) while
    /// reading new turns from stdin until EOF.
    pub async fn spawn_interactive(
        mut self,
    ) -> std::io::Result<(tokio::process::Child, tokio::process::ChildStdin)> {
        self.cmd.kill_on_drop(true);
        let mut child = self.cmd.spawn()?;

        let mut stdin = child.stdin.take().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "no stdin pipe — ensure Stdio::piped() was set before spawn_interactive",
            )
        })?;

        // Write the stored initial prompt (if any). No deadlock risk in interactive mode:
        // the process waits for stdin input before producing stdout, so the pipe
        // buffer cannot fill up from the other direction during this write.
        if let Some(prompt) = self.stdin_prompt.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(prompt.as_bytes()).await?;
            stdin.write_all(b"\n").await?; // CLI reads lines — newline signals end of input
            stdin.flush().await?; // Ensure bytes are delivered to the process
            // stdin is intentionally NOT dropped — kept open for future messages
        }

        Ok((child, stdin))
    }

    /// Spawn the command and pipe the prompt to stdin if needed.
    ///
    /// Stdin is written in a background task to avoid a pipe deadlock:
    /// the CLI writes to stdout during init (hooks, init event), and if we
    /// block here waiting for stdin write_all to complete, neither side
    /// makes progress once the pipe buffers fill up.
    pub async fn spawn(mut self) -> std::io::Result<tokio::process::Child> {
        self.cmd.kill_on_drop(true);
        let mut child = self.cmd.spawn()?;

        // Write prompt to stdin in background (avoids deadlock with stdout pipe)
        if let Some(prompt) = self.stdin_prompt.take() {
            if let Some(stdin) = child.stdin.take() {
                tokio::spawn(async move {
                    use tokio::io::AsyncWriteExt;
                    let mut stdin = stdin;
                    if let Err(e) = stdin.write_all(prompt.as_bytes()).await {
                        tracing::warn!("Failed to write prompt to stdin: {}", e);
                    }
                    // Drop closes stdin, signaling EOF to the CLI
                });
            }
        } else {
            // If stdin is piped but no prompt is provided via stdin, close it
            // immediately so the CLI doesn't wait for additional input.
            let _ = child.stdin.take();
        }

        Ok(child)
    }
}

/// Format a message as a stream-json input line for `--input-format stream-json`.
///
/// The Claude CLI with `-p - --input-format stream-json` reads one JSON message per
/// stdin line. Each message triggers a new turn in the same session.
pub fn format_stream_json_input(content: &str) -> String {
    serde_json::json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": content
        }
    })
    .to_string()
}

/// Add prompt-related args to a CLI command.
///
/// Applies agent-specific tool restrictions via --tools flag (CLI tools)
/// and --allowedTools flag (MCP + CLI tool pre-approvals).
/// See `agent_config/` for the single source of truth on tool configurations.
///
/// When `interactive` is `true`, `-p -` + `--input-format stream-json` are added so the
/// CLI stays in print mode (required for `--output-format stream-json`) while reading
/// structured JSON messages from stdin for multi-turn conversations.
/// The returned `Option<String>` holds the prompt for stdin delivery: `Some(prompt)` in
/// both stdin-pipe mode and interactive mode, `None` when using the `-p <arg>` form.
fn add_prompt_args(
    cmd: &mut Command,
    plugin_dir: &Path,
    prompt: &str,
    agent: Option<&str>,
    resume_session: Option<&str>,
    interactive: bool,
) -> Option<String> {
    // Add resume if continuing an existing session
    if let Some(session_id) = resume_session {
        cmd.args(["--resume", session_id]);
    }

    // Default path: avoid Claude's `--agent` execution mode (currently unstable in
    // some worktree/headless scenarios) and inject the agent behavior via
    // `--append-system-prompt` loaded from our codebase agent markdown.
    // Set RALPHX_USE_NATIVE_AGENT_FLAG=1 to force native --agent mode.
    //
    let use_native_agent_flag = std::env::var("RALPHX_USE_NATIVE_AGENT_FLAG")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    // Default to stdin mode for agent runs due to CLI instability with
    // `--agent` + `-p "<text>"` on some Claude Code versions.
    // Set RALPHX_CLAUDE_PROMPT_MODE=arg to force direct -p arg mode.
    let use_stdin = if agent.is_some() {
        !matches!(std::env::var("RALPHX_CLAUDE_PROMPT_MODE"), Ok(mode) if mode.eq_ignore_ascii_case("arg"))
    } else {
        false
    };
    if let Some(agent_name) = agent {
        if use_native_agent_flag {
            cmd.args(["--agent", agent_name]);
        } else if let Some(prompt_path) = resolve_agent_system_prompt_path(plugin_dir, agent_name) {
            let runtime = claude_runtime_config();
            if runtime.use_append_system_prompt_file {
                if let Some(path_str) = prompt_path.to_str() {
                    cmd.args(["--append-system-prompt-file", path_str]);
                    tracing::debug!(
                        agent = agent_name,
                        path = path_str,
                        "Injected agent prompt via --append-system-prompt-file"
                    );
                } else if let Some(system_prompt) = load_agent_system_prompt(plugin_dir, agent_name)
                {
                    cmd.args(["--append-system-prompt", &system_prompt]);
                    tracing::debug!(
                        agent = agent_name,
                        "Injected agent prompt via --append-system-prompt"
                    );
                }
            } else if let Some(system_prompt) = load_agent_system_prompt(plugin_dir, agent_name) {
                cmd.args(["--append-system-prompt", &system_prompt]);
                tracing::debug!(
                    agent = agent_name,
                    "Injected agent prompt via --append-system-prompt"
                );
            } else {
                tracing::warn!(
                    agent = agent_name,
                    "Failed to load prompt content; falling back to native --agent"
                );
                cmd.args(["--agent", agent_name]);
            }
        } else {
            tracing::warn!(
                agent = agent_name,
                "Agent prompt not found in plugin; falling back to native --agent"
            );
            cmd.args(["--agent", agent_name]);
        }

        // Apply CLI tool restrictions from agent_config
        // Frontmatter tools/disallowedTools only work for subagent spawning,
        // NOT for direct CLI invocations with --agent -p. We must pass --tools flag.
        if let Some(allowed_tools) = get_allowed_tools(agent_name) {
            // Pass --tools even if empty (restricts to MCP-only)
            cmd.args(["--tools", &allowed_tools]);
            tracing::debug!(
                agent = agent_name,
                tools = if allowed_tools.is_empty() {
                    "(MCP only)"
                } else {
                    allowed_tools.as_str()
                },
                "Agent restricted to CLI tools"
            );
        }

        // Pre-approve tools to bypass permission prompts (MCP + CLI permissions)
        if let Some(preapproved) = get_preapproved_tools(agent_name) {
            cmd.args(["--allowedTools", &preapproved]);
            tracing::debug!(agent = agent_name, preapproved = %preapproved, "Agent pre-approved tools");
        }

        // Agent-level model from ralphx.yaml
        if let Some(agent_model) = get_agent_config(agent_name).and_then(|cfg| cfg.model.as_ref()) {
            cmd.args(["--model", agent_model]);
            tracing::debug!(agent = agent_name, model = %agent_model, "Applied agent model");
        }
    }

    if interactive {
        // --output-format stream-json only works with -p (print mode).
        // Use `-p -` to stay in print mode + `--input-format stream-json` so the CLI
        // reads structured JSON messages from stdin (one per line) for multi-turn.
        // The process stays alive until stdin EOF.
        cmd.args(["-p", "-", "--input-format", "stream-json"]);
        tracing::debug!("Claude prompt mode: interactive (-p - + stream-json input)");
        Some(format_stream_json_input(prompt))
    } else if use_stdin {
        // Workaround: pipe prompt via stdin to avoid --agent + -p arg hang (CLI 2.1.38)
        cmd.args(["-p", "-"]);
        tracing::debug!("Claude prompt mode: stdin");
        Some(prompt.to_string())
    } else {
        cmd.args(["-p", prompt]);
        tracing::debug!("Claude prompt mode: arg");
        None
    }
}

/// Configure command for spawning (working dir, stdout/stderr capture)
fn configure_spawn(cmd: &mut Command, working_dir: &Path, needs_stdin: bool) {
    cmd.current_dir(working_dir);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    // Always provide a pipe for stdin.
    // In GUI/non-TTY environments, inheriting stdin can present as closed and
    // Claude may exit early before emitting stream-json output.
    let _ = needs_stdin;
    cmd.stdin(std::process::Stdio::piped());
}

/// Build a ready-to-spawn CLI command with all args configured.
///
/// Combines `build_base_cli_command`, `add_prompt_args`, and `configure_spawn`
/// into a single `SpawnableCommand` that handles stdin piping automatically.
pub fn build_spawnable_command(
    cli_path: &Path,
    plugin_dir: &Path,
    prompt: &str,
    agent: Option<&str>,
    resume_session: Option<&str>,
    working_directory: &Path,
) -> Result<SpawnableCommand, String> {
    let mut cmd = build_base_cli_command(cli_path, plugin_dir, agent)?;
    let stdin_prompt = add_prompt_args(&mut cmd, plugin_dir, prompt, agent, resume_session, false);
    configure_spawn(&mut cmd, working_directory, stdin_prompt.is_some());
    Ok(SpawnableCommand { cmd, stdin_prompt })
}

/// Build a ready-to-spawn interactive CLI command (no `-p` flag).
///
/// Like `build_spawnable_command` but omits `-p` so the process enters
/// interactive/REPL mode. The prompt is stored for delivery via stdin
/// when `spawn_interactive()` is called.
///
/// Use `SpawnableCommand::spawn_interactive()` (instead of `spawn()`) to get
/// back the stdin handle for multi-turn message delivery.
pub fn build_spawnable_interactive_command(
    cli_path: &Path,
    plugin_dir: &Path,
    prompt: &str,
    agent: Option<&str>,
    resume_session: Option<&str>,
    working_directory: &Path,
) -> Result<SpawnableCommand, String> {
    let mut cmd = build_base_cli_command(cli_path, plugin_dir, agent)?;
    // interactive=true: no -p flag; prompt stored in stdin_prompt for spawn_interactive()
    let stdin_prompt = add_prompt_args(&mut cmd, plugin_dir, prompt, agent, resume_session, true);
    configure_spawn(&mut cmd, working_directory, true);
    Ok(SpawnableCommand { cmd, stdin_prompt })
}

/// Register the configured MCP server with Claude Code CLI.
/// This ensures the MCP server is available to Claude regardless of which project directory
/// the user is working in. The server is registered with user scope.
pub async fn register_mcp_server(cli_path: &Path, plugin_dir: &Path) -> Result<(), String> {
    let mcp_server_path = plugin_dir.join("ralphx-mcp-server/build/index.js");

    // Verify the MCP server exists
    if !mcp_server_path.exists() {
        return Err(format!(
            "MCP server not found at: {}",
            mcp_server_path.display()
        ));
    }

    let mcp_server_path_str = mcp_server_path.to_string_lossy().to_string();

    // Build the JSON config for the MCP server
    // IMPORTANT: Do NOT specify an "env" field here. The env field in MCP config
    // REPLACES the parent environment entirely (Node.js spawn behavior). We need
    // the MCP server to INHERIT RALPHX_AGENT_TYPE from Claude CLI's environment
    // (set by Rust when spawning). The MCP server defaults to http://127.0.0.1:3847
    // for TAURI_API_URL if not specified.
    let mcp_config = serde_json::json!({
        "type": "stdio",
        "command": "node",
        "args": [mcp_server_path_str]
    });

    let config_json = serde_json::to_string(&mcp_config)
        .map_err(|e| format!("Failed to serialize MCP config: {}", e))?;
    let mcp_server_name = claude_runtime_config().mcp_server_name.clone();

    // First, try to remove existing registration (ignore errors)
    let remove_result = std::process::Command::new(cli_path)
        .args(["mcp", "remove", &mcp_server_name, "-s", "user"])
        .output();

    match remove_result {
        Ok(output) => {
            if output.status.success() {
                info!(server = %mcp_server_name, "Removed existing MCP registration");
            }
            // Ignore errors - server might not exist yet
        }
        Err(e) => {
            warn!("Failed to run mcp remove (might be ok): {}", e);
        }
    }

    // Register the MCP server with user scope
    let add_result = std::process::Command::new(cli_path)
        .args([
            "mcp",
            "add-json",
            "-s",
            "user",
            &mcp_server_name,
            &config_json,
        ])
        .output()
        .map_err(|e| format!("Failed to run claude mcp add-json: {}", e))?;

    if !add_result.status.success() {
        let stderr = String::from_utf8_lossy(&add_result.stderr);
        return Err(format!("Failed to register MCP server: {}", stderr));
    }

    info!(
        server = %mcp_server_name,
        path = %mcp_server_path.display(),
        "Successfully registered MCP server"
    );

    Ok(())
}

/// Find the Claude CLI path (uses same approach as ClaudeCodeClient)
pub fn find_claude_cli() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("CLAUDE_CLI_PATH") {
        let candidate = PathBuf::from(path);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    if let Ok(path) = which::which("claude") {
        return Some(path);
    }

    let candidates = [
        "/opt/homebrew/bin/claude",
        "/usr/local/bin/claude",
        "/usr/bin/claude",
    ];

    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return Some(path);
        }
    }

    None
}

/// Find the plugin directory relative to the app
pub fn find_plugin_dir() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("RALPHX_PLUGIN_DIR") {
        let candidate = PathBuf::from(path);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    // In development, it's relative to the current working directory
    let dev_path = std::env::current_dir().ok()?.join("ralphx-plugin");

    if dev_path.exists() {
        return Some(dev_path);
    }

    // Try relative to executable
    if let Ok(exe_path) = std::env::current_exe() {
        // Go up from the executable to find the plugin dir
        // In dev: target/debug/ralphx -> ../../../ralphx-plugin
        if let Some(parent) = exe_path.parent() {
            let candidates = [
                parent.join("ralphx-plugin"),
                parent.join("../ralphx-plugin"),
                parent.join("../../ralphx-plugin"),
                parent.join("../../../ralphx-plugin"),
            ];

            for candidate in candidates {
                if let Ok(canonical) = candidate.canonicalize() {
                    if canonical.exists() {
                        return Some(canonical);
                    }
                }
            }
        }
    }

    // Production fallback: app data location where local release provisioning
    // installs plugin runtime assets.
    if let Ok(home) = std::env::var("HOME") {
        let app_data_plugin =
            PathBuf::from(home).join("Library/Application Support/com.ralphx.app/ralphx-plugin");
        if app_data_plugin.exists() {
            return Some(app_data_plugin);
        }
    }

    None
}

/// Resolve plugin directory for a specific working directory context.
///
/// Priority:
/// 1) `<working_dir>/ralphx-plugin`
/// 2) `<working_dir>/../ralphx-plugin`
/// 3) global discovery via `find_plugin_dir()`
/// 4) fallback to `<working_dir>/ralphx-plugin` (even if missing)
pub fn resolve_plugin_dir(working_dir: &Path) -> PathBuf {
    let direct = working_dir.join("ralphx-plugin");
    if direct.exists() {
        return direct;
    }

    if let Some(parent) = working_dir.parent() {
        let parent_candidate = parent.join("ralphx-plugin");
        if parent_candidate.exists() {
            return parent_candidate;
        }
    }

    find_plugin_dir().unwrap_or(direct)
}

// ============================================================================
// Wave 3 stubs — allow mod_tests.rs to compile in TDD red state.
// Tests call these functions and fail at runtime (todo!) until Wave 3 implements them.
// ============================================================================

/// Validate that an MCP tool name matches `^[a-z][a-z0-9_]*$`.
/// Returns `false` for empty strings, names starting with a digit, names with uppercase,
/// or names containing special characters (commas, spaces, hyphens, dots, etc.).
pub fn validate_mcp_tool_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Format the `--allowed-tools` arg value from an optional tool list.
/// - `None` → `None` (agent has no mcp_tools config → no arg injected)
/// - `Some([])` → `Some("__NONE__")` sentinel (explicit empty, do not fall through to TOOL_ALLOWLIST)
/// - `Some([t1, t2, ...])` → `Some("t1,t2,...")`
pub fn format_allowed_tools_arg_value(tools: Option<&[String]>) -> Option<String> {
    match tools {
        None => None,
        Some([]) => Some("__NONE__".to_string()),
        Some(tools) => Some(tools.join(",")),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
#[path = "mod_tests.rs"]
mod create_mcp_config_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// build_spawnable_command calls ensure_claude_spawn_allowed() which returns
    /// Err in tests — exercise the function up to that guard.
    #[test]
    fn test_build_spawnable_command_blocked_in_tests() {
        let result = build_spawnable_command(
            Path::new("/fake/claude"),
            Path::new("/fake/plugin"),
            "test prompt",
            None,
            None,
            Path::new("/tmp"),
        );
        // In test env, ensure_claude_spawn_allowed() returns Err
        assert!(result.is_err(), "should be blocked in test environment");
        assert!(
            result.unwrap_err().contains("disabled"),
            "error should mention spawn disabled"
        );
    }

    /// build_spawnable_interactive_command is also blocked in tests by the same guard.
    #[test]
    fn test_build_spawnable_interactive_command_blocked_in_tests() {
        let result = build_spawnable_interactive_command(
            Path::new("/fake/claude"),
            Path::new("/fake/plugin"),
            "my interactive prompt",
            None,
            None,
            Path::new("/tmp"),
        );
        assert!(result.is_err(), "should be blocked in test environment");
    }

    /// Verify SpawnableCommand::spawn_interactive is a method that exists and the type
    /// compiles correctly. The actual spawn is gated behind ensure_claude_spawn_allowed.
    #[test]
    fn test_spawnable_command_debug_impl() {
        fn assert_debug<T: std::fmt::Debug>() {}
        assert_debug::<SpawnableCommand>();
    }
}
