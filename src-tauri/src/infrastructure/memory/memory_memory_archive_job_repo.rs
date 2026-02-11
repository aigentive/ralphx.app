// In-memory implementation of MemoryArchiveJobRepository for testing

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;
use chrono::Utc;

use crate::domain::entities::{
    MemoryArchiveJob, MemoryArchiveJobId, MemoryArchiveJobStatus, ProcessId,
};
use crate::domain::repositories::MemoryArchiveJobRepository;
use crate::error::{AppError, AppResult};

pub struct InMemoryMemoryArchiveJobRepository {
    jobs: Arc<RwLock<HashMap<MemoryArchiveJobId, MemoryArchiveJob>>>,
}

impl Default for InMemoryMemoryArchiveJobRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryMemoryArchiveJobRepository {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl MemoryArchiveJobRepository for InMemoryMemoryArchiveJobRepository {
    async fn create(&self, job: MemoryArchiveJob) -> AppResult<MemoryArchiveJob> {
        let mut jobs = self.jobs.write().await;
        jobs.insert(job.id.clone(), job.clone());
        Ok(job)
    }

    async fn get_by_id(&self, id: &MemoryArchiveJobId) -> AppResult<Option<MemoryArchiveJob>> {
        let jobs = self.jobs.read().await;
        Ok(jobs.get(id).cloned())
    }

    async fn get_pending_by_project(
        &self,
        project_id: &ProcessId,
    ) -> AppResult<Vec<MemoryArchiveJob>> {
        let jobs = self.jobs.read().await;
        let mut result: Vec<_> = jobs
            .values()
            .filter(|j| {
                j.project_id == *project_id && j.status == MemoryArchiveJobStatus::Pending
            })
            .cloned()
            .collect();
        result.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(result)
    }

    async fn update_status(
        &self,
        id: &MemoryArchiveJobId,
        status: MemoryArchiveJobStatus,
        error_message: Option<String>,
    ) -> AppResult<()> {
        let mut jobs = self.jobs.write().await;
        match jobs.get_mut(id) {
            Some(job) => {
                let now = Utc::now();
                match status {
                    MemoryArchiveJobStatus::Running => {
                        if job.started_at.is_none() {
                            job.started_at = Some(now);
                        }
                    }
                    MemoryArchiveJobStatus::Done | MemoryArchiveJobStatus::Failed => {
                        if job.completed_at.is_none() {
                            job.completed_at = Some(now);
                        }
                    }
                    MemoryArchiveJobStatus::Pending => {}
                }
                job.status = status;
                job.error_message = error_message;
                job.updated_at = now;
                Ok(())
            }
            None => Err(AppError::NotFound(format!("Archive job not found: {}", id))),
        }
    }
}
