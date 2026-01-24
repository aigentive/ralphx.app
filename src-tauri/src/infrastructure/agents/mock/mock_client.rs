// Mock agentic client
// Test implementation that records calls and returns configurable responses

use async_trait::async_trait;
use futures::Stream;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

use crate::domain::agents::{
    AgentConfig, AgentHandle, AgentOutput, AgentResponse, AgentResult, AgenticClient,
    ClientCapabilities, ResponseChunk,
};

/// Type of call made to the mock client
#[derive(Debug, Clone)]
pub enum MockCallType {
    /// Agent was spawned
    Spawn {
        role: crate::domain::agents::AgentRole,
        prompt: String,
    },
    /// Agent was stopped
    Stop { handle_id: String },
    /// Prompt was sent to agent
    SendPrompt { handle_id: String, prompt: String },
    /// Wait for completion was called
    WaitForCompletion { handle_id: String },
    /// Stream response was called
    StreamResponse { handle_id: String, prompt: String },
}

/// A recorded call to the mock client
#[derive(Debug, Clone)]
pub struct MockCall {
    /// Type of call
    pub call_type: MockCallType,
    /// When the call was made
    pub timestamp: Instant,
}

impl MockCall {
    fn new(call_type: MockCallType) -> Self {
        Self {
            call_type,
            timestamp: Instant::now(),
        }
    }
}

/// Mock agentic client for testing
///
/// Records all calls and returns configurable responses.
/// Thread-safe and can be shared across tasks.
pub struct MockAgenticClient {
    /// Pattern -> Response mapping for prompts
    responses: Arc<RwLock<HashMap<String, String>>>,
    /// Recorded calls for assertions
    call_history: Arc<RwLock<Vec<MockCall>>>,
    /// Client capabilities
    capabilities: ClientCapabilities,
    /// Default response when no pattern matches
    default_response: String,
}

impl MockAgenticClient {
    /// Create a new mock client
    pub fn new() -> Self {
        Self {
            responses: Arc::new(RwLock::new(HashMap::new())),
            call_history: Arc::new(RwLock::new(Vec::new())),
            capabilities: ClientCapabilities::mock(),
            default_response: "MOCK_DEFAULT_RESPONSE".to_string(),
        }
    }

    /// Set response for prompts containing the given pattern
    pub async fn when_prompt_contains(&self, pattern: &str, response: &str) {
        self.responses
            .write()
            .await
            .insert(pattern.to_string(), response.to_string());
    }

    /// Set the default response when no pattern matches
    pub async fn set_default_response(&self, response: &str) {
        // Note: This requires making default_response mutable
        // For simplicity, we'll add it to the responses map with a special key
        self.responses
            .write()
            .await
            .insert("__DEFAULT__".to_string(), response.to_string());
    }

    /// Get all recorded calls
    pub async fn get_calls(&self) -> Vec<MockCall> {
        self.call_history.read().await.clone()
    }

    /// Clear call history
    pub async fn clear_calls(&self) {
        self.call_history.write().await.clear();
    }

    /// Get calls of a specific type
    pub async fn get_spawn_calls(&self) -> Vec<MockCall> {
        self.call_history
            .read()
            .await
            .iter()
            .filter(|c| matches!(c.call_type, MockCallType::Spawn { .. }))
            .cloned()
            .collect()
    }

    /// Get calls for a specific handle
    pub async fn get_calls_for_handle(&self, handle_id: &str) -> Vec<MockCall> {
        self.call_history
            .read()
            .await
            .iter()
            .filter(|c| match &c.call_type {
                MockCallType::Stop { handle_id: id } => id == handle_id,
                MockCallType::SendPrompt { handle_id: id, .. } => id == handle_id,
                MockCallType::WaitForCompletion { handle_id: id } => id == handle_id,
                MockCallType::StreamResponse { handle_id: id, .. } => id == handle_id,
                _ => false,
            })
            .cloned()
            .collect()
    }

    /// Find matching response for a prompt
    async fn find_matching_response(&self, prompt: &str) -> String {
        let responses = self.responses.read().await;
        for (pattern, response) in responses.iter() {
            if pattern != "__DEFAULT__" && prompt.contains(pattern) {
                return response.clone();
            }
        }
        // Check for custom default
        if let Some(default) = responses.get("__DEFAULT__") {
            return default.clone();
        }
        self.default_response.clone()
    }

    async fn record_call(&self, call_type: MockCallType) {
        self.call_history.write().await.push(MockCall::new(call_type));
    }
}

impl Default for MockAgenticClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgenticClient for MockAgenticClient {
    async fn spawn_agent(&self, config: AgentConfig) -> AgentResult<AgentHandle> {
        self.record_call(MockCallType::Spawn {
            role: config.role.clone(),
            prompt: config.prompt.clone(),
        })
        .await;
        Ok(AgentHandle::mock(config.role))
    }

    async fn stop_agent(&self, handle: &AgentHandle) -> AgentResult<()> {
        self.record_call(MockCallType::Stop {
            handle_id: handle.id.clone(),
        })
        .await;
        Ok(())
    }

    async fn wait_for_completion(&self, handle: &AgentHandle) -> AgentResult<AgentOutput> {
        self.record_call(MockCallType::WaitForCompletion {
            handle_id: handle.id.clone(),
        })
        .await;
        Ok(AgentOutput {
            success: true,
            content: "MOCK_COMPLETION".to_string(),
            exit_code: Some(0),
            duration_ms: Some(100),
        })
    }

    async fn send_prompt(
        &self,
        handle: &AgentHandle,
        prompt: &str,
    ) -> AgentResult<AgentResponse> {
        self.record_call(MockCallType::SendPrompt {
            handle_id: handle.id.clone(),
            prompt: prompt.to_string(),
        })
        .await;

        let response = self.find_matching_response(prompt).await;
        Ok(AgentResponse {
            content: response,
            model: Some("mock".to_string()),
            tokens_used: Some(10),
        })
    }

    fn stream_response(
        &self,
        handle: &AgentHandle,
        prompt: &str,
    ) -> Pin<Box<dyn Stream<Item = AgentResult<ResponseChunk>> + Send>> {
        // We need to record this call synchronously
        let call_history = self.call_history.clone();
        let handle_id = handle.id.clone();
        let prompt_str = prompt.to_string();

        // Spawn a task to record the call
        tokio::spawn(async move {
            call_history.write().await.push(MockCall::new(MockCallType::StreamResponse {
                handle_id,
                prompt: prompt_str,
            }));
        });

        // Return mock stream
        let chunks = vec![
            Ok(ResponseChunk::new("MOCK_")),
            Ok(ResponseChunk::final_chunk("STREAM")),
        ];
        Box::pin(futures::stream::iter(chunks))
    }

    fn capabilities(&self) -> &ClientCapabilities {
        &self.capabilities
    }

    async fn is_available(&self) -> AgentResult<bool> {
        Ok(true) // Mock is always available
    }
}

#[cfg(test)]
mod tests {
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

        let response = client.send_prompt(&handle, "something random").await.unwrap();
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
        client.when_prompt_contains("reviewer", "I'm a reviewer").await;

        let r1 = client.send_prompt(&handle, "act as worker").await.unwrap();
        assert_eq!(r1.content, "I'm a worker");

        let r2 = client.send_prompt(&handle, "act as reviewer").await.unwrap();
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
}
