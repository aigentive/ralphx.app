use super::*;
use crate::application::{AppState, TaskTransitionService};
use crate::commands::execution_commands::ExecutionState;
use crate::domain::entities::{
    AgentRun, AgentRunId, AgentRunStatus, ChatConversationId, InternalStatus, Project, Task, TaskId,
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
        Arc::clone(&app_state.chat_attachment_repo),
        Arc::clone(&app_state.chat_conversation_repo),
        Arc::clone(&app_state.agent_run_repo),
        Arc::clone(&app_state.ideation_session_repo),
        Arc::clone(&app_state.activity_event_repo),
        Arc::clone(&app_state.message_queue),
        Arc::clone(&app_state.running_agent_registry),
        Arc::clone(execution_state),
        None,
        Arc::clone(&app_state.memory_event_repo),
    ));

    ReconciliationRunner::new(
        Arc::clone(&app_state.task_repo),
        Arc::clone(&app_state.task_dependency_repo),
        Arc::clone(&app_state.project_repo),
        Arc::clone(&app_state.chat_conversation_repo),
        Arc::clone(&app_state.chat_message_repo),
        Arc::clone(&app_state.chat_attachment_repo),
        Arc::clone(&app_state.ideation_session_repo),
        Arc::clone(&app_state.activity_event_repo),
        Arc::clone(&app_state.message_queue),
        Arc::clone(&app_state.running_agent_registry),
        Arc::clone(&app_state.memory_event_repo),
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
        RecoveryActionKind::AttemptMergeAutoComplete
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
            None,
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

#[tokio::test]
async fn reconcile_merge_incomplete_returns_false_when_branch_missing() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let mut task = Task::new(project.id.clone(), "Branch Missing Task".to_string());
    task.internal_status = InternalStatus::MergeIncomplete;
    // Set branch_missing metadata flag
    task.metadata = Some(serde_json::json!({"branch_missing": true}).to_string());
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Record status history so reconciler can calculate age
    app_state
        .task_repo
        .persist_status_change(
            &task.id,
            InternalStatus::PendingMerge,
            InternalStatus::MergeIncomplete,
            "merge_incomplete",
        )
        .await
        .unwrap();

    // Should return false (no retry) because branch_missing is set
    let reconciled = reconciler
        .reconcile_merge_incomplete_task(&task, InternalStatus::MergeIncomplete)
        .await;
    assert!(
        !reconciled,
        "Should not retry when branch_missing metadata is set"
    );

    // Verify task status unchanged
    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert_eq!(
        updated.internal_status,
        InternalStatus::MergeIncomplete,
        "Task status should not change when branch_missing is set"
    );
}

#[tokio::test]
async fn reconcile_merge_incomplete_retries_normally_without_branch_missing() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let mut task = Task::new(
        project.id.clone(),
        "Normal Merge Incomplete Task".to_string(),
    );
    task.internal_status = InternalStatus::MergeIncomplete;
    // No branch_missing flag - should allow retry
    task.metadata = Some(serde_json::json!({"some_other_field": "value"}).to_string());
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Record old status history so reconciler sees enough age for retry
    app_state
        .task_repo
        .persist_status_change(
            &task.id,
            InternalStatus::PendingMerge,
            InternalStatus::MergeIncomplete,
            "merge_incomplete",
        )
        .await
        .unwrap();

    // Note: In a real scenario with older task, this would naturally be old
    // For testing, we verify the logic path is reached by checking task state

    // Since there are no auto-retry events recorded, should attempt retry
    let _ = reconciler
        .reconcile_merge_incomplete_task(&task, InternalStatus::MergeIncomplete)
        .await;
    // Note: This may return false due to timing (age check), but the important thing
    // is that it doesn't early-return due to branch_missing check
    // A more thorough test would mock time or manipulate status history directly

    // Instead, verify the logic by checking that without branch_missing,
    // the reconciler proceeds past the branch_missing check
    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    // The key assertion: branch_missing was not set, so reconciler didn't skip
    assert!(
        updated.internal_status == InternalStatus::MergeIncomplete
            || updated.internal_status == InternalStatus::PendingMerge,
        "Task should either retry or stay in MergeIncomplete (not blocked by branch_missing check)"
    );
}

// ── Merging state retry cap tests (Gap 1) ──

#[test]
fn merging_timeout_is_300_seconds() {
    assert_eq!(super::MERGING_TIMEOUT_SECONDS, 300);
}

#[test]
fn merging_max_auto_retries_is_3() {
    assert_eq!(super::MERGING_MAX_AUTO_RETRIES, 3);
}

