// Claude Code CLI client
// Production implementation using the `claude` CLI
//
// This client supports two modes of operation:
// 1. Simple spawn-and-wait: Use spawn_agent() + wait_for_completion()
// 2. Streaming with persistence: Use spawn_agent_streaming() to get the Child process
//    and handle stream processing externally (used by ExecutionChatService)

use async_trait::async_trait;
use futures::Stream;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Stdio;
use std::time::Instant;
use tokio::process::Child;
use tokio::sync::Mutex;

use crate::domain::agents::{
    AgentConfig, AgentError, AgentHandle, AgentOutput, AgentResponse, AgentResult, AgenticClient,
    ClientCapabilities, ClientType, ResponseChunk,
};

use super::{ensure_claude_spawn_allowed, get_allowed_mcp_tools, get_allowed_tools};

// ============================================================================
// Streaming Event Types
// ============================================================================

/// Events emitted during agent stream processing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    /// Text chunk received from agent
    TextChunk {
        text: String,
    },
    /// Tool call started
    ToolCallStart {
        tool_name: String,
        tool_id: Option<String>,
    },
    /// Tool call input (incremental JSON)
    ToolCallInput {
        tool_name: String,
        tool_id: Option<String>,
        partial_json: String,
    },
    /// Tool call completed
    ToolCallComplete {
        tool_name: String,
        tool_id: Option<String>,
        arguments: serde_json::Value,
    },
    /// Agent execution completed with session ID
    Completed {
        session_id: Option<String>,
    },
    /// Error occurred during execution
    Error {
        message: String,
    },
}

/// Result from spawning an agent in streaming mode
#[derive(Debug)]
pub struct StreamingSpawnResult {
    /// Handle to the spawned agent
    pub handle: AgentHandle,
    /// The spawned child process (stdout is piped for stream processing)
    pub child: Child,
}

