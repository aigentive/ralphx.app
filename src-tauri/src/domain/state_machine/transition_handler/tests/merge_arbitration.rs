// Merge arbitration tests
//
// Extracted from side_effects.rs — tests for merge arbitration logic
// (task ID comparison, timestamp queries, and branch missing metadata).

use crate::domain::entities::task_metadata::{
    MergeRecoveryEvent, MergeRecoveryEventKind, MergeRecoveryMetadata, MergeRecoveryReasonCode,
    MergeRecoverySource,
};
use crate::domain::entities::{InternalStatus, ProjectId, Task, TaskId};
use crate::infrastructure::memory::MemoryTaskRepository;

// ==================
// merge arbitration tests
// ==================

#[test]
fn test_merge_arbitration_task_id_lexical_comparison() {
    let task_alpha = TaskId::from_string("task-alpha".to_string());
    let task_beta = TaskId::from_string("task-beta".to_string());
    let task_x = TaskId::from_string("task-x".to_string());
    let task_y = TaskId::from_string("task-y".to_string());

    // Verify lexical ordering works as expected
    assert!(
        task_alpha.as_str() < task_beta.as_str(),
        "task-alpha < task-beta"
    );
    assert!(task_x.as_str() < task_y.as_str(), "task-x < task-y");
    assert!(task_alpha.as_str() < task_x.as_str(), "task-alpha < task-x");
}

/// Test: get_status_entered_at integration with arbitration logic
///
/// This verifies that we can query pending_merge entry times correctly,
/// which is what the arbitration logic depends on.
#[tokio::test]
async fn test_merge_arbitration_get_pending_merge_timestamp() {
    use crate::domain::repositories::TaskRepository;

    let repo = MemoryTaskRepository::new();
    let project_id = ProjectId::new();
    let task = Task::new(project_id, "Test task".to_string());
    repo.create(task.clone()).await.unwrap();

    // Transition to pending_merge
    repo.persist_status_change(
        &task.id,
        InternalStatus::Executing,
        InternalStatus::PendingMerge,
        "agent",
    )
    .await
    .unwrap();

    // Should be able to retrieve the timestamp
    let timestamp = repo
        .get_status_entered_at(&task.id, InternalStatus::PendingMerge)
        .await
        .unwrap();

    assert!(timestamp.is_some(), "Should have pending_merge timestamp");
}

/// Test: Task without state history returns None for get_status_entered_at
///
/// Documents the edge case where a task is in pending_merge but has no history.
#[tokio::test]
async fn test_merge_arbitration_missing_timestamp_edge_case() {
    use crate::domain::repositories::TaskRepository;

    let repo = MemoryTaskRepository::new();
    let project_id = ProjectId::new();

    // Task in pending_merge but no state history recorded
    let mut task = Task::new(project_id, "Edge case task".to_string());
    task.internal_status = InternalStatus::PendingMerge;
    repo.create(task.clone()).await.unwrap();

    // Should return None since no transition was recorded
    let timestamp = repo
        .get_status_entered_at(&task.id, InternalStatus::PendingMerge)
        .await
        .unwrap();

    assert!(
        timestamp.is_none(),
        "Should return None for missing history"
    );
}

/// Test: Timestamp comparison works correctly with chrono::DateTime
///
/// Documents the comparison behavior for the arbitration logic.
#[test]
fn test_merge_arbitration_timestamp_comparison() {
    use chrono::{Duration, Utc};

    let earlier = Utc::now() - Duration::minutes(30);
    let later = Utc::now() - Duration::minutes(15);

    assert!(
        earlier < later,
        "Earlier timestamp should be less than later"
    );
    assert_eq!(earlier, earlier, "Same timestamps should be equal");
}

// ==================
// branch_missing_metadata tests
// ==================

/// Test: Branch missing sets metadata correctly with AutoRetryTriggered event
///
/// Verifies that when a branch validation fails during programmatic merge,
/// the code records an AutoRetryTriggered event in MergeRecoveryMetadata
/// and sets the branch_missing flag in the task metadata JSON.
#[test]
fn test_branch_missing_metadata_with_auto_retry_event() {
    // Create initial recovery metadata
    let mut recovery = MergeRecoveryMetadata::new();

    // Verify it starts empty
    assert_eq!(recovery.events.len(), 0);

    // Count existing AutoRetryTriggered events (should be 0)
    let attempt_count = recovery
        .events
        .iter()
        .filter(|e| matches!(e.kind, MergeRecoveryEventKind::AutoRetryTriggered))
        .count() as u32
        + 1;

    assert_eq!(attempt_count, 1, "First attempt should be 1");

    // Create AutoRetryTriggered event with BranchNotFound reason
    let event = MergeRecoveryEvent::new(
        MergeRecoveryEventKind::AutoRetryTriggered,
        MergeRecoverySource::Auto,
        MergeRecoveryReasonCode::BranchNotFound,
        "Source branch 'feature/missing' does not exist".to_string(),
    )
    .with_target_branch("main")
    .with_source_branch("feature/missing")
    .with_attempt(attempt_count);

    // Verify event was created correctly
    assert_eq!(event.kind, MergeRecoveryEventKind::AutoRetryTriggered);
    assert_eq!(event.reason_code, MergeRecoveryReasonCode::BranchNotFound);
    assert_eq!(event.source, MergeRecoverySource::Auto);

    // Append event
    recovery.append_event(event);
    assert_eq!(recovery.events.len(), 1);

    // Update task metadata with recovery events
    let updated_json = recovery.update_task_metadata(None).unwrap();

    // Parse and verify the JSON contains merge_recovery
    let metadata: serde_json::Value = serde_json::from_str(&updated_json).unwrap();
    assert!(
        metadata.get("merge_recovery").is_some(),
        "Should contain merge_recovery field"
    );

    // Verify merge_recovery structure
    let merge_recovery = &metadata["merge_recovery"];
    assert_eq!(merge_recovery["version"], 1);
    assert_eq!(merge_recovery["events"].as_array().unwrap().len(), 1);
    assert_eq!(
        merge_recovery["events"][0]["kind"], "auto_retry_triggered",
        "Event kind should be AutoRetryTriggered"
    );
    assert_eq!(
        merge_recovery["events"][0]["reason_code"], "branch_not_found",
        "Event reason_code should be BranchNotFound"
    );

    // Now add branch_missing flag to metadata (simulating what side_effects.rs does)
    let mut metadata_obj = metadata;
    if let Some(obj) = metadata_obj.as_object_mut() {
        obj.insert("branch_missing".to_string(), serde_json::json!(true));
    }
    let final_json = metadata_obj.to_string();

    // Parse again and verify branch_missing flag is set
    let final_metadata: serde_json::Value = serde_json::from_str(&final_json).unwrap();
    assert_eq!(
        final_metadata.get("branch_missing"),
        Some(&serde_json::json!(true)),
        "Should have branch_missing flag set to true"
    );

    // Verify retry count increments on subsequent attempts
    let recovery2 = MergeRecoveryMetadata::from_task_metadata(Some(&final_json))
        .unwrap_or(None)
        .unwrap_or_else(MergeRecoveryMetadata::new);

    let attempt_count2 = recovery2
        .events
        .iter()
        .filter(|e| matches!(e.kind, MergeRecoveryEventKind::AutoRetryTriggered))
        .count() as u32
        + 1;

    assert_eq!(
        attempt_count2, 2,
        "Second attempt should be 2, confirming retry count increments"
    );
}
