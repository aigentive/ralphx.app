use super::*;

use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::domain::entities::{IdeationSession, ProjectId, VerificationStatus};
use crate::domain::repositories::IdeationSessionRepository;
use crate::infrastructure::memory::MemoryIdeationSessionRepository;

fn make_service(
    repo: Arc<MemoryIdeationSessionRepository>,
    config: VerificationReconciliationConfig,
) -> VerificationReconciliationService {
    VerificationReconciliationService::new(
        repo as Arc<dyn IdeationSessionRepository>,
        config,
    )
}

fn default_config() -> VerificationReconciliationConfig {
    VerificationReconciliationConfig {
        stale_after_secs: 5400,      // 90 min
        auto_verify_stale_secs: 600, // 10 min
        interval_secs: 300,
    }
}

#[tokio::test]
async fn test_reconciliation_resets_stuck_session_after_timeout() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();

    // Session stuck in verification for 2 hours (> 90-min threshold)
    let mut session = IdeationSession::new(project_id);
    session.verification_status = VerificationStatus::Reviewing;
    session.verification_in_progress = true;
    session.updated_at = Utc::now() - Duration::hours(2);

    let session_id = session.id.clone();
    repo.create(session).await.unwrap();

    let svc = make_service(repo.clone(), default_config());
    let count = svc.scan_and_reset().await;

    assert_eq!(count, 1, "exactly one stuck session should be reset");

    let after = repo.get_by_id(&session_id).await.unwrap().unwrap();
    assert_eq!(
        after.verification_status,
        VerificationStatus::Unverified,
        "verification_status must be reset to Unverified"
    );
    assert!(
        !after.verification_in_progress,
        "verification_in_progress must be cleared"
    );
    assert!(
        after.verification_metadata.is_none(),
        "verification_metadata must be cleared"
    );
}

#[tokio::test]
async fn test_reconciliation_skips_session_under_timeout() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();

    // Session is only 30 min old — still within the 90-min window
    let mut session = IdeationSession::new(project_id);
    session.verification_status = VerificationStatus::Reviewing;
    session.verification_in_progress = true;
    session.updated_at = Utc::now() - Duration::minutes(30);

    let session_id = session.id.clone();
    repo.create(session).await.unwrap();

    let svc = make_service(repo.clone(), default_config());
    let count = svc.scan_and_reset().await;

    assert_eq!(count, 0, "no sessions should be reset when under timeout");

    let after = repo.get_by_id(&session_id).await.unwrap().unwrap();
    assert_eq!(
        after.verification_status,
        VerificationStatus::Reviewing,
        "status must remain unchanged for fresh sessions"
    );
    assert!(
        after.verification_in_progress,
        "in_progress flag must remain set for fresh sessions"
    );
}

#[tokio::test]
async fn test_reconciliation_ignores_sessions_not_in_progress() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();

    // Session is old but verification_in_progress=false — not stuck
    let mut session = IdeationSession::new(project_id);
    session.verification_status = VerificationStatus::Verified;
    session.verification_in_progress = false;
    session.updated_at = Utc::now() - Duration::hours(10);

    let session_id = session.id.clone();
    repo.create(session).await.unwrap();

    let svc = make_service(repo.clone(), default_config());
    let count = svc.scan_and_reset().await;

    assert_eq!(count, 0, "completed sessions must not be reset");

    let after = repo.get_by_id(&session_id).await.unwrap().unwrap();
    assert_eq!(after.verification_status, VerificationStatus::Verified);
}

#[tokio::test]
async fn test_reconciliation_resets_multiple_stuck_sessions() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();

    // 3 stuck sessions (> 90 min old)
    for _ in 0..3 {
        let mut session = IdeationSession::new(project_id.clone());
        session.verification_in_progress = true;
        session.updated_at = Utc::now() - Duration::hours(3);
        repo.create(session).await.unwrap();
    }

    // 1 fresh session (within timeout — must not be reset)
    let mut fresh = IdeationSession::new(project_id.clone());
    fresh.verification_in_progress = true;
    fresh.updated_at = Utc::now() - Duration::minutes(10);
    repo.create(fresh).await.unwrap();

    let svc = make_service(repo.clone(), default_config());
    let count = svc.scan_and_reset().await;

    assert_eq!(count, 3, "only the 3 stale sessions should be reset");
}

