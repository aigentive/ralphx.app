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
        let map: HashMap<WorkflowId, WorkflowSchema> =
            workflows.into_iter().map(|w| (w.id.clone(), w)).collect();
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
#[path = "memory_workflow_repo_tests.rs"]
mod tests;
