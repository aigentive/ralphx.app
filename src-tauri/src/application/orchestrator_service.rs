// Orchestrator Service
// Connects the Orchestrator agent to the ideation chat system.
// Invokes claude CLI with MCP integration and streams responses back to the UI.
//
// Phase 15 refactor:
// - Tool execution delegated to MCP server (RalphX MCP proxy)
// - Uses --resume flag for follow-up messages (Claude manages conversation context)
// - Passes RALPHX_AGENT_TYPE env var for MCP tool scoping
// - Emits Tauri events for real-time UI updates
// - Tracks agent runs for leave-and-come-back support

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use crate::domain::entities::{
    AgentRun, ChatConversation, ChatConversationId, ChatContextType, ChatMessage,
    IdeationSessionId, ProjectId, TaskId,
};
use crate::domain::repositories::{
    AgentRunRepository, ChatConversationRepository, ChatMessageRepository,
    IdeationSessionRepository, ProjectRepository, TaskRepository,
};
use crate::infrastructure::agents::claude::{
    build_base_cli_command, add_prompt_args, configure_spawn,
    StreamProcessor, StreamEvent as ProcessorStreamEvent, ContentBlockItem,
};

// Re-export stream types for use by other modules
pub use crate::infrastructure::agents::claude::{StreamMessage, ToolCall};

/// Result from orchestrator processing
#[derive(Debug, Clone)]
pub struct OrchestratorResult {
    pub response_text: String,
    pub tool_calls: Vec<ToolCall>,
    /// Claude's session ID for future --resume calls
    pub claude_session_id: Option<String>,
    /// The conversation ID this result belongs to
    pub conversation_id: Option<ChatConversationId>,
}

/// Result of executing a tool call (legacy - kept for compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub tool_name: String,
    pub success: bool,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// Event emitted during orchestrator processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrchestratorEvent {
    /// Agent run has started
    RunStarted {
        run_id: String,
        conversation_id: String,
    },
    /// User message was created and saved
    MessageCreated {
        message_id: String,
        conversation_id: String,
    },
    /// Text chunk received from agent
    TextChunk {
        text: String,
        conversation_id: String,
    },
    /// Tool call detected (MCP will execute it)
    ToolCallDetected {
        tool_name: String,
        tool_id: Option<String>,
        arguments: serde_json::Value,
        conversation_id: String,
    },
    /// Tool call completed (observed from stream)
    ToolCallCompleted {
        tool_name: String,
        tool_id: Option<String>,
        result: Option<serde_json::Value>,
        conversation_id: String,
    },
    /// Processing complete
    RunCompleted {
        conversation_id: String,
        claude_session_id: Option<String>,
        response_text: String,
    },
    /// Error occurred
    Error {
        conversation_id: Option<String>,
        error: String,
    },
}

// ============================================================================
// Tauri Event Payloads (for frontend consumption)
// ============================================================================

/// Payload for chat:chunk event
#[derive(Debug, Clone, Serialize)]
pub struct ChatChunkPayload {
    pub text: String,
    pub conversation_id: String,
}

/// Payload for chat:tool_call event
#[derive(Debug, Clone, Serialize)]
pub struct ChatToolCallPayload {
    pub tool_name: String,
    pub tool_id: Option<String>,
    pub arguments: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub conversation_id: String,
}

/// Payload for chat:run_completed event
#[derive(Debug, Clone, Serialize)]
pub struct ChatRunCompletedPayload {
    pub conversation_id: String,
    pub claude_session_id: Option<String>,
}

/// Payload for chat:message_created event
#[derive(Debug, Clone, Serialize)]
pub struct ChatMessageCreatedPayload {
    pub message_id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
}

/// Payload for execution:error and chat:error events
#[derive(Debug, Clone, Serialize)]
pub struct ChatErrorPayload {
    pub conversation_id: Option<String>,
    pub task_id: Option<String>,
    pub error: String,
    pub stderr: Option<String>,
}

/// Payload for execution:stderr events (real-time stderr streaming)
#[derive(Debug, Clone, Serialize)]
pub struct ChatStderrPayload {
    pub conversation_id: String,
    pub task_id: Option<String>,
    pub content: String,
}

// ============================================================================
// OrchestratorService trait
// ============================================================================

#[async_trait]
pub trait OrchestratorService: Send + Sync {
    /// Send a user message to the orchestrator and process the response
    /// This is the primary API for context-aware chat.
    ///
    /// The service will:
    /// 1. Get or create a conversation for the context
    /// 2. Create an agent run record
    /// 3. Save the user message
    /// 4. Spawn Claude CLI with appropriate flags (--agent or --resume)
    /// 5. Parse streaming output and emit Tauri events
    /// 6. Save the assistant response with tool calls
    /// 7. Update conversation with Claude's session_id
    /// 8. Complete the agent run
    async fn send_message(
        &self,
        session_id: &IdeationSessionId,
        user_message: &str,
    ) -> Result<OrchestratorResult, OrchestratorError>;

    /// Send a message for any context type (ideation, task, project)
    /// This is the generalized API for context-aware chat.
    async fn send_context_message(
        &self,
        context_type: ChatContextType,
        context_id: &str,
        user_message: &str,
    ) -> Result<OrchestratorResult, OrchestratorError>;

    /// Send a message with event streaming via mpsc channel
    /// Returns a receiver for OrchestratorEvent updates
    fn send_message_streaming(
        &self,
        session_id: IdeationSessionId,
        user_message: String,
    ) -> mpsc::Receiver<OrchestratorEvent>;

    /// Check if the orchestrator agent is available
    async fn is_available(&self) -> bool;

    /// Get the active agent run for a conversation, if any
    async fn get_active_run(
        &self,
        conversation_id: &ChatConversationId,
    ) -> Result<Option<AgentRun>, OrchestratorError>;
}

// ============================================================================
// Error type
// ============================================================================

#[derive(Debug, Clone)]
pub enum OrchestratorError {
    AgentNotAvailable(String),
    SpawnFailed(String),
    CommunicationFailed(String),
    ParseError(String),
    SessionNotFound(String),
    ConversationNotFound(String),
    RepositoryError(String),
    AgentRunFailed(String),
}

