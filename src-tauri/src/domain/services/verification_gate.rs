/// Verification gate — checks whether a session's plan is eligible for acceptance
/// or proposal mutation (create/update/delete).
///
/// Called from all 3 acceptance paths: Tauri IPC, internal MCP HTTP, external MCP.
use crate::domain::entities::ideation::{VerificationError, VerificationStatus};
use crate::domain::entities::IdeationSession;
use crate::domain::ideation::config::IdeationSettings;

/// Identifies the proposal mutation operation being gated.
///
/// Used by `check_proposal_verification_gate()` to determine which statuses to block.
/// Distinct from `UpdateSource` which tracks the caller origin (API vs. IPC).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalOperation {
    Create,
    Update,
    Delete,
}

impl std::fmt::Display for ProposalOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProposalOperation::Create => write!(f, "create"),
            ProposalOperation::Update => write!(f, "update"),
            ProposalOperation::Delete => write!(f, "delete"),
        }
    }
}

/// Check if the session's plan is eligible for acceptance.
///
/// # Errors
///
/// Returns a `VerificationError` when the gate blocks acceptance.
pub fn check_verification_gate(
    session: &IdeationSession,
    settings: &IdeationSettings,
) -> Result<(), VerificationError> {
    if !settings.require_verification_for_accept {
        return Ok(());
    }
    match session.verification_status {
        VerificationStatus::Verified | VerificationStatus::Skipped => Ok(()),
        VerificationStatus::Reviewing => {
            let (round, max_rounds) = parse_round_info(&session.verification_metadata);
            Err(VerificationError::InProgress { round, max_rounds })
        }
        VerificationStatus::NeedsRevision => {
            let count = count_unresolved_gaps(&session.verification_metadata);
            Err(VerificationError::HasUnresolvedGaps { count })
        }
        VerificationStatus::Unverified => Err(VerificationError::NotVerified),
    }
}

/// Check if a proposal mutation is allowed given the session's plan verification state.
///
/// # Arguments
///
/// - `session` — the ideation session the proposal belongs to
/// - `settings` — project-level ideation settings
/// - `parent_verification_status` — verification status of the parent session (only relevant when
///   `session.inherited_plan_artifact_id` is set and `session.plan_artifact_id` is None).
///   Pass `None` when the parent session cannot be found — this triggers graceful degradation
///   (proposals are allowed to avoid orphaned child sessions).
/// - `operation` — the mutation being attempted (`Create`, `Update`, or `Delete`)
///
/// # Gate Logic
///
/// - Config bypass: `!settings.require_verification_for_proposals` → `Ok(())`
/// - No plan, no inherited plan → `Ok(())` (passthrough — nothing to verify)
/// - Own plan (`plan_artifact_id.is_some()`) → use `session.verification_status`
/// - Inherited plan only → use `parent_verification_status`; `None` (parent deleted) → `Ok(())`
/// - `Create`: blocks `Unverified`, `Reviewing`, `NeedsRevision`
/// - `Update | Delete`: blocks `Reviewing`, `NeedsRevision` only (allowing edits before verification starts)
///
/// # Errors
///
/// Returns a `VerificationError::Proposal*` variant when the gate blocks the operation.
pub fn check_proposal_verification_gate(
    session: &IdeationSession,
    settings: &IdeationSettings,
    parent_verification_status: Option<VerificationStatus>,
    operation: ProposalOperation,
) -> Result<(), VerificationError> {
    // Config bypass — gate is opt-in
    if !settings.require_verification_for_proposals {
        return Ok(());
    }

    // Determine which verification status to check
    let effective_status = if session.plan_artifact_id.is_some() {
        // Child with own plan → use own status
        session.verification_status
    } else if session.inherited_plan_artifact_id.is_some() {
        // Child with inherited plan → use parent status
        match parent_verification_status {
            // Parent deleted (FK ON DELETE SET NULL) → graceful degradation
            None => return Ok(()),
            Some(status) => status,
        }
    } else {
        // No plan at all → passthrough (nothing to verify)
        return Ok(());
    };

    let op_str = operation.to_string();

    match (operation, effective_status) {
        // Verified or Skipped always allow
        (_, VerificationStatus::Verified | VerificationStatus::Skipped) => Ok(()),

        // Create blocks Unverified
        (ProposalOperation::Create, VerificationStatus::Unverified) => {
            Err(VerificationError::ProposalNotVerified)
        }
        // Update/Delete allow Unverified (editing before review starts is fine)
        (ProposalOperation::Update | ProposalOperation::Delete, VerificationStatus::Unverified) => {
            Ok(())
        }

        // All operations block Reviewing
        (_, VerificationStatus::Reviewing) => {
            let (round, max_rounds) = parse_round_info(&session.verification_metadata);
            Err(VerificationError::ProposalReviewInProgress {
                operation: op_str,
                round,
                max_rounds,
            })
        }

        // All operations block NeedsRevision
        (_, VerificationStatus::NeedsRevision) => {
            let gap_count = count_unresolved_gaps(&session.verification_metadata) as usize;
            Err(VerificationError::ProposalHasUnresolvedGaps {
                operation: op_str,
                gap_count,
            })
        }
    }
}

fn parse_round_info(metadata_json: &Option<String>) -> (u32, u32) {
    metadata_json
        .as_deref()
        .and_then(|s| {
            serde_json::from_str::<crate::domain::entities::ideation::VerificationMetadata>(
                s,
            )
            .ok()
        })
        .map(|m| (m.current_round, m.max_rounds))
        .unwrap_or((0, 0))
}

fn count_unresolved_gaps(metadata_json: &Option<String>) -> u32 {
    metadata_json
        .as_deref()
        .and_then(|s| {
            serde_json::from_str::<crate::domain::entities::ideation::VerificationMetadata>(
                s,
            )
            .ok()
        })
        .map(|m| m.current_gaps.len() as u32)
        .unwrap_or(0)
}

#[cfg(test)]
#[path = "verification_gate_tests.rs"]
mod tests;
