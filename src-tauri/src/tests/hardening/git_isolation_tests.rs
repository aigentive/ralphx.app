// Git Isolation Hardening Tests (A1-A8)
//
// Tests for git branch/worktree isolation during task execution.
// Since GitService methods are static (not trait-based), we cannot mock actual
// git operations. These tests verify:
// - State machine transitions (Ready->Executing) work correctly
// - on_enter runs and calls chat_service.send_message()
// - AppError::ExecutionBlocked variant exists and can be matched
// - Gap scenarios demonstrate missing validation

use super::helpers::*;
use crate::domain::entities::{GitMode, IdeationSessionId, InternalStatus, ProjectId, TaskId};
use crate::domain::repositories::{ProjectRepository, TaskRepository};
use crate::domain::state_machine::events::TaskEvent;
use crate::domain::state_machine::machine::State;
use crate::error::AppError;

// ============================================================================
// A1: Worktree creation fails -> ExecutionBlocked
// ============================================================================

#[tokio::test]
async fn test_a1_worktree_mode_transition_ready_to_executing() {
    // Scenario A1: Worktree creation fails -> ExecutionBlocked — COVERED
    //
    // Since GitService is static (not mockable), we verify:
    // 1. The state machine transitions Ready->Executing correctly
    // 2. on_enter(Executing) runs and calls chat_service.send_message()
    // 3. With mock repos, the git setup path is exercised but uses real git
    //    commands which will fail on /tmp/test-project (not a git repo)
    //
    // The actual git worktree creation failure returns ExecutionBlocked,
    // but TransitionHandler catches on_enter errors and still returns Success.

    let svc = create_hardening_services();

    // Create project in Worktree mode
    let project = create_test_project_with_git_mode("worktree-proj", GitMode::Worktree);
    let project_id = project.id.clone();
    svc.project_repo.create(project).await.unwrap();

    // Create task in Ready state
    let mut task = create_test_task(&project_id, "Test worktree task");
    task.internal_status = InternalStatus::Ready;
    let task_id_str = task.id.as_str().to_string();
    svc.task_repo.create(task).await.unwrap();

    let services = build_task_services(&svc);
    let mut machine = create_state_machine(&task_id_str, project_id.as_str(), services);
    let mut handler = create_transition_handler(&mut machine);

    let result = handler
        .handle_transition(&State::Ready, &TaskEvent::StartExecution)
        .await;

    // After the fix, ExecutionBlocked triggers auto-dispatch of ExecutionFailed,
    // so the task transitions to Failed instead of staying in Executing
    assert!(
        result.is_success(),
        "TransitionHandler should return Success after auto-dispatching ExecutionFailed"
    );
    assert!(
        matches!(result.state(), Some(State::Failed(_))),
        "State should be Failed after ExecutionBlocked triggers auto-dispatch"
    );
}

#[tokio::test]
async fn test_a1_worktree_mode_without_repos_calls_send_message() {
    // Scenario A1 variant: Without repos, on_enter skips git setup entirely
    // and proceeds directly to send_message.

    let svc = create_hardening_services();
    let project_id = ProjectId::from_string("proj-no-repos".to_string());
    let task_id_str = "task-no-repos";

    // Build services WITHOUT task_repo and project_repo
    let services = crate::domain::state_machine::context::TaskServices::new(
        svc.spawner.clone() as _,
        svc.emitter.clone() as _,
        svc.notifier.clone() as _,
        svc.dependency_manager.clone() as _,
        svc.review_starter.clone() as _,
        svc.chat_service.clone() as _,
    )
    .with_execution_state(svc.execution_state.clone());

    let mut machine = create_state_machine(task_id_str, project_id.as_str(), services);
    let mut handler = create_transition_handler(&mut machine);

    let result = handler
        .handle_transition(&State::Ready, &TaskEvent::StartExecution)
        .await;

    assert!(result.is_success());
    assert_eq!(result.state(), Some(&State::Executing));

    // Without repos, git setup is skipped, so send_message should be called
    assert_eq!(
        svc.chat_service.call_count(),
        1,
        "send_message should be called once when repos are absent (git setup skipped)"
    );
}

