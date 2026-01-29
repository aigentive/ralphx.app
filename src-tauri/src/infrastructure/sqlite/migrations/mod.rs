// Database migrations for SQLite
// Creates and updates schema as needed

// Allow items after test module - migrations are defined after tests for readability
#![allow(clippy::items_after_test_module)]

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

mod migrations_v1_v10;
mod migrations_v11_v20;
mod migrations_v21_v26;

use migrations_v1_v10::*;
use migrations_v11_v20::*;
use migrations_v21_v26::*;

/// Current schema version
pub const SCHEMA_VERSION: i32 = 26;

/// Run all pending migrations on the database
pub fn run_migrations(conn: &Connection) -> AppResult<()> {
    // Create migrations table if it doesn't exist
    create_migrations_table(conn)?;

    // Get current version
    let current_version = get_schema_version(conn)?;

    // Run migrations sequentially
    if current_version < 1 {
        migrate_v1(conn)?;
        set_schema_version(conn, 1)?;
    }

    if current_version < 2 {
        migrate_v2(conn)?;
        set_schema_version(conn, 2)?;
    }

    if current_version < 3 {
        migrate_v3(conn)?;
        set_schema_version(conn, 3)?;
    }

    if current_version < 4 {
        migrate_v4(conn)?;
        set_schema_version(conn, 4)?;
    }

    if current_version < 5 {
        migrate_v5(conn)?;
        set_schema_version(conn, 5)?;
    }

    if current_version < 6 {
        migrate_v6(conn)?;
        set_schema_version(conn, 6)?;
    }

    if current_version < 7 {
        migrate_v7(conn)?;
        set_schema_version(conn, 7)?;
    }

    if current_version < 8 {
        migrate_v8(conn)?;
        set_schema_version(conn, 8)?;
    }

    if current_version < 9 {
        migrate_v9(conn)?;
        set_schema_version(conn, 9)?;
    }

    if current_version < 10 {
        migrate_v10(conn)?;
        set_schema_version(conn, 10)?;
    }

    if current_version < 11 {
        migrate_v11(conn)?;
        set_schema_version(conn, 11)?;
    }

    if current_version < 12 {
        migrate_v12(conn)?;
        set_schema_version(conn, 12)?;
    }

    if current_version < 13 {
        migrate_v13(conn)?;
        set_schema_version(conn, 13)?;
    }

    if current_version < 14 {
        migrate_v14(conn)?;
        set_schema_version(conn, 14)?;
    }

    if current_version < 15 {
        migrate_v15(conn)?;
        set_schema_version(conn, 15)?;
    }

    if current_version < 16 {
        migrate_v16(conn)?;
        set_schema_version(conn, 16)?;
    }

    if current_version < 17 {
        migrate_v17(conn)?;
        set_schema_version(conn, 17)?;
    }

    if current_version < 18 {
        migrate_v18(conn)?;
        set_schema_version(conn, 18)?;
    }

    if current_version < 19 {
        migrate_v19(conn)?;
        set_schema_version(conn, 19)?;
    }

    if current_version < 20 {
        migrate_v20(conn)?;
        set_schema_version(conn, 20)?;
    }

    if current_version < 21 {
        migrate_v21(conn)?;
        set_schema_version(conn, 21)?;
    }

    if current_version < 22 {
        migrate_v22(conn)?;
        set_schema_version(conn, 22)?;
    }

    if current_version < 23 {
        migrate_v23(conn)?;
        set_schema_version(conn, 23)?;
    }

    if current_version < 24 {
        migrate_v24(conn)?;
        set_schema_version(conn, 24)?;
    }

    if current_version < 25 {
        migrate_v25(conn)?;
        set_schema_version(conn, 25)?;
    }

    if current_version < 26 {
        migrate_v26(conn)?;
        set_schema_version(conn, 26)?;
    }

    Ok(())
}

/// Create the migrations tracking table
fn create_migrations_table(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
}

/// Get the current schema version
fn get_schema_version(conn: &Connection) -> AppResult<i32> {
    let result: Result<i32, _> = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    );

    result.map_err(|e| AppError::Database(e.to_string()))
}

/// Set the schema version after a migration
fn set_schema_version(conn: &Connection, version: i32) -> AppResult<()> {
    conn.execute(
        "INSERT INTO schema_migrations (version) VALUES (?1)",
        [version],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
}


#[cfg(test)]
mod tests;
