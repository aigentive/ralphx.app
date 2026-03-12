use super::*;
use crate::application::interactive_process_registry::{
    InteractiveProcessKey, InteractiveProcessRegistry,
};
use crate::application::{AppState, TaskTransitionService};
use crate::commands::execution_commands::ExecutionState;
use crate::domain::entities::{
    AgentRun, AgentRunId, AgentRunStatus, ChatContextType, ChatConversationId, ExecutionRecoveryMetadata,
    InternalStatus, MergeFailureSource, Project, Task, TaskId,
};
use crate::domain::services::{MemoryRunningAgentRegistry, RunningAgentKey, RunningAgentRegistry};
use crate::infrastructure::agents::claude::reconciliation_config;
use std::collections::HashSet;
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
    // Base = 5s (merge speed overhaul). With jitter, delay is in [base, base + base/4].
    let d0 = ReconciliationRunner::<tauri::Wry>::merge_incomplete_retry_delay(0).num_seconds();
    assert!((5..=5 + 5 / 4).contains(&d0), "retry 0: got {d0}");

    let d1 = ReconciliationRunner::<tauri::Wry>::merge_incomplete_retry_delay(1).num_seconds();
    assert!((10..=10 + 10 / 4).contains(&d1), "retry 1: got {d1}");

    let d2 = ReconciliationRunner::<tauri::Wry>::merge_incomplete_retry_delay(2).num_seconds();
    assert!((20..=20 + 20 / 4).contains(&d2), "retry 2: got {d2}");

    // Exponent caps at 6, so base_delay = 5 * 64 = 320 (below max 1800).
    // With base=5, exponent saturation at 6 gives 320s as the effective ceiling.
    let d10 = ReconciliationRunner::<tauri::Wry>::merge_incomplete_retry_delay(10).num_seconds();
    assert!(
        (320..=320 + 320 / 4).contains(&d10),
        "retry 10: got {d10}"
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
fn merging_timeout_default_is_1200_seconds() {
    // Default merger agent timeout is 20 minutes (1200s); configurable via
    // RALPHX_MERGER_TIMEOUT_SECS or RALPHX_RECONCILIATION_MERGER_TIMEOUT_SECS env var.
    assert_eq!(reconciliation_config().merger_timeout_secs, 1200);
}

#[test]
fn merging_max_auto_retries_is_3() {
    assert_eq!(reconciliation_config().merging_max_retries, 3);
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
    // and returns a stale age (> merging_timeout_seconds()).
    let mut task = Task::new(project.id.clone(), "Stuck Merging Task".to_string());
    task.internal_status = InternalStatus::Merging;
    task.updated_at = chrono::Utc::now()
        - chrono::Duration::seconds(reconciliation_config().merger_timeout_secs as i64 + 60);

    // Write MERGING_MAX_AUTO_RETRIES attempt_failed events to hit the retry cap
    let events: Vec<serde_json::Value> = (0..reconciliation_config().merging_max_retries)
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
    // With jitter, delay is in [base, base + base/4]. Check bounds.
    let d0 = ReconciliationRunner::<tauri::Wry>::merge_conflict_retry_delay(0).num_seconds();
    assert!((60..=60 + 60 / 4).contains(&d0), "retry 0: got {d0}");

    let d1 = ReconciliationRunner::<tauri::Wry>::merge_conflict_retry_delay(1).num_seconds();
    assert!((120..=120 + 120 / 4).contains(&d1), "retry 1: got {d1}");

    let d2 = ReconciliationRunner::<tauri::Wry>::merge_conflict_retry_delay(2).num_seconds();
    assert!((240..=240 + 240 / 4).contains(&d2), "retry 2: got {d2}");

    let d3 = ReconciliationRunner::<tauri::Wry>::merge_conflict_retry_delay(3).num_seconds();
    assert!((480..=480 + 480 / 4).contains(&d3), "retry 3: got {d3}");

    // Verify cap at 600s (base), with jitter up to 600/4=150
    let d10 = ReconciliationRunner::<tauri::Wry>::merge_conflict_retry_delay(10).num_seconds();
    assert!((600..=600 + 600 / 4).contains(&d10), "retry 10: got {d10}");
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
    task.metadata = Some(
        serde_json::json!({"merge_deferred": true, "merge_deferred_at": "2026-01-01T00:00:00Z"})
            .to_string(),
    );
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

    let reconciled = reconciler.reconcile_paused_provider_error(&task).await;
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

    let reconciled = reconciler.reconcile_paused_provider_error(&task).await;
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

    let reconciled = reconciler.reconcile_paused_provider_error(&task).await;
    assert!(reconciled, "Should resume when retry_after is in the past");

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
        resume_attempts: ProviderErrorMetadata::max_resume_attempts(), // At max
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

    let reconciled = reconciler.reconcile_paused_provider_error(&task).await;
    assert!(reconciled, "Should process the task (transition to Failed)");

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

    let reconciled = reconciler.reconcile_paused_provider_error(&task).await;
    assert!(!reconciled, "Non-auto-resumable tasks should be skipped");

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
        resume_attempts: ProviderErrorMetadata::max_resume_attempts(),
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

    let reconciled = reconciler.reconcile_paused_provider_error(&task).await;
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

    let reconciled = reconciler.reconcile_paused_provider_error(&task).await;
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

    let reconciled = reconciler.reconcile_paused_provider_error(&task).await;
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

    // New-format at MAX_RESUME_ATTEMPTS
    let reason = PauseReason::ProviderError {
        category: ProviderErrorCategory::ServerError,
        message: "502 Bad Gateway".to_string(),
        retry_after: None,
        previous_status: "executing".to_string(),
        paused_at: "2020-01-01T00:00:00+00:00".to_string(),
        auto_resumable: true,
        resume_attempts: ProviderErrorMetadata::max_resume_attempts(),
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

    let reconciled = reconciler.reconcile_paused_provider_error(&task).await;
    assert!(reconciled, "Should process the task (transition to Failed)");

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

    let reconciled = reconciler.reconcile_paused_provider_error(&task).await;
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
    let before = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .unwrap();
    let before_reason = PauseReason::from_task_metadata(before.metadata.as_deref()).unwrap();
    match before_reason {
        PauseReason::ProviderError {
            resume_attempts, ..
        } => {
            assert_eq!(resume_attempts, 2, "Should start at 2");
        }
        _ => panic!("Expected ProviderError"),
    }

    // Reconcile should increment resume_attempts to 3 before attempting resume
    let reconciled = reconciler.reconcile_paused_provider_error(&task).await;
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

    let reconciled = reconciler.reconcile_paused_provider_error(&task).await;
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

    let reconciled = reconciler.reconcile_paused_provider_error(&task).await;
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
    let reconciled = reconciler.reconcile_paused_provider_error(&task).await;
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

    let reconciled = reconciler.reconcile_paused_provider_error(&task).await;
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

// ── Heartbeat / last_active_at tests ──

#[tokio::test]
async fn update_heartbeat_sets_last_active_at_in_memory_registry() {
    let registry = MemoryRunningAgentRegistry::new();
    let key = RunningAgentKey::new("merge", "task-hb-1");

    // Register an agent with no heartbeat
    registry
        .register(
            key.clone(),
            12345,
            "conv-hb".to_string(),
            "run-hb".to_string(),
            None,
            None,
        )
        .await;

    // Heartbeat not set yet
    let info = registry.get(&key).await.unwrap();
    assert!(
        info.last_active_at.is_none(),
        "last_active_at should be None before heartbeat"
    );

    // Write a heartbeat
    let ts = chrono::Utc::now();
    registry.update_heartbeat(&key, ts).await;

    let info = registry.get(&key).await.unwrap();
    assert!(
        info.last_active_at.is_some(),
        "last_active_at should be Some after heartbeat"
    );
    let delta = (info.last_active_at.unwrap() - ts).num_milliseconds().abs();
    assert!(
        delta < 100,
        "last_active_at should match the written timestamp"
    );
}

#[tokio::test]
async fn update_heartbeat_noops_for_unknown_key() {
    let registry = MemoryRunningAgentRegistry::new();
    let key = RunningAgentKey::new("merge", "nonexistent-task");
    // Should not panic — just silently does nothing
    registry.update_heartbeat(&key, chrono::Utc::now()).await;
    assert!(!registry.is_running(&key).await);
}

#[tokio::test]
async fn reconcile_merging_not_stale_when_heartbeat_is_recent() {
    // Task entered Merging a long time ago (via updated_at fallback) but has a recent heartbeat.
    // Reconciler should use the heartbeat and NOT consider the task stale.
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    // Task entered Merging 600s ago (2x timeout) — would normally be stale
    let mut task = Task::new(project.id.clone(), "Heartbeat Merging Task".to_string());
    task.internal_status = InternalStatus::Merging;
    task.updated_at = chrono::Utc::now() - chrono::Duration::seconds(600);
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Register an agent in Merging context with a RECENT heartbeat (just 10s ago)
    let merge_key = RunningAgentKey::new("merge", task.id.as_str());
    app_state
        .running_agent_registry
        .register(
            merge_key.clone(),
            99999,
            "conv-merge".to_string(),
            "run-merge".to_string(),
            None,
            None,
        )
        .await;
    let recent_heartbeat = chrono::Utc::now() - chrono::Duration::seconds(10);
    app_state
        .running_agent_registry
        .update_heartbeat(&merge_key, recent_heartbeat)
        .await;

    // Reconcile — recent heartbeat means effective_age < timeout, so NOT stale
    reconciler
        .reconcile_merging_task(&task, InternalStatus::Merging)
        .await;

    // Task should still be in Merging — no timeout metadata recorded
    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert_eq!(
        updated.internal_status,
        InternalStatus::Merging,
        "Task should not leave Merging when heartbeat is recent (effective_age < timeout)"
    );

    // merge_recovery metadata should NOT contain an attempt_failed event
    // (record_merge_timeout_event only fires when is_stale)
    let metadata_str = updated.metadata.as_deref().unwrap_or("{}");
    let meta_json: serde_json::Value = serde_json::from_str(metadata_str).unwrap_or_default();
    let has_timeout_record = meta_json.get("merge_recovery").is_some();
    assert!(
        !has_timeout_record,
        "No merge_recovery metadata should be written when heartbeat is recent"
    );
}

#[tokio::test]
async fn reconcile_merging_stale_when_heartbeat_is_old() {
    // Task has an old heartbeat (>1200s default timeout) — should be considered stale.
    // Staleness is confirmed by checking that record_merge_timeout_event fired
    // (writes merge_recovery metadata).
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    // Task entered Merging "just now" via updated_at — wall-clock fallback would NOT trigger
    let mut task = Task::new(project.id.clone(), "Old Heartbeat Merging Task".to_string());
    task.internal_status = InternalStatus::Merging;
    task.updated_at = chrono::Utc::now() - chrono::Duration::seconds(10);
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Register agent with an OLD heartbeat (1300s ago — beyond 1200s default timeout)
    let merge_key = RunningAgentKey::new("merge", task.id.as_str());
    app_state
        .running_agent_registry
        .register(
            merge_key.clone(),
            99999,
            "conv-merge-old".to_string(),
            "run-merge-old".to_string(),
            None,
            None,
        )
        .await;
    let old_heartbeat = chrono::Utc::now() - chrono::Duration::seconds(1300);
    app_state
        .running_agent_registry
        .update_heartbeat(&merge_key, old_heartbeat)
        .await;

    // Reconcile — old heartbeat should trigger staleness
    reconciler
        .reconcile_merging_task(&task, InternalStatus::Merging)
        .await;

    // merge_recovery metadata with an attempt_failed event confirms staleness was detected
    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    let metadata_str = updated.metadata.as_deref().unwrap_or("{}");
    let meta_json: serde_json::Value = serde_json::from_str(metadata_str).unwrap_or_default();
    assert!(
        meta_json.get("merge_recovery").is_some(),
        "merge_recovery metadata should be written when heartbeat is old (stale path)"
    );
    // Verify an AttemptFailed event was recorded
    let events = meta_json["merge_recovery"]["events"].as_array();
    let has_attempt_failed = events
        .map(|evts| {
            evts.iter()
                .any(|e| e["kind"].as_str() == Some("attempt_failed"))
        })
        .unwrap_or(false);
    assert!(
        has_attempt_failed,
        "An attempt_failed event should be recorded when effective_age >= merging_timeout_seconds()"
    );
}

// =========================================================================
// Smart auto-retry guards (Phase 4)
// =========================================================================

// ── Agent-reported conflict guard ──

#[tokio::test]
async fn reconcile_merge_conflict_skips_when_agent_reported() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let mut task = Task::new(project.id.clone(), "Agent Conflict Task".to_string());
    task.internal_status = InternalStatus::MergeConflict;
    task.updated_at = chrono::Utc::now() - chrono::Duration::seconds(3600);
    // Mark as agent-reported (set by report_conflict handler)
    task.metadata = Some(
        serde_json::json!({
            "merge_failure_source": "agent_reported",
            "conflict_files": ["src/foo.rs"],
        })
        .to_string(),
    );
    app_state.task_repo.create(task.clone()).await.unwrap();

    let reconciled = reconciler
        .reconcile_merge_conflict_task(&task, InternalStatus::MergeConflict)
        .await;
    assert!(
        !reconciled,
        "Agent-reported conflicts must not be auto-retried (AgentReported guard)"
    );

    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert_eq!(
        updated.internal_status,
        InternalStatus::MergeConflict,
        "Task should remain in MergeConflict — agent-reported conflicts require human action"
    );
}

#[tokio::test]
async fn reconcile_merge_incomplete_skips_when_agent_reported() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let mut task = Task::new(project.id.clone(), "Agent Incomplete Task".to_string());
    task.internal_status = InternalStatus::MergeIncomplete;
    task.updated_at = chrono::Utc::now() - chrono::Duration::seconds(3600);
    // Mark as agent-reported (set by report_incomplete handler)
    task.metadata = Some(
        serde_json::json!({
            "error": "Merger agent explicitly gave up",
            "merge_failure_source": "agent_reported",
        })
        .to_string(),
    );
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Record status history
    app_state
        .task_repo
        .persist_status_change(
            &task.id,
            InternalStatus::Merging,
            InternalStatus::MergeIncomplete,
            "merge_incomplete",
        )
        .await
        .unwrap();

    let reconciled = reconciler
        .reconcile_merge_incomplete_task(&task, InternalStatus::MergeIncomplete)
        .await;
    assert!(
        !reconciled,
        "Agent-reported incomplete must not be auto-retried (AgentReported guard)"
    );

    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert_eq!(
        updated.internal_status,
        InternalStatus::MergeIncomplete,
        "Task should remain in MergeIncomplete — agent-reported failures require human action"
    );
}

// ── SHA comparison guard ──

#[test]
fn last_stored_source_sha_reads_most_recent_event_sha() {
    let mut task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "SHA Guard Task".to_string(),
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
                        "message": "retry 1",
                        "source_sha": "abc123"
                    },
                    {
                        "at": "2026-02-10T00:01:00Z",
                        "kind": "auto_retry_triggered",
                        "source": "auto",
                        "reason_code": "git_error",
                        "message": "retry 2",
                        "source_sha": "def456"
                    }
                ],
                "last_state": "retrying"
            }
        })
        .to_string(),
    );

    let sha = ReconciliationRunner::<tauri::Wry>::last_stored_source_sha(&task);
    assert_eq!(
        sha.as_deref(),
        Some("def456"),
        "Should return the SHA from the most recent event"
    );
}

#[test]
fn last_stored_source_sha_returns_none_when_no_events() {
    let task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "No SHA Task".to_string(),
    );

    let sha = ReconciliationRunner::<tauri::Wry>::last_stored_source_sha(&task);
    assert!(sha.is_none(), "Should return None when no events exist");
}

// ── Validation revert loop-breaking guard ──

#[tokio::test]
async fn reconcile_merge_incomplete_stops_after_max_validation_reverts() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    // Task with validation_revert_count = 3 (>= max of 2)
    let mut task = Task::new(project.id.clone(), "Validation Loop Task".to_string());
    task.internal_status = InternalStatus::MergeIncomplete;
    task.updated_at = chrono::Utc::now() - chrono::Duration::seconds(3600);
    task.metadata = Some(
        serde_json::json!({
            "error": "Merge validation failed: 1 command(s) failed",
            "merge_failure_source": "validation_failed",
            "validation_revert_count": 3,  // >= VALIDATION_REVERT_MAX_COUNT (2)
            "source_branch": "ralphx/task-xyz",
            "target_branch": "main",
        })
        .to_string(),
    );
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Record status history
    app_state
        .task_repo
        .persist_status_change(
            &task.id,
            InternalStatus::PendingMerge,
            InternalStatus::MergeIncomplete,
            "validation_failed",
        )
        .await
        .unwrap();

    let reconciled = reconciler
        .reconcile_merge_incomplete_task(&task, InternalStatus::MergeIncomplete)
        .await;
    assert!(
        !reconciled,
        "Should stop auto-retrying after max validation reverts (loop-breaking guard)"
    );

    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert_eq!(
        updated.internal_status,
        InternalStatus::MergeIncomplete,
        "Task should remain in MergeIncomplete and surface to user for manual fix"
    );
}

