use super::*;
use crate::domain::entities::ideation::{VerificationError, VerificationStatus};
use crate::domain::entities::{ArtifactId, IdeationSession, IdeationSessionId, ProjectId};
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
        require_verification_for_proposals: false,
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

// ============================================================================
// check_proposal_verification_gate() — 9 unit tests (Scenarios 1-9)
// ============================================================================

fn make_session_with_own_plan(status: VerificationStatus) -> IdeationSession {
    let mut session = IdeationSession::builder()
        .id(IdeationSessionId::from_string("own-plan-session".to_string()))
        .project_id(ProjectId::from_string("test-project-id".to_string()))
        .plan_artifact_id(ArtifactId::from_string("test-artifact-id".to_string()))
        .build();
    session.verification_status = status;
    session
}

fn make_no_plan_session() -> IdeationSession {
    IdeationSession::builder()
        .id(IdeationSessionId::from_string("no-plan-session".to_string()))
        .project_id(ProjectId::from_string("test-project-id".to_string()))
        .build()
}

fn make_inherited_plan_session() -> IdeationSession {
    // Child session: inherited_plan_artifact_id set, plan_artifact_id unset
    IdeationSession::builder()
        .id(IdeationSessionId::from_string("child-session".to_string()))
        .project_id(ProjectId::from_string("test-project-id".to_string()))
        .inherited_plan_artifact_id(ArtifactId::from_string("parent-artifact-id".to_string()))
        .build()
}

fn proposal_gate_settings(enabled: bool) -> IdeationSettings {
    IdeationSettings {
        plan_mode: IdeationPlanMode::Optional,
        require_plan_approval: false,
        suggest_plans_for_complex: false,
        auto_link_proposals: false,
        require_verification_for_accept: false,
        require_verification_for_proposals: enabled,
    }
}

/// Scenario 1: Config bypass — require_verification_for_proposals=false → all ops pass regardless of status.
#[test]
fn test_proposal_gate_config_bypass_allows_all() {
    let settings = proposal_gate_settings(false);
    for status in [
        VerificationStatus::Unverified,
        VerificationStatus::Reviewing,
        VerificationStatus::NeedsRevision,
        VerificationStatus::Verified,
        VerificationStatus::Skipped,
    ] {
        for op in [
            ProposalOperation::Create,
            ProposalOperation::Update,
            ProposalOperation::Delete,
        ] {
            let session = make_session_with_own_plan(status);
            let result = check_proposal_verification_gate(&session, &settings, None, op);
            assert!(
                result.is_ok(),
                "gate=false must bypass for status={:?} op={:?}",
                status,
                op
            );
        }
    }
}

/// Scenario 2: No-plan session passthrough — neither own nor inherited plan → Ok regardless.
#[test]
fn test_proposal_gate_no_plan_passthrough() {
    let settings = proposal_gate_settings(true);
    let session = make_no_plan_session();
    for op in [
        ProposalOperation::Create,
        ProposalOperation::Update,
        ProposalOperation::Delete,
    ] {
        assert!(
            check_proposal_verification_gate(&session, &settings, None, op).is_ok(),
            "no-plan session must passthrough for op={:?}",
            op
        );
    }
}

/// Scenario 3: VerificationStatus × ProposalOperation matrix — all 15 combinations.
///
/// Verified/Skipped → all ops allowed.
/// Unverified       → Create blocked (ProposalNotVerified), Update/Delete allowed.
/// Reviewing        → all ops blocked (ProposalReviewInProgress).
/// NeedsRevision    → all ops blocked (ProposalHasUnresolvedGaps).
#[test]
fn test_proposal_gate_status_operation_matrix() {
    let settings = proposal_gate_settings(true);

    // Verified and Skipped → always allow
    for status in [VerificationStatus::Verified, VerificationStatus::Skipped] {
        for op in [
            ProposalOperation::Create,
            ProposalOperation::Update,
            ProposalOperation::Delete,
        ] {
            let session = make_session_with_own_plan(status);
            assert!(
                check_proposal_verification_gate(&session, &settings, None, op).is_ok(),
                "status={:?} op={:?} should be Ok",
                status,
                op
            );
        }
    }

    // Unverified → Create blocked, Update/Delete allowed
    let session = make_session_with_own_plan(VerificationStatus::Unverified);
    assert!(matches!(
        check_proposal_verification_gate(&session, &settings, None, ProposalOperation::Create),
        Err(VerificationError::ProposalNotVerified)
    ));
    assert!(
        check_proposal_verification_gate(&session, &settings, None, ProposalOperation::Update)
            .is_ok()
    );
    assert!(
        check_proposal_verification_gate(&session, &settings, None, ProposalOperation::Delete)
            .is_ok()
    );

    // Reviewing → all blocked
    let session = make_session_with_own_plan(VerificationStatus::Reviewing);
    for op in [
        ProposalOperation::Create,
        ProposalOperation::Update,
        ProposalOperation::Delete,
    ] {
        assert!(
            matches!(
                check_proposal_verification_gate(&session, &settings, None, op),
                Err(VerificationError::ProposalReviewInProgress { .. })
            ),
            "Reviewing should block op={:?}",
            op
        );
    }

    // NeedsRevision → all blocked
    let session = make_session_with_own_plan(VerificationStatus::NeedsRevision);
    for op in [
        ProposalOperation::Create,
        ProposalOperation::Update,
        ProposalOperation::Delete,
    ] {
        assert!(
            matches!(
                check_proposal_verification_gate(&session, &settings, None, op),
                Err(VerificationError::ProposalHasUnresolvedGaps { .. })
            ),
            "NeedsRevision should block op={:?}",
            op
        );
    }
}

