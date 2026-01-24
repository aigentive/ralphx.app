// ReviewService
// Application service for orchestrating review workflow: AI review, fix tasks, and escalation

use crate::domain::entities::{
    InternalStatus, ProjectId, Review, ReviewAction, ReviewActionType, ReviewNote, ReviewOutcome,
    ReviewerType, Task, TaskId,
};
use crate::domain::repositories::{ReviewRepository, TaskRepository};
use crate::domain::review::config::ReviewSettings;
use crate::domain::tools::complete_review::{CompleteReviewInput, ReviewToolOutcome};
use crate::error::{AppError, AppResult};
use std::sync::Arc;

/// Service for orchestrating the review workflow
pub struct ReviewService<R: ReviewRepository, T: TaskRepository> {
    review_repo: Arc<R>,
    task_repo: Arc<T>,
    settings: ReviewSettings,
}

impl<R: ReviewRepository, T: TaskRepository> ReviewService<R, T> {
    /// Create a new review service with default settings
    pub fn new(review_repo: Arc<R>, task_repo: Arc<T>) -> Self {
        Self {
            review_repo,
            task_repo,
            settings: ReviewSettings::default(),
        }
    }

    /// Create a review service with custom settings
    pub fn with_settings(review_repo: Arc<R>, task_repo: Arc<T>, settings: ReviewSettings) -> Self {
        Self {
            review_repo,
            task_repo,
            settings,
        }
    }

    /// Start an AI review for a task
    ///
    /// Creates a Review record in Pending status for the given task.
    /// The actual reviewer agent should be spawned separately.
    pub async fn start_ai_review(&self, task_id: &TaskId, project_id: &ProjectId) -> AppResult<Review> {
        if !self.settings.should_run_ai_review() {
            return Err(AppError::Validation("AI review is disabled".into()));
        }

        // Check if task already has a pending review
        if self.review_repo.has_pending_review(task_id).await? {
            return Err(AppError::Validation(format!(
                "Task {} already has a pending review",
                task_id.as_str()
            )));
        }

        let review = Review::new(project_id.clone(), task_id.clone(), ReviewerType::Ai);
        self.review_repo.create(&review).await?;
        Ok(review)
    }

    /// Process the result of an AI review
    ///
    /// Handles the three possible outcomes: approved, needs_changes, escalate
    pub async fn process_review_result(
        &self,
        review: &mut Review,
        input: &CompleteReviewInput,
    ) -> AppResult<Option<TaskId>> {
        input.validate().map_err(|e| AppError::Validation(e.to_string()))?;

        match input.outcome {
            ReviewToolOutcome::Approved => {
                review.approve(Some(input.notes.clone()));
                self.review_repo.update(review).await?;
                self.add_review_note(&review.task_id, ReviewerType::Ai, ReviewOutcome::Approved, &input.notes).await?;
                self.add_action(&review.id, ReviewActionType::Approved, None).await?;
                Ok(None)
            }
            ReviewToolOutcome::NeedsChanges => {
                let fix_desc = input.fix_description.as_ref()
                    .ok_or_else(|| AppError::Validation("Missing fix_description".into()))?;

                review.request_changes(input.notes.clone());
                self.review_repo.update(review).await?;
                self.add_review_note(&review.task_id, ReviewerType::Ai, ReviewOutcome::ChangesRequested, &input.notes).await?;

                if self.settings.should_auto_create_fix() {
                    let fix_task = self.create_fix_task(&review.task_id, &review.project_id, fix_desc).await?;
                    self.add_action(&review.id, ReviewActionType::CreatedFixTask, Some(fix_task.id.clone())).await?;
                    Ok(Some(fix_task.id))
                } else {
                    self.add_action(&review.id, ReviewActionType::MovedToBacklog, None).await?;
                    Ok(None)
                }
            }
            ReviewToolOutcome::Escalate => {
                review.reject(input.notes.clone());
                self.review_repo.update(review).await?;
                self.add_review_note(&review.task_id, ReviewerType::Ai, ReviewOutcome::Rejected, &input.notes).await?;
                Ok(None)
            }
        }
    }

    /// Create a fix task for the original task
    pub async fn create_fix_task(
        &self,
        original_task_id: &TaskId,
        project_id: &ProjectId,
        fix_description: &str,
    ) -> AppResult<Task> {
        let original = self.task_repo.get_by_id(original_task_id).await?
            .ok_or_else(|| AppError::TaskNotFound(original_task_id.as_str().to_string()))?;

        let mut fix_task = Task::new_with_category(
            project_id.clone(),
            format!("Fix: {}", original.title),
            "fix".to_string(),
        );
        fix_task.set_description(Some(fix_description.to_string()));
        fix_task.set_priority(original.priority + 1); // Higher priority than original

        if self.settings.needs_fix_approval() {
            fix_task.internal_status = InternalStatus::Blocked; // Pending human approval
        } else {
            fix_task.internal_status = InternalStatus::Ready; // Ready to execute
        }

        self.task_repo.create(fix_task).await
    }

