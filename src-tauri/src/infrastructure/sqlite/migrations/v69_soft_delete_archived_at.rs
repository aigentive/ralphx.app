// Migration v69: Add archived_at column to task_proposals, artifacts, and projects tables
//
// Enables soft-delete / archive functionality across the three main entity types.
// archived_at is TEXT (ISO-8601 / RFC-3339 timestamp) NULL — NULL means not archived.

use rusqlite::Connection;

use crate::error::AppResult;
use crate::infrastructure::sqlite::migrations::helpers;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    helpers::add_column_if_not_exists(conn, "task_proposals", "archived_at", "TEXT NULL")?;
    helpers::add_column_if_not_exists(conn, "artifacts", "archived_at", "TEXT NULL")?;
    helpers::add_column_if_not_exists(conn, "projects", "archived_at", "TEXT NULL")?;

    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_task_proposals_archived_at
             ON task_proposals(archived_at);
         CREATE INDEX IF NOT EXISTS idx_artifacts_archived_at
             ON artifacts(archived_at);
         CREATE INDEX IF NOT EXISTS idx_projects_archived_at
             ON projects(archived_at);",
    )?;

    tracing::info!(
        "v69: added archived_at column to task_proposals, artifacts, and projects; \
         created supporting indexes"
    );

    Ok(())
}
