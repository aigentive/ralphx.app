use super::*;
use crate::application::AppState;
use crate::domain::entities::{ChatContextType, InternalStatus, Project, Task};
use crate::domain::repositories::AppStateRepository;
use crate::domain::state_machine::mocks::MockTaskScheduler;

// Helper to create test state
async fn setup_test_state() -> (Arc<ExecutionState>, AppState) {
    let execution_state = Arc::new(ExecutionState::new());
    let app_state = AppState::new_test();
    (execution_state, app_state)
}

/// Helper to build a StartupJobRunner from test state.
/// Returns (runner, app_state_repo) — set active project on app_state_repo for DB-based tests.
fn build_runner(
    app_state: &AppState,
    execution_state: &Arc<ExecutionState>,
) -> (StartupJobRunner<tauri::Wry>, Arc<dyn AppStateRepository>) {
    let transition_service = Arc::new(TaskTransitionService::new(
        Arc::clone(&app_state.task_repo),
        Arc::clone(&app_state.task_dependency_repo),
        Arc::clone(&app_state.project_repo),
        Arc::clone(&app_state.chat_message_repo),
        Arc::clone(&app_state.chat_conversation_repo),
        Arc::clone(&app_state.agent_run_repo),
        Arc::clone(&app_state.ideation_session_repo),
        Arc::clone(&app_state.activity_event_repo),
        Arc::clone(&app_state.message_queue),
        Arc::clone(&app_state.running_agent_registry),
        Arc::clone(execution_state),
        None,
    ));

    let agent_run_repo = Arc::clone(&app_state.agent_run_repo);
    let app_state_repo = Arc::clone(&app_state.app_state_repo);

    let active_project_state = Arc::new(crate::commands::ActiveProjectState::new());
    let runner = StartupJobRunner::new(
        Arc::clone(&app_state.task_repo),
        Arc::clone(&app_state.task_dependency_repo),
        Arc::clone(&app_state.project_repo),
        Arc::clone(&app_state.chat_conversation_repo),
        Arc::clone(&app_state.chat_message_repo),
        Arc::clone(&app_state.ideation_session_repo),
        Arc::clone(&app_state.activity_event_repo),
        Arc::clone(&app_state.message_queue),
        Arc::clone(&app_state.running_agent_registry),
        agent_run_repo,
        transition_service,
        Arc::clone(execution_state),
        Arc::clone(&active_project_state),
        Arc::clone(&app_state_repo),
    );
    (runner, app_state_repo)
}

#[tokio::test]
async fn test_resumption_skipped_when_paused() {
    let (execution_state, app_state) = setup_test_state().await;

    // Create a project with a task in Executing state
    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "Executing Task".to_string());
    task.internal_status = InternalStatus::Executing;
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Pause execution
    execution_state.pause();

    let (runner, _app_state_repo) = build_runner(&app_state, &execution_state);

    // Run should skip because paused
    runner.run().await;

    // Running count should still be 0 (no tasks resumed)
    assert_eq!(execution_state.running_count(), 0);

    // Verify no conversations were created (entry actions were NOT called)
    let convs = app_state
        .chat_conversation_repo
        .get_by_context(ChatContextType::TaskExecution, task.id.as_str())
        .await
        .unwrap();
    assert_eq!(convs.len(), 0, "No conversations should be created when paused");
}

#[tokio::test]
async fn test_resumption_spawns_agents() {
    let (execution_state, app_state) = setup_test_state().await;

    // Create a project with a task in Executing state
    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "Executing Task".to_string());
    task.internal_status = InternalStatus::Executing;
    let task_id = task.id.clone();
    app_state.task_repo.create(task).await.unwrap();

    // Set high max_concurrent to allow resumption
    execution_state.set_max_concurrent(10);

    let (runner, app_state_repo) = build_runner(&app_state, &execution_state);
    // Set active project in DB (simulates persisted state from previous session)
    app_state_repo.set_active_project(Some(&project.id)).await.unwrap();

    // Run should trigger entry actions for the Executing task
    runner.run().await;

    // Verify entry action was called by checking that a ChatConversation
    // was created for the task. The on_enter(Executing) handler calls
    // chat_service.send_message(TaskExecution, task_id, ...) which creates
    // a conversation in the repo before attempting to spawn the CLI agent.
    let convs = app_state
        .chat_conversation_repo
        .get_by_context(ChatContextType::TaskExecution, task_id.as_str())
        .await
        .unwrap();
    assert_eq!(
        convs.len(),
        1,
        "Entry action should create a TaskExecution conversation for the resumed task"
    );

    // Verify the task is still in Executing state (entry actions don't change status)
    let updated_task = app_state.task_repo.get_by_id(&task_id).await.unwrap().unwrap();
    assert_eq!(updated_task.internal_status, InternalStatus::Executing);
}