/// Scenario 7: Update and Delete explicitly allowed on Unverified (editing before review is fine).
#[test]
fn test_proposal_gate_update_delete_allowed_on_unverified() {
    let settings = proposal_gate_settings(true);
    let session = make_session_with_own_plan(VerificationStatus::Unverified);
    assert!(
        check_proposal_verification_gate(
            &session,
            &settings,
            None,
            ProposalOperation::Update
        )
        .is_ok(),
        "Update must be allowed when Unverified"
    );
    assert!(
        check_proposal_verification_gate(
            &session,
            &settings,
            None,
            ProposalOperation::Delete
        )
        .is_ok(),
        "Delete must be allowed when Unverified"
    );
}

/// Scenario 4: Child with inherited plan + parent Verified → all ops allowed.
#[test]
fn test_proposal_gate_inherited_plan_parent_verified_allows() {
    let settings = proposal_gate_settings(true);
    let session = make_inherited_plan_session();
    let parent_status = Some(VerificationStatus::Verified);
    for op in [
        ProposalOperation::Create,
        ProposalOperation::Update,
        ProposalOperation::Delete,
    ] {
        assert!(
            check_proposal_verification_gate(&session, &settings, parent_status, op).is_ok(),
            "Verified parent should allow op={:?}",
            op
        );
    }
}

/// Scenario 5: Child with inherited plan + parent Unverified → Create blocked, Update/Delete allowed.
#[test]
fn test_proposal_gate_inherited_plan_parent_unverified_blocks_create() {
    let settings = proposal_gate_settings(true);
    let session = make_inherited_plan_session();
    let parent_status = Some(VerificationStatus::Unverified);

    assert!(matches!(
        check_proposal_verification_gate(
            &session,
            &settings,
            parent_status,
            ProposalOperation::Create
        ),
        Err(VerificationError::ProposalNotVerified)
    ));
    assert!(
        check_proposal_verification_gate(
            &session,
            &settings,
            parent_status,
            ProposalOperation::Update
        )
        .is_ok()
    );
    assert!(
        check_proposal_verification_gate(
            &session,
            &settings,
            parent_status,
            ProposalOperation::Delete
        )
        .is_ok()
    );
}

/// Scenario 6: Child with own plan uses own verification status; parent status is irrelevant.
#[test]
fn test_proposal_gate_own_plan_uses_own_status_ignores_parent() {
    let settings = proposal_gate_settings(true);

    // Own plan Verified, parent Unverified → Create allowed (own status wins)
    let session = make_session_with_own_plan(VerificationStatus::Verified);
    let parent_unverified = Some(VerificationStatus::Unverified);
    assert!(
        check_proposal_verification_gate(
            &session,
            &settings,
            parent_unverified,
            ProposalOperation::Create
        )
        .is_ok(),
        "Own Verified plan should allow Create even if parent is Unverified"
    );

    // Own plan Unverified, parent Verified → Create blocked (own status wins)
    let session = make_session_with_own_plan(VerificationStatus::Unverified);
    let parent_verified = Some(VerificationStatus::Verified);
    assert!(
        matches!(
            check_proposal_verification_gate(
                &session,
                &settings,
                parent_verified,
                ProposalOperation::Create
            ),
            Err(VerificationError::ProposalNotVerified)
        ),
        "Own Unverified plan should block Create even if parent is Verified"
    );
}

/// Scenario 8: Child with inherited plan + deleted parent (None status) → graceful degradation → all ops allowed.
#[test]
fn test_proposal_gate_inherited_plan_deleted_parent_allows() {
    let settings = proposal_gate_settings(true);
    let session = make_inherited_plan_session();
    // None = parent was deleted (FK ON DELETE SET NULL → parent lookup returns None)
    for op in [
        ProposalOperation::Create,
        ProposalOperation::Update,
        ProposalOperation::Delete,
    ] {
        assert!(
            check_proposal_verification_gate(&session, &settings, None, op).is_ok(),
            "Deleted parent (None) should allow op={:?} via graceful degradation",
            op
        );
    }
}

