use super::*;
use crate::domain::entities::{ActivityEventType, InternalStatus};
use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

fn setup_test_db() -> Connection {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Create project, task, and session for foreign key references
    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'Test', '/path')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title) VALUES ('t1', 'p1', 'feature', 'Task')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO ideation_sessions (id, project_id, title) VALUES ('s1', 'p1', 'Session')",
        [],
    )
    .unwrap();

    conn
}

#[tokio::test]
async fn test_save_and_get_by_id() {
    let conn = setup_test_db();
    let repo = SqliteActivityEventRepository::new(conn);

    let task_id = TaskId::from_string("t1".to_string());
    let event = ActivityEvent::new_task_event(task_id, ActivityEventType::Thinking, "test")
        .with_status(InternalStatus::Executing);

    let saved = repo.save(event.clone()).await.unwrap();
    assert_eq!(saved.id, event.id);

    let found = repo.get_by_id(&event.id).await.unwrap();
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.id, event.id);
    assert_eq!(found.event_type, ActivityEventType::Thinking);
    assert_eq!(found.internal_status, Some(InternalStatus::Executing));
}

#[tokio::test]
async fn test_list_by_task_id_empty() {
    let conn = setup_test_db();
    let repo = SqliteActivityEventRepository::new(conn);

    let task_id = TaskId::from_string("t1".to_string());

    let page = repo
        .list_by_task_id(&task_id, None, 50, None)
        .await
        .unwrap();
    assert!(page.events.is_empty());
    assert!(!page.has_more);
    assert!(page.cursor.is_none());
}

