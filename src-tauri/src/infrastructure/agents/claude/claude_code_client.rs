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
    AgentConfig, AgentError, AgentHandle, AgentOutput, AgentResponse, AgentResult, AgentRole,
    AgenticClient, ClientCapabilities, ClientType, ResponseChunk,
};

use super::{
    apply_common_spawn_env, claude_runtime_config, create_mcp_config, ensure_claude_spawn_allowed,
    get_allowed_tools, get_effective_settings, get_preapproved_tools, sanitize_claude_user_state,
};

// ============================================================================
// Streaming Event Types
// ============================================================================

/// Events emitted during agent stream processing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    /// Text chunk received from agent
    TextChunk { text: String },
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
    Completed { session_id: Option<String> },
    /// Error occurred during execution
    Error { message: String },
}

/// Result from spawning an agent in streaming mode
#[derive(Debug)]
pub struct StreamingSpawnResult {
    /// Handle to the spawned agent
    pub handle: AgentHandle,
    /// The spawned child process (stdout is piped for stream processing)
    pub child: Child,
}

// ============================================================================
// Teammate Interactive Spawn Types
// ============================================================================

/// Configuration for spawning a team teammate in interactive mode (no `-p` flag).
///
/// Unlike `AgentConfig` (print mode), teammates are long-lived interactive sessions
/// that receive messages via Claude Code's native SendMessage tool. The process stays
/// alive until a shutdown_request is received.
#[derive(Debug, Clone)]
pub struct TeammateSpawnConfig {
    /// Teammate name (e.g., "transport-researcher")
    pub name: String,
    /// Team name (e.g., "ideation-abc123")
    pub team_name: String,
    /// Session ID of the team lead that spawned this teammate
    pub parent_session_id: String,
    /// Lead-generated role prompt (passed via --append-system-prompt)
    pub prompt: String,
    /// Model to use (within model ceiling, e.g. "sonnet")
    pub model: String,
    /// Approved CLI tools (e.g. ["Read", "Grep", "Glob"])
    pub tools: Vec<String>,
    /// Approved MCP tools (short names; will be prefixed with mcp__ralphx__)
    pub mcp_tools: Vec<String>,
    /// Agent color for terminal distinction (e.g. "blue", "green")
    pub color: String,
    /// Working directory for the teammate process
    pub working_directory: PathBuf,
    /// Plugin directory path for MCP server and agent discovery
    pub plugin_dir: Option<PathBuf>,
    /// Claude Code agent type controlling built-in tool set (default: "general-purpose")
    pub agent_type: String,
    /// MCP agent type for tool filtering (default: "ideation-team-member")
    pub mcp_agent_type: String,
    /// Additional environment variables
    pub env: HashMap<String, String>,
    /// Optional print-mode prompt. When set, teammate uses `-p <prompt>` (one-shot)
    /// instead of interactive `--append-system-prompt` mode. Used for auto-spawning
    /// teammates detected from the lead's stream.
    pub print_mode_prompt: Option<String>,
}

impl TeammateSpawnConfig {
    /// Create a new teammate config with required fields and sensible defaults.
    pub fn new(
        name: impl Into<String>,
        team_name: impl Into<String>,
        parent_session_id: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            team_name: team_name.into(),
            parent_session_id: parent_session_id.into(),
            prompt: prompt.into(),
            model: "sonnet".to_string(),
            tools: Vec::new(),
            mcp_tools: Vec::new(),
            color: "blue".to_string(),
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            plugin_dir: Some(PathBuf::from("./ralphx-plugin")),
            agent_type: "general-purpose".to_string(),
            mcp_agent_type: "ideation-team-member".to_string(),
            env: HashMap::new(),
            print_mode_prompt: None,
        }
    }

    /// Set the model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set the CLI tools.
    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.tools = tools;
        self
    }

    /// Set the MCP tools.
    pub fn with_mcp_tools(mut self, mcp_tools: Vec<String>) -> Self {
        self.mcp_tools = mcp_tools;
        self
    }

    /// Set the agent color.
    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = color.into();
        self
    }

    /// Set the working directory.
    pub fn with_working_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.working_directory = path.into();
        self
    }

    /// Set the plugin directory.
    pub fn with_plugin_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.plugin_dir = Some(path.into());
        self
    }

    /// Set the Claude Code agent type (controls built-in tool set).
    pub fn with_agent_type(mut self, agent_type: impl Into<String>) -> Self {
        self.agent_type = agent_type.into();
        self
    }

    /// Set the MCP agent type (controls MCP-side tool filtering).
    pub fn with_mcp_agent_type(mut self, mcp_agent_type: impl Into<String>) -> Self {
        self.mcp_agent_type = mcp_agent_type.into();
        self
    }

    /// Add an environment variable.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Set print-mode prompt for one-shot `-p` execution (auto-spawn mode).
    pub fn with_print_mode_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.print_mode_prompt = Some(prompt.into());
        self
    }
}

