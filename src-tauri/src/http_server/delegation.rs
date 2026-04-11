use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::domain::agents::{AgentHandle, AgentHarnessKind};

#[derive(Debug, Clone, serde::Serialize)]
pub struct DelegationJobSnapshot {
    pub job_id: String,
    pub parent_session_id: String,
    pub child_session_id: String,
    pub agent_name: String,
    pub harness: String,
    pub status: String,
    pub content: Option<String>,
    pub error: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone)]
struct DelegationJobRecord {
    snapshot: DelegationJobSnapshot,
    harness: AgentHarnessKind,
    handle: Option<AgentHandle>,
}

#[derive(Clone, Default)]
pub struct DelegationService {
    jobs: Arc<RwLock<HashMap<String, DelegationJobRecord>>>,
}

impl DelegationService {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn register_running(
        &self,
        job_id: String,
        parent_session_id: String,
        child_session_id: String,
        agent_name: String,
        harness: AgentHarnessKind,
        handle: AgentHandle,
    ) -> DelegationJobSnapshot {
        let snapshot = DelegationJobSnapshot {
            job_id: job_id.clone(),
            parent_session_id,
            child_session_id,
            agent_name,
            harness: harness.to_string(),
            status: "running".to_string(),
            content: None,
            error: None,
            started_at: Utc::now().to_rfc3339(),
            completed_at: None,
        };

        self.jobs.write().await.insert(
            job_id,
            DelegationJobRecord {
                snapshot: snapshot.clone(),
                harness,
                handle: Some(handle),
            },
        );

        snapshot
    }

    pub async fn snapshot(&self, job_id: &str) -> Option<DelegationJobSnapshot> {
        self.jobs
            .read()
            .await
            .get(job_id)
            .map(|record| record.snapshot.clone())
    }

    pub async fn mark_completed(&self, job_id: &str, content: String) {
        let mut jobs = self.jobs.write().await;
        let Some(record) = jobs.get_mut(job_id) else {
            return;
        };
        if record.snapshot.status != "running" {
            return;
        }
        record.snapshot.status = "completed".to_string();
        record.snapshot.content = Some(content);
        record.snapshot.error = None;
        record.snapshot.completed_at = Some(Utc::now().to_rfc3339());
        record.handle = None;
    }

    pub async fn mark_failed(&self, job_id: &str, error: String) {
        let mut jobs = self.jobs.write().await;
        let Some(record) = jobs.get_mut(job_id) else {
            return;
        };
        if record.snapshot.status != "running" {
            return;
        }
        record.snapshot.status = "failed".to_string();
        record.snapshot.error = Some(error);
        record.snapshot.completed_at = Some(Utc::now().to_rfc3339());
        record.handle = None;
    }

    pub async fn cancel(
        &self,
        job_id: &str,
    ) -> Option<(AgentHarnessKind, AgentHandle, DelegationJobSnapshot)> {
        let mut jobs = self.jobs.write().await;
        let record = jobs.get_mut(job_id)?;
        let handle = record.handle.take()?;
        record.snapshot.status = "cancelled".to_string();
        record.snapshot.completed_at = Some(Utc::now().to_rfc3339());
        Some((record.harness, handle, record.snapshot.clone()))
    }
}