#[tokio::test]
async fn test_list_by_task_id_pagination() {
    let conn = setup_test_db();
    let repo = SqliteActivityEventRepository::new(conn);

    let task_id = TaskId::from_string("t1".to_string());

    // Create 5 events with small delays to ensure different timestamps
    for i in 0..5 {
        let event = ActivityEvent::new_task_event(
            task_id.clone(),
            ActivityEventType::Text,
            format!("content {}", i),
        );
        repo.save(event).await.unwrap();
        // Small delay to ensure different timestamps
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
    let conn = setup_test_db();
    let repo = SqliteActivityEventRepository::new(conn);

    let session_id = IdeationSessionId::from_string("s1".to_string());

    let event = ActivityEvent::new_session_event(
        session_id.clone(),
        ActivityEventType::ToolCall,
        "tool call",
    );
    repo.save(event).await.unwrap();

    let page = repo
        .list_by_session_id(&session_id, None, 50, None)
        .await
        .unwrap();
    assert_eq!(page.events.len(), 1);
    assert_eq!(page.events[0].event_type, ActivityEventType::ToolCall);
}

#[tokio::test]
async fn test_list_with_filter() {
    let conn = setup_test_db();
    let repo = SqliteActivityEventRepository::new(conn);

    let task_id = TaskId::from_string("t1".to_string());

    // Create different event types
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
    repo.save(ActivityEvent::new_task_event(
        task_id.clone(),
        ActivityEventType::ToolCall,
        "tool",
    ))
    .await
    .unwrap();

    // Filter by event type
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
    let conn = setup_test_db();
    let repo = SqliteActivityEventRepository::new(conn);

    let task_id = TaskId::from_string("t1".to_string());

    repo.save(ActivityEvent::new_task_event(
        task_id.clone(),
        ActivityEventType::Text,
        "test",
    ))
    .await
    .unwrap();
    repo.save(ActivityEvent::new_task_event(
        task_id.clone(),
        ActivityEventType::Text,
        "test2",
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
    let conn = setup_test_db();
    let repo = SqliteActivityEventRepository::new(conn);

    let session_id = IdeationSessionId::from_string("s1".to_string());

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
    let conn = setup_test_db();
    let repo = SqliteActivityEventRepository::new(conn);

    let task_id = TaskId::from_string("t1".to_string());

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
    let conn = setup_test_db();
    let repo = SqliteActivityEventRepository::new(conn);

    let session_id = IdeationSessionId::from_string("s1".to_string());

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
    let conn = setup_test_db();
    let repo = SqliteActivityEventRepository::new(conn);

    let task_id = TaskId::from_string("t1".to_string());

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
async fn test_list_all_returns_all_events() {
    let conn = setup_test_db();
    let repo = SqliteActivityEventRepository::new(conn);

    let task_id = TaskId::from_string("t1".to_string());
    let session_id = IdeationSessionId::from_string("s1".to_string());

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
    let conn = setup_test_db();
    let repo = SqliteActivityEventRepository::new(conn);

    let task_id = TaskId::from_string("t1".to_string());
    let session_id = IdeationSessionId::from_string("s1".to_string());

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
    let conn = setup_test_db();
    let repo = SqliteActivityEventRepository::new(conn);

    let task_id = TaskId::from_string("t1".to_string());
    let session_id = IdeationSessionId::from_string("s1".to_string());

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
    let conn = setup_test_db();
    let repo = SqliteActivityEventRepository::new(conn);

    let task_id = TaskId::from_string("t1".to_string());

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

#[test]
fn test_parse_cursor() {
    // Valid cursor
    let cursor = "2026-01-31T10:30:45+00:00|abc123";
    let result = SqliteActivityEventRepository::parse_cursor(cursor);
    assert!(result.is_some());
    let (ts, id) = result.unwrap();
    assert_eq!(ts, "2026-01-31T10:30:45+00:00");
    assert_eq!(id, "abc123");

    // Invalid cursor (no pipe)
    let cursor = "2026-01-31T10:30:45+00:00:abc123";
    let result = SqliteActivityEventRepository::parse_cursor(cursor);
    assert!(result.is_none());
}

#[test]
fn test_format_cursor() {
    let task_id = TaskId::from_string("t1".to_string());
    let event = ActivityEvent::new_task_event(task_id, ActivityEventType::Text, "test");
    let cursor = SqliteActivityEventRepository::format_cursor(&event);

    // Cursor should contain pipe separator
    assert!(cursor.contains('|'));

    // Should be parseable
    let parsed = SqliteActivityEventRepository::parse_cursor(&cursor);
    assert!(parsed.is_some());
    let (ts, id) = parsed.unwrap();
    assert_eq!(id, event.id.as_str());
    assert!(ts.contains("T")); // ISO timestamp
}

// Tests for Merge context activity persistence (task ef532169)

#[tokio::test]
async fn test_save_merge_context_event_with_merging_status() {
    let conn = setup_test_db();
    let repo = SqliteActivityEventRepository::new(conn);

    let task_id = TaskId::from_string("t1".to_string());
    // Simulate what chat_service_streaming does for Merge context:
    // persists events with internal_status = Merging
    let event =
        ActivityEvent::new_task_event(task_id.clone(), ActivityEventType::Text, "merger output")
            .with_status(InternalStatus::Merging);

    let saved = repo.save(event.clone()).await.unwrap();
    assert_eq!(saved.internal_status, Some(InternalStatus::Merging));

    // Verify it can be retrieved and has the correct status
    let found = repo.get_by_id(&saved.id).await.unwrap().unwrap();
    assert_eq!(found.internal_status, Some(InternalStatus::Merging));
    assert_eq!(found.event_type, ActivityEventType::Text);
    assert_eq!(found.task_id, Some(task_id));
}

#[tokio::test]
async fn test_merge_context_events_queryable_by_status_filter() {
    let conn = setup_test_db();
    let repo = SqliteActivityEventRepository::new(conn);

    let task_id = TaskId::from_string("t1".to_string());

    // Simulate merge agent producing multiple event types under Merging status
    for event_type in [
        ActivityEventType::Thinking,
        ActivityEventType::ToolCall,
        ActivityEventType::Text,
    ] {
        repo.save(
            ActivityEvent::new_task_event(task_id.clone(), event_type, "content")
                .with_status(InternalStatus::Merging),
        )
        .await
        .unwrap();
    }

    // Also save an Executing event (from a different context) — should not appear in Merging filter
    repo.save(
        ActivityEvent::new_task_event(task_id.clone(), ActivityEventType::Text, "exec content")
            .with_status(InternalStatus::Executing),
    )
    .await
    .unwrap();

    // Filter by Merging status — should see only 3 events
    let filter = ActivityEventFilter::new().with_statuses(vec![InternalStatus::Merging]);
    let page = repo
        .list_by_task_id(&task_id, None, 50, Some(&filter))
        .await
        .unwrap();

    assert_eq!(page.events.len(), 3, "Should have 3 Merging events");
    for event in &page.events {
        assert_eq!(event.internal_status, Some(InternalStatus::Merging));
    }
}
