// In-memory ReviewRepository implementation for testing
// Uses HashMap/RwLock for thread-safe in-memory storage

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::domain::entities::{
    ProjectId, Review, ReviewAction, ReviewActionId, ReviewActionType, ReviewId, ReviewNote,
    ReviewNoteId, ReviewStatus, TaskId,
};
use crate::domain::repositories::ReviewRepository;
use crate::error::AppResult;

/// In-memory implementation of ReviewRepository for testing
pub struct MemoryReviewRepository {
    reviews: RwLock<HashMap<String, Review>>,
    actions: RwLock<HashMap<String, ReviewAction>>,
    notes: RwLock<HashMap<String, ReviewNote>>,
}

impl MemoryReviewRepository {
    /// Create a new empty repository
    pub fn new() -> Self {
        Self {
            reviews: RwLock::new(HashMap::new()),
            actions: RwLock::new(HashMap::new()),
            notes: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryReviewRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ReviewRepository for MemoryReviewRepository {
    async fn create(&self, review: &Review) -> AppResult<()> {
        let mut reviews = self.reviews.write().unwrap();
        reviews.insert(review.id.as_str().to_string(), review.clone());
        Ok(())
    }

    async fn get_by_id(&self, id: &ReviewId) -> AppResult<Option<Review>> {
        let reviews = self.reviews.read().unwrap();
        Ok(reviews.get(id.as_str()).cloned())
    }

    async fn get_by_task_id(&self, task_id: &TaskId) -> AppResult<Vec<Review>> {
        let reviews = self.reviews.read().unwrap();
        Ok(reviews
            .values()
            .filter(|r| r.task_id == *task_id)
            .cloned()
            .collect())
    }

    async fn get_pending(&self, project_id: &ProjectId) -> AppResult<Vec<Review>> {
        let reviews = self.reviews.read().unwrap();
        Ok(reviews
            .values()
            .filter(|r| r.project_id == *project_id && r.status == ReviewStatus::Pending)
            .cloned()
            .collect())
    }

    async fn update(&self, review: &Review) -> AppResult<()> {
        let mut reviews = self.reviews.write().unwrap();
        reviews.insert(review.id.as_str().to_string(), review.clone());
        Ok(())
    }

    async fn delete(&self, id: &ReviewId) -> AppResult<()> {
        let mut reviews = self.reviews.write().unwrap();
        reviews.remove(id.as_str());
        Ok(())
    }

    async fn add_action(&self, action: &ReviewAction) -> AppResult<()> {
        let mut actions = self.actions.write().unwrap();
        actions.insert(action.id.as_str().to_string(), action.clone());
        Ok(())
    }

    async fn get_actions(&self, review_id: &ReviewId) -> AppResult<Vec<ReviewAction>> {
        let actions = self.actions.read().unwrap();
        Ok(actions
            .values()
            .filter(|a| a.review_id == *review_id)
            .cloned()
            .collect())
    }

    async fn get_action_by_id(&self, id: &ReviewActionId) -> AppResult<Option<ReviewAction>> {
        let actions = self.actions.read().unwrap();
        Ok(actions.get(id.as_str()).cloned())
    }

    async fn add_note(&self, note: &ReviewNote) -> AppResult<()> {
        let mut notes = self.notes.write().unwrap();
        notes.insert(note.id.as_str().to_string(), note.clone());
        Ok(())
    }

    async fn get_notes_by_task_id(&self, task_id: &TaskId) -> AppResult<Vec<ReviewNote>> {
        let notes = self.notes.read().unwrap();
        let mut result: Vec<_> = notes
            .values()
            .filter(|n| n.task_id == *task_id)
            .cloned()
            .collect();
        // Sort by created_at for consistent ordering
        result.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(result)
    }

    async fn get_note_by_id(&self, id: &ReviewNoteId) -> AppResult<Option<ReviewNote>> {
        let notes = self.notes.read().unwrap();
        Ok(notes.get(id.as_str()).cloned())
    }

    async fn get_by_status(
        &self,
        project_id: &ProjectId,
        status: ReviewStatus,
    ) -> AppResult<Vec<Review>> {
        let reviews = self.reviews.read().unwrap();
        Ok(reviews
            .values()
            .filter(|r| r.project_id == *project_id && r.status == status)
            .cloned()
            .collect())
    }

    async fn count_pending(&self, project_id: &ProjectId) -> AppResult<u32> {
        let reviews = self.reviews.read().unwrap();
        Ok(reviews
            .values()
            .filter(|r| r.project_id == *project_id && r.status == ReviewStatus::Pending)
            .count() as u32)
    }

    async fn has_pending_review(&self, task_id: &TaskId) -> AppResult<bool> {
        let reviews = self.reviews.read().unwrap();
        Ok(reviews
            .values()
            .any(|r| r.task_id == *task_id && r.status == ReviewStatus::Pending))
    }

    async fn count_fix_actions(&self, task_id: &TaskId) -> AppResult<u32> {
        let reviews = self.reviews.read().unwrap();
        let actions = self.actions.read().unwrap();
        let mut count = 0u32;
        for review in reviews.values() {
            if review.task_id == *task_id {
                for action in actions.values() {
                    if action.review_id == review.id
                        && action.action_type == ReviewActionType::CreatedFixTask
                    {
                        count += 1;
                    }
                }
            }
        }
        Ok(count)
    }

    async fn get_fix_actions(&self, task_id: &TaskId) -> AppResult<Vec<ReviewAction>> {
        let reviews = self.reviews.read().unwrap();
        let actions = self.actions.read().unwrap();
        let mut result = Vec::new();
        for review in reviews.values() {
            if review.task_id == *task_id {
                for action in actions.values() {
                    if action.review_id == review.id
                        && action.action_type == ReviewActionType::CreatedFixTask
                    {
                        result.push(action.clone());
                    }
                }
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
#[path = "memory_review_repo_tests.rs"]
mod tests;
