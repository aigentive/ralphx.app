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
#[path = "memory_artifact_flow_repo_tests.rs"]
mod tests;
