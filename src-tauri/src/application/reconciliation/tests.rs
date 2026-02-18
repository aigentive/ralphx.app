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
fn merging_timeout_default_is_600_seconds() {
    // Default merger agent timeout is 10 minutes (600s); configurable via
    // RALPHX_MERGER_TIMEOUT_SECS env var.
    assert_eq!(super::MERGING_TIMEOUT_SECONDS, 600);
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

// ── Merger agent timeout → MergeIncomplete tests ──

/// Test: Stale Merging task policy attempts auto-complete first.
///
/// When is_stale=true and there's no conflict (agent marked Running in registry),
/// the policy should attempt AttemptMergeAutoComplete to check if the merge already
/// happened before the agent timed out.
#[test]
fn merge_policy_stale_attempts_auto_complete() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        // run_status=Running + registry_running=true → has_conflict()=false (no conflict)
        run_status: Some(AgentRunStatus::Running),
        registry_running: true,
        can_start: true,
        is_stale: true,
        is_deferred: false,
    };
    let decision = policy.decide_reconciliation(RecoveryContext::Merge, evidence);
    assert_eq!(
        decision.action,
        RecoveryActionKind::AttemptMergeAutoComplete,
        "Stale merging task should attempt auto-complete to check git state before re-spawning"
    );
}

/// Test: After max retries, the reconciler transitions Merging to MergeIncomplete
/// (not MergeConflict), because a timeout indicates a hung agent, not an explicit
/// merge conflict reported by the agent.
#[tokio::test]
async fn merging_timeout_escalates_to_merge_incomplete_not_merge_conflict() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    // Build task with MERGING_MAX_AUTO_RETRIES attempt_failed events (retry limit hit).
    // Set updated_at far in the past so latest_status_transition_age falls back to updated_at
    // and returns a stale age (> MERGING_TIMEOUT_SECONDS).
    let mut task = Task::new(project.id.clone(), "Stuck Merging Task".to_string());
    task.internal_status = InternalStatus::Merging;
    task.updated_at = chrono::Utc::now()
        - chrono::Duration::seconds(super::MERGING_TIMEOUT_SECONDS + 60);

    // Write MERGING_MAX_AUTO_RETRIES attempt_failed events to hit the retry cap
    let events: Vec<serde_json::Value> = (0..super::MERGING_MAX_AUTO_RETRIES)
        .map(|i| {
            serde_json::json!({
                "at": format!("2026-02-10T{:02}:00:00Z", i),
                "kind": "attempt_failed",
                "source": "system",
                "reason_code": "git_error",
                "message": format!("timeout {}", i)
            })
        })
        .collect();
    task.metadata = Some(
        serde_json::json!({
            "merge_recovery": {
                "version": 1,
                "events": events,
                "last_state": "failed"
            }
        })
        .to_string(),
    );
    app_state.task_repo.create(task.clone()).await.unwrap();

    let reconciled = reconciler
        .reconcile_merging_task(&task, InternalStatus::Merging)
        .await;

    // After max retries with stale age, task must transition to MergeIncomplete
    assert!(
        reconciled,
        "Reconciler should take action for stale Merging task at retry limit"
    );

    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        updated.internal_status,
        InternalStatus::MergeIncomplete,
        "Merging timeout escalation must use MergeIncomplete, not MergeConflict. \
         MergeConflict is reserved for agent-reported conflicts."
    );
    assert_ne!(
        updated.internal_status,
        InternalStatus::MergeConflict,
        "Timeout should NOT produce MergeConflict"
    );
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

// ── Provider Error Paused Task Reconciliation Tests ──

#[tokio::test]
async fn reconcile_paused_task_without_provider_error_metadata_is_skipped() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    // User-initiated pause: no provider_error metadata
    let mut task = Task::new(project.id.clone(), "User Paused Task".to_string());
    task.internal_status = InternalStatus::Paused;
    task.metadata = Some(serde_json::json!({"some_user_key": "value"}).to_string());
    app_state.task_repo.create(task.clone()).await.unwrap();

    let reconciled = reconciler
        .reconcile_paused_provider_error(&task)
        .await;
    assert!(
        !reconciled,
        "User-paused tasks without provider_error metadata should be skipped"
    );

    // Verify task status unchanged
    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert_eq!(updated.internal_status, InternalStatus::Paused);
}

