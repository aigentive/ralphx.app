// Execution Chat Service
//
// Manages persistent worker executions for tasks.
// Extends the context-aware chat system (Phase 15A) to persist and display
// worker execution output as chat conversations.
//
// Key features:
// - Creates chat_conversation and agent_run when spawning worker
// - Persists stream events to chat_messages
// - Captures claude_session_id for --resume support
// - Integrates with ExecutionMessageQueue for message queueing
// - Emits Tauri events for real-time UI updates
//
// Unlike OrchestratorService which handles ideation chat, this service
// is focused on task execution context (worker agent output).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;

use crate::application::orchestrator_service::{
    ChatChunkPayload, ChatMessageCreatedPayload, ChatRunCompletedPayload, ChatToolCallPayload,
};
use crate::infrastructure::agents::claude::{
    build_base_cli_command, add_prompt_args, configure_spawn,
    StreamProcessor, StreamEvent as ProcessorStreamEvent, ToolCall,
};
use crate::domain::entities::{
    AgentRun, ChatConversation, ChatConversationId, ChatContextType, ChatMessage, ChatMessageId,
    MessageRole, TaskId,
};
use crate::domain::repositories::{
    AgentRunRepository, ChatConversationRepository, ChatMessageRepository, TaskRepository,
};
use crate::domain::services::ExecutionMessageQueue;

// ============================================================================
// Types
// ============================================================================

/// Result from spawning a worker with persistence
#[derive(Debug, Clone)]
pub struct SpawnResult {
    /// The conversation ID created for this execution
    pub conversation_id: ChatConversationId,
    /// The agent run ID tracking this execution
    pub agent_run_id: String,
}

/// Result from a completed execution
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub response_text: String,
    pub tool_calls: Vec<ToolCall>,
    pub claude_session_id: Option<String>,
    pub conversation_id: ChatConversationId,
}

/// Event emitted during execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionEvent {
    /// Execution has started
    RunStarted {
        run_id: String,
        conversation_id: String,
        task_id: String,
    },
    /// Text chunk received from worker
    TextChunk {
        text: String,
        conversation_id: String,
    },
    /// Tool call detected
    ToolCallDetected {
        tool_name: String,
        tool_id: Option<String>,
        arguments: serde_json::Value,
        conversation_id: String,
    },
    /// Tool call completed
    ToolCallCompleted {
        tool_name: String,
        tool_id: Option<String>,
        result: Option<serde_json::Value>,
        conversation_id: String,
    },
    /// Execution completed
    RunCompleted {
        conversation_id: String,
        claude_session_id: Option<String>,
        response_text: String,
        task_id: String,
    },
    /// Queue message was sent to worker
    QueueSent {
        message_id: String,
        conversation_id: String,
        task_id: String,
    },
    /// Error occurred
    Error {
        conversation_id: Option<String>,
        task_id: Option<String>,
        error: String,
    },
}

// ============================================================================
// Error type
// ============================================================================

#[derive(Debug, Clone)]
pub enum ExecutionChatError {
    AgentNotAvailable(String),
    SpawnFailed(String),
    CommunicationFailed(String),
    ParseError(String),
    TaskNotFound(String),
    ConversationNotFound(String),
    RepositoryError(String),
    AgentRunFailed(String),
}

impl std::fmt::Display for ExecutionChatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AgentNotAvailable(msg) => write!(f, "Agent not available: {}", msg),
            Self::SpawnFailed(msg) => write!(f, "Failed to spawn agent: {}", msg),
            Self::CommunicationFailed(msg) => write!(f, "Communication failed: {}", msg),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
            Self::TaskNotFound(msg) => write!(f, "Task not found: {}", msg),
            Self::ConversationNotFound(msg) => write!(f, "Conversation not found: {}", msg),
            Self::RepositoryError(msg) => write!(f, "Repository error: {}", msg),
            Self::AgentRunFailed(msg) => write!(f, "Agent run failed: {}", msg),
        }
    }
}

impl std::error::Error for ExecutionChatError {}

// ============================================================================
// ExecutionChatService trait
// ============================================================================

/// Service for managing persistent worker executions
///
/// This service handles:
/// - Spawning worker agents with persistence to database
/// - Persisting stream events as chat messages
/// - Capturing claude_session_id for --resume support
/// - Processing queued messages when worker completes
#[async_trait]
pub trait ExecutionChatService: Send + Sync {
    /// Spawn a worker with persistence
    ///
    /// Creates a new chat_conversation (context_type: 'task_execution')
    /// and agent_run, then spawns the Claude CLI with the worker agent.
    ///
    /// Returns the conversation_id and agent_run_id immediately.
    /// Streaming output is persisted in the background.
    async fn spawn_with_persistence(
        &self,
        task_id: &TaskId,
        prompt: &str,
    ) -> Result<SpawnResult, ExecutionChatError>;

    /// Get the active conversation for a task execution
    async fn get_execution_conversation(
        &self,
        task_id: &TaskId,
    ) -> Result<Option<ChatConversation>, ExecutionChatError>;

    /// List all execution attempts for a task
    async fn list_task_executions(
        &self,
        task_id: &TaskId,
    ) -> Result<Vec<ChatConversation>, ExecutionChatError>;

    /// Complete an execution (update conversation with claude_session_id)
    async fn complete_execution(
        &self,
        conversation_id: &ChatConversationId,
        claude_session_id: Option<String>,
    ) -> Result<(), ExecutionChatError>;

    /// Check if the execution service is available
    async fn is_available(&self) -> bool;

