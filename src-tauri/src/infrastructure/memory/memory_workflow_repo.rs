// Memory-based WorkflowRepository implementation for testing
// Uses RwLock<HashMap> for thread-safe storage

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;

use crate::domain::entities::{WorkflowId, WorkflowSchema};
use crate::domain::repositories::WorkflowRepository;
use crate::error::AppResult;

/// In-memory implementation of WorkflowRepository for testing
/// Uses RwLock<HashMap> for thread-safe storage
pub struct MemoryWorkflowRepository {
    workflows: Arc<RwLock<HashMap<WorkflowId, WorkflowSchema>>>,
}

impl Default for MemoryWorkflowRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryWorkflowRepository {
    /// Create a new empty in-memory workflow repository
    pub fn new() -> Self {
        Self {
            workflows: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create with pre-populated workflows (for tests)
    pub fn with_workflows(workflows: Vec<WorkflowSchema>) -> Self {
        let map: HashMap<WorkflowId, WorkflowSchema> = workflows
            .into_iter()
            .map(|w| (w.id.clone(), w))
            .collect();
        Self {
            workflows: Arc::new(RwLock::new(map)),
        }
    }
}

#[async_trait]
impl WorkflowRepository for MemoryWorkflowRepository {
    async fn create(&self, workflow: WorkflowSchema) -> AppResult<WorkflowSchema> {
        let mut workflows = self.workflows.write().await;
        workflows.insert(workflow.id.clone(), workflow.clone());
        Ok(workflow)
    }

    async fn get_by_id(&self, id: &WorkflowId) -> AppResult<Option<WorkflowSchema>> {
        let workflows = self.workflows.read().await;
        Ok(workflows.get(id).cloned())
    }

    async fn get_all(&self) -> AppResult<Vec<WorkflowSchema>> {
        let workflows = self.workflows.read().await;
        let mut result: Vec<WorkflowSchema> = workflows.values().cloned().collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }

    async fn get_default(&self) -> AppResult<Option<WorkflowSchema>> {
        let workflows = self.workflows.read().await;
        Ok(workflows.values().find(|w| w.is_default).cloned())
    }

    async fn update(&self, workflow: &WorkflowSchema) -> AppResult<()> {
        let mut workflows = self.workflows.write().await;
        workflows.insert(workflow.id.clone(), workflow.clone());
        Ok(())
    }

    async fn delete(&self, id: &WorkflowId) -> AppResult<()> {
        let mut workflows = self.workflows.write().await;
        workflows.remove(id);
        Ok(())
    }

    async fn set_default(&self, id: &WorkflowId) -> AppResult<()> {
        let mut workflows = self.workflows.write().await;

        // Unset any existing default
        for workflow in workflows.values_mut() {
            workflow.is_default = false;
        }

        // Set the new default
        if let Some(workflow) = workflows.get_mut(id) {
            workflow.is_default = true;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{InternalStatus, WorkflowColumn};

    fn create_test_workflow(name: &str) -> WorkflowSchema {
        WorkflowSchema::new(
            name,
            vec![
                WorkflowColumn::new("backlog", "Backlog", InternalStatus::Backlog),
                WorkflowColumn::new("ready", "Ready", InternalStatus::Ready),
                WorkflowColumn::new("done", "Done", InternalStatus::Approved),
            ],
        )
    }

    // ==================== CREATE TESTS ====================

    #[tokio::test]
    async fn test_create_workflow_succeeds() {
        let repo = MemoryWorkflowRepository::new();
        let workflow = create_test_workflow("Test Workflow");

        let result = repo.create(workflow.clone()).await;

        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.id, workflow.id);
        assert_eq!(created.name, "Test Workflow");
    }

    #[tokio::test]
    async fn test_create_workflow_can_be_retrieved() {
        let repo = MemoryWorkflowRepository::new();
        let workflow = create_test_workflow("Test Workflow");

        repo.create(workflow.clone()).await.unwrap();

        let found = repo.get_by_id(&workflow.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test Workflow");
    }

    #[tokio::test]
    async fn test_create_multiple_workflows() {
        let repo = MemoryWorkflowRepository::new();

        repo.create(create_test_workflow("Workflow A"))
            .await
            .unwrap();
        repo.create(create_test_workflow("Workflow B"))
            .await
            .unwrap();

        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    // ==================== GET BY ID TESTS ====================

    #[tokio::test]
    async fn test_get_by_id_returns_none_when_not_found() {
        let repo = MemoryWorkflowRepository::new();
        let id = WorkflowId::new();

        let result = repo.get_by_id(&id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_by_id_returns_workflow_when_found() {
        let workflow = create_test_workflow("My Workflow");
        let repo = MemoryWorkflowRepository::with_workflows(vec![workflow.clone()]);

        let result = repo.get_by_id(&workflow.id).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "My Workflow");
    }

    // ==================== GET ALL TESTS ====================

    #[tokio::test]
    async fn test_get_all_returns_empty_when_no_workflows() {
        let repo = MemoryWorkflowRepository::new();

        let result = repo.get_all().await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_get_all_returns_all_workflows() {
        let workflows = vec![
            create_test_workflow("Workflow A"),
            create_test_workflow("Workflow B"),
            create_test_workflow("Workflow C"),
        ];
        let repo = MemoryWorkflowRepository::with_workflows(workflows);

        let result = repo.get_all().await.unwrap();
        assert_eq!(result.len(), 3);
    }

    #[tokio::test]
    async fn test_get_all_returns_sorted_by_name() {
        let repo = MemoryWorkflowRepository::new();

        repo.create(create_test_workflow("Zebra")).await.unwrap();
        repo.create(create_test_workflow("Alpha")).await.unwrap();
        repo.create(create_test_workflow("Beta")).await.unwrap();

        let result = repo.get_all().await.unwrap();
        assert_eq!(result[0].name, "Alpha");
        assert_eq!(result[1].name, "Beta");
        assert_eq!(result[2].name, "Zebra");
    }

    // ==================== GET DEFAULT TESTS ====================

    #[tokio::test]
    async fn test_get_default_returns_none_when_no_default() {
        let repo = MemoryWorkflowRepository::new();
        repo.create(create_test_workflow("Not Default"))
            .await
            .unwrap();

        let result = repo.get_default().await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_default_returns_default_workflow() {
        let repo = MemoryWorkflowRepository::new();
        let default_workflow = WorkflowSchema::default_ralphx();
        repo.create(default_workflow.clone()).await.unwrap();
        repo.create(create_test_workflow("Not Default"))
            .await
            .unwrap();

        let result = repo.get_default().await.unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().is_default);
    }

    // ==================== UPDATE TESTS ====================

    #[tokio::test]
    async fn test_update_workflow_changes_name() {
        let repo = MemoryWorkflowRepository::new();
        let mut workflow = create_test_workflow("Original Name");
        repo.create(workflow.clone()).await.unwrap();

        workflow.name = "Updated Name".to_string();
        repo.update(&workflow).await.unwrap();

        let found = repo.get_by_id(&workflow.id).await.unwrap().unwrap();
        assert_eq!(found.name, "Updated Name");
    }

    #[tokio::test]
    async fn test_update_workflow_changes_description() {
        let repo = MemoryWorkflowRepository::new();
        let mut workflow = create_test_workflow("Workflow");
        repo.create(workflow.clone()).await.unwrap();

        workflow.description = Some("New description".to_string());
        repo.update(&workflow).await.unwrap();

        let found = repo.get_by_id(&workflow.id).await.unwrap().unwrap();
        assert_eq!(found.description, Some("New description".to_string()));
    }

    // ==================== DELETE TESTS ====================

    #[tokio::test]
    async fn test_delete_workflow_removes_it() {
        let repo = MemoryWorkflowRepository::new();
        let workflow = create_test_workflow("To Delete");
        repo.create(workflow.clone()).await.unwrap();

        repo.delete(&workflow.id).await.unwrap();

        let found = repo.get_by_id(&workflow.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_delete_non_existent_workflow_succeeds() {
        let repo = MemoryWorkflowRepository::new();
        let id = WorkflowId::new();

        let result = repo.delete(&id).await;
        assert!(result.is_ok());
    }

    // ==================== SET DEFAULT TESTS ====================

    #[tokio::test]
    async fn test_set_default_marks_workflow_as_default() {
        let repo = MemoryWorkflowRepository::new();
        let workflow = create_test_workflow("Will be Default");
        repo.create(workflow.clone()).await.unwrap();

        repo.set_default(&workflow.id).await.unwrap();

        let found = repo.get_by_id(&workflow.id).await.unwrap().unwrap();
        assert!(found.is_default);
    }

    #[tokio::test]
    async fn test_set_default_unsets_previous_default() {
        let repo = MemoryWorkflowRepository::new();

        // Create and set first workflow as default
        let workflow1 = WorkflowSchema::default_ralphx();
        repo.create(workflow1.clone()).await.unwrap();

        // Create second workflow
        let workflow2 = create_test_workflow("Second Workflow");
        repo.create(workflow2.clone()).await.unwrap();

        // Set second as default
        repo.set_default(&workflow2.id).await.unwrap();

        // Verify first is no longer default
        let found1 = repo.get_by_id(&workflow1.id).await.unwrap().unwrap();
        assert!(!found1.is_default);

        // Verify second is now default
        let found2 = repo.get_by_id(&workflow2.id).await.unwrap().unwrap();
        assert!(found2.is_default);
    }

    #[tokio::test]
    async fn test_set_default_get_default_returns_new_default() {
        let repo = MemoryWorkflowRepository::new();

        let workflow1 = WorkflowSchema::default_ralphx();
        let workflow2 = create_test_workflow("Becomes Default");

        repo.create(workflow1).await.unwrap();
        repo.create(workflow2.clone()).await.unwrap();

        repo.set_default(&workflow2.id).await.unwrap();

        let default = repo.get_default().await.unwrap().unwrap();
        assert_eq!(default.id, workflow2.id);
    }

    // ==================== WITH_WORKFLOWS TESTS ====================

    #[tokio::test]
    async fn test_with_workflows_constructor() {
        let workflows = vec![
            create_test_workflow("One"),
            create_test_workflow("Two"),
        ];
        let repo = MemoryWorkflowRepository::with_workflows(workflows);

        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    // ==================== THREAD SAFETY TESTS ====================

    #[tokio::test]
    async fn test_concurrent_reads() {
        let workflow = create_test_workflow("Concurrent");
        let repo = Arc::new(MemoryWorkflowRepository::with_workflows(vec![workflow.clone()]));

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let repo = Arc::clone(&repo);
                let id = workflow.id.clone();
                tokio::spawn(async move { repo.get_by_id(&id).await })
            })
            .collect();

        for handle in handles {
            let result = handle.await.unwrap().unwrap();
            assert!(result.is_some());
        }
    }

    #[tokio::test]
    async fn test_concurrent_writes() {
        let repo = Arc::new(MemoryWorkflowRepository::new());

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let repo = Arc::clone(&repo);
                tokio::spawn(async move {
                    let workflow = create_test_workflow(&format!("Workflow {}", i));
                    repo.create(workflow).await
                })
            })
            .collect();

        for handle in handles {
            handle.await.unwrap().unwrap();
        }

        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 10);
    }
}
