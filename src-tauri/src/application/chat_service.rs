// Unified Chat Service
//
// Consolidates OrchestratorService and ExecutionChatService into a single service
// with consistent patterns:
// - Background spawn pattern for ALL contexts (returns immediately, processes in background)
// - Unified event namespace: agent:* instead of chat:*/execution:*
// - Backend message queue with context-aware routing
// - Task state transitions only for TaskExecution context
//
// This service replaces both:
// - OrchestratorService (ideation, task, project contexts)
// - ExecutionChatService (task_execution context)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::domain::entities::{
    AgentRun, ChatConversation, ChatConversationId, ChatContextType, ChatMessage, ChatMessageId,
    IdeationSessionId, MessageRole, ProjectId, TaskId,
};
use crate::domain::repositories::{
    AgentRunRepository, ChatConversationRepository, ChatMessageRepository,
    IdeationSessionRepository, ProjectRepository, TaskRepository,
};
use crate::domain::services::{MessageQueue, QueuedMessage, RunningAgentKey, RunningAgentRegistry};
use crate::infrastructure::agents::claude::{
    add_prompt_args, build_base_cli_command, configure_spawn, ContentBlockItem, StreamEvent,
    StreamProcessor, ToolCall,
};

// ============================================================================
// Types
// ============================================================================

/// Result from sending a message (returns immediately while processing continues in background)
#[derive(Debug, Clone, Serialize)]
pub struct SendResult {
    /// The conversation ID for this chat
    pub conversation_id: String,
    /// The agent run ID tracking this execution
    pub agent_run_id: String,
    /// Whether this is a new conversation (first message)
    pub is_new_conversation: bool,
}

/// A conversation with its messages
#[derive(Debug, Clone)]
pub struct ChatConversationWithMessages {
    pub conversation: ChatConversation,
    pub messages: Vec<ChatMessage>,
}

// ============================================================================
// Unified Event Payloads (agent:* namespace)
// ============================================================================

/// Payload for agent:run_started event
#[derive(Debug, Clone, Serialize)]
pub struct AgentRunStartedPayload {
    pub run_id: String,
    pub conversation_id: String,
    pub context_type: String,
    pub context_id: String,
}

/// Payload for agent:chunk event
#[derive(Debug, Clone, Serialize)]
pub struct AgentChunkPayload {
    pub text: String,
    pub conversation_id: String,
    pub context_type: String,
    pub context_id: String,
}

/// Payload for agent:tool_call event
#[derive(Debug, Clone, Serialize)]
pub struct AgentToolCallPayload {
    pub tool_name: String,
    pub tool_id: Option<String>,
    pub arguments: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub conversation_id: String,
    pub context_type: String,
    pub context_id: String,
}

/// Payload for agent:message_created event
#[derive(Debug, Clone, Serialize)]
pub struct AgentMessageCreatedPayload {
    pub message_id: String,
    pub conversation_id: String,
    pub context_type: String,
    pub context_id: String,
    pub role: String,
    pub content: String,
}

/// Payload for agent:run_completed event
#[derive(Debug, Clone, Serialize)]
pub struct AgentRunCompletedPayload {
    pub conversation_id: String,
    pub context_type: String,
    pub context_id: String,
    pub claude_session_id: Option<String>,
}

/// Payload for agent:error event
#[derive(Debug, Clone, Serialize)]
pub struct AgentErrorPayload {
    pub conversation_id: Option<String>,
    pub context_type: String,
    pub context_id: String,
    pub error: String,
    pub stderr: Option<String>,
}

/// Payload for agent:queue_sent event
#[derive(Debug, Clone, Serialize)]
pub struct AgentQueueSentPayload {
    pub message_id: String,
    pub conversation_id: String,
    pub context_type: String,
    pub context_id: String,
}

// ============================================================================
// Error type
// ============================================================================

#[derive(Debug, Clone)]
pub enum ChatServiceError {
    AgentNotAvailable(String),
    SpawnFailed(String),
    CommunicationFailed(String),
    ParseError(String),
    ContextNotFound(String),
    ConversationNotFound(String),
    RepositoryError(String),
    AgentRunFailed(String),
}

impl std::fmt::Display for ChatServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AgentNotAvailable(msg) => write!(f, "Agent not available: {}", msg),
            Self::SpawnFailed(msg) => write!(f, "Failed to spawn agent: {}", msg),
            Self::CommunicationFailed(msg) => write!(f, "Communication failed: {}", msg),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
            Self::ContextNotFound(msg) => write!(f, "Context not found: {}", msg),
            Self::ConversationNotFound(msg) => write!(f, "Conversation not found: {}", msg),
            Self::RepositoryError(msg) => write!(f, "Repository error: {}", msg),
            Self::AgentRunFailed(msg) => write!(f, "Agent run failed: {}", msg),
        }
    }
}

impl std::error::Error for ChatServiceError {}

// ============================================================================
// ChatService trait
// ============================================================================

/// Unified chat service for all context types
///
/// Key features:
/// - Background spawn pattern: send_message returns immediately
/// - Unified event namespace: all events use agent:* prefix
/// - Backend message queue: messages can be queued while agent is running
/// - Context-aware: routes to appropriate agent based on context type
/// - Task transitions: only TaskExecution context triggers state changes
#[async_trait]
pub trait ChatService: Send + Sync {
    /// Send a message in a context-aware conversation
    ///
    /// Returns immediately with conversation_id and agent_run_id.
    /// Processing happens in background, with events emitted via Tauri.
    ///
    /// Event flow:
    /// 1. agent:run_started
    /// 2. agent:message_created (user message)
    /// 3. agent:chunk (streaming text)
    /// 4. agent:tool_call (tool invocations)
    /// 5. agent:message_created (assistant message)
    /// 6. agent:run_completed or agent:error
    async fn send_message(
        &self,
        context_type: ChatContextType,
        context_id: &str,
        message: &str,
    ) -> Result<SendResult, ChatServiceError>;

    /// Queue a message to be sent when the current agent run completes
    ///
    /// The message is held in the backend queue and automatically sent
    /// via --resume when the current run finishes.
    ///
    /// If `client_id` is provided, that ID will be used for the message,
    /// allowing frontend and backend to use the same ID for tracking.
    async fn queue_message(
        &self,
        context_type: ChatContextType,
        context_id: &str,
        content: &str,
        client_id: Option<&str>,
    ) -> Result<QueuedMessage, ChatServiceError>;

