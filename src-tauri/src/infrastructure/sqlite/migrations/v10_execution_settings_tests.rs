use super::*;
use crate::infrastructure::sqlite::connection::open_memory_connection;

// ==========================================================================
// V10 migration tests - execution_settings table
// ==========================================================================

#[test]
fn test_v10_creates_execution_settings_table() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::table_exists(&conn, "execution_settings"));
}

#[test]
fn test_v10_inserts_default_row() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Check default row exists with id=1
    let (id, max_concurrent, auto_commit, pause_on_failure): (i32, i32, i32, i32) = conn
        .query_row(
            "SELECT id, max_concurrent_tasks, auto_commit, pause_on_failure FROM execution_settings WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();

    assert_eq!(id, 1);
    assert_eq!(max_concurrent, 2);
    assert_eq!(auto_commit, 1);
    assert_eq!(pause_on_failure, 1);
}

#[test]
fn test_v10_updated_at_is_set() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    let updated_at: String = conn
        .query_row(
            "SELECT updated_at FROM execution_settings WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();

    // Should be RFC3339 format
    assert!(updated_at.contains("T"));
    assert!(updated_at.ends_with("+00:00"));
}

#[test]
fn test_v10_only_allows_id_1() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Attempting to insert with id=2 should fail due to CHECK constraint
    let result = conn.execute(
        "INSERT INTO execution_settings (id, max_concurrent_tasks, auto_commit, pause_on_failure, updated_at)
         VALUES (2, 4, 0, 0, strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
        [],
    );
    assert!(result.is_err());
}

#[test]
fn test_v10_settings_can_be_updated() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Update settings
    conn.execute(
        "UPDATE execution_settings SET max_concurrent_tasks = 5, auto_commit = 0 WHERE id = 1",
        [],
    )
    .unwrap();

    // Verify update
    let (max_concurrent, auto_commit): (i32, i32) = conn
        .query_row(
            "SELECT max_concurrent_tasks, auto_commit FROM execution_settings WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    assert_eq!(max_concurrent, 5);
    assert_eq!(auto_commit, 0);
}

#[test]
fn test_v10_migration_is_idempotent() {
    let conn = open_memory_connection().unwrap();

    // Run migrations twice
    run_migrations(&conn).unwrap();
    run_migrations(&conn).unwrap();

    // Table should still exist with one row
    assert!(helpers::table_exists(&conn, "execution_settings"));

    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM execution_settings",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_v10_default_row_preserves_existing_data() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Modify the settings
    conn.execute(
        "UPDATE execution_settings SET max_concurrent_tasks = 8 WHERE id = 1",
        [],
    )
    .unwrap();

    // Run migrations again (simulating upgrade)
    run_migrations(&conn).unwrap();

    // Existing data should be preserved (INSERT OR IGNORE doesn't overwrite)
    let max_concurrent: i32 = conn
        .query_row(
            "SELECT max_concurrent_tasks FROM execution_settings WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(max_concurrent, 8);
}

#[test]
fn test_v10_all_columns_have_correct_types() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Verify columns exist with correct types by inserting valid data
    // (implicitly tests column types through successful operations)
    conn.execute(
        "UPDATE execution_settings
         SET max_concurrent_tasks = 10,
             auto_commit = 1,
             pause_on_failure = 0,
             updated_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')
         WHERE id = 1",
        [],
    )
    .unwrap();

    // Read back all values
    let (max_concurrent, auto_commit, pause_on_failure, updated_at): (i32, i32, i32, String) = conn
        .query_row(
            "SELECT max_concurrent_tasks, auto_commit, pause_on_failure, updated_at
             FROM execution_settings WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();

    assert_eq!(max_concurrent, 10);
    assert_eq!(auto_commit, 1);
    assert_eq!(pause_on_failure, 0);
    assert!(updated_at.contains("T"));
}
