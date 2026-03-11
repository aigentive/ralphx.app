// Review domain entities for AI and human code review
// Includes Review, ReviewAction, and associated enums

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use super::{ProjectId, TaskId};

// ========================================
// Newtype IDs for type safety
// ========================================

/// A unique identifier for a Review
/// Uses newtype pattern to prevent mixing up with other ID types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReviewId(pub String);

impl ReviewId {
    /// Creates a new ReviewId with a random UUID v4
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Creates a ReviewId from an existing string
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ReviewId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ReviewId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A unique identifier for a ReviewAction
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReviewActionId(pub String);

impl ReviewActionId {
    /// Creates a new ReviewActionId with a random UUID v4
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Creates a ReviewActionId from an existing string
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ReviewActionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ReviewActionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ========================================
// Enums
// ========================================

/// Who performed the review
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewerType {
    /// AI reviewer agent
    Ai,
    /// Human reviewer
    Human,
    /// System-initiated action (reconciler, policy limits, agent crash)
    System,
}

impl std::fmt::Display for ReviewerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReviewerType::Ai => write!(f, "ai"),
            ReviewerType::Human => write!(f, "human"),
            ReviewerType::System => write!(f, "system"),
        }
    }
}

impl FromStr for ReviewerType {
    type Err = ParseReviewerTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ai" => Ok(ReviewerType::Ai),
            "human" => Ok(ReviewerType::Human),
            "system" => Ok(ReviewerType::System),
            _ => Err(ParseReviewerTypeError(s.to_string())),
        }
    }
}

/// Error when parsing an invalid reviewer type string
#[derive(Debug, Clone, PartialEq)]
pub struct ParseReviewerTypeError(pub String);

impl std::fmt::Display for ParseReviewerTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid reviewer type: {}", self.0)
    }
}

impl std::error::Error for ParseReviewerTypeError {}

/// Status of a review
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    /// Review is pending
    Pending,
    /// Review passed, work approved
    Approved,
    /// Reviewer requested changes
    ChangesRequested,
    /// Review rejected the work
    Rejected,
}

impl std::fmt::Display for ReviewStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReviewStatus::Pending => write!(f, "pending"),
            ReviewStatus::Approved => write!(f, "approved"),
            ReviewStatus::ChangesRequested => write!(f, "changes_requested"),
            ReviewStatus::Rejected => write!(f, "rejected"),
        }
    }
}

impl FromStr for ReviewStatus {
    type Err = ParseReviewStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(ReviewStatus::Pending),
            "approved" => Ok(ReviewStatus::Approved),
            "changes_requested" => Ok(ReviewStatus::ChangesRequested),
            "rejected" => Ok(ReviewStatus::Rejected),
            _ => Err(ParseReviewStatusError(s.to_string())),
        }
    }
}

/// Error when parsing an invalid review status string
#[derive(Debug, Clone, PartialEq)]
pub struct ParseReviewStatusError(pub String);

impl std::fmt::Display for ParseReviewStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid review status: {}", self.0)
    }
}

impl std::error::Error for ParseReviewStatusError {}

/// Type of action taken during review
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewActionType {
    /// Fix task was created for issues found
    CreatedFixTask,
    /// Task was moved to backlog (gave up fixing)
    MovedToBacklog,
    /// Review approved the work
    Approved,
}

impl std::fmt::Display for ReviewActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReviewActionType::CreatedFixTask => write!(f, "created_fix_task"),
            ReviewActionType::MovedToBacklog => write!(f, "moved_to_backlog"),
            ReviewActionType::Approved => write!(f, "approved"),
        }
    }
}

impl FromStr for ReviewActionType {
    type Err = ParseReviewActionTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "created_fix_task" => Ok(ReviewActionType::CreatedFixTask),
            "moved_to_backlog" => Ok(ReviewActionType::MovedToBacklog),
            "approved" => Ok(ReviewActionType::Approved),
            _ => Err(ParseReviewActionTypeError(s.to_string())),
        }
    }
}

/// Error when parsing an invalid review action type string
#[derive(Debug, Clone, PartialEq)]
pub struct ParseReviewActionTypeError(pub String);

impl std::fmt::Display for ParseReviewActionTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid review action type: {}", self.0)
    }
}

