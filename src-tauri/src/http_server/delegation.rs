use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::http_server::types::DelegatedSessionStatusResponse;

#[derive(Debug, Clone, serde::Serialize)]
pub struct DelegationHistoryEntry {
    pub status: String,
    pub timestamp: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DelegationJobSnapshot {
    pub job_id: String,
    pub parent_context_type: String,
    pub parent_context_id: String,
    pub parent_turn_id: Option<String>,
    pub parent_message_id: Option<String>,
    pub parent_conversation_id: Option<String>,
    pub parent_tool_use_id: Option<String>,
    pub delegated_session_id: String,
    pub delegated_conversation_id: Option<String>,
    pub delegated_agent_run_id: Option<String>,
    pub agent_name: String,
    pub harness: String,
    pub status: String,
    pub content: Option<String>,
    pub error: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub history: Vec<DelegationHistoryEntry>,
    pub delegated_status: Option<DelegatedSessionStatusResponse>,
}

#[derive(Debug, Clone)]
struct DelegationJobRecord {
    snapshot: DelegationJobSnapshot,
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
        parent_context_type: String,
        parent_context_id: String,
        parent_turn_id: Option<String>,
        parent_message_id: Option<String>,
        parent_conversation_id: Option<String>,
        parent_tool_use_id: Option<String>,
        delegated_session_id: String,
        delegated_conversation_id: Option<String>,
        delegated_agent_run_id: Option<String>,
        agent_name: String,
        harness: impl Into<String>,
    ) -> DelegationJobSnapshot {
        let started_at = Utc::now().to_rfc3339();
        let snapshot = DelegationJobSnapshot {
            job_id: job_id.clone(),
            parent_context_type,
            parent_context_id,
            parent_turn_id,
            parent_message_id,
            parent_conversation_id,
            parent_tool_use_id,
            delegated_session_id,
            delegated_conversation_id,
            delegated_agent_run_id,
            agent_name,
            harness: harness.into(),
            status: "running".to_string(),
            content: None,
            error: None,
            started_at: started_at.clone(),
            completed_at: None,
            history: vec![DelegationHistoryEntry {
                status: "running".to_string(),
                timestamp: started_at,
                detail: None,
            }],
            delegated_status: None,
        };

        self.jobs.write().await.insert(
            job_id,
            DelegationJobRecord {
                snapshot: snapshot.clone(),
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
        let completed_at = Utc::now().to_rfc3339();
        record.snapshot.completed_at = Some(completed_at.clone());
        record.snapshot.history.push(DelegationHistoryEntry {
            status: "completed".to_string(),
            timestamp: completed_at,
            detail: None,
        });
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
        let completed_at = Utc::now().to_rfc3339();
        record.snapshot.error = Some(error.clone());
        record.snapshot.completed_at = Some(completed_at.clone());
        record.snapshot.history.push(DelegationHistoryEntry {
            status: "failed".to_string(),
            timestamp: completed_at,
            detail: Some(error),
        });
    }

    pub async fn cancel(&self, job_id: &str) -> Option<DelegationJobSnapshot> {
        let mut jobs = self.jobs.write().await;
        let record = jobs.get_mut(job_id)?;
        if record.snapshot.status != "running" {
            return None;
        }
        record.snapshot.status = "cancelled".to_string();
        let completed_at = Utc::now().to_rfc3339();
        record.snapshot.completed_at = Some(completed_at.clone());
        record.snapshot.history.push(DelegationHistoryEntry {
            status: "cancelled".to_string(),
            timestamp: completed_at,
            detail: None,
        });
        Some(record.snapshot.clone())
    }
}