#[tokio::test]
async fn test_startup_scan_resets_stuck_sessions() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();

    let mut session = IdeationSession::new(project_id);
    session.verification_in_progress = true;
    session.updated_at = Utc::now() - Duration::hours(2);

    let session_id = session.id.clone();
    repo.create(session).await.unwrap();

    let svc = make_service(repo.clone(), default_config());
    svc.startup_scan().await;

    let after = repo.get_by_id(&session_id).await.unwrap().unwrap();
    assert!(!after.verification_in_progress);
    assert_eq!(after.verification_status, VerificationStatus::Unverified);
}

#[tokio::test]
async fn test_reconciliation_empty_repo_returns_zero() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());

    let svc = make_service(repo.clone(), default_config());
    let count = svc.scan_and_reset().await;

    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_reconciler_preserves_metadata() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();

    // Session stuck in verification for 2 hours with metadata
    let mut session = IdeationSession::new(project_id);
    session.verification_status = VerificationStatus::Reviewing;
    session.verification_in_progress = true;
    session.verification_metadata = Some(r#"{"current_round":2,"max_rounds":5,"current_gaps":[],"rounds":[],"best_round_index":null,"parse_failures":[],"convergence_reason":null}"#.to_string());
    session.updated_at = Utc::now() - Duration::hours(2);

    let session_id = session.id.clone();
    repo.create(session).await.unwrap();

    let svc = make_service(repo.clone(), default_config());
    let count = svc.scan_and_reset().await;

    assert_eq!(count, 1, "one stuck session should be reset");

    let after = repo.get_by_id(&session_id).await.unwrap().unwrap();
    assert_eq!(after.verification_status, VerificationStatus::Unverified);
    assert!(!after.verification_in_progress);
    // Metadata should be preserved (not cleared) so frontend can show what happened
    assert!(
        after.verification_metadata.is_some(),
        "verification_metadata must be preserved after reconciliation reset"
    );
}

#[tokio::test]
async fn test_reconciler_auto_verify_shorter_threshold() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();

    let config = VerificationReconciliationConfig {
        stale_after_secs: 5400,      // 90 min for manual
        auto_verify_stale_secs: 600, // 10 min for auto
        interval_secs: 300,
    };

    // Auto-verify session (generation > 0) stuck for 15 minutes — should be reset (> 10 min)
    let mut auto_session = IdeationSession::new(project_id.clone());
    auto_session.verification_status = VerificationStatus::Reviewing;
    auto_session.verification_in_progress = true;
    auto_session.verification_generation = 1;
    auto_session.updated_at = Utc::now() - Duration::minutes(15);
    let auto_id = auto_session.id.clone();
    repo.create(auto_session).await.unwrap();

    // Manual verify session (generation == 0) stuck for 15 minutes — should NOT be reset (< 90 min)
    let mut manual_session = IdeationSession::new(project_id.clone());
    manual_session.verification_status = VerificationStatus::Reviewing;
    manual_session.verification_in_progress = true;
    manual_session.verification_generation = 0;
    manual_session.updated_at = Utc::now() - Duration::minutes(15);
    let manual_id = manual_session.id.clone();
    repo.create(manual_session).await.unwrap();

    let svc = make_service(repo.clone(), config);
    let count = svc.scan_and_reset().await;

    assert_eq!(count, 1, "only the auto-verify session should be reset");

    let auto_after = repo.get_by_id(&auto_id).await.unwrap().unwrap();
    assert_eq!(
        auto_after.verification_status,
        VerificationStatus::Unverified,
        "auto-verify session must be reset"
    );

    let manual_after = repo.get_by_id(&manual_id).await.unwrap().unwrap();
    assert_eq!(
        manual_after.verification_status,
        VerificationStatus::Reviewing,
        "manual-verify session must NOT be reset (shorter than 90-min threshold)"
    );
}

/// ImportedVerified sessions must never be reset by the reconciler.
/// They appear in the stale-sessions query (in_progress=true, old enough) but should be
/// skipped because their pre-verified status must be preserved.
#[tokio::test]
async fn test_reconciler_skips_imported_verified_sessions() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();

    // ImportedVerified session that would otherwise be considered stale (> 90 min old, in_progress=true)
    let mut imported_session = IdeationSession::new(project_id.clone());
    imported_session.verification_status = VerificationStatus::ImportedVerified;
    imported_session.verification_in_progress = true;
    imported_session.updated_at = Utc::now() - Duration::hours(3);
    let imported_id = imported_session.id.clone();
    repo.create(imported_session).await.unwrap();

    // A normal Reviewing session that IS stale — should be reset
    let mut stuck_session = IdeationSession::new(project_id.clone());
    stuck_session.verification_status = VerificationStatus::Reviewing;
    stuck_session.verification_in_progress = true;
    stuck_session.updated_at = Utc::now() - Duration::hours(3);
    let stuck_id = stuck_session.id.clone();
    repo.create(stuck_session).await.unwrap();

    let svc = make_service(repo.clone(), default_config());
    let count = svc.scan_and_reset().await;

    // Only the Reviewing session should be reset; ImportedVerified is preserved
    assert_eq!(count, 1, "only the stuck Reviewing session should be reset");

    let imported_after = repo.get_by_id(&imported_id).await.unwrap().unwrap();
    assert_eq!(
        imported_after.verification_status,
        VerificationStatus::ImportedVerified,
        "ImportedVerified status must not be changed by reconciler"
    );

    let stuck_after = repo.get_by_id(&stuck_id).await.unwrap().unwrap();
    assert_eq!(
        stuck_after.verification_status,
        VerificationStatus::Unverified,
        "Stuck Reviewing session must be reset to Unverified"
    );
}

