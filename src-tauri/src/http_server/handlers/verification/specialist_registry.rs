use super::*;
use crate::application::harness_runtime_registry::default_verification_specialists;

pub async fn get_verification_specialists(
    State(_state): State<HttpServerState>,
) -> Result<Json<SpecialistsResponse>, HttpError> {
    let specialists = default_verification_specialists()
        .iter()
        .map(|s| SpecialistEntryResponse {
            name: s.name.clone(),
            display_name: s.display_name.clone(),
            description: s.description.clone(),
            dispatch_mode: s.dispatch_mode.clone(),
            enabled_by_default: s.enabled_by_default,
        })
        .collect();
    Ok(Json(SpecialistsResponse { specialists }))
}
