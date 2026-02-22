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
mod chat_service_errors;
mod chat_service_handlers;
mod chat_service_helpers;
mod chat_service_merge;
mod chat_service_mock;
mod chat_service_queue;
mod chat_service_recovery;
mod chat_service_replay;
mod chat_service_repository;
mod chat_service_send_background;
mod chat_service_streaming;
mod chat_service_types;
mod streaming_state_cache;

use crate::application::question_state::QuestionState;
use crate::domain::entities::{
    AgentRun, ChatContextType, ChatConversation, ChatConversationId, ChatMessageId,
    IdeationSessionId, TaskId,
};
use crate::domain::repositories::{
    ActivityEventRepository, AgentRunRepository, ChatAttachmentRepository,
    ChatConversationRepository, ChatMessageRepository, IdeationSessionRepository,
    MemoryEventRepository, PlanBranchRepository, ProjectRepository, StateHistoryMetadata,
    TaskDependencyRepository, TaskProposalRepository, TaskRepository, TaskStepRepository,
};
use crate::domain::services::{MessageQueue, QueuedMessage, RunningAgentKey, RunningAgentRegistry};
use async_trait::async_trait;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};
use tokio_util::sync::CancellationToken;
use which::which;

/// Prefix used when formatting agent errors into chat messages.
/// Both the write site (chat_service_handlers) and read site (chat_service_replay)
/// must use this constant to stay in sync.
pub const AGENT_ERROR_PREFIX: &str = "[Agent error:";

// Re-exports from extracted modules
pub use chat_service_errors::{
    classify_agent_error, PauseReason, ProviderErrorCategory, ProviderErrorMetadata, StreamError,
    STALE_SESSION_ERROR,
};
pub use chat_service_helpers::{
    context_type_to_process, get_agent_name, get_assistant_role, resolve_agent_with_team_mode,
};
pub(crate) use chat_service_merge::{MergeAutoCompleteContext, reconcile_merge_auto_complete};
pub use chat_service_mock::{MockChatResponse, MockChatService};
pub use chat_service_replay::{build_rehydration_prompt, ConversationReplay, ReplayBuilder, Turn};
pub use chat_service_streaming::process_stream_background;
pub use chat_service_types::{
    events, AgentChunkPayload, AgentErrorPayload, AgentHookPayload, AgentMessageCreatedPayload,
    AgentQueueSentPayload, AgentRunCompletedPayload, AgentRunStartedPayload,
    AgentTaskCompletedPayload, AgentTaskStartedPayload, AgentToolCallPayload,
    ChatConversationWithMessages, ChatServiceError, SendResult, TeamCostUpdatePayload,
    TeamCreatedPayload, TeamDisbandedPayload, TeamMessagePayload, TeamTeammateIdlePayload,
    TeamTeammateShutdownPayload, TeamTeammateSpawnedPayload,
};
pub use streaming_state_cache::{
    CachedStreamingTask, CachedToolCall, ConversationStreamingState, StreamingStateCache,
};

// Types and errors are now in chat_service_types.rs

/// Shared definition for "meaningful" agent output used by streaming and
/// background completion logic.
pub(crate) fn has_meaningful_output(
    response_text: &str,
    tool_call_count: usize,
    stderr_text: &str,
) -> bool {
    if tool_call_count > 0 {
        return true;
    }
    // If stderr has errors and no tool calls, agent crashed — not meaningful work
    if !stderr_text.trim().is_empty() {
        return false;
    }
    !response_text.trim().is_empty()
}

/// Shared event payload context used by background and streaming modules.
#[derive(Debug, Clone)]
pub(crate) struct EventContextPayload {
    pub conversation_id: String,
    pub context_type: String,
    pub context_id: String,
}

