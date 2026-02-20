// Task dependency repository trait - domain layer abstraction
//
// This trait defines the contract for task dependency persistence.
// Used for tasks that have been applied from proposals to track blockers.

use async_trait::async_trait;

use crate::domain::entities::TaskId;
use crate::error::AppResult;

/// Repository trait for task dependency persistence.
/// Implementations can use SQLite, PostgreSQL, in-memory, etc.
#[async_trait]
pub trait TaskDependencyRepository: Send + Sync {
    /// Add a dependency (task_id depends on depends_on_task_id)
    async fn add_dependency(&self, task_id: &TaskId, depends_on_task_id: &TaskId) -> AppResult<()>;

    /// Remove a dependency
    async fn remove_dependency(
        &self,
        task_id: &TaskId,
        depends_on_task_id: &TaskId,
    ) -> AppResult<()>;

    /// Get all tasks that this task depends on (blockers)
    async fn get_blockers(&self, task_id: &TaskId) -> AppResult<Vec<TaskId>>;

    /// Get all tasks that depend on this task (blocked by this)
    async fn get_blocked_by(&self, task_id: &TaskId) -> AppResult<Vec<TaskId>>;

    /// Check if adding a dependency would create a circular dependency
    async fn has_circular_dependency(
        &self,
        task_id: &TaskId,
        potential_dep: &TaskId,
    ) -> AppResult<bool>;

    /// Clear all dependencies for a task (both directions)
    async fn clear_dependencies(&self, task_id: &TaskId) -> AppResult<()>;

    /// Count blockers for a task
    async fn count_blockers(&self, task_id: &TaskId) -> AppResult<u32>;

    /// Count tasks blocked by this task
    async fn count_blocked_by(&self, task_id: &TaskId) -> AppResult<u32>;

    /// Check if a specific dependency exists
    async fn has_dependency(
        &self,
        task_id: &TaskId,
        depends_on_task_id: &TaskId,
    ) -> AppResult<bool>;
}

#[cfg(test)]
#[path = "task_dependency_repository_tests.rs"]
mod tests;