/// Scenario 9: Child with inherited plan + parent NeedsRevision → all ops blocked (archived parent scenario).
#[test]
fn test_proposal_gate_inherited_plan_parent_needs_revision_blocks() {
    let settings = proposal_gate_settings(true);
    let session = make_inherited_plan_session();
    let parent_needs_revision = Some(VerificationStatus::NeedsRevision);
    for op in [
        ProposalOperation::Create,
        ProposalOperation::Update,
        ProposalOperation::Delete,
    ] {
        assert!(
            matches!(
                check_proposal_verification_gate(
                    &session,
                    &settings,
                    parent_needs_revision,
                    op
                ),
                Err(VerificationError::ProposalHasUnresolvedGaps { .. })
            ),
            "Parent NeedsRevision should block op={:?}",
            op
        );
    }
}

// ============================================================================
// check_verification_gate() — in_progress flag tests
// ============================================================================

/// Test: session with in_progress=true blocks with InProgress regardless of status field being Reviewing
#[test]
fn test_verification_gate_in_progress_blocks_regardless_of_status() {
    let settings = settings_with_required(true);
    let mut session = make_session(VerificationStatus::Reviewing);
    session.verification_in_progress = true;
    let result = check_verification_gate(&session, &settings);
    assert!(
        matches!(result, Err(VerificationError::InProgress { .. })),
        "in_progress=true with Reviewing status should return InProgress"
    );
}

/// Test: session with in_progress=true but status=Unverified still blocks with InProgress.
/// This is an inconsistent state (shouldn't happen in practice) but guards are ordered
/// to check in_progress first.
#[test]
fn test_verification_gate_in_progress_before_status_check() {
    let settings = settings_with_required(true);
    let mut session = make_session(VerificationStatus::Unverified);
    session.verification_in_progress = true;
    let result = check_verification_gate(&session, &settings);
    // in_progress=true should take priority over status=Unverified (which would normally → NotVerified)
    assert!(
        matches!(result, Err(VerificationError::InProgress { .. })),
        "in_progress=true must be checked before status, returning InProgress not NotVerified"
    );
}

/// Test: ImportedVerified sessions pass the acceptance gate (they are pre-verified by import).
#[test]
fn test_gate_allows_imported_verified() {
    let session = make_session(VerificationStatus::ImportedVerified);
    let settings = settings_with_required(true);
    assert!(
        check_verification_gate(&session, &settings).is_ok(),
        "ImportedVerified should pass the acceptance gate"
    );
}

/// Test: ImportedVerified passes all three proposal operations.
#[test]
fn test_proposal_gate_imported_verified_allows_all_operations() {
    let settings = proposal_gate_settings(true);
    let session = make_session_with_own_plan(VerificationStatus::ImportedVerified);
    for op in [
        ProposalOperation::Create,
        ProposalOperation::Update,
        ProposalOperation::Delete,
    ] {
        assert!(
            check_proposal_verification_gate(&session, &settings, None, op).is_ok(),
            "ImportedVerified should allow op={:?}",
            op
        );
    }
}

/// Test: Child session with inherited ImportedVerified parent allows all operations.
#[test]
fn test_proposal_gate_inherited_plan_parent_imported_verified_allows() {
    let settings = proposal_gate_settings(true);
    let session = make_inherited_plan_session();
    let parent_status = Some(VerificationStatus::ImportedVerified);
    for op in [
        ProposalOperation::Create,
        ProposalOperation::Update,
        ProposalOperation::Delete,
    ] {
        assert!(
            check_proposal_verification_gate(&session, &settings, parent_status, op).is_ok(),
            "ImportedVerified parent should allow op={:?}",
            op
        );
    }
}

/// Test: gate passes for ImportedVerified when gate is disabled (config bypass).
#[test]
fn test_gate_passes_imported_verified_when_not_required() {
    let settings = settings_with_required(false);
    let session = make_session(VerificationStatus::ImportedVerified);
    assert!(
        check_verification_gate(&session, &settings).is_ok(),
        "ImportedVerified must pass when require_verification=false"
    );
}

/// Test: session with in_progress=false and status=Reviewing falls through to status match.
#[test]
fn test_verification_gate_reviewing_status_without_in_progress() {
    let settings = settings_with_required(true);
    let mut session = make_session(VerificationStatus::Reviewing);
    session.verification_in_progress = false;
    let result = check_verification_gate(&session, &settings);
    // Should still be InProgress from the Reviewing match arm (defense-in-depth)
    assert!(
        matches!(result, Err(VerificationError::InProgress { .. })),
        "Reviewing status without in_progress flag should still return InProgress"
    );
}