    /// Get the active agent run for a conversation
    async fn get_active_run(
        &self,
        conversation_id: &ChatConversationId,
    ) -> Result<Option<AgentRun>, ExecutionChatError>;
}

// ============================================================================
// ClaudeExecutionChatService - Production implementation
// ============================================================================

/// Production implementation using Claude CLI with persistence
pub struct ClaudeExecutionChatService<R: Runtime = tauri::Wry> {
    cli_path: PathBuf,
    plugin_dir: PathBuf,
    working_directory: PathBuf,
    chat_message_repo: Arc<dyn ChatMessageRepository>,
    conversation_repo: Arc<dyn ChatConversationRepository>,
    agent_run_repo: Arc<dyn AgentRunRepository>,
    task_repo: Arc<dyn TaskRepository>,
    message_queue: Arc<ExecutionMessageQueue>,
    app_handle: Option<AppHandle<R>>,
    model: String,
}

impl<R: Runtime> ClaudeExecutionChatService<R> {
    pub fn new(
        chat_message_repo: Arc<dyn ChatMessageRepository>,
        conversation_repo: Arc<dyn ChatConversationRepository>,
        agent_run_repo: Arc<dyn AgentRunRepository>,
        task_repo: Arc<dyn TaskRepository>,
        message_queue: Arc<ExecutionMessageQueue>,
    ) -> Self {
        let cli_path = which::which("claude").unwrap_or_else(|_| PathBuf::from("claude"));
        // Working directory should be project root (parent of src-tauri), not src-tauri itself
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let working_directory = cwd
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or(cwd.clone());
        let plugin_dir = working_directory.join("ralphx-plugin");

        println!(">>> ClaudeExecutionChatService::new()");
        println!(">>>   cli_path: {:?}", cli_path);
        println!(">>>   cwd: {:?}", cwd);
        println!(">>>   working_directory: {:?}", working_directory);
        println!(">>>   plugin_dir: {:?}", plugin_dir);
        println!(">>>   plugin_dir exists: {}", plugin_dir.exists());

        Self {
            cli_path,
            plugin_dir,
            working_directory,
            chat_message_repo,
            conversation_repo,
            agent_run_repo,
            task_repo,
            message_queue,
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
        self.working_directory = path.into();
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

    /// Build the initial prompt with task context
    fn build_initial_prompt(task_id: &TaskId, prompt: &str) -> String {
        format!(
            "RalphX Task Execution ID: {}\n\n{}",
            task_id.as_str(),
            prompt
        )
    }

    /// Create a Claude CLI command for worker execution
    fn build_command(&self, prompt: &str, resume_session_id: Option<&str>) -> Command {
        let agent = if resume_session_id.is_none() { Some("worker") } else { None };

        let mut cmd = build_base_cli_command(&self.cli_path, &self.plugin_dir);
        cmd.env("RALPHX_AGENT_TYPE", "worker");
        add_prompt_args(&mut cmd, prompt, agent, resume_session_id);
        configure_spawn(&mut cmd, &self.working_directory);
        cmd
    }

    /// Process streaming output from Claude CLI and persist to database
    #[allow(dead_code)] // Used when processing queue (called from spawn_with_persistence background task)
    async fn process_stream(
        &self,
        mut child: tokio::process::Child,
        conversation_id: &ChatConversationId,
        task_id: &TaskId,
    ) -> Result<(String, Vec<ToolCall>, Option<String>), ExecutionChatError> {
        let stdout = child.stdout.take().ok_or_else(|| {
            ExecutionChatError::SpawnFailed("Failed to capture stdout".to_string())
        })?;

        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        let mut processor = StreamProcessor::new();

        let conversation_id_str = conversation_id.as_str();
        let task_id_str = task_id.as_str();

        while let Some(line) = lines
            .next_line()
            .await
            .map_err(|e| ExecutionChatError::CommunicationFailed(e.to_string()))?
        {
            if let Some(msg) = StreamProcessor::parse_line(&line) {
                let events = processor.process_message(msg);

                for event in events {
                    match event {
                        ProcessorStreamEvent::TextChunk(text) => {
                            // Emit execution:chunk event for ChatPanel
                            self.emit_event(
                                "execution:chunk",
                                ChatChunkPayload {
                                    text: text.clone(),
                                    conversation_id: conversation_id_str.to_string(),
                                },
                            );

                            // ALSO emit agent:message event for Activity Stream
                            self.emit_event(
                                "agent:message",
                                serde_json::json!({
                                    "taskId": task_id_str,
                                    "type": "text",
                                    "content": text,
                                    "timestamp": chrono::Utc::now().timestamp_millis(),
                                }),
                            );
                        }
                        ProcessorStreamEvent::ToolCallStarted { name, id } => {
                            self.emit_event(
                                "execution:tool_call",
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
                            // Emit execution:tool_call event for ChatPanel
                            self.emit_event(
                                "execution:tool_call",
                                ChatToolCallPayload {
                                    tool_name: tool_call.name.clone(),
                                    tool_id: tool_call.id.clone(),
                                    arguments: tool_call.arguments.clone(),
                                    result: None,
                                    conversation_id: conversation_id_str.to_string(),
                                },
                            );

                            // ALSO emit agent:message event for Activity Stream
                            self.emit_event(
                                "agent:message",
                                serde_json::json!({
                                    "taskId": task_id_str,
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
                        ProcessorStreamEvent::SessionId(_) => {
                            // Session ID captured in processor.finish()
                        }
                        ProcessorStreamEvent::ToolResultReceived {
                            tool_use_id,
                            result,
                        } => {
                            // Re-emit tool call with result
                            self.emit_event(
                                "execution:tool_call",
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

        // Wait for process to complete
        let status = child
            .wait()
            .await
            .map_err(|e| ExecutionChatError::CommunicationFailed(e.to_string()))?;

        if !status.success() && result.response_text.is_empty() {
            return Err(ExecutionChatError::AgentRunFailed(
                "Agent exited with non-zero status".to_string(),
            ));
        }

        Ok((result.response_text, result.tool_calls, result.session_id))
    }

    /// Process queued messages after worker completes a response
    #[allow(dead_code)] // Will be used when queue processing is fully integrated
    async fn process_queue(
        &self,
        task_id: &TaskId,
        conversation_id: &ChatConversationId,
        current_session_id: &str,
    ) -> Result<(), ExecutionChatError> {
        while let Some(queued_msg) = self.message_queue.pop(task_id) {
            let conversation_id_str = conversation_id.as_str();
            let task_id_str = task_id.as_str();

            // Emit queue sent event
            self.emit_event(
                "execution:queue_sent",
                serde_json::json!({
                    "message_id": queued_msg.id,
                    "conversation_id": conversation_id_str,
                    "task_id": task_id_str,
                }),
            );

            // Persist user message
            let mut user_msg = ChatMessage::user_about_task(task_id.clone(), &queued_msg.content);
            user_msg.conversation_id = Some(*conversation_id);
            let user_msg_id = user_msg.id.as_str().to_string();

            self.chat_message_repo
                .create(user_msg)
                .await
                .map_err(|e| ExecutionChatError::RepositoryError(e.to_string()))?;

            // Emit message created event
            self.emit_event(
                "execution:message_created",
                ChatMessageCreatedPayload {
                    message_id: user_msg_id,
                    conversation_id: conversation_id_str.to_string(),
                    role: "user".to_string(),
                    content: queued_msg.content.clone(),
                },
            );

            // Send via --resume
            let mut cmd = self.build_command(&queued_msg.content, Some(current_session_id));
            let child = cmd
                .spawn()
                .map_err(|e| ExecutionChatError::SpawnFailed(e.to_string()))?;

            // Process streaming output
            let (response_text, tool_calls, new_session_id) = self
                .process_stream(child, conversation_id, task_id)
                .await?;

            // Persist assistant response
            if !response_text.is_empty() || !tool_calls.is_empty() {
                let assistant_msg = ChatMessage {
                    id: ChatMessageId::new(),
                    session_id: None,
                    project_id: None,
                    task_id: Some(task_id.clone()),
                    conversation_id: Some(*conversation_id),
                    role: MessageRole::Worker,
                    content: response_text.clone(),
                    metadata: None,
                    parent_message_id: None,
                    tool_calls: if !tool_calls.is_empty() {
                        Some(serde_json::to_string(&tool_calls).unwrap_or_default())
                    } else {
                        None
                    },
                    created_at: chrono::Utc::now(),
                };

                let assistant_msg_id = assistant_msg.id.as_str().to_string();
                self.chat_message_repo
                    .create(assistant_msg)
                    .await
                    .map_err(|e| ExecutionChatError::RepositoryError(e.to_string()))?;

                // Emit message created event for assistant message
                self.emit_event(
                    "execution:message_created",
                    ChatMessageCreatedPayload {
                        message_id: assistant_msg_id,
                        conversation_id: conversation_id_str.to_string(),
                        role: "worker".to_string(),
                        content: response_text.clone(),
                    },
                );
            }

            // Update session_id if changed (shouldn't normally happen with --resume)
            if let Some(ref _new_id) = new_session_id {
                // Session ID should remain the same when using --resume
                // but we track it just in case
            }
        }

        Ok(())
    }
}

#[async_trait]
impl<R: Runtime> ExecutionChatService for ClaudeExecutionChatService<R> {
    async fn spawn_with_persistence(
        &self,
        task_id: &TaskId,
        prompt: &str,
    ) -> Result<SpawnResult, ExecutionChatError> {
        // Verify task exists
        self.task_repo
            .get_by_id(task_id)
            .await
            .map_err(|e| ExecutionChatError::RepositoryError(e.to_string()))?
            .ok_or_else(|| ExecutionChatError::TaskNotFound(task_id.as_str().to_string()))?;

        // 1. Create new conversation for this execution attempt
        let conversation = ChatConversation::new_task_execution(task_id.clone());
        let conversation_id = conversation.id;
        let conversation_id_str = conversation_id.as_str();

        self.conversation_repo
            .create(conversation.clone())
            .await
            .map_err(|e| ExecutionChatError::RepositoryError(e.to_string()))?;

        // 2. Create agent run record (status: running)
        let agent_run = AgentRun::new(conversation_id);
        let agent_run_id = agent_run.id.as_str().to_string();
        self.agent_run_repo
            .create(agent_run)
            .await
            .map_err(|e| ExecutionChatError::RepositoryError(e.to_string()))?;

        // Emit run started event
        self.emit_event(
            "execution:run_started",
            serde_json::json!({
                "run_id": &agent_run_id,
                "conversation_id": conversation_id_str,
                "task_id": task_id.as_str(),
            }),
        );

        // 3. Build initial prompt with task context
        let initial_prompt = Self::build_initial_prompt(task_id, prompt);

        // 4. Spawn Claude CLI with worker agent
        let mut cmd = self.build_command(&initial_prompt, None);
        println!(">>> spawn_with_persistence: Spawning Claude CLI");
        println!(">>>   cli_path: {:?}", self.cli_path);
        println!(">>>   plugin_dir: {:?}", self.plugin_dir);
        println!(">>>   working_directory: {:?}", self.working_directory);
        println!(">>>   initial_prompt: {}", initial_prompt);

        let child = cmd
            .spawn()
            .map_err(|e| {
                println!(">>> spawn_with_persistence: SPAWN FAILED: {}", e);
                ExecutionChatError::SpawnFailed(e.to_string())
            })?;
        println!(">>> spawn_with_persistence: Claude CLI spawned successfully, pid={:?}", child.id());

        // Clone values needed for async block
        let task_id_clone = task_id.clone();
        let chat_message_repo = Arc::clone(&self.chat_message_repo);
        let conversation_repo = Arc::clone(&self.conversation_repo);
        let agent_run_repo = Arc::clone(&self.agent_run_repo);
        let task_repo = Arc::clone(&self.task_repo);
        let message_queue = Arc::clone(&self.message_queue);
        let app_handle = self.app_handle.clone();
        let agent_run_id_clone = agent_run_id.clone();
        let cli_path = self.cli_path.clone();
        let plugin_dir = self.plugin_dir.clone();
        let working_directory = self.working_directory.clone();

        // 5. Process streaming in background
        tokio::spawn(async move {
            // Create a helper service for processing (without app_handle to avoid complexity)
            let process_result = process_stream_background(
                child,
                &conversation_id,
                &task_id_clone,
                app_handle.clone(),
            )
            .await;

            match process_result {
                Ok((response_text, tool_calls, claude_session_id)) => {
                    // Update conversation with claude_session_id
                    if let Some(ref sess_id) = claude_session_id {
                        let _ = conversation_repo
                            .update_claude_session_id(&conversation_id, sess_id)
                            .await;
                    }

                    // Persist assistant message
                    if !response_text.is_empty() || !tool_calls.is_empty() {
                        let assistant_msg = ChatMessage {
                            id: ChatMessageId::new(),
                            session_id: None,
                            project_id: None,
                            task_id: Some(task_id_clone.clone()),
                            conversation_id: Some(conversation_id),
                            role: MessageRole::Worker,
                            content: response_text.clone(),
                            metadata: None,
                            parent_message_id: None,
                            tool_calls: if !tool_calls.is_empty() {
                                Some(serde_json::to_string(&tool_calls).unwrap_or_default())
                            } else {
                                None
                            },
                            created_at: chrono::Utc::now(),
                        };

                        let assistant_msg_id = assistant_msg.id.as_str().to_string();
                        let _ = chat_message_repo.create(assistant_msg).await;

                        // Emit message created event
                        if let Some(ref handle) = app_handle {
                            let _ = handle.emit(
                                "execution:message_created",
                                ChatMessageCreatedPayload {
                                    message_id: assistant_msg_id,
                                    conversation_id: conversation_id.as_str(),
                                    role: "worker".to_string(),
                                    content: response_text.clone(),
                                },
                            );
                        }
                    }

                    // Complete agent run
                    let _ = agent_run_repo
                        .complete(&crate::domain::entities::AgentRunId::from_string(
                            &agent_run_id_clone,
                        ))
                        .await;

                    // Transition task: Executing → ExecutionDone → PendingReview
                    // (QA is disabled by default, so we skip QaRefining and go directly to review)
                    println!(">>> Worker completed, transitioning task {} to PendingReview", task_id_clone.as_str());
                    if let Ok(Some(mut task)) = task_repo.get_by_id(&task_id_clone).await {
                        // Only transition if still in Executing state
                        if task.internal_status == crate::domain::entities::InternalStatus::Executing {
                            // Skip ExecutionDone and go directly to PendingReview
                            // (ExecutionDone is a transient state when QA is disabled)
                            task.internal_status = crate::domain::entities::InternalStatus::PendingReview;
                            task.touch();
                            let _ = task_repo.update(&task).await;
                            println!(">>> Task {} transitioned to PendingReview", task_id_clone.as_str());

                            // Emit event for UI update (use task:event channel with correct schema)
                            if let Some(ref handle) = app_handle {
                                let _ = handle.emit(
                                    "task:event",
                                    serde_json::json!({
                                        "type": "status_changed",
                                        "taskId": task_id_clone.as_str(),
                                        "from": "executing",
                                        "to": "pending_review",
                                        "changedBy": "agent",
                                    }),
                                );
                            }

                            // TODO: Spawn reviewer agent here
                            // For now, the task just waits in PendingReview for manual approval
                            println!(">>> Task {} awaiting review (AI reviewer not yet implemented)", task_id_clone.as_str());
                        }
                    }

                    // Emit run completed event
                    if let Some(ref handle) = app_handle {
                        let _ = handle.emit(
                            "execution:run_completed",
                            ChatRunCompletedPayload {
                                conversation_id: conversation_id.as_str(),
                                claude_session_id: claude_session_id.clone(),
                            },
                        );
                    }

                    // Process queued messages
                    if let Some(ref sess_id) = claude_session_id {
                        // Process queue (simplified - would need full service context for real impl)
                        while let Some(queued_msg) = message_queue.pop(&task_id_clone) {
                            if let Some(ref handle) = app_handle {
                                let _ = handle.emit(
                                    "execution:queue_sent",
                                    serde_json::json!({
                                        "message_id": queued_msg.id,
                                        "conversation_id": conversation_id.as_str(),
                                        "task_id": task_id_clone.as_str(),
                                    }),
                                );
                            }

                            // Persist user message
                            let mut user_msg = ChatMessage::user_about_task(
                                task_id_clone.clone(),
                                &queued_msg.content,
                            );
                            user_msg.conversation_id = Some(conversation_id);
                            let _ = chat_message_repo.create(user_msg).await;

                            // Send via --resume and process using shared helpers
                            let mut cmd = build_base_cli_command(&cli_path, &plugin_dir);
                            cmd.env("RALPHX_AGENT_TYPE", "worker");
                            add_prompt_args(&mut cmd, &queued_msg.content, None, Some(sess_id));
                            configure_spawn(&mut cmd, &working_directory);

                            if let Ok(child) = cmd.spawn() {
                                if let Ok((response, tools, _)) = process_stream_background(
                                    child,
                                    &conversation_id,
                                    &task_id_clone,
                                    app_handle.clone(),
                                )
                                .await
                                {
                                    // Persist assistant response
                                    if !response.is_empty() || !tools.is_empty() {
                                        let assistant_msg = ChatMessage {
                                            id: ChatMessageId::new(),
                                            session_id: None,
                                            project_id: None,
                                            task_id: Some(task_id_clone.clone()),
                                            conversation_id: Some(conversation_id),
                                            role: MessageRole::Worker,
                                            content: response,
                                            metadata: None,
                                            parent_message_id: None,
                                            tool_calls: if !tools.is_empty() {
                                                Some(
                                                    serde_json::to_string(&tools)
                                                        .unwrap_or_default(),
                                                )
                                            } else {
                                                None
                                            },
                                            created_at: chrono::Utc::now(),
                                        };
                                        let _ = chat_message_repo.create(assistant_msg).await;
                                    }
                                }
                            }
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
                            "execution:error",
                            serde_json::json!({
                                "conversation_id": conversation_id.as_str(),
                                "task_id": task_id_clone.as_str(),
                                "error": e,
                            }),
                        );
                    }
                }
            }
        });

        Ok(SpawnResult {
            conversation_id,
            agent_run_id,
        })
    }

    async fn get_execution_conversation(
        &self,
        task_id: &TaskId,
    ) -> Result<Option<ChatConversation>, ExecutionChatError> {
        self.conversation_repo
            .get_active_for_context(ChatContextType::TaskExecution, task_id.as_str())
            .await
            .map_err(|e| ExecutionChatError::RepositoryError(e.to_string()))
    }

    async fn list_task_executions(
        &self,
        task_id: &TaskId,
    ) -> Result<Vec<ChatConversation>, ExecutionChatError> {
        self.conversation_repo
            .get_by_context(ChatContextType::TaskExecution, task_id.as_str())
            .await
            .map_err(|e| ExecutionChatError::RepositoryError(e.to_string()))
    }

    async fn complete_execution(
        &self,
        conversation_id: &ChatConversationId,
        claude_session_id: Option<String>,
    ) -> Result<(), ExecutionChatError> {
        if let Some(session_id) = claude_session_id {
            self.conversation_repo
                .update_claude_session_id(conversation_id, &session_id)
                .await
                .map_err(|e| ExecutionChatError::RepositoryError(e.to_string()))?;
        }
        Ok(())
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
    ) -> Result<Option<AgentRun>, ExecutionChatError> {
        self.agent_run_repo
            .get_active_for_conversation(conversation_id)
            .await
            .map_err(|e| ExecutionChatError::RepositoryError(e.to_string()))
    }
}

/// Helper function to process stream in background task
async fn process_stream_background<R: Runtime>(
    mut child: tokio::process::Child,
    conversation_id: &ChatConversationId,
    task_id: &TaskId,
    app_handle: Option<AppHandle<R>>,
) -> Result<(String, Vec<ToolCall>, Option<String>), String> {
    println!(">>> process_stream_background: Starting to process stream");
    println!(">>>   conversation_id: {}", conversation_id.as_str());
    println!(">>>   task_id: {}", task_id.as_str());
    println!(">>>   app_handle is_some: {}", app_handle.is_some());

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| {
            println!(">>> process_stream_background: Failed to capture stdout!");
            "Failed to capture stdout".to_string()
        })?;

    println!(">>> process_stream_background: Got stdout, starting to read lines");

    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();
    let mut processor = StreamProcessor::new();

    let conversation_id_str = conversation_id.as_str();
    let task_id_str = task_id.as_str();

    let mut line_count = 0;
    while let Ok(Some(line)) = lines.next_line().await {
        line_count += 1;
        if line_count <= 5 || line_count % 100 == 0 {
            println!(">>> process_stream_background: line {}: {}", line_count, &line[..line.len().min(100)]);
        }
        if let Some(msg) = StreamProcessor::parse_line(&line) {
            let events = processor.process_message(msg);

            for event in events {
                match event {
                    ProcessorStreamEvent::TextChunk(text) => {
                        if let Some(ref handle) = app_handle {
                            println!(">>> EMITTING execution:chunk: {}", &text[..text.len().min(50)]);
                            // Emit execution:chunk event for ChatPanel
                            let _ = handle.emit(
                                "execution:chunk",
                                ChatChunkPayload {
                                    text: text.clone(),
                                    conversation_id: conversation_id_str.to_string(),
                                },
                            );

                            println!(">>> EMITTING agent:message for task {}", task_id_str);
                            // ALSO emit agent:message event for Activity Stream
                            let _ = handle.emit(
                                "agent:message",
                                serde_json::json!({
                                    "taskId": task_id_str,
                                    "type": "text",
                                    "content": text,
                                    "timestamp": chrono::Utc::now().timestamp_millis(),
                                }),
                            );
                        }
                    }
                    ProcessorStreamEvent::ToolCallStarted { name, id } => {
                        if let Some(ref handle) = app_handle {
                            let _ = handle.emit(
                                "execution:tool_call",
                                ChatToolCallPayload {
                                    tool_name: name,
                                    tool_id: id,
                                    arguments: serde_json::Value::Null,
                                    result: None,
                                    conversation_id: conversation_id_str.to_string(),
                                },
                            );
                        }
                    }
                    ProcessorStreamEvent::ToolCallCompleted(tool_call) => {
                        if let Some(ref handle) = app_handle {
                            // Emit execution:tool_call event for ChatPanel
                            let _ = handle.emit(
                                "execution:tool_call",
                                ChatToolCallPayload {
                                    tool_name: tool_call.name.clone(),
                                    tool_id: tool_call.id.clone(),
                                    arguments: tool_call.arguments.clone(),
                                    result: None,
                                    conversation_id: conversation_id_str.to_string(),
                                },
                            );

                            // ALSO emit agent:message event for Activity Stream
                            let _ = handle.emit(
                                "agent:message",
                                serde_json::json!({
                                    "taskId": task_id_str,
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
                    ProcessorStreamEvent::SessionId(_) => {
                        // Session ID captured in processor.finish()
                    }
                    ProcessorStreamEvent::ToolResultReceived {
                        tool_use_id,
                        result,
                    } => {
                        if let Some(ref handle) = app_handle {
                            // Emit execution:tool_call event with result
                            let _ = handle.emit(
                                "execution:tool_call",
                                ChatToolCallPayload {
                                    tool_name: format!("result:{}", tool_use_id),
                                    tool_id: Some(tool_use_id.clone()),
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
    }

    println!(">>> process_stream_background: Finished reading lines, total: {}", line_count);

    let result = processor.finish();

    println!(">>> process_stream_background: Waiting for child process to complete");
    // Wait for process to complete
    let status = child.wait().await.map_err(|e| {
        println!(">>> process_stream_background: Wait failed: {}", e);
        e.to_string()
    })?;

    println!(">>> process_stream_background: Child process exited with status: {:?}", status);
    println!(">>> process_stream_background: response_text length: {}", result.response_text.len());
    println!(">>> process_stream_background: tool_calls count: {}", result.tool_calls.len());
    println!(">>> process_stream_background: session_id: {:?}", result.session_id);

    if !status.success() && result.response_text.is_empty() {
        println!(">>> process_stream_background: FAILED - non-zero exit with empty response");
        return Err("Agent exited with non-zero status".to_string());
    }

    println!(">>> process_stream_background: SUCCESS");
    Ok((result.response_text, result.tool_calls, result.session_id))
}

// ============================================================================
// MockExecutionChatService - For testing
// ============================================================================

/// Mock implementation for testing
pub struct MockExecutionChatService {
    responses: Mutex<Vec<MockExecutionResponse>>,
    is_available: Mutex<bool>,
    conversations: Mutex<Vec<ChatConversation>>,
    active_run: Mutex<Option<AgentRun>>,
}

pub struct MockExecutionResponse {
    pub text: String,
    pub tool_calls: Vec<ToolCall>,
    pub claude_session_id: Option<String>,
}

impl MockExecutionChatService {
    pub fn new() -> Self {
        Self {
            responses: Mutex::new(Vec::new()),
            is_available: Mutex::new(true),
            conversations: Mutex::new(Vec::new()),
            active_run: Mutex::new(None),
        }
    }

    pub async fn set_available(&self, available: bool) {
        *self.is_available.lock().await = available;
    }

    pub async fn queue_response(&self, response: MockExecutionResponse) {
        self.responses.lock().await.push(response);
    }

    pub async fn queue_text_response(&self, text: impl Into<String>) {
        self.queue_response(MockExecutionResponse {
            text: text.into(),
            tool_calls: Vec::new(),
            claude_session_id: Some(uuid::Uuid::new_v4().to_string()),
        })
        .await;
    }

    pub async fn set_active_run(&self, run: Option<AgentRun>) {
        *self.active_run.lock().await = run;
    }

    pub async fn add_conversation(&self, conversation: ChatConversation) {
        self.conversations.lock().await.push(conversation);
    }
}

impl Default for MockExecutionChatService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ExecutionChatService for MockExecutionChatService {
    async fn spawn_with_persistence(
        &self,
        task_id: &TaskId,
        _prompt: &str,
    ) -> Result<SpawnResult, ExecutionChatError> {
        if !*self.is_available.lock().await {
            return Err(ExecutionChatError::AgentNotAvailable(
                "Mock agent not available".to_string(),
            ));
        }

        let conversation = ChatConversation::new_task_execution(task_id.clone());
        let conversation_id = conversation.id;
        let agent_run = AgentRun::new(conversation_id);
        let agent_run_id = agent_run.id.as_str().to_string();

        self.conversations.lock().await.push(conversation);
        *self.active_run.lock().await = Some(agent_run);

        Ok(SpawnResult {
            conversation_id,
            agent_run_id,
        })
    }

    async fn get_execution_conversation(
        &self,
        task_id: &TaskId,
    ) -> Result<Option<ChatConversation>, ExecutionChatError> {
        let conversations = self.conversations.lock().await;
        Ok(conversations
            .iter()
            .find(|c| {
                c.context_type == ChatContextType::TaskExecution
                    && c.context_id == task_id.as_str()
            })
            .cloned())
    }

    async fn list_task_executions(
        &self,
        task_id: &TaskId,
    ) -> Result<Vec<ChatConversation>, ExecutionChatError> {
        let conversations = self.conversations.lock().await;
        Ok(conversations
            .iter()
            .filter(|c| {
                c.context_type == ChatContextType::TaskExecution
                    && c.context_id == task_id.as_str()
            })
            .cloned()
            .collect())
    }

    async fn complete_execution(
        &self,
        conversation_id: &ChatConversationId,
        claude_session_id: Option<String>,
    ) -> Result<(), ExecutionChatError> {
        let mut conversations = self.conversations.lock().await;
        if let Some(conv) = conversations
            .iter_mut()
            .find(|c| c.id == *conversation_id)
        {
            conv.claude_session_id = claude_session_id;
        }
        Ok(())
    }

    async fn is_available(&self) -> bool {
        *self.is_available.lock().await
    }

    async fn get_active_run(
        &self,
        _conversation_id: &ChatConversationId,
    ) -> Result<Option<AgentRun>, ExecutionChatError> {
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
    async fn test_mock_service_spawn() {
        let service = MockExecutionChatService::new();
        let task_id = TaskId::new();

        let result = service
            .spawn_with_persistence(&task_id, "Execute the task")
            .await
            .unwrap();

        assert!(!result.conversation_id.as_str().is_empty());
        assert!(!result.agent_run_id.is_empty());
    }

    #[tokio::test]
    async fn test_mock_service_not_available() {
        let service = MockExecutionChatService::new();
        service.set_available(false).await;

        let task_id = TaskId::new();
        let result = service
            .spawn_with_persistence(&task_id, "Execute the task")
            .await;

        assert!(matches!(
            result,
            Err(ExecutionChatError::AgentNotAvailable(_))
        ));
    }

    #[tokio::test]
    async fn test_mock_service_get_execution_conversation() {
        let service = MockExecutionChatService::new();
        let task_id = TaskId::new();

        // Initially no conversation
        let conv = service.get_execution_conversation(&task_id).await.unwrap();
        assert!(conv.is_none());

        // After spawn, conversation exists
        service
            .spawn_with_persistence(&task_id, "Execute")
            .await
            .unwrap();

        let conv = service.get_execution_conversation(&task_id).await.unwrap();
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().context_type, ChatContextType::TaskExecution);
    }

    #[tokio::test]
    async fn test_mock_service_list_executions() {
        let service = MockExecutionChatService::new();
        let task_id = TaskId::new();

        // Spawn twice (simulating two execution attempts)
        service
            .spawn_with_persistence(&task_id, "First attempt")
            .await
            .unwrap();
        service
            .spawn_with_persistence(&task_id, "Second attempt")
            .await
            .unwrap();

        let executions = service.list_task_executions(&task_id).await.unwrap();
        assert_eq!(executions.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_service_complete_execution() {
        let service = MockExecutionChatService::new();
        let task_id = TaskId::new();

        let result = service
            .spawn_with_persistence(&task_id, "Execute")
            .await
            .unwrap();

        // Complete with session_id
        let session_id = "550e8400-e29b-41d4-a716-446655440000".to_string();
        service
            .complete_execution(&result.conversation_id, Some(session_id.clone()))
            .await
            .unwrap();

        let conv = service.get_execution_conversation(&task_id).await.unwrap();
        assert_eq!(conv.unwrap().claude_session_id, Some(session_id));
    }

    #[tokio::test]
    async fn test_mock_service_is_available() {
        let service = MockExecutionChatService::new();

        assert!(service.is_available().await);

        service.set_available(false).await;
        assert!(!service.is_available().await);
    }

    #[tokio::test]
    async fn test_mock_service_get_active_run() {
        let service = MockExecutionChatService::new();
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

    #[test]
    fn test_execution_error_display() {
        let err = ExecutionChatError::AgentNotAvailable("test".to_string());
        assert!(err.to_string().contains("Agent not available"));

        let err = ExecutionChatError::SpawnFailed("test".to_string());
        assert!(err.to_string().contains("spawn"));

        let err = ExecutionChatError::TaskNotFound("task-123".to_string());
        assert!(err.to_string().contains("Task not found"));

        let err = ExecutionChatError::AgentRunFailed("test".to_string());
        assert!(err.to_string().contains("Agent run failed"));
    }

    #[test]
    fn test_spawn_result() {
        let conversation_id = ChatConversationId::new();
        let result = SpawnResult {
            conversation_id,
            agent_run_id: "run-123".to_string(),
        };

        assert_eq!(result.conversation_id, conversation_id);
        assert_eq!(result.agent_run_id, "run-123");
    }

    #[test]
    fn test_execution_event_serialization() {
        let event = ExecutionEvent::RunStarted {
            run_id: "run-123".to_string(),
            conversation_id: "conv-456".to_string(),
            task_id: "task-789".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("RunStarted"));
        assert!(json.contains("run-123"));

        let event = ExecutionEvent::TextChunk {
            text: "Hello".to_string(),
            conversation_id: "conv-123".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("TextChunk"));
        assert!(json.contains("Hello"));

        let event = ExecutionEvent::RunCompleted {
            conversation_id: "conv-123".to_string(),
            claude_session_id: Some("sess-456".to_string()),
            response_text: "Done".to_string(),
            task_id: "task-789".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("RunCompleted"));
        assert!(json.contains("sess-456"));

        let event = ExecutionEvent::QueueSent {
            message_id: "msg-123".to_string(),
            conversation_id: "conv-456".to_string(),
            task_id: "task-789".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("QueueSent"));
        assert!(json.contains("msg-123"));
    }

    /// Tests for queue processing functionality
    mod queue_processing_tests {
        use super::*;
        use crate::domain::services::ExecutionMessageQueue;

        #[tokio::test]
        async fn test_queue_processing_with_mock_service() {
            // Create mock service with queue
            let _service = MockExecutionChatService::new();
            let message_queue = Arc::new(ExecutionMessageQueue::new());

            // Create a task
            let task_id = TaskId::new();

            // Queue a message before worker completes
            let queued_msg =
                message_queue.queue(task_id.clone(), "Add error handling".to_string());
            assert_eq!(message_queue.get_queued(&task_id).len(), 1);

            // After worker completes, queue should be processed
            // In real implementation, this happens in spawn_with_persistence background task
            // Here we verify the queue pop behavior
            let popped = message_queue.pop(&task_id);
            assert!(popped.is_some());
            assert_eq!(popped.unwrap().id, queued_msg.id);

            // Queue should now be empty
            assert_eq!(message_queue.get_queued(&task_id).len(), 0);
        }

        #[tokio::test]
        async fn test_multiple_queued_messages_processed_in_order() {
            let message_queue = Arc::new(ExecutionMessageQueue::new());
            let task_id = TaskId::new();

            // Queue multiple messages
            let msg1 = message_queue.queue(task_id.clone(), "First message".to_string());
            let msg2 = message_queue.queue(task_id.clone(), "Second message".to_string());
            let msg3 = message_queue.queue(task_id.clone(), "Third message".to_string());

            assert_eq!(message_queue.get_queued(&task_id).len(), 3);

            // Messages should be processed in FIFO order
            let popped1 = message_queue.pop(&task_id);
            assert_eq!(popped1.unwrap().id, msg1.id);

            let popped2 = message_queue.pop(&task_id);
            assert_eq!(popped2.unwrap().id, msg2.id);

            let popped3 = message_queue.pop(&task_id);
            assert_eq!(popped3.unwrap().id, msg3.id);

            // Queue should be empty
            assert!(message_queue.pop(&task_id).is_none());
        }

        #[tokio::test]
        async fn test_queue_empty_when_worker_completes() {
            let message_queue = Arc::new(ExecutionMessageQueue::new());
            let task_id = TaskId::new();

            // No messages queued
            assert_eq!(message_queue.get_queued(&task_id).len(), 0);

            // When worker completes with empty queue, nothing should happen
            let popped = message_queue.pop(&task_id);
            assert!(popped.is_none());
        }

        #[tokio::test]
        async fn test_queue_for_different_tasks_isolated() {
            let message_queue = Arc::new(ExecutionMessageQueue::new());
            let task1 = TaskId::new();
            let task2 = TaskId::new();

            // Queue messages for both tasks
            message_queue.queue(task1.clone(), "Task 1 message".to_string());
            message_queue.queue(task2.clone(), "Task 2 message".to_string());

            // Each task has its own queue
            assert_eq!(message_queue.get_queued(&task1).len(), 1);
            assert_eq!(message_queue.get_queued(&task2).len(), 1);

            // Popping from task1 doesn't affect task2
            let popped = message_queue.pop(&task1);
            assert!(popped.is_some());
            assert_eq!(popped.unwrap().content, "Task 1 message");

            assert_eq!(message_queue.get_queued(&task1).len(), 0);
            assert_eq!(message_queue.get_queued(&task2).len(), 1);
        }

        #[test]
        fn test_queue_sent_event_structure() {
            // Verify ExecutionEvent::QueueSent has correct structure
            let event = ExecutionEvent::QueueSent {
                message_id: "msg-123".to_string(),
                conversation_id: "conv-456".to_string(),
                task_id: "task-789".to_string(),
            };

            match event {
                ExecutionEvent::QueueSent {
                    message_id,
                    conversation_id,
                    task_id,
                } => {
                    assert_eq!(message_id, "msg-123");
                    assert_eq!(conversation_id, "conv-456");
                    assert_eq!(task_id, "task-789");
                }
                _ => panic!("Expected QueueSent event"),
            }
        }

        #[tokio::test]
        async fn test_process_queue_method_signature() {
            // Test that process_queue signature is correct for the implementation
            // The actual process_queue method is private, but we can verify the pattern
            // it follows by testing the queue's pop behavior

            let message_queue = Arc::new(ExecutionMessageQueue::new());
            let task_id = TaskId::new();

            // Queue messages
            message_queue.queue(task_id.clone(), "Message 1".to_string());
            message_queue.queue(task_id.clone(), "Message 2".to_string());

            // Simulate the while loop in process_queue
            let mut processed_count = 0;
            while let Some(_msg) = message_queue.pop(&task_id) {
                processed_count += 1;
                // In real implementation:
                // 1. Persist user message to chat_messages
                // 2. Send via --resume <claude_session_id>
                // 3. Continue streaming and persisting
            }

            assert_eq!(processed_count, 2);
            assert!(message_queue.pop(&task_id).is_none());
        }

        #[tokio::test]
        async fn test_queue_processing_flow() {
            // Test the complete flow:
            // 1. Worker is executing
            // 2. User queues messages
            // 3. Worker completes
            // 4. Queue is processed

            let message_queue = Arc::new(ExecutionMessageQueue::new());
            let task_id = TaskId::new();

            // Simulate worker executing (user can queue messages)
            let is_executing = true;

            if is_executing {
                // User queues messages while worker is running
                message_queue.queue(task_id.clone(), "Additional requirement 1".to_string());
                message_queue.queue(task_id.clone(), "Additional requirement 2".to_string());
            }

            assert_eq!(message_queue.get_queued(&task_id).len(), 2);

            // Worker completes - process queue
            let mut messages_sent = Vec::new();
            while let Some(msg) = message_queue.pop(&task_id) {
                messages_sent.push(msg.content);
            }

            assert_eq!(messages_sent.len(), 2);
            assert_eq!(messages_sent[0], "Additional requirement 1");
            assert_eq!(messages_sent[1], "Additional requirement 2");

            // Queue is now empty
            assert!(message_queue.pop(&task_id).is_none());
        }
    }
}