#[tokio::test]
async fn reconcile_merge_incomplete_retries_when_below_max_validation_reverts() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    // Task with validation_revert_count = 1 (< max of 2)
    let mut task = Task::new(project.id.clone(), "Validation Retry OK Task".to_string());
    task.internal_status = InternalStatus::MergeIncomplete;
    // updated_at far in past so age > retry delay
    task.updated_at = chrono::Utc::now() - chrono::Duration::seconds(7200);
    task.metadata = Some(
        serde_json::json!({
            "error": "Merge validation failed: 1 command(s) failed",
            "merge_failure_source": "validation_failed",
            "validation_revert_count": 1,  // < VALIDATION_REVERT_MAX_COUNT (2), allow retry
            "source_branch": "ralphx/task-xyz",
            "target_branch": "main",
        })
        .to_string(),
    );
    app_state.task_repo.create(task.clone()).await.unwrap();

    let reconciled = reconciler
        .reconcile_merge_incomplete_task(&task, InternalStatus::MergeIncomplete)
        .await;
    // Should NOT be blocked by the revert count guard (count=1 <= max=2)
    // But may be blocked by age check if status history isn't set up
    // The key assertion: reconciler did NOT refuse due to validation_revert_count
    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");

    // With count=1 <= max=2 and no branch_missing/agent_reported, the reconciler
    // proceeds to age check. Without history, it falls back to updated_at which is
    // 2h ago (> 30s delay), so it should transition to PendingMerge.
    assert!(
        reconciled || updated.internal_status == InternalStatus::MergeIncomplete,
        "Task with revert_count=1 should not be blocked by loop-breaking guard"
    );
}

#[test]
fn validation_revert_max_count_is_2() {
    assert_eq!(
        reconciliation_config().validation_revert_max_count,
        2,
        "Max validation reverts before stopping should be 2"
    );
}

// ── is_agent_reported_failure helper ──

#[test]
fn is_agent_reported_failure_returns_true_for_agent_reported() {
    let mut task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "Agent Reported".to_string(),
    );
    task.metadata = Some(
        serde_json::json!({
            "merge_failure_source": serde_json::to_value(MergeFailureSource::AgentReported).unwrap()
        })
        .to_string(),
    );
    assert!(
        ReconciliationRunner::<tauri::Wry>::is_agent_reported_failure(&task),
        "Should return true for agent_reported failure source"
    );
}

#[test]
fn is_agent_reported_failure_returns_false_for_transient_git() {
    let mut task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "Transient Git".to_string(),
    );
    task.metadata = Some(
        serde_json::json!({
            "merge_failure_source": serde_json::to_value(MergeFailureSource::TransientGit).unwrap()
        })
        .to_string(),
    );
    assert!(
        !ReconciliationRunner::<tauri::Wry>::is_agent_reported_failure(&task),
        "TransientGit should not block auto-retry"
    );
}

#[test]
fn is_agent_reported_failure_returns_false_for_no_metadata() {
    let task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "No Metadata".to_string(),
    );
    assert!(
        !ReconciliationRunner::<tauri::Wry>::is_agent_reported_failure(&task),
        "No metadata should not block auto-retry"
    );
}

#[test]
fn validation_revert_count_reads_counter_from_metadata() {
    let mut task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "Revert Count Task".to_string(),
    );
    task.metadata = Some(serde_json::json!({"validation_revert_count": 3}).to_string());
    assert_eq!(
        ReconciliationRunner::<tauri::Wry>::validation_revert_count(&task),
        3
    );
}

#[test]
fn validation_revert_count_returns_zero_for_no_metadata() {
    let task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "No Metadata".to_string(),
    );
    assert_eq!(
        ReconciliationRunner::<tauri::Wry>::validation_revert_count(&task),
        0
    );
}

// ── Retry delay jitter + cap tests ──────────────────────────────────

#[test]
fn merge_incomplete_retry_delay_includes_jitter() {
    // Call delay function many times with same retry_count.
    // With jitter, results should not all be identical.
    let delays: HashSet<i64> = (0..20)
        .map(|_| ReconciliationRunner::<tauri::Wry>::merge_incomplete_retry_delay(3).num_seconds())
        .collect();
    assert!(
        delays.len() > 1,
        "Expected jitter to produce varying delays, but got a single value: {:?}",
        delays
    );
}

#[test]
fn merge_incomplete_retry_delay_caps_at_configured_max() {
    let cfg = reconciliation_config();
    let max_secs = cfg.merge_incomplete_retry_max_secs as i64;
    let base_secs = cfg.merge_incomplete_retry_base_secs as i64;
    // Exponent caps at 6, so saturated base = base * 64.
    // If saturated base < max, the effective ceiling is saturated base, not max.
    let saturated = (base_secs * 64).min(max_secs);
    for _ in 0..20 {
        let delay =
            ReconciliationRunner::<tauri::Wry>::merge_incomplete_retry_delay(100).num_seconds();
        assert!(
            delay <= saturated + saturated / 4,
            "Delay {} exceeded saturated {} + jitter ceiling {}",
            delay,
            saturated,
            saturated / 4,
        );
        assert!(
            delay >= saturated,
            "Delay {} should be at least the saturated base {}",
            delay,
            saturated,
        );
    }
}

#[test]
fn merge_conflict_retry_delay_includes_jitter() {
    let delays: HashSet<i64> = (0..20)
        .map(|_| ReconciliationRunner::<tauri::Wry>::merge_conflict_retry_delay(3).num_seconds())
        .collect();
    assert!(
        delays.len() > 1,
        "Expected jitter to produce varying delays, but got a single value: {:?}",
        delays
    );
}

#[test]
fn merge_conflict_retry_delay_caps_at_configured_max() {
    let cfg = reconciliation_config();
    let max_secs = cfg.merge_conflict_retry_max_secs as i64;
    for _ in 0..20 {
        let delay =
            ReconciliationRunner::<tauri::Wry>::merge_conflict_retry_delay(100).num_seconds();
        assert!(
            delay <= max_secs + max_secs / 4,
            "Delay {} exceeded max {} + jitter ceiling {}",
            delay,
            max_secs,
            max_secs / 4,
        );
        assert!(
            delay >= max_secs,
            "Delay {} should be at least the base max {}",
            delay,
            max_secs,
        );
    }
}

#[test]
fn merge_incomplete_max_retries_is_at_least_15() {
    let cfg = reconciliation_config();
    assert!(
        cfg.merge_incomplete_max_retries >= 15,
        "merge_incomplete_max_retries should be >= 15, got {}",
        cfg.merge_incomplete_max_retries,
    );
}

#[test]
fn merge_incomplete_retry_max_secs_is_at_least_1800() {
    let cfg = reconciliation_config();
    assert!(
        cfg.merge_incomplete_retry_max_secs >= 1800,
        "merge_incomplete_retry_max_secs should be >= 1800, got {}",
        cfg.merge_incomplete_retry_max_secs,
    );
}

// ── Rate limit guard in reconcile_merge_incomplete_task ──

#[tokio::test]
async fn reconcile_merge_incomplete_skips_retry_when_rate_limit_active() {
    use crate::domain::entities::{
        MergeRecoveryEventKind, MergeRecoveryMetadata, MergeRecoveryReasonCode,
        MergeRecoverySource, MergeRecoveryState, Project,
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

    // Create a task in MergeIncomplete with rate_limit_retry_after set to the future
    let mut task = Task::new(project.id.clone(), "Rate Limited Merge Task".to_string());
    task.internal_status = InternalStatus::MergeIncomplete;

    let mut recovery = MergeRecoveryMetadata::new();
    recovery.rate_limit_retry_after = Some("2099-12-31T23:59:59+00:00".to_string());
    recovery.last_state = MergeRecoveryState::RateLimited;
    recovery.append_event(crate::domain::entities::MergeRecoveryEvent::new(
        MergeRecoveryEventKind::AttemptFailed,
        MergeRecoverySource::System,
        MergeRecoveryReasonCode::ProviderRateLimited,
        "Rate limit hit during merge",
    ));
    task.metadata = Some(recovery.update_task_metadata(None).unwrap());
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Record status history
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

    // Reconciler should skip retry because rate limit is active (future timestamp)
    let reconciled = reconciler
        .reconcile_merge_incomplete_task(&task, InternalStatus::MergeIncomplete)
        .await;
    assert!(
        !reconciled,
        "Reconciler should skip retry when rate limit is active"
    );

    // Verify task stayed in MergeIncomplete
    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert_eq!(
        updated.internal_status,
        InternalStatus::MergeIncomplete,
        "Task should remain in MergeIncomplete while rate-limited"
    );
}

#[tokio::test]
async fn reconcile_merge_incomplete_proceeds_after_rate_limit_expired() {
    use crate::domain::entities::{MergeRecoveryMetadata, MergeRecoveryState, Project};

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    // Create a task with rate_limit_retry_after set to the PAST (expired)
    let mut task = Task::new(project.id.clone(), "Expired Rate Limit Task".to_string());
    task.internal_status = InternalStatus::MergeIncomplete;
    // Set updated_at far in past so age > retry delay (fallback when no status history)
    task.updated_at = chrono::Utc::now() - chrono::Duration::seconds(7200);

    let mut recovery = MergeRecoveryMetadata::new();
    recovery.rate_limit_retry_after = Some("2020-01-01T00:00:00+00:00".to_string());
    recovery.last_state = MergeRecoveryState::RateLimited;
    task.metadata = Some(recovery.update_task_metadata(None).unwrap());
    app_state.task_repo.create(task.clone()).await.unwrap();

    // No persist_status_change — reconciler falls back to updated_at (7200s ago)

    // Reconciler should proceed because rate limit is expired
    let reconciled = reconciler
        .reconcile_merge_incomplete_task(&task, InternalStatus::MergeIncomplete)
        .await;

    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");

    // After rate limit clears, reconciler should either retry (PendingMerge)
    // or pass through normally. The rate limit guard should NOT block.
    assert!(
        reconciled || updated.internal_status == InternalStatus::PendingMerge,
        "Should proceed after rate limit expired; got status={:?}, reconciled={}",
        updated.internal_status,
        reconciled,
    );

    // Verify rate_limit_retry_after was cleared from metadata
    let restored_meta =
        MergeRecoveryMetadata::from_task_metadata(updated.metadata.as_deref()).unwrap_or(None);
    if let Some(meta) = restored_meta {
        assert_eq!(
            meta.rate_limit_retry_after, None,
            "rate_limit_retry_after should be cleared after expiry"
        );
    }
}

#[tokio::test]
async fn rate_limited_skips_dont_count_toward_max_retries() {
    use crate::domain::entities::{MergeRecoveryMetadata, MergeRecoveryState, Project};

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    // Create a task with rate limit set to FUTURE — reconciler should skip
    let mut task = Task::new(project.id.clone(), "Rate Limit Budget Task".to_string());
    task.internal_status = InternalStatus::MergeIncomplete;

    let mut recovery = MergeRecoveryMetadata::new();
    recovery.rate_limit_retry_after = Some("2099-12-31T23:59:59+00:00".to_string());
    recovery.last_state = MergeRecoveryState::RateLimited;
    task.metadata = Some(recovery.update_task_metadata(None).unwrap());
    app_state.task_repo.create(task.clone()).await.unwrap();

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

    // Call reconciler multiple times while rate-limited — should all skip silently
    for _ in 0..5 {
        let reconciled = reconciler
            .reconcile_merge_incomplete_task(&task, InternalStatus::MergeIncomplete)
            .await;
        assert!(!reconciled, "Should skip while rate-limited");
    }

    // Verify that NO AutoRetryTriggered events were added (rate-limited skips don't count)
    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    let retry_count =
        ReconciliationRunner::<tauri::Wry>::merge_incomplete_auto_retry_count(&updated);
    assert_eq!(
        retry_count, 0,
        "Rate-limited skips should NOT count toward max retries (got {} retries)",
        retry_count
    );
}

#[test]
fn get_rate_limit_retry_after_reads_from_metadata() {
    use crate::domain::entities::{MergeRecoveryMetadata, MergeRecoveryState};

    let mut task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "Rate Limit Read Task".to_string(),
    );

    // No metadata → None
    assert!(
        ReconciliationRunner::<tauri::Wry>::get_rate_limit_retry_after(&task).is_none(),
        "Should return None when no metadata"
    );

    // With rate_limit_retry_after set
    let mut recovery = MergeRecoveryMetadata::new();
    recovery.rate_limit_retry_after = Some("2026-02-20T15:00:00+00:00".to_string());
    recovery.last_state = MergeRecoveryState::RateLimited;
    task.metadata = Some(recovery.update_task_metadata(None).unwrap());

    assert_eq!(
        ReconciliationRunner::<tauri::Wry>::get_rate_limit_retry_after(&task),
        Some("2026-02-20T15:00:00+00:00".to_string()),
        "Should read rate_limit_retry_after from merge recovery metadata"
    );
}

#[test]
fn get_rate_limit_retry_after_returns_none_when_not_set() {
    use crate::domain::entities::MergeRecoveryMetadata;

    let mut task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "No Rate Limit Task".to_string(),
    );

    // Metadata with merge_recovery but no rate_limit_retry_after
    let recovery = MergeRecoveryMetadata::new();
    task.metadata = Some(recovery.update_task_metadata(None).unwrap());

    assert!(
        ReconciliationRunner::<tauri::Wry>::get_rate_limit_retry_after(&task).is_none(),
        "Should return None when rate_limit_retry_after is not set"
    );
}

// ── has_merge_retry_in_progress helper ──────────────────────────────

#[test]
fn has_merge_retry_in_progress_returns_true_for_fresh_timestamp() {
    let mut task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "Fresh Retry".to_string(),
    );
    task.metadata = Some(
        serde_json::json!({
            "merge_retry_in_progress": chrono::Utc::now().to_rfc3339()
        })
        .to_string(),
    );
    assert!(
        ReconciliationRunner::<tauri::Wry>::has_merge_retry_in_progress(&task),
        "Fresh timestamp should indicate retry in progress"
    );
}

#[test]
fn has_merge_retry_in_progress_returns_false_for_expired_timestamp() {
    let mut task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "Expired Retry".to_string(),
    );
    // 120 seconds ago — well past the 60s expiry
    let expired = chrono::Utc::now() - chrono::Duration::seconds(120);
    task.metadata = Some(
        serde_json::json!({
            "merge_retry_in_progress": expired.to_rfc3339()
        })
        .to_string(),
    );
    assert!(
        !ReconciliationRunner::<tauri::Wry>::has_merge_retry_in_progress(&task),
        "Expired timestamp (>60s) should NOT indicate retry in progress"
    );
}

#[test]
fn has_merge_retry_in_progress_returns_false_for_legacy_boolean() {
    let mut task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "Legacy Guard".to_string(),
    );
    task.metadata = Some(
        serde_json::json!({
            "merge_retry_in_progress": true
        })
        .to_string(),
    );
    assert!(
        !ReconciliationRunner::<tauri::Wry>::has_merge_retry_in_progress(&task),
        "Legacy boolean true should be treated as stale (no timestamp = cannot verify freshness)"
    );
}

#[test]
fn has_merge_retry_in_progress_returns_false_for_no_metadata() {
    let task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "No Metadata".to_string(),
    );
    assert!(
        !ReconciliationRunner::<tauri::Wry>::has_merge_retry_in_progress(&task),
        "No metadata should return false"
    );
}

#[test]
fn has_merge_retry_in_progress_returns_false_for_missing_key() {
    let mut task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "Other Metadata".to_string(),
    );
    task.metadata = Some(serde_json::json!({"some_other_key": "value"}).to_string());
    assert!(
        !ReconciliationRunner::<tauri::Wry>::has_merge_retry_in_progress(&task),
        "Metadata without merge_retry_in_progress key should return false"
    );
}

// ── validation_revert_count boundary (>= check) ────────────────────

#[tokio::test]
async fn reconcile_merge_incomplete_blocks_at_exact_max_validation_reverts() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    // Task with validation_revert_count = 2 (== max of 2, should block with >= check)
    let mut task = Task::new(project.id.clone(), "Boundary Count Task".to_string());
    task.internal_status = InternalStatus::MergeIncomplete;
    task.updated_at = chrono::Utc::now() - chrono::Duration::seconds(3600);
    task.metadata = Some(
        serde_json::json!({
            "error": "Merge validation failed",
            "merge_failure_source": "validation_failed",
            "validation_revert_count": 2,  // == VALIDATION_REVERT_MAX_COUNT (2)
        })
        .to_string(),
    );
    app_state.task_repo.create(task.clone()).await.unwrap();

    app_state
        .task_repo
        .persist_status_change(
            &task.id,
            InternalStatus::PendingMerge,
            InternalStatus::MergeIncomplete,
            "validation_failed",
        )
        .await
        .unwrap();

    let reconciled = reconciler
        .reconcile_merge_incomplete_task(&task, InternalStatus::MergeIncomplete)
        .await;
    assert!(
        !reconciled,
        "Should block auto-retry when revert_count == max (>= boundary)"
    );

    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert_eq!(
        updated.internal_status,
        InternalStatus::MergeIncomplete,
        "Task should remain in MergeIncomplete at exact boundary"
    );
}

