// Memory-based ProcessRepository implementation for testing
// Uses RwLock<HashMap> for thread-safe storage

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;

use crate::domain::entities::research::{ResearchProcess, ResearchProcessId, ResearchProcessStatus};
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

    async fn get_by_status(&self, status: ResearchProcessStatus) -> AppResult<Vec<ResearchProcess>> {
        let processes = self.processes.read().await;
        Ok(processes
            .values()
            .filter(|p| p.status() == status)
            .cloned()
            .collect())
    }

    async fn get_active(&self) -> AppResult<Vec<ResearchProcess>> {
        let processes = self.processes.read().await;
        Ok(processes.values().filter(|p| p.is_active()).cloned().collect())
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::research::{ResearchBrief, ResearchDepthPreset};

    fn create_test_process() -> ResearchProcess {
        let brief = ResearchBrief::new("What architecture should we use?");
        ResearchProcess::new("Test Research", brief, "deep-researcher")
            .with_preset(ResearchDepthPreset::Standard)
    }

    fn create_running_process() -> ResearchProcess {
        let brief = ResearchBrief::new("Running question");
        let mut process = ResearchProcess::new("Running Research", brief, "deep-researcher");
        process.start();
        process
    }

    #[tokio::test]
    async fn test_create_and_get_process() {
        let repo = MemoryProcessRepository::new();
        let process = create_test_process();

        repo.create(process.clone()).await.unwrap();
        let found = repo.get_by_id(&process.id).await.unwrap();

        assert!(found.is_some());
        assert_eq!(found.unwrap().id, process.id);
    }

    #[tokio::test]
    async fn test_get_all_processes() {
        let repo = MemoryProcessRepository::new();
        let process1 = create_test_process();
        let process2 = create_running_process();

        repo.create(process1).await.unwrap();
        repo.create(process2).await.unwrap();

        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_get_by_status() {
        let repo = MemoryProcessRepository::new();
        let pending = create_test_process();
        let running = create_running_process();

        repo.create(pending).await.unwrap();
        repo.create(running).await.unwrap();

        let pending_processes = repo.get_by_status(ResearchProcessStatus::Pending).await.unwrap();
        assert_eq!(pending_processes.len(), 1);

        let running_processes = repo.get_by_status(ResearchProcessStatus::Running).await.unwrap();
        assert_eq!(running_processes.len(), 1);
    }

    #[tokio::test]
    async fn test_get_active() {
        let repo = MemoryProcessRepository::new();
        let pending = create_test_process();
        let running = create_running_process();

        repo.create(pending).await.unwrap();
        repo.create(running).await.unwrap();

        let active = repo.get_active().await.unwrap();
        assert_eq!(active.len(), 2); // Both pending and running are active
    }

    #[tokio::test]
    async fn test_complete_process() {
        let repo = MemoryProcessRepository::new();
        let process = create_running_process();

        repo.create(process.clone()).await.unwrap();
        repo.complete(&process.id).await.unwrap();

        let found = repo.get_by_id(&process.id).await.unwrap().unwrap();
        assert_eq!(found.status(), ResearchProcessStatus::Completed);
    }

    #[tokio::test]
    async fn test_fail_process() {
        let repo = MemoryProcessRepository::new();
        let process = create_running_process();

        repo.create(process.clone()).await.unwrap();
        repo.fail(&process.id, "Test error").await.unwrap();

        let found = repo.get_by_id(&process.id).await.unwrap().unwrap();
        assert_eq!(found.status(), ResearchProcessStatus::Failed);
    }

    #[tokio::test]
    async fn test_delete_process() {
        let repo = MemoryProcessRepository::new();
        let process = create_test_process();

        repo.create(process.clone()).await.unwrap();
        repo.delete(&process.id).await.unwrap();
        let found = repo.get_by_id(&process.id).await.unwrap();

        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_exists() {
        let repo = MemoryProcessRepository::new();
        let process = create_test_process();

        assert!(!repo.exists(&process.id).await.unwrap());
        repo.create(process.clone()).await.unwrap();
        assert!(repo.exists(&process.id).await.unwrap());
    }
}
