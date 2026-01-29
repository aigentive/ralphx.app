// SQLite-based TaskRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

mod helpers;
mod queries;
mod query_builder;

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use rusqlite::Connection;

use crate::domain::entities::{InternalStatus, ProjectId, Task, TaskId};
use crate::domain::repositories::{StatusTransition, TaskRepository};
use crate::error::{AppError, AppResult};

/// SQLite implementation of TaskRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteTaskRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteTaskRepository {
    /// Create a new SQLite task repository with the given connection
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
impl TaskRepository for SqliteTaskRepository {
    async fn create(&self, task: Task) -> AppResult<Task> {
        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title, description, priority, internal_status, needs_review_point, source_proposal_id, plan_artifact_id, created_at, updated_at, started_at, completed_at, archived_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            rusqlite::params![
                task.id.as_str(),
                task.project_id.as_str(),
                task.category,
                task.title,
                task.description,
                task.priority,
                task.internal_status.as_str(),
                task.needs_review_point,
                task.source_proposal_id.as_ref().map(|id| id.as_str()),
                task.plan_artifact_id.as_ref().map(|id| id.as_str()),
                task.created_at.to_rfc3339(),
                task.updated_at.to_rfc3339(),
                task.started_at.map(|dt| dt.to_rfc3339()),
                task.completed_at.map(|dt| dt.to_rfc3339()),
                task.archived_at.map(|dt| dt.to_rfc3339()),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(task)
    }

    async fn get_by_id(&self, id: &TaskId) -> AppResult<Option<Task>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(queries::GET_BY_ID, [id.as_str()], |row| Task::from_row(row));

        match result {
            Ok(task) => Ok(Some(task)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<Task>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(queries::GET_BY_PROJECT)
            .map_err(|e| AppError::Database(e.to_string()))?;

        let tasks = stmt
            .query_map([project_id.as_str()], Task::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(tasks)
    }

    async fn update(&self, task: &Task) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "UPDATE tasks SET project_id = ?2, category = ?3, title = ?4, description = ?5, priority = ?6, internal_status = ?7, source_proposal_id = ?8, plan_artifact_id = ?9, updated_at = ?10, started_at = ?11, completed_at = ?12, archived_at = ?13
             WHERE id = ?1",
            rusqlite::params![
                task.id.as_str(),
                task.project_id.as_str(),
                task.category,
                task.title,
                task.description,
                task.priority,
                task.internal_status.as_str(),
                task.source_proposal_id.as_ref().map(|id| id.as_str()),
                task.plan_artifact_id.as_ref().map(|id| id.as_str()),
                task.updated_at.to_rfc3339(),
                task.started_at.map(|dt| dt.to_rfc3339()),
                task.completed_at.map(|dt| dt.to_rfc3339()),
                task.archived_at.map(|dt| dt.to_rfc3339()),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: &TaskId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(queries::DELETE_TASK, [id.as_str()])
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_by_status(
        &self,
        project_id: &ProjectId,
        status: InternalStatus,
    ) -> AppResult<Vec<Task>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, category, title, description, priority, internal_status, needs_review_point, source_proposal_id, plan_artifact_id, created_at, updated_at, started_at, completed_at, archived_at
                 FROM tasks WHERE project_id = ?1 AND internal_status = ?2
                 ORDER BY priority DESC, created_at ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let tasks = stmt
            .query_map(
                rusqlite::params![project_id.as_str(), status.as_str()],
                |row| Task::from_row(row),
            )
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(tasks)
    }

    async fn persist_status_change(
        &self,
        id: &TaskId,
        from: InternalStatus,
        to: InternalStatus,
        trigger: &str,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;
        let now = Utc::now();
        helpers::persist_status_change_transaction(&conn, id, from, to, trigger, now)
    }

    async fn get_status_history(&self, id: &TaskId) -> AppResult<Vec<StatusTransition>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT from_status, to_status, changed_by, created_at
                 FROM task_state_history WHERE task_id = ?1
                 ORDER BY created_at ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let transitions = stmt
            .query_map([id.as_str()], |row| {
                let from_str: String = row.get(0)?;
                let to_str: String = row.get(1)?;
                let trigger: String = row.get(2)?;
                let created_at_str: String = row.get(3)?;

                let from = from_str.parse().unwrap_or(InternalStatus::Backlog);
                let to = to_str.parse().unwrap_or(InternalStatus::Backlog);
                let timestamp = Task::parse_datetime(created_at_str);

                Ok(StatusTransition::with_timestamp(from, to, trigger, timestamp))
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(transitions)
    }

    async fn get_next_executable(&self, project_id: &ProjectId) -> AppResult<Option<Task>> {
        let conn = self.conn.lock().await;

        // Find READY tasks that have no blockers
        let result = conn.query_row(
            "SELECT t.id, t.project_id, t.category, t.title, t.description, t.priority, t.internal_status, t.needs_review_point, t.source_proposal_id, t.plan_artifact_id, t.created_at, t.updated_at, t.started_at, t.completed_at, t.archived_at
             FROM tasks t
             WHERE t.project_id = ?1
               AND t.internal_status = 'ready'
               AND NOT EXISTS (
                   SELECT 1 FROM task_blockers tb WHERE tb.task_id = t.id
               )
             ORDER BY t.priority DESC, t.created_at ASC
             LIMIT 1",
            [project_id.as_str()],
            |row| Task::from_row(row),
        );

        match result {
            Ok(task) => Ok(Some(task)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_blockers(&self, id: &TaskId) -> AppResult<Vec<Task>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT t.id, t.project_id, t.category, t.title, t.description, t.priority, t.internal_status, t.needs_review_point, t.source_proposal_id, t.plan_artifact_id, t.created_at, t.updated_at, t.started_at, t.completed_at, t.archived_at
                 FROM tasks t
                 INNER JOIN task_blockers tb ON t.id = tb.blocker_id
                 WHERE tb.task_id = ?1",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let tasks = stmt
            .query_map([id.as_str()], Task::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(tasks)
    }

    async fn get_dependents(&self, id: &TaskId) -> AppResult<Vec<Task>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT t.id, t.project_id, t.category, t.title, t.description, t.priority, t.internal_status, t.needs_review_point, t.source_proposal_id, t.plan_artifact_id, t.created_at, t.updated_at, t.started_at, t.completed_at, t.archived_at
                 FROM tasks t
                 INNER JOIN task_blockers tb ON t.id = tb.task_id
                 WHERE tb.blocker_id = ?1",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let tasks = stmt
            .query_map([id.as_str()], Task::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(tasks)
    }

    async fn add_blocker(&self, task_id: &TaskId, blocker_id: &TaskId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT INTO task_blockers (task_id, blocker_id) VALUES (?1, ?2)",
            rusqlite::params![task_id.as_str(), blocker_id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn resolve_blocker(&self, task_id: &TaskId, blocker_id: &TaskId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "DELETE FROM task_blockers WHERE task_id = ?1 AND blocker_id = ?2",
            rusqlite::params![task_id.as_str(), blocker_id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_by_project_filtered(
        &self,
        project_id: &ProjectId,
        include_archived: bool,
    ) -> AppResult<Vec<Task>> {
        let conn = self.conn.lock().await;

        let query = query_builder::build_filtered_query(include_archived);

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| AppError::Database(e.to_string()))?;

        let tasks = stmt
            .query_map([project_id.as_str()], Task::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(tasks)
    }

    async fn archive(&self, task_id: &TaskId) -> AppResult<Task> {
        let conn = self.conn.lock().await;
        let now = Utc::now();

        conn.execute(
            "UPDATE tasks SET archived_at = ?2, updated_at = ?3 WHERE id = ?1",
            rusqlite::params![task_id.as_str(), now.to_rfc3339(), now.to_rfc3339()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Fetch and return the updated task
        let result = conn.query_row(
            "SELECT id, project_id, category, title, description, priority, internal_status, needs_review_point, source_proposal_id, plan_artifact_id, created_at, updated_at, started_at, completed_at, archived_at
             FROM tasks WHERE id = ?1",
            [task_id.as_str()],
            |row| Task::from_row(row),
        );

        match result {
            Ok(task) => Ok(task),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn restore(&self, task_id: &TaskId) -> AppResult<Task> {
        let conn = self.conn.lock().await;
        let now = Utc::now();

        conn.execute(
            "UPDATE tasks SET archived_at = NULL, updated_at = ?2 WHERE id = ?1",
            rusqlite::params![task_id.as_str(), now.to_rfc3339()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Fetch and return the updated task
        let result = conn.query_row(
            "SELECT id, project_id, category, title, description, priority, internal_status, needs_review_point, source_proposal_id, plan_artifact_id, created_at, updated_at, started_at, completed_at, archived_at
             FROM tasks WHERE id = ?1",
            [task_id.as_str()],
            |row| Task::from_row(row),
        );

        match result {
            Ok(task) => Ok(task),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_archived_count(&self, project_id: &ProjectId) -> AppResult<u32> {
        let conn = self.conn.lock().await;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks WHERE project_id = ?1 AND archived_at IS NOT NULL",
                [project_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count as u32)
    }

    async fn list_paginated(
        &self,
        project_id: &ProjectId,
        statuses: Option<Vec<InternalStatus>>,
        offset: u32,
        limit: u32,
        include_archived: bool,
    ) -> AppResult<Vec<Task>> {
        let conn = self.conn.lock().await;

        let status_count = statuses.as_ref().map_or(0, |s| s.len());
        let query = query_builder::build_paginated_query(status_count, include_archived);

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| AppError::Database(e.to_string()))?;

        let tasks = if let Some(ref status_vec) = statuses {
            // Build params: project_id, status1, status2, ..., limit, offset
            let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
            params.push(Box::new(project_id.as_str().to_string()));
            for s in status_vec {
                params.push(Box::new(s.as_str().to_string()));
            }
            params.push(Box::new(limit as i64));
            params.push(Box::new(offset as i64));

            let params_ref: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

            stmt.query_map(params_ref.as_slice(), Task::from_row)
                .map_err(|e| AppError::Database(e.to_string()))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| AppError::Database(e.to_string()))?
        } else {
            stmt.query_map(
                rusqlite::params![project_id.as_str(), limit as i64, offset as i64],
                Task::from_row,
            )
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?
        };

        Ok(tasks)
    }

    async fn count_tasks(
        &self,
        project_id: &ProjectId,
        include_archived: bool,
    ) -> AppResult<u32> {
        let conn = self.conn.lock().await;

        let query = if include_archived {
            "SELECT COUNT(*) FROM tasks WHERE project_id = ?1"
        } else {
            "SELECT COUNT(*) FROM tasks WHERE project_id = ?1 AND archived_at IS NULL"
        };

        let count: i64 = conn
            .query_row(query, [project_id.as_str()], |row| row.get(0))
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count as u32)
    }

    async fn search(
        &self,
        project_id: &ProjectId,
        query: &str,
        include_archived: bool,
    ) -> AppResult<Vec<Task>> {
        let conn = self.conn.lock().await;

        let sql_query = query_builder::build_search_query(include_archived);
        let search_pattern = format!("%{}%", query);

        let mut stmt = conn
            .prepare(&sql_query)
            .map_err(|e| AppError::Database(e.to_string()))?;

        let tasks = stmt
            .query_map(
                rusqlite::params![project_id.as_str(), &search_pattern],
                Task::from_row,
            )
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(tasks)
    }

    async fn get_oldest_ready_task(&self) -> AppResult<Option<Task>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(queries::GET_OLDEST_READY_TASK, [], |row| Task::from_row(row));

        match result {
            Ok(task) => Ok(Some(task)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }
}


#[cfg(test)]
mod tests;
