// TaskQA repository trait - domain layer abstraction
//
// This trait defines the contract for TaskQA persistence.
// TaskQA records store QA artifacts for tasks.

use async_trait::async_trait;

use crate::domain::entities::{TaskId, TaskQA, TaskQAId};
use crate::domain::qa::{AcceptanceCriteria, QAResults, QATestSteps};
use crate::error::AppResult;

/// Repository trait for TaskQA persistence.
/// Implementations can use SQLite, in-memory, etc.
#[async_trait]
pub trait TaskQARepository: Send + Sync {
    /// Create a new TaskQA record
    async fn create(&self, task_qa: &TaskQA) -> AppResult<()>;

    /// Get TaskQA by its ID
    async fn get_by_id(&self, id: &TaskQAId) -> AppResult<Option<TaskQA>>;

    /// Get TaskQA by task ID
    async fn get_by_task_id(&self, task_id: &TaskId) -> AppResult<Option<TaskQA>>;

    /// Update QA prep results
    async fn update_prep(
        &self,
        id: &TaskQAId,
        agent_id: &str,
        criteria: &AcceptanceCriteria,
        steps: &QATestSteps,
    ) -> AppResult<()>;

    /// Update QA refinement results
    async fn update_refinement(
        &self,
        id: &TaskQAId,
        agent_id: &str,
        actual_implementation: &str,
        refined_steps: &QATestSteps,
    ) -> AppResult<()>;

    /// Update QA test results
    async fn update_results(
        &self,
        id: &TaskQAId,
        agent_id: &str,
        results: &QAResults,
        screenshots: &[String],
    ) -> AppResult<()>;

    /// Get tasks that need QA prep (have no acceptance criteria yet)
    async fn get_pending_prep(&self) -> AppResult<Vec<TaskQA>>;

    /// Delete TaskQA by ID
    async fn delete(&self, id: &TaskQAId) -> AppResult<()>;

    /// Delete TaskQA by task ID
    async fn delete_by_task_id(&self, task_id: &TaskId) -> AppResult<()>;

