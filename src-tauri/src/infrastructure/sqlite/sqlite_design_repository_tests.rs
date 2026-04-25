use std::collections::BTreeMap;

use chrono::Utc;

use crate::domain::entities::{
    ChatConversation, DesignApprovalStatus, DesignConfidence, DesignFeedbackStatus, DesignRun,
    DesignRunKind, DesignRunStatus, DesignSchemaVersion, DesignSchemaVersionId,
    DesignSchemaVersionStatus, DesignSourceKind, DesignSourceRef, DesignSourceRole,
    DesignStorageRootRef, DesignStyleguideFeedback, DesignStyleguideFeedbackId,
    DesignStyleguideGroup, DesignStyleguideItem, DesignStyleguideItemId, DesignSystem,
    DesignSystemId, DesignSystemSource, DesignSystemSourceId,
};
use crate::domain::repositories::{
    ChatConversationRepository, DesignRunRepository, DesignSchemaRepository,
    DesignStyleguideFeedbackRepository, DesignStyleguideRepository, DesignSystemRepository,
    DesignSystemSourceRepository,
};
use crate::infrastructure::sqlite::{
    SqliteChatConversationRepository, SqliteDesignRunRepository, SqliteDesignSchemaRepository,
    SqliteDesignStyleguideFeedbackRepository, SqliteDesignStyleguideRepository,
    SqliteDesignSystemRepository, SqliteDesignSystemSourceRepository,
};
use crate::testing::SqliteTestDb;

struct TestRepos {
    db: SqliteTestDb,
    systems: SqliteDesignSystemRepository,
    sources: SqliteDesignSystemSourceRepository,
    schemas: SqliteDesignSchemaRepository,
    styleguide: SqliteDesignStyleguideRepository,
    feedback: SqliteDesignStyleguideFeedbackRepository,
    runs: SqliteDesignRunRepository,
    conversations: SqliteChatConversationRepository,
}

fn setup() -> TestRepos {
    let db = SqliteTestDb::new("design-repository");
    let shared = db.shared_conn();
    TestRepos {
        systems: SqliteDesignSystemRepository::from_shared(shared.clone()),
        sources: SqliteDesignSystemSourceRepository::from_shared(shared.clone()),
        schemas: SqliteDesignSchemaRepository::from_shared(shared.clone()),
        styleguide: SqliteDesignStyleguideRepository::from_shared(shared.clone()),
        feedback: SqliteDesignStyleguideFeedbackRepository::from_shared(shared.clone()),
        runs: SqliteDesignRunRepository::from_shared(shared.clone()),
        conversations: SqliteChatConversationRepository::from_shared(shared),
        db,
    }
}

async fn seed_system(repos: &TestRepos) -> DesignSystem {
    let project = repos.db.seed_project("Design project");
    let mut system = DesignSystem::new(
        project.id.clone(),
        "Core UI",
        DesignStorageRootRef::from_hash_component("root-hash"),
    );
    system.description = Some("Shared visual language".to_string());
    repos.systems.create(system).await.unwrap()
}

fn make_schema(system_id: DesignSystemId, version: &str) -> DesignSchemaVersion {
    DesignSchemaVersion {
        id: DesignSchemaVersionId::new(),
        design_system_id: system_id,
        version: version.to_string(),
        schema_artifact_id: format!("schema-{version}"),
        manifest_artifact_id: format!("manifest-{version}"),
        styleguide_artifact_id: format!("styleguide-{version}"),
        status: DesignSchemaVersionStatus::Draft,
        created_by_run_id: None,
        created_at: Utc::now(),
    }
}

fn make_item(system_id: DesignSystemId, schema_id: DesignSchemaVersionId) -> DesignStyleguideItem {
    DesignStyleguideItem {
        id: DesignStyleguideItemId::new(),
        design_system_id: system_id.clone(),
        schema_version_id: schema_id,
        item_id: "button.primary".to_string(),
        group: DesignStyleguideGroup::Components,
        label: "Primary button".to_string(),
        summary: "Main action control".to_string(),
        preview_artifact_id: Some("preview-1".to_string()),
        source_refs: vec![DesignSourceRef {
            project_id: crate::domain::entities::ProjectId::from_string(
                "source-project".to_string(),
            ),
            path: "frontend/src/Button.tsx".to_string(),
            line: Some(12),
        }],
        confidence: DesignConfidence::High,
        approval_status: DesignApprovalStatus::NeedsReview,
        feedback_status: DesignFeedbackStatus::Open,
        updated_at: Utc::now(),
    }
}

