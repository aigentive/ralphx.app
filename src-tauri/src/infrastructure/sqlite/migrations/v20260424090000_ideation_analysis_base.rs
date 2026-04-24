// Migration v20260424090000: ideation analysis base and workspace metadata

use rusqlite::Connection;

use crate::error::AppResult;

use super::helpers::add_column_if_not_exists;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    add_column_if_not_exists(
        conn,
        "ideation_sessions",
        "analysis_base_ref_kind",
        "TEXT NULL",
    )?;
    add_column_if_not_exists(conn, "ideation_sessions", "analysis_base_ref", "TEXT NULL")?;
    add_column_if_not_exists(
        conn,
        "ideation_sessions",
        "analysis_base_display_name",
        "TEXT NULL",
    )?;
    add_column_if_not_exists(
        conn,
        "ideation_sessions",
        "analysis_workspace_kind",
        "TEXT NOT NULL DEFAULT 'project_root'",
    )?;
    add_column_if_not_exists(
        conn,
        "ideation_sessions",
        "analysis_workspace_path",
        "TEXT NULL",
    )?;
    add_column_if_not_exists(
        conn,
        "ideation_sessions",
        "analysis_base_commit",
        "TEXT NULL",
    )?;
    add_column_if_not_exists(
        conn,
        "ideation_sessions",
        "analysis_base_locked_at",
        "TEXT NULL",
    )?;
    Ok(())
}