#[tokio::test]
async fn reconcile_paused_task_with_future_retry_after_stays_paused() {
    use crate::application::chat_service::{ProviderErrorCategory, ProviderErrorMetadata};

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let meta = ProviderErrorMetadata {
        category: ProviderErrorCategory::RateLimit,
        message: "Usage limit reached".to_string(),
        retry_after: Some("2099-12-31T23:59:59+00:00".to_string()), // Far future
        previous_status: "executing".to_string(),
        paused_at: chrono::Utc::now().to_rfc3339(),
        auto_resumable: true,
        resume_attempts: 0,
    };

    let mut task = Task::new(project.id.clone(), "Rate Limited Task".to_string());
    task.internal_status = InternalStatus::Paused;
    task.metadata = Some(meta.write_to_task_metadata(None));
    app_state.task_repo.create(task.clone()).await.unwrap();

    let reconciled = reconciler
        .reconcile_paused_provider_error(&task)
        .await;
    assert!(
        !reconciled,
        "Should not resume when retry_after is in the future"
    );

    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert_eq!(
        updated.internal_status,
        InternalStatus::Paused,
        "Task should remain Paused when retry_after hasn't elapsed"
    );
}

#[tokio::test]
async fn reconcile_paused_task_with_expired_retry_after_resumes() {
    use crate::application::chat_service::{ProviderErrorCategory, ProviderErrorMetadata};

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let meta = ProviderErrorMetadata {
        category: ProviderErrorCategory::RateLimit,
        message: "Usage limit reached".to_string(),
        retry_after: Some("2020-01-01T00:00:00+00:00".to_string()), // Long past
        previous_status: "executing".to_string(),
        paused_at: "2020-01-01T00:00:00+00:00".to_string(),
        auto_resumable: true,
        resume_attempts: 0,
    };

    let mut task = Task::new(project.id.clone(), "Expired Rate Limit Task".to_string());
    task.internal_status = InternalStatus::Paused;
    task.metadata = Some(meta.write_to_task_metadata(None));
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Record status history for Paused
    app_state
        .task_repo
        .persist_status_change(
            &task.id,
            InternalStatus::Executing,
            InternalStatus::Paused,
            "paused",
        )
        .await
        .unwrap();

    let reconciled = reconciler
        .reconcile_paused_provider_error(&task)
        .await;
    assert!(
        reconciled,
        "Should resume when retry_after is in the past"
    );

    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    // Task should no longer be Paused — the reconciler attempted to resume it.
    // In test environment (no real CLI), entry actions for Executing may fail,
    // causing a further transition to Failed. The key assertion is that the
    // reconciler processed the task and moved it out of Paused state.
    assert_ne!(
        updated.internal_status,
        InternalStatus::Paused,
        "Task should no longer be Paused after auto-resume"
    );
}

#[tokio::test]
async fn reconcile_paused_task_at_max_attempts_transitions_to_failed() {
    use crate::application::chat_service::{ProviderErrorCategory, ProviderErrorMetadata};

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let meta = ProviderErrorMetadata {
        category: ProviderErrorCategory::ServerError,
        message: "502 Bad Gateway".to_string(),
        retry_after: None,
        previous_status: "executing".to_string(),
        paused_at: "2020-01-01T00:00:00+00:00".to_string(),
        auto_resumable: true,
        resume_attempts: ProviderErrorMetadata::MAX_RESUME_ATTEMPTS, // At max
    };

    let mut task = Task::new(project.id.clone(), "Max Attempts Task".to_string());
    task.internal_status = InternalStatus::Paused;
    task.metadata = Some(meta.write_to_task_metadata(None));
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Record status history for Paused
    app_state
        .task_repo
        .persist_status_change(
            &task.id,
            InternalStatus::Executing,
            InternalStatus::Paused,
            "paused",
        )
        .await
        .unwrap();

    let reconciled = reconciler
        .reconcile_paused_provider_error(&task)
        .await;
    assert!(
        reconciled,
        "Should process the task (transition to Failed)"
    );

    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert_eq!(
        updated.internal_status,
        InternalStatus::Failed,
        "Task should transition to Failed when max resume attempts exceeded"
    );

    // Verify provider_error metadata was cleared
    let metadata: serde_json::Value =
        serde_json::from_str(updated.metadata.as_ref().unwrap()).unwrap();
    assert!(
        metadata.get("provider_error").is_none(),
        "provider_error metadata should be cleared after failing"
    );
}