impl std::error::Error for ParseReviewActionTypeError {}

// ========================================
// Review Entity
// ========================================

/// A code review for a task
///
/// Reviews track whether work was verified by AI or human reviewers.
/// Each review has a status (pending, approved, changes_requested, rejected)
/// and can have associated notes and actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    /// Unique identifier
    pub id: ReviewId,
    /// Project this review belongs to
    pub project_id: ProjectId,
    /// Task being reviewed
    pub task_id: TaskId,
    /// Who is performing the review
    pub reviewer_type: ReviewerType,
    /// Current status of the review
    pub status: ReviewStatus,
    /// Notes from the reviewer
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// When the review was created
    pub created_at: DateTime<Utc>,
    /// When the review was completed
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
}

impl Review {
    /// Create a new pending review
    pub fn new(project_id: ProjectId, task_id: TaskId, reviewer_type: ReviewerType) -> Self {
        Self {
            id: ReviewId::new(),
            project_id,
            task_id,
            reviewer_type,
            status: ReviewStatus::Pending,
            notes: None,
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    /// Create a review with a specific ID (for testing or database restoration)
    pub fn with_id(
        id: ReviewId,
        project_id: ProjectId,
        task_id: TaskId,
        reviewer_type: ReviewerType,
    ) -> Self {
        let mut review = Self::new(project_id, task_id, reviewer_type);
        review.id = id;
        review
    }

    /// Check if the review is pending
    pub fn is_pending(&self) -> bool {
        self.status == ReviewStatus::Pending
    }

    /// Check if the review is complete (any non-pending status)
    pub fn is_complete(&self) -> bool {
        self.status != ReviewStatus::Pending
    }

    /// Check if the review was approved
    pub fn is_approved(&self) -> bool {
        self.status == ReviewStatus::Approved
    }

    /// Approve the review
    pub fn approve(&mut self, notes: Option<String>) {
        self.status = ReviewStatus::Approved;
        self.notes = notes;
        self.completed_at = Some(Utc::now());
    }

    /// Request changes
    pub fn request_changes(&mut self, notes: String) {
        self.status = ReviewStatus::ChangesRequested;
        self.notes = Some(notes);
        self.completed_at = Some(Utc::now());
    }

    /// Reject the review
    pub fn reject(&mut self, notes: String) {
        self.status = ReviewStatus::Rejected;
        self.notes = Some(notes);
        self.completed_at = Some(Utc::now());
    }
}

// ========================================
// ReviewAction Entity
// ========================================

/// An action taken during or after a review
///
/// Actions track what happened as a result of a review:
/// - Fix tasks created
/// - Tasks moved to backlog
/// - Approvals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewAction {
    /// Unique identifier
    pub id: ReviewActionId,
    /// Review this action belongs to
    pub review_id: ReviewId,
    /// Type of action taken
    pub action_type: ReviewActionType,
    /// Target task ID (for created_fix_task actions)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_task_id: Option<TaskId>,
    /// When the action was taken
    pub created_at: DateTime<Utc>,
}

impl ReviewAction {
    /// Create a new review action
    pub fn new(review_id: ReviewId, action_type: ReviewActionType) -> Self {
        Self {
            id: ReviewActionId::new(),
            review_id,
            action_type,
            target_task_id: None,
            created_at: Utc::now(),
        }
    }

    /// Create an action with a target task (for fix tasks)
    pub fn with_target_task(
        review_id: ReviewId,
        action_type: ReviewActionType,
        target_task_id: TaskId,
    ) -> Self {
        let mut action = Self::new(review_id, action_type);
        action.target_task_id = Some(target_task_id);
        action
    }

    /// Create an action with a specific ID (for testing or database restoration)
    pub fn with_id(id: ReviewActionId, review_id: ReviewId, action_type: ReviewActionType) -> Self {
        let mut action = Self::new(review_id, action_type);
        action.id = id;
        action
    }