    /// Add a review note to the task's history
    async fn add_review_note(
        &self,
        task_id: &TaskId,
        reviewer: ReviewerType,
        outcome: ReviewOutcome,
        notes: &str,
    ) -> AppResult<()> {
        let note = ReviewNote::with_notes(task_id.clone(), reviewer, outcome, notes.to_string());
        self.review_repo.add_note(&note).await
    }

    /// Add an action record to the review
    async fn add_action(
        &self,
        review_id: &crate::domain::entities::ReviewId,
        action_type: ReviewActionType,
        target_task_id: Option<TaskId>,
    ) -> AppResult<()> {
        let action = if let Some(target) = target_task_id {
            ReviewAction::with_target_task(review_id.clone(), action_type, target)
        } else {
            ReviewAction::new(review_id.clone(), action_type)
        };
        self.review_repo.add_action(&action).await
    }

    /// Get the current settings
    pub fn settings(&self) -> &ReviewSettings {
        &self.settings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{ReviewId, ReviewStatus};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::RwLock;

    // Mock ReviewRepository
    struct MockReviewRepo {
        reviews: RwLock<HashMap<String, Review>>,
        actions: RwLock<Vec<ReviewAction>>,
        notes: RwLock<Vec<ReviewNote>>,
    }

    impl MockReviewRepo {
        fn new() -> Self {
            Self {
                reviews: RwLock::new(HashMap::new()),
                actions: RwLock::new(Vec::new()),
                notes: RwLock::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl ReviewRepository for MockReviewRepo {
        async fn create(&self, review: &Review) -> AppResult<()> {
            self.reviews.write().unwrap().insert(review.id.as_str().to_string(), review.clone());
            Ok(())
        }
        async fn get_by_id(&self, id: &ReviewId) -> AppResult<Option<Review>> {
            Ok(self.reviews.read().unwrap().get(id.as_str()).cloned())
        }
        async fn get_by_task_id(&self, task_id: &TaskId) -> AppResult<Vec<Review>> {
            Ok(self.reviews.read().unwrap().values().filter(|r| r.task_id == *task_id).cloned().collect())
        }
        async fn get_pending(&self, _project_id: &ProjectId) -> AppResult<Vec<Review>> { Ok(vec![]) }
        async fn update(&self, review: &Review) -> AppResult<()> {
            self.reviews.write().unwrap().insert(review.id.as_str().to_string(), review.clone());
            Ok(())
        }
        async fn delete(&self, _id: &ReviewId) -> AppResult<()> { Ok(()) }
        async fn add_action(&self, action: &ReviewAction) -> AppResult<()> {
            self.actions.write().unwrap().push(action.clone());
            Ok(())
        }
        async fn get_actions(&self, _review_id: &ReviewId) -> AppResult<Vec<ReviewAction>> { Ok(vec![]) }
        async fn get_action_by_id(&self, _id: &crate::domain::entities::ReviewActionId) -> AppResult<Option<ReviewAction>> { Ok(None) }
        async fn add_note(&self, note: &ReviewNote) -> AppResult<()> {
            self.notes.write().unwrap().push(note.clone());
            Ok(())
        }
        async fn get_notes_by_task_id(&self, _task_id: &TaskId) -> AppResult<Vec<ReviewNote>> { Ok(vec![]) }
        async fn get_note_by_id(&self, _id: &crate::domain::entities::ReviewNoteId) -> AppResult<Option<ReviewNote>> { Ok(None) }
        async fn get_by_status(&self, _project_id: &ProjectId, _status: ReviewStatus) -> AppResult<Vec<Review>> { Ok(vec![]) }
        async fn count_pending(&self, _project_id: &ProjectId) -> AppResult<u32> { Ok(0) }
        async fn has_pending_review(&self, task_id: &TaskId) -> AppResult<bool> {
            Ok(self.reviews.read().unwrap().values().any(|r| r.task_id == *task_id && r.is_pending()))
        }
    }

    // Mock TaskRepository
    struct MockTaskRepo {
        tasks: RwLock<HashMap<String, Task>>,
    }

    impl MockTaskRepo {
        fn new() -> Self { Self { tasks: RwLock::new(HashMap::new()) } }
        fn add_task(&self, task: Task) { self.tasks.write().unwrap().insert(task.id.as_str().to_string(), task); }
    }

    #[async_trait]
    impl TaskRepository for MockTaskRepo {
        async fn create(&self, task: Task) -> AppResult<Task> {
            self.tasks.write().unwrap().insert(task.id.as_str().to_string(), task.clone());
            Ok(task)
        }
        async fn get_by_id(&self, id: &TaskId) -> AppResult<Option<Task>> {
            Ok(self.tasks.read().unwrap().get(id.as_str()).cloned())
        }
        async fn get_by_project(&self, _project_id: &ProjectId) -> AppResult<Vec<Task>> { Ok(vec![]) }
        async fn update(&self, _task: &Task) -> AppResult<()> { Ok(()) }
        async fn delete(&self, _id: &TaskId) -> AppResult<()> { Ok(()) }
        async fn get_by_status(&self, _project_id: &ProjectId, _status: InternalStatus) -> AppResult<Vec<Task>> { Ok(vec![]) }
        async fn persist_status_change(&self, _id: &TaskId, _from: InternalStatus, _to: InternalStatus, _trigger: &str) -> AppResult<()> { Ok(()) }
        async fn get_status_history(&self, _id: &TaskId) -> AppResult<Vec<crate::domain::repositories::StatusTransition>> { Ok(vec![]) }
        async fn get_next_executable(&self, _project_id: &ProjectId) -> AppResult<Option<Task>> { Ok(None) }
        async fn get_blockers(&self, _id: &TaskId) -> AppResult<Vec<Task>> { Ok(vec![]) }
        async fn get_dependents(&self, _id: &TaskId) -> AppResult<Vec<Task>> { Ok(vec![]) }
        async fn add_blocker(&self, _task_id: &TaskId, _blocker_id: &TaskId) -> AppResult<()> { Ok(()) }
        async fn resolve_blocker(&self, _task_id: &TaskId, _blocker_id: &TaskId) -> AppResult<()> { Ok(()) }
    }

    fn setup() -> (Arc<MockReviewRepo>, Arc<MockTaskRepo>, ProjectId, TaskId) {
        let review_repo = Arc::new(MockReviewRepo::new());
        let task_repo = Arc::new(MockTaskRepo::new());
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());
        let task = Task::new(project_id.clone(), "Original Task".to_string());
        let mut task_with_id = task; task_with_id.id = task_id.clone();
        task_repo.add_task(task_with_id);
        (review_repo, task_repo, project_id, task_id)
    }

    #[tokio::test]
    async fn test_start_ai_review_success() {
        let (review_repo, task_repo, project_id, task_id) = setup();
        let service = ReviewService::new(review_repo.clone(), task_repo);
        let review = service.start_ai_review(&task_id, &project_id).await.unwrap();
        assert_eq!(review.task_id, task_id);
        assert_eq!(review.reviewer_type, ReviewerType::Ai);
        assert!(review.is_pending());
    }

    #[tokio::test]
    async fn test_start_ai_review_disabled() {
        let (review_repo, task_repo, project_id, task_id) = setup();
        let service = ReviewService::with_settings(review_repo, task_repo, ReviewSettings::ai_disabled());
        let result = service.start_ai_review(&task_id, &project_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_start_ai_review_already_pending() {
        let (review_repo, task_repo, project_id, task_id) = setup();
        let service = ReviewService::new(review_repo.clone(), task_repo);
        service.start_ai_review(&task_id, &project_id).await.unwrap();
        let result = service.start_ai_review(&task_id, &project_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_process_review_approved() {
        let (review_repo, task_repo, project_id, task_id) = setup();
        let service = ReviewService::new(review_repo.clone(), task_repo);
        let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
        let input = CompleteReviewInput::approved("All tests pass");
        let result = service.process_review_result(&mut review, &input).await.unwrap();
        assert!(result.is_none());
        assert!(review.is_approved());
    }

    #[tokio::test]
    async fn test_process_review_needs_changes_creates_fix_task() {
        let (review_repo, task_repo, project_id, task_id) = setup();
        let service = ReviewService::new(review_repo.clone(), task_repo.clone());
        let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
        let input = CompleteReviewInput::needs_changes("Missing error handling", "Add try-catch");
        let result = service.process_review_result(&mut review, &input).await.unwrap();
        assert!(result.is_some());
        let fix_task_id = result.unwrap();
        let fix_task = task_repo.get_by_id(&fix_task_id).await.unwrap().unwrap();
        assert!(fix_task.title.starts_with("Fix:"));
        assert_eq!(fix_task.category, "fix");
    }

    #[tokio::test]
    async fn test_process_review_escalate() {
        let (review_repo, task_repo, project_id, task_id) = setup();
        let service = ReviewService::new(review_repo.clone(), task_repo);
        let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
        let input = CompleteReviewInput::escalate("Security concern", "Needs human review");
        let result = service.process_review_result(&mut review, &input).await.unwrap();
        assert!(result.is_none());
        assert_eq!(review.status, ReviewStatus::Rejected);
    }

    #[tokio::test]
    async fn test_fix_task_requires_approval_when_configured() {
        let (review_repo, task_repo, project_id, task_id) = setup();
        let service = ReviewService::with_settings(review_repo, task_repo.clone(), ReviewSettings::with_fix_approval());
        let fix_task = service.create_fix_task(&task_id, &project_id, "Fix the bug").await.unwrap();
        assert_eq!(fix_task.internal_status, InternalStatus::Blocked);
    }
}
