use crate::domain::agents::{AgentLane, AgentLaneSettings};
use crate::domain::repositories::AgentLaneSettingsRepository;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentLaneSettingsBootstrapResult {
    pub global_defaults: HashMap<AgentLane, AgentLaneSettings>,
    pub seeded_global_lanes: Vec<AgentLane>,
}

pub async fn load_or_seed_agent_lane_settings_defaults(
    agent_lane_settings_repo: Arc<dyn AgentLaneSettingsRepository>,
    desired_global_defaults: &HashMap<AgentLane, AgentLaneSettings>,
) -> Result<AgentLaneSettingsBootstrapResult, String> {
    let existing_rows = agent_lane_settings_repo
        .list_global()
        .await
        .map_err(|error| error.to_string())?;
    let mut resolved_defaults = existing_rows
        .into_iter()
        .map(|row| (row.lane, row.settings))
        .collect::<HashMap<_, _>>();
    let mut seeded_global_lanes = Vec::new();

    for (lane, settings) in desired_global_defaults {
        if resolved_defaults.contains_key(lane) {
            continue;
        }

        let row = agent_lane_settings_repo
            .upsert_global(*lane, settings)
            .await
            .map_err(|error| error.to_string())?;
        resolved_defaults.insert(row.lane, row.settings);
        seeded_global_lanes.push(*lane);
    }

    seeded_global_lanes.sort_by_key(|lane| lane.to_string());

    Ok(AgentLaneSettingsBootstrapResult {
        global_defaults: resolved_defaults,
        seeded_global_lanes,
    })
}

#[cfg(test)]
#[path = "agent_lane_settings_bootstrap_tests.rs"]
mod tests;
