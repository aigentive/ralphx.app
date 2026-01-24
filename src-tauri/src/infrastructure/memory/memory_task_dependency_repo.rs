// In-memory TaskDependencyRepository implementation for testing
// Uses RwLock<Vec> for thread-safe in-memory storage

use std::sync::RwLock;

use async_trait::async_trait;

use crate::domain::entities::TaskId;
use crate::domain::repositories::TaskDependencyRepository;
use crate::error::AppResult;

/// In-memory implementation of TaskDependencyRepository for testing
pub struct MemoryTaskDependencyRepository {
    // (task_id, depends_on_task_id)
    dependencies: RwLock<Vec<(String, String)>>,
}

impl MemoryTaskDependencyRepository {
    /// Create a new empty repository
    pub fn new() -> Self {
        Self {
            dependencies: RwLock::new(Vec::new()),
        }
    }
}

impl Default for MemoryTaskDependencyRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TaskDependencyRepository for MemoryTaskDependencyRepository {
    async fn add_dependency(
        &self,
        task_id: &TaskId,
        depends_on_task_id: &TaskId,
    ) -> AppResult<()> {
        self.dependencies.write().unwrap().push((
            task_id.to_string(),
            depends_on_task_id.to_string(),
        ));
        Ok(())
    }

    async fn remove_dependency(
        &self,
        task_id: &TaskId,
        depends_on_task_id: &TaskId,
    ) -> AppResult<()> {
        self.dependencies.write().unwrap().retain(|(t, d)| {
            t != &task_id.to_string() || d != &depends_on_task_id.to_string()
        });
        Ok(())
    }

    async fn get_blockers(&self, task_id: &TaskId) -> AppResult<Vec<TaskId>> {
        Ok(self
            .dependencies
            .read()
            .unwrap()
            .iter()
            .filter(|(t, _)| t == &task_id.to_string())
            .map(|(_, d)| TaskId::from_string(d.clone()))
            .collect())
    }

    async fn get_blocked_by(&self, task_id: &TaskId) -> AppResult<Vec<TaskId>> {
        Ok(self
            .dependencies
            .read()
            .unwrap()
            .iter()
            .filter(|(_, d)| d == &task_id.to_string())
            .map(|(t, _)| TaskId::from_string(t.clone()))
            .collect())
    }

    async fn has_circular_dependency(
        &self,
        _task_id: &TaskId,
        _potential_dep: &TaskId,
    ) -> AppResult<bool> {
        // Simple implementation for testing - always returns false
        Ok(false)
    }

    async fn clear_dependencies(&self, task_id: &TaskId) -> AppResult<()> {
        self.dependencies.write().unwrap().retain(|(t, d)| {
            t != &task_id.to_string() && d != &task_id.to_string()
        });
        Ok(())
    }

    async fn count_blockers(&self, task_id: &TaskId) -> AppResult<u32> {
        Ok(self
            .dependencies
            .read()
            .unwrap()
            .iter()
            .filter(|(t, _)| t == &task_id.to_string())
            .count() as u32)
    }

    async fn count_blocked_by(&self, task_id: &TaskId) -> AppResult<u32> {
        Ok(self
            .dependencies
            .read()
            .unwrap()
            .iter()
            .filter(|(_, d)| d == &task_id.to_string())
            .count() as u32)
    }

    async fn has_dependency(&self, task_id: &TaskId, depends_on_task_id: &TaskId) -> AppResult<bool> {
        Ok(self
            .dependencies
            .read()
            .unwrap()
            .iter()
            .any(|(t, d)| t == &task_id.to_string() && d == &depends_on_task_id.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_and_get_blockers() {
        let repo = MemoryTaskDependencyRepository::new();
        let t1 = TaskId::new();
        let t2 = TaskId::new();

        repo.add_dependency(&t1, &t2).await.unwrap();

        let blockers = repo.get_blockers(&t1).await.unwrap();
        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].to_string(), t2.to_string());
    }

    #[tokio::test]
    async fn test_get_blocked_by() {
        let repo = MemoryTaskDependencyRepository::new();
        let t1 = TaskId::new();
        let t2 = TaskId::new();

        repo.add_dependency(&t1, &t2).await.unwrap();

        let blocked = repo.get_blocked_by(&t2).await.unwrap();
        assert_eq!(blocked.len(), 1);
        assert_eq!(blocked[0].to_string(), t1.to_string());
    }

    #[tokio::test]
    async fn test_remove_dependency() {
        let repo = MemoryTaskDependencyRepository::new();
        let t1 = TaskId::new();
        let t2 = TaskId::new();

        repo.add_dependency(&t1, &t2).await.unwrap();
        repo.remove_dependency(&t1, &t2).await.unwrap();

        let blockers = repo.get_blockers(&t1).await.unwrap();
        assert!(blockers.is_empty());
    }

    #[tokio::test]
    async fn test_clear_dependencies() {
        let repo = MemoryTaskDependencyRepository::new();
        let t1 = TaskId::new();
        let t2 = TaskId::new();
        let t3 = TaskId::new();

        repo.add_dependency(&t1, &t2).await.unwrap();
        repo.add_dependency(&t3, &t1).await.unwrap();

        repo.clear_dependencies(&t1).await.unwrap();

        let blockers = repo.get_blockers(&t1).await.unwrap();
        let blocked = repo.get_blocked_by(&t1).await.unwrap();

        assert!(blockers.is_empty());
        assert!(blocked.is_empty());
    }

    #[tokio::test]
    async fn test_has_dependency() {
        let repo = MemoryTaskDependencyRepository::new();
        let t1 = TaskId::new();
        let t2 = TaskId::new();
        let t3 = TaskId::new();

        repo.add_dependency(&t1, &t2).await.unwrap();

        assert!(repo.has_dependency(&t1, &t2).await.unwrap());
        assert!(!repo.has_dependency(&t1, &t3).await.unwrap());
    }

    #[tokio::test]
    async fn test_count_blockers_and_blocked() {
        let repo = MemoryTaskDependencyRepository::new();
        let t1 = TaskId::new();
        let t2 = TaskId::new();
        let t3 = TaskId::new();

        repo.add_dependency(&t1, &t2).await.unwrap();
        repo.add_dependency(&t1, &t3).await.unwrap();

        assert_eq!(repo.count_blockers(&t1).await.unwrap(), 2);
        assert_eq!(repo.count_blocked_by(&t2).await.unwrap(), 1);
    }
}
