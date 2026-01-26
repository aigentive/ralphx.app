// Claude Code agent implementations
// Uses the claude CLI for agent interactions

mod claude_code_client;
mod stream_processor;

pub use claude_code_client::ClaudeCodeClient;
pub use claude_code_client::{StreamEvent as ClientStreamEvent, StreamingSpawnResult};

// Re-export stream processor types for use by services
pub use stream_processor::{
    StreamProcessor, StreamMessage, StreamEvent, StreamResult,
    ToolCall, ContentBlock, ContentDelta,
    AssistantMessage, AssistantContent,
};

use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{info, warn};

/// Build base CLI arguments for Claude Code
/// These are the common args needed for all Claude CLI invocations with streaming output
pub fn build_base_cli_command(cli_path: &Path, plugin_dir: &Path) -> Command {
    let mut cmd = Command::new(cli_path);

    // Plugin directory for agent/skill discovery
    cmd.args(["--plugin-dir", plugin_dir.to_str().unwrap_or("./ralphx-plugin")]);

    // Output format for streaming JSON
    cmd.args(["--output-format", "stream-json"]);

    // Required for stream-json with -p (print mode)
    cmd.arg("--verbose");

    cmd
}

/// Add prompt-related args to a CLI command
pub fn add_prompt_args(cmd: &mut Command, prompt: &str, agent: Option<&str>, resume_session: Option<&str>) {
    if let Some(session_id) = resume_session {
        // Resume existing session - Claude remembers context
        cmd.args(["--resume", session_id]);
    } else if let Some(agent_name) = agent {
        // New session with specific agent
        cmd.args(["--agent", agent_name]);
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
    let mcp_config = serde_json::json!({
        "type": "stdio",
        "command": "node",
        "args": [mcp_server_path_str],
        "env": {
            "TAURI_API_URL": "http://127.0.0.1:3847"
        }
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
    which::which("claude").ok()
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
