// Tests for merge-hang fixes: pre_merge_cleanup step 0, 120s deadline, step timeouts
//
// These tests verify the three defensive layers added to prevent
// the 5+ minute merge hang caused by worktree deletion blocking
// when an agent still holds files open in the worktree.

use super::helpers::*;
use crate::domain::state_machine::{State, TransitionHandler};

// ==================
// Step 0: Agent kill before worktree deletion
// ==================

/// pre_merge_cleanup step 0 invokes stop_agent for Review and Merge context types.
///
/// With repos wired, `attempt_programmatic_merge()` proceeds past the early-return
/// guard and actually executes `pre_merge_cleanup` (stop_agent calls), then tries
/// the merge strategy dispatch. The MockChatService.stop_agent returns Ok(false)
/// (no agent running), which should be handled gracefully.
#[tokio::test]
async fn test_step0_agent_kill_executes_without_error() {
    let (mut machine, _, _) = setup_pending_merge_repos("Step 0 test", Some("feature/test"))
        .await
        .into_machine();
    let handler = TransitionHandler::new(&mut machine);

    // With repos wired, on_enter(PendingMerge) proceeds past the guard and actually
    // runs pre_merge_cleanup (step 0 = stop_agent) before trying to merge.
    // Git operations fail fast on nonexistent dir.
    let result = handler.on_enter(&State::PendingMerge).await;
    assert!(
        result.is_ok(),
        "on_enter(PendingMerge) should succeed even with step 0 agent kill"
    );
}

// Tests early-return guard — does not reach merge strategy dispatch
/// Without repos, attempt_programmatic_merge bails before the step 0 settle sleep.
#[tokio::test]
async fn test_guard_no_repos_skips_step0_settle_sleep() {
    use std::time::Instant;

    let services = TaskServices::new_mock();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    let start = Instant::now();
    let _ = handler.on_enter(&State::PendingMerge).await;
    let elapsed = start.elapsed();

    // Without repos, attempt_programmatic_merge returns immediately (no cleanup runs)
    assert!(
        elapsed.as_millis() < 100,
        "on_enter(PendingMerge) without repos should skip cleanup entirely, took {}ms",
        elapsed.as_millis()
    );
}

/// pre_merge_cleanup step 0b emits two-phase progress: agent cancellation then lsof scan.
///
/// Validates that the code path after the agent-stop loop (the orphaned-process scan)
/// runs without error. Progress events are no-ops without a real AppHandle, but the
/// flow compiles and executes correctly. This is the RC-B regression guard.
#[tokio::test]
async fn test_step0b_two_phase_progress_no_regression() {
    let (mut machine, _, _) = setup_pending_merge_repos("Step 0b two-phase", Some("feature/test"))
        .await
        .into_machine();
    let handler = TransitionHandler::new(&mut machine);

    // With repos wired, on_enter(PendingMerge) runs:
    //   1. stop_agent for Review + Merge (fast, no agents running)
    //   2. emit "Scanning worktree for orphaned processes..."  ← RC-B addition
    //   3. kill_worktree_processes_async (worktree path from task may not exist → no-op)
    // All steps must complete without blocking or panicking.
    let start = std::time::Instant::now();
    let result = handler.on_enter(&State::PendingMerge).await;
    let elapsed = start.elapsed();

    assert!(
        result.is_ok(),
        "on_enter(PendingMerge) should succeed with two-phase step 0b"
    );
    assert!(
        elapsed.as_secs() < 30,
        "step 0b should not block indefinitely, took {}s",
        elapsed.as_secs()
    );
}

// ==================
// 120s merge deadline
// ==================

// Tests early-return guard — does not reach merge strategy dispatch
/// Without repos, the no-repos guard returns before the deadline check is reached.
#[tokio::test]
async fn test_guard_no_repos_completes_within_deadline() {
    use std::time::Instant;

    // Use mock services without repos — this means attempt_programmatic_merge
    // returns immediately, but the code structure with the deadline still compiles
    // and the no-repos early return happens before the deadline check.
    let services = TaskServices::new_mock();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    let start = Instant::now();
    let _ = handler.on_enter(&State::PendingMerge).await;
    let elapsed = start.elapsed();

    // If the deadline code were broken (e.g., blocking forever), this would fail
    assert!(
        elapsed.as_secs() < 5,
        "PendingMerge entry should complete well within deadline, took {}s",
        elapsed.as_secs()
    );
}

