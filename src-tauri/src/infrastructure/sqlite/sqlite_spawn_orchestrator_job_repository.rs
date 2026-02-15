// SQLite implementation of SpawnOrchestratorJobRepository

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use tokio::sync::Mutex;

use crate::domain::entities::types::{IdeationSessionId, ProjectId};
use crate::domain::entities::{
    SpawnOrchestratorJob, SpawnOrchestratorJobId, SpawnOrchestratorJobStatus,
};
use crate::domain::repositories::SpawnOrchestratorJobRepository;
use crate::error::{AppError, AppResult};

/// SQLite-backed spawn orchestrator job repository
pub struct SqliteSpawnOrchestratorJobRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteSpawnOrchestratorJobRepository {
    /// Create a new repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    /// Create from an Arc-wrapped mutex connection (for sharing)
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Helper to parse a row into a SpawnOrchestratorJob
    fn row_to_spawn_orchestrator_job(
        row: &rusqlite::Row,
    ) -> rusqlite::Result<SpawnOrchestratorJob> {
        let status_str: String = row.get(4)?;
        let status = status_str.parse::<SpawnOrchestratorJobStatus>().map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e))
        })?;

        let created_at_str: String = row.get(7)?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    7,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;

        let updated_at_str: String = row.get(8)?;
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    8,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;

        let started_at: Option<DateTime<Utc>> = row
            .get::<_, Option<String>>(9)?
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let completed_at: Option<DateTime<Utc>> = row
            .get::<_, Option<String>>(10)?
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        Ok(SpawnOrchestratorJob {
            id: SpawnOrchestratorJobId::from(row.get::<_, String>(0)?),
            session_id: IdeationSessionId::from_string(row.get::<_, String>(1)?),
            project_id: ProjectId::from_string(row.get::<_, String>(2)?),
            description: row.get(3)?,
            status,
            error_message: row.get(5)?,
            attempt_count: row.get(6)?,
            created_at,
            updated_at,
            started_at,
            completed_at,
        })
    }
}

#[async_trait]
impl SpawnOrchestratorJobRepository for SqliteSpawnOrchestratorJobRepository {
    async fn create(&self, job: SpawnOrchestratorJob) -> AppResult<SpawnOrchestratorJob> {
        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT INTO spawn_orchestrator_jobs (
                id, session_id, project_id, description, status, error_message,
                attempt_count, created_at, updated_at, started_at, completed_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                job.id.0.as_str(),
                job.session_id.as_str(),
                job.project_id.as_str(),
                job.description,
                job.status.to_string(),
                job.error_message,
                job.attempt_count,
                job.created_at.to_rfc3339(),
                job.updated_at.to_rfc3339(),
                job.started_at.as_ref().map(|dt| dt.to_rfc3339()),
                job.completed_at.as_ref().map(|dt| dt.to_rfc3339()),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(job)
    }

    async fn get_by_id(
        &self,
        id: &SpawnOrchestratorJobId,
    ) -> AppResult<Option<SpawnOrchestratorJob>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, session_id, project_id, description, status, error_message,
                    attempt_count, created_at, updated_at, started_at, completed_at
             FROM spawn_orchestrator_jobs WHERE id = ?1",
            [id.0.as_str()],
            Self::row_to_spawn_orchestrator_job,
        );

        match result {
            Ok(job) => Ok(Some(job)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_pending(&self) -> AppResult<Vec<SpawnOrchestratorJob>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, session_id, project_id, description, status, error_message,
                    attempt_count, created_at, updated_at, started_at, completed_at
             FROM spawn_orchestrator_jobs
             WHERE status = 'pending'
             ORDER BY created_at ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let jobs = stmt
            .query_map([], Self::row_to_spawn_orchestrator_job)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(jobs)
    }

    async fn update_status(
        &self,
        id: &SpawnOrchestratorJobId,
        status: SpawnOrchestratorJobStatus,
        error_message: Option<String>,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;

        let now = Utc::now().to_rfc3339();

        // Build the update query based on status
        let (started_at, completed_at) = match status {
            SpawnOrchestratorJobStatus::Running => (Some(now.clone()), None),
            SpawnOrchestratorJobStatus::Done | SpawnOrchestratorJobStatus::Failed => {
                (None, Some(now.clone()))
            }
            SpawnOrchestratorJobStatus::Pending => (None, None),
        };

        // When starting, also increment attempt_count
        let affected = if status == SpawnOrchestratorJobStatus::Running {
            conn.execute(
                "UPDATE spawn_orchestrator_jobs
             SET status = ?1,
                 error_message = ?2,
                 updated_at = ?3,
                 started_at = ?4,
                 completed_at = ?5,
                 attempt_count = attempt_count + 1
             WHERE id = ?6",
                rusqlite::params![
                    status.to_string(),
                    error_message,
                    now,
                    started_at,
                    completed_at,
                    id.0.as_str(),
                ],
            )
            .map_err(|e| AppError::Database(e.to_string()))?
        } else {
            conn.execute(
                "UPDATE spawn_orchestrator_jobs
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
                    id.0.as_str(),
                ],
            )
            .map_err(|e| AppError::Database(e.to_string()))?
        };

        if affected == 0 {
            return Err(AppError::NotFound(format!(
                "Spawn orchestrator job not found: {}",
                id
            )));
        }

        Ok(())
    }

    async fn claim_pending(&self) -> AppResult<Option<SpawnOrchestratorJob>> {
        let mut conn = self.conn.lock().await;

        // Atomic claim: find oldest pending job and mark as running in a single transaction
        let tx = conn
            .transaction()
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Find the oldest pending or failed job (failed jobs can be retried)
        let job_result = tx.query_row(
            "SELECT id, session_id, project_id, description, status, error_message,
                    attempt_count, created_at, updated_at, started_at, completed_at
             FROM spawn_orchestrator_jobs
             WHERE status = 'pending' OR status = 'failed'
             ORDER BY created_at ASC
             LIMIT 1",
            [],
            Self::row_to_spawn_orchestrator_job,
        );

        let job = match job_result {
            Ok(j) => j,
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                // No pending jobs
                return Ok(None);
            }
            Err(e) => return Err(AppError::Database(e.to_string())),
        };

        let now = Utc::now().to_rfc3339();

        // Mark as running and increment attempt count (can claim pending or failed jobs)
        let affected = tx
            .execute(
                "UPDATE spawn_orchestrator_jobs
             SET status = 'running',
                 updated_at = ?1,
                 started_at = ?2,
                 attempt_count = attempt_count + 1
             WHERE id = ?3 AND (status = 'pending' OR status = 'failed')",
                rusqlite::params![now, now, job.id.0.as_str()],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        if affected == 0 {
            // Job was claimed by another worker (race condition)
            tx.rollback()
                .map_err(|e| AppError::Database(e.to_string()))?;
            return Ok(None);
        }

        tx.commit()
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Return the job with updated status
        Ok(Some(SpawnOrchestratorJob {
            status: SpawnOrchestratorJobStatus::Running,
            started_at: Some(Utc::now()),
            updated_at: Utc::now(),
            attempt_count: job.attempt_count + 1,
            ..job
        }))
    }
}
