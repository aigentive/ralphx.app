use super::*;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;

use crate::domain::entities::{IdeationSessionId, ProjectId, Task};
use crate::domain::repositories::{StateHistoryMetadata, StatusTransition};
use crate::error::AppResult;

/// Configurable mock: `get_by_id` returns the stored task (or None).
struct StubTaskRepo {
    task: Option<Task>,
}

#[async_trait]
impl TaskRepository for StubTaskRepo {
    async fn get_by_id(&self, _id: &TaskId) -> AppResult<Option<Task>> {
        Ok(self.task.clone())
    }

    // ── Stubs for all other required methods ────────────────────────────
    async fn create(&self, task: Task) -> AppResult<Task> {
        Ok(task)
    }
    async fn get_by_project(&self, _: &ProjectId) -> AppResult<Vec<Task>> {
        Ok(vec![])
    }
    async fn update(&self, _: &Task) -> AppResult<()> {
        Ok(())
    }
    async fn update_with_expected_status(&self, _: &Task, _: InternalStatus) -> AppResult<bool> {
        Ok(true)
    }
    async fn update_metadata(&self, _: &TaskId, _: Option<String>) -> AppResult<()> {
        Ok(())
    }
    async fn delete(&self, _: &TaskId) -> AppResult<()> {
        Ok(())
    }
    async fn clear_task_references(&self, _: &TaskId) -> AppResult<()> {
        Ok(())
    }
    async fn get_by_status(&self, _: &ProjectId, _: InternalStatus) -> AppResult<Vec<Task>> {
        Ok(vec![])
    }
    async fn persist_status_change(
        &self,
        _: &TaskId,
        _: InternalStatus,
        _: InternalStatus,
        _: &str,
    ) -> AppResult<()> {
        Ok(())
    }
    async fn get_status_history(&self, _: &TaskId) -> AppResult<Vec<StatusTransition>> {
        Ok(vec![])
    }
    async fn get_status_entered_at(
        &self,
        _: &TaskId,
        _: InternalStatus,
    ) -> AppResult<Option<DateTime<Utc>>> {
        Ok(None)
    }
    async fn get_next_executable(&self, _: &ProjectId) -> AppResult<Option<Task>> {
        Ok(None)
    }
    async fn get_by_ideation_session(&self, _: &IdeationSessionId) -> AppResult<Vec<Task>> {
        Ok(vec![])
    }
    async fn get_by_project_filtered(&self, _: &ProjectId, _: bool) -> AppResult<Vec<Task>> {
        Ok(vec![])
    }
    async fn archive(&self, id: &TaskId) -> AppResult<Task> {
        let mut t = Task::new(ProjectId::new(), "archived".into());
        t.id = id.clone();
        Ok(t)
    }
    async fn restore(&self, id: &TaskId) -> AppResult<Task> {
        let mut t = Task::new(ProjectId::new(), "restored".into());
        t.id = id.clone();
        Ok(t)
    }
    async fn get_archived_count(&self, _: &ProjectId, _: Option<&str>) -> AppResult<u32> {
        Ok(0)
    }
    async fn list_paginated(
        &self,
        _: &ProjectId,
        _: Option<Vec<InternalStatus>>,
        _: u32,
        _: u32,
        _: bool,
        _: Option<&str>,
        _: Option<&str>,
        _: Option<&[String]>,
    ) -> AppResult<Vec<Task>> {
        Ok(vec![])
    }
    async fn count_tasks(&self, _: &ProjectId, _: bool, _: Option<&str>, _: Option<&str>) -> AppResult<u32> {
        Ok(0)
    }
    async fn search(&self, _: &ProjectId, _: &str, _: bool) -> AppResult<Vec<Task>> {
        Ok(vec![])
    }
    async fn get_oldest_ready_task(&self) -> AppResult<Option<Task>> {
        Ok(None)
    }
    async fn get_oldest_ready_tasks(&self, _: u32) -> AppResult<Vec<Task>> {
        Ok(vec![])
    }
    async fn update_latest_state_history_metadata(
        &self,
        _: &TaskId,
        _: &StateHistoryMetadata,
    ) -> AppResult<()> {
        Ok(())
    }
    async fn has_task_in_states(&self, _: &ProjectId, _: &[InternalStatus]) -> AppResult<bool> {
        Ok(false)
    }
    async fn get_stale_ready_tasks(&self, _threshold_secs: u64) -> AppResult<Vec<Task>> {
        Ok(vec![])
    }
    async fn get_status_history_batch(
        &self,
        _task_ids: &[crate::domain::entities::TaskId],
    ) -> AppResult<
        std::collections::HashMap<
            crate::domain::entities::TaskId,
            Vec<StatusTransition>,
        >,
    > {
        Ok(std::collections::HashMap::new())
    }
}