/// ImportedVerified-only repo: reconciler resets 0 sessions.
#[tokio::test]
async fn test_reconciler_only_imported_verified_resets_zero() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();

    // Two ImportedVerified sessions, both stale
    for _ in 0..2 {
        let mut session = IdeationSession::new(project_id.clone());
        session.verification_status = VerificationStatus::ImportedVerified;
        session.verification_in_progress = true;
        session.updated_at = Utc::now() - Duration::hours(5);
        repo.create(session).await.unwrap();
    }

    let svc = make_service(repo.clone(), default_config());
    let count = svc.scan_and_reset().await;

    assert_eq!(count, 0, "no ImportedVerified sessions should be reset");
}

#[tokio::test]
async fn test_orphaned_verification_child_reconciled() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();

    // Parent session stuck in verification for 2 hours (> 90-min threshold)
    let mut parent = IdeationSession::new(project_id.clone());
    parent.verification_status = VerificationStatus::Reviewing;
    parent.verification_in_progress = true;
    parent.verification_generation = 1; // auto-verify
    parent.updated_at = Utc::now() - Duration::hours(2);
    let parent_id = parent.id.clone();
    repo.create(parent).await.unwrap();

    // Orphaned verification child session (not archived)
    let mut child = IdeationSession::new(project_id.clone());
    child.session_purpose = crate::domain::entities::ideation::SessionPurpose::Verification;
    child.parent_session_id = Some(parent_id.clone());
    child.updated_at = Utc::now() - Duration::hours(2);
    let child_id = child.id.clone();
    repo.create(child).await.unwrap();

    let svc = make_service(repo.clone(), default_config());
    let count = svc.scan_and_reset().await;

    assert_eq!(count, 1, "parent session should be reset");

    // Parent should be reset
    let parent_after = repo.get_by_id(&parent_id).await.unwrap().unwrap();
    assert_eq!(parent_after.verification_status, VerificationStatus::Unverified);
    assert!(!parent_after.verification_in_progress);

    // Child should be archived
    let child_after = repo.get_by_id(&child_id).await.unwrap().unwrap();
    assert_eq!(
        child_after.status,
        crate::domain::entities::IdeationSessionStatus::Archived,
        "orphaned verification child must be archived by reconciler"
    );
}

#[tokio::test]
async fn test_reconciler_manual_session_reset_after_long_threshold() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();

    let config = VerificationReconciliationConfig {
        stale_after_secs: 5400,      // 90 min
        auto_verify_stale_secs: 600, // 10 min
        interval_secs: 300,
    };

    // Manual verify session stuck for 2 hours — should be reset (> 90 min)
    let mut session = IdeationSession::new(project_id);
    session.verification_status = VerificationStatus::Reviewing;
    session.verification_in_progress = true;
    session.verification_generation = 0;
    session.updated_at = Utc::now() - Duration::hours(2);
    let session_id = session.id.clone();
    repo.create(session).await.unwrap();

    let svc = make_service(repo.clone(), config);
    let count = svc.scan_and_reset().await;

    assert_eq!(count, 1, "manual session stuck > 90 min should be reset");

    let after = repo.get_by_id(&session_id).await.unwrap().unwrap();
    assert_eq!(after.verification_status, VerificationStatus::Unverified);
}

// ---------------------------------------------------------------------------
// reconcile_verification_on_child_complete tests
// ---------------------------------------------------------------------------

