use super::*;

/// Query params for get_pending_verification_confirmations
#[derive(Debug, serde::Deserialize)]
pub struct PendingVerificationConfirmationsQuery {
    pub project_id: String,
}

/// Get all sessions with pending verification confirmation for a project.
///
/// Returns sessions where `verification_confirmation_status = 'pending'` —
/// i.e. sessions awaiting the user's confirm/dismiss action before verification runs.
pub async fn get_pending_verification_confirmations(
    State(state): State<HttpServerState>,
    axum::extract::Query(params): axum::extract::Query<PendingVerificationConfirmationsQuery>,
) -> Result<Json<PendingVerificationConfirmationsResponse>, HttpError> {
    let project_id = ProjectId::from_string(params.project_id);

    let sessions = state
        .app_state
        .ideation_session_repo
        .get_pending_verification_confirmations(&project_id)
        .await
        .map_err(map_app_err_local)?;

    let cfg = default_verification_config();
    let available_specialists: Vec<SpecialistEntryResponse> = cfg
        .specialists
        .iter()
        .map(|s| SpecialistEntryResponse {
            name: s.name.clone(),
            display_name: s.display_name.clone(),
            description: s.description.clone(),
            dispatch_mode: s.dispatch_mode.clone(),
            enabled_by_default: s.enabled_by_default,
        })
        .collect();

    let items = sessions
        .into_iter()
        .map(|s| PendingVerificationConfirmationItem {
            session_id: s.id.as_str().to_string(),
            session_title: s.title,
            plan_artifact_id: s.plan_artifact_id.map(|id| id.as_str().to_string()),
            available_specialists: available_specialists.clone(),
        })
        .collect();

    Ok(Json(PendingVerificationConfirmationsResponse {
        sessions: items,
    }))
}
