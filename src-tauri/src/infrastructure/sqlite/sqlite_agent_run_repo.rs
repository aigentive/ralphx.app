// SQLite-based AgentRunRepository implementation
// Uses rusqlite with connection pooling for thread-safe access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use rusqlite::Connection;

use crate::domain::entities::{
    AgentRun, AgentRunId, AgentRunStatus, ChatContextType, ChatConversation,
    ChatConversationId, InterruptedConversation,
};
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

    async fn get_interrupted_conversations(&self) -> AppResult<Vec<InterruptedConversation>> {
        let conn = self.conn.lock().await;

        // Query joins chat_conversations with agent_runs to find:
        // - Conversations with a claude_session_id (can use --resume)
        // - Latest agent run is cancelled with "Orphaned on app restart" error
        let mut stmt = conn
            .prepare(
                "SELECT
                    c.id as conv_id,
                    c.context_type,
                    c.context_id,
                    c.claude_session_id,
                    c.title,
                    c.message_count,
                    c.last_message_at,
                    c.created_at as conv_created_at,
                    c.updated_at as conv_updated_at,
                    ar.id as run_id,
                    ar.conversation_id,
                    ar.status,
                    ar.started_at,
                    ar.completed_at,
                    ar.error_message
                FROM chat_conversations c
                INNER JOIN agent_runs ar ON c.id = ar.conversation_id
                WHERE c.claude_session_id IS NOT NULL
                  AND ar.status = 'cancelled'
                  AND ar.error_message = 'Orphaned on app restart'
                  AND ar.id = (
                    SELECT ar2.id FROM agent_runs ar2
                    WHERE ar2.conversation_id = c.id
                    ORDER BY ar2.started_at DESC LIMIT 1
                  )
                ORDER BY ar.started_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let results = stmt
            .query_map([], |row| {
                // Parse conversation fields
                let context_type_str: String = row.get("context_type")?;
                let conv_created_at_str: String = row.get("conv_created_at")?;
                let conv_updated_at_str: String = row.get("conv_updated_at")?;
                let last_message_at_str: Option<String> = row.get("last_message_at")?;

                let conversation = ChatConversation {
                    id: ChatConversationId::from_string(row.get::<_, String>("conv_id")?),
                    context_type: context_type_str.parse().unwrap_or(ChatContextType::Project),
                    context_id: row.get("context_id")?,
                    claude_session_id: row.get("claude_session_id")?,
                    title: row.get("title")?,
                    message_count: row.get("message_count")?,
                    last_message_at: last_message_at_str.and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    }),
                    created_at: chrono::DateTime::parse_from_rfc3339(&conv_created_at_str)
                        .unwrap()
                        .with_timezone(&Utc),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&conv_updated_at_str)
                        .unwrap()
                        .with_timezone(&Utc),
                };

                // Parse agent run fields
                let status_str: String = row.get("status")?;
                let started_at_str: String = row.get("started_at")?;
                let completed_at_str: Option<String> = row.get("completed_at")?;

                let last_run = AgentRun {
                    id: AgentRunId::from_string(row.get::<_, String>("run_id")?),
                    conversation_id: ChatConversationId::from_string(
                        row.get::<_, String>("conversation_id")?,
                    ),
                    status: status_str.parse().unwrap_or(AgentRunStatus::Cancelled),
                    started_at: chrono::DateTime::parse_from_rfc3339(&started_at_str)
                        .unwrap()
                        .with_timezone(&Utc),
                    completed_at: completed_at_str.and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    }),
                    error_message: row.get("error_message")?,
                };

                Ok(InterruptedConversation {
                    conversation,
                    last_run,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::IdeationSessionId;
    use crate::domain::repositories::ChatConversationRepository;
    use crate::infrastructure::sqlite::{
        open_memory_connection, run_migrations, SqliteChatConversationRepository,
    };

    #[tokio::test]
    async fn test_get_interrupted_conversations_returns_empty_when_none() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        let repo = SqliteAgentRunRepository::new(conn);

        let result = repo.get_interrupted_conversations().await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_get_interrupted_conversations_returns_orphaned_conversation() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        let shared_conn = Arc::new(Mutex::new(conn));

        let agent_run_repo = SqliteAgentRunRepository::from_shared(Arc::clone(&shared_conn));
        let conversation_repo =
            SqliteChatConversationRepository::from_shared(Arc::clone(&shared_conn));

        // Create a conversation with claude_session_id
        let mut conversation = ChatConversation::new_ideation(IdeationSessionId::new());
        conversation.claude_session_id = Some("test-session-id".to_string());
        conversation_repo.create(conversation.clone()).await.unwrap();

        // Create an agent run that gets orphaned
        let mut run = AgentRun::new(conversation.id);
        let run_id = run.id;
        run.status = AgentRunStatus::Cancelled;
        run.completed_at = Some(Utc::now());
        run.error_message = Some("Orphaned on app restart".to_string());
        agent_run_repo.create(run).await.unwrap();

        // Get interrupted conversations
        let result = agent_run_repo.get_interrupted_conversations().await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].conversation.id, conversation.id);
        assert_eq!(result[0].last_run.id, run_id);
        assert_eq!(result[0].last_run.status, AgentRunStatus::Cancelled);
        assert_eq!(
            result[0].last_run.error_message,
            Some("Orphaned on app restart".to_string())
        );
    }

    #[tokio::test]
    async fn test_get_interrupted_conversations_ignores_without_session_id() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        let shared_conn = Arc::new(Mutex::new(conn));

        let agent_run_repo = SqliteAgentRunRepository::from_shared(Arc::clone(&shared_conn));
        let conversation_repo =
            SqliteChatConversationRepository::from_shared(Arc::clone(&shared_conn));

        // Create a conversation WITHOUT claude_session_id
        let conversation = ChatConversation::new_ideation(IdeationSessionId::new());
        // Note: claude_session_id is None by default
        conversation_repo.create(conversation.clone()).await.unwrap();

        // Create an orphaned agent run
        let mut run = AgentRun::new(conversation.id);
        run.status = AgentRunStatus::Cancelled;
        run.completed_at = Some(Utc::now());
        run.error_message = Some("Orphaned on app restart".to_string());
        agent_run_repo.create(run).await.unwrap();

        // Should return empty because conversation has no claude_session_id
        let result = agent_run_repo.get_interrupted_conversations().await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_get_interrupted_conversations_ignores_completed_runs() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        let shared_conn = Arc::new(Mutex::new(conn));

        let agent_run_repo = SqliteAgentRunRepository::from_shared(Arc::clone(&shared_conn));
        let conversation_repo =
            SqliteChatConversationRepository::from_shared(Arc::clone(&shared_conn));

        // Create a conversation with claude_session_id
        let mut conversation = ChatConversation::new_ideation(IdeationSessionId::new());
        conversation.claude_session_id = Some("test-session-id".to_string());
        conversation_repo.create(conversation.clone()).await.unwrap();

        // Create a COMPLETED agent run (not orphaned)
        let mut run = AgentRun::new(conversation.id);
        run.status = AgentRunStatus::Completed;
        run.completed_at = Some(Utc::now());
        agent_run_repo.create(run).await.unwrap();

        // Should return empty because run is completed, not orphaned
        let result = agent_run_repo.get_interrupted_conversations().await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_get_interrupted_conversations_ignores_different_error_message() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        let shared_conn = Arc::new(Mutex::new(conn));

        let agent_run_repo = SqliteAgentRunRepository::from_shared(Arc::clone(&shared_conn));
        let conversation_repo =
            SqliteChatConversationRepository::from_shared(Arc::clone(&shared_conn));

        // Create a conversation with claude_session_id
        let mut conversation = ChatConversation::new_ideation(IdeationSessionId::new());
        conversation.claude_session_id = Some("test-session-id".to_string());
        conversation_repo.create(conversation.clone()).await.unwrap();

        // Create a cancelled run with DIFFERENT error message
        let mut run = AgentRun::new(conversation.id);
        run.status = AgentRunStatus::Cancelled;
        run.completed_at = Some(Utc::now());
        run.error_message = Some("User cancelled".to_string());
        agent_run_repo.create(run).await.unwrap();

        // Should return empty because error message doesn't match
        let result = agent_run_repo.get_interrupted_conversations().await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_get_interrupted_conversations_only_latest_run() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        let shared_conn = Arc::new(Mutex::new(conn));

        let agent_run_repo = SqliteAgentRunRepository::from_shared(Arc::clone(&shared_conn));
        let conversation_repo =
            SqliteChatConversationRepository::from_shared(Arc::clone(&shared_conn));

        // Create a conversation with claude_session_id
        let mut conversation = ChatConversation::new_ideation(IdeationSessionId::new());
        conversation.claude_session_id = Some("test-session-id".to_string());
        conversation_repo.create(conversation.clone()).await.unwrap();

        // Create an OLD orphaned run
        let mut old_run = AgentRun::new(conversation.id);
        old_run.status = AgentRunStatus::Cancelled;
        old_run.started_at = Utc::now() - chrono::Duration::hours(1);
        old_run.completed_at = Some(Utc::now() - chrono::Duration::hours(1));
        old_run.error_message = Some("Orphaned on app restart".to_string());
        agent_run_repo.create(old_run).await.unwrap();

        // Create a NEW completed run (the latest one)
        let mut new_run = AgentRun::new(conversation.id);
        new_run.status = AgentRunStatus::Completed;
        new_run.started_at = Utc::now();
        new_run.completed_at = Some(Utc::now());
        agent_run_repo.create(new_run).await.unwrap();

        // Should return empty because the LATEST run is completed, not orphaned
        let result = agent_run_repo.get_interrupted_conversations().await.unwrap();
        assert!(result.is_empty());
    }
}
