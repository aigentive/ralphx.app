// ReviewIssueService
// Application service for managing review issue lifecycle: creation, progress tracking, verification

use std::sync::Arc;

use crate::domain::entities::{
    IssueCategory, IssueProgressSummary, IssueSeverity, IssueStatus, ReviewIssueEntity,
    ReviewIssueId, ReviewNoteId, TaskId, TaskStepId,
};
use crate::error::{AppError, AppResult};
use crate::infrastructure::sqlite::ReviewIssueRepository;

/// Input for creating a single issue from a review
#[derive(Debug, Clone)]
pub struct CreateIssueInput {
    pub title: String,
    pub description: Option<String>,
    pub severity: IssueSeverity,
    pub category: Option<IssueCategory>,
    pub step_id: Option<TaskStepId>,
    pub no_step_reason: Option<String>,
    pub file_path: Option<String>,
    pub line_number: Option<i32>,
    pub code_snippet: Option<String>,
}

impl CreateIssueInput {
    /// Validate that either step_id or no_step_reason is provided
    pub fn validate(&self) -> Result<(), String> {
        if self.step_id.is_none() && self.no_step_reason.is_none() {
            return Err("Either step_id or no_step_reason must be provided for issue".to_string());
        }
        if self.title.trim().is_empty() {
            return Err("Issue title cannot be empty".to_string());
        }
        Ok(())
    }
}

/// Service for orchestrating review issue lifecycle
pub struct ReviewIssueService<R: ReviewIssueRepository> {
    repo: Arc<R>,
}

impl<R: ReviewIssueRepository> ReviewIssueService<R> {
    /// Create a new review issue service
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    /// Create issues from a review
    ///
    /// Creates multiple issues in a single transaction. All issues must have
    /// either step_id or no_step_reason provided.
    pub async fn create_issues_from_review(
        &self,
        review_note_id: ReviewNoteId,
        task_id: TaskId,
        inputs: Vec<CreateIssueInput>,
    ) -> AppResult<Vec<ReviewIssueEntity>> {
        // Validate all inputs first
        for (i, input) in inputs.iter().enumerate() {
            input
                .validate()
                .map_err(|e| AppError::Validation(format!("Issue {}: {}", i + 1, e)))?;
        }

        // Convert inputs to entities
        let issues: Vec<ReviewIssueEntity> = inputs
            .into_iter()
            .map(|input| {
                let mut issue = ReviewIssueEntity::new(
                    review_note_id.clone(),
                    task_id.clone(),
                    input.title,
                    input.severity,
                );
                issue.description = input.description;
                issue.category = input.category;
                issue.step_id = input.step_id;
                issue.no_step_reason = input.no_step_reason;
                issue.file_path = input.file_path;
                issue.line_number = input.line_number;
                issue.code_snippet = input.code_snippet;
                issue
            })
            .collect();

        // Bulk create in a transaction
        self.repo.bulk_create(issues).await
    }

    /// Mark an issue as being worked on
    ///
    /// Transitions issue from Open to InProgress.
    pub async fn mark_issue_in_progress(
        &self,
        issue_id: &ReviewIssueId,
    ) -> AppResult<ReviewIssueEntity> {
        let mut issue = self.get_issue_or_error(issue_id).await?;

        if issue.status != IssueStatus::Open {
            return Err(AppError::Validation(format!(
                "Cannot mark issue as in_progress: current status is {} (expected open)",
                issue.status
            )));
        }

        issue.start_work();
        self.repo.update(&issue).await?;
        Ok(issue)
    }

    /// Mark an issue as addressed
    ///
    /// Transitions issue from Open or InProgress to Addressed.
    pub async fn mark_issue_addressed(
        &self,
        issue_id: &ReviewIssueId,
        resolution_notes: Option<String>,
        attempt_number: i32,
    ) -> AppResult<ReviewIssueEntity> {
        let mut issue = self.get_issue_or_error(issue_id).await?;

        if !issue.needs_work() {
            return Err(AppError::Validation(format!(
                "Cannot mark issue as addressed: current status is {} (expected open or in_progress)",
                issue.status
            )));
        }

        issue.mark_addressed(resolution_notes, attempt_number);
        self.repo.update(&issue).await?;
        Ok(issue)
    }

    /// Verify an issue as fixed
    ///
    /// Transitions issue from Addressed to Verified.
    /// Called when a subsequent review confirms the issue is resolved.
    pub async fn verify_issue(
        &self,
        issue_id: &ReviewIssueId,
        review_note_id: ReviewNoteId,
    ) -> AppResult<ReviewIssueEntity> {
        let mut issue = self.get_issue_or_error(issue_id).await?;

        if issue.status != IssueStatus::Addressed {
            return Err(AppError::Validation(format!(
                "Cannot verify issue: current status is {} (expected addressed)",
                issue.status
            )));
        }

        issue.verify(review_note_id);
        self.repo.update(&issue).await?;
        Ok(issue)
    }

    /// Reopen an issue that was not actually fixed
    ///
    /// Transitions issue from Addressed back to Open.
    /// Called when a subsequent review finds the issue wasn't properly addressed.
    pub async fn reopen_issue(
        &self,
        issue_id: &ReviewIssueId,
        reason: Option<String>,
    ) -> AppResult<ReviewIssueEntity> {
        let mut issue = self.get_issue_or_error(issue_id).await?;

        if issue.status != IssueStatus::Addressed {
            return Err(AppError::Validation(format!(
                "Cannot reopen issue: current status is {} (expected addressed)",
                issue.status
            )));
        }

        issue.reopen(reason);
        self.repo.update(&issue).await?;
        Ok(issue)
    }

    /// Mark an issue as won't fix
    ///
    /// Terminal state indicating the issue will not be addressed.
    pub async fn mark_issue_wontfix(
        &self,
        issue_id: &ReviewIssueId,
        reason: String,
    ) -> AppResult<ReviewIssueEntity> {
        let mut issue = self.get_issue_or_error(issue_id).await?;

        if issue.is_terminal() {
            return Err(AppError::Validation(format!(
                "Cannot mark issue as wontfix: issue is already in terminal status {}",
                issue.status
            )));
        }

        issue.wont_fix(reason);
        self.repo.update(&issue).await?;
        Ok(issue)
    }

    /// Get issue progress summary for a task
    pub async fn get_issue_progress(&self, task_id: &TaskId) -> AppResult<IssueProgressSummary> {
        self.repo.get_summary(task_id).await
    }

    /// Get an issue by ID
    pub async fn get_issue(
        &self,
        issue_id: &ReviewIssueId,
    ) -> AppResult<Option<ReviewIssueEntity>> {
        self.repo.get_by_id(issue_id).await
    }

    /// Get all issues for a task
    pub async fn get_issues_by_task(&self, task_id: &TaskId) -> AppResult<Vec<ReviewIssueEntity>> {
        self.repo.get_by_task_id(task_id).await
    }

    /// Get open issues for a task
    pub async fn get_open_issues_by_task(
        &self,
        task_id: &TaskId,
    ) -> AppResult<Vec<ReviewIssueEntity>> {
        self.repo.get_open_by_task_id(task_id).await
    }

    /// Helper to get an issue or return NotFound error
    async fn get_issue_or_error(&self, issue_id: &ReviewIssueId) -> AppResult<ReviewIssueEntity> {
        self.repo
            .get_by_id(issue_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Issue {} not found", issue_id.as_str())))
    }
}

#[cfg(test)]
#[path = "review_issue_service_tests.rs"]
mod tests;
