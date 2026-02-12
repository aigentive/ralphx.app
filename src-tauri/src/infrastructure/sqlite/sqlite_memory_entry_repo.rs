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
    async fn create(&self, entry: MemoryEntry) -> AppResult<MemoryEntry> {
        let conn = self.conn.lock().await;

        let scope_paths_json = serde_json::to_string(&entry.scope_paths)
            .map_err(|e| AppError::Database(format!("Failed to serialize scope_paths: {}", e)))?;

        conn.execute(
            "INSERT INTO memory_entries (
                id, project_id, bucket, title, summary, details_markdown,
                scope_paths_json, source_context_type, source_context_id,
                source_conversation_id, source_rule_file, quality_score,
                status, content_hash, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            rusqlite::params![
                entry.id.as_str(),
                entry.project_id.as_str(),
                entry.bucket.to_string(),
                entry.title,
                entry.summary,
                entry.details_markdown,
                scope_paths_json,
                entry.source_context_type,
                entry.source_context_id,
                entry.source_conversation_id,
                entry.source_rule_file,
                entry.quality_score,
                entry.status.to_string(),
                entry.content_hash,
                entry.created_at.to_rfc3339(),
                entry.updated_at.to_rfc3339(),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(entry)
    }

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

    async fn find_by_content_hash(
        &self,
        project_id: &ProjectId,
        bucket: &MemoryBucket,
        content_hash: &str,
    ) -> AppResult<Option<MemoryEntry>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, project_id, bucket, title, summary, details_markdown, scope_paths_json,
                    source_context_type, source_context_id, source_conversation_id, source_rule_file,
                    quality_score, status, content_hash, created_at, updated_at
             FROM memory_entries
             WHERE project_id = ?1 AND bucket = ?2 AND content_hash = ?3 AND status = 'active'",
            rusqlite::params![
                project_id.as_str(),
                bucket.to_string(),
                content_hash,
            ],
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

    async fn update_status(&self, id: &MemoryEntryId, status: MemoryStatus) -> AppResult<()> {
        let conn = self.conn.lock().await;

        let affected = conn.execute(
            "UPDATE memory_entries
             SET status = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')
             WHERE id = ?2",
            rusqlite::params![status.to_string(), id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        if affected == 0 {
            return Err(AppError::NotFound(format!("Memory entry not found: {}", id)));
        }

        Ok(())
    }

    async fn update(&self, entry: &MemoryEntry) -> AppResult<()> {
        let conn = self.conn.lock().await;

        let scope_paths_json = serde_json::to_string(&entry.scope_paths)
            .map_err(|e| AppError::Database(format!("Failed to serialize scope_paths: {}", e)))?;

        let affected = conn.execute(
            "UPDATE memory_entries SET
                bucket = ?1, title = ?2, summary = ?3, details_markdown = ?4,
                scope_paths_json = ?5, source_context_type = ?6, source_context_id = ?7,
                source_conversation_id = ?8, source_rule_file = ?9, quality_score = ?10,
                status = ?11, content_hash = ?12, updated_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')
             WHERE id = ?13",
            rusqlite::params![
                entry.bucket.to_string(),
                entry.title,
                entry.summary,
                entry.details_markdown,
                scope_paths_json,
                entry.source_context_type,
                entry.source_context_id,
                entry.source_conversation_id,
                entry.source_rule_file,
                entry.quality_score,
                entry.status.to_string(),
                entry.content_hash,
                entry.id.as_str(),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        if affected == 0 {
            return Err(AppError::NotFound(format!("Memory entry not found: {}", entry.id)));
        }

        Ok(())
    }

    async fn delete(&self, id: &MemoryEntryId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        let affected = conn.execute(
            "DELETE FROM memory_entries WHERE id = ?1",
            [id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        if affected == 0 {
            return Err(AppError::NotFound(format!("Memory entry not found: {}", id)));
        }

        Ok(())
    }

    async fn get_by_paths(
        &self,
        project_id: &ProjectId,
        paths: &[String],
    ) -> AppResult<Vec<MemoryEntry>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare(
            "SELECT id, project_id, bucket, title, summary, details_markdown, scope_paths_json,
                    source_context_type, source_context_id, source_conversation_id, source_rule_file,
                    quality_score, status, content_hash, created_at, updated_at
             FROM memory_entries
             WHERE project_id = ?1 AND status = 'active'
             ORDER BY created_at DESC"
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        let entries = stmt
            .query_map([project_id.as_str()], entry_from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Filter entries by matching paths against scope_paths globs (prefix matching)
        let filtered: Vec<MemoryEntry> = entries
            .into_iter()
            .filter(|entry| {
                paths.iter().any(|path| {
                    entry.scope_paths.iter().any(|glob| {
                        let glob_prefix = glob.trim_end_matches("**").trim_end_matches("*");
                        path.starts_with(glob_prefix)
                    })
                })
            })
            .collect();

        Ok(filtered)
    }
}

#[cfg(test)]
mod tests {
    // Note: Tests would require full schema setup including memory_entries table
    // These are integration tests that would run against a migrated test database
}