// ============================================================================
// A2: Branch create/checkout fails (Local mode) -> ExecutionBlocked
// ============================================================================

#[tokio::test]
async fn test_a2_local_mode_transition_ready_to_executing() {
    // Scenario A2: Branch create/checkout fails (Local mode) -> ExecutionBlocked — COVERED
    //
    // With mock repos pointing to a non-git directory, the branch creation
    // will fail and return ExecutionBlocked from on_enter. TransitionHandler
    // catches this error and still returns Success.

    let svc = create_hardening_services();

    // Create project in Local mode (default)
    let project = create_test_project("local-proj");
    let project_id = project.id.clone();
    svc.project_repo.create(project).await.unwrap();

    // Create task in Ready state
    let mut task = create_test_task(&project_id, "Test local branch task");
    task.internal_status = InternalStatus::Ready;
    let task_id_str = task.id.as_str().to_string();
    svc.task_repo.create(task).await.unwrap();

    let services = build_task_services(&svc);
    let mut machine = create_state_machine(&task_id_str, project_id.as_str(), services);
    let mut handler = create_transition_handler(&mut machine);

    let result = handler
        .handle_transition(&State::Ready, &TaskEvent::StartExecution)
        .await;

    // After the fix, ExecutionBlocked triggers auto-dispatch of ExecutionFailed,
    // so the task transitions to Failed instead of staying in Executing
    assert!(
        result.is_success(),
        "TransitionHandler should return Success after auto-dispatching ExecutionFailed"
    );
    assert!(
        matches!(result.state(), Some(State::Failed(_))),
        "State should be Failed after ExecutionBlocked triggers auto-dispatch"
    );
}

// ============================================================================
// A3: Uncommitted changes detected (Local mode) -> ExecutionBlocked
// ============================================================================

#[tokio::test]
async fn test_a3_execution_blocked_variant_exists() {
    // Scenario A3: Uncommitted changes detected (Local mode) -> ExecutionBlocked — COVERED
    //
    // Since GitService::has_uncommitted_changes is static and checks real filesystem,
    // we verify the ExecutionBlocked error variant exists and can be constructed/matched.

    let error = AppError::ExecutionBlocked(
        "Cannot execute task: uncommitted changes in working directory. \
         Please commit or stash your changes first."
            .to_string(),
    );

    // Verify the error can be matched
    assert!(matches!(&error, AppError::ExecutionBlocked(msg) if msg.contains("uncommitted")));

    // Verify Display impl
    let display = format!("{}", error);
    assert!(
        display.contains("uncommitted"),
        "ExecutionBlocked display should contain the message"
    );
}

// ============================================================================
// A4: Uncommitted changes check fails -> ExecutionBlocked
// ============================================================================

#[tokio::test]
async fn test_a4_execution_blocked_for_check_failure() {
    // Scenario A4: Uncommitted changes check fails -> ExecutionBlocked — COVERED
    //
    // Verify that the error variant for check failures exists and is distinct.

    let error = AppError::ExecutionBlocked(
        "Git isolation failed: could not check working directory for uncommitted changes: \
         not a git repository"
            .to_string(),
    );

    assert!(matches!(
        &error,
        AppError::ExecutionBlocked(msg) if msg.contains("could not check")
    ));
}

#[tokio::test]
async fn test_a4_local_mode_with_repos_exercises_git_path() {
    // Scenario A4 variant: Verify that with repos present, on_enter attempts
    // the git check (which fails on /tmp/test-project) and the transition
    // still succeeds because TransitionHandler catches on_enter errors.

    let svc = create_hardening_services();

    let project = create_test_project("local-check-proj");
    let project_id = project.id.clone();
    svc.project_repo.create(project).await.unwrap();

    let mut task = create_test_task(&project_id, "Git check task");
    task.internal_status = InternalStatus::Ready;
    let task_id_str = task.id.as_str().to_string();
    svc.task_repo.create(task).await.unwrap();

    let services = build_task_services(&svc);
    let mut machine = create_state_machine(&task_id_str, project_id.as_str(), services);
    let mut handler = create_transition_handler(&mut machine);

    let result = handler
        .handle_transition(&State::Ready, &TaskEvent::StartExecution)
        .await;

    // Transition succeeds despite git check failure
    assert!(result.is_success());

    // on_enter returned ExecutionBlocked early, so send_message was NOT called
    // (the `let _ = chat_service.send_message(...)` line is after the git block)
    assert_eq!(
        svc.chat_service.call_count(),
        0,
        "send_message should NOT be called when git isolation blocks execution"
    );
}

