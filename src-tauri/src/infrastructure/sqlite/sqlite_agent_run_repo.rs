// SQLite-based AgentRunRepository implementation
// Uses rusqlite with connection pooling for thread-safe access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use rusqlite::Connection;

use crate::domain::entities::{AgentRun, AgentRunId, AgentRunStatus, ChatConversationId};
use crate::domain::repositories::AgentRunRepository;
use crate::error::{AppError, AppResult};

/// SQLite implementation of AgentRunRepository
pub struct SqliteAgentRunRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteAgentRunRepository {
    /// Create a new SQLite agent run repository with the given connection
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
impl AgentRunRepository for SqliteAgentRunRepository {
    async fn create(&self, run: AgentRun) -> AppResult<AgentRun> {
        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT INTO agent_runs (id, conversation_id, status, started_at, completed_at, error_message)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                run.id.as_str(),
                run.conversation_id.as_str(),
                run.status.to_string(),
                run.started_at.to_rfc3339(),
                run.completed_at.map(|dt| dt.to_rfc3339()),
                run.error_message,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(run)
    }

    async fn get_by_id(&self, id: &AgentRunId) -> AppResult<Option<AgentRun>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, conversation_id, status, started_at, completed_at, error_message
             FROM agent_runs WHERE id = ?1",
            [id.as_str()],
            |row| {
                let status_str: String = row.get("status")?;
                let started_at_str: String = row.get("started_at")?;
                let completed_at_str: Option<String> = row.get("completed_at")?;

                Ok(AgentRun {
                    id: AgentRunId::from_string(row.get::<_, String>("id")?),
                    conversation_id: ChatConversationId::from_string(row.get::<_, String>("conversation_id")?),
                    status: status_str.parse().unwrap_or(AgentRunStatus::Failed),
                    started_at: chrono::DateTime::parse_from_rfc3339(&started_at_str).unwrap().with_timezone(&Utc),
                    completed_at: completed_at_str.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))),
                    error_message: row.get("error_message")?,
                })
            },
        );

        match result {
            Ok(run) => Ok(Some(run)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_latest_for_conversation(
        &self,
        conversation_id: &ChatConversationId,
    ) -> AppResult<Option<AgentRun>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, conversation_id, status, started_at, completed_at, error_message
             FROM agent_runs WHERE conversation_id = ?1 ORDER BY started_at DESC LIMIT 1",
            [conversation_id.as_str()],
            |row| {
                let status_str: String = row.get("status")?;
                let started_at_str: String = row.get("started_at")?;
                let completed_at_str: Option<String> = row.get("completed_at")?;

                Ok(AgentRun {
                    id: AgentRunId::from_string(row.get::<_, String>("id")?),
                    conversation_id: ChatConversationId::from_string(row.get::<_, String>("conversation_id")?),
                    status: status_str.parse().unwrap_or(AgentRunStatus::Failed),
                    started_at: chrono::DateTime::parse_from_rfc3339(&started_at_str).unwrap().with_timezone(&Utc),
                    completed_at: completed_at_str.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))),
                    error_message: row.get("error_message")?,
                })
            },
        );

        match result {
            Ok(run) => Ok(Some(run)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_active_for_conversation(
        &self,
        conversation_id: &ChatConversationId,
    ) -> AppResult<Option<AgentRun>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, conversation_id, status, started_at, completed_at, error_message
             FROM agent_runs WHERE conversation_id = ?1 AND status = 'running' ORDER BY started_at DESC LIMIT 1",
            [conversation_id.as_str()],
            |row| {
                let status_str: String = row.get("status")?;
                let started_at_str: String = row.get("started_at")?;
                let completed_at_str: Option<String> = row.get("completed_at")?;

                Ok(AgentRun {
                    id: AgentRunId::from_string(row.get::<_, String>("id")?),
                    conversation_id: ChatConversationId::from_string(row.get::<_, String>("conversation_id")?),
                    status: status_str.parse().unwrap_or(AgentRunStatus::Failed),
                    started_at: chrono::DateTime::parse_from_rfc3339(&started_at_str).unwrap().with_timezone(&Utc),
                    completed_at: completed_at_str.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))),
                    error_message: row.get("error_message")?,
                })
            },
        );

        match result {
            Ok(run) => Ok(Some(run)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_by_conversation(
        &self,
        conversation_id: &ChatConversationId,
    ) -> AppResult<Vec<AgentRun>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, conversation_id, status, started_at, completed_at, error_message
                 FROM agent_runs WHERE conversation_id = ?1 ORDER BY started_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let runs = stmt
            .query_map([conversation_id.as_str()], |row| {
                let status_str: String = row.get("status")?;
                let started_at_str: String = row.get("started_at")?;
                let completed_at_str: Option<String> = row.get("completed_at")?;

                Ok(AgentRun {
                    id: AgentRunId::from_string(row.get::<_, String>("id")?),
                    conversation_id: ChatConversationId::from_string(row.get::<_, String>("conversation_id")?),
                    status: status_str.parse().unwrap_or(AgentRunStatus::Failed),
                    started_at: chrono::DateTime::parse_from_rfc3339(&started_at_str).unwrap().with_timezone(&Utc),
                    completed_at: completed_at_str.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))),
                    error_message: row.get("error_message")?,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(runs)
    }

    async fn update_status(&self, id: &AgentRunId, status: AgentRunStatus) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "UPDATE agent_runs SET status = ?1 WHERE id = ?2",
            rusqlite::params![status.to_string(), id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn complete(&self, id: &AgentRunId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "UPDATE agent_runs SET status = 'completed', completed_at = ?1, error_message = NULL WHERE id = ?2",
            rusqlite::params![Utc::now().to_rfc3339(), id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn fail(&self, id: &AgentRunId, error_message: &str) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "UPDATE agent_runs SET status = 'failed', completed_at = ?1, error_message = ?2 WHERE id = ?3",
            rusqlite::params![Utc::now().to_rfc3339(), error_message, id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn cancel(&self, id: &AgentRunId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "UPDATE agent_runs SET status = 'cancelled', completed_at = ?1, error_message = NULL WHERE id = ?2",
            rusqlite::params![Utc::now().to_rfc3339(), id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: &AgentRunId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute("DELETE FROM agent_runs WHERE id = ?1", [id.as_str()])
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete_by_conversation(&self, conversation_id: &ChatConversationId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute("DELETE FROM agent_runs WHERE conversation_id = ?1", [conversation_id.as_str()])
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn count_by_status(
        &self,
        conversation_id: &ChatConversationId,
        status: AgentRunStatus,
    ) -> AppResult<u32> {
        let conn = self.conn.lock().await;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM agent_runs WHERE conversation_id = ?1 AND status = ?2",
                [conversation_id.as_str(), status.to_string()],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count as u32)
    }

    async fn cancel_all_running(&self) -> AppResult<u32> {
        let conn = self.conn.lock().await;

        let changes = conn
            .execute(
                "UPDATE agent_runs SET status = 'cancelled', completed_at = ?1, error_message = 'Orphaned on app restart' WHERE status = 'running'",
                rusqlite::params![Utc::now().to_rfc3339()],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(changes as u32)
    }
}
