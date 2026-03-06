// SQLite-based ProcessRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use rusqlite::Connection;

use crate::domain::entities::research::{
    ResearchBrief, ResearchDepth, ResearchOutput, ResearchProcess, ResearchProcessId,
    ResearchProcessStatus, ResearchProgress,
};
use crate::domain::repositories::ProcessRepository;
use crate::error::{AppError, AppResult};

use super::DbConnection;

/// SQLite implementation of ProcessRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteProcessRepository {
    db: DbConnection,
}

impl SqliteProcessRepository {
    /// Create a new SQLite process repository with the given connection
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

    /// Parse a ResearchProcess from a database row
    fn process_from_row(row: &rusqlite::Row<'_>) -> Result<ResearchProcess, rusqlite::Error> {
        let id: String = row.get(0)?;
        let _process_type: String = row.get(1)?; // "research" for now
        let name: String = row.get(2)?;
        let config_json: String = row.get(3)?;
        let status: String = row.get(4)?;
        let current_iteration: i32 = row.get(5)?;
        let created_at: String = row.get(6)?;
        let started_at: Option<String> = row.get(7)?;
        let completed_at: Option<String> = row.get(8)?;

        // Parse config JSON which contains all process details
        let config: ProcessConfig = serde_json::from_str(&config_json)
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

        let created_at_parsed = chrono::DateTime::parse_from_rfc3339(&created_at)
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?
            .with_timezone(&chrono::Utc);

        let started_at_parsed = started_at
            .map(|s| {
                chrono::DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&chrono::Utc))
            })
            .transpose()
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

        let completed_at_parsed = completed_at
            .map(|s| {
                chrono::DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&chrono::Utc))
            })
            .transpose()
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

        let status_parsed: ResearchProcessStatus = status.parse().map_err(
            |e: crate::domain::entities::research::ParseResearchProcessStatusError| {
                rusqlite::Error::InvalidParameterName(e.to_string())
            },
        )?;

        // Reconstruct progress from stored fields
        let mut progress = ResearchProgress::new();
        progress.status = status_parsed;
        progress.current_iteration = current_iteration as u32;
        progress.last_checkpoint = config.last_checkpoint;
        progress.error_message = config.error_message;

        Ok(ResearchProcess {
            id: ResearchProcessId::from_string(id),
            name,
            brief: config.brief,
            depth: config.depth,
            agent_profile_id: config.agent_profile_id,
            output: config.output,
            progress,
            created_at: created_at_parsed,
            started_at: started_at_parsed,
            completed_at: completed_at_parsed,
        })
    }
}

/// Internal config structure for JSON serialization
#[derive(serde::Serialize, serde::Deserialize)]
struct ProcessConfig {
    brief: ResearchBrief,
    depth: ResearchDepth,
    agent_profile_id: String,
    output: ResearchOutput,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_checkpoint: Option<crate::domain::entities::ArtifactId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_message: Option<String>,
}

impl From<&ResearchProcess> for ProcessConfig {
    fn from(process: &ResearchProcess) -> Self {
        Self {
            brief: process.brief.clone(),
            depth: process.depth.clone(),
            agent_profile_id: process.agent_profile_id.clone(),
            output: process.output.clone(),
            last_checkpoint: process.progress.last_checkpoint.clone(),
            error_message: process.progress.error_message.clone(),
        }
    }
}

