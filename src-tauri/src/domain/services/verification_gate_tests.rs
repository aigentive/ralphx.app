use super::*;
use crate::domain::entities::ideation::{VerificationError, VerificationStatus};
use crate::domain::entities::{IdeationSession, IdeationSessionId, ProjectId};
use crate::domain::ideation::config::{IdeationPlanMode, IdeationSettings};

fn make_session(status: VerificationStatus) -> IdeationSession {
    IdeationSession::builder()
        .id(IdeationSessionId::from_string("test-session-id".to_string()))
        .project_id(ProjectId::from_string("test-project-id".to_string()))
        .build()
        // Override verification status after build
        .with_verification_status(status)
}

// We use a small helper to set verification status since builder doesn't expose it after build
impl IdeationSession {
    fn with_verification_status(mut self, status: VerificationStatus) -> Self {
        self.verification_status = status;
        self
    }
}

fn settings_with_required(required: bool) -> IdeationSettings {
    IdeationSettings {
        plan_mode: IdeationPlanMode::Optional,
        require_plan_approval: false,
        suggest_plans_for_complex: false,
        auto_link_proposals: false,
        require_verification_for_accept: required,
    }
}

#[test]
fn test_gate_blocks_unverified_when_required() {
    let session = make_session(VerificationStatus::Unverified);
    let settings = settings_with_required(true);
    let result = check_verification_gate(&session, &settings);
    assert!(matches!(result, Err(VerificationError::NotVerified)));
}

#[test]
fn test_gate_allows_verified() {
    let session = make_session(VerificationStatus::Verified);
    let settings = settings_with_required(true);
    assert!(check_verification_gate(&session, &settings).is_ok());
}

#[test]
fn test_gate_allows_skipped() {
    let session = make_session(VerificationStatus::Skipped);
    let settings = settings_with_required(true);
    assert!(check_verification_gate(&session, &settings).is_ok());
}

#[test]
fn test_gate_blocks_reviewing() {
    let session = make_session(VerificationStatus::Reviewing);
    let settings = settings_with_required(true);
    let result = check_verification_gate(&session, &settings);
    assert!(
        matches!(result, Err(VerificationError::InProgress { .. })),
        "reviewing should block with InProgress"
    );
}

#[test]
fn test_gate_blocks_needs_revision() {
    let session = make_session(VerificationStatus::NeedsRevision);
    let settings = settings_with_required(true);
    let result = check_verification_gate(&session, &settings);
    assert!(
        matches!(result, Err(VerificationError::HasUnresolvedGaps { .. })),
        "needs_revision should block with HasUnresolvedGaps"
    );
}

#[test]
fn test_gate_passes_for_any_status_when_not_required() {
    let settings = settings_with_required(false);
    for status in [
        VerificationStatus::Unverified,
        VerificationStatus::Reviewing,
        VerificationStatus::NeedsRevision,
        VerificationStatus::Verified,
        VerificationStatus::Skipped,
    ] {
        let session = make_session(status);
        assert!(
            check_verification_gate(&session, &settings).is_ok(),
            "gate should pass for {:?} when require_verification=false",
            status
        );
    }
}
