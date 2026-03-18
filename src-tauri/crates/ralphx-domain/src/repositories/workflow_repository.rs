// Workflow repository trait - domain layer abstraction
//
// This trait defines the contract for workflow persistence.
// Implementations can use SQLite, PostgreSQL, in-memory, etc.

use async_trait::async_trait;

use crate::domain::entities::{WorkflowId, WorkflowSchema};
use crate::error::AppResult;

/// Repository trait for WorkflowSchema persistence.
/// Implementations can use SQLite, in-memory, etc.
#[async_trait]
pub trait WorkflowRepository: Send + Sync {
    /// Create a new workflow
    async fn create(&self, workflow: WorkflowSchema) -> AppResult<WorkflowSchema>;

    /// Get workflow by ID
    async fn get_by_id(&self, id: &WorkflowId) -> AppResult<Option<WorkflowSchema>>;

    /// Get all workflows
    async fn get_all(&self) -> AppResult<Vec<WorkflowSchema>>;

    /// Get the default workflow (where is_default = true)
    async fn get_default(&self) -> AppResult<Option<WorkflowSchema>>;

    /// Update a workflow
    async fn update(&self, workflow: &WorkflowSchema) -> AppResult<()>;

    /// Delete a workflow
    async fn delete(&self, id: &WorkflowId) -> AppResult<()>;

    /// Set a workflow as the default (unsets any previous default)
    async fn set_default(&self, id: &WorkflowId) -> AppResult<()>;
}

#[cfg(test)]
#[path = "workflow_repository_tests.rs"]
mod tests;
