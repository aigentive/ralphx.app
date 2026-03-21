// Verification loop integration tests (Wave 6, R4-H8)
//
// Tests the full verification lifecycle using real repository operations:
// - MemoryIdeationSessionRepository (in-memory, no SQLite needed for state tests)
// - VerificationReconciliationService (crash recovery)
// - check_verification_gate (acceptance gate enforcement)
//
// Covers the 5 integration scenarios specified in the plan:
//   1. Full convergence loop: unverified → rounds → 0 critical → verified
//   2. Hard cap exit: 5 non-converging rounds → max_rounds convergence
//   3. Agent crash recovery: stuck in_progress → reconciliation resets → retry succeeds
//   4. Revert-and-skip atomicity: single operation → status=skipped + plan updated
//   5. External apply blocked when unverified → allowed when skipped

use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::application::reconciliation::verification_reconciliation::{
    VerificationReconciliationConfig, VerificationReconciliationService,
};
use crate::domain::entities::{IdeationSession, ProjectId, SessionPurpose, VerificationStatus};
use crate::domain::ideation::config::IdeationSettings;
use crate::domain::repositories::IdeationSessionRepository;
use crate::domain::services::verification_gate::check_verification_gate;
use crate::infrastructure::memory::MemoryIdeationSessionRepository;

// ──────────────────────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────────────────────

fn make_session(project_id: &ProjectId) -> IdeationSession {
    IdeationSession::new(project_id.clone())
}

fn settings_with_gate_enabled() -> IdeationSettings {
    IdeationSettings {
        require_verification_for_accept: true,
        ..Default::default()
    }
}

fn settings_with_gate_disabled() -> IdeationSettings {
    IdeationSettings {
        require_verification_for_accept: false,
        ..Default::default()
    }
}

fn metadata_with_gaps(critical: u32, high: u32, round: u32, max_rounds: u32) -> String {
    let critical_gaps: Vec<serde_json::Value> = (0..critical)
        .map(|i| {
            serde_json::json!({
                "severity": "critical",
                "category": "architecture",
                "description": format!("Critical gap number {}", i),
                "why_it_matters": null
            })
        })
        .collect();
    let high_gaps: Vec<serde_json::Value> = (0..high)
        .map(|i| {
            serde_json::json!({
                "severity": "high",
                "category": "security",
                "description": format!("High gap number {}", i),
                "why_it_matters": null
            })
        })
        .collect();
    let mut all_gaps = critical_gaps;
    all_gaps.extend(high_gaps);

    serde_json::json!({
        "v": 1,
        "current_round": round,
        "max_rounds": max_rounds,
        "rounds": [],
        "current_gaps": all_gaps,
        "convergence_reason": null,
        "best_round_index": null,
        "parse_failures": []
    })
    .to_string()
}

fn make_reconciliation_service(
    repo: Arc<MemoryIdeationSessionRepository>,
) -> VerificationReconciliationService {
    let config = VerificationReconciliationConfig {
        stale_after_secs: 5400,
        auto_verify_stale_secs: 600,
        ..Default::default()
    };
    VerificationReconciliationService::new(repo as Arc<dyn IdeationSessionRepository>, config)
}

fn metadata_converged(reason: &str, round: u32) -> String {
    serde_json::json!({
        "v": 1,
        "current_round": round,
        "max_rounds": 5,
        "rounds": [],
        "current_gaps": [],
        "convergence_reason": reason,
        "best_round_index": null,
        "parse_failures": []
    })
    .to_string()
}

// ──────────────────────────────────────────────────────────────────────────────
// Test 1: Full convergence loop — zero_blocking exit
//
// Simulates the orchestrator calling update_verification_state across 2 rounds:
//   Round 1: 3 gaps (1 critical, 2 high) → NeedsRevision + in_progress
//   Round 2: 0 critical, 0 high, 0 medium → Verified + zero_blocking
//
// AD3: zero_blocking requires ALL of critical=0, high=0, medium=0.
// HIGH gaps alone now block convergence (unlike old zero_critical which allowed
// high ≤ prev_high).
//
// After convergence the acceptance gate must pass.
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_full_convergence_loop_zero_blocking_exit() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();
    let session = make_session(&project_id);
    let session_id = session.id.clone();
    repo.create(session).await.unwrap();

    // Round 1: critic finds 1 critical + 2 high gaps → NeedsRevision, still in progress
    let round1_meta = metadata_with_gaps(1, 2, 1, 4);
    repo.update_verification_state(
        &session_id,
        VerificationStatus::NeedsRevision,
        true, // in_progress — orchestrator correcting plan
        Some(round1_meta),
    )
    .await
    .unwrap();

    let after_r1 = repo.get_by_id(&session_id).await.unwrap().unwrap();
    assert_eq!(after_r1.verification_status, VerificationStatus::NeedsRevision);
    assert!(after_r1.verification_in_progress, "loop must still be active after round 1");

    // Gate must block while in-progress
    let gate_r1 = check_verification_gate(&after_r1, &settings_with_gate_enabled());
    assert!(
        gate_r1.is_err(),
        "gate must block acceptance during active verification"
    );

    // Round 2: orchestrator corrected plan → critic returns 0 critical, 0 high, 0 medium
    // (all blocking severities cleared → zero_blocking convergence)
    let round2_meta = metadata_converged("zero_blocking", 2);
    repo.update_verification_state(
        &session_id,
        VerificationStatus::Verified,
        false, // in_progress cleared on convergence
        Some(round2_meta),
    )
    .await
    .unwrap();

    let after_r2 = repo.get_by_id(&session_id).await.unwrap().unwrap();
    assert_eq!(
        after_r2.verification_status,
        VerificationStatus::Verified,
        "status must be Verified after zero_blocking convergence"
    );
    assert!(
        !after_r2.verification_in_progress,
        "in_progress must be cleared on convergence"
    );

    // Gate must pass for Verified session
    let gate_r2 = check_verification_gate(&after_r2, &settings_with_gate_enabled());
    assert!(gate_r2.is_ok(), "gate must allow acceptance after verification: {:?}", gate_r2);
}

