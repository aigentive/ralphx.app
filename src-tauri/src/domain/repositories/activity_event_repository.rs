// Activity event repository trait - domain layer abstraction
//
// This trait defines the contract for activity event persistence.
// Events can belong to either tasks or ideation sessions.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::domain::entities::{
    ActivityEvent, ActivityEventId, ActivityEventRole, ActivityEventType, IdeationSessionId,
    InternalStatus, TaskId,
};
use crate::error::AppResult;

/// Filter criteria for querying activity events
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActivityEventFilter {
    /// Filter by event type(s)
    pub event_types: Option<Vec<ActivityEventType>>,
    /// Filter by role(s)
    pub roles: Option<Vec<ActivityEventRole>>,
    /// Filter by internal status(es)
    pub statuses: Option<Vec<InternalStatus>>,
}

impl ActivityEventFilter {
    /// Create a new empty filter (no filtering)
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter to specific event types
    pub fn with_event_types(mut self, types: Vec<ActivityEventType>) -> Self {
        self.event_types = Some(types);
        self
    }

    /// Filter to specific roles
    pub fn with_roles(mut self, roles: Vec<ActivityEventRole>) -> Self {
        self.roles = Some(roles);
        self
    }

    /// Filter to specific internal statuses
    pub fn with_statuses(mut self, statuses: Vec<InternalStatus>) -> Self {
        self.statuses = Some(statuses);
        self
    }

    /// Check if the filter is empty (no filtering criteria)
    pub fn is_empty(&self) -> bool {
        self.event_types.is_none() && self.roles.is_none() && self.statuses.is_none()
    }
}

/// Result from a paginated query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEventPage {
    /// The events in this page
    pub events: Vec<ActivityEvent>,
    /// Cursor for the next page (None if no more pages)
    pub cursor: Option<String>,
    /// Whether there are more pages
    pub has_more: bool,
}

/// Repository trait for ActivityEvent persistence.
/// Implementations can use SQLite, PostgreSQL, in-memory, etc.
#[async_trait]
pub trait ActivityEventRepository: Send + Sync {
    /// Save a new activity event
    async fn save(&self, event: ActivityEvent) -> AppResult<ActivityEvent>;

    /// Get an event by ID
    async fn get_by_id(&self, id: &ActivityEventId) -> AppResult<Option<ActivityEvent>>;

    /// List events for a task with cursor-based pagination
    ///
    /// # Arguments
    /// * `task_id` - The task to get events for
    /// * `cursor` - Optional cursor from previous page (format: "timestamp:id")
    /// * `limit` - Maximum number of events to return (max 100)
    /// * `filter` - Optional filter criteria
    ///
    /// # Returns
    /// A page of events ordered by created_at DESC (newest first)
    async fn list_by_task_id(
        &self,
        task_id: &TaskId,
        cursor: Option<&str>,
        limit: u32,
        filter: Option<&ActivityEventFilter>,
    ) -> AppResult<ActivityEventPage>;

    /// List events for an ideation session with cursor-based pagination
    ///
    /// # Arguments
    /// * `session_id` - The session to get events for
    /// * `cursor` - Optional cursor from previous page (format: "timestamp:id")
    /// * `limit` - Maximum number of events to return (max 100)
    /// * `filter` - Optional filter criteria
    ///
    /// # Returns
    /// A page of events ordered by created_at DESC (newest first)
    async fn list_by_session_id(
        &self,
        session_id: &IdeationSessionId,
        cursor: Option<&str>,
        limit: u32,
        filter: Option<&ActivityEventFilter>,
    ) -> AppResult<ActivityEventPage>;

    /// Delete all events for a task
    async fn delete_by_task_id(&self, task_id: &TaskId) -> AppResult<()>;

    /// Delete all events for an ideation session
    async fn delete_by_session_id(&self, session_id: &IdeationSessionId) -> AppResult<()>;

    /// Count events for a task
    async fn count_by_task_id(
        &self,
        task_id: &TaskId,
        filter: Option<&ActivityEventFilter>,
    ) -> AppResult<u64>;

    /// Count events for an ideation session
    async fn count_by_session_id(
        &self,
        session_id: &IdeationSessionId,
        filter: Option<&ActivityEventFilter>,
    ) -> AppResult<u64>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // Mock implementation for testing trait object usage
    struct MockActivityEventRepository;

    #[async_trait]
    impl ActivityEventRepository for MockActivityEventRepository {
        async fn save(&self, event: ActivityEvent) -> AppResult<ActivityEvent> {
            Ok(event)
        }

        async fn get_by_id(&self, _id: &ActivityEventId) -> AppResult<Option<ActivityEvent>> {
            Ok(None)
        }

        async fn list_by_task_id(
            &self,
            _task_id: &TaskId,
            _cursor: Option<&str>,
            _limit: u32,
            _filter: Option<&ActivityEventFilter>,
        ) -> AppResult<ActivityEventPage> {
            Ok(ActivityEventPage {
                events: vec![],
                cursor: None,
                has_more: false,
            })
        }