/// Result from spawning a teammate in interactive mode.
#[derive(Debug)]
pub struct TeammateSpawnResult {
    /// Handle to the spawned teammate
    pub handle: AgentHandle,
    /// The spawned child process (stdout piped for stream processing)
    pub child: Child,
    /// Stdin pipe for sending messages to the teammate
    pub stdin: tokio::process::ChildStdin,
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
        sanitize_claude_user_state();

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
        if let Some(sources) = &claude_runtime_config().setting_sources {
            if !sources.is_empty() {
                args.extend(["--setting-sources".to_string(), sources.join(",")]);
            }
        }
        // Avoid startup parser crashes in slash-command/skills loading path.
        args.push("--disable-slash-commands".to_string());

        // Add plugin directory for agent/skill discovery
        if let Some(plugin_dir) = &config.plugin_dir {
            args.extend(["--plugin-dir".to_string(), plugin_dir.display().to_string()]);

            // Create dynamic MCP config for agent-specific tool filtering
            // Use --strict-mcp-config to ignore user/global MCP servers that can hang
            if let Some(agent) = &config.agent {
                if let Some(temp_path) = create_mcp_config(plugin_dir, agent) {
                    args.extend([
                        "--mcp-config".to_string(),
                        temp_path.display().to_string(),
                        "--strict-mcp-config".to_string(),
                    ]);
                }
            }
        }

        // Add agent name if specified
        if let Some(agent) = &config.agent {
            args.extend(["--agent".to_string(), agent.clone()]);
        }

        // Apply CLI tool restrictions from agent_config
        if let Some(agent_name) = &config.agent {
            if let Some(allowed_tools) = get_allowed_tools(agent_name) {
                args.extend(["--tools".to_string(), allowed_tools]);
            }
        }

        // Add model: explicit config override first, then per-agent default from ralphx.yaml
        if let Some(model) = &config.model {
            args.extend(["--model".to_string(), model.clone()]);
        } else if let Some(agent_name) = &config.agent {
            if let Some(agent_model) =
                crate::infrastructure::agents::claude::get_agent_config(agent_name)
                    .and_then(|cfg| cfg.model.as_ref())
            {
                args.extend(["--model".to_string(), agent_model.clone()]);
            }
        }

        // Add max tokens if specified
        if let Some(max_tokens) = config.max_tokens {
            args.extend(["--max-tokens".to_string(), max_tokens.to_string()]);
        }

        // Permission handling from ralphx.yaml
        let runtime = claude_runtime_config();
        args.extend([
            "--permission-prompt-tool".to_string(),
            runtime.permission_prompt_tool.clone(),
        ]);
        args.extend([
            "--permission-mode".to_string(),
            runtime.permission_mode.clone(),
        ]);
        if runtime.dangerously_skip_permissions {
            args.push("--dangerously-skip-permissions".to_string());
        }
        // Optional settings JSON passed to claude CLI via --settings.
        // Agent-specific profile overrides global profile when configured.
        if let Some(s) = get_effective_settings(config.agent.as_deref()) {
            if let Ok(json) = serde_json::to_string(s) {
                args.extend(["--settings".to_string(), json]);
            }
        }

        // Pre-approve agent-specific tools (MCP + CLI permissions, no prompts)
        if let Some(agent) = &config.agent {
            if let Some(preapproved) = get_preapproved_tools(agent) {
                args.push("--allowedTools".to_string());
                args.push(preapproved);
            }
        }

