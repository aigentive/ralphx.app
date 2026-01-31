// Types for Review commands
// Extracted from review_commands.rs to reduce file size

use serde::{Deserialize, Serialize};

use crate::domain::entities::{Review, ReviewAction, ReviewNote};

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

/// Issue reported during review
#[derive(Debug, Clone, Serialize)]
pub struct ReviewIssue {
    pub severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<i32>,
    pub description: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issues: Option<Vec<ReviewIssue>>,
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
            issues: None, // Will be populated by parse_issues_from_notes in Task 3
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
// Task-based Review Input Types
// ============================================================================

/// Input for approving a task after AI review has passed or escalated
#[derive(Debug, Deserialize)]
pub struct ApproveTaskInput {
    pub task_id: String,
    #[serde(default)]
    pub notes: Option<String>,
}

/// Input for requesting changes on a task after AI review has passed or escalated
#[derive(Debug, Deserialize)]
pub struct RequestTaskChangesInput {
    pub task_id: String,
    pub feedback: String,
}
