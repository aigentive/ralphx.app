// Chat Service Streaming Logic
//
// Extracted from chat_service.rs to improve modularity and reduce file size.
// Handles background stream processing and event emission.

use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::time::{timeout, Duration};
use tracing::info;

use crate::domain::entities::{
    ActivityEvent, ActivityEventType, ChatContextType, ChatConversationId, ChatMessageId, TaskId,
};
use crate::domain::repositories::{ActivityEventRepository, ChatMessageRepository, TaskRepository};
use crate::infrastructure::agents::claude::{
    ContentBlockItem, DiffContext, StreamEvent, StreamProcessor, ToolCall,
};
use crate::application::question_state::QuestionState;
use tokio_util::sync::CancellationToken;

use super::chat_service_errors::StreamError;
use super::{
    event_context, events, has_meaningful_output, AgentChunkPayload, AgentHookPayload,
    AgentTaskCompletedPayload, AgentTaskStartedPayload, AgentToolCallPayload,
};

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
}

impl StreamTimeoutConfig {
    /// Returns timeout thresholds appropriate for the given context type.
    pub fn for_context(context_type: &ChatContextType) -> Self {
        match context_type {
            ChatContextType::Merge => Self {
                line_read_timeout: Duration::from_secs(180),
                parse_stall_timeout: Duration::from_secs(90),
            },
            ChatContextType::Review => Self {
                line_read_timeout: Duration::from_secs(300),
                parse_stall_timeout: Duration::from_secs(120),
            },
            // TaskExecution, Ideation, Task, Project — generous defaults
            _ => Self {
                line_read_timeout: Duration::from_secs(600),
                parse_stall_timeout: Duration::from_secs(180),
            },
        }
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
) -> Result<StreamOutcome, StreamError> {
    let timeout_config = StreamTimeoutConfig::for_context(&context_type);
    tracing::debug!(
        conversation_id = conversation_id.as_str(),
        %context_type,
        context_id,
        line_read_timeout_secs = timeout_config.line_read_timeout.as_secs(),
        parse_stall_timeout_secs = timeout_config.parse_stall_timeout.as_secs(),
        "process_stream_background start"
    );
    let stdout = child.stdout.take().ok_or_else(|| StreamError::ProcessSpawnFailed {
        command: "claude".to_string(),
        error: "Failed to capture stdout".to_string(),
    })?;

    let stderr = child.stderr.take().ok_or_else(|| StreamError::ProcessSpawnFailed {
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
                    } => {
                        if let Some(ref handle) = app_handle {
                            let _ = handle.emit(
                                events::AGENT_TASK_STARTED,
                                AgentTaskStartedPayload {
                                    tool_use_id,
                                    description,
                                    subagent_type,
                                    model,
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
                        if let Some(ref handle) = app_handle {
                            let _ = handle.emit(
                                events::AGENT_TASK_COMPLETED,
                                AgentTaskCompletedPayload {
                                    tool_use_id,
                                    agent_id,
                                    total_duration_ms,
                                    total_tokens,
                                    total_tool_use_count,
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
        return Err(StreamError::AgentExit {
            exit_code: status.code(),
            stderr: error_msg,
        });
    }

    if !status.success() && !has_output {
        return Err(StreamError::AgentExit {
            exit_code: status.code(),
            stderr: stderr_content.trim().to_string(),
        });
    }

    Ok(outcome)
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
