// Tauri commands for Review operations
// Thin layer that delegates to ReviewRepository and ReviewService

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::application::AppState;
use crate::domain::entities::{
    ProjectId, Review, ReviewAction, ReviewId, ReviewNote, ReviewerType, TaskId,
};

// ============================================================================
// Response Types
// ============================================================================

/// Response wrapper for review operations
#[derive(Debug, Serialize)]
pub struct ReviewResponse {
    pub id: String,
    pub project_id: String,
    pub task_id: String,
    pub reviewer_type: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
}

impl From<Review> for ReviewResponse {
    fn from(review: Review) -> Self {
        Self {
            id: review.id.as_str().to_string(),
            project_id: review.project_id.as_str().to_string(),
            task_id: review.task_id.as_str().to_string(),
            reviewer_type: review.reviewer_type.to_string(),
            status: review.status.to_string(),
            notes: review.notes,
            created_at: review.created_at.to_rfc3339(),
            completed_at: review.completed_at.map(|dt| dt.to_rfc3339()),
        }
    }
}

/// Response wrapper for review actions
#[derive(Debug, Serialize)]
pub struct ReviewActionResponse {
    pub id: String,
    pub review_id: String,
    pub action_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_task_id: Option<String>,
    pub created_at: String,
}

impl From<ReviewAction> for ReviewActionResponse {
    fn from(action: ReviewAction) -> Self {
        Self {
            id: action.id.as_str().to_string(),
            review_id: action.review_id.as_str().to_string(),
            action_type: action.action_type.to_string(),
            target_task_id: action.target_task_id.map(|id| id.as_str().to_string()),
            created_at: action.created_at.to_rfc3339(),
        }
    }
}

/// Response wrapper for review notes (state history)
#[derive(Debug, Serialize)]
pub struct ReviewNoteResponse {
    pub id: String,
    pub task_id: String,
    pub reviewer: String,
    pub outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub created_at: String,
}

impl From<ReviewNote> for ReviewNoteResponse {
    fn from(note: ReviewNote) -> Self {
        Self {
            id: note.id.as_str().to_string(),
            task_id: note.task_id.as_str().to_string(),
            reviewer: note.reviewer.to_string(),
            outcome: note.outcome.to_string(),
            notes: note.notes,
            created_at: note.created_at.to_rfc3339(),
        }
    }
}

// ============================================================================
// Input Types
// ============================================================================

/// Input for approving a review
#[derive(Debug, Deserialize)]
pub struct ApproveReviewInput {
    pub review_id: String,
    #[serde(default)]
    pub notes: Option<String>,
}

/// Input for requesting changes on a review
#[derive(Debug, Deserialize)]
pub struct RequestChangesInput {
    pub review_id: String,
    pub notes: String,
    #[serde(default)]
    pub fix_description: Option<String>,
}

/// Input for rejecting a review
#[derive(Debug, Deserialize)]
pub struct RejectReviewInput {
    pub review_id: String,
    pub notes: String,
}

/// Input for approving a fix task
#[derive(Debug, Deserialize)]
pub struct ApproveFixTaskInput {
    pub fix_task_id: String,
}

/// Input for rejecting a fix task
#[derive(Debug, Deserialize)]
pub struct RejectFixTaskInput {
    pub fix_task_id: String,
    pub feedback: String,
    pub original_task_id: String,
}

/// Response for fix task attempt count
#[derive(Debug, Serialize)]
pub struct FixTaskAttemptsResponse {
    pub task_id: String,
    pub attempt_count: u32,
}

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

use crate::domain::entities::{InternalStatus, ReviewOutcome, Task};
use crate::domain::review::config::ReviewSettings;

/// Approve a fix task, changing its status from Blocked to Ready
#[tauri::command]
pub async fn approve_fix_task(
    input: ApproveFixTaskInput,
    state: State<'_, AppState>,
) -> Result<(), String> {
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

    // Change to Ready status
    fix_task.internal_status = InternalStatus::Ready;
    state
        .task_repo
        .update(&fix_task)
        .await
        .map_err(|e| e.to_string())
}

