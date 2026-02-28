use super::*;
use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

#[tokio::test]
async fn test_get_default_settings() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    let repo = SqliteIdeationSettingsRepository::new(conn);

    let settings = repo.get_settings().await.unwrap();
    assert_eq!(settings.plan_mode, IdeationPlanMode::Optional);
    assert!(!settings.require_plan_approval);
    assert!(settings.suggest_plans_for_complex);
    assert!(settings.auto_link_proposals);
}

#[tokio::test]
async fn test_update_settings() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    let repo = SqliteIdeationSettingsRepository::new(conn);

    let new_settings = IdeationSettings {
        plan_mode: IdeationPlanMode::Required,
        require_plan_approval: true,
        suggest_plans_for_complex: false,
        auto_link_proposals: false,
    };

    let updated = repo.update_settings(&new_settings).await.unwrap();
    assert_eq!(updated.plan_mode, IdeationPlanMode::Required);
    assert!(updated.require_plan_approval);
    assert!(!updated.suggest_plans_for_complex);
    assert!(!updated.auto_link_proposals);

    // Verify persistence
    let retrieved = repo.get_settings().await.unwrap();
    assert_eq!(retrieved.plan_mode, IdeationPlanMode::Required);
    assert!(retrieved.require_plan_approval);
    assert!(!retrieved.suggest_plans_for_complex);
    assert!(!retrieved.auto_link_proposals);
}

#[tokio::test]
async fn test_update_settings_all_modes() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    let repo = SqliteIdeationSettingsRepository::new(conn);

    // Test Required mode
    let required_settings = IdeationSettings {
        plan_mode: IdeationPlanMode::Required,
        ..Default::default()
    };
    repo.update_settings(&required_settings).await.unwrap();
    let retrieved = repo.get_settings().await.unwrap();
    assert_eq!(retrieved.plan_mode, IdeationPlanMode::Required);

    // Test Optional mode
    let optional_settings = IdeationSettings {
        plan_mode: IdeationPlanMode::Optional,
        ..Default::default()
    };
    repo.update_settings(&optional_settings).await.unwrap();
    let retrieved = repo.get_settings().await.unwrap();
    assert_eq!(retrieved.plan_mode, IdeationPlanMode::Optional);

    // Test Parallel mode
    let parallel_settings = IdeationSettings {
        plan_mode: IdeationPlanMode::Parallel,
        ..Default::default()
    };
    repo.update_settings(&parallel_settings).await.unwrap();
    let retrieved = repo.get_settings().await.unwrap();
    assert_eq!(retrieved.plan_mode, IdeationPlanMode::Parallel);
}

// ─── from_shared ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_from_shared_returns_defaults() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    let shared = Arc::new(Mutex::new(conn));
    let repo = SqliteIdeationSettingsRepository::from_shared(Arc::clone(&shared));

    let settings = repo.get_settings().await.unwrap();
    assert_eq!(settings.plan_mode, IdeationPlanMode::Optional);
    assert!(!settings.require_plan_approval);
}

// ─── fallback when no row ────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_settings_fallback_when_no_row() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    // Remove the default row (if any) seeded by migrations
    conn.execute("DELETE FROM ideation_settings", []).unwrap();
    let repo = SqliteIdeationSettingsRepository::new(conn);

    let settings = repo.get_settings().await.unwrap();
    // Must return defaults without error
    assert_eq!(settings.plan_mode, IdeationPlanMode::Optional);
    assert!(!settings.require_plan_approval);
    assert!(settings.suggest_plans_for_complex);
    assert!(settings.auto_link_proposals);
}

// ─── second update overrides first ───────────────────────────────────────────

#[tokio::test]
async fn test_update_overrides_previous_update() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    let repo = SqliteIdeationSettingsRepository::new(conn);

    repo.update_settings(&IdeationSettings {
        plan_mode: IdeationPlanMode::Required,
        require_plan_approval: true,
        suggest_plans_for_complex: false,
        auto_link_proposals: false,
    })
    .await
    .unwrap();

    repo.update_settings(&IdeationSettings {
        plan_mode: IdeationPlanMode::Parallel,
        require_plan_approval: false,
        suggest_plans_for_complex: true,
        auto_link_proposals: true,
    })
    .await
    .unwrap();

    let s = repo.get_settings().await.unwrap();
    assert_eq!(s.plan_mode, IdeationPlanMode::Parallel);
    assert!(!s.require_plan_approval);
    assert!(s.suggest_plans_for_complex);
    assert!(s.auto_link_proposals);
}

// ─── boolean fields toggle independently ────────────────────────────────────

#[tokio::test]
async fn test_boolean_fields_toggle_independently() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    let repo = SqliteIdeationSettingsRepository::new(conn);

    // Enable only require_plan_approval, disable the rest
    repo.update_settings(&IdeationSettings {
        plan_mode: IdeationPlanMode::Optional,
        require_plan_approval: true,
        suggest_plans_for_complex: false,
        auto_link_proposals: false,
    })
    .await
    .unwrap();

    let s = repo.get_settings().await.unwrap();
    assert!(s.require_plan_approval);
    assert!(!s.suggest_plans_for_complex);
    assert!(!s.auto_link_proposals);

    // Flip: disable require_plan_approval, enable the other two
    repo.update_settings(&IdeationSettings {
        plan_mode: IdeationPlanMode::Optional,
        require_plan_approval: false,
        suggest_plans_for_complex: true,
        auto_link_proposals: true,
    })
    .await
    .unwrap();

    let s2 = repo.get_settings().await.unwrap();
    assert!(!s2.require_plan_approval);
    assert!(s2.suggest_plans_for_complex);
    assert!(s2.auto_link_proposals);
}
