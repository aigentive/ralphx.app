// SQLite-based MemoryEntryRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use rusqlite::Connection;

use crate::domain::entities::{MemoryBucket, MemoryEntry, MemoryEntryId, MemoryStatus};
use crate::domain::entities::types::ProjectId;
use crate::domain::repositories::MemoryEntryRepository;
use crate::error::{AppError, AppResult};

/// SQLite implementation of MemoryEntryRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteMemoryEntryRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteMemoryEntryRepository {
    /// Create a new SQLite memory entry repository with the given connection
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

/// Helper to deserialize a MemoryEntry from a database row
fn entry_from_row(row: &rusqlite::Row) -> rusqlite::Result<MemoryEntry> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let bucket_str: String = row.get(2)?;
    let title: String = row.get(3)?;
    let summary: String = row.get(4)?;
    let details_markdown: String = row.get(5)?;
    let scope_paths_json: String = row.get(6)?;
    let source_context_type: Option<String> = row.get(7)?;
    let source_context_id: Option<String> = row.get(8)?;
    let source_conversation_id: Option<String> = row.get(9)?;
    let source_rule_file: Option<String> = row.get(10)?;
    let quality_score: Option<f64> = row.get(11)?;
    let status_str: String = row.get(12)?;
    let content_hash: String = row.get(13)?;
    let created_at_str: String = row.get(14)?;
    let updated_at_str: String = row.get(15)?;

    let bucket = bucket_str
        .parse::<MemoryBucket>()
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

    let status = status_str
        .parse::<MemoryStatus>()
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

    let scope_paths = MemoryEntry::scope_paths_from_json(&scope_paths_json)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

    let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
        .with_timezone(&Utc);

    let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
        .with_timezone(&Utc);

    Ok(MemoryEntry {
        id: MemoryEntryId::from(id),
        project_id: ProjectId::from_string(project_id),
        bucket,
        title,
        summary,
        details_markdown,
        scope_paths,
        source_context_type,
        source_context_id,
        source_conversation_id,
        source_rule_file,
        quality_score,
        status,
        content_hash,
        created_at,
        updated_at,
    })
}

#[async_trait]
impl MemoryEntryRepository for SqliteMemoryEntryRepository {
    async fn get_by_id(&self, id: &MemoryEntryId) -> AppResult<Option<MemoryEntry>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, project_id, bucket, title, summary, details_markdown, scope_paths_json,
                    source_context_type, source_context_id, source_conversation_id, source_rule_file,
                    quality_score, status, content_hash, created_at, updated_at
             FROM memory_entries WHERE id = ?1",
            [id.as_str()],
            entry_from_row,
        );

        match result {
            Ok(entry) => Ok(Some(entry)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<MemoryEntry>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, bucket, title, summary, details_markdown, scope_paths_json,
                        source_context_type, source_context_id, source_conversation_id, source_rule_file,
                        quality_score, status, content_hash, created_at, updated_at
                 FROM memory_entries
                 WHERE project_id = ?1
                 ORDER BY created_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let entries = stmt
            .query_map([project_id.as_str()], entry_from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(entries)
    }

    async fn get_by_project_and_status(
        &self,
        project_id: &ProjectId,
        status: MemoryStatus,
    ) -> AppResult<Vec<MemoryEntry>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, bucket, title, summary, details_markdown, scope_paths_json,
                        source_context_type, source_context_id, source_conversation_id, source_rule_file,
                        quality_score, status, content_hash, created_at, updated_at
                 FROM memory_entries
                 WHERE project_id = ?1 AND status = ?2
                 ORDER BY created_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let entries = stmt
            .query_map(
                rusqlite::params![project_id.as_str(), status.to_string()],
                entry_from_row,
            )
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(entries)
    }

    async fn get_by_project_and_bucket(
        &self,
        project_id: &ProjectId,
        bucket: MemoryBucket,
    ) -> AppResult<Vec<MemoryEntry>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, bucket, title, summary, details_markdown, scope_paths_json,
                        source_context_type, source_context_id, source_conversation_id, source_rule_file,
                        quality_score, status, content_hash, created_at, updated_at
                 FROM memory_entries
                 WHERE project_id = ?1 AND bucket = ?2
                 ORDER BY created_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let entries = stmt
            .query_map(
                rusqlite::params![project_id.as_str(), bucket.to_string()],
                entry_from_row,
            )
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(entries)
    }

    async fn get_by_rule_file(
        &self,
        project_id: &ProjectId,
        rule_file: &str,
    ) -> AppResult<Vec<MemoryEntry>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, bucket, title, summary, details_markdown, scope_paths_json,
                        source_context_type, source_context_id, source_conversation_id, source_rule_file,
                        quality_score, status, content_hash, created_at, updated_at
                 FROM memory_entries
                 WHERE project_id = ?1 AND source_rule_file = ?2
                 ORDER BY created_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let entries = stmt
            .query_map(rusqlite::params![project_id.as_str(), rule_file], entry_from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(entries)
    }

    async fn get_by_content_hash(&self, content_hash: &str) -> AppResult<Vec<MemoryEntry>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, bucket, title, summary, details_markdown, scope_paths_json,
                        source_context_type, source_context_id, source_conversation_id, source_rule_file,
                        quality_score, status, content_hash, created_at, updated_at
                 FROM memory_entries
                 WHERE content_hash = ?1
                 ORDER BY created_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let entries = stmt
            .query_map([content_hash], entry_from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::sqlite::connection::create_in_memory_connection;

    // Note: Tests would require full schema setup including memory_entries table
    // These are integration tests that would run against a migrated test database
}