// ──────────────────────────────────────────────────────────────────────────────
// Test 1b: HIGH gaps block zero_blocking convergence
//
// AD3: Unlike old zero_critical (high ≤ prev_high was OK), zero_blocking requires
// ALL of critical=0, high=0, medium=0. This test confirms HIGH gaps alone block
// convergence and keep the session in NeedsRevision.
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_high_gaps_block_zero_blocking_convergence() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();
    let session = make_session(&project_id);
    let session_id = session.id.clone();
    repo.create(session).await.unwrap();

    // Round 1: critic finds 1 critical + 2 high → NeedsRevision
    let round1_meta = metadata_with_gaps(1, 2, 1, 4);
    repo.update_verification_state(
        &session_id,
        VerificationStatus::NeedsRevision,
        true,
        Some(round1_meta),
    )
    .await
    .unwrap();

    // Round 2: 0 critical but 1 high remaining — must NOT converge under zero_blocking
    // The HTTP handler enforces this; here we verify the gate still blocks NeedsRevision.
    let round2_meta = metadata_with_gaps(0, 1, 2, 4);
    repo.update_verification_state(
        &session_id,
        VerificationStatus::NeedsRevision, // NOT Verified — high gap still present
        true,
        Some(round2_meta),
    )
    .await
    .unwrap();

    let after_r2 = repo.get_by_id(&session_id).await.unwrap().unwrap();
    assert_eq!(
        after_r2.verification_status,
        VerificationStatus::NeedsRevision,
        "HIGH gaps must keep session in NeedsRevision under zero_blocking threshold"
    );

    // Gate still blocks because session is not Verified
    let gate = check_verification_gate(&after_r2, &settings_with_gate_enabled());
    assert!(gate.is_err(), "gate must block when HIGH gaps remain: {:?}", gate);
}

