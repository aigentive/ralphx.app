// Memory-based TaskQARepository implementation for testing
// Uses RwLock<HashMap> for thread-safe storage without a real database

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;
use chrono::Utc;

use crate::domain::entities::{TaskId, TaskQA, TaskQAId};
use crate::domain::qa::{AcceptanceCriteria, QAResults, QATestSteps};
use crate::domain::repositories::TaskQARepository;
use crate::error::AppResult;

/// In-memory implementation of TaskQARepository for testing
/// Uses RwLock<HashMap> for thread-safe storage
pub struct MemoryTaskQARepository {
    records: Arc<RwLock<HashMap<TaskQAId, TaskQA>>>,
}

impl Default for MemoryTaskQARepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryTaskQARepository {
    /// Create a new empty in-memory TaskQA repository
    pub fn new() -> Self {
        Self {
            records: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create with pre-populated records (for tests)
    pub fn with_records(records: Vec<TaskQA>) -> Self {
        let map: HashMap<TaskQAId, TaskQA> = records
            .into_iter()
            .map(|r| (r.id.clone(), r))
            .collect();
        Self {
            records: Arc::new(RwLock::new(map)),
        }
    }
}

#[async_trait]
impl TaskQARepository for MemoryTaskQARepository {
    async fn create(&self, task_qa: &TaskQA) -> AppResult<()> {
        let mut records = self.records.write().await;
        records.insert(task_qa.id.clone(), task_qa.clone());
        Ok(())
    }

    async fn get_by_id(&self, id: &TaskQAId) -> AppResult<Option<TaskQA>> {
        let records = self.records.read().await;
        Ok(records.get(id).cloned())
    }

    async fn get_by_task_id(&self, task_id: &TaskId) -> AppResult<Option<TaskQA>> {
        let records = self.records.read().await;
        Ok(records.values().find(|r| r.task_id == *task_id).cloned())
    }

    async fn update_prep(
        &self,
        id: &TaskQAId,
        agent_id: &str,
        criteria: &AcceptanceCriteria,
        steps: &QATestSteps,
    ) -> AppResult<()> {
        let mut records = self.records.write().await;
        if let Some(record) = records.get_mut(id) {
            record.prep_agent_id = Some(agent_id.to_string());
            record.acceptance_criteria = Some(criteria.clone());
            record.qa_test_steps = Some(steps.clone());
            record.prep_completed_at = Some(Utc::now());
        }
        Ok(())
    }

    async fn update_refinement(
        &self,
        id: &TaskQAId,
        agent_id: &str,
        actual_implementation: &str,
        refined_steps: &QATestSteps,
    ) -> AppResult<()> {
        let mut records = self.records.write().await;
        if let Some(record) = records.get_mut(id) {
            record.refinement_agent_id = Some(agent_id.to_string());
            record.actual_implementation = Some(actual_implementation.to_string());
            record.refined_test_steps = Some(refined_steps.clone());
            record.refinement_completed_at = Some(Utc::now());
        }
        Ok(())
    }

    async fn update_results(
        &self,
        id: &TaskQAId,
        agent_id: &str,
        results: &QAResults,
        screenshots: &[String],
    ) -> AppResult<()> {
        let mut records = self.records.write().await;
        if let Some(record) = records.get_mut(id) {
            record.test_agent_id = Some(agent_id.to_string());
            record.test_results = Some(results.clone());
            record.screenshots = screenshots.to_vec();
            record.test_completed_at = Some(Utc::now());
        }
        Ok(())
    }

    async fn get_pending_prep(&self) -> AppResult<Vec<TaskQA>> {
        let records = self.records.read().await;
        Ok(records
            .values()
            .filter(|r| r.acceptance_criteria.is_none())
            .cloned()
            .collect())
    }

    async fn delete(&self, id: &TaskQAId) -> AppResult<()> {
        let mut records = self.records.write().await;
        records.remove(id);
        Ok(())
    }

    async fn delete_by_task_id(&self, task_id: &TaskId) -> AppResult<()> {
        let mut records = self.records.write().await;
        records.retain(|_, r| r.task_id != *task_id);
        Ok(())
    }

    async fn exists_for_task(&self, task_id: &TaskId) -> AppResult<bool> {
        let records = self.records.read().await;
        Ok(records.values().any(|r| r.task_id == *task_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::qa::{AcceptanceCriterion, QAStepResult, QATestStep};

    fn create_test_task_qa(task_id: &str) -> TaskQA {
        TaskQA::new(TaskId::from_string(task_id.to_string()))
    }

    #[tokio::test]
    async fn test_create_and_get_by_id() {
        let repo = MemoryTaskQARepository::new();
        let task_qa = create_test_task_qa("task-123");
        let qa_id = task_qa.id.clone();

        repo.create(&task_qa).await.unwrap();

        let retrieved = repo.get_by_id(&qa_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().task_id.as_str(), "task-123");
    }

    #[tokio::test]
    async fn test_get_by_task_id() {
        let repo = MemoryTaskQARepository::new();
        let task_qa = create_test_task_qa("task-123");
        let task_id = task_qa.task_id.clone();

        repo.create(&task_qa).await.unwrap();

        let retrieved = repo.get_by_task_id(&task_id).await.unwrap();
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_get_by_id_returns_none_for_missing() {
        let repo = MemoryTaskQARepository::new();
        let qa_id = TaskQAId::new();

        let retrieved = repo.get_by_id(&qa_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_update_prep() {
        let repo = MemoryTaskQARepository::new();
        let task_qa = create_test_task_qa("task-123");
        let qa_id = task_qa.id.clone();
        repo.create(&task_qa).await.unwrap();

        let criteria = AcceptanceCriteria::from_criteria(vec![
            AcceptanceCriterion::visual("AC1", "Test visual"),
        ]);
        let steps = QATestSteps::from_steps(vec![
            QATestStep::new("QA1", "AC1", "Test step", vec![], "Expected"),
        ]);

        repo.update_prep(&qa_id, "agent-1", &criteria, &steps)
            .await
            .unwrap();

        let retrieved = repo.get_by_id(&qa_id).await.unwrap().unwrap();
        assert!(retrieved.acceptance_criteria.is_some());
        assert!(retrieved.qa_test_steps.is_some());
        assert!(retrieved.prep_completed_at.is_some());
        assert_eq!(retrieved.prep_agent_id, Some("agent-1".to_string()));
    }

    #[tokio::test]
    async fn test_update_refinement() {
        let repo = MemoryTaskQARepository::new();
        let task_qa = create_test_task_qa("task-123");
        let qa_id = task_qa.id.clone();
        repo.create(&task_qa).await.unwrap();

        let refined_steps = QATestSteps::from_steps(vec![
            QATestStep::new("QA1", "AC1", "Refined step", vec![], "Expected"),
        ]);

        repo.update_refinement(&qa_id, "agent-2", "Added button", &refined_steps)
            .await
            .unwrap();

        let retrieved = repo.get_by_id(&qa_id).await.unwrap().unwrap();
        assert!(retrieved.actual_implementation.is_some());
        assert!(retrieved.refined_test_steps.is_some());
        assert!(retrieved.refinement_completed_at.is_some());
    }

    #[tokio::test]
    async fn test_update_results() {
        let repo = MemoryTaskQARepository::new();
        let task_qa = create_test_task_qa("task-123");
        let qa_id = task_qa.id.clone();
        repo.create(&task_qa).await.unwrap();

        let results = QAResults::from_results(
            "task-123",
            vec![QAStepResult::passed("QA1", Some("ss.png".into()))],
        );
        let screenshots = vec!["ss.png".to_string()];

        repo.update_results(&qa_id, "agent-3", &results, &screenshots)
            .await
            .unwrap();

        let retrieved = repo.get_by_id(&qa_id).await.unwrap().unwrap();
        assert!(retrieved.test_results.is_some());
        assert!(!retrieved.screenshots.is_empty());
        assert!(retrieved.test_completed_at.is_some());
    }

    #[tokio::test]
    async fn test_get_pending_prep() {
        let repo = MemoryTaskQARepository::new();

        // Create two task QA records - one will have prep completed
        let task_qa1 = create_test_task_qa("task-1");
        let qa_id1 = task_qa1.id.clone();
        let task_qa2 = create_test_task_qa("task-2");

        repo.create(&task_qa1).await.unwrap();
        repo.create(&task_qa2).await.unwrap();

        // Complete prep for first one
        let criteria = AcceptanceCriteria::from_criteria(vec![
            AcceptanceCriterion::visual("AC1", "Test"),
        ]);
        let steps = QATestSteps::from_steps(vec![
            QATestStep::new("QA1", "AC1", "Step", vec![], "Expected"),
        ]);
        repo.update_prep(&qa_id1, "agent-1", &criteria, &steps)
            .await
            .unwrap();

        // Get pending prep - should only return task-2
        let pending = repo.get_pending_prep().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].task_id.as_str(), "task-2");
    }

    #[tokio::test]
    async fn test_delete() {
        let repo = MemoryTaskQARepository::new();
        let task_qa = create_test_task_qa("task-123");
        let qa_id = task_qa.id.clone();
        repo.create(&task_qa).await.unwrap();

        repo.delete(&qa_id).await.unwrap();

        let retrieved = repo.get_by_id(&qa_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_delete_by_task_id() {
        let repo = MemoryTaskQARepository::new();
        let task_qa = create_test_task_qa("task-123");
        let task_id = task_qa.task_id.clone();
        repo.create(&task_qa).await.unwrap();

        repo.delete_by_task_id(&task_id).await.unwrap();

        let retrieved = repo.get_by_task_id(&task_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_exists_for_task() {
        let repo = MemoryTaskQARepository::new();
        let task_id = TaskId::from_string("task-123".to_string());

        assert!(!repo.exists_for_task(&task_id).await.unwrap());

        let task_qa = TaskQA::new(task_id.clone());
        repo.create(&task_qa).await.unwrap();

        assert!(repo.exists_for_task(&task_id).await.unwrap());
    }

    #[tokio::test]
    async fn test_with_records_prepopulates() {
        let task_qa1 = create_test_task_qa("task-1");
        let task_qa2 = create_test_task_qa("task-2");
        let qa_id1 = task_qa1.id.clone();
        let qa_id2 = task_qa2.id.clone();

        let repo = MemoryTaskQARepository::with_records(vec![task_qa1, task_qa2]);

        assert!(repo.get_by_id(&qa_id1).await.unwrap().is_some());
        assert!(repo.get_by_id(&qa_id2).await.unwrap().is_some());
    }
}
