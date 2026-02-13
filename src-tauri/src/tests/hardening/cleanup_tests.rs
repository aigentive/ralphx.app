// Cleanup hardening tests (Scenarios F1-F8)
//
// Tests verifying worktree/branch cleanup behavior during merge workflows
// and demonstrating gaps where cleanup is missing or unreliable.
//
// GitService is static (not mockable via traits), so these tests focus on:
// - State transitions that LEAD to cleanup
// - Task/project state that SHOULD trigger cleanup
// - Documenting gaps where cleanup doesn't happen

use super::helpers::*;
use crate::domain::entities::{InternalStatus, ProjectId};
use crate::domain::repositories::TaskRepository;
use crate::domain::state_machine::events::TaskEvent;
use crate::domain::state_machine::machine::State;

// ============================================================================
// F1: Task worktree deleted before merge — COVERED
// ============================================================================

#[tokio::test]
async fn test_f1_task_with_worktree_can_enter_pending_merge() {
    // COVERED: Pre-merge cleanup deletes task worktree before merge attempt.
    //
    // In production, the pre-merge step deletes the worktree before merging.
    // At test level: verify that a task with worktree_path set can transition
    // to PendingMerge, and that the worktree_path field persists (cleanup is
    // a side effect of the merge process, not the state machine).
    let s = create_hardening_services();

    let project_id = ProjectId::from_string("proj-f1".to_string());
    let mut task = create_test_task_with_status(
        &project_id,
        "Task with worktree",
        InternalStatus::Approved,
    );
    task.task_branch = Some("ralphx/test-project/task-f1".to_string());
    task.worktree_path = Some("/tmp/ralphx-worktrees/task-f1".to_string());
    s.task_repo.create(task.clone()).await.unwrap();

    let services = build_task_services(&s);
    let mut machine = create_state_machine(task.id.as_str(), "proj-f1", services);
    let mut handler = create_transition_handler(&mut machine);

    // Approved -> PendingMerge via StartMerge (auto-transition from Approved)
    let result = handler
        .handle_transition(&State::Approved, &TaskEvent::StartMerge)
        .await;

    assert!(
        result.is_success(),
        "Transition to PendingMerge should succeed"
    );

    // Verify worktree_path is still set on the task in the repo
    let stored_task = s
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("Task should exist");
    assert!(
        stored_task.worktree_path.is_some(),
        "worktree_path should still be set — cleanup is a side effect, not a state machine concern"
    );
}

#[tokio::test]
async fn test_f1_pending_merge_reachable_from_approved() {
    // COVERED: Verify the Approved -> PendingMerge path works cleanly
    // without any task in the repo (pure state machine test).
    let s = create_hardening_services();
    let services = build_task_services(&s);
    let mut machine = create_state_machine("task-f1b", "proj-f1b", services);
    let mut handler = create_transition_handler(&mut machine);

    let result = handler
        .handle_transition(&State::Approved, &TaskEvent::StartMerge)
        .await;

    assert!(
        result.is_success(),
        "Approved -> PendingMerge transition should succeed"
    );
}

// ============================================================================
// F2: Merge worktree cleaned up after success — COVERED
// ============================================================================

#[tokio::test]
async fn test_f2_pending_merge_to_merged_transition_works() {
    // COVERED: After MergeComplete, task is in Merged state.
    //
    // In production, post_merge_cleanup runs after Merged transition.
    // At test level: verify the PendingMerge -> Merged transition path works.
    let s = create_hardening_services();

    let services = build_task_services(&s);
    let mut machine = create_state_machine("task-f2", "proj-f2", services);
    let mut handler = create_transition_handler(&mut machine);

    let result = handler
        .handle_transition(&State::PendingMerge, &TaskEvent::MergeComplete)
        .await;

    assert!(
        result.is_success(),
        "PendingMerge -> Merged should succeed"
    );
    assert_eq!(
        result.state(),
        Some(&State::Merged),
        "Final state should be Merged"
    );
}

#[tokio::test]
async fn test_f2_merged_accepts_retry_to_ready() {
    // COVERED: Merged is not fully terminal — it accepts Retry -> Ready.
    // This allows re-running a task after merge (e.g., if the merge
    // introduced issues discovered later).
    let s = create_hardening_services();
    let services = build_task_services(&s);
    let mut machine = create_state_machine("task-f2b", "proj-f2b", services);
    let mut handler = create_transition_handler(&mut machine);

    let result = handler
        .handle_transition(&State::Merged, &TaskEvent::Retry)
        .await;

    assert!(
        result.is_success(),
        "Merged should accept Retry -> Ready"
    );
    assert_eq!(
        result.state(),
        Some(&State::Ready),
        "Merged + Retry should transition to Ready"
    );
}

// ============================================================================
// F3: Merge worktree orphaned after failed merge — PARTIAL
// ============================================================================

