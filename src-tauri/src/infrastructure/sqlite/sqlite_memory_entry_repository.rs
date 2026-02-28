// SQLite implementation of MemoryEntryRepository

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use tokio::sync::Mutex;

use crate::domain::entities::{MemoryBucket, MemoryEntry, MemoryEntryId, MemoryStatus};
use crate::domain::entities::types::ProjectId;
use crate::domain::repositories::MemoryEntryRepository;
use crate::error::{AppError, AppResult};

use super::DbConnection;

/// SQLite-backed memory entry repository
pub struct SqliteMemoryEntryRepository {
    db: DbConnection,
}

impl SqliteMemoryEntryRepository {
    /// Create a new repository with the given connection
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

    /// Helper to parse a row into a MemoryEntry
    fn row_to_memory_entry(row: &rusqlite::Row) -> rusqlite::Result<MemoryEntry> {
        let scope_paths_json: String = row.get(6)?;
        let scope_paths: Vec<String> = serde_json::from_str(&scope_paths_json)
            .unwrap_or_default();

        let bucket_str: String = row.get(2)?;
        let bucket = bucket_str.parse::<MemoryBucket>()
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Text,
                Box::new(e),
            ))?;

        let status_str: String = row.get(12)?;
        let status = status_str.parse::<MemoryStatus>()
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                12,
                rusqlite::types::Type::Text,
                Box::new(e),
            ))?;

        let created_at_str: String = row.get(14)?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                14,
                rusqlite::types::Type::Text,
                Box::new(e),
            ))?;

        let updated_at_str: String = row.get(15)?;
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                15,
                rusqlite::types::Type::Text,
                Box::new(e),
            ))?;

        Ok(MemoryEntry {
            id: MemoryEntryId::from_string(row.get::<_, String>(0)?),
            project_id: ProjectId::from_string(row.get::<_, String>(1)?),
            bucket,
            title: row.get(3)?,
            summary: row.get(4)?,
            details_markdown: row.get(5)?,
            scope_paths,
            source_context_type: row.get(7)?,
            source_context_id: row.get(8)?,
            source_conversation_id: row.get(9)?,
            source_rule_file: row.get(10)?,
            quality_score: row.get(11)?,
            status,
            content_hash: row.get(13)?,
            created_at,
            updated_at,
        })
    }
}

#[async_trait]
impl MemoryEntryRepository for SqliteMemoryEntryRepository {
    async fn create(&self, entry: MemoryEntry) -> AppResult<MemoryEntry> {
        let scope_paths_json = serde_json::to_string(&entry.scope_paths)
            .map_err(|e| AppError::Database(format!("Failed to serialize scope_paths: {}", e)))?;
        let id = entry.id.as_str().to_string();
        let project_id = entry.project_id.as_str().to_string();
        let bucket_str = entry.bucket.to_string();
        let title = entry.title.clone();
        let summary = entry.summary.clone();
        let details_markdown = entry.details_markdown.clone();
        let source_context_type = entry.source_context_type.clone();
        let source_context_id = entry.source_context_id.clone();
        let source_conversation_id = entry.source_conversation_id.clone();
        let source_rule_file = entry.source_rule_file.clone();
        let quality_score = entry.quality_score;
        let status_str = entry.status.to_string();
        let content_hash = entry.content_hash.clone();
        let created_at_str = entry.created_at.to_rfc3339();
        let updated_at_str = entry.updated_at.to_rfc3339();

        self.db
            .run(move |conn| {
                conn.execute(
                    "INSERT INTO memory_entries (
                        id, project_id, bucket, title, summary, details_markdown,
                        scope_paths_json, source_context_type, source_context_id,
                        source_conversation_id, source_rule_file, quality_score,
                        status, content_hash, created_at, updated_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                    rusqlite::params![
                        id,
                        project_id,
                        bucket_str,
                        title,
                        summary,
                        details_markdown,
                        scope_paths_json,
                        source_context_type,
                        source_context_id,
                        source_conversation_id,
                        source_rule_file,
                        quality_score,
                        status_str,
                        content_hash,
                        created_at_str,
                        updated_at_str,
                    ],
                )?;
                Ok(entry)
            })
            .await
    }

