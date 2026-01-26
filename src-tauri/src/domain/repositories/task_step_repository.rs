// TaskStepRepository trait - domain layer abstraction for task step persistence
//
// This trait defines the contract for task step data persistence.
// Implementations can use SQLite, in-memory storage, etc.

use async_trait::async_trait;
use std::collections::HashMap;

use crate::domain::entities::{TaskId, TaskStep, TaskStepId, TaskStepStatus};
use crate::error::AppResult;

/// Repository trait for TaskStep persistence.
/// Provides CRUD operations and step-specific queries.
#[async_trait]
pub trait TaskStepRepository: Send + Sync {
    // ═══════════════════════════════════════════════════════════════════════
    // CRUD Operations
    // ═══════════════════════════════════════════════════════════════════════

    /// Create a new task step
    async fn create(&self, step: TaskStep) -> AppResult<TaskStep>;

    /// Get step by ID
    async fn get_by_id(&self, id: &TaskStepId) -> AppResult<Option<TaskStep>>;

    /// Get all steps for a task, ordered by sort_order ASC
    async fn get_by_task(&self, task_id: &TaskId) -> AppResult<Vec<TaskStep>>;

    /// Get steps for a task filtered by status
    async fn get_by_task_and_status(
        &self,
        task_id: &TaskId,
        status: TaskStepStatus,
    ) -> AppResult<Vec<TaskStep>>;

    /// Update a task step
    async fn update(&self, step: &TaskStep) -> AppResult<()>;

    /// Delete a task step
    async fn delete(&self, id: &TaskStepId) -> AppResult<()>;

    /// Delete all steps for a task
    async fn delete_by_task(&self, task_id: &TaskId) -> AppResult<()>;

    // ═══════════════════════════════════════════════════════════════════════
    // Query Operations
    // ═══════════════════════════════════════════════════════════════════════

    /// Count steps by status for a given task
    /// Returns a HashMap with counts for each status
    async fn count_by_status(
        &self,
        task_id: &TaskId,
    ) -> AppResult<HashMap<TaskStepStatus, u32>>;

    // ═══════════════════════════════════════════════════════════════════════
    // Bulk Operations
    // ═══════════════════════════════════════════════════════════════════════

    /// Create multiple steps in a single transaction
    /// Returns the created steps with their assigned IDs
    async fn bulk_create(&self, steps: Vec<TaskStep>) -> AppResult<Vec<TaskStep>>;

    /// Reorder steps for a task
    /// Updates sort_order for each step based on the provided order of IDs
    /// step_ids[0] gets sort_order 0, step_ids[1] gets sort_order 1, etc.
    async fn reorder(&self, task_id: &TaskId, step_ids: Vec<TaskStepId>) -> AppResult<()>;
}
