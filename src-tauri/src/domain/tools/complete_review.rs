// complete_review tool input schema for the reviewer agent
// This defines the structure the AI reviewer uses to report review outcomes

use crate::domain::entities::{IssueCategory, IssueSeverity, TaskStepId};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

// ========================================
// ReviewToolOutcome Enum
// ========================================

/// Possible outcomes from the reviewer agent's complete_review tool
/// Note: This is distinct from ReviewStatus which tracks the review entity state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewToolOutcome {
    /// Work verified, task complete - transitions to approved
    Approved,
    /// Issues found that can be fixed - creates fix task
    NeedsChanges,
    /// Needs human review (security-sensitive, design decision, unclear requirements)
    Escalate,
}

impl std::fmt::Display for ReviewToolOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReviewToolOutcome::Approved => write!(f, "approved"),
            ReviewToolOutcome::NeedsChanges => write!(f, "needs_changes"),
            ReviewToolOutcome::Escalate => write!(f, "escalate"),
        }
    }
}

impl FromStr for ReviewToolOutcome {
    type Err = ParseReviewToolOutcomeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "approved" => Ok(ReviewToolOutcome::Approved),
            "needs_changes" => Ok(ReviewToolOutcome::NeedsChanges),
            "escalate" => Ok(ReviewToolOutcome::Escalate),
            _ => Err(ParseReviewToolOutcomeError(s.to_string())),
        }
    }
}

/// Error when parsing an invalid review tool outcome string
#[derive(Debug, Clone, PartialEq)]
pub struct ParseReviewToolOutcomeError(pub String);

impl std::fmt::Display for ParseReviewToolOutcomeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid review tool outcome: '{}', expected 'approved', 'needs_changes', or 'escalate'",
            self.0
        )
    }
}

impl std::error::Error for ParseReviewToolOutcomeError {}

// ========================================
// ReviewIssueInput
// ========================================

/// Input struct for creating a structured issue during review
///
/// Each issue must either link to a specific task step (via `step_id`) or
/// provide a justification for why it doesn't relate to a step (via `no_step_reason`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewIssueInput {
    /// Short title describing the issue
    pub title: String,
    /// Optional detailed description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Severity of the issue (critical, major, minor, suggestion)
    pub severity: IssueSeverity,
    /// Category of the issue (bug, missing, quality, design)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<IssueCategory>,
    /// Optional link to a specific task step this issue relates to
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_id: Option<TaskStepId>,
    /// Required justification if step_id is None
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_step_reason: Option<String>,
    /// Optional file path where issue was found
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    /// Optional line number in the file
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_number: Option<i32>,
    /// Optional code snippet showing the issue
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_snippet: Option<String>,
}

impl ReviewIssueInput {
    /// Create a new issue input with required fields only
    pub fn new(title: impl Into<String>, severity: IssueSeverity) -> Self {
        Self {
            title: title.into(),
            description: None,
            severity,
            category: None,
            step_id: None,
            no_step_reason: None,
            file_path: None,
            line_number: None,
            code_snippet: None,
        }
    }

    /// Set the step_id for this issue
    pub fn with_step_id(mut self, step_id: TaskStepId) -> Self {
        self.step_id = Some(step_id);
        self
    }

    /// Set the no_step_reason for this issue
    pub fn with_no_step_reason(mut self, reason: impl Into<String>) -> Self {
        self.no_step_reason = Some(reason.into());
        self
    }

    /// Set the category for this issue
    pub fn with_category(mut self, category: IssueCategory) -> Self {
        self.category = Some(category);
        self
    }

    /// Set the description for this issue
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the file location for this issue
    pub fn with_file_location(
        mut self,
        file_path: impl Into<String>,
        line_number: Option<i32>,
    ) -> Self {
        self.file_path = Some(file_path.into());
        self.line_number = line_number;
        self
    }