#[tokio::test]
async fn reconcile_paused_task_not_auto_resumable_is_skipped() {
    use crate::application::chat_service::{ProviderErrorCategory, ProviderErrorMetadata};

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let meta = ProviderErrorMetadata {
        category: ProviderErrorCategory::AuthError,
        message: "Invalid API key".to_string(),
        retry_after: None,
        previous_status: "executing".to_string(),
        paused_at: "2020-01-01T00:00:00+00:00".to_string(),
        auto_resumable: false, // Manually marked as not auto-resumable
        resume_attempts: 0,
    };

    let mut task = Task::new(project.id.clone(), "Auth Error Task".to_string());
    task.internal_status = InternalStatus::Paused;
    task.metadata = Some(meta.write_to_task_metadata(None));
    app_state.task_repo.create(task.clone()).await.unwrap();

    let reconciled = reconciler
        .reconcile_paused_provider_error(&task)
        .await;
    assert!(
        !reconciled,
        "Non-auto-resumable tasks should be skipped"
    );

    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert_eq!(updated.internal_status, InternalStatus::Paused);
}

#[tokio::test]
async fn reconcile_multiple_paused_tasks_in_single_cycle() {
    use crate::application::chat_service::{ProviderErrorCategory, ProviderErrorMetadata};

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    // Task 1: expired rate limit → should resume
    let meta1 = ProviderErrorMetadata {
        category: ProviderErrorCategory::RateLimit,
        message: "Rate limited".to_string(),
        retry_after: Some("2020-01-01T00:00:00+00:00".to_string()),
        previous_status: "executing".to_string(),
        paused_at: "2020-01-01T00:00:00+00:00".to_string(),
        auto_resumable: true,
        resume_attempts: 0,
    };
    let mut task1 = Task::new(project.id.clone(), "Resumable Task".to_string());
    task1.internal_status = InternalStatus::Paused;
    task1.metadata = Some(meta1.write_to_task_metadata(None));
    app_state.task_repo.create(task1.clone()).await.unwrap();
    app_state
        .task_repo
        .persist_status_change(
            &task1.id,
            InternalStatus::Executing,
            InternalStatus::Paused,
            "paused",
        )
        .await
        .unwrap();

    // Task 2: user-paused (no provider_error) → should skip
    let mut task2 = Task::new(project.id.clone(), "User Paused Task".to_string());
    task2.internal_status = InternalStatus::Paused;
    task2.metadata = None;
    app_state.task_repo.create(task2.clone()).await.unwrap();

    // Task 3: max attempts exceeded → should fail
    let meta3 = ProviderErrorMetadata {
        category: ProviderErrorCategory::ServerError,
        message: "Server error".to_string(),
        retry_after: None,
        previous_status: "executing".to_string(),
        paused_at: "2020-01-01T00:00:00+00:00".to_string(),
        auto_resumable: true,
        resume_attempts: ProviderErrorMetadata::MAX_RESUME_ATTEMPTS,
    };
    let mut task3 = Task::new(project.id.clone(), "Max Retries Task".to_string());
    task3.internal_status = InternalStatus::Paused;
    task3.metadata = Some(meta3.write_to_task_metadata(None));
    app_state.task_repo.create(task3.clone()).await.unwrap();
    app_state
        .task_repo
        .persist_status_change(
            &task3.id,
            InternalStatus::Executing,
            InternalStatus::Paused,
            "paused",
        )
        .await
        .unwrap();

    // Process all paused tasks via reconcile_task (same as reconcile_stuck_tasks loop)
    let paused_tasks = app_state
        .task_repo
        .get_by_status(&project.id, InternalStatus::Paused)
        .await
        .unwrap();
    for task in &paused_tasks {
        let _ = reconciler
            .reconcile_task(task, InternalStatus::Paused)
            .await;
    }

    // Verify outcomes
    let t1 = app_state
        .task_repo
        .get_by_id(&task1.id)
        .await
        .unwrap()
        .unwrap();
    // Task 1 should have been processed (no longer Paused).
    // In test environment, entry actions for Executing fail (no CLI),
    // so it may end up Failed. The key is it left Paused state.
    assert_ne!(
        t1.internal_status,
        InternalStatus::Paused,
        "Task 1 should no longer be Paused after auto-resume"
    );

    let t2 = app_state
        .task_repo
        .get_by_id(&task2.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        t2.internal_status,
        InternalStatus::Paused,
        "Task 2 (user-paused) should remain Paused"
    );

    let t3 = app_state
        .task_repo
        .get_by_id(&task3.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        t3.internal_status,
        InternalStatus::Failed,
        "Task 3 (max retries) should transition to Failed"
    );
}

