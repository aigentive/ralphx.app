// In-memory ExternalEventsRepository for tests

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::domain::repositories::external_events_repository::{
    ExternalEventRecord, ExternalEventsRepository,
};
use crate::error::AppResult;

pub struct MemoryExternalEventsRepository {
    events: Arc<RwLock<Vec<ExternalEventRecord>>>,
    next_id: Arc<AtomicI64>,
}

impl MemoryExternalEventsRepository {
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            next_id: Arc::new(AtomicI64::new(1)),
        }
    }
}

impl Default for MemoryExternalEventsRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ExternalEventsRepository for MemoryExternalEventsRepository {
    async fn insert_event(
        &self,
        event_type: &str,
        project_id: &str,
        payload: &str,
    ) -> AppResult<i64> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let mut events = self.events.write().await;
        events.push(ExternalEventRecord {
            id,
            event_type: event_type.to_string(),
            project_id: project_id.to_string(),
            payload: payload.to_string(),
            created_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        });
        Ok(id)
    }

    async fn get_events_after_cursor(
        &self,
        project_ids: &[String],
        cursor: i64,
        limit: i64,
    ) -> AppResult<Vec<ExternalEventRecord>> {
        let events = self.events.read().await;
        let result: Vec<ExternalEventRecord> = events
            .iter()
            .filter(|e| e.id > cursor && project_ids.contains(&e.project_id))
            .take(limit as usize)
            .cloned()
            .collect();
        Ok(result)
    }

    async fn cleanup_old_events(&self) -> AppResult<u64> {
        // No-op for tests
        Ok(0)
    }
}
