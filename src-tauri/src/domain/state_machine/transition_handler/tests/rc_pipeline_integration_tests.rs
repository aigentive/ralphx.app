// RC integration tests: merge pipeline failure scenarios from logs-21
//
// Real git repos + MemoryTaskRepository + MockChatService (per CLAUDE.md rule 1.5).
// No real agent spawns — mock spawner only.
//
// Scenario 1 — RC1: Cleanup timeout doesn't kill merge attempt
//   Before fix: kill_worktree_processes_async used spawn_blocking(lsof). When the
//   Tokio timeout fired, the OS lsof process kept running, consuming the merge deadline.
//   After fix: tokio::process::Command + kill_on_drop(true) — lsof is killed on timeout.
//
//   Tests:
//   1a. kill_worktree_processes_async returns within timeout bound (lsof process killed)
//   1b. Merge succeeds even when task.worktree_path triggers the lsof scan in step 0b
//   1c. Merge with worktree_path completes steps 1-5 and reaches Merged state
//
// Scenario 2 — RC2: running_count TOCTOU
//   Before fix: spawn_deferred_merge_retry guarded try_retry_main_merges with
//   running_count == 0, checked inside a tokio::spawn. Auto-transitions (e.g.
//   PendingReview→Reviewing, ~72ms window) could increment running_count before
//   the spawned future evaluated it, causing the retry to be silently skipped.
//   After fix: guard removed — retry ALWAYS fires. Authoritative gate is inside
//   check_main_merge_deferral, which reads running_count fresh at merge-start time.
//
//   Tests:
//   2a. spawn_deferred_merge_retry fires try_retry_main_merges even with running_count > 0
//   2b. Concurrent merge retry doesn't interfere: merge task succeeds while retry fires

use super::helpers::*;
use crate::commands::ExecutionState;
use crate::domain::entities::{InternalStatus, MergeStrategy, Project, ProjectId, Task};
use crate::domain::state_machine::services::TaskScheduler;
use crate::domain::state_machine::{State, TransitionHandler};

// ─────────────────────────────────────────────────────────────────────────────
// Shared helper: services with task/project repos + mock scheduler + mock chat
// ─────────────────────────────────────────────────────────────────────────────

