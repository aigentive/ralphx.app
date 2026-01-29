use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use tauri::Emitter;
use uuid::Uuid;

use super::*;
use crate::application::PermissionDecision;

pub async fn request_permission(
    State(state): State<HttpServerState>,
    Json(input): Json<PermissionRequestInput>,
) -> Json<PermissionRequestResponse> {
    let request_id = Uuid::new_v4().to_string();

    // Store pending request with metadata (clone once for storage)
    state
        .app_state
        .permission_state
        .register(
            request_id.clone(),
            input.tool_name.clone(),
            input.tool_input.clone(),
            input.context.clone(),
        )
        .await;

    // Emit Tauri event to frontend using references (no additional clones)
    if let Some(ref app_handle) = state.app_state.app_handle {
        let _ = app_handle.emit(
            "permission:request",
            serde_json::json!({
                "request_id": &request_id,
                "tool_name": &input.tool_name,
                "tool_input": &input.tool_input,
                "context": &input.context,
            }),
        );
    }

    Json(PermissionRequestResponse { request_id })
}

pub async fn await_permission(
    State(state): State<HttpServerState>,
    Path(request_id): Path<String>,
) -> Result<Json<PermissionDecision>, StatusCode> {
    // Get the receiver for this request
    let mut rx = {
        let pending = state.app_state.permission_state.pending.lock().await;
        match pending.get(&request_id).map(|req| req.sender.subscribe()) {
            Some(rx) => rx,
            None => return Err(StatusCode::NOT_FOUND),
        }
    };

    // Wait for decision with 5 minute timeout
    let timeout = tokio::time::Duration::from_secs(300);
    let start = tokio::time::Instant::now();

    // Use loop to poll for changes
    loop {
        // Check if value is Some - extract and drop borrow immediately
        let maybe_decision: Option<PermissionDecision> = {
            let current = rx.borrow();
            current.clone()
        };

        if let Some(decision) = maybe_decision {
            // Clean up
            state.app_state.permission_state.remove(&request_id).await;
            return Ok(Json(decision));
        }

        // Check timeout
        if start.elapsed() >= timeout {
            state.app_state.permission_state.remove(&request_id).await;
            return Err(StatusCode::REQUEST_TIMEOUT);
        }

        // Wait for change with remaining timeout
        let remaining = timeout.saturating_sub(start.elapsed());
        match tokio::time::timeout(remaining, rx.changed()).await {
            Ok(Ok(())) => continue, // Value changed, loop again to check
            Ok(Err(_)) => {
                // Channel closed
                state.app_state.permission_state.remove(&request_id).await;
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
            Err(_) => {
                // Timeout
                state.app_state.permission_state.remove(&request_id).await;
                return Err(StatusCode::REQUEST_TIMEOUT);
            }
        }
    }
}

pub async fn resolve_permission(
    State(state): State<HttpServerState>,
    Json(input): Json<ResolvePermissionInput>,
) -> StatusCode {
    let resolved = state
        .app_state
        .permission_state
        .resolve(
            &input.request_id,
            PermissionDecision {
                decision: input.decision,
                message: input.message,
            },
        )
        .await;

    if resolved {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}
