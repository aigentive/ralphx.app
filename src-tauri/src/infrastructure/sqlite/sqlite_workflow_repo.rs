// SQLite-based WorkflowRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use rusqlite::Connection;

use crate::domain::entities::{WorkflowId, WorkflowSchema};
use crate::domain::repositories::WorkflowRepository;
use crate::error::{AppError, AppResult};

/// SQLite implementation of WorkflowRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteWorkflowRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteWorkflowRepository {
    /// Create a new SQLite workflow repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    /// Create from an Arc-wrapped mutex connection (for sharing)
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Parse a WorkflowSchema from a database row
    fn workflow_from_row(row: &rusqlite::Row<'_>) -> Result<WorkflowSchema, rusqlite::Error> {
        let id: String = row.get(0)?;
        let schema_json: String = row.get(3)?;
        let is_default: i32 = row.get(4)?;

        // Parse the JSON schema
        let mut schema: WorkflowSchema = serde_json::from_str(&schema_json)
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

        // Override with database values
        schema.id = WorkflowId::from_string(id);
        schema.is_default = is_default != 0;

        Ok(schema)
    }
}

#[async_trait]
impl WorkflowRepository for SqliteWorkflowRepository {
    async fn create(&self, workflow: WorkflowSchema) -> AppResult<WorkflowSchema> {
        let conn = self.conn.lock().await;

        let schema_json = serde_json::to_string(&workflow)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;

        conn.execute(
            "INSERT INTO workflows (id, name, description, schema_json, is_default)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                workflow.id.as_str(),
                workflow.name,
                workflow.description,
                schema_json,
                if workflow.is_default { 1 } else { 0 },
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(workflow)
    }

    async fn get_by_id(&self, id: &WorkflowId) -> AppResult<Option<WorkflowSchema>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, name, description, schema_json, is_default
             FROM workflows WHERE id = ?1",
            [id.as_str()],
            |row| Self::workflow_from_row(row),
        );

        match result {
            Ok(workflow) => Ok(Some(workflow)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_all(&self) -> AppResult<Vec<WorkflowSchema>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, schema_json, is_default
                 FROM workflows ORDER BY name ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let workflows = stmt
            .query_map([], |row| Self::workflow_from_row(row))
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(workflows)
    }

    async fn get_default(&self) -> AppResult<Option<WorkflowSchema>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, name, description, schema_json, is_default
             FROM workflows WHERE is_default = 1",
            [],
            |row| Self::workflow_from_row(row),
        );