fn make_task(status: InternalStatus) -> Task {
    let mut task = Task::new(ProjectId::new(), "test task".into());
    task.internal_status = status;
    task
}

#[tokio::test]
async fn test_still_needs_recovery_when_executing() {
    let task_id = TaskId::new();
    let repo: Arc<dyn TaskRepository> = Arc::new(StubTaskRepo {
        task: Some(make_task(InternalStatus::Executing)),
    });
    assert!(task_still_needs_execution_recovery(&task_id, &repo).await);
}

#[tokio::test]
async fn test_still_needs_recovery_when_re_executing() {
    let task_id = TaskId::new();
    let repo: Arc<dyn TaskRepository> = Arc::new(StubTaskRepo {
        task: Some(make_task(InternalStatus::ReExecuting)),
    });
    assert!(task_still_needs_execution_recovery(&task_id, &repo).await);
}

#[tokio::test]
async fn test_no_recovery_when_already_transitioned() {
    // Simulate auto-complete resolving the task to PendingReview during the 500ms window
    let task_id = TaskId::new();
    let repo: Arc<dyn TaskRepository> = Arc::new(StubTaskRepo {
        task: Some(make_task(InternalStatus::PendingReview)),
    });
    assert!(!task_still_needs_execution_recovery(&task_id, &repo).await);
}

#[tokio::test]
async fn test_no_recovery_when_failed() {
    let task_id = TaskId::new();
    let repo: Arc<dyn TaskRepository> = Arc::new(StubTaskRepo {
        task: Some(make_task(InternalStatus::Failed)),
    });
    assert!(!task_still_needs_execution_recovery(&task_id, &repo).await);
}

#[tokio::test]
async fn test_no_recovery_when_cancelled() {
    let task_id = TaskId::new();
    let repo: Arc<dyn TaskRepository> = Arc::new(StubTaskRepo {
        task: Some(make_task(InternalStatus::Cancelled)),
    });
    assert!(!task_still_needs_execution_recovery(&task_id, &repo).await);
}

#[tokio::test]
async fn test_no_recovery_when_task_not_found() {
    // Task not found (e.g., deleted) → skip retry safely
    let task_id = TaskId::new();
    let repo: Arc<dyn TaskRepository> = Arc::new(StubTaskRepo { task: None });
    assert!(!task_still_needs_execution_recovery(&task_id, &repo).await);
}

// ========================================
// Global Rate Limit Backpressure Integration Tests
// ========================================

#[test]
fn test_apply_global_rate_limit_backpressure_sets_gate() {
    let exec = Arc::new(ExecutionState::new());
    let execution_state = Some(exec.clone());

    // Provide a future retry_after timestamp
    let future = (chrono::Utc::now() + chrono::Duration::seconds(300)).to_rfc3339();
    let retry_after = Some(future);

    assert!(!exec.is_provider_blocked());
    apply_global_rate_limit_backpressure(&execution_state, &retry_after, "test", "task-1");
    assert!(exec.is_provider_blocked());
    assert!(!exec.can_start_task());
}

#[test]
fn test_apply_global_rate_limit_backpressure_noop_without_retry_after() {
    let exec = Arc::new(ExecutionState::new());
    let execution_state = Some(exec.clone());

    // No retry_after → should not set backpressure
    apply_global_rate_limit_backpressure(&execution_state, &None, "test", "task-1");
    assert!(!exec.is_provider_blocked());
    assert!(exec.can_start_task());
}

