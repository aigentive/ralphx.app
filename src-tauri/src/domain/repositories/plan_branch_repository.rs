// PlanBranch repository trait - domain layer abstraction
//
// Defines the contract for plan branch persistence.
// Implementations can use SQLite, in-memory, etc.

use async_trait::async_trait;

use crate::domain::entities::{ArtifactId, IdeationSessionId, PlanBranch, PlanBranchId, PlanBranchStatus, ProjectId, TaskId};
use crate::error::AppResult;

/// Repository trait for PlanBranch persistence.
#[async_trait]
pub trait PlanBranchRepository: Send + Sync {
    /// Create a new plan branch record
    async fn create(&self, branch: PlanBranch) -> AppResult<PlanBranch>;

    /// Get plan branch by plan artifact ID (unique constraint)
    async fn get_by_plan_artifact_id(&self, id: &ArtifactId) -> AppResult<Option<PlanBranch>>;

    /// Get plan branch by session ID (unique constraint, primary lookup)
    async fn get_by_session_id(&self, session_id: &IdeationSessionId) -> AppResult<Option<PlanBranch>>;

    /// Get plan branch by its merge task ID
    async fn get_by_merge_task_id(&self, task_id: &TaskId) -> AppResult<Option<PlanBranch>>;

    /// Get all plan branches for a project
    async fn get_by_project_id(&self, project_id: &ProjectId) -> AppResult<Vec<PlanBranch>>;

    /// Update plan branch status
    async fn update_status(&self, id: &PlanBranchId, status: PlanBranchStatus) -> AppResult<()>;

    /// Set the merge task ID for a plan branch
    async fn set_merge_task_id(&self, id: &PlanBranchId, task_id: &TaskId) -> AppResult<()>;

    /// Mark a plan branch as merged (sets status to Merged and merged_at timestamp)
    async fn set_merged(&self, id: &PlanBranchId) -> AppResult<()>;
}
