// Tauri commands for Review operations
// Thin layer that delegates to ReviewRepository and ReviewService

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::application::AppState;
use crate::domain::entities::{
    ProjectId, Review, ReviewAction, ReviewId, ReviewNote, TaskId,
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
}
