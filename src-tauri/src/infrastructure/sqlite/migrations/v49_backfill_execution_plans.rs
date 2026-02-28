// Migration v49: Backfill execution_plans for existing accepted/archived sessions
//
// Creates ExecutionPlan records for existing accepted/archived sessions that predate
// the execution_plans table (v46). Links their tasks and plan_branches to the new
// ExecutionPlan records, ensuring backward compatibility.
//
// Also adds execution_plan_id to project_active_plan for future use (nullable, additive).
//
// # Rollback notes
// This migration is additive and idempotent. To revert manually:
//   DELETE FROM execution_plans WHERE id IN (SELECT execution_plan_id FROM tasks WHERE ...);
//   UPDATE tasks SET execution_plan_id = NULL WHERE ...;
//   UPDATE plan_branches SET execution_plan_id = NULL WHERE ...;
// The migration system does not support automated rollback.

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

use super::helpers;

/// Migration v49: Backfill execution_plans for existing accepted/archived sessions
pub fn migrate(conn: &Connection) -> AppResult<()> {
    // Step 1: Add execution_plan_id to project_active_plan (nullable, for future use)
    helpers::add_column_if_not_exists(
        conn,
        "project_active_plan",
        "execution_plan_id",
        "TEXT REFERENCES execution_plans(id)",
    )?;

    // Step 2: Find all accepted/archived sessions that have no execution_plan yet
    let mut stmt = conn
        .prepare(
            "SELECT s.id FROM ideation_sessions s
             LEFT JOIN execution_plans ep ON ep.session_id = s.id
             WHERE s.status IN ('accepted', 'archived') AND ep.id IS NULL",
        )
        .map_err(|e| AppError::Database(format!("Failed to prepare backfill query: {}", e)))?;

    let session_ids: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| AppError::Database(format!("Failed to query sessions: {}", e)))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::Database(format!("Failed to collect sessions: {}", e)))?;

    drop(stmt);

    tracing::info!(
        "v49 backfill: found {} session(s) needing ExecutionPlan",
        session_ids.len()
    );

    // Step 3: For each session, create an ExecutionPlan and link tasks/branches
    for session_id in &session_ids {
        let execution_plan_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        // Create the execution_plan record
        conn.execute(
            "INSERT INTO execution_plans (id, session_id, status, created_at)
             VALUES (?1, ?2, 'active', ?3)",
            rusqlite::params![execution_plan_id, session_id, now],
        )
        .map_err(|e| {
            AppError::Database(format!(
                "Failed to insert execution_plan for session {}: {}",
                session_id, e
            ))
        })?;

        // Link tasks that belong to this session and have no execution_plan_id
        conn.execute(
            "UPDATE tasks SET execution_plan_id = ?1
             WHERE ideation_session_id = ?2 AND execution_plan_id IS NULL",
            rusqlite::params![execution_plan_id, session_id],
        )
        .map_err(|e| {
            AppError::Database(format!(
                "Failed to link tasks for session {}: {}",
                session_id, e
            ))
        })?;

        // Link plan_branches that belong to this session and have no execution_plan_id
        conn.execute(
            "UPDATE plan_branches SET execution_plan_id = ?1
             WHERE session_id = ?2 AND execution_plan_id IS NULL",
            rusqlite::params![execution_plan_id, session_id],
        )
        .map_err(|e| {
            AppError::Database(format!(
                "Failed to link plan_branches for session {}: {}",
                session_id, e
            ))
        })?;

        // Populate execution_plan_id on project_active_plan entries for this session
        conn.execute(
            "UPDATE project_active_plan SET execution_plan_id = ?1
             WHERE ideation_session_id = ?2 AND execution_plan_id IS NULL",
            rusqlite::params![execution_plan_id, session_id],
        )
        .map_err(|e| {
            AppError::Database(format!(
                "Failed to update active_plan for session {}: {}",
                session_id, e
            ))
        })?;
    }

    tracing::info!(
        "v49 backfill: created {} execution_plan(s) and linked existing data",
        session_ids.len()
    );

    Ok(())
}
