use std::collections::BTreeMap;

use chrono::Utc;

use super::{
    MemoryDesignRunRepository, MemoryDesignSchemaRepository,
    MemoryDesignStyleguideFeedbackRepository, MemoryDesignStyleguideRepository,
    MemoryDesignSystemRepository, MemoryDesignSystemSourceRepository,
};
use crate::domain::entities::{
    ChatConversationId, DesignApprovalStatus, DesignConfidence, DesignFeedbackStatus, DesignRun,
    DesignRunKind, DesignSchemaVersion, DesignSchemaVersionId, DesignSchemaVersionStatus,
    DesignSourceKind, DesignSourceRef, DesignSourceRole, DesignStorageRootRef,
    DesignStyleguideFeedback, DesignStyleguideFeedbackId, DesignStyleguideGroup,
    DesignStyleguideItem, DesignStyleguideItemId, DesignSystem, DesignSystemId, DesignSystemSource,
    DesignSystemSourceId, ProjectId,
};
use crate::domain::repositories::{
    DesignRunRepository, DesignSchemaRepository, DesignStyleguideFeedbackRepository,
    DesignStyleguideRepository, DesignSystemRepository, DesignSystemSourceRepository,
};

#[tokio::test]
async fn design_system_repo_lists_and_archives_by_project() {
    let repo = MemoryDesignSystemRepository::new();
    let project_id = ProjectId::new();
    let other_project_id = ProjectId::new();

    let design_system = DesignSystem::new(
        project_id.clone(),
        "Product UI",
        DesignStorageRootRef::from_hash_component("design-a"),
    );
    let other_system = DesignSystem::new(
        other_project_id.clone(),
        "Other UI",
        DesignStorageRootRef::from_hash_component("design-b"),
    );

    repo.create(design_system.clone()).await.unwrap();
    repo.create(other_system).await.unwrap();

    let project_systems = repo.list_by_project(&project_id, false).await.unwrap();
    assert_eq!(project_systems.len(), 1);
    assert_eq!(project_systems[0].id, design_system.id);

    repo.archive(&design_system.id).await.unwrap();
    assert!(repo
        .list_by_project(&project_id, false)
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        repo.list_by_project(&project_id, true).await.unwrap().len(),
        1
    );
}

#[tokio::test]
async fn design_related_repositories_round_trip_records() {
    let source_repo = MemoryDesignSystemSourceRepository::new();
    let schema_repo = MemoryDesignSchemaRepository::new();
    let styleguide_repo = MemoryDesignStyleguideRepository::new();
    let feedback_repo = MemoryDesignStyleguideFeedbackRepository::new();
    let run_repo = MemoryDesignRunRepository::new();

    let design_system_id = DesignSystemId::new();
    let project_id = ProjectId::new();
    let schema_version_id = DesignSchemaVersionId::new();
    let item_id = DesignStyleguideItemId::new();

    let source = DesignSystemSource {
        id: DesignSystemSourceId::new(),
        design_system_id: design_system_id.clone(),
        project_id: project_id.clone(),
        role: DesignSourceRole::Primary,
        selected_paths: vec!["frontend/src".to_string()],
        source_kind: DesignSourceKind::ProjectCheckout,
        git_commit: Some("abc123".to_string()),
        source_hashes: BTreeMap::new(),
        last_analyzed_at: None,
    };
    source_repo
        .replace_for_design_system(&design_system_id, vec![source.clone()])
        .await
        .unwrap();
    assert_eq!(
        source_repo
            .list_by_design_system(&design_system_id)
            .await
            .unwrap()[0]
            .selected_paths,
        source.selected_paths
    );

    let schema_version = DesignSchemaVersion {
        id: schema_version_id.clone(),
        design_system_id: design_system_id.clone(),
        version: "0.1.0".to_string(),
        schema_artifact_id: "artifact-schema".to_string(),
        manifest_artifact_id: "artifact-manifest".to_string(),
        styleguide_artifact_id: "artifact-styleguide".to_string(),
        status: DesignSchemaVersionStatus::Draft,
        created_by_run_id: None,
        created_at: Utc::now(),
    };
    schema_repo
        .create_version(schema_version.clone())
        .await
        .unwrap();
    assert_eq!(
        schema_repo
            .get_current_for_design_system(&design_system_id)
            .await
            .unwrap()
            .unwrap()
            .id,
        schema_version.id
    );

    let source_ref = DesignSourceRef {
        project_id,
        path: "frontend/src/components/Button.tsx".to_string(),
        line: Some(12),
    };
    let styleguide_item = DesignStyleguideItem {
        id: item_id.clone(),
        design_system_id: design_system_id.clone(),
        schema_version_id: schema_version_id.clone(),
        item_id: "button.primary".to_string(),
        group: DesignStyleguideGroup::Components,
        label: "Primary button".to_string(),
        summary: "Primary action control".to_string(),
        preview_artifact_id: Some("preview-button".to_string()),
        source_refs: vec![source_ref.clone()],
        confidence: DesignConfidence::High,
        approval_status: DesignApprovalStatus::NeedsReview,
        feedback_status: DesignFeedbackStatus::None,
        updated_at: Utc::now(),
    };
    styleguide_repo
        .replace_items_for_schema_version(&schema_version_id, vec![styleguide_item])
        .await
        .unwrap();
    styleguide_repo.approve_item(&item_id).await.unwrap();
    let item = styleguide_repo
        .get_item(&design_system_id, "button.primary")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(item.approval_status, DesignApprovalStatus::Approved);

    let feedback = DesignStyleguideFeedback {
        id: DesignStyleguideFeedbackId::new(),
        design_system_id: design_system_id.clone(),
        schema_version_id,
        item_id: "button.primary".to_string(),
        conversation_id: ChatConversationId::new(),
        message_id: None,
        preview_artifact_id: Some("preview-button".to_string()),
        source_refs: vec![source_ref],
        feedback: "Increase focus contrast".to_string(),
        status: DesignFeedbackStatus::Open,
        created_at: Utc::now(),
        resolved_at: None,
    };
    feedback_repo.create(feedback.clone()).await.unwrap();
    assert_eq!(
        feedback_repo
            .list_open_by_design_system(&design_system_id)
            .await
            .unwrap()[0]
            .id,
        feedback.id
    );

    let run = DesignRun::queued(
        design_system_id.clone(),
        DesignRunKind::Create,
        "Initial styleguide",
    );
    run_repo.create(run.clone()).await.unwrap();
    assert_eq!(
        run_repo
            .list_by_design_system(&design_system_id)
            .await
            .unwrap()[0]
            .id,
        run.id
    );
}