impl std::fmt::Display for OrchestratorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AgentNotAvailable(msg) => write!(f, "Agent not available: {}", msg),
            Self::SpawnFailed(msg) => write!(f, "Failed to spawn agent: {}", msg),
            Self::CommunicationFailed(msg) => write!(f, "Communication failed: {}", msg),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
            Self::SessionNotFound(msg) => write!(f, "Session not found: {}", msg),
            Self::ConversationNotFound(msg) => write!(f, "Conversation not found: {}", msg),
            Self::RepositoryError(msg) => write!(f, "Repository error: {}", msg),
            Self::AgentRunFailed(msg) => write!(f, "Agent run failed: {}", msg),
        }
    }
}

impl std::error::Error for OrchestratorError {}

// ============================================================================
// ClaudeOrchestratorService - Production implementation
// ============================================================================

/// Determines which agent to use based on context type
fn get_agent_name(context_type: &ChatContextType) -> &'static str {
    match context_type {
        ChatContextType::Ideation => "orchestrator-ideation",
        ChatContextType::Task => "chat-task",
        ChatContextType::Project => "chat-project",
        // TaskExecution conversations are created by ExecutionChatService (Phase 15B)
        // and don't use the orchestrator pattern
        ChatContextType::TaskExecution => "worker",
    }
}

/// Production orchestrator service using Claude CLI with MCP integration
///
/// Key changes from previous implementation:
/// - Tool execution is delegated to MCP server (no more execute_tool_call)
/// - Uses --resume flag for follow-up messages (Claude manages conversation context)
/// - Passes RALPHX_AGENT_TYPE env var for MCP tool scoping
/// - Emits Tauri events for real-time UI updates
/// - Tracks agent runs for leave-and-come-back support
pub struct ClaudeOrchestratorService<R: Runtime = tauri::Wry> {
    cli_path: PathBuf,
    plugin_dir: PathBuf,
    default_working_directory: PathBuf,
    chat_message_repo: Arc<dyn ChatMessageRepository>,
    conversation_repo: Arc<dyn ChatConversationRepository>,
    agent_run_repo: Arc<dyn AgentRunRepository>,
    project_repo: Arc<dyn ProjectRepository>,
    task_repo: Arc<dyn TaskRepository>,
    ideation_session_repo: Arc<dyn IdeationSessionRepository>,
    app_handle: Option<AppHandle<R>>,
    model: String,
}

impl<R: Runtime> ClaudeOrchestratorService<R> {
    pub fn new(
        chat_message_repo: Arc<dyn ChatMessageRepository>,
        conversation_repo: Arc<dyn ChatConversationRepository>,
        agent_run_repo: Arc<dyn AgentRunRepository>,
        project_repo: Arc<dyn ProjectRepository>,
        task_repo: Arc<dyn TaskRepository>,
        ideation_session_repo: Arc<dyn IdeationSessionRepository>,
    ) -> Self {
        let cli_path = which::which("claude").unwrap_or_else(|_| PathBuf::from("claude"));
        // Default working directory (fallback if project lookup fails)
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let default_working_directory = cwd
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or(cwd);
        let plugin_dir = default_working_directory.join("ralphx-plugin");

        Self {
            cli_path,
            plugin_dir,
            default_working_directory,
            chat_message_repo,
            conversation_repo,
            agent_run_repo,
            project_repo,
            task_repo,
            ideation_session_repo,
            app_handle: None,
            model: "sonnet".to_string(),
        }
    }