#[tokio::test]
async fn test_design_system_repository_create_list_archive() {
    let repos = setup();
    let system = seed_system(&repos).await;

    let loaded = repos
        .systems
        .get_by_id(&system.id)
        .await
        .unwrap()
        .expect("system");
    assert_eq!(loaded.name, "Core UI");
    assert_eq!(loaded.storage_root_ref.as_str(), "root-hash");

    let visible = repos
        .systems
        .list_by_project(&system.primary_project_id, false)
        .await
        .unwrap();
    assert_eq!(visible.len(), 1);

    repos.systems.archive(&system.id).await.unwrap();
    assert!(repos
        .systems
        .list_by_project(&system.primary_project_id, false)
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        repos
            .systems
            .list_by_project(&system.primary_project_id, true)
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn test_design_sources_and_schema_versions_round_trip() {
    let repos = setup();
    let system = seed_system(&repos).await;
    let mut source_hashes = BTreeMap::new();
    source_hashes.insert(
        "frontend/src/Button.tsx".to_string(),
        "sha256:abc".to_string(),
    );

    repos
        .sources
        .replace_for_design_system(
            &system.id,
            vec![DesignSystemSource {
                id: DesignSystemSourceId::new(),
                design_system_id: system.id.clone(),
                project_id: system.primary_project_id.clone(),
                role: DesignSourceRole::Primary,
                selected_paths: vec!["frontend/src".to_string()],
                source_kind: DesignSourceKind::ProjectCheckout,
                git_commit: Some("abc123".to_string()),
                source_hashes,
                last_analyzed_at: Some(Utc::now()),
            }],
        )
        .await
        .unwrap();

    let sources = repos
        .sources
        .list_by_design_system(&system.id)
        .await
        .unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].selected_paths, vec!["frontend/src"]);
    assert_eq!(
        sources[0].source_hashes.get("frontend/src/Button.tsx"),
        Some(&"sha256:abc".to_string())
    );

    let schema = repos
        .schemas
        .create_version(make_schema(system.id.clone(), "v1"))
        .await
        .unwrap();
    let mut updated_system = system.clone();
    updated_system.current_schema_version_id = Some(schema.id.clone());
    updated_system.status = crate::domain::entities::DesignSystemStatus::SchemaReady;
    updated_system.updated_at = Utc::now();
    repos.systems.update(&updated_system).await.unwrap();

    let current = repos
        .schemas
        .get_current_for_design_system(&system.id)
        .await
        .unwrap()
        .expect("current schema");
    assert_eq!(current.id, schema.id);
    assert_eq!(
        repos.schemas.list_versions(&system.id).await.unwrap().len(),
        1
    );
}

#[tokio::test]
async fn test_styleguide_feedback_and_runs_round_trip() {
    let repos = setup();
    let system = seed_system(&repos).await;
    let schema = repos
        .schemas
        .create_version(make_schema(system.id.clone(), "v1"))
        .await
        .unwrap();
    let item = make_item(system.id.clone(), schema.id.clone());
    let item_id = item.id.clone();

    repos
        .styleguide
        .replace_items_for_schema_version(&schema.id, vec![item])
        .await
        .unwrap();

    let items = repos
        .styleguide
        .list_items(&system.id, Some(&schema.id))
        .await
        .unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].item_id, "button.primary");

    repos.styleguide.approve_item(&item_id).await.unwrap();
    let approved = repos
        .styleguide
        .get_item(&system.id, "button.primary")
        .await
        .unwrap()
        .expect("item");
    assert_eq!(approved.approval_status, DesignApprovalStatus::Approved);

    let conversation = repos
        .conversations
        .create(ChatConversation::new_design(system.id.clone()))
        .await
        .unwrap();
    let feedback = repos
        .feedback
        .create(DesignStyleguideFeedback {
            id: DesignStyleguideFeedbackId::new(),
            design_system_id: system.id.clone(),
            schema_version_id: schema.id.clone(),
            item_id: "button.primary".to_string(),
            conversation_id: conversation.id,
            message_id: None,
            preview_artifact_id: Some("preview-1".to_string()),
            source_refs: approved.source_refs.clone(),
            feedback: "Increase contrast".to_string(),
            status: DesignFeedbackStatus::Open,
            created_at: Utc::now(),
            resolved_at: None,
        })
        .await
        .unwrap();
    assert_eq!(
        repos
            .feedback
            .list_open_by_design_system(&system.id)
            .await
            .unwrap()
            .len(),
        1
    );
    let loaded_feedback = repos
        .feedback
        .get_by_id(&feedback.id)
        .await
        .unwrap()
        .expect("feedback");
    assert_eq!(loaded_feedback.feedback, "Increase contrast");

    let mut run = DesignRun::queued(system.id.clone(), DesignRunKind::ItemFeedback, "Feedback");
    run.status = DesignRunStatus::Completed;
    run.output_artifact_ids = vec!["artifact-1".to_string()];
    let run = repos.runs.create(run).await.unwrap();
    let loaded_run = repos.runs.get_by_id(&run.id).await.unwrap().expect("run");
    assert_eq!(loaded_run.output_artifact_ids, vec!["artifact-1"]);
    assert_eq!(
        repos
            .runs
            .list_by_design_system(&system.id)
            .await
            .unwrap()
            .len(),
        1
    );
}