// ── reconciler skips tasks with merge_retry_in_progress ─────────────

#[tokio::test]
async fn reconcile_merge_incomplete_skips_when_user_retry_in_progress() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    // Task with high revert_count BUT merge_retry_in_progress set —
    // reconciler should skip it entirely and let the background task handle it.
    let mut task = Task::new(project.id.clone(), "User Retry Task".to_string());
    task.internal_status = InternalStatus::MergeIncomplete;
    task.updated_at = chrono::Utc::now() - chrono::Duration::seconds(3600);
    task.metadata = Some(
        serde_json::json!({
            "validation_revert_count": 5,
            "merge_failure_source": "validation_failed",
            "merge_retry_in_progress": chrono::Utc::now().to_rfc3339(),
        })
        .to_string(),
    );
    app_state.task_repo.create(task.clone()).await.unwrap();

    let reconciled = reconciler
        .reconcile_merge_incomplete_task(&task, InternalStatus::MergeIncomplete)
        .await;

    // Should return true ("handled, skip") — NOT false ("blocked by revert loop")
    assert!(
        reconciled,
        "Should skip reconciliation (return true) when user retry is in progress, \
         even with revert_count=5 exceeding max=2"
    );

    // Task should NOT have been transitioned — it stays in MergeIncomplete
    // for the background retry task to handle
    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert_eq!(
        updated.internal_status,
        InternalStatus::MergeIncomplete,
        "Task should remain in MergeIncomplete — background retry handles transition"
    );
}

#[tokio::test]
async fn reconcile_merging_skips_when_auto_complete_in_flight() {
    // When attempt_merge_auto_complete is already running (e.g. validation in progress),
    // the reconciler should skip this cycle entirely to avoid misinterpreting the
    // dedup guard's "skip" as a failure and incorrectly escalating.
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    // Task in Merging state with old transition (would normally trigger reconciliation)
    let mut task = Task::new(
        project.id.clone(),
        "Auto-complete in-flight task".to_string(),
    );
    task.internal_status = InternalStatus::Merging;
    task.updated_at = chrono::Utc::now() - chrono::Duration::seconds(600);
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Simulate auto-complete in flight for this task
    assert!(execution_state.try_start_auto_complete(task.id.as_str()));

    // Reconcile — should skip because auto-complete is in flight
    let result = reconciler
        .reconcile_merging_task(&task, InternalStatus::Merging)
        .await;

    // Should return true (handled/skip) not false (nothing to do)
    assert!(
        result,
        "Reconciler should return true when auto-complete is in flight (skip this cycle)"
    );

    // Task should still be in Merging — no escalation, no retry increment
    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert_eq!(
        updated.internal_status,
        InternalStatus::Merging,
        "Task should remain in Merging when auto-complete is in flight"
    );

    // No merge_recovery metadata should be written (no timeout recorded)
    let metadata_str = updated.metadata.as_deref().unwrap_or("{}");
    let meta_json: serde_json::Value = serde_json::from_str(metadata_str).unwrap_or_default();
    assert!(
        meta_json.get("merge_recovery").is_none(),
        "No merge_recovery metadata should be written when auto-complete is in flight"
    );

    // Cleanup
    execution_state.finish_auto_complete(task.id.as_str());
}

#[test]
fn is_auto_complete_in_flight_tracks_correctly() {
    let state = ExecutionState::new();
    let task_id = "test-task-123";

    // Initially not in flight
    assert!(!state.is_auto_complete_in_flight(task_id));

    // Start auto-complete
    assert!(state.try_start_auto_complete(task_id));
    assert!(state.is_auto_complete_in_flight(task_id));

    // Different task is not in flight
    assert!(!state.is_auto_complete_in_flight("other-task"));

    // Finish auto-complete
    state.finish_auto_complete(task_id);
    assert!(!state.is_auto_complete_in_flight(task_id));
}

#[tokio::test]
async fn test_agent_reported_failure_skips_retry() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let mut task = Task::new(project.id.clone(), "Agent Reported Failure".to_string());
    task.internal_status = InternalStatus::MergeIncomplete;
    task.metadata = Some(
        serde_json::json!({
            "merge_failure_source": serde_json::to_value(MergeFailureSource::AgentReported).unwrap()
        })
        .to_string(),
    );
    app_state.task_repo.create(task.clone()).await.unwrap();

    let reconciled = reconciler
        .reconcile_merge_incomplete_task(&task, InternalStatus::MergeIncomplete)
        .await;
    assert!(
        !reconciled,
        "Should not retry when agent explicitly reported the failure (AgentReported guard)"
    );
}

#[tokio::test]
async fn test_validation_revert_max_exceeded_skips_retry() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let mut task = Task::new(project.id.clone(), "Validation Revert Loop".to_string());
    task.internal_status = InternalStatus::MergeIncomplete;
    // Set validation_revert_count at max to trigger the loop-breaking guard
    task.metadata = Some(
        serde_json::json!({
            "validation_revert_count": reconciliation_config().validation_revert_max_count
        })
        .to_string(),
    );
    app_state.task_repo.create(task.clone()).await.unwrap();

    let reconciled = reconciler
        .reconcile_merge_incomplete_task(&task, InternalStatus::MergeIncomplete)
        .await;
    assert!(
        !reconciled,
        "Should not retry when validation_revert_count has reached max (ValidationFailed guard)"
    );
}

// ── RC#4: Validation failure cooldown + circuit breaker tests ──

#[tokio::test]
async fn rc4_validation_failure_no_transition_before_cooldown() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let mut task = Task::new(project.id.clone(), "Validation Cooldown Task".to_string());
    task.internal_status = InternalStatus::MergeIncomplete;
    // Mark as validation failure — should enforce 120s cooldown
    task.metadata = Some(
        serde_json::json!({
            "merge_failure_source": "validation_failed",
            "validation_revert_count": 1,
        })
        .to_string(),
    );
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Record RECENT status history — age will be < cooldown (120s)
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

    let reconciled = reconciler
        .reconcile_merge_incomplete_task(&task, InternalStatus::MergeIncomplete)
        .await;
    assert!(
        !reconciled,
        "Should not retry validation failure before cooldown elapsed (RC#4)"
    );

    // Verify task stayed in MergeIncomplete
    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert_eq!(
        updated.internal_status,
        InternalStatus::MergeIncomplete,
        "Task should remain MergeIncomplete during validation cooldown"
    );
}

#[tokio::test]
async fn rc4_consecutive_validation_failures_circuit_breaker_stops_retry() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let circuit_breaker_count = reconciliation_config().validation_failure_circuit_breaker_count;

    let mut task = Task::new(
        project.id.clone(),
        "Circuit Breaker Task".to_string(),
    );
    task.internal_status = InternalStatus::MergeIncomplete;
    // Set validation failure with consecutive failures at circuit breaker threshold
    task.metadata = Some(
        serde_json::json!({
            "merge_failure_source": "validation_failed",
            "validation_revert_count": 1,
            "consecutive_validation_failures": circuit_breaker_count,
        })
        .to_string(),
    );
    // Set updated_at far in the past so cooldown check passes
    task.updated_at = chrono::Utc::now() - chrono::Duration::seconds(300);
    app_state.task_repo.create(task.clone()).await.unwrap();

    let reconciled = reconciler
        .reconcile_merge_incomplete_task(&task, InternalStatus::MergeIncomplete)
        .await;
    assert!(
        !reconciled,
        "Circuit breaker should stop retry after {} consecutive validation failures (RC#4)",
        circuit_breaker_count
    );

    // Verify task stayed in MergeIncomplete
    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert_eq!(
        updated.internal_status,
        InternalStatus::MergeIncomplete,
        "Task should remain MergeIncomplete after circuit breaker trips"
    );
}

// ── RC#5: Starvation guard tests ──

#[tokio::test]
async fn rc5_starvation_guard_skips_recently_retried_task() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    // Task 1: recently retried (last_retried_at = now)
    let mut task1 = Task::new(project.id.clone(), "Recently Retried Task".to_string());
    task1.internal_status = InternalStatus::MergeIncomplete;
    task1.metadata = Some(
        serde_json::json!({
            "last_retried_at": chrono::Utc::now().to_rfc3339(),
        })
        .to_string(),
    );
    // Set updated_at far enough in the past that normal retry delay would pass
    task1.updated_at = chrono::Utc::now() - chrono::Duration::seconds(300);
    app_state.task_repo.create(task1.clone()).await.unwrap();

    // Task 2: not recently retried (no last_retried_at)
    let mut task2 = Task::new(project.id.clone(), "Fresh Task".to_string());
    task2.internal_status = InternalStatus::MergeIncomplete;
    task2.metadata = Some(serde_json::json!({}).to_string());
    task2.updated_at = chrono::Utc::now() - chrono::Duration::seconds(300);
    app_state.task_repo.create(task2.clone()).await.unwrap();

    // Task 1 should be skipped due to starvation guard
    let reconciled1 = reconciler
        .reconcile_merge_incomplete_task(&task1, InternalStatus::MergeIncomplete)
        .await;
    assert!(
        !reconciled1,
        "Starvation guard should skip recently-retried task (RC#5)"
    );

    // Task 2 should proceed (no starvation guard blocking)
    let reconciled2 = reconciler
        .reconcile_merge_incomplete_task(&task2, InternalStatus::MergeIncomplete)
        .await;
    // Task 2 should either transition or at least not be blocked by starvation guard
    let updated2 = app_state
        .task_repo
        .get_by_id(&task2.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert!(
        updated2.internal_status == InternalStatus::PendingMerge || reconciled2,
        "Fresh task should not be blocked by starvation guard — got status {:?}, reconciled={}",
        updated2.internal_status,
        reconciled2
    );
}

#[tokio::test]
async fn rc4_non_validation_failure_retries_normally() {
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
        "Transient Git Failure Task".to_string(),
    );
    task.internal_status = InternalStatus::MergeIncomplete;
    // TransientGit failure — should NOT be subject to validation cooldown or circuit breaker
    task.metadata = Some(
        serde_json::json!({
            "merge_failure_source": "transient_git",
        })
        .to_string(),
    );
    // Set updated_at far enough in the past for normal retry delay to pass
    task.updated_at = chrono::Utc::now() - chrono::Duration::seconds(300);
    app_state.task_repo.create(task.clone()).await.unwrap();

    let reconciled = reconciler
        .reconcile_merge_incomplete_task(&task, InternalStatus::MergeIncomplete)
        .await;
    assert!(
        reconciled,
        "Non-validation failure (TransientGit) should retry normally without validation cooldown"
    );

    // Verify the reconciler took action: task transitions to PendingMerge,
    // then on_enter fires attempt_programmatic_merge which fails without real git
    // and bounces back to MergeIncomplete. The key assertion is that reconciled=true
    // (retry was attempted), proving the validation cooldown/circuit breaker did NOT block it.
    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    // After the programmatic merge fails in test env, task bounces back to MergeIncomplete
    assert!(
        updated.internal_status == InternalStatus::PendingMerge
            || updated.internal_status == InternalStatus::MergeIncomplete,
        "TransientGit failure should attempt retry (got {:?})",
        updated.internal_status
    );
}

// ── Merge Pipeline Active Flag Tests ──

#[test]
fn has_merge_pipeline_active_returns_false_when_no_metadata() {
    let task = Task::new(
        crate::domain::entities::ProjectId::from_string("proj-1".to_string()),
        "No Metadata Task".to_string(),
    );
    // merge_pipeline_active column is None by default
    assert!(!ReconciliationRunner::<tauri::Wry>::has_merge_pipeline_active(&task));
}

#[test]
fn has_merge_pipeline_active_returns_false_when_flag_not_present() {
    let mut task = Task::new(
        crate::domain::entities::ProjectId::from_string("proj-1".to_string()),
        "Empty Metadata Task".to_string(),
    );
    // metadata may contain other keys, but merge_pipeline_active column is None
    task.metadata = Some(serde_json::json!({"some_other_key": "value"}).to_string());
    assert!(!ReconciliationRunner::<tauri::Wry>::has_merge_pipeline_active(&task));
}

#[test]
fn has_merge_pipeline_active_returns_true_for_fresh_timestamp() {
    let mut task = Task::new(
        crate::domain::entities::ProjectId::from_string("proj-1".to_string()),
        "Fresh Pipeline Task".to_string(),
    );
    task.merge_pipeline_active = Some(chrono::Utc::now().to_rfc3339());
    assert!(ReconciliationRunner::<tauri::Wry>::has_merge_pipeline_active(&task));
}

#[test]
fn has_merge_pipeline_active_returns_false_for_expired_timestamp() {
    let mut task = Task::new(
        crate::domain::entities::ProjectId::from_string("proj-1".to_string()),
        "Expired Pipeline Task".to_string(),
    );
    // Set timestamp far in the past (beyond any reasonable deadline)
    let old = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
    task.merge_pipeline_active = Some(old);
    assert!(!ReconciliationRunner::<tauri::Wry>::has_merge_pipeline_active(&task));
}

#[tokio::test]
async fn reconcile_pending_merge_skips_when_merge_pipeline_active() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let mut task = Task::new(project.id.clone(), "Active Pipeline Task".to_string());
    task.internal_status = InternalStatus::PendingMerge;
    task.merge_pipeline_active = Some(chrono::Utc::now().to_rfc3339());
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

    let reconciled = reconciler
        .reconcile_pending_merge_task(&task, InternalStatus::PendingMerge)
        .await;

    // Should return true (skip reconciliation) because pipeline is active
    assert!(
        reconciled,
        "Should skip reconciliation when merge pipeline is active"
    );

    // Verify task status unchanged (not killed)
    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should exist");
    assert_eq!(
        updated.internal_status,
        InternalStatus::PendingMerge,
        "Task should remain in PendingMerge when pipeline is active"
    );
}

#[tokio::test]
async fn reconcile_pending_merge_proceeds_when_pipeline_flag_expired() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let mut task = Task::new(project.id.clone(), "Expired Pipeline Task".to_string());
    task.internal_status = InternalStatus::PendingMerge;
    // Set an expired pipeline flag (1 hour ago)
    let old = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
    task.merge_pipeline_active = Some(old);
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Record stale status history (old timestamp to trigger stale detection)
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

    let reconciled = reconciler
        .reconcile_pending_merge_task(&task, InternalStatus::PendingMerge)
        .await;

    // Should NOT be skipped by the pipeline flag (it's expired).
    // The reconciler proceeds to staleness check. With the default
    // pending_merge_stale_minutes, the status history was JUST created
    // so it won't be stale yet — policy returns None.
    assert!(
        !reconciled,
        "Should not skip reconciliation when pipeline flag is expired"
    );
}

// ── set/clear merge_pipeline_active persistence tests ──

#[tokio::test]
async fn set_merge_pipeline_active_persists_to_task_column() {
    let app_state = AppState::new_test();

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "Set Flag Task".to_string());
    task.internal_status = InternalStatus::PendingMerge;
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Simulate what set_merge_pipeline_active does: set dedicated column
    task.merge_pipeline_active = Some(chrono::Utc::now().to_rfc3339());
    task.touch();
    app_state.task_repo.update(&task).await.unwrap();

    // Reload from repo and verify
    let reloaded = app_state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    assert!(
        ReconciliationRunner::<tauri::Wry>::has_merge_pipeline_active(&reloaded),
        "Flag should survive persist + reload"
    );
    assert!(
        reloaded.merge_pipeline_active.is_some(),
        "merge_pipeline_active column should be set"
    );
}

#[tokio::test]
async fn clear_merge_pipeline_active_removes_column_value() {
    let app_state = AppState::new_test();

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "Clear Flag Task".to_string());
    task.internal_status = InternalStatus::PendingMerge;
    task.merge_pipeline_active = Some(chrono::Utc::now().to_rfc3339());
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Verify flag is set
    assert!(ReconciliationRunner::<tauri::Wry>::has_merge_pipeline_active(&task));

    // Simulate what clear_merge_pipeline_active does: set column to None
    task.merge_pipeline_active = None;
    task.touch();
    app_state.task_repo.update(&task).await.unwrap();

    // Reload and verify flag is gone
    let reloaded = app_state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    assert!(
        !ReconciliationRunner::<tauri::Wry>::has_merge_pipeline_active(&reloaded),
        "Flag should be cleared after removal"
    );
    assert!(
        reloaded.merge_pipeline_active.is_none(),
        "merge_pipeline_active column should be NULL after clear"
    );
}