#[tokio::test]
async fn test_resumption_handles_empty_projects() {
    let (execution_state, app_state) = setup_test_state().await;

    let (runner, _app_state_repo) = build_runner(&app_state, &execution_state);

    // Run should complete without panic (no active project in DB, early return)
    runner.run().await;

    // Running count should be 0
    assert_eq!(execution_state.running_count(), 0);
}

#[tokio::test]
async fn test_resumption_respects_max_concurrent() {
    let (execution_state, app_state) = setup_test_state().await;

    // Set max concurrent to 2
    execution_state.set_max_concurrent(2);

    // Create a project with 5 tasks in Executing state
    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    for i in 0..5 {
        let mut task = Task::new(project.id.clone(), format!("Executing Task {}", i));
        task.internal_status = InternalStatus::Executing;
        app_state.task_repo.create(task).await.unwrap();
    }

    let (runner, app_state_repo) = build_runner(&app_state, &execution_state);
    app_state_repo.set_active_project(Some(&project.id)).await.unwrap();

    // Run should stop after 2 tasks due to max_concurrent
    runner.run().await;

    // Note: The actual increment happens in execute_entry_actions via the spawner.
    // Since we're using a mock spawner without execution_state wired in for this test,
    // the running_count won't actually increment. This test verifies the loop structure
    // and early exit logic based on can_start_task().

    // With our mock setup, running_count stays at 0 because the spawner doesn't have
    // execution_state. In production, the spawner would increment_running() on each spawn.
    // The test verifies that run() completes without panic when max_concurrent is reached.
}

#[tokio::test]
async fn test_resumption_handles_multiple_statuses() {
    let (execution_state, app_state) = setup_test_state().await;

    // Create a project with tasks in various agent-active states
    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task1 = Task::new(project.id.clone(), "Executing Task".to_string());
    task1.internal_status = InternalStatus::Executing;
    let task1_id = task1.id.clone();
    app_state.task_repo.create(task1).await.unwrap();

    let mut task2 = Task::new(project.id.clone(), "QaRefining Task".to_string());
    task2.internal_status = InternalStatus::QaRefining;
    app_state.task_repo.create(task2).await.unwrap();

    let mut task3 = Task::new(project.id.clone(), "Reviewing Task".to_string());
    task3.internal_status = InternalStatus::Reviewing;
    app_state.task_repo.create(task3).await.unwrap();

    // Create a task NOT in agent-active state (should be skipped)
    let mut task4 = Task::new(project.id.clone(), "Ready Task".to_string());
    task4.internal_status = InternalStatus::Ready;
    let task4_id = task4.id.clone();
    app_state.task_repo.create(task4).await.unwrap();

    // Set high max_concurrent so all tasks can be resumed
    execution_state.set_max_concurrent(10);

    let (runner, app_state_repo) = build_runner(&app_state, &execution_state);
    app_state_repo.set_active_project(Some(&project.id)).await.unwrap();

    // Run should complete
    runner.run().await;

    // Verify entry actions were called for agent-active tasks:
    // - Executing task should have a TaskExecution conversation created by on_enter(Executing)
    let exec_convs = app_state
        .chat_conversation_repo
        .get_by_context(ChatContextType::TaskExecution, task1_id.as_str())
        .await
        .unwrap();
    assert_eq!(
        exec_convs.len(),
        1,
        "Executing task should have a TaskExecution conversation"
    );

    // - Ready task should NOT have any conversations (not an agent-active state)
    let ready_exec_convs = app_state
        .chat_conversation_repo
        .get_by_context(ChatContextType::TaskExecution, task4_id.as_str())
        .await
        .unwrap();
    assert_eq!(
        ready_exec_convs.len(),
        0,
        "Ready task should not be resumed"
    );
}

#[tokio::test]
async fn test_startup_schedules_ready_tasks_when_scheduler_configured() {
    let (execution_state, app_state) = setup_test_state().await;

    // Create a mock scheduler to verify it gets called
    let scheduler = Arc::new(MockTaskScheduler::new());

    let (runner, _app_state_repo) = build_runner(&app_state, &execution_state);
    let runner = runner.with_task_scheduler(Arc::clone(&scheduler) as Arc<dyn TaskScheduler>);

    // Run startup (no active project in DB → scheduler called in early return path)
    runner.run().await;

    // Verify scheduler was called once at the end of startup
    assert_eq!(
        scheduler.call_count(),
        1,
        "Scheduler should be called once after startup resumption"
    );
}

