use serde::Serialize;
use tauri::State;

use crate::application::{
    build_lane_harness_availability, probe_supported_harnesses, resolve_lane_harness_config,
    AppState, AGENT_LANES, IDEATION_LANES,
};
use crate::domain::agents::AgentLane;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentLaneHarnessAvailabilityResponse {
    pub project_id: Option<String>,
    pub lane: String,
    pub configured_harness: Option<String>,
    pub effective_harness: String,
    pub binary_path: Option<String>,
    pub binary_found: bool,
    pub probe_succeeded: bool,
    pub available: bool,
    pub missing_core_exec_features: Vec<String>,
    pub error: Option<String>,
}

pub type LaneHarnessAvailabilityResponse = AgentLaneHarnessAvailabilityResponse;
pub type IdeationLaneHarnessAvailabilityResponse = AgentLaneHarnessAvailabilityResponse;

fn to_response(
    project_id: &Option<String>,
    availability: crate::application::ideation_harness_availability::LaneHarnessAvailability,
) -> AgentLaneHarnessAvailabilityResponse {
    AgentLaneHarnessAvailabilityResponse {
        project_id: project_id.clone(),
        lane: availability.lane.to_string(),
        configured_harness: availability
            .configured_harness
            .map(|value| value.to_string()),
        effective_harness: availability.effective_harness.to_string(),
        binary_path: availability.binary_path,
        binary_found: availability.binary_found,
        probe_succeeded: availability.probe_succeeded,
        available: availability.available,
        missing_core_exec_features: availability.missing_core_exec_features,
        error: availability.error,
    }
}

async fn get_harness_availability_for_lanes(
    project_id: Option<String>,
    app_state: State<'_, AppState>,
    lanes: &[AgentLane],
) -> Result<Vec<AgentLaneHarnessAvailabilityResponse>, String> {
    let probes = probe_supported_harnesses();
    let mut responses = Vec::with_capacity(lanes.len());

    for lane in lanes {
        let config = resolve_lane_harness_config(
            &app_state.agent_lane_settings_repo,
            project_id.as_deref(),
            *lane,
        )
        .await;
        let availability = build_lane_harness_availability(config, &probes);
        responses.push(to_response(&project_id, availability));
    }

    Ok(responses)
}

#[tauri::command]
pub async fn get_ideation_harness_availability(
    project_id: Option<String>,
    app_state: State<'_, AppState>,
) -> Result<Vec<IdeationLaneHarnessAvailabilityResponse>, String> {
    get_harness_availability_for_lanes(project_id, app_state, &IDEATION_LANES).await
}

#[tauri::command]
pub async fn get_agent_harness_availability(
    project_id: Option<String>,
    app_state: State<'_, AppState>,
) -> Result<Vec<AgentLaneHarnessAvailabilityResponse>, String> {
    get_harness_availability_for_lanes(project_id, app_state, &AGENT_LANES).await
}