    pub fn with_cli_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.cli_path = path.into();
        self
    }

    pub fn with_plugin_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.plugin_dir = path.into();
        self
    }

    pub fn with_working_directory(mut self, path: impl Into<PathBuf>) -> Self {
        self.default_working_directory = path.into();
        self
    }

    /// Resolve the project's working directory from a context
    async fn resolve_project_working_directory(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> PathBuf {
        let project_id = match context_type {
            ChatContextType::Project => Some(ProjectId::from_string(context_id.to_string())),
            ChatContextType::Task | ChatContextType::TaskExecution => {
                // Get project_id from task
                if let Ok(Some(task)) = self
                    .task_repo
                    .get_by_id(&TaskId::from_string(context_id.to_string()))
                    .await
                {
                    Some(task.project_id)
                } else {
                    None
                }
            }
            ChatContextType::Ideation => {
                // Get project_id from ideation session
                if let Ok(Some(session)) = self
                    .ideation_session_repo
                    .get_by_id(&IdeationSessionId::from_string(context_id.to_string()))
                    .await
                {
                    Some(session.project_id)
                } else {
                    None
                }
            }
        };

        // Look up the project's working directory
        if let Some(pid) = project_id {
            if let Ok(Some(project)) = self.project_repo.get_by_id(&pid).await {
                return PathBuf::from(&project.working_directory);
            }
        }

        // Fallback to default
        self.default_working_directory.clone()
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_app_handle(mut self, app_handle: AppHandle<R>) -> Self {
        self.app_handle = Some(app_handle);
        self
    }

    /// Emit a Tauri event if app_handle is available
    fn emit_event(&self, event: &str, payload: impl Serialize + Clone) {
        if let Some(ref handle) = self.app_handle {
            let _ = handle.emit(event, payload);
        }
    }

    /// Get or create a conversation for the given ideation session
    async fn get_or_create_conversation(
        &self,
        session_id: &IdeationSessionId,
    ) -> Result<ChatConversation, OrchestratorError> {
        self.get_or_create_conversation_for_context(ChatContextType::Ideation, session_id.as_str())
            .await
    }

    /// Get or create a conversation for any context type
    async fn get_or_create_conversation_for_context(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> Result<ChatConversation, OrchestratorError> {
        use crate::domain::entities::{ProjectId, TaskId};

        // Try to get existing active conversation
        if let Some(conv) = self
            .conversation_repo
            .get_active_for_context(context_type, context_id)
            .await
            .map_err(|e| OrchestratorError::RepositoryError(e.to_string()))?
        {
            return Ok(conv);
        }

        // Create new conversation based on context type
        let conv = match context_type {
            ChatContextType::Ideation => {
                ChatConversation::new_ideation(IdeationSessionId::from_string(context_id))
            }
            ChatContextType::Task => {
                ChatConversation::new_task(TaskId::from_string(context_id.to_string()))
            }
            ChatContextType::Project => {
                ChatConversation::new_project(ProjectId::from_string(context_id.to_string()))
            }
            ChatContextType::TaskExecution => {
                ChatConversation::new_task_execution(TaskId::from_string(context_id.to_string()))
            }
        };

        self.conversation_repo
            .create(conv.clone())
            .await
            .map_err(|e| OrchestratorError::RepositoryError(e.to_string()))
    }

    /// Build the initial prompt for any context type
    fn build_context_initial_prompt(
        context_type: ChatContextType,
        context_id: &str,
        user_message: &str,
    ) -> String {
        match context_type {
            ChatContextType::Ideation => {
                format!(
                    "RalphX Ideation Session ID: {}\n\n\
                     User's message: {}",
                    context_id, user_message
                )
            }
            ChatContextType::Task => {
                format!(
                    "RalphX Task ID: {}\n\n\
                     You are helping the user with questions about this specific task.\n\n\
                     User's message: {}",
                    context_id, user_message
                )
            }
            ChatContextType::Project => {
                format!(
                    "RalphX Project ID: {}\n\n\
                     You are helping the user with project-level questions and suggestions.\n\n\
                     User's message: {}",
                    context_id, user_message
                )
            }
            ChatContextType::TaskExecution => {
                // TaskExecution uses the worker agent, which has different context needs
                format!(
                    "RalphX Task Execution - Task ID: {}\n\n\
                     User's message: {}",
                    context_id, user_message
                )
            }
        }
    }

    /// Create a Claude CLI command with appropriate flags
    fn build_command(
        &self,
        conversation: &ChatConversation,
        user_message: &str,
        session_id: &IdeationSessionId,
        working_directory: &Path,
    ) -> Command {
        self.build_command_for_context(conversation, user_message, session_id.as_str(), working_directory)
    }

    /// Create a Claude CLI command for any context type
    fn build_command_for_context(
        &self,
        conversation: &ChatConversation,
        user_message: &str,
        context_id: &str,
        working_directory: &Path,
    ) -> Command {
        let mut cmd = build_base_cli_command(&self.cli_path, &self.plugin_dir);

        // Pass agent type for MCP tool scoping
        let agent_name = get_agent_name(&conversation.context_type);
        cmd.env("RALPHX_AGENT_TYPE", agent_name);

        // First message vs follow-up
        let (prompt, resume_session, agent) = if let Some(ref claude_session_id) = conversation.claude_session_id {
            (user_message.to_string(), Some(claude_session_id.as_str()), None)
        } else {
            let initial_prompt = Self::build_context_initial_prompt(
                conversation.context_type,
                context_id,
                user_message,
            );
            (initial_prompt, None, Some(agent_name))
        };

        add_prompt_args(&mut cmd, &prompt, agent, resume_session);
        configure_spawn(&mut cmd, working_directory);

        cmd
    }

    /// Process streaming output from Claude CLI
    /// Returns accumulated text, tool calls, content blocks, and claude_session_id
    async fn process_stream(
        &self,
        mut child: tokio::process::Child,
        conversation_id: &ChatConversationId,
    ) -> Result<(String, Vec<ToolCall>, Vec<ContentBlockItem>, Option<String>), OrchestratorError> {
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| OrchestratorError::SpawnFailed("Failed to capture stdout".to_string()))?;

        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        let mut processor = StreamProcessor::new();

        let conversation_id_str = conversation_id.as_str();
        let mut line_count = 0;

        while let Some(line) = lines
            .next_line()
            .await
            .map_err(|e| OrchestratorError::CommunicationFailed(e.to_string()))?
        {
            line_count += 1;
            tracing::debug!("Stream line {}: {}", line_count, &line[..line.len().min(100)]);

            if let Some(msg) = StreamProcessor::parse_line(&line) {
                let events = processor.process_message(msg);

                // Emit Tauri events based on processor events
                for event in events {
                    match event {
                        ProcessorStreamEvent::TextChunk(text) => {
                            self.emit_event(
                                "chat:chunk",
                                ChatChunkPayload {
                                    text,
                                    conversation_id: conversation_id_str.to_string(),
                                },
                            );
                        }
                        ProcessorStreamEvent::ToolCallStarted { name, id } => {
                            self.emit_event(
                                "chat:tool_call",
                                ChatToolCallPayload {
                                    tool_name: name,
                                    tool_id: id,
                                    arguments: serde_json::Value::Null,
                                    result: None,
                                    conversation_id: conversation_id_str.to_string(),
                                },
                            );
                        }
                        ProcessorStreamEvent::ToolCallCompleted(tool_call) => {
                            self.emit_event(
                                "chat:tool_call",
                                ChatToolCallPayload {
                                    tool_name: tool_call.name.clone(),
                                    tool_id: tool_call.id.clone(),
                                    arguments: tool_call.arguments.clone(),
                                    result: None,
                                    conversation_id: conversation_id_str.to_string(),
                                },
                            );
                        }
                        ProcessorStreamEvent::SessionId(_) => {
                            // Session ID is captured in processor.finish()
                        }
                        ProcessorStreamEvent::ToolResultReceived {
                            tool_use_id,
                            result,
                        } => {
                            // Re-emit tool call with result
                            self.emit_event(
                                "chat:tool_call",
                                ChatToolCallPayload {
                                    tool_name: format!("result:{}", tool_use_id),
                                    tool_id: Some(tool_use_id),
                                    arguments: serde_json::Value::Null,
                                    result: Some(result),
                                    conversation_id: conversation_id_str.to_string(),
                                },
                            );
                        }
                    }
                }
            }
        }

        let result = processor.finish();
        println!(
            "[DEBUG] Stream loop complete: {} lines, response_len={}, tool_calls={}",
            line_count, result.response_text.len(), result.tool_calls.len()
        );

        // Capture stderr for error reporting
        let stderr = child.stderr.take();

        // Wait for process to complete
        let status = child
            .wait()
            .await
            .map_err(|e| OrchestratorError::CommunicationFailed(e.to_string()))?;

        println!("[DEBUG] Process exit status: {:?}", status);

        // Always try to read stderr for debugging
        let stderr_content = if let Some(mut stderr) = stderr {
            use tokio::io::AsyncReadExt;
            let mut buf = String::new();
            let _ = stderr.read_to_string(&mut buf).await;
            buf
        } else {
            String::new()
        };

        if !stderr_content.is_empty() {
            println!("[DEBUG] stderr output: {}", stderr_content);
        }

        if !status.success() && result.response_text.is_empty() {
            let error_msg = if stderr_content.is_empty() {
                "Agent exited with non-zero status".to_string()
            } else {
                format!("Agent failed: {}", stderr_content.trim())
            };

            return Err(OrchestratorError::AgentRunFailed(error_msg));
        }

        Ok((result.response_text, result.tool_calls, result.content_blocks, result.session_id))
    }

}

#[async_trait]
impl<R: Runtime> OrchestratorService for ClaudeOrchestratorService<R> {
    async fn send_message(
        &self,
        session_id: &IdeationSessionId,
        user_message: &str,
    ) -> Result<OrchestratorResult, OrchestratorError> {
        // 1. Get or create conversation for this context
        let conversation = self.get_or_create_conversation(session_id).await?;
        let conversation_id = conversation.id;
        let conversation_id_str = conversation_id.as_str();

        // 2. Create agent run record (status: running)
        let agent_run = AgentRun::new(conversation_id);
        let agent_run_id = agent_run.id;
        self.agent_run_repo
            .create(agent_run)
            .await
            .map_err(|e| OrchestratorError::RepositoryError(e.to_string()))?;

        // Emit run started event
        self.emit_event(
            "chat:run_started",
            serde_json::json!({
                "run_id": agent_run_id.as_str(),
                "conversation_id": conversation_id_str,
            }),
        );

        // 3. Store user message immediately (with conversation_id)
        let mut user_msg = ChatMessage::user_in_session(session_id.clone(), user_message);
        user_msg.conversation_id = Some(conversation_id);
        let user_msg_id = user_msg.id.as_str().to_string();
        self.chat_message_repo
            .create(user_msg)
            .await
            .map_err(|e| OrchestratorError::RepositoryError(e.to_string()))?;

        // Emit message created event
        self.emit_event(
            "chat:message_created",
            ChatMessageCreatedPayload {
                message_id: user_msg_id,
                conversation_id: conversation_id_str.to_string(),
                role: "user".to_string(),
                content: user_message.to_string(),
            },
        );

        // 4. Resolve project's working directory
        let working_directory = self
            .resolve_project_working_directory(ChatContextType::Ideation, session_id.as_str())
            .await;

        // 5. Spawn Claude CLI and process streaming output
        let mut cmd = self.build_command(&conversation, user_message, session_id, &working_directory);
        let child = cmd.spawn().map_err(|e| OrchestratorError::SpawnFailed(e.to_string()))?;
        let result = self.process_stream(child, &conversation_id).await;

        // Handle result (complete or fail the agent run)
        match result {
            Ok((response_text, tool_calls, content_blocks, claude_session_id)) => {
                // 6. If this was a new session, store Claude's session_id
                if conversation.claude_session_id.is_none() {
                    if let Some(ref sess_id) = claude_session_id {
                        self.conversation_repo
                            .update_claude_session_id(&conversation_id, sess_id)
                            .await
                            .map_err(|e| OrchestratorError::RepositoryError(e.to_string()))?;
                    }
                }

                // 7. Store assistant message with tool_calls and content_blocks (for UI display)
                if !response_text.is_empty() || !tool_calls.is_empty() {
                    let mut assistant_msg =
                        ChatMessage::orchestrator_in_session(session_id.clone(), &response_text);
                    assistant_msg.conversation_id = Some(conversation_id);

                    // Serialize tool_calls to JSON
                    if !tool_calls.is_empty() {
                        assistant_msg.tool_calls =
                            Some(serde_json::to_string(&tool_calls).unwrap_or_default());
                    }

                    // Serialize content_blocks to JSON (preserves interleaving order)
                    if !content_blocks.is_empty() {
                        assistant_msg.content_blocks =
                            Some(serde_json::to_string(&content_blocks).unwrap_or_default());
                    }

                    let assistant_msg_id = assistant_msg.id.as_str().to_string();
                    self.chat_message_repo
                        .create(assistant_msg)
                        .await
                        .map_err(|e| OrchestratorError::RepositoryError(e.to_string()))?;

                    // Emit message created event for assistant message
                    self.emit_event(
                        "chat:message_created",
                        ChatMessageCreatedPayload {
                            message_id: assistant_msg_id,
                            conversation_id: conversation_id_str.to_string(),
                            role: "orchestrator".to_string(),
                            content: response_text.clone(),
                        },
                    );
                }

                // 8. Complete agent run
                self.agent_run_repo
                    .complete(&agent_run_id)
                    .await
                    .map_err(|e| OrchestratorError::RepositoryError(e.to_string()))?;

                // Emit run completed event
                self.emit_event(
                    "chat:run_completed",
                    ChatRunCompletedPayload {
                        conversation_id: conversation_id_str.to_string(),
                        claude_session_id: claude_session_id.clone(),
                    },
                );

                Ok(OrchestratorResult {
                    response_text,
                    tool_calls,
                    claude_session_id,
                    conversation_id: Some(conversation_id),
                })
            }
            Err(e) => {
                // Fail the agent run
                self.agent_run_repo
                    .fail(&agent_run_id, &e.to_string())
                    .await
                    .map_err(|err| OrchestratorError::RepositoryError(err.to_string()))?;

                // Emit error event
                self.emit_event(
                    "chat:error",
                    serde_json::json!({
                        "conversation_id": conversation_id_str,
                        "error": e.to_string(),
                    }),
                );

                Err(e)
            }
        }
    }

    async fn send_context_message(
        &self,
        context_type: ChatContextType,
        context_id: &str,
        user_message: &str,
    ) -> Result<OrchestratorResult, OrchestratorError> {
        use crate::domain::entities::{MessageRole, ProjectId, TaskId};

        // 1. Get or create conversation for this context
        let conversation = self
            .get_or_create_conversation_for_context(context_type, context_id)
            .await?;
        let conversation_id = conversation.id;
        let conversation_id_str = conversation_id.as_str();

        // 2. Create agent run record (status: running)
        let agent_run = AgentRun::new(conversation_id);
        let agent_run_id = agent_run.id;
        self.agent_run_repo
            .create(agent_run)
            .await
            .map_err(|e| OrchestratorError::RepositoryError(e.to_string()))?;

        // Emit run started event
        self.emit_event(
            "chat:run_started",
            serde_json::json!({
                "run_id": agent_run_id.as_str(),
                "conversation_id": conversation_id_str,
            }),
        );

        // 3. Store user message immediately (with conversation_id)
        // Create message based on context type
        let mut user_msg = match context_type {
            ChatContextType::Ideation => {
                ChatMessage::user_in_session(
                    IdeationSessionId::from_string(context_id),
                    user_message,
                )
            }
            ChatContextType::Task | ChatContextType::TaskExecution => {
                ChatMessage::user_about_task(
                    TaskId::from_string(context_id.to_string()),
                    user_message,
                )
            }
            ChatContextType::Project => {
                ChatMessage::user_in_project(
                    ProjectId::from_string(context_id.to_string()),
                    user_message,
                )
            }
        };
        user_msg.conversation_id = Some(conversation_id);
        let user_msg_id = user_msg.id.as_str().to_string();
        self.chat_message_repo
            .create(user_msg)
            .await
            .map_err(|e| OrchestratorError::RepositoryError(e.to_string()))?;

        // Emit message created event
        self.emit_event(
            "chat:message_created",
            ChatMessageCreatedPayload {
                message_id: user_msg_id,
                conversation_id: conversation_id_str.to_string(),
                role: "user".to_string(),
                content: user_message.to_string(),
            },
        );

        // 4. Resolve project's working directory
        let working_directory = self
            .resolve_project_working_directory(context_type, context_id)
            .await;

        // 5. Spawn Claude CLI and process streaming output
        let mut cmd = self.build_command_for_context(&conversation, user_message, context_id, &working_directory);
        let child = cmd.spawn().map_err(|e| OrchestratorError::SpawnFailed(e.to_string()))?;
        let result = self.process_stream(child, &conversation_id).await;
        tracing::info!("Stream processing complete: success={}", result.is_ok());

        // Handle result (complete or fail the agent run)
        match result {
            Ok((response_text, tool_calls, content_blocks, claude_session_id)) => {
                // 6. If this was a new session, store Claude's session_id
                if conversation.claude_session_id.is_none() {
                    if let Some(ref sess_id) = claude_session_id {
                        self.conversation_repo
                            .update_claude_session_id(&conversation_id, sess_id)
                            .await
                            .map_err(|e| OrchestratorError::RepositoryError(e.to_string()))?;
                    }
                }

                // 7. Store assistant message with tool_calls and content_blocks (for UI display)
                if !response_text.is_empty() || !tool_calls.is_empty() {
                    // Create assistant message based on context type
                    let mut assistant_msg = match context_type {
                        ChatContextType::Ideation => ChatMessage::orchestrator_in_session(
                            IdeationSessionId::from_string(context_id),
                            &response_text,
                        ),
                        ChatContextType::Task | ChatContextType::TaskExecution => {
                            let mut msg = ChatMessage::user_about_task(
                                TaskId::from_string(context_id.to_string()),
                                &response_text,
                            );
                            msg.role = MessageRole::Orchestrator;
                            msg
                        }
                        ChatContextType::Project => {
                            let mut msg = ChatMessage::user_in_project(
                                ProjectId::from_string(context_id.to_string()),
                                &response_text,
                            );
                            msg.role = MessageRole::Orchestrator;
                            msg
                        }
                    };
                    assistant_msg.conversation_id = Some(conversation_id);

                    // Serialize tool_calls to JSON
                    if !tool_calls.is_empty() {
                        assistant_msg.tool_calls =
                            Some(serde_json::to_string(&tool_calls).unwrap_or_default());
                    }

                    // Serialize content_blocks to JSON (preserves interleaving order)
                    if !content_blocks.is_empty() {
                        assistant_msg.content_blocks =
                            Some(serde_json::to_string(&content_blocks).unwrap_or_default());
                    }

                    let assistant_msg_id = assistant_msg.id.as_str().to_string();
                    self.chat_message_repo
                        .create(assistant_msg)
                        .await
                        .map_err(|e| OrchestratorError::RepositoryError(e.to_string()))?;

                    // Emit message created event for assistant message
                    self.emit_event(
                        "chat:message_created",
                        ChatMessageCreatedPayload {
                            message_id: assistant_msg_id,
                            conversation_id: conversation_id_str.to_string(),
                            role: "orchestrator".to_string(),
                            content: response_text.clone(),
                        },
                    );
                }

                // 8. Complete agent run
                self.agent_run_repo
                    .complete(&agent_run_id)
                    .await
                    .map_err(|e| OrchestratorError::RepositoryError(e.to_string()))?;

                // Emit run completed event
                self.emit_event(
                    "chat:run_completed",
                    ChatRunCompletedPayload {
                        conversation_id: conversation_id_str.to_string(),
                        claude_session_id: claude_session_id.clone(),
                    },
                );

                Ok(OrchestratorResult {
                    response_text,
                    tool_calls,
                    claude_session_id,
                    conversation_id: Some(conversation_id),
                })
            }
            Err(e) => {
                // Fail the agent run
                self.agent_run_repo
                    .fail(&agent_run_id, &e.to_string())
                    .await
                    .map_err(|err| OrchestratorError::RepositoryError(err.to_string()))?;

                // Emit error event
                self.emit_event(
                    "chat:error",
                    serde_json::json!({
                        "conversation_id": conversation_id_str,
                        "error": e.to_string(),
                    }),
                );

                Err(e)
            }
        }
    }

    fn send_message_streaming(
        &self,
        session_id: IdeationSessionId,
        user_message: String,
    ) -> mpsc::Receiver<OrchestratorEvent> {
        let (tx, rx) = mpsc::channel(100);

        let cli_path = self.cli_path.clone();
        let plugin_dir = self.plugin_dir.clone();
        let working_directory = self.default_working_directory.clone();
        let chat_repo = Arc::clone(&self.chat_message_repo);
        let conversation_repo = Arc::clone(&self.conversation_repo);
        let agent_run_repo = Arc::clone(&self.agent_run_repo);

        tokio::spawn(async move {
            // Get or create conversation
            let context_type = ChatContextType::Ideation;
            let context_id = session_id.as_str();

            let conversation = match conversation_repo
                .get_active_for_context(context_type, context_id)
                .await
            {
                Ok(Some(conv)) => conv,
                Ok(None) => {
                    let conv = ChatConversation::new_ideation(session_id.clone());
                    match conversation_repo.create(conv.clone()).await {
                        Ok(created) => created,
                        Err(e) => {
                            let _ = tx
                                .send(OrchestratorEvent::Error {
                                    conversation_id: None,
                                    error: e.to_string(),
                                })
                                .await;
                            return;
                        }
                    }
                }
                Err(e) => {
                    let _ = tx
                        .send(OrchestratorEvent::Error {
                            conversation_id: None,
                            error: e.to_string(),
                        })
                        .await;
                    return;
                }
            };

            let conversation_id = conversation.id;
            let conversation_id_str = conversation_id.as_str();

            // Create agent run
            let agent_run = AgentRun::new(conversation_id);
            let agent_run_id = agent_run.id;
            if let Err(e) = agent_run_repo.create(agent_run).await {
                let _ = tx
                    .send(OrchestratorEvent::Error {
                        conversation_id: Some(conversation_id_str.to_string()),
                        error: e.to_string(),
                    })
                    .await;
                return;
            }

            let _ = tx
                .send(OrchestratorEvent::RunStarted {
                    run_id: agent_run_id.as_str(),
                    conversation_id: conversation_id_str.to_string(),
                })
                .await;

            // Store user message
            let mut user_msg = ChatMessage::user_in_session(session_id.clone(), &user_message);
            user_msg.conversation_id = Some(conversation_id);
            let user_msg_id = user_msg.id.as_str().to_string();
            if let Err(e) = chat_repo.create(user_msg).await {
                let _ = tx
                    .send(OrchestratorEvent::Error {
                        conversation_id: Some(conversation_id_str.to_string()),
                        error: e.to_string(),
                    })
                    .await;
                return;
            }

            let _ = tx
                .send(OrchestratorEvent::MessageCreated {
                    message_id: user_msg_id,
                    conversation_id: conversation_id_str.to_string(),
                })
                .await;

            // Build command
            let agent_name = get_agent_name(&conversation.context_type);

            let (prompt, resume_session, agent) = if let Some(ref claude_session_id) = conversation.claude_session_id {
                (user_message.clone(), Some(claude_session_id.as_str()), None)
            } else {
                let initial_prompt = format!(
                    "RalphX Ideation Session ID: {}\n\nUser's message: {}",
                    session_id.as_str(),
                    &user_message
                );
                (initial_prompt, None, Some(agent_name))
            };

            let mut cmd = build_base_cli_command(&cli_path, &plugin_dir);
            cmd.env("RALPHX_AGENT_TYPE", agent_name);
            add_prompt_args(&mut cmd, &prompt, agent, resume_session);
            configure_spawn(&mut cmd, &working_directory);

            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    let _ = agent_run_repo.fail(&agent_run_id, &e.to_string()).await;
                    let _ = tx
                        .send(OrchestratorEvent::Error {
                            conversation_id: Some(conversation_id_str.to_string()),
                            error: e.to_string(),
                        })
                        .await;
                    return;
                }
            };

            let stdout = match child.stdout.take() {
                Some(s) => s,
                None => {
                    let _ = agent_run_repo
                        .fail(&agent_run_id, "Failed to capture stdout")
                        .await;
                    let _ = tx
                        .send(OrchestratorEvent::Error {
                            conversation_id: Some(conversation_id_str.to_string()),
                            error: "Failed to capture stdout".to_string(),
                        })
                        .await;
                    return;
                }
            };

            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            let mut processor = StreamProcessor::new();

            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(msg) = StreamProcessor::parse_line(&line) {
                    let events = processor.process_message(msg);

                    for event in events {
                        match event {
                            ProcessorStreamEvent::TextChunk(text) => {
                                let _ = tx
                                    .send(OrchestratorEvent::TextChunk {
                                        text,
                                        conversation_id: conversation_id_str.to_string(),
                                    })
                                    .await;
                            }
                            ProcessorStreamEvent::ToolCallStarted { name, id } => {
                                let _ = tx
                                    .send(OrchestratorEvent::ToolCallDetected {
                                        tool_name: name,
                                        tool_id: id,
                                        arguments: serde_json::Value::Null,
                                        conversation_id: conversation_id_str.to_string(),
                                    })
                                    .await;
                            }
                            ProcessorStreamEvent::ToolCallCompleted(tool_call) => {
                                let _ = tx
                                    .send(OrchestratorEvent::ToolCallCompleted {
                                        tool_name: tool_call.name,
                                        tool_id: tool_call.id,
                                        result: None,
                                        conversation_id: conversation_id_str.to_string(),
                                    })
                                    .await;
                            }
                            ProcessorStreamEvent::SessionId(_) => {
                                // Session ID captured in processor.finish()
                            }
                            ProcessorStreamEvent::ToolResultReceived {
                                tool_use_id,
                                result,
                            } => {
                                let _ = tx
                                    .send(OrchestratorEvent::ToolCallCompleted {
                                        tool_name: format!("result:{}", tool_use_id),
                                        tool_id: Some(tool_use_id),
                                        result: Some(result),
                                        conversation_id: conversation_id_str.to_string(),
                                    })
                                    .await;
                            }
                        }
                    }
                }
            }

            let result = processor.finish();

            // Wait for process
            let _ = child.wait().await;

            // Update conversation with claude_session_id if new
            if conversation.claude_session_id.is_none() {
                if let Some(ref sess_id) = result.session_id {
                    let _ = conversation_repo
                        .update_claude_session_id(&conversation_id, sess_id)
                        .await;
                }
            }

            // Store response
            if !result.response_text.is_empty() || !result.tool_calls.is_empty() {
                let mut assistant_msg =
                    ChatMessage::orchestrator_in_session(session_id.clone(), &result.response_text);
                assistant_msg.conversation_id = Some(conversation_id);
                if !result.tool_calls.is_empty() {
                    assistant_msg.tool_calls =
                        Some(serde_json::to_string(&result.tool_calls).unwrap_or_default());
                }
                if !result.content_blocks.is_empty() {
                    assistant_msg.content_blocks =
                        Some(serde_json::to_string(&result.content_blocks).unwrap_or_default());
                }
                let _ = chat_repo.create(assistant_msg).await;
            }

            // Complete agent run
            let _ = agent_run_repo.complete(&agent_run_id).await;

            let _ = tx
                .send(OrchestratorEvent::RunCompleted {
                    conversation_id: conversation_id_str.to_string(),
                    claude_session_id: result.session_id,
                    response_text: result.response_text,
                })
                .await;
        });

        rx
    }

    async fn is_available(&self) -> bool {
        if self.cli_path.exists() {
            return true;
        }
        which::which(&self.cli_path).is_ok()
    }

    async fn get_active_run(
        &self,
        conversation_id: &ChatConversationId,
    ) -> Result<Option<AgentRun>, OrchestratorError> {
        self.agent_run_repo
            .get_active_for_conversation(conversation_id)
            .await
            .map_err(|e| OrchestratorError::RepositoryError(e.to_string()))
    }
}