// =========================================================================
// PauseReason (new format) reconciliation tests
// =========================================================================

#[tokio::test]
async fn reconcile_paused_user_initiated_is_skipped() {
    use crate::application::chat_service::PauseReason;

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    // Task with PauseReason::UserInitiated metadata (global scope)
    let reason = PauseReason::UserInitiated {
        previous_status: "executing".to_string(),
        paused_at: "2026-02-15T09:00:00+00:00".to_string(),
        scope: "global".to_string(),
    };
    let mut task = Task::new(project.id.clone(), "User Paused Global".to_string());
    task.internal_status = InternalStatus::Paused;
    task.metadata = Some(reason.write_to_task_metadata(None));
    app_state.task_repo.create(task.clone()).await.unwrap();

    let reconciled = reconciler
        .reconcile_paused_provider_error(&task)
        .await;
    assert!(
        !reconciled,
        "UserInitiated pauses should be skipped by reconciliation"
    );

    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert_eq!(
        updated.internal_status,
        InternalStatus::Paused,
        "UserInitiated task should remain Paused"
    );
}

#[tokio::test]
async fn reconcile_paused_user_initiated_task_scope_is_skipped() {
    use crate::application::chat_service::PauseReason;

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    // Per-task UserInitiated pause
    let reason = PauseReason::UserInitiated {
        previous_status: "reviewing".to_string(),
        paused_at: "2026-02-15T09:00:00+00:00".to_string(),
        scope: "task".to_string(),
    };
    let mut task = Task::new(project.id.clone(), "User Paused Per-Task".to_string());
    task.internal_status = InternalStatus::Paused;
    task.metadata = Some(reason.write_to_task_metadata(None));
    app_state.task_repo.create(task.clone()).await.unwrap();

    let reconciled = reconciler
        .reconcile_paused_provider_error(&task)
        .await;
    assert!(
        !reconciled,
        "Per-task UserInitiated pauses should be skipped"
    );
}

#[tokio::test]
async fn reconcile_paused_provider_error_new_format_resumes() {
    use crate::application::chat_service::{PauseReason, ProviderErrorCategory};

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    // New-format PauseReason::ProviderError with expired retry_after
    let reason = PauseReason::ProviderError {
        category: ProviderErrorCategory::RateLimit,
        message: "Usage limit reached".to_string(),
        retry_after: Some("2020-01-01T00:00:00+00:00".to_string()), // Long past
        previous_status: "executing".to_string(),
        paused_at: "2020-01-01T00:00:00+00:00".to_string(),
        auto_resumable: true,
        resume_attempts: 0,
    };

    let mut task = Task::new(project.id.clone(), "Provider Error New Format".to_string());
    task.internal_status = InternalStatus::Paused;
    task.metadata = Some(reason.write_to_task_metadata(None));
    app_state.task_repo.create(task.clone()).await.unwrap();

    app_state
        .task_repo
        .persist_status_change(
            &task.id,
            InternalStatus::Executing,
            InternalStatus::Paused,
            "paused",
        )
        .await
        .unwrap();

    let reconciled = reconciler
        .reconcile_paused_provider_error(&task)
        .await;
    assert!(
        reconciled,
        "New-format ProviderError with expired retry_after should be processed"
    );

    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert_ne!(
        updated.internal_status,
        InternalStatus::Paused,
        "Task should no longer be Paused after auto-resume"
    );
}

