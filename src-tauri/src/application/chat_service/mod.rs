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

mod chat_service_helpers;
mod chat_service_mock;
mod chat_service_streaming;
mod chat_service_types;

use async_trait::async_trait;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::process::Command;

use crate::application::task_transition_service::TaskTransitionService;
use crate::domain::entities::{
    AgentRun, ChatConversation, ChatConversationId, ChatContextType, ChatMessage, ChatMessageId,
    IdeationSessionId, InternalStatus, MessageRole, ProjectId, TaskId,
};
use crate::domain::repositories::{
    AgentRunRepository, ChatConversationRepository, ChatMessageRepository,
    IdeationSessionRepository, ProjectRepository, TaskRepository,
};
use crate::domain::services::{MessageQueue, QueuedMessage, RunningAgentKey, RunningAgentRegistry};
use crate::infrastructure::agents::claude::{
    add_prompt_args, build_base_cli_command, configure_spawn, ContentBlockItem, ToolCall,
};

// Re-exports from extracted modules
pub use chat_service_helpers::{get_agent_name, get_assistant_role};
pub use chat_service_mock::{MockChatResponse, MockChatService};
pub use chat_service_streaming::process_stream_background;
pub use chat_service_types::{
    AgentChunkPayload, AgentErrorPayload, AgentMessageCreatedPayload, AgentQueueSentPayload,
    AgentRunCompletedPayload, AgentRunStartedPayload, AgentToolCallPayload,
    ChatConversationWithMessages, ChatServiceError, SendResult,
};

// Types and errors are now in chat_service_types.rs

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

// Helper functions are now in chat_service_helpers.rs

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
    execution_state: Option<Arc<crate::commands::ExecutionState>>,
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
            execution_state: None,
            model: "sonnet".to_string(),
        }
    }

    pub fn with_execution_state(mut self, state: Arc<crate::commands::ExecutionState>) -> Self {
        self.execution_state = Some(state);
        self
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
            ChatContextType::Task | ChatContextType::TaskExecution | ChatContextType::Review => {
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
            ChatContextType::Review => {
                format!(
                    "RalphX Review Session. Task ID: {}.\n\n\
                     You are reviewing this task. Examine the work, provide feedback, and determine if it meets quality standards.\n\n\
                     User's message: {}",
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

        // Add task scope for task-related contexts
        match conversation.context_type {
            ChatContextType::Task | ChatContextType::TaskExecution | ChatContextType::Review => {
                cmd.env("RALPHX_TASK_ID", &conversation.context_id);
            }
            _ => {}
        }

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
            ChatContextType::Task | ChatContextType::TaskExecution | ChatContextType::Review => {
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
            ChatContextType::Review => ChatMessage {
                id: ChatMessageId::new(),
                session_id: None,
                project_id: None,
                task_id: Some(TaskId::from_string(context_id.to_string())),
                conversation_id: Some(conversation_id),
                role: MessageRole::Reviewer,
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
        let project_repo = Arc::clone(&self.project_repo);
        let ideation_session_repo = Arc::clone(&self.ideation_session_repo);
        let message_queue = Arc::clone(&self.message_queue);
        let running_agent_registry = Arc::clone(&self.running_agent_registry);
        let execution_state = self.execution_state.clone();
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
                    // Use TaskTransitionService for proper entry/exit actions
                    // Requires execution_state for proper running count tracking
                    if context_type_clone == ChatContextType::TaskExecution {
                        if let Some(ref exec_state) = execution_state {
                            let task_id = TaskId::from_string(context_id_clone.clone());
                            if let Ok(Some(task)) = task_repo.get_by_id(&task_id).await {
                                if task.internal_status == InternalStatus::Executing {
                                    let transition_service = TaskTransitionService::new(
                                        Arc::clone(&task_repo),
                                        Arc::clone(&project_repo),
                                        Arc::clone(&chat_message_repo),
                                        Arc::clone(&conversation_repo),
                                        Arc::clone(&agent_run_repo),
                                        Arc::clone(&ideation_session_repo),
                                        Arc::clone(&message_queue),
                                        Arc::clone(&running_agent_registry),
                                        Arc::clone(exec_state),
                                        app_handle.clone(),
                                    );
                                    if let Err(e) = transition_service
                                        .transition_task(&task_id, InternalStatus::PendingReview)
                                        .await
                                    {
                                        tracing::error!(
                                            "Failed to transition task {} to PendingReview: {}",
                                            task_id.as_str(),
                                            e
                                        );
                                    }
                                }
                            }
                        } else {
                            tracing::warn!(
                                "Cannot transition task {} - no execution_state available",
                                context_id_clone
                            );
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

                            // Add task scope for task-related contexts
                            match context_type_clone {
                                ChatContextType::Task | ChatContextType::TaskExecution => {
                                    cmd.env("RALPHX_TASK_ID", &context_id_clone);
                                }
                                _ => {}
                            }

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
            ChatContextType::Review => {
                ChatConversation::new_review(TaskId::from_string(context_id.to_string()))
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
// Module re-exports are at the top of this file
// ============================================================================
