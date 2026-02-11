// SQLite implementation of MemoryEventRepository

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde_json::Value as JsonValue;
use std::sync::Mutex;

use crate::domain::entities::{MemoryActorType, MemoryEvent, MemoryEventId, ProcessId};
use crate::domain::repositories::MemoryEventRepository;
use crate::error::{AppError, AppResult};

/// SQLite-backed memory event repository
pub struct SqliteMemoryEventRepository {
    conn: Mutex<Connection>,
}

impl SqliteMemoryEventRepository {
    /// Create a new repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }

    /// Helper to parse a row into a MemoryEvent
    fn row_to_memory_event(row: &rusqlite::Row) -> rusqlite::Result<MemoryEvent> {
        let actor_type_str: String = row.get(3)?;
        let actor_type = actor_type_str.parse::<MemoryActorType>()
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                3,
                rusqlite::types::Type::Text,
                Box::new(e),
            ))?;

        let details_json_str: String = row.get(4)?;
        let details: JsonValue = serde_json::from_str(&details_json_str)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                4,
                rusqlite::types::Type::Text,
                Box::new(e),
            ))?;

        let created_at_str: String = row.get(5)?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                5,
                rusqlite::types::Type::Text,
                Box::new(e),
            ))?;

        Ok(MemoryEvent {
            id: MemoryEventId::from_string(row.get::<_, String>(0)?),
            project_id: ProcessId::from_string(row.get::<_, String>(1)?),
            event_type: row.get(2)?,
            actor_type,
            details,
            created_at,
        })
    }
}

#[async_trait]
impl MemoryEventRepository for SqliteMemoryEventRepository {
    async fn create(&self, event: MemoryEvent) -> AppResult<MemoryEvent> {
        let conn = self.conn.lock()
            .map_err(|e| AppError::Database(e.to_string()))?;

        let details_json = serde_json::to_string(&event.details)
            .map_err(|e| AppError::Database(format!("Failed to serialize details: {}", e)))?;

        conn.execute(
            "INSERT INTO memory_events (
                id, project_id, event_type, actor_type, details_json, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                event.id.as_str(),
                event.project_id.as_str(),
                event.event_type,
                event.actor_type.to_string(),
                details_json,
                event.created_at.to_rfc3339(),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(event)
    }

    async fn get_by_project(&self, project_id: &ProcessId) -> AppResult<Vec<MemoryEvent>> {
        let conn = self.conn.lock()
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut stmt = conn.prepare(
            "SELECT id, project_id, event_type, actor_type, details_json, created_at
             FROM memory_events
             WHERE project_id = ?1
             ORDER BY created_at DESC"
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        let events = stmt
            .query_map([project_id.as_str()], Self::row_to_memory_event)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(events)
    }

    async fn get_by_type(&self, event_type: &str) -> AppResult<Vec<MemoryEvent>> {
        let conn = self.conn.lock()
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut stmt = conn.prepare(
            "SELECT id, project_id, event_type, actor_type, details_json, created_at
             FROM memory_events
             WHERE event_type = ?1
             ORDER BY created_at DESC"
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        let events = stmt
            .query_map([event_type], Self::row_to_memory_event)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(events)
    }
}
