// Chat Service Streaming Logic
//
// Extracted from chat_service.rs to improve modularity and reduce file size.
// Handles background stream processing and event emission.

use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::time::{timeout, Duration};
use tracing::info;

use crate::application::question_state::QuestionState;
use crate::application::team_events;
use crate::application::team_state_tracker::{TeammateHandle, TeammateStatus};
use crate::domain::entities::{
    ActivityEvent, ActivityEventType, ChatContextType, ChatConversationId, ChatMessageId, TaskId,
};
use crate::domain::repositories::{ActivityEventRepository, ChatMessageRepository, TaskRepository};
use crate::infrastructure::agents::claude::{
    apply_common_spawn_env, ClaudeCodeClient, ContentBlockItem, DiffContext, StreamEvent,
    StreamProcessor, TeammateSpawnConfig, ToolCall,
};
use tokio_util::sync::CancellationToken;

use super::chat_service_errors::StreamError;
use super::{
    event_context, events, has_meaningful_output, AgentChunkPayload, AgentHookPayload,
    AgentTaskCompletedPayload, AgentTaskStartedPayload, AgentToolCallPayload,
};

/// Final flush of accumulated content to DB before returning an error.
///
/// Ensures that any content streamed before timeout/cancellation/parse-stall
/// is persisted, so that the error handler can later append (rather than overwrite).
async fn flush_content_before_error(
    chat_message_repo: &Option<Arc<dyn ChatMessageRepository>>,
    assistant_message_id: &Option<String>,
    response_text: &str,
    tool_calls: &[ToolCall],
    content_blocks: &[ContentBlockItem],
) {
    if let (Some(ref repo), Some(ref msg_id)) = (chat_message_repo, assistant_message_id) {
        let current_tools = serde_json::to_string(tool_calls).ok();
        let current_blocks = serde_json::to_string(content_blocks).ok();
        let _ = repo
            .update_content(
                &ChatMessageId::from_string(msg_id.clone()),
                response_text,
                current_tools.as_deref(),
                current_blocks.as_deref(),
            )
            .await;
    }
}

/// Per-context-type timeout thresholds for stream processing.
///
/// Different agent contexts have different expected run durations.
/// Task execution needs generous timeouts for long-running commands,
/// while merge/review contexts should fail-fast on stalls.
#[derive(Debug, Clone)]
pub struct StreamTimeoutConfig {
    /// Max time to wait for a single line of stdout before killing the agent.
    pub line_read_timeout: Duration,
    /// Max time to tolerate stdout traffic with no parseable stream events.
    pub parse_stall_timeout: Duration,
    /// Teammate name (set when streaming a team member's output).
    #[allow(dead_code)]
    pub teammate_name: Option<String>,
    /// Teammate display color (set when streaming a team member's output).
    #[allow(dead_code)]
    pub teammate_color: Option<String>,
}

impl StreamTimeoutConfig {
    /// Returns timeout thresholds appropriate for the given context type.
    pub fn for_context(context_type: &ChatContextType) -> Self {
        match context_type {
            ChatContextType::Merge => Self {
                line_read_timeout: Duration::from_secs(180),
                parse_stall_timeout: Duration::from_secs(90),
                teammate_name: None,
                teammate_color: None,
            },
            ChatContextType::Review => Self {
                line_read_timeout: Duration::from_secs(300),
                parse_stall_timeout: Duration::from_secs(120),
                teammate_name: None,
                teammate_color: None,
            },
            // TaskExecution, Ideation, Task, Project — generous defaults
            _ => Self {
                line_read_timeout: Duration::from_secs(600),
                parse_stall_timeout: Duration::from_secs(180),
                teammate_name: None,
                teammate_color: None,
            },
        }
    }

    /// Attach team member identity to this config (builder pattern).
    #[allow(dead_code)]
    pub fn with_teammate(mut self, name: String, color: String) -> Self {
        self.teammate_name = Some(name);
        self.teammate_color = Some(color);
        self
    }
}

#[derive(Debug, Clone)]
pub struct StreamOutcome {
    pub response_text: String,
    pub tool_calls: Vec<ToolCall>,
    pub content_blocks: Vec<ContentBlockItem>,
    pub session_id: Option<String>,
}

impl StreamOutcome {
    pub fn has_meaningful_output(&self) -> bool {
        has_meaningful_output(&self.response_text, self.tool_calls.len())
    }
}

// ============================================================================
// Background stream processing
// ============================================================================

