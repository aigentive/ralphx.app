// SQLite-based MethodologyRepository implementation for production use
// All rusqlite calls go through DbConnection::run() (spawn_blocking + blocking_lock)
// to prevent blocking the tokio async runtime / timer driver.

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use rusqlite::Connection;

use crate::domain::entities::methodology::{MethodologyExtension, MethodologyId};
use crate::domain::repositories::MethodologyRepository;
use crate::error::{AppError, AppResult};
use crate::infrastructure::sqlite::DbConnection;

/// SQLite implementation of MethodologyRepository for production use
pub struct SqliteMethodologyRepository {
    db: DbConnection,
}

impl SqliteMethodologyRepository {
    /// Create a new SQLite methodology repository with the given connection
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
        let config = MethodologyConfig::from(&methodology);
        let config_json = serde_json::to_string(&config)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;
        let id_str = methodology.id.as_str().to_string();
        let name = methodology.name.clone();
        let description = methodology.description.clone();
        let is_active = methodology.is_active as i32;
        let created_at_str = methodology.created_at.to_rfc3339();

        self.db
            .run(move |conn| {
                conn.execute(
                    "INSERT INTO methodology_extensions (id, name, description, config_json, is_active, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![id_str, name, description, config_json, is_active, created_at_str],
                )?;
                Ok(())
            })
            .await?;
        Ok(methodology)
    }

    async fn get_by_id(&self, id: &MethodologyId) -> AppResult<Option<MethodologyExtension>> {
        let id_str = id.as_str().to_string();
        self.db
            .query_optional(move |conn| {
                conn.query_row(
                    "SELECT id, name, description, config_json, is_active, created_at
                     FROM methodology_extensions WHERE id = ?1",
                    rusqlite::params![id_str],
                    SqliteMethodologyRepository::methodology_from_row,
                )
            })
            .await
    }

    async fn get_all(&self) -> AppResult<Vec<MethodologyExtension>> {
        self.db
            .run(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, name, description, config_json, is_active, created_at
                     FROM methodology_extensions ORDER BY name ASC",
                )?;
                let methodologies = stmt
                    .query_map([], SqliteMethodologyRepository::methodology_from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(methodologies)
            })
            .await
    }

    async fn get_active(&self) -> AppResult<Option<MethodologyExtension>> {
        self.db
            .query_optional(move |conn| {
                conn.query_row(
                    "SELECT id, name, description, config_json, is_active, created_at
                     FROM methodology_extensions WHERE is_active = 1",
                    [],
                    SqliteMethodologyRepository::methodology_from_row,
                )
            })
            .await
    }

    async fn activate(&self, id: &MethodologyId) -> AppResult<()> {
        let id_str = id.as_str().to_string();
        self.db
            .run(move |conn| {
                conn.execute("UPDATE methodology_extensions SET is_active = 0", [])?;
                let updated = conn.execute(
                    "UPDATE methodology_extensions SET is_active = 1 WHERE id = ?1",
                    rusqlite::params![id_str],
                )?;
                if updated == 0 {
                    return Err(AppError::NotFound(format!(
                        "Methodology not found: {}",
                        id_str
                    )));
                }
                Ok(())
            })
            .await
    }

    async fn deactivate(&self, id: &MethodologyId) -> AppResult<()> {
        let id_str = id.as_str().to_string();
        self.db
            .run(move |conn| {
                let updated = conn.execute(
                    "UPDATE methodology_extensions SET is_active = 0 WHERE id = ?1",
                    rusqlite::params![id_str],
                )?;
                if updated == 0 {
                    return Err(AppError::NotFound(format!(
                        "Methodology not found: {}",
                        id_str
                    )));
                }
                Ok(())
            })
            .await
    }

    async fn update(&self, methodology: &MethodologyExtension) -> AppResult<()> {
        let config = MethodologyConfig::from(methodology);
        let config_json = serde_json::to_string(&config)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;
        let id_str = methodology.id.as_str().to_string();
        let name = methodology.name.clone();
        let description = methodology.description.clone();
        let is_active = methodology.is_active as i32;

        self.db
            .run(move |conn| {
                let updated = conn.execute(
                    "UPDATE methodology_extensions
                     SET name = ?2, description = ?3, config_json = ?4, is_active = ?5
                     WHERE id = ?1",
                    rusqlite::params![id_str, name, description, config_json, is_active],
                )?;
                if updated == 0 {
                    return Err(AppError::NotFound(format!(
                        "Methodology not found: {}",
                        id_str
                    )));
                }
                Ok(())
            })
            .await
    }

    async fn delete(&self, id: &MethodologyId) -> AppResult<()> {
        let id_str = id.as_str().to_string();
        self.db
            .run(move |conn| {
                conn.execute(
                    "DELETE FROM methodology_extensions WHERE id = ?1",
                    rusqlite::params![id_str],
                )?;
                Ok(())
            })
            .await
    }

    async fn exists(&self, id: &MethodologyId) -> AppResult<bool> {
        let id_str = id.as_str().to_string();
        self.db
            .run(move |conn| {
                let count: i32 = conn.query_row(
                    "SELECT COUNT(*) FROM methodology_extensions WHERE id = ?1",
                    rusqlite::params![id_str],
                    |row| row.get(0),
                )?;
                Ok(count > 0)
            })
            .await
    }
}

impl SqliteMethodologyRepository {
    /// Seeds built-in methodologies (BMAD and GSD) if they don't exist.
    /// Returns the number of methodologies seeded.
    pub async fn seed_builtin_methodologies(&self) -> AppResult<usize> {
        let builtin_methodologies = MethodologyExtension::builtin_methodologies();
        self.db
            .run(move |conn| {
                let mut seeded_count = 0;
                for methodology in builtin_methodologies {
                    let config = MethodologyConfig::from(&methodology);
                    let config_json = serde_json::to_string(&config).map_err(|e| {
                        AppError::Database(format!("JSON serialization error: {}", e))
                    })?;
                    let created_at_str = methodology.created_at.to_rfc3339();
                    let rows = conn.execute(
                        "INSERT OR IGNORE INTO methodology_extensions (id, name, description, config_json, is_active, created_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        rusqlite::params![
                            methodology.id.as_str(),
                            methodology.name,
                            methodology.description,
                            config_json,
                            methodology.is_active as i32,
                            created_at_str,
                        ],
                    )?;
                    if rows > 0 {
                        seeded_count += 1;
                    }
                }
                Ok(seeded_count)
            })
            .await
    }
}

#[cfg(test)]
#[path = "sqlite_methodology_repo_tests.rs"]
mod tests;
