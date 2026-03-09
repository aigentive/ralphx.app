use rusqlite::Connection;
use crate::error::AppResult;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    crate::infrastructure::sqlite::migrations::helpers::add_column_if_not_exists(
        conn,
        "project_metrics_config",
        "working_days_per_week",
        "INTEGER NOT NULL DEFAULT 5",
    )
}
