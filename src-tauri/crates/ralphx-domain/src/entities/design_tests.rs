use super::*;

#[test]
fn design_system_new_starts_as_draft_with_opaque_storage_ref() {
    let project_id = ProjectId::from_string("project-1".to_string());
    let storage_ref = DesignStorageRootRef::from_hash_component("hash-abc123");

    let system = DesignSystem::new(project_id.clone(), "RalphX Design", storage_ref.clone());

    assert_eq!(system.primary_project_id, project_id);
    assert_eq!(system.name, "RalphX Design");
    assert_eq!(system.status, DesignSystemStatus::Draft);
    assert_eq!(system.storage_root_ref, storage_ref);
    assert!(system.current_schema_version_id.is_none());
    assert!(system.archived_at.is_none());
}

#[test]
fn design_statuses_serialize_to_snake_case_contract_values() {
    assert_eq!(
        serde_json::to_string(&DesignSystemStatus::SchemaReady).unwrap(),
        "\"schema_ready\""
    );
    assert_eq!(
        serde_json::to_string(&DesignSourceKind::ProjectCheckout).unwrap(),
        "\"project_checkout\""
    );
    assert_eq!(
        serde_json::to_string(&DesignRunKind::GenerateComponent).unwrap(),
        "\"generate_component\""
    );
    assert_eq!(
        serde_json::to_string(&DesignFeedbackStatus::InProgress).unwrap(),
        "\"in_progress\""
    );
}

#[test]
fn source_refs_store_relative_metadata_not_storage_roots() {
    let source = DesignSourceRef {
        project_id: ProjectId::from_string("project-1".to_string()),
        path: "frontend/src/components/ui/button.tsx".to_string(),
        line: Some(24),
    };

    assert_eq!(source.path, "frontend/src/components/ui/button.tsx");
    assert_eq!(source.line, Some(24));
}

#[test]
fn queued_design_run_has_no_started_or_completed_timestamps() {
    let system_id = DesignSystemId::from_string("design-system-1");

    let run = DesignRun::queued(system_id.clone(), DesignRunKind::Create, "Initial analysis");

    assert_eq!(run.design_system_id, system_id);
    assert_eq!(run.kind, DesignRunKind::Create);
    assert_eq!(run.status, DesignRunStatus::Queued);
    assert_eq!(run.input_summary, "Initial analysis");
    assert!(run.output_artifact_ids.is_empty());
    assert!(run.started_at.is_none());
    assert!(run.completed_at.is_none());
}

#[test]
fn styleguide_item_carries_item_level_review_contract() {
    let item = DesignStyleguideItem {
        id: DesignStyleguideItemId::from_string("row-1"),
        design_system_id: DesignSystemId::from_string("design-system-1"),
        schema_version_id: DesignSchemaVersionId::from_string("schema-1"),
        item_id: "components.buttons".to_string(),
        group: DesignStyleguideGroup::Components,
        label: "Buttons".to_string(),
        summary: "Primary and secondary button patterns.".to_string(),
        preview_artifact_id: Some("design-preview-buttons".to_string()),
        source_refs: vec![DesignSourceRef {
            project_id: ProjectId::from_string("project-1".to_string()),
            path: "frontend/src/components/ui/button.tsx".to_string(),
            line: None,
        }],
        confidence: DesignConfidence::Medium,
        approval_status: DesignApprovalStatus::NeedsReview,
        feedback_status: DesignFeedbackStatus::None,
        updated_at: Utc::now(),
    };

    assert_eq!(item.item_id, "components.buttons");
    assert_eq!(item.group, DesignStyleguideGroup::Components);
    assert_eq!(item.approval_status, DesignApprovalStatus::NeedsReview);
    assert_eq!(item.feedback_status, DesignFeedbackStatus::None);
    assert_eq!(item.source_refs.len(), 1);
}
