// Tauri commands for Review operations
// Thin layer that delegates to ReviewRepository and ReviewService

use tauri::{Emitter, State};

use crate::application::AppState;
use crate::domain::entities::{ProjectId, ReviewId, ReviewNote, ReviewOutcome, ReviewerType, TaskId};

// Re-export types for external use
pub use super::review_commands_types::{
    ApproveFixTaskInput, ApproveReviewInput, FixTaskAttemptsResponse, IssueProgressResponse,
    MarkIssueAddressedInput, MarkIssueInProgressInput, RejectFixTaskInput, RejectReviewInput,
    ReopenIssueInput, RequestChangesInput, ReviewActionResponse, ReviewIssueResponse,
    ReviewNoteResponse, ReviewResponse, VerifyIssueInput,
};

// ============================================================================
// Commands - Read operations
// ============================================================================

/// Get all pending reviews for a project
#[tauri::command]
pub async fn get_pending_reviews(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ReviewResponse>, String> {
    let project_id = ProjectId::from_string(project_id);
    state
        .review_repo
        .get_pending(&project_id)
        .await
        .map(|reviews| reviews.into_iter().map(ReviewResponse::from).collect())
        .map_err(|e| e.to_string())
}

/// Get a single review by ID
#[tauri::command]
pub async fn get_review_by_id(
    review_id: String,
    state: State<'_, AppState>,
) -> Result<Option<ReviewResponse>, String> {
    let review_id = ReviewId::from_string(review_id);
    state
        .review_repo
        .get_by_id(&review_id)
        .await
        .map(|opt| opt.map(ReviewResponse::from))
        .map_err(|e| e.to_string())
}

/// Get all reviews for a task
#[tauri::command]
pub async fn get_reviews_by_task_id(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ReviewResponse>, String> {
    let task_id = TaskId::from_string(task_id);
    state
        .review_repo
        .get_by_task_id(&task_id)
        .await
        .map(|reviews| reviews.into_iter().map(ReviewResponse::from).collect())
        .map_err(|e| e.to_string())
}

/// Get the state history (review notes) for a task
/// Parses embedded issues JSON from notes if present
#[tauri::command]
pub async fn get_task_state_history(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ReviewNoteResponse>, String> {
    let task_id = TaskId::from_string(task_id);
    state
        .review_repo
        .get_notes_by_task_id(&task_id)
        .await
        .map(|notes| notes.into_iter().map(ReviewNoteResponse::from).collect())
        .map_err(|e| e.to_string())
}

// ============================================================================
// Commands - Write operations (human review actions)
// ============================================================================

/// Approve a review
#[tauri::command]
pub async fn approve_review(
    input: ApproveReviewInput,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let review_id = ReviewId::from_string(input.review_id);

    // Get the review
    let mut review = state
        .review_repo
        .get_by_id(&review_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Review not found: {}", review_id.as_str()))?;

    // Check if pending
    if !review.is_pending() {
        return Err(format!(
            "Review {} is not pending (current: {})",
            review_id.as_str(),
            review.status
        ));
    }

    // Approve the review
    review.approve(input.notes);
    state
        .review_repo
        .update(&review)
        .await
        .map_err(|e| e.to_string())
}

/// Request changes on a review
#[tauri::command]
pub async fn request_changes(
    input: RequestChangesInput,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let review_id = ReviewId::from_string(input.review_id);

    // Get the review
    let mut review = state
        .review_repo
        .get_by_id(&review_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Review not found: {}", review_id.as_str()))?;

    // Check if pending
    if !review.is_pending() {
        return Err(format!(
            "Review {} is not pending (current: {})",
            review_id.as_str(),
            review.status
        ));
    }

    // Request changes
    review.request_changes(input.notes);
    state
        .review_repo
        .update(&review)
        .await
        .map_err(|e| e.to_string())?;

    // Return fix_description if provided (caller can use it to create fix task)
    Ok(input.fix_description)
}

/// Reject a review
#[tauri::command]
pub async fn reject_review(
    input: RejectReviewInput,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let review_id = ReviewId::from_string(input.review_id);

    // Get the review
    let mut review = state
        .review_repo
        .get_by_id(&review_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Review not found: {}", review_id.as_str()))?;

    // Check if pending
    if !review.is_pending() {
        return Err(format!(
            "Review {} is not pending (current: {})",
            review_id.as_str(),
            review.status
        ));
    }

    // Reject the review
    review.reject(input.notes);
    state
        .review_repo
        .update(&review)
        .await
        .map_err(|e| e.to_string())
}

// ============================================================================
// Commands - Fix task operations
// ============================================================================

use crate::domain::entities::{InternalStatus, Task};
use crate::domain::review::config::ReviewSettings;

/// Approve a fix task, changing its status from Blocked to Ready
#[tauri::command]
pub async fn approve_fix_task(
    input: ApproveFixTaskInput,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use crate::commands::emit_queue_changed;

    let fix_task_id = TaskId::from_string(input.fix_task_id);

    // Get the fix task
    let mut fix_task = state
        .task_repo
        .get_by_id(&fix_task_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Fix task not found: {}", fix_task_id.as_str()))?;

    // Verify it's in Blocked status
    if fix_task.internal_status != InternalStatus::Blocked {
        return Err(format!(
            "Fix task {} is not in Blocked status (current: {:?})",
            fix_task_id.as_str(),
            fix_task.internal_status
        ));
    }

    let project_id = fix_task.project_id.clone();

    // Change to Ready status
    fix_task.internal_status = InternalStatus::Ready;
    state
        .task_repo
        .update(&fix_task)
        .await
        .map_err(|e| e.to_string())?;

    // Emit queue_changed since we're transitioning a task to Ready status
    emit_queue_changed(&state, &project_id, &app).await;

    Ok(())
}

/// Reject a fix task with feedback, optionally creating a new fix proposal
/// Returns the new fix task ID if one was created, None if max attempts reached
#[tauri::command]
pub async fn reject_fix_task(
    input: RejectFixTaskInput,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Option<String>, String> {
    use crate::commands::emit_queue_changed;

    let fix_task_id = TaskId::from_string(input.fix_task_id);
    let original_task_id = TaskId::from_string(input.original_task_id);
    let settings = ReviewSettings::default();

    // Get and update fix task to Failed
    let mut fix_task = state
        .task_repo
        .get_by_id(&fix_task_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Fix task not found: {}", fix_task_id.as_str()))?;

    fix_task.internal_status = InternalStatus::Failed;
    state.task_repo.update(&fix_task).await.map_err(|e| e.to_string())?;

    // Get original task
    let original_task = state
        .task_repo
        .get_by_id(&original_task_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Original task not found: {}", original_task_id.as_str()))?;

    let project_id = original_task.project_id.clone();

    // Count fix attempts for original task
    let attempt_count = state
        .review_repo
        .count_fix_actions(&original_task_id)
        .await
        .map_err(|e| e.to_string())?;

    // Check if max attempts exceeded
    if settings.exceeded_max_attempts(attempt_count) {
        // Move original task to backlog
        let mut original = original_task;
        original.internal_status = InternalStatus::Backlog;
        state.task_repo.update(&original).await.map_err(|e| e.to_string())?;

        // Add review note about max attempts
        let note = ReviewNote::with_notes(
            original_task_id.clone(),
            ReviewerType::Ai,
            ReviewOutcome::Rejected,
            format!(
                "Max fix attempts ({}) reached. Task moved to backlog. Last feedback: {}",
                settings.max_fix_attempts, input.feedback
            ),
        );
        state.review_repo.add_note(&note).await.map_err(|e| e.to_string())?;

        return Ok(None);
    }

    // Create new fix task with feedback
    let new_fix_description = format!(
        "Previous fix rejected. Feedback: {}\n\nOriginal issue: {}",
        input.feedback,
        fix_task.description.as_deref().unwrap_or("No description")
    );

    let mut new_fix_task = Task::new_with_category(
        project_id.clone(),
        format!("Fix: {}", original_task.title),
        "fix".to_string(),
    );
    new_fix_task.set_description(Some(new_fix_description));
    new_fix_task.set_priority(original_task.priority + 1);

    let should_emit_queue_changed = if settings.needs_fix_approval() {
        new_fix_task.internal_status = InternalStatus::Blocked;
        false
    } else {
        new_fix_task.internal_status = InternalStatus::Ready;
        true
    };

    let created = state
        .task_repo
        .create(new_fix_task)
        .await
        .map_err(|e| e.to_string())?;

    // Emit queue_changed if the new fix task is in Ready status
    if should_emit_queue_changed {
        emit_queue_changed(&state, &project_id, &app).await;
    }

    Ok(Some(created.id.as_str().to_string()))
}

/// Get the number of fix attempts for a task
#[tauri::command]
pub async fn get_fix_task_attempts(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<FixTaskAttemptsResponse, String> {
    let task_id = TaskId::from_string(task_id.clone());

    let attempt_count = state
        .review_repo
        .count_fix_actions(&task_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(FixTaskAttemptsResponse {
        task_id: task_id.as_str().to_string(),
        attempt_count,
    })
}

// ============================================================================
// Commands - Task-based approval (for human review after AI review)
// ============================================================================

use super::review_commands_types::{ApproveTaskInput, RequestTaskChangesInput};
use crate::application::{TaskSchedulerService, TaskTransitionService};
use crate::commands::execution_commands::ExecutionState;
use crate::domain::state_machine::services::TaskScheduler;
use std::sync::Arc;

/// Approve a task after AI review has passed or escalated
/// This is the human approval action that transitions the task to Approved status
#[tauri::command]
pub async fn approve_task_for_review(
    input: ApproveTaskInput,
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let task_id = TaskId::from_string(input.task_id);

    // 1. Get task and validate state is ReviewPassed or Escalated
    let task = state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Task not found: {}", task_id.as_str()))?;

    if task.internal_status != InternalStatus::ReviewPassed
        && task.internal_status != InternalStatus::Escalated
    {
        return Err(format!(
            "Task must be in 'review_passed' or 'escalated' status to approve. Current status: {}. \
            This action is only available after the AI reviewer has approved or escalated the task.",
            task.internal_status.as_str()
        ));
    }

    // 2. Create a human approval review note
    let review_note = ReviewNote::with_notes(
        task_id.clone(),
        ReviewerType::Human,
        ReviewOutcome::Approved,
        input.notes.unwrap_or_else(|| "Approved by user".to_string()),
    );
    state
        .review_repo
        .add_note(&review_note)
        .await
        .map_err(|e| e.to_string())?;

    // 3. Create scheduler for post-merge scheduling (Approved → PendingMerge path)
    let scheduler_concrete = Arc::new(TaskSchedulerService::new(
        Arc::clone(&execution_state),
        Arc::clone(&state.project_repo),
        Arc::clone(&state.task_repo),
        Arc::clone(&state.task_dependency_repo),
        Arc::clone(&state.chat_message_repo),
        Arc::clone(&state.chat_conversation_repo),
        Arc::clone(&state.agent_run_repo),
        Arc::clone(&state.ideation_session_repo),
        Arc::clone(&state.activity_event_repo),
        Arc::clone(&state.message_queue),
        Arc::clone(&state.running_agent_registry),
        Some(app.clone()),
    )
    .with_plan_branch_repo(Arc::clone(&state.plan_branch_repo)));
    scheduler_concrete.set_self_ref(Arc::clone(&scheduler_concrete) as Arc<dyn TaskScheduler>);
    let task_scheduler: Arc<dyn TaskScheduler> = scheduler_concrete;

    // 4. Transition to Approved using TaskTransitionService
    let transition_service = TaskTransitionService::new(
        Arc::clone(&state.task_repo),
        Arc::clone(&state.task_dependency_repo),
        Arc::clone(&state.project_repo),
        Arc::clone(&state.chat_message_repo),
        Arc::clone(&state.chat_conversation_repo),
        Arc::clone(&state.agent_run_repo),
        Arc::clone(&state.ideation_session_repo),
        Arc::clone(&state.activity_event_repo),
        Arc::clone(&state.message_queue),
        Arc::clone(&state.running_agent_registry),
        Arc::clone(&execution_state),
        Some(app.clone()),
    )
    .with_task_scheduler(task_scheduler)
    .with_plan_branch_repo(Arc::clone(&state.plan_branch_repo));

    let old_status = task.internal_status.as_str().to_string();
    transition_service
        .transition_task(&task_id, InternalStatus::Approved)
        .await
        .map_err(|e| e.to_string())?;

    // 5. Emit events
    let _ = app.emit("review:human_approved", serde_json::json!({
        "task_id": task_id.as_str(),
    }));
    let _ = app.emit("task:status_changed", serde_json::json!({
        "task_id": task_id.as_str(),
        "old_status": old_status,
        "new_status": "approved",
    }));

    Ok(())
}

/// Request changes on a task after AI review has passed or escalated
/// This transitions the task to RevisionNeeded status for re-execution
#[tauri::command]
pub async fn request_task_changes_for_review(
    input: RequestTaskChangesInput,
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let task_id = TaskId::from_string(input.task_id);

    // 1. Get task and validate state is ReviewPassed or Escalated
    let task = state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Task not found: {}", task_id.as_str()))?;

    if task.internal_status != InternalStatus::ReviewPassed
        && task.internal_status != InternalStatus::Escalated
    {
        return Err(format!(
            "Task must be in 'review_passed' or 'escalated' status to request changes. Current status: {}. \
            This action is only available after the AI reviewer has approved or escalated the task.",
            task.internal_status.as_str()
        ));
    }

    // 2. Create a human changes-requested review note
    let review_note = ReviewNote::with_notes(
        task_id.clone(),
        ReviewerType::Human,
        ReviewOutcome::ChangesRequested,
        input.feedback.clone(),
    );
    state
        .review_repo
        .add_note(&review_note)
        .await
        .map_err(|e| e.to_string())?;

    // 3. Transition to RevisionNeeded (will auto-trigger re-execution)
    let transition_service = TaskTransitionService::new(
        Arc::clone(&state.task_repo),
        Arc::clone(&state.task_dependency_repo),
        Arc::clone(&state.project_repo),
        Arc::clone(&state.chat_message_repo),
        Arc::clone(&state.chat_conversation_repo),
        Arc::clone(&state.agent_run_repo),
        Arc::clone(&state.ideation_session_repo),
        Arc::clone(&state.activity_event_repo),
        Arc::clone(&state.message_queue),
        Arc::clone(&state.running_agent_registry),
        Arc::clone(&execution_state),
        Some(app.clone()),
    )
    .with_plan_branch_repo(Arc::clone(&state.plan_branch_repo));

    let old_status = task.internal_status.as_str().to_string();
    transition_service
        .transition_task(&task_id, InternalStatus::RevisionNeeded)
        .await
        .map_err(|e| e.to_string())?;

    // 4. Emit events
    let _ = app.emit("review:human_changes_requested", serde_json::json!({
        "task_id": task_id.as_str(),
        "feedback": input.feedback,
    }));
    let _ = app.emit("task:status_changed", serde_json::json!({
        "task_id": task_id.as_str(),
        "old_status": old_status,
        "new_status": "revision_needed",
    }));

    Ok(())
}

// ============================================================================
// Commands - Review Issue operations
// ============================================================================

use crate::domain::entities::ReviewIssueId;
use crate::domain::entities::ReviewNoteId;

/// Get issues for a task, optionally filtered by status
/// status_filter: "open" returns only open issues, "all" or None returns all issues
#[tauri::command]
pub async fn get_task_issues(
    task_id: String,
    status_filter: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<ReviewIssueResponse>, String> {
    let task_id = TaskId::from_string(task_id);

    let issues = match status_filter.as_deref() {
        Some("open") => {
            state
                .review_issue_repo
                .get_open_by_task_id(&task_id)
                .await
                .map_err(|e| e.to_string())?
        }
        _ => {
            state
                .review_issue_repo
                .get_by_task_id(&task_id)
                .await
                .map_err(|e| e.to_string())?
        }
    };

    Ok(issues.into_iter().map(ReviewIssueResponse::from).collect())
}

/// Get issue progress summary for a task
#[tauri::command]
pub async fn get_issue_progress(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<IssueProgressResponse, String> {
    let task_id = TaskId::from_string(task_id);

    let summary = state
        .review_issue_repo
        .get_summary(&task_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(IssueProgressResponse::from(summary))
}

/// Verify that an issue has been fixed (Addressed -> Verified)
/// Called by the reviewer during subsequent review
#[tauri::command]
pub async fn verify_issue(
    input: VerifyIssueInput,
    state: State<'_, AppState>,
) -> Result<ReviewIssueResponse, String> {
    let issue_id = ReviewIssueId::from_string(input.issue_id);
    let review_note_id = ReviewNoteId::from_string(input.review_note_id);

    let mut issue = state
        .review_issue_repo
        .get_by_id(&issue_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Issue not found: {}", issue_id.as_str()))?;

    if issue.status != crate::domain::entities::IssueStatus::Addressed {
        return Err(format!(
            "Cannot verify issue: current status is {} (expected addressed)",
            issue.status
        ));
    }

    issue.verify(review_note_id);
    state
        .review_issue_repo
        .update(&issue)
        .await
        .map_err(|e| e.to_string())?;

    Ok(ReviewIssueResponse::from(issue))
}

/// Reopen an issue that was not actually fixed (Addressed -> Open)
/// Called by the reviewer during subsequent review when fix is insufficient
#[tauri::command]
pub async fn reopen_issue(
    input: ReopenIssueInput,
    state: State<'_, AppState>,
) -> Result<ReviewIssueResponse, String> {
    let issue_id = ReviewIssueId::from_string(input.issue_id);

    let mut issue = state
        .review_issue_repo
        .get_by_id(&issue_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Issue not found: {}", issue_id.as_str()))?;

    if issue.status != crate::domain::entities::IssueStatus::Addressed {
        return Err(format!(
            "Cannot reopen issue: current status is {} (expected addressed)",
            issue.status
        ));
    }

    issue.reopen(input.reason);
    state
        .review_issue_repo
        .update(&issue)
        .await
        .map_err(|e| e.to_string())?;

    Ok(ReviewIssueResponse::from(issue))
}

// ============================================================================
// Commands - Execute Agent Issue Tracking
// ============================================================================

/// Mark an issue as being worked on (Open -> InProgress)
/// Called by the worker/execute agent when starting to work on an issue
#[tauri::command]
pub async fn mark_issue_in_progress(
    input: MarkIssueInProgressInput,
    state: State<'_, AppState>,
) -> Result<ReviewIssueResponse, String> {
    let issue_id = ReviewIssueId::from_string(input.issue_id);

    let mut issue = state
        .review_issue_repo
        .get_by_id(&issue_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Issue not found: {}", issue_id.as_str()))?;

    if issue.status != crate::domain::entities::IssueStatus::Open {
        return Err(format!(
            "Cannot mark issue as in_progress: current status is {} (expected open)",
            issue.status
        ));
    }

    issue.start_work();
    state
        .review_issue_repo
        .update(&issue)
        .await
        .map_err(|e| e.to_string())?;

    Ok(ReviewIssueResponse::from(issue))
}

/// Mark an issue as addressed (Open/InProgress -> Addressed)
/// Called by the worker/execute agent after completing work on an issue
#[tauri::command]
pub async fn mark_issue_addressed(
    input: MarkIssueAddressedInput,
    state: State<'_, AppState>,
) -> Result<ReviewIssueResponse, String> {
    let issue_id = ReviewIssueId::from_string(input.issue_id);

    let mut issue = state
        .review_issue_repo
        .get_by_id(&issue_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Issue not found: {}", issue_id.as_str()))?;

    if !issue.needs_work() {
        return Err(format!(
            "Cannot mark issue as addressed: current status is {} (expected open or in_progress)",
            issue.status
        ));
    }

    issue.mark_addressed(Some(input.resolution_notes), input.attempt_number);
    state
        .review_issue_repo
        .update(&issue)
        .await
        .map_err(|e| e.to_string())?;

    Ok(ReviewIssueResponse::from(issue))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{Review, ReviewStatus, ReviewerType};

    async fn setup_test_state() -> AppState {
        AppState::new_test()
    }

    #[tokio::test]
    async fn test_get_pending_reviews_empty() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("proj-1".to_string());

        let reviews = state.review_repo.get_pending(&project_id).await.expect("Failed to get pending reviews in test");
        assert!(reviews.is_empty());
    }

    #[tokio::test]
    async fn test_get_pending_reviews_returns_pending() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());

        // Create a pending review
        let review = Review::new(project_id.clone(), task_id, ReviewerType::Ai);
        state.review_repo.create(&review).await.expect("Failed to create review in test");

        let reviews = state.review_repo.get_pending(&project_id).await.expect("Failed to get pending reviews in test");
        assert_eq!(reviews.len(), 1);
        assert!(reviews[0].is_pending());
    }

    #[tokio::test]
    async fn test_get_review_by_id() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());

        let review = Review::new(project_id, task_id, ReviewerType::Human);
        let review_id = review.id.clone();
        state.review_repo.create(&review).await.expect("Failed to create review in test");

        let retrieved = state.review_repo.get_by_id(&review_id).await.expect("Failed to get review by id in test");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.expect("Expected to find review").id, review_id);
    }

    #[tokio::test]
    async fn test_get_review_by_id_not_found() {
        let state = setup_test_state().await;
        let nonexistent = ReviewId::from_string("nonexistent");

        let retrieved = state.review_repo.get_by_id(&nonexistent).await.expect("Failed to get review by id in test");
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_get_reviews_by_task_id() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());

        // Create two reviews for same task
        let review1 = Review::new(project_id.clone(), task_id.clone(), ReviewerType::Ai);
        let review2 = Review::new(project_id, task_id.clone(), ReviewerType::Human);
        state.review_repo.create(&review1).await.expect("Failed to create review1 in test");
        state.review_repo.create(&review2).await.expect("Failed to create review2 in test");

        let reviews = state.review_repo.get_by_task_id(&task_id).await.expect("Failed to get reviews by task id in test");
        assert_eq!(reviews.len(), 2);
    }

    #[tokio::test]
    async fn test_approve_review() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());

        let review = Review::new(project_id, task_id, ReviewerType::Ai);
        let review_id = review.id.clone();
        state.review_repo.create(&review).await.expect("Failed to create review in test");

        // Approve via repository (simulating what the command does)
        let mut review = state.review_repo.get_by_id(&review_id).await.expect("Failed to get review by id in test").expect("Expected to find review");
        review.approve(Some("Looks good!".to_string()));
        state.review_repo.update(&review).await.expect("Failed to update review in test");

        // Verify
        let updated = state.review_repo.get_by_id(&review_id).await.expect("Failed to get review by id in test").expect("Expected to find updated review");
        assert!(updated.is_approved());
        assert_eq!(updated.notes, Some("Looks good!".to_string()));
    }

    #[tokio::test]
    async fn test_request_changes() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());

        let review = Review::new(project_id, task_id, ReviewerType::Human);
        let review_id = review.id.clone();
        state.review_repo.create(&review).await.expect("Failed to create review in test");

        // Request changes via repository
        let mut review = state.review_repo.get_by_id(&review_id).await.expect("Failed to get review by id in test").expect("Expected to find review");
        review.request_changes("Missing tests".to_string());
        state.review_repo.update(&review).await.expect("Failed to update review in test");

        // Verify
        let updated = state.review_repo.get_by_id(&review_id).await.expect("Failed to get review by id in test").expect("Expected to find updated review");
        assert_eq!(updated.status, ReviewStatus::ChangesRequested);
        assert_eq!(updated.notes, Some("Missing tests".to_string()));
    }

    #[tokio::test]
    async fn test_reject_review() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());

        let review = Review::new(project_id, task_id, ReviewerType::Ai);
        let review_id = review.id.clone();
        state.review_repo.create(&review).await.expect("Failed to create review in test");

        // Reject via repository
        let mut review = state.review_repo.get_by_id(&review_id).await.expect("Failed to get review by id in test").expect("Expected to find review");
        review.reject("Fundamentally wrong".to_string());
        state.review_repo.update(&review).await.expect("Failed to update review in test");

        // Verify
        let updated = state.review_repo.get_by_id(&review_id).await.expect("Failed to get review by id in test").expect("Expected to find updated review");
        assert_eq!(updated.status, ReviewStatus::Rejected);
        assert_eq!(updated.notes, Some("Fundamentally wrong".to_string()));
    }

    #[tokio::test]
    async fn test_review_response_conversion() {
        let project_id = ProjectId::from_string("proj-123".to_string());
        let task_id = TaskId::from_string("task-456".to_string());
        let review = Review::new(project_id, task_id, ReviewerType::Human);
        let response = ReviewResponse::from(review);

        assert!(!response.id.is_empty());
        assert_eq!(response.project_id, "proj-123");
        assert_eq!(response.task_id, "task-456");
        assert_eq!(response.reviewer_type, "human");
        assert_eq!(response.status, "pending");
        assert!(response.notes.is_none());
        assert!(response.completed_at.is_none());

        // Verify it serializes to JSON
        let json = serde_json::to_string(&response).expect("Failed to serialize response to JSON in test");
        assert!(json.contains("\"reviewer_type\":\"human\""));
    }

    // ========================================
    // Fix Task Command Tests
    // ========================================

    use crate::domain::entities::Project;

    async fn create_task_for_tests(state: &AppState, project_id: ProjectId) -> Task {
        // Create a project first (required for task creation)
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        let mut project_with_id = project;
        project_with_id.id = project_id.clone();
        state.project_repo.create(project_with_id).await.expect("Failed to create project in test");

        // Create a task
        let mut task = Task::new(project_id, "Test Task".to_string());
        task.internal_status = InternalStatus::PendingReview;
        state.task_repo.create(task.clone()).await.expect("Failed to create task in test");
        task
    }

    async fn create_blocked_fix_task(state: &AppState, project_id: ProjectId) -> (Task, Task) {
        // Create a project first
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        let mut project_with_id = project;
        project_with_id.id = project_id.clone();
        state.project_repo.create(project_with_id).await.expect("Failed to create project in test");

        // Create original task
        let mut original = Task::new(project_id.clone(), "Original Task".to_string());
        original.internal_status = InternalStatus::RevisionNeeded;
        let original = state.task_repo.create(original).await.expect("Failed to create original task in test");

        // Create fix task (blocked, waiting for approval)
        let mut fix_task = Task::new_with_category(
            project_id,
            "Fix: Original Task".to_string(),
            "fix".to_string(),
        );
        fix_task.internal_status = InternalStatus::Blocked;
        let fix_task = state.task_repo.create(fix_task).await.expect("Failed to create fix task in test");

        (original, fix_task)
    }

    #[tokio::test]
    async fn test_approve_fix_task_success() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("proj-1".to_string());

        // Create original task and blocked fix task
        let (_original, fix_task) = create_blocked_fix_task(&state, project_id).await;

        // Verify fix task is blocked initially
        let task = state.task_repo.get_by_id(&fix_task.id).await.expect("Failed to get task by id in test").expect("Expected to find task");
        assert_eq!(task.internal_status, InternalStatus::Blocked);

        // Approve it directly (simulating what the command does)
        let mut task = state.task_repo.get_by_id(&fix_task.id).await.expect("Failed to get task by id in test").expect("Expected to find task");
        assert_eq!(task.internal_status, InternalStatus::Blocked);
        task.internal_status = InternalStatus::Ready;
        state.task_repo.update(&task).await.expect("Failed to update task in test");

        // Verify it's now Ready
        let updated = state.task_repo.get_by_id(&fix_task.id).await.expect("Failed to get task by id in test").expect("Expected to find updated task");
        assert_eq!(updated.internal_status, InternalStatus::Ready);
    }

    #[tokio::test]
    async fn test_approve_fix_task_not_blocked_fails() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("proj-1".to_string());

        // Create a task that is Ready (not Blocked)
        let task = create_task_for_tests(&state, project_id).await;

        // Set it to Ready
        let mut task = state.task_repo.get_by_id(&task.id).await.expect("Failed to get task by id in test").expect("Expected to find task");
        task.internal_status = InternalStatus::Ready;
        state.task_repo.update(&task).await.expect("Failed to update task in test");

        // Simulating the command logic - should reject non-Blocked tasks
        let task = state.task_repo.get_by_id(&task.id).await.expect("Failed to get task by id in test").expect("Expected to find task");
        assert_ne!(task.internal_status, InternalStatus::Blocked);
        // In the real command, this returns an error
    }

    #[tokio::test]
    async fn test_approve_fix_task_not_found() {
        let state = setup_test_state().await;

        let nonexistent_id = TaskId::from_string("nonexistent".to_string());

        // Task not found
        let result = state.task_repo.get_by_id(&nonexistent_id).await.expect("Failed to get task by id in test");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_reject_fix_task_creates_new_fix() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("proj-1".to_string());

        // Create original task and blocked fix task
        let (original, fix_task) = create_blocked_fix_task(&state, project_id).await;

        // Create a review for the original task (required for fix attempt counting)
        let review = Review::new(
            original.project_id.clone(),
            original.id.clone(),
            ReviewerType::Ai,
        );
        state.review_repo.create(&review).await.expect("Failed to create review in test");

        // Simulate reject_fix_task logic:
        // 1. Mark fix task as Failed
        let mut fix = state.task_repo.get_by_id(&fix_task.id).await.expect("Failed to get task by id in test").expect("Expected to find fix task");
        fix.internal_status = InternalStatus::Failed;
        state.task_repo.update(&fix).await.expect("Failed to update fix task in test");

        // 2. Create new fix task
        let mut new_fix_task = Task::new_with_category(
            original.project_id.clone(),
            format!("Fix: {}", original.title),
            "fix".to_string(),
        );
        new_fix_task.set_description(Some(format!(
            "Previous fix rejected. Feedback: {}\n\nOriginal issue: {}",
            "Not good enough",
            fix.description.as_deref().unwrap_or("No description")
        )));
        new_fix_task.set_priority(original.priority + 1);
        new_fix_task.internal_status = InternalStatus::Ready;
        let created = state.task_repo.create(new_fix_task).await.expect("Failed to create new fix task in test");

        // Verify new fix task was created
        assert!(created.title.starts_with("Fix:"));
        assert!(created.description.as_ref().expect("Expected description to be set").contains("Not good enough"));

        // Original fix task should be Failed
        let old_fix = state.task_repo.get_by_id(&fix_task.id).await.expect("Failed to get task by id in test").expect("Expected to find old fix task");
        assert_eq!(old_fix.internal_status, InternalStatus::Failed);
    }

    #[tokio::test]
    async fn test_get_fix_task_attempts_zero() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("proj-1".to_string());

        // Create a task
        let task = create_task_for_tests(&state, project_id).await;

        // Get fix attempts (should be 0)
        let count = state.review_repo.count_fix_actions(&task.id).await.expect("Failed to count fix actions in test");

        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_fix_task_attempts_response_serialization() {
        let response = FixTaskAttemptsResponse {
            task_id: "task-123".to_string(),
            attempt_count: 2,
        };

        let json = serde_json::to_string(&response).expect("Failed to serialize response to JSON in test");
        assert!(json.contains("\"task_id\":\"task-123\""));
        assert!(json.contains("\"attempt_count\":2"));
    }
}
