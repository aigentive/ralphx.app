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
