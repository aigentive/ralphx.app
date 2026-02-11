// Tests for v23_plan_selection_stats migration

use super::v23_plan_selection_stats;
use crate::infrastructure::sqlite::open_memory_connection;

#[test]
fn test_migration_v23_creates_plan_selection_stats_table() {
    let conn = open_memory_connection().unwrap();

    // Run migration
    v23_plan_selection_stats::migrate(&conn).unwrap();

    // Verify table exists
    let table_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='plan_selection_stats'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(table_count, 1, "plan_selection_stats table should exist");
}

#[test]
fn test_migration_v23_creates_indexes() {
    let conn = open_memory_connection().unwrap();

    // Run migration
    v23_plan_selection_stats::migrate(&conn).unwrap();

    // Verify session index exists
    let index_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_plan_selection_stats_session'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(index_count, 1, "Session index should exist");

    // Verify last_selected index exists
    let index_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_plan_selection_stats_last_selected'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(index_count, 1, "last_selected_at index should exist");
}

#[test]
fn test_migration_v23_is_idempotent() {
    let conn = open_memory_connection().unwrap();

    // Run migration twice
    v23_plan_selection_stats::migrate(&conn).unwrap();
    v23_plan_selection_stats::migrate(&conn).unwrap();

    // Should not error and table should still exist
    let table_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='plan_selection_stats'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(table_count, 1);
}
