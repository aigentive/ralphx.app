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
use which::which;
use tokio::process::Command;
use crate::domain::entities::{
    AgentRun, ChatConversation, ChatConversationId, ChatContextType, IdeationSessionId, TaskId,
};
use crate::domain::repositories::{
    ActivityEventRepository, AgentRunRepository, ChatConversationRepository, ChatMessageRepository,
    IdeationSessionRepository, PlanBranchRepository, ProjectRepository, StateHistoryMetadata, TaskDependencyRepository,
    TaskRepository,
};
use crate::domain::services::{MessageQueue, QueuedMessage, RunningAgentKey, RunningAgentRegistry};

// Re-exports from extracted modules
pub use chat_service_helpers::{get_agent_name, get_assistant_role};
pub use chat_service_mock::{MockChatResponse, MockChatService};
pub use chat_service_streaming::process_stream_background;
pub(crate) use chat_service_send_background::reconcile_merge_auto_complete;
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
    task_dependency_repo: Arc<dyn TaskDependencyRepository>,
    ideation_session_repo: Arc<dyn IdeationSessionRepository>,
    activity_event_repo: Arc<dyn ActivityEventRepository>,
    message_queue: Arc<MessageQueue>,
    running_agent_registry: Arc<dyn RunningAgentRegistry>,
    app_handle: Option<AppHandle<R>>,
    execution_state: Option<Arc<crate::commands::ExecutionState>>,
    plan_branch_repo: Option<Arc<dyn PlanBranchRepository>>,
    model: String,
}