    async fn get_by_id(&self, id: &MemoryEntryId) -> AppResult<Option<MemoryEntry>> {
        let id = id.as_str().to_string();
        self.db
            .query_optional(move |conn| {
                conn.query_row(
                    "SELECT id, project_id, bucket, title, summary, details_markdown,
                            scope_paths_json, source_context_type, source_context_id,
                            source_conversation_id, source_rule_file, quality_score,
                            status, content_hash, created_at, updated_at
                     FROM memory_entries WHERE id = ?1",
                    [id.as_str()],
                    SqliteMemoryEntryRepository::row_to_memory_entry,
                )
            })
            .await
    }

    async fn find_by_content_hash(
        &self,
        project_id: &ProjectId,
        bucket: &MemoryBucket,
        content_hash: &str,
    ) -> AppResult<Option<MemoryEntry>> {
        let project_id = project_id.as_str().to_string();
        let bucket_str = bucket.to_string();
        let content_hash = content_hash.to_string();
        self.db
            .query_optional(move |conn| {
                conn.query_row(
                    "SELECT id, project_id, bucket, title, summary, details_markdown,
                            scope_paths_json, source_context_type, source_context_id,
                            source_conversation_id, source_rule_file, quality_score,
                            status, content_hash, created_at, updated_at
                     FROM memory_entries
                     WHERE project_id = ?1 AND bucket = ?2 AND content_hash = ?3 AND status = 'active'",
                    rusqlite::params![project_id, bucket_str, content_hash],
                    SqliteMemoryEntryRepository::row_to_memory_entry,
                )
            })
            .await
    }

    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<MemoryEntry>> {
        let project_id = project_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, project_id, bucket, title, summary, details_markdown,
                            scope_paths_json, source_context_type, source_context_id,
                            source_conversation_id, source_rule_file, quality_score,
                            status, content_hash, created_at, updated_at
                     FROM memory_entries
                     WHERE project_id = ?1 AND status = 'active'
                     ORDER BY created_at DESC",
                )?;
                let entries = stmt
                    .query_map([project_id.as_str()], SqliteMemoryEntryRepository::row_to_memory_entry)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(entries)
            })
            .await
    }

    async fn get_by_project_and_bucket(
        &self,
        project_id: &ProjectId,
        bucket: &MemoryBucket,
    ) -> AppResult<Vec<MemoryEntry>> {
        let project_id = project_id.as_str().to_string();
        let bucket_str = bucket.to_string();
        self.db
            .run(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, project_id, bucket, title, summary, details_markdown,
                            scope_paths_json, source_context_type, source_context_id,
                            source_conversation_id, source_rule_file, quality_score,
                            status, content_hash, created_at, updated_at
                     FROM memory_entries
                     WHERE project_id = ?1 AND bucket = ?2 AND status = 'active'
                     ORDER BY created_at DESC",
                )?;
                let entries = stmt
                    .query_map(
                        rusqlite::params![project_id, bucket_str],
                        SqliteMemoryEntryRepository::row_to_memory_entry,
                    )?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(entries)
            })
            .await
    }

    async fn update_status(&self, id: &MemoryEntryId, status: MemoryStatus) -> AppResult<()> {
        let not_found_msg = format!("Memory entry not found: {}", id);
        let id_str = id.as_str().to_string();
        let status_str = status.to_string();

        let affected = self
            .db
            .run(move |conn| {
                Ok(conn.execute(
                    "UPDATE memory_entries
                     SET status = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')
                     WHERE id = ?2",
                    rusqlite::params![status_str, id_str],
                )?)
            })
            .await?;

        if affected == 0 {
            return Err(AppError::NotFound(not_found_msg));
        }

        Ok(())
    }

    async fn update(&self, entry: &MemoryEntry) -> AppResult<()> {
        let scope_paths_json = serde_json::to_string(&entry.scope_paths)
            .map_err(|e| AppError::Database(format!("Failed to serialize scope_paths: {}", e)))?;
        let not_found_msg = format!("Memory entry not found: {}", entry.id);
        let id_str = entry.id.as_str().to_string();
        let bucket_str = entry.bucket.to_string();
        let title = entry.title.clone();
        let summary = entry.summary.clone();
        let details_markdown = entry.details_markdown.clone();
        let source_context_type = entry.source_context_type.clone();
        let source_context_id = entry.source_context_id.clone();
        let source_conversation_id = entry.source_conversation_id.clone();
        let source_rule_file = entry.source_rule_file.clone();
        let quality_score = entry.quality_score;
        let status_str = entry.status.to_string();
        let content_hash = entry.content_hash.clone();

        let affected = self
            .db
            .run(move |conn| {
                Ok(conn.execute(
                    "UPDATE memory_entries SET
                        bucket = ?1, title = ?2, summary = ?3, details_markdown = ?4,
                        scope_paths_json = ?5, source_context_type = ?6, source_context_id = ?7,
                        source_conversation_id = ?8, source_rule_file = ?9, quality_score = ?10,
                        status = ?11, content_hash = ?12, updated_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')
                     WHERE id = ?13",
                    rusqlite::params![
                        bucket_str,
                        title,
                        summary,
                        details_markdown,
                        scope_paths_json,
                        source_context_type,
                        source_context_id,
                        source_conversation_id,
                        source_rule_file,
                        quality_score,
                        status_str,
                        content_hash,
                        id_str,
                    ],
                )?)
            })
            .await?;

        if affected == 0 {
            return Err(AppError::NotFound(not_found_msg));
        }

        Ok(())
    }

    async fn delete(&self, id: &MemoryEntryId) -> AppResult<()> {
        let not_found_msg = format!("Memory entry not found: {}", id);
        let id_str = id.as_str().to_string();

        let affected = self
            .db
            .run(move |conn| {
                Ok(conn.execute(
                    "DELETE FROM memory_entries WHERE id = ?1",
                    [id_str.as_str()],
                )?)
            })
            .await?;

        if affected == 0 {
            return Err(AppError::NotFound(not_found_msg));
        }

        Ok(())
    }

    async fn get_by_paths(
        &self,
        project_id: &ProjectId,
        paths: &[String],
    ) -> AppResult<Vec<MemoryEntry>> {
        let project_id = project_id.as_str().to_string();
        let entries: Vec<MemoryEntry> = self
            .db
            .run(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, project_id, bucket, title, summary, details_markdown,
                            scope_paths_json, source_context_type, source_context_id,
                            source_conversation_id, source_rule_file, quality_score,
                            status, content_hash, created_at, updated_at
                     FROM memory_entries
                     WHERE project_id = ?1 AND status = 'active'
                     ORDER BY created_at DESC",
                )?;
                let entries = stmt
                    .query_map([project_id.as_str()], SqliteMemoryEntryRepository::row_to_memory_entry)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(entries)
            })
            .await?;

        // Filter entries by matching any of the provided paths against scope_paths globs
        // This is a simple implementation that checks for prefix matches
        // A more sophisticated implementation would use proper glob matching
        let filtered: Vec<MemoryEntry> = entries
            .into_iter()
            .filter(|entry| {
                // Match if any of the provided paths matches any of the entry's scope_paths
                paths.iter().any(|path| {
                    entry.scope_paths.iter().any(|glob| {
                        // Simple prefix matching (more sophisticated glob matching would go here)
                        let glob_prefix = glob.trim_end_matches("**").trim_end_matches("*");
                        path.starts_with(glob_prefix)
                    })
                })
            })
            .collect();

        Ok(filtered)
    }
}
