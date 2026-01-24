// Memory-based MethodologyRepository implementation for testing
// Uses RwLock<HashMap> for thread-safe storage

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;

use crate::domain::entities::methodology::{MethodologyExtension, MethodologyId};
use crate::domain::repositories::MethodologyRepository;
use crate::error::AppResult;

/// In-memory implementation of MethodologyRepository for testing
pub struct MemoryMethodologyRepository {
    methodologies: Arc<RwLock<HashMap<MethodologyId, MethodologyExtension>>>,
}

impl Default for MemoryMethodologyRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryMethodologyRepository {
    pub fn new() -> Self {
        Self {
            methodologies: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_methodologies(methodologies: Vec<MethodologyExtension>) -> Self {
        let map: HashMap<MethodologyId, MethodologyExtension> =
            methodologies.into_iter().map(|m| (m.id.clone(), m)).collect();
        Self {
            methodologies: Arc::new(RwLock::new(map)),
        }
    }
}

#[async_trait]
impl MethodologyRepository for MemoryMethodologyRepository {
    async fn create(&self, methodology: MethodologyExtension) -> AppResult<MethodologyExtension> {
        let mut methodologies = self.methodologies.write().await;
        methodologies.insert(methodology.id.clone(), methodology.clone());
        Ok(methodology)
    }

    async fn get_by_id(&self, id: &MethodologyId) -> AppResult<Option<MethodologyExtension>> {
        let methodologies = self.methodologies.read().await;
        Ok(methodologies.get(id).cloned())
    }

    async fn get_all(&self) -> AppResult<Vec<MethodologyExtension>> {
        let methodologies = self.methodologies.read().await;
        let mut result: Vec<MethodologyExtension> = methodologies.values().cloned().collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }

    async fn get_active(&self) -> AppResult<Option<MethodologyExtension>> {
        let methodologies = self.methodologies.read().await;
        Ok(methodologies.values().find(|m| m.is_active).cloned())
    }

    async fn activate(&self, id: &MethodologyId) -> AppResult<()> {
        let mut methodologies = self.methodologies.write().await;

        // Deactivate any currently active methodology
        for methodology in methodologies.values_mut() {
            methodology.is_active = false;
        }

        // Activate the requested one
        if let Some(methodology) = methodologies.get_mut(id) {
            methodology.activate();
        }

        Ok(())
    }

    async fn deactivate(&self, id: &MethodologyId) -> AppResult<()> {
        let mut methodologies = self.methodologies.write().await;
        if let Some(methodology) = methodologies.get_mut(id) {
            methodology.deactivate();
        }
        Ok(())
    }

    async fn update(&self, methodology: &MethodologyExtension) -> AppResult<()> {
        let mut methodologies = self.methodologies.write().await;
        methodologies.insert(methodology.id.clone(), methodology.clone());
        Ok(())
    }

    async fn delete(&self, id: &MethodologyId) -> AppResult<()> {
        let mut methodologies = self.methodologies.write().await;
        methodologies.remove(id);
        Ok(())
    }

    async fn exists(&self, id: &MethodologyId) -> AppResult<bool> {
        let methodologies = self.methodologies.read().await;
        Ok(methodologies.contains_key(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::status::InternalStatus;
    use crate::domain::entities::workflow::{WorkflowColumn, WorkflowSchema};

    fn create_test_workflow() -> WorkflowSchema {
        WorkflowSchema::new(
            "Test Workflow",
            vec![
                WorkflowColumn::new("backlog", "Backlog", InternalStatus::Backlog),
                WorkflowColumn::new("done", "Done", InternalStatus::Approved),
            ],
        )
    }

    fn create_test_methodology() -> MethodologyExtension {
        let workflow = create_test_workflow();
        MethodologyExtension::new("Test Method", workflow)
            .with_description("A test methodology")
    }

    fn create_active_methodology() -> MethodologyExtension {
        let workflow = create_test_workflow();
        let mut methodology = MethodologyExtension::new("Active Method", workflow);
        methodology.activate();
        methodology
    }

    #[tokio::test]
    async fn test_create_and_get_methodology() {
        let repo = MemoryMethodologyRepository::new();
        let methodology = create_test_methodology();

        repo.create(methodology.clone()).await.unwrap();
        let found = repo.get_by_id(&methodology.id).await.unwrap();

        assert!(found.is_some());
        assert_eq!(found.unwrap().id, methodology.id);
    }

    #[tokio::test]
    async fn test_get_all_methodologies() {
        let repo = MemoryMethodologyRepository::new();
        let methodology1 = create_test_methodology();
        let methodology2 = create_active_methodology();

        repo.create(methodology1).await.unwrap();
        repo.create(methodology2).await.unwrap();

        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_get_active_none() {
        let repo = MemoryMethodologyRepository::new();
        let methodology = create_test_methodology();

        repo.create(methodology).await.unwrap();

        let active = repo.get_active().await.unwrap();
        assert!(active.is_none());
    }

    #[tokio::test]
    async fn test_get_active_some() {
        let repo = MemoryMethodologyRepository::new();
        let inactive = create_test_methodology();
        let active = create_active_methodology();

        repo.create(inactive).await.unwrap();
        repo.create(active.clone()).await.unwrap();

        let found = repo.get_active().await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, active.id);
    }

    #[tokio::test]
    async fn test_activate_deactivates_previous() {
        let repo = MemoryMethodologyRepository::new();
        let methodology1 = create_active_methodology();
        let methodology2 = create_test_methodology();

        repo.create(methodology1.clone()).await.unwrap();
        repo.create(methodology2.clone()).await.unwrap();

        // Activate methodology2
        repo.activate(&methodology2.id).await.unwrap();

        // methodology1 should no longer be active
        let found1 = repo.get_by_id(&methodology1.id).await.unwrap().unwrap();
        assert!(!found1.is_active);

        // methodology2 should be active
        let found2 = repo.get_by_id(&methodology2.id).await.unwrap().unwrap();
        assert!(found2.is_active);
    }

    #[tokio::test]
    async fn test_deactivate() {
        let repo = MemoryMethodologyRepository::new();
        let methodology = create_active_methodology();

        repo.create(methodology.clone()).await.unwrap();
        repo.deactivate(&methodology.id).await.unwrap();

        let found = repo.get_by_id(&methodology.id).await.unwrap().unwrap();
        assert!(!found.is_active);
    }

    #[tokio::test]
    async fn test_delete_methodology() {
        let repo = MemoryMethodologyRepository::new();
        let methodology = create_test_methodology();

        repo.create(methodology.clone()).await.unwrap();
        repo.delete(&methodology.id).await.unwrap();
        let found = repo.get_by_id(&methodology.id).await.unwrap();

        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_exists() {
        let repo = MemoryMethodologyRepository::new();
        let methodology = create_test_methodology();

        assert!(!repo.exists(&methodology.id).await.unwrap());
        repo.create(methodology.clone()).await.unwrap();
        assert!(repo.exists(&methodology.id).await.unwrap());
    }
}