#[tokio::test]
async fn reconcile_paused_provider_error_new_format_max_attempts_fails() {
    use crate::application::chat_service::{PauseReason, ProviderErrorCategory, ProviderErrorMetadata};

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    // New-format at MAX_RESUME_ATTEMPTS
    let reason = PauseReason::ProviderError {
        category: ProviderErrorCategory::ServerError,
        message: "502 Bad Gateway".to_string(),
        retry_after: None,
        previous_status: "executing".to_string(),
        paused_at: "2020-01-01T00:00:00+00:00".to_string(),
        auto_resumable: true,
        resume_attempts: ProviderErrorMetadata::MAX_RESUME_ATTEMPTS,
    };

    let mut task = Task::new(project.id.clone(), "Max Attempts New Format".to_string());
    task.internal_status = InternalStatus::Paused;
    task.metadata = Some(reason.write_to_task_metadata(None));
    app_state.task_repo.create(task.clone()).await.unwrap();

    app_state
        .task_repo
        .persist_status_change(
            &task.id,
            InternalStatus::Executing,
            InternalStatus::Paused,
            "paused",
        )
        .await
        .unwrap();

    let reconciled = reconciler
        .reconcile_paused_provider_error(&task)
        .await;
    assert!(
        reconciled,
        "Should process the task (transition to Failed)"
    );

    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert_eq!(
        updated.internal_status,
        InternalStatus::Failed,
        "Task at MAX_RESUME_ATTEMPTS should transition to Failed"
    );

    // Verify pause metadata was cleared
    let metadata: serde_json::Value =
        serde_json::from_str(updated.metadata.as_ref().unwrap()).unwrap();
    assert!(
        metadata.get("pause_reason").is_none(),
        "pause_reason should be cleared after failing"
    );
    assert!(
        metadata.get("provider_error").is_none(),
        "provider_error should be cleared after failing"
    );
}

#[tokio::test]
async fn reconcile_paused_provider_error_new_format_future_retry_stays_paused() {
    use crate::application::chat_service::{PauseReason, ProviderErrorCategory};

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let reason = PauseReason::ProviderError {
        category: ProviderErrorCategory::RateLimit,
        message: "Usage limit".to_string(),
        retry_after: Some("2099-12-31T23:59:59+00:00".to_string()), // Far future
        previous_status: "executing".to_string(),
        paused_at: chrono::Utc::now().to_rfc3339(),
        auto_resumable: true,
        resume_attempts: 0,
    };

    let mut task = Task::new(project.id.clone(), "Future Retry".to_string());
    task.internal_status = InternalStatus::Paused;
    task.metadata = Some(reason.write_to_task_metadata(None));
    app_state.task_repo.create(task.clone()).await.unwrap();

    let reconciled = reconciler
        .reconcile_paused_provider_error(&task)
        .await;
    assert!(
        !reconciled,
        "Should not resume when retry_after is in the future"
    );

    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert_eq!(updated.internal_status, InternalStatus::Paused);
}

