// Claude Code agent implementations
// Uses the claude CLI for agent interactions

mod agent_config;
pub mod agent_names;
mod claude_code_client;
pub mod effort_resolver;
mod generated_plugin;
pub mod model_labels;
pub mod model_resolver;
pub mod node_utils;
mod stream_processor;

#[allow(unused_imports)]
pub use agent_config::team_config::{
    env_variant_override, get_team_constraints, validate_child_team_config, validate_team_plan,
    ApprovedTeamPlan, ApprovedTeammate, ProcessMapping, ProcessSlot, TeamConstraintError,
    TeamConstraints, TeamConstraintsConfig, TeamMode, TeammateSpawnRequest,
    resolve_process_agent,
};
pub use agent_config::{
    agent_configs, agent_harness_defaults_config, claude_runtime_config, config_path, defer_merge_enabled,
    execution_defaults_config, external_mcp_config, file_logging_enabled, get_agent_config,
    get_allowed_tools, get_effective_settings, get_effective_settings_profile, get_preapproved_tools, git_runtime_config,
    ideation_activity_threshold_secs, limits_config, process_mapping, reconciliation_config,
    resolve_file_logging_early, scheduler_config, stream_timeouts,
    supervisor_runtime_config, team_constraints_config, ui_feature_flags_config,
    validate_external_mcp_config, verification_config, AgentConfig, AllRuntimeConfig,
    AgentHarnessDefaultsConfig, ExecutionDefaultsConfig, ExternalMcpConfig, GitRuntimeConfig, LimitsConfig,
    ReconciliationConfig, SchedulerConfig, SpecialistEntry, StreamTimeoutsConfig,
    SupervisorRuntimeConfig, UiFeatureFlagsConfig, VerificationConfig,
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

// Re-export effort resolver helpers for use by services
pub use effort_resolver::{effort_bucket_for_agent, resolve_effort_with_source, resolve_ideation_effort};

// Re-export model resolver helpers for use by services
pub use model_resolver::{
    resolve_ideation_model, resolve_ideation_subagent_model_with_source, resolve_model_with_source,
    resolve_verifier_subagent_model_with_source, ResolvedModel,
};
#[allow(unused_imports)]
pub(crate) use generated_plugin::materialize_generated_plugin_dir;


use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{info, warn};

const PRIMARY_PLUGIN_DIR_REL: &str = "plugins/app";
const LEGACY_PLUGIN_DIR_REL: &str = "ralphx-plugin";

fn plugin_repo_root(plugin_dir: &Path) -> PathBuf {
    let parent = plugin_dir.parent().unwrap_or(plugin_dir);
    if plugin_dir.ends_with(Path::new(PRIMARY_PLUGIN_DIR_REL)) {
        parent
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| parent.to_path_buf())
    } else {
        parent.to_path_buf()
    }
}

#[allow(clippy::manual_find)]
fn first_existing_plugin_dir(candidates: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    candidates.into_iter().find(|candidate| candidate.exists())
}

pub(crate) fn find_base_plugin_dir() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("RALPHX_PLUGIN_DIR") {
        let candidate = PathBuf::from(path);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    // In development, prefer plugins/app and keep ralphx-plugin as a legacy fallback.
    if let Ok(current_dir) = std::env::current_dir() {
        if let Some(candidate) = first_existing_plugin_dir([
            current_dir.join(PRIMARY_PLUGIN_DIR_REL),
            current_dir.join(LEGACY_PLUGIN_DIR_REL),
        ]) {
            return Some(candidate);
        }
    }

    // Try relative to executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let candidates = [
                parent.join(PRIMARY_PLUGIN_DIR_REL),
                parent.join(LEGACY_PLUGIN_DIR_REL),
                parent.join(format!("../{PRIMARY_PLUGIN_DIR_REL}")),
                parent.join(format!("../{LEGACY_PLUGIN_DIR_REL}")),
                parent.join(format!("../../{PRIMARY_PLUGIN_DIR_REL}")),
                parent.join(format!("../../{LEGACY_PLUGIN_DIR_REL}")),
                parent.join(format!("../../../{PRIMARY_PLUGIN_DIR_REL}")),
                parent.join(format!("../../../{LEGACY_PLUGIN_DIR_REL}")),
            ];

            if let Some(candidate) = first_existing_plugin_dir(candidates) {
                return Some(candidate);
            }
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        if let Some(candidate) = first_existing_plugin_dir([
            PathBuf::from(&home).join(format!(
                "Library/Application Support/com.ralphx.app/{PRIMARY_PLUGIN_DIR_REL}"
            )),
            PathBuf::from(home)
                .join("Library/Application Support/com.ralphx.app/ralphx-plugin"),
        ]) {
            return Some(candidate);
        }
    }

    None
}

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