// ============================================================================
// MockOrchestratorService - For testing
// ============================================================================

/// Mock orchestrator service for testing
pub struct MockOrchestratorService {
    responses: Mutex<Vec<MockResponse>>,
    is_available: Mutex<bool>,
    active_run: Mutex<Option<AgentRun>>,
}

pub struct MockResponse {
    pub text: String,
    pub tool_calls: Vec<ToolCall>,
    pub claude_session_id: Option<String>,
}

impl MockOrchestratorService {
    pub fn new() -> Self {
        Self {
            responses: Mutex::new(Vec::new()),
            is_available: Mutex::new(true),
            active_run: Mutex::new(None),
        }
    }

    pub async fn set_available(&self, available: bool) {
        *self.is_available.lock().await = available;
    }

    pub async fn queue_response(&self, response: MockResponse) {
        self.responses.lock().await.push(response);
    }

    pub async fn queue_text_response(&self, text: impl Into<String>) {
        self.queue_response(MockResponse {
            text: text.into(),
            tool_calls: Vec::new(),
            claude_session_id: None,
        })
        .await;
    }

    pub async fn set_active_run(&self, run: Option<AgentRun>) {
        *self.active_run.lock().await = run;
    }
}

impl Default for MockOrchestratorService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OrchestratorService for MockOrchestratorService {
    async fn send_message(
        &self,
        _session_id: &IdeationSessionId,
        _user_message: &str,
    ) -> Result<OrchestratorResult, OrchestratorError> {
        if !*self.is_available.lock().await {
            return Err(OrchestratorError::AgentNotAvailable(
                "Mock agent not available".to_string(),
            ));
        }

        let mut responses = self.responses.lock().await;
        if let Some(response) = responses.pop() {
            Ok(OrchestratorResult {
                response_text: response.text,
                tool_calls: response.tool_calls,
                claude_session_id: response.claude_session_id,
                conversation_id: None,
            })
        } else {
            Ok(OrchestratorResult {
                response_text: "I'm here to help with your ideation session.".to_string(),
                tool_calls: Vec::new(),
                claude_session_id: None,
                conversation_id: None,
            })
        }
    }