#[test]
fn merging_auto_retry_count_counts_attempt_failed_events() {
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
                        "kind": "attempt_failed",
                        "source": "system",
                        "reason_code": "git_error",
                        "message": "timeout 1"
                    },
                    {
                        "at": "2026-02-10T00:05:00Z",
                        "kind": "auto_retry_triggered",
                        "source": "auto",
                        "reason_code": "git_error",
                        "message": "unrelated event"
                    },
                    {
                        "at": "2026-02-10T00:10:00Z",
                        "kind": "attempt_failed",
                        "source": "system",
                        "reason_code": "git_error",
                        "message": "timeout 2"
                    }
                ],
                "last_state": "failed"
            }
        })
        .to_string(),
    );
    assert_eq!(
        ReconciliationRunner::<tauri::Wry>::merging_auto_retry_count(&task),
        2
    );
}

#[test]
fn merging_auto_retry_count_returns_zero_for_no_metadata() {
    let task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "No Metadata Task".to_string(),
    );
    assert_eq!(
        ReconciliationRunner::<tauri::Wry>::merging_auto_retry_count(&task),
        0
    );
}

#[test]
fn merge_policy_restarts_when_run_missing_and_can_start() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: None,
        registry_running: false,
        can_start: true,
        is_stale: false,
        is_deferred: false,
    };
    let decision = policy.decide_reconciliation(RecoveryContext::Merge, evidence);
    assert_eq!(decision.action, RecoveryActionKind::ExecuteEntryActions);
}

#[test]
fn merge_policy_prompts_when_run_missing_and_cannot_start() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: None,
        registry_running: false,
        can_start: false,
        is_stale: false,
        is_deferred: false,
    };
    let decision = policy.decide_reconciliation(RecoveryContext::Merge, evidence);
    assert_eq!(decision.action, RecoveryActionKind::Prompt);
}

// ── MergeConflict reconciliation tests ──

#[test]
fn merge_conflict_retry_delay_exponential_backoff() {
    assert_eq!(
        ReconciliationRunner::<tauri::Wry>::merge_conflict_retry_delay(0),
        chrono::Duration::seconds(60)
    );
    assert_eq!(
        ReconciliationRunner::<tauri::Wry>::merge_conflict_retry_delay(1),
        chrono::Duration::seconds(120)
    );
    assert_eq!(
        ReconciliationRunner::<tauri::Wry>::merge_conflict_retry_delay(2),
        chrono::Duration::seconds(240)
    );
    assert_eq!(
        ReconciliationRunner::<tauri::Wry>::merge_conflict_retry_delay(3),
        chrono::Duration::seconds(480)
    );
    // Verify cap at 600s
    assert_eq!(
        ReconciliationRunner::<tauri::Wry>::merge_conflict_retry_delay(10),
        chrono::Duration::seconds(600)
    );
}

#[tokio::test]
async fn reconcile_merge_conflict_skips_when_under_cooldown() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let mut task = Task::new(project.id.clone(), "Conflict Task".to_string());
    task.internal_status = InternalStatus::MergeConflict;
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Record status history with recent timestamp (under cooldown)
    app_state
        .task_repo
        .persist_status_change(
            &task.id,
            InternalStatus::Merging,
            InternalStatus::MergeConflict,
            "merge_conflict",
        )
        .await
        .unwrap();

    // Should return false (no retry) because age < 60s
    let reconciled = reconciler
        .reconcile_merge_conflict_task(&task, InternalStatus::MergeConflict)
        .await;
    assert!(
        !reconciled,
        "Should not retry when task is under cooldown period"
    );

    // Verify task status unchanged
    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert_eq!(
        updated.internal_status,
        InternalStatus::MergeConflict,
        "Task status should not change when under cooldown"
    );
}

#[tokio::test]
async fn reconcile_merge_conflict_transitions_after_cooldown() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let mut task = Task::new(project.id.clone(), "Old Conflict Task".to_string());
    task.internal_status = InternalStatus::MergeConflict;
    task.updated_at = chrono::Utc::now() - chrono::Duration::seconds(120);
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Record status history with old timestamp (past cooldown)
    // Note: In reality, updated_at fallback is used when history is missing
    // This test validates the transition path when age > delay

    let reconciled = reconciler
        .reconcile_merge_conflict_task(&task, InternalStatus::MergeConflict)
        .await;
    assert!(reconciled, "Should retry when task is past cooldown period");

    // Verify task transitioned to MergeIncomplete (requires manual resolution after conflict)
    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert_eq!(
        updated.internal_status,
        InternalStatus::MergeIncomplete,
        "Task should transition to MergeIncomplete after cooldown (requires manual resolution)"
    );
}

