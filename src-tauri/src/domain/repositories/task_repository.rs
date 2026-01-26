// Task repository trait - domain layer abstraction
//
// This trait defines the contract for task persistence.
// Implementations can use SQLite, PostgreSQL, in-memory, etc.

use async_trait::async_trait;

use crate::domain::entities::{InternalStatus, ProjectId, Task, TaskId};
use crate::domain::repositories::StatusTransition;
use crate::error::AppResult;

/// Repository trait for Task persistence.
/// Implementations can use SQLite, PostgreSQL, in-memory, etc.
#[async_trait]
pub trait TaskRepository: Send + Sync {
    // ═══════════════════════════════════════════════════════════════════════
    // CRUD Operations
    // ═══════════════════════════════════════════════════════════════════════

    /// Create a new task
    async fn create(&self, task: Task) -> AppResult<Task>;

    /// Get task by ID
    async fn get_by_id(&self, id: &TaskId) -> AppResult<Option<Task>>;

    /// Get all tasks for a project
    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<Task>>;

    /// Update a task
    async fn update(&self, task: &Task) -> AppResult<()>;

    /// Delete a task
    async fn delete(&self, id: &TaskId) -> AppResult<()>;

    // ═══════════════════════════════════════════════════════════════════════
    // Status Operations (Phase 3 will add full state machine integration)
    // ═══════════════════════════════════════════════════════════════════════

    /// Get tasks by status
    async fn get_by_status(
        &self,
        project_id: &ProjectId,
        status: InternalStatus,
    ) -> AppResult<Vec<Task>>;

    /// Persist a status change with audit log entry
    async fn persist_status_change(
        &self,
        id: &TaskId,
        from: InternalStatus,
        to: InternalStatus,
        trigger: &str,
    ) -> AppResult<()>;

    /// Get status history for audit
    async fn get_status_history(&self, id: &TaskId) -> AppResult<Vec<StatusTransition>>;

    // ═══════════════════════════════════════════════════════════════════════
    // Query Operations
    // ═══════════════════════════════════════════════════════════════════════

    /// Get next task ready for execution (READY status, no blockers)
    async fn get_next_executable(&self, project_id: &ProjectId) -> AppResult<Option<Task>>;

    /// Get tasks blocking a given task
    async fn get_blockers(&self, id: &TaskId) -> AppResult<Vec<Task>>;

    /// Get tasks blocked by a given task
    async fn get_dependents(&self, id: &TaskId) -> AppResult<Vec<Task>>;

    /// Add a blocker relationship
    async fn add_blocker(&self, task_id: &TaskId, blocker_id: &TaskId) -> AppResult<()>;

    /// Remove/resolve a blocker relationship
    async fn resolve_blocker(&self, task_id: &TaskId, blocker_id: &TaskId) -> AppResult<()>;

    // ═══════════════════════════════════════════════════════════════════════
    // Archive Operations (Phase 18 - Soft Delete)
    // ═══════════════════════════════════════════════════════════════════════

    /// Get tasks by project, optionally including archived
    async fn get_by_project_filtered(
        &self,
        project_id: &ProjectId,
        include_archived: bool,
    ) -> AppResult<Vec<Task>>;

    /// Archive a task (soft delete)
    async fn archive(&self, task_id: &TaskId) -> AppResult<Task>;

    /// Restore an archived task
    async fn restore(&self, task_id: &TaskId) -> AppResult<Task>;

    /// Count archived tasks for a project
    async fn get_archived_count(&self, project_id: &ProjectId) -> AppResult<u32>;

    // ═══════════════════════════════════════════════════════════════════════
    // Pagination Operations (Phase 18 - Infinite Scroll)
    // ═══════════════════════════════════════════════════════════════════════

    /// Get paginated tasks for a project
    ///
    /// # Arguments
    /// * `project_id` - The project ID
    /// * `status` - Optional status filter
    /// * `offset` - Number of tasks to skip
    /// * `limit` - Maximum number of tasks to return
    /// * `include_archived` - Whether to include archived tasks
    ///
    /// # Returns
    /// * Tasks ordered by created_at DESC (newest first)
    async fn list_paginated(
        &self,
        project_id: &ProjectId,
        status: Option<InternalStatus>,
        offset: u32,
        limit: u32,
        include_archived: bool,
    ) -> AppResult<Vec<Task>>;

    /// Count total tasks for a project
    ///
    /// # Arguments
    /// * `project_id` - The project ID
    /// * `include_archived` - Whether to include archived tasks in the count
    ///
    /// # Returns
    /// * Total count of tasks matching the criteria
    async fn count_tasks(
        &self,
        project_id: &ProjectId,
        include_archived: bool,
    ) -> AppResult<u32>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // Mock implementation for testing trait object usage
    struct MockTaskRepository;

    #[async_trait]
    impl TaskRepository for MockTaskRepository {
        async fn create(&self, task: Task) -> AppResult<Task> {
            Ok(task)
        }

        async fn get_by_id(&self, _id: &TaskId) -> AppResult<Option<Task>> {
            Ok(None)
        }

        async fn get_by_project(&self, _project_id: &ProjectId) -> AppResult<Vec<Task>> {
            Ok(vec![])
        }

        async fn update(&self, _task: &Task) -> AppResult<()> {
            Ok(())
        }

        async fn delete(&self, _id: &TaskId) -> AppResult<()> {
            Ok(())
        }