    async fn send_context_message(
        &self,
        _context_type: ChatContextType,
        _context_id: &str,
        _user_message: &str,
    ) -> Result<OrchestratorResult, OrchestratorError> {
        if !*self.is_available.lock().await {
            return Err(OrchestratorError::AgentNotAvailable(
                "Mock agent not available".to_string(),
            ));
        }

        let mut responses = self.responses.lock().await;
        if let Some(response) = responses.pop() {
            Ok(OrchestratorResult {
                response_text: response.text,
                tool_calls: response.tool_calls,
                claude_session_id: response.claude_session_id,
                conversation_id: None,
            })
        } else {
            Ok(OrchestratorResult {
                response_text: "I'm here to help.".to_string(),
                tool_calls: Vec::new(),
                claude_session_id: None,
                conversation_id: None,
            })
        }
    }

    fn send_message_streaming(
        &self,
        _session_id: IdeationSessionId,
        _user_message: String,
    ) -> mpsc::Receiver<OrchestratorEvent> {
        let (tx, rx) = mpsc::channel(10);

        let conversation_id = ChatConversationId::new().as_str();

        tokio::spawn(async move {
            let _ = tx
                .send(OrchestratorEvent::TextChunk {
                    text: "Mock streaming response".to_string(),
                    conversation_id: conversation_id.to_string(),
                })
                .await;
            let _ = tx
                .send(OrchestratorEvent::RunCompleted {
                    conversation_id: conversation_id.to_string(),
                    claude_session_id: None,
                    response_text: "Mock streaming response".to_string(),
                })
                .await;
        });

        rx
    }