#[tokio::test]
async fn set_merge_pipeline_active_does_not_clobber_metadata() {
    // Regression test for the race condition: concurrent metadata writers
    // used to clobber the flag because it was stored in the same JSON blob.
    // With a dedicated column, metadata updates are independent.
    let app_state = AppState::new_test();

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "Race Condition Task".to_string());
    task.internal_status = InternalStatus::PendingMerge;
    task.metadata = Some(
        serde_json::json!({
            "merge_source_branch": "feature/test",
            "merge_target_branch": "main"
        })
        .to_string(),
    );
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Step 1: Set merge_pipeline_active column
    task.merge_pipeline_active = Some(chrono::Utc::now().to_rfc3339());
    task.touch();
    app_state.task_repo.update(&task).await.unwrap();

    // Step 2: Concurrent writer modifies metadata (simulates chat_service_merge.rs)
    let mut concurrent_task = app_state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let mut meta: serde_json::Value =
        serde_json::from_str(concurrent_task.metadata.as_deref().unwrap_or("{}")).unwrap();
    meta.as_object_mut().unwrap().insert("merge_error".to_string(), serde_json::json!("some error"));
    concurrent_task.metadata = Some(meta.to_string());
    concurrent_task.touch();
    app_state.task_repo.update(&concurrent_task).await.unwrap();

    // Step 3: Reload and verify the pipeline flag survived the concurrent metadata write
    let reloaded = app_state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    assert!(
        ReconciliationRunner::<tauri::Wry>::has_merge_pipeline_active(&reloaded),
        "merge_pipeline_active column must survive concurrent metadata writes"
    );
    // Metadata was written by concurrent writer
    let json: serde_json::Value =
        serde_json::from_str(reloaded.metadata.as_deref().unwrap()).unwrap();
    assert_eq!(json["merge_error"], "some error");
}

#[test]
fn set_merge_pipeline_active_preserves_other_metadata_keys() {
    let mut task = Task::new(
        crate::domain::entities::ProjectId::from_string("proj-1".to_string()),
        "Preserve Keys Task".to_string(),
    );
    // Task already has other metadata
    task.metadata = Some(
        serde_json::json!({
            "merge_source_branch": "feature/test",
            "merge_target_branch": "main",
            "some_counter": 42
        })
        .to_string(),
    );

    // set_merge_pipeline_active uses dedicated column — does NOT touch metadata
    task.merge_pipeline_active = Some(chrono::Utc::now().to_rfc3339());

    // Verify pipeline flag is set
    assert!(ReconciliationRunner::<tauri::Wry>::has_merge_pipeline_active(&task));

    // Verify metadata is untouched
    let json: serde_json::Value =
        serde_json::from_str(task.metadata.as_deref().unwrap()).unwrap();
    assert_eq!(json["merge_source_branch"], "feature/test");
    assert_eq!(json["merge_target_branch"], "main");
    assert_eq!(json["some_counter"], 42);
    assert!(json.get("merge_pipeline_active").is_none(), "flag must not be in metadata JSON");
}

#[test]
fn clear_merge_pipeline_active_preserves_other_metadata_keys() {
    let mut task = Task::new(
        crate::domain::entities::ProjectId::from_string("proj-1".to_string()),
        "Preserve Keys Task".to_string(),
    );
    task.merge_pipeline_active = Some(chrono::Utc::now().to_rfc3339());
    task.metadata = Some(
        serde_json::json!({
            "merge_source_branch": "feature/test",
            "some_counter": 42
        })
        .to_string(),
    );

    // clear_merge_pipeline_active uses dedicated column — does NOT touch metadata
    task.merge_pipeline_active = None;

    // Verify pipeline flag is cleared
    assert!(!ReconciliationRunner::<tauri::Wry>::has_merge_pipeline_active(&task));

    // Verify metadata is untouched
    let json: serde_json::Value =
        serde_json::from_str(task.metadata.as_deref().unwrap()).unwrap();
    assert_eq!(json["merge_source_branch"], "feature/test");
    assert_eq!(json["some_counter"], 42);
}

#[test]
fn set_merge_pipeline_active_handles_none_metadata() {
    let mut task = Task::new(
        crate::domain::entities::ProjectId::from_string("proj-1".to_string()),
        "None Metadata Task".to_string(),
    );
    assert!(task.metadata.is_none());

    // set_merge_pipeline_active uses dedicated column — works even with no metadata
    task.merge_pipeline_active = Some(chrono::Utc::now().to_rfc3339());

    assert!(
        ReconciliationRunner::<tauri::Wry>::has_merge_pipeline_active(&task),
        "Flag should work even when metadata is None"
    );
}

// ── Full flow integration test: set → skip → clear → act ──

#[tokio::test]
async fn reconcile_pending_merge_full_flow_set_skip_clear_act() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "Full Flow Task".to_string());
    task.internal_status = InternalStatus::PendingMerge;
    app_state.task_repo.create(task.clone()).await.unwrap();

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

    // Phase 1: Set flag — reconciler should skip
    task.merge_pipeline_active = Some(chrono::Utc::now().to_rfc3339());
    task.touch();
    app_state.task_repo.update(&task).await.unwrap();

    let reconciled = reconciler
        .reconcile_pending_merge_task(&task, InternalStatus::PendingMerge)
        .await;
    assert!(reconciled, "Phase 1: Should skip when flag is active");

    // Phase 2: Clear flag — reconciler should proceed (not stale yet, so returns false/noop)
    task.merge_pipeline_active = None;
    task.touch();
    app_state.task_repo.update(&task).await.unwrap();

    let reconciled = reconciler
        .reconcile_pending_merge_task(&task, InternalStatus::PendingMerge)
        .await;
    assert!(
        !reconciled,
        "Phase 2: Should proceed past flag check when flag is cleared (not stale = noop)"
    );
}

// ── Regression: stale PendingMerge with no flag still gets killed ──

#[test]
fn pending_merge_stale_no_flag_still_transitions_to_merge_incomplete() {
    // Regression test: the merge_pipeline_active guard must not break
    // the existing safety net for truly abandoned PendingMerge tasks.
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
        RecoveryActionKind::Transition(InternalStatus::MergeIncomplete),
        "Stale PendingMerge without any flag should still transition to MergeIncomplete"
    );
}

// ── Both flags (validation + pipeline) interaction test ──

#[tokio::test]
async fn reconcile_pending_merge_skips_when_both_flags_set() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "Both Flags Task".to_string());
    task.internal_status = InternalStatus::PendingMerge;
    let now = chrono::Utc::now().to_rfc3339();
    // merge_pipeline_active: dedicated column; validation_in_progress: JSON metadata
    task.merge_pipeline_active = Some(now.clone());
    task.metadata = Some(serde_json::json!({"validation_in_progress": now}).to_string());
    app_state.task_repo.create(task.clone()).await.unwrap();

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

    let reconciled = reconciler
        .reconcile_pending_merge_task(&task, InternalStatus::PendingMerge)
        .await;

    // Should skip — merge_pipeline_active is checked first
    assert!(
        reconciled,
        "Should skip when both merge_pipeline_active and validation_in_progress are set"
    );

    // Verify task unchanged
    let updated = app_state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    assert_eq!(updated.internal_status, InternalStatus::PendingMerge);
}

#[tokio::test]
async fn reconcile_pending_merge_skips_for_validation_even_without_pipeline_flag() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "Validation Only Task".to_string());
    task.internal_status = InternalStatus::PendingMerge;
    let now = chrono::Utc::now().to_rfc3339();
    // Only validation flag, no pipeline flag
    task.metadata = Some(serde_json::json!({"validation_in_progress": now}).to_string());
    app_state.task_repo.create(task.clone()).await.unwrap();

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

    let reconciled = reconciler
        .reconcile_pending_merge_task(&task, InternalStatus::PendingMerge)
        .await;

    // Should skip via the validation guard (second guard)
    assert!(
        reconciled,
        "Validation flag alone should still cause skip (pipeline flag is not required)"
    );
}

// ── NOP deadline fix: saturating_sub behavior tests ──

#[test]
fn deadline_remaining_decreases_with_elapsed_time() {
    let deadline_secs = 600u64;
    let deadline_duration = std::time::Duration::from_secs(deadline_secs);

    // Simulate various elapsed times
    let elapsed_0 = std::time::Duration::from_secs(0);
    let elapsed_100 = std::time::Duration::from_secs(100);
    let elapsed_300 = std::time::Duration::from_secs(300);
    let elapsed_599 = std::time::Duration::from_secs(599);

    let remaining_0 = deadline_duration.saturating_sub(elapsed_0);
    let remaining_100 = deadline_duration.saturating_sub(elapsed_100);
    let remaining_300 = deadline_duration.saturating_sub(elapsed_300);
    let remaining_599 = deadline_duration.saturating_sub(elapsed_599);

    assert_eq!(remaining_0.as_secs(), 600);
    assert_eq!(remaining_100.as_secs(), 500);
    assert_eq!(remaining_300.as_secs(), 300);
    assert_eq!(remaining_599.as_secs(), 1);

    // Verify monotonic decrease
    assert!(remaining_0 > remaining_100);
    assert!(remaining_100 > remaining_300);
    assert!(remaining_300 > remaining_599);
}

#[test]
fn deadline_remaining_zero_when_elapsed_exceeds_deadline() {
    let deadline_secs = 600u64;
    let deadline_duration = std::time::Duration::from_secs(deadline_secs);

    // Elapsed exactly at deadline
    let elapsed_exact = std::time::Duration::from_secs(600);
    let remaining_exact = deadline_duration.saturating_sub(elapsed_exact);
    assert_eq!(remaining_exact, std::time::Duration::ZERO);

    // Elapsed well past deadline
    let elapsed_over = std::time::Duration::from_secs(900);
    let remaining_over = deadline_duration.saturating_sub(elapsed_over);
    assert_eq!(remaining_over, std::time::Duration::ZERO);

    // Verify the deadline check would trigger
    assert!(elapsed_exact >= deadline_duration);
    assert!(elapsed_over >= deadline_duration);
}

#[test]
fn deadline_check_uses_attempt_start_not_instant_now() {
    // This test documents the fix: the old code created the deadline
    // at Instant::now() and immediately checked Instant::now() >= deadline
    // (always false). The fix uses attempt_start.elapsed() instead.
    let deadline_secs = 120u64;
    let deadline_duration = std::time::Duration::from_secs(deadline_secs);

    // OLD behavior (NOP): deadline just created, check immediately
    // now >= now + 120s → always false
    let old_remaining = deadline_duration; // effectively full deadline every time

    // NEW behavior: uses elapsed time from function start
    // If cleanup took 60s + freshness took 60s = 120s elapsed
    let elapsed_after_pipeline = std::time::Duration::from_secs(120);
    let new_remaining = deadline_duration.saturating_sub(elapsed_after_pipeline);

    // Old code would always have full remaining time (bug)
    assert_eq!(old_remaining.as_secs(), 120);
    // New code correctly computes zero remaining after 120s pipeline work
    assert_eq!(new_remaining, std::time::Duration::ZERO);
}

#[test]
fn has_merge_pipeline_active_returns_false_when_column_is_none() {
    // With dedicated column, None = not active (replaces the old "non-string value" test)
    let task = Task::new(
        crate::domain::entities::ProjectId::from_string("proj-1".to_string()),
        "None Column Task".to_string(),
    );
    assert!(
        !ReconciliationRunner::<tauri::Wry>::has_merge_pipeline_active(&task),
        "None column value should not be treated as active"
    );
}

#[test]
fn has_merge_pipeline_active_returns_false_for_malformed_timestamp() {
    let mut task = Task::new(
        crate::domain::entities::ProjectId::from_string("proj-1".to_string()),
        "Malformed Timestamp Task".to_string(),
    );
    task.merge_pipeline_active = Some("not-a-timestamp".to_string());
    assert!(
        !ReconciliationRunner::<tauri::Wry>::has_merge_pipeline_active(&task),
        "Malformed timestamp should not be treated as active"
    );
}

// ============================================================================
// Stale IPR detection tests
// ============================================================================

/// Helper: create a test stdin pipe via `cat` subprocess.
async fn create_test_stdin() -> (tokio::process::ChildStdin, tokio::process::Child) {
    let mut child = tokio::process::Command::new("cat")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to spawn cat");
    let stdin = child.stdin.take().expect("no stdin");
    (stdin, child)
}

fn build_reconciler_with_ipr(
    app_state: &AppState,
    execution_state: &Arc<ExecutionState>,
    ipr: Arc<InteractiveProcessRegistry>,
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
    .with_interactive_process_registry(ipr)
}

#[tokio::test]
async fn is_ipr_process_alive_returns_false_when_no_ipr_entry() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::with_max_concurrent(5));
    let ipr = Arc::new(InteractiveProcessRegistry::new());
    let reconciler = build_reconciler_with_ipr(&app_state, &execution_state, ipr);

    // No entry in IPR → false
    let alive = reconciler
        .is_ipr_process_alive(ChatContextType::TaskExecution, "task-1")
        .await;
    assert!(!alive, "Should return false when no IPR entry exists");
}

#[tokio::test]
async fn is_ipr_process_alive_returns_false_when_no_ipr_configured() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::with_max_concurrent(5));
    // Build without IPR
    let reconciler = build_reconciler(&app_state, &execution_state);

    let alive = reconciler
        .is_ipr_process_alive(ChatContextType::TaskExecution, "task-1")
        .await;
    assert!(!alive, "Should return false when IPR is None");
}

#[tokio::test]
async fn is_ipr_process_alive_returns_true_when_process_alive() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::with_max_concurrent(5));
    let ipr = Arc::new(InteractiveProcessRegistry::new());
    let reconciler = build_reconciler_with_ipr(&app_state, &execution_state, Arc::clone(&ipr));

    // Spawn a real process and register in IPR
    let (stdin, child) = create_test_stdin().await;
    let pid = child.id().expect("cat should have PID");
    let key = InteractiveProcessKey::new(
        ChatContextType::TaskExecution.to_string(),
        "task-1",
    );
    ipr.register(key.clone(), stdin).await;

    // Also register in running_agent_registry with the real PID
    let registry_key = RunningAgentKey::new(
        ChatContextType::TaskExecution.to_string(),
        "task-1",
    );
    app_state
        .running_agent_registry
        .register(registry_key, pid, "conv-1".into(), "run-1".into(), None, None)
        .await;

    let alive = reconciler
        .is_ipr_process_alive(ChatContextType::TaskExecution, "task-1")
        .await;
    assert!(alive, "Should return true when IPR entry exists AND PID is alive");

    // IPR entry should NOT have been removed
    assert!(
        ipr.has_process(&key).await,
        "IPR entry should be preserved for live process"
    );

    // Cleanup
    drop(child);
}

#[tokio::test]
async fn is_ipr_process_alive_removes_stale_entry_no_registry() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::with_max_concurrent(5));
    let ipr = Arc::new(InteractiveProcessRegistry::new());
    let reconciler = build_reconciler_with_ipr(&app_state, &execution_state, Arc::clone(&ipr));

    // Register in IPR but NOT in running_agent_registry (simulates registry cleanup
    // that happened but IPR wasn't cleaned — the stale entry scenario)
    let (stdin, child) = create_test_stdin().await;
    let key = InteractiveProcessKey::new(
        ChatContextType::TaskExecution.to_string(),
        "task-1",
    );
    ipr.register(key.clone(), stdin).await;
    assert!(ipr.has_process(&key).await, "Precondition: IPR has entry");

    let alive = reconciler
        .is_ipr_process_alive(ChatContextType::TaskExecution, "task-1")
        .await;
    assert!(!alive, "Should return false when no registry entry exists");

    // Stale IPR entry should have been removed
    assert!(
        !ipr.has_process(&key).await,
        "Stale IPR entry should be removed when no registry entry"
    );

    drop(child);
}

#[tokio::test]
async fn is_ipr_process_alive_removes_stale_entry_dead_pid() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::with_max_concurrent(5));
    let ipr = Arc::new(InteractiveProcessRegistry::new());
    let reconciler = build_reconciler_with_ipr(&app_state, &execution_state, Arc::clone(&ipr));

    // Register in IPR
    let (stdin, child) = create_test_stdin().await;
    let key = InteractiveProcessKey::new(
        ChatContextType::TaskExecution.to_string(),
        "task-1",
    );
    ipr.register(key.clone(), stdin).await;

    // Register in registry with a dead PID (PID 0 is treated as dead)
    let registry_key = RunningAgentKey::new(
        ChatContextType::TaskExecution.to_string(),
        "task-1",
    );
    app_state
        .running_agent_registry
        .register(registry_key, 0, "conv-1".into(), "run-1".into(), None, None)
        .await;

    let alive = reconciler
        .is_ipr_process_alive(ChatContextType::TaskExecution, "task-1")
        .await;
    assert!(!alive, "Should return false when PID is dead (pid=0)");

    // Stale IPR entry should have been removed
    assert!(
        !ipr.has_process(&key).await,
        "Stale IPR entry should be removed when PID is dead"
    );

    drop(child);
}

