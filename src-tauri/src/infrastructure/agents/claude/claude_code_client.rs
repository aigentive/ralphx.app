// Claude Code CLI client
// Production implementation using the `claude` CLI

use async_trait::async_trait;
use futures::Stream;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Stdio;
use tokio::process::Child;
use tokio::sync::Mutex;

use crate::domain::agents::{
    AgentConfig, AgentError, AgentHandle, AgentOutput, AgentResponse, AgentResult, AgenticClient,
    ClientCapabilities, ClientType, ResponseChunk,
};

lazy_static! {
    /// Global tracker for spawned child processes
    static ref PROCESSES: Mutex<HashMap<String, Child>> = Mutex::new(HashMap::new());
}

/// Client for Claude Code CLI
///
/// Uses the `claude` CLI tool to spawn and communicate with Claude agents.
pub struct ClaudeCodeClient {
    /// Path to the claude CLI
    cli_path: PathBuf,
    /// Client capabilities
    capabilities: ClientCapabilities,
}

impl ClaudeCodeClient {
    /// Create a new Claude Code client
    ///
    /// Attempts to find `claude` in PATH, falls back to "claude" if not found
    pub fn new() -> Self {
        let cli_path = which::which("claude").unwrap_or_else(|_| PathBuf::from("claude"));
        Self {
            cli_path,
            capabilities: ClientCapabilities::claude_code(),
        }
    }

    /// Create with a specific CLI path
    pub fn with_cli_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.cli_path = path.into();
        self
    }

    /// Get the CLI path
    pub fn cli_path(&self) -> &PathBuf {
        &self.cli_path
    }
}

impl Default for ClaudeCodeClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgenticClient for ClaudeCodeClient {
    async fn spawn_agent(&self, config: AgentConfig) -> AgentResult<AgentHandle> {
        // Check if CLI is available first
        if !self.cli_path.exists() && which::which(&self.cli_path).is_err() {
            return Err(AgentError::CliNotAvailable(format!(
                "claude CLI not found at {:?}",
                self.cli_path
            )));
        }

        let mut args = vec!["-p".to_string(), config.prompt.clone()];

        // Add output format for streaming
        args.extend(["--output-format".to_string(), "stream-json".to_string()]);

        // Add plugin directory for agent/skill discovery
        if let Some(plugin_dir) = &config.plugin_dir {
            args.extend(["--plugin-dir".to_string(), plugin_dir.display().to_string()]);
        }

        // Add agent name if specified
        if let Some(agent) = &config.agent {
            args.extend(["--agent".to_string(), agent.clone()]);
        }

        // Add model if specified
        if let Some(model) = &config.model {
            args.extend(["--model".to_string(), model.clone()]);
        }

        // Add max tokens if specified
        if let Some(max_tokens) = config.max_tokens {
            args.extend(["--max-tokens".to_string(), max_tokens.to_string()]);
        }

        // Add permission prompt tool for UI-based approval of non-pre-approved tools
        // The MCP tool name format: mcp__<server>__<tool>
        args.extend([
            "--permission-prompt-tool".to_string(),
            "mcp__ralphx__permission_request".to_string(),
        ]);

        // Build command
        let mut cmd = tokio::process::Command::new(&self.cli_path);
        cmd.args(&args)
            .current_dir(&config.working_directory)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        // Add environment variables
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        // Spawn the process
        let child = cmd
            .spawn()
            .map_err(|e| AgentError::SpawnFailed(e.to_string()))?;

        let handle = AgentHandle::new(ClientType::ClaudeCode, config.role);

        // Store the child process
        PROCESSES.lock().await.insert(handle.id.clone(), child);

        Ok(handle)
    }

    async fn stop_agent(&self, handle: &AgentHandle) -> AgentResult<()> {
        let mut processes = PROCESSES.lock().await;
        if let Some(mut child) = processes.remove(&handle.id) {
            child
                .kill()
                .await
                .map_err(|e| AgentError::CommunicationFailed(e.to_string()))?;
        }
        // If not found, consider it already stopped (no error)
        Ok(())
    }

    async fn wait_for_completion(&self, handle: &AgentHandle) -> AgentResult<AgentOutput> {
        let mut processes = PROCESSES.lock().await;
        let child = processes
            .remove(&handle.id)
            .ok_or_else(|| AgentError::NotFound(handle.id.clone()))?;

        let output = child
            .wait_with_output()
            .await
            .map_err(|e| AgentError::CommunicationFailed(e.to_string()))?;

        Ok(AgentOutput {
            success: output.status.success(),
            content: String::from_utf8_lossy(&output.stdout).to_string(),
            exit_code: output.status.code(),
            duration_ms: None, // TODO: Track start time for duration
        })
    }