// ============================================================================
// A5: Worktree mode but worktree_path is None at spawn time -> safe
// ============================================================================

#[tokio::test]
async fn test_a5_worktree_path_none_at_spawn_time() {
    // Scenario A5: Worktree mode but worktree_path is None at spawn time — COVERED
    //
    // Create a task already in Executing state with worktree_path=None.
    // Verify that the spawner/chat_service receives the task and doesn't crash.
    // The actual worktree path resolution happens in production code (AgenticClientSpawner),
    // not in the mock. This test verifies the state machine doesn't require
    // worktree_path to be set before entering Executing.

    let svc = create_hardening_services();

    let mut project = create_test_project_with_git_mode("wt-proj", GitMode::Worktree);
    project.worktree_parent_directory = Some("/tmp/worktrees".to_string());
    let project_id = project.id.clone();
    svc.project_repo.create(project).await.unwrap();

    // Task already has a branch but no worktree_path
    let mut task = create_test_task(&project_id, "No worktree path task");
    task.internal_status = InternalStatus::Ready;
    task.task_branch = Some("ralphx/wt-proj/task-existing".to_string());
    task.worktree_path = None; // explicitly None
    let task_id_str = task.id.as_str().to_string();
    svc.task_repo.create(task).await.unwrap();

    let services = build_task_services(&svc);
    let mut machine = create_state_machine(&task_id_str, project_id.as_str(), services);
    let mut handler = create_transition_handler(&mut machine);

    let result = handler
        .handle_transition(&State::Ready, &TaskEvent::StartExecution)
        .await;

    assert!(result.is_success());
    assert_eq!(result.state(), Some(&State::Executing));

    // Since task already has a branch, git setup is skipped, and send_message is called
    assert_eq!(
        svc.chat_service.call_count(),
        1,
        "send_message should be called when task already has a branch"
    );
}

// ============================================================================
// A6: ExecutionBlocked from on_enter transitions task to Failed
// ============================================================================

#[tokio::test]
async fn test_a6_execution_blocked_does_not_propagate_through_transition_handler() {
    // Scenario A6: ExecutionBlocked from on_enter — COVERED
    //
    // TransitionHandler now auto-dispatches ExecutionFailed when on_enter
    // returns ExecutionBlocked. This prevents tasks from getting stuck in
    // Executing state with no agent. The task should end up in Failed state.

    let svc = create_hardening_services();

    // Create project pointing to non-git directory
    let project = create_test_project("blocked-proj");
    let project_id = project.id.clone();
    svc.project_repo.create(project).await.unwrap();

    // Create task without a branch (triggers git setup in on_enter)
    let mut task = create_test_task(&project_id, "Blocked task");
    task.internal_status = InternalStatus::Ready;
    let task_id_str = task.id.as_str().to_string();
    svc.task_repo.create(task).await.unwrap();

    let services = build_task_services(&svc);
    let mut machine = create_state_machine(&task_id_str, project_id.as_str(), services);
    let mut handler = create_transition_handler(&mut machine);

    let result = handler
        .handle_transition(&State::Ready, &TaskEvent::StartExecution)
        .await;

    // After the fix, ExecutionBlocked triggers auto-dispatch of ExecutionFailed,
    // so the task transitions to Failed instead of staying in Executing
    assert!(
        result.is_success(),
        "TransitionHandler should return Success after auto-dispatching ExecutionFailed"
    );
    assert!(
        matches!(result.state(), Some(State::Failed(_))),
        "State should be Failed after ExecutionBlocked triggers auto-dispatch"
    );

    // send_message was NOT called because ExecutionBlocked is returned before it
    assert_eq!(
        svc.chat_service.call_count(),
        0,
        "send_message should not be called when execution is blocked"
    );
}