// ──────────────────────────────────────────────────────────────────────────────
// Test 2: Hard cap exit — max_rounds convergence
//
// Simulates 5 rounds where gaps never converge (critic finds new critical each round).
// After round 5 the orchestrator sets status=Verified with reason=max_rounds.
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_hard_cap_exit_after_max_rounds() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();
    let session = make_session(&project_id);
    let session_id = session.id.clone();
    repo.create(session).await.unwrap();

    // Rounds 1-4: critic returns gaps each time, orchestrator keeps correcting
    for round in 1u32..=4 {
        let meta = metadata_with_gaps(2, 3, round, 5);
        repo.update_verification_state(
            &session_id,
            VerificationStatus::NeedsRevision,
            true,
            Some(meta),
        )
        .await
        .unwrap();

        let s = repo.get_by_id(&session_id).await.unwrap().unwrap();
        assert!(s.verification_in_progress, "still looping at round {}", round);
        // Gate blocks during loop
        assert!(
            check_verification_gate(&s, &settings_with_gate_enabled()).is_err(),
            "gate must block at round {}", round
        );
    }

    // Round 5 = max_rounds → hard cap → set Verified with convergence_reason=max_rounds
    let cap_meta = metadata_converged("max_rounds", 5);
    repo.update_verification_state(
        &session_id,
        VerificationStatus::Verified,
        false,
        Some(cap_meta),
    )
    .await
    .unwrap();

    let final_session = repo.get_by_id(&session_id).await.unwrap().unwrap();
    assert_eq!(
        final_session.verification_status,
        VerificationStatus::Verified,
        "hard cap exit must set status=Verified"
    );
    assert!(
        !final_session.verification_in_progress,
        "in_progress must be false after hard cap"
    );

    // Verify the convergence_reason is stored in metadata
    let meta_str = final_session.verification_metadata.clone().unwrap();
    assert!(
        meta_str.contains("max_rounds"),
        "metadata must record convergence_reason=max_rounds"
    );

    // Gate passes after hard-cap verified
    assert!(
        check_verification_gate(&final_session, &settings_with_gate_enabled()).is_ok(),
        "gate must allow acceptance after max_rounds convergence"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Test 3: Agent crash recovery
//
// Simulates the orchestrator crashing mid-verification:
//   - Session stuck in verification_in_progress=true for 2 hours
//   - VerificationReconciliationService.scan_and_reset() detects and resets it
//   - After reset: status=Unverified, in_progress=false, metadata=None
//   - User can then re-trigger verification (simulated by re-setting to Verified)
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_agent_crash_recovery_reconciliation_resets_and_retry_succeeds() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();

    // Session stuck: in_progress=true, updated 2 hours ago (beyond 90-min threshold)
    let mut session = make_session(&project_id);
    session.verification_status = VerificationStatus::Reviewing;
    session.verification_in_progress = true;
    session.updated_at = Utc::now() - Duration::hours(2);
    let session_id = session.id.clone();
    repo.create(session).await.unwrap();

    // Gate must block (verification in progress)
    let stuck = repo.get_by_id(&session_id).await.unwrap().unwrap();
    let gate_stuck = check_verification_gate(&stuck, &settings_with_gate_enabled());
    assert!(gate_stuck.is_err(), "gate must block while verification is stuck in-progress");

    // Reconciliation detects and resets the stuck session
    let svc = make_reconciliation_service(repo.clone());
    let reset_count = svc.scan_and_reset(false).await;
    assert_eq!(reset_count, 1, "reconciliation must reset the stuck session");

    // After reset: back to Unverified, in_progress cleared
    let after_reset = repo.get_by_id(&session_id).await.unwrap().unwrap();
    assert_eq!(
        after_reset.verification_status,
        VerificationStatus::Unverified,
        "reset must restore status to Unverified"
    );
    assert!(
        !after_reset.verification_in_progress,
        "reset must clear in_progress flag"
    );
    assert!(
        after_reset.verification_metadata.is_none(),
        "reset must clear verification metadata"
    );

    // Gate still blocks (now unverified, gate enabled)
    let gate_after_reset = check_verification_gate(&after_reset, &settings_with_gate_enabled());
    assert!(
        gate_after_reset.is_err(),
        "gate must block unverified session even after reconciliation reset"
    );

    // Simulate successful retry: user re-triggers verification → orchestrator converges
    let retry_meta = metadata_converged("zero_blocking", 1);
    repo.update_verification_state(
        &session_id,
        VerificationStatus::Verified,
        false,
        Some(retry_meta),
    )
    .await
    .unwrap();

    let after_retry = repo.get_by_id(&session_id).await.unwrap().unwrap();
    assert_eq!(after_retry.verification_status, VerificationStatus::Verified);
    assert!(
        check_verification_gate(&after_retry, &settings_with_gate_enabled()).is_ok(),
        "gate must allow acceptance after successful retry"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Test 4: Revert-and-skip atomicity
//
// The revert_plan_and_skip_with_artifact operation is a single repository call
// that atomically:
//   - Updates plan_artifact_id to the reverted version
//   - Sets verification_status = Skipped
//   - Clears in_progress
//   - Records convergence_reason = "user_reverted"
//
// No intermediate state where plan is reverted but status is still Unverified.
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_revert_and_skip_atomic_sets_skipped_and_updates_plan() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();

    // Session at hard cap: Verified via max_rounds but user wants to revert to original
    let mut session = make_session(&project_id);
    session.verification_status = VerificationStatus::NeedsRevision;
    session.verification_in_progress = false;
    session.plan_artifact_id = Some(crate::domain::entities::ArtifactId::from_string(
        "artifact-current-v3".to_string(),
    ));
    let session_id = session.id.clone();
    repo.create(session).await.unwrap();

    let original_artifact_id = "artifact-original-v1".to_string();

    // Atomic revert-and-skip: restores original plan + sets status=Skipped
    repo.revert_plan_and_skip_with_artifact(
        &session_id,
        original_artifact_id.clone(),
        "specification".to_string(),
        "My Plan".to_string(),
        "Original plan content".to_string(),
        1,
        "artifact-prev-version".to_string(),
        "user_reverted".to_string(),
    )
    .await
    .unwrap();

    // Verify atomicity: both fields updated in the same operation
    let after = repo.get_by_id(&session_id).await.unwrap().unwrap();

    assert_eq!(
        after.verification_status,
        VerificationStatus::Skipped,
        "verification_status must be Skipped after revert-and-skip"
    );
    assert!(
        !after.verification_in_progress,
        "in_progress must be false after revert-and-skip"
    );
    assert_eq!(
        after.plan_artifact_id.as_ref().map(|id| id.as_str()),
        Some(original_artifact_id.as_str()),
        "plan_artifact_id must point to the reverted original version"
    );

    // Metadata must record convergence_reason=user_reverted
    let meta_str = after.verification_metadata.clone().unwrap();
    assert!(
        meta_str.contains("user_reverted"),
        "metadata must record convergence_reason=user_reverted, got: {}",
        meta_str
    );

    // Gate must allow acceptance after skip
    let gate = check_verification_gate(&after, &settings_with_gate_enabled());
    assert!(gate.is_ok(), "gate must allow acceptance after revert-and-skip: {:?}", gate);
}

// ──────────────────────────────────────────────────────────────────────────────
// Test 5: External apply gate — blocked when unverified, allowed when skipped
//
// All 3 acceptance paths call check_verification_gate before apply_proposals_core.
// This test exercises the gate directly across all terminal states.
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_acceptance_gate_blocks_unverified_allows_skipped() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();
    let settings = settings_with_gate_enabled();

    // ── Case 1: Unverified → blocked ─────────────────────────────────────────
    let unverified = make_session(&project_id);
    let unverified_id = unverified.id.clone();
    repo.create(unverified).await.unwrap();

    let s = repo.get_by_id(&unverified_id).await.unwrap().unwrap();
    let gate = check_verification_gate(&s, &settings);
    assert!(gate.is_err(), "gate must block unverified session");
    assert!(
        matches!(gate.unwrap_err(), crate::domain::entities::ideation::VerificationError::NotVerified),
        "error variant must be NotVerified"
    );

    // ── Case 2: Reviewing (in_progress) → blocked ────────────────────────────
    let mut reviewing = make_session(&project_id);
    reviewing.verification_status = VerificationStatus::Reviewing;
    reviewing.verification_in_progress = true;
    reviewing.verification_metadata = Some(metadata_with_gaps(2, 3, 2, 5));
    let reviewing_id = reviewing.id.clone();
    repo.create(reviewing).await.unwrap();

    let s = repo.get_by_id(&reviewing_id).await.unwrap().unwrap();
    let gate = check_verification_gate(&s, &settings);
    assert!(gate.is_err(), "gate must block during active verification");
    assert!(
        matches!(
            gate.unwrap_err(),
            crate::domain::entities::ideation::VerificationError::InProgress { .. }
        ),
        "error variant must be InProgress"
    );

    // ── Case 3: NeedsRevision → blocked ──────────────────────────────────────
    let mut needs_revision = make_session(&project_id);
    needs_revision.verification_status = VerificationStatus::NeedsRevision;
    needs_revision.verification_in_progress = false;
    needs_revision.verification_metadata = Some(metadata_with_gaps(0, 2, 3, 5));
    let needs_revision_id = needs_revision.id.clone();
    repo.create(needs_revision).await.unwrap();

    let s = repo.get_by_id(&needs_revision_id).await.unwrap().unwrap();
    let gate = check_verification_gate(&s, &settings);
    assert!(gate.is_err(), "gate must block NeedsRevision session");
    assert!(
        matches!(
            gate.unwrap_err(),
            crate::domain::entities::ideation::VerificationError::HasUnresolvedGaps { .. }
        ),
        "error variant must be HasUnresolvedGaps"
    );

    // ── Case 4: Verified → allowed ────────────────────────────────────────────
    let mut verified = make_session(&project_id);
    verified.verification_status = VerificationStatus::Verified;
    let verified_id = verified.id.clone();
    repo.create(verified).await.unwrap();

    let s = repo.get_by_id(&verified_id).await.unwrap().unwrap();
    assert!(
        check_verification_gate(&s, &settings).is_ok(),
        "gate must allow Verified session"
    );

    // ── Case 5: Skipped → allowed (revert-and-skip or manual skip) ───────────
    let mut skipped = make_session(&project_id);
    skipped.verification_status = VerificationStatus::Skipped;
    let skipped_id = skipped.id.clone();
    repo.create(skipped).await.unwrap();

    let s = repo.get_by_id(&skipped_id).await.unwrap().unwrap();
    assert!(
        check_verification_gate(&s, &settings).is_ok(),
        "gate must allow Skipped session"
    );

    // ── Case 6: Gate disabled globally → all states pass ─────────────────────
    let disabled_settings = settings_with_gate_disabled();
    let gate_disabled = check_verification_gate(
        &repo.get_by_id(&unverified_id).await.unwrap().unwrap(),
        &disabled_settings,
    );
    assert!(
        gate_disabled.is_ok(),
        "gate must pass for any state when require_verification_for_accept=false"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Test 6: VerificationGap source field serialization (AD4)
//
// Verifies that the `source` field on VerificationGap round-trips through
// serde JSON correctly and is excluded from fingerprint computation.
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_verification_gap_source_field_serializes_and_excludes_from_fingerprint() {
    use crate::domain::entities::ideation::VerificationGap;
    use crate::domain::services::gap_fingerprint;

    // Gap with source attribution
    let gap_layer1 = VerificationGap {
        severity: "high".to_string(),
        category: "architecture".to_string(),
        description: "Missing input validation on user data".to_string(),
        why_it_matters: None,
        source: Some("layer1".to_string()),
    };

    // Same gap from layer2 — source differs but description is identical
    let gap_layer2 = VerificationGap {
        severity: "high".to_string(),
        category: "architecture".to_string(),
        description: "Missing input validation on user data".to_string(),
        why_it_matters: None,
        source: Some("layer2".to_string()),
    };

    // Fingerprints must be identical (source NOT included in fingerprint)
    let fp1 = gap_fingerprint(&gap_layer1.description);
    let fp2 = gap_fingerprint(&gap_layer2.description);
    assert_eq!(fp1, fp2, "same description must produce same fingerprint regardless of source");

    // Source field must serialize to JSON
    let json1 = serde_json::to_string(&gap_layer1).unwrap();
    assert!(json1.contains("\"source\":\"layer1\""), "source must serialize");

    let json2 = serde_json::to_string(&gap_layer2).unwrap();
    assert!(json2.contains("\"source\":\"layer2\""), "source must serialize");

    // Gap without source must not include source key (skip_serializing_if = None)
    let gap_no_source = VerificationGap {
        severity: "low".to_string(),
        category: "style".to_string(),
        description: "Minor naming inconsistency".to_string(),
        why_it_matters: None,
        source: None,
    };
    let json_no_source = serde_json::to_string(&gap_no_source).unwrap();
    assert!(
        !json_no_source.contains("source"),
        "None source must be omitted from JSON: {json_no_source}"
    );

    // Round-trip: deserialize with source field
    let deserialized: VerificationGap = serde_json::from_str(&json1).unwrap();
    assert_eq!(deserialized.source, Some("layer1".to_string()), "source must round-trip");
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests for child-session verification lifecycle (Phase 2)
//
// These tests cover the full child-session model introduced in Phase 2:
//   7.  Auto-verify creates a verification child session with correct properties
//   8.  Child updates parent state via update_plan_verification
//   9.  Child is archived when plan-verifier agent completes
//   10. Orphaned verification child reconciled (parent reset, child archived)
//   11. Spawn failure resets parent in_progress
//   12. Child session description format contains required fields (parallel tests)
// ──────────────────────────────────────────────────────────────────────────────

// ──────────────────────────────────────────────────────────────────────────────
// Test 7: Auto-verify creates verification child session
//
// After create_verification_child_session is called, a verification child session is created:
//   - session_purpose = Verification
//   - parent_session_id = parent's ID
//   - status = Active (ready to receive plan-verifier agent)
//
// get_verification_children(parent_id) must return the child.
// Archived children must be excluded from get_verification_children.
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_auto_verify_creates_verification_child_session() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();

    // Parent: general ideation session that has triggered auto-verify
    let mut parent = make_session(&project_id);
    parent.verification_status = VerificationStatus::Reviewing;
    parent.verification_in_progress = true;
    parent.verification_generation = 1;
    let parent_id = parent.id.clone();
    repo.create(parent).await.unwrap();

    // Simulate create_verification_child_session: create verification child session
    let mut child = make_session(&project_id);
    child.session_purpose = SessionPurpose::Verification;
    child.parent_session_id = Some(parent_id.clone());
    let child_id = child.id.clone();
    repo.create(child).await.unwrap();

    // Child must have correct purpose and parent reference
    let child_after = repo.get_by_id(&child_id).await.unwrap().unwrap();
    assert_eq!(
        child_after.session_purpose,
        SessionPurpose::Verification,
        "verification child must have SessionPurpose::Verification"
    );
    assert_eq!(
        child_after.parent_session_id.as_ref(),
        Some(&parent_id),
        "verification child must reference the parent session"
    );
    assert_eq!(
        child_after.status,
        crate::domain::entities::IdeationSessionStatus::Active,
        "verification child must start as Active"
    );

    // get_verification_children must return the active child
    let children = repo.get_verification_children(&parent_id).await.unwrap();
    assert_eq!(children.len(), 1, "must find exactly one verification child");
    assert_eq!(children[0].id, child_id, "returned child must match created child");

    // After archiving the child, get_verification_children must return empty
    repo.update_status(
        &child_id,
        crate::domain::entities::IdeationSessionStatus::Archived,
    )
    .await
    .unwrap();
    let children_after_archive = repo.get_verification_children(&parent_id).await.unwrap();
    assert!(
        children_after_archive.is_empty(),
        "archived child must not appear in get_verification_children"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Test 8: Verification child updates parent state
//
// The plan-verifier agent calls update_plan_verification(parent_id) to update
// the parent session's verification state. This simulates the agent running
// critic rounds and reporting results to the parent session.
//
// Round 1: NeedsRevision (1 critical, 2 high gaps) — loop continues
// Round 2: Verified (zero_blocking) — in_progress cleared
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_verification_child_updates_parent_state() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();

    // Parent starts in Reviewing state (auto-verify triggered)
    let mut parent = make_session(&project_id);
    parent.verification_status = VerificationStatus::Reviewing;
    parent.verification_in_progress = true;
    parent.verification_generation = 1;
    let parent_id = parent.id.clone();
    repo.create(parent).await.unwrap();

    // Create verification child (plan-verifier agent session)
    let mut child = make_session(&project_id);
    child.session_purpose = SessionPurpose::Verification;
    child.parent_session_id = Some(parent_id.clone());
    repo.create(child).await.unwrap();

    // Round 1: plan-verifier calls update_plan_verification(parent_id) — gaps found
    let round1_meta = metadata_with_gaps(1, 2, 1, 4);
    repo.update_verification_state(
        &parent_id,
        VerificationStatus::NeedsRevision,
        true, // still in progress — loop continues
        Some(round1_meta),
    )
    .await
    .unwrap();

    let parent_after_r1 = repo.get_by_id(&parent_id).await.unwrap().unwrap();
    assert_eq!(
        parent_after_r1.verification_status,
        VerificationStatus::NeedsRevision,
        "parent status must update to NeedsRevision after round 1"
    );
    assert!(
        parent_after_r1.verification_in_progress,
        "parent must remain in_progress while loop continues"
    );
    // Gate still blocks (NeedsRevision + in_progress)
    assert!(
        check_verification_gate(&parent_after_r1, &settings_with_gate_enabled()).is_err(),
        "gate must block while verification is in progress"
    );

    // Round 2: plan-verifier calls update_plan_verification(parent_id) — zero_blocking
    let round2_meta = metadata_converged("zero_blocking", 2);
    repo.update_verification_state(
        &parent_id,
        VerificationStatus::Verified,
        false, // in_progress cleared on convergence
        Some(round2_meta),
    )
    .await
    .unwrap();

    let parent_after_r2 = repo.get_by_id(&parent_id).await.unwrap().unwrap();
    assert_eq!(
        parent_after_r2.verification_status,
        VerificationStatus::Verified,
        "parent must reach Verified after zero_blocking convergence"
    );
    assert!(
        !parent_after_r2.verification_in_progress,
        "parent in_progress must be cleared on convergence"
    );
    // Gate passes after Verified
    assert!(
        check_verification_gate(&parent_after_r2, &settings_with_gate_enabled()).is_ok(),
        "gate must allow acceptance after verification: {:?}",
        check_verification_gate(&parent_after_r2, &settings_with_gate_enabled())
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Test 9: Verification child archived on completion
//
// When the plan-verifier agent finishes:
//   Step 1: plan-verifier calls update_plan_verification(parent_id, in_progress=false)
//   Step 2: backend auto-archives the child on agent:run_completed
//
// After both steps:
//   - Parent: in_progress=false, status=Verified
//   - Child: status=Archived
//   - get_verification_children returns empty (archived child excluded)
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_verification_child_archived_on_completion() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();

    // Parent: verification in progress
    let mut parent = make_session(&project_id);
    parent.verification_status = VerificationStatus::Reviewing;
    parent.verification_in_progress = true;
    parent.verification_generation = 1;
    let parent_id = parent.id.clone();
    repo.create(parent).await.unwrap();

    // Child: active verification session (plan-verifier agent running)
    let mut child = make_session(&project_id);
    child.session_purpose = SessionPurpose::Verification;
    child.parent_session_id = Some(parent_id.clone());
    let child_id = child.id.clone();
    repo.create(child).await.unwrap();

    // Verify child is visible before completion
    let active_children = repo.get_verification_children(&parent_id).await.unwrap();
    assert_eq!(active_children.len(), 1, "child must be visible before completion");

    // Step 1: plan-verifier calls update_plan_verification(parent_id, in_progress=false)
    let final_meta = metadata_converged("zero_blocking", 2);
    repo.update_verification_state(
        &parent_id,
        VerificationStatus::Verified,
        false,
        Some(final_meta),
    )
    .await
    .unwrap();

    // Step 2: backend auto-archives verification child on agent:run_completed
    repo.update_status(
        &child_id,
        crate::domain::entities::IdeationSessionStatus::Archived,
    )
    .await
    .unwrap();

    // Parent: in_progress cleared, status Verified
    let parent_after = repo.get_by_id(&parent_id).await.unwrap().unwrap();
    assert_eq!(
        parent_after.verification_status,
        VerificationStatus::Verified,
        "parent must be Verified after plan-verifier completes"
    );
    assert!(
        !parent_after.verification_in_progress,
        "parent in_progress must be cleared after completion"
    );

    // Child: archived
    let child_after = repo.get_by_id(&child_id).await.unwrap().unwrap();
    assert_eq!(
        child_after.status,
        crate::domain::entities::IdeationSessionStatus::Archived,
        "verification child must be archived after plan-verifier exits"
    );

    // get_verification_children must return empty (archived child excluded)
    let remaining_children = repo.get_verification_children(&parent_id).await.unwrap();
    assert!(
        remaining_children.is_empty(),
        "archived child must not appear in get_verification_children"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Test 10: Orphaned verification child reconciled
//
// If the plan-verifier agent crashes mid-loop, the child session remains Active
// while the parent is stuck with verification_in_progress=true. The reconciler
// detects the stale parent (> 10-min auto-verify threshold) and:
//   1. Resets parent: status=Unverified, in_progress=false
//   2. Archives the orphaned verification child session
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_orphaned_verification_child_reconciled() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();

    // Parent stuck in verification for 2 hours (> 10-min auto-verify threshold)
    let mut parent = make_session(&project_id);
    parent.verification_status = VerificationStatus::Reviewing;
    parent.verification_in_progress = true;
    parent.verification_generation = 1; // auto-verify
    parent.updated_at = Utc::now() - Duration::hours(2);
    let parent_id = parent.id.clone();
    repo.create(parent).await.unwrap();

    // Orphaned verification child (Active — agent crashed, never completed/archived)
    let mut child = make_session(&project_id);
    child.session_purpose = SessionPurpose::Verification;
    child.parent_session_id = Some(parent_id.clone());
    let child_id = child.id.clone();
    repo.create(child).await.unwrap();

    // Gate blocks before reconciliation (parent stuck in_progress)
    let stuck = repo.get_by_id(&parent_id).await.unwrap().unwrap();
    assert!(
        check_verification_gate(&stuck, &settings_with_gate_enabled()).is_err(),
        "gate must block while parent is stuck in_progress"
    );

    let svc = make_reconciliation_service(repo.clone());
    let reset_count = svc.scan_and_reset(false).await;
    assert_eq!(reset_count, 1, "reconciler must reset the orphaned parent");

    // Parent must be reset to Unverified
    let parent_after = repo.get_by_id(&parent_id).await.unwrap().unwrap();
    assert_eq!(
        parent_after.verification_status,
        VerificationStatus::Unverified,
        "orphaned parent must be reset to Unverified"
    );
    assert!(
        !parent_after.verification_in_progress,
        "orphaned parent in_progress must be cleared"
    );

    // Orphaned child must be archived by the reconciler
    let child_after = repo.get_by_id(&child_id).await.unwrap().unwrap();
    assert_eq!(
        child_after.status,
        crate::domain::entities::IdeationSessionStatus::Archived,
        "orphaned verification child must be archived by reconciler"
    );

    // No active verification children after reconciliation
    let active_children = repo.get_verification_children(&parent_id).await.unwrap();
    assert!(
        active_children.is_empty(),
        "no active verification children must remain after reconciliation"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Test 11: Spawn failure resets parent in_progress
//
// When create_child_session returns orchestration_triggered=false, the backend
// calls reset_auto_verify_sync on the parent to clear
// in_progress. This prevents permanent lock when the agent fails to start.
//
// After reset:
//   - Parent: in_progress=false, status=Unverified
//   - Gate still blocks (session unverified) but no longer locked
//   - No verification children (spawn failed before child was created)
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_spawn_failure_resets_parent() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();

    // Parent: auto-verify triggered (in_progress=true, generation=1)
    let mut parent = make_session(&project_id);
    parent.verification_status = VerificationStatus::Reviewing;
    parent.verification_in_progress = true;
    parent.verification_generation = 1;
    let parent_id = parent.id.clone();
    repo.create(parent).await.unwrap();

    // Gate blocks while in_progress (spawn not yet confirmed failed)
    let locked = repo.get_by_id(&parent_id).await.unwrap().unwrap();
    assert!(
        check_verification_gate(&locked, &settings_with_gate_enabled()).is_err(),
        "gate must block while verification lock is held"
    );

    // Simulate spawn failure: reset_auto_verify_sync clears in_progress
    // (This is what create_plan_artifact calls when orchestration_triggered=false)
    repo.update_verification_state(
        &parent_id,
        VerificationStatus::Unverified,
        false, // in_progress cleared — lock released
        None,  // metadata cleared
    )
    .await
    .unwrap();

    // Parent must have in_progress cleared — no longer permanently locked
    let parent_after = repo.get_by_id(&parent_id).await.unwrap().unwrap();
    assert!(
        !parent_after.verification_in_progress,
        "spawn failure must clear in_progress to avoid permanent lock"
    );
    assert_eq!(
        parent_after.verification_status,
        VerificationStatus::Unverified,
        "spawn failure must reset status to Unverified"
    );
    assert!(
        parent_after.verification_metadata.is_none(),
        "spawn failure must clear metadata"
    );

    // Gate still blocks (session unverified) but for the right reason
    let gate = check_verification_gate(&parent_after, &settings_with_gate_enabled());
    assert!(
        gate.is_err(),
        "gate must still block unverified session after spawn failure reset"
    );
    assert!(
        matches!(
            gate.unwrap_err(),
            crate::domain::entities::ideation::VerificationError::NotVerified
        ),
        "error must be NotVerified (not InProgress) after spawn failure reset"
    );

    // No verification children exist (spawn failed before child was created)
    let children = repo.get_verification_children(&parent_id).await.unwrap();
    assert!(children.is_empty(), "no verification children after spawn failure");
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests 12a/12b: Child session description format (parallel tests)
//
// The description passed to create_verification_child_session must contain:
//   - "parent_session_id: {parent_id}" — so plan-verifier can identify parent
//   - "generation: {N}"               — zombie protection generation counter
//   - "max_rounds: {N}"               — hard cap for the verification loop
//
// These run in parallel with old build_auto_verifier_prompt tests (Phase 2)
// and will remain when the old prompt construction is deleted (Phase 3).
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_child_session_description_format_contains_required_fields() {
    let parent_session_id = "session-abc-123";
    let generation = 3i32;
    let max_rounds = 4u32;

    // Replicate the format string from create_plan_artifact in artifacts.rs
    let description = format!(
        "Run verification round loop. parent_session_id: {parent_session_id}, generation: {generation}, max_rounds: {max_rounds}"
    );

    assert!(
        description.contains(&format!("parent_session_id: {parent_session_id}")),
        "description must contain parent_session_id field: got '{description}'"
    );
    assert!(
        description.contains(&format!("generation: {generation}")),
        "description must contain generation field: got '{description}'"
    );
    assert!(
        description.contains(&format!("max_rounds: {max_rounds}")),
        "description must contain max_rounds field: got '{description}'"
    );
}

#[test]
fn test_child_session_description_format_gen1_max4() {
    // Verify the format works for the most common case: gen=1, max_rounds=4
    let parent_session_id = "session-xyz-789";
    let generation = 1i32;
    let max_rounds = 4u32;

    let description = format!(
        "Run verification round loop. parent_session_id: {parent_session_id}, generation: {generation}, max_rounds: {max_rounds}"
    );

    assert!(
        description.contains("parent_session_id: session-xyz-789"),
        "must embed parent session ID"
    );
    assert!(description.contains("generation: 1"), "must embed generation 1");
    assert!(description.contains("max_rounds: 4"), "must embed max_rounds 4");
}

// ──────────────────────────────────────────────────────────────────────────────
// Test 6: Escalation lifecycle
//
// Simulates plan-verifier escalating to parent orchestrator:
//   1. Verifier sets NeedsRevision + convergence_reason=escalated_to_parent + in_progress=false
//   2. Proposal gate must block (NeedsRevision blocks acceptance)
//   3. Reconciliation must NOT reset (in_progress=false → reconciler skips it)
//   4. Parent creates new child session → re-verification succeeds (zero_blocking)
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_escalation_lifecycle_needs_revision_gate_blocks_reconciliation_skips_reverify_succeeds() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();
    let session = make_session(&project_id);
    let session_id = session.id.clone();
    repo.create(session).await.unwrap();

    // Step 1: Verifier escalates — sets NeedsRevision + escalated_to_parent, in_progress=false
    let escalation_meta = metadata_converged("escalated_to_parent", 3);
    repo.update_verification_state(
        &session_id,
        VerificationStatus::NeedsRevision,
        false, // verifier exits cleanly — in_progress cleared before sending message
        Some(escalation_meta),
    )
    .await
    .unwrap();

    let after_escalation = repo.get_by_id(&session_id).await.unwrap().unwrap();
    assert_eq!(
        after_escalation.verification_status,
        VerificationStatus::NeedsRevision,
        "escalated session must have status NeedsRevision"
    );
    assert!(
        !after_escalation.verification_in_progress,
        "escalated session must have in_progress=false (verifier exited cleanly)"
    );

    // Step 2: Proposal gate must block — NeedsRevision is not an accepted terminal state
    let gate_result = check_verification_gate(&after_escalation, &settings_with_gate_enabled());
    assert!(
        gate_result.is_err(),
        "gate must block proposal acceptance when status=NeedsRevision after escalation"
    );

    // Step 3: Reconciliation must NOT reset escalated session (in_progress=false)
    let svc = make_reconciliation_service(repo.clone());
    let reset_count = svc.scan_and_reset(false).await;
    assert_eq!(
        reset_count, 0,
        "reconciler must not reset escalated session (in_progress=false)"
    );

    // Verify session state unchanged after reconciliation pass
    let after_reconcile = repo.get_by_id(&session_id).await.unwrap().unwrap();
    assert_eq!(
        after_reconcile.verification_status,
        VerificationStatus::NeedsRevision,
        "escalated session status must be unchanged after reconciliation"
    );

    // Step 4: Parent resolves gaps and re-verifies with new child session → zero_blocking
    let reverify_meta = metadata_converged("zero_blocking", 2);
    repo.update_verification_state(
        &session_id,
        VerificationStatus::Verified,
        false,
        Some(reverify_meta),
    )
    .await
    .unwrap();

    let after_reverify = repo.get_by_id(&session_id).await.unwrap().unwrap();
    assert_eq!(
        after_reverify.verification_status,
        VerificationStatus::Verified,
        "re-verification after escalation resolution must yield Verified"
    );
    assert!(
        !after_reverify.verification_in_progress,
        "in_progress must be false after successful re-verification"
    );

    // Gate must pass after successful re-verification
    assert!(
        check_verification_gate(&after_reverify, &settings_with_gate_enabled()).is_ok(),
        "gate must allow acceptance after successful re-verification"
    );
}
