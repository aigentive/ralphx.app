use super::*;

use crate::domain::entities::task_metadata::StopRetryingReason;
use crate::domain::entities::ProjectId;
use crate::infrastructure::memory::MemoryTaskRepository;

// ── startup_resume_count backward-compat tests ─────────────────────────────

#[test]
fn startup_resume_count_absent_in_top_level_json_returns_none() {
    // {} has no execution_recovery key — from_json returns Ok(None)
    let result = ExecutionRecoveryMetadata::from_json("{}").unwrap();
    assert!(result.is_none());
}

#[test]
fn startup_resume_count_absent_in_execution_recovery_deserializes_to_zero() {
    // Old JSON with execution_recovery but no startup_resume_count or last_startup_resume_at
    let json = r#"{"execution_recovery": {"version": 1, "events": [], "last_state": "retrying", "stop_retrying": false}}"#;
    let meta = ExecutionRecoveryMetadata::from_json(json).unwrap().unwrap();
    assert_eq!(meta.startup_resume_count, 0);
    assert!(meta.last_startup_resume_at.is_none());
}

#[test]
fn startup_resume_count_absent_with_extra_fields_deserializes_to_zero() {
    // Old JSON that also has auto_recovery_count but not startup_resume fields
    let json = r#"{"execution_recovery": {"version": 1, "events": [], "last_state": "retrying", "stop_retrying": false, "auto_recovery_count": 2}}"#;
    let meta = ExecutionRecoveryMetadata::from_json(json).unwrap().unwrap();
    assert_eq!(meta.startup_resume_count, 0);
    assert!(meta.last_startup_resume_at.is_none());
    assert_eq!(meta.auto_recovery_count, 2);
}

// ── record_execution_active_startup_resume round-trip test ─────────────────

#[tokio::test]
async fn startup_resume_helper_increments_count_and_sets_timestamp() {
    let repo = MemoryTaskRepository::new();
    let task = Task::new(ProjectId::new(), "test task".to_string());
    repo.create(task.clone()).await.unwrap();

    // First call: count goes from 0 → 1
    let count = record_execution_active_startup_resume(&task, &repo).await.unwrap();
    assert_eq!(count, 1);

    // Read back from repo and verify both fields
    let updated = repo.get_by_id(&task.id).await.unwrap().unwrap();
    let meta = ExecutionRecoveryMetadata::from_task_metadata(updated.metadata.as_deref())
        .unwrap()
        .unwrap();
    assert_eq!(meta.startup_resume_count, 1);
    assert!(meta.last_startup_resume_at.is_some());
}

#[tokio::test]
async fn startup_resume_helper_increments_count_on_repeated_calls() {
    let repo = MemoryTaskRepository::new();
    let task = Task::new(ProjectId::new(), "test task".to_string());
    repo.create(task.clone()).await.unwrap();

    let count1 = record_execution_active_startup_resume(&task, &repo).await.unwrap();
    assert_eq!(count1, 1);

    // Second call must read from repo's updated metadata to get count=2
    let updated_after_first = repo.get_by_id(&task.id).await.unwrap().unwrap();
    let count2 = record_execution_active_startup_resume(&updated_after_first, &repo).await.unwrap();
    assert_eq!(count2, 2);

    let final_task = repo.get_by_id(&task.id).await.unwrap().unwrap();
    let meta = ExecutionRecoveryMetadata::from_task_metadata(final_task.metadata.as_deref())
        .unwrap()
        .unwrap();
    assert_eq!(meta.startup_resume_count, 2);
    assert!(meta.last_startup_resume_at.is_some());
}

// ── existing stop_retrying_reason_to_code tests ───────────────────────────

#[test]
fn stop_retrying_reason_to_code_maps_git_branch_lost() {
    assert_eq!(
        stop_retrying_reason_to_code(&StopRetryingReason::GitBranchLost),
        ExecutionRecoveryReasonCode::GitBranchLost,
    );
}

#[test]
fn stop_retrying_reason_to_code_maps_structural_git_error() {
    assert_eq!(
        stop_retrying_reason_to_code(&StopRetryingReason::StructuralGitError),
        ExecutionRecoveryReasonCode::StructuralGitError,
    );
}

#[test]
fn stop_retrying_reason_to_code_maps_git_isolation_exhausted() {
    assert_eq!(
        stop_retrying_reason_to_code(&StopRetryingReason::GitIsolationExhausted),
        ExecutionRecoveryReasonCode::GitIsolationExhausted,
    );
}

#[test]
fn stop_retrying_reason_to_code_maps_other_variants_to_unknown() {
    assert_eq!(
        stop_retrying_reason_to_code(&StopRetryingReason::MaxRetriesExceeded),
        ExecutionRecoveryReasonCode::Unknown,
    );
    assert_eq!(
        stop_retrying_reason_to_code(&StopRetryingReason::ManualStop),
        ExecutionRecoveryReasonCode::Unknown,
    );
    assert_eq!(
        stop_retrying_reason_to_code(&StopRetryingReason::Unknown),
        ExecutionRecoveryReasonCode::Unknown,
    );
}
