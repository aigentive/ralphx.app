use super::*;
use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

#[tokio::test]
async fn test_get_default_settings() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    let repo = SqliteReviewSettingsRepository::new(conn);

    let settings = repo.get_settings().await.unwrap();
    assert!(settings.ai_review_enabled);
    assert!(settings.ai_review_auto_fix);
    assert!(!settings.require_fix_approval);
    assert!(!settings.require_human_review);
    assert_eq!(settings.max_fix_attempts, 3);
    assert_eq!(settings.max_revision_cycles, 5);
}

#[tokio::test]
async fn test_update_settings() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    let repo = SqliteReviewSettingsRepository::new(conn);

    let new_settings = ReviewSettings {
        ai_review_enabled: false,
        ai_review_auto_fix: false,
        require_fix_approval: true,
        require_human_review: true,
        max_fix_attempts: 5,
        max_revision_cycles: 10,
    };

    let updated = repo.update_settings(&new_settings).await.unwrap();
    assert!(!updated.ai_review_enabled);
    assert!(!updated.ai_review_auto_fix);
    assert!(updated.require_fix_approval);
    assert!(updated.require_human_review);
    assert_eq!(updated.max_fix_attempts, 5);
    assert_eq!(updated.max_revision_cycles, 10);

    // Verify persistence
    let retrieved = repo.get_settings().await.unwrap();
    assert!(!retrieved.ai_review_enabled);
    assert!(!retrieved.ai_review_auto_fix);
    assert!(retrieved.require_fix_approval);
    assert!(retrieved.require_human_review);
    assert_eq!(retrieved.max_fix_attempts, 5);
    assert_eq!(retrieved.max_revision_cycles, 10);
}

#[tokio::test]
async fn test_update_max_revision_cycles() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    let repo = SqliteReviewSettingsRepository::new(conn);

    let new_settings = ReviewSettings {
        max_revision_cycles: 2,
        ..Default::default()
    };

    repo.update_settings(&new_settings).await.unwrap();
    let retrieved = repo.get_settings().await.unwrap();
    assert_eq!(retrieved.max_revision_cycles, 2);
}
