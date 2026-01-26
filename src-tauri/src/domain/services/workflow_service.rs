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

        let columns = workflow
            .columns
            .iter()
            .map(AppliedColumn::from)
            .collect();

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
mod tests {
    use super::*;
    use crate::domain::entities::{ColumnBehavior, WorkflowColumn};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use tokio::sync::Mutex;

    // ==================== Mock Repository ====================

    struct MockWorkflowRepository {
        workflows: Mutex<HashMap<String, WorkflowSchema>>,
        default_id: Mutex<Option<String>>,
    }

    impl MockWorkflowRepository {
        fn new() -> Self {
            Self {
                workflows: Mutex::new(HashMap::new()),
                default_id: Mutex::new(None),
            }
        }

        async fn add_workflow(&self, workflow: WorkflowSchema) {
            let mut workflows = self.workflows.lock().await;
            if workflow.is_default {
                let mut default_id = self.default_id.lock().await;
                *default_id = Some(workflow.id.as_str().to_string());
            }
            workflows.insert(workflow.id.as_str().to_string(), workflow);
        }
    }

    #[async_trait]
    impl WorkflowRepository for MockWorkflowRepository {
        async fn create(&self, workflow: WorkflowSchema) -> AppResult<WorkflowSchema> {
            self.add_workflow(workflow.clone()).await;
            Ok(workflow)
        }

        async fn get_by_id(&self, id: &WorkflowId) -> AppResult<Option<WorkflowSchema>> {
            let workflows = self.workflows.lock().await;
            Ok(workflows.get(id.as_str()).cloned())
        }

        async fn get_all(&self) -> AppResult<Vec<WorkflowSchema>> {
            let workflows = self.workflows.lock().await;
            Ok(workflows.values().cloned().collect())
        }

        async fn get_default(&self) -> AppResult<Option<WorkflowSchema>> {
            let default_id = self.default_id.lock().await;
            if let Some(id) = default_id.as_ref() {
                let workflows = self.workflows.lock().await;
                return Ok(workflows.get(id).cloned());
            }
            Ok(None)
        }

        async fn update(&self, workflow: &WorkflowSchema) -> AppResult<()> {
            let mut workflows = self.workflows.lock().await;
            workflows.insert(workflow.id.as_str().to_string(), workflow.clone());
            Ok(())
        }

        async fn delete(&self, id: &WorkflowId) -> AppResult<()> {
            let mut workflows = self.workflows.lock().await;
            workflows.remove(id.as_str());
            Ok(())
        }

        async fn set_default(&self, id: &WorkflowId) -> AppResult<()> {
            let mut default_id = self.default_id.lock().await;
            let mut workflows = self.workflows.lock().await;

            // Unset old default
            if let Some(old_id) = default_id.as_ref() {
                if let Some(old_workflow) = workflows.get_mut(old_id) {
                    old_workflow.is_default = false;
                }
            }

            // Set new default
            if let Some(workflow) = workflows.get_mut(id.as_str()) {
                workflow.is_default = true;
                *default_id = Some(id.as_str().to_string());
            }

            Ok(())
        }
    }

    // ==================== Test Helpers ====================

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

    fn create_service_with_mock() -> (WorkflowService<MockWorkflowRepository>, Arc<MockWorkflowRepository>) {
        let repo = Arc::new(MockWorkflowRepository::new());
        let service = WorkflowService::new(repo.clone());
        (service, repo)
    }

    // ==================== get_active_workflow Tests ====================

    #[tokio::test]
    async fn get_active_workflow_returns_default() {
        let (service, repo) = create_service_with_mock();

        let workflow = WorkflowSchema::default_ralphx();
        repo.add_workflow(workflow.clone()).await;

        let result = service.get_active_workflow().await;
        assert!(result.is_ok());

        let active = result.unwrap();
        assert_eq!(active.name, "RalphX Default");
        assert!(active.is_default);
    }

    #[tokio::test]
    async fn get_active_workflow_fallback_when_no_default() {
        let (service, _repo) = create_service_with_mock();

        let result = service.get_active_workflow().await;
        assert!(result.is_ok());

        let active = result.unwrap();
        assert_eq!(active.name, "RalphX Default");
    }

    #[tokio::test]
    async fn get_active_workflow_returns_custom_default() {
        let (service, repo) = create_service_with_mock();

        let mut custom = create_test_workflow();
        custom.is_default = true;
        repo.add_workflow(custom.clone()).await;

        let result = service.get_active_workflow().await;
        assert!(result.is_ok());

        let active = result.unwrap();
        assert_eq!(active.name, "Test Workflow");
    }