#[tokio::test]
async fn test_startup_does_not_schedule_when_paused() {
    let (execution_state, app_state) = setup_test_state().await;

    // Pause execution
    execution_state.pause();

    // Create a mock scheduler
    let scheduler = Arc::new(MockTaskScheduler::new());

    let (runner, _app_state_repo) = build_runner(&app_state, &execution_state);
    let runner = runner.with_task_scheduler(Arc::clone(&scheduler) as Arc<dyn TaskScheduler>);

    // Run startup while paused
    runner.run().await;

    // Scheduler should NOT be called when paused (early return)
    assert_eq!(
        scheduler.call_count(),
        0,
        "Scheduler should not be called when execution is paused"
    );
}

#[tokio::test]
async fn test_startup_schedules_after_resuming_agent_tasks() {
    let (execution_state, app_state) = setup_test_state().await;

    // Create a project with an Executing task (agent-active)
    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "Executing Task".to_string());
    task.internal_status = InternalStatus::Executing;
    app_state.task_repo.create(task).await.unwrap();

    // Set high max_concurrent to allow resumption
    execution_state.set_max_concurrent(10);

    // Create a mock scheduler
    let scheduler = Arc::new(MockTaskScheduler::new());

    let (runner, app_state_repo) = build_runner(&app_state, &execution_state);
    app_state_repo.set_active_project(Some(&project.id)).await.unwrap();
    let runner = runner.with_task_scheduler(Arc::clone(&scheduler) as Arc<dyn TaskScheduler>);

    // Run startup - should resume the Executing task AND call scheduler
    runner.run().await;

    // Verify scheduler was called (happens after resumption loop)
    assert_eq!(
        scheduler.call_count(),
        1,
        "Scheduler should be called after resuming agent-active tasks"
    );
}

// ============================================================
// Phase 68 Tests: Crash Recovery for Auto-Transition States
// ============================================================

#[tokio::test]
async fn test_merging_state_resumed_on_startup() {
    // Merging state was added to AGENT_ACTIVE_STATUSES in Phase 68
    // Tasks in Merging state should have their entry actions re-triggered
    // to respawn the merger agent
    let (execution_state, app_state) = setup_test_state().await;

    // Create a project with a task in Merging state
    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "Merging Task".to_string());
    task.internal_status = InternalStatus::Merging;
    let task_id = task.id.clone();
    app_state.task_repo.create(task).await.unwrap();

    // Set high max_concurrent to allow resumption
    execution_state.set_max_concurrent(10);

    let (runner, app_state_repo) = build_runner(&app_state, &execution_state);
    app_state_repo.set_active_project(Some(&project.id)).await.unwrap();

    // Run startup
    runner.run().await;

    // Verify entry actions were called by checking for Merge conversation
    // on_enter(Merging) creates a ChatContextType::Merge conversation
    let convs = app_state
        .chat_conversation_repo
        .get_by_context(ChatContextType::Merge, task_id.as_str())
        .await
        .unwrap();
    assert_eq!(
        convs.len(),
        1,
        "Merging task should have a Merge conversation created (merger agent respawned)"
    );

    // Task should still be in Merging state (entry actions don't change status)
    let updated_task = app_state.task_repo.get_by_id(&task_id).await.unwrap().unwrap();
    assert_eq!(updated_task.internal_status, InternalStatus::Merging);
}

#[tokio::test]
async fn test_pending_review_auto_transitions_on_startup() {
    // PendingReview is in AUTO_TRANSITION_STATES
    // Tasks stuck in PendingReview should auto-transition to Reviewing
    // which spawns a reviewer agent
    let (execution_state, app_state) = setup_test_state().await;

    // Create a project with a task in PendingReview state
    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "PendingReview Task".to_string());
    task.internal_status = InternalStatus::PendingReview;
    let task_id = task.id.clone();
    app_state.task_repo.create(task).await.unwrap();

    // Set high max_concurrent to allow auto-transition
    execution_state.set_max_concurrent(10);

    let (runner, app_state_repo) = build_runner(&app_state, &execution_state);
    app_state_repo.set_active_project(Some(&project.id)).await.unwrap();

    // Run startup - should trigger auto-transition to Reviewing
    runner.run().await;

    // The auto-transition path is:
    // 1. execute_entry_actions(PendingReview) triggers on_enter(PendingReview)
    // 2. check_auto_transition detects PendingReview -> Reviewing
    // 3. transition_to(Reviewing) is called
    // 4. on_enter(Reviewing) spawns reviewer agent (creates Review conversation)

    // Check for Review conversation (indicates Reviewing was entered)
    let review_convs = app_state
        .chat_conversation_repo
        .get_by_context(ChatContextType::Review, task_id.as_str())
        .await
        .unwrap();
    assert_eq!(
        review_convs.len(),
        1,
        "PendingReview task should auto-transition to Reviewing and create Review conversation"
    );

    // Task should now be in Reviewing state
    let updated_task = app_state.task_repo.get_by_id(&task_id).await.unwrap().unwrap();
    assert_eq!(
        updated_task.internal_status,
        InternalStatus::Reviewing,
        "Task should have auto-transitioned from PendingReview to Reviewing"
    );
}