    /// Check if TaskQA exists for a task
    async fn exists_for_task(&self, task_id: &TaskId) -> AppResult<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};
    use chrono::Utc;

    // Mock implementation for testing trait object usage
    struct MockTaskQARepository {
        records: RwLock<HashMap<String, TaskQA>>,
    }

    impl MockTaskQARepository {
        fn new() -> Self {
            Self {
                records: RwLock::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl TaskQARepository for MockTaskQARepository {
        async fn create(&self, task_qa: &TaskQA) -> AppResult<()> {
            let mut records = self.records.write().unwrap();
            records.insert(task_qa.id.as_str().to_string(), task_qa.clone());
            Ok(())
        }

        async fn get_by_id(&self, id: &TaskQAId) -> AppResult<Option<TaskQA>> {
            let records = self.records.read().unwrap();
            Ok(records.get(id.as_str()).cloned())
        }

        async fn get_by_task_id(&self, task_id: &TaskId) -> AppResult<Option<TaskQA>> {
            let records = self.records.read().unwrap();
            Ok(records.values().find(|r| r.task_id == *task_id).cloned())
        }

        async fn update_prep(
            &self,
            id: &TaskQAId,
            agent_id: &str,
            criteria: &AcceptanceCriteria,
            steps: &QATestSteps,
        ) -> AppResult<()> {
            let mut records = self.records.write().unwrap();
            if let Some(record) = records.get_mut(id.as_str()) {
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
            let mut records = self.records.write().unwrap();
            if let Some(record) = records.get_mut(id.as_str()) {
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
            let mut records = self.records.write().unwrap();
            if let Some(record) = records.get_mut(id.as_str()) {
                record.test_agent_id = Some(agent_id.to_string());
                record.test_results = Some(results.clone());
                record.screenshots = screenshots.to_vec();
                record.test_completed_at = Some(Utc::now());
            }
            Ok(())
        }

        async fn get_pending_prep(&self) -> AppResult<Vec<TaskQA>> {
            let records = self.records.read().unwrap();
            Ok(records
                .values()
                .filter(|r| r.acceptance_criteria.is_none())
                .cloned()
                .collect())
        }

        async fn delete(&self, id: &TaskQAId) -> AppResult<()> {
            let mut records = self.records.write().unwrap();
            records.remove(id.as_str());
            Ok(())
        }

        async fn delete_by_task_id(&self, task_id: &TaskId) -> AppResult<()> {
            let mut records = self.records.write().unwrap();
            records.retain(|_, r| r.task_id != *task_id);
            Ok(())
        }

        async fn exists_for_task(&self, task_id: &TaskId) -> AppResult<bool> {
            let records = self.records.read().unwrap();
            Ok(records.values().any(|r| r.task_id == *task_id))
        }
    }

    #[test]
    fn test_task_qa_repository_trait_can_be_object_safe() {
        let repo: Arc<dyn TaskQARepository> = Arc::new(MockTaskQARepository::new());
        assert!(Arc::strong_count(&repo) == 1);
    }

    #[tokio::test]
    async fn test_mock_repository_create_and_get() {
        let repo = MockTaskQARepository::new();
        let task_id = TaskId::from_string("task-123".to_string());
        let task_qa = TaskQA::new(task_id.clone());
        let qa_id = task_qa.id.clone();

        repo.create(&task_qa).await.unwrap();

        let retrieved = repo.get_by_id(&qa_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().task_id, task_id);
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_task_id() {
        let repo = MockTaskQARepository::new();
        let task_id = TaskId::from_string("task-123".to_string());
        let task_qa = TaskQA::new(task_id.clone());

        repo.create(&task_qa).await.unwrap();

        let retrieved = repo.get_by_task_id(&task_id).await.unwrap();
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_mock_repository_update_prep() {
        use crate::domain::qa::{AcceptanceCriterion, QATestStep};

        let repo = MockTaskQARepository::new();
        let task_id = TaskId::from_string("task-123".to_string());
        let task_qa = TaskQA::new(task_id);
        let qa_id = task_qa.id.clone();

        repo.create(&task_qa).await.unwrap();

        let criteria = AcceptanceCriteria::from_criteria(vec![
            AcceptanceCriterion::visual("AC1", "Test"),
        ]);
        let steps = QATestSteps::from_steps(vec![
            QATestStep::new("QA1", "AC1", "Step", vec![], "Expected"),
        ]);

        repo.update_prep(&qa_id, "agent-1", &criteria, &steps).await.unwrap();

        let retrieved = repo.get_by_id(&qa_id).await.unwrap().unwrap();
        assert!(retrieved.acceptance_criteria.is_some());
        assert!(retrieved.qa_test_steps.is_some());
        assert!(retrieved.prep_completed_at.is_some());
    }

    #[tokio::test]
    async fn test_mock_repository_update_refinement() {
        use crate::domain::qa::QATestStep;

        let repo = MockTaskQARepository::new();
        let task_id = TaskId::from_string("task-123".to_string());
        let task_qa = TaskQA::new(task_id);
        let qa_id = task_qa.id.clone();

        repo.create(&task_qa).await.unwrap();

        let refined_steps = QATestSteps::from_steps(vec![
            QATestStep::new("QA1", "AC1", "Refined", vec![], "Expected"),
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
    async fn test_mock_repository_update_results() {
        use crate::domain::qa::QAStepResult;

        let repo = MockTaskQARepository::new();
        let task_id = TaskId::from_string("task-123".to_string());
        let task_qa = TaskQA::new(task_id.clone());
        let qa_id = task_qa.id.clone();

        repo.create(&task_qa).await.unwrap();

        let results = QAResults::from_results(
            task_id.as_str(),
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
    async fn test_mock_repository_get_pending_prep() {
        use crate::domain::qa::{AcceptanceCriterion, QATestStep};

        let repo = MockTaskQARepository::new();

        // Create two tasks - one with prep complete, one pending
        let task_id1 = TaskId::from_string("task-1".to_string());
        let task_qa1 = TaskQA::new(task_id1);
        let qa_id1 = task_qa1.id.clone();
        repo.create(&task_qa1).await.unwrap();

        let task_id2 = TaskId::from_string("task-2".to_string());
        let task_qa2 = TaskQA::new(task_id2);
        repo.create(&task_qa2).await.unwrap();

        // Complete prep for first task
        let criteria = AcceptanceCriteria::from_criteria(vec![
            AcceptanceCriterion::visual("AC1", "Test"),
        ]);
        let steps = QATestSteps::from_steps(vec![
            QATestStep::new("QA1", "AC1", "Step", vec![], "Expected"),
        ]);
        repo.update_prep(&qa_id1, "agent-1", &criteria, &steps).await.unwrap();

        // Get pending prep tasks
        let pending = repo.get_pending_prep().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].task_id, TaskId::from_string("task-2".to_string()));
    }

    #[tokio::test]
    async fn test_mock_repository_delete() {
        let repo = MockTaskQARepository::new();
        let task_id = TaskId::from_string("task-123".to_string());
        let task_qa = TaskQA::new(task_id);
        let qa_id = task_qa.id.clone();

        repo.create(&task_qa).await.unwrap();
        repo.delete(&qa_id).await.unwrap();

        let retrieved = repo.get_by_id(&qa_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_mock_repository_delete_by_task_id() {
        let repo = MockTaskQARepository::new();
        let task_id = TaskId::from_string("task-123".to_string());
        let task_qa = TaskQA::new(task_id.clone());

        repo.create(&task_qa).await.unwrap();
        repo.delete_by_task_id(&task_id).await.unwrap();

        let retrieved = repo.get_by_task_id(&task_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_mock_repository_exists_for_task() {
        let repo = MockTaskQARepository::new();
        let task_id = TaskId::from_string("task-123".to_string());

        assert!(!repo.exists_for_task(&task_id).await.unwrap());

        let task_qa = TaskQA::new(task_id.clone());
        repo.create(&task_qa).await.unwrap();

        assert!(repo.exists_for_task(&task_id).await.unwrap());
    }
}
