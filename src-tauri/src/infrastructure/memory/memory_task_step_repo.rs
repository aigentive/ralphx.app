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

    async fn count_by_status(
        &self,
        task_id: &TaskId,
    ) -> AppResult<HashMap<TaskStepStatus, u32>> {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_stores_step() {
        let repo = MemoryTaskStepRepository::new();
        let task_id = TaskId::new();
        let step = TaskStep::new(task_id.clone(), "Test step".to_string(), 0, "user".to_string());

        let created = repo.create(step.clone()).await.unwrap();
        assert_eq!(created.id, step.id);
        assert_eq!(created.title, "Test step");

        let retrieved = repo.get_by_id(&step.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Test step");
    }

    #[tokio::test]
    async fn get_by_task_filters_and_sorts() {
        let repo = MemoryTaskStepRepository::new();
        let task_id = TaskId::new();
        let other_task_id = TaskId::new();

        // Create steps with sort_order 2, 0, 1 to test sorting
        let step1 = TaskStep::new(task_id.clone(), "Step 2".to_string(), 2, "user".to_string());
        let step2 = TaskStep::new(task_id.clone(), "Step 0".to_string(), 0, "user".to_string());
        let step3 = TaskStep::new(task_id.clone(), "Step 1".to_string(), 1, "user".to_string());
        let step4 = TaskStep::new(other_task_id, "Other step".to_string(), 0, "user".to_string());

        repo.create(step1).await.unwrap();
        repo.create(step2).await.unwrap();
        repo.create(step3).await.unwrap();
        repo.create(step4).await.unwrap();

        let steps = repo.get_by_task(&task_id).await.unwrap();
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0].title, "Step 0");
        assert_eq!(steps[1].title, "Step 1");
        assert_eq!(steps[2].title, "Step 2");
    }

    #[tokio::test]
    async fn get_by_task_and_status_filters_correctly() {
        let repo = MemoryTaskStepRepository::new();
        let task_id = TaskId::new();

        let step1 = TaskStep::new(task_id.clone(), "Pending".to_string(), 0, "user".to_string());
        let mut step2 = TaskStep::new(task_id.clone(), "In Progress".to_string(), 1, "user".to_string());
        step2.status = TaskStepStatus::InProgress;
        let mut step3 = TaskStep::new(task_id.clone(), "Completed".to_string(), 2, "user".to_string());
        step3.status = TaskStepStatus::Completed;

        repo.create(step1).await.unwrap();
        repo.create(step2).await.unwrap();
        repo.create(step3).await.unwrap();

        let pending = repo.get_by_task_and_status(&task_id, TaskStepStatus::Pending).await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].title, "Pending");

        let in_progress = repo.get_by_task_and_status(&task_id, TaskStepStatus::InProgress).await.unwrap();
        assert_eq!(in_progress.len(), 1);
        assert_eq!(in_progress[0].title, "In Progress");
    }

    #[tokio::test]
    async fn update_modifies_step() {
        let repo = MemoryTaskStepRepository::new();
        let task_id = TaskId::new();
        let mut step = TaskStep::new(task_id, "Original".to_string(), 0, "user".to_string());

        repo.create(step.clone()).await.unwrap();

        step.title = "Updated".to_string();
        step.status = TaskStepStatus::Completed;
        repo.update(&step).await.unwrap();

        let retrieved = repo.get_by_id(&step.id).await.unwrap().unwrap();
        assert_eq!(retrieved.title, "Updated");
        assert_eq!(retrieved.status, TaskStepStatus::Completed);
    }

    #[tokio::test]
    async fn delete_removes_step() {
        let repo = MemoryTaskStepRepository::new();
        let task_id = TaskId::new();
        let step = TaskStep::new(task_id, "Test".to_string(), 0, "user".to_string());

        repo.create(step.clone()).await.unwrap();
        assert!(repo.get_by_id(&step.id).await.unwrap().is_some());

        repo.delete(&step.id).await.unwrap();
        assert!(repo.get_by_id(&step.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_by_task_removes_all_steps() {
        let repo = MemoryTaskStepRepository::new();
        let task_id = TaskId::new();
        let other_task_id = TaskId::new();

        let step1 = TaskStep::new(task_id.clone(), "Step 1".to_string(), 0, "user".to_string());
        let step2 = TaskStep::new(task_id.clone(), "Step 2".to_string(), 1, "user".to_string());
        let step3 = TaskStep::new(other_task_id.clone(), "Other".to_string(), 0, "user".to_string());

        repo.create(step1).await.unwrap();
        repo.create(step2).await.unwrap();
        repo.create(step3).await.unwrap();

        repo.delete_by_task(&task_id).await.unwrap();

        let task_steps = repo.get_by_task(&task_id).await.unwrap();
        assert_eq!(task_steps.len(), 0);

        let other_steps = repo.get_by_task(&other_task_id).await.unwrap();
        assert_eq!(other_steps.len(), 1);
    }

    #[tokio::test]
    async fn count_by_status_counts_correctly() {
        let repo = MemoryTaskStepRepository::new();
        let task_id = TaskId::new();

        let step1 = TaskStep::new(task_id.clone(), "Step 1".to_string(), 0, "user".to_string());
        let mut step2 = TaskStep::new(task_id.clone(), "Step 2".to_string(), 1, "user".to_string());
        step2.status = TaskStepStatus::InProgress;
        let mut step3 = TaskStep::new(task_id.clone(), "Step 3".to_string(), 2, "user".to_string());
        step3.status = TaskStepStatus::Completed;
        let mut step4 = TaskStep::new(task_id.clone(), "Step 4".to_string(), 3, "user".to_string());
        step4.status = TaskStepStatus::Completed;

        repo.create(step1).await.unwrap();
        repo.create(step2).await.unwrap();
        repo.create(step3).await.unwrap();
        repo.create(step4).await.unwrap();

        let counts = repo.count_by_status(&task_id).await.unwrap();

        assert_eq!(counts.get(&TaskStepStatus::Pending), Some(&1));
        assert_eq!(counts.get(&TaskStepStatus::InProgress), Some(&1));
        assert_eq!(counts.get(&TaskStepStatus::Completed), Some(&2));
    }

    #[tokio::test]
    async fn bulk_create_creates_all_steps() {
        let repo = MemoryTaskStepRepository::new();
        let task_id = TaskId::new();

        let steps = vec![
            TaskStep::new(task_id.clone(), "Step 1".to_string(), 0, "user".to_string()),
            TaskStep::new(task_id.clone(), "Step 2".to_string(), 1, "user".to_string()),
            TaskStep::new(task_id.clone(), "Step 3".to_string(), 2, "user".to_string()),
        ];

        let created = repo.bulk_create(steps).await.unwrap();
        assert_eq!(created.len(), 3);

        let retrieved = repo.get_by_task(&task_id).await.unwrap();
        assert_eq!(retrieved.len(), 3);
    }

    #[tokio::test]
    async fn reorder_updates_sort_order() {
        let repo = MemoryTaskStepRepository::new();
        let task_id = TaskId::new();

        let step1 = TaskStep::new(task_id.clone(), "Step 1".to_string(), 0, "user".to_string());
        let step2 = TaskStep::new(task_id.clone(), "Step 2".to_string(), 1, "user".to_string());
        let step3 = TaskStep::new(task_id.clone(), "Step 3".to_string(), 2, "user".to_string());

        let id1 = step1.id.clone();
        let id2 = step2.id.clone();
        let id3 = step3.id.clone();

        repo.create(step1).await.unwrap();
        repo.create(step2).await.unwrap();
        repo.create(step3).await.unwrap();

        // Reorder: step3, step1, step2
        repo.reorder(&task_id, vec![id3.clone(), id1.clone(), id2.clone()]).await.unwrap();

        let steps = repo.get_by_task(&task_id).await.unwrap();
        assert_eq!(steps[0].id, id3);
        assert_eq!(steps[0].sort_order, 0);
        assert_eq!(steps[1].id, id1);
        assert_eq!(steps[1].sort_order, 1);
        assert_eq!(steps[2].id, id2);
        assert_eq!(steps[2].sort_order, 2);
    }
}
