use super::*;
use crate::domain::agents::{AgentRole, ClientType};
use futures::StreamExt;

#[tokio::test]
async fn test_mock_client_new() {
    let client = MockAgenticClient::new();
    assert!(client.is_available().await.unwrap());
}

#[tokio::test]
async fn test_spawn_agent_records_call() {
    let client = MockAgenticClient::new();
    let config = AgentConfig::worker("test prompt");

    let handle = client.spawn_agent(config).await.unwrap();

    assert_eq!(handle.client_type, ClientType::Mock);
    assert_eq!(handle.role, AgentRole::Worker);

    let calls = client.get_spawn_calls().await;
    assert_eq!(calls.len(), 1);
    if let MockCallType::Spawn { role, prompt } = &calls[0].call_type {
        assert_eq!(*role, AgentRole::Worker);
        assert_eq!(prompt, "test prompt");
    } else {
        panic!("Expected Spawn call");
    }
}

#[tokio::test]
async fn test_when_prompt_contains_sets_response() {
    let client = MockAgenticClient::new();
    let handle = AgentHandle::mock(AgentRole::Worker);

    client.when_prompt_contains("hello", "world").await;

    let response = client.send_prompt(&handle, "say hello").await.unwrap();
    assert_eq!(response.content, "world");
}

#[tokio::test]
async fn test_default_response_when_no_match() {
    let client = MockAgenticClient::new();
    let handle = AgentHandle::mock(AgentRole::Worker);

    let response = client
        .send_prompt(&handle, "something random")
        .await
        .unwrap();
    assert_eq!(response.content, "MOCK_DEFAULT_RESPONSE");
}

#[tokio::test]
async fn test_custom_default_response() {
    let client = MockAgenticClient::new();
    let handle = AgentHandle::mock(AgentRole::Worker);

    client.set_default_response("CUSTOM_DEFAULT").await;

    let response = client.send_prompt(&handle, "no match").await.unwrap();
    assert_eq!(response.content, "CUSTOM_DEFAULT");
}

#[tokio::test]
async fn test_stop_agent_records_call() {
    let client = MockAgenticClient::new();
    let handle = AgentHandle::with_id("test-id", ClientType::Mock, AgentRole::Worker);

    client.stop_agent(&handle).await.unwrap();

    let calls = client.get_calls_for_handle("test-id").await;
    assert_eq!(calls.len(), 1);
    assert!(matches!(calls[0].call_type, MockCallType::Stop { .. }));
}

#[tokio::test]
async fn test_wait_for_completion_returns_success() {
    let client = MockAgenticClient::new();
    let handle = AgentHandle::mock(AgentRole::Worker);

    let output = client.wait_for_completion(&handle).await.unwrap();

    assert!(output.success);
    assert_eq!(output.content, "MOCK_COMPLETION");
    assert_eq!(output.exit_code, Some(0));
}

#[tokio::test]
async fn test_send_prompt_records_call() {
    let client = MockAgenticClient::new();
    let handle = AgentHandle::with_id("handle-1", ClientType::Mock, AgentRole::Worker);

    client.send_prompt(&handle, "test prompt").await.unwrap();

    let calls = client.get_calls_for_handle("handle-1").await;
    assert_eq!(calls.len(), 1);
    if let MockCallType::SendPrompt { handle_id, prompt } = &calls[0].call_type {
        assert_eq!(handle_id, "handle-1");
        assert_eq!(prompt, "test prompt");
    } else {
        panic!("Expected SendPrompt call");
    }
}

#[tokio::test]
async fn test_stream_response_returns_chunks() {
    let client = MockAgenticClient::new();
    let handle = AgentHandle::mock(AgentRole::Worker);

    let mut stream = client.stream_response(&handle, "test");

    let chunk1 = stream.next().await.unwrap().unwrap();
    assert_eq!(chunk1.content, "MOCK_");
    assert!(!chunk1.is_final);

    let chunk2 = stream.next().await.unwrap().unwrap();
    assert_eq!(chunk2.content, "STREAM");
    assert!(chunk2.is_final);
}

#[tokio::test]
async fn test_capabilities_returns_mock() {
    let client = MockAgenticClient::new();
    let caps = client.capabilities();
    assert_eq!(caps.client_type, ClientType::Mock);
    assert!(caps.supports_shell);
    assert!(!caps.supports_mcp);
}

#[tokio::test]
async fn test_clear_calls() {
    let client = MockAgenticClient::new();
    let config = AgentConfig::worker("test");

    client.spawn_agent(config).await.unwrap();
    assert_eq!(client.get_calls().await.len(), 1);

    client.clear_calls().await;
    assert_eq!(client.get_calls().await.len(), 0);
}

#[tokio::test]
async fn test_multiple_responses() {
    let client = MockAgenticClient::new();
    let handle = AgentHandle::mock(AgentRole::Worker);

    client.when_prompt_contains("worker", "I'm a worker").await;
    client
        .when_prompt_contains("reviewer", "I'm a reviewer")
        .await;

    let r1 = client.send_prompt(&handle, "act as worker").await.unwrap();
    assert_eq!(r1.content, "I'm a worker");

    let r2 = client
        .send_prompt(&handle, "act as reviewer")
        .await
        .unwrap();
    assert_eq!(r2.content, "I'm a reviewer");
}

#[tokio::test]
async fn test_response_includes_model_info() {
    let client = MockAgenticClient::new();
    let handle = AgentHandle::mock(AgentRole::Worker);

    let response = client.send_prompt(&handle, "test").await.unwrap();

    assert_eq!(response.model, Some("mock".to_string()));
    assert_eq!(response.tokens_used, Some(10));
}

#[tokio::test]
async fn test_default_trait() {
    let client = MockAgenticClient::default();
    assert!(client.is_available().await.unwrap());
}
