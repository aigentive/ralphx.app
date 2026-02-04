use super::*;
use crate::application::{AppState, TaskTransitionService};
use crate::commands::execution_commands::ExecutionState;
use crate::domain::entities::{InternalStatus, Project, Task};
use std::sync::Arc;

fn build_reconciler(
    app_state: &AppState,
    execution_state: &Arc<ExecutionState>,
) -> ReconciliationRunner<tauri::Wry> {
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

    ReconciliationRunner::new(
        Arc::clone(&app_state.task_repo),
        Arc::clone(&app_state.task_dependency_repo),
        Arc::clone(&app_state.project_repo),
        Arc::clone(&app_state.chat_conversation_repo),
        Arc::clone(&app_state.chat_message_repo),
        Arc::clone(&app_state.ideation_session_repo),
        Arc::clone(&app_state.activity_event_repo),
        Arc::clone(&app_state.message_queue),
        Arc::clone(&app_state.running_agent_registry),
        Arc::clone(&app_state.agent_run_repo),
        transition_service,
        Arc::clone(execution_state),
        None,
    )
}

#[test]
fn execution_policy_advances_on_completed_run() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: Some(AgentRunStatus::Completed),
        registry_running: false,
        can_start: true,
        is_stale: false,
    };

    let decision = policy.decide_reconciliation(RecoveryContext::Execution, evidence);
    assert_eq!(
        decision.action,
        RecoveryActionKind::Transition(InternalStatus::PendingReview)
    );
}

#[test]
fn execution_policy_restarts_when_run_missing() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: None,
        registry_running: false,
        can_start: true,
        is_stale: false,
    };

    let decision = policy.decide_reconciliation(RecoveryContext::Execution, evidence);
    assert_eq!(decision.action, RecoveryActionKind::ExecuteEntryActions);
}

#[test]
fn execution_policy_prompts_on_conflict() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: Some(AgentRunStatus::Running),
        registry_running: false,
        can_start: true,
        is_stale: false,
    };

    let decision = policy.decide_reconciliation(RecoveryContext::Execution, evidence);
    assert_eq!(decision.action, RecoveryActionKind::Prompt);
}

#[test]
fn review_policy_restarts_on_completed_run() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: Some(AgentRunStatus::Completed),
        registry_running: false,
        can_start: true,
        is_stale: false,
    };

    let decision = policy.decide_reconciliation(RecoveryContext::Review, evidence);
    assert_eq!(decision.action, RecoveryActionKind::ExecuteEntryActions);
}

#[test]
fn merge_policy_verifies_on_completed_run() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: Some(AgentRunStatus::Completed),
        registry_running: false,
        can_start: true,
        is_stale: false,
    };

    let decision = policy.decide_reconciliation(RecoveryContext::Merge, evidence);
    assert_eq!(decision.action, RecoveryActionKind::AttemptMergeAutoComplete);
}

#[test]
fn qa_policy_retries_when_stale() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: None,
        registry_running: false,
        can_start: true,
        is_stale: true,
    };

    let decision = policy.decide_reconciliation(RecoveryContext::QaTesting, evidence);
    assert_eq!(decision.action, RecoveryActionKind::ExecuteEntryActions);
}

#[test]
fn stop_policy_resets_when_not_completed() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: Some(AgentRunStatus::Running),
        registry_running: true,
        can_start: true,
        is_stale: false,
    };

    let decision = policy.decide_execution_stop(evidence);
    assert_eq!(
        decision.action,
        RecoveryActionKind::Transition(InternalStatus::Ready)
    );
}

#[tokio::test]
async fn recover_execution_stop_noops_for_paused_task() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "Paused Task".to_string());
    task.internal_status = InternalStatus::Paused;
    app_state.task_repo.create(task.clone()).await.unwrap();

    let recovered = reconciler.recover_execution_stop(&task.id).await;
    assert!(!recovered, "Paused tasks should not be recovered on stop");
}

#[tokio::test]
async fn recover_execution_stop_noops_for_stopped_task() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "Stopped Task".to_string());
    task.internal_status = InternalStatus::Stopped;
    app_state.task_repo.create(task.clone()).await.unwrap();

    let recovered = reconciler.recover_execution_stop(&task.id).await;
    assert!(!recovered, "Stopped tasks should not be recovered on stop");
}