/// Helper: create a linked parent+child pair in the repo.
/// Parent has verification_in_progress=true; child has session_purpose=Verification.
async fn make_parent_child_pair(
    repo: &Arc<crate::infrastructure::memory::MemoryIdeationSessionRepository>,
    parent_metadata: Option<String>,
) -> (
    crate::domain::entities::IdeationSessionId,
    crate::domain::entities::IdeationSessionId,
) {
    let project_id = ProjectId::new();

    let mut parent = IdeationSession::new(project_id.clone());
    parent.verification_status = VerificationStatus::Reviewing;
    parent.verification_in_progress = true;
    parent.verification_metadata = parent_metadata;
    let parent_id = parent.id.clone();
    repo.create(parent).await.unwrap();

    let mut child = IdeationSession::new(project_id);
    child.session_purpose = crate::domain::entities::SessionPurpose::Verification;
    child.parent_session_id = Some(parent_id.clone());
    let child_id = child.id.clone();
    repo.create(child).await.unwrap();

    (parent_id, child_id)
}

#[tokio::test]
async fn test_reconcile_child_complete_convergence_zero_blocking() {
    let repo = Arc::new(crate::infrastructure::memory::MemoryIdeationSessionRepository::new());
    let dyn_repo: Arc<dyn crate::domain::repositories::IdeationSessionRepository> =
        repo.clone();

    let metadata_json = r#"{"v":1,"current_round":2,"max_rounds":5,"rounds":[{"fingerprints":[],"gap_score":0}],"current_gaps":[],"convergence_reason":"zero_blocking","best_round_index":null,"parse_failures":[]}"#;
    let (parent_id, child_id) =
        make_parent_child_pair(&repo, Some(metadata_json.to_string())).await;

    reconcile_verification_on_child_complete::<tauri::Wry>(
        &parent_id,
        &child_id,
        &dyn_repo,
        None,
    )
    .await;

    let parent_after = repo.get_by_id(&parent_id).await.unwrap().unwrap();
    assert_eq!(
        parent_after.verification_status,
        VerificationStatus::Verified,
        "zero_blocking convergence should map to Verified"
    );
    assert!(
        !parent_after.verification_in_progress,
        "in_progress must be cleared after reconciliation"
    );

    let child_after = repo.get_by_id(&child_id).await.unwrap().unwrap();
    assert_eq!(
        child_after.status,
        crate::domain::entities::IdeationSessionStatus::Archived,
        "child must be archived after reconciliation"
    );
}

#[tokio::test]
async fn test_reconcile_child_complete_convergence_jaccard_converged() {
    let repo = Arc::new(crate::infrastructure::memory::MemoryIdeationSessionRepository::new());
    let dyn_repo: Arc<dyn crate::domain::repositories::IdeationSessionRepository> =
        repo.clone();

    let metadata_json = r#"{"v":1,"current_round":3,"max_rounds":5,"rounds":[],"current_gaps":[],"convergence_reason":"jaccard_converged","best_round_index":null,"parse_failures":[]}"#;
    let (parent_id, child_id) =
        make_parent_child_pair(&repo, Some(metadata_json.to_string())).await;

    reconcile_verification_on_child_complete::<tauri::Wry>(
        &parent_id,
        &child_id,
        &dyn_repo,
        None,
    )
    .await;

    let parent_after = repo.get_by_id(&parent_id).await.unwrap().unwrap();
    assert_eq!(parent_after.verification_status, VerificationStatus::Verified);
    assert!(!parent_after.verification_in_progress);
}

#[tokio::test]
async fn test_reconcile_child_complete_max_rounds_needs_revision() {
    let repo = Arc::new(crate::infrastructure::memory::MemoryIdeationSessionRepository::new());
    let dyn_repo: Arc<dyn crate::domain::repositories::IdeationSessionRepository> =
        repo.clone();

    let metadata_json = r#"{"v":1,"current_round":5,"max_rounds":5,"rounds":[],"current_gaps":[],"convergence_reason":"max_rounds","best_round_index":null,"parse_failures":[]}"#;
    let (parent_id, child_id) =
        make_parent_child_pair(&repo, Some(metadata_json.to_string())).await;

    reconcile_verification_on_child_complete::<tauri::Wry>(
        &parent_id,
        &child_id,
        &dyn_repo,
        None,
    )
    .await;

    let parent_after = repo.get_by_id(&parent_id).await.unwrap().unwrap();
    assert_eq!(
        parent_after.verification_status,
        VerificationStatus::NeedsRevision,
        "max_rounds convergence should map to NeedsRevision"
    );
    assert!(!parent_after.verification_in_progress);

    let child_after = repo.get_by_id(&child_id).await.unwrap().unwrap();
    assert_eq!(
        child_after.status,
        crate::domain::entities::IdeationSessionStatus::Archived
    );
}