    /// Validate the issue input
    ///
    /// Returns Ok(()) if valid, or the first validation error encountered.
    pub fn validate(&self) -> Result<(), ReviewIssueValidationError> {
        // Title must not be empty
        if self.title.trim().is_empty() {
            return Err(ReviewIssueValidationError::EmptyTitle);
        }

        // Either step_id OR no_step_reason must be provided
        let has_step = self.step_id.is_some();
        let has_reason = self
            .no_step_reason
            .as_ref()
            .is_some_and(|r| !r.trim().is_empty());

        if !has_step && !has_reason {
            return Err(ReviewIssueValidationError::MissingStepOrReason);
        }

        Ok(())
    }
}

/// Validation errors for ReviewIssueInput
#[derive(Debug, Clone, PartialEq)]
pub enum ReviewIssueValidationError {
    /// Title field is empty
    EmptyTitle,
    /// Either step_id or no_step_reason must be provided
    MissingStepOrReason,
}

impl std::fmt::Display for ReviewIssueValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReviewIssueValidationError::EmptyTitle => {
                write!(f, "issue title cannot be empty")
            }
            ReviewIssueValidationError::MissingStepOrReason => {
                write!(f, "issue must have either step_id or no_step_reason")
            }
        }
    }
}

impl std::error::Error for ReviewIssueValidationError {}

// ========================================
// CompleteReviewInput
// ========================================

/// Input schema for the complete_review tool used by the reviewer agent
///
/// The reviewer agent calls this tool to report the outcome of a code review.
/// Based on the outcome, different fields are required:
/// - `approved`: Only `notes` is required, `issues` is optional
/// - `needs_changes`: `notes`, `fix_description`, and non-empty `issues` are required
/// - `escalate`: `notes` and `escalation_reason` are required
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteReviewInput {
    /// The review outcome
    pub outcome: ReviewToolOutcome,
    /// Detailed review notes explaining the decision
    pub notes: String,
    /// Structured issues found during review (required if outcome is needs_changes)
    #[serde(default)]
    pub issues: Vec<ReviewIssueInput>,
    /// Description for the fix task (required if outcome is needs_changes)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fix_description: Option<String>,
    /// Reason for escalation to human (required if outcome is escalate)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub escalation_reason: Option<String>,
}

/// Validation errors for CompleteReviewInput
#[derive(Debug, Clone, PartialEq)]
pub enum CompleteReviewValidationError {
    /// Notes field is empty
    EmptyNotes,
    /// fix_description is required when outcome is needs_changes
    MissingFixDescription,
    /// fix_description is empty when outcome is needs_changes
    EmptyFixDescription,
    /// escalation_reason is required when outcome is escalate
    MissingEscalationReason,
    /// escalation_reason is empty when outcome is escalate
    EmptyEscalationReason,
    /// issues is required when outcome is needs_changes
    MissingIssues,
    /// An issue has a validation error (index, error)
    InvalidIssue(usize, ReviewIssueValidationError),
}

impl std::fmt::Display for CompleteReviewValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompleteReviewValidationError::EmptyNotes => {
                write!(f, "notes field cannot be empty")
            }
            CompleteReviewValidationError::MissingFixDescription => {
                write!(
                    f,
                    "fix_description is required when outcome is 'needs_changes'"
                )
            }
            CompleteReviewValidationError::EmptyFixDescription => {
                write!(
                    f,
                    "fix_description cannot be empty when outcome is 'needs_changes'"
                )
            }
            CompleteReviewValidationError::MissingEscalationReason => {
                write!(
                    f,
                    "escalation_reason is required when outcome is 'escalate'"
                )
            }
            CompleteReviewValidationError::EmptyEscalationReason => {
                write!(
                    f,
                    "escalation_reason cannot be empty when outcome is 'escalate'"
                )
            }
            CompleteReviewValidationError::MissingIssues => {
                write!(f, "issues are required when outcome is 'needs_changes'")
            }
            CompleteReviewValidationError::InvalidIssue(idx, err) => {
                write!(f, "issue at index {}: {}", idx, err)
            }
        }
    }
}

