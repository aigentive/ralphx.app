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