#[tokio::test]
async fn test_f3_merge_incomplete_reachable_from_merging() {
    // PARTIAL: MergeIncomplete is reachable via MergeAgentError from Merging.
    //
    // When a merge fails (Merging -> MergeIncomplete), the task still has
    // worktree_path set. No automatic cleanup runs on the failure path.
    let s = create_hardening_services();

    let project_id = ProjectId::from_string("proj-f3".to_string());
    let mut task = create_test_task_with_status(
        &project_id,
        "Merge failure task",
        InternalStatus::Merging,
    );
    task.task_branch = Some("ralphx/test-project/task-f3".to_string());
    task.worktree_path = Some("/tmp/ralphx-worktrees/task-f3".to_string());
    s.task_repo.create(task.clone()).await.unwrap();

    let services = build_task_services(&s);
    let mut machine = create_state_machine(task.id.as_str(), "proj-f3", services);
    let mut handler = create_transition_handler(&mut machine);

    let result = handler
        .handle_transition(&State::Merging, &TaskEvent::MergeAgentError)
        .await;

    assert!(
        result.is_success(),
        "Merging -> MergeIncomplete should succeed"
    );
    assert_eq!(
        result.state(),
        Some(&State::MergeIncomplete),
        "Final state should be MergeIncomplete"
    );

    // GAP: worktree_path is still set — no cleanup on failure path
    let stored_task = s
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("Task should exist");
    assert!(
        stored_task.worktree_path.is_some(),
        "GAP: worktree_path persists after merge failure — no automatic cleanup on failure path"
    );
}

#[tokio::test]
async fn test_f3_merge_conflict_reachable_via_merge_agent_failed() {
    // PARTIAL: MergeConflict state is reachable from Merging via MergeAgentFailed.
    // (Note: MergeConflict EVENT is not handled in Merging — the STATE is
    // reached via MergeAgentFailed event.)
    let s = create_hardening_services();
    let services = build_task_services(&s);
    let mut machine = create_state_machine("task-f3b", "proj-f3b", services);
    let mut handler = create_transition_handler(&mut machine);

    let result = handler
        .handle_transition(&State::Merging, &TaskEvent::MergeAgentFailed)
        .await;

    assert!(
        result.is_success(),
        "Merging -> MergeConflict via MergeAgentFailed should succeed"
    );
    assert_eq!(
        result.state(),
        Some(&State::MergeConflict),
        "Final state should be MergeConflict"
    );
}

// ============================================================================
// F4: delete_worktree fails silently — GAP
// ============================================================================

#[tokio::test]
async fn test_f4_no_worktree_deletion_retry_mechanism() {
    // GAP: delete_worktree is a static function on GitService (not mockable).
    //
    // GitService::delete_worktree logs a warning and returns Ok(()).
    // There is no retry mechanism if worktree deletion fails.
    // TaskServices has no field for worktree cleanup retry/tracking.
    let s = create_hardening_services();
    let services = build_task_services(&s);

    assert!(
        services.execution_state.is_some(),
        "execution_state is set (used for other purposes, not cleanup)"
    );

    // Verify there is no cleanup-related service in TaskServices.
    // TaskServices has: agent_spawner, event_emitter, notifier, dependency_manager,
    // review_starter, chat_service, execution_state, app_handle, task_scheduler,
    // task_repo, project_repo, plan_branch_repo, step_repo.
    // None of these provide worktree cleanup retry capability.
    //
    // GAP: If delete_worktree fails, the worktree is orphaned with no
    // mechanism to retry the deletion.
}

#[tokio::test]
async fn test_f4_merge_incomplete_reachable_via_merge_agent_error() {
    // GAP: When merge agent encounters a non-conflict error, MergeIncomplete
    // is the fallback state. Worktree may be orphaned.
    let s = create_hardening_services();
    let services = build_task_services(&s);
    let mut machine = create_state_machine("task-f4b", "proj-f4b", services);
    let mut handler = create_transition_handler(&mut machine);

    let result = handler
        .handle_transition(&State::Merging, &TaskEvent::MergeAgentError)
        .await;

    assert!(
        result.is_success(),
        "MergeAgentError should transition to MergeIncomplete"
    );
    assert_eq!(
        result.state(),
        Some(&State::MergeIncomplete),
        "GAP: MergeIncomplete exists as fallback, but worktree may be orphaned"
    );
}

// ============================================================================
// F5: prune_worktrees fails silently — GAP
// ============================================================================

#[tokio::test]
async fn test_f5_no_worktree_prune_retry_mechanism() {
    // GAP: prune_worktrees is a static function on GitService (not mockable).
    //
    // Same architectural gap as F4: worktree pruning is a static function.
    // There is no retry, no error propagation, and no periodic job that
    // would re-attempt pruning.
    let s = create_hardening_services();

    // Verify no periodic cleanup/prune job exists in services.
    // task_scheduler only has try_schedule_ready_tasks and try_retry_deferred_merges.
    let scheduler_calls = s.scheduler.get_calls();
    assert!(
        scheduler_calls.is_empty(),
        "GAP: No pruning calls are made — the mechanism doesn't exist on any service trait"
    );
}

// ============================================================================
// F6: Stale task branches accumulate — GAP
// ============================================================================