        // Build command
        let mut cmd = tokio::process::Command::new(&self.cli_path);
        cmd.args(&args)
            .current_dir(&config.working_directory)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());
        apply_common_spawn_env(&mut cmd);
        if let Some(plugin_dir) = &config.plugin_dir {
            cmd.env("CLAUDE_PLUGIN_ROOT", plugin_dir);
        }

        // Add environment variables
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        // Spawn the process and record start time for duration tracking
        tracing::info!(cmd = ?cmd, "Spawning CLI agent (agentic)");
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

    async fn send_prompt(&self, _handle: &AgentHandle, prompt: &str) -> AgentResult<AgentResponse> {
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
            Ok(ResponseChunk::new(
                "Use spawn_agent_streaming() for production streaming",
            )),
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
        sanitize_claude_user_state();
        let mut args = Vec::new();

        // Prompt
        args.extend(["-p".to_string(), config.prompt.clone()]);

        // Output format for streaming
        args.extend(["--output-format".to_string(), "stream-json".to_string()]);
        args.push("--verbose".to_string()); // Required for stream-json with -p
        if let Some(sources) = &claude_runtime_config().setting_sources {
            if !sources.is_empty() {
                args.extend(["--setting-sources".to_string(), sources.join(",")]);
            }
        }
        // Avoid startup parser crashes in slash-command/skills loading path.
        args.push("--disable-slash-commands".to_string());

        // Plugin directory for agent/skill discovery
        if let Some(plugin_dir) = &config.plugin_dir {
            args.extend(["--plugin-dir".to_string(), plugin_dir.display().to_string()]);

            // Create dynamic MCP config for agent-specific tool filtering
            // Use --strict-mcp-config to ignore user/global MCP servers that can hang
            if let Some(agent) = &config.agent {
                if let Some(temp_path) = create_mcp_config(plugin_dir, agent) {
                    args.extend([
                        "--mcp-config".to_string(),
                        temp_path.display().to_string(),
                        "--strict-mcp-config".to_string(),
                    ]);
                }
            }
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
                tracing::debug!(agent = %agent_name, tools = if allowed_tools.is_empty() { "(MCP only)" } else { allowed_tools.as_str() }, "Agent restricted to CLI tools");
                args.extend(["--tools".to_string(), allowed_tools]);
            }
        }

        // Model override: explicit config first, then per-agent default from ralphx.yaml
        if let Some(model) = &config.model {
            args.extend(["--model".to_string(), model.clone()]);
        } else if let Some(agent_name) = &config.agent {
            if let Some(agent_model) =
                crate::infrastructure::agents::claude::get_agent_config(agent_name)
                    .and_then(|cfg| cfg.model.as_ref())
            {
                args.extend(["--model".to_string(), agent_model.clone()]);
            }
        }

        // Max tokens
        if let Some(max_tokens) = config.max_tokens {
            args.extend(["--max-tokens".to_string(), max_tokens.to_string()]);
        }

        // Permission handling from ralphx.yaml
        let runtime = claude_runtime_config();
        args.extend([
            "--permission-prompt-tool".to_string(),
            runtime.permission_prompt_tool.clone(),
        ]);
        args.extend([
            "--permission-mode".to_string(),
            runtime.permission_mode.clone(),
        ]);
        if runtime.dangerously_skip_permissions {
            args.push("--dangerously-skip-permissions".to_string());
        }
        // Optional settings JSON passed to claude CLI via --settings.
        // Agent-specific profile overrides global profile when configured.
        if let Some(s) = get_effective_settings(config.agent.as_deref()) {
            if let Ok(json) = serde_json::to_string(s) {
                args.extend(["--settings".to_string(), json]);
            }
        }

        // Pre-approve agent-specific tools (MCP + CLI permissions, no prompts)
        if let Some(agent) = &config.agent {
            if let Some(preapproved) = get_preapproved_tools(agent) {
                args.push("--allowedTools".to_string());
                args.push(preapproved);
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
        apply_common_spawn_env(&mut cmd);
        if let Some(plugin_dir) = &config.plugin_dir {
            cmd.env("CLAUDE_PLUGIN_ROOT", plugin_dir);
        }

        // Add environment variables from config
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        // Spawn the process
        tracing::info!(cmd = ?cmd, "Spawning CLI agent (streaming)");
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

// ============================================================================
// Teammate Interactive Spawn Support
// ============================================================================

impl ClaudeCodeClient {
    /// Build CLI arguments for an interactive teammate spawn.
    ///
    /// Key differences from `build_cli_args`:
    /// - **No `-p` flag** — teammates are interactive sessions
    /// - **Team CLI flags** — `--agent-id`, `--agent-name`, `--team-name`, etc.
    /// - **`--append-system-prompt`** — lead-generated role prompt
    /// - **`--dangerously-skip-permissions`** — automated teammates skip prompts
    pub fn build_teammate_cli_args(&self, config: &TeammateSpawnConfig) -> Vec<String> {
        sanitize_claude_user_state();
        let mut args = Vec::new();

        // Output format for streaming (same as other modes)
        args.extend(["--output-format".to_string(), "stream-json".to_string()]);
        args.push("--verbose".to_string());

        // Setting sources from runtime config
        if let Some(sources) = &claude_runtime_config().setting_sources {
            if !sources.is_empty() {
                args.extend(["--setting-sources".to_string(), sources.join(",")]);
            }
        }

        // Avoid startup parser crashes in slash-command/skills loading path
        args.push("--disable-slash-commands".to_string());

        // Plugin directory for agent/skill discovery
        if let Some(plugin_dir) = &config.plugin_dir {
            args.extend(["--plugin-dir".to_string(), plugin_dir.display().to_string()]);

            // Create dynamic MCP config with MCP agent type for tool filtering
            // Uses mcp_agent_type (e.g., "ideation-team-member") not the Claude Code agent_type
            if let Some(temp_path) = create_mcp_config(plugin_dir, &config.mcp_agent_type) {
                args.extend([
                    "--mcp-config".to_string(),
                    temp_path.display().to_string(),
                    "--strict-mcp-config".to_string(),
                ]);
            }
        }

        // --- Team-specific CLI flags ---
        args.extend([
            "--agent-id".to_string(),
            format!("{}@{}", config.name, config.team_name),
        ]);
        args.extend(["--agent-name".to_string(), config.name.clone()]);
        args.extend(["--team-name".to_string(), config.team_name.clone()]);
        args.extend(["--agent-color".to_string(), config.color.clone()]);
        args.extend([
            "--parent-session-id".to_string(),
            config.parent_session_id.clone(),
        ]);
        // Claude Code agent type controls built-in tool set (e.g., "general-purpose")
        args.extend(["--agent-type".to_string(), config.agent_type.clone()]);

        // Model selection (within model ceiling)
        args.extend(["--model".to_string(), config.model.clone()]);

        // CLI tools restriction
        if !config.tools.is_empty() {
            args.extend(["--tools".to_string(), config.tools.join(",")]);
        }

        // Pre-approved MCP tools (prefixed with mcp__ralphx__)
        if !config.mcp_tools.is_empty() {
            let mcp_server_name = &claude_runtime_config().mcp_server_name;
            let prefixed: Vec<String> = config
                .mcp_tools
                .iter()
                .map(|t| format!("mcp__{mcp_server_name}__{t}"))
                .collect();
            args.extend(["--allowedTools".to_string(), prefixed.join(",")]);
        }

        // Prompt mode: print-mode (-p) for auto-spawned teammates, interactive
        // (--append-system-prompt) for manually spawned teammates
        if let Some(ref prompt) = config.print_mode_prompt {
            args.extend(["-p".to_string(), prompt.clone()]);
        } else {
            args.extend(["--append-system-prompt".to_string(), config.prompt.clone()]);
        }

        // Skip permissions for automated teammates
        args.push("--dangerously-skip-permissions".to_string());

        args
    }

    /// Build environment variables for a teammate spawn.
    ///
    /// Returns the team-specific env vars that must be set on the process
    /// in addition to the common spawn env.
    pub fn build_teammate_env_vars(config: &TeammateSpawnConfig) -> HashMap<String, String> {
        let mut env = HashMap::new();

        // Team feature flags (required for agent teams)
        env.insert("CLAUDECODE".to_string(), "1".to_string());
        env.insert(
            "CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS".to_string(),
            "1".to_string(),
        );

        // MCP agent type for tool filtering (also available as env fallback)
        env.insert(
            "RALPHX_AGENT_TYPE".to_string(),
            config.mcp_agent_type.clone(),
        );

        // Merge in any custom env vars from config
        for (key, value) in &config.env {
            env.insert(key.clone(), value.clone());
        }

        env
    }

    /// Spawn a teammate in interactive mode for agent team participation.
    ///
    /// Unlike `spawn_agent` (print mode with `-p`), this spawns an interactive session:
    /// - No `-p` flag — the teammate stays alive for multi-turn messaging
    /// - stdin is piped for message injection
    /// - Team env vars (`CLAUDECODE=1`, `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1`)
    /// - Team CLI flags (`--agent-id`, `--agent-name`, `--team-name`, etc.)
    /// - Role prompt via `--append-system-prompt`
    ///
    /// The caller is responsible for:
    /// 1. Writing messages to the returned stdin pipe
    /// 2. Processing stdout for stream-json events
    /// 3. Monitoring the process lifecycle
    /// 4. Sending shutdown_request when done
    pub async fn spawn_teammate_interactive(
        &self,
        config: TeammateSpawnConfig,
    ) -> AgentResult<TeammateSpawnResult> {
        if let Err(err) = ensure_claude_spawn_allowed() {
            return Err(AgentError::SpawnNotAllowed(err));
        }

        // Check if CLI is available
        if !self.cli_path.exists() && which::which(&self.cli_path).is_err() {
            return Err(AgentError::CliNotAvailable(format!(
                "claude CLI not found at {:?}",
                self.cli_path
            )));
        }

        let args = self.build_teammate_cli_args(&config);
        let team_env = Self::build_teammate_env_vars(&config);

        // Build command
        let mut cmd = tokio::process::Command::new(&self.cli_path);
        cmd.args(&args)
            .current_dir(&config.working_directory)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped()); // Piped for message injection (NOT null)

        // Apply common RalphX spawn env vars
        apply_common_spawn_env(&mut cmd);

        // Plugin root env var
        if let Some(plugin_dir) = &config.plugin_dir {
            cmd.env("CLAUDE_PLUGIN_ROOT", plugin_dir);
        }

        // Team-specific env vars
        for (key, value) in &team_env {
            cmd.env(key, value);
        }

        // Spawn the process
        tracing::info!(
            teammate = %config.name,
            team = %config.team_name,
            model = %config.model,
            agent_type = %config.agent_type,
            "Spawning teammate (interactive)"
        );

        let mut child = cmd
            .spawn()
            .map_err(|e| AgentError::SpawnFailed(e.to_string()))?;

        // Take stdin pipe before returning
        let stdin = child.stdin.take().ok_or_else(|| {
            AgentError::SpawnFailed("Failed to capture stdin pipe for teammate".to_string())
        })?;

        let handle = AgentHandle::new(
            ClientType::ClaudeCode,
            AgentRole::Custom(format!("teammate:{}", config.name)),
        );

        Ok(TeammateSpawnResult {
            handle,
            child,
            stdin,
        })
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
        // Use fully-qualified name as would be used in production
        let config = AgentConfig::worker("Test")
            .with_agent(crate::infrastructure::agents::claude::agent_names::AGENT_SESSION_NAMER);

        let args = client.build_cli_args(&config, None);

        // session-namer has allowed_tools = Some("") meaning no CLI tools
        // get_allowed_tools strips the ralphx: prefix for AGENT_CONFIGS lookup
        let tools_idx = args
            .iter()
            .position(|a| a == "--tools")
            .expect("--tools flag must be present");
        assert_eq!(
            args[tools_idx + 1],
            "",
            "session-namer should have empty tools (MCP only)"
        );
    }

    #[test]
    fn test_build_cli_args_no_tools_for_unknown_agent() {
        let client = ClaudeCodeClient::new();
        let config = AgentConfig::worker("Test").with_agent("unknown-agent-xyz");

        let args = client.build_cli_args(&config, None);

        // Unknown agent should NOT have --tools restriction
        assert!(
            !args.contains(&"--tools".to_string()),
            "unknown agent should not have --tools flag"
        );
    }

    #[test]
    fn test_build_cli_args_restricted_agent_tools() {
        let client = ClaudeCodeClient::new();
        // Use fully-qualified name as would be used in production
        let config = AgentConfig::worker("Test").with_agent(
            crate::infrastructure::agents::claude::agent_names::AGENT_ORCHESTRATOR_IDEATION,
        );

        let args = client.build_cli_args(&config, None);

        let tools_idx = args
            .iter()
            .position(|a| a == "--tools")
            .expect("--tools flag must be present");
        assert_eq!(
            args[tools_idx + 1],
            "Read,Grep,Glob,Bash,WebFetch,WebSearch,Skill,Task",
            "orchestrator-ideation should have base tools + Task"
        );
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
    fn test_build_cli_args_uses_agent_model_when_not_overridden() {
        let client = ClaudeCodeClient::new();
        let config = AgentConfig::worker("Test")
            .with_agent(crate::infrastructure::agents::claude::agent_names::AGENT_MERGER);

        let args = client.build_cli_args(&config, None);
        let model_idx = args
            .iter()
            .position(|a| a == "--model")
            .expect("--model flag must be present");
        assert_eq!(args[model_idx + 1], "opus");
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

    // ==================== Teammate Interactive Spawn Tests ====================

    fn test_teammate_config() -> TeammateSpawnConfig {
        TeammateSpawnConfig::new(
            "transport-researcher",
            "ideation-abc123",
            "lead-session-uuid",
            "You are a transport research specialist. Investigate WebSocket vs SSE.",
        )
        .with_model("sonnet")
        .with_tools(vec![
            "Read".to_string(),
            "Grep".to_string(),
            "Glob".to_string(),
        ])
        .with_mcp_tools(vec![
            "get_session_plan".to_string(),
            "list_session_proposals".to_string(),
        ])
        .with_color("blue")
        .with_working_dir("/tmp/test")
        .with_plugin_dir("/test/ralphx-plugin")
    }

    #[test]
    fn test_teammate_spawn_config_new_defaults() {
        let config = TeammateSpawnConfig::new("researcher", "team-1", "session-1", "Do research");

        assert_eq!(config.name, "researcher");
        assert_eq!(config.team_name, "team-1");
        assert_eq!(config.parent_session_id, "session-1");
        assert_eq!(config.prompt, "Do research");
        assert_eq!(config.model, "sonnet");
        assert_eq!(config.color, "blue");
        assert_eq!(config.agent_type, "general-purpose");
        assert_eq!(config.mcp_agent_type, "ideation-team-member");
        assert!(config.tools.is_empty());
        assert!(config.mcp_tools.is_empty());
        assert!(config.env.is_empty());
    }

    #[test]
    fn test_teammate_spawn_config_builder_chain() {
        let config = TeammateSpawnConfig::new("dev", "team-x", "sess-1", "Code stuff")
            .with_model("haiku")
            .with_tools(vec!["Read".to_string()])
            .with_mcp_tools(vec!["get_task_context".to_string()])
            .with_color("green")
            .with_working_dir("/work")
            .with_plugin_dir("/plugins")
            .with_agent_type("Bash")
            .with_mcp_agent_type("worker-team-member")
            .with_env("CUSTOM_VAR", "value");

        assert_eq!(config.model, "haiku");
        assert_eq!(config.tools, vec!["Read"]);
        assert_eq!(config.mcp_tools, vec!["get_task_context"]);
        assert_eq!(config.color, "green");
        assert_eq!(config.working_directory, PathBuf::from("/work"));
        assert_eq!(config.plugin_dir, Some(PathBuf::from("/plugins")));
        assert_eq!(config.agent_type, "Bash");
        assert_eq!(config.mcp_agent_type, "worker-team-member");
        assert_eq!(config.env.get("CUSTOM_VAR"), Some(&"value".to_string()));
    }

    #[test]
    fn test_build_teammate_cli_args_no_print_flag() {
        let client = ClaudeCodeClient::new();
        let config = test_teammate_config();
        let args = client.build_teammate_cli_args(&config);

        // CRITICAL: No -p flag — interactive mode
        assert!(
            !args.contains(&"-p".to_string()),
            "Teammate args must NOT contain -p flag (interactive mode)"
        );
    }

    #[test]
    fn test_build_teammate_cli_args_has_output_format() {
        let client = ClaudeCodeClient::new();
        let config = test_teammate_config();
        let args = client.build_teammate_cli_args(&config);

        assert!(args.contains(&"--output-format".to_string()));
        assert!(args.contains(&"stream-json".to_string()));
        assert!(args.contains(&"--verbose".to_string()));
    }

    #[test]
    fn test_build_teammate_cli_args_has_team_flags() {
        let client = ClaudeCodeClient::new();
        let config = test_teammate_config();
        let args = client.build_teammate_cli_args(&config);

        // --agent-id <name>@<team-name>
        let agent_id_idx = args
            .iter()
            .position(|a| a == "--agent-id")
            .expect("--agent-id flag must be present");
        assert_eq!(
            args[agent_id_idx + 1],
            "transport-researcher@ideation-abc123"
        );

        // --agent-name
        let agent_name_idx = args
            .iter()
            .position(|a| a == "--agent-name")
            .expect("--agent-name flag must be present");
        assert_eq!(args[agent_name_idx + 1], "transport-researcher");

        // --team-name
        let team_name_idx = args
            .iter()
            .position(|a| a == "--team-name")
            .expect("--team-name flag must be present");
        assert_eq!(args[team_name_idx + 1], "ideation-abc123");

        // --agent-color
        let color_idx = args
            .iter()
            .position(|a| a == "--agent-color")
            .expect("--agent-color flag must be present");
        assert_eq!(args[color_idx + 1], "blue");

        // --parent-session-id
        let parent_idx = args
            .iter()
            .position(|a| a == "--parent-session-id")
            .expect("--parent-session-id flag must be present");
        assert_eq!(args[parent_idx + 1], "lead-session-uuid");

        // --agent-type (Claude Code built-in tool set)
        let agent_type_idx = args
            .iter()
            .position(|a| a == "--agent-type")
            .expect("--agent-type flag must be present");
        assert_eq!(args[agent_type_idx + 1], "general-purpose");
    }

    #[test]
    fn test_build_teammate_cli_args_has_model() {
        let client = ClaudeCodeClient::new();
        let config = test_teammate_config();
        let args = client.build_teammate_cli_args(&config);

        let model_idx = args
            .iter()
            .position(|a| a == "--model")
            .expect("--model flag must be present");
        assert_eq!(args[model_idx + 1], "sonnet");
    }

    #[test]
    fn test_build_teammate_cli_args_has_tools() {
        let client = ClaudeCodeClient::new();
        let config = test_teammate_config();
        let args = client.build_teammate_cli_args(&config);

        let tools_idx = args
            .iter()
            .position(|a| a == "--tools")
            .expect("--tools flag must be present");
        assert_eq!(args[tools_idx + 1], "Read,Grep,Glob");
    }

    #[test]
    fn test_build_teammate_cli_args_no_tools_when_empty() {
        let client = ClaudeCodeClient::new();
        let config = TeammateSpawnConfig::new("r", "t", "s", "p");
        let args = client.build_teammate_cli_args(&config);

        assert!(
            !args.contains(&"--tools".to_string()),
            "Empty tools should not produce --tools flag"
        );
    }

    #[test]
    fn test_build_teammate_cli_args_mcp_tools_prefixed() {
        let client = ClaudeCodeClient::new();
        let config = test_teammate_config();
        let args = client.build_teammate_cli_args(&config);

        let allowed_idx = args
            .iter()
            .position(|a| a == "--allowedTools")
            .expect("--allowedTools flag must be present");
        let allowed_value = &args[allowed_idx + 1];

        // MCP tools should be prefixed with mcp__ralphx__
        assert!(
            allowed_value.contains("mcp__ralphx__get_session_plan"),
            "MCP tools must be prefixed: got {allowed_value}"
        );
        assert!(
            allowed_value.contains("mcp__ralphx__list_session_proposals"),
            "MCP tools must be prefixed: got {allowed_value}"
        );
    }

    #[test]
    fn test_build_teammate_cli_args_no_allowed_tools_when_empty() {
        let client = ClaudeCodeClient::new();
        let config = TeammateSpawnConfig::new("r", "t", "s", "p");
        let args = client.build_teammate_cli_args(&config);

        assert!(
            !args.contains(&"--allowedTools".to_string()),
            "Empty MCP tools should not produce --allowedTools flag"
        );
    }

    #[test]
    fn test_build_teammate_cli_args_has_system_prompt() {
        let client = ClaudeCodeClient::new();
        let config = test_teammate_config();
        let args = client.build_teammate_cli_args(&config);

        let prompt_idx = args
            .iter()
            .position(|a| a == "--append-system-prompt")
            .expect("--append-system-prompt flag must be present");
        assert!(args[prompt_idx + 1].contains("transport research specialist"));
    }

    #[test]
    fn test_build_teammate_cli_args_has_skip_permissions() {
        let client = ClaudeCodeClient::new();
        let config = test_teammate_config();
        let args = client.build_teammate_cli_args(&config);

        assert!(
            args.contains(&"--dangerously-skip-permissions".to_string()),
            "Teammates must skip permissions"
        );
    }

    #[test]
    fn test_build_teammate_cli_args_has_disable_slash_commands() {
        let client = ClaudeCodeClient::new();
        let config = test_teammate_config();
        let args = client.build_teammate_cli_args(&config);

        assert!(args.contains(&"--disable-slash-commands".to_string()));
    }

    #[test]
    fn test_build_teammate_cli_args_has_plugin_dir() {
        let client = ClaudeCodeClient::new();
        let config = test_teammate_config();
        let args = client.build_teammate_cli_args(&config);

        assert!(args.contains(&"--plugin-dir".to_string()));
        assert!(args.contains(&"/test/ralphx-plugin".to_string()));
    }

    #[test]
    fn test_build_teammate_cli_args_custom_agent_type() {
        let client = ClaudeCodeClient::new();
        let config = test_teammate_config().with_agent_type("Bash");
        let args = client.build_teammate_cli_args(&config);

        let agent_type_idx = args
            .iter()
            .position(|a| a == "--agent-type")
            .expect("--agent-type flag must be present");
        assert_eq!(args[agent_type_idx + 1], "Bash");
    }

    #[test]
    fn test_build_teammate_env_vars_has_team_flags() {
        let config = test_teammate_config();
        let env = ClaudeCodeClient::build_teammate_env_vars(&config);

        assert_eq!(env.get("CLAUDECODE"), Some(&"1".to_string()));
        assert_eq!(
            env.get("CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS"),
            Some(&"1".to_string())
        );
    }

    #[test]
    fn test_build_teammate_env_vars_has_agent_type() {
        let config = test_teammate_config();
        let env = ClaudeCodeClient::build_teammate_env_vars(&config);

        assert_eq!(
            env.get("RALPHX_AGENT_TYPE"),
            Some(&"ideation-team-member".to_string())
        );
    }

    #[test]
    fn test_build_teammate_env_vars_custom_mcp_agent_type() {
        let config = test_teammate_config().with_mcp_agent_type("worker-team-member");
        let env = ClaudeCodeClient::build_teammate_env_vars(&config);

        assert_eq!(
            env.get("RALPHX_AGENT_TYPE"),
            Some(&"worker-team-member".to_string())
        );
    }

    #[test]
    fn test_build_teammate_env_vars_includes_custom_env() {
        let config = test_teammate_config()
            .with_env("RALPHX_PROJECT_ID", "proj-123")
            .with_env("RALPHX_SESSION_ID", "sess-456");
        let env = ClaudeCodeClient::build_teammate_env_vars(&config);

        assert_eq!(
            env.get("RALPHX_PROJECT_ID"),
            Some(&"proj-123".to_string())
        );
        assert_eq!(
            env.get("RALPHX_SESSION_ID"),
            Some(&"sess-456".to_string())
        );
        // Team flags still present
        assert_eq!(env.get("CLAUDECODE"), Some(&"1".to_string()));
    }

    #[tokio::test]
    async fn test_spawn_teammate_interactive_blocked_in_tests() {
        let client =
            ClaudeCodeClient::new().with_cli_path("/nonexistent/path/to/claude_binary_12345");
        let config = test_teammate_config();

        let result = client.spawn_teammate_interactive(config).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(AgentError::SpawnNotAllowed(_))));
    }

    #[test]
    fn test_teammate_spawn_result_debug() {
        fn assert_debug<T: std::fmt::Debug>() {}
        assert_debug::<TeammateSpawnResult>();
    }

    #[test]
    fn test_teammate_spawn_config_debug_and_clone() {
        let config = test_teammate_config();
        let cloned = config.clone();
        assert_eq!(cloned.name, "transport-researcher");
        // Verify Debug is implemented (compile-time check)
        let _debug = format!("{:?}", cloned);
    }

    #[test]
    fn test_build_teammate_cli_args_full_integration() {
        // Verify the complete arg list for a realistic teammate spawn
        let client = ClaudeCodeClient::new();
        let config = TeammateSpawnConfig::new(
            "react-state-sync-researcher",
            "ideation-session-789",
            "c43c3747-44d8-437b-9a25-911032eec2ea",
            "You are a React state management specialist. Analyze existing Zustand stores.",
        )
        .with_model("sonnet")
        .with_tools(vec![
            "Read".to_string(),
            "Grep".to_string(),
            "Glob".to_string(),
            "WebSearch".to_string(),
        ])
        .with_mcp_tools(vec![
            "get_session_plan".to_string(),
            "get_plan_artifact".to_string(),
        ])
        .with_color("green")
        .with_working_dir("/Users/test/project");

        let args = client.build_teammate_cli_args(&config);

        // Verify NO -p flag
        assert!(!args.contains(&"-p".to_string()));

        // Verify all required flags are present
        let required_flags = vec![
            "--output-format",
            "--verbose",
            "--disable-slash-commands",
            "--agent-id",
            "--agent-name",
            "--team-name",
            "--agent-color",
            "--parent-session-id",
            "--agent-type",
            "--model",
            "--tools",
            "--allowedTools",
            "--append-system-prompt",
            "--dangerously-skip-permissions",
        ];
        for flag in &required_flags {
            assert!(
                args.contains(&flag.to_string()),
                "Missing required flag: {flag}"
            );
        }

        // Verify agent-id format: name@team-name
        let agent_id_idx = args.iter().position(|a| a == "--agent-id").unwrap();
        assert_eq!(
            args[agent_id_idx + 1],
            "react-state-sync-researcher@ideation-session-789"
        );

        // Verify tools are comma-separated
        let tools_idx = args.iter().position(|a| a == "--tools").unwrap();
        assert_eq!(args[tools_idx + 1], "Read,Grep,Glob,WebSearch");

        // Verify MCP tools are prefixed and comma-separated
        let allowed_idx = args.iter().position(|a| a == "--allowedTools").unwrap();
        assert_eq!(
            args[allowed_idx + 1],
            "mcp__ralphx__get_session_plan,mcp__ralphx__get_plan_artifact"
        );
    }
}
