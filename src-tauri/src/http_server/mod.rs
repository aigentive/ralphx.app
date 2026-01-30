// HTTP server for MCP proxy - exposes Tauri commands via HTTP
// This allows the MCP server to call RalphX functionality via REST API

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use crate::application::AppState;
use crate::commands::ExecutionState;
use crate::error::AppResult;

// ============================================================================
// Submodules
// ============================================================================

mod types;
mod helpers;
mod handlers;

pub use types::*;
use handlers::*;

pub async fn start_http_server(app_state: Arc<AppState>, execution_state: Arc<ExecutionState>) -> AppResult<()> {
    let state = HttpServerState {
        app_state,
        execution_state,
    };

    let app = Router::new()
        // Ideation tools (orchestrator-ideation agent)
        .route("/api/create_task_proposal", post(create_task_proposal))
        .route("/api/update_task_proposal", post(update_task_proposal))
        .route("/api/delete_task_proposal", post(delete_task_proposal))
        .route("/api/add_proposal_dependency", post(add_proposal_dependency))
        // Dependency suggester tools (dependency-suggester agent)
        .route("/api/apply_proposal_dependencies", post(apply_proposal_dependencies))
        // Proposal query tools (orchestrator-ideation agent)
        .route("/api/list_session_proposals/:session_id", get(list_session_proposals))
        .route("/api/proposal/:proposal_id", get(get_proposal))
        // Dependency analysis tools (orchestrator-ideation agent)
        .route("/api/analyze_dependencies/:session_id", get(analyze_session_dependencies))
        // Session tools (session-namer agent)
        .route("/api/update_session_title", post(update_session_title))
        // Plan artifact tools (orchestrator-ideation agent)
        .route("/api/create_plan_artifact", post(create_plan_artifact))
        .route("/api/update_plan_artifact", post(update_plan_artifact))
        .route("/api/get_plan_artifact/:artifact_id", get(get_plan_artifact))
        .route("/api/get_plan_artifact/:artifact_id/history", get(get_plan_artifact_history))
        .route("/api/link_proposals_to_plan", post(link_proposals_to_plan))
        .route("/api/get_session_plan/:session_id", get(get_session_plan))
        // Task tools (chat-task agent)
        .route("/api/update_task", post(update_task))
        .route("/api/add_task_note", post(add_task_note))
        .route("/api/get_task_details", post(get_task_details))
        // Project tools (chat-project agent)
        .route("/api/list_tasks", post(list_tasks))
        .route("/api/suggest_task", post(suggest_task))
        // Review tools (reviewer agent)
        .route("/api/complete_review", post(complete_review))
        .route("/api/review_notes/:task_id", get(get_review_notes))
        // Review chat tools (review-chat agent) - post-review human decision
        .route("/api/approve_task", post(approve_task))
        .route("/api/request_task_changes", post(request_task_changes))
        // Worker context tools (worker agent)
        .route("/api/task_context/:task_id", get(get_task_context))
        .route("/api/artifact/:artifact_id", get(get_artifact_full))
        .route("/api/artifact/:artifact_id/version/:version", get(get_artifact_version))
        .route("/api/artifact/:artifact_id/related", get(get_related_artifacts))
        .route("/api/artifacts/search", post(search_artifacts))
        // Task step endpoints (worker agent)
        .route("/api/task_steps/:task_id", get(get_task_steps_http))
        .route("/api/start_step", post(start_step_http))
        .route("/api/complete_step", post(complete_step_http))
        .route("/api/skip_step", post(skip_step_http))
        .route("/api/fail_step", post(fail_step_http))
        .route("/api/add_step", post(add_step_http))
        .route("/api/step_progress/:task_id", get(get_step_progress_http))
        // Permission bridge endpoints
        .route("/api/permission/request", post(request_permission))
        .route("/api/permission/await/:request_id", get(await_permission))
        .route("/api/permission/resolve", post(resolve_permission))
        .with_state(state)
        .layer(CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3847")
        .await
        .map_err(|e| crate::error::AppError::Infrastructure(format!("Failed to bind HTTP server to port 3847: {}", e)))?;

    tracing::info!("MCP HTTP server listening on http://127.0.0.1:3847");

    axum::serve(listener, app)
        .await
        .map_err(|e| crate::error::AppError::Infrastructure(format!("HTTP server crashed: {}", e)))?;

    Ok(())
}
