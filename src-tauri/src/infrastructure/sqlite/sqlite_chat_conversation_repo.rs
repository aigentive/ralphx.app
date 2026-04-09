// SQLite-based ChatConversationRepository implementation
// Uses DbConnection for non-blocking SQLite access via spawn_blocking

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use rusqlite::Connection;

use crate::domain::agents::{AgentHarnessKind, ProviderSessionRef};
use crate::domain::entities::{
    legacy_claude_session_alias, ChatContextType, ChatConversation, ChatConversationId,
};
use crate::domain::repositories::ChatConversationRepository;
use crate::error::AppResult;
use crate::infrastructure::sqlite::DbConnection;

/// Parse datetime string handling both RFC3339 and SQLite's CURRENT_TIMESTAMP formats
fn parse_datetime(s: &str) -> DateTime<Utc> {
    // Try RFC3339 first (e.g., "2026-01-26T06:42:37.662598+00:00")
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return dt.with_timezone(&Utc);
    }

    // Try SQLite's CURRENT_TIMESTAMP format (e.g., "2026-01-26 07:06:32")
    if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Utc.from_utc_datetime(&ndt);
    }

    // Fallback to now
    Utc::now()
}

fn row_to_conversation(row: &rusqlite::Row) -> rusqlite::Result<ChatConversation> {
    let context_type_str: String = row.get("context_type")?;
    let claude_session_id: Option<String> = row.get("claude_session_id")?;
    let provider_session_id: Option<String> = row.get("provider_session_id")?;
    let provider_harness = row
        .get::<_, Option<String>>("provider_harness")?
        .and_then(|value| value.parse::<AgentHarnessKind>().ok());
    let last_message_at_str: Option<String> = row.get("last_message_at")?;
    let created_at_str: String = row.get("created_at")?;
    let updated_at_str: String = row.get("updated_at")?;

    let created_at = parse_datetime(&created_at_str);
    let updated_at = parse_datetime(&updated_at_str);

    let mut conversation = ChatConversation {
        id: ChatConversationId::from_string(row.get::<_, String>("id")?),
        context_type: context_type_str.parse().unwrap_or(ChatContextType::Ideation),
        context_id: row.get("context_id")?,
        claude_session_id,
        provider_session_id,
        provider_harness,
        title: row.get("title")?,
        message_count: row.get("message_count")?,
        last_message_at: last_message_at_str.and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
        }),
        created_at,
        updated_at,
        parent_conversation_id: row.get("parent_conversation_id")?,
    };
    conversation.normalize_provider_session_fields();
    Ok(conversation)
}

/// SQLite implementation of ChatConversationRepository
pub struct SqliteChatConversationRepository {
    db: DbConnection,
}

impl SqliteChatConversationRepository {
    /// Create a new SQLite chat conversation repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            db: DbConnection::new(conn),
        }
    }

    /// Create from an Arc-wrapped mutex connection (for sharing)
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            db: DbConnection::from_shared(conn),
        }
    }
}

#[async_trait]
impl ChatConversationRepository for SqliteChatConversationRepository {
    async fn create(&self, conversation: ChatConversation) -> AppResult<ChatConversation> {
        let id = conversation.id.as_str().to_string();
        let context_type = conversation.context_type.to_string();
        let context_id = conversation.context_id.clone();
        let claude_session_id = conversation.claude_session_id.clone();
        let provider_session_id = conversation.provider_session_id.clone();
        let provider_harness = conversation.provider_harness.map(|value| value.to_string());
        let title = conversation.title.clone();
        let message_count = conversation.message_count;
        let last_message_at = conversation.last_message_at.map(|dt| dt.to_rfc3339());
        let created_at = conversation.created_at.to_rfc3339();
        let updated_at = conversation.updated_at.to_rfc3339();
        let parent_conversation_id = conversation.parent_conversation_id.clone();

        self.db.run(move |conn| {
            conn.execute(
                "INSERT INTO chat_conversations (
                    id, context_type, context_id, claude_session_id, provider_session_id,
                    provider_harness, title, message_count, last_message_at, created_at,
                    updated_at, parent_conversation_id
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                rusqlite::params![
                    id, context_type, context_id, claude_session_id, provider_session_id,
                    provider_harness, title, message_count, last_message_at, created_at,
                    updated_at, parent_conversation_id,
                ],
            )?;
            Ok(())
        }).await?;

        Ok(conversation)
    }

