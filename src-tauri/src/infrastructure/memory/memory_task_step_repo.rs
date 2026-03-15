// Memory-based TaskStepRepository implementation for testing
// Uses RwLock<HashMap> for thread-safe storage without a real database

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;

use crate::domain::entities::{TaskId, TaskStep, TaskStepId, TaskStepStatus};
use crate::domain::repositories::TaskStepRepository;
use crate::error::AppResult;

/// In-memory implementation of TaskStepRepository for testing
/// Uses RwLock<HashMap> for thread-safe storage
pub struct MemoryTaskStepRepository {
    steps: Arc<RwLock<HashMap<TaskStepId, TaskStep>>>,
}

impl Default for MemoryTaskStepRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryTaskStepRepository {
    /// Create a new empty in-memory task step repository
    pub fn new() -> Self {
        Self {
            steps: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create with pre-populated steps (for tests)
    pub fn with_steps(steps: Vec<TaskStep>) -> Self {
        let map: HashMap<TaskStepId, TaskStep> =
            steps.into_iter().map(|s| (s.id.clone(), s)).collect();
        Self {
            steps: Arc::new(RwLock::new(map)),
        }
    }
}

#[async_trait]
impl TaskStepRepository for MemoryTaskStepRepository {
    async fn create(&self, step: TaskStep) -> AppResult<TaskStep> {
        let mut steps = self.steps.write().await;
        steps.insert(step.id.clone(), step.clone());
        Ok(step)
    }

    async fn get_by_id(&self, id: &TaskStepId) -> AppResult<Option<TaskStep>> {
        let steps = self.steps.read().await;
        Ok(steps.get(id).cloned())
    }

    async fn get_by_task(&self, task_id: &TaskId) -> AppResult<Vec<TaskStep>> {
        let steps = self.steps.read().await;
        let mut result: Vec<TaskStep> = steps
            .values()
            .filter(|s| s.task_id == *task_id)
            .cloned()
            .collect();
        // Sort by sort_order ASC (as per spec)
        result.sort_by(|a, b| a.sort_order.cmp(&b.sort_order));
        Ok(result)
    }

    async fn get_by_task_and_status(
        &self,
        task_id: &TaskId,
        status: TaskStepStatus,
    ) -> AppResult<Vec<TaskStep>> {
        let steps = self.steps.read().await;
        let mut result: Vec<TaskStep> = steps
            .values()
            .filter(|s| s.task_id == *task_id && s.status == status)
            .cloned()
            .collect();
        // Sort by sort_order ASC
        result.sort_by(|a, b| a.sort_order.cmp(&b.sort_order));
        Ok(result)
    }

    async fn update(&self, step: &TaskStep) -> AppResult<()> {
        let mut steps = self.steps.write().await;
        steps.insert(step.id.clone(), step.clone());
        Ok(())
    }

    async fn delete(&self, id: &TaskStepId) -> AppResult<()> {
        let mut steps = self.steps.write().await;
        steps.remove(id);
        Ok(())
    }

    async fn delete_by_task(&self, task_id: &TaskId) -> AppResult<()> {
        let mut steps = self.steps.write().await;
        steps.retain(|_, step| step.task_id != *task_id);
        Ok(())
    }

    async fn count_by_status(&self, task_id: &TaskId) -> AppResult<HashMap<TaskStepStatus, u32>> {
        let steps = self.steps.read().await;
        let mut counts: HashMap<TaskStepStatus, u32> = HashMap::new();

        for step in steps.values() {
            if step.task_id == *task_id {
                *counts.entry(step.status).or_insert(0) += 1;
            }
        }

        Ok(counts)
    }

    async fn bulk_create(&self, steps_to_create: Vec<TaskStep>) -> AppResult<Vec<TaskStep>> {
        let mut steps = self.steps.write().await;
        let mut created = Vec::new();

        for step in steps_to_create {
            steps.insert(step.id.clone(), step.clone());
            created.push(step);
        }

        Ok(created)
    }

    async fn reorder(&self, task_id: &TaskId, step_ids: Vec<TaskStepId>) -> AppResult<()> {
        let mut steps = self.steps.write().await;

        // Update sort_order for each step based on its position in step_ids
        for (index, step_id) in step_ids.iter().enumerate() {
            if let Some(step) = steps.get_mut(step_id) {
                if step.task_id == *task_id {
                    step.sort_order = index as i32;
                    step.touch();
                }
            }
        }

        Ok(())
    }

    async fn reset_all_to_pending(&self, task_id: &TaskId) -> AppResult<u32> {
        let mut steps = self.steps.write().await;
        let now = chrono::Utc::now();
        let mut count = 0u32;

        for step in steps.values_mut() {
            if step.task_id == *task_id && step.status != TaskStepStatus::Pending {
                step.status = TaskStepStatus::Pending;
                step.started_at = None;
                step.completed_at = None;
                step.completion_note = None;
                step.updated_at = now;
                count += 1;
            }
        }

        Ok(count)
    }
}

#[cfg(test)]
#[path = "memory_task_step_repo_tests.rs"]
mod tests;
