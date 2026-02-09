// Chat Service Streaming Logic
//
// Extracted from chat_service.rs to improve modularity and reduce file size.
// Handles background stream processing and event emission.

use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::info;

use crate::domain::entities::{
    ActivityEvent, ActivityEventType, ChatContextType, ChatConversationId, ChatMessageId, TaskId,
};
use crate::domain::repositories::{ActivityEventRepository, ChatMessageRepository, TaskRepository};
use crate::infrastructure::agents::claude::{ContentBlockItem, DiffContext, StreamEvent, StreamProcessor, ToolCall};

use super::{events, AgentChunkPayload, AgentTaskCompletedPayload, AgentTaskStartedPayload, AgentToolCallPayload};

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
) -> Result<(String, Vec<ToolCall>, Vec<ContentBlockItem>, Option<String>), String> {
    tracing::debug!(
        conversation_id = conversation_id.as_str(),
        %context_type,
        context_id,
        "process_stream_background start"
    );
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
    let debug_path = std::env::temp_dir()
        .join(format!("ralphx-stream-debug-{}.log", conversation_id_str));
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

    // Debounced flush for incremental persistence (every 2 seconds)
    let mut last_flush = std::time::Instant::now();
    const FLUSH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

    while let Ok(Some(line)) = lines.next_line().await {
        lines_seen += 1;
        if debug_lines.len() < 50 {
            debug_lines.push(line.clone());
        }
        if let Some(parsed) = StreamProcessor::parse_line(&line) {
            lines_parsed += 1;
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
                    StreamEvent::ToolCallStarted { name, id, parent_tool_use_id } => {
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
                    StreamEvent::ToolCallCompleted { mut tool_call, parent_tool_use_id } => {
                        // Capture old file content for Edit/Write tool calls
                        let name_lower = tool_call.name.to_lowercase();
                        if name_lower == "edit" || name_lower == "write" {
                            if let Some(file_path) = tool_call.arguments.get("file_path").and_then(|v| v.as_str()) {
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
                                if let Some(ContentBlockItem::ToolUse { diff_context, .. }) = processor.content_blocks.last_mut() {
                                    *diff_context = serde_json::to_value(&diff_ctx).ok();
                                }
                            }
                        }

                        let diff_context_value = tool_call.diff_context.as_ref()
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
                    StreamEvent::TaskStarted { tool_use_id, description, subagent_type, model } => {
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
                    StreamEvent::TaskCompleted { tool_use_id, agent_id, total_duration_ms, total_tokens, total_tool_use_count } => {
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
                    StreamEvent::ToolResultReceived { tool_use_id, result, parent_tool_use_id } => {
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
        }

        // Debounced flush: persist accumulated content every 2s for crash recovery
        if last_flush.elapsed() >= FLUSH_INTERVAL {
            if let (Some(ref repo), Some(ref msg_id)) = (&chat_message_repo, &assistant_message_id) {
                let current_text = processor.response_text.clone();
                let current_tools = serde_json::to_string(&processor.tool_calls).ok();
                let _ = repo.update_content(
                    &ChatMessageId::from_string(msg_id.clone()),
                    &current_text,
                    current_tools.as_deref(),
                    None, // content_blocks only on final update
                ).await;
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
    let status = child.wait().await.map_err(|e| e.to_string())?;
    tracing::debug!(
        conversation_id = %conversation_id_str,
        success = status.success(),
        response_len = result.response_text.len(),
        tool_calls = result.tool_calls.len(),
        "Stream finished"
    );

    if result.response_text.is_empty() {
        let payload = if debug_lines.is_empty() {
            format!(
                "no stdout lines captured\n\nstderr:\n{}",
                stderr_content.trim()
            )
        } else {
            format!(
                "stdout sample:\n{}\n\nstderr:\n{}",
                debug_lines.join("\n"),
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
        return Err(error_msg);
    }

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
