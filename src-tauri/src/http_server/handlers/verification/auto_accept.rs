use super::*;

pub async fn set_auto_accept_verification(
    State(state): State<HttpServerState>,
    Json(req): Json<AutoAcceptVerificationRequest>,
) -> Result<Json<VerificationActionResponse>, HttpError> {
    let mut auto_accept = state.app_state.auto_accept_sessions.lock().await;
    if req.enabled {
        auto_accept.insert(req.session_id);
    } else {
        auto_accept.remove(&req.session_id);
    }
    Ok(Json(VerificationActionResponse {
        status: "ok".to_string(),
    }))
}