impl<R: Runtime> ClaudeChatService<R> {
    pub fn new(
        chat_message_repo: Arc<dyn ChatMessageRepository>,
        conversation_repo: Arc<dyn ChatConversationRepository>,
        agent_run_repo: Arc<dyn AgentRunRepository>,
        project_repo: Arc<dyn ProjectRepository>,
        task_repo: Arc<dyn TaskRepository>,
        task_dependency_repo: Arc<dyn TaskDependencyRepository>,
        ideation_session_repo: Arc<dyn IdeationSessionRepository>,
        activity_event_repo: Arc<dyn ActivityEventRepository>,
        message_queue: Arc<MessageQueue>,
        running_agent_registry: Arc<dyn RunningAgentRegistry>,
    ) -> Self {
        let cli_path = crate::infrastructure::agents::claude::find_claude_cli()
            .unwrap_or_else(|| PathBuf::from("claude"));
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
            task_dependency_repo,
            ideation_session_repo,
            activity_event_repo,
            message_queue,
            running_agent_registry,
            app_handle: None,
            execution_state: None,
            plan_branch_repo: None,
            model: "sonnet".to_string(),
        }
    }

    pub fn with_execution_state(mut self, state: Arc<crate::commands::ExecutionState>) -> Self {
        self.execution_state = Some(state);
        self
    }

    pub fn with_plan_branch_repo(mut self, repo: Arc<dyn PlanBranchRepository>) -> Self {
        self.plan_branch_repo = Some(repo);
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
        project_id: Option<&str>,
    ) -> Result<Command, ChatServiceError> {
        chat_service_context::build_command(
            &self.cli_path,
            &self.plugin_dir,
            conversation,
            user_message,
            working_directory,
            entity_status,
            project_id,
        )
        .map_err(ChatServiceError::SpawnFailed)
    }

    /// Fetch entity status for context types that support it
    /// Used for dynamic agent resolution based on entity state
    async fn get_entity_status(&self, context_type: ChatContextType, context_id: &str) -> Option<String> {
        match context_type {
            // Task-related contexts: look up task status
            ChatContextType::Task | ChatContextType::TaskExecution | ChatContextType::Review | ChatContextType::Merge => {
                let task_id = TaskId::from_string(context_id.to_string());
                if let Ok(Some(task)) = self.task_repo.get_by_id(&task_id).await {
                    Some(task.internal_status.as_str().to_string())
                } else {
                    None
                }
            }
            // Ideation context: look up session status for read-only mode
            ChatContextType::Ideation => {
                let session_id = IdeationSessionId::from_string(context_id);
                if let Ok(Some(session)) = self.ideation_session_repo.get_by_id(&session_id).await {
                    Some(session.status.to_string())
                } else {
                    None
                }
            }
            // Other contexts don't have status-based agent resolution yet
            ChatContextType::Project => None,
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
        eprintln!(
            "[STREAM_DEBUG] chat_service.send_message start (context_type={}, context_id={}, message_len={})",
            context_type,
            context_id,
            message.len()
        );
        // 1. Get or create conversation
        let conversation = self
            .get_or_create_conversation(context_type, context_id)
            .await?;
        eprintln!(
            "[STREAM_DEBUG] chat_service.send_message conversation (id={}, session_id={:?})",
            conversation.id.as_str(),
            conversation.claude_session_id
        );
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
        eprintln!(
            "[STREAM_DEBUG] chat_service.send_message agent_run created (run_id={})",
            agent_run_id
        );

        // 2a. Update state history metadata for task-related contexts
        // This links the conversation_id and agent_run_id to the state history entry,
        // enabling history navigation to show the correct conversation for each state.
        // Best-effort: don't fail send_message if metadata update fails.
        if matches!(
            context_type,
            ChatContextType::TaskExecution | ChatContextType::Review | ChatContextType::Merge
        ) {
            let task_id = TaskId::from_string(context_id.to_string());
            let metadata = StateHistoryMetadata {
                conversation_id: conversation_id.as_str().to_string(),
                agent_run_id: agent_run_id.clone(),
            };
            // Ignore errors - state history metadata is non-critical for message flow
            let _ = self.task_repo.update_latest_state_history_metadata(&task_id, &metadata).await;
        }

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
        eprintln!(
            "[STREAM_DEBUG] chat_service.send_message user message stored (message_id={})",
            user_msg_id
        );

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

        // 6. Resolve working directory
        let mut working_directory = self
            .resolve_working_directory(context_type, context_id)
            .await;
        if !working_directory.exists() {
            eprintln!(
                "[STREAM_DEBUG] chat_service.send_message working_directory missing, falling back to default (missing={})",
                working_directory.display()
            );
            working_directory = self.default_working_directory.clone();
        }
        eprintln!(
            "[STREAM_DEBUG] chat_service.send_message working_directory={}",
            working_directory.display()
        );

        // 6a. Fetch entity status for dynamic agent resolution
        let entity_status = self.get_entity_status(context_type, context_id).await;

        // 6b. Resolve project ID for RALPHX_PROJECT_ID env var
        let project_id = chat_service_context::resolve_project_id(
            context_type,
            context_id,
            Arc::clone(&self.project_repo),
            Arc::clone(&self.task_repo),
            Arc::clone(&self.ideation_session_repo),
        )
        .await;

        // 7. Increment running count for task execution contexts BEFORE spawning
        // This tracks concurrency for agent-active states (Executing, Reviewing, ReExecuting)
        // The count is decremented in TransitionHandler::on_exit when leaving these states
        // IMPORTANT: Must increment before spawn to ensure scheduling respects capacity
        if matches!(context_type, ChatContextType::TaskExecution | ChatContextType::Review) {
            if let Some(ref exec) = self.execution_state {
                exec.increment_running();
                // Emit status_changed event to frontend for real-time UI update
                if let Some(ref handle) = self.app_handle {
                    exec.emit_status_changed(handle, "task_started");
                }
            }
        }

        // 7a. Build and spawn command
        if !self.cli_path.exists() && which(&self.cli_path).is_err() {
            eprintln!(
                "[STREAM_DEBUG] chat_service.send_message missing Claude CLI at {}",
                self.cli_path.display()
            );
            return Err(ChatServiceError::SpawnFailed(format!(
                "Claude CLI not found at {}",
                self.cli_path.display()
            )));
        }

        eprintln!(
            "[STREAM_DEBUG] chat_service.send_message building command (cli_path={})",
            self.cli_path.display()
        );
        let mut cmd = self.build_command(
            &conversation,
            message,
            &working_directory,
            entity_status.as_deref(),
            project_id.as_deref(),
        )?;
        eprintln!("[STREAM_DEBUG] chat_service.send_message command built");
        eprintln!("[STREAM_DEBUG] chat_service.send_message spawning CLI");
        let child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                eprintln!(
                    "[STREAM_DEBUG] chat_service.send_message spawn failed: {}",
                    e
                );
                return Err(ChatServiceError::SpawnFailed(e.to_string()));
            }
        };
        eprintln!(
            "[STREAM_DEBUG] chat_service.send_message spawn ok (pid={:?})",
            child.id()
        );

        // 7b. Register the process in the running agent registry
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
        let task_dependency_repo = Arc::clone(&self.task_dependency_repo);
        let project_repo = Arc::clone(&self.project_repo);
        let ideation_session_repo = Arc::clone(&self.ideation_session_repo);
        let activity_event_repo = Arc::clone(&self.activity_event_repo);
        let message_queue = Arc::clone(&self.message_queue);
        let running_agent_registry = Arc::clone(&self.running_agent_registry);
        let execution_state = self.execution_state.clone();
        let plan_branch_repo = self.plan_branch_repo.clone();
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
            task_dependency_repo,
            project_repo,
            ideation_session_repo,
            activity_event_repo,
            message_queue,
            running_agent_registry,
            execution_state,
            plan_branch_repo,
            app_handle,
        );
        eprintln!(
            "[STREAM_DEBUG] chat_service.send_message background spawn kicked (conversation_id={})",
            conversation_id.as_str()
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
