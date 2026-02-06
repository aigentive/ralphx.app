// Message Queue Processing
//
// Handles queued messages that were sent while an agent was running.
// These messages are automatically processed via --resume after the initial run completes.

use std::path::Path;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};

use crate::domain::entities::{ChatContextType, ChatConversationId};
use crate::domain::repositories::{ActivityEventRepository, ChatMessageRepository, TaskRepository};
use crate::domain::services::MessageQueue;
use crate::infrastructure::agents::claude::{add_prompt_args, build_base_cli_command, configure_spawn};

use super::chat_service_context;
use super::chat_service_helpers::get_agent_name;
use super::chat_service_streaming::process_stream_background;
use super::chat_service_types::{AgentQueueSentPayload, AgentRunStartedPayload};

/// Process all queued messages for a context with retry loop
///
/// Returns the total number of messages processed.
///
/// This handles race conditions where messages can be queued while we're processing,
/// so it keeps checking until the queue is stable-empty.
#[allow(dead_code)]
pub async fn process_message_queue<R: Runtime + 'static>(
    context_type: ChatContextType,
    context_id: &str,
    conversation_id: ChatConversationId,
    session_id: &str,
    message_queue: Arc<MessageQueue>,
    chat_message_repo: Arc<dyn ChatMessageRepository>,
    activity_event_repo: Arc<dyn ActivityEventRepository>,
    task_repo: Arc<dyn TaskRepository>,
    cli_path: &Path,
    plugin_dir: &Path,
    working_directory: &Path,
    app_handle: Option<AppHandle<R>>,
) -> u32 {
    let mut total_processed = 0u32;

    // Outer loop: keep processing until queue is stable-empty
    loop {
        let queue_count = message_queue.get_queued(context_type, context_id).len();

        if queue_count == 0 {
            // Queue is empty, wait briefly then check once more for race condition
            if total_processed > 0 {
                // We processed messages, give a small window for late arrivals
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                let final_count = message_queue.get_queued(context_type, context_id).len();
                if final_count == 0 {
                    tracing::info!(
                        "[QUEUE] Queue processing complete: {} total messages processed",
                        total_processed
                    );
                    break;
                }
                tracing::info!(
                    "[QUEUE] Found {} late-arriving messages, continuing...",
                    final_count
                );
            } else {
                tracing::info!("[QUEUE] No queued messages to process");
                break;
            }
        }

        tracing::info!(
            "[QUEUE] Processing queue: session_id={}, context={}/{}, pending={}",
            session_id,
            context_type,
            context_id,
            queue_count
        );

        // Inner loop: process all currently queued messages
        while let Some(queued_msg) = message_queue.pop(context_type, context_id) {
            total_processed += 1;
            tracing::info!(
                "[QUEUE] Processing queued message id={}, content_len={}",
                queued_msg.id,
                queued_msg.content.len()
            );

            // Emit queue sent event (removes from frontend optimistic UI)
            if let Some(ref handle) = app_handle {
                let _ = handle.emit(
                    "agent:queue_sent",
                    AgentQueueSentPayload {
                        message_id: queued_msg.id.clone(),
                        conversation_id: conversation_id.as_str().to_string(),
                        context_type: context_type.to_string(),
                        context_id: context_id.to_string(),
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
                        conversation_id: conversation_id.as_str().to_string(),
                        context_type: context_type.to_string(),
                        context_id: context_id.to_string(),
                    },
                );
            }

            // Persist user message
            let user_msg = chat_service_context::create_user_message(
                context_type,
                context_id,
                &queued_msg.content,
                conversation_id,
            );
            let user_msg_id = user_msg.id.as_str().to_string();
            let _ = chat_message_repo.create(user_msg).await;

            // Emit user message created
            if let Some(ref handle) = app_handle {
                let _ = handle.emit(
                    "agent:message_created",
                    super::chat_service_types::AgentMessageCreatedPayload {
                        message_id: user_msg_id,
                        conversation_id: conversation_id.as_str().to_string(),
                        context_type: context_type.to_string(),
                        context_id: context_id.to_string(),
                        role: "user".to_string(),
                        content: queued_msg.content.clone(),
                    },
                );
            }

            // Build and spawn resume command
            let agent_name = get_agent_name(&context_type);
            let mut cmd = match build_base_cli_command(cli_path, plugin_dir, Some(agent_name)) {
                Ok(cmd) => cmd,
                Err(err) => {
                    eprintln!(
                        "[STREAM_DEBUG] queue spawn blocked: {} (context_type={}, context_id={})",
                        err, context_type, context_id
                    );
                    return total_processed;
                }
            };
            cmd.env("RALPHX_AGENT_TYPE", agent_name);

            // Add task scope for task-related contexts
            match context_type {
                ChatContextType::Task | ChatContextType::TaskExecution => {
                    cmd.env("RALPHX_TASK_ID", context_id);
                }
                _ => {}
            }

            add_prompt_args(&mut cmd, &queued_msg.content, None, Some(session_id));
            configure_spawn(&mut cmd, working_directory);

            match cmd.spawn() {
                Ok(child) => {
                    eprintln!(
                        "[STREAM_DEBUG] queue spawn ok (context_type={}, context_id={}, conversation_id={})",
                        context_type,
                        context_id,
                        conversation_id.as_str()
                    );

                    // Create empty assistant message before queue stream
                    let queue_assistant_msg = chat_service_context::create_assistant_message(
                        context_type, context_id, "", conversation_id, &[], &[],
                    );
                    let queue_assistant_msg_id = queue_assistant_msg.id.as_str().to_string();
                    let _ = chat_message_repo.create(queue_assistant_msg).await;

                    match process_stream_background(
                        child,
                        context_type,
                        context_id,
                        &conversation_id,
                        app_handle.clone(),
                        Some(Arc::clone(&activity_event_repo)),
                        Some(Arc::clone(&task_repo)),
                        Some(Arc::clone(&chat_message_repo)),
                        Some(queue_assistant_msg_id.clone()),
                    )
                    .await
                    {
                        Ok((response, tools, blocks, _)) => {
                            if !response.is_empty() || !tools.is_empty() {
                                let tool_calls_json = serde_json::to_string(&tools).ok();
                                let content_blocks_json = serde_json::to_string(&blocks).ok();
                                let _ = chat_message_repo.update_content(
                                    &crate::domain::entities::ChatMessageId::from_string(queue_assistant_msg_id.clone()),
                                    &response,
                                    tool_calls_json.as_deref(),
                                    content_blocks_json.as_deref(),
                                ).await;

                                // Emit assistant message created
                                if let Some(ref handle) = app_handle {
                                    let _ = handle.emit(
                                        "agent:message_created",
                                        super::chat_service_types::AgentMessageCreatedPayload {
                                            message_id: queue_assistant_msg_id,
                                            conversation_id: conversation_id
                                                .as_str()
                                                .to_string(),
                                            context_type: context_type.to_string(),
                                            context_id: context_id.to_string(),
                                            role: super::chat_service_helpers::get_assistant_role(
                                                &context_type,
                                            )
                                            .to_string(),
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
                            tracing::error!("Failed to process queued message stream: {}", e);
                            // Emit error event
                            if let Some(ref handle) = app_handle {
                                let _ = handle.emit(
                                    "agent:error",
                                    super::chat_service_types::AgentErrorPayload {
                                        conversation_id: Some(
                                            conversation_id.as_str().to_string(),
                                        ),
                                        context_type: context_type.to_string(),
                                        context_id: context_id.to_string(),
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
                            super::chat_service_types::AgentErrorPayload {
                                conversation_id: Some(conversation_id.as_str().to_string()),
                                context_type: context_type.to_string(),
                                context_id: context_id.to_string(),
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

    total_processed
}
