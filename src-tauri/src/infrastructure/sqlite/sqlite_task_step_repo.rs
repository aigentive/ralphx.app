// SQLite-based TaskStepRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use rusqlite::Connection;

use crate::domain::entities::{TaskId, TaskStep, TaskStepId, TaskStepStatus};
use crate::domain::repositories::TaskStepRepository;
use crate::error::{AppError, AppResult};

/// SQLite implementation of TaskStepRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteTaskStepRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteTaskStepRepository {
    /// Create a new SQLite task step repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    /// Create from an Arc-wrapped mutex connection (for sharing)
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl TaskStepRepository for SqliteTaskStepRepository {
    async fn create(&self, step: TaskStep) -> AppResult<TaskStep> {
        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT INTO task_steps (id, task_id, title, description, status, sort_order, depends_on, created_by, completion_note, created_at, updated_at, started_at, completed_at, parent_step_id, scope_context)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            rusqlite::params![
                step.id.as_str(),
                step.task_id.as_str(),
                step.title,
                step.description,
                step.status.to_db_string(),
                step.sort_order,
                step.depends_on.as_ref().map(|id| id.as_str()),
                step.created_by,
                step.completion_note,
                step.created_at.to_rfc3339(),
                step.updated_at.to_rfc3339(),
                step.started_at.map(|dt| dt.to_rfc3339()),
                step.completed_at.map(|dt| dt.to_rfc3339()),
                step.parent_step_id.as_ref().map(|id| id.as_str()),
                step.scope_context,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(step)
    }

    async fn get_by_id(&self, id: &TaskStepId) -> AppResult<Option<TaskStep>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, task_id, title, description, status, sort_order, depends_on, created_by, completion_note, created_at, updated_at, started_at, completed_at, parent_step_id, scope_context
             FROM task_steps WHERE id = ?1",
            [id.as_str()],
            |row| TaskStep::from_row(row),
        );

        match result {
            Ok(step) => Ok(Some(step)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_by_task(&self, task_id: &TaskId) -> AppResult<Vec<TaskStep>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, task_id, title, description, status, sort_order, depends_on, created_by, completion_note, created_at, updated_at, started_at, completed_at, parent_step_id, scope_context
                 FROM task_steps WHERE task_id = ?1
                 ORDER BY sort_order ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let steps = stmt
            .query_map([task_id.as_str()], TaskStep::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(steps)
    }

    async fn get_by_task_and_status(
        &self,
        task_id: &TaskId,
        status: TaskStepStatus,
    ) -> AppResult<Vec<TaskStep>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, task_id, title, description, status, sort_order, depends_on, created_by, completion_note, created_at, updated_at, started_at, completed_at, parent_step_id, scope_context
                 FROM task_steps WHERE task_id = ?1 AND status = ?2
                 ORDER BY sort_order ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let steps = stmt
            .query_map(
                rusqlite::params![task_id.as_str(), status.to_db_string()],
                TaskStep::from_row,
            )
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(steps)
    }

    async fn update(&self, step: &TaskStep) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "UPDATE task_steps SET task_id = ?2, title = ?3, description = ?4, status = ?5, sort_order = ?6, depends_on = ?7, created_by = ?8, completion_note = ?9, updated_at = ?10, started_at = ?11, completed_at = ?12, parent_step_id = ?13, scope_context = ?14
             WHERE id = ?1",
            rusqlite::params![
                step.id.as_str(),
                step.task_id.as_str(),
                step.title,
                step.description,
                step.status.to_db_string(),
                step.sort_order,
                step.depends_on.as_ref().map(|id| id.as_str()),
                step.created_by,
                step.completion_note,
                step.updated_at.to_rfc3339(),
                step.started_at.map(|dt| dt.to_rfc3339()),
                step.completed_at.map(|dt| dt.to_rfc3339()),
                step.parent_step_id.as_ref().map(|id| id.as_str()),
                step.scope_context,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: &TaskStepId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute("DELETE FROM task_steps WHERE id = ?1", [id.as_str()])
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete_by_task(&self, task_id: &TaskId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "DELETE FROM task_steps WHERE task_id = ?1",
            [task_id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn count_by_status(&self, task_id: &TaskId) -> AppResult<HashMap<TaskStepStatus, u32>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT status, COUNT(*) as count
                 FROM task_steps WHERE task_id = ?1
                 GROUP BY status",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut counts = HashMap::new();
        let rows = stmt
            .query_map([task_id.as_str()], |row| {
                let status_str: String = row.get(0)?;
                let count: u32 = row.get(1)?;
                Ok((status_str, count))
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        for row in rows {
            let (status_str, count) = row.map_err(|e| AppError::Database(e.to_string()))?;
            if let Ok(status) = TaskStepStatus::from_db_string(&status_str) {
                counts.insert(status, count);
            }
        }

        Ok(counts)
    }

    async fn bulk_create(&self, steps: Vec<TaskStep>) -> AppResult<Vec<TaskStep>> {
        let conn = self.conn.lock().await;

        // Use a transaction for atomicity
        conn.execute("BEGIN TRANSACTION", [])
            .map_err(|e| AppError::Database(e.to_string()))?;

        for step in &steps {
            let result = conn.execute(
                "INSERT INTO task_steps (id, task_id, title, description, status, sort_order, depends_on, created_by, completion_note, created_at, updated_at, started_at, completed_at, parent_step_id, scope_context)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                rusqlite::params![
                    step.id.as_str(),
                    step.task_id.as_str(),
                    step.title,
                    step.description,
                    step.status.to_db_string(),
                    step.sort_order,
                    step.depends_on.as_ref().map(|id| id.as_str()),
                    step.created_by,
                    step.completion_note,
                    step.created_at.to_rfc3339(),
                    step.updated_at.to_rfc3339(),
                    step.started_at.map(|dt| dt.to_rfc3339()),
                    step.completed_at.map(|dt| dt.to_rfc3339()),
                    step.parent_step_id.as_ref().map(|id| id.as_str()),
                    step.scope_context,
                ],
            );

            if let Err(e) = result {
                let _ = conn.execute("ROLLBACK", []);
                return Err(AppError::Database(e.to_string()));
            }
        }

        conn.execute("COMMIT", [])
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(steps)
    }

    async fn reorder(&self, task_id: &TaskId, step_ids: Vec<TaskStepId>) -> AppResult<()> {
        let conn = self.conn.lock().await;

        // Use a transaction for atomicity
        conn.execute("BEGIN TRANSACTION", [])
            .map_err(|e| AppError::Database(e.to_string()))?;

        for (index, step_id) in step_ids.iter().enumerate() {
            let result = conn.execute(
                "UPDATE task_steps SET sort_order = ?1 WHERE id = ?2 AND task_id = ?3",
                rusqlite::params![index as i32, step_id.as_str(), task_id.as_str()],
            );

            if let Err(e) = result {
                let _ = conn.execute("ROLLBACK", []);
                return Err(AppError::Database(e.to_string()));
            }
        }

        conn.execute("COMMIT", [])
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
#[path = "sqlite_task_step_repo_tests.rs"]
mod tests;
