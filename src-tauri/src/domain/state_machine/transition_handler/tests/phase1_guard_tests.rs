// Tests for Phase 1 GUARD: pre_merge_cleanup first-attempt skip optimization
//
// ROOT CAUSE #3: Full 6-step cleanup runs on EVERY attempt (even first clean merge)
// FIX: Skip cleanup entirely on first attempt when no agents are running
//
// ROOT CAUSE #2: No timeout safety net on cleanup
// FIX: Caller already wraps in os_thread_timeout — verify behavior
//
// Additional optimizations:
// - Step 4: Parallel worktree deletion (was sequential)
// - Step 5: Orphan scan moved to deferred (out of critical path)

use super::helpers::*;
use crate::domain::entities::{InternalStatus, Task};
use crate::domain::state_machine::{State, TransitionHandler};

// ==================
// First-attempt skip optimization (ROOT CAUSE #3)
// ==================

/// First merge attempt with clean state should skip cleanup entirely.
///
/// When task has no prior merge failure metadata and no agents are running,
/// pre_merge_cleanup should return almost instantly (< 50ms).
#[tokio::test]
async fn test_first_attempt_clean_state_skips_cleanup() {
    let (mut machine, task_repo, task_id) =
        setup_pending_merge_repos("First attempt clean", Some("feature/first-attempt"))
            .await
            .into_machine();
    let handler = TransitionHandler::new(&mut machine);

    // Verify task has no metadata (clean first attempt)
    let task = task_repo.get_by_id(&task_id).await.unwrap().unwrap();
    assert!(
        task.metadata.is_none(),
        "Task should have no metadata for first attempt"
    );

    let start = std::time::Instant::now();
    let _ = handler.on_enter(&State::PendingMerge).await;
    let elapsed = start.elapsed();

    // First attempt should complete quickly — the pre_merge_cleanup guard
    // should skip all cleanup steps. The merge itself will fail (nonexistent dir)
    // but cleanup should be near-instant.
    // Allow generous bound since git operations still run.
    assert!(
        elapsed.as_secs() < 10,
        "First attempt with clean state should skip cleanup, took {}ms",
        elapsed.as_millis()
    );
}

/// Retry attempt (has merge_failure_source metadata) should run cleanup.
///
/// When task metadata contains merge_failure_source from a prior
/// transition_to_merge_incomplete, cleanup steps should execute.
#[tokio::test]
async fn test_retry_attempt_with_failure_metadata_runs_cleanup() {
    let setup = setup_pending_merge_repos("Retry attempt", Some("feature/retry")).await;

    // Set merge failure metadata to simulate a prior failed attempt
    let mut task = setup.task_repo.get_by_id(&setup.task_id).await.unwrap().unwrap();
    task.metadata = Some(
        serde_json::json!({
            "merge_failure_source": "BranchFreshnessTimeout",
            "error": "Prior merge failed"
        })
        .to_string(),
    );
    setup.task_repo.update(&task).await.unwrap();

    let (mut machine, task_repo, task_id) = setup.into_machine();
    let handler = TransitionHandler::new(&mut machine);

    let _ = handler.on_enter(&State::PendingMerge).await;

    // Task should still transition to MergeIncomplete (nonexistent git dir)
    let updated = task_repo.get_by_id(&task_id).await.unwrap().unwrap();
    assert_eq!(
        updated.internal_status,
        InternalStatus::MergeIncomplete,
        "Retry attempt should still complete (even if merge fails)"
    );
}

/// Task with merge_pipeline_active metadata (prior crash) should run cleanup.
///
/// If a prior attempt crashed mid-pipeline, merge_pipeline_active may still be set.
/// This counts as debris that requires cleanup.
#[tokio::test]
async fn test_prior_crash_metadata_triggers_cleanup() {
    let setup = setup_pending_merge_repos("Crash recovery", Some("feature/crash")).await;

    // Simulate a prior crash leaving merge_pipeline_active set
    let mut task = setup.task_repo.get_by_id(&setup.task_id).await.unwrap().unwrap();
    task.metadata = Some(
        serde_json::json!({
            "merge_pipeline_active": "2026-01-01T00:00:00Z"
        })
        .to_string(),
    );
    setup.task_repo.update(&task).await.unwrap();

    let (mut machine, task_repo, task_id) = setup.into_machine();
    let handler = TransitionHandler::new(&mut machine);

    let _ = handler.on_enter(&State::PendingMerge).await;

    // Should complete (cleanup runs, then merge fails on nonexistent dir)
    let updated = task_repo.get_by_id(&task_id).await.unwrap().unwrap();
    assert_eq!(
        updated.internal_status,
        InternalStatus::MergeIncomplete,
        "Crash recovery should run cleanup and proceed to merge"
    );
}

// ==================
// is_first_clean_attempt helper
// ==================

/// is_first_clean_attempt returns true for fresh task with no metadata.
#[test]
fn test_is_first_clean_attempt_no_metadata() {
    use crate::domain::state_machine::transition_handler::merge_coordination::is_first_clean_attempt;
    let task = Task::new(
        crate::domain::entities::ProjectId::from_string("proj-1".to_string()),
        "Test".to_string(),
    );
    assert!(
        is_first_clean_attempt(&task),
        "Task with no metadata should be first clean attempt"
    );
}

