use super::*;
use crate::domain::entities::{ActivityEventType, InternalStatus};

#[tokio::test]
async fn test_save_and_get_by_id() {
    let repo = MemoryActivityEventRepository::new();
    let task_id = TaskId::new();
    let event = ActivityEvent::new_task_event(task_id, ActivityEventType::Text, "test");
    let event_id = event.id.clone();

    repo.save(event.clone()).await.unwrap();

    let found = repo.get_by_id(&event_id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, event_id);
}

#[tokio::test]
async fn test_list_by_task_id_empty() {
    let repo = MemoryActivityEventRepository::new();
    let task_id = TaskId::new();

    let page = repo
        .list_by_task_id(&task_id, None, 50, None)
        .await
        .unwrap();
    assert!(page.events.is_empty());
    assert!(!page.has_more);
}

#[tokio::test]
async fn test_list_by_task_id_with_events() {
    let repo = MemoryActivityEventRepository::new();
    let task_id = TaskId::new();

    // Create a few events
    for i in 0..3 {
        let event = ActivityEvent::new_task_event(
            task_id.clone(),
            ActivityEventType::Text,
            format!("content {}", i),
        );
        repo.save(event).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(2)).await;
    }

    let page = repo
        .list_by_task_id(&task_id, None, 50, None)
        .await
        .unwrap();
    assert_eq!(page.events.len(), 3);
    assert!(!page.has_more);
}

#[tokio::test]
async fn test_list_by_task_id_pagination() {
    let repo = MemoryActivityEventRepository::new();
    let task_id = TaskId::new();

    // Create 5 events
    for i in 0..5 {
        let event = ActivityEvent::new_task_event(
            task_id.clone(),
            ActivityEventType::Text,
            format!("content {}", i),
        );
        repo.save(event).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(2)).await;
    }

    // First page
    let page1 = repo.list_by_task_id(&task_id, None, 3, None).await.unwrap();
    assert_eq!(page1.events.len(), 3);
    assert!(page1.has_more);
    assert!(page1.cursor.is_some());

    // Second page
    let page2 = repo
        .list_by_task_id(&task_id, page1.cursor.as_deref(), 3, None)
        .await
        .unwrap();
    assert_eq!(page2.events.len(), 2);
    assert!(!page2.has_more);
}

#[tokio::test]
async fn test_list_by_session_id() {
    let repo = MemoryActivityEventRepository::new();
    let session_id = IdeationSessionId::new();

    let event =
        ActivityEvent::new_session_event(session_id.clone(), ActivityEventType::ToolCall, "x");
    repo.save(event).await.unwrap();

    let page = repo
        .list_by_session_id(&session_id, None, 50, None)
        .await
        .unwrap();
    assert_eq!(page.events.len(), 1);
}

#[tokio::test]
async fn test_list_with_filter() {
    let repo = MemoryActivityEventRepository::new();
    let task_id = TaskId::new();

    repo.save(ActivityEvent::new_task_event(
        task_id.clone(),
        ActivityEventType::Thinking,
        "thinking",
    ))
    .await
    .unwrap();
    repo.save(ActivityEvent::new_task_event(
        task_id.clone(),
        ActivityEventType::Text,
        "text",
    ))
    .await
    .unwrap();

    let filter = ActivityEventFilter::new().with_event_types(vec![ActivityEventType::Thinking]);
    let page = repo
        .list_by_task_id(&task_id, None, 50, Some(&filter))
        .await
        .unwrap();
    assert_eq!(page.events.len(), 1);
    assert_eq!(page.events[0].event_type, ActivityEventType::Thinking);
}

#[tokio::test]
async fn test_delete_by_task_id() {
    let repo = MemoryActivityEventRepository::new();
    let task_id = TaskId::new();

    repo.save(ActivityEvent::new_task_event(
        task_id.clone(),
        ActivityEventType::Text,
        "test",
    ))
    .await
    .unwrap();

    repo.delete_by_task_id(&task_id).await.unwrap();

    let page = repo
        .list_by_task_id(&task_id, None, 50, None)
        .await
        .unwrap();
    assert!(page.events.is_empty());
}

#[tokio::test]
async fn test_delete_by_session_id() {
    let repo = MemoryActivityEventRepository::new();
    let session_id = IdeationSessionId::new();

    repo.save(ActivityEvent::new_session_event(
        session_id.clone(),
        ActivityEventType::Text,
        "test",
    ))
    .await
    .unwrap();

    repo.delete_by_session_id(&session_id).await.unwrap();

    let page = repo
        .list_by_session_id(&session_id, None, 50, None)
        .await
        .unwrap();
    assert!(page.events.is_empty());
}

#[tokio::test]
async fn test_count_by_task_id() {
    let repo = MemoryActivityEventRepository::new();
    let task_id = TaskId::new();

    for _ in 0..3 {
        repo.save(ActivityEvent::new_task_event(
            task_id.clone(),
            ActivityEventType::Text,
            "test",
        ))
        .await
        .unwrap();
    }

    let count = repo.count_by_task_id(&task_id, None).await.unwrap();
    assert_eq!(count, 3);
}

