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
use crate::application::team_state_tracker::TeammateStatus;
use crate::infrastructure::agents::claude::stream_timeouts;
use crate::domain::entities::{
    ActivityEvent, ActivityEventType, ChatContextType, ChatConversationId, ChatMessageId, TaskId,
};
use crate::domain::repositories::{ActivityEventRepository, ChatMessageRepository, TaskRepository};
use crate::domain::services::{RunningAgentKey, RunningAgentRegistry};
use crate::infrastructure::agents::claude::{
    ContentBlockItem, DiffContext, StreamEvent, StreamProcessor, ToolCall,
};
use tokio_util::sync::CancellationToken;

use super::chat_service_errors::StreamError;
use super::streaming_state_cache::{CachedStreamingTask, CachedToolCall, StreamingStateCache};
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
        let cfg = stream_timeouts();
        match context_type {
            ChatContextType::Merge => Self {
                line_read_timeout: Duration::from_secs(cfg.merge_line_read_secs),
                parse_stall_timeout: Duration::from_secs(cfg.merge_parse_stall_secs),
                teammate_name: None,
                teammate_color: None,
            },
            ChatContextType::Review => Self {
                line_read_timeout: Duration::from_secs(cfg.review_line_read_secs),
                parse_stall_timeout: Duration::from_secs(cfg.review_parse_stall_secs),
                teammate_name: None,
                teammate_color: None,
            },
            // TaskExecution, Ideation, Task, Project — generous defaults
            _ => Self {
                line_read_timeout: Duration::from_secs(cfg.default_line_read_secs),
                parse_stall_timeout: Duration::from_secs(cfg.default_parse_stall_secs),
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
    pub stderr_text: String,
}

impl StreamOutcome {
    pub fn has_meaningful_output(&self) -> bool {
        has_meaningful_output(&self.response_text, self.tool_calls.len(), &self.stderr_text)
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
/// * `streaming_state_cache` - Cache for streaming state to hydrate frontend on navigation
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
    streaming_state_cache: StreamingStateCache,
    running_agent_registry: Option<Arc<dyn RunningAgentRegistry>>,
) -> Result<StreamOutcome, StreamError> {
    let mut timeout_config = StreamTimeoutConfig::for_context(&context_type);
    // Team leads wait long periods while teammates work — use team-specific timeout
    if team_mode {
        let cfg = stream_timeouts();
        timeout_config.line_read_timeout = Duration::from_secs(cfg.team_line_read_secs);
        timeout_config.parse_stall_timeout = Duration::from_secs(cfg.team_parse_stall_secs);
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

    // Parse task_id for activity persistence (for TaskExecution and Merge contexts).
    // Merge context uses the task_id as context_id, so the mapping is identical.
    let task_id_for_persistence =
        if matches!(context_type, ChatContextType::TaskExecution | ChatContextType::Merge) {
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
    let mut stream_seq: u64 = 0;
    let mut last_parsed_at = std::time::Instant::now();

    // Debounced flush for incremental persistence (every 2 seconds)
    let mut last_flush = std::time::Instant::now();
    const FLUSH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

    // Throttled heartbeat: update last_active_at every 5s on any parsed event
    let heartbeat_key = running_agent_registry.as_ref().map(|_| {
        RunningAgentKey::new(context_type.to_string(), context_id)
    });
    let mut last_heartbeat = std::time::Instant::now();
    const HEARTBEAT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);

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
                        // Update streaming state cache
                        streaming_state_cache.append_text(&conversation_id_str, &text).await;

                        if let Some(ref handle) = app_handle {
                            // Unified event
                            let _ = handle.emit(
                                events::AGENT_CHUNK,
                                AgentChunkPayload {
                                    text: text.clone(),
                                    conversation_id: conversation_id_str.clone(),
                                    context_type: context_type_str.clone(),
                                    context_id: context_id_str.clone(),
                                    seq: stream_seq,
                                },
                            );
                            stream_seq += 1;

                            // Activity stream event for task execution and merge
                            if matches!(context_type, ChatContextType::TaskExecution | ChatContextType::Merge) {
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
                        // Activity stream event for task execution and merge
                        if matches!(context_type, ChatContextType::TaskExecution | ChatContextType::Merge) {
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
                        // Update streaming state cache with started tool call
                        let cached_tool = CachedToolCall {
                            id: id.clone().unwrap_or_default(),
                            name: name.clone(),
                            arguments: serde_json::Value::Null,
                            result: None,
                            diff_context: None,
                            parent_tool_use_id: parent_tool_use_id.clone(),
                        };
                        streaming_state_cache.upsert_tool_call(&conversation_id_str, cached_tool).await;

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
                                    seq: stream_seq,
                                },
                            );
                            stream_seq += 1;
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

                        // Update streaming state cache with completed tool call
                        let cached_tool = CachedToolCall {
                            id: tool_call.id.clone().unwrap_or_default(),
                            name: tool_call.name.clone(),
                            arguments: tool_call.arguments.clone(),
                            result: None,
                            diff_context: diff_context_value.clone(),
                            parent_tool_use_id: parent_tool_use_id.clone(),
                        };
                        streaming_state_cache.upsert_tool_call(&conversation_id_str, cached_tool).await;

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
                                    seq: stream_seq,
                                },
                            );
                            stream_seq += 1;

                            // Activity stream event for task execution and merge
                            if matches!(context_type, ChatContextType::TaskExecution | ChatContextType::Merge) {
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

                        // Update streaming state cache with new task
                        let cached_task = CachedStreamingTask {
                            tool_use_id: tool_use_id.clone(),
                            description: description.clone(),
                            subagent_type: subagent_type.clone(),
                            model: model.clone(),
                            status: "running".to_string(),
                            teammate_name: tm_name.clone(),
                        };
                        streaming_state_cache.add_task(&conversation_id_str, cached_task).await;

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
                                    seq: stream_seq,
                                },
                            );
                            stream_seq += 1;
                        }
                    }
                    StreamEvent::TaskCompleted {
                        tool_use_id,
                        agent_id,
                        total_duration_ms,
                        total_tokens,
                        total_tool_use_count,
                    } => {
                        // Update streaming state cache - mark task as completed
                        streaming_state_cache.complete_task(&conversation_id_str, &tool_use_id).await;

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
                                    seq: stream_seq,
                                },
                            );
                            stream_seq += 1;
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
                    StreamEvent::TeammateSpawned { teammate_name, team_name, agent_id: _, model, color, prompt: _, agent_type: _ } => {
                        // Register teammate via TeamService (persistence + events).
                        // May already exist from approve_team_plan — add_teammate is idempotent.
                        // NOTE: CLI worker processes are spawned in approve_team_plan (teams.rs),
                        // NOT here, to avoid double-spawn when both paths fire.
                        if let Some(ref service) = team_service {
                            let _ = service.add_teammate(&team_name, &teammate_name, &color, &model, "team-member").await;
                        } else if let Some(ref handle) = app_handle {
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
                                    seq: stream_seq,
                                },
                            );
                            stream_seq += 1;

                            // Activity stream event for task execution and merge
                            if matches!(context_type, ChatContextType::TaskExecution | ChatContextType::Merge) {
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

        // Throttled heartbeat: write last_active_at every 5s on any parsed event
        if lines_parsed > 0 && last_heartbeat.elapsed() >= HEARTBEAT_INTERVAL {
            if let (Some(ref registry), Some(ref key)) = (&running_agent_registry, &heartbeat_key) {
                registry.update_heartbeat(key, chrono::Utc::now()).await;
            }
            last_heartbeat = std::time::Instant::now();
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
        stderr_text: stderr_content,
    };

    // Final flush of accumulated content so post-loop error returns don't lose data
    flush_content_before_error(
        &chat_message_repo, &assistant_message_id,
        &outcome.response_text, &outcome.tool_calls, &outcome.content_blocks,
    ).await;

    // Check if cancellation was requested during/after stream processing.
    // Fixes race where EOF from killed process wins the tokio::select! over
    // the cancellation token, causing the loop to break instead of returning
    // Err(Cancelled). If the token is cancelled, always return Cancelled.
    if cancellation_token.is_cancelled() {
        return Err(StreamError::Cancelled);
    }

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
                outcome.stderr_text.trim(),
            )
        } else {
            format!(
                "stdout sample:\n{}\n\nexit_code: {:?}\nexit_signal: {:?}\n\nstderr:\n{}",
                debug_lines.join("\n"),
                status.code(),
                signal,
                outcome.stderr_text.trim()
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
            super::chat_service_errors::classify_provider_error(&outcome.stderr_text)
        {
            return Err(provider_err);
        }
        return Err(StreamError::AgentExit {
            exit_code: status.code(),
            stderr: error_msg,
        });
    }

    if !status.success() && !has_output {
        let stderr_trimmed = outcome.stderr_text.trim().to_string();
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

#[cfg(test)]
#[path = "chat_service_streaming_tests.rs"]
mod tests;