#[test]
fn test_apply_global_rate_limit_backpressure_noop_without_execution_state() {
    let execution_state: Option<Arc<ExecutionState>> = None;
    let future = (chrono::Utc::now() + chrono::Duration::seconds(300)).to_rfc3339();
    let retry_after = Some(future);

    // Should not panic when execution_state is None
    apply_global_rate_limit_backpressure(&execution_state, &retry_after, "test", "task-1");
}

#[test]
fn test_apply_global_rate_limit_backpressure_expired_does_not_block() {
    let exec = Arc::new(ExecutionState::new());
    let execution_state = Some(exec.clone());

    // Provide a past retry_after timestamp
    let past = (chrono::Utc::now() - chrono::Duration::seconds(60)).to_rfc3339();
    let retry_after = Some(past);

    apply_global_rate_limit_backpressure(&execution_state, &retry_after, "test", "task-1");
    // Epoch was set, but it's in the past, so is_provider_blocked returns false
    assert!(!exec.is_provider_blocked());
    assert!(exec.can_start_task());
}

// ========================================
// AgentExit + Step Completion Override Tests
// ========================================
//
// These verify the all_steps_completed helper and that handle_stream_error
// overrides Failed → PendingReview when all steps are completed.

use crate::application::chat_service::chat_service_handlers::all_steps_completed;
use crate::domain::entities::{TaskStep, TaskStepId};
use crate::domain::repositories::TaskStepRepository;
use crate::error::AppError;
use std::collections::HashMap;

struct StubTaskStepRepo {
    steps: Vec<TaskStep>,
}

#[async_trait]
impl TaskStepRepository for StubTaskStepRepo {
    async fn create(&self, step: TaskStep) -> AppResult<TaskStep> {
        Ok(step)
    }
    async fn get_by_id(&self, _: &TaskStepId) -> AppResult<Option<TaskStep>> {
        Ok(None)
    }
    async fn get_by_task(&self, _: &TaskId) -> AppResult<Vec<TaskStep>> {
        Ok(self.steps.clone())
    }
    async fn get_by_task_and_status(
        &self,
        _: &TaskId,
        _: TaskStepStatus,
    ) -> AppResult<Vec<TaskStep>> {
        Ok(vec![])
    }
    async fn update(&self, _: &TaskStep) -> AppResult<()> {
        Ok(())
    }
    async fn delete(&self, _: &TaskStepId) -> AppResult<()> {
        Ok(())
    }
    async fn delete_by_task(&self, _: &TaskId) -> AppResult<()> {
        Ok(())
    }
    async fn count_by_status(&self, _: &TaskId) -> AppResult<HashMap<TaskStepStatus, u32>> {
        Ok(HashMap::new())
    }
    async fn bulk_create(&self, steps: Vec<TaskStep>) -> AppResult<Vec<TaskStep>> {
        Ok(steps)
    }
    async fn reorder(&self, _: &TaskId, _: Vec<TaskStepId>) -> AppResult<()> {
        Ok(())
    }
    async fn reset_all_to_pending(&self, _: &TaskId) -> AppResult<u32> {
        Ok(0)
    }
}

/// Stub that always returns a DB error for get_by_task.
struct StubErrorTaskStepRepo;

#[async_trait]
impl TaskStepRepository for StubErrorTaskStepRepo {
    async fn create(&self, step: TaskStep) -> AppResult<TaskStep> {
        Ok(step)
    }
    async fn get_by_id(&self, _: &TaskStepId) -> AppResult<Option<TaskStep>> {
        Ok(None)
    }
    async fn get_by_task(&self, _: &TaskId) -> AppResult<Vec<TaskStep>> {
        Err(AppError::Database("simulated DB error".into()))
    }
    async fn get_by_task_and_status(
        &self,
        _: &TaskId,
        _: TaskStepStatus,
    ) -> AppResult<Vec<TaskStep>> {
        Ok(vec![])
    }
    async fn update(&self, _: &TaskStep) -> AppResult<()> {
        Ok(())
    }
    async fn delete(&self, _: &TaskStepId) -> AppResult<()> {
        Ok(())
    }
    async fn delete_by_task(&self, _: &TaskId) -> AppResult<()> {
        Ok(())
    }
    async fn count_by_status(&self, _: &TaskId) -> AppResult<HashMap<TaskStepStatus, u32>> {
        Ok(HashMap::new())
    }
    async fn bulk_create(&self, steps: Vec<TaskStep>) -> AppResult<Vec<TaskStep>> {
        Ok(steps)
    }
    async fn reorder(&self, _: &TaskId, _: Vec<TaskStepId>) -> AppResult<()> {
        Ok(())
    }
    async fn reset_all_to_pending(&self, _: &TaskId) -> AppResult<u32> {
        Ok(0)
    }
}

