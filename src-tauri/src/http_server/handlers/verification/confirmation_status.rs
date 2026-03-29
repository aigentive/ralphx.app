use super::*;

pub async fn get_confirmation_status(
    State(state): State<HttpServerState>,
    Path(session_id): Path<String>,
) -> Result<Json<ConfirmationStatusResponse>, HttpError> {
    let pending = state.app_state.pending_verifications.lock().await;
    match pending.get(&session_id) {
        Some(entry) => {
            let available_specialists = entry
                .available_specialists
                .iter()
                .map(|s| SpecialistEntryResponse {
                    name: s.name.clone(),
                    display_name: s.display_name.clone(),
                    description: s.description.clone(),
                    dispatch_mode: s.dispatch_mode.clone(),
                    enabled_by_default: s.enabled_by_default,
                })
                .collect();
            Ok(Json(ConfirmationStatusResponse {
                session_id,
                status: "pending".to_string(),
                plan_artifact_id: Some(entry.plan_artifact_id.clone()),
                available_specialists: Some(available_specialists),
            }))
        }
        None => Ok(Json(ConfirmationStatusResponse {
            session_id,
            status: "not_applicable".to_string(),
            plan_artifact_id: None,
            available_specialists: None,
        })),
    }
}
