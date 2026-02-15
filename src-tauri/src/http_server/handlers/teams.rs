// Team HTTP handlers — teammate spawn requests from MCP proxy

use axum::{extract::State, http::StatusCode, Json};
use tracing::error;

use super::HttpServerState;

use serde::{Deserialize, Serialize};

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct RequestTeammateSpawnRequest {
    pub team_name: String,
    pub teammate_name: String,
    pub color: String,
    pub model: String,
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct RequestTeammateSpawnResponse {
    pub success: bool,
    pub message: String,
    pub teammate_name: String,
}

// ============================================================================
// Handler
// ============================================================================

/// HTTP handler: register a teammate spawn request.
///
/// This records the teammate in TeamStateTracker (status = Spawning) so the
/// frontend can start showing the teammate card immediately. The actual process
/// spawn is managed by the ChatService/spawn orchestrator on the Tauri side.
pub async fn request_teammate_spawn(
    State(state): State<HttpServerState>,
    Json(req): Json<RequestTeammateSpawnRequest>,
) -> Result<Json<RequestTeammateSpawnResponse>, StatusCode> {
    let tracker = &state.team_tracker;

    // Validate team exists
    if !tracker.team_exists(&req.team_name).await {
        error!("Team not found: {}", req.team_name);
        return Err(StatusCode::NOT_FOUND);
    }

    // Add teammate (will be in Spawning status)
    tracker
        .add_teammate(
            &req.team_name,
            &req.teammate_name,
            &req.color,
            &req.model,
            &req.role,
        )
        .await
        .map_err(|e| {
            error!(
                "Failed to add teammate '{}' to team '{}': {}",
                req.teammate_name, req.team_name, e
            );
            StatusCode::CONFLICT
        })?;

    Ok(Json(RequestTeammateSpawnResponse {
        success: true,
        message: format!(
            "Teammate '{}' registered in team '{}'",
            req.teammate_name, req.team_name
        ),
        teammate_name: req.teammate_name,
    }))
}
