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
}

impl std::fmt::Display for ReviewerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReviewerType::Ai => write!(f, "ai"),
            ReviewerType::Human => write!(f, "human"),
        }
    }
}

impl FromStr for ReviewerType {
    type Err = ParseReviewerTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ai" => Ok(ReviewerType::Ai),
            "human" => Ok(ReviewerType::Human),
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
    pub fn with_id(
        id: ReviewActionId,
        review_id: ReviewId,
        action_type: ReviewActionType,
    ) -> Self {
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
mod tests {
    use super::*;

    // ===== ReviewId Tests =====

    #[test]
    fn test_review_id_new_generates_valid_uuid() {
        let id = ReviewId::new();
        assert_eq!(id.as_str().len(), 36);
        assert!(uuid::Uuid::parse_str(id.as_str()).is_ok());
    }

    #[test]
    fn test_review_id_from_string() {
        let id = ReviewId::from_string("rev-123");
        assert_eq!(id.as_str(), "rev-123");
    }

    #[test]
    fn test_review_id_equality() {
        let id1 = ReviewId::from_string("rev-abc");
        let id2 = ReviewId::from_string("rev-abc");
        let id3 = ReviewId::from_string("rev-xyz");
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_review_id_serialization() {
        let id = ReviewId::from_string("rev-serialize");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"rev-serialize\"");
        let parsed: ReviewId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, parsed);
    }

    // ===== ReviewActionId Tests =====

    #[test]
    fn test_review_action_id_new_generates_valid_uuid() {
        let id = ReviewActionId::new();
        assert_eq!(id.as_str().len(), 36);
        assert!(uuid::Uuid::parse_str(id.as_str()).is_ok());
    }

    #[test]
    fn test_review_action_id_from_string() {
        let id = ReviewActionId::from_string("action-123");
        assert_eq!(id.as_str(), "action-123");
    }

    // ===== ReviewerType Tests =====

    #[test]
    fn test_reviewer_type_display() {
        assert_eq!(format!("{}", ReviewerType::Ai), "ai");
        assert_eq!(format!("{}", ReviewerType::Human), "human");
    }

    #[test]
    fn test_reviewer_type_from_str() {
        assert_eq!(ReviewerType::from_str("ai").unwrap(), ReviewerType::Ai);
        assert_eq!(ReviewerType::from_str("AI").unwrap(), ReviewerType::Ai);
        assert_eq!(ReviewerType::from_str("human").unwrap(), ReviewerType::Human);
        assert_eq!(ReviewerType::from_str("HUMAN").unwrap(), ReviewerType::Human);
        assert!(ReviewerType::from_str("invalid").is_err());
    }

    #[test]
    fn test_reviewer_type_serialization() {
        let ai = ReviewerType::Ai;
        let json = serde_json::to_string(&ai).unwrap();
        assert_eq!(json, "\"ai\"");
        let parsed: ReviewerType = serde_json::from_str(&json).unwrap();
        assert_eq!(ai, parsed);
    }

    // ===== ReviewStatus Tests =====

    #[test]
    fn test_review_status_display() {
        assert_eq!(format!("{}", ReviewStatus::Pending), "pending");
        assert_eq!(format!("{}", ReviewStatus::Approved), "approved");
        assert_eq!(format!("{}", ReviewStatus::ChangesRequested), "changes_requested");
        assert_eq!(format!("{}", ReviewStatus::Rejected), "rejected");
    }

    #[test]
    fn test_review_status_from_str() {
        assert_eq!(ReviewStatus::from_str("pending").unwrap(), ReviewStatus::Pending);
        assert_eq!(ReviewStatus::from_str("approved").unwrap(), ReviewStatus::Approved);
        assert_eq!(ReviewStatus::from_str("changes_requested").unwrap(), ReviewStatus::ChangesRequested);
        assert_eq!(ReviewStatus::from_str("rejected").unwrap(), ReviewStatus::Rejected);
        assert!(ReviewStatus::from_str("invalid").is_err());
    }

    #[test]
    fn test_review_status_serialization() {
        let status = ReviewStatus::ChangesRequested;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"changes_requested\"");
        let parsed: ReviewStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, parsed);
    }

    // ===== ReviewActionType Tests =====

    #[test]
    fn test_review_action_type_display() {
        assert_eq!(format!("{}", ReviewActionType::CreatedFixTask), "created_fix_task");
        assert_eq!(format!("{}", ReviewActionType::MovedToBacklog), "moved_to_backlog");
        assert_eq!(format!("{}", ReviewActionType::Approved), "approved");
    }

    #[test]
    fn test_review_action_type_from_str() {
        assert_eq!(ReviewActionType::from_str("created_fix_task").unwrap(), ReviewActionType::CreatedFixTask);
        assert_eq!(ReviewActionType::from_str("moved_to_backlog").unwrap(), ReviewActionType::MovedToBacklog);
        assert_eq!(ReviewActionType::from_str("approved").unwrap(), ReviewActionType::Approved);
        assert!(ReviewActionType::from_str("invalid").is_err());
    }

    #[test]
    fn test_review_action_type_serialization() {
        let action_type = ReviewActionType::CreatedFixTask;
        let json = serde_json::to_string(&action_type).unwrap();
        assert_eq!(json, "\"created_fix_task\"");
        let parsed: ReviewActionType = serde_json::from_str(&json).unwrap();
        assert_eq!(action_type, parsed);
    }

    // ===== Review Tests =====

    #[test]
    fn test_review_new() {
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());
        let review = Review::new(project_id.clone(), task_id.clone(), ReviewerType::Ai);

        assert_eq!(review.project_id, project_id);
        assert_eq!(review.task_id, task_id);
        assert_eq!(review.reviewer_type, ReviewerType::Ai);
        assert_eq!(review.status, ReviewStatus::Pending);
        assert!(review.notes.is_none());
        assert!(review.completed_at.is_none());
        assert!(review.is_pending());
        assert!(!review.is_complete());
    }

    #[test]
    fn test_review_with_id() {
        let id = ReviewId::from_string("rev-custom");
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());
        let review = Review::with_id(id.clone(), project_id, task_id, ReviewerType::Human);

        assert_eq!(review.id, id);
    }

    #[test]
    fn test_review_approve() {
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());
        let mut review = Review::new(project_id, task_id, ReviewerType::Ai);

        review.approve(Some("Looks good!".to_string()));

        assert!(review.is_approved());
        assert!(review.is_complete());
        assert!(!review.is_pending());
        assert_eq!(review.notes, Some("Looks good!".to_string()));
        assert!(review.completed_at.is_some());
    }

    #[test]
    fn test_review_request_changes() {
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());
        let mut review = Review::new(project_id, task_id, ReviewerType::Ai);

        review.request_changes("Missing tests".to_string());

        assert_eq!(review.status, ReviewStatus::ChangesRequested);
        assert!(review.is_complete());
        assert!(!review.is_approved());
        assert_eq!(review.notes, Some("Missing tests".to_string()));
        assert!(review.completed_at.is_some());
    }

    #[test]
    fn test_review_reject() {
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());
        let mut review = Review::new(project_id, task_id, ReviewerType::Human);

        review.reject("Fundamentally wrong approach".to_string());

        assert_eq!(review.status, ReviewStatus::Rejected);
        assert!(review.is_complete());
        assert!(!review.is_approved());
        assert_eq!(review.notes, Some("Fundamentally wrong approach".to_string()));
        assert!(review.completed_at.is_some());
    }

    #[test]
    fn test_review_serialization() {
        let project_id = ProjectId::from_string("proj-1".to_string());
        let task_id = TaskId::from_string("task-1".to_string());
        let review = Review::new(project_id, task_id, ReviewerType::Ai);

        let json = serde_json::to_string(&review).unwrap();
        let parsed: Review = serde_json::from_str(&json).unwrap();

        assert_eq!(review.id, parsed.id);
        assert_eq!(review.project_id, parsed.project_id);
        assert_eq!(review.task_id, parsed.task_id);
        assert_eq!(review.reviewer_type, parsed.reviewer_type);
        assert_eq!(review.status, parsed.status);
    }

    // ===== ReviewAction Tests =====

    #[test]
    fn test_review_action_new() {
        let review_id = ReviewId::from_string("rev-1");
        let action = ReviewAction::new(review_id.clone(), ReviewActionType::Approved);

        assert_eq!(action.review_id, review_id);
        assert_eq!(action.action_type, ReviewActionType::Approved);
        assert!(action.target_task_id.is_none());
        assert!(!action.is_fix_task_action());
    }

    #[test]
    fn test_review_action_with_target_task() {
        let review_id = ReviewId::from_string("rev-1");
        let target_task = TaskId::from_string("fix-task-1".to_string());
        let action = ReviewAction::with_target_task(
            review_id.clone(),
            ReviewActionType::CreatedFixTask,
            target_task.clone(),
        );

        assert_eq!(action.review_id, review_id);
        assert_eq!(action.action_type, ReviewActionType::CreatedFixTask);
        assert_eq!(action.target_task_id, Some(target_task));
        assert!(action.is_fix_task_action());
    }

    #[test]
    fn test_review_action_with_id() {
        let id = ReviewActionId::from_string("action-custom");
        let review_id = ReviewId::from_string("rev-1");
        let action = ReviewAction::with_id(id.clone(), review_id, ReviewActionType::Approved);

        assert_eq!(action.id, id);
    }

    #[test]
    fn test_review_action_serialization() {
        let review_id = ReviewId::from_string("rev-1");
        let action = ReviewAction::new(review_id, ReviewActionType::MovedToBacklog);

        let json = serde_json::to_string(&action).unwrap();
        let parsed: ReviewAction = serde_json::from_str(&json).unwrap();

        assert_eq!(action.id, parsed.id);
        assert_eq!(action.review_id, parsed.review_id);
        assert_eq!(action.action_type, parsed.action_type);
    }

    // ===== ReviewNoteId Tests =====

    #[test]
    fn test_review_note_id_new_generates_valid_uuid() {
        let id = ReviewNoteId::new();
        assert_eq!(id.as_str().len(), 36);
        assert!(uuid::Uuid::parse_str(id.as_str()).is_ok());
    }

    #[test]
    fn test_review_note_id_from_string() {
        let id = ReviewNoteId::from_string("note-123");
        assert_eq!(id.as_str(), "note-123");
    }

    #[test]
    fn test_review_note_id_equality() {
        let id1 = ReviewNoteId::from_string("note-abc");
        let id2 = ReviewNoteId::from_string("note-abc");
        let id3 = ReviewNoteId::from_string("note-xyz");
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_review_note_id_serialization() {
        let id = ReviewNoteId::from_string("note-serialize");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"note-serialize\"");
        let parsed: ReviewNoteId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, parsed);
    }

    // ===== ReviewOutcome Tests =====

    #[test]
    fn test_review_outcome_display() {
        assert_eq!(format!("{}", ReviewOutcome::Approved), "approved");
        assert_eq!(format!("{}", ReviewOutcome::ChangesRequested), "changes_requested");
        assert_eq!(format!("{}", ReviewOutcome::Rejected), "rejected");
    }

    #[test]
    fn test_review_outcome_from_str() {
        assert_eq!(ReviewOutcome::from_str("approved").unwrap(), ReviewOutcome::Approved);
        assert_eq!(ReviewOutcome::from_str("changes_requested").unwrap(), ReviewOutcome::ChangesRequested);
        assert_eq!(ReviewOutcome::from_str("rejected").unwrap(), ReviewOutcome::Rejected);
        assert!(ReviewOutcome::from_str("invalid").is_err());
    }

    #[test]
    fn test_review_outcome_serialization() {
        let outcome = ReviewOutcome::ChangesRequested;
        let json = serde_json::to_string(&outcome).unwrap();
        assert_eq!(json, "\"changes_requested\"");
        let parsed: ReviewOutcome = serde_json::from_str(&json).unwrap();
        assert_eq!(outcome, parsed);
    }

    // ===== ReviewNote Tests =====

    #[test]
    fn test_review_note_new() {
        let task_id = TaskId::from_string("task-1".to_string());
        let note = ReviewNote::new(task_id.clone(), ReviewerType::Ai, ReviewOutcome::Approved);

        assert_eq!(note.task_id, task_id);
        assert_eq!(note.reviewer, ReviewerType::Ai);
        assert_eq!(note.outcome, ReviewOutcome::Approved);
        assert!(note.notes.is_none());
        assert!(note.is_positive());
        assert!(!note.is_negative());
    }

    #[test]
    fn test_review_note_with_notes() {
        let task_id = TaskId::from_string("task-1".to_string());
        let note = ReviewNote::with_notes(
            task_id.clone(),
            ReviewerType::Human,
            ReviewOutcome::ChangesRequested,
            "Missing tests".to_string(),
        );

        assert_eq!(note.task_id, task_id);
        assert_eq!(note.reviewer, ReviewerType::Human);
        assert_eq!(note.outcome, ReviewOutcome::ChangesRequested);
        assert_eq!(note.notes, Some("Missing tests".to_string()));
        assert!(!note.is_positive());
        assert!(note.is_negative());
    }

    #[test]
    fn test_review_note_with_id() {
        let id = ReviewNoteId::from_string("note-custom");
        let task_id = TaskId::from_string("task-1".to_string());
        let note = ReviewNote::with_id(id.clone(), task_id, ReviewerType::Ai, ReviewOutcome::Approved);

        assert_eq!(note.id, id);
    }

    #[test]
    fn test_review_note_is_positive() {
        let task_id = TaskId::from_string("task-1".to_string());

        let approved = ReviewNote::new(task_id.clone(), ReviewerType::Ai, ReviewOutcome::Approved);
        let changes = ReviewNote::new(task_id.clone(), ReviewerType::Ai, ReviewOutcome::ChangesRequested);
        let rejected = ReviewNote::new(task_id, ReviewerType::Ai, ReviewOutcome::Rejected);

        assert!(approved.is_positive());
        assert!(!changes.is_positive());
        assert!(!rejected.is_positive());
    }

    #[test]
    fn test_review_note_is_negative() {
        let task_id = TaskId::from_string("task-1".to_string());

        let approved = ReviewNote::new(task_id.clone(), ReviewerType::Ai, ReviewOutcome::Approved);
        let changes = ReviewNote::new(task_id.clone(), ReviewerType::Ai, ReviewOutcome::ChangesRequested);
        let rejected = ReviewNote::new(task_id, ReviewerType::Ai, ReviewOutcome::Rejected);

        assert!(!approved.is_negative());
        assert!(changes.is_negative());
        assert!(rejected.is_negative());
    }

    #[test]
    fn test_review_note_serialization() {
        let task_id = TaskId::from_string("task-1".to_string());
        let note = ReviewNote::with_notes(
            task_id,
            ReviewerType::Human,
            ReviewOutcome::Approved,
            "Looks good!".to_string(),
        );

        let json = serde_json::to_string(&note).unwrap();
        let parsed: ReviewNote = serde_json::from_str(&json).unwrap();

        assert_eq!(note.id, parsed.id);
        assert_eq!(note.task_id, parsed.task_id);
        assert_eq!(note.reviewer, parsed.reviewer);
        assert_eq!(note.outcome, parsed.outcome);
        assert_eq!(note.notes, parsed.notes);
    }
}