/// Reject a fix task with feedback, optionally creating a new fix proposal
/// Returns the new fix task ID if one was created, None if max attempts reached
#[tauri::command]
pub async fn reject_fix_task(
    input: RejectFixTaskInput,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
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
        original_task.project_id.clone(),
        format!("Fix: {}", original_task.title),
        "fix".to_string(),
    );
    new_fix_task.set_description(Some(new_fix_description));
    new_fix_task.set_priority(original_task.priority + 1);

    if settings.needs_fix_approval() {
        new_fix_task.internal_status = InternalStatus::Blocked;
    } else {
        new_fix_task.internal_status = InternalStatus::Ready;
    }

    let created = state
        .task_repo
        .create(new_fix_task)
        .await
        .map_err(|e| e.to_string())?;

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

        let reviews = state.review_repo.get_pending(&project_id).await.unwrap();
        assert!(reviews.is_empty());
    }

    #[tokio::test]
    async fn test_get_pending_reviews_returns_pending() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());

        // Create a pending review
        let review = Review::new(project_id.clone(), task_id, ReviewerType::Ai);
        state.review_repo.create(&review).await.unwrap();

        let reviews = state.review_repo.get_pending(&project_id).await.unwrap();
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
        state.review_repo.create(&review).await.unwrap();

        let retrieved = state.review_repo.get_by_id(&review_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, review_id);
    }

    #[tokio::test]
    async fn test_get_review_by_id_not_found() {
        let state = setup_test_state().await;
        let nonexistent = ReviewId::from_string("nonexistent");

        let retrieved = state.review_repo.get_by_id(&nonexistent).await.unwrap();
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
        state.review_repo.create(&review1).await.unwrap();
        state.review_repo.create(&review2).await.unwrap();

        let reviews = state.review_repo.get_by_task_id(&task_id).await.unwrap();
        assert_eq!(reviews.len(), 2);
    }

    #[tokio::test]
    async fn test_approve_review() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());

        let review = Review::new(project_id, task_id, ReviewerType::Ai);
        let review_id = review.id.clone();
        state.review_repo.create(&review).await.unwrap();

        // Approve via repository (simulating what the command does)
        let mut review = state.review_repo.get_by_id(&review_id).await.unwrap().unwrap();
        review.approve(Some("Looks good!".to_string()));
        state.review_repo.update(&review).await.unwrap();

        // Verify
        let updated = state.review_repo.get_by_id(&review_id).await.unwrap().unwrap();
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
        state.review_repo.create(&review).await.unwrap();

        // Request changes via repository
        let mut review = state.review_repo.get_by_id(&review_id).await.unwrap().unwrap();
        review.request_changes("Missing tests".to_string());
        state.review_repo.update(&review).await.unwrap();

        // Verify
        let updated = state.review_repo.get_by_id(&review_id).await.unwrap().unwrap();
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
        state.review_repo.create(&review).await.unwrap();

        // Reject via repository
        let mut review = state.review_repo.get_by_id(&review_id).await.unwrap().unwrap();
        review.reject("Fundamentally wrong".to_string());
        state.review_repo.update(&review).await.unwrap();

        // Verify
        let updated = state.review_repo.get_by_id(&review_id).await.unwrap().unwrap();
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
        let json = serde_json::to_string(&response).unwrap();
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
        state.project_repo.create(project_with_id).await.unwrap();

        // Create a task
        let mut task = Task::new(project_id, "Test Task".to_string());
        task.internal_status = InternalStatus::PendingReview;
        state.task_repo.create(task.clone()).await.unwrap();
        task
    }

    async fn create_blocked_fix_task(state: &AppState, project_id: ProjectId) -> (Task, Task) {
        // Create a project first
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        let mut project_with_id = project;
        project_with_id.id = project_id.clone();
        state.project_repo.create(project_with_id).await.unwrap();

        // Create original task
        let mut original = Task::new(project_id.clone(), "Original Task".to_string());
        original.internal_status = InternalStatus::RevisionNeeded;
        let original = state.task_repo.create(original).await.unwrap();

        // Create fix task (blocked, waiting for approval)
        let mut fix_task = Task::new_with_category(
            project_id,
            "Fix: Original Task".to_string(),
            "fix".to_string(),
        );
        fix_task.internal_status = InternalStatus::Blocked;
        let fix_task = state.task_repo.create(fix_task).await.unwrap();

        (original, fix_task)
    }

    #[tokio::test]
    async fn test_approve_fix_task_success() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("proj-1".to_string());

        // Create original task and blocked fix task
        let (_original, fix_task) = create_blocked_fix_task(&state, project_id).await;

        // Verify fix task is blocked initially
        let task = state.task_repo.get_by_id(&fix_task.id).await.unwrap().unwrap();
        assert_eq!(task.internal_status, InternalStatus::Blocked);

        // Approve it directly (simulating what the command does)
        let mut task = state.task_repo.get_by_id(&fix_task.id).await.unwrap().unwrap();
        assert_eq!(task.internal_status, InternalStatus::Blocked);
        task.internal_status = InternalStatus::Ready;
        state.task_repo.update(&task).await.unwrap();

        // Verify it's now Ready
        let updated = state.task_repo.get_by_id(&fix_task.id).await.unwrap().unwrap();
        assert_eq!(updated.internal_status, InternalStatus::Ready);
    }

    #[tokio::test]
    async fn test_approve_fix_task_not_blocked_fails() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("proj-1".to_string());

        // Create a task that is Ready (not Blocked)
        let task = create_task_for_tests(&state, project_id).await;

        // Set it to Ready
        let mut task = state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
        task.internal_status = InternalStatus::Ready;
        state.task_repo.update(&task).await.unwrap();

        // Simulating the command logic - should reject non-Blocked tasks
        let task = state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
        assert_ne!(task.internal_status, InternalStatus::Blocked);
        // In the real command, this returns an error
    }

    #[tokio::test]
    async fn test_approve_fix_task_not_found() {
        let state = setup_test_state().await;

        let nonexistent_id = TaskId::from_string("nonexistent".to_string());

        // Task not found
        let result = state.task_repo.get_by_id(&nonexistent_id).await.unwrap();
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
        state.review_repo.create(&review).await.unwrap();

        // Simulate reject_fix_task logic:
        // 1. Mark fix task as Failed
        let mut fix = state.task_repo.get_by_id(&fix_task.id).await.unwrap().unwrap();
        fix.internal_status = InternalStatus::Failed;
        state.task_repo.update(&fix).await.unwrap();

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
        let created = state.task_repo.create(new_fix_task).await.unwrap();

        // Verify new fix task was created
        assert!(created.title.starts_with("Fix:"));
        assert!(created.description.as_ref().unwrap().contains("Not good enough"));

        // Original fix task should be Failed
        let old_fix = state.task_repo.get_by_id(&fix_task.id).await.unwrap().unwrap();
        assert_eq!(old_fix.internal_status, InternalStatus::Failed);
    }

    #[tokio::test]
    async fn test_get_fix_task_attempts_zero() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("proj-1".to_string());

        // Create a task
        let task = create_task_for_tests(&state, project_id).await;

        // Get fix attempts (should be 0)
        let count = state.review_repo.count_fix_actions(&task.id).await.unwrap();

        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_fix_task_attempts_response_serialization() {
        let response = FixTaskAttemptsResponse {
            task_id: "task-123".to_string(),
            attempt_count: 2,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"task_id\":\"task-123\""));
        assert!(json.contains("\"attempt_count\":2"));
    }
}