        async fn get_by_status(
            &self,
            _project_id: &ProjectId,
            _status: InternalStatus,
        ) -> AppResult<Vec<Task>> {
            Ok(vec![])
        }

        async fn persist_status_change(
            &self,
            _id: &TaskId,
            _from: InternalStatus,
            _to: InternalStatus,
            _trigger: &str,
        ) -> AppResult<()> {
            Ok(())
        }

        async fn get_status_history(&self, _id: &TaskId) -> AppResult<Vec<StatusTransition>> {
            Ok(vec![])
        }

        async fn get_next_executable(&self, _project_id: &ProjectId) -> AppResult<Option<Task>> {
            Ok(None)
        }

        async fn get_blockers(&self, _id: &TaskId) -> AppResult<Vec<Task>> {
            Ok(vec![])
        }

        async fn get_dependents(&self, _id: &TaskId) -> AppResult<Vec<Task>> {
            Ok(vec![])
        }

        async fn add_blocker(&self, _task_id: &TaskId, _blocker_id: &TaskId) -> AppResult<()> {
            Ok(())
        }

        async fn resolve_blocker(&self, _task_id: &TaskId, _blocker_id: &TaskId) -> AppResult<()> {
            Ok(())
        }

        async fn get_by_project_filtered(
            &self,
            _project_id: &ProjectId,
            _include_archived: bool,
        ) -> AppResult<Vec<Task>> {
            Ok(vec![])
        }

        async fn archive(&self, task_id: &TaskId) -> AppResult<Task> {
            let project_id = ProjectId::new();
            let mut task = Task::new(project_id, "Archived task".to_string());
            task.id = task_id.clone();
            Ok(task)
        }

        async fn restore(&self, task_id: &TaskId) -> AppResult<Task> {
            let project_id = ProjectId::new();
            let mut task = Task::new(project_id, "Restored task".to_string());
            task.id = task_id.clone();
            Ok(task)
        }

        async fn get_archived_count(&self, _project_id: &ProjectId) -> AppResult<u32> {
            Ok(0)
        }

        async fn list_paginated(
            &self,
            _project_id: &ProjectId,
            _status: Option<InternalStatus>,
            _offset: u32,
            _limit: u32,
            _include_archived: bool,
        ) -> AppResult<Vec<Task>> {
            Ok(vec![])
        }

        async fn count_tasks(
            &self,
            _project_id: &ProjectId,
            _include_archived: bool,
        ) -> AppResult<u32> {
            Ok(0)
        }
    }

    #[test]
    fn test_task_repository_trait_can_be_object_safe() {
        // Verify that TaskRepository can be used as a trait object
        let repo: Arc<dyn TaskRepository> = Arc::new(MockTaskRepository);
        assert!(Arc::strong_count(&repo) == 1);
    }

    #[tokio::test]
    async fn test_mock_task_repository_create() {
        let repo = MockTaskRepository;
        let project_id = ProjectId::new();
        let task = Task::new(project_id, "Test task".to_string());

        let result = repo.create(task.clone()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, task.id);
    }

    #[tokio::test]
    async fn test_mock_task_repository_get_by_id_returns_none() {
        let repo = MockTaskRepository;
        let task_id = TaskId::new();

        let result = repo.get_by_id(&task_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_mock_task_repository_get_by_project() {
        let repo = MockTaskRepository;
        let project_id = ProjectId::new();

        let result = repo.get_by_project(&project_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_task_repository_update() {
        let repo = MockTaskRepository;
        let project_id = ProjectId::new();
        let task = Task::new(project_id, "Test task".to_string());

        let result = repo.update(&task).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_task_repository_delete() {
        let repo = MockTaskRepository;
        let task_id = TaskId::new();

        let result = repo.delete(&task_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_task_repository_get_by_status() {
        let repo = MockTaskRepository;
        let project_id = ProjectId::new();

        let result = repo.get_by_status(&project_id, InternalStatus::Backlog).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_task_repository_persist_status_change() {
        let repo = MockTaskRepository;
        let task_id = TaskId::new();

        let result = repo
            .persist_status_change(&task_id, InternalStatus::Backlog, InternalStatus::Ready, "user")
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_task_repository_get_status_history() {
        let repo = MockTaskRepository;
        let task_id = TaskId::new();

        let result = repo.get_status_history(&task_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_task_repository_get_next_executable() {
        let repo = MockTaskRepository;
        let project_id = ProjectId::new();

        let result = repo.get_next_executable(&project_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_mock_task_repository_blocker_operations() {
        let repo = MockTaskRepository;
        let task_id = TaskId::new();
        let blocker_id = TaskId::new();

        // Add blocker
        let result = repo.add_blocker(&task_id, &blocker_id).await;
        assert!(result.is_ok());

        // Get blockers
        let result = repo.get_blockers(&task_id).await;
        assert!(result.is_ok());

        // Get dependents
        let result = repo.get_dependents(&blocker_id).await;
        assert!(result.is_ok());

        // Resolve blocker
        let result = repo.resolve_blocker(&task_id, &blocker_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_task_repository_trait_object_in_arc() {
        let repo: Arc<dyn TaskRepository> = Arc::new(MockTaskRepository);
        let project_id = ProjectId::new();
        let task = Task::new(project_id.clone(), "Test via trait object".to_string());

        // Use through trait object
        let result = repo.create(task).await;
        assert!(result.is_ok());

        let tasks = repo.get_by_project(&project_id).await;
        assert!(tasks.is_ok());
    }
}