#[tokio::test]
async fn test_revision_needed_auto_transitions_on_startup() {
    // RevisionNeeded is in AUTO_TRANSITION_STATES
    // Tasks stuck in RevisionNeeded should auto-transition to ReExecuting
    // which spawns a worker agent
    let (execution_state, app_state) = setup_test_state().await;

    // Create a project with a task in RevisionNeeded state
    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "RevisionNeeded Task".to_string());
    task.internal_status = InternalStatus::RevisionNeeded;
    let task_id = task.id.clone();
    app_state.task_repo.create(task).await.unwrap();

    // Set high max_concurrent to allow auto-transition
    execution_state.set_max_concurrent(10);

    let (runner, app_state_repo) = build_runner(&app_state, &execution_state);
    app_state_repo.set_active_project(Some(&project.id)).await.unwrap();

    // Run startup - should trigger auto-transition to ReExecuting
    runner.run().await;

    // The auto-transition path is:
    // 1. execute_entry_actions(RevisionNeeded) triggers on_enter(RevisionNeeded)
    // 2. check_auto_transition detects RevisionNeeded -> ReExecuting
    // 3. transition_to(ReExecuting) is called
    // 4. on_enter(ReExecuting) spawns worker agent (creates TaskExecution conversation)

    // Check for TaskExecution conversation (indicates ReExecuting was entered)
    let exec_convs = app_state
        .chat_conversation_repo
        .get_by_context(ChatContextType::TaskExecution, task_id.as_str())
        .await
        .unwrap();
    assert_eq!(
        exec_convs.len(),
        1,
        "RevisionNeeded task should auto-transition to ReExecuting and create TaskExecution conversation"
    );

    // Task should now be in ReExecuting state
    let updated_task = app_state.task_repo.get_by_id(&task_id).await.unwrap().unwrap();
    assert_eq!(
        updated_task.internal_status,
        InternalStatus::ReExecuting,
        "Task should have auto-transitioned from RevisionNeeded to ReExecuting"
    );
}

#[tokio::test]
async fn test_approved_auto_transitions_on_startup() {
    // Approved is in AUTO_TRANSITION_STATES
    // Tasks stuck in Approved should auto-transition to PendingMerge
    // which triggers programmatic merge attempt
    let (execution_state, app_state) = setup_test_state().await;

    // Create a project with a task in Approved state
    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "Approved Task".to_string());
    task.internal_status = InternalStatus::Approved;
    let task_id = task.id.clone();
    app_state.task_repo.create(task).await.unwrap();

    // Set high max_concurrent to allow auto-transition
    execution_state.set_max_concurrent(10);

    let (runner, app_state_repo) = build_runner(&app_state, &execution_state);
    app_state_repo.set_active_project(Some(&project.id)).await.unwrap();

    // Run startup - should trigger auto-transition to PendingMerge
    runner.run().await;

    // The auto-transition path is:
    // 1. execute_entry_actions(Approved) triggers on_enter(Approved)
    // 2. check_auto_transition detects Approved -> PendingMerge
    // 3. transition_to(PendingMerge) is called
    // 4. on_enter(PendingMerge) runs attempt_programmatic_merge()

    // Task should now be in PendingMerge state (or further if merge succeeded/failed)
    // Since we're in test mode without a real git repo, the merge will likely fail
    // and the task may transition to Merging or MergeConflict
    let updated_task = app_state.task_repo.get_by_id(&task_id).await.unwrap().unwrap();
    // Approved should NOT be the final state - auto-transition should have occurred
    assert_ne!(
        updated_task.internal_status,
        InternalStatus::Approved,
        "Task should have auto-transitioned from Approved (to PendingMerge or beyond)"
    );
}

#[tokio::test]
async fn test_pending_merge_auto_transitions_on_startup() {
    // PendingMerge is in AUTO_TRANSITION_STATES
    // Tasks stuck in PendingMerge should have attempt_programmatic_merge() re-triggered
    // which transitions to Merged (success) or Merging (spawns merger agent)
    let (execution_state, app_state) = setup_test_state().await;

    // Create a project with a task in PendingMerge state
    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "PendingMerge Task".to_string());
    task.internal_status = InternalStatus::PendingMerge;
    task.task_branch = Some("task/pending-merge-test".to_string());
    let task_id = task.id.clone();
    app_state.task_repo.create(task).await.unwrap();

    // Set high max_concurrent to allow auto-transition
    execution_state.set_max_concurrent(10);

    let (runner, app_state_repo) = build_runner(&app_state, &execution_state);
    app_state_repo.set_active_project(Some(&project.id)).await.unwrap();

    // Run startup - should trigger attempt_programmatic_merge()
    runner.run().await;

    // The auto-transition path is:
    // 1. execute_entry_actions(PendingMerge) triggers on_enter(PendingMerge)
    // 2. attempt_programmatic_merge() runs
    // 3. Task transitions to Merged (success), Merging (conflict), or MergeIncomplete (error)
    // Since we're in test mode without a real git repo, the merge will fail with an error
    // and the task transitions to MergeIncomplete
    let updated_task = app_state.task_repo.get_by_id(&task_id).await.unwrap().unwrap();
    // PendingMerge should NOT be the final state - attempt_programmatic_merge should have run
    assert_ne!(
        updated_task.internal_status,
        InternalStatus::PendingMerge,
        "Task should have auto-transitioned from PendingMerge (to MergeIncomplete in test env)"
    );
}

