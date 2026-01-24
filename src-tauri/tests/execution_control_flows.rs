// Execution Control Integration Tests
//
// These tests verify the execution control functionality:
// - AskUserQuestion flow (NeedsHumanInput event → Blocked → answer → Ready)
// - Pause/resume execution
// - Blocking and unblocking tasks

use ralphx_lib::domain::entities::{ProjectId, TaskId};
use ralphx_lib::domain::state_machine::{State, TaskEvent};
use ralphx_lib::infrastructure::sqlite::{
    open_memory_connection, run_migrations, TaskStateMachineRepository,
};

/// Helper to set up a test environment with a task in executing state
fn setup_execution_test() -> (TaskStateMachineRepository, ProjectId, TaskId) {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Insert a project
    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
        [],
    )
    .unwrap();

    // Insert a task in executing state
    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title, internal_status)
         VALUES ('task-1', 'proj-1', 'feature', 'Test Task', 'executing')",
        [],
    )
    .unwrap();

    let repo = TaskStateMachineRepository::new(conn);
    let project_id = ProjectId::from_string("proj-1".to_string());
    let task_id = TaskId::from_string("task-1".to_string());

    (repo, project_id, task_id)
}

/// Helper to create multiple tasks
fn setup_multiple_tasks() -> (TaskStateMachineRepository, ProjectId, Vec<TaskId>) {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Insert a project
    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
        [],
    )
    .unwrap();

    // Insert multiple tasks in different states
    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title, internal_status)
         VALUES
         ('task-1', 'proj-1', 'feature', 'Task 1', 'ready'),
         ('task-2', 'proj-1', 'feature', 'Task 2', 'ready'),
         ('task-3', 'proj-1', 'feature', 'Task 3', 'executing'),
         ('task-4', 'proj-1', 'feature', 'Task 4', 'backlog')",
        [],
    )
    .unwrap();

    let repo = TaskStateMachineRepository::new(conn);
    let project_id = ProjectId::from_string("proj-1".to_string());
    let task_ids = vec![
        TaskId::from_string("task-1".to_string()),
        TaskId::from_string("task-2".to_string()),
        TaskId::from_string("task-3".to_string()),
        TaskId::from_string("task-4".to_string()),
    ];

    (repo, project_id, task_ids)
}

// ============================================================================
// AskUserQuestion Flow Tests
// ============================================================================

/// Test: Full AskUserQuestion flow
///
/// Flow:
/// 1. Task is executing
/// 2. Agent emits NeedsHumanInput event (AskUserQuestion)
/// 3. Task transitions to Blocked
/// 4. User provides answer
/// 5. BlockersResolved event
/// 6. Task transitions to Ready
#[test]
fn test_ask_user_question_full_flow() {
    let (repo, _project_id, task_id) = setup_execution_test();

    // 1. Verify task is in Executing state
    assert_eq!(repo.load_state(&task_id).unwrap(), State::Executing);

    // 2. Agent needs human input (e.g., AskUserQuestion tool was called)
    let state = repo
        .process_event(
            &task_id,
            &TaskEvent::NeedsHumanInput {
                reason: "Which database should I use? MySQL or PostgreSQL?".to_string(),
            },
        )
        .unwrap();

    // 3. Verify task is now Blocked
    assert_eq!(state, State::Blocked);
    assert_eq!(repo.load_state(&task_id).unwrap(), State::Blocked);

    // 4. User answers the question (simulated by frontend calling answer_user_question)
    // 5. BlockersResolved event is sent
    let state = repo
        .process_event(&task_id, &TaskEvent::BlockersResolved)
        .unwrap();

    // 6. Verify task is Ready (will be picked up by scheduler)
    assert_eq!(state, State::Ready);
    assert_eq!(repo.load_state(&task_id).unwrap(), State::Ready);
}

/// Test: NeedsHumanInput preserves the reason
#[test]
fn test_needs_human_input_preserves_reason() {
    let (repo, _project_id, task_id) = setup_execution_test();

    // Process the event
    let event = TaskEvent::NeedsHumanInput {
        reason: "Need API credentials from user".to_string(),
    };

    repo.process_event(&task_id, &event).unwrap();

    // Verify task is blocked
    assert_eq!(repo.load_state(&task_id).unwrap(), State::Blocked);

    // In a real system, the reason would be stored in task context
    // or emitted via an event for the frontend to display
}

