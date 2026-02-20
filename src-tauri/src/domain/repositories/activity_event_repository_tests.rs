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

    async fn list_all(
        &self,
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
    let filter = ActivityEventFilter::new().with_statuses(vec![InternalStatus::Executing]);
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