#[tokio::test]
async fn test_reconcile_child_complete_crashed_mid_round() {
    let repo = Arc::new(crate::infrastructure::memory::MemoryIdeationSessionRepository::new());
    let dyn_repo: Arc<dyn crate::domain::repositories::IdeationSessionRepository> =
        repo.clone();

    // Has rounds but no convergence_reason — agent crashed mid-round
    let metadata_json = r#"{"v":1,"current_round":2,"max_rounds":5,"rounds":[{"fingerprints":[],"gap_score":5}],"current_gaps":[],"convergence_reason":null,"best_round_index":null,"parse_failures":[]}"#;
    let (parent_id, child_id) =
        make_parent_child_pair(&repo, Some(metadata_json.to_string())).await;

    reconcile_verification_on_child_complete::<tauri::Wry>(
        &parent_id,
        &child_id,
        &dyn_repo,
        None,
    )
    .await;

    let parent_after = repo.get_by_id(&parent_id).await.unwrap().unwrap();
    assert_eq!(
        parent_after.verification_status,
        VerificationStatus::NeedsRevision,
        "crashed mid-round should result in NeedsRevision"
    );
    assert!(!parent_after.verification_in_progress);

    // Verify convergence_reason was set to agent_crashed_mid_round in metadata
    let meta: serde_json::Value =
        serde_json::from_str(parent_after.verification_metadata.as_deref().unwrap()).unwrap();
    assert_eq!(
        meta["convergence_reason"].as_str().unwrap(),
        "agent_crashed_mid_round"
    );

    let child_after = repo.get_by_id(&child_id).await.unwrap().unwrap();
    assert_eq!(
        child_after.status,
        crate::domain::entities::IdeationSessionStatus::Archived
    );
}

#[tokio::test]
async fn test_reconcile_child_complete_no_metadata() {
    let repo = Arc::new(crate::infrastructure::memory::MemoryIdeationSessionRepository::new());
    let dyn_repo: Arc<dyn crate::domain::repositories::IdeationSessionRepository> =
        repo.clone();

    // No metadata at all — agent completed without any updates
    let (parent_id, child_id) = make_parent_child_pair(&repo, None).await;

    reconcile_verification_on_child_complete::<tauri::Wry>(
        &parent_id,
        &child_id,
        &dyn_repo,
        None,
    )
    .await;

    let parent_after = repo.get_by_id(&parent_id).await.unwrap().unwrap();
    assert_eq!(
        parent_after.verification_status,
        VerificationStatus::Unverified,
        "no metadata should result in Unverified"
    );
    assert!(!parent_after.verification_in_progress);

    let child_after = repo.get_by_id(&child_id).await.unwrap().unwrap();
    assert_eq!(
        child_after.status,
        crate::domain::entities::IdeationSessionStatus::Archived
    );
}

#[tokio::test]
async fn test_reconcile_child_complete_malformed_metadata() {
    let repo = Arc::new(crate::infrastructure::memory::MemoryIdeationSessionRepository::new());
    let dyn_repo: Arc<dyn crate::domain::repositories::IdeationSessionRepository> =
        repo.clone();

    // Malformed JSON — parse fails, treated as None → Unverified
    let (parent_id, child_id) =
        make_parent_child_pair(&repo, Some("{invalid json".to_string())).await;

    reconcile_verification_on_child_complete::<tauri::Wry>(
        &parent_id,
        &child_id,
        &dyn_repo,
        None,
    )
    .await;

    let parent_after = repo.get_by_id(&parent_id).await.unwrap().unwrap();
    assert_eq!(
        parent_after.verification_status,
        VerificationStatus::Unverified,
        "malformed metadata should be treated as None → Unverified"
    );
    assert!(!parent_after.verification_in_progress);
}

