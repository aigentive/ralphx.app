// SQLite-based MemoryArchiveRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use rusqlite::Connection;

use crate::domain::entities::types::ProjectId;
use crate::domain::entities::{
    ArchiveJobPayload, ArchiveJobStatus, ArchiveJobType, MemoryArchiveJob, MemoryArchiveJobId,
};
use crate::domain::repositories::MemoryArchiveRepository;
use crate::error::{AppError, AppResult};

/// SQLite implementation of MemoryArchiveRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteMemoryArchiveRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteMemoryArchiveRepository {
    /// Create a new SQLite memory archive repository with the given connection
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

/// Helper to deserialize a MemoryArchiveJob from a database row
fn job_from_row(row: &rusqlite::Row) -> rusqlite::Result<MemoryArchiveJob> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let job_type_str: String = row.get(2)?;
    let payload_json: String = row.get(3)?;
    let status_str: String = row.get(4)?;
    let error_message: Option<String> = row.get(5)?;
    let created_at_str: String = row.get(6)?;
    let updated_at_str: String = row.get(7)?;
    let started_at_str: Option<String> = row.get(8)?;
    let completed_at_str: Option<String> = row.get(9)?;

    let job_type = job_type_str
        .parse::<ArchiveJobType>()
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

    let status = status_str
        .parse::<ArchiveJobStatus>()
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

    let payload = ArchiveJobPayload::from_json(&payload_json)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

    let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
        .with_timezone(&Utc);

    let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
        .with_timezone(&Utc);

    let started_at = started_at_str
        .as_ref()
        .map(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })
        .transpose()?;

    let completed_at = completed_at_str
        .as_ref()
        .map(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })
        .transpose()?;

    Ok(MemoryArchiveJob {
        id: MemoryArchiveJobId::from(id),
        project_id: ProjectId::from_string(project_id),
        job_type,
        payload,
        status,
        error_message,
        created_at,
        updated_at,
        started_at,
        completed_at,
    })
}

#[async_trait]
impl MemoryArchiveRepository for SqliteMemoryArchiveRepository {
    async fn create(&self, job: MemoryArchiveJob) -> AppResult<MemoryArchiveJob> {
        let conn = self.conn.lock().await;

        let payload_json = job.payload.to_json()?;

        conn.execute(
            "INSERT INTO memory_archive_jobs (id, project_id, job_type, payload_json, status, error_message, created_at, updated_at, started_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                job.id.0,
                job.project_id.as_str(),
                job.job_type.to_string(),
                payload_json,
                job.status.to_string(),
                job.error_message,
                job.created_at.to_rfc3339(),
                job.updated_at.to_rfc3339(),
                job.started_at.map(|dt| dt.to_rfc3339()),
                job.completed_at.map(|dt| dt.to_rfc3339()),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(job)
    }

    async fn get_by_id(&self, id: &MemoryArchiveJobId) -> AppResult<Option<MemoryArchiveJob>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, project_id, job_type, payload_json, status, error_message, created_at, updated_at, started_at, completed_at
             FROM memory_archive_jobs WHERE id = ?1",
            [&id.0],
            job_from_row,
        );