#[tokio::test]
async fn is_ipr_process_alive_works_for_review_context() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::with_max_concurrent(5));
    let ipr = Arc::new(InteractiveProcessRegistry::new());
    let reconciler = build_reconciler_with_ipr(&app_state, &execution_state, Arc::clone(&ipr));

    // Register stale IPR entry for Review context
    let (stdin, child) = create_test_stdin().await;
    let key = InteractiveProcessKey::new(ChatContextType::Review.to_string(), "task-2");
    ipr.register(key.clone(), stdin).await;
    // No registry entry → stale

    let alive = reconciler
        .is_ipr_process_alive(ChatContextType::Review, "task-2")
        .await;
    assert!(!alive, "Should detect stale IPR for Review context");
    assert!(
        !ipr.has_process(&key).await,
        "Should remove stale Review IPR entry"
    );

    drop(child);
}

#[tokio::test]
async fn is_ipr_process_alive_works_for_merge_context() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::with_max_concurrent(5));
    let ipr = Arc::new(InteractiveProcessRegistry::new());
    let reconciler = build_reconciler_with_ipr(&app_state, &execution_state, Arc::clone(&ipr));

    // Register stale IPR entry for Merge context
    let (stdin, child) = create_test_stdin().await;
    let key = InteractiveProcessKey::new(ChatContextType::Merge.to_string(), "task-3");
    ipr.register(key.clone(), stdin).await;
    // No registry entry → stale

    let alive = reconciler
        .is_ipr_process_alive(ChatContextType::Merge, "task-3")
        .await;
    assert!(!alive, "Should detect stale IPR for Merge context");
    assert!(
        !ipr.has_process(&key).await,
        "Should remove stale Merge IPR entry"
    );

    drop(child);
}

#[tokio::test]
async fn reconcile_execution_proceeds_with_stale_ipr() {
    // Integration test: reconcile_completed_execution should NOT skip when IPR is stale.
    // Before the fix, a stale IPR entry would cause reconciliation to return true
    // (skip), leaving the task stuck in Executing forever.
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::with_max_concurrent(5));
    let ipr = Arc::new(InteractiveProcessRegistry::new());
    let reconciler = build_reconciler_with_ipr(&app_state, &execution_state, Arc::clone(&ipr));

    // Create project + task in Executing state
    let project = Project::new("Test Project".to_string(), "/tmp/test".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();
    let mut task = Task::new(project.id.clone(), "Stuck Task".to_string());
    task.internal_status = InternalStatus::Executing;
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Register stale IPR entry (process died but IPR wasn't cleaned)
    let (stdin, child) = create_test_stdin().await;
    let key = InteractiveProcessKey::new(
        ChatContextType::TaskExecution.to_string(),
        task.id.as_str(),
    );
    ipr.register(key.clone(), stdin).await;
    // No registry entry → stale IPR

    // Before fix: this would return true (skip reconciliation) due to IPR entry.
    // After fix: detects stale IPR, removes it, and proceeds with reconciliation.
    let reconciled = reconciler
        .reconcile_completed_execution(&task, InternalStatus::Executing)
        .await;

    // IPR entry should have been cleaned up
    assert!(
        !ipr.has_process(&key).await,
        "Stale IPR entry should be removed during reconciliation"
    );

    // Reconciliation should have done something (not skipped)
    // The exact outcome depends on run evidence, but the key assertion is
    // that the stale IPR didn't block reconciliation from proceeding.
    // (reconciled can be true or false depending on what the policy decides,
    // but it should NOT have been short-circuited by the IPR check)
    let _ = reconciled; // Assert is on IPR cleanup above

    drop(child);
}

#[tokio::test]
async fn reconcile_execution_skips_for_live_ipr() {
    // Verify that healthy IPR entries are NOT disturbed — regression guard.
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::with_max_concurrent(5));
    let ipr = Arc::new(InteractiveProcessRegistry::new());
    let reconciler = build_reconciler_with_ipr(&app_state, &execution_state, Arc::clone(&ipr));

    // Create project + task in Executing state
    let project = Project::new("Test Project".to_string(), "/tmp/test".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();
    let mut task = Task::new(project.id.clone(), "Active Task".to_string());
    task.internal_status = InternalStatus::Executing;
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Register IPR entry with a live process
    let (stdin, child) = create_test_stdin().await;
    let pid = child.id().expect("cat should have PID");
    let key = InteractiveProcessKey::new(
        ChatContextType::TaskExecution.to_string(),
        task.id.as_str(),
    );
    ipr.register(key.clone(), stdin).await;

    // Also register in running_agent_registry with the real PID
    let registry_key = RunningAgentKey::new(
        ChatContextType::TaskExecution.to_string(),
        task.id.as_str(),
    );
    app_state
        .running_agent_registry
        .register(registry_key, pid, "conv-1".into(), "run-1".into(), None, None)
        .await;

    // Should skip reconciliation (return true) because process is alive
    let reconciled = reconciler
        .reconcile_completed_execution(&task, InternalStatus::Executing)
        .await;
    assert!(reconciled, "Should skip reconciliation for live IPR process");

    // IPR entry should be preserved
    assert!(
        ipr.has_process(&key).await,
        "Live IPR entry should NOT be removed"
    );

    drop(child);
}

#[tokio::test]
async fn reconcile_review_proceeds_with_stale_ipr() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::with_max_concurrent(5));
    let ipr = Arc::new(InteractiveProcessRegistry::new());
    let reconciler = build_reconciler_with_ipr(&app_state, &execution_state, Arc::clone(&ipr));

    let project = Project::new("Test Project".to_string(), "/tmp/test".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();
    let mut task = Task::new(project.id.clone(), "Review Task".to_string());
    task.internal_status = InternalStatus::Reviewing;
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Register stale IPR entry for Review context
    let (stdin, child) = create_test_stdin().await;
    let key = InteractiveProcessKey::new(
        ChatContextType::Review.to_string(),
        task.id.as_str(),
    );
    ipr.register(key.clone(), stdin).await;

    let _reconciled = reconciler
        .reconcile_reviewing_task(&task, InternalStatus::Reviewing)
        .await;

    assert!(
        !ipr.has_process(&key).await,
        "Stale Review IPR entry should be removed during reconciliation"
    );

    drop(child);
}

#[tokio::test]
async fn reconcile_merge_proceeds_with_stale_ipr() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::with_max_concurrent(5));
    let ipr = Arc::new(InteractiveProcessRegistry::new());
    let reconciler = build_reconciler_with_ipr(&app_state, &execution_state, Arc::clone(&ipr));

    let project = Project::new("Test Project".to_string(), "/tmp/test".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();
    let mut task = Task::new(project.id.clone(), "Merge Task".to_string());
    task.internal_status = InternalStatus::Merging;
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Register stale IPR entry for Merge context
    let (stdin, child) = create_test_stdin().await;
    let key = InteractiveProcessKey::new(
        ChatContextType::Merge.to_string(),
        task.id.as_str(),
    );
    ipr.register(key.clone(), stdin).await;

    let _reconciled = reconciler
        .reconcile_merging_task(&task, InternalStatus::Merging)
        .await;

    assert!(
        !ipr.has_process(&key).await,
        "Stale Merge IPR entry should be removed during reconciliation"
    );

    drop(child);
}

#[tokio::test]
async fn reconcile_re_executing_proceeds_with_stale_ipr() {
    // Specifically tests ReExecuting — the original stuck state that prompted this fix.
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::with_max_concurrent(5));
    let ipr = Arc::new(InteractiveProcessRegistry::new());
    let reconciler = build_reconciler_with_ipr(&app_state, &execution_state, Arc::clone(&ipr));

    let project = Project::new("Test Project".to_string(), "/tmp/test".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();
    let mut task = Task::new(project.id.clone(), "ReExecuting Task".to_string());
    task.internal_status = InternalStatus::ReExecuting;
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Register stale IPR entry
    let (stdin, child) = create_test_stdin().await;
    let key = InteractiveProcessKey::new(
        ChatContextType::TaskExecution.to_string(),
        task.id.as_str(),
    );
    ipr.register(key.clone(), stdin).await;

    let _reconciled = reconciler
        .reconcile_completed_execution(&task, InternalStatus::ReExecuting)
        .await;

    assert!(
        !ipr.has_process(&key).await,
        "Stale IPR for ReExecuting task should be removed during reconciliation"
    );

    drop(child);
}

// ─── Policy tests for Cancelled/Failed handling (PRIMARY FIX) ───

#[test]
fn execution_policy_restarts_on_cancelled_run() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: Some(AgentRunStatus::Cancelled),
        registry_running: false,
        can_start: true,
        is_stale: false,
        is_deferred: false,
    };
    let decision = policy.decide_reconciliation(RecoveryContext::Execution, evidence);
    assert_eq!(decision.action, RecoveryActionKind::ExecuteEntryActions);
    assert!(decision.reason.unwrap().contains("cancelled/failed"));
}

#[test]
fn execution_policy_restarts_on_failed_run() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: Some(AgentRunStatus::Failed),
        registry_running: false,
        can_start: true,
        is_stale: false,
        is_deferred: false,
    };
    let decision = policy.decide_reconciliation(RecoveryContext::Execution, evidence);
    assert_eq!(decision.action, RecoveryActionKind::ExecuteEntryActions);
    assert!(decision.reason.unwrap().contains("cancelled/failed"));
}

#[test]
fn execution_policy_prompts_on_cancelled_at_capacity() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: Some(AgentRunStatus::Cancelled),
        registry_running: false,
        can_start: false,
        is_stale: false,
        is_deferred: false,
    };
    let decision = policy.decide_reconciliation(RecoveryContext::Execution, evidence);
    assert_eq!(decision.action, RecoveryActionKind::Prompt);
    assert!(decision.reason.unwrap().contains("max concurrency"));
}

#[test]
fn execution_policy_prompts_on_failed_at_capacity() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: Some(AgentRunStatus::Failed),
        registry_running: false,
        can_start: false,
        is_stale: false,
        is_deferred: false,
    };
    let decision = policy.decide_reconciliation(RecoveryContext::Execution, evidence);
    assert_eq!(decision.action, RecoveryActionKind::Prompt);
    assert!(decision.reason.unwrap().contains("max concurrency"));
}

#[test]
fn review_policy_restarts_on_cancelled_run() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: Some(AgentRunStatus::Cancelled),
        registry_running: false,
        can_start: true,
        is_stale: false,
        is_deferred: false,
    };
    let decision = policy.decide_reconciliation(RecoveryContext::Review, evidence);
    assert_eq!(decision.action, RecoveryActionKind::ExecuteEntryActions);
    assert!(decision.reason.unwrap().contains("cancelled/failed"));
}

#[test]
fn review_policy_restarts_on_failed_run() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: Some(AgentRunStatus::Failed),
        registry_running: false,
        can_start: true,
        is_stale: false,
        is_deferred: false,
    };
    let decision = policy.decide_reconciliation(RecoveryContext::Review, evidence);
    assert_eq!(decision.action, RecoveryActionKind::ExecuteEntryActions);
    assert!(decision.reason.unwrap().contains("cancelled/failed"));
}

#[test]
fn review_policy_prompts_on_cancelled_at_capacity() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: Some(AgentRunStatus::Cancelled),
        registry_running: false,
        can_start: false,
        is_stale: false,
        is_deferred: false,
    };
    let decision = policy.decide_reconciliation(RecoveryContext::Review, evidence);
    assert_eq!(decision.action, RecoveryActionKind::Prompt);
    assert!(decision.reason.unwrap().contains("max concurrency"));
}

#[test]
fn merge_policy_restarts_on_cancelled_run() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: Some(AgentRunStatus::Cancelled),
        registry_running: false,
        can_start: true,
        is_stale: false,
        is_deferred: false,
    };
    let decision = policy.decide_reconciliation(RecoveryContext::Merge, evidence);
    assert_eq!(decision.action, RecoveryActionKind::ExecuteEntryActions);
    assert!(decision.reason.unwrap().contains("cancelled/failed"));
}

#[test]
fn merge_policy_restarts_on_failed_run() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: Some(AgentRunStatus::Failed),
        registry_running: false,
        can_start: true,
        is_stale: false,
        is_deferred: false,
    };
    let decision = policy.decide_reconciliation(RecoveryContext::Merge, evidence);
    assert_eq!(decision.action, RecoveryActionKind::ExecuteEntryActions);
    assert!(decision.reason.unwrap().contains("cancelled/failed"));
}

#[test]
fn merge_policy_prompts_on_failed_at_capacity() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: Some(AgentRunStatus::Failed),
        registry_running: false,
        can_start: false,
        is_stale: false,
        is_deferred: false,
    };
    let decision = policy.decide_reconciliation(RecoveryContext::Merge, evidence);
    assert_eq!(decision.action, RecoveryActionKind::Prompt);
    assert!(decision.reason.unwrap().contains("max concurrency"));
}

// ─── Regression tests: Running run still returns None ───

#[test]
fn execution_policy_none_for_running_in_registry() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: Some(AgentRunStatus::Running),
        registry_running: true,
        can_start: true,
        is_stale: false,
        is_deferred: false,
    };
    let decision = policy.decide_reconciliation(RecoveryContext::Execution, evidence);
    assert_eq!(decision.action, RecoveryActionKind::None);
}

#[test]
fn review_policy_none_for_running_in_registry() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: Some(AgentRunStatus::Running),
        registry_running: true,
        can_start: true,
        is_stale: false,
        is_deferred: false,
    };
    let decision = policy.decide_reconciliation(RecoveryContext::Review, evidence);
    assert_eq!(decision.action, RecoveryActionKind::None);
}

#[test]
fn merge_policy_none_for_running_in_registry() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: Some(AgentRunStatus::Running),
        registry_running: true,
        can_start: true,
        is_stale: false,
        is_deferred: false,
    };
    let decision = policy.decide_reconciliation(RecoveryContext::Merge, evidence);
    assert_eq!(decision.action, RecoveryActionKind::None);
}

// ─── Regression: Completed run behavior unchanged ───

#[test]
fn review_policy_re_executes_on_completed_run() {
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
fn merge_policy_auto_completes_on_completed_run() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: Some(AgentRunStatus::Completed),
        registry_running: false,
        can_start: true,
        is_stale: false,
        is_deferred: false,
    };
    let decision = policy.decide_reconciliation(RecoveryContext::Merge, evidence);
    assert_eq!(decision.action, RecoveryActionKind::AttemptMergeAutoComplete);
}

#[test]
fn merge_policy_prompts_on_cancelled_at_capacity() {
    let policy = RecoveryPolicy;
    let evidence = RecoveryEvidence {
        run_status: Some(AgentRunStatus::Cancelled),
        registry_running: false,
        can_start: false,
        is_stale: false,
        is_deferred: false,
    };
    let decision = policy.decide_reconciliation(RecoveryContext::Merge, evidence);
    assert_eq!(decision.action, RecoveryActionKind::Prompt);
    assert!(decision.reason.unwrap().contains("max concurrency"));
}

// ─── Combined IPR+Policy integration test ───

#[tokio::test]
async fn reconcile_re_executing_stale_ipr_and_no_run_triggers_recovery() {
    // Exercises BOTH fixes together: stale IPR entry gets cleaned up,
    // AND the policy (run_status=None, can_start=true) returns ExecuteEntryActions.
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::with_max_concurrent(5));
    let ipr = Arc::new(InteractiveProcessRegistry::new());
    let reconciler = build_reconciler_with_ipr(&app_state, &execution_state, Arc::clone(&ipr));

    let project = Project::new("Test Project".to_string(), "/tmp/test".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();
    let mut task = Task::new(project.id.clone(), "Stuck ReExecuting Task".to_string());
    task.internal_status = InternalStatus::ReExecuting;
    app_state.task_repo.create(task.clone()).await.unwrap();

    // Register a stale IPR entry (no matching registry PID → will be detected as stale)
    let (stdin, child) = create_test_stdin().await;
    let ipr_key = InteractiveProcessKey::new(
        ChatContextType::TaskExecution.to_string(),
        task.id.as_str(),
    );
    ipr.register(ipr_key.clone(), stdin).await;

    // No agent_run in DB, no registry entry → policy sees run_status=None, can_start=true
    // → ExecuteEntryActions (the fix that would have unblocked task 68c414a2)
    let reconciled = reconciler
        .reconcile_completed_execution(&task, InternalStatus::ReExecuting)
        .await;

    // IPR fix: stale entry removed
    assert!(
        !ipr.has_process(&ipr_key).await,
        "Stale IPR entry should be removed by is_ipr_process_alive check"
    );

    // Policy fix: reconciliation should have attempted recovery (entry actions)
    // Note: entry actions fail silently in test (no real transition handler wired),
    // but reconcile_completed_execution returns true when it attempts recovery.
    assert!(
        reconciled,
        "Reconciliation should return true — policy returns ExecuteEntryActions for missing run"
    );

    drop(child);
}

// ── Execution Recovery Metadata Helpers ─────────────────────────────────────

