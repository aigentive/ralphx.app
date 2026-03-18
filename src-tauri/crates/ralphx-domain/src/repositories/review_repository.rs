// Review repository trait - domain layer abstraction
//
// This trait defines the contract for Review, ReviewAction, and ReviewNote persistence.
// These records store code review data for tasks.

use async_trait::async_trait;

use crate::domain::entities::{
    ProjectId, Review, ReviewAction, ReviewActionId, ReviewId, ReviewNote, ReviewNoteId,
    ReviewStatus, TaskId,
};
use crate::error::AppResult;

/// Repository trait for Review persistence.
/// Implementations can use SQLite, in-memory, etc.
#[async_trait]
pub trait ReviewRepository: Send + Sync {
    // ========================================
    // Review methods
    // ========================================

    /// Create a new review
    async fn create(&self, review: &Review) -> AppResult<()>;

    /// Get review by its ID
    async fn get_by_id(&self, id: &ReviewId) -> AppResult<Option<Review>>;

    /// Get reviews by task ID
    async fn get_by_task_id(&self, task_id: &TaskId) -> AppResult<Vec<Review>>;

    /// Get pending reviews (status = pending) for a project
    async fn get_pending(&self, project_id: &ProjectId) -> AppResult<Vec<Review>>;

    /// Update a review (status, notes, completed_at)
    async fn update(&self, review: &Review) -> AppResult<()>;

    /// Delete a review by ID
    async fn delete(&self, id: &ReviewId) -> AppResult<()>;

    // ========================================
    // ReviewAction methods
    // ========================================

    /// Add an action to a review
    async fn add_action(&self, action: &ReviewAction) -> AppResult<()>;

    /// Get actions for a review
    async fn get_actions(&self, review_id: &ReviewId) -> AppResult<Vec<ReviewAction>>;

    /// Get action by its ID
    async fn get_action_by_id(&self, id: &ReviewActionId) -> AppResult<Option<ReviewAction>>;

    // ========================================
    // ReviewNote methods
    // ========================================

    /// Add a review note (for history)
    async fn add_note(&self, note: &ReviewNote) -> AppResult<()>;

    /// Get notes for a task (review history)
    async fn get_notes_by_task_id(&self, task_id: &TaskId) -> AppResult<Vec<ReviewNote>>;

    /// Get note by its ID
    async fn get_note_by_id(&self, id: &ReviewNoteId) -> AppResult<Option<ReviewNote>>;

    // ========================================
    // Query methods
    // ========================================

    /// Get reviews by status for a project
    async fn get_by_status(
        &self,
        project_id: &ProjectId,
        status: ReviewStatus,
    ) -> AppResult<Vec<Review>>;

    /// Count pending reviews for a project
    async fn count_pending(&self, project_id: &ProjectId) -> AppResult<u32>;

    /// Check if a task has any pending reviews
    async fn has_pending_review(&self, task_id: &TaskId) -> AppResult<bool>;

    /// Count fix task actions for a task (number of fix tasks created during reviews)
    async fn count_fix_actions(&self, task_id: &TaskId) -> AppResult<u32>;

    /// Get fix task actions for a task (to find fix tasks created for this task)
    async fn get_fix_actions(&self, task_id: &TaskId) -> AppResult<Vec<ReviewAction>>;
}

#[cfg(test)]
#[path = "review_repository_tests.rs"]
mod tests;
