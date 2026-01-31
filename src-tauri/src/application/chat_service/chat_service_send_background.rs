// Background processing for send_message
//
// Extracted from chat_service/mod.rs to reduce file size.
// Handles stream processing, task transitions, queue processing, and event emissions.

use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::process::Child;

use crate::application::task_transition_service::TaskTransitionService;
use crate::application::task_scheduler_service::TaskSchedulerService;
use crate::domain::state_machine::services::TaskScheduler;
use crate::commands::ExecutionState;
use crate::domain::entities::{
    AgentRunId, ChatConversationId, ChatContextType, InternalStatus, TaskId,
};
use crate::domain::repositories::{
    ActivityEventRepository, AgentRunRepository, ChatConversationRepository, ChatMessageRepository,
    IdeationSessionRepository, ProjectRepository, TaskRepository,
};
use crate::domain::services::{MessageQueue, RunningAgentKey, RunningAgentRegistry};
use crate::infrastructure::agents::claude::{
    add_prompt_args, build_base_cli_command, configure_spawn,
};

use super::chat_service_context;
use super::chat_service_helpers::{get_agent_name, get_assistant_role};
use super::chat_service_streaming::process_stream_background;
use super::chat_service_types::{
    events, AgentErrorPayload, AgentMessageCreatedPayload, AgentQueueSentPayload,
    AgentRunCompletedPayload, AgentRunStartedPayload,
};

