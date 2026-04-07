use super::*;
use crate::application::AppState;
use crate::domain::agents::{AgentHarnessKind, AgentLane, AgentLaneSettings, LogicalEffort};
use std::collections::HashMap;
use std::sync::Arc;

fn codex_defaults(model: &str, effort: LogicalEffort) -> AgentLaneSettings {
    let mut settings = AgentLaneSettings::new(AgentHarnessKind::Codex);
    settings.model = Some(model.to_string());
    settings.effort = Some(effort);
    settings.fallback_harness = Some(AgentHarnessKind::Claude);
    settings
}

#[tokio::test]
async fn test_load_or_seed_agent_lane_settings_defaults_seeds_missing_global_rows() {
    let app_state = AppState::new_test();
    let desired_defaults = HashMap::from([
        (
            AgentLane::IdeationPrimary,
            codex_defaults("gpt-5.4", LogicalEffort::XHigh),
        ),
        (
            AgentLane::IdeationVerifier,
            codex_defaults("gpt-5.4-mini", LogicalEffort::Medium),
        ),
    ]);

    let result = load_or_seed_agent_lane_settings_defaults(
        Arc::clone(&app_state.agent_lane_settings_repo),
        &desired_defaults,
    )
    .await
    .unwrap();

    assert_eq!(
        result.seeded_global_lanes,
        vec![AgentLane::IdeationPrimary, AgentLane::IdeationVerifier]
    );
    assert_eq!(
        result.global_defaults.get(&AgentLane::IdeationPrimary),
        desired_defaults.get(&AgentLane::IdeationPrimary)
    );
    assert_eq!(
        result.global_defaults.get(&AgentLane::IdeationVerifier),
        desired_defaults.get(&AgentLane::IdeationVerifier)
    );
}

#[tokio::test]
async fn test_load_or_seed_agent_lane_settings_defaults_preserves_existing_rows() {
    let app_state = AppState::new_test();
    let stored_defaults = AgentLaneSettings::new(AgentHarnessKind::Claude);
    let desired_defaults = HashMap::from([
        (
            AgentLane::IdeationPrimary,
            codex_defaults("gpt-5.4", LogicalEffort::XHigh),
        ),
        (
            AgentLane::IdeationVerifier,
            codex_defaults("gpt-5.4-mini", LogicalEffort::Medium),
        ),
    ]);

    app_state
        .agent_lane_settings_repo
        .upsert_global(AgentLane::IdeationPrimary, &stored_defaults)
        .await
        .unwrap();

    let result = load_or_seed_agent_lane_settings_defaults(
        Arc::clone(&app_state.agent_lane_settings_repo),
        &desired_defaults,
    )
    .await
    .unwrap();

    assert_eq!(result.seeded_global_lanes, vec![AgentLane::IdeationVerifier]);
    assert_eq!(
        result.global_defaults.get(&AgentLane::IdeationPrimary),
        Some(&stored_defaults)
    );
    assert_eq!(
        result.global_defaults.get(&AgentLane::IdeationVerifier),
        desired_defaults.get(&AgentLane::IdeationVerifier)
    );
}

#[tokio::test]
async fn test_load_or_seed_agent_lane_settings_defaults_leaves_empty_desired_defaults_alone() {
    let app_state = AppState::new_test();
    let desired_defaults = HashMap::new();

    let result = load_or_seed_agent_lane_settings_defaults(
        Arc::clone(&app_state.agent_lane_settings_repo),
        &desired_defaults,
    )
    .await
    .unwrap();

    assert!(result.seeded_global_lanes.is_empty());
    assert!(result.global_defaults.is_empty());
}
