// Helper functions for task repository operations

use chrono::{DateTime, Utc};
use rusqlite::Connection;

use crate::domain::entities::{InternalStatus, TaskId};
use crate::error::{AppError, AppResult};

/// Execute a status change transaction atomically
/// Updates task status and inserts history record in a single transaction
pub(super) fn persist_status_change_transaction(
    conn: &Connection,
    id: &TaskId,
    from: InternalStatus,
    to: InternalStatus,
    trigger: &str,
    now: DateTime<Utc>,
) -> AppResult<()> {
    // Use a transaction for atomicity
    conn.execute("BEGIN TRANSACTION", [])
        .map_err(|e| AppError::Database(e.to_string()))?;

    // Update task status
    let update_result = conn.execute(
        "UPDATE tasks SET internal_status = ?2, updated_at = ?3 WHERE id = ?1",
        rusqlite::params![id.as_str(), to.as_str(), now.to_rfc3339()],
    );

    if let Err(e) = update_result {
        let _ = conn.execute("ROLLBACK", []);
        return Err(AppError::Database(e.to_string()));
    }

    // Insert history record
    let history_id = uuid::Uuid::new_v4().to_string();
    let insert_result = conn.execute(
        "INSERT INTO task_state_history (id, task_id, from_status, to_status, changed_by, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            history_id,
            id.as_str(),
            from.as_str(),
            to.as_str(),
            trigger,
            now.to_rfc3339()
        ],
    );

    if let Err(e) = insert_result {
        let _ = conn.execute("ROLLBACK", []);
        return Err(AppError::Database(e.to_string()));
    }

    conn.execute("COMMIT", [])
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}