#[tokio::test]
async fn test_reconcile_child_complete_imported_verified_guard() {
    let repo = Arc::new(crate::infrastructure::memory::MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();
    let dyn_repo: Arc<dyn crate::domain::repositories::IdeationSessionRepository> =
        repo.clone();

    // Parent is ImportedVerified — must not be reconciled
    let mut parent = IdeationSession::new(project_id.clone());
    parent.verification_status = VerificationStatus::ImportedVerified;
    parent.verification_in_progress = true;
    let parent_id = parent.id.clone();
    repo.create(parent).await.unwrap();

    let mut child = IdeationSession::new(project_id);
    child.session_purpose = crate::domain::entities::SessionPurpose::Verification;
    child.parent_session_id = Some(parent_id.clone());
    let child_id = child.id.clone();
    repo.create(child).await.unwrap();

    reconcile_verification_on_child_complete::<tauri::Wry>(
        &parent_id,
        &child_id,
        &dyn_repo,
        None,
    )
    .await;

    let parent_after = repo.get_by_id(&parent_id).await.unwrap().unwrap();
    assert_eq!(
        parent_after.verification_status,
        VerificationStatus::ImportedVerified,
        "ImportedVerified status must not be changed by reconciliation"
    );
    // in_progress left untouched (guard returns before updating)
    assert!(parent_after.verification_in_progress);
}

#[tokio::test]
async fn test_reconcile_child_complete_orphan_sibling_archived() {
    let repo = Arc::new(crate::infrastructure::memory::MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();
    let dyn_repo: Arc<dyn crate::domain::repositories::IdeationSessionRepository> =
        repo.clone();

    let mut parent = IdeationSession::new(project_id.clone());
    parent.verification_status = VerificationStatus::Reviewing;
    parent.verification_in_progress = true;
    let parent_id = parent.id.clone();
    repo.create(parent).await.unwrap();

    // Primary child (the one completing)
    let mut child1 = IdeationSession::new(project_id.clone());
    child1.session_purpose = crate::domain::entities::SessionPurpose::Verification;
    child1.parent_session_id = Some(parent_id.clone());
    let child1_id = child1.id.clone();
    repo.create(child1).await.unwrap();

    // Orphan sibling child (should be archived by orphan cleanup)
    let mut child2 = IdeationSession::new(project_id);
    child2.session_purpose = crate::domain::entities::SessionPurpose::Verification;
    child2.parent_session_id = Some(parent_id.clone());
    let child2_id = child2.id.clone();
    repo.create(child2).await.unwrap();

    reconcile_verification_on_child_complete::<tauri::Wry>(
        &parent_id,
        &child1_id,
        &dyn_repo,
        None,
    )
    .await;

    let child1_after = repo.get_by_id(&child1_id).await.unwrap().unwrap();
    assert_eq!(
        child1_after.status,
        crate::domain::entities::IdeationSessionStatus::Archived,
        "primary child must be archived"
    );

    let child2_after = repo.get_by_id(&child2_id).await.unwrap().unwrap();
    assert_eq!(
        child2_after.status,
        crate::domain::entities::IdeationSessionStatus::Archived,
        "orphan sibling child must be archived by orphan cleanup"
    );
}

// ---------------------------------------------------------------------------
// startup_reset_all_in_progress tests
// ---------------------------------------------------------------------------

/// All in-progress sessions must be reset regardless of updated_at timestamp.
/// This is the key difference from startup_scan() which uses TTL thresholds.
#[tokio::test]
async fn test_startup_reset_all_in_progress_ignores_ttl() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();

    // 5-minute-old auto-verify session — would NOT be reset by startup_scan (< 10-min auto threshold)
    let mut recent_auto = IdeationSession::new(project_id.clone());
    recent_auto.verification_status = VerificationStatus::Reviewing;
    recent_auto.verification_in_progress = true;
    recent_auto.verification_generation = 1; // auto-verify
    recent_auto.updated_at = Utc::now() - Duration::minutes(5);
    let recent_auto_id = recent_auto.id.clone();
    repo.create(recent_auto).await.unwrap();

    // 30-minute-old manual-verify session — would NOT be reset by startup_scan (< 90-min manual threshold)
    let mut mid_manual = IdeationSession::new(project_id.clone());
    mid_manual.verification_status = VerificationStatus::Reviewing;
    mid_manual.verification_in_progress = true;
    mid_manual.verification_generation = 0; // manual verify
    mid_manual.updated_at = Utc::now() - Duration::minutes(30);
    let mid_manual_id = mid_manual.id.clone();
    repo.create(mid_manual).await.unwrap();

    // 2-hour-old session — would also be reset by startup_scan
    let mut old_session = IdeationSession::new(project_id.clone());
    old_session.verification_status = VerificationStatus::Reviewing;
    old_session.verification_in_progress = true;
    old_session.updated_at = Utc::now() - Duration::hours(2);
    let old_id = old_session.id.clone();
    repo.create(old_session).await.unwrap();

    let svc = make_service(repo.clone(), default_config());
    svc.startup_reset_all_in_progress().await;

    // ALL three sessions must be reset — startup_reset_all_in_progress ignores TTL
    for (id, label) in [
        (&recent_auto_id, "recent auto-verify (5 min)"),
        (&mid_manual_id, "mid manual-verify (30 min)"),
        (&old_id, "old session (2 hours)"),
    ] {
        let after = repo.get_by_id(id).await.unwrap().unwrap();
        assert_eq!(
            after.verification_status,
            VerificationStatus::Unverified,
            "{} must be reset to Unverified",
            label
        );
        assert!(
            !after.verification_in_progress,
            "{} in_progress must be cleared",
            label
        );
        // Metadata must contain app_restart convergence_reason
        let meta: serde_json::Value =
            serde_json::from_str(after.verification_metadata.as_deref().unwrap()).unwrap();
        assert_eq!(
            meta["convergence_reason"].as_str().unwrap(),
            "app_restart",
            "{} convergence_reason must be app_restart",
            label
        );
    }
}

/// startup_reset_all_in_progress must preserve ImportedVerified status.
#[tokio::test]
async fn test_startup_reset_skips_imported_verified() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();

    // ImportedVerified session with in_progress=true (should not be touched)
    let mut imported = IdeationSession::new(project_id.clone());
    imported.verification_status = VerificationStatus::ImportedVerified;
    imported.verification_in_progress = true;
    imported.updated_at = Utc::now() - Duration::minutes(5);
    let imported_id = imported.id.clone();
    repo.create(imported).await.unwrap();

    // Normal in-progress session (should be reset)
    let mut normal = IdeationSession::new(project_id.clone());
    normal.verification_status = VerificationStatus::Reviewing;
    normal.verification_in_progress = true;
    normal.updated_at = Utc::now() - Duration::minutes(5);
    let normal_id = normal.id.clone();
    repo.create(normal).await.unwrap();

    let svc = make_service(repo.clone(), default_config());
    svc.startup_reset_all_in_progress().await;

    // ImportedVerified must be unchanged
    let imported_after = repo.get_by_id(&imported_id).await.unwrap().unwrap();
    assert_eq!(
        imported_after.verification_status,
        VerificationStatus::ImportedVerified,
        "ImportedVerified must not be changed by startup reset"
    );
    assert!(
        imported_after.verification_in_progress,
        "ImportedVerified in_progress must remain set"
    );

    // Normal session must be reset
    let normal_after = repo.get_by_id(&normal_id).await.unwrap().unwrap();
    assert_eq!(normal_after.verification_status, VerificationStatus::Unverified);
    assert!(!normal_after.verification_in_progress);
}