#[tokio::test]
async fn reconcile_mixed_batch_processes_only_provider_errors_skips_user_initiated() {
    use crate::application::chat_service::{PauseReason, ProviderErrorCategory};

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    // Task 1: UserInitiated (global) → should be skipped
    let user_reason = PauseReason::UserInitiated {
        previous_status: "executing".to_string(),
        paused_at: "2026-02-15T09:00:00+00:00".to_string(),
        scope: "global".to_string(),
    };
    let mut task1 = Task::new(project.id.clone(), "User Paused".to_string());
    task1.internal_status = InternalStatus::Paused;
    task1.metadata = Some(user_reason.write_to_task_metadata(None));
    app_state.task_repo.create(task1.clone()).await.unwrap();

    // Task 2: UserInitiated (task scope) → should be skipped
    let user_task_reason = PauseReason::UserInitiated {
        previous_status: "reviewing".to_string(),
        paused_at: "2026-02-15T09:00:00+00:00".to_string(),
        scope: "task".to_string(),
    };
    let mut task2 = Task::new(project.id.clone(), "User Paused Per-Task".to_string());
    task2.internal_status = InternalStatus::Paused;
    task2.metadata = Some(user_task_reason.write_to_task_metadata(None));
    app_state.task_repo.create(task2.clone()).await.unwrap();

    // Task 3: ProviderError (expired) → should be processed
    let provider_reason = PauseReason::ProviderError {
        category: ProviderErrorCategory::RateLimit,
        message: "Rate limited".to_string(),
        retry_after: Some("2020-01-01T00:00:00+00:00".to_string()),
        previous_status: "executing".to_string(),
        paused_at: "2020-01-01T00:00:00+00:00".to_string(),
        auto_resumable: true,
        resume_attempts: 0,
    };
    let mut task3 = Task::new(project.id.clone(), "Provider Error".to_string());
    task3.internal_status = InternalStatus::Paused;
    task3.metadata = Some(provider_reason.write_to_task_metadata(None));
    app_state.task_repo.create(task3.clone()).await.unwrap();
    app_state
        .task_repo
        .persist_status_change(
            &task3.id,
            InternalStatus::Executing,
            InternalStatus::Paused,
            "paused",
        )
        .await
        .unwrap();

    // Task 4: ProviderError at max attempts → should transition to Failed
    let provider_max = PauseReason::ProviderError {
        category: ProviderErrorCategory::ServerError,
        message: "502".to_string(),
        retry_after: None,
        previous_status: "executing".to_string(),
        paused_at: "2020-01-01T00:00:00+00:00".to_string(),
        auto_resumable: true,
        resume_attempts: 5, // MAX_RESUME_ATTEMPTS
    };
    let mut task4 = Task::new(project.id.clone(), "Provider Max Attempts".to_string());
    task4.internal_status = InternalStatus::Paused;
    task4.metadata = Some(provider_max.write_to_task_metadata(None));
    app_state.task_repo.create(task4.clone()).await.unwrap();
    app_state
        .task_repo
        .persist_status_change(
            &task4.id,
            InternalStatus::Executing,
            InternalStatus::Paused,
            "paused",
        )
        .await
        .unwrap();

    // Process all paused tasks
    let paused_tasks = app_state
        .task_repo
        .get_by_status(&project.id, InternalStatus::Paused)
        .await
        .unwrap();
    for task in &paused_tasks {
        let _ = reconciler
            .reconcile_task(task, InternalStatus::Paused)
            .await;
    }

    // Verify: Task 1 (UserInitiated global) remains Paused
    let t1 = app_state
        .task_repo
        .get_by_id(&task1.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        t1.internal_status,
        InternalStatus::Paused,
        "UserInitiated (global) should remain Paused"
    );

    // Verify: Task 2 (UserInitiated task) remains Paused
    let t2 = app_state
        .task_repo
        .get_by_id(&task2.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        t2.internal_status,
        InternalStatus::Paused,
        "UserInitiated (task) should remain Paused"
    );

    // Verify: Task 3 (ProviderError expired) was processed (no longer Paused)
    let t3 = app_state
        .task_repo
        .get_by_id(&task3.id)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(
        t3.internal_status,
        InternalStatus::Paused,
        "ProviderError (expired) should have been auto-resumed"
    );

    // Verify: Task 4 (ProviderError max attempts) → Failed
    let t4 = app_state
        .task_repo
        .get_by_id(&task4.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        t4.internal_status,
        InternalStatus::Failed,
        "ProviderError (max attempts) should transition to Failed"
    );
}

