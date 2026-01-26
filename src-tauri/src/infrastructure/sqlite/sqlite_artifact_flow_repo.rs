// SQLite-based ArtifactFlowRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use rusqlite::Connection;

use crate::domain::entities::{ArtifactFlow, ArtifactFlowId};
use crate::domain::repositories::ArtifactFlowRepository;
use crate::error::{AppError, AppResult};

/// SQLite implementation of ArtifactFlowRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteArtifactFlowRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteArtifactFlowRepository {
    /// Create a new SQLite artifact flow repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    /// Create from an Arc-wrapped mutex connection (for sharing)
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Parse an ArtifactFlow from a database row
    fn flow_from_row(row: &rusqlite::Row<'_>) -> Result<ArtifactFlow, rusqlite::Error> {
        let id: String = row.get(0)?;
        let name: String = row.get(1)?;
        let trigger_json: String = row.get(2)?;
        let steps_json: String = row.get(3)?;
        let is_active: i32 = row.get(4)?;
        let created_at: String = row.get(5)?;

        // Parse the JSON fields
        let trigger = serde_json::from_str(&trigger_json)
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let steps = serde_json::from_str(&steps_json)
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let created_at_parsed = chrono::DateTime::parse_from_rfc3339(&created_at)
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?
            .with_timezone(&chrono::Utc);

        Ok(ArtifactFlow {
            id: ArtifactFlowId::from_string(id),
            name,
            trigger,
            steps,
            is_active: is_active != 0,
            created_at: created_at_parsed,
        })
    }
}

#[async_trait]
impl ArtifactFlowRepository for SqliteArtifactFlowRepository {
    async fn create(&self, flow: ArtifactFlow) -> AppResult<ArtifactFlow> {
        let conn = self.conn.lock().await;

        let trigger_json = serde_json::to_string(&flow.trigger)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;
        let steps_json = serde_json::to_string(&flow.steps)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;
        let created_at_str = flow.created_at.to_rfc3339();

        conn.execute(
            "INSERT INTO artifact_flows (id, name, trigger_json, steps_json, is_active, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                flow.id.as_str(),
                flow.name,
                trigger_json,
                steps_json,
                if flow.is_active { 1 } else { 0 },
                created_at_str,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(flow)
    }

    async fn get_by_id(&self, id: &ArtifactFlowId) -> AppResult<Option<ArtifactFlow>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, name, trigger_json, steps_json, is_active, created_at
             FROM artifact_flows WHERE id = ?1",
            [id.as_str()],
            Self::flow_from_row,
        );

