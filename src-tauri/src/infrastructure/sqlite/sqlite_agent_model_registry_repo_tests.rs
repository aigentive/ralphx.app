use crate::domain::agents::{
    AgentHarnessKind, AgentModelDefinition, AgentModelSource, LogicalEffort,
};
use crate::domain::repositories::AgentModelRegistryRepository;
use crate::infrastructure::sqlite::SqliteAgentModelRegistryRepository;
use crate::testing::SqliteTestDb;

fn setup_repo() -> (SqliteTestDb, SqliteAgentModelRegistryRepository) {
    let db = SqliteTestDb::new("sqlite_agent_model_registry_repo_tests");
    let repo = SqliteAgentModelRegistryRepository::from_shared(db.shared_conn());
    (db, repo)
}

#[tokio::test]
async fn upsert_and_list_custom_model() {
    let (_db, repo) = setup_repo();
    let model = AgentModelDefinition::custom(
        AgentHarnessKind::Codex,
        "gpt-5.6",
        "GPT-5.6",
        "GPT-5.6",
        Some("Future Codex model".to_string()),
        vec![
            LogicalEffort::Low,
            LogicalEffort::Medium,
            LogicalEffort::High,
            LogicalEffort::XHigh,
        ],
        LogicalEffort::XHigh,
        true,
    );

    let saved = repo.upsert_custom_model(&model).await.unwrap();
    let rows = repo.list_custom_models().await.unwrap();

    assert_eq!(saved.source, AgentModelSource::Custom);
    assert_eq!(saved.model_id, "gpt-5.6");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].default_effort, LogicalEffort::XHigh);
    assert_eq!(rows[0].supported_efforts.len(), 4);
    assert!(rows[0].created_at.is_some());
    assert!(rows[0].updated_at.is_some());
}

#[tokio::test]
async fn upsert_normalizes_default_effort_to_supported_efforts() {
    let (_db, repo) = setup_repo();
    let model = AgentModelDefinition::custom(
        AgentHarnessKind::Codex,
        "gpt-5.6-mini",
        "",
        "",
        None,
        vec![LogicalEffort::Low, LogicalEffort::Medium],
        LogicalEffort::XHigh,
        true,
    );

    let saved = repo.upsert_custom_model(&model).await.unwrap();

    assert_eq!(saved.label, "gpt-5.6-mini");
    assert_eq!(saved.menu_label, "gpt-5.6-mini");
    assert_eq!(saved.default_effort, LogicalEffort::Low);
}

#[tokio::test]
async fn delete_custom_model_removes_row() {
    let (_db, repo) = setup_repo();
    let model = AgentModelDefinition::custom(
        AgentHarnessKind::Claude,
        "claude-opus-4-7",
        "Claude Opus 4.7",
        "Claude Opus 4.7",
        None,
        vec![
            LogicalEffort::High,
            LogicalEffort::XHigh,
            LogicalEffort::Max,
        ],
        LogicalEffort::XHigh,
        true,
    );
    repo.upsert_custom_model(&model).await.unwrap();

    assert!(repo
        .delete_custom_model(AgentHarnessKind::Claude, "claude-opus-4-7")
        .await
        .unwrap());
    assert!(repo.list_custom_models().await.unwrap().is_empty());
}
