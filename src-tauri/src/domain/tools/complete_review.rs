// complete_review tool input schema for the reviewer agent
// This defines the structure the AI reviewer uses to report review outcomes

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
// CompleteReviewInput
// ========================================

/// Input schema for the complete_review tool used by the reviewer agent
///
/// The reviewer agent calls this tool to report the outcome of a code review.
/// Based on the outcome, different fields are required:
/// - `approved`: Only `notes` is required
/// - `needs_changes`: `notes` and `fix_description` are required
/// - `escalate`: `notes` and `escalation_reason` are required
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteReviewInput {
    /// The review outcome
    pub outcome: ReviewToolOutcome,
    /// Detailed review notes explaining the decision
    pub notes: String,
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
}

impl std::fmt::Display for CompleteReviewValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompleteReviewValidationError::EmptyNotes => {
                write!(f, "notes field cannot be empty")
            }
            CompleteReviewValidationError::MissingFixDescription => {
                write!(f, "fix_description is required when outcome is 'needs_changes'")
            }
            CompleteReviewValidationError::EmptyFixDescription => {
                write!(f, "fix_description cannot be empty when outcome is 'needs_changes'")
            }
            CompleteReviewValidationError::MissingEscalationReason => {
                write!(f, "escalation_reason is required when outcome is 'escalate'")
            }
            CompleteReviewValidationError::EmptyEscalationReason => {
                write!(f, "escalation_reason cannot be empty when outcome is 'escalate'")
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
            fix_description: None,
            escalation_reason: None,
        }
    }

    /// Create a new needs_changes review input
    pub fn needs_changes(notes: impl Into<String>, fix_description: impl Into<String>) -> Self {
        Self {
            outcome: ReviewToolOutcome::NeedsChanges,
            notes: notes.into(),
            fix_description: Some(fix_description.into()),
            escalation_reason: None,
        }
    }

    /// Create a new escalate review input
    pub fn escalate(notes: impl Into<String>, escalation_reason: impl Into<String>) -> Self {
        Self {
            outcome: ReviewToolOutcome::Escalate,
            notes: notes.into(),
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

        match self.outcome {
            ReviewToolOutcome::Approved => {
                // No additional validation for approved
                Ok(())
            }
            ReviewToolOutcome::NeedsChanges => {
                // fix_description is required
                match &self.fix_description {
                    None => Err(CompleteReviewValidationError::MissingFixDescription),
                    Some(desc) if desc.trim().is_empty() => {
                        Err(CompleteReviewValidationError::EmptyFixDescription)
                    }
                    Some(_) => Ok(()),
                }
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
mod tests {
    use super::*;

    // ===== ReviewToolOutcome Tests =====

    #[test]
    fn test_review_tool_outcome_display() {
        assert_eq!(format!("{}", ReviewToolOutcome::Approved), "approved");
        assert_eq!(format!("{}", ReviewToolOutcome::NeedsChanges), "needs_changes");
        assert_eq!(format!("{}", ReviewToolOutcome::Escalate), "escalate");
    }

    #[test]
    fn test_review_tool_outcome_from_str() {
        assert_eq!(
            ReviewToolOutcome::from_str("approved").unwrap(),
            ReviewToolOutcome::Approved
        );
        assert_eq!(
            ReviewToolOutcome::from_str("APPROVED").unwrap(),
            ReviewToolOutcome::Approved
        );
        assert_eq!(
            ReviewToolOutcome::from_str("needs_changes").unwrap(),
            ReviewToolOutcome::NeedsChanges
        );
        assert_eq!(
            ReviewToolOutcome::from_str("escalate").unwrap(),
            ReviewToolOutcome::Escalate
        );
    }

    #[test]
    fn test_review_tool_outcome_from_str_invalid() {
        let result = ReviewToolOutcome::from_str("invalid");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.0, "invalid");
        assert!(err.to_string().contains("invalid review tool outcome"));
    }

    #[test]
    fn test_review_tool_outcome_serialization() {
        let outcome = ReviewToolOutcome::NeedsChanges;
        let json = serde_json::to_string(&outcome).unwrap();
        assert_eq!(json, "\"needs_changes\"");

        let parsed: ReviewToolOutcome = serde_json::from_str(&json).unwrap();
        assert_eq!(outcome, parsed);
    }

    // ===== CompleteReviewInput Constructor Tests =====

    #[test]
    fn test_complete_review_input_approved() {
        let input = CompleteReviewInput::approved("Looks good, all tests pass");

        assert_eq!(input.outcome, ReviewToolOutcome::Approved);
        assert_eq!(input.notes, "Looks good, all tests pass");
        assert!(input.fix_description.is_none());
        assert!(input.escalation_reason.is_none());
        assert!(input.is_approved());
        assert!(!input.is_needs_changes());
        assert!(!input.is_escalation());
    }

    #[test]
    fn test_complete_review_input_needs_changes() {
        let input = CompleteReviewInput::needs_changes(
            "Missing error handling",
            "Add try-catch blocks around API calls",
        );

        assert_eq!(input.outcome, ReviewToolOutcome::NeedsChanges);
        assert_eq!(input.notes, "Missing error handling");
        assert_eq!(
            input.fix_description,
            Some("Add try-catch blocks around API calls".to_string())
        );
        assert!(input.escalation_reason.is_none());
        assert!(!input.is_approved());
        assert!(input.is_needs_changes());
        assert!(!input.is_escalation());
    }

    #[test]
    fn test_complete_review_input_escalate() {
        let input = CompleteReviewInput::escalate(
            "Security-sensitive authentication changes",
            "Changes bypass the standard auth flow, needs human review",
        );

        assert_eq!(input.outcome, ReviewToolOutcome::Escalate);
        assert_eq!(input.notes, "Security-sensitive authentication changes");
        assert!(input.fix_description.is_none());
        assert_eq!(
            input.escalation_reason,
            Some("Changes bypass the standard auth flow, needs human review".to_string())
        );
        assert!(!input.is_approved());
        assert!(!input.is_needs_changes());
        assert!(input.is_escalation());
    }

    // ===== Validation Tests =====

    #[test]
    fn test_validate_approved_valid() {
        let input = CompleteReviewInput::approved("All criteria met");
        assert!(input.validate().is_ok());
        assert!(input.is_valid());
    }

    #[test]
    fn test_validate_approved_empty_notes() {
        let input = CompleteReviewInput {
            outcome: ReviewToolOutcome::Approved,
            notes: "".to_string(),
            fix_description: None,
            escalation_reason: None,
        };
        assert_eq!(
            input.validate(),
            Err(CompleteReviewValidationError::EmptyNotes)
        );
        assert!(!input.is_valid());
    }

    #[test]
    fn test_validate_approved_whitespace_notes() {
        let input = CompleteReviewInput {
            outcome: ReviewToolOutcome::Approved,
            notes: "   \n\t  ".to_string(),
            fix_description: None,
            escalation_reason: None,
        };
        assert_eq!(
            input.validate(),
            Err(CompleteReviewValidationError::EmptyNotes)
        );
    }

    #[test]
    fn test_validate_needs_changes_valid() {
        let input = CompleteReviewInput::needs_changes("Issues found", "Fix the bug");
        assert!(input.validate().is_ok());
        assert!(input.is_valid());
    }

    #[test]
    fn test_validate_needs_changes_missing_fix_description() {
        let input = CompleteReviewInput {
            outcome: ReviewToolOutcome::NeedsChanges,
            notes: "Issues found".to_string(),
            fix_description: None,
            escalation_reason: None,
        };
        assert_eq!(
            input.validate(),
            Err(CompleteReviewValidationError::MissingFixDescription)
        );
    }

    #[test]
    fn test_validate_needs_changes_empty_fix_description() {
        let input = CompleteReviewInput {
            outcome: ReviewToolOutcome::NeedsChanges,
            notes: "Issues found".to_string(),
            fix_description: Some("".to_string()),
            escalation_reason: None,
        };
        assert_eq!(
            input.validate(),
            Err(CompleteReviewValidationError::EmptyFixDescription)
        );
    }

    #[test]
    fn test_validate_needs_changes_whitespace_fix_description() {
        let input = CompleteReviewInput {
            outcome: ReviewToolOutcome::NeedsChanges,
            notes: "Issues found".to_string(),
            fix_description: Some("   ".to_string()),
            escalation_reason: None,
        };
        assert_eq!(
            input.validate(),
            Err(CompleteReviewValidationError::EmptyFixDescription)
        );
    }

    #[test]
    fn test_validate_escalate_valid() {
        let input = CompleteReviewInput::escalate("Security concern", "Needs human review");
        assert!(input.validate().is_ok());
        assert!(input.is_valid());
    }

    #[test]
    fn test_validate_escalate_missing_escalation_reason() {
        let input = CompleteReviewInput {
            outcome: ReviewToolOutcome::Escalate,
            notes: "Security concern".to_string(),
            fix_description: None,
            escalation_reason: None,
        };
        assert_eq!(
            input.validate(),
            Err(CompleteReviewValidationError::MissingEscalationReason)
        );
    }

    #[test]
    fn test_validate_escalate_empty_escalation_reason() {
        let input = CompleteReviewInput {
            outcome: ReviewToolOutcome::Escalate,
            notes: "Security concern".to_string(),
            fix_description: None,
            escalation_reason: Some("".to_string()),
        };
        assert_eq!(
            input.validate(),
            Err(CompleteReviewValidationError::EmptyEscalationReason)
        );
    }

    #[test]
    fn test_validate_escalate_whitespace_escalation_reason() {
        let input = CompleteReviewInput {
            outcome: ReviewToolOutcome::Escalate,
            notes: "Security concern".to_string(),
            fix_description: None,
            escalation_reason: Some("  \t\n  ".to_string()),
        };
        assert_eq!(
            input.validate(),
            Err(CompleteReviewValidationError::EmptyEscalationReason)
        );
    }

    // ===== Serialization Tests =====

    #[test]
    fn test_complete_review_input_serialization_approved() {
        let input = CompleteReviewInput::approved("All good");
        let json = serde_json::to_string(&input).unwrap();

        // Should not include optional fields that are None
        assert!(!json.contains("fix_description"));
        assert!(!json.contains("escalation_reason"));

        let parsed: CompleteReviewInput = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.outcome, input.outcome);
        assert_eq!(parsed.notes, input.notes);
    }

    #[test]
    fn test_complete_review_input_serialization_needs_changes() {
        let input = CompleteReviewInput::needs_changes("Issues", "Fix them");
        let json = serde_json::to_string(&input).unwrap();

        assert!(json.contains("\"fix_description\":\"Fix them\""));

        let parsed: CompleteReviewInput = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.outcome, input.outcome);
        assert_eq!(parsed.fix_description, input.fix_description);
    }

    #[test]
    fn test_complete_review_input_serialization_escalate() {
        let input = CompleteReviewInput::escalate("Concern", "Need human");
        let json = serde_json::to_string(&input).unwrap();

        assert!(json.contains("\"escalation_reason\":\"Need human\""));

        let parsed: CompleteReviewInput = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.outcome, input.outcome);
        assert_eq!(parsed.escalation_reason, input.escalation_reason);
    }

    #[test]
    fn test_complete_review_input_deserialization_with_defaults() {
        // Test deserializing JSON that doesn't have optional fields
        let json = r#"{"outcome":"approved","notes":"LGTM"}"#;
        let input: CompleteReviewInput = serde_json::from_str(json).unwrap();

        assert_eq!(input.outcome, ReviewToolOutcome::Approved);
        assert_eq!(input.notes, "LGTM");
        assert!(input.fix_description.is_none());
        assert!(input.escalation_reason.is_none());
    }

    // ===== Error Display Tests =====

    #[test]
    fn test_validation_error_display() {
        assert_eq!(
            CompleteReviewValidationError::EmptyNotes.to_string(),
            "notes field cannot be empty"
        );
        assert_eq!(
            CompleteReviewValidationError::MissingFixDescription.to_string(),
            "fix_description is required when outcome is 'needs_changes'"
        );
        assert_eq!(
            CompleteReviewValidationError::EmptyFixDescription.to_string(),
            "fix_description cannot be empty when outcome is 'needs_changes'"
        );
        assert_eq!(
            CompleteReviewValidationError::MissingEscalationReason.to_string(),
            "escalation_reason is required when outcome is 'escalate'"
        );
        assert_eq!(
            CompleteReviewValidationError::EmptyEscalationReason.to_string(),
            "escalation_reason cannot be empty when outcome is 'escalate'"
        );
    }
}
