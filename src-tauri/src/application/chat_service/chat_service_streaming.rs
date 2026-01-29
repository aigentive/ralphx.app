// Chat Service Streaming Logic
//
// Extracted from chat_service.rs to improve modularity and reduce file size.
// Handles background stream processing and event emission.

use tauri::{AppHandle, Emitter, Runtime};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::domain::entities::{ChatContextType, ChatConversationId};
use crate::infrastructure::agents::claude::{ContentBlockItem, StreamEvent, StreamProcessor, ToolCall};

use super::{events, AgentChunkPayload, AgentToolCallPayload};

// ============================================================================
// Background stream processing
// ============================================================================

/// Process stream output in background, emitting events
pub async fn process_stream_background<R: Runtime>(
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

    while let Ok(Some(line)) = lines.next_line().await {
        if let Some(msg) = StreamProcessor::parse_line(&line) {
            let stream_events = processor.process_message(msg);

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

                            // Legacy event - unified to chat:* for all context types
                            let _ = handle.emit(
                                events::CHAT_CHUNK,
                                serde_json::json!({
                                    "text": text,
                                    "conversation_id": conversation_id_str,
                                }),
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
                            }
                        }
                    }
                    StreamEvent::ToolCallStarted { name, id } => {
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
                                },
                            );

                            // Legacy event - unified to chat:* for all context types
                            let _ = handle.emit(
                                events::CHAT_TOOL_CALL,
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
                                events::AGENT_TOOL_CALL,
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

                            // Legacy event - unified to chat:* for all context types
                            let _ = handle.emit(
                                events::CHAT_TOOL_CALL,
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
                                    events::AGENT_MESSAGE,
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
                                events::AGENT_TOOL_CALL,
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

                            // Legacy event - unified to chat:* for all context types
                            let _ = handle.emit(
                                events::CHAT_TOOL_CALL,
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
