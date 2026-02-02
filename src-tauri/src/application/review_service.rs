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

    // ========================================
    // Fix Task Workflow Methods
    // ========================================

    /// Approve a fix task, changing its status from Blocked to Ready
    ///
    /// This is called when a human approves an AI-proposed fix task.
    pub async fn approve_fix_task(&self, fix_task_id: &TaskId) -> AppResult<()> {
        let mut fix_task = self.task_repo.get_by_id(fix_task_id).await?
            .ok_or_else(|| AppError::TaskNotFound(fix_task_id.as_str().to_string()))?;

        if fix_task.internal_status != InternalStatus::Blocked {
            return Err(AppError::Validation(format!(
                "Fix task {} is not in Blocked status (current: {:?})",
                fix_task_id.as_str(),
                fix_task.internal_status
            )));
        }

        fix_task.internal_status = InternalStatus::Ready;
        self.task_repo.update(&fix_task).await
    }

    /// Reject a fix task with feedback and optionally create a new fix proposal
    ///
    /// If the original task has not exceeded max_fix_attempts, creates a new fix task
    /// with the provided feedback. Otherwise, moves the original task to backlog.
    ///
    /// Returns: Some(new_fix_task_id) if new fix created, None if max attempts reached
    pub async fn reject_fix_task(
        &self,
        fix_task_id: &TaskId,
        feedback: &str,
        original_task_id: &TaskId,
    ) -> AppResult<Option<TaskId>> {
        // Get fix task and mark as rejected
        let mut fix_task = self.task_repo.get_by_id(fix_task_id).await?
            .ok_or_else(|| AppError::TaskNotFound(fix_task_id.as_str().to_string()))?;

        fix_task.internal_status = InternalStatus::Failed;
        self.task_repo.update(&fix_task).await?;

        // Get original task
        let original_task = self.task_repo.get_by_id(original_task_id).await?
            .ok_or_else(|| AppError::TaskNotFound(original_task_id.as_str().to_string()))?;

        // Count fix attempts for original task
        let attempt_count = self.get_fix_attempt_count(original_task_id).await?;

        // Check if we've exceeded max attempts
        if self.settings.exceeded_max_attempts(attempt_count) {
            // Move original task to backlog
            let mut orig = original_task;
            orig.internal_status = InternalStatus::Backlog;
            self.task_repo.update(&orig).await?;

            // Add a note about max attempts reached
            self.add_review_note(
                original_task_id,
                ReviewerType::Ai,
                ReviewOutcome::Rejected,
                &format!(
                    "Max fix attempts ({}) reached. Task moved to backlog. Last feedback: {}",
                    self.settings.max_fix_attempts, feedback
                ),
            ).await?;

            return Ok(None);
        }

        // Create new fix task with feedback
        let new_fix_description = format!(
            "Previous fix rejected. Feedback: {}\n\nOriginal issue: {}",
            feedback,
            fix_task.description.as_deref().unwrap_or("No description")
        );

        let new_fix_task = self.create_fix_task(
            original_task_id,
            &original_task.project_id,
            &new_fix_description,
        ).await?;

        Ok(Some(new_fix_task.id))
    }

    /// Get the number of fix task attempts for a task
    pub async fn get_fix_attempt_count(&self, task_id: &TaskId) -> AppResult<u32> {
        self.review_repo.count_fix_actions(task_id).await
    }

    /// Move a task to backlog (used when giving up on fixes)
    pub async fn move_to_backlog(&self, task_id: &TaskId, reason: &str) -> AppResult<()> {
        let mut task = self.task_repo.get_by_id(task_id).await?
            .ok_or_else(|| AppError::TaskNotFound(task_id.as_str().to_string()))?;

        task.internal_status = InternalStatus::Backlog;
        self.task_repo.update(&task).await?;

        // Add a note about why it was moved to backlog
        self.add_review_note(
            task_id,
            ReviewerType::Ai,
            ReviewOutcome::Rejected,
            reason,
        ).await
    }

    // ========================================
    // Human Review Methods
    // ========================================

    /// Start a human review for a task
    ///
    /// Creates a Review record in Pending status for manual human review.
    /// The task should already be in a state that requires human review
    /// (e.g., escalated from AI, or require_human_review is enabled).
    pub async fn start_human_review(&self, task_id: &TaskId, project_id: &ProjectId) -> AppResult<Review> {
        // Check if task already has a pending review
        if self.review_repo.has_pending_review(task_id).await? {
            return Err(AppError::Validation(format!(
                "Task {} already has a pending review",
                task_id.as_str()
            )));
        }

        // Verify task exists
        let _task = self.task_repo.get_by_id(task_id).await?
            .ok_or_else(|| AppError::TaskNotFound(task_id.as_str().to_string()))?;

        let review = Review::new(project_id.clone(), task_id.clone(), ReviewerType::Human);
        self.review_repo.create(&review).await?;
        Ok(review)
    }

    /// Approve a human review
    ///
    /// Marks the review as approved with optional notes.
    /// The task should transition to the approved state.
    pub async fn approve_human_review(
        &self,
        review_id: &crate::domain::entities::ReviewId,
        notes: Option<String>,
    ) -> AppResult<()> {
        let mut review = self.review_repo.get_by_id(review_id).await?
            .ok_or_else(|| AppError::Validation(format!(
                "Review {} not found",
                review_id.as_str()
            )))?;

        if !review.is_pending() {
            return Err(AppError::Validation(format!(
                "Review {} is not pending (current: {})",
                review_id.as_str(),
                review.status
            )));
        }

        review.approve(notes.clone());
        self.review_repo.update(&review).await?;

        // Add review note for history
        self.add_review_note(
            &review.task_id,
            ReviewerType::Human,
            ReviewOutcome::Approved,
            notes.as_deref().unwrap_or("Approved by human reviewer"),
        ).await?;

        // Add action record
        self.add_action(&review.id, ReviewActionType::Approved, None).await
    }

    /// Request changes during human review
    ///
    /// Marks the review as changes_requested and optionally creates a fix task.
    /// Returns the fix task ID if one was created.
    pub async fn request_changes(
        &self,
        review_id: &crate::domain::entities::ReviewId,
        notes: String,
        fix_description: Option<String>,
    ) -> AppResult<Option<TaskId>> {
        let mut review = self.review_repo.get_by_id(review_id).await?
            .ok_or_else(|| AppError::Validation(format!(
                "Review {} not found",
                review_id.as_str()
            )))?;

        if !review.is_pending() {
            return Err(AppError::Validation(format!(
                "Review {} is not pending (current: {})",
                review_id.as_str(),
                review.status
            )));
        }

        review.request_changes(notes.clone());
        self.review_repo.update(&review).await?;

        // Add review note for history
        self.add_review_note(
            &review.task_id,
            ReviewerType::Human,
            ReviewOutcome::ChangesRequested,
            &notes,
        ).await?;

        // Create fix task if description provided
        if let Some(fix_desc) = fix_description {
            let fix_task = self.create_fix_task(&review.task_id, &review.project_id, &fix_desc).await?;
            self.add_action(&review.id, ReviewActionType::CreatedFixTask, Some(fix_task.id.clone())).await?;
            Ok(Some(fix_task.id))
        } else {
            Ok(None)
        }
    }

    /// Reject a human review
    ///
    /// Marks the review as rejected with notes.
    /// The task should transition to a failed/rejected state.
    pub async fn reject_human_review(
        &self,
        review_id: &crate::domain::entities::ReviewId,
        notes: String,
    ) -> AppResult<()> {
        let mut review = self.review_repo.get_by_id(review_id).await?
            .ok_or_else(|| AppError::Validation(format!(
                "Review {} not found",
                review_id.as_str()
            )))?;

        if !review.is_pending() {
            return Err(AppError::Validation(format!(
                "Review {} is not pending (current: {})",
                review_id.as_str(),
                review.status
            )));
        }

        review.reject(notes.clone());
        self.review_repo.update(&review).await?;

        // Add review note for history
        self.add_review_note(
            &review.task_id,
            ReviewerType::Human,
            ReviewOutcome::Rejected,
            &notes,
        ).await?;

        // Move task to failed status
        let mut task = self.task_repo.get_by_id(&review.task_id).await?
            .ok_or_else(|| AppError::TaskNotFound(review.task_id.as_str().to_string()))?;

        task.internal_status = InternalStatus::Failed;
        self.task_repo.update(&task).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{ReviewId, ReviewStatus};
    use crate::domain::repositories::StateHistoryMetadata;
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
        async fn count_fix_actions(&self, task_id: &TaskId) -> AppResult<u32> {
            let reviews = self.reviews.read().unwrap();
            let actions = self.actions.read().unwrap();
            let mut count = 0u32;
            for review in reviews.values() {
                if review.task_id == *task_id {
                    for action in actions.iter() {
                        if action.review_id == review.id
                            && action.action_type == crate::domain::entities::ReviewActionType::CreatedFixTask
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
                    for action in actions.iter() {
                        if action.review_id == review.id
                            && action.action_type == crate::domain::entities::ReviewActionType::CreatedFixTask
                        {
                            result.push(action.clone());
                        }
                    }
                }
            }
            Ok(result)
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
        async fn update(&self, task: &Task) -> AppResult<()> {
            self.tasks.write().unwrap().insert(task.id.as_str().to_string(), task.clone());
            Ok(())
        }
        async fn delete(&self, _id: &TaskId) -> AppResult<()> { Ok(()) }
        async fn get_by_status(&self, _project_id: &ProjectId, _status: InternalStatus) -> AppResult<Vec<Task>> { Ok(vec![]) }
        async fn persist_status_change(&self, _id: &TaskId, _from: InternalStatus, _to: InternalStatus, _trigger: &str) -> AppResult<()> { Ok(()) }
        async fn get_status_history(&self, _id: &TaskId) -> AppResult<Vec<crate::domain::repositories::StatusTransition>> { Ok(vec![]) }
        async fn get_next_executable(&self, _project_id: &ProjectId) -> AppResult<Option<Task>> { Ok(None) }
        async fn get_blockers(&self, _id: &TaskId) -> AppResult<Vec<Task>> { Ok(vec![]) }
        async fn get_dependents(&self, _id: &TaskId) -> AppResult<Vec<Task>> { Ok(vec![]) }
        async fn add_blocker(&self, _task_id: &TaskId, _blocker_id: &TaskId) -> AppResult<()> { Ok(()) }
        async fn resolve_blocker(&self, _task_id: &TaskId, _blocker_id: &TaskId) -> AppResult<()> { Ok(()) }
        async fn get_by_project_filtered(&self, _project_id: &ProjectId, _include_archived: bool) -> AppResult<Vec<Task>> { Ok(vec![]) }
        async fn archive(&self, task_id: &TaskId) -> AppResult<Task> {
            if let Some(task) = self.tasks.read().unwrap().get(task_id.as_str()) {
                let mut archived = task.clone();
                archived.archived_at = Some(chrono::Utc::now());
                self.tasks.write().unwrap().insert(task_id.as_str().to_string(), archived.clone());
                Ok(archived)
            } else {
                Err(crate::error::AppError::NotFound(format!("Task {} not found", task_id.as_str())))
            }
        }
        async fn restore(&self, task_id: &TaskId) -> AppResult<Task> {
            if let Some(task) = self.tasks.read().unwrap().get(task_id.as_str()) {
                let mut restored = task.clone();
                restored.archived_at = None;
                self.tasks.write().unwrap().insert(task_id.as_str().to_string(), restored.clone());
                Ok(restored)
            } else {
                Err(crate::error::AppError::NotFound(format!("Task {} not found", task_id.as_str())))
            }
        }
        async fn get_archived_count(&self, _project_id: &ProjectId) -> AppResult<u32> { Ok(0) }

        async fn list_paginated(
            &self,
            _project_id: &ProjectId,
            _statuses: Option<Vec<InternalStatus>>,
            _offset: u32,
            _limit: u32,
            _include_archived: bool,
        ) -> AppResult<Vec<Task>> {
            Ok(self.tasks.read().unwrap().values().cloned().collect())
        }

        async fn count_tasks(&self, _project_id: &ProjectId, _include_archived: bool) -> AppResult<u32> {
            Ok(self.tasks.read().unwrap().len() as u32)
        }

        async fn search(
            &self,
            _project_id: &ProjectId,
            _query: &str,
            _include_archived: bool,
        ) -> AppResult<Vec<Task>> {
            Ok(vec![])
        }

        async fn get_oldest_ready_task(&self) -> AppResult<Option<Task>> {
            Ok(None)
        }

        async fn get_oldest_ready_tasks(&self, _limit: u32) -> AppResult<Vec<Task>> {
            Ok(vec![])
        }

        async fn update_latest_state_history_metadata(
            &self,
            _task_id: &TaskId,
            _metadata: &StateHistoryMetadata,
        ) -> AppResult<()> {
            Ok(())
        }

        async fn has_task_in_states(
            &self,
            _project_id: &ProjectId,
            _statuses: &[InternalStatus],
        ) -> AppResult<bool> {
            Ok(false)
        }
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
        use crate::domain::entities::IssueSeverity;
        use crate::domain::tools::ReviewIssueInput;
        let (review_repo, task_repo, project_id, task_id) = setup();
        let service = ReviewService::new(review_repo.clone(), task_repo.clone());
        let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
        let issue = ReviewIssueInput::new("Missing error handling", IssueSeverity::Major)
            .with_no_step_reason("General code quality");
        let input = CompleteReviewInput::needs_changes_with_issues(
            "Missing error handling",
            "Add try-catch",
            vec![issue],
        );
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

    // ========================================
    // Fix Task Workflow Tests
    // ========================================

    #[tokio::test]
    async fn test_approve_fix_task_success() {
        let (review_repo, task_repo, project_id, task_id) = setup();
        let service = ReviewService::with_settings(review_repo, task_repo.clone(), ReviewSettings::with_fix_approval());

        // Create a fix task (will be Blocked due to with_fix_approval)
        let fix_task = service.create_fix_task(&task_id, &project_id, "Fix the bug").await.unwrap();
        assert_eq!(fix_task.internal_status, InternalStatus::Blocked);

        // Approve it
        service.approve_fix_task(&fix_task.id).await.unwrap();

        // Verify it's now Ready
        let updated = task_repo.get_by_id(&fix_task.id).await.unwrap().unwrap();
        assert_eq!(updated.internal_status, InternalStatus::Ready);
    }

    #[tokio::test]
    async fn test_approve_fix_task_not_blocked_fails() {
        let (review_repo, task_repo, project_id, task_id) = setup();
        // Use default settings (fix tasks are Ready, not Blocked)
        let service = ReviewService::new(review_repo, task_repo.clone());

        // Create a fix task (will be Ready)
        let fix_task = service.create_fix_task(&task_id, &project_id, "Fix the bug").await.unwrap();
        assert_eq!(fix_task.internal_status, InternalStatus::Ready);

        // Trying to approve should fail
        let result = service.approve_fix_task(&fix_task.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_approve_fix_task_not_found() {
        let (review_repo, task_repo, _project_id, _task_id) = setup();
        let service = ReviewService::new(review_repo, task_repo);

        let nonexistent_id = TaskId::from_string("nonexistent".to_string());
        let result = service.approve_fix_task(&nonexistent_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_reject_fix_task_creates_new_fix() {
        use crate::domain::entities::IssueSeverity;
        use crate::domain::tools::ReviewIssueInput;
        let (review_repo, task_repo, project_id, task_id) = setup();
        let service = ReviewService::with_settings(review_repo.clone(), task_repo.clone(), ReviewSettings::with_fix_approval());

        // Create a review and fix task
        let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
        let issue = ReviewIssueInput::new("Missing tests", IssueSeverity::Major)
            .with_no_step_reason("General requirement");
        let input = CompleteReviewInput::needs_changes_with_issues(
            "Missing tests",
            "Add tests",
            vec![issue],
        );
        let fix_task_id = service.process_review_result(&mut review, &input).await.unwrap().unwrap();

        // Reject the fix task
        let new_fix_id = service.reject_fix_task(&fix_task_id, "Not good enough", &task_id).await.unwrap();

        // Should have created a new fix task
        assert!(new_fix_id.is_some());
        let new_fix = task_repo.get_by_id(&new_fix_id.unwrap()).await.unwrap().unwrap();
        assert!(new_fix.title.starts_with("Fix:"));
        assert!(new_fix.description.as_ref().unwrap().contains("Not good enough"));

        // Original fix task should be Failed
        let old_fix = task_repo.get_by_id(&fix_task_id).await.unwrap().unwrap();
        assert_eq!(old_fix.internal_status, InternalStatus::Failed);
    }

    #[tokio::test]
    async fn test_reject_fix_task_max_attempts_moves_to_backlog() {
        use crate::domain::entities::IssueSeverity;
        use crate::domain::tools::ReviewIssueInput;
        let (review_repo, task_repo, project_id, task_id) = setup();
        // Set max_fix_attempts to 1
        let settings = ReviewSettings::with_max_attempts(1);
        let service = ReviewService::with_settings(review_repo.clone(), task_repo.clone(), settings);

        // Create a review and fix task
        let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
        let issue = ReviewIssueInput::new("Missing tests", IssueSeverity::Major)
            .with_no_step_reason("General requirement");
        let input = CompleteReviewInput::needs_changes_with_issues(
            "Missing tests",
            "Add tests",
            vec![issue],
        );
        let fix_task_id = service.process_review_result(&mut review, &input).await.unwrap().unwrap();

        // At this point we have 1 fix action, which equals max_fix_attempts
        // Reject should move original to backlog
        let new_fix_id = service.reject_fix_task(&fix_task_id, "Still not good", &task_id).await.unwrap();

        // Should NOT have created a new fix task
        assert!(new_fix_id.is_none());

        // Original task should be in Backlog
        let original = task_repo.get_by_id(&task_id).await.unwrap().unwrap();
        assert_eq!(original.internal_status, InternalStatus::Backlog);
    }

    #[tokio::test]
    async fn test_get_fix_attempt_count() {
        use crate::domain::entities::IssueSeverity;
        use crate::domain::tools::ReviewIssueInput;
        let (review_repo, task_repo, project_id, task_id) = setup();
        let service = ReviewService::new(review_repo.clone(), task_repo.clone());

        // Initially 0
        assert_eq!(service.get_fix_attempt_count(&task_id).await.unwrap(), 0);

        // Create a review and add a fix task
        let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
        let issue = ReviewIssueInput::new("Missing tests", IssueSeverity::Major)
            .with_no_step_reason("General requirement");
        let input = CompleteReviewInput::needs_changes_with_issues(
            "Missing tests",
            "Add tests",
            vec![issue],
        );
        service.process_review_result(&mut review, &input).await.unwrap();

        // Now should be 1
        assert_eq!(service.get_fix_attempt_count(&task_id).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_move_to_backlog() {
        let (review_repo, task_repo, _project_id, task_id) = setup();
        let service = ReviewService::new(review_repo.clone(), task_repo.clone());

        // Update task status to something other than Backlog
        let mut task = task_repo.get_by_id(&task_id).await.unwrap().unwrap();
        task.internal_status = InternalStatus::PendingReview;
        task_repo.update(&task).await.unwrap();

        // Move to backlog
        service.move_to_backlog(&task_id, "Too complex to fix automatically").await.unwrap();

        // Verify it's in Backlog
        let updated = task_repo.get_by_id(&task_id).await.unwrap().unwrap();
        assert_eq!(updated.internal_status, InternalStatus::Backlog);
    }

    // ========================================
    // Human Review Tests
    // ========================================

    #[tokio::test]
    async fn test_start_human_review_success() {
        let (review_repo, task_repo, project_id, task_id) = setup();
        let service = ReviewService::new(review_repo.clone(), task_repo);

        let review = service.start_human_review(&task_id, &project_id).await.unwrap();

        assert_eq!(review.task_id, task_id);
        assert_eq!(review.reviewer_type, ReviewerType::Human);
        assert!(review.is_pending());
    }

    #[tokio::test]
    async fn test_start_human_review_already_pending() {
        let (review_repo, task_repo, project_id, task_id) = setup();
        let service = ReviewService::new(review_repo.clone(), task_repo);

        // Start first human review
        service.start_human_review(&task_id, &project_id).await.unwrap();

        // Trying to start another should fail
        let result = service.start_human_review(&task_id, &project_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_start_human_review_task_not_found() {
        let (review_repo, task_repo, project_id, _task_id) = setup();
        let service = ReviewService::new(review_repo, task_repo);

        let nonexistent_id = TaskId::from_string("nonexistent".to_string());
        let result = service.start_human_review(&nonexistent_id, &project_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_approve_human_review_success() {
        let (review_repo, task_repo, project_id, task_id) = setup();
        let service = ReviewService::new(review_repo.clone(), task_repo);

        // Start a human review
        let review = service.start_human_review(&task_id, &project_id).await.unwrap();

        // Approve it
        service.approve_human_review(&review.id, Some("Looks good!".to_string())).await.unwrap();

        // Verify review is approved
        let updated = review_repo.get_by_id(&review.id).await.unwrap().unwrap();
        assert!(updated.is_approved());
        assert_eq!(updated.notes, Some("Looks good!".to_string()));
    }

    #[tokio::test]
    async fn test_approve_human_review_without_notes() {
        let (review_repo, task_repo, project_id, task_id) = setup();
        let service = ReviewService::new(review_repo.clone(), task_repo);

        let review = service.start_human_review(&task_id, &project_id).await.unwrap();
        service.approve_human_review(&review.id, None).await.unwrap();

        let updated = review_repo.get_by_id(&review.id).await.unwrap().unwrap();
        assert!(updated.is_approved());
    }

    #[tokio::test]
    async fn test_approve_human_review_not_pending() {
        let (review_repo, task_repo, project_id, task_id) = setup();
        let service = ReviewService::new(review_repo.clone(), task_repo);

        // Start and approve a review
        let review = service.start_human_review(&task_id, &project_id).await.unwrap();
        service.approve_human_review(&review.id, None).await.unwrap();

        // Trying to approve again should fail
        let result = service.approve_human_review(&review.id, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_approve_human_review_not_found() {
        let (review_repo, task_repo, _project_id, _task_id) = setup();
        let service = ReviewService::new(review_repo, task_repo);

        let nonexistent_id = ReviewId::from_string("nonexistent");
        let result = service.approve_human_review(&nonexistent_id, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_request_changes_without_fix() {
        let (review_repo, task_repo, project_id, task_id) = setup();
        let service = ReviewService::new(review_repo.clone(), task_repo);

        let review = service.start_human_review(&task_id, &project_id).await.unwrap();
        let result = service.request_changes(&review.id, "Missing tests".to_string(), None).await.unwrap();

        // Should not create a fix task
        assert!(result.is_none());

        // Review should be changes_requested
        let updated = review_repo.get_by_id(&review.id).await.unwrap().unwrap();
        assert_eq!(updated.status, ReviewStatus::ChangesRequested);
        assert_eq!(updated.notes, Some("Missing tests".to_string()));
    }

    #[tokio::test]
    async fn test_request_changes_with_fix() {
        let (review_repo, task_repo, project_id, task_id) = setup();
        let service = ReviewService::new(review_repo.clone(), task_repo.clone());

        let review = service.start_human_review(&task_id, &project_id).await.unwrap();
        let result = service.request_changes(
            &review.id,
            "Missing tests".to_string(),
            Some("Add unit tests for validation".to_string()),
        ).await.unwrap();

        // Should have created a fix task
        assert!(result.is_some());
        let fix_task_id = result.unwrap();

        // Verify fix task was created
        let fix_task = task_repo.get_by_id(&fix_task_id).await.unwrap().unwrap();
        assert!(fix_task.title.starts_with("Fix:"));
        assert_eq!(fix_task.category, "fix");
        assert!(fix_task.description.as_ref().unwrap().contains("Add unit tests"));
    }

    #[tokio::test]
    async fn test_request_changes_not_pending() {
        let (review_repo, task_repo, project_id, task_id) = setup();
        let service = ReviewService::new(review_repo.clone(), task_repo);

        // Start and approve a review
        let review = service.start_human_review(&task_id, &project_id).await.unwrap();
        service.approve_human_review(&review.id, None).await.unwrap();

        // Trying to request changes should fail
        let result = service.request_changes(&review.id, "Changes needed".to_string(), None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_reject_human_review_success() {
        let (review_repo, task_repo, project_id, task_id) = setup();
        let service = ReviewService::new(review_repo.clone(), task_repo.clone());

        let review = service.start_human_review(&task_id, &project_id).await.unwrap();
        service.reject_human_review(&review.id, "Fundamentally wrong approach".to_string()).await.unwrap();

        // Verify review is rejected
        let updated_review = review_repo.get_by_id(&review.id).await.unwrap().unwrap();
        assert_eq!(updated_review.status, ReviewStatus::Rejected);
        assert_eq!(updated_review.notes, Some("Fundamentally wrong approach".to_string()));

        // Verify task is Failed
        let updated_task = task_repo.get_by_id(&task_id).await.unwrap().unwrap();
        assert_eq!(updated_task.internal_status, InternalStatus::Failed);
    }

    #[tokio::test]
    async fn test_reject_human_review_not_pending() {
        let (review_repo, task_repo, project_id, task_id) = setup();
        let service = ReviewService::new(review_repo.clone(), task_repo);

        // Start and reject a review
        let review = service.start_human_review(&task_id, &project_id).await.unwrap();
        service.reject_human_review(&review.id, "Bad approach".to_string()).await.unwrap();

        // Trying to reject again should fail
        let result = service.reject_human_review(&review.id, "Still bad".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_reject_human_review_not_found() {
        let (review_repo, task_repo, _project_id, _task_id) = setup();
        let service = ReviewService::new(review_repo, task_repo);

        let nonexistent_id = ReviewId::from_string("nonexistent");
        let result = service.reject_human_review(&nonexistent_id, "Rejected".to_string()).await;
        assert!(result.is_err());
    }
}
