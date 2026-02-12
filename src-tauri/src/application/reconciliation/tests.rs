use super::*;
use crate::application::{AppState, TaskTransitionService};
use crate::commands::execution_commands::ExecutionState;
use crate::domain::entities::{
    AgentRun, AgentRunId, AgentRunStatus, ChatConversationId, InternalStatus, Project, Task,
    TaskId,
};
use crate::domain::services::RunningAgentKey;
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
        is_deferred: false,
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
        is_deferred: false,
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
        is_deferred: false,
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
        is_deferred: false,
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
        is_deferred: false,
    };

    let decision = policy.decide_reconciliation(RecoveryContext::Merge, evidence);
    assert_eq!(
        decision.action,
        RecoveryActionKind::AttemptMergeAutoComplete
    );
}

#[test]
fn merge_policy_times_out_when_stale() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: Some(AgentRunStatus::Running),
        registry_running: true,
        can_start: true,
        is_stale: true,
        is_deferred: false,
    };

    let decision = policy.decide_reconciliation(RecoveryContext::Merge, evidence);
    assert_eq!(
        decision.action,
        RecoveryActionKind::Transition(InternalStatus::MergeIncomplete)
    );
}

#[test]
fn qa_policy_retries_when_stale() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: None,
        registry_running: false,
        can_start: true,
        is_stale: true,
        is_deferred: false,
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
        is_deferred: false,
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
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

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
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let mut task = Task::new(project.id.clone(), "Stopped Task".to_string());
    task.internal_status = InternalStatus::Stopped;
    app_state.task_repo.create(task.clone()).await.unwrap();

    let recovered = reconciler.recover_execution_stop(&task.id).await;
    assert!(!recovered, "Stopped tasks should not be recovered on stop");
}

#[test]
fn pending_merge_policy_noop_when_not_stale() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: None,
        registry_running: false,
        can_start: true,
        is_stale: false,
        is_deferred: false,
    };

    let decision = policy.decide_reconciliation(RecoveryContext::PendingMerge, evidence);
    assert_eq!(decision.action, RecoveryActionKind::None);
}

#[test]
fn pending_merge_policy_retriggers_when_stale_and_deferred() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: None,
        registry_running: false,
        can_start: true,
        is_stale: true,
        is_deferred: true,
    };

    let decision = policy.decide_reconciliation(RecoveryContext::PendingMerge, evidence);
    assert_eq!(decision.action, RecoveryActionKind::ExecuteEntryActions);
}

#[test]
fn pending_merge_policy_transitions_to_merge_incomplete_when_stale_not_deferred() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: None,
        registry_running: false,
        can_start: true,
        is_stale: true,
        is_deferred: false,
    };

    let decision = policy.decide_reconciliation(RecoveryContext::PendingMerge, evidence);
    assert_eq!(
        decision.action,
        RecoveryActionKind::Transition(InternalStatus::MergeIncomplete)
    );
}

#[test]
fn pending_merge_deferred_waits_when_not_stale() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: None,
        registry_running: false,
        can_start: true,
        is_stale: false,
        is_deferred: true,
    };

    let decision = policy.decide_reconciliation(RecoveryContext::PendingMerge, evidence);
    assert_eq!(decision.action, RecoveryActionKind::None);
}

#[test]
fn merge_incomplete_retry_delay_uses_exponential_backoff_and_cap() {
    assert_eq!(
        ReconciliationRunner::<tauri::Wry>::merge_incomplete_retry_delay(0),
        chrono::Duration::seconds(30)
    );
    assert_eq!(
        ReconciliationRunner::<tauri::Wry>::merge_incomplete_retry_delay(1),
        chrono::Duration::seconds(60)
    );
    assert_eq!(
        ReconciliationRunner::<tauri::Wry>::merge_incomplete_retry_delay(2),
        chrono::Duration::seconds(120)
    );
    assert_eq!(
        ReconciliationRunner::<tauri::Wry>::merge_incomplete_retry_delay(10),
        chrono::Duration::seconds(300)
    );
}

#[test]
fn merge_incomplete_retry_count_reads_auto_retry_events() {
    let mut task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "Retry Count Task".to_string(),
    );
    task.metadata = Some(
        serde_json::json!({
            "merge_recovery": {
                "version": 1,
                "events": [
                    {
                        "at": "2026-02-10T00:00:00Z",
                        "kind": "auto_retry_triggered",
                        "source": "auto",
                        "reason_code": "git_error",
                        "message": "retry 1"
                    },
                    {
                        "at": "2026-02-10T00:01:00Z",
                        "kind": "manual_retry",
                        "source": "user",
                        "reason_code": "git_error",
                        "message": "manual"
                    },
                    {
                        "at": "2026-02-10T00:02:00Z",
                        "kind": "auto_retry_triggered",
                        "source": "auto",
                        "reason_code": "git_error",
                        "message": "retry 2"
                    }
                ],
                "last_state": "retrying"
            }
        })
        .to_string(),
    );

    assert_eq!(
        ReconciliationRunner::<tauri::Wry>::merge_incomplete_auto_retry_count(&task),
        2
    );
}