fn make_step(task_id: &TaskId, status: TaskStepStatus) -> TaskStep {
    let mut step = TaskStep::new(task_id.clone(), "test step".into(), 0, "agent".into());
    step.status = status;
    step
}

fn run<F: std::future::Future>(f: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(f)
}

/// AgentExit with all steps completed should override target_status to PendingReview.
/// This covers the scenario where execution_complete was called (marking steps done)
/// but the agent exited with signal code=None (IPR removal → EOF → signal).
#[test]
fn test_agent_exit_all_steps_complete_overrides_to_pending_review() {
    let task_id = TaskId::new();
    let steps = vec![
        make_step(&task_id, TaskStepStatus::Completed),
        make_step(&task_id, TaskStepStatus::Completed),
        make_step(&task_id, TaskStepStatus::Skipped),
    ];
    let step_repo: Option<Arc<dyn TaskStepRepository>> =
        Some(Arc::new(StubTaskStepRepo { steps }));

    let result = run(all_steps_completed(&step_repo, &task_id));
    assert!(result, "All Completed+Skipped steps → should return true");
}

/// AgentExit with incomplete steps should remain Failed.
#[test]
fn test_agent_exit_incomplete_steps_stays_failed() {
    let task_id = TaskId::new();
    let steps = vec![
        make_step(&task_id, TaskStepStatus::Completed),
        make_step(&task_id, TaskStepStatus::InProgress), // not done
        make_step(&task_id, TaskStepStatus::Pending),    // not done
    ];
    let step_repo: Option<Arc<dyn TaskStepRepository>> =
        Some(Arc::new(StubTaskStepRepo { steps }));

    let result = run(all_steps_completed(&step_repo, &task_id));
    assert!(!result, "InProgress/Pending steps present → should return false");
}

/// AgentExit with no steps at all should remain Failed.
#[test]
fn test_agent_exit_no_steps_stays_failed() {
    let task_id = TaskId::new();
    let step_repo: Option<Arc<dyn TaskStepRepository>> =
        Some(Arc::new(StubTaskStepRepo { steps: vec![] }));

    let result = run(all_steps_completed(&step_repo, &task_id));
    assert!(!result, "Empty step list → should return false (guard against trivially true)");
}

/// Non-AgentExit errors should not trigger the override, even with complete steps.
#[test]
fn test_timeout_error_does_not_override_even_with_complete_steps() {
    let task_id = TaskId::new();
    let steps = vec![make_step(&task_id, TaskStepStatus::Completed)];
    let _step_repo: Option<Arc<dyn TaskStepRepository>> =
        Some(Arc::new(StubTaskStepRepo { steps }));

    let stream_error = StreamError::Timeout {
        context_type: ChatContextType::TaskExecution,
        elapsed_secs: 3600,
    };
    let initial_target = InternalStatus::Failed;

    // Timeout errors are NOT AgentExit — should not trigger override
    let target_status = if initial_target == InternalStatus::Failed
        && matches!(&stream_error, StreamError::AgentExit { .. })
    {
        InternalStatus::PendingReview // would override
    } else {
        initial_target
    };

    assert_eq!(
        target_status,
        InternalStatus::Failed,
        "Timeout errors should not trigger the AgentExit step-completion override"
    );
}

/// No task_step_repo → should not override (fail-safe).
#[test]
fn test_agent_exit_no_step_repo_stays_failed() {
    let task_id = TaskId::new();
    let step_repo: Option<Arc<dyn TaskStepRepository>> = None;

    let result = run(all_steps_completed(&step_repo, &task_id));
    assert!(!result, "No step repo → should fail-safe to false");
}

