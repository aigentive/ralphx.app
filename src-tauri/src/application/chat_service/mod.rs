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

mod chat_service_context;
mod chat_service_helpers;
mod chat_service_mock;
mod chat_service_queue;
mod chat_service_repository;
mod chat_service_send_background;
mod chat_service_streaming;
mod chat_service_types;

use async_trait::async_trait;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::process::Command;
use crate::domain::entities::{
    AgentRun, ChatConversation, ChatConversationId, ChatContextType, TaskId,
};
use crate::domain::repositories::{
    AgentRunRepository, ChatConversationRepository, ChatMessageRepository,
    IdeationSessionRepository, ProjectRepository, TaskRepository,
};
use crate::domain::services::{MessageQueue, QueuedMessage, RunningAgentKey, RunningAgentRegistry};

// Re-exports from extracted modules
pub use chat_service_helpers::{get_agent_name, get_assistant_role};
pub use chat_service_mock::{MockChatResponse, MockChatService};
pub use chat_service_streaming::process_stream_background;
pub use chat_service_types::{
    events, AgentChunkPayload, AgentErrorPayload, AgentMessageCreatedPayload,
    AgentQueueSentPayload, AgentRunCompletedPayload, AgentRunStartedPayload, AgentToolCallPayload,
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
        chat_service_context::resolve_working_directory(
            context_type,
            context_id,
            Arc::clone(&self.project_repo),
            Arc::clone(&self.task_repo),
            Arc::clone(&self.ideation_session_repo),
            &self.default_working_directory,
        )
        .await
    }


    /// Create a Claude CLI command
    fn build_command(
        &self,
        conversation: &ChatConversation,
        user_message: &str,
        working_directory: &Path,
        entity_status: Option<&str>,
    ) -> Command {
        chat_service_context::build_command(
            &self.cli_path,
            &self.plugin_dir,
            conversation,
            user_message,
            working_directory,
            entity_status,
        )
    }

    /// Fetch entity status for context types that support it
    /// Used for dynamic agent resolution based on entity state
    async fn get_entity_status(&self, context_type: ChatContextType, context_id: &str) -> Option<String> {
        match context_type {
            // Task-related contexts: look up task status
            ChatContextType::Task | ChatContextType::TaskExecution | ChatContextType::Review => {
                let task_id = TaskId::from_string(context_id.to_string());
                if let Ok(Some(task)) = self.task_repo.get_by_id(&task_id).await {
                    Some(task.internal_status.as_str().to_string())
                } else {
                    None
                }
            }
            // Other contexts don't have status-based agent resolution yet
            ChatContextType::Ideation | ChatContextType::Project => None,
        }
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
        let user_msg = chat_service_context::create_user_message(
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

        // 6a. Fetch entity status for dynamic agent resolution
        let entity_status = self.get_entity_status(context_type, context_id).await;

        // 7. Build and spawn command
        let mut cmd = self.build_command(
            &conversation,
            message,
            &working_directory,
            entity_status.as_deref(),
        );
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

        // 7b. Increment running count for task execution contexts
        // This tracks concurrency for agent-active states (Executing, Reviewing, ReExecuting)
        // The count is decremented in TransitionHandler::on_exit when leaving these states
        if matches!(context_type, ChatContextType::TaskExecution | ChatContextType::Review) {
            if let Some(ref exec) = self.execution_state {
                exec.increment_running();
                // Emit status_changed event to frontend for real-time UI update
                if let Some(ref handle) = self.app_handle {
                    exec.emit_status_changed(handle, "task_started");
                }
            }
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

        // 9. Process stream in background (extracted to separate module)
        chat_service_send_background::spawn_send_message_background(
            child,
            context_type_clone,
            context_id_clone,
            conversation_id_clone,
            agent_run_id_clone,
            stored_session_id_clone,
            working_directory_clone,
            cli_path,
            plugin_dir,
            chat_message_repo,
            conversation_repo,
            agent_run_repo,
            task_repo,
            project_repo,
            ideation_session_repo,
            message_queue,
            running_agent_registry,
            execution_state,
            app_handle,
        );

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
        chat_service_repository::get_or_create_conversation(
            Arc::clone(&self.conversation_repo),
            context_type,
            context_id,
        )
        .await
    }

    async fn get_conversation_with_messages(
        &self,
        conversation_id: &ChatConversationId,
    ) -> Result<Option<ChatConversationWithMessages>, ChatServiceError> {
        chat_service_repository::get_conversation_with_messages(
            Arc::clone(&self.conversation_repo),
            Arc::clone(&self.chat_message_repo),
            conversation_id,
        )
        .await
    }

    async fn list_conversations(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> Result<Vec<ChatConversation>, ChatServiceError> {
        chat_service_repository::list_conversations(
            Arc::clone(&self.conversation_repo),
            context_type,
            context_id,
        )
        .await
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
