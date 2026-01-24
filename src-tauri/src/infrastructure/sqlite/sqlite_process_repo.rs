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

/// SQLite implementation of ProcessRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteProcessRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteProcessRepository {
    /// Create a new SQLite process repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    /// Create from an Arc-wrapped mutex connection (for sharing)
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
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
                chrono::DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
            })
            .transpose()
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

        let completed_at_parsed = completed_at
            .map(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
            })
            .transpose()
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

        let status_parsed: ResearchProcessStatus = status
            .parse()
            .map_err(|e: crate::domain::entities::research::ParseResearchProcessStatusError| {
                rusqlite::Error::InvalidParameterName(e.to_string())
            })?;

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
        let conn = self.conn.lock().await;

        let config = ProcessConfig::from(&process);
        let config_json = serde_json::to_string(&config)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;
        let created_at_str = process.created_at.to_rfc3339();
        let started_at_str = process.started_at.map(|dt| dt.to_rfc3339());
        let completed_at_str = process.completed_at.map(|dt| dt.to_rfc3339());

        conn.execute(
            "INSERT INTO processes (id, type, name, config_json, status, current_iteration, created_at, started_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                process.id.as_str(),
                "research",
                process.name,
                config_json,
                process.status().as_str(),
                process.progress.current_iteration as i32,
                created_at_str,
                started_at_str,
                completed_at_str,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(process)
    }

    async fn get_by_id(&self, id: &ResearchProcessId) -> AppResult<Option<ResearchProcess>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, type, name, config_json, status, current_iteration, created_at, started_at, completed_at
             FROM processes WHERE id = ?1",
            [id.as_str()],
            |row| Self::process_from_row(row),
        );

        match result {
            Ok(process) => Ok(Some(process)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_all(&self) -> AppResult<Vec<ResearchProcess>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, type, name, config_json, status, current_iteration, created_at, started_at, completed_at
                 FROM processes WHERE type = 'research' ORDER BY created_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let processes = stmt
            .query_map([], |row| Self::process_from_row(row))
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(processes)
    }

    async fn get_by_status(
        &self,
        status: ResearchProcessStatus,
    ) -> AppResult<Vec<ResearchProcess>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, type, name, config_json, status, current_iteration, created_at, started_at, completed_at
                 FROM processes WHERE type = 'research' AND status = ?1 ORDER BY created_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let processes = stmt
            .query_map([status.as_str()], |row| Self::process_from_row(row))
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(processes)
    }

    async fn get_active(&self) -> AppResult<Vec<ResearchProcess>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, type, name, config_json, status, current_iteration, created_at, started_at, completed_at
                 FROM processes WHERE type = 'research' AND status IN ('pending', 'running') ORDER BY created_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let processes = stmt
            .query_map([], |row| Self::process_from_row(row))
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(processes)
    }

    async fn update_progress(&self, process: &ResearchProcess) -> AppResult<()> {
        let conn = self.conn.lock().await;

        let config = ProcessConfig::from(process);
        let config_json = serde_json::to_string(&config)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;
        let started_at_str = process.started_at.map(|dt| dt.to_rfc3339());
        let completed_at_str = process.completed_at.map(|dt| dt.to_rfc3339());

        conn.execute(
            "UPDATE processes SET config_json = ?2, status = ?3, current_iteration = ?4, started_at = ?5, completed_at = ?6
             WHERE id = ?1",
            rusqlite::params![
                process.id.as_str(),
                config_json,
                process.status().as_str(),
                process.progress.current_iteration as i32,
                started_at_str,
                completed_at_str,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn update(&self, process: &ResearchProcess) -> AppResult<()> {
        let conn = self.conn.lock().await;

        let config = ProcessConfig::from(process);
        let config_json = serde_json::to_string(&config)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;
        let started_at_str = process.started_at.map(|dt| dt.to_rfc3339());
        let completed_at_str = process.completed_at.map(|dt| dt.to_rfc3339());

        conn.execute(
            "UPDATE processes SET name = ?2, config_json = ?3, status = ?4, current_iteration = ?5, started_at = ?6, completed_at = ?7
             WHERE id = ?1",
            rusqlite::params![
                process.id.as_str(),
                process.name,
                config_json,
                process.status().as_str(),
                process.progress.current_iteration as i32,
                started_at_str,
                completed_at_str,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn complete(&self, id: &ResearchProcessId) -> AppResult<()> {
        let conn = self.conn.lock().await;
        let completed_at = Utc::now().to_rfc3339();

        conn.execute(
            "UPDATE processes SET status = 'completed', completed_at = ?2 WHERE id = ?1",
            rusqlite::params![id.as_str(), completed_at],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn fail(&self, id: &ResearchProcessId, error: &str) -> AppResult<()> {
        let conn = self.conn.lock().await;
        let completed_at = Utc::now().to_rfc3339();

        // First get the current config to update the error message
        let current_config: String = conn
            .query_row(
                "SELECT config_json FROM processes WHERE id = ?1",
                [id.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut config: ProcessConfig = serde_json::from_str(&current_config)
            .map_err(|e| AppError::Database(format!("JSON parse error: {}", e)))?;
        config.error_message = Some(error.to_string());

        let updated_config = serde_json::to_string(&config)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;

        conn.execute(
            "UPDATE processes SET status = 'failed', config_json = ?2, completed_at = ?3 WHERE id = ?1",
            rusqlite::params![id.as_str(), updated_config, completed_at],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: &ResearchProcessId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute("DELETE FROM processes WHERE id = ?1", [id.as_str()])
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn exists(&self, id: &ResearchProcessId) -> AppResult<bool> {
        let conn = self.conn.lock().await;

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM processes WHERE id = ?1",
                [id.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::research::{CustomDepth, ResearchDepthPreset};
    use crate::domain::entities::ArtifactType;
    use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

    fn setup_test_db() -> Connection {
        let conn = open_memory_connection().expect("Failed to open memory connection");
        run_migrations(&conn).expect("Failed to run migrations");
        conn
    }

    fn create_test_process() -> ResearchProcess {
        let brief = ResearchBrief::new("What architecture should we use?")
            .with_context("Building a new web application")
            .with_constraint("Must be scalable");
        ResearchProcess::new("Architecture Research", brief, "deep-researcher")
            .with_preset(ResearchDepthPreset::Standard)
    }

    fn create_running_process() -> ResearchProcess {
        let brief = ResearchBrief::new("Which database to choose?");
        let mut process =
            ResearchProcess::new("Database Research", brief, "deep-researcher")
                .with_preset(ResearchDepthPreset::QuickScan);
        process.start();
        process.advance();
        process.advance();
        process
    }

    #[tokio::test]
    async fn test_create_process() {
        let conn = setup_test_db();
        let repo = SqliteProcessRepository::new(conn);
        let process = create_test_process();

        let result = repo.create(process.clone()).await;
        assert!(result.is_ok());

        let created = result.unwrap();
        assert_eq!(created.id, process.id);
        assert_eq!(created.name, "Architecture Research");
    }

    #[tokio::test]
    async fn test_get_by_id_found() {
        let conn = setup_test_db();
        let repo = SqliteProcessRepository::new(conn);
        let process = create_test_process();

        repo.create(process.clone()).await.unwrap();

        let result = repo.get_by_id(&process.id).await;
        assert!(result.is_ok());

        let found = result.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Architecture Research");
    }

    #[tokio::test]
    async fn test_get_by_id_not_found() {
        let conn = setup_test_db();
        let repo = SqliteProcessRepository::new(conn);
        let id = ResearchProcessId::new();

        let result = repo.get_by_id(&id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_all_empty() {
        let conn = setup_test_db();
        let repo = SqliteProcessRepository::new(conn);

        let result = repo.get_all().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_all_with_processes() {
        let conn = setup_test_db();
        let repo = SqliteProcessRepository::new(conn);

        let process1 = create_test_process();
        let process2 = create_running_process();

        repo.create(process1).await.unwrap();
        repo.create(process2).await.unwrap();

        let result = repo.get_all().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_get_by_status_pending() {
        let conn = setup_test_db();
        let repo = SqliteProcessRepository::new(conn);

        let pending = create_test_process();
        let running = create_running_process();

        repo.create(pending).await.unwrap();
        repo.create(running).await.unwrap();

        let result = repo.get_by_status(ResearchProcessStatus::Pending).await;
        assert!(result.is_ok());
        let processes = result.unwrap();
        assert_eq!(processes.len(), 1);
        assert_eq!(processes[0].status(), ResearchProcessStatus::Pending);
    }

    #[tokio::test]
    async fn test_get_by_status_running() {
        let conn = setup_test_db();
        let repo = SqliteProcessRepository::new(conn);

        let pending = create_test_process();
        let running = create_running_process();

        repo.create(pending).await.unwrap();
        repo.create(running).await.unwrap();

        let result = repo.get_by_status(ResearchProcessStatus::Running).await;
        assert!(result.is_ok());
        let processes = result.unwrap();
        assert_eq!(processes.len(), 1);
        assert_eq!(processes[0].status(), ResearchProcessStatus::Running);
    }

    #[tokio::test]
    async fn test_get_active() {
        let conn = setup_test_db();
        let repo = SqliteProcessRepository::new(conn);

        let pending = create_test_process();
        let running = create_running_process();

        // Create a completed process
        let brief = ResearchBrief::new("Completed question");
        let mut completed = ResearchProcess::new("Completed Research", brief, "researcher");
        completed.start();
        completed.complete();

        repo.create(pending).await.unwrap();
        repo.create(running).await.unwrap();
        repo.create(completed).await.unwrap();

        let result = repo.get_active().await;
        assert!(result.is_ok());
        let active = result.unwrap();
        assert_eq!(active.len(), 2);
    }

    #[tokio::test]
    async fn test_update_progress() {
        let conn = setup_test_db();
        let repo = SqliteProcessRepository::new(conn);

        let mut process = create_test_process();
        repo.create(process.clone()).await.unwrap();

        // Update progress
        process.start();
        process.advance();
        process.advance();
        process.advance();

        repo.update_progress(&process).await.unwrap();

        let loaded = repo.get_by_id(&process.id).await.unwrap().unwrap();
        assert_eq!(loaded.progress.current_iteration, 3);
        assert_eq!(loaded.status(), ResearchProcessStatus::Running);
        assert!(loaded.started_at.is_some());
    }

    #[tokio::test]
    async fn test_update() {
        let conn = setup_test_db();
        let repo = SqliteProcessRepository::new(conn);

        let mut process = create_test_process();
        repo.create(process.clone()).await.unwrap();

        process.name = "Updated Research Name".to_string();
        process.start();

        repo.update(&process).await.unwrap();

        let loaded = repo.get_by_id(&process.id).await.unwrap().unwrap();
        assert_eq!(loaded.name, "Updated Research Name");
        assert_eq!(loaded.status(), ResearchProcessStatus::Running);
    }

    #[tokio::test]
    async fn test_complete() {
        let conn = setup_test_db();
        let repo = SqliteProcessRepository::new(conn);

        let process = create_running_process();
        repo.create(process.clone()).await.unwrap();

        repo.complete(&process.id).await.unwrap();

        let loaded = repo.get_by_id(&process.id).await.unwrap().unwrap();
        assert_eq!(loaded.status(), ResearchProcessStatus::Completed);
        assert!(loaded.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_fail() {
        let conn = setup_test_db();
        let repo = SqliteProcessRepository::new(conn);

        let process = create_running_process();
        repo.create(process.clone()).await.unwrap();

        repo.fail(&process.id, "Network timeout").await.unwrap();

        let loaded = repo.get_by_id(&process.id).await.unwrap().unwrap();
        assert_eq!(loaded.status(), ResearchProcessStatus::Failed);
        assert!(loaded.completed_at.is_some());
        assert_eq!(
            loaded.progress.error_message,
            Some("Network timeout".to_string())
        );
    }

    #[tokio::test]
    async fn test_delete() {
        let conn = setup_test_db();
        let repo = SqliteProcessRepository::new(conn);

        let process = create_test_process();
        repo.create(process.clone()).await.unwrap();

        repo.delete(&process.id).await.unwrap();

        let found = repo.get_by_id(&process.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_exists_true() {
        let conn = setup_test_db();
        let repo = SqliteProcessRepository::new(conn);

        let process = create_test_process();
        repo.create(process.clone()).await.unwrap();

        let result = repo.exists(&process.id).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_exists_false() {
        let conn = setup_test_db();
        let repo = SqliteProcessRepository::new(conn);

        let id = ResearchProcessId::new();

        let result = repo.exists(&id).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_brief_preserved() {
        let conn = setup_test_db();
        let repo = SqliteProcessRepository::new(conn);

        let brief = ResearchBrief::new("Main question")
            .with_context("Context info")
            .with_scope("Backend only")
            .with_constraints(["Constraint 1", "Constraint 2"]);
        let process = ResearchProcess::new("Brief Test", brief, "researcher");
        repo.create(process.clone()).await.unwrap();

        let loaded = repo.get_by_id(&process.id).await.unwrap().unwrap();

        assert_eq!(loaded.brief.question, "Main question");
        assert_eq!(loaded.brief.context, Some("Context info".to_string()));
        assert_eq!(loaded.brief.scope, Some("Backend only".to_string()));
        assert_eq!(loaded.brief.constraints.len(), 2);
    }

    #[tokio::test]
    async fn test_preset_depth_preserved() {
        let conn = setup_test_db();
        let repo = SqliteProcessRepository::new(conn);

        let brief = ResearchBrief::new("Question");
        let process = ResearchProcess::new("Depth Test", brief, "researcher")
            .with_preset(ResearchDepthPreset::DeepDive);
        repo.create(process.clone()).await.unwrap();

        let loaded = repo.get_by_id(&process.id).await.unwrap().unwrap();

        assert!(loaded.depth.is_preset());
        let resolved = loaded.resolved_depth();
        assert_eq!(resolved.max_iterations, 200);
        assert_eq!(resolved.timeout_hours, 8.0);
    }

    #[tokio::test]
    async fn test_custom_depth_preserved() {
        let conn = setup_test_db();
        let repo = SqliteProcessRepository::new(conn);

        let brief = ResearchBrief::new("Question");
        let process = ResearchProcess::new("Custom Depth Test", brief, "researcher")
            .with_custom_depth(CustomDepth::new(150, 5.0, 30));
        repo.create(process.clone()).await.unwrap();

        let loaded = repo.get_by_id(&process.id).await.unwrap().unwrap();

        assert!(loaded.depth.is_custom());
        let resolved = loaded.resolved_depth();
        assert_eq!(resolved.max_iterations, 150);
        assert_eq!(resolved.timeout_hours, 5.0);
        assert_eq!(resolved.checkpoint_interval, 30);
    }

    #[tokio::test]
    async fn test_output_config_preserved() {
        let conn = setup_test_db();
        let repo = SqliteProcessRepository::new(conn);

        let brief = ResearchBrief::new("Question");
        let output = ResearchOutput::new("custom-bucket")
            .with_artifact_type(ArtifactType::Findings)
            .with_artifact_type(ArtifactType::Recommendations);
        let process = ResearchProcess::new("Output Test", brief, "researcher")
            .with_output(output);
        repo.create(process.clone()).await.unwrap();

        let loaded = repo.get_by_id(&process.id).await.unwrap().unwrap();

        assert_eq!(loaded.output.target_bucket, "custom-bucket");
        assert_eq!(loaded.output.artifact_types.len(), 2);
        assert!(loaded.output.artifact_types.contains(&ArtifactType::Findings));
        assert!(loaded.output.artifact_types.contains(&ArtifactType::Recommendations));
    }

    #[tokio::test]
    async fn test_timestamps_preserved() {
        let conn = setup_test_db();
        let repo = SqliteProcessRepository::new(conn);

        let process = create_test_process();
        let original_created_at = process.created_at;
        repo.create(process.clone()).await.unwrap();

        let loaded = repo.get_by_id(&process.id).await.unwrap().unwrap();

        // Timestamps should match (allowing for RFC3339 precision)
        let diff = (loaded.created_at - original_created_at)
            .num_milliseconds()
            .abs();
        assert!(diff < 1000, "Timestamps differ by {}ms", diff);
    }

    #[tokio::test]
    async fn test_from_shared_connection() {
        let conn = setup_test_db();
        let shared = Arc::new(Mutex::new(conn));

        let repo1 = SqliteProcessRepository::from_shared(shared.clone());
        let repo2 = SqliteProcessRepository::from_shared(shared.clone());

        // Create via repo1
        let process = create_test_process();
        repo1.create(process.clone()).await.unwrap();

        // Read via repo2
        let found = repo2.get_by_id(&process.id).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_checkpoint_preserved() {
        let conn = setup_test_db();
        let repo = SqliteProcessRepository::new(conn);

        let brief = ResearchBrief::new("Question");
        let mut process = ResearchProcess::new("Checkpoint Test", brief, "researcher");
        process.start();
        let checkpoint_id = crate::domain::entities::ArtifactId::from_string("checkpoint-artifact-1");
        process.checkpoint(checkpoint_id.clone());

        repo.create(process.clone()).await.unwrap();

        let loaded = repo.get_by_id(&process.id).await.unwrap().unwrap();
        assert_eq!(loaded.progress.last_checkpoint, Some(checkpoint_id));
    }

    #[tokio::test]
    async fn test_get_all_ordered_by_created_at_desc() {
        let conn = setup_test_db();
        let repo = SqliteProcessRepository::new(conn);

        // Create processes with slight time differences
        let process1 = create_test_process();
        repo.create(process1.clone()).await.unwrap();

        // Small delay to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let process2 = create_running_process();
        repo.create(process2.clone()).await.unwrap();

        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 2);
        // Most recent first
        assert_eq!(all[0].id, process2.id);
        assert_eq!(all[1].id, process1.id);
    }
}