/// Orphaned verification children linked to reset parents must be archived.
#[tokio::test]
async fn test_startup_reset_archives_orphaned_children() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();

    // Parent with in-progress verification (5 minutes old — below any TTL threshold)
    let mut parent = IdeationSession::new(project_id.clone());
    parent.verification_status = VerificationStatus::Reviewing;
    parent.verification_in_progress = true;
    parent.updated_at = Utc::now() - Duration::minutes(5);
    let parent_id = parent.id.clone();
    repo.create(parent).await.unwrap();

    // Orphaned verification child session
    let mut child = IdeationSession::new(project_id.clone());
    child.session_purpose = crate::domain::entities::SessionPurpose::Verification;
    child.parent_session_id = Some(parent_id.clone());
    let child_id = child.id.clone();
    repo.create(child).await.unwrap();

    let svc = make_service(repo.clone(), default_config());
    svc.startup_reset_all_in_progress().await;

    // Parent must be reset
    let parent_after = repo.get_by_id(&parent_id).await.unwrap().unwrap();
    assert_eq!(parent_after.verification_status, VerificationStatus::Unverified);
    assert!(!parent_after.verification_in_progress);

    // Child must be archived
    let child_after = repo.get_by_id(&child_id).await.unwrap().unwrap();
    assert_eq!(
        child_after.status,
        crate::domain::entities::IdeationSessionStatus::Archived,
        "orphaned verification child must be archived during startup reset"
    );
}

/// Empty repo: startup_reset_all_in_progress is a no-op.
#[tokio::test]
async fn test_startup_reset_empty_repo_is_noop() {
    let repo = Arc::new(MemoryIdeationSessionRepository::new());
    let svc = make_service(repo.clone(), default_config());
    // Should not panic
    svc.startup_reset_all_in_progress().await;
}

