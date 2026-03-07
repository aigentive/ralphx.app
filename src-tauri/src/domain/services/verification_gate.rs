/// Verification gate — checks whether a session's plan is eligible for acceptance.
///
/// Called from all 3 acceptance paths: Tauri IPC, internal MCP HTTP, external MCP.
use crate::domain::entities::ideation::{VerificationError, VerificationStatus};
use crate::domain::entities::IdeationSession;
use crate::domain::ideation::config::IdeationSettings;

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