/// Process stream output in background, emitting events and persisting activity events
///
/// # Arguments
/// * `child` - The spawned Claude CLI process
/// * `context_type` - The chat context type
/// * `context_id` - The context ID (task_id, project_id, etc.)
/// * `conversation_id` - The conversation ID
/// * `app_handle` - Tauri app handle for events
/// * `activity_event_repo` - Repository for persisting activity events (optional)
/// * `task_repo` - Task repository for fetching current status (optional)
/// * `chat_message_repo` - Chat message repository for incremental persistence (optional)
/// * `assistant_message_id` - Pre-created assistant message ID for incremental updates (optional)
/// * `question_state` - QuestionState for checking pending questions (optional)
pub async fn process_stream_background<R: Runtime>(
    mut child: tokio::process::Child,
    context_type: ChatContextType,
    context_id: &str,
    conversation_id: &ChatConversationId,
    app_handle: Option<AppHandle<R>>,
    activity_event_repo: Option<Arc<dyn ActivityEventRepository>>,
    task_repo: Option<Arc<dyn TaskRepository>>,
    chat_message_repo: Option<Arc<dyn ChatMessageRepository>>,
    assistant_message_id: Option<String>,
    question_state: Option<Arc<QuestionState>>,
    cancellation_token: CancellationToken,
    team_service: Option<std::sync::Arc<crate::application::TeamService>>,
    team_mode: bool,
) -> Result<StreamOutcome, StreamError> {
    let mut timeout_config = StreamTimeoutConfig::for_context(&context_type);
    // Team leads wait long periods while teammates work — use 1-hour timeout
    if team_mode {
        timeout_config.line_read_timeout = Duration::from_secs(3600);
        timeout_config.parse_stall_timeout = Duration::from_secs(3600);
    }
    tracing::debug!(
        conversation_id = conversation_id.as_str(),
        %context_type,
        context_id,
        line_read_timeout_secs = timeout_config.line_read_timeout.as_secs(),
        parse_stall_timeout_secs = timeout_config.parse_stall_timeout.as_secs(),
        "process_stream_background start"
    );
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| StreamError::ProcessSpawnFailed {
            command: "claude".to_string(),
            error: "Failed to capture stdout".to_string(),
        })?;

    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| StreamError::ProcessSpawnFailed {
            command: "claude".to_string(),
            error: "Failed to capture stderr".to_string(),
        })?;

    let event_ctx = event_context(conversation_id, &context_type, context_id);
    let conversation_id_str = event_ctx.conversation_id.clone();
    let context_type_str = event_ctx.context_type.clone();
    let context_id_str = event_ctx.context_id.clone();
    let debug_path =
        std::env::temp_dir().join(format!("ralphx-stream-debug-{}.log", conversation_id_str));
    tracing::debug!(
        path = %debug_path.display(),
        "Debug log path (written on parse failure)"
    );

    // Parse task_id for activity persistence (only for TaskExecution context)
    let task_id_for_persistence = if context_type == ChatContextType::TaskExecution {
        Some(TaskId::from_string(context_id.to_string()))
    } else {
        None
    };

    // Spawn stderr reader
    let _stderr_handle = app_handle.clone();
    let _stderr_conv_id = conversation_id_str.clone();
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
    let mut debug_lines: Vec<String> = Vec::new();
    let mut lines_seen: usize = 0;
    let mut lines_parsed: usize = 0;
    let mut last_parsed_at = std::time::Instant::now();

    // Debounced flush for incremental persistence (every 2 seconds)
    let mut last_flush = std::time::Instant::now();
    const FLUSH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

    // Track Task tool_use_id → (team_name, teammate_name) for teammate lifecycle
    let mut teammate_task_map: HashMap<String, (String, String)> = HashMap::new();

    loop {
        // Race line-read (with timeout) against cancellation token
        let line = tokio::select! {
            biased;
            _ = cancellation_token.cancelled() => {
                tracing::info!(
                    conversation_id = %conversation_id_str,
                    lines_seen,
                    "Stream cancelled via cancellation token, killing agent"
                );
                let _ = child.kill().await;
                flush_content_before_error(
                    &chat_message_repo, &assistant_message_id,
                    &processor.response_text, &processor.tool_calls, &processor.content_blocks,
                ).await;
                return Err(StreamError::Cancelled);
            }
            read_result = timeout(timeout_config.line_read_timeout, lines.next_line()) => {
                match read_result {
                    Ok(Ok(Some(line))) => line,
                    Ok(Ok(None)) => break, // EOF — stream ended normally
                    Ok(Err(e)) => {
                        tracing::error!(
                            conversation_id = %conversation_id_str,
                            error = %e,
                            "Stream read error"
                        );
                        break;
                    }
                    Err(_) => {
                        // Timeout — no output for configured timeout seconds
                        // Check if agent is waiting for user input on a pending question
                        if let Some(ref qs) = question_state {
                            if qs.has_pending_for_session(context_id).await {
                                tracing::info!(
                                    conversation_id = %conversation_id_str,
                                    context_id,
                                    lines_seen,
                                    "Stream no output but pending question exists, resetting timeout"
                                );
                                continue;
                            }
                        }

                        tracing::warn!(
                            conversation_id = %conversation_id_str,
                            lines_seen,
                            lines_parsed,
                            "Stream timeout: no output for {} seconds, killing agent",
                            timeout_config.line_read_timeout.as_secs()
                        );
                        let _ = child.kill().await;
                        flush_content_before_error(
                            &chat_message_repo, &assistant_message_id,
                            &processor.response_text, &processor.tool_calls, &processor.content_blocks,
                        ).await;
                        return Err(StreamError::Timeout {
                            context_type,
                            elapsed_secs: timeout_config.line_read_timeout.as_secs(),
                        });
                    }
                }
            }
        };

        lines_seen += 1;
        if debug_lines.len() < 50 {
            debug_lines.push(line.clone());
        }
        if let Some(parsed) = StreamProcessor::parse_line(&line) {
            lines_parsed += 1;
            last_parsed_at = std::time::Instant::now();
            let stream_events = processor.process_parsed_line(parsed);

            for event in stream_events {
                match event {
                    StreamEvent::TextChunk(text) => {
                        if let Some(ref handle) = app_handle {
                            // Unified event
                            let _ = handle.emit(
                                events::AGENT_CHUNK,
                                AgentChunkPayload {
                                    text: text.clone(),
                                    conversation_id: conversation_id_str.clone(),
                                    context_type: context_type_str.clone(),
                                    context_id: context_id_str.clone(),
                                },
                            );

                            // Activity stream event for task execution
                            if context_type == ChatContextType::TaskExecution {
                                let _ = handle.emit(
                                    events::AGENT_MESSAGE,
                                    serde_json::json!({
                                        "taskId": context_id_str,
                                        "type": "text",
                                        "content": text,
                                        "timestamp": chrono::Utc::now().timestamp_millis(),
                                    }),
                                );

                                // Persist activity event to database
                                if let (Some(ref repo), Some(ref task_id)) =
                                    (&activity_event_repo, &task_id_for_persistence)
                                {
                                    let event = ActivityEvent::new_task_event(
                                        task_id.clone(),
                                        ActivityEventType::Text,
                                        text.clone(),
                                    );
                                    // Fetch current task status and add to event
                                    let event = if let Some(ref t_repo) = task_repo {
                                        if let Ok(Some(task)) = t_repo.get_by_id(task_id).await {
                                            event.with_status(task.internal_status)
                                        } else {
                                            event
                                        }
                                    } else {
                                        event
                                    };
                                    let _ = repo.save(event).await;
                                }
                            }
                        }
                    }
                    StreamEvent::Thinking(text) => {
                        // Activity stream event for task execution
                        if context_type == ChatContextType::TaskExecution {
                            if let Some(ref handle) = app_handle {
                                let _ = handle.emit(
                                    events::AGENT_MESSAGE,
                                    serde_json::json!({
                                        "taskId": context_id_str,
                                        "type": "thinking",
                                        "content": text,
                                        "timestamp": chrono::Utc::now().timestamp_millis(),
                                    }),
                                );
                            }

                            // Persist activity event to database
                            if let (Some(ref repo), Some(ref task_id)) =
                                (&activity_event_repo, &task_id_for_persistence)
                            {
                                let event = ActivityEvent::new_task_event(
                                    task_id.clone(),
                                    ActivityEventType::Thinking,
                                    text.clone(),
                                );
                                // Fetch current task status and add to event
                                let event = if let Some(ref t_repo) = task_repo {
                                    if let Ok(Some(task)) = t_repo.get_by_id(task_id).await {
                                        event.with_status(task.internal_status)
                                    } else {
                                        event
                                    }
                                } else {
                                    event
                                };
                                let _ = repo.save(event).await;
                            }
                        }
                    }
                    StreamEvent::ToolCallStarted {
                        name,
                        id,
                        parent_tool_use_id,
                    } => {
                        if let Some(ref handle) = app_handle {
                            let _ = handle.emit(
                                events::AGENT_TOOL_CALL,
                                AgentToolCallPayload {
                                    tool_name: name.clone(),
                                    tool_id: id.clone(),
                                    arguments: serde_json::Value::Null,
                                    result: None,
                                    conversation_id: conversation_id_str.clone(),
                                    context_type: context_type_str.clone(),
                                    context_id: context_id_str.clone(),
                                    diff_context: None,
                                    parent_tool_use_id,
                                },
                            );
                        }
                    }
                    StreamEvent::ToolCallCompleted {
                        mut tool_call,
                        parent_tool_use_id,
                    } => {
                        // Capture old file content for Edit/Write tool calls
                        let name_lower = tool_call.name.to_lowercase();
                        if name_lower == "edit" || name_lower == "write" {
                            if let Some(file_path) = tool_call
                                .arguments
                                .get("file_path")
                                .and_then(|v| v.as_str())
                            {
                                let old_content = std::fs::read_to_string(file_path).ok();
                                let diff_ctx = DiffContext {
                                    old_content,
                                    file_path: file_path.to_string(),
                                };
                                tool_call.diff_context = Some(diff_ctx.clone());

                                // Update processor's stored tool_call and content_block
                                // (they were pushed before this event was emitted)
                                if let Some(last_tc) = processor.tool_calls.last_mut() {
                                    last_tc.diff_context = Some(diff_ctx.clone());
                                }
                                if let Some(ContentBlockItem::ToolUse { diff_context, .. }) =
                                    processor.content_blocks.last_mut()
                                {
                                    *diff_context = serde_json::to_value(&diff_ctx).ok();
                                }
                            }
                        }

                        let diff_context_value = tool_call
                            .diff_context
                            .as_ref()
                            .and_then(|dc| serde_json::to_value(dc).ok());

                        if let Some(ref handle) = app_handle {
                            let _ = handle.emit(
                                events::AGENT_TOOL_CALL,
                                AgentToolCallPayload {
                                    tool_name: tool_call.name.clone(),
                                    tool_id: tool_call.id.clone(),
                                    arguments: tool_call.arguments.clone(),
                                    result: None,
                                    conversation_id: conversation_id_str.clone(),
                                    context_type: context_type_str.clone(),
                                    context_id: context_id_str.clone(),
                                    diff_context: diff_context_value,
                                    parent_tool_use_id: parent_tool_use_id.clone(),
                                },
                            );

                            // Activity stream event for task execution
                            if context_type == ChatContextType::TaskExecution {
                                let tool_content = format!(
                                    "{} ({})",
                                    tool_call.name,
                                    serde_json::to_string(&tool_call.arguments).unwrap_or_default()
                                );
                                let tool_metadata = serde_json::json!({
                                    "tool_name": tool_call.name,
                                    "arguments": tool_call.arguments,
                                });

                                let _ = handle.emit(
                                    events::AGENT_MESSAGE,
                                    serde_json::json!({
                                        "taskId": context_id_str,
                                        "type": "tool_call",
                                        "content": tool_content,
                                        "timestamp": chrono::Utc::now().timestamp_millis(),
                                        "metadata": tool_metadata,
                                    }),
                                );

                                // Persist activity event to database
                                if let (Some(ref repo), Some(ref task_id)) =
                                    (&activity_event_repo, &task_id_for_persistence)
                                {
                                    let event = ActivityEvent::new_task_event(
                                        task_id.clone(),
                                        ActivityEventType::ToolCall,
                                        tool_content,
                                    )
                                    .with_metadata(tool_metadata.to_string());
                                    // Fetch current task status and add to event
                                    let event = if let Some(ref t_repo) = task_repo {
                                        if let Ok(Some(task)) = t_repo.get_by_id(task_id).await {
                                            event.with_status(task.internal_status)
                                        } else {
                                            event
                                        }
                                    } else {
                                        event
                                    };
                                    let _ = repo.save(event).await;
                                }
                            }
                        }
                    }
                    StreamEvent::SessionId(_) => {
                        // Captured in processor.finish()
                    }
                    StreamEvent::TaskStarted {
                        tool_use_id,
                        description,
                        subagent_type,
                        model,
                        teammate_name: tm_name,
                        team_name: tm_team,
                    } => {
                        // Track teammate Task calls for lifecycle management
                        if let (Some(ref tn), Some(ref tt)) = (&tm_name, &tm_team) {
                            teammate_task_map.insert(tool_use_id.clone(), (tt.clone(), tn.clone()));

                            // Update status to Running via TeamService (persistence + events)
                            if let Some(ref service) = team_service {
                                let _ = service.update_teammate_status(tt, tn, TeammateStatus::Running).await;
                            }

                            // Emit agent:run_started with teammate_name for frontend
                            if let Some(ref handle) = app_handle {
                                let _ = handle.emit(
                                    events::AGENT_RUN_STARTED,
                                    serde_json::json!({
                                        "teammate_name": tn,
                                        "team_name": tt,
                                        "context_type": context_type_str,
                                        "context_id": context_id_str,
                                    }),
                                );
                            }
                        }

                        if let Some(ref handle) = app_handle {
                            let _ = handle.emit(
                                events::AGENT_TASK_STARTED,
                                AgentTaskStartedPayload {
                                    tool_use_id,
                                    description,
                                    subagent_type,
                                    model,
                                    teammate_name: tm_name,
                                    conversation_id: conversation_id_str.clone(),
                                    context_type: context_type_str.clone(),
                                    context_id: context_id_str.clone(),
                                },
                            );
                        }
                    }
                    StreamEvent::TaskCompleted {
                        tool_use_id,
                        agent_id,
                        total_duration_ms,
                        total_tokens,
                        total_tool_use_count,
                    } => {
                        // Check if this completes a teammate Task
                        let tm_name_for_payload = if let Some((tt, tn)) = teammate_task_map.remove(&tool_use_id) {
                            // Update status to Idle via TeamService (persistence + events)
                            if let Some(ref service) = team_service {
                                let _ = service.update_teammate_status(&tt, &tn, TeammateStatus::Idle).await;
                            }

                            // Emit agent:run_completed with teammate_name for frontend
                            if let Some(ref handle) = app_handle {
                                let _ = handle.emit(
                                    events::AGENT_RUN_COMPLETED,
                                    serde_json::json!({
                                        "teammate_name": tn,
                                        "team_name": tt,
                                        "context_type": context_type_str,
                                        "context_id": context_id_str,
                                    }),
                                );
                            }

                            Some(tn)
                        } else {
                            None
                        };

                        if let Some(ref handle) = app_handle {
                            let _ = handle.emit(
                                events::AGENT_TASK_COMPLETED,
                                AgentTaskCompletedPayload {
                                    tool_use_id,
                                    agent_id,
                                    total_duration_ms,
                                    total_tokens,
                                    total_tool_use_count,
                                    teammate_name: tm_name_for_payload,
                                    conversation_id: conversation_id_str.clone(),
                                    context_type: context_type_str.clone(),
                                    context_id: context_id_str.clone(),
                                },
                            );
                        }
                    }
                    StreamEvent::HookStarted {
                        hook_id,
                        hook_name,
                        hook_event,
                    } => {
                        if let Some(ref handle) = app_handle {
                            let _ = handle.emit(
                                events::AGENT_HOOK,
                                AgentHookPayload {
                                    hook_type: "started".to_string(),
                                    hook_name: Some(hook_name),
                                    hook_event: Some(hook_event),
                                    hook_id: Some(hook_id),
                                    output: None,
                                    outcome: None,
                                    exit_code: None,
                                    reason: None,
                                    conversation_id: conversation_id_str.clone(),
                                    context_type: context_type_str.clone(),
                                    context_id: context_id_str.clone(),
                                    timestamp: chrono::Utc::now().timestamp_millis(),
                                },
                            );
                        }
                    }
                    StreamEvent::HookCompleted {
                        hook_id,
                        hook_name,
                        hook_event,
                        output,
                        exit_code,
                        outcome,
                    } => {
                        if let Some(ref handle) = app_handle {
                            let _ = handle.emit(
                                events::AGENT_HOOK,
                                AgentHookPayload {
                                    hook_type: "completed".to_string(),
                                    hook_name: Some(hook_name),
                                    hook_event: Some(hook_event),
                                    hook_id: Some(hook_id),
                                    output,
                                    outcome,
                                    exit_code,
                                    reason: None,
                                    conversation_id: conversation_id_str.clone(),
                                    context_type: context_type_str.clone(),
                                    context_id: context_id_str.clone(),
                                    timestamp: chrono::Utc::now().timestamp_millis(),
                                },
                            );
                        }
                    }
                    StreamEvent::HookBlock { reason } => {
                        if let Some(ref handle) = app_handle {
                            let _ = handle.emit(
                                events::AGENT_HOOK,
                                AgentHookPayload {
                                    hook_type: "block".to_string(),
                                    hook_name: None,
                                    hook_event: None,
                                    hook_id: None,
                                    output: None,
                                    outcome: None,
                                    exit_code: None,
                                    reason: Some(reason),
                                    conversation_id: conversation_id_str.clone(),
                                    context_type: context_type_str.clone(),
                                    context_id: context_id_str.clone(),
                                    timestamp: chrono::Utc::now().timestamp_millis(),
                                },
                            );
                        }
                    }

                    StreamEvent::TeamCreated { team_name, config_path: _ } => {
                        // Create team via TeamService (persistence + events)
                        if let Some(ref service) = team_service {
                            if !service.team_exists(&team_name).await {
                                let _ = service.create_team(&team_name, &context_id_str, &context_type_str).await;
                            }
                        } else if let Some(ref handle) = app_handle {
                            // Fallback: emit event directly if no service available
                            team_events::emit_team_created(
                                handle,
                                &team_name,
                                &context_id_str,
                                &context_type_str,
                            );
                        }
                    }
                    StreamEvent::TeammateSpawned { teammate_name, team_name, agent_id: _, model, color, prompt, agent_type } => {
                        // Register teammate via TeamService (persistence + events)
                        // May already exist from approve_team_plan — add_teammate returns error if duplicate
                        if let Some(ref service) = team_service {
                            let _ = service.add_teammate(&team_name, &teammate_name, &color, &model, "team-member").await;
                        } else if let Some(ref handle) = app_handle {
                            // Fallback: emit event directly if no service available
                            team_events::emit_teammate_spawned(
                                handle,
                                &team_name,
                                &teammate_name,
                                &color,
                                &model,
                                "team-member",
                                &context_type_str,
                                &context_id_str,
                            );
                        }

                        // Auto-spawn a separate CLI worker process for this teammate.
                        // The lead registers teammates in-process via Task tool, but each
                        // teammate needs its own CLI process to actually execute work.
                        if !prompt.is_empty() {
                            if let (Some(ref service), Some(ref handle)) = (&team_service, &app_handle) {
                                let parent_session_id = processor.session_id.clone().unwrap_or_default();
                                let service = service.clone();
                                let handle_clone = handle.clone();
                                let ctx_type = context_type_str.clone();
                                let ctx_id = context_id_str.clone();

                                tokio::spawn(async move {
                                    spawn_teammate_worker(
                                        &teammate_name,
                                        &team_name,
                                        &parent_session_id,
                                        &prompt,
                                        &model,
                                        &color,
                                        &agent_type,
                                        &ctx_type,
                                        &ctx_id,
                                        service,
                                        handle_clone,
                                    ).await;
                                });
                            }
                        }
                    }
                    StreamEvent::TeamMessageSent { sender, recipient, content, message_type } => {
                        // Persist message and emit full-payload event via TeamService
                        use crate::application::team_state_tracker::TeamMessageType;

                        let msg_type = match message_type.as_str() {
                            "broadcast" => TeamMessageType::Broadcast,
                            _ => TeamMessageType::TeammateMessage,
                        };

                        if let Some(ref service) = team_service {
                            let _ = service
                                .add_teammate_message(
                                    // Derive team_name from active teams
                                    &{
                                        let teams = service.list_teams().await;
                                        teams.into_iter().next().unwrap_or_default()
                                    },
                                    &sender,
                                    recipient.as_deref(),
                                    &content,
                                    msg_type,
                                )
                                .await;
                        } else if let Some(ref handle) = app_handle {
                            // Fallback: emit event directly without persistence
                            let _ = handle.emit(
                                events::TEAM_MESSAGE,
                                serde_json::json!({
                                    "sender": sender,
                                    "recipient": recipient,
                                    "content": content,
                                    "message_type": message_type,
                                    "context_type": context_type_str,
                                    "context_id": context_id_str,
                                }),
                            );
                        }
                    }
                    StreamEvent::TeamDeleted { team_name } => {
                        // Disband team via TeamService (persistence + events)
                        if let Some(ref service) = team_service {
                            let _ = service.disband_team(&team_name).await;
                        } else if let Some(ref handle) = app_handle {
                            // Fallback: emit event directly if no service available
                            team_events::emit_team_disbanded(
                                handle,
                                &team_name,
                                &context_type_str,
                                &context_id_str,
                            );
                        }
                    }

                    StreamEvent::ToolResultReceived {
                        tool_use_id,
                        result,
                        parent_tool_use_id,
                    } => {
                        if let Some(ref handle) = app_handle {
                            let _ = handle.emit(
                                events::AGENT_TOOL_CALL,
                                AgentToolCallPayload {
                                    tool_name: format!("result:{}", tool_use_id),
                                    tool_id: Some(tool_use_id.clone()),
                                    arguments: serde_json::Value::Null,
                                    result: Some(result.clone()),
                                    conversation_id: conversation_id_str.clone(),
                                    context_type: context_type_str.clone(),
                                    context_id: context_id_str.clone(),
                                    diff_context: None,
                                    parent_tool_use_id,
                                },
                            );

                            // Activity stream event for task execution
                            if context_type == ChatContextType::TaskExecution {
                                let result_content =
                                    serde_json::to_string(&result).unwrap_or_default();
                                let result_metadata = serde_json::json!({
                                    "tool_use_id": tool_use_id,
                                });

                                let _ = handle.emit(
                                    events::AGENT_MESSAGE,
                                    serde_json::json!({
                                        "taskId": context_id_str,
                                        "type": "tool_result",
                                        "content": result_content,
                                        "timestamp": chrono::Utc::now().timestamp_millis(),
                                        "metadata": result_metadata,
                                    }),
                                );

                                // Persist activity event to database
                                if let (Some(ref repo), Some(ref task_id)) =
                                    (&activity_event_repo, &task_id_for_persistence)
                                {
                                    let event = ActivityEvent::new_task_event(
                                        task_id.clone(),
                                        ActivityEventType::ToolResult,
                                        result_content,
                                    )
                                    .with_metadata(result_metadata.to_string());
                                    // Fetch current task status and add to event
                                    let event = if let Some(ref t_repo) = task_repo {
                                        if let Ok(Some(task)) = t_repo.get_by_id(task_id).await {
                                            event.with_status(task.internal_status)
                                        } else {
                                            event
                                        }
                                    } else {
                                        event
                                    };
                                    let _ = repo.save(event).await;
                                }
                            }
                        }
                    }
                }
            }
        } else if lines_seen > 0 && last_parsed_at.elapsed() >= timeout_config.parse_stall_timeout {
            // Check if agent is waiting for user input on a pending question
            if let Some(ref qs) = question_state {
                if qs.has_pending_for_session(context_id).await {
                    tracing::info!(
                        conversation_id = %conversation_id_str,
                        context_id,
                        lines_seen,
                        "Stream parse stall but pending question exists, resetting stall timer"
                    );
                    last_parsed_at = std::time::Instant::now();
                    // Continue processing — the next timeout will be reset
                } else {
                    tracing::warn!(
                        conversation_id = %conversation_id_str,
                        lines_seen,
                        lines_parsed,
                        stall_secs = timeout_config.parse_stall_timeout.as_secs(),
                        "Stream parse stall: received stdout but no parseable events, killing agent"
                    );
                    let _ = child.kill().await;
                    flush_content_before_error(
                        &chat_message_repo, &assistant_message_id,
                        &processor.response_text, &processor.tool_calls, &processor.content_blocks,
                    ).await;
                    return Err(StreamError::ParseStall {
                        context_type,
                        elapsed_secs: timeout_config.parse_stall_timeout.as_secs(),
                        lines_seen,
                        lines_parsed,
                    });
                }
            } else {
                tracing::warn!(
                    conversation_id = %conversation_id_str,
                    lines_seen,
                    lines_parsed,
                    stall_secs = timeout_config.parse_stall_timeout.as_secs(),
                    "Stream parse stall: received stdout but no parseable events, killing agent"
                );
                let _ = child.kill().await;
                flush_content_before_error(
                    &chat_message_repo, &assistant_message_id,
                    &processor.response_text, &processor.tool_calls, &processor.content_blocks,
                ).await;
                return Err(StreamError::ParseStall {
                    context_type,
                    elapsed_secs: timeout_config.parse_stall_timeout.as_secs(),
                    lines_seen,
                    lines_parsed,
                });
            }
        }

        // Debounced flush: persist accumulated content every 2s for crash recovery
        if last_flush.elapsed() >= FLUSH_INTERVAL {
            if let (Some(ref repo), Some(ref msg_id)) = (&chat_message_repo, &assistant_message_id)
            {
                let current_text = processor.response_text.clone();
                let current_tools = serde_json::to_string(&processor.tool_calls).ok();
                let _ = repo
                    .update_content(
                        &ChatMessageId::from_string(msg_id.clone()),
                        &current_text,
                        current_tools.as_deref(),
                        None, // content_blocks only on final update
                    )
                    .await;
            }
            last_flush = std::time::Instant::now();
        }

        if lines_seen % 50 == 0 {
            tracing::debug!(
                conversation_id = %conversation_id_str,
                lines_seen,
                lines_parsed,
                response_len = processor.response_text.len(),
                tool_calls = processor.tool_calls.len(),
                "Stream progress"
            );
        }
    }

    let result = processor.finish();

    // Wait for stderr task
    let stderr_content = stderr_task.await.unwrap_or_default();

    // Wait for process
    let status = child.wait().await.map_err(|e| StreamError::AgentExit {
        exit_code: None,
        stderr: e.to_string(),
    })?;
    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        status.signal()
    };
    #[cfg(not(unix))]
    let signal: Option<i32> = None;

    // Log stderr and exit metadata when agent produced no output (critical diagnostic)
    if lines_seen == 0 {
        let stderr_preview = &stderr_content[..stderr_content.len().min(2000)];
        tracing::warn!(
            conversation_id = %conversation_id_str,
            exit_code = status.code(),
            exit_signal = signal,
            stderr_len = stderr_content.len(),
            "Stream ended with ZERO lines from stdout. stderr: {}",
            stderr_preview
        );
    }

    let outcome = StreamOutcome {
        response_text: result.response_text,
        tool_calls: result.tool_calls,
        content_blocks: result.content_blocks,
        session_id: result.session_id,
    };

    // Final flush of accumulated content so post-loop error returns don't lose data
    flush_content_before_error(
        &chat_message_repo, &assistant_message_id,
        &outcome.response_text, &outcome.tool_calls, &outcome.content_blocks,
    ).await;

    tracing::debug!(
        conversation_id = %conversation_id_str,
        success = status.success(),
        exit_code = status.code(),
        exit_signal = signal,
        response_len = outcome.response_text.len(),
        tool_calls = outcome.tool_calls.len(),
        "Stream finished"
    );

    let has_output = outcome.has_meaningful_output();

    if !has_output {
        let payload = if debug_lines.is_empty() {
            format!(
                "no stdout lines captured\n\nexit_code: {:?}\nexit_signal: {:?}\n\nstderr:\n{}",
                status.code(),
                signal,
                stderr_content.trim(),
            )
        } else {
            format!(
                "stdout sample:\n{}\n\nexit_code: {:?}\nexit_signal: {:?}\n\nstderr:\n{}",
                debug_lines.join("\n"),
                status.code(),
                signal,
                stderr_content.trim()
            )
        };
        let _ = std::fs::write(&debug_path, payload);
        info!(
            path = %debug_path.display(),
            conversation_id = %conversation_id_str,
            "Wrote stream debug log"
        );
    }

    if result.is_error {
        let error_msg = if !result.errors.is_empty() {
            result.errors.join("; ")
        } else {
            "Agent failed during execution".to_string()
        };
        // Check for recoverable provider errors before returning generic AgentExit
        if let Some(provider_err) =
            super::chat_service_errors::classify_provider_error(&error_msg)
        {
            return Err(provider_err);
        }
        // Also check stderr for provider error patterns
        if let Some(provider_err) =
            super::chat_service_errors::classify_provider_error(&stderr_content)
        {
            return Err(provider_err);
        }
        return Err(StreamError::AgentExit {
            exit_code: status.code(),
            stderr: error_msg,
        });
    }

    if !status.success() && !has_output {
        let stderr_trimmed = stderr_content.trim().to_string();
        // Check for recoverable provider errors in stderr
        if let Some(provider_err) =
            super::chat_service_errors::classify_provider_error(&stderr_trimmed)
        {
            return Err(provider_err);
        }
        return Err(StreamError::AgentExit {
            exit_code: status.code(),
            stderr: stderr_trimmed,
        });
    }

    Ok(outcome)
}