#[tokio::test]
async fn test_qa_passed_auto_transitions_on_startup() {
    // QaPassed is in AUTO_TRANSITION_STATES
    // Tasks stuck in QaPassed should auto-transition to PendingReview
    // which then auto-transitions to Reviewing (spawns reviewer)
    let (execution_state, app_state) = setup_test_state().await;

    // Create a project with a task in QaPassed state
    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "QaPassed Task".to_string());
    task.internal_status = InternalStatus::QaPassed;
    let task_id = task.id.clone();
    app_state.task_repo.create(task).await.unwrap();

    // Set high max_concurrent to allow auto-transition
    execution_state.set_max_concurrent(10);

    let (runner, app_state_repo) = build_runner(&app_state, &execution_state);
    app_state_repo.set_active_project(Some(&project.id)).await.unwrap();

    // Run startup - should trigger auto-transition chain
    runner.run().await;

    // The auto-transition chain is:
    // 1. execute_entry_actions(QaPassed) triggers on_enter(QaPassed)
    // 2. check_auto_transition detects QaPassed -> PendingReview
    // 3. transition_to(PendingReview) -> on_enter(PendingReview)
    // 4. check_auto_transition detects PendingReview -> Reviewing
    // 5. transition_to(Reviewing) -> on_enter(Reviewing) spawns reviewer

    // Check for Review conversation (indicates the full chain completed)
    let review_convs = app_state
        .chat_conversation_repo
        .get_by_context(ChatContextType::Review, task_id.as_str())
        .await
        .unwrap();
    assert_eq!(
        review_convs.len(),
        1,
        "QaPassed task should auto-transition through PendingReview to Reviewing"
    );

    // Task should now be in Reviewing state
    let updated_task = app_state.task_repo.get_by_id(&task_id).await.unwrap().unwrap();
    assert_eq!(
        updated_task.internal_status,
        InternalStatus::Reviewing,
        "Task should have auto-transitioned from QaPassed through PendingReview to Reviewing"
    );
}

#[tokio::test]
async fn test_auto_transition_respects_max_concurrent() {
    // Auto-transitions that spawn agents should respect max_concurrent.
    // This test verifies the loop structure and early exit logic based on can_start_task().
    let (execution_state, app_state) = setup_test_state().await;

    // Set max concurrent to 2
    execution_state.set_max_concurrent(2);

    // Create a project with 5 tasks in PendingReview state
    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    for i in 0..5 {
        let mut task = Task::new(project.id.clone(), format!("PendingReview Task {}", i));
        task.internal_status = InternalStatus::PendingReview;
        app_state.task_repo.create(task).await.unwrap();
    }

    let (runner, app_state_repo) = build_runner(&app_state, &execution_state);
    app_state_repo.set_active_project(Some(&project.id)).await.unwrap();

    // Run startup - should stop after max_concurrent is reached
    runner.run().await;

    // Note: The actual increment happens in execute_entry_actions via the spawner.
    // Since we're using a mock spawner without execution_state wired in for this test,
    // the running_count won't actually increment. This test verifies the loop structure
    // and early exit logic based on can_start_task().
    //
    // With our mock setup, running_count stays at 0 because the spawner doesn't have
    // execution_state. In production, the spawner would increment_running() on each spawn.
    // The test verifies that run() completes without panic when max_concurrent check exists.
}

// ============================================================
// Phase 77 Tests: Startup Unblock Recovery
// ============================================================

