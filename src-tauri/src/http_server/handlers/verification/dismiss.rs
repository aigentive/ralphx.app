use super::*;

pub async fn dismiss_verification(
    State(state): State<HttpServerState>,
    Json(req): Json<DismissVerificationRequest>,
) -> Result<Json<VerificationActionResponse>, HttpError> {
    let mut pending = state.app_state.pending_verifications.lock().await;
    // No-op if no pending entry exists
    pending.remove(&req.session_id);
    Ok(Json(VerificationActionResponse {
        status: "ok".to_string(),
    }))
}
