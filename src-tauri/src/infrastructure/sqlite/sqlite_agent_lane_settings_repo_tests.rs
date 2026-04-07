use super::*;
use crate::domain::agents::{AgentHarnessKind, AgentLane, AgentLaneSettings, LogicalEffort};
use crate::testing::SqliteTestDb;

fn setup_repo() -> (SqliteTestDb, SqliteAgentLaneSettingsRepository) {
    let db = SqliteTestDb::new("sqlite-agent-lane-settings-repo");
    let repo = SqliteAgentLaneSettingsRepository::from_shared(db.shared_conn());
    (db, repo)
}

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
    let (_db, repo) = setup_repo();

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
    assert_eq!(row.settings.fallback_harness, Some(AgentHarnessKind::Claude));
}

#[tokio::test]
async fn test_upsert_and_get_project_lane_settings() {
    let (_db, repo) = setup_repo();

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
async fn test_upsert_reuses_existing_row_id() {
    let (_db, repo) = setup_repo();

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

#[tokio::test]
async fn test_list_scoped_lane_settings() {
    let (_db, repo) = setup_repo();

    repo.upsert_global(AgentLane::IdeationPrimary, &codex_settings("gpt-5.4"))
        .await
        .unwrap();
    repo.upsert_for_project(
        "project-1",
        AgentLane::IdeationVerifier,
        &codex_settings("gpt-5.4-mini"),
    )
    .await
    .unwrap();

    assert_eq!(repo.list_global().await.unwrap().len(), 1);
    assert_eq!(repo.list_for_project("project-1").await.unwrap().len(), 1);
}