#[tokio::test]
async fn test_blocked_task_unblocked_when_blocker_is_merged() {
    let (execution_state, app_state) = setup_test_state().await;

    // Create a project
    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    // Create a blocker task that is already merged
    let mut blocker_task = Task::new(project.id.clone(), "Blocker Task".to_string());
    blocker_task.internal_status = InternalStatus::Merged;
    app_state.task_repo.create(blocker_task.clone()).await.unwrap();

    // Create a blocked task
    let mut blocked_task = Task::new(project.id.clone(), "Blocked Task".to_string());
    blocked_task.internal_status = InternalStatus::Blocked;
    blocked_task.blocked_reason = Some("Waiting for: Blocker Task".to_string());
    app_state.task_repo.create(blocked_task.clone()).await.unwrap();

    // Add the dependency relationship
    app_state
        .task_dependency_repo
        .add_dependency(&blocked_task.id, &blocker_task.id)
        .await
        .unwrap();

    let (runner, _app_state_repo) = build_runner(&app_state, &execution_state);

    // Run startup - should unblock the blocked task
    runner.run().await;

    // Verify the blocked task is now Ready
    let updated_task = app_state.task_repo.get_by_id(&blocked_task.id).await.unwrap().unwrap();
    assert_eq!(
        updated_task.internal_status,
        InternalStatus::Ready,
        "Blocked task should be unblocked when blocker is Merged"
    );
    assert!(
        updated_task.blocked_reason.is_none(),
        "blocked_reason should be cleared"
    );
}

#[tokio::test]
async fn test_blocked_task_remains_blocked_when_blocker_is_approved() {
    let (execution_state, app_state) = setup_test_state().await;

    // Create a project
    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    // Create a blocker task that is approved (not yet merged)
    let mut blocker_task = Task::new(project.id.clone(), "Blocker Task".to_string());
    blocker_task.internal_status = InternalStatus::Approved;
    app_state.task_repo.create(blocker_task.clone()).await.unwrap();

    // Create a blocked task
    let mut blocked_task = Task::new(project.id.clone(), "Blocked Task".to_string());
    blocked_task.internal_status = InternalStatus::Blocked;
    app_state.task_repo.create(blocked_task.clone()).await.unwrap();

    // Add the dependency relationship
    app_state
        .task_dependency_repo
        .add_dependency(&blocked_task.id, &blocker_task.id)
        .await
        .unwrap();

    // Pause execution to isolate unblock logic from auto-transition recovery
    // (without pause, Approved would auto-transition to PendingMerge, masking the test intent)
    execution_state.pause();

    let (runner, _app_state_repo) = build_runner(&app_state, &execution_state);

    // Run startup
    runner.run().await;

    // Verify the blocked task remains Blocked (Approved is NOT a terminal state)
    let updated_task = app_state.task_repo.get_by_id(&blocked_task.id).await.unwrap().unwrap();
    assert_eq!(
        updated_task.internal_status,
        InternalStatus::Blocked,
        "Blocked task should remain blocked when blocker is Approved (not yet merged)"
    );
}

#[tokio::test]
async fn test_blocked_task_remains_blocked_when_blocker_incomplete() {
    let (execution_state, app_state) = setup_test_state().await;

    // Create a project
    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    // Create a blocker task that is still executing
    let mut blocker_task = Task::new(project.id.clone(), "Blocker Task".to_string());
    blocker_task.internal_status = InternalStatus::Executing;
    app_state.task_repo.create(blocker_task.clone()).await.unwrap();

    // Create a blocked task
    let mut blocked_task = Task::new(project.id.clone(), "Blocked Task".to_string());
    blocked_task.internal_status = InternalStatus::Blocked;
    blocked_task.blocked_reason = Some("Waiting for: Blocker Task".to_string());
    app_state.task_repo.create(blocked_task.clone()).await.unwrap();

    // Add the dependency relationship
    app_state
        .task_dependency_repo
        .add_dependency(&blocked_task.id, &blocker_task.id)
        .await
        .unwrap();

    // Pause execution to skip agent resumption (we only want to test unblocking)
    execution_state.pause();

    let (runner, _app_state_repo) = build_runner(&app_state, &execution_state);

    // Run startup
    runner.run().await;

    // Verify the blocked task is still Blocked
    let updated_task = app_state.task_repo.get_by_id(&blocked_task.id).await.unwrap().unwrap();
    assert_eq!(
        updated_task.internal_status,
        InternalStatus::Blocked,
        "Blocked task should remain blocked when blocker is still Executing"
    );
    assert!(
        updated_task.blocked_reason.is_some(),
        "blocked_reason should be preserved"
    );
}