#[tokio::test]
async fn test_count_by_session_id() {
    let repo = MemoryActivityEventRepository::new();
    let session_id = IdeationSessionId::new();

    for _ in 0..2 {
        repo.save(ActivityEvent::new_session_event(
            session_id.clone(),
            ActivityEventType::Text,
            "test",
        ))
        .await
        .unwrap();
    }

    let count = repo.count_by_session_id(&session_id, None).await.unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn test_count_with_filter() {
    let repo = MemoryActivityEventRepository::new();
    let task_id = TaskId::new();

    repo.save(ActivityEvent::new_task_event(
        task_id.clone(),
        ActivityEventType::Thinking,
        "thinking",
    ))
    .await
    .unwrap();
    repo.save(ActivityEvent::new_task_event(
        task_id.clone(),
        ActivityEventType::Text,
        "text",
    ))
    .await
    .unwrap();

    let filter = ActivityEventFilter::new().with_event_types(vec![ActivityEventType::Thinking]);
    let count = repo
        .count_by_task_id(&task_id, Some(&filter))
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn test_filter_by_status() {
    let repo = MemoryActivityEventRepository::new();
    let task_id = TaskId::new();

    repo.save(
        ActivityEvent::new_task_event(task_id.clone(), ActivityEventType::Text, "executing")
            .with_status(InternalStatus::Executing),
    )
    .await
    .unwrap();
    repo.save(
        ActivityEvent::new_task_event(task_id.clone(), ActivityEventType::Text, "ready")
            .with_status(InternalStatus::Ready),
    )
    .await
    .unwrap();

    let filter = ActivityEventFilter::new().with_statuses(vec![InternalStatus::Executing]);
    let page = repo
        .list_by_task_id(&task_id, None, 50, Some(&filter))
        .await
        .unwrap();
    assert_eq!(page.events.len(), 1);
    assert_eq!(
        page.events[0].internal_status,
        Some(InternalStatus::Executing)
    );
}

#[tokio::test]
async fn test_list_all_returns_all_events() {
    let repo = MemoryActivityEventRepository::new();
    let task_id = TaskId::new();
    let session_id = IdeationSessionId::new();

    // Create events for both task and session
    repo.save(ActivityEvent::new_task_event(
        task_id.clone(),
        ActivityEventType::Text,
        "task event 1",
    ))
    .await
    .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(2)).await;
    repo.save(ActivityEvent::new_session_event(
        session_id.clone(),
        ActivityEventType::Text,
        "session event 1",
    ))
    .await
    .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(2)).await;
    repo.save(ActivityEvent::new_task_event(
        task_id.clone(),
        ActivityEventType::ToolCall,
        "task event 2",
    ))
    .await
    .unwrap();

    // list_all should return all 3 events
    let page = repo.list_all(None, 50, None).await.unwrap();
    assert_eq!(page.events.len(), 3);
    assert!(!page.has_more);
}

#[tokio::test]
async fn test_list_all_with_task_id_filter() {
    let repo = MemoryActivityEventRepository::new();
    let task_id = TaskId::new();
    let session_id = IdeationSessionId::new();

    // Create events for both task and session
    repo.save(ActivityEvent::new_task_event(
        task_id.clone(),
        ActivityEventType::Text,
        "task event",
    ))
    .await
    .unwrap();
    repo.save(ActivityEvent::new_session_event(
        session_id.clone(),
        ActivityEventType::Text,
        "session event",
    ))
    .await
    .unwrap();

    // Filter by task_id
    let filter = ActivityEventFilter::new().with_task_id(task_id.clone());
    let page = repo.list_all(None, 50, Some(&filter)).await.unwrap();
    assert_eq!(page.events.len(), 1);
    assert_eq!(page.events[0].task_id, Some(task_id));
}

#[tokio::test]
async fn test_list_all_with_session_id_filter() {
    let repo = MemoryActivityEventRepository::new();
    let task_id = TaskId::new();
    let session_id = IdeationSessionId::new();

    // Create events for both task and session
    repo.save(ActivityEvent::new_task_event(
        task_id.clone(),
        ActivityEventType::Text,
        "task event",
    ))
    .await
    .unwrap();
    repo.save(ActivityEvent::new_session_event(
        session_id.clone(),
        ActivityEventType::Text,
        "session event",
    ))
    .await
    .unwrap();

    // Filter by session_id
    let filter = ActivityEventFilter::new().with_session_id(session_id.clone());
    let page = repo.list_all(None, 50, Some(&filter)).await.unwrap();
    assert_eq!(page.events.len(), 1);
    assert_eq!(page.events[0].ideation_session_id, Some(session_id));
}

#[tokio::test]
async fn test_list_all_pagination() {
    let repo = MemoryActivityEventRepository::new();
    let task_id = TaskId::new();

    // Create 5 events with delays
    for i in 0..5 {
        let event = ActivityEvent::new_task_event(
            task_id.clone(),
            ActivityEventType::Text,
            format!("content {}", i),
        );
        repo.save(event).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(2)).await;
    }

    // First page
    let page1 = repo.list_all(None, 3, None).await.unwrap();
    assert_eq!(page1.events.len(), 3);
    assert!(page1.has_more);
    assert!(page1.cursor.is_some());

    // Second page
    let page2 = repo
        .list_all(page1.cursor.as_deref(), 3, None)
        .await
        .unwrap();
    assert_eq!(page2.events.len(), 2);
    assert!(!page2.has_more);
}
