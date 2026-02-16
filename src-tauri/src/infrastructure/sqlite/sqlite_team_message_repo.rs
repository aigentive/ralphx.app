// SQLite-based TeamMessageRepository implementation
// Uses rusqlite with mutex-protected connection for thread-safe access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use rusqlite::Connection;

use crate::domain::entities::team::{TeamMessageId, TeamMessageRecord, TeamSessionId};
use crate::domain::repositories::TeamMessageRepository;
use crate::error::{AppError, AppResult};

fn parse_datetime(s: &str) -> DateTime<Utc> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return dt.with_timezone(&Utc);
    }
    if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Utc.from_utc_datetime(&ndt);
    }
    Utc::now()
}

fn row_to_message(row: &rusqlite::Row<'_>) -> rusqlite::Result<TeamMessageRecord> {
    let created_at_str: String = row.get("created_at")?;
    Ok(TeamMessageRecord {
        id: TeamMessageId::from_string(row.get::<_, String>("id")?),
        team_session_id: TeamSessionId::from_string(row.get::<_, String>("team_session_id")?),
        sender: row.get("sender")?,
        recipient: row.get("recipient")?,
        content: row.get("content")?,
        message_type: row.get("message_type")?,
        created_at: parse_datetime(&created_at_str),
    })
}

pub struct SqliteTeamMessageRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteTeamMessageRepository {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl TeamMessageRepository for SqliteTeamMessageRepository {
    async fn create(&self, message: TeamMessageRecord) -> AppResult<TeamMessageRecord> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO team_messages (id, team_session_id, sender, recipient, content, message_type, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                message.id.as_str(),
                message.team_session_id.as_str(),
                message.sender,
                message.recipient,
                message.content,
                message.message_type,
                message.created_at.to_rfc3339(),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(message)
    }

    async fn get_by_session(
        &self,
        session_id: &TeamSessionId,
    ) -> AppResult<Vec<TeamMessageRecord>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn
            .prepare(
                "SELECT id, team_session_id, sender, recipient, content, message_type, created_at
                 FROM team_messages WHERE team_session_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let messages = stmt
            .query_map([session_id.as_str()], row_to_message)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(messages)
    }

    async fn get_recent_by_session(
        &self,
        session_id: &TeamSessionId,
        limit: u32,
    ) -> AppResult<Vec<TeamMessageRecord>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn
            .prepare(
                "SELECT id, team_session_id, sender, recipient, content, message_type, created_at
                 FROM team_messages WHERE team_session_id = ?1 ORDER BY created_at DESC LIMIT ?2",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut messages: Vec<TeamMessageRecord> = stmt
            .query_map(rusqlite::params![session_id.as_str(), limit], row_to_message)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        messages.reverse();
        Ok(messages)
    }

    async fn count_by_session(&self, session_id: &TeamSessionId) -> AppResult<u32> {
        let conn = self.conn.lock().await;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM team_messages WHERE team_session_id = ?1",
                [session_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(count as u32)
    }

    async fn delete_by_session(&self, session_id: &TeamSessionId) -> AppResult<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "DELETE FROM team_messages WHERE team_session_id = ?1",
            [session_id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    async fn delete(&self, id: &TeamMessageId) -> AppResult<()> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM team_messages WHERE id = ?1", [id.as_str()])
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }
}