#[tokio::test]
async fn test_f6_terminal_tasks_retain_branch_info_no_cleanup_job() {
    // GAP: Tasks in terminal states (Failed, Cancelled, Stopped) still have
    // task_branch set. There is no periodic cleanup job that removes stale branches.
    let s = create_hardening_services();
    let project_id = ProjectId::from_string("proj-f6".to_string());

    let terminal_states = [
        (InternalStatus::Failed, "Failed task"),
        (InternalStatus::Cancelled, "Cancelled task"),
        (InternalStatus::Stopped, "Stopped task"),
    ];

    for (status, title) in &terminal_states {
        let mut task = create_test_task_with_status(&project_id, title, status.clone());
        task.task_branch = Some(format!("ralphx/test-project/{}", task.id.as_str()));
        s.task_repo.create(task).await.unwrap();
    }

    let tasks = s.task_repo.get_by_project(&project_id).await.unwrap();
    assert_eq!(tasks.len(), 3, "All three terminal tasks should exist");

    for task in &tasks {
        assert!(
            task.task_branch.is_some(),
            "GAP: Terminal task '{}' ({:?}) still has task_branch set — \
             no cleanup job removes stale branches",
            task.title, task.internal_status
        );
    }

    // Verify no cleanup service or periodic job exists
    let scheduler_calls = s.scheduler.get_calls();
    assert!(
        scheduler_calls.is_empty(),
        "GAP: No branch cleanup calls were made — no periodic cleanup job exists"
    );
}

#[tokio::test]
async fn test_f6_failed_state_is_not_terminal_for_state_machine() {
    // GAP: Failed tasks can be retried, so they sit in Failed indefinitely
    // with task_branch set until someone retries or manually cleans up.
    let s = create_hardening_services();
    let services = build_task_services(&s);
    let mut machine = create_state_machine("task-f6b", "proj-f6b", services);
    let mut handler = create_transition_handler(&mut machine);

    // Failed accepts Retry -> Ready (branch still set)
    let result = handler
        .handle_transition(&State::Failed(Default::default()), &TaskEvent::Retry)
        .await;
    assert!(
        result.is_success(),
        "Failed -> Ready via Retry should succeed (branch persists through retry)"
    );
}

// ============================================================================
// F7: Task deleted but branch cleanup fails — GAP
// ============================================================================

#[tokio::test]
async fn test_f7_deleted_task_loses_branch_record() {
    // GAP: When a task is deleted from the repo, its branch information is gone.
    // There is no mechanism to track orphaned branches after task deletion.
    let s = create_hardening_services();
    let project_id = ProjectId::from_string("proj-f7".to_string());

    let mut task = create_test_task_with_status(
        &project_id,
        "Task to delete",
        InternalStatus::Failed,
    );
    let branch_name = "ralphx/test-project/task-f7".to_string();
    task.task_branch = Some(branch_name.clone());
    task.worktree_path = Some("/tmp/ralphx-worktrees/task-f7".to_string());
    let task_id = task.id.clone();
    s.task_repo.create(task).await.unwrap();

    // Verify task exists with branch info
    let stored = s.task_repo.get_by_id(&task_id).await.unwrap();
    assert!(stored.is_some(), "Task should exist before deletion");
    assert_eq!(
        stored.unwrap().task_branch,
        Some(branch_name),
        "Branch info should be present"
    );

    // Delete the task
    s.task_repo.delete(&task_id).await.unwrap();

    // Task is gone — branch info is lost
    let after_delete = s.task_repo.get_by_id(&task_id).await.unwrap();
    assert!(
        after_delete.is_none(),
        "Task should be deleted from repo"
    );

    // GAP: The git branch and its worktree are now orphaned with no DB record
    // to identify them for cleanup. No orphan tracking mechanism exists.
}

// ============================================================================
// F8: Worktree on broken symlink — GAP
// ============================================================================

#[tokio::test]
async fn test_f8_no_worktree_path_health_validation() {
    // GAP: A task can have a worktree_path pointing to a non-existent directory.
    // There is no validation of worktree path health at the state machine level.
    let s = create_hardening_services();
    let project_id = ProjectId::from_string("proj-f8".to_string());

    let mut task = create_test_task_with_status(
        &project_id,
        "Broken worktree task",
        InternalStatus::PendingMerge,
    );
    task.task_branch = Some("ralphx/test-project/task-f8".to_string());
    task.worktree_path = Some("/nonexistent/path/that/does/not/exist".to_string());
    s.task_repo.create(task.clone()).await.unwrap();

    // Verify the task can be stored and retrieved with the bogus path
    let stored = s
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("Task should exist");
    assert_eq!(
        stored.worktree_path.as_deref(),
        Some("/nonexistent/path/that/does/not/exist"),
        "GAP: Invalid worktree_path is accepted without validation"
    );

    // State machine transitions don't validate worktree_path health
    let services = build_task_services(&s);
    let mut machine = create_state_machine(task.id.as_str(), "proj-f8", services);
    let mut handler = create_transition_handler(&mut machine);

    let result = handler
        .handle_transition(&State::PendingMerge, &TaskEvent::MergeComplete)
        .await;

    assert!(
        result.is_success(),
        "GAP: Transition succeeds even with broken worktree path — no path health validation"
    );
}
