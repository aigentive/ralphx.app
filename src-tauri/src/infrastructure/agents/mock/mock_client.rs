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
        self.call_history
            .write()
            .await
            .push(MockCall::new(call_type));
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

    async fn send_prompt(&self, handle: &AgentHandle, prompt: &str) -> AgentResult<AgentResponse> {
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
            call_history
                .write()
                .await
                .push(MockCall::new(MockCallType::StreamResponse {
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
#[path = "mock_client_tests.rs"]
mod tests;
