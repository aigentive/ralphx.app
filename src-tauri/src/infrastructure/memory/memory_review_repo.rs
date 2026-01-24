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
mod tests {
    use super::*;
    use crate::domain::entities::{ReviewOutcome, ReviewerType};

    #[tokio::test]
    async fn test_create_and_get_review() {
        let repo = MemoryReviewRepository::new();
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());
        let review = Review::new(project_id, task_id, ReviewerType::Ai);
        let review_id = review.id.clone();

        repo.create(&review).await.unwrap();

        let retrieved = repo.get_by_id(&review_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, review_id);
    }

    #[tokio::test]
    async fn test_get_by_task_id() {
        let repo = MemoryReviewRepository::new();
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());

        let review1 = Review::new(project_id.clone(), task_id.clone(), ReviewerType::Ai);
        let review2 = Review::new(project_id, task_id.clone(), ReviewerType::Human);

        repo.create(&review1).await.unwrap();
        repo.create(&review2).await.unwrap();

        let reviews = repo.get_by_task_id(&task_id).await.unwrap();
        assert_eq!(reviews.len(), 2);
    }

    #[tokio::test]
    async fn test_get_pending() {
        let repo = MemoryReviewRepository::new();
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());

        let mut review = Review::new(project_id.clone(), task_id.clone(), ReviewerType::Ai);
        repo.create(&review).await.unwrap();

        let pending = repo.get_pending(&project_id).await.unwrap();
        assert_eq!(pending.len(), 1);

        review.approve(Some("Good".to_string()));
        repo.update(&review).await.unwrap();

        let pending = repo.get_pending(&project_id).await.unwrap();
        assert_eq!(pending.len(), 0);
    }

    #[tokio::test]
    async fn test_add_and_get_action() {
        let repo = MemoryReviewRepository::new();
        let review_id = ReviewId::from_string("rev-1");
        let fix_task_id = TaskId::from_string("fix-1".to_string());

        let action = ReviewAction::with_target_task(
            review_id.clone(),
            ReviewActionType::CreatedFixTask,
            fix_task_id,
        );
        let action_id = action.id.clone();

        repo.add_action(&action).await.unwrap();

        let actions = repo.get_actions(&review_id).await.unwrap();
        assert_eq!(actions.len(), 1);

        let retrieved = repo.get_action_by_id(&action_id).await.unwrap();
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_add_and_get_note() {
        let repo = MemoryReviewRepository::new();
        let task_id = TaskId::from_string("task-1".to_string());

        let note1 = ReviewNote::with_notes(
            task_id.clone(),
            ReviewerType::Ai,
            ReviewOutcome::ChangesRequested,
            "Missing tests".to_string(),
        );
        let note2 = ReviewNote::with_notes(
            task_id.clone(),
            ReviewerType::Ai,
            ReviewOutcome::Approved,
            "Looks good now".to_string(),
        );
        let note1_id = note1.id.clone();

        repo.add_note(&note1).await.unwrap();
        repo.add_note(&note2).await.unwrap();

        let notes = repo.get_notes_by_task_id(&task_id).await.unwrap();
        assert_eq!(notes.len(), 2);

        let retrieved = repo.get_note_by_id(&note1_id).await.unwrap();
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_count_and_has_pending() {
        let repo = MemoryReviewRepository::new();
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());

        assert_eq!(repo.count_pending(&project_id).await.unwrap(), 0);
        assert!(!repo.has_pending_review(&task_id).await.unwrap());

        let review = Review::new(project_id.clone(), task_id.clone(), ReviewerType::Ai);
        repo.create(&review).await.unwrap();

        assert_eq!(repo.count_pending(&project_id).await.unwrap(), 1);
        assert!(repo.has_pending_review(&task_id).await.unwrap());
    }

    #[tokio::test]
    async fn test_count_fix_actions() {
        let repo = MemoryReviewRepository::new();
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());

        let review = Review::new(project_id, task_id.clone(), ReviewerType::Ai);
        let review_id = review.id.clone();
        repo.create(&review).await.unwrap();

        assert_eq!(repo.count_fix_actions(&task_id).await.unwrap(), 0);

        let fix_task_id = TaskId::from_string("fix-1".to_string());
        let action = ReviewAction::with_target_task(
            review_id,
            ReviewActionType::CreatedFixTask,
            fix_task_id,
        );
        repo.add_action(&action).await.unwrap();

        assert_eq!(repo.count_fix_actions(&task_id).await.unwrap(), 1);
    }
}