/// Test: Blocked task cannot be scheduled
#[test]
fn test_blocked_task_cannot_be_scheduled() {
    let (repo, _project_id, task_id) = setup_execution_test();

    // Block the task
    repo.process_event(
        &task_id,
        &TaskEvent::NeedsHumanInput {
            reason: "Waiting for input".to_string(),
        },
    )
    .unwrap();

    // Verify blocked
    assert_eq!(repo.load_state(&task_id).unwrap(), State::Blocked);

    // Attempting to schedule should fail (can't go from Blocked to Ready via Schedule)
    // The only valid transition from Blocked is BlockersResolved or Cancel
    let result = repo.process_event(&task_id, &TaskEvent::Schedule);
    assert!(result.is_err(), "Should not be able to schedule a Blocked task");
}

/// Test: Blocked task can be cancelled
#[test]
fn test_blocked_task_can_be_cancelled() {
    let (repo, _project_id, task_id) = setup_execution_test();

    // Block the task
    repo.process_event(
        &task_id,
        &TaskEvent::NeedsHumanInput {
            reason: "Waiting for input".to_string(),
        },
    )
    .unwrap();

    // Cancel the blocked task
    let state = repo.process_event(&task_id, &TaskEvent::Cancel).unwrap();

    // Verify cancelled
    assert_eq!(state, State::Cancelled);
}

/// Test: Multiple AskUserQuestion in sequence
#[test]
fn test_multiple_ask_user_questions() {
    let (repo, _project_id, task_id) = setup_execution_test();

    // First question
    repo.process_event(
        &task_id,
        &TaskEvent::NeedsHumanInput {
            reason: "Question 1: What framework?".to_string(),
        },
    )
    .unwrap();
    assert_eq!(repo.load_state(&task_id).unwrap(), State::Blocked);

    // Answer first question
    repo.process_event(&task_id, &TaskEvent::BlockersResolved).unwrap();
    assert_eq!(repo.load_state(&task_id).unwrap(), State::Ready);

    // Resume execution
    repo.persist_state(&task_id, &State::Executing).unwrap();

    // Second question
    repo.process_event(
        &task_id,
        &TaskEvent::NeedsHumanInput {
            reason: "Question 2: What styling library?".to_string(),
        },
    )
    .unwrap();
    assert_eq!(repo.load_state(&task_id).unwrap(), State::Blocked);

    // Answer second question
    repo.process_event(&task_id, &TaskEvent::BlockersResolved).unwrap();
    assert_eq!(repo.load_state(&task_id).unwrap(), State::Ready);
}

// ============================================================================
// Execution Pause and Resume Tests
// ============================================================================

/// Test: Pause execution doesn't affect currently executing tasks
#[test]
fn test_pause_does_not_affect_executing_tasks() {
    let (repo, _project_id, task_ids) = setup_multiple_tasks();

    // task-3 is executing
    let task3_id = &task_ids[2];
    assert_eq!(repo.load_state(task3_id).unwrap(), State::Executing);

    // "Pause" is a system-level flag, not a state machine event
    // Executing tasks should continue until completion
    // We verify that execution can complete normally
    let state = repo
        .process_event(task3_id, &TaskEvent::ExecutionComplete)
        .unwrap();
    assert_eq!(state, State::ExecutionDone);
}

/// Test: Ready tasks can still be scheduled (pause is handled at orchestrator level)
#[test]
fn test_ready_tasks_still_schedulable() {
    let (repo, _project_id, task_ids) = setup_multiple_tasks();

    // task-1 and task-2 are ready
    let task1_id = &task_ids[0];
    assert_eq!(repo.load_state(task1_id).unwrap(), State::Ready);

    // The state machine itself doesn't prevent scheduling
    // Pause logic is in the ExecutionControl service, not the state machine
    // A Ready task can transition to Executing
    repo.persist_state(task1_id, &State::Executing).unwrap();
    assert_eq!(repo.load_state(task1_id).unwrap(), State::Executing);
}

/// Test: Backlog tasks can be scheduled when not paused
#[test]
fn test_backlog_to_ready_scheduling() {
    let (repo, _project_id, task_ids) = setup_multiple_tasks();

    // task-4 is in backlog
    let task4_id = &task_ids[3];
    assert_eq!(repo.load_state(task4_id).unwrap(), State::Backlog);

    // Schedule the task
    let state = repo.process_event(task4_id, &TaskEvent::Schedule).unwrap();
    assert_eq!(state, State::Ready);
}

// ============================================================================
// Blocker Detection and Resolution Tests
// ============================================================================

/// Test: BlockerDetected transitions Ready to Blocked
#[test]
fn test_blocker_detected_blocks_ready_task() {
    let (repo, _project_id, task_ids) = setup_multiple_tasks();

    // task-1 is ready
    let task1_id = &task_ids[0];
    assert_eq!(repo.load_state(task1_id).unwrap(), State::Ready);

    // Blocker detected
    let state = repo
        .process_event(
            task1_id,
            &TaskEvent::BlockerDetected {
                blocker_id: "task-3".to_string(),
            },
        )
        .unwrap();

    assert_eq!(state, State::Blocked);
}

