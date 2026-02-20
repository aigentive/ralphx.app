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
        let map: HashMap<MethodologyId, MethodologyExtension> = methodologies
            .into_iter()
            .map(|m| (m.id.clone(), m))
            .collect();
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
#[path = "memory_methodology_repo_tests.rs"]
mod tests;
