use std::sync::Arc;

use tracing::{info, warn};

use crate::AppState;
use crate::application::TeamStateTracker;
use crate::application::runtime_wiring::build_http_app_state;
use crate::commands::ExecutionState;
use crate::http_server;
use crate::infrastructure;

pub(crate) fn start_server_boot(
    app_state: &AppState,
    app_handle: tauri::AppHandle,
    http_execution_state: Arc<ExecutionState>,
    http_team_tracker: TeamStateTracker,
) {
    // Start HTTP server for MCP proxy on port 3847
    // Create a second AppState sharing the Tauri AppState's DB connection,
    // plus shared in-memory state (question_state, permission_state, message_queue)
    // so MCP handlers and Tauri commands operate on the same data.
    let http_app_state = build_http_app_state(app_state, app_handle)
        .expect("Failed to initialize AppState for HTTP server");
    // Spawn HTTP server with pre-cloned state
    tauri::async_runtime::spawn(async move {
        if let Err(e) =
            http_server::start_http_server(http_app_state, http_execution_state, http_team_tracker)
                .await
        {
            tracing::error!("HTTP server failed: {}", e);
        }
    });

    // Register configured MCP server with Claude Code CLI
    // This ensures the MCP tools are available regardless of user's working directory
    if let (Some(cli_path), Some(plugin_dir)) = (
        infrastructure::agents::claude::find_claude_cli(),
        infrastructure::agents::claude::find_plugin_dir(),
    ) {
        info!("Registering configured MCP server...");
        tauri::async_runtime::spawn(async move {
            match infrastructure::agents::claude::register_mcp_server(&cli_path, &plugin_dir).await
            {
                Ok(()) => info!("Configured MCP server registered successfully"),
                Err(e) => warn!("Failed to register configured MCP server: {}", e),
            }
        });
    } else {
        warn!("Could not find Claude CLI or plugin directory - MCP server not registered");
    }
}
