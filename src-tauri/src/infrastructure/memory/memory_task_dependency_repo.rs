// In-memory TaskDependencyRepository implementation for testing
// Uses RwLock<Vec> for thread-safe in-memory storage

use std::collections::HashMap;
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
    async fn add_dependency(&self, task_id: &TaskId, depends_on_task_id: &TaskId) -> AppResult<()> {
        self.dependencies
            .write()
            .unwrap()
            .push((task_id.to_string(), depends_on_task_id.to_string()));
        Ok(())
    }

    async fn remove_dependency(
        &self,
        task_id: &TaskId,
        depends_on_task_id: &TaskId,
    ) -> AppResult<()> {
        self.dependencies
            .write()
            .unwrap()
            .retain(|(t, d)| t != &task_id.to_string() || d != &depends_on_task_id.to_string());
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
        self.dependencies
            .write()
            .unwrap()
            .retain(|(t, d)| t != &task_id.to_string() && d != &task_id.to_string());
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

    async fn has_dependency(
        &self,
        task_id: &TaskId,
        depends_on_task_id: &TaskId,
    ) -> AppResult<bool> {
        Ok(self
            .dependencies
            .read()
            .unwrap()
            .iter()
            .any(|(t, d)| t == &task_id.to_string() && d == &depends_on_task_id.to_string()))
    }

    async fn get_blockers_batch(
        &self,
        task_ids: &[TaskId],
    ) -> AppResult<HashMap<TaskId, Vec<TaskId>>> {
        let deps = self.dependencies.read().unwrap();
        let id_set: std::collections::HashSet<String> =
            task_ids.iter().map(|id| id.to_string()).collect();
        let mut result: HashMap<TaskId, Vec<TaskId>> = HashMap::new();
        for (task_id_str, dep_id_str) in deps.iter() {
            if id_set.contains(task_id_str) {
                result
                    .entry(TaskId::from_string(task_id_str.clone()))
                    .or_default()
                    .push(TaskId::from_string(dep_id_str.clone()));
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
#[path = "memory_task_dependency_repo_tests.rs"]
mod tests;
