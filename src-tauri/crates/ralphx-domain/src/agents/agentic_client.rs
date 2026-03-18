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
    async fn send_prompt(&self, handle: &AgentHandle, prompt: &str) -> AgentResult<AgentResponse>;

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
#[path = "agentic_client_tests.rs"]
mod tests;
