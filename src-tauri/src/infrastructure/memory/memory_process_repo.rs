// Memory-based ProcessRepository implementation for testing
// Uses RwLock<HashMap> for thread-safe storage

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;

use crate::domain::entities::research::{
    ResearchProcess, ResearchProcessId, ResearchProcessStatus,
};
use crate::domain::repositories::ProcessRepository;
use crate::error::AppResult;

/// In-memory implementation of ProcessRepository for testing
pub struct MemoryProcessRepository {
    processes: Arc<RwLock<HashMap<ResearchProcessId, ResearchProcess>>>,
}

impl Default for MemoryProcessRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryProcessRepository {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_processes(processes: Vec<ResearchProcess>) -> Self {
        let map: HashMap<ResearchProcessId, ResearchProcess> =
            processes.into_iter().map(|p| (p.id.clone(), p)).collect();
        Self {
            processes: Arc::new(RwLock::new(map)),
        }
    }
}

#[async_trait]
impl ProcessRepository for MemoryProcessRepository {
    async fn create(&self, process: ResearchProcess) -> AppResult<ResearchProcess> {
        let mut processes = self.processes.write().await;
        processes.insert(process.id.clone(), process.clone());
        Ok(process)
    }

    async fn get_by_id(&self, id: &ResearchProcessId) -> AppResult<Option<ResearchProcess>> {
        let processes = self.processes.read().await;
        Ok(processes.get(id).cloned())
    }

    async fn get_all(&self) -> AppResult<Vec<ResearchProcess>> {
        let processes = self.processes.read().await;
        let mut result: Vec<ResearchProcess> = processes.values().cloned().collect();
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(result)
    }

    async fn get_by_status(
        &self,
        status: ResearchProcessStatus,
    ) -> AppResult<Vec<ResearchProcess>> {
        let processes = self.processes.read().await;
        Ok(processes
            .values()
            .filter(|p| p.status() == status)
            .cloned()
            .collect())
    }

    async fn get_active(&self) -> AppResult<Vec<ResearchProcess>> {
        let processes = self.processes.read().await;
        Ok(processes
            .values()
            .filter(|p| p.is_active())
            .cloned()
            .collect())
    }

    async fn update_progress(&self, process: &ResearchProcess) -> AppResult<()> {
        let mut processes = self.processes.write().await;
        processes.insert(process.id.clone(), process.clone());
        Ok(())
    }

    async fn update(&self, process: &ResearchProcess) -> AppResult<()> {
        let mut processes = self.processes.write().await;
        processes.insert(process.id.clone(), process.clone());
        Ok(())
    }

    async fn complete(&self, id: &ResearchProcessId) -> AppResult<()> {
        let mut processes = self.processes.write().await;
        if let Some(process) = processes.get_mut(id) {
            process.complete();
        }
        Ok(())
    }

    async fn fail(&self, id: &ResearchProcessId, error: &str) -> AppResult<()> {
        let mut processes = self.processes.write().await;
        if let Some(process) = processes.get_mut(id) {
            process.fail(error);
        }
        Ok(())
    }

    async fn delete(&self, id: &ResearchProcessId) -> AppResult<()> {
        let mut processes = self.processes.write().await;
        processes.remove(id);
        Ok(())
    }

    async fn exists(&self, id: &ResearchProcessId) -> AppResult<bool> {
        let processes = self.processes.read().await;
        Ok(processes.contains_key(id))
    }

    async fn fail_all_active(&self, reason: &str) -> AppResult<usize> {
        let mut processes = self.processes.write().await;
        let mut count = 0usize;
        for process in processes.values_mut() {
            if process.is_active() {
                process.fail(reason);
                count += 1;
            }
        }
        Ok(count)
    }
}

#[cfg(test)]
#[path = "memory_process_repo_tests.rs"]
mod tests;