    async fn get_by_id(&self, id: &ChatConversationId) -> AppResult<Option<ChatConversation>> {
        let id_str = id.as_str().to_string();
        self.db.query_optional(move |conn| {
            conn.query_row(
                "SELECT id, context_type, context_id, claude_session_id, provider_session_id,
                        provider_harness, title, message_count, last_message_at, created_at,
                        updated_at, parent_conversation_id
                 FROM chat_conversations WHERE id = ?1",
                [&id_str],
                row_to_conversation,
            )
        }).await
    }

    async fn get_by_context(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> AppResult<Vec<ChatConversation>> {
        let context_type_str = context_type.to_string();
        let context_id_str = context_id.to_string();
        self.db.run(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, context_type, context_id, claude_session_id, provider_session_id,
                        provider_harness, title, message_count, last_message_at, created_at,
                        updated_at, parent_conversation_id
                 FROM chat_conversations WHERE context_type = ?1 AND context_id = ?2 ORDER BY created_at DESC",
            )?;
            let conversations = stmt
                .query_map([context_type_str, context_id_str], row_to_conversation)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(conversations)
        }).await
    }

    async fn get_active_for_context(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> AppResult<Option<ChatConversation>> {
        let context_type_str = context_type.to_string();
        let context_id_str = context_id.to_string();
        self.db.query_optional(move |conn| {
            conn.query_row(
                "SELECT id, context_type, context_id, claude_session_id, provider_session_id,
                        provider_harness, title, message_count, last_message_at, created_at,
                        updated_at, parent_conversation_id
                 FROM chat_conversations WHERE context_type = ?1 AND context_id = ?2 ORDER BY created_at DESC LIMIT 1",
                [context_type_str, context_id_str],
                row_to_conversation,
            )
        }).await
    }

    async fn update_provider_session_ref(
        &self,
        id: &ChatConversationId,
        session_ref: &ProviderSessionRef,
    ) -> AppResult<()> {
        let id_str = id.as_str().to_string();
        let session_id = session_ref.provider_session_id.clone();
        let harness = session_ref.harness.to_string();
        let claude_session_id =
            legacy_claude_session_alias(Some(session_ref.harness), Some(session_id.as_str()));
        self.db.run(move |conn| {
            conn.execute(
                "UPDATE chat_conversations
                 SET claude_session_id = ?1,
                     provider_session_id = ?2,
                     provider_harness = ?3,
                     updated_at = ?4
                 WHERE id = ?5",
                rusqlite::params![
                    claude_session_id,
                    session_id,
                    harness,
                    Utc::now().to_rfc3339(),
                    id_str
                ],
            )?;
            Ok(())
        }).await
    }

    async fn clear_provider_session_ref(&self, id: &ChatConversationId) -> AppResult<()> {
        let id_str = id.as_str().to_string();
        self.db.run(move |conn| {
            conn.execute(
                "UPDATE chat_conversations
                 SET claude_session_id = NULL,
                     provider_session_id = NULL,
                     provider_harness = NULL,
                     updated_at = ?1
                 WHERE id = ?2",
                rusqlite::params![Utc::now().to_rfc3339(), id_str],
            )?;
            Ok(())
        }).await
    }

    async fn update_title(&self, id: &ChatConversationId, title: &str) -> AppResult<()> {
        let id_str = id.as_str().to_string();
        let title = title.to_string();
        self.db.run(move |conn| {
            conn.execute(
                "UPDATE chat_conversations SET title = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![title, Utc::now().to_rfc3339(), id_str],
            )?;
            Ok(())
        }).await
    }

    async fn update_message_stats(
        &self,
        id: &ChatConversationId,
        message_count: i64,
        last_message_at: chrono::DateTime<chrono::Utc>,
    ) -> AppResult<()> {
        let id_str = id.as_str().to_string();
        let last_message_at_str = last_message_at.to_rfc3339();
        self.db.run(move |conn| {
            conn.execute(
                "UPDATE chat_conversations SET message_count = ?1, last_message_at = ?2, updated_at = ?3 WHERE id = ?4",
                rusqlite::params![message_count, last_message_at_str, Utc::now().to_rfc3339(), id_str],
            )?;
            Ok(())
        }).await
    }

    async fn delete(&self, id: &ChatConversationId) -> AppResult<()> {
        let id_str = id.as_str().to_string();
        self.db.run(move |conn| {
            conn.execute("DELETE FROM chat_conversations WHERE id = ?1", [id_str])?;
            Ok(())
        }).await
    }

    async fn delete_by_context(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> AppResult<()> {
        let context_type_str = context_type.to_string();
        let context_id_str = context_id.to_string();
        self.db.run(move |conn| {
            conn.execute(
                "DELETE FROM chat_conversations WHERE context_type = ?1 AND context_id = ?2",
                [context_type_str, context_id_str],
            )?;
            Ok(())
        }).await
    }
}
