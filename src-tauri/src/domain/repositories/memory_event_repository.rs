// Memory event repository trait

use async_trait::async_trait;

use crate::domain::entities::{MemoryEvent, ProcessId};
use crate::error::AppResult;

/// Repository trait for MemoryEvent persistence
#[async_trait]
pub trait MemoryEventRepository: Send + Sync {
    /// Create a new memory event
    async fn create(&self, event: MemoryEvent) -> AppResult<MemoryEvent>;

    /// Get events for a project
    async fn get_by_project(&self, project_id: &ProcessId) -> AppResult<Vec<MemoryEvent>>;

    /// Get events by type
    async fn get_by_type(&self, event_type: &str) -> AppResult<Vec<MemoryEvent>>;
}
