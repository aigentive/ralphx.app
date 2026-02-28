use super::*;
use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

#[tokio::test]
async fn test_get_default_global_settings() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    let repo = SqliteExecutionSettingsRepository::new(conn);

    // Get global defaults (project_id = None)
    let settings = repo.get_settings(None).await.unwrap();
    assert_eq!(settings.max_concurrent_tasks, 10);
    assert!(settings.auto_commit);
    assert!(settings.pause_on_failure);
}

#[tokio::test]
async fn test_update_global_settings() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    let repo = SqliteExecutionSettingsRepository::new(conn);

    let new_settings = ExecutionSettings {
        max_concurrent_tasks: 4,
        auto_commit: false,
        pause_on_failure: false,
    };

    // Update global defaults
    let updated = repo.update_settings(None, &new_settings).await.unwrap();
    assert_eq!(updated.max_concurrent_tasks, 4);
    assert!(!updated.auto_commit);
    assert!(!updated.pause_on_failure);

    // Verify persistence
    let retrieved = repo.get_settings(None).await.unwrap();
    assert_eq!(retrieved.max_concurrent_tasks, 4);
    assert!(!retrieved.auto_commit);
    assert!(!retrieved.pause_on_failure);
}

#[tokio::test]
async fn test_per_project_settings() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    let repo = SqliteExecutionSettingsRepository::new(conn);

    let project_id = ProjectId::from_string("test-project-123".to_string());

    // Initially, get_settings for a project should return global defaults
    let settings = repo.get_settings(Some(&project_id)).await.unwrap();
    assert_eq!(settings.max_concurrent_tasks, 10); // global default

    // Create project-specific settings
    let project_settings = ExecutionSettings {
        max_concurrent_tasks: 5,
        auto_commit: false,
        pause_on_failure: true,
    };

    repo.update_settings(Some(&project_id), &project_settings)
        .await
        .unwrap();

    // Now get_settings should return project-specific values
    let retrieved = repo.get_settings(Some(&project_id)).await.unwrap();
    assert_eq!(retrieved.max_concurrent_tasks, 5);
    assert!(!retrieved.auto_commit);
    assert!(retrieved.pause_on_failure);

    // Global settings should remain unchanged
    let global = repo.get_settings(None).await.unwrap();
    assert_eq!(global.max_concurrent_tasks, 10);
}

#[tokio::test]
async fn test_global_execution_settings() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    let repo = SqliteGlobalExecutionSettingsRepository::new(conn);

    // Get default global settings
    let settings = repo.get_settings().await.unwrap();
    assert_eq!(settings.global_max_concurrent, 20);

    // Update global settings
    let new_settings = GlobalExecutionSettings {
        global_max_concurrent: 30,
    };
    let updated = repo.update_settings(&new_settings).await.unwrap();
    assert_eq!(updated.global_max_concurrent, 30);

    // Verify persistence
    let retrieved = repo.get_settings().await.unwrap();
    assert_eq!(retrieved.global_max_concurrent, 30);
}

#[tokio::test]
async fn test_global_max_concurrent_capped_at_50() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    let repo = SqliteGlobalExecutionSettingsRepository::new(conn);

    // Try to set above max
    let new_settings = GlobalExecutionSettings {
        global_max_concurrent: 100,
    };
    let updated = repo.update_settings(&new_settings).await.unwrap();

    // Should be clamped to 50
    assert_eq!(updated.global_max_concurrent, 50);

    // Verify persistence
    let retrieved = repo.get_settings().await.unwrap();
    assert_eq!(retrieved.global_max_concurrent, 50);
}

#[tokio::test]
async fn test_shared_connection() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    let shared_conn = Arc::new(Mutex::new(conn));

    let repo = SqliteExecutionSettingsRepository::from_shared(Arc::clone(&shared_conn));

    let settings = repo.get_settings(None).await.unwrap();
    assert_eq!(settings.max_concurrent_tasks, 10);
}

#[tokio::test]
async fn test_global_execution_settings_are_not_per_project() {
    // Verifies that SqliteGlobalExecutionSettingsRepository manages a single shared row,
    // not per-project rows — changing global settings affects all consumers.
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    let shared_conn = Arc::new(Mutex::new(conn));

    let repo_a =
        SqliteGlobalExecutionSettingsRepository::from_shared(Arc::clone(&shared_conn));
    let repo_b =
        SqliteGlobalExecutionSettingsRepository::from_shared(Arc::clone(&shared_conn));

    // Update via repo_a
    repo_a
        .update_settings(&GlobalExecutionSettings {
            global_max_concurrent: 25,
        })
        .await
        .unwrap();

    // repo_b (same underlying storage) should see the same value
    let settings_b = repo_b.get_settings().await.unwrap();
    assert_eq!(settings_b.global_max_concurrent, 25);
}

#[tokio::test]
async fn test_project_settings_do_not_bleed_across_projects() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    let repo = SqliteExecutionSettingsRepository::new(conn);

    let project_x = ProjectId::from_string("project-x".to_string());
    let project_y = ProjectId::from_string("project-y".to_string());

    // Set distinct settings for each project
    repo.update_settings(
        Some(&project_x),
        &ExecutionSettings {
            max_concurrent_tasks: 3,
            auto_commit: false,
            pause_on_failure: false,
        },
    )
    .await
    .unwrap();

    repo.update_settings(
        Some(&project_y),
        &ExecutionSettings {
            max_concurrent_tasks: 7,
            auto_commit: true,
            pause_on_failure: true,
        },
    )
    .await
    .unwrap();

    let x_settings = repo.get_settings(Some(&project_x)).await.unwrap();
    let y_settings = repo.get_settings(Some(&project_y)).await.unwrap();

    assert_eq!(x_settings.max_concurrent_tasks, 3);
    assert_eq!(y_settings.max_concurrent_tasks, 7);
    assert!(!x_settings.auto_commit);
    assert!(y_settings.auto_commit);
}