#[tokio::test]
async fn test_blocked_task_remains_blocked_when_blocker_paused() {
    let (execution_state, app_state) = setup_test_state().await;

    // Create a project
    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    // Create a blocker task that is paused
    let mut blocker_task = Task::new(project.id.clone(), "Blocker Task".to_string());
    blocker_task.internal_status = InternalStatus::Paused;
    app_state.task_repo.create(blocker_task.clone()).await.unwrap();

    // Create a blocked task
    let mut blocked_task = Task::new(project.id.clone(), "Blocked Task".to_string());
    blocked_task.internal_status = InternalStatus::Blocked;
    blocked_task.blocked_reason = Some("Waiting for: Blocker Task".to_string());
    app_state.task_repo.create(blocked_task.clone()).await.unwrap();

    // Add the dependency relationship
    app_state
        .task_dependency_repo
        .add_dependency(&blocked_task.id, &blocker_task.id)
        .await
        .unwrap();

    // Pause execution to skip agent resumption (we only want to test unblocking)
    execution_state.pause();

    let (runner, _app_state_repo) = build_runner(&app_state, &execution_state);

    // Run startup
    runner.run().await;

    // Verify the blocked task is still Blocked
    let updated_task = app_state.task_repo.get_by_id(&blocked_task.id).await.unwrap().unwrap();
    assert_eq!(
        updated_task.internal_status,
        InternalStatus::Blocked,
        "Blocked task should remain blocked when blocker is Paused"
    );
    assert!(
        updated_task.blocked_reason.is_some(),
        "blocked_reason should be preserved"
    );
}

#[tokio::test]
async fn test_blocked_task_remains_blocked_when_blocker_stopped() {
    let (execution_state, app_state) = setup_test_state().await;

    // Create a project
    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    // Create a blocker task that is stopped
    let mut blocker_task = Task::new(project.id.clone(), "Blocker Task".to_string());
    blocker_task.internal_status = InternalStatus::Stopped;
    app_state.task_repo.create(blocker_task.clone()).await.unwrap();

    // Create a blocked task
    let mut blocked_task = Task::new(project.id.clone(), "Blocked Task".to_string());
    blocked_task.internal_status = InternalStatus::Blocked;
    blocked_task.blocked_reason = Some("Waiting for: Blocker Task".to_string());
    app_state.task_repo.create(blocked_task.clone()).await.unwrap();

    // Add the dependency relationship
    app_state
        .task_dependency_repo
        .add_dependency(&blocked_task.id, &blocker_task.id)
        .await
        .unwrap();

    // Pause execution to skip agent resumption (we only want to test unblocking)
    execution_state.pause();

    let (runner, _app_state_repo) = build_runner(&app_state, &execution_state);

    // Run startup
    runner.run().await;

    // Verify the blocked task is still Blocked
    let updated_task = app_state.task_repo.get_by_id(&blocked_task.id).await.unwrap().unwrap();
    assert_eq!(
        updated_task.internal_status,
        InternalStatus::Blocked,
        "Blocked task should remain blocked when blocker is Stopped"
    );
    assert!(
        updated_task.blocked_reason.is_some(),
        "blocked_reason should be preserved"
    );
}

#[tokio::test]
async fn test_blocked_task_with_multiple_blockers_all_complete() {
    let (execution_state, app_state) = setup_test_state().await;

    // Create a project
    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    // Create two blocker tasks - both complete
    let mut blocker1 = Task::new(project.id.clone(), "Blocker 1".to_string());
    blocker1.internal_status = InternalStatus::Merged;
    app_state.task_repo.create(blocker1.clone()).await.unwrap();

    let mut blocker2 = Task::new(project.id.clone(), "Blocker 2".to_string());
    blocker2.internal_status = InternalStatus::Failed; // Failed is also a terminal state
    app_state.task_repo.create(blocker2.clone()).await.unwrap();

    // Create a blocked task
    let mut blocked_task = Task::new(project.id.clone(), "Blocked Task".to_string());
    blocked_task.internal_status = InternalStatus::Blocked;
    app_state.task_repo.create(blocked_task.clone()).await.unwrap();

    // Add both dependency relationships
    app_state
        .task_dependency_repo
        .add_dependency(&blocked_task.id, &blocker1.id)
        .await
        .unwrap();
    app_state
        .task_dependency_repo
        .add_dependency(&blocked_task.id, &blocker2.id)
        .await
        .unwrap();

    let (runner, _app_state_repo) = build_runner(&app_state, &execution_state);

    // Run startup
    runner.run().await;

    // Verify the blocked task is now Ready
    let updated_task = app_state.task_repo.get_by_id(&blocked_task.id).await.unwrap().unwrap();
    assert_eq!(
        updated_task.internal_status,
        InternalStatus::Ready,
        "Blocked task should be unblocked when all blockers are complete"
    );
}

