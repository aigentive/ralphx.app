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
mod tests {
    use super::*;
    use crate::domain::entities::{
        ReviewActionType, ReviewOutcome, ReviewerType,
    };
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};

    // Mock implementation for testing trait object usage
    struct MockReviewRepository {
        reviews: RwLock<HashMap<String, Review>>,
        actions: RwLock<HashMap<String, ReviewAction>>,
        notes: RwLock<HashMap<String, ReviewNote>>,
    }

    impl MockReviewRepository {
        fn new() -> Self {
            Self {
                reviews: RwLock::new(HashMap::new()),
                actions: RwLock::new(HashMap::new()),
                notes: RwLock::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl ReviewRepository for MockReviewRepository {
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

    #[test]
    fn test_review_repository_trait_can_be_object_safe() {
        let repo: Arc<dyn ReviewRepository> = Arc::new(MockReviewRepository::new());
        assert!(Arc::strong_count(&repo) == 1);
    }

    #[tokio::test]
    async fn test_mock_repository_create_and_get() {
        let repo = MockReviewRepository::new();
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
    async fn test_mock_repository_get_by_task_id() {
        let repo = MockReviewRepository::new();
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());

        // Create two reviews for the same task
        let review1 = Review::new(project_id.clone(), task_id.clone(), ReviewerType::Ai);
        let review2 = Review::new(project_id, task_id.clone(), ReviewerType::Human);

        repo.create(&review1).await.unwrap();
        repo.create(&review2).await.unwrap();

        let reviews = repo.get_by_task_id(&task_id).await.unwrap();
        assert_eq!(reviews.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_repository_get_pending() {
        let repo = MockReviewRepository::new();
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());

        let mut review = Review::new(project_id.clone(), task_id.clone(), ReviewerType::Ai);
        repo.create(&review).await.unwrap();

        // Initially pending
        let pending = repo.get_pending(&project_id).await.unwrap();
        assert_eq!(pending.len(), 1);

        // Approve the review
        review.approve(Some("Good".to_string()));
        repo.update(&review).await.unwrap();

        // No longer pending
        let pending = repo.get_pending(&project_id).await.unwrap();
        assert_eq!(pending.len(), 0);
    }

    #[tokio::test]
    async fn test_mock_repository_update() {
        let repo = MockReviewRepository::new();
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());

        let mut review = Review::new(project_id, task_id, ReviewerType::Ai);
        let review_id = review.id.clone();
        repo.create(&review).await.unwrap();

        review.approve(Some("Looks good".to_string()));
        repo.update(&review).await.unwrap();

        let retrieved = repo.get_by_id(&review_id).await.unwrap().unwrap();
        assert_eq!(retrieved.status, ReviewStatus::Approved);
        assert_eq!(retrieved.notes, Some("Looks good".to_string()));
    }

    #[tokio::test]
    async fn test_mock_repository_delete() {
        let repo = MockReviewRepository::new();
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());

        let review = Review::new(project_id, task_id, ReviewerType::Ai);
        let review_id = review.id.clone();

        repo.create(&review).await.unwrap();
        repo.delete(&review_id).await.unwrap();

        let retrieved = repo.get_by_id(&review_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_mock_repository_add_and_get_action() {
        let repo = MockReviewRepository::new();
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());

        let review = Review::new(project_id, task_id.clone(), ReviewerType::Ai);
        let review_id = review.id.clone();
        repo.create(&review).await.unwrap();

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
        assert!(actions[0].is_fix_task_action());

        let retrieved = repo.get_action_by_id(&action_id).await.unwrap();
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_mock_repository_add_and_get_note() {
        let repo = MockReviewRepository::new();
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
        assert_eq!(retrieved.unwrap().outcome, ReviewOutcome::ChangesRequested);
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_status() {
        let repo = MockReviewRepository::new();
        let project_id = ProjectId::from_string("proj-1".to_string());

        let task1 = TaskId::from_string("task-1".to_string());
        let task2 = TaskId::from_string("task-2".to_string());

        let review1 = Review::new(project_id.clone(), task1, ReviewerType::Ai);
        let mut review2 = Review::new(project_id.clone(), task2, ReviewerType::Ai);

        // Leave review1 pending, approve review2
        repo.create(&review1).await.unwrap();

        review2.approve(None);
        repo.create(&review2).await.unwrap();

        let pending = repo.get_by_status(&project_id, ReviewStatus::Pending).await.unwrap();
        assert_eq!(pending.len(), 1);

        let approved = repo.get_by_status(&project_id, ReviewStatus::Approved).await.unwrap();
        assert_eq!(approved.len(), 1);
    }

    #[tokio::test]
    async fn test_mock_repository_count_pending() {
        let repo = MockReviewRepository::new();
        let project_id = ProjectId::from_string("proj-1".to_string());

        let task1 = TaskId::from_string("task-1".to_string());
        let task2 = TaskId::from_string("task-2".to_string());
        let task3 = TaskId::from_string("task-3".to_string());

        repo.create(&Review::new(project_id.clone(), task1, ReviewerType::Ai)).await.unwrap();
        repo.create(&Review::new(project_id.clone(), task2, ReviewerType::Ai)).await.unwrap();

        let mut approved = Review::new(project_id.clone(), task3, ReviewerType::Ai);
        approved.approve(None);
        repo.create(&approved).await.unwrap();

        let count = repo.count_pending(&project_id).await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_mock_repository_has_pending_review() {
        let repo = MockReviewRepository::new();
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());

        // No review yet
        assert!(!repo.has_pending_review(&task_id).await.unwrap());

        // Create pending review
        let review = Review::new(project_id, task_id.clone(), ReviewerType::Ai);
        repo.create(&review).await.unwrap();

        assert!(repo.has_pending_review(&task_id).await.unwrap());
    }

    #[tokio::test]
    async fn test_mock_repository_count_fix_actions() {
        let repo = MockReviewRepository::new();
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());

        // No fix actions yet
        assert_eq!(repo.count_fix_actions(&task_id).await.unwrap(), 0);

        // Create a review and add fix task action
        let review = Review::new(project_id, task_id.clone(), ReviewerType::Ai);
        let review_id = review.id.clone();
        repo.create(&review).await.unwrap();

        let fix_task_id = TaskId::from_string("fix-1".to_string());
        let action = ReviewAction::with_target_task(
            review_id.clone(),
            ReviewActionType::CreatedFixTask,
            fix_task_id,
        );
        repo.add_action(&action).await.unwrap();

        assert_eq!(repo.count_fix_actions(&task_id).await.unwrap(), 1);

        // Add another fix task action
        let fix_task_id_2 = TaskId::from_string("fix-2".to_string());
        let action2 = ReviewAction::with_target_task(
            review_id,
            ReviewActionType::CreatedFixTask,
            fix_task_id_2,
        );
        repo.add_action(&action2).await.unwrap();

        assert_eq!(repo.count_fix_actions(&task_id).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_mock_repository_get_fix_actions() {
        let repo = MockReviewRepository::new();
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());

        // Create a review and add actions
        let review = Review::new(project_id, task_id.clone(), ReviewerType::Ai);
        let review_id = review.id.clone();
        repo.create(&review).await.unwrap();

        let fix_task_id = TaskId::from_string("fix-1".to_string());
        let action1 = ReviewAction::with_target_task(
            review_id.clone(),
            ReviewActionType::CreatedFixTask,
            fix_task_id.clone(),
        );
        repo.add_action(&action1).await.unwrap();

        // Add a non-fix action (should not be returned)
        let action2 = ReviewAction::new(review_id, ReviewActionType::Approved);
        repo.add_action(&action2).await.unwrap();

        let fix_actions = repo.get_fix_actions(&task_id).await.unwrap();
        assert_eq!(fix_actions.len(), 1);
        assert!(fix_actions[0].is_fix_task_action());
        assert_eq!(fix_actions[0].target_task_id, Some(fix_task_id));
    }
}