    /// Check if this action created a fix task
    pub fn is_fix_task_action(&self) -> bool {
        self.action_type == ReviewActionType::CreatedFixTask
    }
}

// ========================================
// ReviewNote Entity (for review history)
// ========================================

/// A unique identifier for a ReviewNote
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReviewNoteId(pub String);

impl ReviewNoteId {
    /// Creates a new ReviewNoteId with a random UUID v4
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Creates a ReviewNoteId from an existing string
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ReviewNoteId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ReviewNoteId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Outcome of a review (for review notes history)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewOutcome {
    /// Review approved the work
    Approved,
    /// Reviewer requested changes
    ChangesRequested,
    /// Review rejected the work
    Rejected,
}

impl std::fmt::Display for ReviewOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReviewOutcome::Approved => write!(f, "approved"),
            ReviewOutcome::ChangesRequested => write!(f, "changes_requested"),
            ReviewOutcome::Rejected => write!(f, "rejected"),
        }
    }
}

impl FromStr for ReviewOutcome {
    type Err = ParseReviewOutcomeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "approved" => Ok(ReviewOutcome::Approved),
            "changes_requested" => Ok(ReviewOutcome::ChangesRequested),
            "rejected" => Ok(ReviewOutcome::Rejected),
            _ => Err(ParseReviewOutcomeError(s.to_string())),
        }
    }
}

/// Error when parsing an invalid review outcome string
#[derive(Debug, Clone, PartialEq)]
pub struct ParseReviewOutcomeError(pub String);

impl std::fmt::Display for ParseReviewOutcomeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid review outcome: {}", self.0)
    }
}

impl std::error::Error for ParseReviewOutcomeError {}

/// Issue found during review
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReviewIssue {
    pub severity: String, // "critical" | "major" | "minor" | "suggestion"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<i32>,
    pub description: String,
}

/// A note from a reviewer (part of review history)
///
/// ReviewNotes store the feedback from each review attempt.
/// A task can have multiple review notes over time as it goes
/// through multiple review cycles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewNote {
    /// Unique identifier
    pub id: ReviewNoteId,
    /// Task this note belongs to
    pub task_id: TaskId,
    /// Who made the review (ai or human)
    pub reviewer: ReviewerType,
    /// Outcome of the review
    pub outcome: ReviewOutcome,
    /// Short summary for display in timeline
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Full notes/feedback from the reviewer
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// Issues found during review (stored as JSON in DB)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issues: Option<Vec<ReviewIssue>>,
    /// When the note was created
    pub created_at: DateTime<Utc>,
}

impl ReviewNote {
    /// Create a new review note
    pub fn new(task_id: TaskId, reviewer: ReviewerType, outcome: ReviewOutcome) -> Self {
        Self {
            id: ReviewNoteId::new(),
            task_id,
            reviewer,
            outcome,
            summary: None,
            notes: None,
            issues: None,
            created_at: Utc::now(),
        }
    }

    /// Create a review note with all fields
    pub fn with_content(
        task_id: TaskId,
        reviewer: ReviewerType,
        outcome: ReviewOutcome,
        summary: Option<String>,
        notes: Option<String>,
        issues: Option<Vec<ReviewIssue>>,
    ) -> Self {
        Self {
            id: ReviewNoteId::new(),
            task_id,
            reviewer,
            outcome,
            summary,
            notes,
            issues,
            created_at: Utc::now(),
        }
    }

    /// Create a review note with just notes (convenience method)
    pub fn with_notes(
        task_id: TaskId,
        reviewer: ReviewerType,
        outcome: ReviewOutcome,
        notes: String,
    ) -> Self {
        Self::with_content(task_id, reviewer, outcome, None, Some(notes), None)
    }

    /// Create a review note with a specific ID (for testing or database restoration)
    pub fn with_id(
        id: ReviewNoteId,
        task_id: TaskId,
        reviewer: ReviewerType,
        outcome: ReviewOutcome,
    ) -> Self {
        let mut note = Self::new(task_id, reviewer, outcome);
        note.id = id;
        note
    }

    /// Check if the review was positive (approved)
    pub fn is_positive(&self) -> bool {
        self.outcome == ReviewOutcome::Approved
    }

    /// Check if the review was negative (changes requested or rejected)
    pub fn is_negative(&self) -> bool {
        matches!(
            self.outcome,
            ReviewOutcome::ChangesRequested | ReviewOutcome::Rejected
        )
    }
}

#[cfg(test)]
#[path = "review_tests.rs"]
mod tests;