pub(crate) fn event_context(
    conversation_id: &ChatConversationId,
    context_type: &ChatContextType,
    context_id: &str,
) -> EventContextPayload {
    EventContextPayload {
        conversation_id: conversation_id.as_str().to_string(),
        context_type: context_type.to_string(),
        context_id: context_id.to_string(),
    }
}

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
    async fn is_agent_running(&self, context_type: ChatContextType, context_id: &str) -> bool;

    /// Override team mode at runtime (interior mutability).
    /// Default is a no-op; ClaudeChatService uses AtomicBool.
    fn set_team_mode(&self, _mode: bool) {}

    /// Override plan branch repo at runtime (interior mutability).
    /// Default is a no-op; ClaudeChatService uses std::sync::Mutex.
    fn set_plan_branch_repo(&self, _repo: Arc<dyn PlanBranchRepository>) {}
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
    chat_attachment_repo: Arc<dyn ChatAttachmentRepository>,
    conversation_repo: Arc<dyn ChatConversationRepository>,
    agent_run_repo: Arc<dyn AgentRunRepository>,
    project_repo: Arc<dyn ProjectRepository>,
    task_repo: Arc<dyn TaskRepository>,
    task_dependency_repo: Arc<dyn TaskDependencyRepository>,
    ideation_session_repo: Arc<dyn IdeationSessionRepository>,
    activity_event_repo: Arc<dyn ActivityEventRepository>,
    message_queue: Arc<MessageQueue>,
    running_agent_registry: Arc<dyn RunningAgentRegistry>,
    memory_event_repo: Arc<dyn MemoryEventRepository>,
    app_handle: Option<AppHandle<R>>,
    execution_state: Option<Arc<crate::commands::ExecutionState>>,
    question_state: Option<Arc<QuestionState>>,
    plan_branch_repo: std::sync::Mutex<Option<Arc<dyn PlanBranchRepository>>>,
    task_proposal_repo: Option<Arc<dyn TaskProposalRepository>>,
    task_step_repo: Option<Arc<dyn TaskStepRepository>>,
    model: String,
    /// When true, agent resolution uses team-lead variants if configured.
    /// Uses AtomicBool for interior mutability so team_mode can be set
    /// after Arc-wrapping (e.g., per-task metadata override).
    team_mode: AtomicBool,
    /// Team service for managing agent teams lifecycle (persistence + events).
    team_service: Option<std::sync::Arc<crate::application::TeamService>>,
    /// Cache for streaming state, used to hydrate frontend on navigation.
    streaming_state_cache: StreamingStateCache,
}

impl<R: Runtime> ClaudeChatService<R> {
    pub fn new(
        chat_message_repo: Arc<dyn ChatMessageRepository>,
        chat_attachment_repo: Arc<dyn ChatAttachmentRepository>,
        conversation_repo: Arc<dyn ChatConversationRepository>,
        agent_run_repo: Arc<dyn AgentRunRepository>,
        project_repo: Arc<dyn ProjectRepository>,
        task_repo: Arc<dyn TaskRepository>,
        task_dependency_repo: Arc<dyn TaskDependencyRepository>,
        ideation_session_repo: Arc<dyn IdeationSessionRepository>,
        activity_event_repo: Arc<dyn ActivityEventRepository>,
        message_queue: Arc<MessageQueue>,
        running_agent_registry: Arc<dyn RunningAgentRegistry>,
        memory_event_repo: Arc<dyn MemoryEventRepository>,
    ) -> Self {
        let cli_path = crate::infrastructure::agents::claude::find_claude_cli()
            .unwrap_or_else(|| PathBuf::from("claude"));
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let default_working_directory = if cwd.file_name().is_some_and(|name| name == "src-tauri") {
            cwd.parent().map(|p| p.to_path_buf()).unwrap_or(cwd)
        } else {
            cwd
        };
        let plugin_dir =
            crate::infrastructure::agents::claude::resolve_plugin_dir(&default_working_directory);

        Self {
            cli_path,
            plugin_dir,
            default_working_directory,
            chat_message_repo,
            chat_attachment_repo,
            conversation_repo,
            agent_run_repo,
            project_repo,
            task_repo,
            task_dependency_repo,
            ideation_session_repo,
            activity_event_repo,
            message_queue,
            running_agent_registry,
            memory_event_repo,
            app_handle: None,
            execution_state: None,
            question_state: None,
            plan_branch_repo: std::sync::Mutex::new(None),
            task_proposal_repo: None,
            task_step_repo: None,
            model: "sonnet".to_string(),
            team_mode: AtomicBool::new(false),
            team_service: None,
            streaming_state_cache: StreamingStateCache::new(),
        }
    }