        match result {
            Ok(job) => Ok(Some(job)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn update(&self, job: &MemoryArchiveJob) -> AppResult<()> {
        let conn = self.conn.lock().await;

        let payload_json = job.payload.to_json()?;

        conn.execute(
            "UPDATE memory_archive_jobs
             SET project_id = ?2, job_type = ?3, payload_json = ?4, status = ?5, error_message = ?6, updated_at = ?7, started_at = ?8, completed_at = ?9
             WHERE id = ?1",
            rusqlite::params![
                job.id.0,
                job.project_id.as_str(),
                job.job_type.to_string(),
                payload_json,
                job.status.to_string(),
                job.error_message,
                job.updated_at.to_rfc3339(),
                job.started_at.map(|dt| dt.to_rfc3339()),
                job.completed_at.map(|dt| dt.to_rfc3339()),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: &MemoryArchiveJobId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute("DELETE FROM memory_archive_jobs WHERE id = ?1", [&id.0])
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<MemoryArchiveJob>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, job_type, payload_json, status, error_message, created_at, updated_at, started_at, completed_at
                 FROM memory_archive_jobs
                 WHERE project_id = ?1
                 ORDER BY created_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let jobs = stmt
            .query_map([project_id.as_str()], job_from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(jobs)
    }

    async fn get_by_status(&self, status: ArchiveJobStatus) -> AppResult<Vec<MemoryArchiveJob>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, job_type, payload_json, status, error_message, created_at, updated_at, started_at, completed_at
                 FROM memory_archive_jobs
                 WHERE status = ?1
                 ORDER BY created_at ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let jobs = stmt
            .query_map([status.to_string()], job_from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(jobs)
    }

    async fn get_by_project_and_status(
        &self,
        project_id: &ProjectId,
        status: ArchiveJobStatus,
    ) -> AppResult<Vec<MemoryArchiveJob>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, job_type, payload_json, status, error_message, created_at, updated_at, started_at, completed_at
                 FROM memory_archive_jobs
                 WHERE project_id = ?1 AND status = ?2
                 ORDER BY created_at ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let jobs = stmt
            .query_map(
                rusqlite::params![project_id.as_str(), status.to_string()],
                job_from_row,
            )
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(jobs)
    }

    async fn get_by_project_and_type(
        &self,
        project_id: &ProjectId,
        job_type: ArchiveJobType,
    ) -> AppResult<Vec<MemoryArchiveJob>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, job_type, payload_json, status, error_message, created_at, updated_at, started_at, completed_at
                 FROM memory_archive_jobs
                 WHERE project_id = ?1 AND job_type = ?2
                 ORDER BY created_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let jobs = stmt
            .query_map(
                rusqlite::params![project_id.as_str(), job_type.to_string()],
                job_from_row,
            )
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(jobs)
    }

    async fn claim_next(&self) -> AppResult<Option<MemoryArchiveJob>> {
        let conn = self.conn.lock().await;

        // Start a transaction for atomic claim operation
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Find the next claimable job (pending or failed, oldest first)
        let result: rusqlite::Result<MemoryArchiveJob> = tx.query_row(
            "SELECT id, project_id, job_type, payload_json, status, error_message, created_at, updated_at, started_at, completed_at
             FROM memory_archive_jobs
             WHERE status IN ('pending', 'failed')
             ORDER BY created_at ASC
             LIMIT 1",
            [],
            job_from_row,
        );

        let mut job = match result {
            Ok(job) => job,
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
                return Ok(None);
            }
            Err(e) => {
                return Err(AppError::Database(e.to_string()));
            }
        };

        // Mark as running
        job.start();

        tx.execute(
            "UPDATE memory_archive_jobs
             SET status = ?1, updated_at = ?2, started_at = ?3
             WHERE id = ?4",
            rusqlite::params![
                job.status.to_string(),
                job.updated_at.to_rfc3339(),
                job.started_at.map(|dt| dt.to_rfc3339()),
                job.id.0,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        tx.commit().map_err(|e| AppError::Database(e.to_string()))?;

        Ok(Some(job))
    }

    async fn claim_next_for_project(
        &self,
        project_id: &ProjectId,
    ) -> AppResult<Option<MemoryArchiveJob>> {
        let conn = self.conn.lock().await;

        // Start a transaction for atomic claim operation
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Find the next claimable job for this project (pending or failed, oldest first)
        let result: rusqlite::Result<MemoryArchiveJob> = tx.query_row(
            "SELECT id, project_id, job_type, payload_json, status, error_message, created_at, updated_at, started_at, completed_at
             FROM memory_archive_jobs
             WHERE project_id = ?1 AND status IN ('pending', 'failed')
             ORDER BY created_at ASC
             LIMIT 1",
            [project_id.as_str()],
            job_from_row,
        );

        let mut job = match result {
            Ok(job) => job,
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
                return Ok(None);
            }
            Err(e) => {
                return Err(AppError::Database(e.to_string()));
            }
        };

        // Mark as running
        job.start();

        tx.execute(
            "UPDATE memory_archive_jobs
             SET status = ?1, updated_at = ?2, started_at = ?3
             WHERE id = ?4",
            rusqlite::params![
                job.status.to_string(),
                job.updated_at.to_rfc3339(),
                job.started_at.map(|dt| dt.to_rfc3339()),
                job.id.0,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        tx.commit().map_err(|e| AppError::Database(e.to_string()))?;

        Ok(Some(job))
    }

    async fn count_by_status(&self, status: ArchiveJobStatus) -> AppResult<u32> {
        let conn = self.conn.lock().await;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memory_archive_jobs WHERE status = ?1",
                [status.to_string()],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count as u32)
    }

    async fn count_claimable(&self) -> AppResult<u32> {
        let conn = self.conn.lock().await;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memory_archive_jobs WHERE status IN ('pending', 'failed')",
                [],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count as u32)
    }

    async fn count_claimable_for_project(&self, project_id: &ProjectId) -> AppResult<u32> {
        let conn = self.conn.lock().await;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memory_archive_jobs WHERE project_id = ?1 AND status IN ('pending', 'failed')",
                [project_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count as u32)
    }

    async fn delete_completed_older_than(&self, days: u32) -> AppResult<u32> {
        let conn = self.conn.lock().await;

        let cutoff_date = Utc::now() - chrono::Duration::days(days as i64);

        let deleted = conn
            .execute(
                "DELETE FROM memory_archive_jobs
                 WHERE status = 'done' AND completed_at < ?1",
                [cutoff_date.to_rfc3339()],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(deleted as u32)
    }

    async fn delete_by_project(&self, project_id: &ProjectId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "DELETE FROM memory_archive_jobs WHERE project_id = ?1",
            [project_id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::ArchiveJobPayload;
    use crate::infrastructure::sqlite::connection::open_memory_connection;
    use crate::infrastructure::sqlite::migrations::run_migrations;

    async fn setup_test_repo() -> SqliteMemoryArchiveRepository {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        // Insert a test project (required for foreign key)
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('test-project', 'Test Project', '/test/path')",
            [],
        )
        .unwrap();
        SqliteMemoryArchiveRepository::new(conn)
    }

    #[tokio::test]
    async fn test_create_and_get_job() {
        let repo = setup_test_repo().await;
        let project_id = ProjectId::from_string("test-project".to_string());
        let payload = ArchiveJobPayload::memory_snapshot("mem_123");
        let job =
            MemoryArchiveJob::new(project_id.clone(), ArchiveJobType::MemorySnapshot, payload);

        let created_job = repo.create(job.clone()).await.unwrap();
        assert_eq!(created_job.id, job.id);

        let retrieved_job = repo.get_by_id(&job.id).await.unwrap();
        assert!(retrieved_job.is_some());
        assert_eq!(retrieved_job.unwrap().id, job.id);
    }

    #[tokio::test]
    async fn test_claim_next() {
        let repo = setup_test_repo().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create two jobs
        let job1 = MemoryArchiveJob::new(
            project_id.clone(),
            ArchiveJobType::MemorySnapshot,
            ArchiveJobPayload::memory_snapshot("mem_1"),
        );
        let job2 = MemoryArchiveJob::new(
            project_id.clone(),
            ArchiveJobType::RuleSnapshot,
            ArchiveJobPayload::rule_snapshot("rule_1"),
        );

        repo.create(job1.clone()).await.unwrap();
        // Add a small delay to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        repo.create(job2.clone()).await.unwrap();

        // Claim the first job (oldest)
        let claimed = repo.claim_next().await.unwrap();
        assert!(claimed.is_some());
        let claimed_job = claimed.unwrap();
        assert_eq!(claimed_job.id, job1.id);
        assert_eq!(claimed_job.status, ArchiveJobStatus::Running);

        // Claim the second job
        let claimed2 = repo.claim_next().await.unwrap();
        assert!(claimed2.is_some());
        assert_eq!(claimed2.unwrap().id, job2.id);

        // No more claimable jobs
        let claimed3 = repo.claim_next().await.unwrap();
        assert!(claimed3.is_none());
    }

    #[tokio::test]
    async fn test_update_job_status() {
        let repo = setup_test_repo().await;
        let project_id = ProjectId::from_string("test-project".to_string());
        let mut job = MemoryArchiveJob::new(
            project_id,
            ArchiveJobType::MemorySnapshot,
            ArchiveJobPayload::memory_snapshot("mem_123"),
        );

        repo.create(job.clone()).await.unwrap();

        job.start();
        repo.update(&job).await.unwrap();

        let updated = repo.get_by_id(&job.id).await.unwrap().unwrap();
        assert_eq!(updated.status, ArchiveJobStatus::Running);
        assert!(updated.started_at.is_some());

        job.complete();
        repo.update(&job).await.unwrap();

        let completed = repo.get_by_id(&job.id).await.unwrap().unwrap();
        assert_eq!(completed.status, ArchiveJobStatus::Done);
        assert!(completed.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_get_by_status() {
        let repo = setup_test_repo().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        let mut job1 = MemoryArchiveJob::new(
            project_id.clone(),
            ArchiveJobType::MemorySnapshot,
            ArchiveJobPayload::memory_snapshot("mem_1"),
        );
        let job2 = MemoryArchiveJob::new(
            project_id,
            ArchiveJobType::RuleSnapshot,
            ArchiveJobPayload::rule_snapshot("rule_1"),
        );

        repo.create(job1.clone()).await.unwrap();
        repo.create(job2).await.unwrap();

        // Mark job1 as running
        job1.start();
        repo.update(&job1).await.unwrap();

        let pending_jobs = repo.get_by_status(ArchiveJobStatus::Pending).await.unwrap();
        assert_eq!(pending_jobs.len(), 1);

        let running_jobs = repo.get_by_status(ArchiveJobStatus::Running).await.unwrap();
        assert_eq!(running_jobs.len(), 1);
    }

    #[tokio::test]
    async fn test_count_claimable() {
        let repo = setup_test_repo().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        assert_eq!(repo.count_claimable().await.unwrap(), 0);

        let job1 = MemoryArchiveJob::new(
            project_id.clone(),
            ArchiveJobType::MemorySnapshot,
            ArchiveJobPayload::memory_snapshot("mem_1"),
        );
        let mut job2 = MemoryArchiveJob::new(
            project_id,
            ArchiveJobType::RuleSnapshot,
            ArchiveJobPayload::rule_snapshot("rule_1"),
        );

        repo.create(job1).await.unwrap();
        repo.create(job2.clone()).await.unwrap();

        assert_eq!(repo.count_claimable().await.unwrap(), 2);

        // Mark job2 as running
        job2.start();
        repo.update(&job2).await.unwrap();

        assert_eq!(repo.count_claimable().await.unwrap(), 1);

        // Fail job2 - should be claimable again
        job2.fail("Test error");
        repo.update(&job2).await.unwrap();

        assert_eq!(repo.count_claimable().await.unwrap(), 2);
    }
}
