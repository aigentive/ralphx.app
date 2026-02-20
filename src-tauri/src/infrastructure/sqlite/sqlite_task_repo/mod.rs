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

use crate::domain::entities::{IdeationSessionId, InternalStatus, ProjectId, Task, TaskId};
use crate::domain::repositories::{StateHistoryMetadata, StatusTransition, TaskRepository};
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
            "INSERT INTO tasks (id, project_id, category, title, description, priority, internal_status, needs_review_point, source_proposal_id, plan_artifact_id, ideation_session_id, created_at, updated_at, started_at, completed_at, archived_at, blocked_reason, task_branch, worktree_path, merge_commit_sha, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            rusqlite::params![
                task.id.as_str(),
                task.project_id.as_str(),
                task.category.to_string(),
                task.title,
                task.description,
                task.priority,
                task.internal_status.as_str(),
                task.needs_review_point,
                task.source_proposal_id.as_ref().map(|id| id.as_str()),
                task.plan_artifact_id.as_ref().map(|id| id.as_str()),
                task.ideation_session_id.as_ref().map(|id| id.as_str()),
                task.created_at.to_rfc3339(),
                task.updated_at.to_rfc3339(),
                task.started_at.map(|dt| dt.to_rfc3339()),
                task.completed_at.map(|dt| dt.to_rfc3339()),
                task.archived_at.map(|dt| dt.to_rfc3339()),
                task.blocked_reason,
                task.task_branch,
                task.worktree_path,
                task.merge_commit_sha,
                task.metadata,
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

    async fn get_by_ideation_session(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<Vec<Task>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(queries::GET_BY_IDEATION_SESSION)
            .map_err(|e| AppError::Database(e.to_string()))?;

        let tasks = stmt
            .query_map([session_id.as_str()], Task::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(tasks)
    }

    async fn update(&self, task: &Task) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "UPDATE tasks SET project_id = ?2, category = ?3, title = ?4, description = ?5, priority = ?6, internal_status = ?7, source_proposal_id = ?8, plan_artifact_id = ?9, ideation_session_id = ?10, updated_at = ?11, started_at = ?12, completed_at = ?13, archived_at = ?14, blocked_reason = ?15, task_branch = ?16, worktree_path = ?17, merge_commit_sha = ?18, metadata = ?19
             WHERE id = ?1",
            rusqlite::params![
                task.id.as_str(),
                task.project_id.as_str(),
                task.category.to_string(),
                task.title,
                task.description,
                task.priority,
                task.internal_status.as_str(),
                task.source_proposal_id.as_ref().map(|id| id.as_str()),
                task.plan_artifact_id.as_ref().map(|id| id.as_str()),
                task.ideation_session_id.as_ref().map(|id| id.as_str()),
                task.updated_at.to_rfc3339(),
                task.started_at.map(|dt| dt.to_rfc3339()),
                task.completed_at.map(|dt| dt.to_rfc3339()),
                task.archived_at.map(|dt| dt.to_rfc3339()),
                task.blocked_reason,
                task.task_branch,
                task.worktree_path,
                task.merge_commit_sha,
                task.metadata,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_with_expected_status(
        &self,
        task: &Task,
        expected_status: InternalStatus,
    ) -> AppResult<bool> {
        let conn = self.conn.lock().await;

        let rows_affected = conn.execute(
            "UPDATE tasks SET project_id = ?2, category = ?3, title = ?4, description = ?5, priority = ?6, internal_status = ?7, source_proposal_id = ?8, plan_artifact_id = ?9, ideation_session_id = ?10, updated_at = ?11, started_at = ?12, completed_at = ?13, archived_at = ?14, blocked_reason = ?15, task_branch = ?16, worktree_path = ?17, merge_commit_sha = ?18, metadata = ?19
             WHERE id = ?1 AND internal_status = ?20",
            rusqlite::params![
                task.id.as_str(),
                task.project_id.as_str(),
                task.category.to_string(),
                task.title,
                task.description,
                task.priority,
                task.internal_status.as_str(),
                task.source_proposal_id.as_ref().map(|id| id.as_str()),
                task.plan_artifact_id.as_ref().map(|id| id.as_str()),
                task.ideation_session_id.as_ref().map(|id| id.as_str()),
                task.updated_at.to_rfc3339(),
                task.started_at.map(|dt| dt.to_rfc3339()),
                task.completed_at.map(|dt| dt.to_rfc3339()),
                task.archived_at.map(|dt| dt.to_rfc3339()),
                task.blocked_reason,
                task.task_branch,
                task.worktree_path,
                task.merge_commit_sha,
                task.metadata,
                expected_status.as_str(),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows_affected > 0)
    }

    async fn update_metadata(&self, id: &TaskId, metadata: Option<String>) -> AppResult<()> {
        let conn = self.conn.lock().await;
        let now = Utc::now();

        conn.execute(
            "UPDATE tasks SET metadata = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![metadata, now.to_rfc3339(), id.as_str()],
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

    async fn clear_task_references(&self, id: &TaskId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        // Clear task_proposals.created_task_id
        conn.execute(queries::CLEAR_TASK_PROPOSAL_REFERENCES, [id.as_str()])
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Clear artifacts.task_id
        conn.execute(queries::CLEAR_ARTIFACT_REFERENCES, [id.as_str()])
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
                "SELECT id, project_id, category, title, description, priority, internal_status, needs_review_point, source_proposal_id, plan_artifact_id, ideation_session_id, created_at, updated_at, started_at, completed_at, archived_at, blocked_reason, task_branch, worktree_path, merge_commit_sha, metadata
                 FROM tasks WHERE project_id = ?1 AND internal_status = ?2 AND archived_at IS NULL
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
                "SELECT from_status, to_status, changed_by, created_at, metadata
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
                let metadata_json: Option<String> = row.get(4)?;

                let from = from_str.parse().unwrap_or(InternalStatus::Backlog);
                let to = to_str.parse().unwrap_or(InternalStatus::Backlog);
                let timestamp = Task::parse_datetime(created_at_str);

                // Parse metadata JSON to extract conversation_id and agent_run_id
                let (conversation_id, agent_run_id) = metadata_json
                    .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
                    .map(|v| {
                        let conv_id = v
                            .get("conversation_id")
                            .and_then(|v| v.as_str())
                            .map(String::from);
                        let run_id = v
                            .get("agent_run_id")
                            .and_then(|v| v.as_str())
                            .map(String::from);
                        (conv_id, run_id)
                    })
                    .unwrap_or((None, None));

                Ok(StatusTransition::with_metadata(
                    from,
                    to,
                    trigger,
                    timestamp,
                    conversation_id,
                    agent_run_id,
                ))
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(transitions)
    }

    async fn get_status_entered_at(
        &self,
        task_id: &TaskId,
        status: InternalStatus,
    ) -> AppResult<Option<chrono::DateTime<Utc>>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT created_at
             FROM task_state_history
             WHERE task_id = ?1 AND to_status = ?2
             ORDER BY created_at ASC
             LIMIT 1",
            rusqlite::params![task_id.as_str(), status.as_str()],
            |row| {
                let created_at_str: String = row.get(0)?;
                Ok(created_at_str)
            },
        );

        match result {
            Ok(timestamp_str) => {
                let timestamp = Task::parse_datetime(timestamp_str);
                Ok(Some(timestamp))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_next_executable(&self, project_id: &ProjectId) -> AppResult<Option<Task>> {
        let conn = self.conn.lock().await;

        // Find READY tasks that have no unsatisfied dependencies
        let result = conn.query_row(
            "SELECT t.id, t.project_id, t.category, t.title, t.description, t.priority, t.internal_status, t.needs_review_point, t.source_proposal_id, t.plan_artifact_id, t.ideation_session_id, t.created_at, t.updated_at, t.started_at, t.completed_at, t.archived_at, t.blocked_reason, t.task_branch, t.worktree_path, t.merge_commit_sha, t.metadata
             FROM tasks t
             WHERE t.project_id = ?1
               AND t.internal_status = 'ready'
               AND NOT EXISTS (
                   SELECT 1 FROM task_dependencies td
                   JOIN tasks blocker ON blocker.id = td.depends_on_task_id
                   WHERE td.task_id = t.id
                   AND blocker.internal_status NOT IN ('merged', 'cancelled', 'merge_incomplete')
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
            "SELECT id, project_id, category, title, description, priority, internal_status, needs_review_point, source_proposal_id, plan_artifact_id, ideation_session_id, created_at, updated_at, started_at, completed_at, archived_at, blocked_reason, task_branch, worktree_path, merge_commit_sha, metadata
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
            "SELECT id, project_id, category, title, description, priority, internal_status, needs_review_point, source_proposal_id, plan_artifact_id, ideation_session_id, created_at, updated_at, started_at, completed_at, archived_at, blocked_reason, task_branch, worktree_path, merge_commit_sha, metadata
             FROM tasks WHERE id = ?1",
            [task_id.as_str()],
            |row| Task::from_row(row),
        );

        match result {
            Ok(task) => Ok(task),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_archived_count(
        &self,
        project_id: &ProjectId,
        ideation_session_id: Option<&str>,
    ) -> AppResult<u32> {
        let conn = self.conn.lock().await;

        let (query, params): (String, Vec<Box<dyn rusqlite::ToSql>>) = if let Some(sid) =
            ideation_session_id
        {
            (
                "SELECT COUNT(*) FROM tasks WHERE project_id = ?1 AND archived_at IS NOT NULL AND ideation_session_id = ?2".to_string(),
                vec![Box::new(project_id.as_str().to_string()), Box::new(sid.to_string())]
            )
        } else {
            (
                "SELECT COUNT(*) FROM tasks WHERE project_id = ?1 AND archived_at IS NOT NULL"
                    .to_string(),
                vec![Box::new(project_id.as_str().to_string())],
            )
        };

        let params_ref: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let count: i64 = conn
            .query_row(&query, params_ref.as_slice(), |row| row.get(0))
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
        ideation_session_id: Option<&str>,
    ) -> AppResult<Vec<Task>> {
        let conn = self.conn.lock().await;

        let status_count = statuses.as_ref().map_or(0, |s| s.len());
        let has_session_filter = ideation_session_id.is_some();
        let query = query_builder::build_paginated_query(
            status_count,
            include_archived,
            has_session_filter,
        );

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Build params: project_id, [statuses...], [session_id?], limit, offset
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        params.push(Box::new(project_id.as_str().to_string()));

        if let Some(ref status_vec) = statuses {
            for s in status_vec {
                params.push(Box::new(s.as_str().to_string()));
            }
        }

        if let Some(sid) = ideation_session_id {
            params.push(Box::new(sid.to_string()));
        }

        params.push(Box::new(limit as i64));
        params.push(Box::new(offset as i64));

        let params_ref: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let tasks = stmt
            .query_map(params_ref.as_slice(), Task::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(tasks)
    }

    async fn count_tasks(
        &self,
        project_id: &ProjectId,
        include_archived: bool,
        ideation_session_id: Option<&str>,
    ) -> AppResult<u32> {
        let conn = self.conn.lock().await;

        let (query, params): (String, Vec<Box<dyn rusqlite::ToSql>>) = match (include_archived, ideation_session_id) {
            (true, Some(sid)) => (
                "SELECT COUNT(*) FROM tasks WHERE project_id = ?1 AND ideation_session_id = ?2".to_string(),
                vec![Box::new(project_id.as_str().to_string()), Box::new(sid.to_string())]
            ),
            (true, None) => (
                "SELECT COUNT(*) FROM tasks WHERE project_id = ?1".to_string(),
                vec![Box::new(project_id.as_str().to_string())]
            ),
            (false, Some(sid)) => (
                "SELECT COUNT(*) FROM tasks WHERE project_id = ?1 AND archived_at IS NULL AND ideation_session_id = ?2".to_string(),
                vec![Box::new(project_id.as_str().to_string()), Box::new(sid.to_string())]
            ),
            (false, None) => (
                "SELECT COUNT(*) FROM tasks WHERE project_id = ?1 AND archived_at IS NULL".to_string(),
                vec![Box::new(project_id.as_str().to_string())]
            ),
        };

        let params_ref: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let count: i64 = conn
            .query_row(&query, params_ref.as_slice(), |row| row.get(0))
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

        let result = conn.query_row(queries::GET_OLDEST_READY_TASK, [], |row| {
            Task::from_row(row)
        });

        match result {
            Ok(task) => Ok(Some(task)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_oldest_ready_tasks(&self, limit: u32) -> AppResult<Vec<Task>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(queries::GET_OLDEST_READY_TASKS)
            .map_err(|e| AppError::Database(e.to_string()))?;

        let tasks = stmt
            .query_map([limit as i64], Task::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(tasks)
    }

    async fn get_stale_ready_tasks(&self, threshold_secs: u64) -> AppResult<Vec<Task>> {
        use chrono::{Duration, Utc};

        let cutoff = Utc::now() - Duration::seconds(threshold_secs as i64);
        let cutoff_str = cutoff.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string();

        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(queries::GET_STALE_READY_TASKS)
            .map_err(|e| AppError::Database(e.to_string()))?;

        let tasks = stmt
            .query_map([cutoff_str], Task::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(tasks)
    }

    async fn update_latest_state_history_metadata(
        &self,
        task_id: &TaskId,
        metadata: &StateHistoryMetadata,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;
        helpers::update_latest_state_history_metadata_sync(&conn, task_id, metadata)
    }

    async fn has_task_in_states(
        &self,
        project_id: &ProjectId,
        statuses: &[InternalStatus],
    ) -> AppResult<bool> {
        if statuses.is_empty() {
            return Ok(false);
        }

        let conn = self.conn.lock().await;

        // Build a query with placeholders for each status
        let placeholders: Vec<String> = (2..=statuses.len() + 1)
            .map(|i| format!("?{}", i))
            .collect();
        let placeholders_str = placeholders.join(", ");

        let query = format!(
            "SELECT 1 FROM tasks
             WHERE project_id = ?1
               AND internal_status IN ({})
               AND archived_at IS NULL
             LIMIT 1",
            placeholders_str
        );

        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        params.push(Box::new(project_id.as_str().to_string()));
        for status in statuses {
            params.push(Box::new(status.as_str().to_string()));
        }

        let params_ref: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let result: rusqlite::Result<i32> =
            conn.query_row(&query, params_ref.as_slice(), |row| row.get(0));

        match result {
            Ok(_) => Ok(true),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests;
