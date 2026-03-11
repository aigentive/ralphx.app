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
use crate::domain::entities::{IdeationSession, ProjectId, VerificationStatus};
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
// Test 1: Full convergence loop — 0 critical exit
//
// Simulates the orchestrator calling update_verification_state across 2 rounds:
//   Round 1: 3 gaps (1 critical, 2 high) → NeedsRevision + in_progress
//   Round 2: 0 critical, 1 high (count did not increase) → Verified + zero_critical
//
// After convergence the acceptance gate must pass.
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_full_convergence_loop_zero_critical_exit() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();
    let session = make_session(&project_id);
    let session_id = session.id.clone();
    repo.create(session).await.unwrap();

    // Round 1: critic finds 1 critical + 2 high gaps → NeedsRevision, still in progress
    let round1_meta = metadata_with_gaps(1, 2, 1, 5);
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

    // Round 2: orchestrator corrected plan → critic returns 0 critical, 1 high
    // (high_count 1 ≤ previous 2 → zero_critical convergence)
    let round2_meta = metadata_converged("zero_critical", 2);
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
        "status must be Verified after zero-critical convergence"
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
    let config = VerificationReconciliationConfig {
        stale_after_secs: 5400,      // 90 min
        auto_verify_stale_secs: 600, // 10 min
        interval_secs: 300,
    };
    let svc = VerificationReconciliationService::new(
        repo.clone() as Arc<dyn IdeationSessionRepository>,
        config,
    );
    let reset_count = svc.scan_and_reset().await;
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
    let retry_meta = metadata_converged("zero_critical", 1);
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
