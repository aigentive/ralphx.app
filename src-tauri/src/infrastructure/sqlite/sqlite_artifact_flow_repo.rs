// SQLite-based ArtifactFlowRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use rusqlite::Connection;

use super::DbConnection;
use crate::domain::entities::{ArtifactFlow, ArtifactFlowId};
use crate::domain::repositories::ArtifactFlowRepository;
use crate::error::{AppError, AppResult};

/// SQLite implementation of ArtifactFlowRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteArtifactFlowRepository {
    db: DbConnection,
}

impl SqliteArtifactFlowRepository {
    /// Create a new SQLite artifact flow repository with the given connection
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

    /// Parse an ArtifactFlow from a database row
    fn flow_from_row(row: &rusqlite::Row<'_>) -> Result<ArtifactFlow, rusqlite::Error> {
        let id: String = row.get(0)?;
        let name: String = row.get(1)?;
        let trigger_json: String = row.get(2)?;
        let steps_json: String = row.get(3)?;
        let is_active: i32 = row.get(4)?;
        let created_at: String = row.get(5)?;

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
        let trigger_json = serde_json::to_string(&flow.trigger)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;
        let steps_json = serde_json::to_string(&flow.steps)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;
        let created_at_str = flow.created_at.to_rfc3339();

        self.db
            .run(move |conn| {
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
                )?;
                Ok(flow)
            })
            .await
    }

    async fn get_by_id(&self, id: &ArtifactFlowId) -> AppResult<Option<ArtifactFlow>> {
        let id = id.as_str().to_string();
        self.db
            .run(move |conn| {
                let result = conn.query_row(
                    "SELECT id, name, trigger_json, steps_json, is_active, created_at
                     FROM artifact_flows WHERE id = ?1",
                    [id.as_str()],
                    SqliteArtifactFlowRepository::flow_from_row,
                );
                match result {
                    Ok(flow) => Ok(Some(flow)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(AppError::Database(e.to_string())),
                }
            })
            .await
    }

    async fn get_all(&self) -> AppResult<Vec<ArtifactFlow>> {
        self.db
            .run(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, name, trigger_json, steps_json, is_active, created_at
                     FROM artifact_flows ORDER BY name ASC",
                )?;
                let flows = stmt
                    .query_map([], SqliteArtifactFlowRepository::flow_from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(flows)
            })
            .await
    }

    async fn get_active(&self) -> AppResult<Vec<ArtifactFlow>> {
        self.db
            .run(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, name, trigger_json, steps_json, is_active, created_at
                     FROM artifact_flows WHERE is_active = 1 ORDER BY name ASC",
                )?;
                let flows = stmt
                    .query_map([], SqliteArtifactFlowRepository::flow_from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(flows)
            })
            .await
    }

    async fn update(&self, flow: &ArtifactFlow) -> AppResult<()> {
        let trigger_json = serde_json::to_string(&flow.trigger)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;
        let steps_json = serde_json::to_string(&flow.steps)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;
        let id = flow.id.as_str().to_string();
        let name = flow.name.clone();
        let is_active = flow.is_active;

        self.db
            .run(move |conn| {
                conn.execute(
                    "UPDATE artifact_flows SET name = ?2, trigger_json = ?3, steps_json = ?4, is_active = ?5
                     WHERE id = ?1",
                    rusqlite::params![
                        id,
                        name,
                        trigger_json,
                        steps_json,
                        if is_active { 1 } else { 0 },
                    ],
                )?;
                Ok(())
            })
            .await
    }

    async fn delete(&self, id: &ArtifactFlowId) -> AppResult<()> {
        let id = id.as_str().to_string();
        self.db
            .run(move |conn| {
                conn.execute("DELETE FROM artifact_flows WHERE id = ?1", [id.as_str()])?;
                Ok(())
            })
            .await
    }

    async fn set_active(&self, id: &ArtifactFlowId, is_active: bool) -> AppResult<()> {
        let id = id.as_str().to_string();
        self.db
            .run(move |conn| {
                conn.execute(
                    "UPDATE artifact_flows SET is_active = ?2 WHERE id = ?1",
                    rusqlite::params![id.as_str(), if is_active { 1 } else { 0 }],
                )?;
                Ok(())
            })
            .await
    }

    async fn exists(&self, id: &ArtifactFlowId) -> AppResult<bool> {
        let id = id.as_str().to_string();
        self.db
            .run(move |conn| {
                let count: i32 = conn.query_row(
                    "SELECT COUNT(*) FROM artifact_flows WHERE id = ?1",
                    [id.as_str()],
                    |row| row.get(0),
                )?;
                Ok(count > 0)
            })
            .await
    }
}

#[cfg(test)]
#[path = "sqlite_artifact_flow_repo_tests.rs"]
mod tests;
