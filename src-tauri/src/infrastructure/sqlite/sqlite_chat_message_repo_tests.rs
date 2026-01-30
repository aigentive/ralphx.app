// Tests for SqliteChatMessageRepository

use super::sqlite_chat_message_repo::SqliteChatMessageRepository;
use crate::domain::entities::{ChatMessage, ChatMessageId, IdeationSession, IdeationSessionId, ProjectId, TaskId};
use crate::domain::repositories::ChatMessageRepository;
use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};
use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    fn create_test_project(conn: &Connection, id: &ProjectId, name: &str, path: &str) {
        conn.execute(
            "INSERT INTO projects (id, name, working_directory, git_mode, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'single_branch', datetime('now'), datetime('now'))",
            rusqlite::params![id.as_str(), name, path],
        )
        .unwrap();
    }

    fn create_test_session(conn: &Connection, project_id: &ProjectId) -> IdeationSessionId {
        let session = IdeationSession::builder()
            .project_id(project_id.clone())
            .title("Test Session")
            .build();

        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, title, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'active', datetime('now'), datetime('now'))",
            rusqlite::params![
                session.id.as_str(),
                project_id.as_str(),
                "Test Session"
            ],
        )
        .unwrap();

        session.id
    }

    fn create_test_task(conn: &Connection, project_id: &ProjectId) -> TaskId {
        let task_id = TaskId::new();
        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title, description, internal_status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'), datetime('now'))",
            rusqlite::params![
                task_id.as_str(),
                project_id.as_str(),
                "feature",
                "Test Task",
                "",
                "backlog",
            ],
        )
        .unwrap();
        task_id
    }

    // ==================== CREATE TESTS ====================

    #[tokio::test]
    async fn test_create_inserts_message_and_returns_it() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);
        let message = ChatMessage::user_in_session(session_id.clone(), "Hello, world!");

        let result = repo.create(message.clone()).await;

        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.id, message.id);
        assert_eq!(created.content, "Hello, world!");
        assert_eq!(created.session_id, Some(session_id));
    }

    #[tokio::test]
    async fn test_create_message_with_metadata() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);
        let message = ChatMessage::user_in_session(session_id, "With metadata")
            .with_metadata(r#"{"key": "value"}"#);

        let result = repo.create(message.clone()).await;

        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.metadata, Some(r#"{"key": "value"}"#.to_string()));
    }

    #[tokio::test]
    async fn test_create_message_with_parent() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        // Create parent message
        let parent = ChatMessage::user_in_session(session_id.clone(), "Parent message");
        repo.create(parent.clone()).await.unwrap();

        // Create child message
        let child = ChatMessage::orchestrator_in_session(session_id, "Reply to parent")
            .with_parent(parent.id.clone());
        let result = repo.create(child.clone()).await;

        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.parent_message_id, Some(parent.id));
    }

    #[tokio::test]
    async fn test_create_duplicate_id_fails() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);
        let message = ChatMessage::user_in_session(session_id, "Duplicate");

        repo.create(message.clone()).await.unwrap();
        let result = repo.create(message).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_project_message() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let repo = SqliteChatMessageRepository::new(conn);
        let message = ChatMessage::user_in_project(project_id.clone(), "Project-level chat");

        let result = repo.create(message.clone()).await;

        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.project_id, Some(project_id));
        assert!(created.session_id.is_none());
    }

    #[tokio::test]
    async fn test_create_task_message() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let task_id = create_test_task(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);
        let message = ChatMessage::user_about_task(task_id.clone(), "Task-specific chat");

        let result = repo.create(message.clone()).await;

        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.task_id, Some(task_id));
    }

    // ==================== GET BY ID TESTS ====================

    #[tokio::test]
    async fn test_get_by_id_retrieves_message_correctly() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);
        let message = ChatMessage::user_in_session(session_id.clone(), "Find me");

        repo.create(message.clone()).await.unwrap();
        let result = repo.get_by_id(&message.id).await;

        assert!(result.is_ok());
        let found = result.unwrap();
        assert!(found.is_some());
        let found_message = found.unwrap();
        assert_eq!(found_message.id, message.id);
        assert_eq!(found_message.content, "Find me");
        assert_eq!(found_message.session_id, Some(session_id));
    }

    #[tokio::test]
    async fn test_get_by_id_returns_none_for_nonexistent() {
        let conn = setup_test_db();
        let repo = SqliteChatMessageRepository::new(conn);
        let id = ChatMessageId::new();

        let result = repo.get_by_id(&id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_by_id_preserves_all_fields() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);
        let message = ChatMessage::orchestrator_in_session(session_id.clone(), "Full message")
            .with_metadata(r#"{"context": "test"}"#);

        repo.create(message.clone()).await.unwrap();
        let found = repo.get_by_id(&message.id).await.unwrap().unwrap();

        assert_eq!(found.id, message.id);
        assert_eq!(found.session_id, Some(session_id));
        assert_eq!(found.content, "Full message");
        assert_eq!(found.metadata, Some(r#"{"context": "test"}"#.to_string()));
        assert!(found.is_orchestrator());
    }

    // ==================== GET BY SESSION TESTS ====================

    #[tokio::test]
    async fn test_get_by_session_returns_all_messages() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_in_session(session_id.clone(), "First");
        let msg2 = ChatMessage::orchestrator_in_session(session_id.clone(), "Second");
        let msg3 = ChatMessage::user_in_session(session_id.clone(), "Third");

        repo.create(msg1).await.unwrap();
        repo.create(msg2).await.unwrap();
        repo.create(msg3).await.unwrap();

        let result = repo.get_by_session(&session_id).await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 3);
    }

    #[tokio::test]
    async fn test_get_by_session_ordered_by_created_at_asc() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        // Create messages with slight delays to ensure different timestamps
        let msg1 = ChatMessage::user_in_session(session_id.clone(), "First");
        repo.create(msg1.clone()).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let msg2 = ChatMessage::orchestrator_in_session(session_id.clone(), "Second");
        repo.create(msg2.clone()).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let msg3 = ChatMessage::user_in_session(session_id.clone(), "Third");
        repo.create(msg3.clone()).await.unwrap();

        let messages = repo.get_by_session(&session_id).await.unwrap();

        // Should be in ascending order (oldest first)
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].content, "First");
        assert_eq!(messages[1].content, "Second");
        assert_eq!(messages[2].content, "Third");
    }

    #[tokio::test]
    async fn test_get_by_session_returns_empty_for_no_messages() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let result = repo.get_by_session(&session_id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_by_session_filters_by_session() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id1 = create_test_session(&conn, &project_id);
        let session_id2 = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_in_session(session_id1.clone(), "Session 1 message");
        let msg2 = ChatMessage::user_in_session(session_id2.clone(), "Session 2 message");

        repo.create(msg1).await.unwrap();
        repo.create(msg2).await.unwrap();

        let messages = repo.get_by_session(&session_id1).await.unwrap();

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].session_id, Some(session_id1));
    }

    // ==================== GET BY PROJECT TESTS ====================

    #[tokio::test]
    async fn test_get_by_project_returns_project_messages_only() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        // Create a project message (no session)
        let project_msg = ChatMessage::user_in_project(project_id.clone(), "Project chat");
        // Create a session message
        let session_msg = ChatMessage::user_in_session(session_id.clone(), "Session chat");

        repo.create(project_msg).await.unwrap();
        repo.create(session_msg).await.unwrap();

        let result = repo.get_by_project(&project_id).await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        // Should only return the project message, not the session message
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Project chat");
        assert!(messages[0].session_id.is_none());
    }

    #[tokio::test]
    async fn test_get_by_project_returns_empty_for_no_messages() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let repo = SqliteChatMessageRepository::new(conn);

        let result = repo.get_by_project(&project_id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_by_project_filters_by_project() {
        let conn = setup_test_db();
        let project_id1 = ProjectId::new();
        let project_id2 = ProjectId::new();
        create_test_project(&conn, &project_id1, "Project 1", "/path1");
        create_test_project(&conn, &project_id2, "Project 2", "/path2");

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_in_project(project_id1.clone(), "P1 message");
        let msg2 = ChatMessage::user_in_project(project_id2.clone(), "P2 message");

        repo.create(msg1).await.unwrap();
        repo.create(msg2).await.unwrap();

        let messages = repo.get_by_project(&project_id1).await.unwrap();

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].project_id, Some(project_id1));
    }

    // ==================== GET BY TASK TESTS ====================

    #[tokio::test]
    async fn test_get_by_task_returns_task_messages() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let task_id = create_test_task(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_about_task(task_id.clone(), "Task question");
        let msg2 = ChatMessage::user_about_task(task_id.clone(), "Follow-up");

        repo.create(msg1).await.unwrap();
        repo.create(msg2).await.unwrap();

        let result = repo.get_by_task(&task_id).await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 2);
    }

    #[tokio::test]
    async fn test_get_by_task_returns_empty_for_no_messages() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let task_id = create_test_task(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let result = repo.get_by_task(&task_id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_by_task_filters_by_task() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let task_id1 = create_test_task(&conn, &project_id);
        let task_id2 = create_test_task(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_about_task(task_id1.clone(), "Task 1 msg");
        let msg2 = ChatMessage::user_about_task(task_id2.clone(), "Task 2 msg");

        repo.create(msg1).await.unwrap();
        repo.create(msg2).await.unwrap();

        let messages = repo.get_by_task(&task_id1).await.unwrap();

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].task_id, Some(task_id1));
    }

    // ==================== DELETE BY SESSION TESTS ====================

    #[tokio::test]
    async fn test_delete_by_session_removes_all_session_messages() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_in_session(session_id.clone(), "Message 1");
        let msg2 = ChatMessage::orchestrator_in_session(session_id.clone(), "Message 2");

        repo.create(msg1).await.unwrap();
        repo.create(msg2).await.unwrap();

        let delete_result = repo.delete_by_session(&session_id).await;
        assert!(delete_result.is_ok());

        let messages = repo.get_by_session(&session_id).await.unwrap();
        assert!(messages.is_empty());
    }

    #[tokio::test]
    async fn test_delete_by_session_does_not_affect_other_sessions() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id1 = create_test_session(&conn, &project_id);
        let session_id2 = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_in_session(session_id1.clone(), "Session 1");
        let msg2 = ChatMessage::user_in_session(session_id2.clone(), "Session 2");

        repo.create(msg1).await.unwrap();
        repo.create(msg2).await.unwrap();

        repo.delete_by_session(&session_id1).await.unwrap();

        let session1_messages = repo.get_by_session(&session_id1).await.unwrap();
        let session2_messages = repo.get_by_session(&session_id2).await.unwrap();

        assert!(session1_messages.is_empty());
        assert_eq!(session2_messages.len(), 1);
    }

    #[tokio::test]
    async fn test_delete_by_session_nonexistent_succeeds() {
        let conn = setup_test_db();
        let repo = SqliteChatMessageRepository::new(conn);
        let session_id = IdeationSessionId::new();

        let result = repo.delete_by_session(&session_id).await;
        assert!(result.is_ok());
    }

    // ==================== DELETE BY PROJECT TESTS ====================

    #[tokio::test]
    async fn test_delete_by_project_removes_all_project_messages() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_in_project(project_id.clone(), "Message 1");
        let msg2 = ChatMessage::user_in_project(project_id.clone(), "Message 2");

        repo.create(msg1).await.unwrap();
        repo.create(msg2).await.unwrap();

        let delete_result = repo.delete_by_project(&project_id).await;
        assert!(delete_result.is_ok());

        let messages = repo.get_by_project(&project_id).await.unwrap();
        assert!(messages.is_empty());
    }

    // ==================== DELETE BY TASK TESTS ====================

    #[tokio::test]
    async fn test_delete_by_task_removes_all_task_messages() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let task_id = create_test_task(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_about_task(task_id.clone(), "Message 1");
        let msg2 = ChatMessage::user_about_task(task_id.clone(), "Message 2");

        repo.create(msg1).await.unwrap();
        repo.create(msg2).await.unwrap();

        let delete_result = repo.delete_by_task(&task_id).await;
        assert!(delete_result.is_ok());

        let messages = repo.get_by_task(&task_id).await.unwrap();
        assert!(messages.is_empty());
    }

    // ==================== DELETE SINGLE MESSAGE TESTS ====================

    #[tokio::test]
    async fn test_delete_removes_single_message() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_in_session(session_id.clone(), "Keep");
        let msg2 = ChatMessage::user_in_session(session_id.clone(), "Delete");

        repo.create(msg1.clone()).await.unwrap();
        repo.create(msg2.clone()).await.unwrap();

        let delete_result = repo.delete(&msg2.id).await;
        assert!(delete_result.is_ok());

        let found = repo.get_by_id(&msg2.id).await.unwrap();
        assert!(found.is_none());

        // Other message should still exist
        let kept = repo.get_by_id(&msg1.id).await.unwrap();
        assert!(kept.is_some());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_succeeds() {
        let conn = setup_test_db();
        let repo = SqliteChatMessageRepository::new(conn);
        let id = ChatMessageId::new();

        let result = repo.delete(&id).await;
        assert!(result.is_ok());
    }

    // ==================== COUNT BY SESSION TESTS ====================

    #[tokio::test]
    async fn test_count_by_session_returns_zero_for_no_messages() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let result = repo.count_by_session(&session_id).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_count_by_session_counts_correctly() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_in_session(session_id.clone(), "One");
        let msg2 = ChatMessage::orchestrator_in_session(session_id.clone(), "Two");
        let msg3 = ChatMessage::user_in_session(session_id.clone(), "Three");

        repo.create(msg1).await.unwrap();
        repo.create(msg2).await.unwrap();
        repo.create(msg3).await.unwrap();

        let result = repo.count_by_session(&session_id).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_count_by_session_filters_by_session() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id1 = create_test_session(&conn, &project_id);
        let session_id2 = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_in_session(session_id1.clone(), "S1-1");
        let msg2 = ChatMessage::user_in_session(session_id1.clone(), "S1-2");
        let msg3 = ChatMessage::user_in_session(session_id2.clone(), "S2-1");

        repo.create(msg1).await.unwrap();
        repo.create(msg2).await.unwrap();
        repo.create(msg3).await.unwrap();

        let count1 = repo.count_by_session(&session_id1).await.unwrap();
        let count2 = repo.count_by_session(&session_id2).await.unwrap();

        assert_eq!(count1, 2);
        assert_eq!(count2, 1);
    }

    // ==================== GET RECENT BY SESSION TESTS ====================

    #[tokio::test]
    async fn test_get_recent_by_session_returns_limited_messages() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        // Create 5 messages
        for i in 1..=5 {
            let msg = ChatMessage::user_in_session(session_id.clone(), format!("Message {}", i));
            repo.create(msg).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        let result = repo.get_recent_by_session(&session_id, 3).await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 3);
    }

    #[tokio::test]
    async fn test_get_recent_by_session_returns_most_recent() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        // Create messages with delays
        for i in 1..=5 {
            let msg = ChatMessage::user_in_session(session_id.clone(), format!("Message {}", i));
            repo.create(msg).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        let messages = repo.get_recent_by_session(&session_id, 2).await.unwrap();

        // Should return the last 2 messages in ascending order
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content, "Message 4");
        assert_eq!(messages[1].content, "Message 5");
    }

    #[tokio::test]
    async fn test_get_recent_by_session_returns_all_if_fewer_than_limit() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_in_session(session_id.clone(), "Only one");
        repo.create(msg1).await.unwrap();

        let messages = repo.get_recent_by_session(&session_id, 10).await.unwrap();

        assert_eq!(messages.len(), 1);
    }

    #[tokio::test]
    async fn test_get_recent_by_session_returns_in_ascending_order() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_in_session(session_id.clone(), "First");
        repo.create(msg1).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let msg2 = ChatMessage::orchestrator_in_session(session_id.clone(), "Second");
        repo.create(msg2).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let msg3 = ChatMessage::user_in_session(session_id.clone(), "Third");
        repo.create(msg3).await.unwrap();

        let messages = repo.get_recent_by_session(&session_id, 3).await.unwrap();

        // Should be in ascending order (oldest to newest)
        assert_eq!(messages[0].content, "First");
        assert_eq!(messages[1].content, "Second");
        assert_eq!(messages[2].content, "Third");
    }

    // ==================== SHARED CONNECTION TESTS ====================

    #[tokio::test]
    async fn test_from_shared_works_correctly() {
        use std::sync::Arc;
        use tokio::sync::Mutex;

        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let shared_conn = Arc::new(Mutex::new(conn));
        let repo = SqliteChatMessageRepository::from_shared(shared_conn);

        let message = ChatMessage::user_in_session(session_id, "Shared connection test");

        let result = repo.create(message.clone()).await;
        assert!(result.is_ok());

        let found = repo.get_by_id(&message.id).await.unwrap();
        assert!(found.is_some());
    }

    // ==================== ROLE TESTS ====================

    #[tokio::test]
    async fn test_message_roles_are_preserved() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let user_msg = ChatMessage::user_in_session(session_id.clone(), "User");
        let orch_msg = ChatMessage::orchestrator_in_session(session_id.clone(), "Orchestrator");
        let sys_msg = ChatMessage::system_in_session(session_id.clone(), "System");

        repo.create(user_msg.clone()).await.unwrap();
        repo.create(orch_msg.clone()).await.unwrap();
        repo.create(sys_msg.clone()).await.unwrap();

        let found_user = repo.get_by_id(&user_msg.id).await.unwrap().unwrap();
        let found_orch = repo.get_by_id(&orch_msg.id).await.unwrap().unwrap();
        let found_sys = repo.get_by_id(&sys_msg.id).await.unwrap().unwrap();

        assert!(found_user.is_user());
        assert!(found_orch.is_orchestrator());
        assert!(found_sys.is_system());
    }

    // ==================== CASCADE DELETE TESTS ====================

    #[tokio::test]
    async fn test_cascade_delete_when_session_deleted() {
        use std::sync::Arc;
        use tokio::sync::Mutex;

        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let shared_conn = Arc::new(Mutex::new(conn));
        let repo = SqliteChatMessageRepository::from_shared(shared_conn.clone());

        let msg = ChatMessage::user_in_session(session_id.clone(), "Will be cascaded");
        repo.create(msg.clone()).await.unwrap();

        // Delete the session directly using the shared connection
        {
            let conn = shared_conn.lock().await;
            conn.execute(
                "DELETE FROM ideation_sessions WHERE id = ?1",
                [session_id.as_str()],
            )
            .unwrap();
        }

        // Message should be gone due to CASCADE
        let found = repo.get_by_id(&msg.id).await.unwrap();
        assert!(found.is_none());
    }
