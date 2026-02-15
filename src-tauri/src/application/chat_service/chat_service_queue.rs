// Message Queue Processing
//
// Handles queued messages that were sent while an agent was running.
// These messages are automatically processed via --resume after the initial run completes.

use std::path::Path;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};

use super::chat_service_context;
use super::chat_service_helpers::get_assistant_role;
use super::chat_service_streaming::process_stream_background;
use super::chat_service_types::{
    AgentErrorPayload, AgentMessageCreatedPayload, AgentQueueSentPayload, AgentRunStartedPayload,
};
use super::has_meaningful_output;
use crate::application::question_state::QuestionState;
use crate::domain::entities::{ChatContextType, ChatConversationId};
use crate::domain::repositories::{ActivityEventRepository, ChatMessageRepository, IdeationSessionRepository, TaskRepository};
use crate::domain::services::MessageQueue;
use tokio_util::sync::CancellationToken;

/// Process all queued messages for a context with retry loop.
///
/// Returns the total number of messages processed.
///
/// This handles race conditions where messages can be queued while we're processing,
/// so it keeps checking until the queue is stable-empty (50ms late-arrival check).
#[allow(clippy::too_many_arguments)]
pub(super) async fn process_queued_messages<R: Runtime + 'static>(
    context_type: ChatContextType,
    context_id: &str,
    conversation_id: ChatConversationId,
    session_id: &str,
    message_queue: &Arc<MessageQueue>,
    chat_message_repo: &Arc<dyn ChatMessageRepository>,
    chat_attachment_repo: &Arc<dyn crate::domain::repositories::ChatAttachmentRepository>,
    activity_event_repo: &Arc<dyn ActivityEventRepository>,
    task_repo: &Arc<dyn TaskRepository>,
    ideation_session_repo: &Arc<dyn IdeationSessionRepository>,
    cli_path: &Path,
    plugin_dir: &Path,
    working_directory: &Path,
    question_state: Option<Arc<QuestionState>>,
    app_handle: Option<AppHandle<R>>,
    project_id: Option<&str>,
    team_mode: bool,
    cancellation_token: CancellationToken,
    run_chain_id: Option<&str>,
    parent_run_id: Option<&str>,
) -> u32 {
    let mut total_processed = 0u32;

    // Outer loop: keep processing until queue is stable-empty
    loop {
        // Check cancellation before each iteration
        if cancellation_token.is_cancelled() {
            tracing::info!(
                "[QUEUE] Cancellation requested, stopping queue processing after {} messages",
                total_processed
            );
            break;
        }

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
            if cancellation_token.is_cancelled() {
                tracing::info!("[QUEUE] Cancellation requested mid-queue, stopping");
                break;
            }
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
            tracing::info!(
                queued_run_id = %queued_run_id,
                run_chain_id = run_chain_id.unwrap_or("none"),
                parent_run_id = parent_run_id.unwrap_or("none"),
                "[QUEUE] Continuation run"
            );
            if let Some(ref handle) = app_handle {
                let _ = handle.emit(
                    "agent:run_started",
                    AgentRunStartedPayload {
                        run_id: queued_run_id.clone(),
                        conversation_id: conversation_id.as_str().to_string(),
                        context_type: context_type.to_string(),
                        context_id: context_id.to_string(),
                        run_chain_id: run_chain_id.map(|s| s.to_string()),
                        parent_run_id: parent_run_id.map(|s| s.to_string()),
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

            // Link pending attachments to the user message
            if let Ok(pending_attachments) = chat_attachment_repo
                .find_by_conversation_id(&conversation_id)
                .await
            {
                let pending: Vec<_> = pending_attachments
                    .into_iter()
                    .filter(|a| a.message_id.is_none())
                    .collect();

                if !pending.is_empty() {
                    let attachment_ids: Vec<_> = pending.iter().map(|a| a.id.clone()).collect();
                    let _ = chat_attachment_repo
                        .update_message_ids(
                            &attachment_ids,
                            &crate::domain::entities::ChatMessageId::from_string(&user_msg_id),
                        )
                        .await;
                    tracing::debug!(
                        message_id = %user_msg_id,
                        attachment_count = pending.len(),
                        "[QUEUE] Linked attachments to user message"
                    );
                }
            }

            // Emit user message created
            if let Some(ref handle) = app_handle {
                let _ = handle.emit(
                    "agent:message_created",
                    AgentMessageCreatedPayload {
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
            let spawnable = match chat_service_context::build_resume_command(
                cli_path,
                plugin_dir,
                context_type,
                context_id,
                &queued_msg.content,
                working_directory,
                session_id,
                project_id,
                team_mode,
                Arc::clone(chat_attachment_repo),
                Arc::clone(ideation_session_repo),
                Arc::clone(task_repo),
            )
            .await
            {
                Ok(cmd) => cmd,
                Err(err) => {
                    tracing::warn!(
                        error = %err,
                        %context_type,
                        context_id,
                        "queue spawn blocked"
                    );
                    return total_processed;
                }
            };

            tracing::info!(cmd = ?spawnable, "Spawning CLI agent (queue resume)");
            match spawnable.spawn().await {
                Ok(child) => {
                    // Create empty assistant message before queue stream
                    let queue_assistant_msg = chat_service_context::create_assistant_message(
                        context_type,
                        context_id,
                        "",
                        conversation_id,
                        &[],
                        &[],
                    );
                    let queue_assistant_msg_id = queue_assistant_msg.id.as_str().to_string();
                    let _ = chat_message_repo.create(queue_assistant_msg).await;

                    match process_stream_background(
                        child,
                        context_type,
                        context_id,
                        &conversation_id,
                        app_handle.clone(),
                        Some(Arc::clone(activity_event_repo)),
                        Some(Arc::clone(task_repo)),
                        Some(Arc::clone(chat_message_repo)),
                        Some(queue_assistant_msg_id.clone()),
                        question_state.clone(),
                        cancellation_token.clone(),
                    )
                    .await
                    {
                        Ok(outcome) => {
                            let response = outcome.response_text;
                            let tools = outcome.tool_calls;
                            let blocks = outcome.content_blocks;
                            if has_meaningful_output(&response, tools.len()) {
                                let tool_calls_json = serde_json::to_string(&tools).ok();
                                let content_blocks_json = serde_json::to_string(&blocks).ok();
                                let _ = chat_message_repo
                                    .update_content(
                                        &crate::domain::entities::ChatMessageId::from_string(
                                            queue_assistant_msg_id.clone(),
                                        ),
                                        &response,
                                        tool_calls_json.as_deref(),
                                        content_blocks_json.as_deref(),
                                    )
                                    .await;

                                // Emit assistant message created
                                if let Some(ref handle) = app_handle {
                                    let _ = handle.emit(
                                        "agent:message_created",
                                        AgentMessageCreatedPayload {
                                            message_id: queue_assistant_msg_id,
                                            conversation_id: conversation_id.as_str().to_string(),
                                            context_type: context_type.to_string(),
                                            context_id: context_id.to_string(),
                                            role: get_assistant_role(&context_type).to_string(),
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
                            let error_string = e.to_string();
                            tracing::error!(
                                "Failed to process queued message stream: {}",
                                error_string
                            );
                            // Emit error event
                            if let Some(ref handle) = app_handle {
                                let _ = handle.emit(
                                    "agent:error",
                                    AgentErrorPayload {
                                        conversation_id: Some(conversation_id.as_str().to_string()),
                                        context_type: context_type.to_string(),
                                        context_id: context_id.to_string(),
                                        error: error_string.clone(),
                                        stderr: Some(error_string),
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
                            AgentErrorPayload {
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