impl std::error::Error for CompleteReviewValidationError {}

impl CompleteReviewInput {
    /// Create a new approved review input
    pub fn approved(notes: impl Into<String>) -> Self {
        Self {
            outcome: ReviewToolOutcome::Approved,
            notes: notes.into(),
            issues: Vec::new(),
            fix_description: None,
            escalation_reason: None,
        }
    }

    /// Create a new needs_changes review input
    ///
    /// Note: This is a legacy constructor that creates an empty issues list.
    /// For proper validation, use `needs_changes_with_issues` instead.
    pub fn needs_changes(notes: impl Into<String>, fix_description: impl Into<String>) -> Self {
        Self {
            outcome: ReviewToolOutcome::NeedsChanges,
            notes: notes.into(),
            issues: Vec::new(),
            fix_description: Some(fix_description.into()),
            escalation_reason: None,
        }
    }

    /// Create a new needs_changes review input with structured issues
    pub fn needs_changes_with_issues(
        notes: impl Into<String>,
        fix_description: impl Into<String>,
        issues: Vec<ReviewIssueInput>,
    ) -> Self {
        Self {
            outcome: ReviewToolOutcome::NeedsChanges,
            notes: notes.into(),
            issues,
            fix_description: Some(fix_description.into()),
            escalation_reason: None,
        }
    }

    /// Create a new escalate review input
    pub fn escalate(notes: impl Into<String>, escalation_reason: impl Into<String>) -> Self {
        Self {
            outcome: ReviewToolOutcome::Escalate,
            notes: notes.into(),
            issues: Vec::new(),
            fix_description: None,
            escalation_reason: Some(escalation_reason.into()),
        }
    }

    /// Validate the input according to the outcome
    ///
    /// Returns Ok(()) if valid, or the first validation error encountered.
    pub fn validate(&self) -> Result<(), CompleteReviewValidationError> {
        // Notes must not be empty
        if self.notes.trim().is_empty() {
            return Err(CompleteReviewValidationError::EmptyNotes);
        }

        // Validate all issues if present
        for (idx, issue) in self.issues.iter().enumerate() {
            if let Err(e) = issue.validate() {
                return Err(CompleteReviewValidationError::InvalidIssue(idx, e));
            }
        }

        match self.outcome {
            ReviewToolOutcome::Approved => {
                // No additional validation for approved
                // Issues are optional for approved
                Ok(())
            }
            ReviewToolOutcome::NeedsChanges => {
                // fix_description is required
                match &self.fix_description {
                    None => return Err(CompleteReviewValidationError::MissingFixDescription),
                    Some(desc) if desc.trim().is_empty() => {
                        return Err(CompleteReviewValidationError::EmptyFixDescription)
                    }
                    Some(_) => {}
                }

                // issues are required for needs_changes
                if self.issues.is_empty() {
                    return Err(CompleteReviewValidationError::MissingIssues);
                }

                Ok(())
            }
            ReviewToolOutcome::Escalate => {
                // escalation_reason is required
                match &self.escalation_reason {
                    None => Err(CompleteReviewValidationError::MissingEscalationReason),
                    Some(reason) if reason.trim().is_empty() => {
                        Err(CompleteReviewValidationError::EmptyEscalationReason)
                    }
                    Some(_) => Ok(()),
                }
            }
        }
    }

    /// Check if this input is valid
    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }

    /// Check if this is an approval
    pub fn is_approved(&self) -> bool {
        self.outcome == ReviewToolOutcome::Approved
    }

    /// Check if this needs changes
    pub fn is_needs_changes(&self) -> bool {
        self.outcome == ReviewToolOutcome::NeedsChanges
    }

    /// Check if this is an escalation
    pub fn is_escalation(&self) -> bool {
        self.outcome == ReviewToolOutcome::Escalate
    }
}

#[cfg(test)]
#[path = "complete_review_tests.rs"]
mod tests;