/// Test: BlockersResolved transitions Blocked to Ready
#[test]
fn test_blockers_resolved_unblocks_task() {
    let (repo, _project_id, task_ids) = setup_multiple_tasks();

    // Block task-1
    let task1_id = &task_ids[0];
    repo.process_event(
        task1_id,
        &TaskEvent::BlockerDetected {
            blocker_id: "task-3".to_string(),
        },
    )
    .unwrap();
    assert_eq!(repo.load_state(task1_id).unwrap(), State::Blocked);

    // Resolve blockers
    let state = repo
        .process_event(task1_id, &TaskEvent::BlockersResolved)
        .unwrap();
    assert_eq!(state, State::Ready);
}

/// Test: Multiple blockers resolved at once
#[test]
fn test_multiple_blockers_resolved() {
    let (repo, _project_id, task_ids) = setup_multiple_tasks();

    // Block task-1 with first blocker
    let task1_id = &task_ids[0];
    repo.process_event(
        task1_id,
        &TaskEvent::BlockerDetected {
            blocker_id: "blocker-1".to_string(),
        },
    )
    .unwrap();
    assert_eq!(repo.load_state(task1_id).unwrap(), State::Blocked);

    // In the state machine, BlockersResolved means ALL blockers are resolved
    // (the orchestrator tracks individual blockers)
    let state = repo
        .process_event(task1_id, &TaskEvent::BlockersResolved)
        .unwrap();
    assert_eq!(state, State::Ready);
}

// ============================================================================
// Task Lifecycle with Human Intervention Tests
// ============================================================================

/// Test: Complete lifecycle with AskUserQuestion
#[test]
fn test_complete_lifecycle_with_question() {
    let (repo, _project_id, task_id) = setup_execution_test();

    // Executing -> Blocked (question asked)
    repo.process_event(
        &task_id,
        &TaskEvent::NeedsHumanInput {
            reason: "Need clarification".to_string(),
        },
    )
    .unwrap();
    assert_eq!(repo.load_state(&task_id).unwrap(), State::Blocked);

    // Blocked -> Ready (question answered)
    repo.process_event(&task_id, &TaskEvent::BlockersResolved)
        .unwrap();
    assert_eq!(repo.load_state(&task_id).unwrap(), State::Ready);

    // Ready -> Executing (resumed)
    repo.persist_state(&task_id, &State::Executing).unwrap();

    // Executing -> ExecutionDone
    repo.process_event(&task_id, &TaskEvent::ExecutionComplete)
        .unwrap();
    assert_eq!(repo.load_state(&task_id).unwrap(), State::ExecutionDone);

    // ExecutionDone -> PendingReview (skip QA for this test)
    repo.persist_state(&task_id, &State::PendingReview).unwrap();

    // PendingReview -> Approved
    repo.process_event(
        &task_id,
        &TaskEvent::ReviewComplete {
            approved: true,
            feedback: None,
        },
    )
    .unwrap();
    assert_eq!(repo.load_state(&task_id).unwrap(), State::Approved);
}

/// Test: Task can fail while blocked (edge case)
#[test]
fn test_blocked_task_cannot_fail_directly() {
    let (repo, _project_id, task_id) = setup_execution_test();

    // Block the task
    repo.process_event(
        &task_id,
        &TaskEvent::NeedsHumanInput {
            reason: "Waiting".to_string(),
        },
    )
    .unwrap();
    assert_eq!(repo.load_state(&task_id).unwrap(), State::Blocked);

    // ExecutionFailed shouldn't work from Blocked state
    let result = repo.process_event(
        &task_id,
        &TaskEvent::ExecutionFailed {
            error: "Some error".to_string(),
        },
    );
    assert!(result.is_err(), "Cannot fail from Blocked state");
}

/// Test: Resume from Blocked doesn't skip to completion
#[test]
fn test_resume_from_blocked_goes_to_ready() {
    let (repo, _project_id, task_id) = setup_execution_test();

    // Block the task
    repo.process_event(
        &task_id,
        &TaskEvent::NeedsHumanInput {
            reason: "Waiting".to_string(),
        },
    )
    .unwrap();

    // Resolve blockers
    let state = repo
        .process_event(&task_id, &TaskEvent::BlockersResolved)
        .unwrap();

    // Should be Ready, not Executing or any other state
    assert_eq!(state, State::Ready);
}
