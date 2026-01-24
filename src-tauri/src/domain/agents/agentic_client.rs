// Agentic client trait
// Abstraction over agentic AI clients (Claude, Codex, Gemini, etc.)

use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

use super::capabilities::ClientCapabilities;
use super::error::AgentResult;
use super::types::{AgentConfig, AgentHandle, AgentOutput, AgentResponse, ResponseChunk};

/// Abstraction over agentic AI clients (Claude, Codex, Gemini, etc.)
///
/// This trait provides a unified interface for spawning and communicating
/// with AI agents, regardless of the underlying implementation.
#[async_trait]
pub trait AgenticClient: Send + Sync {
    /// Spawn a new agent with the given configuration
    ///
    /// Returns a handle that can be used to communicate with the agent
    async fn spawn_agent(&self, config: AgentConfig) -> AgentResult<AgentHandle>;

    /// Stop a running agent
    ///
    /// Terminates the agent process and cleans up resources
    async fn stop_agent(&self, handle: &AgentHandle) -> AgentResult<()>;

    /// Wait for an agent to complete its work
    ///
    /// Blocks until the agent finishes and returns its output
    async fn wait_for_completion(&self, handle: &AgentHandle) -> AgentResult<AgentOutput>;

    /// Send a prompt to an agent and get a complete response
    ///
    /// This is a convenience method that spawns an agent, sends a prompt,
    /// waits for completion, and returns the response
    async fn send_prompt(
        &self,
        handle: &AgentHandle,
        prompt: &str,
    ) -> AgentResult<AgentResponse>;

    /// Stream responses from an agent
    ///
    /// Returns a stream of response chunks that can be processed as they arrive
    fn stream_response(
        &self,
        handle: &AgentHandle,
        prompt: &str,
    ) -> Pin<Box<dyn Stream<Item = AgentResult<ResponseChunk>> + Send>>;

    /// Get the capabilities of this client
    fn capabilities(&self) -> &ClientCapabilities;

    /// Check if this client is available (CLI installed, API key set, etc.)
    async fn is_available(&self) -> AgentResult<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::agents::types::{AgentRole, ClientType};
    use futures::StreamExt;
    use std::sync::Arc;

    // Test implementation of AgenticClient for verification
    struct TestClient {
        capabilities: ClientCapabilities,
    }

    impl TestClient {
        fn new() -> Self {
            Self {
                capabilities: ClientCapabilities::mock(),
            }
        }
    }

    #[async_trait]
    impl AgenticClient for TestClient {
        async fn spawn_agent(&self, config: AgentConfig) -> AgentResult<AgentHandle> {
            Ok(AgentHandle::mock(config.role))
        }

        async fn stop_agent(&self, _handle: &AgentHandle) -> AgentResult<()> {
            Ok(())
        }

        async fn wait_for_completion(&self, _handle: &AgentHandle) -> AgentResult<AgentOutput> {
            Ok(AgentOutput::success("completed"))
        }

        async fn send_prompt(
            &self,
            _handle: &AgentHandle,
            _prompt: &str,
        ) -> AgentResult<AgentResponse> {
            Ok(AgentResponse::new("response"))
        }

        fn stream_response(
            &self,
            _handle: &AgentHandle,
            _prompt: &str,
        ) -> Pin<Box<dyn Stream<Item = AgentResult<ResponseChunk>> + Send>> {
            let chunks = vec![
                Ok(ResponseChunk::new("chunk1")),
                Ok(ResponseChunk::final_chunk("chunk2")),
            ];
            Box::pin(futures::stream::iter(chunks))
        }

        fn capabilities(&self) -> &ClientCapabilities {
            &self.capabilities
        }

        async fn is_available(&self) -> AgentResult<bool> {
            Ok(true)
        }
    }

    #[tokio::test]
    async fn test_spawn_agent() {
        let client = TestClient::new();
        let config = AgentConfig::worker("test prompt");
        let handle = client.spawn_agent(config).await.unwrap();
        assert_eq!(handle.client_type, ClientType::Mock);
        assert_eq!(handle.role, AgentRole::Worker);
    }

    #[tokio::test]
    async fn test_stop_agent() {
        let client = TestClient::new();
        let handle = AgentHandle::mock(AgentRole::Worker);
        let result = client.stop_agent(&handle).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wait_for_completion() {
        let client = TestClient::new();
        let handle = AgentHandle::mock(AgentRole::Worker);
        let output = client.wait_for_completion(&handle).await.unwrap();
        assert!(output.success);
        assert_eq!(output.content, "completed");
    }

    #[tokio::test]
    async fn test_send_prompt() {
        let client = TestClient::new();
        let handle = AgentHandle::mock(AgentRole::Worker);
        let response = client.send_prompt(&handle, "test").await.unwrap();
        assert_eq!(response.content, "response");
    }

    #[tokio::test]
    async fn test_stream_response() {
        let client = TestClient::new();
        let handle = AgentHandle::mock(AgentRole::Worker);
        let mut stream = client.stream_response(&handle, "test");

        let chunk1 = stream.next().await.unwrap().unwrap();
        assert_eq!(chunk1.content, "chunk1");
        assert!(!chunk1.is_final);

        let chunk2 = stream.next().await.unwrap().unwrap();
        assert_eq!(chunk2.content, "chunk2");
        assert!(chunk2.is_final);
    }

    #[tokio::test]
    async fn test_capabilities() {
        let client = TestClient::new();
        let caps = client.capabilities();
        assert_eq!(caps.client_type, ClientType::Mock);
    }

    #[tokio::test]
    async fn test_is_available() {
        let client = TestClient::new();
        let available = client.is_available().await.unwrap();
        assert!(available);
    }

    #[tokio::test]
    async fn test_trait_is_object_safe() {
        // Verify the trait can be used as a trait object
        let client: Arc<dyn AgenticClient> = Arc::new(TestClient::new());
        let config = AgentConfig::worker("test");
        let result = client.spawn_agent(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_trait_send_sync() {
        // Verify the trait object is Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<TestClient>();
    }
}