fn make_services_with_repos_and_state(
    task_repo: Arc<MemoryTaskRepository>,
    project_repo: Arc<MemoryProjectRepository>,
    execution_state: Arc<ExecutionState>,
) -> (TaskServices, Arc<MockTaskScheduler>) {
    let scheduler = Arc::new(MockTaskScheduler::new());
    let services = TaskServices::new(
        Arc::new(MockAgentSpawner::new()) as Arc<dyn AgentSpawner>,
        Arc::new(MockEventEmitter::new()) as Arc<dyn EventEmitter>,
        Arc::new(MockNotifier::new()) as Arc<dyn Notifier>,
        Arc::new(MockDependencyManager::new()) as Arc<dyn DependencyManager>,
        Arc::new(MockReviewStarter::new()) as Arc<dyn ReviewStarter>,
        Arc::new(MockChatService::new()) as Arc<dyn ChatService>,
    )
    .with_task_scheduler(Arc::clone(&scheduler) as Arc<dyn TaskScheduler>)
    .with_task_repo(Arc::clone(&task_repo) as Arc<dyn TaskRepository>)
    .with_project_repo(Arc::clone(&project_repo) as Arc<dyn ProjectRepository>)
    .with_execution_state(execution_state);
    (services, scheduler)
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 1 — RC1 tests
// ─────────────────────────────────────────────────────────────────────────────

/// 1a. kill_worktree_processes_async with a 1-second timeout returns within 2 seconds
/// even when the worktree exists and lsof must scan it.
///
/// Before RC1: spawn_blocking left the OS lsof thread running after Tokio timeout.
/// After RC1: kill_on_drop sends SIGKILL to the lsof child process at timeout.
///
/// Observable behavior: function returns promptly regardless of how long lsof
/// would have blocked, because the process is killed rather than abandoned.
#[tokio::test]
async fn test_rc1_lsof_kill_on_drop_returns_within_timeout_bound() {
    let git_repo = setup_real_git_repo();

    // 1-second timeout — tight enough to show cancellation works
    let start = std::time::Instant::now();
    crate::domain::services::kill_worktree_processes_async(git_repo.path(), 1, false).await;
    let elapsed = start.elapsed();

    // Must return within 2x the timeout. The old spawn_blocking approach returned
    // the future at 1s but the OS thread/lsof process kept running indefinitely.
    // With kill_on_drop, the function returns AND the process is gone.
    assert!(
        elapsed.as_millis() < 2500,
        "RC1: kill_worktree_processes_async must return within timeout bound, \
         got {}ms (expected <2500ms). lsof process may not have been killed.",
        elapsed.as_millis()
    );
}

/// 1b. Merge succeeds when task.worktree_path exists, triggering the lsof scan in
/// pre_merge_cleanup step 0b.
///
/// This exercises the RC1 fix end-to-end: the worktree_path causes step 0b to call
/// kill_worktree_processes_async. With the fix, the lsof scan completes or times out
/// within the configured bound, and steps 1-5 still run, allowing the merge to succeed.
///
/// Before RC1: a long-running lsof scan could fill the merge deadline budget,
/// causing steps 1-5 to be skipped and the merge to fail with MergeIncomplete.
#[tokio::test]
async fn test_rc1_merge_succeeds_despite_worktree_path_lsof_scan() {
    let git_repo = setup_real_git_repo();
    let repo_path = git_repo.path();

    // Create a real worktree so task.worktree_path points to an existing path.
    // This causes pre_merge_cleanup step 0b to run kill_worktree_processes_async.
    let task_wt_path = repo_path.join(".worktrees").join("rc1-test-task-wt");
    let _ = std::process::Command::new("git")
        .args([
            "worktree",
            "add",
            task_wt_path.to_str().unwrap(),
            &git_repo.task_branch,
        ])
        .current_dir(repo_path)
        .output()
        .expect("create task worktree for RC1 test");

    assert!(
        task_wt_path.exists(),
        "Precondition: worktree path must exist to trigger lsof scan in step 0b"
    );

    let task_repo = Arc::new(MemoryTaskRepository::new());
    let project_repo = Arc::new(MemoryProjectRepository::new());
    let execution_state = Arc::new(ExecutionState::new());

    let project_id = ProjectId::from_string("proj-1".to_string());
    let mut task = Task::new(project_id.clone(), "RC1 lsof scan test".to_string());
    task.internal_status = InternalStatus::PendingMerge;
    task.task_branch = Some(git_repo.task_branch.clone());
    // Set worktree_path — this triggers the lsof scan in step 0b
    task.worktree_path = Some(task_wt_path.to_string_lossy().to_string());
    let task_id = task.id.clone();
    task_repo.create(task).await.unwrap();

    let mut project = Project::new("test-project".to_string(), git_repo.path_string());
    project.id = project_id;
    project.base_branch = Some("main".to_string());
    project.merge_strategy = MergeStrategy::Merge;
    project_repo.create(project).await.unwrap();

    let (services, _) = make_services_with_repos_and_state(
        Arc::clone(&task_repo),
        Arc::clone(&project_repo),
        execution_state,
    );
    let context =
        crate::domain::state_machine::context::TaskContext::new(task_id.as_str(), "proj-1", services);
    let mut machine = crate::domain::state_machine::TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    let start = std::time::Instant::now();
    let _ = handler.on_enter(&State::PendingMerge).await;
    let elapsed = start.elapsed();

    let updated = task_repo.get_by_id(&task_id).await.unwrap().unwrap();
    assert_eq!(
        updated.internal_status,
        InternalStatus::Merged,
        "RC1: Merge must succeed even with worktree_path triggering lsof scan. \
         Got {:?}. Metadata: {:?}. This means steps 1-5 were blocked by step 0b.",
        updated.internal_status,
        updated.metadata,
    );

    // Pipeline should complete in bounded time — lsof timeout (10s) is the worst case
    // for step 0b, plus a few seconds for steps 1-5 and strategy dispatch.
    assert!(
        elapsed.as_secs() < 30,
        "RC1: Merge pipeline with worktree lsof scan should complete within 30s, took {}s",
        elapsed.as_secs()
    );
}

/// 1c. Merge with worktree_path completes cleanup steps 1-5 and all steps run.
///
/// Verifies that even after lsof step 0b runs (with worktree path), the merge
/// still proceeds through the full cleanup chain to strategy dispatch. The
/// assertion is the final state (Merged) proving all pipeline stages ran.
///
/// This is distinct from 1b in that it focuses on confirming that steps AFTER
/// 0b are not skipped, by using a task branch that would fail if steps 1-5
/// weren't run (stale index.lock present).
#[tokio::test]
async fn test_rc1_cleanup_steps_run_after_lsof_step_with_stale_lock() {
    let git_repo = setup_real_git_repo();
    let repo_path = git_repo.path();

    // Create a stale index.lock to test that step 1 still runs after step 0b
    let lock_path = repo_path.join(".git").join("index.lock");
    std::fs::write(&lock_path, "stale lock").unwrap();

    // Backdate the lock so remove_stale_index_lock considers it stale
    let backdated = chrono::Utc::now() - chrono::Duration::seconds(120);
    let touch_ts = backdated.format("%Y%m%d%H%M.%S").to_string();
    let _ = std::process::Command::new("touch")
        .args(["-t", &touch_ts, lock_path.to_str().unwrap()])
        .output()
        .expect("backdate index.lock");

    // Create a worktree to trigger step 0b lsof scan
    let task_wt_path = repo_path.join(".worktrees").join("rc1-lock-test-wt");
    let _ = std::process::Command::new("git")
        .args([
            "worktree",
            "add",
            task_wt_path.to_str().unwrap(),
            &git_repo.task_branch,
        ])
        .current_dir(repo_path)
        .output()
        .expect("create worktree");

    let task_repo = Arc::new(MemoryTaskRepository::new());
    let project_repo = Arc::new(MemoryProjectRepository::new());
    let execution_state = Arc::new(ExecutionState::new());

    let project_id = ProjectId::from_string("proj-1".to_string());
    let mut task = Task::new(project_id.clone(), "RC1 steps test".to_string());
    task.internal_status = InternalStatus::PendingMerge;
    task.task_branch = Some(git_repo.task_branch.clone());
    task.worktree_path = Some(task_wt_path.to_string_lossy().to_string());
    // Simulate a retry so Phase 1 GUARD runs cleanup (stale lock + worktree removal)
    task.metadata = Some(serde_json::json!({"merge_failure_source": "test_prior_failure"}).to_string());
    let task_id = task.id.clone();
    task_repo.create(task).await.unwrap();

    let mut project = Project::new("test-project".to_string(), git_repo.path_string());
    project.id = project_id;
    project.base_branch = Some("main".to_string());
    project.merge_strategy = MergeStrategy::Merge;
    project_repo.create(project).await.unwrap();

    let (services, _) = make_services_with_repos_and_state(
        Arc::clone(&task_repo),
        Arc::clone(&project_repo),
        execution_state,
    );
    let context =
        crate::domain::state_machine::context::TaskContext::new(task_id.as_str(), "proj-1", services);
    let mut machine = crate::domain::state_machine::TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    let _ = handler.on_enter(&State::PendingMerge).await;

    // If step 0b blocked all subsequent steps, step 1 would never remove index.lock
    // and git operations would fail → task would end up in MergeIncomplete, not Merged.
    assert!(!lock_path.exists(), "Step 1 should have removed the stale index.lock \
        (if it still exists, steps 1-5 were blocked by step 0b lsof scan)");

    let updated = task_repo.get_by_id(&task_id).await.unwrap().unwrap();
    assert_eq!(
        updated.internal_status,
        InternalStatus::Merged,
        "RC1: All cleanup steps 0b-5 ran and merge succeeded. Got {:?}. Metadata: {:?}",
        updated.internal_status,
        updated.metadata,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 2 — RC2 tests
// ─────────────────────────────────────────────────────────────────────────────

/// 2a. spawn_deferred_merge_retry fires try_retry_main_merges even when
/// running_count > 0 (integration scenario with real git repo services).
///
/// Simulates the production race window: Task B (worker) exits PendingMerge
/// while Task A (reviewer) is still running (running_count = 1). Before the RC2 fix,
/// try_retry_main_merges would be skipped. After the fix, it always fires.
///
/// The authoritative gate (check_main_merge_deferral inside attempt_programmatic_merge)
/// then reads running_count fresh at merge-start time. With defer_merge_enabled = false
/// (current production config), the gate is bypassed and merge proceeds normally.
///
/// Observable assertion: try_retry_main_merges is called regardless of running_count.
#[tokio::test]
async fn test_rc2_retry_always_fires_when_running_count_positive() {
    let execution_state = Arc::new(ExecutionState::new());
    // Simulate: reviewer is active (running_count = 1)
    execution_state.increment_running();
    assert_eq!(execution_state.running_count(), 1, "Precondition: one agent running");

    let scheduler = Arc::new(MockTaskScheduler::new());
    let services = TaskServices::new_mock()
        .with_task_scheduler(
            Arc::clone(&scheduler) as Arc<dyn TaskScheduler>,
        )
        .with_execution_state(Arc::clone(&execution_state));

    let context = create_context_with_services("merge-task-1", "proj-1", services);
    let mut machine = crate::domain::state_machine::TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    // Simulate: merge task exits PendingMerge (e.g., completed via MergeIncomplete retry)
    handler.on_exit(&State::PendingMerge, &State::MergeIncomplete).await;

    let sched = Arc::clone(&scheduler);
    assert!(
        wait_for_condition(
            || {
                let s = Arc::clone(&sched);
                async move {
                    s.get_calls()
                        .iter()
                        .any(|c| c.method == "try_retry_main_merges")
                }
            },
            5000,
        )
        .await,
        "RC2: try_retry_main_merges must fire even when running_count=1. \
         Before the fix, the running_count guard in spawn_deferred_merge_retry \
         would have skipped this call."
    );
}

/// 2b. Concurrent scenario: merge task with running_count > 0 doesn't cause
/// interference. When try_retry_main_merges fires and re-invokes attempt_programmatic_merge
/// (via MockTaskScheduler's no-op), the merge task previously in PendingMerge still
/// completes correctly in a separate context.
///
/// This test uses a real git repo to verify the full pipeline: the merge task
/// (with no main_merge_deferred metadata) reaches Merged while another task's
/// agent is "running" (running_count = 1). With defer_merge_enabled = false,
/// the gate is bypassed and the merge proceeds immediately.
#[tokio::test]
async fn test_rc2_merge_proceeds_correctly_while_agents_running() {
    let git_repo = setup_real_git_repo();

    let execution_state = Arc::new(ExecutionState::new());
    // Simulate a reviewer agent still active (running_count = 1)
    execution_state.increment_running();

    let task_repo = Arc::new(MemoryTaskRepository::new());
    let project_repo = Arc::new(MemoryProjectRepository::new());

    let project_id = ProjectId::from_string("proj-1".to_string());
    let mut task = Task::new(project_id.clone(), "RC2 concurrent merge test".to_string());
    task.internal_status = InternalStatus::PendingMerge;
    task.task_branch = Some(git_repo.task_branch.clone());
    let task_id = task.id.clone();
    task_repo.create(task).await.unwrap();

    let mut project = Project::new("test-project".to_string(), git_repo.path_string());
    project.id = project_id;
    project.base_branch = Some("main".to_string());
    project.merge_strategy = MergeStrategy::Merge;
    project_repo.create(project).await.unwrap();

    let (services, scheduler) = make_services_with_repos_and_state(
        Arc::clone(&task_repo),
        Arc::clone(&project_repo),
        Arc::clone(&execution_state),
    );
    let context =
        crate::domain::state_machine::context::TaskContext::new(task_id.as_str(), "proj-1", services);
    let mut machine = crate::domain::state_machine::TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    // Trigger PendingMerge entry — with running_count=1, the merge still proceeds
    // (defer_merge_enabled=false in production config means no deferral gate).
    let _ = handler.on_enter(&State::PendingMerge).await;

    let updated = task_repo.get_by_id(&task_id).await.unwrap().unwrap();
    assert_eq!(
        updated.internal_status,
        InternalStatus::Merged,
        "RC2: Merge must complete successfully even with running_count=1. \
         Got {:?}. Metadata: {:?}",
        updated.internal_status,
        updated.metadata,
    );

    // Verify git log shows merge landed on main
    let log_output = std::process::Command::new("git")
        .args(["log", "--oneline", "main"])
        .current_dir(git_repo.path())
        .output()
        .expect("git log");
    let log_str = String::from_utf8_lossy(&log_output.stdout);
    assert!(
        log_str.contains("feature") || log_str.contains("add feature"),
        "RC2: Merge commit should appear on main. Log:\n{}",
        log_str
    );

    // In production, the state machine calls on_exit(PendingMerge) after on_enter completes
    // and the task transitions to Merged. In the test context, on_enter updates the repo
    // directly without dispatching through the state machine event loop, so we fire on_exit
    // explicitly to simulate what task_transition_service would do.
    handler.on_exit(&State::PendingMerge, &State::Merged).await;

    // After PendingMerge exit, spawn_deferred_merge_retry fires try_retry_main_merges
    // regardless of running_count = 1 (RC2 fix: guard removed from spawned future).
    let sched = Arc::clone(&scheduler);
    assert!(
        wait_for_condition(
            || {
                let s = Arc::clone(&sched);
                async move {
                    s.get_calls()
                        .iter()
                        .any(|c| c.method == "try_retry_main_merges")
                }
            },
            5000,
        )
        .await,
        "RC2: try_retry_main_merges must fire after PendingMerge exit \
         (TOCTOU fix: guard removed, always fires regardless of running_count)"
    );
}