// ============================================================================
// Teammate auto-spawn helper
// ============================================================================

/// Spawn a separate CLI worker process for a teammate detected in the lead's stream.
///
/// Builds a `TeammateSpawnConfig` in print mode (`-p <prompt>`), spawns the process,
/// starts a background stream processor for its stdout, and registers the handle
/// in TeamService for lifecycle management.
async fn spawn_teammate_worker<R: Runtime>(
    teammate_name: &str,
    team_name: &str,
    parent_session_id: &str,
    prompt: &str,
    model: &str,
    color: &str,
    agent_type: &str,
    context_type: &str,
    context_id: &str,
    service: std::sync::Arc<crate::application::TeamService>,
    app_handle: AppHandle<R>,
) {
    tracing::info!(
        teammate = %teammate_name,
        team = %team_name,
        model = %model,
        agent_type = %agent_type,
        "Auto-spawning teammate worker process"
    );

    // Build spawn config with print mode (-p) so the teammate starts working immediately
    let spawn_config = TeammateSpawnConfig::new(
        teammate_name,
        team_name,
        parent_session_id,
        prompt, // Used as --append-system-prompt fallback (not used in print mode)
    )
    .with_model(model)
    .with_color(color)
    .with_agent_type(agent_type)
    .with_print_mode_prompt(prompt);

    let client = ClaudeCodeClient::new();
    let args = client.build_teammate_cli_args(&spawn_config);
    let env_vars = ClaudeCodeClient::build_teammate_env_vars(&spawn_config);

    // Build and spawn the command
    let mut cmd = tokio::process::Command::new(client.cli_path());
    cmd.args(&args)
        .current_dir(&spawn_config.working_directory)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .stdin(std::process::Stdio::null()); // Print mode: no stdin needed

    // Apply common RalphX spawn env vars
    apply_common_spawn_env(&mut cmd);

    // Plugin root env var
    if let Some(plugin_dir) = &spawn_config.plugin_dir {
        cmd.env("CLAUDE_PLUGIN_ROOT", plugin_dir);
    }

    // Team-specific env vars (CLAUDECODE=1, CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1, etc.)
    for (key, value) in &env_vars {
        cmd.env(key, value);
    }

    match cmd.spawn() {
        Ok(mut child) => {
            tracing::info!(
                teammate = %teammate_name,
                team = %team_name,
                pid = ?child.id(),
                "Teammate worker process spawned successfully"
            );

            // Take stdout for stream processing
            let stdout = child.stdout.take();

            // Start background stream processor for this teammate's output
            let stream_task = match stdout {
                Some(stdout) => {
                    Some(
                        crate::application::team_stream_processor::start_teammate_stream(
                            stdout,
                            team_name.to_string(),
                            teammate_name.to_string(),
                            context_type.to_string(),
                            context_id.to_string(),
                            app_handle.clone(),
                            service.tracker_arc(),
                            Some(service.clone()),
                        ),
                    )
                }
                None => {
                    tracing::warn!(
                        teammate = %teammate_name,
                        "No stdout pipe available for teammate stream processing"
                    );
                    None
                }
            };

            // Register the process handle for lifecycle management
            let handle = TeammateHandle {
                child,
                stream_task,
                stdin: None, // Print mode: no stdin pipe
            };

            if let Err(e) = service
                .set_teammate_handle(team_name, teammate_name, handle)
                .await
            {
                tracing::error!(
                    teammate = %teammate_name,
                    team = %team_name,
                    error = %e,
                    "Failed to register teammate handle"
                );
            }

            // Update status to Running
            let _ = service
                .update_teammate_status(team_name, teammate_name, TeammateStatus::Running)
                .await;
        }
        Err(e) => {
            tracing::error!(
                teammate = %teammate_name,
                team = %team_name,
                error = %e,
                "Failed to spawn teammate worker process"
            );

            // Update status to Failed
            let _ = service
                .update_teammate_status(team_name, teammate_name, TeammateStatus::Failed)
                .await;

            // Emit failure event
            let _ = app_handle.emit(
                "team:teammate_spawn_failed",
                serde_json::json!({
                    "team_name": team_name,
                    "teammate_name": teammate_name,
                    "error": e.to_string(),
                }),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_config_task_execution() {
        let config = StreamTimeoutConfig::for_context(&ChatContextType::TaskExecution);
        assert_eq!(config.line_read_timeout, Duration::from_secs(600));
        assert_eq!(config.parse_stall_timeout, Duration::from_secs(180));
    }

    #[test]
    fn test_timeout_config_review() {
        let config = StreamTimeoutConfig::for_context(&ChatContextType::Review);
        assert_eq!(config.line_read_timeout, Duration::from_secs(300));
        assert_eq!(config.parse_stall_timeout, Duration::from_secs(120));
    }

    #[test]
    fn test_timeout_config_merge() {
        let config = StreamTimeoutConfig::for_context(&ChatContextType::Merge);
        assert_eq!(config.line_read_timeout, Duration::from_secs(180));
        assert_eq!(config.parse_stall_timeout, Duration::from_secs(90));
    }

    #[test]
    fn test_timeout_config_ideation_uses_defaults() {
        let config = StreamTimeoutConfig::for_context(&ChatContextType::Ideation);
        assert_eq!(config.line_read_timeout, Duration::from_secs(600));
        assert_eq!(config.parse_stall_timeout, Duration::from_secs(180));
    }

    #[test]
    fn test_timeout_config_task_uses_defaults() {
        let config = StreamTimeoutConfig::for_context(&ChatContextType::Task);
        assert_eq!(config.line_read_timeout, Duration::from_secs(600));
        assert_eq!(config.parse_stall_timeout, Duration::from_secs(180));
    }

    #[test]
    fn test_timeout_config_project_uses_defaults() {
        let config = StreamTimeoutConfig::for_context(&ChatContextType::Project);
        assert_eq!(config.line_read_timeout, Duration::from_secs(600));
        assert_eq!(config.parse_stall_timeout, Duration::from_secs(180));
    }

    #[test]
    fn test_timeout_config_with_teammate() {
        let config = StreamTimeoutConfig::for_context(&ChatContextType::Ideation)
            .with_teammate("researcher".to_string(), "#ff6b35".to_string());
        assert_eq!(config.teammate_name, Some("researcher".to_string()));
        assert_eq!(config.teammate_color, Some("#ff6b35".to_string()));
        // Timeouts should be unchanged
        assert_eq!(config.line_read_timeout, Duration::from_secs(600));
    }

    #[test]
    fn test_timeout_config_default_no_teammate() {
        let config = StreamTimeoutConfig::for_context(&ChatContextType::Ideation);
        assert!(config.teammate_name.is_none());
        assert!(config.teammate_color.is_none());
    }

    #[test]
    fn test_merge_shorter_than_review_shorter_than_default() {
        let merge = StreamTimeoutConfig::for_context(&ChatContextType::Merge);
        let review = StreamTimeoutConfig::for_context(&ChatContextType::Review);
        let default = StreamTimeoutConfig::for_context(&ChatContextType::TaskExecution);

        assert!(merge.line_read_timeout < review.line_read_timeout);
        assert!(review.line_read_timeout < default.line_read_timeout);
        assert!(merge.parse_stall_timeout < review.parse_stall_timeout);
        assert!(review.parse_stall_timeout < default.parse_stall_timeout);
    }
}
