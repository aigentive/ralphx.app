//! Tests for migration v20260406130000: add verification_critic_results table

use rusqlite::Connection;

use super::v20260406130000_verification_critic_results;

fn open_fresh_db() -> Connection {
    Connection::open_in_memory().expect("Failed to open in-memory DB")
}

#[test]
fn test_v20260406130000_migration_fresh_db() {
    let conn = open_fresh_db();
    v20260406130000_verification_critic_results::migrate(&conn)
        .expect("Migration should apply cleanly on a fresh DB");

    // Verify the table exists by inserting a row
    conn.execute(
        "INSERT INTO verification_critic_results
            (id, parent_session_id, verification_session_id, verification_generation,
             round, critic_kind, artifact_id, status, created_at, updated_at)
         VALUES ('id-1', 'parent-1', 'vsess-1', 1, 1, 'completeness', 'art-1', 'complete',
                 '2026-04-06T13:00:00+00:00', '2026-04-06T13:00:00+00:00')",
        [],
    )
    .expect("Should be able to insert a row after migration");

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM verification_critic_results",
            [],
            |row| row.get(0),
        )
        .expect("Should be able to query the table");

    assert_eq!(count, 1, "Should have one row after insert");
}

#[test]
fn test_v20260406130000_migration_existing_db() {
    let conn = open_fresh_db();

    // Simulate a DB that already has some existing tables (prior migration state)
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        INSERT INTO schema_migrations (version) VALUES (20260406120000);",
    )
    .expect("Failed to set up existing DB state");

    // Migration must apply cleanly on top of this
    v20260406130000_verification_critic_results::migrate(&conn)
        .expect("Migration should apply cleanly on an existing DB");

    // Applying again (idempotent via IF NOT EXISTS) must not error
    v20260406130000_verification_critic_results::migrate(&conn)
        .expect("Migration should be idempotent — IF NOT EXISTS prevents error on re-run");
}

#[test]
fn test_v20260406130000_unique_constraint_on_duplicate_submit() {
    let conn = open_fresh_db();
    v20260406130000_verification_critic_results::migrate(&conn)
        .expect("Migration should apply cleanly");

    let insert_sql = "INSERT INTO verification_critic_results
        (id, parent_session_id, verification_session_id, verification_generation,
         round, critic_kind, artifact_id, status, created_at, updated_at)
     VALUES (?1, 'parent-1', 'vsess-1', 1, 1, 'completeness', ?2, 'complete',
             '2026-04-06T13:00:00+00:00', '2026-04-06T13:00:00+00:00')";

    // First insert succeeds
    conn.execute(insert_sql, rusqlite::params!["id-1", "art-1"])
        .expect("First insert should succeed");

    // Second insert with same (parent_session_id, verification_generation, round, critic_kind)
    // but different id and artifact_id must fail with UNIQUE constraint violation
    let result = conn.execute(insert_sql, rusqlite::params!["id-2", "art-2"]);

    match result {
        Err(rusqlite::Error::SqliteFailure(err, _)) => {
            // SQLITE_CONSTRAINT_UNIQUE extended error code = 2067
            assert_eq!(
                err.extended_code, 2067,
                "Expected SQLITE_CONSTRAINT_UNIQUE (2067), got extended_code={}",
                err.extended_code
            );
        }
        Err(other) => panic!("Expected SqliteFailure with code 2067, got: {:?}", other),
        Ok(_) => panic!("Expected UNIQUE constraint violation, but insert succeeded"),
    }
}