lazy_static! {
    /// Global tracker for spawned child processes with their start time
    static ref PROCESSES: Mutex<HashMap<String, (Child, Instant)>> = Mutex::new(HashMap::new());
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
        if let Err(err) = ensure_claude_spawn_allowed() {
            return Err(AgentError::SpawnNotAllowed(err));
        }

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
        args.push("--verbose".to_string()); // Required for stream-json with -p

        // Add plugin directory for agent/skill discovery
        if let Some(plugin_dir) = &config.plugin_dir {
            args.extend(["--plugin-dir".to_string(), plugin_dir.display().to_string()]);
        }

        // Add agent name if specified
        if let Some(agent) = &config.agent {
            args.extend(["--agent".to_string(), agent.clone()]);
        }

        // Apply CLI tool restrictions from agent_config
        if let Some(agent_name) = &config.agent {
            if let Some(allowed_tools) = get_allowed_tools(agent_name) {
                args.extend(["--tools".to_string(), allowed_tools.to_string()]);
            }
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

        // Pre-approve agent-specific MCP tools (no permission prompts)
        if let Some(agent) = &config.agent {
            if let Some(mcp_tools) = get_allowed_mcp_tools(agent) {
                args.push("--allowedTools".to_string());
                args.push(mcp_tools);
            }
        }

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

        // Spawn the process and record start time for duration tracking
        let start_time = Instant::now();
        let child = cmd
            .spawn()
            .map_err(|e| AgentError::SpawnFailed(e.to_string()))?;

        let handle = AgentHandle::new(ClientType::ClaudeCode, config.role);

        // Store the child process with its start time
        PROCESSES
            .lock()
            .await
            .insert(handle.id.clone(), (child, start_time));

        Ok(handle)
    }

    async fn stop_agent(&self, handle: &AgentHandle) -> AgentResult<()> {
        let mut processes = PROCESSES.lock().await;
        if let Some((mut child, _start_time)) = processes.remove(&handle.id) {
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
        let (child, start_time) = processes
            .remove(&handle.id)
            .ok_or_else(|| AgentError::NotFound(handle.id.clone()))?;

        let output = child
            .wait_with_output()
            .await
            .map_err(|e| AgentError::CommunicationFailed(e.to_string()))?;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        Ok(AgentOutput {
            success: output.status.success(),
            content: String::from_utf8_lossy(&output.stdout).to_string(),
            exit_code: output.status.code(),
            duration_ms: Some(duration_ms),
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
        // Note: Production streaming uses spawn_agent_streaming() which returns Child process
        // for external stream handling (see ExecutionChatService). This trait method is a
        // placeholder for potential future trait-level streaming support.
        let chunks = vec![
            Ok(ResponseChunk::new("Use spawn_agent_streaming() for production streaming")),
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

// ============================================================================
// Streaming Spawn Support
// ============================================================================

impl ClaudeCodeClient {
    /// Build CLI arguments from an AgentConfig
    ///
    /// This is used by both spawn_agent and spawn_agent_streaming to ensure
    /// consistent argument construction.
    fn build_cli_args(&self, config: &AgentConfig, resume_session_id: Option<&str>) -> Vec<String> {
        let mut args = Vec::new();

        // Prompt
        args.extend(["-p".to_string(), config.prompt.clone()]);

        // Output format for streaming
        args.extend(["--output-format".to_string(), "stream-json".to_string()]);
        args.push("--verbose".to_string()); // Required for stream-json with -p

        // Plugin directory for agent/skill discovery
        if let Some(plugin_dir) = &config.plugin_dir {
            args.extend(["--plugin-dir".to_string(), plugin_dir.display().to_string()]);
        }

        // Resume session - always include agent to enforce tool restrictions
        if let Some(session_id) = resume_session_id {
            args.extend(["--resume".to_string(), session_id.to_string()]);
            // CRITICAL: Also pass --agent to enforce disallowedTools on resume
            // Without this, resumed sessions bypass tool restrictions
            if let Some(agent) = &config.agent {
                args.extend(["--agent".to_string(), agent.clone()]);
            }
        } else if let Some(agent) = &config.agent {
            args.extend(["--agent".to_string(), agent.clone()]);
        }

        // Apply CLI tool restrictions from agent_config
        // Frontmatter tools/disallowedTools only work for subagent spawning,
        // NOT for direct CLI invocations with --agent -p. We must pass --tools flag.
        if let Some(agent_name) = &config.agent {
            if let Some(allowed_tools) = get_allowed_tools(agent_name) {
                args.extend(["--tools".to_string(), allowed_tools.to_string()]);
                eprintln!("[CLI] Agent {} restricted to CLI tools: {:?}", agent_name,
                    if allowed_tools.is_empty() { "(MCP only)" } else { allowed_tools });
            }
        }

        // Model override
        if let Some(model) = &config.model {
            args.extend(["--model".to_string(), model.clone()]);
        }

        // Max tokens
        if let Some(max_tokens) = config.max_tokens {
            args.extend(["--max-tokens".to_string(), max_tokens.to_string()]);
        }

        // Permission prompt tool for UI-based approval
        args.extend([
            "--permission-prompt-tool".to_string(),
            "mcp__ralphx__permission_request".to_string(),
        ]);

        // Pre-approve agent-specific MCP tools (no permission prompts)
        if let Some(agent) = &config.agent {
            if let Some(mcp_tools) = get_allowed_mcp_tools(agent) {
                args.push("--allowedTools".to_string());
                args.push(mcp_tools);
            }
        }

        args
    }

    /// Spawn an agent in streaming mode, returning the Child process for external processing
    ///
    /// Unlike `spawn_agent`, this method does NOT store the child process internally.
    /// The caller is responsible for:
    /// 1. Processing stdout for stream-json events
    /// 2. Waiting for the process to complete
    /// 3. Capturing the claude_session_id from the Result event
    ///
    /// This is used by ExecutionChatService to persist stream events to the database
    /// while emitting Tauri events for real-time UI updates.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let result = client.spawn_agent_streaming(config, None).await?;
    /// let stdout = result.child.stdout.take().unwrap();
    /// let reader = BufReader::new(stdout);
    /// // Process stream-json lines from reader...
    /// ```
    pub async fn spawn_agent_streaming(
        &self,
        config: AgentConfig,
        resume_session_id: Option<&str>,
    ) -> AgentResult<StreamingSpawnResult> {
        if let Err(err) = ensure_claude_spawn_allowed() {
            return Err(AgentError::SpawnNotAllowed(err));
        }

        // Check if CLI is available first
        if !self.cli_path.exists() && which::which(&self.cli_path).is_err() {
            return Err(AgentError::CliNotAvailable(format!(
                "claude CLI not found at {:?}",
                self.cli_path
            )));
        }

        let args = self.build_cli_args(&config, resume_session_id);

        // Build command
        let mut cmd = tokio::process::Command::new(&self.cli_path);
        cmd.args(&args)
            .current_dir(&config.working_directory)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        // Add environment variables from config
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        // Spawn the process
        let child = cmd
            .spawn()
            .map_err(|e| AgentError::SpawnFailed(e.to_string()))?;

        let handle = AgentHandle::new(ClientType::ClaudeCode, config.role);

        Ok(StreamingSpawnResult { handle, child })
    }

    /// Check if the Claude CLI is available
    ///
    /// This is a simpler version of is_available() that doesn't require async.
    pub fn cli_available(&self) -> bool {
        self.cli_path.exists() || which::which(&self.cli_path).is_ok()
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
    async fn test_spawn_agent_blocked_in_tests() {
        let client =
            ClaudeCodeClient::new().with_cli_path("/nonexistent/path/to/claude_binary_12345");
        let config = AgentConfig::worker("test");

        let result = client.spawn_agent(config).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(AgentError::SpawnNotAllowed(_))));
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

    // ==================== Streaming Spawn Tests ====================

    #[test]
    fn test_build_cli_args_basic() {
        let client = ClaudeCodeClient::new();
        let config = AgentConfig::worker("Test prompt");

        let args = client.build_cli_args(&config, None);

        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"Test prompt".to_string()));
        assert!(args.contains(&"--output-format".to_string()));
        assert!(args.contains(&"stream-json".to_string()));
        assert!(args.contains(&"--permission-prompt-tool".to_string()));
        assert!(args.contains(&"mcp__ralphx__permission_request".to_string()));
    }

    #[test]
    fn test_build_cli_args_with_agent() {
        let client = ClaudeCodeClient::new();
        let config = AgentConfig::worker("Test").with_agent("worker");

        let args = client.build_cli_args(&config, None);

        assert!(args.contains(&"--agent".to_string()));
        assert!(args.contains(&"worker".to_string()));
    }

    #[test]
    fn test_build_cli_args_with_resume() {
        let client = ClaudeCodeClient::new();
        let config = AgentConfig::worker("Test").with_agent("worker");

        let args = client.build_cli_args(&config, Some("session-123"));

        // When resuming, both --resume AND --agent should be present
        // to ensure tool restrictions (disallowedTools) are enforced
        assert!(args.contains(&"--resume".to_string()));
        assert!(args.contains(&"session-123".to_string()));
        // Agent MUST be present when resuming to enforce disallowedTools
        assert!(args.contains(&"--agent".to_string()));
        assert!(args.contains(&"worker".to_string()));
    }

    #[test]
    fn test_build_cli_args_applies_tools_restriction() {
        let client = ClaudeCodeClient::new();
        let config = AgentConfig::worker("Test").with_agent("session-namer");

        let args = client.build_cli_args(&config, None);

        // session-namer has allowed_tools = Some("") meaning no CLI tools
        let tools_idx = args.iter().position(|a| a == "--tools").expect("--tools flag must be present");
        assert_eq!(args[tools_idx + 1], "", "session-namer should have empty tools (MCP only)");
    }

    #[test]
    fn test_build_cli_args_no_tools_for_unknown_agent() {
        let client = ClaudeCodeClient::new();
        let config = AgentConfig::worker("Test").with_agent("unknown-agent-xyz");

        let args = client.build_cli_args(&config, None);

        // Unknown agent should NOT have --tools restriction
        assert!(!args.contains(&"--tools".to_string()), "unknown agent should not have --tools flag");
    }

    #[test]
    fn test_build_cli_args_restricted_agent_tools() {
        let client = ClaudeCodeClient::new();
        let config = AgentConfig::worker("Test").with_agent("orchestrator-ideation");

        let args = client.build_cli_args(&config, None);

        let tools_idx = args.iter().position(|a| a == "--tools").expect("--tools flag must be present");
        assert_eq!(args[tools_idx + 1], "Read,Grep,Glob", "orchestrator-ideation should have Read,Grep,Glob");
    }

    #[test]
    fn test_build_cli_args_with_model() {
        let client = ClaudeCodeClient::new();
        let config = AgentConfig::worker("Test").with_model("opus");

        let args = client.build_cli_args(&config, None);

        assert!(args.contains(&"--model".to_string()));
        assert!(args.contains(&"opus".to_string()));
    }

    #[test]
    fn test_build_cli_args_with_plugin_dir() {
        let client = ClaudeCodeClient::new();
        let config = AgentConfig::worker("Test").with_plugin_dir("/custom/plugin");

        let args = client.build_cli_args(&config, None);

        assert!(args.contains(&"--plugin-dir".to_string()));
        assert!(args.contains(&"/custom/plugin".to_string()));
    }

    #[tokio::test]
    async fn test_spawn_agent_streaming_blocked_in_tests() {
        let client =
            ClaudeCodeClient::new().with_cli_path("/nonexistent/path/to/claude_binary_12345");
        let config = AgentConfig::worker("test");

        let result = client.spawn_agent_streaming(config, None).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(AgentError::SpawnNotAllowed(_))));
    }

    #[test]
    fn test_cli_available_with_nonexistent_path() {
        let client =
            ClaudeCodeClient::new().with_cli_path("/nonexistent/path/to/claude_binary_12345");
        assert!(!client.cli_available());
    }

    // ==================== StreamEvent Tests ====================

    #[test]
    fn test_stream_event_text_chunk_serialization() {
        let event = StreamEvent::TextChunk {
            text: "Hello world".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("TextChunk"));
        assert!(json.contains("Hello world"));

        // Deserialize back
        let parsed: StreamEvent = serde_json::from_str(&json).unwrap();
        if let StreamEvent::TextChunk { text } = parsed {
            assert_eq!(text, "Hello world");
        } else {
            panic!("Expected TextChunk");
        }
    }

    #[test]
    fn test_stream_event_tool_call_start_serialization() {
        let event = StreamEvent::ToolCallStart {
            tool_name: "Read".to_string(),
            tool_id: Some("tool-123".to_string()),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("ToolCallStart"));
        assert!(json.contains("Read"));
        assert!(json.contains("tool-123"));
    }

    #[test]
    fn test_stream_event_tool_call_complete_serialization() {
        let event = StreamEvent::ToolCallComplete {
            tool_name: "Write".to_string(),
            tool_id: None,
            arguments: serde_json::json!({"path": "test.txt"}),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("ToolCallComplete"));
        assert!(json.contains("Write"));
        assert!(json.contains("path"));
    }

    #[test]
    fn test_stream_event_completed_serialization() {
        let event = StreamEvent::Completed {
            session_id: Some("sess-456".to_string()),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Completed"));
        assert!(json.contains("sess-456"));
    }

    #[test]
    fn test_stream_event_error_serialization() {
        let event = StreamEvent::Error {
            message: "Something went wrong".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Error"));
        assert!(json.contains("Something went wrong"));
    }

    #[test]
    fn test_streaming_spawn_result_debug() {
        // StreamingSpawnResult is Debug
        fn assert_debug<T: std::fmt::Debug>() {}
        assert_debug::<StreamingSpawnResult>();
    }
}
