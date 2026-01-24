// Memory-based ArtifactFlowRepository implementation for testing
// Uses RwLock<HashMap> for thread-safe storage

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;

use crate::domain::entities::{ArtifactFlow, ArtifactFlowId};
use crate::domain::repositories::ArtifactFlowRepository;
use crate::error::AppResult;

/// In-memory implementation of ArtifactFlowRepository for testing
pub struct MemoryArtifactFlowRepository {
    flows: Arc<RwLock<HashMap<ArtifactFlowId, ArtifactFlow>>>,
}

impl Default for MemoryArtifactFlowRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryArtifactFlowRepository {
    pub fn new() -> Self {
        Self {
            flows: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_flows(flows: Vec<ArtifactFlow>) -> Self {
        let map: HashMap<ArtifactFlowId, ArtifactFlow> =
            flows.into_iter().map(|f| (f.id.clone(), f)).collect();
        Self {
            flows: Arc::new(RwLock::new(map)),
        }
    }
}

#[async_trait]
impl ArtifactFlowRepository for MemoryArtifactFlowRepository {
    async fn create(&self, flow: ArtifactFlow) -> AppResult<ArtifactFlow> {
        let mut flows = self.flows.write().await;
        flows.insert(flow.id.clone(), flow.clone());
        Ok(flow)
    }

    async fn get_by_id(&self, id: &ArtifactFlowId) -> AppResult<Option<ArtifactFlow>> {
        let flows = self.flows.read().await;
        Ok(flows.get(id).cloned())
    }

    async fn get_all(&self) -> AppResult<Vec<ArtifactFlow>> {
        let flows = self.flows.read().await;
        let mut result: Vec<ArtifactFlow> = flows.values().cloned().collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }

    async fn get_active(&self) -> AppResult<Vec<ArtifactFlow>> {
        let flows = self.flows.read().await;
        Ok(flows.values().filter(|f| f.is_active).cloned().collect())
    }

    async fn update(&self, flow: &ArtifactFlow) -> AppResult<()> {
        let mut flows = self.flows.write().await;
        flows.insert(flow.id.clone(), flow.clone());
        Ok(())
    }

    async fn delete(&self, id: &ArtifactFlowId) -> AppResult<()> {
        let mut flows = self.flows.write().await;
        flows.remove(id);
        Ok(())
    }

    async fn set_active(&self, id: &ArtifactFlowId, is_active: bool) -> AppResult<()> {
        let mut flows = self.flows.write().await;
        if let Some(flow) = flows.get_mut(id) {
            flow.is_active = is_active;
        }
        Ok(())
    }

    async fn exists(&self, id: &ArtifactFlowId) -> AppResult<bool> {
        let flows = self.flows.read().await;
        Ok(flows.contains_key(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{ArtifactBucketId, ArtifactFlowStep, ArtifactFlowTrigger};

    fn create_test_flow() -> ArtifactFlow {
        ArtifactFlow::new("Test Flow", ArtifactFlowTrigger::on_artifact_created())
            .with_step(ArtifactFlowStep::copy(ArtifactBucketId::from_string(
                "test-bucket",
            )))
    }

    #[tokio::test]
    async fn test_create_and_get_flow() {
        let repo = MemoryArtifactFlowRepository::new();
        let flow = create_test_flow();

        repo.create(flow.clone()).await.unwrap();
        let found = repo.get_by_id(&flow.id).await.unwrap();

        assert!(found.is_some());
        assert_eq!(found.unwrap().id, flow.id);
    }

    #[tokio::test]
    async fn test_get_all_flows() {
        let repo = MemoryArtifactFlowRepository::new();
        let flow1 = create_test_flow();
        let flow2 = ArtifactFlow::new("Another Flow", ArtifactFlowTrigger::on_task_completed());

        repo.create(flow1).await.unwrap();
        repo.create(flow2).await.unwrap();

        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_get_active_flows() {
        let repo = MemoryArtifactFlowRepository::new();
        let active = create_test_flow();
        let inactive = ArtifactFlow::new("Inactive", ArtifactFlowTrigger::on_artifact_created())
            .set_active(false);

        repo.create(active).await.unwrap();
        repo.create(inactive).await.unwrap();

        let active_flows = repo.get_active().await.unwrap();
        assert_eq!(active_flows.len(), 1);
    }

    #[tokio::test]
    async fn test_set_active() {
        let repo = MemoryArtifactFlowRepository::new();
        let flow = create_test_flow();

        repo.create(flow.clone()).await.unwrap();
        repo.set_active(&flow.id, false).await.unwrap();

        let found = repo.get_by_id(&flow.id).await.unwrap().unwrap();
        assert!(!found.is_active);
    }

    #[tokio::test]
    async fn test_delete_flow() {
        let repo = MemoryArtifactFlowRepository::new();
        let flow = create_test_flow();

        repo.create(flow.clone()).await.unwrap();
        repo.delete(&flow.id).await.unwrap();
        let found = repo.get_by_id(&flow.id).await.unwrap();

        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_exists() {
        let repo = MemoryArtifactFlowRepository::new();
        let flow = create_test_flow();

        assert!(!repo.exists(&flow.id).await.unwrap());
        repo.create(flow.clone()).await.unwrap();
        assert!(repo.exists(&flow.id).await.unwrap());
    }
}