        match result {
            Ok(workflow) => Ok(Some(workflow)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn update(&self, workflow: &WorkflowSchema) -> AppResult<()> {
        let conn = self.conn.lock().await;

        let schema_json = serde_json::to_string(workflow)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;

        conn.execute(
            "UPDATE workflows SET name = ?2, description = ?3, schema_json = ?4, is_default = ?5
             WHERE id = ?1",
            rusqlite::params![
                workflow.id.as_str(),
                workflow.name,
                workflow.description,
                schema_json,
                if workflow.is_default { 1 } else { 0 },
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: &WorkflowId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute("DELETE FROM workflows WHERE id = ?1", [id.as_str()])
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn set_default(&self, id: &WorkflowId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        // First, unset any existing default
        conn.execute("UPDATE workflows SET is_default = 0 WHERE is_default = 1", [])
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Then set the new default
        conn.execute(
            "UPDATE workflows SET is_default = 1 WHERE id = ?1",
            [id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}

impl SqliteWorkflowRepository {
    /// Seeds built-in workflows (default RalphX and Jira-compatible) if they don't exist.
    /// Returns the number of workflows seeded.
    pub async fn seed_builtin_workflows(&self) -> AppResult<usize> {
        let builtin_workflows = vec![
            WorkflowSchema::default_ralphx(),
            WorkflowSchema::jira_compatible(),
        ];

        let mut seeded_count = 0;

        for workflow in builtin_workflows {
            // Check if workflow already exists
            if self.get_by_id(&workflow.id).await?.is_none() {
                self.create(workflow).await?;
                seeded_count += 1;
            }
        }

        Ok(seeded_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{InternalStatus, WorkflowColumn};
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
                WorkflowColumn::new("ready", "Ready", InternalStatus::Ready),
                WorkflowColumn::new("done", "Done", InternalStatus::Approved),
            ],
        )
    }

    #[tokio::test]
    async fn test_create_workflow() {
        let conn = setup_test_db();
        let repo = SqliteWorkflowRepository::new(conn);
        let workflow = create_test_workflow();

        let result = repo.create(workflow.clone()).await;
        assert!(result.is_ok());

        let created = result.unwrap();
        assert_eq!(created.id, workflow.id);
        assert_eq!(created.name, "Test Workflow");
    }

    #[tokio::test]
    async fn test_get_by_id_found() {
        let conn = setup_test_db();
        let repo = SqliteWorkflowRepository::new(conn);
        let workflow = create_test_workflow();

        repo.create(workflow.clone()).await.unwrap();

        let result = repo.get_by_id(&workflow.id).await;
        assert!(result.is_ok());

        let found = result.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test Workflow");
    }

    #[tokio::test]
    async fn test_get_by_id_not_found() {
        let conn = setup_test_db();
        let repo = SqliteWorkflowRepository::new(conn);
        let id = WorkflowId::new();

        let result = repo.get_by_id(&id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_all_empty() {
        let conn = setup_test_db();
        let repo = SqliteWorkflowRepository::new(conn);

        let result = repo.get_all().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_all_with_workflows() {
        let conn = setup_test_db();
        let repo = SqliteWorkflowRepository::new(conn);

        let workflow1 = create_test_workflow();
        let mut workflow2 = create_test_workflow();
        workflow2.id = WorkflowId::new();
        workflow2.name = "Another Workflow".to_string();

        repo.create(workflow1).await.unwrap();
        repo.create(workflow2).await.unwrap();

        let result = repo.get_all().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_get_all_returns_sorted_by_name() {
        let conn = setup_test_db();
        let repo = SqliteWorkflowRepository::new(conn);

        let mut workflow1 = create_test_workflow();
        workflow1.name = "Zebra Workflow".to_string();

        let mut workflow2 = create_test_workflow();
        workflow2.id = WorkflowId::new();
        workflow2.name = "Alpha Workflow".to_string();

        repo.create(workflow1).await.unwrap();
        repo.create(workflow2).await.unwrap();

        let result = repo.get_all().await.unwrap();
        assert_eq!(result[0].name, "Alpha Workflow");
        assert_eq!(result[1].name, "Zebra Workflow");
    }

    #[tokio::test]
    async fn test_get_default_none() {
        let conn = setup_test_db();
        let repo = SqliteWorkflowRepository::new(conn);

        // Create a non-default workflow
        let workflow = create_test_workflow();
        repo.create(workflow).await.unwrap();

        let result = repo.get_default().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_default_returns_default() {
        let conn = setup_test_db();
        let repo = SqliteWorkflowRepository::new(conn);

        let workflow = WorkflowSchema::default_ralphx();
        repo.create(workflow).await.unwrap();

        let result = repo.get_default().await;
        assert!(result.is_ok());

        let default = result.unwrap();
        assert!(default.is_some());
        assert!(default.unwrap().is_default);
    }

    #[tokio::test]
    async fn test_update_workflow() {
        let conn = setup_test_db();
        let repo = SqliteWorkflowRepository::new(conn);

        let mut workflow = create_test_workflow();
        repo.create(workflow.clone()).await.unwrap();

        workflow.name = "Updated Name".to_string();
        workflow.description = Some("New description".to_string());

        let result = repo.update(&workflow).await;
        assert!(result.is_ok());

        let updated = repo.get_by_id(&workflow.id).await.unwrap().unwrap();
        assert_eq!(updated.name, "Updated Name");
        assert_eq!(updated.description, Some("New description".to_string()));
    }

    #[tokio::test]
    async fn test_delete_workflow() {
        let conn = setup_test_db();
        let repo = SqliteWorkflowRepository::new(conn);

        let workflow = create_test_workflow();
        repo.create(workflow.clone()).await.unwrap();

        let result = repo.delete(&workflow.id).await;
        assert!(result.is_ok());

        let found = repo.get_by_id(&workflow.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_set_default_unsets_previous() {
        let conn = setup_test_db();
        let repo = SqliteWorkflowRepository::new(conn);

        // Create first workflow as default
        let workflow1 = WorkflowSchema::default_ralphx();
        repo.create(workflow1.clone()).await.unwrap();

        // Create second non-default workflow
        let workflow2 = create_test_workflow();
        repo.create(workflow2.clone()).await.unwrap();

        // Set second as default
        repo.set_default(&workflow2.id).await.unwrap();

        // Verify first is no longer default
        let updated1 = repo.get_by_id(&workflow1.id).await.unwrap().unwrap();
        assert!(!updated1.is_default);

        // Verify second is now default
        let updated2 = repo.get_by_id(&workflow2.id).await.unwrap().unwrap();
        assert!(updated2.is_default);

        // Verify get_default returns the second
        let default = repo.get_default().await.unwrap().unwrap();
        assert_eq!(default.id, workflow2.id);
    }

    #[tokio::test]
    async fn test_workflow_columns_preserved() {
        let conn = setup_test_db();
        let repo = SqliteWorkflowRepository::new(conn);

        let workflow = WorkflowSchema::default_ralphx();
        repo.create(workflow.clone()).await.unwrap();

        let loaded = repo.get_by_id(&workflow.id).await.unwrap().unwrap();
        assert_eq!(loaded.columns.len(), 7);

        // Verify column mappings
        let draft = loaded.columns.iter().find(|c| c.id == "draft");
        assert!(draft.is_some());
        assert_eq!(draft.unwrap().maps_to, InternalStatus::Backlog);

        let done = loaded.columns.iter().find(|c| c.id == "done");
        assert!(done.is_some());
        assert_eq!(done.unwrap().maps_to, InternalStatus::Approved);
    }

    #[tokio::test]
    async fn test_workflow_with_behavior_preserved() {
        let conn = setup_test_db();
        let repo = SqliteWorkflowRepository::new(conn);

        use crate::domain::entities::ColumnBehavior;

        let mut workflow = create_test_workflow();
        workflow.columns[0] = workflow.columns[0].clone().with_behavior(
            ColumnBehavior::new()
                .with_skip_review(true)
                .with_agent_profile("fast-worker"),
        );

        repo.create(workflow.clone()).await.unwrap();

        let loaded = repo.get_by_id(&workflow.id).await.unwrap().unwrap();
        let behavior = loaded.columns[0].behavior.as_ref().unwrap();
        assert_eq!(behavior.skip_review, Some(true));
        assert_eq!(behavior.agent_profile, Some("fast-worker".to_string()));
    }

    #[tokio::test]
    async fn test_from_shared_connection() {
        let conn = setup_test_db();
        let shared = Arc::new(Mutex::new(conn));

        let repo1 = SqliteWorkflowRepository::from_shared(shared.clone());
        let repo2 = SqliteWorkflowRepository::from_shared(shared.clone());

        // Create via repo1
        let workflow = create_test_workflow();
        repo1.create(workflow.clone()).await.unwrap();

        // Read via repo2
        let found = repo2.get_by_id(&workflow.id).await.unwrap();
        assert!(found.is_some());
    }

    // ==================== SEEDING TESTS ====================

    #[tokio::test]
    async fn test_seed_builtin_workflows_creates_both() {
        let conn = setup_test_db();
        let repo = SqliteWorkflowRepository::new(conn);

        let count = repo.seed_builtin_workflows().await.unwrap();
        assert_eq!(count, 2);

        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_seed_builtin_workflows_creates_default() {
        let conn = setup_test_db();
        let repo = SqliteWorkflowRepository::new(conn);

        repo.seed_builtin_workflows().await.unwrap();

        let default = repo.get_default().await.unwrap();
        assert!(default.is_some());
        assert_eq!(default.unwrap().name, "RalphX Default");
    }

    #[tokio::test]
    async fn test_seed_builtin_workflows_creates_jira() {
        let conn = setup_test_db();
        let repo = SqliteWorkflowRepository::new(conn);

        repo.seed_builtin_workflows().await.unwrap();

        let jira_id = crate::domain::entities::WorkflowId::from_string("jira-compat");
        let jira = repo.get_by_id(&jira_id).await.unwrap();
        assert!(jira.is_some());
        assert_eq!(jira.unwrap().name, "Jira Compatible");
    }

    #[tokio::test]
    async fn test_seed_builtin_workflows_is_idempotent() {
        let conn = setup_test_db();
        let repo = SqliteWorkflowRepository::new(conn);

        // Seed twice
        let count1 = repo.seed_builtin_workflows().await.unwrap();
        let count2 = repo.seed_builtin_workflows().await.unwrap();

        // First seed creates 2, second creates 0
        assert_eq!(count1, 2);
        assert_eq!(count2, 0);

        // Still only 2 workflows
        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_seed_builtin_workflows_preserves_existing() {
        let conn = setup_test_db();
        let repo = SqliteWorkflowRepository::new(conn);

        // Create a custom workflow
        let custom = create_test_workflow();
        repo.create(custom).await.unwrap();

        // Seed built-ins
        repo.seed_builtin_workflows().await.unwrap();

        // Should have 3 workflows total
        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 3);
    }

    #[tokio::test]
    async fn test_seed_builtin_workflows_skips_existing_builtin() {
        let conn = setup_test_db();
        let repo = SqliteWorkflowRepository::new(conn);

        // Manually create the default workflow
        let default = WorkflowSchema::default_ralphx();
        repo.create(default).await.unwrap();

        // Seed should only create Jira (skip default since it exists)
        let count = repo.seed_builtin_workflows().await.unwrap();
        assert_eq!(count, 1);

        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 2);
    }
}