    async fn send_prompt(
        &self,
        _handle: &AgentHandle,
        prompt: &str,
    ) -> AgentResult<AgentResponse> {
        // For send_prompt, we spawn a new one-shot agent
        let config = AgentConfig::worker(prompt);

        let handle = self.spawn_agent(config).await?;
        let output = self.wait_for_completion(&handle).await?;

        Ok(AgentResponse {
            content: output.content,
            model: Some("claude".to_string()),
            tokens_used: None,
        })
    }

    fn stream_response(
        &self,
        _handle: &AgentHandle,
        _prompt: &str,
    ) -> Pin<Box<dyn Stream<Item = AgentResult<ResponseChunk>> + Send>> {
        // TODO: Implement proper streaming
        // For now, return a simple stream with a placeholder
        let chunks = vec![
            Ok(ResponseChunk::new("Streaming not yet implemented")),
            Ok(ResponseChunk::final_chunk("")),
        ];
        Box::pin(futures::stream::iter(chunks))
    }

    fn capabilities(&self) -> &ClientCapabilities {
        &self.capabilities
    }

    async fn is_available(&self) -> AgentResult<bool> {
        // Check if the CLI exists
        if self.cli_path.exists() {
            return Ok(true);
        }

        // Try to find it in PATH
        match which::which(&self.cli_path) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::agents::AgentRole;

    #[test]
    fn test_claude_code_client_new() {
        let client = ClaudeCodeClient::new();
        // CLI might or might not exist, but client should be created
        assert_eq!(client.capabilities.client_type, ClientType::ClaudeCode);
    }

    #[test]
    fn test_claude_code_client_with_cli_path() {
        let client = ClaudeCodeClient::new().with_cli_path("/custom/path/claude");
        assert_eq!(client.cli_path, PathBuf::from("/custom/path/claude"));
    }

    #[test]
    fn test_capabilities_claude_code() {
        let client = ClaudeCodeClient::new();
        let caps = client.capabilities();
        assert_eq!(caps.client_type, ClientType::ClaudeCode);
        assert!(caps.supports_shell);
        assert!(caps.supports_filesystem);
        assert!(caps.supports_streaming);
        assert!(caps.supports_mcp);
        assert_eq!(caps.max_context_tokens, 200_000);
    }

    #[test]
    fn test_capabilities_has_models() {
        let client = ClaudeCodeClient::new();
        let caps = client.capabilities();
        assert!(caps.has_model("claude-sonnet-4-5-20250929"));
        assert!(caps.has_model("claude-opus-4-5-20251101"));
        assert!(caps.has_model("claude-haiku-4-5-20251001"));
    }

    #[test]
    fn test_cli_path_getter() {
        let client = ClaudeCodeClient::new().with_cli_path("/test/claude");
        assert_eq!(client.cli_path(), &PathBuf::from("/test/claude"));
    }

    #[test]
    fn test_default_trait() {
        let client = ClaudeCodeClient::default();
        assert_eq!(client.capabilities().client_type, ClientType::ClaudeCode);
    }

    #[tokio::test]
    async fn test_is_available_with_nonexistent_path() {
        let client =
            ClaudeCodeClient::new().with_cli_path("/nonexistent/path/to/claude_binary_12345");
        let available = client.is_available().await.unwrap();
        assert!(!available);
    }

    #[tokio::test]
    async fn test_spawn_agent_fails_with_nonexistent_cli() {
        let client =
            ClaudeCodeClient::new().with_cli_path("/nonexistent/path/to/claude_binary_12345");
        let config = AgentConfig::worker("test");

        let result = client.spawn_agent(config).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(AgentError::CliNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_stop_agent_nonexistent_handle() {
        let client = ClaudeCodeClient::new();
        let handle = AgentHandle::with_id("nonexistent", ClientType::ClaudeCode, AgentRole::Worker);

        // Should not error - just means already stopped
        let result = client.stop_agent(&handle).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wait_for_completion_nonexistent_handle() {
        let client = ClaudeCodeClient::new();
        let handle = AgentHandle::with_id("nonexistent", ClientType::ClaudeCode, AgentRole::Worker);

        let result = client.wait_for_completion(&handle).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(AgentError::NotFound(_))));
    }
}