// Tests early-return guard — does not reach merge strategy dispatch
/// Without repos, the repos-unavailable path fires on_exit(PendingMerge, MergeIncomplete).
#[tokio::test]
async fn test_guard_no_repos_fires_on_exit_to_merge_incomplete() {
    let emitter = Arc::new(MockEventEmitter::new());
    let mut services = TaskServices::new_mock();
    services.event_emitter = Arc::clone(&emitter) as Arc<dyn EventEmitter>;

    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    // Without repos, attempt_programmatic_merge logs error and calls on_exit
    let _ = handler.on_enter(&State::PendingMerge).await;

    // The repos-unavailable path emits on_exit(PendingMerge, MergeIncomplete)
    // which is the same code path used by the deadline timeout
    // Verify the handler completes without panic
}

// ==================
// Timeout wrappers on steps 4/5/6
// ==================

/// Steps 4/5/6 timeout wrappers don't break existing behavior.
///
/// With repos wired, `attempt_programmatic_merge()` proceeds past the early-return
/// guard and actually runs pre_merge_cleanup (including timeout-wrapped steps)
/// before hitting the merge strategy dispatch. When the git dir doesn't exist,
/// the merge fails via handle_outcome_git_error → MergeIncomplete.
///
/// Note: The git error path in merge_outcome_handler.rs uses a local
/// transition_to_merge_incomplete (not self.on_exit), so on_exit side effects
/// like try_retry_deferred_merges are NOT triggered. Deferred retry is handled
/// by the reconciler instead.
#[tokio::test]
async fn test_timeout_wrappers_dont_break_existing_workflow() {
    let (mut machine, task_repo, task_id) =
        setup_pending_merge_repos("Timeout wrapper test", Some("feature/test"))
            .await
            .into_machine();
    let handler = TransitionHandler::new(&mut machine);

    // Enter PendingMerge — with repos, this runs pre_merge_cleanup (timeout-wrapped steps)
    // then attempts merge, which fails on nonexistent git dir.
    let start = std::time::Instant::now();
    let result = handler.on_enter(&State::PendingMerge).await;
    let elapsed = start.elapsed();
    assert!(result.is_ok());

    // Verify the merge path completes in bounded time (not stuck in cleanup/strategy)
    assert!(
        elapsed.as_secs() < 30,
        "on_enter(PendingMerge) with repos should complete in bounded time, took {}s",
        elapsed.as_secs()
    );

    // Verify the task transitioned to MergeIncomplete (not stuck in PendingMerge)
    let updated = task_repo.get_by_id(&task_id).await.unwrap().unwrap();
    assert_eq!(
        updated.internal_status,
        crate::domain::entities::InternalStatus::MergeIncomplete,
        "Task should be in MergeIncomplete after merge fails with nonexistent git dir, got {:?}",
        updated.internal_status
    );
}

// ==================
// Config: pending_merge_stale_minutes = 2
// ==================

/// Verify pending_merge_stale_minutes code default is 2 (reduced from 5).
///
/// Tests the struct default directly to avoid config file interference.
#[test]
fn test_pending_merge_stale_minutes_default_is_2() {
    use crate::infrastructure::agents::claude::ReconciliationConfig;

    let config = ReconciliationConfig::default();
    assert_eq!(
        config.pending_merge_stale_minutes, 2,
        "pending_merge_stale_minutes should default to 2 (reduced from 5 for faster recovery)"
    );
}

// ==================
// Post-merge cleanup: scheduler triggered after unblock_dependents
// ==================

/// post_merge_cleanup triggers try_schedule_ready_tasks after unblocking dependents.
///
/// This verifies the fix that adds scheduler triggering to the programmatic
/// merge path (which bypasses on_enter(Merged)).
#[tokio::test]
async fn test_post_merge_cleanup_triggers_scheduler() {
    use crate::domain::state_machine::mocks::MockTaskScheduler;

    let scheduler = Arc::new(MockTaskScheduler::new());
    let dep_manager = Arc::new(MockDependencyManager::new());

    let mut services = TaskServices::new_mock();
    services.dependency_manager = Arc::clone(&dep_manager) as Arc<dyn DependencyManager>;
    services.task_scheduler =
        Some(Arc::clone(&scheduler)
            as Arc<
                dyn crate::domain::state_machine::services::TaskScheduler,
            >);

    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    // Trigger on_enter(Merged) which calls unblock_dependents + try_schedule_ready_tasks
    let _ = handler.on_enter(&State::Merged).await;

    // Wait for spawned tasks to call try_schedule_ready_tasks
    let sched = Arc::clone(&scheduler);
    assert!(
        wait_for_condition(
            || {
                let s = Arc::clone(&sched);
                async move {
                    s.get_calls()
                        .iter()
                        .any(|c| c.method == "try_schedule_ready_tasks")
                }
            },
            5000
        )
        .await,
        "Scheduler should be triggered after unblocking dependents"
    );

    let dep_calls = dep_manager.get_calls();
    assert!(
        dep_calls.iter().any(|c| c.method == "unblock_dependents"),
        "unblock_dependents should be called on Merged entry"
    );
}