    pub fn with_execution_state(mut self, state: Arc<crate::commands::ExecutionState>) -> Self {
        self.execution_state = Some(state);
        self
    }

    pub fn with_question_state(mut self, state: Arc<QuestionState>) -> Self {
        self.question_state = Some(state);
        self
    }

    pub fn with_plan_branch_repo(self, repo: Arc<dyn PlanBranchRepository>) -> Self {
        *self.plan_branch_repo.lock().unwrap() = Some(repo);
        self
    }

    pub fn with_task_proposal_repo(mut self, repo: Arc<dyn TaskProposalRepository>) -> Self {
        self.task_proposal_repo = Some(repo);
        self
    }

    pub fn with_task_step_repo(mut self, repo: Arc<dyn TaskStepRepository>) -> Self {
        self.task_step_repo = Some(repo);
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

    pub fn with_team_mode(mut self, team_mode: bool) -> Self {
        self.team_mode = AtomicBool::new(team_mode);
        self
    }

    pub fn with_team_service(
        mut self,
        service: std::sync::Arc<crate::application::TeamService>,
    ) -> Self {
        self.team_service = Some(service);
        self
    }

    pub fn with_streaming_state_cache(mut self, cache: StreamingStateCache) -> Self {
        self.streaming_state_cache = cache;
        self
    }

    pub fn with_app_handle(mut self, app_handle: AppHandle<R>) -> Self {
        self.app_handle = Some(app_handle);
        self
    }

    /// Get a reference to the streaming state cache.
    ///
    /// Used by HTTP handlers to fetch current streaming state for hydration.
    pub fn streaming_state_cache(&self) -> &StreamingStateCache {
        &self.streaming_state_cache
    }

    /// Emit a Tauri event if app_handle is available
    fn emit_event(&self, event: &str, payload: impl Serialize + Clone) {
        if let Some(ref handle) = self.app_handle {
            let _ = handle.emit(event, payload);
        }
    }

    /// Resolve the project's working directory from a context.
    ///
    /// Returns `Err` for Merge contexts that resolve to the primary repo
    /// (hard error to prevent fixer agent from corrupting user's checkout).
    async fn resolve_working_directory(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> Result<PathBuf, String> {
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

    /// Create a spawnable Claude CLI command.
    async fn build_command(
        &self,
        conversation: &ChatConversation,
        user_message: &str,
        working_directory: &Path,
        entity_status: Option<&str>,
        project_id: Option<&str>,
    ) -> Result<crate::infrastructure::agents::claude::SpawnableCommand, ChatServiceError> {
        chat_service_context::build_command(
            &self.cli_path,
            &self.plugin_dir,
            conversation,
            user_message,
            working_directory,
            entity_status,
            project_id,
            self.team_mode.load(Ordering::Relaxed),
            Arc::clone(&self.chat_attachment_repo),
        )
        .await
        .map_err(ChatServiceError::SpawnFailed)
    }

    /// Fetch entity status for context types that support it
    /// Used for dynamic agent resolution based on entity state
    async fn get_entity_status(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> Option<String> {
        match context_type {
            // Task-related contexts: look up task status
            ChatContextType::Task
            | ChatContextType::TaskExecution
            | ChatContextType::Review
            | ChatContextType::Merge => {
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
        tracing::debug!(
            %context_type,
            context_id,
            message_len = message.len(),
            "chat_service.send_message start"
        );
        // 1. Get or create conversation
        let conversation = self
            .get_or_create_conversation(context_type, context_id)
            .await?;
        tracing::debug!(
            conversation_id = conversation.id.as_str(),
            session_id = ?conversation.claude_session_id,
            "chat_service.send_message conversation"
        );

        // 1b. Atomic guard: claim the agent slot to prevent TOCTOU race.
        //     If an agent is already registered for this context, queue the message.
        //     Create the AgentRun early so its ID can be stored in the slot for ownership tracking.
        let agent_run = AgentRun::new(conversation.id);
        let agent_run_id = agent_run.id.as_str().to_string();
        let run_chain_id = agent_run.run_chain_id.clone();

        let registry_key = RunningAgentKey::new(context_type.to_string(), context_id);
        if let Err(_existing) = self
            .running_agent_registry
            .try_register(
                registry_key.clone(),
                conversation.id.as_str().to_string(),
                agent_run_id.clone(),
            )
            .await
        {
            tracing::warn!(
                %context_type,
                context_id,
                "chat_service.send_message agent already running — auto-queuing message"
            );
            let queued = self
                .message_queue
                .queue(context_type, context_id, message.to_string());
            self.emit_event(
                "agent:queue_sent",
                AgentQueueSentPayload {
                    message_id: queued.id.clone(),
                    conversation_id: conversation.id.as_str().to_string(),
                    context_type: context_type.to_string(),
                    context_id: context_id.to_string(),
                },
            );
            return Err(ChatServiceError::AgentAlreadyRunning(format!(
                "Message queued (id: {}). An agent is already running for {} {}.",
                queued.id, context_type, context_id
            )));
        }

        // From here on, we hold the agent slot. Any early return must unregister.
        let mut running_incremented = false;

        // Cleanup macro: unregisters slot + decrements running count on failure.
        // Uses textual expansion so `.await` works inside the async fn body.
        macro_rules! cleanup_and_err {
            ($err:expr) => {{
                self.running_agent_registry
                    .unregister(&registry_key, &agent_run_id)
                    .await;
                if running_incremented {
                    if let Some(ref exec) = self.execution_state {
                        exec.decrement_running();
                        if let Some(ref handle) = self.app_handle {
                            exec.emit_status_changed(handle, "slot_cleanup");
                        }
                    }
                }
                return Err($err);
            }};
        }

        let conversation_id = conversation.id;
        let is_new_conversation = conversation.claude_session_id.is_none();
        let stored_session_id = conversation.claude_session_id.clone();

        // 2. Persist agent run record (created earlier before try_register for ownership tracking)
        if let Err(e) = self.agent_run_repo.create(agent_run).await {
            cleanup_and_err!(ChatServiceError::RepositoryError(e.to_string()));
        }
        tracing::debug!(
            run_id = %agent_run_id,
            "chat_service.send_message agent_run created"
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
            let _ = self
                .task_repo
                .update_latest_state_history_metadata(&task_id, &metadata)
                .await;
        }

        // 3. Emit run started event
        self.emit_event(
            "agent:run_started",
            AgentRunStartedPayload {
                run_id: agent_run_id.clone(),
                conversation_id: conversation_id.as_str().to_string(),
                context_type: context_type.to_string(),
                context_id: context_id.to_string(),
                run_chain_id: run_chain_id.clone(),
                parent_run_id: None,
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
        if let Err(e) = self.chat_message_repo.create(user_msg).await {
            cleanup_and_err!(ChatServiceError::RepositoryError(e.to_string()));
        }
        tracing::debug!(
            message_id = %user_msg_id,
            "chat_service.send_message user message stored"
        );

        // 4b. Link pending attachments to the user message
        let pending_attachments = match self
            .chat_attachment_repo
            .find_by_conversation_id(&conversation_id)
            .await
        {
            Ok(v) => v
                .into_iter()
                .filter(|a| a.message_id.is_none())
                .collect::<Vec<_>>(),
            Err(e) => {
                cleanup_and_err!(ChatServiceError::RepositoryError(e.to_string()));
            }
        };

        if !pending_attachments.is_empty() {
            let attachment_ids: Vec<_> = pending_attachments.iter().map(|a| a.id.clone()).collect();
            if let Err(e) = self
                .chat_attachment_repo
                .update_message_ids(&attachment_ids, &ChatMessageId::from_string(&user_msg_id))
                .await
            {
                cleanup_and_err!(ChatServiceError::RepositoryError(e.to_string()));
            }
            tracing::debug!(
                message_id = %user_msg_id,
                attachment_count = pending_attachments.len(),
                "chat_service.send_message linked attachments to user message"
            );
        }

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
        let mut working_directory = match self
            .resolve_working_directory(context_type, context_id)
            .await
        {
            Ok(dir) => dir,
            Err(e) => {
                cleanup_and_err!(ChatServiceError::SpawnFailed(e));
            }
        };
        if !working_directory.exists() {
            tracing::warn!(
                context_type = ?context_type,
                context_id = context_id,
                missing = %working_directory.display(),
                default = %self.default_working_directory.display(),
                "chat_service.send_message: resolved working_directory does not exist, \
                 falling back to default. Agent may operate in unexpected directory."
            );
            working_directory = self.default_working_directory.clone();
        }
        tracing::debug!(
            working_directory = %working_directory.display(),
            "chat_service.send_message working_directory resolved"
        );

        // 6a. Fetch entity status for dynamic agent resolution
        let entity_status = self.get_entity_status(context_type, context_id).await;

        // 6b. Resolve project ID for RALPHX_PROJECT_ID env var
        let project_id = chat_service_context::resolve_project_id(
            context_type,
            context_id,
            Arc::clone(&self.task_repo),
            Arc::clone(&self.ideation_session_repo),
        )
        .await;

        // 7. Increment running count for task execution contexts BEFORE spawning
        // This tracks concurrency for agent-active states (Executing, Reviewing, ReExecuting)
        // The count is decremented in TransitionHandler::on_exit when leaving these states
        // IMPORTANT: Must increment before spawn to ensure scheduling respects capacity
        if matches!(
            context_type,
            ChatContextType::TaskExecution | ChatContextType::Review | ChatContextType::Merge
        ) {
            if let Some(ref exec) = self.execution_state {
                exec.increment_running();
                running_incremented = true;
                // Emit status_changed event to frontend for real-time UI update
                if let Some(ref handle) = self.app_handle {
                    exec.emit_status_changed(handle, "task_started");
                }
            }
        }

        // 7a. Build and spawn command
        if !self.cli_path.exists() && which(&self.cli_path).is_err() {
            tracing::warn!(
                cli_path = %self.cli_path.display(),
                "chat_service.send_message missing Claude CLI"
            );
            cleanup_and_err!(ChatServiceError::SpawnFailed(format!(
                "Claude CLI not found at {}",
                self.cli_path.display()
            )));
        }

        tracing::debug!(
            cli_path = %self.cli_path.display(),
            "chat_service.send_message building command"
        );
        let spawnable = match self
            .build_command(
                &conversation,
                message,
                &working_directory,
                entity_status.as_deref(),
                project_id.as_deref(),
            )
            .await
        {
            Ok(s) => s,
            Err(e) => {
                cleanup_and_err!(e);
            }
        };
        tracing::info!(cmd = ?spawnable, "Spawning CLI agent");
        let child = match spawnable.spawn().await {
            Ok(child) => child,
            Err(e) => {
                tracing::error!(error = %e, "chat_service.send_message spawn failed");
                cleanup_and_err!(ChatServiceError::SpawnFailed(e.to_string()));
            }
        };
        tracing::debug!(pid = ?child.id(), "chat_service.send_message spawn ok");

        let registry_worktree = working_directory.to_string_lossy().to_string();

        // 7b. Update process details in registry now that spawn succeeded
        let cancellation_token = CancellationToken::new();
        if let Some(pid) = child.id() {
            if let Err(e) = self
                .running_agent_registry
                .update_agent_process(
                    &registry_key,
                    pid,
                    &conversation_id.as_str(),
                    &agent_run_id,
                    Some(registry_worktree.clone()),
                    Some(cancellation_token.clone()),
                )
                .await
            {
                tracing::error!(
                    pid,
                    error = %e,
                    "chat_service.send_message: failed to update agent process in registry — slot claimed but PID not persisted"
                );
            }
        }

        // 8. Build background context and spawn
        let team_mode_val = self.team_mode.load(Ordering::Relaxed);
        let resolved_agent_name = chat_service_helpers::resolve_agent_with_team_mode(
            &context_type,
            entity_status.as_deref(),
            team_mode_val,
        )
        .to_string();

        let bg_ctx = chat_service_send_background::BackgroundRunContext {
            child,
            context_type,
            context_id: context_id.to_string(),
            conversation_id,
            agent_run_id: agent_run_id.clone(),
            stored_session_id,
            working_directory,
            cli_path: self.cli_path.clone(),
            plugin_dir: self.plugin_dir.clone(),
            repos: chat_service_send_background::BackgroundRunRepos {
                chat_message_repo: Arc::clone(&self.chat_message_repo),
                chat_attachment_repo: Arc::clone(&self.chat_attachment_repo),
                conversation_repo: Arc::clone(&self.conversation_repo),
                agent_run_repo: Arc::clone(&self.agent_run_repo),
                task_repo: Arc::clone(&self.task_repo),
                task_dependency_repo: Arc::clone(&self.task_dependency_repo),
                project_repo: Arc::clone(&self.project_repo),
                ideation_session_repo: Arc::clone(&self.ideation_session_repo),
                activity_event_repo: Arc::clone(&self.activity_event_repo),
                memory_event_repo: Arc::clone(&self.memory_event_repo),
                message_queue: Arc::clone(&self.message_queue),
                running_agent_registry: Arc::clone(&self.running_agent_registry),
                task_proposal_repo: self.task_proposal_repo.clone(),
                task_step_repo: self.task_step_repo.clone(),
            },
            execution_state: self.execution_state.clone(),
            question_state: self.question_state.clone(),
            plan_branch_repo: self.plan_branch_repo.lock().unwrap().clone(),
            app_handle: self.app_handle.clone(),
            run_chain_id,
            is_retry_attempt: false,
            user_message_content: Some(message.to_string()),
            conversation: Some(conversation.clone()),
            agent_name: Some(resolved_agent_name),
            team_mode: team_mode_val,
            cancellation_token,
            team_service: self.team_service.clone(),
            streaming_state_cache: self.streaming_state_cache.clone(),
        };

        // 9. Process stream in background (extracted to separate module)
        chat_service_send_background::spawn_send_message_background(bg_ctx);
        tracing::debug!(
            conversation_id = conversation_id.as_str(),
            "chat_service.send_message background spawn kicked"
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
                        run_chain_id: None,
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

    async fn is_agent_running(&self, context_type: ChatContextType, context_id: &str) -> bool {
        let key = RunningAgentKey::new(context_type.to_string(), context_id);
        self.running_agent_registry.is_running(&key).await
    }

    fn set_team_mode(&self, mode: bool) {
        self.team_mode.store(mode, Ordering::Relaxed);
    }

    fn set_plan_branch_repo(&self, repo: Arc<dyn PlanBranchRepository>) {
        *self.plan_branch_repo.lock().unwrap() = Some(repo);
    }
}

// ============================================================================
// Module re-exports are at the top of this file
// ============================================================================
