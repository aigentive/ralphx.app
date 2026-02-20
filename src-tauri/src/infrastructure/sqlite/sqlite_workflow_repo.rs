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
            .query_map([], Self::workflow_from_row)
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
        conn.execute(
            "UPDATE workflows SET is_default = 0 WHERE is_default = 1",
            [],
        )
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
#[path = "sqlite_workflow_repo_tests.rs"]
mod tests;
