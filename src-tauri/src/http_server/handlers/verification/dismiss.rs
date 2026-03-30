use super::*;

pub async fn dismiss_verification(
    State(state): State<HttpServerState>,
    Json(req): Json<DismissVerificationRequest>,
) -> Result<Json<VerificationActionResponse>, HttpError> {
    let session_id = IdeationSessionId::from_string(req.session_id);
    state
        .app_state
        .ideation_session_repo
        .set_verification_confirmation_status(
            &session_id,
            Some(VerificationConfirmationStatus::Rejected),
        )
        .await
        .map_err(map_app_err_local)?;
    Ok(Json(VerificationActionResponse {
        status: "ok".to_string(),
    }))
}