    // ==================== apply_workflow Tests ====================

    #[tokio::test]
    async fn apply_workflow_with_id() {
        let (service, repo) = create_service_with_mock();

        let workflow = create_test_workflow();
        let id = workflow.id.clone();
        repo.add_workflow(workflow).await;

        let result = service.apply_workflow(Some(&id)).await;
        assert!(result.is_ok());

        let applied = result.unwrap();
        assert_eq!(applied.workflow_name, "Test Workflow");
        assert_eq!(applied.columns.len(), 3);
    }

    #[tokio::test]
    async fn apply_workflow_without_id_uses_default() {
        let (service, repo) = create_service_with_mock();

        let workflow = WorkflowSchema::default_ralphx();
        repo.add_workflow(workflow).await;

        let result = service.apply_workflow(None).await;
        assert!(result.is_ok());

        let applied = result.unwrap();
        assert_eq!(applied.workflow_name, "RalphX Default");
    }

    #[tokio::test]
    async fn apply_workflow_not_found_error() {
        let (service, _repo) = create_service_with_mock();

        let id = WorkflowId::from_string("nonexistent");
        let result = service.apply_workflow(Some(&id)).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn apply_workflow_maps_columns_correctly() {
        let (service, repo) = create_service_with_mock();

        let workflow = WorkflowSchema::default_ralphx();
        let id = workflow.id.clone();
        repo.add_workflow(workflow).await;

        let applied = service.apply_workflow(Some(&id)).await.unwrap();

        // Check column mappings
        let draft = applied.columns.iter().find(|c| c.id == "draft");
        assert!(draft.is_some());
        assert_eq!(draft.unwrap().maps_to, InternalStatus::Backlog);

        let done = applied.columns.iter().find(|c| c.id == "done");
        assert!(done.is_some());
        assert_eq!(done.unwrap().maps_to, InternalStatus::Approved);
    }

    #[tokio::test]
    async fn apply_workflow_includes_colors_and_icons() {
        let (service, repo) = create_service_with_mock();

        let mut workflow = create_test_workflow();
        workflow.columns[0] = workflow.columns[0]
            .clone()
            .with_color("#ff6b35")
            .with_icon("inbox");
        let id = workflow.id.clone();
        repo.add_workflow(workflow).await;

        let applied = service.apply_workflow(Some(&id)).await.unwrap();

        let col = &applied.columns[0];
        assert_eq!(col.color, Some("#ff6b35".to_string()));
        assert_eq!(col.icon, Some("inbox".to_string()));
    }

    #[tokio::test]
    async fn apply_workflow_includes_agent_profile() {
        let (service, repo) = create_service_with_mock();

        let mut workflow = create_test_workflow();
        workflow.columns[0] = workflow.columns[0]
            .clone()
            .with_behavior(ColumnBehavior::new().with_agent_profile("fast-worker"));
        let id = workflow.id.clone();
        repo.add_workflow(workflow).await;

        let applied = service.apply_workflow(Some(&id)).await.unwrap();

        let col = &applied.columns[0];
        assert_eq!(col.agent_profile, Some("fast-worker".to_string()));
    }

    // ==================== validate_column_mappings Tests ====================

    #[tokio::test]
    async fn validate_column_mappings_valid_workflow() {
        let (service, _repo) = create_service_with_mock();

        let workflow = create_test_workflow();
        let result = service.validate_column_mappings(&workflow);

        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn validate_column_mappings_empty_columns() {
        let (service, _repo) = create_service_with_mock();

        let workflow = WorkflowSchema::new("Empty", vec![]);
        let result = service.validate_column_mappings(&workflow);

        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].error.contains("at least one column"));
    }