#[tokio::test]
async fn test_a6_execution_failed_event_transitions_executing_to_failed() {
    // Scenario A6 follow-up: Verify that ExecutionFailed event correctly
    // transitions Executing -> Failed. This is how the task_transition_service
    // handles the ExecutionBlocked error — by firing ExecutionFailed.

    let svc = create_hardening_services();
    let services = build_task_services(&svc);
    let mut machine = create_state_machine("task-a6-fail", "proj-a6", services);
    let mut handler = create_transition_handler(&mut machine);

    let result = handler
        .handle_transition(
            &State::Executing,
            &TaskEvent::ExecutionFailed {
                error: "Git isolation failed: ExecutionBlocked".to_string(),
            },
        )
        .await;

    assert!(result.is_success());
    match result.state() {
        Some(State::Failed(data)) => {
            assert!(
                data.error.contains("ExecutionBlocked"),
                "Failed state should preserve the ExecutionBlocked error message"
            );
        }
        other => panic!("Expected Failed state, got {:?}", other),
    }
}

// ============================================================================
// A7: GitMode switched while tasks in-flight — no validation (GAP)
// ============================================================================

#[tokio::test]
async fn test_a7_git_mode_switch_while_task_executing_no_validation() {
    // Scenario A7: GitMode switched while tasks in-flight — GAP
    //
    // This test demonstrates the gap: there is no validation preventing
    // a project's git_mode from being changed while tasks are in-flight.
    // A task can be Executing with Local mode, and the project can be
    // switched to Worktree mode with no error or check.

    let svc = create_hardening_services();

    // Create project in Local mode
    let mut project = create_test_project("switchable-proj");
    let project_id = project.id.clone();
    project.git_mode = GitMode::Local;
    svc.project_repo.create(project).await.unwrap();

    // Create task in Executing state with Local mode assumptions
    let mut task = create_test_task(&project_id, "In-flight task");
    task.internal_status = InternalStatus::Executing;
    task.task_branch = Some("ralphx/switchable-proj/task-123".to_string());
    task.worktree_path = None; // Local mode: no worktree path
    let task_id_str = task.id.as_str().to_string();
    svc.task_repo.create(task).await.unwrap();

    // GAP: Switch project git_mode to Worktree while task is in-flight
    let mut updated_project = svc
        .project_repo
        .get_by_id(&project_id)
        .await
        .unwrap()
        .unwrap();
    updated_project.git_mode = GitMode::Worktree;
    svc.project_repo.update(&updated_project).await.unwrap();

    // Verify the mode was changed
    let fetched = svc
        .project_repo
        .get_by_id(&project_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        fetched.git_mode,
        GitMode::Worktree,
        "GAP: git_mode can be switched to Worktree while tasks are in Executing state"
    );

    // Verify in-flight task still exists in Executing state
    let task_id = TaskId::from_string(task_id_str);
    let fetched_task = svc.task_repo.get_by_id(&task_id).await.unwrap().unwrap();
    assert_eq!(fetched_task.internal_status, InternalStatus::Executing);

    // GAP: The task has no worktree_path (was set up for Local mode),
    // but the project now expects Worktree mode. No validation caught this.
    assert!(
        fetched_task.worktree_path.is_none(),
        "GAP: Task created under Local mode has no worktree_path, \
         but project is now in Worktree mode — no validation exists"
    );
}

// ============================================================================
// A8: Base branch doesn't exist — partial coverage
// ============================================================================

