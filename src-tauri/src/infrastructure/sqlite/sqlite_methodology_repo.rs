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
            .query_map([], |row| Self::methodology_from_row(row))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::methodology::{MethodologyPhase, MethodologyTemplate};
    use crate::domain::entities::status::InternalStatus;
    use crate::domain::entities::workflow::{WorkflowColumn, WorkflowSchema};
    use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

    fn setup_test_db() -> Connection {
        let conn = open_memory_connection().expect("Failed to open memory connection");
        run_migrations(&conn).expect("Failed to run migrations");
        conn
    }

    fn create_test_workflow() -> WorkflowSchema {
        WorkflowSchema::new(
            "Test Workflow",
            vec![
                WorkflowColumn::new("backlog", "Backlog", InternalStatus::Backlog),
                WorkflowColumn::new("in_progress", "In Progress", InternalStatus::Executing),
                WorkflowColumn::new("done", "Done", InternalStatus::Approved),
            ],
        )
    }

    fn create_test_methodology() -> MethodologyExtension {
        let workflow = create_test_workflow();
        MethodologyExtension::new("Test Method", workflow)
            .with_description("A test methodology for unit testing")
    }

    fn create_full_methodology() -> MethodologyExtension {
        let workflow = WorkflowSchema::new(
            "BMAD Workflow",
            vec![
                WorkflowColumn::new("brainstorm", "Brainstorm", InternalStatus::Backlog),
                WorkflowColumn::new("research", "Research", InternalStatus::Executing),
                WorkflowColumn::new("prd-draft", "PRD Draft", InternalStatus::Executing),
                WorkflowColumn::new("architecture", "Architecture", InternalStatus::Executing),
                WorkflowColumn::new("sprint", "Sprint", InternalStatus::Executing),
                WorkflowColumn::new("done", "Done", InternalStatus::Approved),
            ],
        );

        MethodologyExtension::new("BMAD Method", workflow)
            .with_description("Breakthrough Method for Agile AI-Driven Development")
            .with_agent_profiles(["bmad-analyst", "bmad-pm", "bmad-architect", "bmad-developer"])
            .with_skills(["skills/prd-creation", "skills/architecture-review"])
            .with_phase(
                MethodologyPhase::new("analysis", "Analysis", 0)
                    .with_description("Analyze requirements")
                    .with_agent_profiles(["bmad-analyst"])
                    .with_columns(["brainstorm", "research"]),
            )
            .with_phase(
                MethodologyPhase::new("planning", "Planning", 1)
                    .with_agent_profiles(["bmad-pm"])
                    .with_column("prd-draft"),
            )
            .with_phase(
                MethodologyPhase::new("solutioning", "Solutioning", 2)
                    .with_agent_profiles(["bmad-architect"])
                    .with_column("architecture"),
            )
            .with_phase(
                MethodologyPhase::new("implementation", "Implementation", 3)
                    .with_agent_profiles(["bmad-developer"])
                    .with_column("sprint"),
            )
            .with_template(
                MethodologyTemplate::new("prd", "templates/prd.md")
                    .with_name("PRD Template")
                    .with_description("Product Requirements Document"),
            )
            .with_template(MethodologyTemplate::new("design_doc", "templates/design.md"))
            .with_hooks_config(serde_json::json!({
                "phase_gates": {
                    "analysis": ["requirements_complete"],
                    "planning": ["prd_approved"]
                }
            }))
    }

    #[tokio::test]
    async fn test_create_methodology() {
        let conn = setup_test_db();
        let repo = SqliteMethodologyRepository::new(conn);
        let methodology = create_test_methodology();

        let result = repo.create(methodology.clone()).await;
        assert!(result.is_ok());

        let created = result.unwrap();
        assert_eq!(created.id, methodology.id);
        assert_eq!(created.name, "Test Method");
    }

    #[tokio::test]
    async fn test_get_by_id_found() {
        let conn = setup_test_db();
        let repo = SqliteMethodologyRepository::new(conn);
        let methodology = create_test_methodology();

        repo.create(methodology.clone()).await.unwrap();

        let result = repo.get_by_id(&methodology.id).await;
        assert!(result.is_ok());

        let found = result.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test Method");
    }

    #[tokio::test]
    async fn test_get_by_id_not_found() {
        let conn = setup_test_db();
        let repo = SqliteMethodologyRepository::new(conn);
        let id = MethodologyId::new();

        let result = repo.get_by_id(&id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_all_empty() {
        let conn = setup_test_db();
        let repo = SqliteMethodologyRepository::new(conn);

        let result = repo.get_all().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_all_with_methodologies() {
        let conn = setup_test_db();
        let repo = SqliteMethodologyRepository::new(conn);

        let methodology1 = create_test_methodology();
        let methodology2 = create_full_methodology();

        repo.create(methodology1).await.unwrap();
        repo.create(methodology2).await.unwrap();

        let result = repo.get_all().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_get_all_ordered_by_name() {
        let conn = setup_test_db();
        let repo = SqliteMethodologyRepository::new(conn);

        let workflow = create_test_workflow();
        let z_method = MethodologyExtension::new("Zebra Method", workflow.clone());
        let a_method = MethodologyExtension::new("Alpha Method", workflow);

        repo.create(z_method).await.unwrap();
        repo.create(a_method).await.unwrap();

        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].name, "Alpha Method");
        assert_eq!(all[1].name, "Zebra Method");
    }

    #[tokio::test]
    async fn test_get_active_none() {
        let conn = setup_test_db();
        let repo = SqliteMethodologyRepository::new(conn);

        let methodology = create_test_methodology(); // not active
        repo.create(methodology).await.unwrap();

        let result = repo.get_active().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_active_some() {
        let conn = setup_test_db();
        let repo = SqliteMethodologyRepository::new(conn);

        let methodology = create_test_methodology();
        repo.create(methodology.clone()).await.unwrap();
        repo.activate(&methodology.id).await.unwrap();

        let result = repo.get_active().await;
        assert!(result.is_ok());
        let active = result.unwrap();
        assert!(active.is_some());
        assert!(active.unwrap().is_active);
    }

    #[tokio::test]
    async fn test_activate() {
        let conn = setup_test_db();
        let repo = SqliteMethodologyRepository::new(conn);

        let methodology = create_test_methodology();
        repo.create(methodology.clone()).await.unwrap();

        assert!(!methodology.is_active);

        repo.activate(&methodology.id).await.unwrap();

        let loaded = repo.get_by_id(&methodology.id).await.unwrap().unwrap();
        assert!(loaded.is_active);
    }

    #[tokio::test]
    async fn test_activate_deactivates_others() {
        let conn = setup_test_db();
        let repo = SqliteMethodologyRepository::new(conn);

        let workflow = create_test_workflow();
        let method1 = MethodologyExtension::new("Method 1", workflow.clone());
        let method2 = MethodologyExtension::new("Method 2", workflow);

        repo.create(method1.clone()).await.unwrap();
        repo.create(method2.clone()).await.unwrap();

        // Activate method1
        repo.activate(&method1.id).await.unwrap();
        let loaded1 = repo.get_by_id(&method1.id).await.unwrap().unwrap();
        assert!(loaded1.is_active);

        // Activate method2 - should deactivate method1
        repo.activate(&method2.id).await.unwrap();
        let loaded1 = repo.get_by_id(&method1.id).await.unwrap().unwrap();
        let loaded2 = repo.get_by_id(&method2.id).await.unwrap().unwrap();
        assert!(!loaded1.is_active);
        assert!(loaded2.is_active);
    }

    #[tokio::test]
    async fn test_activate_not_found() {
        let conn = setup_test_db();
        let repo = SqliteMethodologyRepository::new(conn);
        let id = MethodologyId::new();

        let result = repo.activate(&id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_deactivate() {
        let conn = setup_test_db();
        let repo = SqliteMethodologyRepository::new(conn);

        let methodology = create_test_methodology();
        repo.create(methodology.clone()).await.unwrap();
        repo.activate(&methodology.id).await.unwrap();

        repo.deactivate(&methodology.id).await.unwrap();

        let loaded = repo.get_by_id(&methodology.id).await.unwrap().unwrap();
        assert!(!loaded.is_active);
    }

    #[tokio::test]
    async fn test_deactivate_not_found() {
        let conn = setup_test_db();
        let repo = SqliteMethodologyRepository::new(conn);
        let id = MethodologyId::new();

        let result = repo.deactivate(&id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update() {
        let conn = setup_test_db();
        let repo = SqliteMethodologyRepository::new(conn);

        let mut methodology = create_test_methodology();
        repo.create(methodology.clone()).await.unwrap();

        methodology.name = "Updated Method Name".to_string();
        methodology.description = Some("Updated description".to_string());

        repo.update(&methodology).await.unwrap();

        let loaded = repo.get_by_id(&methodology.id).await.unwrap().unwrap();
        assert_eq!(loaded.name, "Updated Method Name");
        assert_eq!(loaded.description, Some("Updated description".to_string()));
    }

    #[tokio::test]
    async fn test_update_not_found() {
        let conn = setup_test_db();
        let repo = SqliteMethodologyRepository::new(conn);

        let methodology = create_test_methodology();

        let result = repo.update(&methodology).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete() {
        let conn = setup_test_db();
        let repo = SqliteMethodologyRepository::new(conn);

        let methodology = create_test_methodology();
        repo.create(methodology.clone()).await.unwrap();

        repo.delete(&methodology.id).await.unwrap();

        let found = repo.get_by_id(&methodology.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_exists_true() {
        let conn = setup_test_db();
        let repo = SqliteMethodologyRepository::new(conn);

        let methodology = create_test_methodology();
        repo.create(methodology.clone()).await.unwrap();

        let result = repo.exists(&methodology.id).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_exists_false() {
        let conn = setup_test_db();
        let repo = SqliteMethodologyRepository::new(conn);

        let id = MethodologyId::new();

        let result = repo.exists(&id).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_full_methodology_preserved() {
        let conn = setup_test_db();
        let repo = SqliteMethodologyRepository::new(conn);

        let methodology = create_full_methodology();
        repo.create(methodology.clone()).await.unwrap();

        let loaded = repo.get_by_id(&methodology.id).await.unwrap().unwrap();

        assert_eq!(loaded.name, "BMAD Method");
        assert_eq!(
            loaded.description,
            Some("Breakthrough Method for Agile AI-Driven Development".to_string())
        );
        assert_eq!(loaded.agent_profiles.len(), 4);
        assert!(loaded.agent_profiles.contains(&"bmad-analyst".to_string()));
        assert_eq!(loaded.skills.len(), 2);
        assert_eq!(loaded.phases.len(), 4);
        assert_eq!(loaded.templates.len(), 2);
        assert!(loaded.hooks_config.is_some());
    }

    #[tokio::test]
    async fn test_phases_preserved() {
        let conn = setup_test_db();
        let repo = SqliteMethodologyRepository::new(conn);

        let methodology = create_full_methodology();
        repo.create(methodology.clone()).await.unwrap();

        let loaded = repo.get_by_id(&methodology.id).await.unwrap().unwrap();

        let sorted = loaded.sorted_phases();
        assert_eq!(sorted.len(), 4);
        assert_eq!(sorted[0].name, "Analysis");
        assert_eq!(sorted[0].order, 0);
        assert_eq!(sorted[0].description, Some("Analyze requirements".to_string()));
        assert_eq!(sorted[0].agent_profiles, vec!["bmad-analyst"]);
        assert_eq!(sorted[0].column_ids, vec!["brainstorm", "research"]);
    }

    #[tokio::test]
    async fn test_templates_preserved() {
        let conn = setup_test_db();
        let repo = SqliteMethodologyRepository::new(conn);

        let methodology = create_full_methodology();
        repo.create(methodology.clone()).await.unwrap();

        let loaded = repo.get_by_id(&methodology.id).await.unwrap().unwrap();

        assert_eq!(loaded.templates.len(), 2);
        let prd_template = loaded
            .templates
            .iter()
            .find(|t| t.artifact_type == "prd")
            .unwrap();
        assert_eq!(prd_template.template_path, "templates/prd.md");
        assert_eq!(prd_template.name, Some("PRD Template".to_string()));
        assert_eq!(
            prd_template.description,
            Some("Product Requirements Document".to_string())
        );
    }

    #[tokio::test]
    async fn test_hooks_config_preserved() {
        let conn = setup_test_db();
        let repo = SqliteMethodologyRepository::new(conn);

        let methodology = create_full_methodology();
        repo.create(methodology.clone()).await.unwrap();

        let loaded = repo.get_by_id(&methodology.id).await.unwrap().unwrap();

        assert!(loaded.hooks_config.is_some());
        let hooks = loaded.hooks_config.unwrap();
        assert!(hooks.get("phase_gates").is_some());
    }

    #[tokio::test]
    async fn test_workflow_preserved() {
        let conn = setup_test_db();
        let repo = SqliteMethodologyRepository::new(conn);

        let methodology = create_full_methodology();
        repo.create(methodology.clone()).await.unwrap();

        let loaded = repo.get_by_id(&methodology.id).await.unwrap().unwrap();

        assert_eq!(loaded.workflow.name, "BMAD Workflow");
        assert_eq!(loaded.workflow.columns.len(), 6);
        assert_eq!(loaded.workflow.columns[0].id, "brainstorm");
        assert_eq!(loaded.workflow.columns[0].name, "Brainstorm");
    }

    #[tokio::test]
    async fn test_timestamps_preserved() {
        let conn = setup_test_db();
        let repo = SqliteMethodologyRepository::new(conn);

        let methodology = create_test_methodology();
        let original_created_at = methodology.created_at;
        repo.create(methodology.clone()).await.unwrap();

        let loaded = repo.get_by_id(&methodology.id).await.unwrap().unwrap();

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

        let repo1 = SqliteMethodologyRepository::from_shared(shared.clone());
        let repo2 = SqliteMethodologyRepository::from_shared(shared.clone());

        // Create via repo1
        let methodology = create_test_methodology();
        repo1.create(methodology.clone()).await.unwrap();

        // Read via repo2
        let found = repo2.get_by_id(&methodology.id).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_methodology_without_description() {
        let conn = setup_test_db();
        let repo = SqliteMethodologyRepository::new(conn);

        let workflow = create_test_workflow();
        let methodology = MethodologyExtension::new("No Description", workflow);
        repo.create(methodology.clone()).await.unwrap();

        let loaded = repo.get_by_id(&methodology.id).await.unwrap().unwrap();
        assert!(loaded.description.is_none());
    }

    #[tokio::test]
    async fn test_methodology_with_empty_collections() {
        let conn = setup_test_db();
        let repo = SqliteMethodologyRepository::new(conn);

        let workflow = create_test_workflow();
        let methodology = MethodologyExtension::new("Empty Collections", workflow);
        repo.create(methodology.clone()).await.unwrap();

        let loaded = repo.get_by_id(&methodology.id).await.unwrap().unwrap();
        assert!(loaded.agent_profiles.is_empty());
        assert!(loaded.skills.is_empty());
        assert!(loaded.phases.is_empty());
        assert!(loaded.templates.is_empty());
        assert!(loaded.hooks_config.is_none());
    }
}
