// Claude Code agent implementations
// Uses the claude CLI for agent interactions

mod agent_config;
pub mod agent_names;
mod claude_code_client;
mod stream_processor;

pub use agent_config::{get_agent_config, get_allowed_tools, get_preapproved_tools, AgentConfig, AGENT_CONFIGS};
pub use claude_code_client::ClaudeCodeClient;
pub use claude_code_client::{StreamEvent as ClientStreamEvent, StreamingSpawnResult};

// Re-export stream processor types for use by services
pub use stream_processor::{
    StreamProcessor, StreamMessage, StreamEvent, StreamResult, ParsedLine,
    ToolCall, ContentBlock, ContentDelta, ContentBlockItem,
    AssistantMessage, AssistantContent, DiffContext,
};

use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{info, warn};

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
/// Used when passing agent type to the MCP server or looking up AGENT_CONFIGS
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
    let mut cmd = Command::new(cli_path);

    // Disable auto-updater: the CLI force-reinstalls itself on every -p invocation,
    // consuming the entire session without ever processing the prompt.
    cmd.env("DISABLE_AUTOUPDATER", "true");

    // Plugin directory for agent/skill discovery
    cmd.args(["--plugin-dir", plugin_dir.to_str().unwrap_or("./ralphx-plugin")]);

    // Output format for streaming JSON
    cmd.args(["--output-format", "stream-json"]);

    // Required for stream-json with -p (print mode)
    cmd.arg("--verbose");

    // If agent_type is provided, create a dynamic MCP config that passes it
    // to the MCP server via CLI args (since env vars don't propagate to MCP servers)
    // Use --strict-mcp-config to ignore user/global MCP servers (e.g. context7)
    // which can hang during init and block the agent from ever processing.
    if let Some(agent) = agent_type {
        if let Some(temp_path) = create_mcp_config(plugin_dir, agent) {
            cmd.args(["--mcp-config", temp_path.to_str().unwrap_or(""), "--strict-mcp-config"]);
            tracing::debug!(path = %temp_path.display(), agent_type = agent, "Dynamic MCP config written (strict)");
        }
    }

    Ok(cmd)
}

/// Create a dynamic MCP config temp file for an agent.
///
/// Writes a JSON config that starts the RalphX MCP server with the agent's type
/// passed via `--agent-type` CLI arg (for tool filtering). Returns the temp file path.
/// Uses UUID in filename to avoid race conditions between parallel agent spawns.
pub fn create_mcp_config(plugin_dir: &Path, agent_type: &str) -> Option<PathBuf> {
    let mcp_server_path = plugin_dir.join("ralphx-mcp-server/build/index.js");
    let mcp_server_path_str = mcp_server_path.to_string_lossy().to_string();

    // Strip plugin prefix for MCP server's --agent-type param
    let short_name = mcp_agent_type(agent_type);

    let mcp_config = serde_json::json!({
        "mcpServers": {
            "ralphx": {
                "type": "stdio",
                "command": "node",
                "args": [mcp_server_path_str, "--agent-type", short_name]
            }
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
        f.debug_struct("SpawnableCommand")
            .field("cmd", &self.cmd)
            .field("uses_stdin", &self.stdin_prompt.is_some())
            .finish()
    }
}

impl SpawnableCommand {
    /// Set an environment variable on the underlying command.
    pub fn env(&mut self, key: &str, val: &str) -> &mut Self {
        self.cmd.env(key, val);
        self
    }

    /// Spawn the command and pipe the prompt to stdin if needed.
    ///
    /// Stdin is written in a background task to avoid a pipe deadlock:
    /// the CLI writes to stdout during init (hooks, init event), and if we
    /// block here waiting for stdin write_all to complete, neither side
    /// makes progress once the pipe buffers fill up.
    pub async fn spawn(mut self) -> std::io::Result<tokio::process::Child> {
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
        }

        Ok(child)
    }
}

/// Add prompt-related args to a CLI command.
///
/// Applies agent-specific tool restrictions via --tools flag (CLI tools)
/// and --allowedTools flag (MCP + CLI tool pre-approvals).
/// See `agent_config.rs` for the single source of truth on tool configurations.
fn add_prompt_args(cmd: &mut Command, prompt: &str, agent: Option<&str>, resume_session: Option<&str>) -> Option<String> {
    // Add resume if continuing an existing session
    if let Some(session_id) = resume_session {
        cmd.args(["--resume", session_id]);
    }

    // CRITICAL: Always add agent if provided, even when resuming
    // This ensures disallowedTools and other agent restrictions are enforced
    // Without this, resumed sessions bypass tool restrictions
    let use_stdin = agent.is_some();
    if let Some(agent_name) = agent {
        cmd.args(["--agent", agent_name]);

        // Apply CLI tool restrictions from agent_config
        // Frontmatter tools/disallowedTools only work for subagent spawning,
        // NOT for direct CLI invocations with --agent -p. We must pass --tools flag.
        if let Some(allowed_tools) = get_allowed_tools(agent_name) {
            // Pass --tools even if empty (restricts to MCP-only)
            cmd.args(["--tools", &allowed_tools]);
            tracing::debug!(agent = agent_name, tools = if allowed_tools.is_empty() { "(MCP only)" } else { allowed_tools.as_str() }, "Agent restricted to CLI tools");
        }

        // Pre-approve tools to bypass permission prompts (MCP + CLI permissions)
        if let Some(preapproved) = get_preapproved_tools(agent_name) {
            cmd.args(["--allowedTools", &preapproved]);
            tracing::debug!(agent = agent_name, preapproved = %preapproved, "Agent pre-approved tools");
        }
    }

    if use_stdin {
        // Workaround: pipe prompt via stdin to avoid --agent + -p arg hang (CLI 2.1.38)
        cmd.args(["-p", "-"]);
        Some(prompt.to_string())
    } else {
        cmd.args(["-p", prompt]);
        None
    }
}

/// Configure command for spawning (working dir, stdout/stderr capture)
fn configure_spawn(cmd: &mut Command, working_dir: &Path, needs_stdin: bool) {
    cmd.current_dir(working_dir);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    if needs_stdin {
        cmd.stdin(std::process::Stdio::piped());
    }
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
    let stdin_prompt = add_prompt_args(&mut cmd, prompt, agent, resume_session);
    configure_spawn(&mut cmd, working_directory, stdin_prompt.is_some());
    Ok(SpawnableCommand { cmd, stdin_prompt })
}

/// Register the RalphX MCP server with Claude Code CLI
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

    // First, try to remove existing registration (ignore errors)
    let remove_result = std::process::Command::new(cli_path)
        .args(["mcp", "remove", "ralphx", "-s", "user"])
        .output();

    match remove_result {
        Ok(output) => {
            if output.status.success() {
                info!("Removed existing ralphx MCP registration");
            }
            // Ignore errors - server might not exist yet
        }
        Err(e) => {
            warn!("Failed to run mcp remove (might be ok): {}", e);
        }
    }

    // Register the MCP server with user scope
    let add_result = std::process::Command::new(cli_path)
        .args(["mcp", "add-json", "-s", "user", "ralphx", &config_json])
        .output()
        .map_err(|e| format!("Failed to run claude mcp add-json: {}", e))?;

    if !add_result.status.success() {
        let stderr = String::from_utf8_lossy(&add_result.stderr);
        return Err(format!("Failed to register MCP server: {}", stderr));
    }

    info!(
        "Successfully registered RalphX MCP server at: {}",
        mcp_server_path.display()
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
    // In development, it's relative to the current working directory
    let dev_path = std::env::current_dir()
        .ok()?
        .join("ralphx-plugin");

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

    None
}
