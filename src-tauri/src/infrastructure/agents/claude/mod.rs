// Claude Code agent implementations
// Uses the claude CLI for agent interactions

mod agent_config;
mod claude_code_client;
mod stream_processor;

pub use agent_config::{get_agent_config, get_allowed_tools, get_allowed_mcp_tools, AgentConfig, AGENT_CONFIGS};
pub use claude_code_client::ClaudeCodeClient;
pub use claude_code_client::{StreamEvent as ClientStreamEvent, StreamingSpawnResult};

// Re-export stream processor types for use by services
pub use stream_processor::{
    StreamProcessor, StreamMessage, StreamEvent, StreamResult,
    ToolCall, ContentBlock, ContentDelta, ContentBlockItem,
    AssistantMessage, AssistantContent,
};

use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{info, warn};

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

    // Plugin directory for agent/skill discovery
    cmd.args(["--plugin-dir", plugin_dir.to_str().unwrap_or("./ralphx-plugin")]);

    // Output format for streaming JSON
    cmd.args(["--output-format", "stream-json"]);

    // Required for stream-json with -p (print mode)
    cmd.arg("--verbose");

    // If agent_type is provided, create a dynamic MCP config that passes it
    // to the MCP server via CLI args (since env vars don't propagate to MCP servers)
    if let Some(agent) = agent_type {
        let mcp_server_path = plugin_dir.join("ralphx-mcp-server/build/index.js");
        let mcp_server_path_str = mcp_server_path.to_string_lossy().to_string();

        // Create MCP config with agent type as CLI arg
        let mcp_config = serde_json::json!({
            "mcpServers": {
                "ralphx": {
                    "type": "stdio",
                    "command": "node",
                    "args": [mcp_server_path_str, "--agent-type", agent]
                }
            }
        });

        // Write to temp file and add --mcp-config arg
        if let Ok(config_json) = serde_json::to_string(&mcp_config) {
            // Use a temp file in system temp dir
            let temp_path = std::env::temp_dir().join(format!("ralphx-mcp-{}.json", std::process::id()));
            if std::fs::write(&temp_path, &config_json).is_ok() {
                cmd.args(["--mcp-config", temp_path.to_str().unwrap_or("")]);
                eprintln!("[MCP] Dynamic config written to {} with agent_type={}", temp_path.display(), agent);
            }
        }
    }

    Ok(cmd)
}

/// Add prompt-related args to a CLI command
///
/// Applies agent-specific tool restrictions via --tools flag (CLI tools)
/// and --allowedTools flag (MCP tools for permission bypass).
/// See `agent_config.rs` for the single source of truth on tool configurations.
pub fn add_prompt_args(cmd: &mut Command, prompt: &str, agent: Option<&str>, resume_session: Option<&str>) {
    // Add resume if continuing an existing session
    if let Some(session_id) = resume_session {
        cmd.args(["--resume", session_id]);
    }

    // CRITICAL: Always add agent if provided, even when resuming
    // This ensures disallowedTools and other agent restrictions are enforced
    // Without this, resumed sessions bypass tool restrictions
    if let Some(agent_name) = agent {
        cmd.args(["--agent", agent_name]);

        // Apply CLI tool restrictions from agent_config
        // Frontmatter tools/disallowedTools only work for subagent spawning,
        // NOT for direct CLI invocations with --agent -p. We must pass --tools flag.
        if let Some(allowed_tools) = get_allowed_tools(agent_name) {
            // Pass --tools even if empty (restricts to MCP-only)
            cmd.args(["--tools", allowed_tools]);
            eprintln!("[CLI] Agent {} restricted to CLI tools: {:?}", agent_name,
                if allowed_tools.is_empty() { "(MCP only)" } else { allowed_tools });
        }

        // Pre-approve MCP tools to bypass permission prompts
        if let Some(mcp_tools) = get_allowed_mcp_tools(agent_name) {
            cmd.args(["--allowedTools", &mcp_tools]);
            eprintln!("[CLI] Agent {} pre-approved MCP tools: {}", agent_name, mcp_tools);
        }
    }

    // Add the prompt
    cmd.args(["-p", prompt]);
}

/// Configure command for spawning (working dir, stdout/stderr capture)
pub fn configure_spawn(cmd: &mut Command, working_dir: &Path) {
    cmd.current_dir(working_dir);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
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
