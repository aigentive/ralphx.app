// In-memory implementation of MemoryEventRepository for testing

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;

use crate::domain::entities::{MemoryEvent, MemoryEventId, ProcessId};
use crate::domain::repositories::MemoryEventRepository;
use crate::error::AppResult;

pub struct InMemoryMemoryEventRepository {
    events: Arc<RwLock<HashMap<MemoryEventId, MemoryEvent>>>,
}

impl Default for InMemoryMemoryEventRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryMemoryEventRepository {
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl MemoryEventRepository for InMemoryMemoryEventRepository {
    async fn create(&self, event: MemoryEvent) -> AppResult<MemoryEvent> {
        let mut events = self.events.write().await;
        events.insert(event.id.clone(), event.clone());
        Ok(event)
    }

    async fn get_by_project(&self, project_id: &ProcessId) -> AppResult<Vec<MemoryEvent>> {
        let events = self.events.read().await;
        let mut result: Vec<_> = events
            .values()
            .filter(|e| e.project_id == *project_id)
            .cloned()
            .collect();
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(result)
    }

    async fn get_by_type(&self, event_type: &str) -> AppResult<Vec<MemoryEvent>> {
        let events = self.events.read().await;
        let mut result: Vec<_> = events
            .values()
            .filter(|e| e.event_type == event_type)
            .cloned()
            .collect();
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(result)
    }
}
