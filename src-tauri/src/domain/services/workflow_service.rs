// WorkflowService - domain service for workflow management
//
// Provides business logic for:
// - Getting the active/default workflow
// - Applying workflows to generate dynamic Kanban columns
// - Validating column mappings to internal statuses

use std::sync::Arc;

use crate::domain::entities::{InternalStatus, WorkflowColumn, WorkflowId, WorkflowSchema};
use crate::domain::repositories::WorkflowRepository;
use crate::error::{AppError, AppResult};

/// Result of applying a workflow - contains the columns for the Kanban board
#[derive(Debug, Clone)]
pub struct AppliedWorkflow {
    /// The workflow ID
    pub workflow_id: WorkflowId,
    /// The workflow name
    pub workflow_name: String,
    /// The columns to display in the Kanban board
    pub columns: Vec<AppliedColumn>,
}

/// A column ready for Kanban display
#[derive(Debug, Clone)]
pub struct AppliedColumn {
    /// Column ID (unique within workflow)
    pub id: String,
    /// Display name
    pub name: String,
    /// The internal status this column maps to
    pub maps_to: InternalStatus,
    /// Optional color for the column
    pub color: Option<String>,
    /// Optional icon for the column
    pub icon: Option<String>,
    /// Override agent profile for this column
    pub agent_profile: Option<String>,
}

impl From<&WorkflowColumn> for AppliedColumn {
    fn from(col: &WorkflowColumn) -> Self {
        Self {
            id: col.id.clone(),
            name: col.name.clone(),
            maps_to: col.maps_to,
            color: col.color.clone(),
            icon: col.icon.clone(),
            agent_profile: col.behavior.as_ref().and_then(|b| b.agent_profile.clone()),
        }
    }
}

/// Validation error for column mappings
#[derive(Debug, Clone)]
pub struct ColumnMappingError {
    pub column_id: String,
    pub column_name: String,
    pub error: String,
}

/// Result of validating column mappings
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ColumnMappingError>,
}

/// Service for workflow-related business logic
pub struct WorkflowService<R: WorkflowRepository> {
    repository: Arc<R>,
}

impl<R: WorkflowRepository> WorkflowService<R> {
    /// Create a new WorkflowService with the given repository
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    /// Get the active (default) workflow.
    /// Returns the default workflow, or the built-in RalphX default if none is set.
    pub async fn get_active_workflow(&self) -> AppResult<WorkflowSchema> {
        match self.repository.get_default().await? {
            Some(workflow) => Ok(workflow),
            None => {
                // Fallback to built-in default
                Ok(WorkflowSchema::default_ralphx())
            }
        }
    }

    /// Apply a workflow by ID to generate columns for the Kanban board.
    /// If workflow_id is None, uses the active (default) workflow.
    pub async fn apply_workflow(
        &self,
        workflow_id: Option<&WorkflowId>,
    ) -> AppResult<AppliedWorkflow> {
        let workflow = match workflow_id {
            Some(id) => self
                .repository
                .get_by_id(id)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("Workflow not found: {}", id)))?,
            None => self.get_active_workflow().await?,
        };

        let columns = workflow.columns.iter().map(AppliedColumn::from).collect();

        Ok(AppliedWorkflow {
            workflow_id: workflow.id,
            workflow_name: workflow.name,
            columns,
        })
    }

    /// Validate column mappings in a workflow schema.
    /// Checks that:
    /// - All columns have unique IDs
    /// - All columns have non-empty names
    /// - All internal status mappings are valid
    pub fn validate_column_mappings(&self, workflow: &WorkflowSchema) -> ValidationResult {
        let mut errors = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for (idx, col) in workflow.columns.iter().enumerate() {
            // Check for duplicate column IDs
            if !seen_ids.insert(&col.id) {
                errors.push(ColumnMappingError {
                    column_id: col.id.clone(),
                    column_name: col.name.clone(),
                    error: "Duplicate column ID".to_string(),
                });
            }

            // Check for empty column ID
            if col.id.trim().is_empty() {
                errors.push(ColumnMappingError {
                    column_id: format!("column_{}", idx),
                    column_name: col.name.clone(),
                    error: "Column ID cannot be empty".to_string(),
                });
            }

            // Check for empty column name
            if col.name.trim().is_empty() {
                errors.push(ColumnMappingError {
                    column_id: col.id.clone(),
                    column_name: col.name.clone(),
                    error: "Column name cannot be empty".to_string(),
                });
            }
        }

        // Check for at least one column
        if workflow.columns.is_empty() {
            errors.push(ColumnMappingError {
                column_id: "".to_string(),
                column_name: "".to_string(),
                error: "Workflow must have at least one column".to_string(),
            });
        }

        ValidationResult {
            is_valid: errors.is_empty(),
            errors,
        }
    }

    /// Get all available workflows
    pub async fn get_all_workflows(&self) -> AppResult<Vec<WorkflowSchema>> {
        self.repository.get_all().await
    }

    /// Get a specific workflow by ID
    pub async fn get_workflow(&self, id: &WorkflowId) -> AppResult<Option<WorkflowSchema>> {
        self.repository.get_by_id(id).await
    }

    /// Set a workflow as the default
    pub async fn set_default_workflow(&self, id: &WorkflowId) -> AppResult<()> {
        // Verify the workflow exists
        let _ = self
            .repository
            .get_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Workflow not found: {}", id)))?;

        self.repository.set_default(id).await
    }
}

#[cfg(test)]
#[path = "workflow_service_tests.rs"]
mod tests;
