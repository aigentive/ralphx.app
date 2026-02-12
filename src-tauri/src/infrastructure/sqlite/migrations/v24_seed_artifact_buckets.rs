use rusqlite::Connection;
use crate::error::AppResult;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    let buckets = [
        ("research-outputs", "Research Outputs"),
        ("work-context", "Work Context"),
        ("code-changes", "Code Changes"),
        ("prd-library", "PRD Library"),
    ];

    let default_config = r#"{"accepted_types":[],"writers":[],"readers":["all"]}"#;

    for (id, name) in &buckets {
        conn.execute(
            "INSERT OR IGNORE INTO artifact_buckets (id, name, config_json, is_system)
             VALUES (?1, ?2, ?3, 1)",
            rusqlite::params![id, name, default_config],
        )
        .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
    }

    Ok(())
}