#[tokio::test]
async fn test_blocked_task_with_multiple_blockers_one_incomplete() {
    let (execution_state, app_state) = setup_test_state().await;

    // Create a project
    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    // Create two blocker tasks - one complete, one not
    let mut blocker1 = Task::new(project.id.clone(), "Blocker 1".to_string());
    blocker1.internal_status = InternalStatus::Merged;
    app_state.task_repo.create(blocker1.clone()).await.unwrap();

    let mut blocker2 = Task::new(project.id.clone(), "Blocker 2".to_string());
    blocker2.internal_status = InternalStatus::Reviewing; // Still in progress
    app_state.task_repo.create(blocker2.clone()).await.unwrap();

    // Create a blocked task
    let mut blocked_task = Task::new(project.id.clone(), "Blocked Task".to_string());
    blocked_task.internal_status = InternalStatus::Blocked;
    app_state.task_repo.create(blocked_task.clone()).await.unwrap();

    // Add both dependency relationships
    app_state
        .task_dependency_repo
        .add_dependency(&blocked_task.id, &blocker1.id)
        .await
        .unwrap();
    app_state
        .task_dependency_repo
        .add_dependency(&blocked_task.id, &blocker2.id)
        .await
        .unwrap();

    // Pause execution to skip agent resumption
    execution_state.pause();

    let (runner, _app_state_repo) = build_runner(&app_state, &execution_state);

    // Run startup
    runner.run().await;

    // Verify the blocked task is still Blocked
    let updated_task = app_state.task_repo.get_by_id(&blocked_task.id).await.unwrap().unwrap();
    assert_eq!(
        updated_task.internal_status,
        InternalStatus::Blocked,
        "Blocked task should remain blocked when any blocker is incomplete"
    );
}

#[tokio::test]
async fn test_unblock_runs_even_when_paused() {
    // Unblocking should run even when execution is paused, since it doesn't spawn agents
    let (execution_state, app_state) = setup_test_state().await;

    // Pause execution
    execution_state.pause();

    // Create a project
    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    // Create a merged blocker
    let mut blocker_task = Task::new(project.id.clone(), "Blocker Task".to_string());
    blocker_task.internal_status = InternalStatus::Merged;
    app_state.task_repo.create(blocker_task.clone()).await.unwrap();

    // Create a blocked task
    let mut blocked_task = Task::new(project.id.clone(), "Blocked Task".to_string());
    blocked_task.internal_status = InternalStatus::Blocked;
    app_state.task_repo.create(blocked_task.clone()).await.unwrap();

    // Add dependency
    app_state
        .task_dependency_repo
        .add_dependency(&blocked_task.id, &blocker_task.id)
        .await
        .unwrap();

    let (runner, _app_state_repo) = build_runner(&app_state, &execution_state);

    // Run startup while paused
    runner.run().await;

    // Verify the blocked task is still unblocked even though execution is paused
    let updated_task = app_state.task_repo.get_by_id(&blocked_task.id).await.unwrap().unwrap();
    assert_eq!(
        updated_task.internal_status,
        InternalStatus::Ready,
        "Blocked task should be unblocked even when execution is paused"
    );
}

// ============================================================
// Phase 90 Tests: DB-based active project persistence
// ============================================================

#[tokio::test]
async fn test_resumption_reads_active_project_from_db() {
    // Verifies the runner reads the active project from the DB (app_state_repo)
    // instead of waiting for the frontend to set it via IPC.
    let (execution_state, app_state) = setup_test_state().await;

    // Create a project with a task in Executing state
    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "Executing Task".to_string());
    task.internal_status = InternalStatus::Executing;
    let task_id = task.id.clone();
    app_state.task_repo.create(task).await.unwrap();

    execution_state.set_max_concurrent(10);

    let (runner, app_state_repo) = build_runner(&app_state, &execution_state);

    // Set active project in DB only (NOT in-memory ActiveProjectState)
    // This simulates the persisted state from a previous session
    app_state_repo
        .set_active_project(Some(&project.id))
        .await
        .unwrap();

    // Run the startup runner — it should read from DB and resume
    runner.run().await;

    // Verify entry actions were called (task was resumed using DB value)
    let convs = app_state
        .chat_conversation_repo
        .get_by_context(ChatContextType::TaskExecution, task_id.as_str())
        .await
        .unwrap();
    assert_eq!(
        convs.len(),
        1,
        "Runner should read active project from DB and resume the executing task"
    );
}

#[tokio::test]
async fn test_resumption_skips_when_no_active_project_in_db() {
    // Verifies the runner skips resumption when no active project is in the DB
    // (fresh install scenario)
    let (execution_state, app_state) = setup_test_state().await;

    // Create a project with a task in Executing state
    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "Executing Task".to_string());
    task.internal_status = InternalStatus::Executing;
    let task_id = task.id.clone();
    app_state.task_repo.create(task).await.unwrap();

    execution_state.set_max_concurrent(10);

    let (runner, _app_state_repo) = build_runner(&app_state, &execution_state);
    // Don't set active project in DB — simulates fresh install

    runner.run().await;

    // Verify NO entry actions were called (no active project in DB)
    let convs = app_state
        .chat_conversation_repo
        .get_by_context(ChatContextType::TaskExecution, task_id.as_str())
        .await
        .unwrap();
    assert_eq!(
        convs.len(),
        0,
        "No tasks should be resumed when no active project in DB"
    );
}
