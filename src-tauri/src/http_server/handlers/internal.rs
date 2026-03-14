// Internal HTTP handlers — no CORS, localhost-only access
//
// These endpoints are called by the internal MCP server (ralphx-mcp-server)
// running as a child process on the same machine. Since these are not browser
// clients, CORS is not required or applied.
//
// Security posture: same as all other /api/* routes — server binds to
// 127.0.0.1:3847 (localhost-only). No auth token required; access is
// restricted by network topology (only processes on the same machine can
// reach this server).

use axum::{extract::State, http::StatusCode, Json};

use super::*;
use crate::commands::ideation_commands::{
    create_cross_project_session_impl, CreateCrossProjectSessionInput, IdeationSessionResponse,
};

// ============================================================================
// GET /api/internal/projects
// ============================================================================

/// Returns all projects without scope filtering.
///
/// Unlike `GET /api/external/projects` (which requires a `ProjectScope` header
/// and filters by it), this endpoint returns every project in the database.
/// It is used by the internal MCP server to let ideation agents discover
/// available projects when orchestrating cross-project session creation.
pub async fn list_projects_internal(
    State(state): State<HttpServerState>,
) -> Result<Json<ListProjectsResponse>, StatusCode> {
    let projects = state
        .app_state
        .project_repo
        .get_all()
        .await
        .map_err(|e| {
            tracing::error!("list_projects_internal: failed to query projects: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let mut summaries = Vec::new();
    for project in &projects {
        let task_count = state
            .app_state
            .task_repo
            .count_tasks(&project.id, false, None, None)
            .await
            .unwrap_or(0);

        summaries.push(ProjectSummary {
            id: project.id.to_string(),
            name: project.name.clone(),
            description: None,
            created_at: project.created_at.to_rfc3339(),
            task_count,
        });
    }

    Ok(Json(ListProjectsResponse { projects: summaries }))
}

// ============================================================================
// POST /api/internal/cross_project/create_session
// ============================================================================

/// Creates a new ideation session in a target project by inheriting a verified
/// plan from a source session. Delegates to the shared
/// `create_cross_project_session_impl` function (same logic used by the Tauri
/// IPC command) to avoid code duplication.
///
/// Used by the internal MCP server's `create_cross_project_session` tool.
///
/// # Errors
///
/// - `400 Bad Request` — source plan is not verified, or no plan artifact.
/// - `422 Unprocessable Entity` — circular import detected, chain too deep,
///   or target path does not exist on disk.
/// - `500 Internal Server Error` — unexpected database or application error.
pub async fn create_cross_project_session_http(
    State(state): State<HttpServerState>,
    Json(input): Json<CreateCrossProjectSessionInput>,
) -> Result<Json<IdeationSessionResponse>, HttpError> {
    let app_handle = state.app_state.app_handle.as_ref().ok_or_else(|| {
        tracing::error!("create_cross_project_session_http: app_handle not available");
        HttpError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: Some("App handle not available".to_string()),
        }
    })?;

    create_cross_project_session_impl(app_handle, &state.app_state, input)
        .await
        .map(Json)
        .map_err(|e| {
            // Map well-known error prefixes to appropriate HTTP status codes.
            // `create_cross_project_session_impl` returns plain String errors
            // (matching the Tauri command contract).
            if e.contains("CIRCULAR_IMPORT")
                || e.contains("SELF_REFERENCE")
                || e.contains("CHAIN_TOO_DEEP")
            {
                tracing::warn!(
                    "create_cross_project_session_http: circular import rejected: {}",
                    e
                );
                HttpError::validation(e)
            } else if e.contains("not verified") || e.contains("no plan artifact") {
                HttpError {
                    status: StatusCode::BAD_REQUEST,
                    message: Some(e),
                }
            } else if e.contains("does not exist on disk") {
                HttpError::validation(e)
            } else {
                tracing::error!("create_cross_project_session_http: unexpected error: {}", e);
                HttpError {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    message: Some(e),
                }
            }
        })
}

#[cfg(test)]
#[path = "internal_handler_tests.rs"]
mod tests;