/// Resolve the `--effort` level for a given agent type.
///
/// Priority: `AgentConfig.effort` > `ClaudeRuntimeConfig.default_effort`
pub fn resolve_effort(agent_type: Option<&str>) -> String {
    let default = claude_runtime_config().default_effort.clone();
    match agent_type {
        Some(name) => get_agent_config(name)
            .and_then(|c| c.effort.clone())
            .unwrap_or(default),
        None => default,
    }
}

/// Resolve the `--model` value for a given agent type.
///
/// Priority: `AgentConfig.model` > hardcoded default `"sonnet"`.
/// Used as the YAML fallback layer (levels 3–4) in the ideation model resolution chain.
pub fn resolve_model(agent_type: Option<&str>) -> String {
    match agent_type {
        Some(name) => get_agent_config(name)
            .and_then(|c| c.model.clone())
            .unwrap_or_else(|| "sonnet".to_string()),
        None => "sonnet".to_string(),
    }
}

/// Resolve the `--permission-mode` for a given agent type.
///
/// Priority: `AgentConfig.permission_mode` > `ClaudeRuntimeConfig.permission_mode`
pub fn resolve_permission_mode(agent_type: Option<&str>) -> String {
    let default = claude_runtime_config().permission_mode.clone();
    match agent_type {
        Some(name) => get_agent_config(name)
            .and_then(|c| c.permission_mode.clone())
            .unwrap_or(default),
        None => default,
    }
}

pub fn build_base_cli_command(
    cli_path: &Path,
    plugin_dir: &Path,
    agent_type: Option<&str>,
    is_external_mcp: bool,
    effort_override: Option<&str>,
    model_override: Option<&str>,
) -> Result<Command, String> {
    build_base_cli_command_inner(
        cli_path,
        plugin_dir,
        agent_type,
        is_external_mcp,
        effort_override,
        model_override,
        true,
    )
}

