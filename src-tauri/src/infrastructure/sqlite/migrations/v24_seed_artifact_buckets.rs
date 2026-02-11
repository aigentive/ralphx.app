use crate::error::AppResult;
use rusqlite::Connection;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    let buckets = [
        ("research-outputs", "Research Outputs"),
        ("work-context", "Work Context"),
        ("code-changes", "Code Changes"),
        ("prd-library", "PRD Library"),
    ];

    for (id, name) in &buckets {
        conn.execute(
            "INSERT OR IGNORE INTO artifact_buckets (id, name, config_json, is_system)
             VALUES (?1, ?2, '{}', 1)",
            rusqlite::params![id, name],
        )
        .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
    }

    Ok(())
}
