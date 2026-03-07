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
        stale_after_secs: 5400, // 90 min
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