    async fn is_available(&self) -> bool {
        *self.is_available.lock().await
    }

    async fn get_active_run(
        &self,
        _conversation_id: &ChatConversationId,
    ) -> Result<Option<AgentRun>, OrchestratorError> {
        Ok(self.active_run.lock().await.clone())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_service_default_response() {
        let service = MockOrchestratorService::new();
        let session_id = IdeationSessionId::new();

        let result = service.send_message(&session_id, "Hello").await.unwrap();

        assert!(result.response_text.contains("help"));
        assert!(result.tool_calls.is_empty());
        assert!(result.claude_session_id.is_none());
    }

    #[tokio::test]
    async fn test_mock_service_queued_response() {
        let service = MockOrchestratorService::new();
        let session_id = IdeationSessionId::new();

        service.queue_text_response("Custom response").await;

        let result = service.send_message(&session_id, "Hello").await.unwrap();

        assert_eq!(result.response_text, "Custom response");
    }

    #[tokio::test]
    async fn test_mock_service_queued_response_with_session_id() {
        let service = MockOrchestratorService::new();
        let session_id = IdeationSessionId::new();

        service
            .queue_response(MockResponse {
                text: "Response with session".to_string(),
                tool_calls: Vec::new(),
                claude_session_id: Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
            })
            .await;

        let result = service.send_message(&session_id, "Hello").await.unwrap();

        assert_eq!(result.response_text, "Response with session");
        assert_eq!(
            result.claude_session_id,
            Some("550e8400-e29b-41d4-a716-446655440000".to_string())
        );
    }

    #[tokio::test]
    async fn test_mock_service_not_available() {
        let service = MockOrchestratorService::new();
        let session_id = IdeationSessionId::new();

        service.set_available(false).await;

        let result = service.send_message(&session_id, "Hello").await;

        assert!(matches!(
            result,
            Err(OrchestratorError::AgentNotAvailable(_))
        ));
    }

    #[tokio::test]
    async fn test_mock_service_is_available() {
        let service = MockOrchestratorService::new();

        assert!(service.is_available().await);

        service.set_available(false).await;
        assert!(!service.is_available().await);
    }

    #[tokio::test]
    async fn test_mock_service_streaming() {
        let service = MockOrchestratorService::new();
        let session_id = IdeationSessionId::new();

        let mut rx = service.send_message_streaming(session_id, "Hello".to_string());

        let mut events = Vec::new();
        while let Some(event) = rx.recv().await {
            events.push(event);
        }

        assert!(!events.is_empty());
        assert!(events
            .iter()
            .any(|e| matches!(e, OrchestratorEvent::TextChunk { .. })));
        assert!(events
            .iter()
            .any(|e| matches!(e, OrchestratorEvent::RunCompleted { .. })));
    }

    #[tokio::test]
    async fn test_mock_service_get_active_run() {
        let service = MockOrchestratorService::new();
        let conversation_id = ChatConversationId::new();

        // Initially no active run
        let run = service.get_active_run(&conversation_id).await.unwrap();
        assert!(run.is_none());

        // Set an active run
        let agent_run = AgentRun::new(conversation_id);
        service.set_active_run(Some(agent_run.clone())).await;

        let run = service.get_active_run(&conversation_id).await.unwrap();
        assert!(run.is_some());
        assert_eq!(run.unwrap().id, agent_run.id);
    }

    // Stream parsing tests moved to infrastructure::agents::claude::stream_processor
    // See stream_processor.rs for comprehensive parsing tests

    #[test]
    fn test_tool_call_with_id() {
        let tool_call = ToolCall {
            id: Some("toolu_01ABC".to_string()),
            name: "create_task_proposal".to_string(),
            arguments: serde_json::json!({"title": "Test task"}),
            result: None,
        };

        assert_eq!(tool_call.id, Some("toolu_01ABC".to_string()));
        assert_eq!(tool_call.name, "create_task_proposal");
    }

    #[test]
    fn test_tool_call_result_success() {
        let result = ToolCallResult {
            tool_name: "test_tool".to_string(),
            success: true,
            result: Some(serde_json::json!({"key": "value"})),
            error: None,
        };

        assert!(result.success);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_tool_call_result_failure() {
        let result = ToolCallResult {
            tool_name: "test_tool".to_string(),
            success: false,
            result: None,
            error: Some("Something went wrong".to_string()),
        };

        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_orchestrator_error_display() {
        let err = OrchestratorError::AgentNotAvailable("test".to_string());
        assert!(err.to_string().contains("Agent not available"));

        let err = OrchestratorError::SpawnFailed("test".to_string());
        assert!(err.to_string().contains("spawn"));

        let err = OrchestratorError::AgentRunFailed("test".to_string());
        assert!(err.to_string().contains("Agent run failed"));
    }

    #[test]
    fn test_get_agent_name() {
        assert_eq!(
            get_agent_name(&ChatContextType::Ideation),
            "orchestrator-ideation"
        );
        assert_eq!(get_agent_name(&ChatContextType::Task), "chat-task");
        assert_eq!(get_agent_name(&ChatContextType::Project), "chat-project");
    }

    #[test]
    fn test_orchestrator_event_serialization() {
        // Test that events can be serialized (needed for Tauri events)
        let event = OrchestratorEvent::TextChunk {
            text: "Hello".to_string(),
            conversation_id: "conv-123".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("TextChunk"));
        assert!(json.contains("Hello"));

        let event = OrchestratorEvent::RunCompleted {
            conversation_id: "conv-123".to_string(),
            claude_session_id: Some("sess-456".to_string()),
            response_text: "Done".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("RunCompleted"));
        assert!(json.contains("sess-456"));
    }

    #[test]
    fn test_chat_payloads_serialization() {
        let payload = ChatChunkPayload {
            text: "Hello".to_string(),
            conversation_id: "conv-123".to_string(),
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("Hello"));
        assert!(json.contains("conv-123"));

        let payload = ChatRunCompletedPayload {
            conversation_id: "conv-123".to_string(),
            claude_session_id: Some("sess-456".to_string()),
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("conv-123"));
        assert!(json.contains("sess-456"));
    }
}