#[async_trait]
impl ProcessRepository for SqliteProcessRepository {
    async fn create(&self, process: ResearchProcess) -> AppResult<ResearchProcess> {
        let config = ProcessConfig::from(&process);
        let config_json = serde_json::to_string(&config)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;
        let id = process.id.as_str().to_string();
        let name = process.name.clone();
        let status_str = process.status().as_str().to_string();
        let current_iteration = process.progress.current_iteration as i32;
        let created_at_str = process.created_at.to_rfc3339();
        let started_at_str = process.started_at.map(|dt| dt.to_rfc3339());
        let completed_at_str = process.completed_at.map(|dt| dt.to_rfc3339());

        self.db
            .run(move |conn| {
                conn.execute(
                    "INSERT INTO processes (id, type, name, config_json, status, current_iteration, created_at, started_at, completed_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![
                        id,
                        "research",
                        name,
                        config_json,
                        status_str,
                        current_iteration,
                        created_at_str,
                        started_at_str,
                        completed_at_str,
                    ],
                )?;
                Ok(process)
            })
            .await
    }

    async fn get_by_id(&self, id: &ResearchProcessId) -> AppResult<Option<ResearchProcess>> {
        let id = id.as_str().to_string();
        self.db
            .query_optional(move |conn| {
                conn.query_row(
                    "SELECT id, type, name, config_json, status, current_iteration, created_at, started_at, completed_at
                     FROM processes WHERE id = ?1",
                    [id.as_str()],
                    SqliteProcessRepository::process_from_row,
                )
            })
            .await
    }

    async fn get_all(&self) -> AppResult<Vec<ResearchProcess>> {
        self.db
            .run(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, type, name, config_json, status, current_iteration, created_at, started_at, completed_at
                     FROM processes WHERE type = 'research' ORDER BY created_at DESC",
                )?;
                let processes = stmt
                    .query_map([], SqliteProcessRepository::process_from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(processes)
            })
            .await
    }

    async fn get_by_status(
        &self,
        status: ResearchProcessStatus,
    ) -> AppResult<Vec<ResearchProcess>> {
        let status_str = status.as_str().to_string();
        self.db
            .run(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, type, name, config_json, status, current_iteration, created_at, started_at, completed_at
                     FROM processes WHERE type = 'research' AND status = ?1 ORDER BY created_at DESC",
                )?;
                let processes = stmt
                    .query_map([status_str.as_str()], SqliteProcessRepository::process_from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(processes)
            })
            .await
    }

    async fn get_active(&self) -> AppResult<Vec<ResearchProcess>> {
        self.db
            .run(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, type, name, config_json, status, current_iteration, created_at, started_at, completed_at
                     FROM processes WHERE type = 'research' AND status IN ('pending', 'running') ORDER BY created_at DESC",
                )?;
                let processes = stmt
                    .query_map([], SqliteProcessRepository::process_from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(processes)
            })
            .await
    }

    async fn update_progress(&self, process: &ResearchProcess) -> AppResult<()> {
        let config = ProcessConfig::from(process);
        let config_json = serde_json::to_string(&config)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;
        let id = process.id.as_str().to_string();
        let status_str = process.status().as_str().to_string();
        let current_iteration = process.progress.current_iteration as i32;
        let started_at_str = process.started_at.map(|dt| dt.to_rfc3339());
        let completed_at_str = process.completed_at.map(|dt| dt.to_rfc3339());

        self.db
            .run(move |conn| {
                conn.execute(
                    "UPDATE processes SET config_json = ?2, status = ?3, current_iteration = ?4, started_at = ?5, completed_at = ?6
                     WHERE id = ?1",
                    rusqlite::params![
                        id,
                        config_json,
                        status_str,
                        current_iteration,
                        started_at_str,
                        completed_at_str,
                    ],
                )?;
                Ok(())
            })
            .await
    }

    async fn update(&self, process: &ResearchProcess) -> AppResult<()> {
        let config = ProcessConfig::from(process);
        let config_json = serde_json::to_string(&config)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;
        let id = process.id.as_str().to_string();
        let name = process.name.clone();
        let status_str = process.status().as_str().to_string();
        let current_iteration = process.progress.current_iteration as i32;
        let started_at_str = process.started_at.map(|dt| dt.to_rfc3339());
        let completed_at_str = process.completed_at.map(|dt| dt.to_rfc3339());

        self.db
            .run(move |conn| {
                conn.execute(
                    "UPDATE processes SET name = ?2, config_json = ?3, status = ?4, current_iteration = ?5, started_at = ?6, completed_at = ?7
                     WHERE id = ?1",
                    rusqlite::params![
                        id,
                        name,
                        config_json,
                        status_str,
                        current_iteration,
                        started_at_str,
                        completed_at_str,
                    ],
                )?;
                Ok(())
            })
            .await
    }

    async fn complete(&self, id: &ResearchProcessId) -> AppResult<()> {
        let id = id.as_str().to_string();
        let completed_at = Utc::now().to_rfc3339();

        self.db
            .run(move |conn| {
                conn.execute(
                    "UPDATE processes SET status = 'completed', completed_at = ?2 WHERE id = ?1",
                    rusqlite::params![id, completed_at],
                )?;
                Ok(())
            })
            .await
    }

    async fn fail(&self, id: &ResearchProcessId, error: &str) -> AppResult<()> {
        let id = id.as_str().to_string();
        let error = error.to_string();
        let completed_at = Utc::now().to_rfc3339();

        self.db
            .run(move |conn| {
                // First get the current config to update the error message
                let current_config: String = conn.query_row(
                    "SELECT config_json FROM processes WHERE id = ?1",
                    [id.as_str()],
                    |row| row.get(0),
                )?;

                let mut config: ProcessConfig = serde_json::from_str(&current_config)
                    .map_err(|e| AppError::Database(format!("JSON parse error: {}", e)))?;
                config.error_message = Some(error);

                let updated_config = serde_json::to_string(&config)
                    .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;

                conn.execute(
                    "UPDATE processes SET status = 'failed', config_json = ?2, completed_at = ?3 WHERE id = ?1",
                    rusqlite::params![id, updated_config, completed_at],
                )?;
                Ok(())
            })
            .await
    }

    async fn delete(&self, id: &ResearchProcessId) -> AppResult<()> {
        let id = id.as_str().to_string();

        self.db
            .run(move |conn| {
                conn.execute("DELETE FROM processes WHERE id = ?1", [id.as_str()])?;
                Ok(())
            })
            .await
    }

    async fn exists(&self, id: &ResearchProcessId) -> AppResult<bool> {
        let id = id.as_str().to_string();

        self.db
            .run(move |conn| {
                let count: i32 = conn.query_row(
                    "SELECT COUNT(*) FROM processes WHERE id = ?1",
                    [id.as_str()],
                    |row| row.get(0),
                )?;
                Ok(count > 0)
            })
            .await
    }

    async fn fail_all_active(&self, _reason: &str) -> AppResult<usize> {
        let completed_at = Utc::now().to_rfc3339();

        self.db
            .run(move |conn| {
                let count = conn.execute(
                    "UPDATE processes SET status = 'failed', completed_at = ?1 WHERE status IN ('pending', 'running')",
                    rusqlite::params![completed_at],
                )?;
                Ok(count)
            })
            .await
    }
}

#[cfg(test)]
#[path = "sqlite_process_repo_tests.rs"]
mod tests;
