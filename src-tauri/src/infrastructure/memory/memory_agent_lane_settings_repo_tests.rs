use super::*;
use crate::domain::agents::{AgentHarnessKind, AgentLane, AgentLaneSettings, LogicalEffort};

fn codex_settings(model: &str) -> AgentLaneSettings {
    AgentLaneSettings {
        harness: AgentHarnessKind::Codex,
        model: Some(model.to_string()),
        effort: Some(LogicalEffort::XHigh),
        approval_policy: Some("on-request".to_string()),
        sandbox_mode: Some("workspace-write".to_string()),
        fallback_harness: Some(AgentHarnessKind::Claude),
    }
}

#[tokio::test]
async fn test_upsert_and_get_global_lane_settings() {
    let repo = MemoryAgentLaneSettingsRepository::new();

    repo.upsert_global(AgentLane::IdeationPrimary, &codex_settings("gpt-5.4"))
        .await
        .unwrap();

    let row = repo
        .get_global(AgentLane::IdeationPrimary)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.project_id, None);
    assert_eq!(row.settings.harness, AgentHarnessKind::Codex);
    assert_eq!(row.settings.model.as_deref(), Some("gpt-5.4"));
}

#[tokio::test]
async fn test_upsert_and_get_project_lane_settings() {
    let repo = MemoryAgentLaneSettingsRepository::new();

    repo.upsert_for_project(
        "project-1",
        AgentLane::IdeationVerifier,
        &codex_settings("gpt-5.4-mini"),
    )
    .await
    .unwrap();

    let row = repo
        .get_for_project("project-1", AgentLane::IdeationVerifier)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.project_id.as_deref(), Some("project-1"));
    assert_eq!(row.settings.model.as_deref(), Some("gpt-5.4-mini"));
}

#[tokio::test]
async fn test_upsert_reuses_row_id() {
    let repo = MemoryAgentLaneSettingsRepository::new();

    let first = repo
        .upsert_global(AgentLane::IdeationPrimary, &codex_settings("gpt-5.4"))
        .await
        .unwrap();
    let second = repo
        .upsert_global(AgentLane::IdeationPrimary, &codex_settings("gpt-5.4-mini"))
        .await
        .unwrap();

    assert_eq!(first.id, second.id);
    assert_eq!(second.settings.model.as_deref(), Some("gpt-5.4-mini"));
}