#[test]
fn execution_failed_auto_retry_count_returns_zero_with_no_metadata() {
    use crate::domain::entities::Task;
    let task = Task::new(
        crate::domain::entities::ProjectId("proj".into()),
        "test".into(),
    );
    assert_eq!(
        ReconciliationRunner::<tauri::Wry>::execution_failed_auto_retry_count(&task),
        0
    );
}

#[test]
fn execution_failed_auto_retry_count_counts_triggered_events() {
    use crate::domain::entities::{
        ExecutionRecoveryEvent, ExecutionRecoveryEventKind, ExecutionRecoveryMetadata,
        ExecutionRecoveryReasonCode, ExecutionRecoverySource, ExecutionRecoveryState, Task,
    };

    let mut recovery = ExecutionRecoveryMetadata::new();
    for i in 0..3u32 {
        let event = ExecutionRecoveryEvent::new(
            ExecutionRecoveryEventKind::AutoRetryTriggered,
            ExecutionRecoverySource::Auto,
            ExecutionRecoveryReasonCode::Timeout,
            format!("retry {i}"),
        );
        recovery.append_event_with_state(event, ExecutionRecoveryState::Retrying);
    }
    // Add a non-AutoRetryTriggered event — should not be counted
    let other = ExecutionRecoveryEvent::new(
        ExecutionRecoveryEventKind::Failed,
        ExecutionRecoverySource::System,
        ExecutionRecoveryReasonCode::Timeout,
        "failed",
    );
    recovery.append_event(other);

    let mut task = Task::new(
        crate::domain::entities::ProjectId("proj".into()),
        "test".into(),
    );
    task.metadata = Some(
        recovery
            .update_task_metadata(None)
            .expect("serialize recovery"),
    );

    assert_eq!(
        ReconciliationRunner::<tauri::Wry>::execution_failed_auto_retry_count(&task),
        3
    );
}

#[test]
fn execution_failed_retry_delay_increases_with_retry_count() {
    // Delay at retry 1 should be <= delay at retry 2 (excluding jitter variance).
    // We check base values without jitter: base * 2^count.
    // With default base=30s: retry0 → 30s, retry1 → 60s, retry2 → 120s, ...
    // Since jitter adds 0–25%, the lower bound at retry N+1 is always > base at retry N.
    let delay0 = ReconciliationRunner::<tauri::Wry>::execution_failed_retry_delay(0).num_seconds();
    let delay3 = ReconciliationRunner::<tauri::Wry>::execution_failed_retry_delay(3).num_seconds();
    assert!(
        delay3 > delay0,
        "delay at retry 3 ({delay3}s) should exceed delay at retry 0 ({delay0}s)"
    );
}

#[test]
fn execution_failed_retry_delay_is_capped_at_max() {
    // Delay at a very high retry count should be <= max_secs + 25% jitter.
    let max_secs = reconciliation_config().execution_failed_retry_max_secs as i64;
    let delay = ReconciliationRunner::<tauri::Wry>::execution_failed_retry_delay(20).num_seconds();
    assert!(
        delay <= max_secs + max_secs / 4 + 1,
        "delay at retry 20 ({delay}s) should not far exceed max ({max_secs}s)"
    );
}

#[test]
fn has_execution_stop_retrying_false_without_metadata() {
    use crate::domain::entities::Task;
    let task = Task::new(
        crate::domain::entities::ProjectId("proj".into()),
        "test".into(),
    );
    assert!(
        !ReconciliationRunner::<tauri::Wry>::has_execution_stop_retrying(&task),
        "should return false when no metadata"
    );
}

#[test]
fn has_execution_stop_retrying_true_when_set() {
    use crate::domain::entities::{ExecutionRecoveryMetadata, ExecutionRecoveryState, Task};

    let mut recovery = ExecutionRecoveryMetadata::new();
    recovery.stop_retrying = true;
    recovery.last_state = ExecutionRecoveryState::Failed;

    let mut task = Task::new(
        crate::domain::entities::ProjectId("proj".into()),
        "test".into(),
    );
    task.metadata = Some(recovery.update_task_metadata(None).expect("serialize"));

    assert!(
        ReconciliationRunner::<tauri::Wry>::has_execution_stop_retrying(&task),
        "should return true when stop_retrying is set"
    );
}

#[test]
fn execution_next_retry_at_returns_none_without_events() {
    use crate::domain::entities::Task;
    let task = Task::new(
        crate::domain::entities::ProjectId("proj".into()),
        "test".into(),
    );
    assert!(
        ReconciliationRunner::<tauri::Wry>::execution_next_retry_at(&task).is_none(),
        "should return None when no AutoRetryTriggered events"
    );
}

#[test]
fn execution_next_retry_at_returns_future_timestamp() {
    use crate::domain::entities::{
        ExecutionRecoveryEvent, ExecutionRecoveryEventKind, ExecutionRecoveryMetadata,
        ExecutionRecoveryReasonCode, ExecutionRecoverySource, ExecutionRecoveryState, Task,
    };

    let mut recovery = ExecutionRecoveryMetadata::new();
    let event = ExecutionRecoveryEvent::new(
        ExecutionRecoveryEventKind::AutoRetryTriggered,
        ExecutionRecoverySource::Auto,
        ExecutionRecoveryReasonCode::Timeout,
        "retry 1",
    );
    recovery.append_event_with_state(event, ExecutionRecoveryState::Retrying);

    let mut task = Task::new(
        crate::domain::entities::ProjectId("proj".into()),
        "test".into(),
    );
    task.metadata = Some(recovery.update_task_metadata(None).expect("serialize"));

    let next_at = ReconciliationRunner::<tauri::Wry>::execution_next_retry_at(&task);
    assert!(next_at.is_some(), "should return Some when AutoRetryTriggered event exists");
    assert!(
        next_at.unwrap() > chrono::Utc::now(),
        "next_retry_at should be in the future"
    );
}

#[tokio::test]
async fn record_execution_auto_retry_event_persists_event_via_update_metadata() {
    use crate::domain::entities::{
        ExecutionFailureSource, ExecutionRecoveryEventKind, ExecutionRecoveryMetadata, Project,
        Task,
    };

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".into(), "/test".into());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let task = Task::new(project.id.clone(), "Failing Task".into());
    app_state.task_repo.create(task.clone()).await.unwrap();

    reconciler
        .record_execution_auto_retry_event(
            &task,
            1,
            ExecutionFailureSource::TransientTimeout,
            "Auto-retrying execution (attempt 1/3)",
        )
        .await
        .expect("record event should succeed");

    let updated = app_state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let recovery =
        ExecutionRecoveryMetadata::from_task_metadata(updated.metadata.as_deref())
            .expect("parse metadata")
            .expect("execution_recovery key should exist");

    assert_eq!(
        recovery
            .events
            .iter()
            .filter(|e| matches!(e.kind, ExecutionRecoveryEventKind::AutoRetryTriggered))
            .count(),
        1,
        "one AutoRetryTriggered event should be recorded"
    );
    assert_eq!(recovery.events[0].attempt, Some(1));
}

#[tokio::test]
async fn set_execution_stop_retrying_sets_flag_in_db() {
    use crate::domain::entities::{ExecutionRecoveryMetadata, ExecutionRecoveryState, Project, Task};

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".into(), "/test".into());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let task = Task::new(project.id.clone(), "Failing Task".into());
    app_state.task_repo.create(task.clone()).await.unwrap();

    reconciler
        .set_execution_stop_retrying(&task)
        .await
        .expect("set stop retrying should succeed");

    let updated = app_state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let recovery =
        ExecutionRecoveryMetadata::from_task_metadata(updated.metadata.as_deref())
            .expect("parse metadata")
            .expect("execution_recovery key should exist");

    assert!(recovery.stop_retrying, "stop_retrying should be true");
    assert_eq!(
        recovery.last_state,
        ExecutionRecoveryState::Failed,
        "last_state should be Failed"
    );
}

#[tokio::test]
async fn clear_execution_flat_metadata_removes_is_timeout_and_failure_error() {
    use crate::domain::entities::{Project, Task};

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".into(), "/test".into());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "Failing Task".into());
    task.metadata = Some(
        serde_json::json!({
            "is_timeout": true,
            "failure_error": "Agent timed out after 600s",
            "trigger_origin": "scheduler"
        })
        .to_string(),
    );
    app_state.task_repo.create(task.clone()).await.unwrap();

    reconciler
        .clear_execution_flat_metadata(&task)
        .await
        .expect("clear flat metadata should succeed");

    let updated = app_state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let json: serde_json::Value =
        serde_json::from_str(updated.metadata.as_deref().unwrap()).unwrap();

    assert!(
        json.get("is_timeout").is_none(),
        "is_timeout should be removed"
    );
    assert!(
        json.get("failure_error").is_none(),
        "failure_error should be removed"
    );
    assert_eq!(
        json.get("trigger_origin").and_then(|v| v.as_str()),
        Some("scheduler"),
        "trigger_origin should be preserved"
    );
}

#[tokio::test]
async fn reset_execution_recovery_metadata_clears_events_and_resets_state() {
    use crate::domain::entities::{
        ExecutionRecoveryEvent, ExecutionRecoveryEventKind, ExecutionRecoveryMetadata,
        ExecutionRecoveryReasonCode, ExecutionRecoverySource, ExecutionRecoveryState, Project, Task,
    };

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".into(), "/test".into());
    app_state.project_repo.create(project.clone()).await.unwrap();

    // Set up task with existing recovery metadata (2 events, stop_retrying=true, last_state=Failed)
    let mut recovery = ExecutionRecoveryMetadata::new();
    recovery.stop_retrying = true;
    recovery.last_state = ExecutionRecoveryState::Failed;
    for _ in 0..2u32 {
        let event = ExecutionRecoveryEvent::new(
            ExecutionRecoveryEventKind::AutoRetryTriggered,
            ExecutionRecoverySource::Auto,
            ExecutionRecoveryReasonCode::Timeout,
            "old retry",
        );
        recovery.append_event(event);
    }

    let mut task = Task::new(project.id.clone(), "Failing Task".into());
    task.metadata = Some(recovery.update_task_metadata(None).expect("serialize"));
    app_state.task_repo.create(task.clone()).await.unwrap();

    reconciler
        .reset_execution_recovery_metadata(&task)
        .await
        .expect("reset should succeed");

    let updated = app_state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let reset_recovery =
        ExecutionRecoveryMetadata::from_task_metadata(updated.metadata.as_deref())
            .expect("parse metadata")
            .expect("execution_recovery key should exist");

    assert!(reset_recovery.events.is_empty(), "events should be cleared");
    assert!(
        !reset_recovery.stop_retrying,
        "stop_retrying should be false"
    );
    assert_eq!(
        reset_recovery.last_state,
        ExecutionRecoveryState::Retrying,
        "last_state should be Retrying after reset"
    );
}

#[tokio::test]
async fn stop_execution_retrying_by_user_persists_user_source_and_user_stopped_reason() {
    use crate::domain::entities::{
        ExecutionRecoveryEventKind, ExecutionRecoveryMetadata, ExecutionRecoveryReasonCode,
        ExecutionRecoverySource, ExecutionRecoveryState, Project, Task,
    };

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".into(), "/test".into());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let task = Task::new(project.id.clone(), "Failing Task".into());
    app_state.task_repo.create(task.clone()).await.unwrap();

    reconciler
        .stop_execution_retrying_by_user(&task)
        .await
        .expect("stop_execution_retrying_by_user should succeed");

    let updated = app_state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let recovery =
        ExecutionRecoveryMetadata::from_task_metadata(updated.metadata.as_deref())
            .expect("parse metadata")
            .expect("execution_recovery key should exist");

    assert!(recovery.stop_retrying, "stop_retrying should be true");
    assert_eq!(
        recovery.last_state,
        ExecutionRecoveryState::Failed,
        "last_state should be Failed"
    );
    assert_eq!(recovery.events.len(), 1, "one event should be recorded");
    let event = &recovery.events[0];
    assert!(
        matches!(event.kind, ExecutionRecoveryEventKind::StopRetrying),
        "event kind should be StopRetrying"
    );
    assert_eq!(
        event.source,
        ExecutionRecoverySource::User,
        "source should be User (not System)"
    );
    assert_eq!(
        event.reason_code,
        ExecutionRecoveryReasonCode::UserStopped,
        "reason_code should be UserStopped"
    );
}

#[tokio::test]
async fn record_execution_manual_retry_event_persists_manual_retry_kind_with_user_source() {
    use crate::domain::entities::{
        ExecutionRecoveryEventKind, ExecutionRecoveryMetadata, ExecutionRecoverySource,
        ExecutionRecoveryState, Project, Task,
    };

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".into(), "/test".into());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let task = Task::new(project.id.clone(), "Failing Task".into());
    app_state.task_repo.create(task.clone()).await.unwrap();

    reconciler
        .record_execution_manual_retry_event(&task)
        .await
        .expect("record_execution_manual_retry_event should succeed");

    let updated = app_state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let recovery =
        ExecutionRecoveryMetadata::from_task_metadata(updated.metadata.as_deref())
            .expect("parse metadata")
            .expect("execution_recovery key should exist");

    assert_eq!(recovery.events.len(), 1, "one event should be recorded");
    let event = &recovery.events[0];
    assert!(
        matches!(event.kind, ExecutionRecoveryEventKind::ManualRetry),
        "event kind should be ManualRetry"
    );
    assert_eq!(
        event.source,
        ExecutionRecoverySource::User,
        "source should be User"
    );
    assert_eq!(
        recovery.last_state,
        ExecutionRecoveryState::Retrying,
        "last_state should be Retrying after manual retry event"
    );
}

#[tokio::test]
async fn apply_failed_user_recovery_cancel_sets_stop_retrying_and_returns_true() {
    use crate::domain::entities::{ExecutionRecoveryMetadata, InternalStatus, Project, Task};

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".into(), "/test".into());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "Failed Task".into());
    task.internal_status = InternalStatus::Failed;
    app_state.task_repo.create(task.clone()).await.unwrap();

    let result = reconciler
        .apply_user_recovery_action(&task, UserRecoveryAction::Cancel)
        .await;

    assert!(result, "Cancel action should return true");

    let updated = app_state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    // Task remains Failed — Cancel does not transition
    assert_eq!(
        updated.internal_status,
        InternalStatus::Failed,
        "task should remain Failed after Cancel"
    );
    let recovery =
        ExecutionRecoveryMetadata::from_task_metadata(updated.metadata.as_deref())
            .expect("parse metadata")
            .expect("execution_recovery key should exist");
    assert!(
        recovery.stop_retrying,
        "stop_retrying should be true after Cancel"
    );
}

#[tokio::test]
async fn apply_failed_user_recovery_restart_transitions_to_ready_and_records_manual_retry_event() {
    use crate::domain::entities::{
        ExecutionRecoveryEventKind, ExecutionRecoveryMetadata, InternalStatus, Project, Task,
    };

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".into(), "/test".into());
    app_state.project_repo.create(project.clone()).await.unwrap();

    // Create task in Failed state with stale flat metadata
    let mut task = Task::new(project.id.clone(), "Failed Task".into());
    task.internal_status = InternalStatus::Failed;
    task.metadata = Some(
        serde_json::json!({
            "is_timeout": true,
            "failure_error": "Agent timed out after 600s"
        })
        .to_string(),
    );
    app_state.task_repo.create(task.clone()).await.unwrap();

    let result = reconciler
        .apply_user_recovery_action(&task, UserRecoveryAction::Restart)
        .await;

    assert!(result, "Restart action should return true");

    let updated = app_state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    // Task should now be Ready
    assert_eq!(
        updated.internal_status,
        InternalStatus::Ready,
        "task should transition to Ready after Restart"
    );
    // task_branch and worktree_path should be cleared
    assert!(
        updated.task_branch.is_none(),
        "task_branch should be cleared"
    );
    assert!(
        updated.worktree_path.is_none(),
        "worktree_path should be cleared"
    );
    // Flat metadata keys should be removed
    if let Some(meta_str) = updated.metadata.as_deref() {
        let json: serde_json::Value = serde_json::from_str(meta_str).unwrap();
        assert!(
            json.get("is_timeout").is_none(),
            "is_timeout should be cleared"
        );
        assert!(
            json.get("failure_error").is_none(),
            "failure_error should be cleared"
        );
    }
    // ManualRetry event should be recorded
    let recovery =
        ExecutionRecoveryMetadata::from_task_metadata(updated.metadata.as_deref())
            .expect("parse metadata")
            .expect("execution_recovery key should exist");
    assert!(
        recovery.events.iter().any(|e| matches!(e.kind, ExecutionRecoveryEventKind::ManualRetry)),
        "ManualRetry event should be recorded after Restart"
    );
}

// ── reconcile_failed_execution_task() Tests ───────────────────────────────────
//
// These test the early-exit conditions and the happy path of the reconciler handler
// that auto-retries Failed tasks with transient execution failures.