fn build_base_cli_command_inner(
    cli_path: &Path,
    plugin_dir: &Path,
    agent_type: Option<&str>,
    is_external_mcp: bool,
    effort_override: Option<&str>,
    model_override: Option<&str>,
    enforce_spawn_guard: bool,
) -> Result<Command, String> {
    if enforce_spawn_guard {
        ensure_claude_spawn_allowed()?;
    }
    sanitize_claude_user_state();
    let mut cmd = Command::new(cli_path);

    // Apply common environment hardening and debug flags for CLI spawns.
    apply_common_spawn_env(&mut cmd);
    cmd.env("CLAUDE_PLUGIN_ROOT", plugin_dir);

    // Propagate external trigger context so MCP server can set child session origin correctly.
    if is_external_mcp {
        cmd.env("RALPHX_IS_EXTERNAL_TRIGGER", "1");
    }

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
        plugin_dir.to_str().unwrap_or("./plugins/app"),
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
    let permission_mode = resolve_permission_mode(agent_type);
    cmd.args(["--permission-mode", &permission_mode]);
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

    // Effort level for this agent — use explicit override when provided, otherwise resolve from config.
    let effort_resolved;
    let effort = match effort_override {
        Some(e) => e,
        None => {
            effort_resolved = resolve_effort(agent_type);
            &effort_resolved
        }
    };
    cmd.args(["--effort", effort]);

    // Model for this agent — use explicit override when provided, otherwise resolve from agent config.
    let model_resolved;
    let model = match model_override {
        Some(m) => Some(m),
        None => {
            model_resolved = agent_type
                .and_then(|a| get_agent_config(a))
                .and_then(|cfg| cfg.model.clone());
            model_resolved.as_deref()
        }
    };
    if let Some(m) = model {
        cmd.args(["--model", m]);
    }

    // If agent_type is provided, create a dynamic MCP config that passes it
    // to the MCP server via CLI args (since env vars don't propagate to MCP servers).
    // Always enforce strict MCP isolation from user/global servers.
    // Hard error on invalid config — MCP is critical infra, fail loud.
    if let Some(agent) = agent_type {
        let temp_path = create_mcp_config(plugin_dir, agent, is_external_mcp).map_err(|e| {
            tracing::error!(error = %e, agent = %agent, "MCP config creation failed");
            e
        })?;
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

    Ok(cmd)
}

pub(crate) fn resolve_agent_system_prompt_path(
    plugin_dir: &Path,
    agent_name: &str,
) -> Option<PathBuf> {
    let short = mcp_agent_type(agent_name);
    let project_root = plugin_repo_root(plugin_dir);
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

pub(crate) fn load_agent_system_prompt(plugin_dir: &Path, agent_name: &str) -> Option<String> {
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

/// Validate a generated MCP config JSON value for required fields.
///
/// Checks that the config has `mcpServers`, at least one server entry, and that
/// each server entry has `command` and `args`. Returns an error message on failure.
pub(crate) fn validate_mcp_config_json(
    config: &serde_json::Value,
    server_name: &str,
) -> Result<(), String> {
    let mcp_servers = config
        .get("mcpServers")
        .ok_or_else(|| "missing 'mcpServers' key".to_string())?;

    let server = mcp_servers
        .get(server_name)
        .ok_or_else(|| format!("missing server entry '{server_name}' in mcpServers"))?;

    if server.get("command").is_none() {
        return Err(format!(
            "server '{server_name}' missing required 'command' field"
        ));
    }
    if server.get("args").is_none() {
        return Err(format!(
            "server '{server_name}' missing required 'args' field"
        ));
    }

    Ok(())
}

/// MCP tools that require live human interaction and must be excluded when an agent
/// is spawned from an external (non-interactive) context such as an external MCP request.
/// Without this filter the agent would long-poll for human input that never arrives → deadlock.
pub const INTERACTIVE_TOOLS: &[&str] = &["ask_user_question"];

/// Remove `INTERACTIVE_TOOLS` entries from `tools`, returning a filtered list.
pub fn filter_interactive_tools(tools: &[String]) -> Vec<String> {
    tools
        .iter()
        .filter(|name| !INTERACTIVE_TOOLS.contains(&name.as_str()))
        .cloned()
        .collect()
}

/// Create a dynamic MCP config temp file for an agent.
///
/// Writes a JSON config that starts the configured MCP server with the agent's type
/// passed via `--agent-type` CLI arg (for tool filtering). Returns the temp file path.
/// Uses UUID in filename to avoid race conditions between parallel agent spawns.
///
/// When `is_external_mcp` is `true`, interactive-only tools (see `INTERACTIVE_TOOLS`) are
/// stripped from the `--allowed-tools` arg to prevent deadlocks in unattended contexts.
///
/// # Errors
///
/// Returns `Err` when the config JSON fails validation (missing required fields) or
/// when the temp file cannot be written. Errors propagate to agent spawn failure.
pub fn create_mcp_config(
    plugin_dir: &Path,
    agent_type: &str,
    is_external_mcp: bool,
) -> Result<PathBuf, String> {
    // ${CLAUDE_PLUGIN_ROOT} in .mcp.json means the plugin_dir itself.
    // spawn_teammate_interactive sets CLAUDE_PLUGIN_ROOT=plugin_dir, so expansion must match.
    let mcp_server_path = plugin_dir.join("ralphx-mcp-server/build/index.js");
    let mcp_server_path_str = mcp_server_path.to_string_lossy().to_string();
    // Resolve node path robustly — delegates to node_utils::find_node_binary() so
    // both stdio MCP registration and the external MCP supervisor use identical logic.
    let node_command = node_utils::find_node_binary()
        .to_string_lossy()
        .into_owned();

    // Strip plugin prefix for MCP server's --agent-type param
    let short_name = mcp_agent_type(agent_type);
    let mcp_server_name = &claude_runtime_config().mcp_server_name;

    // Start from plugin_dir/.mcp.json when available, then inject agent scoping args.
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
            // .mcp.json uses ${CLAUDE_PLUGIN_ROOT} as the plugin dir.
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
        // When is_external_mcp=true, strip interactive-only tools (e.g. ask_user_question) to
        // prevent deadlocks where the agent waits for human input that will never arrive.
        let validated_tools: Option<Vec<String>> = get_agent_config(agent_type).map(|cfg| {
            let tools: Vec<String> = cfg.allowed_mcp_tools
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
                .collect();
            if is_external_mcp {
                filter_interactive_tools(&tools)
            } else {
                tools
            }
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

    // Validate required fields before writing. This is always valid when built with
    // serde_json::json!, but explicit validation catches future regressions early.
    validate_mcp_config_json(&mcp_config, mcp_server_name)
        .map_err(|e| format!("Critical: MCP server config invalid — {e}"))?;

    let config_json = serde_json::to_string(&mcp_config)
        .map_err(|e| format!("Failed to serialize MCP config: {e}"))?;
    let temp_path = std::env::temp_dir().join(format!(
        "ralphx-mcp-{}-{}.json",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    {
        use std::io::Write as _;
        use std::os::unix::fs::OpenOptionsExt as _;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(&temp_path)
            .map_err(|e| format!("Failed to create MCP config temp file: {e}"))?;
        f.write_all(config_json.as_bytes())
            .map_err(|e| format!("Failed to write MCP config temp file: {e}"))?;
    }
    Ok(temp_path)
}

/// A ready-to-spawn CLI command that handles stdin piping automatically.
///
/// **CLI bug workaround (2.1.38):** `--agent` + `-p "text"` causes the CLI to
/// hang silently. Piping via stdin with `-p -` works correctly. `SpawnableCommand`
/// encapsulates this so callers just call `spawn()`.
pub struct SpawnableCommand {
    cmd: Command,
    stdin_prompt: Option<String>,
    prompt_arg_debug_redaction: Option<PromptArgDebugRedaction>,
}

#[derive(Debug, Clone)]
struct PromptArgDebugRedaction {
    arg_index: usize,
    artifact_path: PathBuf,
}

struct DebugCommandView<'a> {
    cmd: &'a Command,
    prompt_arg_debug_redaction: Option<&'a PromptArgDebugRedaction>,
}

impl std::fmt::Debug for DebugCommandView<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let std_cmd = self.cmd.as_std();
        let mut args = std_cmd
            .get_args()
            .map(|value| value.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        if let Some(redaction) = self.prompt_arg_debug_redaction {
            if let Some(arg) = args.get_mut(redaction.arg_index) {
                *arg = format!(
                    "<prompt logged at {}>",
                    redaction.artifact_path.display()
                );
            }
        }

        let envs = std_cmd
            .get_envs()
            .filter_map(|(key, value)| {
                value.map(|val| {
                    (
                        key.to_string_lossy().into_owned(),
                        val.to_string_lossy().into_owned(),
                    )
                })
            })
            .collect::<Vec<_>>();

        f.debug_struct("Command")
            .field("program", &std_cmd.get_program().to_string_lossy().into_owned())
            .field(
                "current_dir",
                &std_cmd
                    .get_current_dir()
                    .map(|path| path.to_string_lossy().into_owned()),
            )
            .field("args", &args)
            .field("envs", &envs)
            .finish()
    }
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
            .field(
                "cmd",
                &DebugCommandView {
                    cmd: &self.cmd,
                    prompt_arg_debug_redaction: self.prompt_arg_debug_redaction.as_ref(),
                },
            )
            .field("uses_stdin", &self.stdin_prompt.is_some())
            .field("stdin_prompt_len", &prompt_len)
            .field("stdin_prompt_preview", &prompt_preview)
            .finish()
    }
}

impl SpawnableCommand {
    pub(crate) fn new(cmd: Command, stdin_prompt: Option<String>) -> Self {
        Self {
            cmd,
            stdin_prompt,
            prompt_arg_debug_redaction: None,
        }
    }

    pub(crate) fn with_prompt_arg_debug_redaction(
        mut self,
        arg_index: usize,
        artifact_path: PathBuf,
    ) -> Self {
        self.prompt_arg_debug_redaction = Some(PromptArgDebugRedaction {
            arg_index,
            artifact_path,
        });
        self
    }

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

    /// Returns environment variables explicitly set on this command.
    ///
    /// For use in tests only — verifies env-var injection without spawning the process.
    #[doc(hidden)]
    pub fn get_envs_for_test(&self) -> Vec<(std::ffi::OsString, std::ffi::OsString)> {
        self.cmd
            .as_std()
            .get_envs()
            .filter_map(|(k, v)| v.map(|val| (k.to_os_string(), val.to_os_string())))
            .collect()
    }

    /// Returns CLI arguments currently configured on this command.
    #[doc(hidden)]
    pub fn get_args_for_test(&self) -> Vec<String> {
        self.cmd
            .as_std()
            .get_args()
            .map(|value| value.to_string_lossy().into_owned())
            .collect()
    }

    /// Returns the stored stdin prompt for test assertions.
    #[doc(hidden)]
    pub fn get_stdin_prompt_for_test(&self) -> Option<&str> {
        self.stdin_prompt.as_deref()
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
            std::io::Error::other(
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
    effort_override: Option<&str>,
    model_override: Option<&str>,
) -> Result<SpawnableCommand, String> {
    let mut cmd =
        build_base_cli_command(cli_path, plugin_dir, agent, false, effort_override, model_override)?;
    let stdin_prompt = add_prompt_args(&mut cmd, plugin_dir, prompt, agent, resume_session, false);
    configure_spawn(&mut cmd, working_directory, stdin_prompt.is_some());
    Ok(SpawnableCommand::new(cmd, stdin_prompt))
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
    is_external_mcp: bool,
    effort_override: Option<&str>,
    model_override: Option<&str>,
) -> Result<SpawnableCommand, String> {
    let mut cmd = build_base_cli_command(
        cli_path,
        plugin_dir,
        agent,
        is_external_mcp,
        effort_override,
        model_override,
    )?;
    // interactive=true: no -p flag; prompt stored in stdin_prompt for spawn_interactive()
    let stdin_prompt = add_prompt_args(&mut cmd, plugin_dir, prompt, agent, resume_session, true);
    configure_spawn(&mut cmd, working_directory, true);
    Ok(SpawnableCommand::new(cmd, stdin_prompt))
}

#[cfg(test)]
pub fn build_spawnable_interactive_command_for_test(
    cli_path: &Path,
    plugin_dir: &Path,
    prompt: &str,
    agent: Option<&str>,
    resume_session: Option<&str>,
    working_directory: &Path,
    is_external_mcp: bool,
    effort_override: Option<&str>,
    model_override: Option<&str>,
) -> Result<SpawnableCommand, String> {
    let mut cmd = build_base_cli_command_inner(
        cli_path,
        plugin_dir,
        agent,
        is_external_mcp,
        effort_override,
        model_override,
        false,
    )?;
    let stdin_prompt = add_prompt_args(&mut cmd, plugin_dir, prompt, agent, resume_session, true);
    configure_spawn(&mut cmd, working_directory, true);
    Ok(SpawnableCommand::new(cmd, stdin_prompt))
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
    let base_plugin_dir = find_base_plugin_dir()?;
    match generated_plugin::materialize_generated_plugin_dir(&base_plugin_dir) {
        Ok(generated_dir) => Some(generated_dir),
        Err(error) => {
            warn!(
                base_plugin_dir = %base_plugin_dir.display(),
                error = %error,
                "Failed to materialize generated Claude plugin dir; falling back to base plugin dir"
            );
            Some(base_plugin_dir)
        }
    }
}

/// Resolve plugin directory for a specific working directory context.
///
/// Priority:
/// 1) `<working_dir>/plugins/app`
/// 2) `<working_dir>/ralphx-plugin`
/// 3) `<working_dir>/../plugins/app`
/// 4) `<working_dir>/../ralphx-plugin`
/// 5) global discovery via `find_plugin_dir()`
/// 6) fallback to `<working_dir>/plugins/app` (even if missing)
pub fn resolve_base_plugin_dir(working_dir: &Path) -> PathBuf {
    if let Some(candidate) = first_existing_plugin_dir([
        working_dir.join(PRIMARY_PLUGIN_DIR_REL),
        working_dir.join(LEGACY_PLUGIN_DIR_REL),
    ]) {
        return candidate;
    }

    if let Some(parent) = working_dir.parent() {
        if let Some(candidate) = first_existing_plugin_dir([
            parent.join(PRIMARY_PLUGIN_DIR_REL),
            parent.join(LEGACY_PLUGIN_DIR_REL),
        ]) {
            return candidate;
        }
    }

    find_base_plugin_dir().unwrap_or_else(|| working_dir.join(PRIMARY_PLUGIN_DIR_REL))
}

pub fn resolve_plugin_dir(working_dir: &Path) -> PathBuf {
    let base_plugin_dir = resolve_base_plugin_dir(working_dir);
    if !base_plugin_dir.exists() {
        return base_plugin_dir;
    }

    match generated_plugin::materialize_generated_plugin_dir(&base_plugin_dir) {
        Ok(generated_dir) => generated_dir,
        Err(error) => {
            warn!(
                base_plugin_dir = %base_plugin_dir.display(),
                error = %error,
                "Failed to materialize generated Claude plugin dir for working directory; falling back to base plugin dir"
            );
            base_plugin_dir
        }
    }
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
    use std::path::{Path, PathBuf};

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
            None,
            None,
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
            false,
            None,
            None,
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

    /// build_base_cli_command with is_external_mcp=true is also blocked in tests by the
    /// same spawn guard. The env var propagation logic (RALPHX_IS_EXTERNAL_TRIGGER=1)
    /// executes after the guard; this test confirms the function accepts the flag and
    /// returns the expected blocked error in the test environment.
    #[test]
    fn test_build_base_cli_command_external_mcp_blocked_in_tests() {
        let result = build_base_cli_command(
            Path::new("/fake/claude"),
            Path::new("/fake/plugin"),
            None,
            true, // is_external_mcp=true
            None,
            None,
        );
        assert!(result.is_err(), "should be blocked in test environment");
        assert!(
            result.unwrap_err().contains("disabled"),
            "error should mention spawn disabled"
        );
    }

    /// build_base_cli_command with is_external_mcp=false is also blocked in tests.
    #[test]
    fn test_build_base_cli_command_internal_mcp_blocked_in_tests() {
        let result = build_base_cli_command(
            Path::new("/fake/claude"),
            Path::new("/fake/plugin"),
            None,
            false, // is_external_mcp=false
            None,
            None,
        );
        assert!(result.is_err(), "should be blocked in test environment");
    }

    #[test]
    fn test_plugin_repo_root_supports_nested_plugins_app_layout() {
        let plugin_dir = PathBuf::from("/tmp/ralphx/plugins/app");
        assert_eq!(plugin_repo_root(&plugin_dir), PathBuf::from("/tmp/ralphx"));
    }

    #[test]
    fn test_resolve_plugin_dir_prefers_plugins_app_in_working_dir() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let working_dir = temp_dir.path().join("repo");
        std::fs::create_dir_all(working_dir.join(PRIMARY_PLUGIN_DIR_REL)).unwrap();
        std::fs::create_dir_all(working_dir.join(PRIMARY_PLUGIN_DIR_REL).join("agents")).unwrap();
        std::fs::create_dir_all(
            working_dir.join(PRIMARY_PLUGIN_DIR_REL).join("ralphx-mcp-server/build"),
        )
        .unwrap();
        std::fs::write(
            working_dir.join(PRIMARY_PLUGIN_DIR_REL).join(".mcp.json"),
            r#"{"mcpServers":{"ralphx":{"type":"stdio","command":"node","args":["${CLAUDE_PLUGIN_ROOT}/ralphx-mcp-server/build/index.js"]}}}"#,
        )
        .unwrap();
        std::fs::write(
            working_dir.join(PRIMARY_PLUGIN_DIR_REL).join("agents/worker.md"),
            "---\nname: worker\n---\nLegacy Worker Prompt",
        )
        .unwrap();
        std::fs::create_dir_all(working_dir.join(LEGACY_PLUGIN_DIR_REL)).unwrap();

        assert_eq!(
            resolve_base_plugin_dir(&working_dir),
            working_dir.join(PRIMARY_PLUGIN_DIR_REL)
        );
        assert_eq!(
            resolve_plugin_dir(&working_dir),
            working_dir.join(".artifacts/generated/claude-plugin")
        );
    }

    #[test]
    fn test_resolve_plugin_dir_falls_back_to_legacy_working_dir() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let working_dir = temp_dir.path().join("repo");
        std::fs::create_dir_all(working_dir.join(LEGACY_PLUGIN_DIR_REL)).unwrap();

        assert_eq!(
            resolve_base_plugin_dir(&working_dir),
            working_dir.join(LEGACY_PLUGIN_DIR_REL)
        );
    }

    #[test]
    fn test_resolve_plugin_dir_checks_parent_plugins_app_before_legacy() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let repo_root = temp_dir.path().join("repo");
        let working_dir = repo_root.join("frontend");
        std::fs::create_dir_all(repo_root.join(PRIMARY_PLUGIN_DIR_REL)).unwrap();
        std::fs::create_dir_all(repo_root.join(LEGACY_PLUGIN_DIR_REL)).unwrap();
        std::fs::create_dir_all(&working_dir).unwrap();

        assert_eq!(
            resolve_base_plugin_dir(&working_dir),
            repo_root.join(PRIMARY_PLUGIN_DIR_REL)
        );
    }

    #[test]
    fn test_materialize_generated_plugin_dir_generates_canonical_agents_and_preserves_runtime_assets() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let repo_root = temp_dir.path().join("repo");
        let plugin_dir = repo_root.join(PRIMARY_PLUGIN_DIR_REL);
        std::fs::create_dir_all(plugin_dir.join("agents")).unwrap();
        std::fs::create_dir_all(plugin_dir.join("ralphx-mcp-server/build")).unwrap();
        std::fs::write(plugin_dir.join("ralphx-mcp-server/build/index.js"), "// fake").unwrap();
        std::fs::write(
            plugin_dir.join(".mcp.json"),
            r#"{"mcpServers":{"ralphx":{"type":"stdio","command":"node","args":["${CLAUDE_PLUGIN_ROOT}/ralphx-mcp-server/build/index.js"]}}}"#,
        )
        .unwrap();
        std::fs::write(
            plugin_dir.join("agents/session-namer.md"),
            "---\nname: session-namer\n---\nLegacy Session Namer Prompt",
        )
        .unwrap();
        std::fs::write(
            plugin_dir.join("agents/worker.md"),
            "---\nname: worker\n---\nLegacy Worker Prompt",
        )
        .unwrap();

        std::fs::create_dir_all(repo_root.join("agents/session-namer/shared")).unwrap();
        std::fs::write(
            repo_root.join("agents/session-namer/agent.yaml"),
            "name: session-namer\nrole: session_namer\n",
        )
        .unwrap();
        std::fs::write(
            repo_root.join("agents/session-namer/shared/prompt.md"),
            "Canonical Session Namer Prompt",
        )
        .unwrap();

        let generated_dir =
            materialize_generated_plugin_dir(&plugin_dir).expect("generated plugin dir");
        let generated_session_namer = std::fs::read_to_string(
            generated_dir.join("agents/session-namer.md"),
        )
        .expect("generated session namer");
        assert!(
            generated_session_namer.contains("Canonical Session Namer Prompt"),
            "generated session namer should use canonical prompt body"
        );
        assert!(
            generated_session_namer.contains("name: session-namer"),
            "generated session namer should preserve legacy frontmatter"
        );

        let worker_metadata = std::fs::symlink_metadata(generated_dir.join("agents/worker.md"))
            .expect("worker metadata");
        assert!(
            worker_metadata.file_type().is_symlink(),
            "non-canonical agent prompts should remain linked from the base plugin dir"
        );
        assert!(
            generated_dir.join("ralphx-mcp-server/build/index.js").exists(),
            "generated plugin dir should keep MCP runtime assets available"
        );
    }
}
