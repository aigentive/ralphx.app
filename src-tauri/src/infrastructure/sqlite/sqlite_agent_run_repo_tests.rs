use super::*;
use crate::domain::entities::IdeationSessionId;
use crate::domain::repositories::ChatConversationRepository;
use crate::infrastructure::sqlite::{
    open_memory_connection, run_migrations, SqliteChatConversationRepository,
};

#[tokio::test]
async fn test_get_interrupted_conversations_returns_empty_when_none() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    let repo = SqliteAgentRunRepository::new(conn);

    let result = repo.get_interrupted_conversations().await.unwrap();
    assert!(result.is_empty());
}

#[tokio::test]
async fn test_get_interrupted_conversations_returns_orphaned_conversation() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    let shared_conn = Arc::new(Mutex::new(conn));

    let agent_run_repo = SqliteAgentRunRepository::from_shared(Arc::clone(&shared_conn));
    let conversation_repo =
        SqliteChatConversationRepository::from_shared(Arc::clone(&shared_conn));

    // Create a conversation with claude_session_id
    let mut conversation = ChatConversation::new_ideation(IdeationSessionId::new());
    conversation.claude_session_id = Some("test-session-id".to_string());
    conversation_repo
        .create(conversation.clone())
        .await
        .unwrap();

    // Create an agent run that gets orphaned
    let mut run = AgentRun::new(conversation.id);
    let run_id = run.id;
    run.status = AgentRunStatus::Cancelled;
    run.completed_at = Some(Utc::now());
    run.error_message = Some("Orphaned on app restart".to_string());
    agent_run_repo.create(run).await.unwrap();

    // Get interrupted conversations
    let result = agent_run_repo
        .get_interrupted_conversations()
        .await
        .unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].conversation.id, conversation.id);
    assert_eq!(result[0].last_run.id, run_id);
    assert_eq!(result[0].last_run.status, AgentRunStatus::Cancelled);
    assert_eq!(
        result[0].last_run.error_message,
        Some("Orphaned on app restart".to_string())
    );
}

#[tokio::test]
async fn test_get_interrupted_conversations_ignores_without_session_id() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    let shared_conn = Arc::new(Mutex::new(conn));

    let agent_run_repo = SqliteAgentRunRepository::from_shared(Arc::clone(&shared_conn));
    let conversation_repo =
        SqliteChatConversationRepository::from_shared(Arc::clone(&shared_conn));

    // Create a conversation WITHOUT claude_session_id
    let conversation = ChatConversation::new_ideation(IdeationSessionId::new());
    // Note: claude_session_id is None by default
    conversation_repo
        .create(conversation.clone())
        .await
        .unwrap();

    // Create an orphaned agent run
    let mut run = AgentRun::new(conversation.id);
    run.status = AgentRunStatus::Cancelled;
    run.completed_at = Some(Utc::now());
    run.error_message = Some("Orphaned on app restart".to_string());
    agent_run_repo.create(run).await.unwrap();

    // Should return empty because conversation has no claude_session_id
    let result = agent_run_repo
        .get_interrupted_conversations()
        .await
        .unwrap();
    assert!(result.is_empty());
}

#[tokio::test]
async fn test_get_interrupted_conversations_ignores_completed_runs() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    let shared_conn = Arc::new(Mutex::new(conn));

    let agent_run_repo = SqliteAgentRunRepository::from_shared(Arc::clone(&shared_conn));
    let conversation_repo =
        SqliteChatConversationRepository::from_shared(Arc::clone(&shared_conn));

    // Create a conversation with claude_session_id
    let mut conversation = ChatConversation::new_ideation(IdeationSessionId::new());
    conversation.claude_session_id = Some("test-session-id".to_string());
    conversation_repo
        .create(conversation.clone())
        .await
        .unwrap();

    // Create a COMPLETED agent run (not orphaned)
    let mut run = AgentRun::new(conversation.id);
    run.status = AgentRunStatus::Completed;
    run.completed_at = Some(Utc::now());
    agent_run_repo.create(run).await.unwrap();

    // Should return empty because run is completed, not orphaned
    let result = agent_run_repo
        .get_interrupted_conversations()
        .await
        .unwrap();
    assert!(result.is_empty());
}

#[tokio::test]
async fn test_get_interrupted_conversations_ignores_different_error_message() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    let shared_conn = Arc::new(Mutex::new(conn));

    let agent_run_repo = SqliteAgentRunRepository::from_shared(Arc::clone(&shared_conn));
    let conversation_repo =
        SqliteChatConversationRepository::from_shared(Arc::clone(&shared_conn));

    // Create a conversation with claude_session_id
    let mut conversation = ChatConversation::new_ideation(IdeationSessionId::new());
    conversation.claude_session_id = Some("test-session-id".to_string());
    conversation_repo
        .create(conversation.clone())
        .await
        .unwrap();

    // Create a cancelled run with DIFFERENT error message
    let mut run = AgentRun::new(conversation.id);
    run.status = AgentRunStatus::Cancelled;
    run.completed_at = Some(Utc::now());
    run.error_message = Some("User cancelled".to_string());
    agent_run_repo.create(run).await.unwrap();

    // Should return empty because error message doesn't match
    let result = agent_run_repo
        .get_interrupted_conversations()
        .await
        .unwrap();
    assert!(result.is_empty());
}

#[tokio::test]
async fn test_get_interrupted_conversations_only_latest_run() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    let shared_conn = Arc::new(Mutex::new(conn));

    let agent_run_repo = SqliteAgentRunRepository::from_shared(Arc::clone(&shared_conn));
    let conversation_repo =
        SqliteChatConversationRepository::from_shared(Arc::clone(&shared_conn));

    // Create a conversation with claude_session_id
    let mut conversation = ChatConversation::new_ideation(IdeationSessionId::new());
    conversation.claude_session_id = Some("test-session-id".to_string());
    conversation_repo
        .create(conversation.clone())
        .await
        .unwrap();

    // Create an OLD orphaned run
    let mut old_run = AgentRun::new(conversation.id);
    old_run.status = AgentRunStatus::Cancelled;
    old_run.started_at = Utc::now() - chrono::Duration::hours(1);
    old_run.completed_at = Some(Utc::now() - chrono::Duration::hours(1));
    old_run.error_message = Some("Orphaned on app restart".to_string());
    agent_run_repo.create(old_run).await.unwrap();

    // Create a NEW completed run (the latest one)
    let mut new_run = AgentRun::new(conversation.id);
    new_run.status = AgentRunStatus::Completed;
    new_run.started_at = Utc::now();
    new_run.completed_at = Some(Utc::now());
    agent_run_repo.create(new_run).await.unwrap();

    // Should return empty because the LATEST run is completed, not orphaned
    let result = agent_run_repo
        .get_interrupted_conversations()
        .await
        .unwrap();
    assert!(result.is_empty());
}