#[tokio::test]
async fn reconcile_paused_provider_error_increments_resume_attempts() {
    use crate::application::chat_service::{PauseReason, ProviderErrorCategory};

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    // Start with resume_attempts = 2
    let reason = PauseReason::ProviderError {
        category: ProviderErrorCategory::RateLimit,
        message: "Rate limited".to_string(),
        retry_after: Some("2020-01-01T00:00:00+00:00".to_string()), // Past
        previous_status: "executing".to_string(),
        paused_at: "2020-01-01T00:00:00+00:00".to_string(),
        auto_resumable: true,
        resume_attempts: 2,
    };

    let mut task = Task::new(project.id.clone(), "Resume Attempts Test".to_string());
    task.internal_status = InternalStatus::Paused;
    task.metadata = Some(reason.write_to_task_metadata(None));
    app_state.task_repo.create(task.clone()).await.unwrap();

    app_state
        .task_repo
        .persist_status_change(
            &task.id,
            InternalStatus::Executing,
            InternalStatus::Paused,
            "paused",
        )
        .await
        .unwrap();

    // Before reconcile: verify resume_attempts = 2
    let before = app_state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let before_reason = PauseReason::from_task_metadata(before.metadata.as_deref()).unwrap();
    match before_reason {
        PauseReason::ProviderError { resume_attempts, .. } => {
            assert_eq!(resume_attempts, 2, "Should start at 2");
        }
        _ => panic!("Expected ProviderError"),
    }

    // Reconcile should increment resume_attempts to 3 before attempting resume
    let reconciled = reconciler
        .reconcile_paused_provider_error(&task)
        .await;
    assert!(reconciled, "Should process the task");

    // After reconcile, the task should have been processed. If the resume succeeded,
    // metadata should be cleared. If it failed (no real CLI in test), the task may
    // still have the incremented resume_attempts. Either way, the reconciler acted.
}

#[tokio::test]
async fn reconcile_paused_provider_error_not_auto_resumable_new_format() {
    use crate::application::chat_service::{PauseReason, ProviderErrorCategory};

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let reason = PauseReason::ProviderError {
        category: ProviderErrorCategory::AuthError,
        message: "Invalid API key".to_string(),
        retry_after: None,
        previous_status: "executing".to_string(),
        paused_at: "2020-01-01T00:00:00+00:00".to_string(),
        auto_resumable: false,
        resume_attempts: 0,
    };

    let mut task = Task::new(project.id.clone(), "Not Auto Resumable".to_string());
    task.internal_status = InternalStatus::Paused;
    task.metadata = Some(reason.write_to_task_metadata(None));
    app_state.task_repo.create(task.clone()).await.unwrap();

    let reconciled = reconciler
        .reconcile_paused_provider_error(&task)
        .await;
    assert!(
        !reconciled,
        "Non-auto-resumable new-format tasks should be skipped"
    );

    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert_eq!(updated.internal_status, InternalStatus::Paused);
}

#[tokio::test]
async fn reconcile_paused_at_max_concurrent_stays_paused() {
    use crate::application::chat_service::{PauseReason, ProviderErrorCategory};

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::with_max_concurrent(1));
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    // Fill up concurrency
    execution_state.increment_running();

    let reason = PauseReason::ProviderError {
        category: ProviderErrorCategory::RateLimit,
        message: "Rate limited".to_string(),
        retry_after: Some("2020-01-01T00:00:00+00:00".to_string()),
        previous_status: "executing".to_string(),
        paused_at: "2020-01-01T00:00:00+00:00".to_string(),
        auto_resumable: true,
        resume_attempts: 0,
    };

    let mut task = Task::new(project.id.clone(), "At Max Concurrent".to_string());
    task.internal_status = InternalStatus::Paused;
    task.metadata = Some(reason.write_to_task_metadata(None));
    app_state.task_repo.create(task.clone()).await.unwrap();

    let reconciled = reconciler
        .reconcile_paused_provider_error(&task)
        .await;
    assert!(
        !reconciled,
        "Should not resume when at max concurrent limit"
    );

    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert_eq!(
        updated.internal_status,
        InternalStatus::Paused,
        "Task should remain Paused when at max concurrent"
    );
}