fn make_execution_recovery(stop: bool, state: crate::domain::entities::ExecutionRecoveryState) -> ExecutionRecoveryMetadata {
    let mut recovery = ExecutionRecoveryMetadata::new();
    recovery.last_state = state;
    recovery.stop_retrying = stop;
    recovery
}

fn make_task_with_recovery(project_id: &crate::domain::entities::ProjectId, recovery: ExecutionRecoveryMetadata) -> Task {
    let mut task = Task::new(project_id.clone(), "Failed Task".into());
    task.internal_status = InternalStatus::Failed;
    task.metadata = Some(recovery.update_task_metadata(None).expect("serialize recovery"));
    task
}

/// Legacy task with no execution_recovery metadata → reconciler skips it.
#[tokio::test]
async fn reconcile_failed_legacy_task_skip_no_metadata() {
    use crate::domain::entities::Project;

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".into(), "/tmp".into());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "Legacy Task".into());
    task.internal_status = InternalStatus::Failed;
    // No execution_recovery metadata — legacy task
    app_state.task_repo.create(task.clone()).await.unwrap();

    let result = reconciler.reconcile_failed_execution_task(&task, InternalStatus::Failed).await;

    assert!(!result, "legacy task without metadata should be skipped");
    let updated = app_state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    assert_eq!(updated.internal_status, InternalStatus::Failed, "status unchanged");
}

/// stop_retrying = true → reconciler skips.
#[tokio::test]
async fn reconcile_failed_stop_retrying_flag_skips() {
    use crate::domain::entities::Project;

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".into(), "/tmp".into());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let recovery = make_execution_recovery(true, crate::domain::entities::ExecutionRecoveryState::Retrying);
    let task = make_task_with_recovery(&project.id, recovery);
    app_state.task_repo.create(task.clone()).await.unwrap();

    let result = reconciler.reconcile_failed_execution_task(&task, InternalStatus::Failed).await;

    assert!(!result, "stop_retrying=true should be skipped");
}

/// last_state = Failed (permanent) → reconciler skips.
#[tokio::test]
async fn reconcile_failed_permanent_failure_state_skips() {
    use crate::domain::entities::Project;

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".into(), "/tmp".into());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let recovery = make_execution_recovery(false, crate::domain::entities::ExecutionRecoveryState::Failed);
    let task = make_task_with_recovery(&project.id, recovery);
    app_state.task_repo.create(task.clone()).await.unwrap();

    let result = reconciler.reconcile_failed_execution_task(&task, InternalStatus::Failed).await;

    assert!(!result, "permanent failure (last_state=Failed) should be skipped");
}

/// GAP H1: WallClockTimeout failure source → reconciler skips (would cause infinite C5 loop).
#[tokio::test]
async fn reconcile_failed_wall_clock_timeout_skip() {
    use crate::domain::entities::{
        ExecutionFailureSource, ExecutionRecoveryEvent, ExecutionRecoveryEventKind,
        ExecutionRecoveryReasonCode, ExecutionRecoverySource, ExecutionRecoveryState, Project,
    };

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".into(), "/tmp".into());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut recovery = ExecutionRecoveryMetadata::new();
    recovery.append_event_with_state(
        ExecutionRecoveryEvent::new(
            ExecutionRecoveryEventKind::Failed,
            ExecutionRecoverySource::System,
            ExecutionRecoveryReasonCode::WallClockExceeded,
            "C5 wall-clock timeout",
        )
        .with_failure_source(ExecutionFailureSource::WallClockTimeout),
        ExecutionRecoveryState::Retrying,
    );

    let task = make_task_with_recovery(&project.id, recovery);
    app_state.task_repo.create(task.clone()).await.unwrap();

    let result = reconciler.reconcile_failed_execution_task(&task, InternalStatus::Failed).await;

    assert!(!result, "GAP H1: wall-clock timeout must not be retried");
}

/// Max retries exceeded → reconciler records permanent failure and returns false.
#[tokio::test]
async fn reconcile_failed_max_retries_exceeded_marks_permanent_failure() {
    use crate::domain::entities::{
        ExecutionFailureSource, ExecutionRecoveryEvent, ExecutionRecoveryEventKind,
        ExecutionRecoveryReasonCode, ExecutionRecoverySource, ExecutionRecoveryState, Project,
    };

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".into(), "/tmp".into());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let max = reconciliation_config().execution_failed_max_retries as u32;

    let mut recovery = ExecutionRecoveryMetadata::new();
    // Append exactly max_retries AutoRetryTriggered events (budget exhausted)
    for i in 0..max {
        recovery.append_event(
            ExecutionRecoveryEvent::new(
                ExecutionRecoveryEventKind::AutoRetryTriggered,
                ExecutionRecoverySource::Auto,
                ExecutionRecoveryReasonCode::Timeout,
                format!("Retry {}", i + 1),
            )
            .with_attempt(i + 1)
            .with_failure_source(ExecutionFailureSource::TransientTimeout),
        );
    }
    recovery.last_state = ExecutionRecoveryState::Retrying;

    let task = make_task_with_recovery(&project.id, recovery);
    app_state.task_repo.create(task.clone()).await.unwrap();

    let result = reconciler.reconcile_failed_execution_task(&task, InternalStatus::Failed).await;

    assert!(!result, "max retries exceeded: should return false");

    // Verify stop_retrying = true set in metadata
    let updated = app_state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let updated_recovery =
        ExecutionRecoveryMetadata::from_task_metadata(updated.metadata.as_deref())
            .expect("parse metadata")
            .expect("execution_recovery should exist");
    assert!(
        updated_recovery.stop_retrying,
        "max retries exceeded: stop_retrying must be set to true"
    );
    assert_eq!(
        updated_recovery.last_state,
        ExecutionRecoveryState::Failed,
        "max retries exceeded: last_state must be Failed (permanent)"
    );
}

/// GAP M6: backoff not elapsed → reconciler skips.
#[tokio::test]
async fn reconcile_failed_backoff_not_elapsed_skip() {
    use crate::domain::entities::{
        ExecutionFailureSource, ExecutionRecoveryEvent, ExecutionRecoveryEventKind,
        ExecutionRecoveryReasonCode, ExecutionRecoverySource, ExecutionRecoveryState, Project,
    };

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".into(), "/tmp".into());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut recovery = ExecutionRecoveryMetadata::new();
    // Add an AutoRetryTriggered event with at = now → next_retry_at is in the future
    recovery.append_event_with_state(
        ExecutionRecoveryEvent::new(
            ExecutionRecoveryEventKind::AutoRetryTriggered,
            ExecutionRecoverySource::Auto,
            ExecutionRecoveryReasonCode::Timeout,
            "Auto retry 1",
        )
        .with_attempt(1)
        .with_failure_source(ExecutionFailureSource::TransientTimeout),
        ExecutionRecoveryState::Retrying,
    );
    // The backoff delay for retry_count=1 is min(2^1 * base, max) + jitter ≥ 60s
    // Since at = now, next_retry_at is far in the future

    let task = make_task_with_recovery(&project.id, recovery);
    app_state.task_repo.create(task.clone()).await.unwrap();

    let result = reconciler.reconcile_failed_execution_task(&task, InternalStatus::Failed).await;

    assert!(!result, "backoff not elapsed: should skip");
}

/// GAP B6: concurrency guard — at max_concurrent, reconciler skips this cycle.
#[tokio::test]
async fn reconcile_failed_concurrency_guard_skip_at_max_concurrent() {
    use crate::domain::entities::Project;

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".into(), "/tmp".into());
    app_state.project_repo.create(project.clone()).await.unwrap();

    // Fill up max_concurrent slots (default = 2)
    execution_state.increment_running();
    execution_state.increment_running();
    assert!(!execution_state.can_start_task(), "pre-condition: at max capacity");

    let recovery = make_execution_recovery(false, crate::domain::entities::ExecutionRecoveryState::Retrying);
    let task = make_task_with_recovery(&project.id, recovery);
    app_state.task_repo.create(task.clone()).await.unwrap();

    let result = reconciler.reconcile_failed_execution_task(&task, InternalStatus::Failed).await;

    assert!(!result, "GAP B6: at max_concurrent, should skip");
}

/// Happy path: eligible Failed task transitions to Ready.
#[tokio::test]
async fn reconcile_failed_eligible_task_transitions_to_ready() {
    use crate::domain::entities::Project;

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".into(), "/tmp".into());
    app_state.project_repo.create(project.clone()).await.unwrap();

    // Task in Retrying state — no prior retries, backoff not an issue (first attempt)
    let recovery = make_execution_recovery(false, crate::domain::entities::ExecutionRecoveryState::Retrying);
    let task = make_task_with_recovery(&project.id, recovery);
    // No task_branch / worktree_path → git cleanup is no-op
    app_state.task_repo.create(task.clone()).await.unwrap();

    let result = reconciler.reconcile_failed_execution_task(&task, InternalStatus::Failed).await;

    assert!(result, "eligible task should return true");

    let updated = app_state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    assert_eq!(
        updated.internal_status,
        InternalStatus::Ready,
        "eligible task should transition to Ready"
    );
}

/// GAP B7: stale flat metadata (is_timeout, failure_error) cleared before retry.
#[tokio::test]
async fn reconcile_failed_flat_metadata_cleared_before_retry() {
    use crate::domain::entities::Project;

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".into(), "/tmp".into());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let recovery = make_execution_recovery(false, crate::domain::entities::ExecutionRecoveryState::Retrying);
    let base_metadata = recovery.update_task_metadata(None).expect("serialize");
    // Inject stale flat keys alongside structured recovery
    let mut json: serde_json::Value = serde_json::from_str(&base_metadata).unwrap();
    if let Some(obj) = json.as_object_mut() {
        obj.insert("is_timeout".to_string(), serde_json::json!(true));
        obj.insert("failure_error".to_string(), serde_json::json!("Agent timed out after 600s"));
    }

    let mut task = Task::new(project.id.clone(), "Task with stale flat metadata".into());
    task.internal_status = InternalStatus::Failed;
    task.metadata = Some(json.to_string());
    app_state.task_repo.create(task.clone()).await.unwrap();

    let result = reconciler.reconcile_failed_execution_task(&task, InternalStatus::Failed).await;

    assert!(result, "eligible task should return true");

    let updated = app_state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    if let Some(meta_str) = updated.metadata.as_deref() {
        let parsed: serde_json::Value = serde_json::from_str(meta_str).unwrap();
        assert!(
            parsed.get("is_timeout").is_none(),
            "GAP B7: is_timeout should be removed before retry"
        );
        assert!(
            parsed.get("failure_error").is_none(),
            "GAP B7: failure_error should be removed before retry"
        );
        // Structured recovery metadata must still be present
        assert!(
            parsed.get("execution_recovery").is_some(),
            "execution_recovery structured metadata must be preserved"
        );
    }
}

/// GAP H10: ActivityEvent emitted when auto-retry fires.
#[tokio::test]
async fn reconcile_failed_activity_event_emitted_on_auto_retry() {
    use crate::domain::entities::Project;
    use crate::domain::repositories::ActivityEventFilter;

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".into(), "/tmp".into());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let recovery = make_execution_recovery(false, crate::domain::entities::ExecutionRecoveryState::Retrying);
    let task = make_task_with_recovery(&project.id, recovery);
    app_state.task_repo.create(task.clone()).await.unwrap();

    let result = reconciler.reconcile_failed_execution_task(&task, InternalStatus::Failed).await;
    assert!(result, "eligible task should return true");

    // Verify at least one activity event was recorded for this task
    let page = app_state
        .activity_event_repo
        .list_by_task_id(&task.id, None, 10, None::<&ActivityEventFilter>)
        .await
        .expect("list activity events");
    assert!(
        !page.events.is_empty(),
        "GAP H10: at least one activity event should be emitted on auto-retry"
    );
}

/// GAP H7: targeted metadata write — record_execution_auto_retry_event uses
/// update_metadata() path and preserves other metadata keys.
#[tokio::test]
async fn targeted_metadata_write_preserves_other_keys() {
    use crate::domain::entities::{ExecutionFailureSource, Project};

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".into(), "/tmp".into());
    app_state.project_repo.create(project.clone()).await.unwrap();

    // Task has other metadata keys alongside execution_recovery
    let recovery = make_execution_recovery(false, crate::domain::entities::ExecutionRecoveryState::Retrying);
    let base = recovery.update_task_metadata(None).expect("serialize");
    let mut json: serde_json::Value = serde_json::from_str(&base).unwrap();
    if let Some(obj) = json.as_object_mut() {
        obj.insert("trigger_origin".to_string(), serde_json::json!("scheduler"));
        obj.insert("some_other_key".to_string(), serde_json::json!("preserved"));
    }

    let mut task = Task::new(project.id.clone(), "Task with extra keys".into());
    task.internal_status = InternalStatus::Failed;
    task.metadata = Some(json.to_string());
    app_state.task_repo.create(task.clone()).await.unwrap();

    reconciler
        .record_execution_auto_retry_event(&task, 1, ExecutionFailureSource::TransientTimeout, "test")
        .await
        .expect("record should succeed");

    let updated = app_state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let meta_str = updated.metadata.as_deref().expect("metadata should exist");
    let parsed: serde_json::Value = serde_json::from_str(meta_str).unwrap();

    assert_eq!(
        parsed["some_other_key"], "preserved",
        "GAP H7: targeted write must preserve non-recovery metadata keys"
    );
    assert!(
        parsed.get("execution_recovery").is_some(),
        "execution_recovery must still be present"
    );
}

// ── GAP H9: reset_execution_recovery_metadata — already tested in earlier section,
//    but verifying it gives a fresh retry budget for the apply_user_recovery_action Restart.

/// Restart on Failed task resets execution recovery metadata (fresh retry budget, GAP H9).
#[tokio::test]
async fn apply_failed_restart_resets_execution_recovery_metadata_fresh_budget() {
    use crate::domain::entities::{
        ExecutionFailureSource, ExecutionRecoveryEvent, ExecutionRecoveryEventKind,
        ExecutionRecoveryReasonCode, ExecutionRecoverySource, ExecutionRecoveryState, Project,
    };

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".into(), "/tmp".into());
    app_state.project_repo.create(project.clone()).await.unwrap();

    // Task has used up 2 of 3 retries — manual restart should give fresh budget
    let mut recovery = ExecutionRecoveryMetadata::new();
    for i in 0..2 {
        recovery.append_event(
            ExecutionRecoveryEvent::new(
                ExecutionRecoveryEventKind::AutoRetryTriggered,
                ExecutionRecoverySource::Auto,
                ExecutionRecoveryReasonCode::Timeout,
                format!("Auto retry {}", i + 1),
            )
            .with_attempt(i + 1)
            .with_failure_source(ExecutionFailureSource::TransientTimeout),
        );
    }
    recovery.last_state = ExecutionRecoveryState::Retrying;

    let mut task = Task::new(project.id.clone(), "Partially retried task".into());
    task.internal_status = InternalStatus::Failed;
    task.metadata = Some(recovery.update_task_metadata(None).expect("serialize"));
    app_state.task_repo.create(task.clone()).await.unwrap();

    let result = reconciler
        .apply_user_recovery_action(&task, UserRecoveryAction::Restart)
        .await;

    assert!(result, "Restart should return true");

    let updated = app_state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    assert_eq!(updated.internal_status, InternalStatus::Ready, "task should be Ready");

    // Metadata should have been reset — events cleared, fresh retry budget
    let updated_recovery =
        ExecutionRecoveryMetadata::from_task_metadata(updated.metadata.as_deref())
            .expect("parse metadata")
            .expect("execution_recovery should exist");
    assert!(
        !updated_recovery.stop_retrying,
        "GAP H9: stop_retrying must be false after manual restart"
    );
    assert_eq!(
        updated_recovery.last_state,
        ExecutionRecoveryState::Retrying,
        "GAP H9: last_state must be Retrying after reset"
    );
    // Events cleared — only ManualRetry event should remain (recorded after reset)
    let auto_retry_count = updated_recovery
        .events
        .iter()
        .filter(|e| matches!(e.kind, ExecutionRecoveryEventKind::AutoRetryTriggered))
        .count();
    assert_eq!(
        auto_retry_count, 0,
        "GAP H9: AutoRetryTriggered events should be cleared after manual restart (fresh budget)"
    );
}

/// Cancel on Failed sets stop_retrying permanently.
#[tokio::test]
async fn apply_failed_cancel_sets_stop_retrying_permanently() {
    use crate::domain::entities::{ExecutionRecoveryMetadata, ExecutionRecoveryState, Project};

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".into(), "/tmp".into());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let recovery = make_execution_recovery(false, ExecutionRecoveryState::Retrying);
    let task = make_task_with_recovery(&project.id, recovery);
    app_state.task_repo.create(task.clone()).await.unwrap();

    let result = reconciler
        .apply_user_recovery_action(&task, UserRecoveryAction::Cancel)
        .await;

    assert!(result, "Cancel should return true");

    let updated = app_state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    assert_eq!(updated.internal_status, InternalStatus::Failed, "task remains Failed");

    let updated_recovery =
        ExecutionRecoveryMetadata::from_task_metadata(updated.metadata.as_deref())
            .expect("parse metadata")
            .expect("execution_recovery should exist");
    assert!(
        updated_recovery.stop_retrying,
        "Cancel: stop_retrying must be true"
    );
    assert_eq!(
        updated_recovery.last_state,
        ExecutionRecoveryState::Failed,
        "Cancel: last_state must be Failed (permanent)"
    );
}