        async fn list_by_session_id(
            &self,
            _session_id: &IdeationSessionId,
            _cursor: Option<&str>,
            _limit: u32,
            _filter: Option<&ActivityEventFilter>,
        ) -> AppResult<ActivityEventPage> {
            Ok(ActivityEventPage {
                events: vec![],
                cursor: None,
                has_more: false,
            })
        }

        async fn delete_by_task_id(&self, _task_id: &TaskId) -> AppResult<()> {
            Ok(())
        }

        async fn delete_by_session_id(&self, _session_id: &IdeationSessionId) -> AppResult<()> {
            Ok(())
        }

        async fn count_by_task_id(
            &self,
            _task_id: &TaskId,
            _filter: Option<&ActivityEventFilter>,
        ) -> AppResult<u64> {
            Ok(0)
        }

        async fn count_by_session_id(
            &self,
            _session_id: &IdeationSessionId,
            _filter: Option<&ActivityEventFilter>,
        ) -> AppResult<u64> {
            Ok(0)
        }
    }

    #[test]
    fn activity_event_filter_new_is_empty() {
        let filter = ActivityEventFilter::new();
        assert!(filter.is_empty());
    }

    #[test]
    fn activity_event_filter_with_event_types() {
        let filter = ActivityEventFilter::new()
            .with_event_types(vec![ActivityEventType::Thinking, ActivityEventType::Text]);
        assert!(!filter.is_empty());
        assert!(filter.event_types.is_some());
        assert_eq!(filter.event_types.unwrap().len(), 2);
    }

    #[test]
    fn activity_event_filter_with_roles() {
        let filter = ActivityEventFilter::new().with_roles(vec![ActivityEventRole::Agent]);
        assert!(!filter.is_empty());
        assert!(filter.roles.is_some());
    }

    #[test]
    fn activity_event_filter_with_statuses() {
        let filter =
            ActivityEventFilter::new().with_statuses(vec![InternalStatus::Executing]);
        assert!(!filter.is_empty());
        assert!(filter.statuses.is_some());
    }

    #[test]
    fn activity_event_filter_combined() {
        let filter = ActivityEventFilter::new()
            .with_event_types(vec![ActivityEventType::Thinking])
            .with_roles(vec![ActivityEventRole::Agent])
            .with_statuses(vec![InternalStatus::Executing]);
        assert!(!filter.is_empty());
    }

    #[test]
    fn activity_event_repository_trait_is_object_safe() {
        let repo: Arc<dyn ActivityEventRepository> = Arc::new(MockActivityEventRepository);
        assert!(Arc::strong_count(&repo) == 1);
    }

    #[tokio::test]
    async fn mock_repository_save() {
        let repo = MockActivityEventRepository;
        let task_id = TaskId::new();
        let event = ActivityEvent::new_task_event(task_id, ActivityEventType::Text, "test");

        let result = repo.save(event.clone()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, event.id);
    }

    #[tokio::test]
    async fn mock_repository_get_by_id_returns_none() {
        let repo = MockActivityEventRepository;
        let event_id = ActivityEventId::new();

        let result = repo.get_by_id(&event_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn mock_repository_list_by_task_id_returns_empty_page() {
        let repo = MockActivityEventRepository;
        let task_id = TaskId::new();

        let result = repo.list_by_task_id(&task_id, None, 50, None).await;
        assert!(result.is_ok());
        let page = result.unwrap();
        assert!(page.events.is_empty());
        assert!(!page.has_more);
        assert!(page.cursor.is_none());
    }

    #[tokio::test]
    async fn mock_repository_list_by_session_id_returns_empty_page() {
        let repo = MockActivityEventRepository;
        let session_id = IdeationSessionId::new();

        let result = repo.list_by_session_id(&session_id, None, 50, None).await;
        assert!(result.is_ok());
        let page = result.unwrap();
        assert!(page.events.is_empty());
        assert!(!page.has_more);
    }

    #[tokio::test]
    async fn mock_repository_delete_by_task_id() {
        let repo = MockActivityEventRepository;
        let task_id = TaskId::new();

        let result = repo.delete_by_task_id(&task_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn mock_repository_delete_by_session_id() {
        let repo = MockActivityEventRepository;
        let session_id = IdeationSessionId::new();

        let result = repo.delete_by_session_id(&session_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn mock_repository_count_by_task_id() {
        let repo = MockActivityEventRepository;
        let task_id = TaskId::new();

        let result = repo.count_by_task_id(&task_id, None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn mock_repository_count_by_session_id() {
        let repo = MockActivityEventRepository;
        let session_id = IdeationSessionId::new();

        let result = repo.count_by_session_id(&session_id, None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn repository_trait_object_in_arc() {
        let repo: Arc<dyn ActivityEventRepository> = Arc::new(MockActivityEventRepository);
        let task_id = TaskId::new();

        let result = repo.list_by_task_id(&task_id, None, 50, None).await;
        assert!(result.is_ok());
    }
}