    #[tokio::test]
    async fn validate_column_mappings_duplicate_ids() {
        let (service, _repo) = create_service_with_mock();

        let workflow = WorkflowSchema::new(
            "Duplicate IDs",
            vec![
                WorkflowColumn::new("same", "First", InternalStatus::Backlog),
                WorkflowColumn::new("same", "Second", InternalStatus::Ready),
            ],
        );
        let result = service.validate_column_mappings(&workflow);

        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.error.contains("Duplicate")));
    }

    #[tokio::test]
    async fn validate_column_mappings_empty_id() {
        let (service, _repo) = create_service_with_mock();

        let workflow = WorkflowSchema::new(
            "Empty ID",
            vec![WorkflowColumn::new("", "Column", InternalStatus::Backlog)],
        );
        let result = service.validate_column_mappings(&workflow);

        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.error.contains("ID cannot be empty")));
    }

    #[tokio::test]
    async fn validate_column_mappings_empty_name() {
        let (service, _repo) = create_service_with_mock();

        let workflow = WorkflowSchema::new(
            "Empty Name",
            vec![WorkflowColumn::new("col", "", InternalStatus::Backlog)],
        );
        let result = service.validate_column_mappings(&workflow);

        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.error.contains("name cannot be empty")));
    }

    #[tokio::test]
    async fn validate_column_mappings_whitespace_only() {
        let (service, _repo) = create_service_with_mock();

        let workflow = WorkflowSchema::new(
            "Whitespace",
            vec![WorkflowColumn::new("  ", "  ", InternalStatus::Backlog)],
        );
        let result = service.validate_column_mappings(&workflow);

        assert!(!result.is_valid);
        assert!(result.errors.len() >= 2); // Both ID and name errors
    }

    #[tokio::test]
    async fn validate_column_mappings_default_ralphx() {
        let (service, _repo) = create_service_with_mock();

        let workflow = WorkflowSchema::default_ralphx();
        let result = service.validate_column_mappings(&workflow);

        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn validate_column_mappings_jira_compatible() {
        let (service, _repo) = create_service_with_mock();

        let workflow = WorkflowSchema::jira_compatible();
        let result = service.validate_column_mappings(&workflow);

        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    // ==================== get_all_workflows Tests ====================

    #[tokio::test]
    async fn get_all_workflows_empty() {
        let (service, _repo) = create_service_with_mock();

        let result = service.get_all_workflows().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_all_workflows_returns_all() {
        let (service, repo) = create_service_with_mock();

        repo.add_workflow(create_test_workflow()).await;
        repo.add_workflow(WorkflowSchema::default_ralphx()).await;

        let result = service.get_all_workflows().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    // ==================== get_workflow Tests ====================

    #[tokio::test]
    async fn get_workflow_found() {
        let (service, repo) = create_service_with_mock();

        let workflow = create_test_workflow();
        let id = workflow.id.clone();
        repo.add_workflow(workflow).await;

        let result = service.get_workflow(&id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[tokio::test]
    async fn get_workflow_not_found() {
        let (service, _repo) = create_service_with_mock();

        let id = WorkflowId::new();
        let result = service.get_workflow(&id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    // ==================== set_default_workflow Tests ====================

    #[tokio::test]
    async fn set_default_workflow_success() {
        let (service, repo) = create_service_with_mock();

        let workflow = create_test_workflow();
        let id = workflow.id.clone();
        repo.add_workflow(workflow).await;

        let result = service.set_default_workflow(&id).await;
        assert!(result.is_ok());

        // Verify it's now the default
        let default = service.get_active_workflow().await.unwrap();
        assert_eq!(default.id, id);
    }

    #[tokio::test]
    async fn set_default_workflow_not_found() {
        let (service, _repo) = create_service_with_mock();

        let id = WorkflowId::new();
        let result = service.set_default_workflow(&id).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    // ==================== AppliedColumn From Tests ====================

    #[test]
    fn applied_column_from_workflow_column() {
        let col = WorkflowColumn::new("test", "Test Column", InternalStatus::Executing)
            .with_color("#aabbcc")
            .with_icon("play")
            .with_behavior(ColumnBehavior::new().with_agent_profile("worker"));

        let applied = AppliedColumn::from(&col);

        assert_eq!(applied.id, "test");
        assert_eq!(applied.name, "Test Column");
        assert_eq!(applied.maps_to, InternalStatus::Executing);
        assert_eq!(applied.color, Some("#aabbcc".to_string()));
        assert_eq!(applied.icon, Some("play".to_string()));
        assert_eq!(applied.agent_profile, Some("worker".to_string()));
    }

    #[test]
    fn applied_column_from_minimal_column() {
        let col = WorkflowColumn::new("min", "Minimal", InternalStatus::Backlog);

        let applied = AppliedColumn::from(&col);

        assert_eq!(applied.id, "min");
        assert_eq!(applied.name, "Minimal");
        assert_eq!(applied.maps_to, InternalStatus::Backlog);
        assert!(applied.color.is_none());
        assert!(applied.icon.is_none());
        assert!(applied.agent_profile.is_none());
    }
}
