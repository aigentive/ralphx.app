// SQLite-based MethodologyRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use rusqlite::Connection;

use crate::domain::entities::methodology::{MethodologyExtension, MethodologyId};
use crate::domain::repositories::MethodologyRepository;
use crate::error::{AppError, AppResult};

/// SQLite implementation of MethodologyRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteMethodologyRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteMethodologyRepository {
    /// Create a new SQLite methodology repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    /// Create from an Arc-wrapped mutex connection (for sharing)
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Parse a MethodologyExtension from a database row
    fn methodology_from_row(
        row: &rusqlite::Row<'_>,
    ) -> Result<MethodologyExtension, rusqlite::Error> {
        let id: String = row.get(0)?;
        let name: String = row.get(1)?;
        let description: Option<String> = row.get(2)?;
        let config_json: String = row.get(3)?;
        let is_active: i32 = row.get(4)?;
        let created_at: String = row.get(5)?;

        // Parse config JSON which contains workflow, phases, templates, etc.
        let config: MethodologyConfig = serde_json::from_str(&config_json)
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

        let created_at_parsed = chrono::DateTime::parse_from_rfc3339(&created_at)
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?
            .with_timezone(&chrono::Utc);

        Ok(MethodologyExtension {
            id: MethodologyId::from_string(id),
            name,
            description,
            agent_profiles: config.agent_profiles,
            skills: config.skills,
            workflow: config.workflow,
            phases: config.phases,
            templates: config.templates,
            plan_artifact_config: config.plan_artifact_config,
            plan_templates: config.plan_templates,
            hooks_config: config.hooks_config,
            is_active: is_active != 0,
            created_at: created_at_parsed,
        })
    }
}

/// Internal config structure for JSON serialization
/// Stores the complex fields that don't map to direct columns
#[derive(serde::Serialize, serde::Deserialize)]
struct MethodologyConfig {
    #[serde(default)]
    agent_profiles: Vec<String>,
    #[serde(default)]
    skills: Vec<String>,
    workflow: crate::domain::entities::workflow::WorkflowSchema,
    #[serde(default)]
    phases: Vec<crate::domain::entities::methodology::MethodologyPhase>,
    #[serde(default)]
    templates: Vec<crate::domain::entities::methodology::MethodologyTemplate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    plan_artifact_config:
        Option<crate::domain::entities::methodology::MethodologyPlanArtifactConfig>,
    #[serde(default)]
    plan_templates: Vec<crate::domain::entities::methodology::MethodologyPlanTemplate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hooks_config: Option<serde_json::Value>,
}

impl From<&MethodologyExtension> for MethodologyConfig {
    fn from(methodology: &MethodologyExtension) -> Self {
        Self {
            agent_profiles: methodology.agent_profiles.clone(),
            skills: methodology.skills.clone(),
            workflow: methodology.workflow.clone(),
            phases: methodology.phases.clone(),
            templates: methodology.templates.clone(),
            plan_artifact_config: methodology.plan_artifact_config.clone(),
            plan_templates: methodology.plan_templates.clone(),
            hooks_config: methodology.hooks_config.clone(),
        }
    }
}

#[async_trait]
impl MethodologyRepository for SqliteMethodologyRepository {
    async fn create(&self, methodology: MethodologyExtension) -> AppResult<MethodologyExtension> {
        let conn = self.conn.lock().await;

        let config = MethodologyConfig::from(&methodology);
        let config_json = serde_json::to_string(&config)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;
        let created_at_str = methodology.created_at.to_rfc3339();

        conn.execute(
            "INSERT INTO methodology_extensions (id, name, description, config_json, is_active, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                methodology.id.as_str(),
                methodology.name,
                methodology.description,
                config_json,
                methodology.is_active as i32,
                created_at_str,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(methodology)
    }

    async fn get_by_id(&self, id: &MethodologyId) -> AppResult<Option<MethodologyExtension>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, name, description, config_json, is_active, created_at
             FROM methodology_extensions WHERE id = ?1",
            [id.as_str()],
            |row| Self::methodology_from_row(row),
        );

        match result {
            Ok(methodology) => Ok(Some(methodology)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_all(&self) -> AppResult<Vec<MethodologyExtension>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, config_json, is_active, created_at
                 FROM methodology_extensions ORDER BY name ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let methodologies = stmt
            .query_map([], Self::methodology_from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(methodologies)
    }

    async fn get_active(&self) -> AppResult<Option<MethodologyExtension>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, name, description, config_json, is_active, created_at
             FROM methodology_extensions WHERE is_active = 1",
            [],
            |row| Self::methodology_from_row(row),
        );

        match result {
            Ok(methodology) => Ok(Some(methodology)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn activate(&self, id: &MethodologyId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        // First, deactivate all methodologies
        conn.execute("UPDATE methodology_extensions SET is_active = 0", [])
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Then, activate the specified one
        let updated = conn
            .execute(
                "UPDATE methodology_extensions SET is_active = 1 WHERE id = ?1",
                [id.as_str()],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        if updated == 0 {
            return Err(AppError::NotFound(format!(
                "Methodology not found: {}",
                id.as_str()
            )));
        }

        Ok(())
    }

    async fn deactivate(&self, id: &MethodologyId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        let updated = conn
            .execute(
                "UPDATE methodology_extensions SET is_active = 0 WHERE id = ?1",
                [id.as_str()],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        if updated == 0 {
            return Err(AppError::NotFound(format!(
                "Methodology not found: {}",
                id.as_str()
            )));
        }

        Ok(())
    }

    async fn update(&self, methodology: &MethodologyExtension) -> AppResult<()> {
        let conn = self.conn.lock().await;

        let config = MethodologyConfig::from(methodology);
        let config_json = serde_json::to_string(&config)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;

        let updated = conn
            .execute(
                "UPDATE methodology_extensions
                 SET name = ?2, description = ?3, config_json = ?4, is_active = ?5
                 WHERE id = ?1",
                rusqlite::params![
                    methodology.id.as_str(),
                    methodology.name,
                    methodology.description,
                    config_json,
                    methodology.is_active as i32,
                ],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        if updated == 0 {
            return Err(AppError::NotFound(format!(
                "Methodology not found: {}",
                methodology.id.as_str()
            )));
        }

        Ok(())
    }

    async fn delete(&self, id: &MethodologyId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "DELETE FROM methodology_extensions WHERE id = ?1",
            [id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn exists(&self, id: &MethodologyId) -> AppResult<bool> {
        let conn = self.conn.lock().await;

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM methodology_extensions WHERE id = ?1",
                [id.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count > 0)
    }
}

impl SqliteMethodologyRepository {
    /// Seeds built-in methodologies (BMAD and GSD) if they don't exist.
    /// Returns the number of methodologies seeded.
    pub async fn seed_builtin_methodologies(&self) -> AppResult<usize> {
        let builtin_methodologies = MethodologyExtension::builtin_methodologies();
        let mut seeded_count = 0;

        for methodology in builtin_methodologies {
            // Check if methodology already exists
            if !self.exists(&methodology.id).await? {
                self.create(methodology).await?;
                seeded_count += 1;
            }
        }

        Ok(seeded_count)
    }
}

#[cfg(test)]
#[path = "sqlite_methodology_repo_tests.rs"]
mod tests;
