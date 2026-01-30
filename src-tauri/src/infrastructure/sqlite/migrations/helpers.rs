// Migration helpers for safe schema modifications
//
// These helpers ensure migrations are idempotent and safe to re-run.

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

/// Check if a column exists in a table
pub fn column_exists(conn: &Connection, table: &str, column: &str) -> bool {
    let sql = format!("PRAGMA table_info({})", table);
    let mut stmt = match conn.prepare(&sql) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let rows = match stmt.query_map([], |row| row.get::<_, String>(1)) {
        Ok(r) => r,
        Err(_) => return false,
    };
    for row in rows.flatten() {
        if row == column {
            return true;
        }
    }
    false
}

/// Check if a table exists
pub fn table_exists(conn: &Connection, table: &str) -> bool {
    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            [table],
            |row| row.get(0),
        )
        .unwrap_or(0);
    count > 0
}

/// Check if an index exists
pub fn index_exists(conn: &Connection, index: &str) -> bool {
    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?1",
            [index],
            |row| row.get(0),
        )
        .unwrap_or(0);
    count > 0
}

/// Add column if it doesn't exist (SQLite doesn't support IF NOT EXISTS for columns)
pub fn add_column_if_not_exists(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> AppResult<()> {
    if !column_exists(conn, table, column) {
        let sql = format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, definition);
        conn.execute(&sql, [])
            .map_err(|e| AppError::Database(e.to_string()))?;
    }
    Ok(())
}

/// Create index if it doesn't exist
pub fn create_index_if_not_exists(
    conn: &Connection,
    index_name: &str,
    table: &str,
    columns: &str,
) -> AppResult<()> {
    let sql = format!(
        "CREATE INDEX IF NOT EXISTS {} ON {}({})",
        index_name, table, columns
    );
    conn.execute(&sql, [])
        .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
}