// ---------------------------------------------------------------------------
// reset_verification_on_child_error tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_reset_verification_on_child_error_agent_error() {
    let repo = Arc::new(crate::infrastructure::memory::MemoryIdeationSessionRepository::new());
    let dyn_repo: Arc<dyn crate::domain::repositories::IdeationSessionRepository> =
        repo.clone();

    let (parent_id, child_id) = make_parent_child_pair(&repo, None).await;

    reset_verification_on_child_error::<tauri::Wry>(&child_id, &dyn_repo, None, "agent_error")
        .await;

    let parent_after = repo.get_by_id(&parent_id).await.unwrap().unwrap();
    assert_eq!(
        parent_after.verification_status,
        VerificationStatus::Unverified,
        "agent error should reset parent to Unverified"
    );
    assert!(
        !parent_after.verification_in_progress,
        "in_progress must be cleared after error reset"
    );

    // Metadata should contain the agent_error convergence_reason
    let meta: serde_json::Value =
        serde_json::from_str(parent_after.verification_metadata.as_deref().unwrap()).unwrap();
    assert_eq!(meta["convergence_reason"].as_str().unwrap(), "agent_error");

    let child_after = repo.get_by_id(&child_id).await.unwrap().unwrap();
    assert_eq!(
        child_after.status,
        crate::domain::entities::IdeationSessionStatus::Archived,
        "child must be archived after error reset"
    );
}

#[tokio::test]
async fn test_reset_verification_on_child_error_user_stopped() {
    let repo = Arc::new(crate::infrastructure::memory::MemoryIdeationSessionRepository::new());
    let dyn_repo: Arc<dyn crate::domain::repositories::IdeationSessionRepository> =
        repo.clone();

    let (parent_id, child_id) = make_parent_child_pair(&repo, None).await;

    reset_verification_on_child_error::<tauri::Wry>(&child_id, &dyn_repo, None, "user_stopped")
        .await;

    let parent_after = repo.get_by_id(&parent_id).await.unwrap().unwrap();
    assert_eq!(
        parent_after.verification_status,
        VerificationStatus::Unverified,
        "user stop should reset parent to Unverified"
    );
    assert!(!parent_after.verification_in_progress);

    let meta: serde_json::Value =
        serde_json::from_str(parent_after.verification_metadata.as_deref().unwrap()).unwrap();
    assert_eq!(meta["convergence_reason"].as_str().unwrap(), "user_stopped");
}

#[tokio::test]
async fn test_reset_verification_noop_for_non_verification_child() {
    let repo = Arc::new(crate::infrastructure::memory::MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();
    let dyn_repo: Arc<dyn crate::domain::repositories::IdeationSessionRepository> =
        repo.clone();

    // Parent with in_progress=true
    let mut parent = IdeationSession::new(project_id.clone());
    parent.verification_in_progress = true;
    parent.verification_status = VerificationStatus::Reviewing;
    let parent_id = parent.id.clone();
    repo.create(parent).await.unwrap();

    // Child is General purpose (NOT Verification) — should be a no-op
    let mut child = IdeationSession::new(project_id);
    child.session_purpose = crate::domain::entities::SessionPurpose::General;
    child.parent_session_id = Some(parent_id.clone());
    let child_id = child.id.clone();
    repo.create(child).await.unwrap();

    reset_verification_on_child_error::<tauri::Wry>(&child_id, &dyn_repo, None, "agent_error")
        .await;

    // Parent should be unchanged (not a verification child)
    let parent_after = repo.get_by_id(&parent_id).await.unwrap().unwrap();
    assert_eq!(parent_after.verification_status, VerificationStatus::Reviewing);
    assert!(parent_after.verification_in_progress, "parent must be unchanged for general child");
}

#[tokio::test]
async fn test_reset_verification_imported_verified_guard() {
    let repo = Arc::new(crate::infrastructure::memory::MemoryIdeationSessionRepository::new());
    let project_id = ProjectId::new();
    let dyn_repo: Arc<dyn crate::domain::repositories::IdeationSessionRepository> =
        repo.clone();

    // Parent is ImportedVerified — must not be reset
    let mut parent = IdeationSession::new(project_id.clone());
    parent.verification_status = VerificationStatus::ImportedVerified;
    parent.verification_in_progress = true;
    let parent_id = parent.id.clone();
    repo.create(parent).await.unwrap();

    let mut child = IdeationSession::new(project_id);
    child.session_purpose = crate::domain::entities::SessionPurpose::Verification;
    child.parent_session_id = Some(parent_id.clone());
    let child_id = child.id.clone();
    repo.create(child).await.unwrap();

    reset_verification_on_child_error::<tauri::Wry>(&child_id, &dyn_repo, None, "agent_error")
        .await;

    let parent_after = repo.get_by_id(&parent_id).await.unwrap().unwrap();
    assert_eq!(
        parent_after.verification_status,
        VerificationStatus::ImportedVerified,
        "ImportedVerified must not be overwritten by error reset"
    );
}
