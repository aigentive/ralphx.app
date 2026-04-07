use serde::Serialize;
use tauri::State;

use crate::application::{
    build_ideation_lane_harness_availability, probe_claude_harness, probe_codex_harness,
    resolve_lane_harness_config, AppState, IDEATION_LANES,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdeationLaneHarnessAvailabilityResponse {
    pub project_id: Option<String>,
    pub lane: String,
    pub configured_harness: Option<String>,
    pub fallback_harness: Option<String>,
    pub effective_harness: String,
    pub fallback_activated: bool,
    pub binary_path: Option<String>,
    pub binary_found: bool,
    pub probe_succeeded: bool,
    pub available: bool,
    pub missing_core_exec_features: Vec<String>,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn get_ideation_harness_availability(
    project_id: Option<String>,
    app_state: State<'_, AppState>,
) -> Result<Vec<IdeationLaneHarnessAvailabilityResponse>, String> {
    let claude_probe = probe_claude_harness();
    let codex_probe = probe_codex_harness();
    let mut responses = Vec::with_capacity(IDEATION_LANES.len());

    for lane in IDEATION_LANES {
        let config = resolve_lane_harness_config(
            &app_state.agent_lane_settings_repo,
            project_id.as_deref(),
            lane,
        )
        .await;
        let availability =
            build_ideation_lane_harness_availability(config, &claude_probe, &codex_probe);
        responses.push(IdeationLaneHarnessAvailabilityResponse {
            project_id: project_id.clone(),
            lane: availability.lane.to_string(),
            configured_harness: availability.configured_harness.map(|value| value.to_string()),
            fallback_harness: availability.fallback_harness.map(|value| value.to_string()),
            effective_harness: availability.effective_harness.to_string(),
            fallback_activated: availability.fallback_activated,
            binary_path: availability.binary_path,
            binary_found: availability.binary_found,
            probe_succeeded: availability.probe_succeeded,
            available: availability.available,
            missing_core_exec_features: availability.missing_core_exec_features,
            error: availability.error,
        });
    }

    Ok(responses)
}
