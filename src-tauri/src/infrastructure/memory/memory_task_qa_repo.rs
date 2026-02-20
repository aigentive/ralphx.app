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
        let map: HashMap<TaskQAId, TaskQA> =
            records.into_iter().map(|r| (r.id.clone(), r)).collect();
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
#[path = "memory_task_qa_repo_tests.rs"]
mod tests;