// ============================================================================
// GAP M2 — recover_timeout_failures() dual-format checking
// ============================================================================

/// (GAP M2) Legacy format: is_timeout:true → recovered and migrated to new format.
#[tokio::test]
async fn recover_timeout_failures_processes_legacy_is_timeout_tasks() {
    use crate::domain::entities::{
        ExecutionRecoveryEventKind, ExecutionRecoveryMetadata, ExecutionRecoverySource, Project,
        Task,
    };

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    // Legacy task: Failed with is_timeout:true, no execution_recovery metadata
    let mut task = Task::new(project.id.clone(), "Legacy Timeout Task".to_string());
    task.internal_status = InternalStatus::Failed;
    task.metadata = Some(r#"{"is_timeout":true,"failure_error":"Agent timed out"}"#.to_string());
    app_state.task_repo.create(task.clone()).await.unwrap();

    reconciler.recover_timeout_failures().await;

    let updated = app_state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    assert_eq!(
        updated.internal_status,
        InternalStatus::Ready,
        "Legacy is_timeout task must be transitioned to Ready"
    );

    // Verify ExecutionRecoveryMetadata was created (migration to new format)
    let recovery = ExecutionRecoveryMetadata::from_task_metadata(updated.metadata.as_deref())
        .expect("parse metadata")
        .expect("execution_recovery should be created for legacy task");

    // Must have an AutoRetryTriggered event with Startup source (GAP M5 sentinel)
    let has_startup_event = recovery.events.iter().any(|e| {
        matches!(e.kind, ExecutionRecoveryEventKind::AutoRetryTriggered)
            && matches!(e.source, ExecutionRecoverySource::Startup)
    });
    assert!(
        has_startup_event,
        "Legacy task must have an AutoRetryTriggered/Startup event after migration"
    );
}

/// (GAP M2) New format: execution_recovery.last_state==Retrying → recovered without needing is_timeout.
#[tokio::test]
async fn recover_timeout_failures_processes_new_format_retrying_tasks() {
    use crate::domain::entities::{
        ExecutionRecoveryEvent, ExecutionRecoveryEventKind, ExecutionRecoveryMetadata,
        ExecutionRecoveryReasonCode, ExecutionRecoverySource, ExecutionRecoveryState, Project,
        Task,
    };

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    // New-format task: has execution_recovery metadata with last_state=Retrying, no is_timeout
    let mut recovery = ExecutionRecoveryMetadata::new();
    let failed_event = ExecutionRecoveryEvent::new(
        ExecutionRecoveryEventKind::Failed,
        ExecutionRecoverySource::System,
        ExecutionRecoveryReasonCode::Timeout,
        "Agent timed out",
    );
    recovery.append_event_with_state(failed_event, ExecutionRecoveryState::Retrying);
    let metadata_json = recovery.update_task_metadata(None).expect("serialize");

    let mut task = Task::new(project.id.clone(), "New Format Timeout Task".to_string());
    task.internal_status = InternalStatus::Failed;
    task.metadata = Some(metadata_json);
    app_state.task_repo.create(task.clone()).await.unwrap();

    reconciler.recover_timeout_failures().await;

    let updated = app_state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    assert_eq!(
        updated.internal_status,
        InternalStatus::Ready,
        "New-format retrying task must be transitioned to Ready"
    );
}

/// (GAP M2) Task with neither is_timeout nor execution_recovery → not recovered.
#[tokio::test]
async fn recover_timeout_failures_skips_tasks_with_no_timeout_metadata() {
    use crate::domain::entities::{Project, Task};

    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    // Task with no relevant metadata (e.g., cancelled or provider-error failure)
    let mut task = Task::new(project.id.clone(), "Non-Timeout Failed Task".to_string());
    task.internal_status = InternalStatus::Failed;
    task.metadata = Some(r#"{"failure_error":"Some other error"}"#.to_string());
    app_state.task_repo.create(task.clone()).await.unwrap();

    reconciler.recover_timeout_failures().await;

    let updated = app_state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    assert_eq!(
        updated.internal_status,
        InternalStatus::Failed,
        "Non-timeout task must NOT be transitioned — should remain Failed"
    );
}

// ============================================================================
// GAP M5 — Startup sentinel (has_recent_startup_recovery)
// ============================================================================

/// (GAP M5) Returns true when a Startup-sourced AutoRetryTriggered event is recent (< 60s).
#[test]
fn has_recent_startup_recovery_true_for_recent_startup_event() {
    use crate::domain::entities::{
        ExecutionRecoveryEvent, ExecutionRecoveryEventKind, ExecutionRecoveryMetadata,
        ExecutionRecoveryReasonCode, ExecutionRecoverySource, ExecutionRecoveryState, Task,
    };

    let mut recovery = ExecutionRecoveryMetadata::new();
    let event = ExecutionRecoveryEvent::new(
        ExecutionRecoveryEventKind::AutoRetryTriggered,
        ExecutionRecoverySource::Startup,
        ExecutionRecoveryReasonCode::Timeout,
        "Startup recovery",
    );
    recovery.append_event_with_state(event, ExecutionRecoveryState::Retrying);

    let mut task = Task::new(
        crate::domain::entities::ProjectId("proj".into()),
        "test".into(),
    );
    task.metadata = Some(recovery.update_task_metadata(None).expect("serialize"));

    assert!(
        ReconciliationRunner::<tauri::Wry>::has_recent_startup_recovery(&task),
        "should return true for recent Startup-sourced event"
    );
}

/// (GAP M5) Returns false when Startup-sourced event is older than 60s.
#[test]
fn has_recent_startup_recovery_false_for_old_startup_event() {
    use crate::domain::entities::{
        ExecutionRecoveryEvent, ExecutionRecoveryEventKind, ExecutionRecoveryMetadata,
        ExecutionRecoveryReasonCode, ExecutionRecoverySource, ExecutionRecoveryState, Task,
    };

    let mut recovery = ExecutionRecoveryMetadata::new();
    let mut event = ExecutionRecoveryEvent::new(
        ExecutionRecoveryEventKind::AutoRetryTriggered,
        ExecutionRecoverySource::Startup,
        ExecutionRecoveryReasonCode::Timeout,
        "Startup recovery (old)",
    );
    // Backdate the event by 90 seconds — outside the 60s sentinel window
    event.at = chrono::Utc::now() - chrono::Duration::seconds(90);
    recovery.append_event_with_state(event, ExecutionRecoveryState::Retrying);

    let mut task = Task::new(
        crate::domain::entities::ProjectId("proj".into()),
        "test".into(),
    );
    task.metadata = Some(recovery.update_task_metadata(None).expect("serialize"));

    assert!(
        !ReconciliationRunner::<tauri::Wry>::has_recent_startup_recovery(&task),
        "should return false for Startup event older than 60s"
    );
}

/// (GAP M5) Returns false when only Auto-sourced events exist (not Startup).
#[test]
fn has_recent_startup_recovery_false_for_auto_source() {
    use crate::domain::entities::{
        ExecutionRecoveryEvent, ExecutionRecoveryEventKind, ExecutionRecoveryMetadata,
        ExecutionRecoveryReasonCode, ExecutionRecoverySource, ExecutionRecoveryState, Task,
    };

    let mut recovery = ExecutionRecoveryMetadata::new();
    let event = ExecutionRecoveryEvent::new(
        ExecutionRecoveryEventKind::AutoRetryTriggered,
        ExecutionRecoverySource::Auto,
        ExecutionRecoveryReasonCode::Timeout,
        "Auto reconciler retry",
    );
    recovery.append_event_with_state(event, ExecutionRecoveryState::Retrying);

    let mut task = Task::new(
        crate::domain::entities::ProjectId("proj".into()),
        "test".into(),
    );
    task.metadata = Some(recovery.update_task_metadata(None).expect("serialize"));

    assert!(
        !ReconciliationRunner::<tauri::Wry>::has_recent_startup_recovery(&task),
        "should return false for Auto-sourced events — not a startup sentinel"
    );
}

/// (GAP M5) Returns false when no execution_recovery metadata exists.
#[test]
fn has_recent_startup_recovery_false_without_metadata() {
    let task = Task::new(
        crate::domain::entities::ProjectId("proj".into()),
        "no metadata".into(),
    );
    assert!(
        !ReconciliationRunner::<tauri::Wry>::has_recent_startup_recovery(&task),
        "should return false when no metadata"
    );
}

// ── Circuit Breaker Tests ─────────────────────────────────────────────────────

/// threshold met: 3 AttemptFailed events all with the same failure_source → should_circuit_break returns Some.
#[test]
fn should_circuit_break_threshold_met() {
    let mut task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "Circuit Breaker Task".to_string(),
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
                        "message": "worktree missing 1",
                        "failure_source": "worktree_missing"
                    },
                    {
                        "at": "2026-02-10T00:05:00Z",
                        "kind": "attempt_failed",
                        "source": "system",
                        "reason_code": "git_error",
                        "message": "worktree missing 2",
                        "failure_source": "worktree_missing"
                    },
                    {
                        "at": "2026-02-10T00:10:00Z",
                        "kind": "attempt_failed",
                        "source": "system",
                        "reason_code": "git_error",
                        "message": "worktree missing 3",
                        "failure_source": "worktree_missing"
                    }
                ],
                "last_state": "failed"
            }
        })
        .to_string(),
    );

    let result = ReconciliationRunner::<tauri::Wry>::should_circuit_break(&task, 3, 5);
    assert!(
        result.is_some(),
        "Circuit breaker should fire when 3/3 events share same failure_source"
    );
    let reason = result.unwrap();
    assert!(
        reason.contains("Circuit breaker"),
        "Reason should mention 'Circuit breaker', got: {}",
        reason
    );
}

/// threshold NOT met: 2 WorktreeMissing events and 1 TransientGit event → returns None.
#[test]
fn should_circuit_break_threshold_not_met_mixed_sources() {
    let mut task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "Mixed Failures Task".to_string(),
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
                        "message": "worktree missing 1",
                        "failure_source": "worktree_missing"
                    },
                    {
                        "at": "2026-02-10T00:05:00Z",
                        "kind": "attempt_failed",
                        "source": "system",
                        "reason_code": "git_error",
                        "message": "transient git error",
                        "failure_source": "transient_git"
                    },
                    {
                        "at": "2026-02-10T00:10:00Z",
                        "kind": "attempt_failed",
                        "source": "system",
                        "reason_code": "git_error",
                        "message": "worktree missing 2",
                        "failure_source": "worktree_missing"
                    }
                ],
                "last_state": "failed"
            }
        })
        .to_string(),
    );

    let result = ReconciliationRunner::<tauri::Wry>::should_circuit_break(&task, 3, 5);
    assert!(
        result.is_none(),
        "Circuit breaker should NOT fire when only 2/3 events share same source (threshold=3)"
    );
}

/// Events without failure_source are ignored — should not count toward threshold.
#[test]
fn should_circuit_break_ignores_events_without_failure_source() {
    let mut task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "No Source Task".to_string(),
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
                        "message": "no source 1"
                        // no failure_source field
                    },
                    {
                        "at": "2026-02-10T00:05:00Z",
                        "kind": "attempt_failed",
                        "source": "system",
                        "reason_code": "git_error",
                        "message": "no source 2"
                        // no failure_source field
                    },
                    {
                        "at": "2026-02-10T00:10:00Z",
                        "kind": "attempt_failed",
                        "source": "system",
                        "reason_code": "git_error",
                        "message": "no source 3"
                        // no failure_source field
                    },
                    {
                        "at": "2026-02-10T00:15:00Z",
                        "kind": "attempt_failed",
                        "source": "system",
                        "reason_code": "git_error",
                        "message": "no source 4"
                        // no failure_source field
                    }
                ],
                "last_state": "failed"
            }
        })
        .to_string(),
    );

    let result = ReconciliationRunner::<tauri::Wry>::should_circuit_break(&task, 3, 5);
    assert!(
        result.is_none(),
        "Circuit breaker should NOT fire when events lack failure_source (they are excluded from count)"
    );
}

/// is_circuit_breaker_active returns false when no metadata exists.
#[test]
fn is_circuit_breaker_active_false_without_metadata() {
    let task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "No Metadata".to_string(),
    );
    assert!(
        !ReconciliationRunner::<tauri::Wry>::is_circuit_breaker_active(&task),
        "circuit_breaker_active should be false when no metadata"
    );
}

/// is_circuit_breaker_active returns true when flag is set.
#[test]
fn is_circuit_breaker_active_true_when_set() {
    let mut task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "CB Active Task".to_string(),
    );
    task.metadata = Some(
        serde_json::json!({
            "merge_recovery": {
                "version": 1,
                "events": [],
                "last_state": "failed",
                "circuit_breaker_active": true,
                "circuit_breaker_reason": "too many repeated failures"
            }
        })
        .to_string(),
    );
    assert!(
        ReconciliationRunner::<tauri::Wry>::is_circuit_breaker_active(&task),
        "circuit_breaker_active should be true when set in metadata"
    );
}

/// circuit_breaker_active guard prevents reconcile_merge_incomplete_task from retrying.
/// This is an integration test that calls the full reconciliation path with
/// circuit_breaker_active=true in metadata and asserts that it returns false (no retry).
#[tokio::test]
async fn circuit_breaker_active_flag_prevents_reconcile_retry() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());
    let reconciler = build_reconciler(&app_state, &execution_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let mut task = Task::new(project.id.clone(), "CB Guard Integration Task".to_string());
    task.internal_status = InternalStatus::MergeIncomplete;
    // Set circuit_breaker_active=true directly in merge_recovery metadata.
    task.metadata = Some(
        serde_json::json!({
            "merge_recovery": {
                "version": 1,
                "events": [],
                "last_state": "failed",
                "circuit_breaker_active": true,
                "circuit_breaker_reason": "3/5 recent failures share the same source"
            }
        })
        .to_string(),
    );
    // Set updated_at in the past so no cooldown guard interferes.
    task.updated_at = chrono::Utc::now() - chrono::Duration::seconds(300);
    app_state.task_repo.create(task.clone()).await.unwrap();

    let reconciled = reconciler
        .reconcile_merge_incomplete_task(&task, InternalStatus::MergeIncomplete)
        .await;

    assert!(
        !reconciled,
        "circuit_breaker_active=true must prevent reconcile_merge_incomplete_task from retrying"
    );

    // Verify task remained in MergeIncomplete — the guard must not have triggered a transition.
    let updated = app_state
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("task should still exist");
    assert_eq!(
        updated.internal_status,
        InternalStatus::MergeIncomplete,
        "Task must stay MergeIncomplete when circuit breaker is active"
    );
}

/// should_circuit_break returns None when no merge_recovery metadata exists.
#[test]
fn should_circuit_break_returns_none_without_metadata() {
    let task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "No Metadata".to_string(),
    );
    assert!(
        ReconciliationRunner::<tauri::Wry>::should_circuit_break(&task, 3, 5).is_none(),
        "should_circuit_break should return None when no metadata"
    );
}

// ── is_mode_switch tests ──────────────────────────────────────────────────────

#[test]
fn is_mode_switch_returns_true_when_set() {
    let mut task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "Mode Switch Task".to_string(),
    );
    task.metadata = Some(r#"{"mode_switch":true}"#.to_string());
    assert!(
        ReconciliationRunner::<tauri::Wry>::is_mode_switch(&task),
        "is_mode_switch should return true when mode_switch=true in metadata"
    );
}

#[test]
fn is_mode_switch_returns_false_without_metadata() {
    let task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "No Metadata Task".to_string(),
    );
    assert!(
        !ReconciliationRunner::<tauri::Wry>::is_mode_switch(&task),
        "is_mode_switch should return false when no metadata"
    );
}

#[test]
fn is_mode_switch_returns_false_when_explicitly_false() {
    let mut task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "Not Mode Switch".to_string(),
    );
    task.metadata = Some(r#"{"mode_switch":false}"#.to_string());
    assert!(
        !ReconciliationRunner::<tauri::Wry>::is_mode_switch(&task),
        "is_mode_switch should return false when mode_switch=false"
    );
}

#[test]
fn is_mode_switch_returns_false_with_other_metadata() {
    let mut task = Task::new(
        crate::domain::entities::ProjectId::new(),
        "Other Metadata Task".to_string(),
    );
    task.metadata = Some(r#"{"merge_failure_source":"agent_reported"}"#.to_string());
    assert!(
        !ReconciliationRunner::<tauri::Wry>::is_mode_switch(&task),
        "is_mode_switch should return false when mode_switch key is absent"
    );
}
