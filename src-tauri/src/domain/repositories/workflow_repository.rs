// Workflow repository trait - domain layer abstraction
//
// This trait defines the contract for workflow persistence.
// Implementations can use SQLite, PostgreSQL, in-memory, etc.

use async_trait::async_trait;

use crate::domain::entities::{WorkflowId, WorkflowSchema};
use crate::error::AppResult;

/// Repository trait for WorkflowSchema persistence.
/// Implementations can use SQLite, in-memory, etc.
#[async_trait]
pub trait WorkflowRepository: Send + Sync {
    /// Create a new workflow
    async fn create(&self, workflow: WorkflowSchema) -> AppResult<WorkflowSchema>;

    /// Get workflow by ID
    async fn get_by_id(&self, id: &WorkflowId) -> AppResult<Option<WorkflowSchema>>;

    /// Get all workflows
    async fn get_all(&self) -> AppResult<Vec<WorkflowSchema>>;

    /// Get the default workflow (where is_default = true)
    async fn get_default(&self) -> AppResult<Option<WorkflowSchema>>;

    /// Update a workflow
    async fn update(&self, workflow: &WorkflowSchema) -> AppResult<()>;

    /// Delete a workflow
    async fn delete(&self, id: &WorkflowId) -> AppResult<()>;

    /// Set a workflow as the default (unsets any previous default)
    async fn set_default(&self, id: &WorkflowId) -> AppResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{InternalStatus, WorkflowColumn};
    use std::sync::Arc;

    // Mock implementation for testing trait object usage
    struct MockWorkflowRepository {
        return_workflow: Option<WorkflowSchema>,
    }

    impl MockWorkflowRepository {
        fn new() -> Self {
            Self {
                return_workflow: None,
            }
        }

        fn with_workflow(workflow: WorkflowSchema) -> Self {
            Self {
                return_workflow: Some(workflow),
            }
        }
    }

    #[async_trait]
    impl WorkflowRepository for MockWorkflowRepository {
        async fn create(&self, workflow: WorkflowSchema) -> AppResult<WorkflowSchema> {
            Ok(workflow)
        }

        async fn get_by_id(&self, _id: &WorkflowId) -> AppResult<Option<WorkflowSchema>> {
            Ok(self.return_workflow.clone())
        }

        async fn get_all(&self) -> AppResult<Vec<WorkflowSchema>> {
            match &self.return_workflow {
                Some(w) => Ok(vec![w.clone()]),
                None => Ok(vec![]),
            }
        }

        async fn get_default(&self) -> AppResult<Option<WorkflowSchema>> {
            if let Some(w) = &self.return_workflow {
                if w.is_default {
                    return Ok(Some(w.clone()));
                }
            }
            Ok(None)
        }

        async fn update(&self, _workflow: &WorkflowSchema) -> AppResult<()> {
            Ok(())
        }

        async fn delete(&self, _id: &WorkflowId) -> AppResult<()> {
            Ok(())
        }

        async fn set_default(&self, _id: &WorkflowId) -> AppResult<()> {
            Ok(())
        }
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

    #[test]
    fn test_workflow_repository_trait_can_be_object_safe() {
        let repo: Arc<dyn WorkflowRepository> = Arc::new(MockWorkflowRepository::new());
        assert!(Arc::strong_count(&repo) == 1);
    }

    #[tokio::test]
    async fn test_mock_workflow_repository_create() {
        let repo = MockWorkflowRepository::new();
        let workflow = create_test_workflow();

        let result = repo.create(workflow.clone()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, workflow.id);
    }

    #[tokio::test]
    async fn test_mock_workflow_repository_get_by_id_returns_none() {
        let repo = MockWorkflowRepository::new();
        let workflow_id = WorkflowId::new();

        let result = repo.get_by_id(&workflow_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_mock_workflow_repository_get_by_id_returns_workflow() {
        let workflow = create_test_workflow();
        let repo = MockWorkflowRepository::with_workflow(workflow.clone());

        let result = repo.get_by_id(&workflow.id).await;
        assert!(result.is_ok());
        let returned = result.unwrap();
        assert!(returned.is_some());
        assert_eq!(returned.unwrap().id, workflow.id);
    }

    #[tokio::test]
    async fn test_mock_workflow_repository_get_all_empty() {
        let repo = MockWorkflowRepository::new();

        let result = repo.get_all().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_workflow_repository_get_all_with_workflow() {
        let workflow = create_test_workflow();
        let repo = MockWorkflowRepository::with_workflow(workflow.clone());

        let result = repo.get_all().await;
        assert!(result.is_ok());
        let workflows = result.unwrap();
        assert_eq!(workflows.len(), 1);
        assert_eq!(workflows[0].id, workflow.id);
    }

    #[tokio::test]
    async fn test_mock_workflow_repository_get_default_none() {
        let repo = MockWorkflowRepository::new();

        let result = repo.get_default().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_mock_workflow_repository_get_default_returns_workflow() {
        let workflow = WorkflowSchema::default_ralphx();
        let repo = MockWorkflowRepository::with_workflow(workflow.clone());

        let result = repo.get_default().await;
        assert!(result.is_ok());
        let returned = result.unwrap();
        assert!(returned.is_some());
        assert!(returned.unwrap().is_default);
    }

    #[tokio::test]
    async fn test_mock_workflow_repository_get_default_skips_non_default() {
        let workflow = create_test_workflow(); // is_default = false
        let repo = MockWorkflowRepository::with_workflow(workflow);

        let result = repo.get_default().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_mock_workflow_repository_update() {
        let repo = MockWorkflowRepository::new();
        let workflow = create_test_workflow();

        let result = repo.update(&workflow).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_workflow_repository_delete() {
        let repo = MockWorkflowRepository::new();
        let workflow_id = WorkflowId::new();

        let result = repo.delete(&workflow_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_workflow_repository_set_default() {
        let repo = MockWorkflowRepository::new();
        let workflow_id = WorkflowId::new();

        let result = repo.set_default(&workflow_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_workflow_repository_trait_object_in_arc() {
        let workflow = create_test_workflow();
        let repo: Arc<dyn WorkflowRepository> =
            Arc::new(MockWorkflowRepository::with_workflow(workflow.clone()));

        // Use through trait object
        let result = repo.get_by_id(&workflow.id).await;
        assert!(result.is_ok());

        let all = repo.get_all().await;
        assert!(all.is_ok());
        assert_eq!(all.unwrap().len(), 1);
    }
}