/// Spawn background task to process agent run, handle stream, transitions, and queue.
///
/// This function encapsulates the entire tokio::spawn background logic from send_message.
/// It processes the agent run stream, handles task state transitions (for TaskExecution),
/// and processes any queued messages using --resume.
///
/// # Arguments
/// - `child`: The spawned Claude CLI process
/// - `context_type`: The chat context type
/// - `context_id`: The context ID (task_id, project_id, etc.)
/// - `conversation_id`: The conversation ID
/// - `agent_run_id`: The agent run ID
/// - `stored_session_id`: Optional claude_session_id from conversation (for queue processing)
/// - `working_directory`: Working directory for spawned commands
/// - `cli_path`: Path to Claude CLI
/// - `plugin_dir`: Path to plugin directory
/// - `chat_message_repo`: Message repository
/// - `conversation_repo`: Conversation repository
/// - `agent_run_repo`: Agent run repository
/// - `task_repo`: Task repository
/// - `project_repo`: Project repository
/// - `ideation_session_repo`: Ideation session repository
/// - `activity_event_repo`: Activity event repository (for persistence)
/// - `message_queue`: Message queue
/// - `running_agent_registry`: Running agent registry
/// - `execution_state`: Execution state (for task transitions)
/// - `app_handle`: Tauri app handle (for events)
#[allow(clippy::too_many_arguments)]
pub fn spawn_send_message_background<R: Runtime>(
    child: Child,
    context_type: ChatContextType,
    context_id: String,
    conversation_id: ChatConversationId,
    agent_run_id: String,
    stored_session_id: Option<String>,
    working_directory: PathBuf,
    cli_path: PathBuf,
    plugin_dir: PathBuf,
    chat_message_repo: Arc<dyn ChatMessageRepository>,
    conversation_repo: Arc<dyn ChatConversationRepository>,
    agent_run_repo: Arc<dyn AgentRunRepository>,
    task_repo: Arc<dyn TaskRepository>,
    project_repo: Arc<dyn ProjectRepository>,
    ideation_session_repo: Arc<dyn IdeationSessionRepository>,
    activity_event_repo: Arc<dyn ActivityEventRepository>,
    message_queue: Arc<MessageQueue>,
    running_agent_registry: Arc<RunningAgentRegistry>,
    execution_state: Option<Arc<ExecutionState>>,
    app_handle: Option<AppHandle<R>>,
) {
    tokio::spawn(async move {
        // Create key for unregistering
        let registry_key = RunningAgentKey::new(context_type.to_string(), &context_id);

        let result = process_stream_background(
            child,
            context_type,
            &context_id,
            &conversation_id,
            app_handle.clone(),
            Some(Arc::clone(&activity_event_repo)),
            Some(Arc::clone(&task_repo)),
        )
        .await;

        // Unregister the process when done (whether success or failure)
        running_agent_registry.unregister(&registry_key).await;

        match result {
            Ok((response_text, tool_calls, content_blocks, claude_session_id)) => {
                // Debug: Log what we got from stream processing
                tracing::info!(
                    "[CHAT_SERVICE] Stream complete: context={}/{}, response_len={}, tool_calls={}, session_id={:?}",
                    context_type,
                    context_id,
                    response_text.len(),
                    tool_calls.len(),
                    claude_session_id
                );

                // Update conversation with claude_session_id
                if let Some(ref sess_id) = claude_session_id {
                    tracing::info!("[CHAT_SERVICE] Updating conversation with session_id={}", sess_id);
                    let _ = conversation_repo
                        .update_claude_session_id(&conversation_id, sess_id)
                        .await;
                } else {
                    tracing::warn!("[CHAT_SERVICE] No claude_session_id captured from stream - queue processing will be skipped!");
                }

                // Persist assistant message
                if !response_text.is_empty() || !tool_calls.is_empty() {
                    let assistant_msg = chat_service_context::create_assistant_message(
                        context_type,
                        &context_id,
                        &response_text,
                        conversation_id,
                        &tool_calls,
                        &content_blocks,
                    );
                    let assistant_msg_id = assistant_msg.id.as_str().to_string();
                    let _ = chat_message_repo.create(assistant_msg).await;

                    // Emit message created
                    if let Some(ref handle) = app_handle {
                        let _ = handle.emit(
                            "agent:message_created",
                            AgentMessageCreatedPayload {
                                message_id: assistant_msg_id.clone(),
                                conversation_id: conversation_id.as_str().to_string(),
                                context_type: context_type.to_string(),
                                context_id: context_id.clone(),
                                role: get_assistant_role(&context_type).to_string(),
                                content: response_text.clone(),
                            },
                        );

                        // Legacy event
                        let _ = handle.emit(
                            if context_type == ChatContextType::TaskExecution {
                                "execution:message_created"
                            } else {
                                "chat:message_created"
                            },
                            serde_json::json!({
                                "message_id": assistant_msg_id,
                                "conversation_id": conversation_id.as_str(),
                                "role": get_assistant_role(&context_type).to_string(),
                                "content": response_text,
                            }),
                        );
                    }
                }

                // Complete agent run
                let _ = agent_run_repo
                    .complete(&AgentRunId::from_string(&agent_run_id))
                    .await;

                // Handle task state transition (only for TaskExecution)
                // Use TaskTransitionService for proper entry/exit actions
                // Requires execution_state for proper running count tracking
                if context_type == ChatContextType::TaskExecution {
                    if let Some(ref exec_state) = execution_state {
                        let task_id = TaskId::from_string(context_id.clone());
                        if let Ok(Some(task)) = task_repo.get_by_id(&task_id).await {
                            // Handle both first execution (Executing) and re-execution (ReExecuting)
                            if task.internal_status == InternalStatus::Executing
                                || task.internal_status == InternalStatus::ReExecuting
                            {
                                // Create scheduler for auto-scheduling next Ready task
                                let task_scheduler: Arc<dyn TaskScheduler> = Arc::new(TaskSchedulerService::new(
                                    Arc::clone(exec_state),
                                    Arc::clone(&project_repo),
                                    Arc::clone(&task_repo),
                                    Arc::clone(&chat_message_repo),
                                    Arc::clone(&conversation_repo),
                                    Arc::clone(&agent_run_repo),
                                    Arc::clone(&ideation_session_repo),
                                    Arc::clone(&activity_event_repo),
                                    Arc::clone(&message_queue),
                                    Arc::clone(&running_agent_registry),
                                    app_handle.clone(),
                                ));

                                let transition_service = TaskTransitionService::new(
                                    Arc::clone(&task_repo),
                                    Arc::clone(&project_repo),
                                    Arc::clone(&chat_message_repo),
                                    Arc::clone(&conversation_repo),
                                    Arc::clone(&agent_run_repo),
                                    Arc::clone(&ideation_session_repo),
                                    Arc::clone(&activity_event_repo),
                                    Arc::clone(&message_queue),
                                    Arc::clone(&running_agent_registry),
                                    Arc::clone(exec_state),
                                    app_handle.clone(),
                                )
                                .with_task_scheduler(task_scheduler);
                                if let Err(e) = transition_service
                                    .transition_task(&task_id, InternalStatus::PendingReview)
                                    .await
                                {
                                    tracing::error!(
                                        "Failed to transition task {} to PendingReview: {}",
                                        task_id.as_str(),
                                        e
                                    );
                                }
                            }
                        }
                    } else {
                        tracing::warn!(
                            "Cannot transition task {} - no execution_state available",
                            context_id
                        );
                    }
                }

                // Check if there are queued messages to process
                // If yes, DON'T emit run_completed yet - emit it after queue processing
                // Use the stream's session_id if available, otherwise fall back to stored session_id
                let effective_session_id = claude_session_id.clone().or(stored_session_id.clone());
                let initial_queue_count = message_queue.get_queued(context_type, &context_id).len();
                let has_session_for_queue = effective_session_id.is_some();
                let will_process_queue = initial_queue_count > 0 && has_session_for_queue;

                if initial_queue_count > 0 && claude_session_id.is_none() && stored_session_id.is_some() {
                    tracing::info!(
                        "[QUEUE] Stream had no session_id, using stored session_id from conversation for queue processing"
                    );
                }

                // Only emit run_completed if there's no queue to process
                // If there IS a queue, we'll emit run_completed after all queue messages are processed
                if !will_process_queue {
                    if let Some(ref handle) = app_handle {
                        let _ = handle.emit(
                            "agent:run_completed",
                            AgentRunCompletedPayload {
                                conversation_id: conversation_id.as_str().to_string(),
                                context_type: context_type.to_string(),
                                context_id: context_id.clone(),
                                claude_session_id: effective_session_id.clone(),
                            },
                        );

                        // Legacy event - unified to chat:* for all context types
                        let _ = handle.emit(
                            events::CHAT_RUN_COMPLETED,
                            serde_json::json!({
                                "conversation_id": conversation_id.as_str(),
                                "claude_session_id": effective_session_id,
                            }),
                        );
                    }
                } else {
                    tracing::info!(
                        "[QUEUE] Deferring run_completed: {} queued messages to process first",
                        initial_queue_count
                    );
                }

                // Process queued messages with retry loop to handle race conditions
                // Messages can be queued while we're processing, so we keep checking until empty
                if let Some(ref sess_id) = effective_session_id {
                    let mut total_processed = 0u32;

                    // Outer loop: keep processing until queue is stable-empty
                    loop {
                        let queue_count = message_queue.get_queued(context_type, &context_id).len();

                        if queue_count == 0 {
                            // Queue is empty, wait briefly then check once more for race condition
                            if total_processed > 0 {
                                // We processed messages, give a small window for late arrivals
                                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                                let final_count = message_queue.get_queued(context_type, &context_id).len();
                                if final_count == 0 {
                                    tracing::info!("[QUEUE] Queue processing complete: {} total messages processed", total_processed);
                                    break;
                                }
                                tracing::info!("[QUEUE] Found {} late-arriving messages, continuing...", final_count);
                            } else {
                                tracing::info!("[QUEUE] No queued messages to process");
                                break;
                            }
                        }

                        tracing::info!(
                            "[QUEUE] Processing queue: session_id={}, context={}/{}, pending={}",
                            sess_id, context_type, context_id, queue_count
                        );

                        // Inner loop: process all currently queued messages
                        while let Some(queued_msg) =
                            message_queue.pop(context_type, &context_id)
                        {
                            total_processed += 1;
                        tracing::info!("[QUEUE] Processing queued message id={}, content_len={}", queued_msg.id, queued_msg.content.len());

                        // Emit queue sent event (removes from frontend optimistic UI)
                        if let Some(ref handle) = app_handle {
                            let _ = handle.emit(
                                "agent:queue_sent",
                                AgentQueueSentPayload {
                                    message_id: queued_msg.id.clone(),
                                    conversation_id: conversation_id.as_str().to_string(),
                                    context_type: context_type.to_string(),
                                    context_id: context_id.clone(),
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
                                    context_id: context_id.clone(),
                                },
                            );
                        }

                        // Persist user message
                        let user_msg = chat_service_context::create_user_message(
                            context_type,
                            &context_id,
                            &queued_msg.content,
                            conversation_id,
                        );
                        let user_msg_id = user_msg.id.as_str().to_string();
                        let _ = chat_message_repo.create(user_msg).await;

                        // Emit user message created
                        if let Some(ref handle) = app_handle {
                            let _ = handle.emit(
                                "agent:message_created",
                                AgentMessageCreatedPayload {
                                    message_id: user_msg_id,
                                    conversation_id: conversation_id.as_str().to_string(),
                                    context_type: context_type.to_string(),
                                    context_id: context_id.clone(),
                                    role: "user".to_string(),
                                    content: queued_msg.content.clone(),
                                },
                            );
                        }

                        // Build and spawn resume command
                        let agent_name = get_agent_name(&context_type);
                        let mut cmd = build_base_cli_command(cli_path.as_path(), plugin_dir.as_path(), Some(agent_name));
                        cmd.env("RALPHX_AGENT_TYPE", agent_name);

                        // Add task scope for task-related contexts
                        match context_type {
                            ChatContextType::Task | ChatContextType::TaskExecution => {
                                cmd.env("RALPHX_TASK_ID", &context_id);
                            }
                            _ => {}
                        }

                        add_prompt_args(&mut cmd, &queued_msg.content, None, Some(sess_id));
                        configure_spawn(&mut cmd, &working_directory);

                        match cmd.spawn() {
                            Ok(child) => {
                                match process_stream_background(
                                    child,
                                    context_type,
                                    &context_id,
                                    &conversation_id,
                                    app_handle.clone(),
                                    Some(Arc::clone(&activity_event_repo)),
                                    Some(Arc::clone(&task_repo)),
                                )
                                .await
                                {
                                    Ok((response, tools, blocks, _)) => {
                                        if !response.is_empty() || !tools.is_empty() {
                                            let assistant_msg =
                                                chat_service_context::create_assistant_message(
                                                    context_type,
                                                    &context_id,
                                                    &response,
                                                    conversation_id,
                                                    &tools,
                                                    &blocks,
                                                );
                                            let assistant_msg_id = assistant_msg.id.as_str().to_string();
                                            let _ = chat_message_repo.create(assistant_msg).await;

                                            // Emit assistant message created
                                            if let Some(ref handle) = app_handle {
                                                let _ = handle.emit(
                                                    "agent:message_created",
                                                    AgentMessageCreatedPayload {
                                                        message_id: assistant_msg_id,
                                                        conversation_id: conversation_id.as_str().to_string(),
                                                        context_type: context_type.to_string(),
                                                        context_id: context_id.clone(),
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
                                        tracing::error!(
                                            "Failed to process queued message stream: {}",
                                            e
                                        );
                                        // Emit error event
                                        if let Some(ref handle) = app_handle {
                                            let _ = handle.emit(
                                                "agent:error",
                                                AgentErrorPayload {
                                                    conversation_id: Some(conversation_id.as_str().to_string()),
                                                    context_type: context_type.to_string(),
                                                    context_id: context_id.clone(),
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
                                        AgentErrorPayload {
                                            conversation_id: Some(conversation_id.as_str().to_string()),
                                            context_type: context_type.to_string(),
                                            context_id: context_id.clone(),
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

                    // After ALL queue processing is done, emit the final run_completed
                    // This prevents UI flickering between individual queued messages
                    if total_processed > 0 {
                        tracing::info!("[QUEUE] Emitting final run_completed after processing {} queued messages", total_processed);
                        if let Some(ref handle) = app_handle {
                            let _ = handle.emit(
                                "agent:run_completed",
                                AgentRunCompletedPayload {
                                    conversation_id: conversation_id.as_str().to_string(),
                                    context_type: context_type.to_string(),
                                    context_id: context_id.clone(),
                                    claude_session_id: Some(sess_id.clone()),
                                },
                            );

                            // Legacy event - unified to chat:* for all context types
                            let _ = handle.emit(
                                events::CHAT_RUN_COMPLETED,
                                serde_json::json!({
                                    "conversation_id": conversation_id.as_str(),
                                    "claude_session_id": sess_id,
                                }),
                            );
                        }
                    }
                } else {
                    // effective_session_id is None - no session ID from stream OR stored conversation
                    let queue_count = message_queue.get_queued(context_type, &context_id).len();
                    if queue_count > 0 {
                        tracing::warn!(
                            "[QUEUE] SKIPPING {} queued messages because no session_id available (neither from stream nor stored)!",
                            queue_count
                        );
                    }
                }
            }
            Err(e) => {
                // Fail the agent run
                let _ = agent_run_repo
                    .fail(&AgentRunId::from_string(&agent_run_id), &e)
                    .await;

                // Emit error event
                if let Some(ref handle) = app_handle {
                    let _ = handle.emit(
                        "agent:error",
                        AgentErrorPayload {
                            conversation_id: Some(conversation_id.as_str().to_string()),
                            context_type: context_type.to_string(),
                            context_id: context_id.clone(),
                            error: e.clone(),
                            stderr: Some(e.clone()),
                        },
                    );

                    // Legacy event
                    let _ = handle.emit(
                        if context_type == ChatContextType::TaskExecution {
                            "execution:error"
                        } else {
                            "chat:error"
                        },
                        serde_json::json!({
                            "conversation_id": conversation_id.as_str(),
                            "error": e,
                        }),
                    );
                }
            }
        }
    });
}