#[test]
fn merge_incomplete_non_retryable_reason_detects_unresolvable_source_branch_error() {
    let mut task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "Merge Incomplete".to_string(),
    );
    task.metadata = Some(
        serde_json::json!({
            "error": "Empty source branch resolved. This typically means plan_branch_repo was unavailable."
        })
        .to_string(),
    );

    let reason = ReconciliationRunner::<tauri::Wry>::merge_incomplete_non_retryable_reason_from_error(&task);
    assert!(reason.is_some());
    assert!(
        reason.unwrap().contains("source branch"),
        "expected source branch reason"
    );
}

#[test]
fn merge_incomplete_non_retryable_reason_detects_missing_source_branch_reason_code() {
    let mut task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "Merge Incomplete".to_string(),
    );
    task.metadata = Some(
        serde_json::json!({
            "merge_recovery_reason": "missing_source_branch"
        })
        .to_string(),
    );

    let reason =
        ReconciliationRunner::<tauri::Wry>::merge_incomplete_non_retryable_reason_from_error(&task);
    assert!(reason.is_some());
    assert!(
        reason.unwrap().contains("source branch"),
        "expected missing source branch reason"
    );
}

#[test]
fn latest_deferred_blocker_id_reads_latest_blocker_from_metadata() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let blocker_1 = TaskId::new();
    let blocker_2 = TaskId::new();

    let mut task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "Deferred Task".to_string(),
    );
    task.metadata = Some(
        serde_json::json!({
            "merge_recovery": {
                "version": 1,
                "events": [
                    {
                        "at": "2026-02-10T00:00:00Z",
                        "kind": "deferred",
                        "source": "system",
                        "reason_code": "target_branch_busy",
                        "message": "deferred 1",
                        "blocking_task_id": blocker_1.as_str()
                    },
                    {
                        "at": "2026-02-10T00:01:00Z",
                        "kind": "deferred",
                        "source": "system",
                        "reason_code": "target_branch_busy",
                        "message": "deferred 2",
                        "blocking_task_id": blocker_2.as_str()
                    }
                ],
                "last_state": "deferred"
            }
        })
        .to_string(),
    );

    assert_eq!(
        reconciler.latest_deferred_blocker_id(&task),
        Some(blocker_2)
    );
}

#[tokio::test]
async fn latest_status_transition_age_falls_back_to_updated_at_when_history_missing() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let mut task = Task::new(project.id.clone(), "No History".to_string());
    task.internal_status = InternalStatus::MergeIncomplete;
    task.updated_at = chrono::Utc::now() - chrono::Duration::minutes(12);
    app_state.task_repo.create(task.clone()).await.unwrap();

    let age = reconciler
        .latest_status_transition_age(&task, InternalStatus::MergeIncomplete)
        .await
        .expect("age should be available via fallback");

    assert!(
        age >= chrono::Duration::minutes(11),
        "expected fallback age from updated_at, got {:?}",
        age
    );
}

#[tokio::test]
async fn reconcile_stuck_tasks_prunes_stale_registry_for_terminal_task() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    // Task already moved to terminal state, but runtime registry still has an old TaskExecution run.
    let mut task = Task::new(project.id.clone(), "Terminal Task".to_string());
    task.internal_status = InternalStatus::Merged;
    app_state.task_repo.create(task.clone()).await.unwrap();

    let run = AgentRun::new(ChatConversationId::new());
    let run_id = run.id;
    app_state.agent_run_repo.create(run).await.unwrap();

    let key = RunningAgentKey::new("task_execution", task.id.as_str());
    app_state
        .running_agent_registry
        .register(
            key.clone(),
            999_999, // guaranteed nonexistent PID for liveness check
            "conv-stale".to_string(),
            run_id.as_str(),
            Some("/tmp/stale".to_string()),
        )
        .await;

    assert!(app_state.running_agent_registry.is_running(&key).await);
    assert_eq!(execution_state.running_count(), 0);

    reconciler.reconcile_stuck_tasks().await;

    assert!(
        !app_state.running_agent_registry.is_running(&key).await,
        "stale running_agents entry should be pruned"
    );
    assert_eq!(
        execution_state.running_count(),
        0,
        "execution running_count should be synced to cleaned registry count"
    );

    let updated = app_state
        .agent_run_repo
        .get_by_id(&AgentRunId::from_string(run_id.as_str()))
        .await
        .unwrap()
        .expect("run should still exist");
    assert_eq!(
        updated.status,
        AgentRunStatus::Cancelled,
        "stale running run should be cancelled after GC"
    );
}