        match result {
            Ok(flow) => Ok(Some(flow)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_all(&self) -> AppResult<Vec<ArtifactFlow>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, trigger_json, steps_json, is_active, created_at
                 FROM artifact_flows ORDER BY name ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let flows = stmt
            .query_map([], Self::flow_from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(flows)
    }

    async fn get_active(&self) -> AppResult<Vec<ArtifactFlow>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, trigger_json, steps_json, is_active, created_at
                 FROM artifact_flows WHERE is_active = 1 ORDER BY name ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let flows = stmt
            .query_map([], Self::flow_from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(flows)
    }

    async fn update(&self, flow: &ArtifactFlow) -> AppResult<()> {
        let conn = self.conn.lock().await;

        let trigger_json = serde_json::to_string(&flow.trigger)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;
        let steps_json = serde_json::to_string(&flow.steps)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;

        conn.execute(
            "UPDATE artifact_flows SET name = ?2, trigger_json = ?3, steps_json = ?4, is_active = ?5
             WHERE id = ?1",
            rusqlite::params![
                flow.id.as_str(),
                flow.name,
                trigger_json,
                steps_json,
                if flow.is_active { 1 } else { 0 },
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: &ArtifactFlowId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute("DELETE FROM artifact_flows WHERE id = ?1", [id.as_str()])
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn set_active(&self, id: &ArtifactFlowId, is_active: bool) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "UPDATE artifact_flows SET is_active = ?2 WHERE id = ?1",
            rusqlite::params![id.as_str(), if is_active { 1 } else { 0 }],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn exists(&self, id: &ArtifactFlowId) -> AppResult<bool> {
        let conn = self.conn.lock().await;

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM artifact_flows WHERE id = ?1",
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
    use crate::domain::entities::{
        ArtifactBucketId, ArtifactFlowFilter, ArtifactFlowStep, ArtifactFlowTrigger, ArtifactType,
    };
    use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

    fn setup_test_db() -> Connection {
        let conn = open_memory_connection().expect("Failed to open memory connection");
        run_migrations(&conn).expect("Failed to run migrations");
        conn
    }

    fn create_test_flow() -> ArtifactFlow {
        ArtifactFlow::new("Test Flow", ArtifactFlowTrigger::on_artifact_created())
            .with_step(ArtifactFlowStep::copy(ArtifactBucketId::from_string(
                "test-bucket",
            )))
    }

    fn create_flow_with_filter() -> ArtifactFlow {
        ArtifactFlow::new(
            "Filtered Flow",
            ArtifactFlowTrigger::on_artifact_created().with_filter(
                ArtifactFlowFilter::new()
                    .with_artifact_types(vec![ArtifactType::Recommendations])
                    .with_source_bucket(ArtifactBucketId::from_string("research-outputs")),
            ),
        )
        .with_step(ArtifactFlowStep::copy(ArtifactBucketId::from_string(
            "prd-library",
        )))
        .with_step(ArtifactFlowStep::spawn_process(
            "task_decomposition",
            "orchestrator",
        ))
    }

    #[tokio::test]
    async fn test_create_artifact_flow() {
        let conn = setup_test_db();
        let repo = SqliteArtifactFlowRepository::new(conn);
        let flow = create_test_flow();

        let result = repo.create(flow.clone()).await;
        assert!(result.is_ok());

        let created = result.unwrap();
        assert_eq!(created.id, flow.id);
        assert_eq!(created.name, "Test Flow");
    }

    #[tokio::test]
    async fn test_get_by_id_found() {
        let conn = setup_test_db();
        let repo = SqliteArtifactFlowRepository::new(conn);
        let flow = create_test_flow();

        repo.create(flow.clone()).await.unwrap();

        let result = repo.get_by_id(&flow.id).await;
        assert!(result.is_ok());

        let found = result.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test Flow");
    }

    #[tokio::test]
    async fn test_get_by_id_not_found() {
        let conn = setup_test_db();
        let repo = SqliteArtifactFlowRepository::new(conn);
        let id = ArtifactFlowId::new();

        let result = repo.get_by_id(&id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_all_empty() {
        let conn = setup_test_db();
        let repo = SqliteArtifactFlowRepository::new(conn);

        let result = repo.get_all().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_all_with_flows() {
        let conn = setup_test_db();
        let repo = SqliteArtifactFlowRepository::new(conn);

        let flow1 = create_test_flow();
        let flow2 = create_flow_with_filter();

        repo.create(flow1).await.unwrap();
        repo.create(flow2).await.unwrap();

        let result = repo.get_all().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_get_all_returns_sorted_by_name() {
        let conn = setup_test_db();
        let repo = SqliteArtifactFlowRepository::new(conn);

        let mut flow1 = create_test_flow();
        flow1.name = "Zebra Flow".to_string();

        let mut flow2 = create_test_flow();
        flow2.id = ArtifactFlowId::new();
        flow2.name = "Alpha Flow".to_string();

        repo.create(flow1).await.unwrap();
        repo.create(flow2).await.unwrap();

        let result = repo.get_all().await.unwrap();
        assert_eq!(result[0].name, "Alpha Flow");
        assert_eq!(result[1].name, "Zebra Flow");
    }

    #[tokio::test]
    async fn test_get_active_filters_inactive() {
        let conn = setup_test_db();
        let repo = SqliteArtifactFlowRepository::new(conn);

        let active_flow = create_test_flow();
        let inactive_flow = create_flow_with_filter().set_active(false);

        repo.create(active_flow.clone()).await.unwrap();
        repo.create(inactive_flow).await.unwrap();

        let result = repo.get_active().await;
        assert!(result.is_ok());

        let active = result.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, active_flow.id);
    }

    #[tokio::test]
    async fn test_get_active_returns_all_active() {
        let conn = setup_test_db();
        let repo = SqliteArtifactFlowRepository::new(conn);

        let flow1 = create_test_flow();
        let flow2 = create_flow_with_filter();

        repo.create(flow1).await.unwrap();
        repo.create(flow2).await.unwrap();

        let result = repo.get_active().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_update_flow() {
        let conn = setup_test_db();
        let repo = SqliteArtifactFlowRepository::new(conn);

        let mut flow = create_test_flow();
        repo.create(flow.clone()).await.unwrap();

        flow.name = "Updated Name".to_string();
        flow.is_active = false;

        let result = repo.update(&flow).await;
        assert!(result.is_ok());

        let updated = repo.get_by_id(&flow.id).await.unwrap().unwrap();
        assert_eq!(updated.name, "Updated Name");
        assert!(!updated.is_active);
    }

    #[tokio::test]
    async fn test_delete_flow() {
        let conn = setup_test_db();
        let repo = SqliteArtifactFlowRepository::new(conn);

        let flow = create_test_flow();
        repo.create(flow.clone()).await.unwrap();

        let result = repo.delete(&flow.id).await;
        assert!(result.is_ok());

        let found = repo.get_by_id(&flow.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_set_active_true() {
        let conn = setup_test_db();
        let repo = SqliteArtifactFlowRepository::new(conn);

        let flow = create_test_flow().set_active(false);
        repo.create(flow.clone()).await.unwrap();

        repo.set_active(&flow.id, true).await.unwrap();

        let updated = repo.get_by_id(&flow.id).await.unwrap().unwrap();
        assert!(updated.is_active);
    }

    #[tokio::test]
    async fn test_set_active_false() {
        let conn = setup_test_db();
        let repo = SqliteArtifactFlowRepository::new(conn);

        let flow = create_test_flow(); // default is active
        repo.create(flow.clone()).await.unwrap();

        repo.set_active(&flow.id, false).await.unwrap();

        let updated = repo.get_by_id(&flow.id).await.unwrap().unwrap();
        assert!(!updated.is_active);
    }

    #[tokio::test]
    async fn test_exists_true() {
        let conn = setup_test_db();
        let repo = SqliteArtifactFlowRepository::new(conn);

        let flow = create_test_flow();
        repo.create(flow.clone()).await.unwrap();

        let result = repo.exists(&flow.id).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_exists_false() {
        let conn = setup_test_db();
        let repo = SqliteArtifactFlowRepository::new(conn);

        let id = ArtifactFlowId::new();

        let result = repo.exists(&id).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_trigger_filter_preserved() {
        let conn = setup_test_db();
        let repo = SqliteArtifactFlowRepository::new(conn);

        let flow = create_flow_with_filter();
        repo.create(flow.clone()).await.unwrap();

        let loaded = repo.get_by_id(&flow.id).await.unwrap().unwrap();

        // Verify trigger and filter are preserved
        assert!(loaded.trigger.filter.is_some());
        let filter = loaded.trigger.filter.as_ref().unwrap();
        assert!(filter.artifact_types.is_some());
        assert_eq!(
            filter.artifact_types.as_ref().unwrap()[0],
            ArtifactType::Recommendations
        );
        assert!(filter.source_bucket.is_some());
        assert_eq!(
            filter.source_bucket.as_ref().unwrap().as_str(),
            "research-outputs"
        );
    }

    #[tokio::test]
    async fn test_multiple_steps_preserved() {
        let conn = setup_test_db();
        let repo = SqliteArtifactFlowRepository::new(conn);

        let flow = create_flow_with_filter();
        repo.create(flow.clone()).await.unwrap();

        let loaded = repo.get_by_id(&flow.id).await.unwrap().unwrap();

        assert_eq!(loaded.steps.len(), 2);
        assert!(loaded.steps[0].is_copy());
        assert!(loaded.steps[1].is_spawn_process());

        // Verify step details
        if let crate::domain::entities::ArtifactFlowStep::Copy { to_bucket } = &loaded.steps[0] {
            assert_eq!(to_bucket.as_str(), "prd-library");
        } else {
            panic!("Expected copy step");
        }

        if let crate::domain::entities::ArtifactFlowStep::SpawnProcess {
            process_type,
            agent_profile,
        } = &loaded.steps[1]
        {
            assert_eq!(process_type, "task_decomposition");
            assert_eq!(agent_profile, "orchestrator");
        } else {
            panic!("Expected spawn_process step");
        }
    }

    #[tokio::test]
    async fn test_created_at_preserved() {
        let conn = setup_test_db();
        let repo = SqliteArtifactFlowRepository::new(conn);

        let flow = create_test_flow();
        let original_created_at = flow.created_at;
        repo.create(flow.clone()).await.unwrap();

        let loaded = repo.get_by_id(&flow.id).await.unwrap().unwrap();

        // Timestamps should match (allowing for microsecond precision differences)
        let diff = (loaded.created_at - original_created_at)
            .num_milliseconds()
            .abs();
        assert!(diff < 1000, "Timestamps differ by {}ms", diff);
    }

    #[tokio::test]
    async fn test_from_shared_connection() {
        let conn = setup_test_db();
        let shared = Arc::new(Mutex::new(conn));

        let repo1 = SqliteArtifactFlowRepository::from_shared(shared.clone());
        let repo2 = SqliteArtifactFlowRepository::from_shared(shared.clone());

        // Create via repo1
        let flow = create_test_flow();
        repo1.create(flow.clone()).await.unwrap();

        // Read via repo2
        let found = repo2.get_by_id(&flow.id).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_update_steps() {
        let conn = setup_test_db();
        let repo = SqliteArtifactFlowRepository::new(conn);

        let mut flow = create_test_flow();
        repo.create(flow.clone()).await.unwrap();

        // Add a new step
        flow.steps.push(ArtifactFlowStep::spawn_process(
            "verification",
            "reviewer",
        ));
        repo.update(&flow).await.unwrap();

        let loaded = repo.get_by_id(&flow.id).await.unwrap().unwrap();
        assert_eq!(loaded.steps.len(), 2);
    }

    #[tokio::test]
    async fn test_update_trigger() {
        let conn = setup_test_db();
        let repo = SqliteArtifactFlowRepository::new(conn);

        let mut flow = create_test_flow();
        repo.create(flow.clone()).await.unwrap();

        // Change trigger
        flow.trigger = ArtifactFlowTrigger::on_task_completed();
        repo.update(&flow).await.unwrap();

        let loaded = repo.get_by_id(&flow.id).await.unwrap().unwrap();
        assert_eq!(
            loaded.trigger.event,
            crate::domain::entities::ArtifactFlowEvent::TaskCompleted
        );
    }
}
