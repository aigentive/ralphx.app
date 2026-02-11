// SQLite implementation of MemoryArchiveJobRepository

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde_json::Value as JsonValue;
use std::sync::Mutex;

use crate::domain::entities::{
    MemoryArchiveJob, MemoryArchiveJobId, MemoryArchiveJobStatus, MemoryArchiveJobType, ProcessId,
};
use crate::domain::repositories::MemoryArchiveJobRepository;
use crate::error::{AppError, AppResult};

/// SQLite-backed memory archive job repository
pub struct SqliteMemoryArchiveJobRepository {
    conn: Mutex<Connection>,
}

impl SqliteMemoryArchiveJobRepository {
    /// Create a new repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }

    /// Helper to parse a row into a MemoryArchiveJob
    fn row_to_memory_archive_job(row: &rusqlite::Row) -> rusqlite::Result<MemoryArchiveJob> {
        let job_type_str: String = row.get(2)?;
        let job_type = job_type_str.parse::<MemoryArchiveJobType>()
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Text,
                Box::new(e),
            ))?;

        let payload_json_str: String = row.get(3)?;
        let payload: JsonValue = serde_json::from_str(&payload_json_str)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                3,
                rusqlite::types::Type::Text,
                Box::new(e),
            ))?;

        let status_str: String = row.get(4)?;
        let status = status_str.parse::<MemoryArchiveJobStatus>()
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                4,
                rusqlite::types::Type::Text,
                Box::new(e),
            ))?;

        let created_at_str: String = row.get(6)?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                6,
                rusqlite::types::Type::Text,
                Box::new(e),
            ))?;

        let updated_at_str: String = row.get(7)?;
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                7,
                rusqlite::types::Type::Text,
                Box::new(e),
            ))?;

        let started_at: Option<DateTime<Utc>> = row.get::<_, Option<String>>(8)?
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let completed_at: Option<DateTime<Utc>> = row.get::<_, Option<String>>(9)?
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        Ok(MemoryArchiveJob {
            id: MemoryArchiveJobId::from_string(row.get::<_, String>(0)?),
            project_id: ProcessId::from_string(row.get::<_, String>(1)?),
            job_type,
            payload,
            status,
            error_message: row.get(5)?,
            created_at,
            updated_at,
            started_at,
            completed_at,
        })
    }
}

#[async_trait]
impl MemoryArchiveJobRepository for SqliteMemoryArchiveJobRepository {
    async fn create(&self, job: MemoryArchiveJob) -> AppResult<MemoryArchiveJob> {
        let conn = self.conn.lock()
            .map_err(|e| AppError::Database(e.to_string()))?;

        let payload_json = serde_json::to_string(&job.payload)
            .map_err(|e| AppError::Database(format!("Failed to serialize payload: {}", e)))?;

        conn.execute(
            "INSERT INTO memory_archive_jobs (
                id, project_id, job_type, payload_json, status, error_message,
                created_at, updated_at, started_at, completed_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                job.id.as_str(),
                job.project_id.as_str(),
                job.job_type.to_string(),
                payload_json,
                job.status.to_string(),
                job.error_message,
                job.created_at.to_rfc3339(),
                job.updated_at.to_rfc3339(),
                job.started_at.as_ref().map(|dt| dt.to_rfc3339()),
                job.completed_at.as_ref().map(|dt| dt.to_rfc3339()),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(job)
    }

    async fn get_by_id(&self, id: &MemoryArchiveJobId) -> AppResult<Option<MemoryArchiveJob>> {
        let conn = self.conn.lock()
            .map_err(|e| AppError::Database(e.to_string()))?;

        let result = conn.query_row(
            "SELECT id, project_id, job_type, payload_json, status, error_message,
                    created_at, updated_at, started_at, completed_at
             FROM memory_archive_jobs WHERE id = ?1",
            [id.as_str()],
            Self::row_to_memory_archive_job,
        );

        match result {
            Ok(job) => Ok(Some(job)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_pending_by_project(
        &self,
        project_id: &ProcessId,
    ) -> AppResult<Vec<MemoryArchiveJob>> {
        let conn = self.conn.lock()
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut stmt = conn.prepare(
            "SELECT id, project_id, job_type, payload_json, status, error_message,
                    created_at, updated_at, started_at, completed_at
             FROM memory_archive_jobs
             WHERE project_id = ?1 AND status = 'pending'
             ORDER BY created_at ASC"
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        let jobs = stmt
            .query_map([project_id.as_str()], Self::row_to_memory_archive_job)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(jobs)
    }

    async fn update_status(
        &self,
        id: &MemoryArchiveJobId,
        status: MemoryArchiveJobStatus,
        error_message: Option<String>,
    ) -> AppResult<()> {
        let conn = self.conn.lock()
            .map_err(|e| AppError::Database(e.to_string()))?;

        let now = Utc::now().to_rfc3339();

        // Build the update query based on status
        let (started_at, completed_at) = match status {
            MemoryArchiveJobStatus::Running => (Some(now.clone()), None),
            MemoryArchiveJobStatus::Done | MemoryArchiveJobStatus::Failed => {
                (None, Some(now.clone()))
            }
            MemoryArchiveJobStatus::Pending => (None, None),
        };

        let affected = conn.execute(
            "UPDATE memory_archive_jobs
             SET status = ?1,
                 error_message = ?2,
                 updated_at = ?3,
                 started_at = COALESCE(?4, started_at),
                 completed_at = COALESCE(?5, completed_at)
             WHERE id = ?6",
            rusqlite::params![
                status.to_string(),
                error_message,
                now,
                started_at,
                completed_at,
                id.as_str(),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        if affected == 0 {
            return Err(AppError::NotFound(format!("Archive job not found: {}", id)));
        }

        Ok(())
    }
}
