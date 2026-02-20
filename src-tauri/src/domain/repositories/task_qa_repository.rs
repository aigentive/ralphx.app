// TaskQA repository trait - domain layer abstraction
//
// This trait defines the contract for TaskQA persistence.
// TaskQA records store QA artifacts for tasks.

use async_trait::async_trait;

use crate::domain::entities::{TaskId, TaskQA, TaskQAId};
use crate::domain::qa::{AcceptanceCriteria, QAResults, QATestSteps};
use crate::error::AppResult;

/// Repository trait for TaskQA persistence.
/// Implementations can use SQLite, in-memory, etc.
#[async_trait]
pub trait TaskQARepository: Send + Sync {
    /// Create a new TaskQA record
    async fn create(&self, task_qa: &TaskQA) -> AppResult<()>;

    /// Get TaskQA by its ID
    async fn get_by_id(&self, id: &TaskQAId) -> AppResult<Option<TaskQA>>;

    /// Get TaskQA by task ID
    async fn get_by_task_id(&self, task_id: &TaskId) -> AppResult<Option<TaskQA>>;

    /// Update QA prep results
    async fn update_prep(
        &self,
        id: &TaskQAId,
        agent_id: &str,
        criteria: &AcceptanceCriteria,
        steps: &QATestSteps,
    ) -> AppResult<()>;

    /// Update QA refinement results
    async fn update_refinement(
        &self,
        id: &TaskQAId,
        agent_id: &str,
        actual_implementation: &str,
        refined_steps: &QATestSteps,
    ) -> AppResult<()>;

    /// Update QA test results
    async fn update_results(
        &self,
        id: &TaskQAId,
        agent_id: &str,
        results: &QAResults,
        screenshots: &[String],
    ) -> AppResult<()>;

    /// Get tasks that need QA prep (have no acceptance criteria yet)
    async fn get_pending_prep(&self) -> AppResult<Vec<TaskQA>>;

    /// Delete TaskQA by ID
    async fn delete(&self, id: &TaskQAId) -> AppResult<()>;

    /// Delete TaskQA by task ID
    async fn delete_by_task_id(&self, task_id: &TaskId) -> AppResult<()>;

    /// Check if TaskQA exists for a task
    async fn exists_for_task(&self, task_id: &TaskId) -> AppResult<bool>;
}

#[cfg(test)]
#[path = "task_qa_repository_tests.rs"]
mod tests;
