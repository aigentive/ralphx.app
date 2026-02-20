use super::*;
use crate::application::chat_service::{CachedStreamingTask, CachedToolCall};
use crate::application::AppState;
use crate::commands::ExecutionState;
use crate::domain::entities::{ChatContextType, TaskId, IdeationSessionId};
use crate::domain::services::RunningAgentKey;
use std::sync::Arc;

async fn setup_test_state() -> HttpServerState {
    let app_state = Arc::new(AppState::new_test());
    let execution_state = Arc::new(ExecutionState::new());
    let tracker = crate::application::TeamStateTracker::new();
    let team_service = Arc::new(crate::application::TeamService::new_without_events(
        Arc::new(tracker.clone()),
    ));

    HttpServerState {
        app_state,
        execution_state,
        team_tracker: tracker,
        team_service,
    }
}

#[tokio::test]
async fn test_get_active_state_returns_not_found_for_nonexistent_conversation() {
    let state = setup_test_state().await;

    let result = get_conversation_active_state(
        State(state),
        Path("nonexistent-conversation-id".to_string()),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_active_state_returns_empty_for_inactive_conversation() {
    let state = setup_test_state().await;

    // Create a conversation using the context-specific constructor
    let task_id = TaskId::new();
    let conversation = crate::domain::entities::ChatConversation::new_task(task_id.clone());
    let conversation_id = conversation.id.as_str().to_string();
    state
        .app_state
        .chat_conversation_repo
        .create(conversation)
        .await
        .unwrap();

    let response = get_conversation_active_state(State(state), Path(conversation_id.clone()))
        .await
        .unwrap();

    assert!(!response.0.is_active);
    assert!(response.0.tool_calls.is_empty());
    assert!(response.0.streaming_tasks.is_empty());
    assert!(response.0.partial_text.is_empty());
}

#[tokio::test]
async fn test_get_active_state_returns_cached_tool_calls() {
    let state = setup_test_state().await;

    // Create a conversation
    let task_id = TaskId::new();
    let conversation = crate::domain::entities::ChatConversation::new_task(task_id.clone());
    let conversation_id = conversation.id.as_str().to_string();
    state
        .app_state
        .chat_conversation_repo
        .create(conversation)
        .await
        .unwrap();

    // Add tool call to cache
    let tool_call = CachedToolCall {
        id: "toolu_001".to_string(),
        name: "bash".to_string(),
        arguments: serde_json::json!({"command": "ls -la"}),
        result: Some(serde_json::json!({"output": "file1.txt"})),
        diff_context: None,
        parent_tool_use_id: None,
    };
    state
        .app_state
        .streaming_state_cache
        .upsert_tool_call(&conversation_id, tool_call)
        .await;

    let response = get_conversation_active_state(State(state), Path(conversation_id))
        .await
        .unwrap();

    assert!(!response.0.is_active); // No agent registered
    assert_eq!(response.0.tool_calls.len(), 1);
    assert_eq!(response.0.tool_calls[0].id, "toolu_001");
    assert_eq!(response.0.tool_calls[0].name, "bash");
    assert!(response.0.tool_calls[0].result.is_some());
}

#[tokio::test]
async fn test_get_active_state_returns_cached_streaming_tasks() {
    let state = setup_test_state().await;

    // Create a conversation
    let task_id = TaskId::new();
    let conversation = crate::domain::entities::ChatConversation::new_task(task_id.clone());
    let conversation_id = conversation.id.as_str().to_string();
    state
        .app_state
        .chat_conversation_repo
        .create(conversation)
        .await
        .unwrap();

    // Add streaming task to cache
    let task = CachedStreamingTask {
        tool_use_id: "toolu_002".to_string(),
        description: Some("Running tests".to_string()),
        subagent_type: Some("ralphx:coder".to_string()),
        model: Some("sonnet".to_string()),
        status: "running".to_string(),
        teammate_name: None,
    };
    state
        .app_state
        .streaming_state_cache
        .add_task(&conversation_id, task)
        .await;

    let response = get_conversation_active_state(State(state), Path(conversation_id))
        .await
        .unwrap();

    assert_eq!(response.0.streaming_tasks.len(), 1);
    assert_eq!(response.0.streaming_tasks[0].tool_use_id, "toolu_002");
    assert_eq!(response.0.streaming_tasks[0].status, "running");
}

#[tokio::test]
async fn test_get_active_state_returns_partial_text() {
    let state = setup_test_state().await;

    // Create a conversation
    let session_id = IdeationSessionId::new();
    let conversation = crate::domain::entities::ChatConversation::new_ideation(session_id.clone());
    let conversation_id = conversation.id.as_str().to_string();
    state
        .app_state
        .chat_conversation_repo
        .create(conversation)
        .await
        .unwrap();

    // Append text to cache
    state
        .app_state
        .streaming_state_cache
        .append_text(&conversation_id, "Hello ")
        .await;
    state
        .app_state
        .streaming_state_cache
        .append_text(&conversation_id, "world!")
        .await;

    let response = get_conversation_active_state(State(state), Path(conversation_id))
        .await
        .unwrap();

    assert_eq!(response.0.partial_text, "Hello world!");
}

#[tokio::test]
async fn test_get_active_state_reflects_running_agent() {
    let state = setup_test_state().await;

    // Create a conversation
    let task_id = TaskId::new();
    let conversation = crate::domain::entities::ChatConversation::new_task(task_id.clone());
    let conversation_id = conversation.id.as_str().to_string();
    let context_id = conversation.context_id.clone();
    state
        .app_state
        .chat_conversation_repo
        .create(conversation)
        .await
        .unwrap();

    // Register a running agent
    let key = RunningAgentKey::new(ChatContextType::Task.to_string(), context_id);
    state
        .app_state
        .running_agent_registry
        .register(
            key,
            12345,
            conversation_id.clone(),
            "run-001".to_string(),
            None,
            None,
        )
        .await;

    let response = get_conversation_active_state(State(state), Path(conversation_id))
        .await
        .unwrap();

    assert!(response.0.is_active);
}

#[tokio::test]
async fn test_get_active_state_combines_all_data() {
    let state = setup_test_state().await;

    // Create a conversation
    let task_id = TaskId::new();
    let conversation = crate::domain::entities::ChatConversation::new_task_execution(task_id.clone());
    let conversation_id = conversation.id.as_str().to_string();
    let context_id = conversation.context_id.clone();
    state
        .app_state
        .chat_conversation_repo
        .create(conversation)
        .await
        .unwrap();

    // Register running agent
    let key = RunningAgentKey::new(
        ChatContextType::TaskExecution.to_string(),
        context_id,
    );
    state
        .app_state
        .running_agent_registry
        .register(
            key,
            99999,
            conversation_id.clone(),
            "run-combined".to_string(),
            None,
            None,
        )
        .await;

    // Add tool call
    let tool_call = CachedToolCall {
        id: "toolu_combined".to_string(),
        name: "read".to_string(),
        arguments: serde_json::json!({"file_path": "/tmp/test.txt"}),
        result: None,
        diff_context: None,
        parent_tool_use_id: None,
    };
    state
        .app_state
        .streaming_state_cache
        .upsert_tool_call(&conversation_id, tool_call)
        .await;

    // Add streaming task
    let task = CachedStreamingTask {
        tool_use_id: "toolu_task".to_string(),
        description: Some("Reading file".to_string()),
        subagent_type: None,
        model: None,
        status: "completed".to_string(),
        teammate_name: None,
    };
    state
        .app_state
        .streaming_state_cache
        .add_task(&conversation_id, task)
        .await;

    // Add partial text
    state
        .app_state
        .streaming_state_cache
        .append_text(&conversation_id, "Analyzing...")
        .await;

    let response = get_conversation_active_state(State(state), Path(conversation_id))
        .await
        .unwrap();

    assert!(response.0.is_active);
    assert_eq!(response.0.tool_calls.len(), 1);
    assert_eq!(response.0.streaming_tasks.len(), 1);
    assert_eq!(response.0.partial_text, "Analyzing...");
}