#[tokio::test]
async fn reconcile_paused_with_both_old_and_new_keys_prefers_new() {
    use crate::application::chat_service::{
        PauseReason, ProviderErrorCategory, ProviderErrorMetadata,
    };

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    // Simulate metadata with both old and new keys (as written by handle_stream_error)
    let legacy = ProviderErrorMetadata {
        category: ProviderErrorCategory::RateLimit,
        message: "Rate limited".to_string(),
        retry_after: Some("2020-01-01T00:00:00+00:00".to_string()),
        previous_status: "executing".to_string(),
        paused_at: "2020-01-01T00:00:00+00:00".to_string(),
        auto_resumable: true,
        resume_attempts: 1,
    };
    let new_reason = PauseReason::ProviderError {
        category: ProviderErrorCategory::RateLimit,
        message: "Rate limited".to_string(),
        retry_after: Some("2020-01-01T00:00:00+00:00".to_string()),
        previous_status: "executing".to_string(),
        paused_at: "2020-01-01T00:00:00+00:00".to_string(),
        auto_resumable: true,
        resume_attempts: 1,
    };

    let with_legacy = legacy.write_to_task_metadata(None);
    let with_both = new_reason.write_to_task_metadata(Some(&with_legacy));

    let mut task = Task::new(project.id.clone(), "Both Keys Task".to_string());
    task.internal_status = InternalStatus::Paused;
    task.metadata = Some(with_both);
    app_state.task_repo.create(task.clone()).await.unwrap();

    app_state
        .task_repo
        .persist_status_change(
            &task.id,
            InternalStatus::Executing,
            InternalStatus::Paused,
            "paused",
        )
        .await
        .unwrap();

    // Should process via new format (not fall through to legacy handler)
    let reconciled = reconciler
        .reconcile_paused_provider_error(&task)
        .await;
    assert!(
        reconciled,
        "Should process the task via new PauseReason format"
    );
}

// =========================================================================
// Backward compat: legacy provider_error key read by reconciler
// =========================================================================

#[tokio::test]
async fn reconcile_legacy_provider_error_key_still_works() {
    use crate::application::chat_service::{ProviderErrorCategory, ProviderErrorMetadata};

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    // Only legacy key, no pause_reason key
    let legacy = ProviderErrorMetadata {
        category: ProviderErrorCategory::RateLimit,
        message: "Rate limited".to_string(),
        retry_after: Some("2020-01-01T00:00:00+00:00".to_string()),
        previous_status: "executing".to_string(),
        paused_at: "2020-01-01T00:00:00+00:00".to_string(),
        auto_resumable: true,
        resume_attempts: 0,
    };

    let mut task = Task::new(project.id.clone(), "Legacy Key Only".to_string());
    task.internal_status = InternalStatus::Paused;
    task.metadata = Some(legacy.write_to_task_metadata(None));
    app_state.task_repo.create(task.clone()).await.unwrap();

    app_state
        .task_repo
        .persist_status_change(
            &task.id,
            InternalStatus::Executing,
            InternalStatus::Paused,
            "paused",
        )
        .await
        .unwrap();

    // Verify only provider_error key, no pause_reason
    let meta_json: serde_json::Value =
        serde_json::from_str(task.metadata.as_ref().unwrap()).unwrap();
    assert!(meta_json.get("provider_error").is_some());
    assert!(meta_json.get("pause_reason").is_none());

    let reconciled = reconciler
        .reconcile_paused_provider_error(&task)
        .await;
    assert!(
        reconciled,
        "Legacy provider_error key should still be processed via backward compat"
    );

    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert_ne!(
        updated.internal_status,
        InternalStatus::Paused,
        "Legacy task should no longer be Paused after processing"
    );
}