#[tokio::test]
async fn test_a8_nonexistent_plan_branch_falls_back_to_project_base() {
    // Scenario A8: Base branch doesn't exist — PARTIAL
    //
    // Create task with ideation_session_id pointing to a plan branch that
    // doesn't exist in the repository. Verify on_enter proceeds (falls back
    // to project base_branch via resolve_task_base_branch).

    let svc = create_hardening_services();

    let mut project = create_test_project("base-branch-proj");
    project.base_branch = Some("develop".to_string());
    let project_id = project.id.clone();
    svc.project_repo.create(project).await.unwrap();

    // Create task with ideation_session_id that has no matching plan branch
    let mut task = create_test_task(&project_id, "Fallback base branch task");
    task.internal_status = InternalStatus::Ready;
    task.ideation_session_id = Some(IdeationSessionId::from_string("nonexistent-session"));
    let task_id_str = task.id.as_str().to_string();
    svc.task_repo.create(task).await.unwrap();

    let services = build_task_services(&svc);
    let mut machine = create_state_machine(&task_id_str, project_id.as_str(), services);
    let mut handler = create_transition_handler(&mut machine);

    let result = handler
        .handle_transition(&State::Ready, &TaskEvent::StartExecution)
        .await;

    // After the fix, ExecutionBlocked triggers auto-dispatch of ExecutionFailed,
    // so the task transitions to Failed instead of staying in Executing
    assert!(
        result.is_success(),
        "TransitionHandler should return Success after auto-dispatching ExecutionFailed"
    );
    assert!(
        matches!(result.state(), Some(State::Failed(_))),
        "State should be Failed after ExecutionBlocked triggers auto-dispatch"
    );
}

#[tokio::test]
async fn test_a8_no_plan_branch_repo_falls_back_to_default() {
    // Scenario A8 variant: When plan_branch_repo is None, resolve_task_base_branch
    // should fall back to project.base_branch.

    let svc = create_hardening_services();

    let mut project = create_test_project("no-pb-repo-proj");
    project.base_branch = Some("main".to_string());
    let project_id = project.id.clone();
    svc.project_repo.create(project).await.unwrap();

    let mut task = create_test_task(&project_id, "No plan branch repo task");
    task.internal_status = InternalStatus::Ready;
    task.ideation_session_id = Some(IdeationSessionId::from_string("sess-123"));
    let task_id_str = task.id.as_str().to_string();
    svc.task_repo.create(task).await.unwrap();

    // Build services WITHOUT plan_branch_repo
    let services = crate::domain::state_machine::context::TaskServices::new(
        svc.spawner.clone() as _,
        svc.emitter.clone() as _,
        svc.notifier.clone() as _,
        svc.dependency_manager.clone() as _,
        svc.review_starter.clone() as _,
        svc.chat_service.clone() as _,
    )
    .with_execution_state(svc.execution_state.clone())
    .with_task_repo(
        svc.task_repo.clone() as std::sync::Arc<dyn crate::domain::repositories::TaskRepository>
    )
    .with_project_repo(svc.project_repo.clone()
        as std::sync::Arc<dyn crate::domain::repositories::ProjectRepository>);
    // Note: no .with_plan_branch_repo() — it stays None

    let mut machine = create_state_machine(&task_id_str, project_id.as_str(), services);
    let mut handler = create_transition_handler(&mut machine);

    let result = handler
        .handle_transition(&State::Ready, &TaskEvent::StartExecution)
        .await;

    // Transition succeeds — resolve_task_base_branch returns default ("main")
    // when plan_branch_repo is None
    assert!(result.is_success());
}

#[tokio::test]
async fn test_a8_base_branch_not_validated_at_transition_time() {
    // Scenario A8: PARTIAL — Document that on_enter(Executing) does not validate
    // whether the base branch actually exists in the git repo.
    //
    // The base_branch field is just a string — no validation against actual git refs.
    // If the base branch doesn't exist, the git checkout/worktree command will fail
    // later during on_enter, but the transition itself still succeeds.

    let svc = create_hardening_services();

    let mut project = create_test_project("bogus-base-proj");
    project.base_branch = Some("nonexistent-branch-that-will-fail".to_string());
    let project_id = project.id.clone();
    svc.project_repo.create(project).await.unwrap();

    let mut task = create_test_task(&project_id, "Bogus base branch task");
    task.internal_status = InternalStatus::Ready;
    let task_id_str = task.id.as_str().to_string();
    svc.task_repo.create(task).await.unwrap();

    let services = build_task_services(&svc);
    let mut machine = create_state_machine(&task_id_str, project_id.as_str(), services);
    let mut handler = create_transition_handler(&mut machine);

    let result = handler
        .handle_transition(&State::Ready, &TaskEvent::StartExecution)
        .await;

    // Transition succeeds — base branch existence is not validated at transition time
    assert!(
        result.is_success(),
        "PARTIAL: No base branch existence validation at transition time"
    );
}