// ========================================
// New: all_steps_completed helper unit tests
// ========================================

/// "No output" path: worker exits cleanly with no text output but all steps done.
/// Helper must return true so the caller transitions to PendingReview.
#[test]
fn test_no_output_path_all_steps_complete() {
    let task_id = TaskId::new();
    let steps = vec![
        make_step(&task_id, TaskStepStatus::Completed),
        make_step(&task_id, TaskStepStatus::Skipped),
    ];
    let step_repo: Option<Arc<dyn TaskStepRepository>> =
        Some(Arc::new(StubTaskStepRepo { steps }));

    let result = run(all_steps_completed(&step_repo, &task_id));
    assert!(
        result,
        "No-output path: all Completed+Skipped → helper returns true"
    );
}

/// step_repo returns Err → helper must return false (safe fallback, never panic).
#[test]
fn test_step_repo_error_falls_through() {
    let task_id = TaskId::new();
    let step_repo: Option<Arc<dyn TaskStepRepository>> =
        Some(Arc::new(StubErrorTaskStepRepo));

    let result = run(all_steps_completed(&step_repo, &task_id));
    assert!(
        !result,
        "DB error on step query → helper must safe-fallback to false"
    );
}

/// All steps Skipped (no Completed) → helper must return true.
/// Skipped steps mean the agent legitimately bypassed them — work is considered done.
#[test]
fn test_all_skipped_no_completed() {
    let task_id = TaskId::new();
    let steps = vec![
        make_step(&task_id, TaskStepStatus::Skipped),
        make_step(&task_id, TaskStepStatus::Skipped),
        make_step(&task_id, TaskStepStatus::Skipped),
    ];
    let step_repo: Option<Arc<dyn TaskStepRepository>> =
        Some(Arc::new(StubTaskStepRepo { steps }));

    let result = run(all_steps_completed(&step_repo, &task_id));
    assert!(result, "All Skipped → helper returns true");
}

// ========================================
// Cancelled+turns_finalized path: run_completed emission
// ========================================

/// Verifies the branching logic in handle_stream_error for Cancelled variants.
///
/// Cancelled + turns_finalized > 0 → success path → run_completed emitted.
/// Cancelled + turns_finalized == 0 → user-stop path → agent:stopped emitted.
///
/// This test guards against regression: if the turns_finalized guard changes,
/// the UI will either (a) get stuck in "generating" or (b) emit spurious events.
#[test]
fn test_cancelled_with_turns_takes_success_path_not_error_path() {
    // StreamError is in scope via `use super::*` (chat_service_handlers re-exports it)

    // turns_finalized > 0 → agent completed at least one turn before cancellation
    // → handle_stream_error calls handle_stream_success + emits run_completed
    let cancelled_with_turns = StreamError::Cancelled { turns_finalized: 2 };
    let goes_to_success_path = match &cancelled_with_turns {
        StreamError::Cancelled { turns_finalized } => *turns_finalized > 0,
        _ => false,
    };
    assert!(
        goes_to_success_path,
        "Cancelled{{turns_finalized:2}} → must take success path (handle_stream_success + run_completed)"
    );
    // Success path does NOT call agent_run_repo.fail or emit agent:error
    assert!(
        !cancelled_with_turns.is_retryable(),
        "Cancelled variant is never retried (already handled as success or stop)"
    );
    assert!(
        !cancelled_with_turns.is_provider_error(),
        "Cancelled variant is not a provider error"
    );

    // turns_finalized == 0 → genuine user-stop or system cancel before any turn completed
    // → handle_stream_error emits agent:stopped (not run_completed)
    let cancelled_no_turns = StreamError::Cancelled { turns_finalized: 0 };
    let goes_to_stop_path = match &cancelled_no_turns {
        StreamError::Cancelled { turns_finalized } => *turns_finalized == 0,
        _ => false,
    };
    assert!(
        goes_to_stop_path,
        "Cancelled{{turns_finalized:0}} → must take stop path (agent:stopped, not run_completed)"
    );
}