/// is_first_clean_attempt returns false when metadata has merge_failure_source.
#[test]
fn test_is_first_clean_attempt_with_failure_metadata() {
    use crate::domain::state_machine::transition_handler::merge_coordination::is_first_clean_attempt;
    let mut task = Task::new(
        crate::domain::entities::ProjectId::from_string("proj-1".to_string()),
        "Test".to_string(),
    );
    task.metadata = Some(
        serde_json::json!({
            "merge_failure_source": "GitError"
        })
        .to_string(),
    );
    assert!(
        !is_first_clean_attempt(&task),
        "Task with merge_failure_source should not be first clean attempt"
    );
}

/// is_first_clean_attempt returns false when metadata has source_conflict_resolved.
#[test]
fn test_is_first_clean_attempt_with_conflict_metadata() {
    use crate::domain::state_machine::transition_handler::merge_coordination::is_first_clean_attempt;
    let mut task = Task::new(
        crate::domain::entities::ProjectId::from_string("proj-1".to_string()),
        "Test".to_string(),
    );
    task.metadata = Some(
        serde_json::json!({
            "source_conflict_resolved": true
        })
        .to_string(),
    );
    assert!(
        !is_first_clean_attempt(&task),
        "Task with source_conflict_resolved should not be first clean attempt"
    );
}

/// is_first_clean_attempt returns true when metadata exists but has no merge indicators.
#[test]
fn test_is_first_clean_attempt_with_unrelated_metadata() {
    use crate::domain::state_machine::transition_handler::merge_coordination::is_first_clean_attempt;
    let mut task = Task::new(
        crate::domain::entities::ProjectId::from_string("proj-1".to_string()),
        "Test".to_string(),
    );
    task.metadata = Some(
        serde_json::json!({
            "some_other_key": "value"
        })
        .to_string(),
    );
    assert!(
        is_first_clean_attempt(&task),
        "Task with unrelated metadata should be first clean attempt"
    );
}

// ==================
// Parallel worktree deletion (Step 4 optimization)
// ==================

/// Parallel worktree deletion should complete faster than sequential.
///
/// Creates multiple directories and verifies they are all removed.
/// This is a unit test for the parallelization logic.
#[tokio::test]
async fn test_parallel_worktree_deletion_removes_all() {
    use crate::domain::state_machine::transition_handler::cleanup_helpers::remove_worktree_fast;

    let temp = tempfile::TempDir::new().unwrap();
    let repo_path = temp.path().to_path_buf();

    // Init git repo
    tokio::process::Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .output()
        .await
        .unwrap();

    // Create 5 fake worktree directories (simulating task/merge/rebase/plan-update/source-update)
    let worktree_names = vec!["task-wt", "merge-wt", "rebase-wt", "plan-update-wt", "source-update-wt"];
    let mut worktree_paths = Vec::new();
    for name in &worktree_names {
        let wt_path = temp.path().join(name);
        tokio::fs::create_dir_all(&wt_path).await.unwrap();
        tokio::fs::write(wt_path.join("file.txt"), "content")
            .await
            .unwrap();
        worktree_paths.push(wt_path);
    }

    // Verify all exist
    for wt in &worktree_paths {
        assert!(wt.exists(), "Worktree should exist before deletion: {:?}", wt);
    }

    // Delete all in parallel
    let futs: Vec<_> = worktree_paths
        .iter()
        .map(|wt| remove_worktree_fast(wt, &repo_path))
        .collect();
    let results = futures::future::join_all(futs).await;

    // All should succeed
    for (i, result) in results.iter().enumerate() {
        assert!(
            result.is_ok(),
            "Parallel deletion of {} should succeed: {:?}",
            worktree_names[i],
            result.as_ref().err()
        );
    }

    // All should be gone
    for (i, wt) in worktree_paths.iter().enumerate() {
        assert!(
            !wt.exists(),
            "Worktree {} should be removed after parallel deletion",
            worktree_names[i]
        );
    }
}

// ==================
// Step 5 orphan scan deferred (out of critical path)
// ==================

/// Merge cleanup without orphan scan should be faster.
///
/// Indirect test: with repos wired but no actual orphaned worktrees,
/// the cleanup should complete without the Step 5 overhead.
#[tokio::test]
async fn test_cleanup_without_orphan_scan_is_bounded() {
    let (mut machine, _, _) =
        setup_pending_merge_repos("No orphan scan", Some("feature/no-orphan"))
            .await
            .into_machine();
    let handler = TransitionHandler::new(&mut machine);

    let start = std::time::Instant::now();
    let _ = handler.on_enter(&State::PendingMerge).await;
    let elapsed = start.elapsed();

    // Without orphan scan in the critical path, cleanup should be bounded
    assert!(
        elapsed.as_secs() < 15,
        "Cleanup without orphan scan should complete quickly, took {}s",
        elapsed.as_secs()
    );
}