    /// Get all queued messages for a context
    async fn get_queued_messages(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> Result<Vec<QueuedMessage>, ChatServiceError>;

    /// Delete a queued message before it's sent
    async fn delete_queued_message(
        &self,
        context_type: ChatContextType,
        context_id: &str,
        message_id: &str,
    ) -> Result<bool, ChatServiceError>;

    /// Get or create a conversation for a context
    async fn get_or_create_conversation(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> Result<ChatConversation, ChatServiceError>;

    /// Get a conversation by ID with all its messages
    async fn get_conversation_with_messages(
        &self,
        conversation_id: &ChatConversationId,
    ) -> Result<Option<ChatConversationWithMessages>, ChatServiceError>;

    /// List all conversations for a context
    async fn list_conversations(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> Result<Vec<ChatConversation>, ChatServiceError>;

    /// Get the active agent run for a conversation
    async fn get_active_run(
        &self,
        conversation_id: &ChatConversationId,
    ) -> Result<Option<AgentRun>, ChatServiceError>;

    /// Check if the chat service (Claude CLI) is available
    async fn is_available(&self) -> bool;

    /// Stop a running agent for a context
    ///
    /// Sends SIGTERM to the running agent process and emits an agent:stopped event.
    /// Returns true if an agent was stopped, false if no agent was running.
    async fn stop_agent(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> Result<bool, ChatServiceError>;

    /// Check if an agent is running for a context
    async fn is_agent_running(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> bool;
}

// ============================================================================
// ClaudeChatService - Production implementation
// ============================================================================

/// Determines which agent to use based on context type
fn get_agent_name(context_type: &ChatContextType) -> &'static str {
    match context_type {
        ChatContextType::Ideation => "orchestrator-ideation",
        ChatContextType::Task => "chat-task",
        ChatContextType::Project => "chat-project",
        ChatContextType::TaskExecution => "worker",
    }
}

/// Get the message role for a context type
fn get_assistant_role(context_type: &ChatContextType) -> MessageRole {
    match context_type {
        ChatContextType::TaskExecution => MessageRole::Worker,
        _ => MessageRole::Orchestrator,
    }
}

/// Production implementation using Claude CLI
pub struct ClaudeChatService<R: Runtime = tauri::Wry> {
    cli_path: PathBuf,
    plugin_dir: PathBuf,
    default_working_directory: PathBuf,
    chat_message_repo: Arc<dyn ChatMessageRepository>,
    conversation_repo: Arc<dyn ChatConversationRepository>,
    agent_run_repo: Arc<dyn AgentRunRepository>,
    project_repo: Arc<dyn ProjectRepository>,
    task_repo: Arc<dyn TaskRepository>,
    ideation_session_repo: Arc<dyn IdeationSessionRepository>,
    message_queue: Arc<MessageQueue>,
    running_agent_registry: Arc<RunningAgentRegistry>,
    app_handle: Option<AppHandle<R>>,
    model: String,
}

impl<R: Runtime> ClaudeChatService<R> {
    pub fn new(
        chat_message_repo: Arc<dyn ChatMessageRepository>,
        conversation_repo: Arc<dyn ChatConversationRepository>,
        agent_run_repo: Arc<dyn AgentRunRepository>,
        project_repo: Arc<dyn ProjectRepository>,
        task_repo: Arc<dyn TaskRepository>,
        ideation_session_repo: Arc<dyn IdeationSessionRepository>,
        message_queue: Arc<MessageQueue>,
        running_agent_registry: Arc<RunningAgentRegistry>,
    ) -> Self {
        let cli_path = which::which("claude").unwrap_or_else(|_| PathBuf::from("claude"));
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let default_working_directory = cwd.parent().map(|p| p.to_path_buf()).unwrap_or(cwd);
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
            message_queue,
            running_agent_registry,
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

    /// Resolve the project's working directory from a context
    async fn resolve_working_directory(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> PathBuf {
        let project_id = match context_type {
            ChatContextType::Project => Some(ProjectId::from_string(context_id.to_string())),
            ChatContextType::Task | ChatContextType::TaskExecution => {
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
                if let Ok(Some(session)) = self
                    .ideation_session_repo
                    .get_by_id(&IdeationSessionId::from_string(context_id))
                    .await
                {
                    Some(session.project_id)
                } else {
                    None
                }
            }
        };

        if let Some(pid) = project_id {
            if let Ok(Some(project)) = self.project_repo.get_by_id(&pid).await {
                return PathBuf::from(&project.working_directory);
            }
        }

        self.default_working_directory.clone()
    }

    /// Build the initial prompt for a context
    fn build_initial_prompt(
        context_type: ChatContextType,
        context_id: &str,
        user_message: &str,
    ) -> String {
        match context_type {
            ChatContextType::Ideation => {
                format!(
                    "RalphX Ideation Session ID: {}\n\nUser's message: {}",
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
                format!(
                    "RalphX Task Execution ID: {}\n\n{}",
                    context_id, user_message
                )
            }
        }
    }

    /// Create a Claude CLI command
    fn build_command(
        &self,
        conversation: &ChatConversation,
        user_message: &str,
        working_directory: &Path,
    ) -> Command {
        let mut cmd = build_base_cli_command(&self.cli_path, &self.plugin_dir);

        let agent_name = get_agent_name(&conversation.context_type);
        cmd.env("RALPHX_AGENT_TYPE", agent_name);

        let (prompt, resume_session, agent) =
            if let Some(ref claude_session_id) = conversation.claude_session_id {
                (user_message.to_string(), Some(claude_session_id.as_str()), None)
            } else {
                let initial_prompt = Self::build_initial_prompt(
                    conversation.context_type,
                    &conversation.context_id,
                    user_message,
                );
                (initial_prompt, None, Some(agent_name))
            };

        add_prompt_args(&mut cmd, &prompt, agent, resume_session);
        configure_spawn(&mut cmd, working_directory);

        cmd
    }

    /// Create a user message based on context type
    fn create_user_message(
        context_type: ChatContextType,
        context_id: &str,
        content: &str,
        conversation_id: ChatConversationId,
    ) -> ChatMessage {
        let mut msg = match context_type {
            ChatContextType::Ideation => {
                ChatMessage::user_in_session(IdeationSessionId::from_string(context_id), content)
            }
            ChatContextType::Task | ChatContextType::TaskExecution => {
                ChatMessage::user_about_task(TaskId::from_string(context_id.to_string()), content)
            }
            ChatContextType::Project => {
                ChatMessage::user_in_project(ProjectId::from_string(context_id.to_string()), content)
            }
        };
        msg.conversation_id = Some(conversation_id);
        msg
    }

    /// Create an assistant message based on context type
    fn create_assistant_message(
        context_type: ChatContextType,
        context_id: &str,
        content: &str,
        conversation_id: ChatConversationId,
        tool_calls: &[ToolCall],
        content_blocks: &[ContentBlockItem],
    ) -> ChatMessage {
        let mut msg = match context_type {
            ChatContextType::Ideation => ChatMessage::orchestrator_in_session(
                IdeationSessionId::from_string(context_id),
                content,
            ),
            ChatContextType::Task => {
                let mut m = ChatMessage::user_about_task(
                    TaskId::from_string(context_id.to_string()),
                    content,
                );
                m.role = MessageRole::Orchestrator;
                m
            }
            ChatContextType::Project => {
                let mut m = ChatMessage::user_in_project(
                    ProjectId::from_string(context_id.to_string()),
                    content,
                );
                m.role = MessageRole::Orchestrator;
                m
            }
            ChatContextType::TaskExecution => ChatMessage {
                id: ChatMessageId::new(),
                session_id: None,
                project_id: None,
                task_id: Some(TaskId::from_string(context_id.to_string())),
                conversation_id: Some(conversation_id),
                role: MessageRole::Worker,
                content: content.to_string(),
                metadata: None,
                parent_message_id: None,
                tool_calls: None,
                content_blocks: None,
                created_at: chrono::Utc::now(),
            },
        };

        msg.conversation_id = Some(conversation_id);

        if !tool_calls.is_empty() {
            msg.tool_calls = Some(serde_json::to_string(tool_calls).unwrap_or_default());
        }
        if !content_blocks.is_empty() {
            msg.content_blocks = Some(serde_json::to_string(content_blocks).unwrap_or_default());
        }

        msg
    }
}

#[async_trait]
impl<R: Runtime + 'static> ChatService for ClaudeChatService<R> {
    async fn send_message(
        &self,
        context_type: ChatContextType,
        context_id: &str,
        message: &str,
    ) -> Result<SendResult, ChatServiceError> {
        // 1. Get or create conversation
        let conversation = self
            .get_or_create_conversation(context_type, context_id)
            .await?;
        let conversation_id = conversation.id;
        let is_new_conversation = conversation.claude_session_id.is_none();
        let stored_session_id = conversation.claude_session_id.clone();

        // 2. Create agent run record
        let agent_run = AgentRun::new(conversation_id);
        let agent_run_id = agent_run.id.as_str().to_string();
        self.agent_run_repo
            .create(agent_run)
            .await
            .map_err(|e| ChatServiceError::RepositoryError(e.to_string()))?;

        // 3. Emit run started event
        self.emit_event(
            "agent:run_started",
            AgentRunStartedPayload {
                run_id: agent_run_id.clone(),
                conversation_id: conversation_id.as_str().to_string(),
                context_type: context_type.to_string(),
                context_id: context_id.to_string(),
            },
        );

        // Also emit legacy events for backwards compatibility
        self.emit_event(
            if context_type == ChatContextType::TaskExecution {
                "execution:run_started"
            } else {
                "chat:run_started"
            },
            serde_json::json!({
                "run_id": &agent_run_id,
                "conversation_id": conversation_id.as_str(),
                "task_id": if context_type == ChatContextType::TaskExecution { Some(context_id) } else { None },
            }),
        );

        // 4. Store user message
        let user_msg = Self::create_user_message(
            context_type,
            context_id,
            message,
            conversation_id,
        );
        let user_msg_id = user_msg.id.as_str().to_string();
        self.chat_message_repo
            .create(user_msg)
            .await
            .map_err(|e| ChatServiceError::RepositoryError(e.to_string()))?;

        // 5. Emit message created event
        self.emit_event(
            "agent:message_created",
            AgentMessageCreatedPayload {
                message_id: user_msg_id.clone(),
                conversation_id: conversation_id.as_str().to_string(),
                context_type: context_type.to_string(),
                context_id: context_id.to_string(),
                role: "user".to_string(),
                content: message.to_string(),
            },
        );

        // Also emit legacy event
        self.emit_event(
            if context_type == ChatContextType::TaskExecution {
                "execution:message_created"
            } else {
                "chat:message_created"
            },
            serde_json::json!({
                "message_id": user_msg_id,
                "conversation_id": conversation_id.as_str(),
                "role": "user",
                "content": message,
            }),
        );

        // 6. Resolve working directory
        let working_directory = self
            .resolve_working_directory(context_type, context_id)
            .await;

        // 7. Build and spawn command
        let mut cmd = self.build_command(&conversation, message, &working_directory);
        let child = cmd
            .spawn()
            .map_err(|e| ChatServiceError::SpawnFailed(e.to_string()))?;

        // 7a. Register the process in the running agent registry
        let child_pid = child.id();
        if let Some(pid) = child_pid {
            let registry_key = RunningAgentKey::new(context_type.to_string(), context_id);
            self.running_agent_registry.register(
                registry_key,
                pid,
                conversation_id.as_str().to_string(),
                agent_run_id.clone(),
            ).await;
        }

        // 8. Clone values for background task
        let context_type_clone = context_type;
        let context_id_clone = context_id.to_string();
        let conversation_id_clone = conversation_id;
        let agent_run_id_clone = agent_run_id.clone();
        let chat_message_repo = Arc::clone(&self.chat_message_repo);
        let conversation_repo = Arc::clone(&self.conversation_repo);
        let agent_run_repo = Arc::clone(&self.agent_run_repo);
        let task_repo = Arc::clone(&self.task_repo);
        let message_queue = Arc::clone(&self.message_queue);
        let running_agent_registry = Arc::clone(&self.running_agent_registry);
        let app_handle = self.app_handle.clone();
        let cli_path = self.cli_path.clone();
        let plugin_dir = self.plugin_dir.clone();
        let working_directory_clone = working_directory;
        let stored_session_id_clone = stored_session_id;

        // 9. Process stream in background
        tokio::spawn(async move {
            // Create key for unregistering
            let registry_key = RunningAgentKey::new(context_type_clone.to_string(), &context_id_clone);

            let result = process_stream_background(
                child,
                context_type_clone,
                &context_id_clone,
                &conversation_id_clone,
                app_handle.clone(),
            )
            .await;

            // Unregister the process when done (whether success or failure)
            running_agent_registry.unregister(&registry_key).await;

            match result {
                Ok((response_text, tool_calls, content_blocks, claude_session_id)) => {
                    // Debug: Log what we got from stream processing
                    tracing::info!(
                        "[CHAT_SERVICE] Stream complete: context={}/{}, response_len={}, tool_calls={}, session_id={:?}",
                        context_type_clone,
                        context_id_clone,
                        response_text.len(),
                        tool_calls.len(),
                        claude_session_id
                    );

                    // Update conversation with claude_session_id
                    if let Some(ref sess_id) = claude_session_id {
                        tracing::info!("[CHAT_SERVICE] Updating conversation with session_id={}", sess_id);
                        let _ = conversation_repo
                            .update_claude_session_id(&conversation_id_clone, sess_id)
                            .await;
                    } else {
                        tracing::warn!("[CHAT_SERVICE] No claude_session_id captured from stream - queue processing will be skipped!");
                    }

                    // Persist assistant message
                    if !response_text.is_empty() || !tool_calls.is_empty() {
                        let assistant_msg = ClaudeChatService::<R>::create_assistant_message(
                            context_type_clone,
                            &context_id_clone,
                            &response_text,
                            conversation_id_clone,
                            &tool_calls,
                            &content_blocks,
                        );
                        let assistant_msg_id = assistant_msg.id.as_str().to_string();
                        let _ = chat_message_repo.create(assistant_msg).await;

                        // Emit message created
                        if let Some(ref handle) = app_handle {
                            let _ = handle.emit(
                                "agent:message_created",
                                AgentMessageCreatedPayload {
                                    message_id: assistant_msg_id.clone(),
                                    conversation_id: conversation_id_clone.as_str().to_string(),
                                    context_type: context_type_clone.to_string(),
                                    context_id: context_id_clone.clone(),
                                    role: get_assistant_role(&context_type_clone).to_string(),
                                    content: response_text.clone(),
                                },
                            );

                            // Legacy event
                            let _ = handle.emit(
                                if context_type_clone == ChatContextType::TaskExecution {
                                    "execution:message_created"
                                } else {
                                    "chat:message_created"
                                },
                                serde_json::json!({
                                    "message_id": assistant_msg_id,
                                    "conversation_id": conversation_id_clone.as_str(),
                                    "role": get_assistant_role(&context_type_clone).to_string(),
                                    "content": response_text,
                                }),
                            );
                        }
                    }

                    // Complete agent run
                    let _ = agent_run_repo
                        .complete(&crate::domain::entities::AgentRunId::from_string(
                            &agent_run_id_clone,
                        ))
                        .await;

                    // Handle task state transition (only for TaskExecution)
                    if context_type_clone == ChatContextType::TaskExecution {
                        let task_id = TaskId::from_string(context_id_clone.clone());
                        if let Ok(Some(mut task)) = task_repo.get_by_id(&task_id).await {
                            if task.internal_status
                                == crate::domain::entities::InternalStatus::Executing
                            {
                                task.internal_status =
                                    crate::domain::entities::InternalStatus::PendingReview;
                                task.touch();
                                let _ = task_repo.update(&task).await;

                                if let Some(ref handle) = app_handle {
                                    let _ = handle.emit(
                                        "task:event",
                                        serde_json::json!({
                                            "type": "status_changed",
                                            "taskId": context_id_clone,
                                            "from": "executing",
                                            "to": "pending_review",
                                            "changedBy": "agent",
                                        }),
                                    );
                                }
                            }
                        }
                    }

                    // Check if there are queued messages to process
                    // If yes, DON'T emit run_completed yet - emit it after queue processing
                    // Use the stream's session_id if available, otherwise fall back to stored session_id
                    let effective_session_id = claude_session_id.clone().or_else(|| stored_session_id_clone.clone());
                    let initial_queue_count = message_queue.get_queued(context_type_clone, &context_id_clone).len();
                    let has_session_for_queue = effective_session_id.is_some();
                    let will_process_queue = initial_queue_count > 0 && has_session_for_queue;

                    if initial_queue_count > 0 && claude_session_id.is_none() && stored_session_id_clone.is_some() {
                        tracing::info!(
                            "[QUEUE] Stream had no session_id, using stored session_id from conversation for queue processing"
                        );
                    }

                    // Only emit run_completed if there's no queue to process
                    // If there IS a queue, we'll emit run_completed after all queue messages are processed
                    if !will_process_queue {
                        if let Some(ref handle) = app_handle {
                            let _ = handle.emit(
                                "agent:run_completed",
                                AgentRunCompletedPayload {
                                    conversation_id: conversation_id_clone.as_str().to_string(),
                                    context_type: context_type_clone.to_string(),
                                    context_id: context_id_clone.clone(),
                                    claude_session_id: effective_session_id.clone(),
                                },
                            );

                            // Legacy event
                            let _ = handle.emit(
                                if context_type_clone == ChatContextType::TaskExecution {
                                    "execution:run_completed"
                                } else {
                                    "chat:run_completed"
                                },
                                serde_json::json!({
                                    "conversation_id": conversation_id_clone.as_str(),
                                    "claude_session_id": effective_session_id,
                                }),
                            );
                        }
                    } else {
                        tracing::info!(
                            "[QUEUE] Deferring run_completed: {} queued messages to process first",
                            initial_queue_count
                        );
                    }

                    // Process queued messages with retry loop to handle race conditions
                    // Messages can be queued while we're processing, so we keep checking until empty
                    if let Some(ref sess_id) = effective_session_id {
                        let mut total_processed = 0u32;

                        // Outer loop: keep processing until queue is stable-empty
                        loop {
                            let queue_count = message_queue.get_queued(context_type_clone, &context_id_clone).len();

                            if queue_count == 0 {
                                // Queue is empty, wait briefly then check once more for race condition
                                if total_processed > 0 {
                                    // We processed messages, give a small window for late arrivals
                                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                                    let final_count = message_queue.get_queued(context_type_clone, &context_id_clone).len();
                                    if final_count == 0 {
                                        tracing::info!("[QUEUE] Queue processing complete: {} total messages processed", total_processed);
                                        break;
                                    }
                                    tracing::info!("[QUEUE] Found {} late-arriving messages, continuing...", final_count);
                                } else {
                                    tracing::info!("[QUEUE] No queued messages to process");
                                    break;
                                }
                            }

                            tracing::info!(
                                "[QUEUE] Processing queue: session_id={}, context={}/{}, pending={}",
                                sess_id, context_type_clone, context_id_clone, queue_count
                            );

                            // Inner loop: process all currently queued messages
                            while let Some(queued_msg) =
                                message_queue.pop(context_type_clone, &context_id_clone)
                            {
                                total_processed += 1;
                            tracing::info!("[QUEUE] Processing queued message id={}, content_len={}", queued_msg.id, queued_msg.content.len());

                            // Emit queue sent event (removes from frontend optimistic UI)
                            if let Some(ref handle) = app_handle {
                                let _ = handle.emit(
                                    "agent:queue_sent",
                                    AgentQueueSentPayload {
                                        message_id: queued_msg.id.clone(),
                                        conversation_id: conversation_id_clone.as_str().to_string(),
                                        context_type: context_type_clone.to_string(),
                                        context_id: context_id_clone.clone(),
                                    },
                                );
                            }

                            // Emit run_started for the queued message (so frontend shows activity)
                            let queued_run_id = uuid::Uuid::new_v4().to_string();
                            if let Some(ref handle) = app_handle {
                                let _ = handle.emit(
                                    "agent:run_started",
                                    AgentRunStartedPayload {
                                        run_id: queued_run_id.clone(),
                                        conversation_id: conversation_id_clone.as_str().to_string(),
                                        context_type: context_type_clone.to_string(),
                                        context_id: context_id_clone.clone(),
                                    },
                                );
                            }

                            // Persist user message
                            let user_msg = ClaudeChatService::<R>::create_user_message(
                                context_type_clone,
                                &context_id_clone,
                                &queued_msg.content,
                                conversation_id_clone,
                            );
                            let user_msg_id = user_msg.id.as_str().to_string();
                            let _ = chat_message_repo.create(user_msg).await;

                            // Emit user message created
                            if let Some(ref handle) = app_handle {
                                let _ = handle.emit(
                                    "agent:message_created",
                                    AgentMessageCreatedPayload {
                                        message_id: user_msg_id,
                                        conversation_id: conversation_id_clone.as_str().to_string(),
                                        context_type: context_type_clone.to_string(),
                                        context_id: context_id_clone.clone(),
                                        role: "user".to_string(),
                                        content: queued_msg.content.clone(),
                                    },
                                );
                            }

                            // Build and spawn resume command
                            let agent_name = get_agent_name(&context_type_clone);
                            let mut cmd = build_base_cli_command(&cli_path, &plugin_dir);
                            cmd.env("RALPHX_AGENT_TYPE", agent_name);
                            add_prompt_args(&mut cmd, &queued_msg.content, None, Some(sess_id));
                            configure_spawn(&mut cmd, &working_directory_clone);

                            match cmd.spawn() {
                                Ok(child) => {
                                    match process_stream_background(
                                        child,
                                        context_type_clone,
                                        &context_id_clone,
                                        &conversation_id_clone,
                                        app_handle.clone(),
                                    )
                                    .await
                                    {
                                        Ok((response, tools, blocks, _)) => {
                                            if !response.is_empty() || !tools.is_empty() {
                                                let assistant_msg =
                                                    ClaudeChatService::<R>::create_assistant_message(
                                                        context_type_clone,
                                                        &context_id_clone,
                                                        &response,
                                                        conversation_id_clone,
                                                        &tools,
                                                        &blocks,
                                                    );
                                                let assistant_msg_id = assistant_msg.id.as_str().to_string();
                                                let _ = chat_message_repo.create(assistant_msg).await;

                                                // Emit assistant message created
                                                if let Some(ref handle) = app_handle {
                                                    let _ = handle.emit(
                                                        "agent:message_created",
                                                        AgentMessageCreatedPayload {
                                                            message_id: assistant_msg_id,
                                                            conversation_id: conversation_id_clone.as_str().to_string(),
                                                            context_type: context_type_clone.to_string(),
                                                            context_id: context_id_clone.clone(),
                                                            role: get_assistant_role(&context_type_clone).to_string(),
                                                            content: response.clone(),
                                                        },
                                                    );
                                                }
                                            }

                                            // NOTE: Don't emit run_completed here for each queued message.
                                            // We emit a single run_completed after ALL queue processing is done,
                                            // to prevent UI flickering between messages.
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                "Failed to process queued message stream: {}",
                                                e
                                            );
                                            // Emit error event
                                            if let Some(ref handle) = app_handle {
                                                let _ = handle.emit(
                                                    "agent:error",
                                                    AgentErrorPayload {
                                                        conversation_id: Some(conversation_id_clone.as_str().to_string()),
                                                        context_type: context_type_clone.to_string(),
                                                        context_id: context_id_clone.clone(),
                                                        error: e.clone(),
                                                        stderr: Some(e),
                                                    },
                                                );
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Failed to spawn queued message command: {}", e);
                                    // Emit error event
                                    if let Some(ref handle) = app_handle {
                                        let _ = handle.emit(
                                            "agent:error",
                                            AgentErrorPayload {
                                                conversation_id: Some(conversation_id_clone.as_str().to_string()),
                                                context_type: context_type_clone.to_string(),
                                                context_id: context_id_clone.clone(),
                                                error: e.to_string(),
                                                stderr: None,
                                            },
                                        );
                                    }
                                }
                            }
                        }
                        // End of inner while loop, outer loop continues to check for more
                        }

                        // After ALL queue processing is done, emit the final run_completed
                        // This prevents UI flickering between individual queued messages
                        if total_processed > 0 {
                            tracing::info!("[QUEUE] Emitting final run_completed after processing {} queued messages", total_processed);
                            if let Some(ref handle) = app_handle {
                                let _ = handle.emit(
                                    "agent:run_completed",
                                    AgentRunCompletedPayload {
                                        conversation_id: conversation_id_clone.as_str().to_string(),
                                        context_type: context_type_clone.to_string(),
                                        context_id: context_id_clone.clone(),
                                        claude_session_id: Some(sess_id.clone()),
                                    },
                                );

                                // Legacy event
                                let _ = handle.emit(
                                    if context_type_clone == ChatContextType::TaskExecution {
                                        "execution:run_completed"
                                    } else {
                                        "chat:run_completed"
                                    },
                                    serde_json::json!({
                                        "conversation_id": conversation_id_clone.as_str(),
                                        "claude_session_id": sess_id,
                                    }),
                                );
                            }
                        }
                    } else {
                        // effective_session_id is None - no session ID from stream OR stored conversation
                        let queue_count = message_queue.get_queued(context_type_clone, &context_id_clone).len();
                        if queue_count > 0 {
                            tracing::warn!(
                                "[QUEUE] SKIPPING {} queued messages because no session_id available (neither from stream nor stored)!",
                                queue_count
                            );
                        }
                    }
                }
                Err(e) => {
                    // Fail the agent run
                    let _ = agent_run_repo
                        .fail(
                            &crate::domain::entities::AgentRunId::from_string(&agent_run_id_clone),
                            &e,
                        )
                        .await;

                    // Emit error event
                    if let Some(ref handle) = app_handle {
                        let _ = handle.emit(
                            "agent:error",
                            AgentErrorPayload {
                                conversation_id: Some(conversation_id_clone.as_str().to_string()),
                                context_type: context_type_clone.to_string(),
                                context_id: context_id_clone.clone(),
                                error: e.clone(),
                                stderr: Some(e.clone()),
                            },
                        );

                        // Legacy event
                        let _ = handle.emit(
                            if context_type_clone == ChatContextType::TaskExecution {
                                "execution:error"
                            } else {
                                "chat:error"
                            },
                            serde_json::json!({
                                "conversation_id": conversation_id_clone.as_str(),
                                "error": e,
                            }),
                        );
                    }
                }
            }
        });

        // Return immediately
        Ok(SendResult {
            conversation_id: conversation_id.as_str().to_string(),
            agent_run_id,
            is_new_conversation,
        })
    }

    async fn queue_message(
        &self,
        context_type: ChatContextType,
        context_id: &str,
        content: &str,
        client_id: Option<&str>,
    ) -> Result<QueuedMessage, ChatServiceError> {
        Ok(match client_id {
            Some(id) => self.message_queue.queue_with_client_id(
                context_type,
                context_id,
                content.to_string(),
                id.to_string(),
            ),
            None => self
                .message_queue
                .queue(context_type, context_id, content.to_string()),
        })
    }

    async fn get_queued_messages(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> Result<Vec<QueuedMessage>, ChatServiceError> {
        Ok(self.message_queue.get_queued(context_type, context_id))
    }

    async fn delete_queued_message(
        &self,
        context_type: ChatContextType,
        context_id: &str,
        message_id: &str,
    ) -> Result<bool, ChatServiceError> {
        Ok(self
            .message_queue
            .delete(context_type, context_id, message_id))
    }

    async fn get_or_create_conversation(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> Result<ChatConversation, ChatServiceError> {
        // Try to get existing active conversation
        if let Some(conv) = self
            .conversation_repo
            .get_active_for_context(context_type, context_id)
            .await
            .map_err(|e| ChatServiceError::RepositoryError(e.to_string()))?
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
            .map_err(|e| ChatServiceError::RepositoryError(e.to_string()))
    }

    async fn get_conversation_with_messages(
        &self,
        conversation_id: &ChatConversationId,
    ) -> Result<Option<ChatConversationWithMessages>, ChatServiceError> {
        let conversation = match self
            .conversation_repo
            .get_by_id(conversation_id)
            .await
            .map_err(|e| ChatServiceError::RepositoryError(e.to_string()))?
        {
            Some(c) => c,
            None => return Ok(None),
        };

        let messages = self
            .chat_message_repo
            .get_by_conversation(conversation_id)
            .await
            .map_err(|e| ChatServiceError::RepositoryError(e.to_string()))?;

        Ok(Some(ChatConversationWithMessages {
            conversation,
            messages,
        }))
    }

    async fn list_conversations(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> Result<Vec<ChatConversation>, ChatServiceError> {
        self.conversation_repo
            .get_by_context(context_type, context_id)
            .await
            .map_err(|e| ChatServiceError::RepositoryError(e.to_string()))
    }

    async fn get_active_run(
        &self,
        conversation_id: &ChatConversationId,
    ) -> Result<Option<AgentRun>, ChatServiceError> {
        self.agent_run_repo
            .get_active_for_conversation(conversation_id)
            .await
            .map_err(|e| ChatServiceError::RepositoryError(e.to_string()))
    }

    async fn is_available(&self) -> bool {
        if self.cli_path.exists() {
            return true;
        }
        which::which(&self.cli_path).is_ok()
    }

    async fn stop_agent(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> Result<bool, ChatServiceError> {
        let key = RunningAgentKey::new(context_type.to_string(), context_id);

        match self.running_agent_registry.stop(&key).await {
            Ok(Some(info)) => {
                // Emit stopped event
                self.emit_event(
                    "agent:stopped",
                    serde_json::json!({
                        "conversation_id": info.conversation_id,
                        "agent_run_id": info.agent_run_id,
                        "context_type": context_type.to_string(),
                        "context_id": context_id,
                    }),
                );

                // Mark the agent run as failed with a stopped message
                let _ = self
                    .agent_run_repo
                    .fail(
                        &crate::domain::entities::AgentRunId::from_string(&info.agent_run_id),
                        "Agent stopped by user",
                    )
                    .await;

                // Also emit run_completed so frontend knows agent is no longer running
                self.emit_event(
                    "agent:run_completed",
                    AgentRunCompletedPayload {
                        conversation_id: info.conversation_id,
                        context_type: context_type.to_string(),
                        context_id: context_id.to_string(),
                        claude_session_id: None,
                    },
                );

                Ok(true)
            }
            Ok(None) => {
                // No agent was running
                Ok(false)
            }
            Err(e) => Err(ChatServiceError::AgentRunFailed(e)),
        }
    }

    async fn is_agent_running(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> bool {
        let key = RunningAgentKey::new(context_type.to_string(), context_id);
        self.running_agent_registry.is_running(&key).await
    }
}

// ============================================================================
// Background stream processing
// ============================================================================

/// Process stream output in background, emitting events
async fn process_stream_background<R: Runtime>(
    mut child: tokio::process::Child,
    context_type: ChatContextType,
    context_id: &str,
    conversation_id: &ChatConversationId,
    app_handle: Option<AppHandle<R>>,
) -> Result<(String, Vec<ToolCall>, Vec<ContentBlockItem>, Option<String>), String> {
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture stdout".to_string())?;

    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Failed to capture stderr".to_string())?;

    let conversation_id_str = conversation_id.as_str().to_string();
    let context_type_str = context_type.to_string();
    let context_id_str = context_id.to_string();

    // Spawn stderr reader
    let stderr_handle = app_handle.clone();
    let stderr_conv_id = conversation_id_str.clone();
    let stderr_task = tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        let mut stderr_content = String::new();

        while let Ok(Some(line)) = lines.next_line().await {
            stderr_content.push_str(&line);
            stderr_content.push('\n');
        }

        stderr_content
    });

    // Process stdout
    let stdout_reader = BufReader::new(stdout);
    let mut lines = stdout_reader.lines();
    let mut processor = StreamProcessor::new();

    while let Ok(Some(line)) = lines.next_line().await {
        if let Some(msg) = StreamProcessor::parse_line(&line) {
            let events = processor.process_message(msg);

            for event in events {
                match event {
                    StreamEvent::TextChunk(text) => {
                        if let Some(ref handle) = app_handle {
                            // Unified event
                            let _ = handle.emit(
                                "agent:chunk",
                                AgentChunkPayload {
                                    text: text.clone(),
                                    conversation_id: conversation_id_str.clone(),
                                    context_type: context_type_str.clone(),
                                    context_id: context_id_str.clone(),
                                },
                            );

                            // Legacy event
                            let legacy_event = if context_type == ChatContextType::TaskExecution {
                                "execution:chunk"
                            } else {
                                "chat:chunk"
                            };
                            let _ = handle.emit(
                                legacy_event,
                                serde_json::json!({
                                    "text": text,
                                    "conversation_id": conversation_id_str,
                                }),
                            );

                            // Activity stream event for task execution
                            if context_type == ChatContextType::TaskExecution {
                                let _ = handle.emit(
                                    "agent:message",
                                    serde_json::json!({
                                        "taskId": context_id_str,
                                        "type": "text",
                                        "content": text,
                                        "timestamp": chrono::Utc::now().timestamp_millis(),
                                    }),
                                );
                            }
                        }
                    }
                    StreamEvent::ToolCallStarted { name, id } => {
                        if let Some(ref handle) = app_handle {
                            let _ = handle.emit(
                                "agent:tool_call",
                                AgentToolCallPayload {
                                    tool_name: name.clone(),
                                    tool_id: id.clone(),
                                    arguments: serde_json::Value::Null,
                                    result: None,
                                    conversation_id: conversation_id_str.clone(),
                                    context_type: context_type_str.clone(),
                                    context_id: context_id_str.clone(),
                                },
                            );

                            // Legacy event
                            let legacy_event = if context_type == ChatContextType::TaskExecution {
                                "execution:tool_call"
                            } else {
                                "chat:tool_call"
                            };
                            let _ = handle.emit(
                                legacy_event,
                                serde_json::json!({
                                    "tool_name": name,
                                    "tool_id": id,
                                    "arguments": serde_json::Value::Null,
                                    "result": null,
                                    "conversation_id": conversation_id_str,
                                }),
                            );
                        }
                    }
                    StreamEvent::ToolCallCompleted(tool_call) => {
                        if let Some(ref handle) = app_handle {
                            let _ = handle.emit(
                                "agent:tool_call",
                                AgentToolCallPayload {
                                    tool_name: tool_call.name.clone(),
                                    tool_id: tool_call.id.clone(),
                                    arguments: tool_call.arguments.clone(),
                                    result: None,
                                    conversation_id: conversation_id_str.clone(),
                                    context_type: context_type_str.clone(),
                                    context_id: context_id_str.clone(),
                                },
                            );

                            // Legacy event
                            let legacy_event = if context_type == ChatContextType::TaskExecution {
                                "execution:tool_call"
                            } else {
                                "chat:tool_call"
                            };
                            let _ = handle.emit(
                                legacy_event,
                                serde_json::json!({
                                    "tool_name": tool_call.name,
                                    "tool_id": tool_call.id,
                                    "arguments": tool_call.arguments,
                                    "result": null,
                                    "conversation_id": conversation_id_str,
                                }),
                            );

                            // Activity stream event for task execution
                            if context_type == ChatContextType::TaskExecution {
                                let _ = handle.emit(
                                    "agent:message",
                                    serde_json::json!({
                                        "taskId": context_id_str,
                                        "type": "tool_call",
                                        "content": format!("{} ({})", tool_call.name, serde_json::to_string(&tool_call.arguments).unwrap_or_default()),
                                        "timestamp": chrono::Utc::now().timestamp_millis(),
                                        "metadata": {
                                            "tool_name": tool_call.name,
                                            "arguments": tool_call.arguments,
                                        },
                                    }),
                                );
                            }
                        }
                    }
                    StreamEvent::SessionId(_) => {
                        // Captured in processor.finish()
                    }
                    StreamEvent::ToolResultReceived { tool_use_id, result } => {
                        if let Some(ref handle) = app_handle {
                            let _ = handle.emit(
                                "agent:tool_call",
                                AgentToolCallPayload {
                                    tool_name: format!("result:{}", tool_use_id),
                                    tool_id: Some(tool_use_id.clone()),
                                    arguments: serde_json::Value::Null,
                                    result: Some(result.clone()),
                                    conversation_id: conversation_id_str.clone(),
                                    context_type: context_type_str.clone(),
                                    context_id: context_id_str.clone(),
                                },
                            );

                            // Legacy event
                            let legacy_event = if context_type == ChatContextType::TaskExecution {
                                "execution:tool_call"
                            } else {
                                "chat:tool_call"
                            };
                            let _ = handle.emit(
                                legacy_event,
                                serde_json::json!({
                                    "tool_name": format!("result:{}", tool_use_id),
                                    "tool_id": tool_use_id,
                                    "arguments": serde_json::Value::Null,
                                    "result": result,
                                    "conversation_id": conversation_id_str,
                                }),
                            );
                        }
                    }
                }
            }
        }
    }

    let result = processor.finish();

    // Wait for stderr task
    let stderr_content = stderr_task.await.unwrap_or_default();

    // Wait for process
    let status = child.wait().await.map_err(|e| e.to_string())?;

    if !status.success() && result.response_text.is_empty() {
        let error_msg = if stderr_content.is_empty() {
            "Agent exited with non-zero status".to_string()
        } else {
            format!("Agent failed: {}", stderr_content.trim())
        };
        return Err(error_msg);
    }

    Ok((
        result.response_text,
        result.tool_calls,
        result.content_blocks,
        result.session_id,
    ))
}

// ============================================================================
// MockChatService - For testing
// ============================================================================

use tokio::sync::Mutex;

/// Mock chat service for testing
pub struct MockChatService {
    responses: Mutex<Vec<MockChatResponse>>,
    is_available: Mutex<bool>,
    conversations: Mutex<Vec<ChatConversation>>,
    active_run: Mutex<Option<AgentRun>>,
    message_queue: Arc<MessageQueue>,
    call_count: std::sync::atomic::AtomicU32,
}

pub struct MockChatResponse {
    pub text: String,
    pub tool_calls: Vec<ToolCall>,
    pub claude_session_id: Option<String>,
}

impl MockChatService {
    pub fn new() -> Self {
        Self {
            responses: Mutex::new(Vec::new()),
            is_available: Mutex::new(true),
            conversations: Mutex::new(Vec::new()),
            active_run: Mutex::new(None),
            message_queue: Arc::new(MessageQueue::new()),
            call_count: std::sync::atomic::AtomicU32::new(0),
        }
    }

    pub fn with_queue(message_queue: Arc<MessageQueue>) -> Self {
        Self {
            responses: Mutex::new(Vec::new()),
            is_available: Mutex::new(true),
            conversations: Mutex::new(Vec::new()),
            active_run: Mutex::new(None),
            message_queue,
            call_count: std::sync::atomic::AtomicU32::new(0),
        }
    }

    pub fn call_count(&self) -> u32 {
        self.call_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub async fn set_available(&self, available: bool) {
        *self.is_available.lock().await = available;
    }

    pub async fn queue_response(&self, response: MockChatResponse) {
        self.responses.lock().await.push(response);
    }

    pub async fn queue_text_response(&self, text: impl Into<String>) {
        self.queue_response(MockChatResponse {
            text: text.into(),
            tool_calls: Vec::new(),
            claude_session_id: Some(uuid::Uuid::new_v4().to_string()),
        })
        .await;
    }

    pub async fn set_active_run(&self, run: Option<AgentRun>) {
        *self.active_run.lock().await = run;
    }

    pub async fn add_conversation(&self, conv: ChatConversation) {
        self.conversations.lock().await.push(conv);
    }
}

impl Default for MockChatService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ChatService for MockChatService {
    async fn send_message(
        &self,
        context_type: ChatContextType,
        context_id: &str,
        _message: &str,
    ) -> Result<SendResult, ChatServiceError> {
        self.call_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        if !*self.is_available.lock().await {
            return Err(ChatServiceError::AgentNotAvailable(
                "Mock agent not available".to_string(),
            ));
        }

        let conversation = self
            .get_or_create_conversation(context_type, context_id)
            .await?;
        let agent_run = AgentRun::new(conversation.id);

        Ok(SendResult {
            conversation_id: conversation.id.as_str().to_string(),
            agent_run_id: agent_run.id.as_str().to_string(),
            is_new_conversation: conversation.claude_session_id.is_none(),
        })
    }

    async fn queue_message(
        &self,
        context_type: ChatContextType,
        context_id: &str,
        content: &str,
        client_id: Option<&str>,
    ) -> Result<QueuedMessage, ChatServiceError> {
        Ok(match client_id {
            Some(id) => self.message_queue.queue_with_client_id(
                context_type,
                context_id,
                content.to_string(),
                id.to_string(),
            ),
            None => self
                .message_queue
                .queue(context_type, context_id, content.to_string()),
        })
    }

    async fn get_queued_messages(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> Result<Vec<QueuedMessage>, ChatServiceError> {
        Ok(self.message_queue.get_queued(context_type, context_id))
    }

    async fn delete_queued_message(
        &self,
        context_type: ChatContextType,
        context_id: &str,
        message_id: &str,
    ) -> Result<bool, ChatServiceError> {
        Ok(self
            .message_queue
            .delete(context_type, context_id, message_id))
    }

    async fn get_or_create_conversation(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> Result<ChatConversation, ChatServiceError> {
        let conversations = self.conversations.lock().await;

        if let Some(conv) = conversations
            .iter()
            .find(|c| c.context_type == context_type && c.context_id == context_id)
        {
            return Ok(conv.clone());
        }
        drop(conversations);

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

        self.conversations.lock().await.push(conv.clone());
        Ok(conv)
    }

    async fn get_conversation_with_messages(
        &self,
        conversation_id: &ChatConversationId,
    ) -> Result<Option<ChatConversationWithMessages>, ChatServiceError> {
        let conversations = self.conversations.lock().await;
        let conv = conversations
            .iter()
            .find(|c| c.id == *conversation_id)
            .cloned();

        Ok(conv.map(|c| ChatConversationWithMessages {
            conversation: c,
            messages: Vec::new(),
        }))
    }

    async fn list_conversations(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> Result<Vec<ChatConversation>, ChatServiceError> {
        let conversations = self.conversations.lock().await;
        Ok(conversations
            .iter()
            .filter(|c| c.context_type == context_type && c.context_id == context_id)
            .cloned()
            .collect())
    }

    async fn get_active_run(
        &self,
        _conversation_id: &ChatConversationId,
    ) -> Result<Option<AgentRun>, ChatServiceError> {
        Ok(self.active_run.lock().await.clone())
    }

    async fn is_available(&self) -> bool {
        *self.is_available.lock().await
    }

    async fn stop_agent(
        &self,
        _context_type: ChatContextType,
        _context_id: &str,
    ) -> Result<bool, ChatServiceError> {
        // Mock implementation - always returns false (no agent to stop)
        Ok(false)
    }

    async fn is_agent_running(
        &self,
        _context_type: ChatContextType,
        _context_id: &str,
    ) -> bool {
        // Mock implementation - always returns false
        false
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_service_send_message() {
        let service = MockChatService::new();

        let result = service
            .send_message(ChatContextType::Ideation, "session-1", "Hello")
            .await
            .unwrap();

        assert!(!result.conversation_id.is_empty());
        assert!(!result.agent_run_id.is_empty());
        assert!(result.is_new_conversation);
    }

    #[tokio::test]
    async fn test_mock_service_not_available() {
        let service = MockChatService::new();
        service.set_available(false).await;

        let result = service
            .send_message(ChatContextType::Ideation, "session-1", "Hello")
            .await;

        assert!(matches!(
            result,
            Err(ChatServiceError::AgentNotAvailable(_))
        ));
    }

    #[tokio::test]
    async fn test_mock_service_queue_operations() {
        let service = MockChatService::new();

        // Queue messages (no client_id)
        let msg1 = service
            .queue_message(ChatContextType::Ideation, "session-1", "Message 1", None)
            .await
            .unwrap();
        let msg2 = service
            .queue_message(ChatContextType::Ideation, "session-1", "Message 2", None)
            .await
            .unwrap();

        // Get queued
        let queued = service
            .get_queued_messages(ChatContextType::Ideation, "session-1")
            .await
            .unwrap();
        assert_eq!(queued.len(), 2);

        // Delete one
        let deleted = service
            .delete_queued_message(ChatContextType::Ideation, "session-1", &msg1.id)
            .await
            .unwrap();
        assert!(deleted);

        // Verify
        let queued = service
            .get_queued_messages(ChatContextType::Ideation, "session-1")
            .await
            .unwrap();
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0].id, msg2.id);
    }

    #[tokio::test]
    async fn test_mock_service_conversation_management() {
        let service = MockChatService::new();

        // Get or create conversation
        let conv1 = service
            .get_or_create_conversation(ChatContextType::Ideation, "session-1")
            .await
            .unwrap();

        // Same context returns same conversation
        let conv2 = service
            .get_or_create_conversation(ChatContextType::Ideation, "session-1")
            .await
            .unwrap();

        assert_eq!(conv1.id, conv2.id);

        // Different context returns different conversation
        let conv3 = service
            .get_or_create_conversation(ChatContextType::Task, "task-1")
            .await
            .unwrap();

        assert_ne!(conv1.id, conv3.id);
    }

    #[tokio::test]
    async fn test_mock_service_list_conversations() {
        let service = MockChatService::new();

        // Create conversations
        service
            .get_or_create_conversation(ChatContextType::Ideation, "session-1")
            .await
            .unwrap();
        service
            .get_or_create_conversation(ChatContextType::Ideation, "session-1")
            .await
            .unwrap(); // Same
        service
            .get_or_create_conversation(ChatContextType::Task, "task-1")
            .await
            .unwrap();

        // List by context
        let ideation_convs = service
            .list_conversations(ChatContextType::Ideation, "session-1")
            .await
            .unwrap();
        assert_eq!(ideation_convs.len(), 1);

        let task_convs = service
            .list_conversations(ChatContextType::Task, "task-1")
            .await
            .unwrap();
        assert_eq!(task_convs.len(), 1);
    }

    #[tokio::test]
    async fn test_mock_service_is_available() {
        let service = MockChatService::new();

        assert!(service.is_available().await);

        service.set_available(false).await;
        assert!(!service.is_available().await);
    }

    #[test]
    fn test_get_agent_name() {
        assert_eq!(
            get_agent_name(&ChatContextType::Ideation),
            "orchestrator-ideation"
        );
        assert_eq!(get_agent_name(&ChatContextType::Task), "chat-task");
        assert_eq!(get_agent_name(&ChatContextType::Project), "chat-project");
        assert_eq!(get_agent_name(&ChatContextType::TaskExecution), "worker");
    }

    #[test]
    fn test_get_assistant_role() {
        assert_eq!(
            get_assistant_role(&ChatContextType::Ideation),
            MessageRole::Orchestrator
        );
        assert_eq!(
            get_assistant_role(&ChatContextType::Task),
            MessageRole::Orchestrator
        );
        assert_eq!(
            get_assistant_role(&ChatContextType::Project),
            MessageRole::Orchestrator
        );
        assert_eq!(
            get_assistant_role(&ChatContextType::TaskExecution),
            MessageRole::Worker
        );
    }

    #[test]
    fn test_chat_service_error_display() {
        let err = ChatServiceError::AgentNotAvailable("test".to_string());
        assert!(err.to_string().contains("Agent not available"));

        let err = ChatServiceError::SpawnFailed("test".to_string());
        assert!(err.to_string().contains("spawn"));

        let err = ChatServiceError::ContextNotFound("test".to_string());
        assert!(err.to_string().contains("Context not found"));
    }

    #[test]
    fn test_send_result_serialization() {
        let result = SendResult {
            conversation_id: "conv-123".to_string(),
            agent_run_id: "run-456".to_string(),
            is_new_conversation: true,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("conv-123"));
        assert!(json.contains("run-456"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_event_payloads_serialization() {
        let payload = AgentRunStartedPayload {
            run_id: "run-1".to_string(),
            conversation_id: "conv-1".to_string(),
            context_type: "ideation".to_string(),
            context_id: "session-1".to_string(),
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("run-1"));
        assert!(json.contains("ideation"));

        let payload = AgentChunkPayload {
            text: "Hello".to_string(),
            conversation_id: "conv-1".to_string(),
            context_type: "ideation".to_string(),
            context_id: "session-1".to_string(),
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("Hello"));

        let payload = AgentRunCompletedPayload {
            conversation_id: "conv-1".to_string(),
            context_type: "ideation".to_string(),
            context_id: "session-1".to_string(),
            claude_session_id: Some("sess-123".to_string()),
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("sess-123"));
    }
}
