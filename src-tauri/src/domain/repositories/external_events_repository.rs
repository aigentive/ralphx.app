// ExternalEventsRepository — trait for persisting external events to the DB
//
// The `external_events` table is the bridge between internal state machine
// transitions and external consumers (e.g. the external MCP server via poll/SSE).
//
// Schema (created in migration v56):
//   id INTEGER PRIMARY KEY AUTOINCREMENT
//   event_type TEXT NOT NULL
//   project_id TEXT NOT NULL
//   payload TEXT NOT NULL  (JSON)
//   created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))

use async_trait::async_trait;

use crate::error::AppResult;

/// A single event from the external_events table.
#[derive(Debug, Clone)]
pub struct ExternalEventRecord {
    pub id: i64,
    pub event_type: String,
    pub project_id: String,
    pub payload: String,
    pub created_at: String,
}

/// Repository trait for the external_events table.
#[async_trait]
pub trait ExternalEventsRepository: Send + Sync {
    /// Insert a new external event. Returns the ROWID of the newly inserted row.
    async fn insert_event(
        &self,
        event_type: &str,
        project_id: &str,
        payload: &str,
    ) -> AppResult<i64>;

    /// Return events for the given project IDs where id > cursor, ordered ASC, up to limit rows.
    async fn get_events_after_cursor(
        &self,
        project_ids: &[String],
        cursor: i64,
        limit: i64,
    ) -> AppResult<Vec<ExternalEventRecord>>;

    /// Delete old events:
    ///   - entries older than 24 hours, OR
    ///   - entries beyond the 10 000-row high-water mark per project
    /// Returns the number of rows deleted.
    async fn cleanup_old_events(&self) -> AppResult<u64>;
}