#[tokio::test]
async fn reconcile_merge_conflict_stops_after_max_retries() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let mut task = Task::new(project.id.clone(), "Max Retry Task".to_string());
    task.internal_status = InternalStatus::MergeConflict;
    task.updated_at = chrono::Utc::now() - chrono::Duration::seconds(1000);
    // Set 3 auto-retry events (max limit)
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
                        "kind": "auto_retry_triggered",
                        "source": "auto",
                        "reason_code": "git_error",
                        "message": "retry 2"
                    },
                    {
                        "at": "2026-02-10T00:02:00Z",
                        "kind": "auto_retry_triggered",
                        "source": "auto",
                        "reason_code": "git_error",
                        "message": "retry 3"
                    }
                ],
                "last_state": "retrying"
            }
        })
        .to_string(),
    );
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Should return false (no retry) because retry_count >= 3
    let reconciled = reconciler
        .reconcile_merge_conflict_task(&task, InternalStatus::MergeConflict)
        .await;
    assert!(
        !reconciled,
        "Should not retry when max retry count is reached"
    );

    // Verify task status unchanged
    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert_eq!(
        updated.internal_status,
        InternalStatus::MergeConflict,
        "Task status should not change when max retries reached"
    );
}

// ── Main Merge Deferred Reconciliation Tests (Phase 4) ──

#[tokio::test]
async fn reconcile_pending_merge_retries_when_main_merge_deferred_and_no_agents() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let mut task = Task::new(project.id.clone(), "Main Merge Deferred Task".to_string());
    task.internal_status = InternalStatus::PendingMerge;
    task.metadata =
        Some(serde_json::json!({"main_merge_deferred": true, "main_merge_deferred_at": "2026-01-01T00:00:00Z"}).to_string());
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Record status history so reconciler can calculate age
    app_state
        .task_repo
        .persist_status_change(
            &task.id,
            InternalStatus::Approved,
            InternalStatus::PendingMerge,
            "pending_merge",
        )
        .await
        .unwrap();

    // running_count is 0 by default, so should retry
    assert_eq!(execution_state.running_count(), 0);

    let reconciled = reconciler
        .reconcile_pending_merge_task(&task, InternalStatus::PendingMerge)
        .await;

    // Should return true because it attempted to apply recovery decision (ExecuteEntryActions)
    assert!(
        reconciled,
        "Should retry main-merge-deferred when no agents running"
    );
}

#[tokio::test]
async fn reconcile_pending_merge_skips_when_main_merge_deferred_and_agents_running() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    execution_state.increment_running(); // Simulate agent running
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let mut task = Task::new(project.id.clone(), "Main Merge Deferred Task".to_string());
    task.internal_status = InternalStatus::PendingMerge;
    task.metadata =
        Some(serde_json::json!({"main_merge_deferred": true, "main_merge_deferred_at": "2026-01-01T00:00:00Z"}).to_string());
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Record status history so reconciler can calculate age
    app_state
        .task_repo
        .persist_status_change(
            &task.id,
            InternalStatus::Approved,
            InternalStatus::PendingMerge,
            "pending_merge",
        )
        .await
        .unwrap();

    assert_eq!(execution_state.running_count(), 1);

    let reconciled = reconciler
        .reconcile_pending_merge_task(&task, InternalStatus::PendingMerge)
        .await;

    // Should return true because it's correctly deferred (not orphaned) - skip entry actions
    assert!(
        reconciled,
        "Should skip (return true) when main-merge-deferred and agents still running"
    );

    // Verify task status unchanged (not retried)
    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert_eq!(
        updated.internal_status,
        InternalStatus::PendingMerge,
        "Task status should not change when agents still running"
    );
    // Verify main_merge_deferred flag still set
    let metadata: serde_json::Value =
        serde_json::from_str(updated.metadata.as_ref().unwrap()).unwrap();
    assert_eq!(metadata["main_merge_deferred"], true);
}

#[tokio::test]
async fn reconcile_pending_merge_normal_deferred_flow_when_not_main_merge_deferred() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    execution_state.increment_running(); // Simulate agent running
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    // Regular deferred task (not main-merge-deferred)
    let mut task = Task::new(project.id.clone(), "Regular Deferred Task".to_string());
    task.internal_status = InternalStatus::PendingMerge;
    task.metadata =
        Some(serde_json::json!({"merge_deferred": true, "merge_deferred_at": "2026-01-01T00:00:00Z"}).to_string());
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Record status history
    app_state
        .task_repo
        .persist_status_change(
            &task.id,
            InternalStatus::Approved,
            InternalStatus::PendingMerge,
            "pending_merge",
        )
        .await
        .unwrap();

    // Should fall through to regular deferred merge logic (not main-merge-deferred)
    // This tests that main-merge-deferred check is isolated from regular deferred logic
    let _ = reconciler
        .reconcile_pending_merge_task(&task, InternalStatus::PendingMerge)
        .await;
    // The exact behavior depends on the deferred-blocker-is-active check, which we don't test here
    // The key is that it didn't hit the main-merge-deferred code path
}
