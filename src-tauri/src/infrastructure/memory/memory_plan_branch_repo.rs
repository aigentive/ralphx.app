// Memory-based PlanBranchRepository implementation for testing

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::domain::entities::{ArtifactId, IdeationSessionId, PlanBranch, PlanBranchId, PlanBranchStatus, ProjectId, TaskId};
use crate::domain::repositories::PlanBranchRepository;
use crate::error::AppResult;

pub struct MemoryPlanBranchRepository {
    branches: Arc<RwLock<HashMap<String, PlanBranch>>>,
}

impl Default for MemoryPlanBranchRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryPlanBranchRepository {
    pub fn new() -> Self {
        Self {
            branches: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl PlanBranchRepository for MemoryPlanBranchRepository {
    async fn create(&self, branch: PlanBranch) -> AppResult<PlanBranch> {
        let mut branches = self.branches.write().await;
        branches.insert(branch.id.as_str().to_string(), branch.clone());
        Ok(branch)
    }

    async fn get_by_plan_artifact_id(&self, id: &ArtifactId) -> AppResult<Option<PlanBranch>> {
        let branches = self.branches.read().await;
        Ok(branches.values().find(|b| b.plan_artifact_id == *id).cloned())
    }

    async fn get_by_session_id(&self, session_id: &IdeationSessionId) -> AppResult<Option<PlanBranch>> {
        let branches = self.branches.read().await;
        Ok(branches.values().find(|b| b.session_id == *session_id).cloned())
    }

    async fn get_by_merge_task_id(&self, task_id: &TaskId) -> AppResult<Option<PlanBranch>> {
        let branches = self.branches.read().await;
        Ok(branches.values().find(|b| b.merge_task_id.as_ref() == Some(task_id)).cloned())
    }

    async fn get_by_project_id(&self, project_id: &ProjectId) -> AppResult<Vec<PlanBranch>> {
        let branches = self.branches.read().await;
        Ok(branches.values().filter(|b| b.project_id == *project_id).cloned().collect())
    }

    async fn update_status(&self, id: &PlanBranchId, status: PlanBranchStatus) -> AppResult<()> {
        let mut branches = self.branches.write().await;
        if let Some(branch) = branches.get_mut(id.as_str()) {
            branch.status = status;
        }
        Ok(())
    }

    async fn set_merge_task_id(&self, id: &PlanBranchId, task_id: &TaskId) -> AppResult<()> {
        let mut branches = self.branches.write().await;
        if let Some(branch) = branches.get_mut(id.as_str()) {
            branch.merge_task_id = Some(task_id.clone());
        }
        Ok(())
    }

    async fn set_merged(&self, id: &PlanBranchId) -> AppResult<()> {
        let mut branches = self.branches.write().await;
        if let Some(branch) = branches.get_mut(id.as_str()) {
            branch.status = PlanBranchStatus::Merged;
            branch.merged_at = Some(chrono::Utc::now());
        }
        Ok(())
    }
}
