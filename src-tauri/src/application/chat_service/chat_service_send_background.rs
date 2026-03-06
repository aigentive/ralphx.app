// Background processing for send_message
//
// Extracted from chat_service/mod.rs to reduce file size.
// Handles stream processing, task transitions, queue processing, and event emissions.

use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::process::Child;
use tracing::Instrument;

use super::chat_service_context;
use super::chat_service_helpers::get_assistant_role;
use super::chat_service_streaming::process_stream_background;
use super::chat_service_types::{AgentMessageCreatedPayload, AgentRunCompletedPayload};
use super::{event_context, has_meaningful_output, EventContextPayload, StreamingStateCache};
use crate::application::interactive_process_registry::{
    InteractiveProcessKey, InteractiveProcessRegistry,
};
use crate::application::memory_orchestration::trigger_memory_pipelines;
use crate::application::question_state::QuestionState;
use crate::commands::ExecutionState;
use crate::domain::entities::ChatConversation;
use crate::domain::entities::{AgentRunId, ChatContextType, ChatConversationId};
use crate::domain::repositories::{
    ActivityEventRepository, AgentRunRepository, ChatAttachmentRepository,
    ChatConversationRepository, ChatMessageRepository, IdeationSessionRepository,
    MemoryEventRepository, PlanBranchRepository, ProjectRepository, ReviewRepository,
    TaskDependencyRepository, TaskProposalRepository, TaskRepository, TaskStepRepository,
};
use crate::domain::services::{MessageQueue, RunningAgentKey, RunningAgentRegistry};
use tokio_util::sync::CancellationToken;

/// All repository and service dependencies grouped together.
pub(super) struct BackgroundRunRepos {
    pub chat_message_repo: Arc<dyn ChatMessageRepository>,
    pub chat_attachment_repo: Arc<dyn ChatAttachmentRepository>,
    pub conversation_repo: Arc<dyn ChatConversationRepository>,
    pub agent_run_repo: Arc<dyn AgentRunRepository>,
    pub task_repo: Arc<dyn TaskRepository>,
    pub task_dependency_repo: Arc<dyn TaskDependencyRepository>,
    pub project_repo: Arc<dyn ProjectRepository>,
    pub ideation_session_repo: Arc<dyn IdeationSessionRepository>,
    pub task_proposal_repo: Option<Arc<dyn TaskProposalRepository>>,
    pub activity_event_repo: Arc<dyn ActivityEventRepository>,
    pub memory_event_repo: Arc<dyn MemoryEventRepository>,
    pub message_queue: Arc<MessageQueue>,
    pub running_agent_registry: Arc<dyn RunningAgentRegistry>,
    pub task_step_repo: Option<Arc<dyn TaskStepRepository>>,
    pub review_repo: Option<Arc<dyn ReviewRepository>>,
}

/// Full context for a background agent run, replacing 29 individual parameters.
pub(super) struct BackgroundRunContext<R: Runtime> {
    // Process
    pub child: Child,
    // Context identification
    pub context_type: ChatContextType,
    pub context_id: String,
    pub conversation_id: ChatConversationId,
    pub agent_run_id: String,
    pub stored_session_id: Option<String>,
    // Paths
    pub working_directory: PathBuf,
    pub cli_path: PathBuf,
    pub plugin_dir: PathBuf,
    // Repositories and services
    pub repos: BackgroundRunRepos,
    // State
    pub execution_state: Option<Arc<ExecutionState>>,
    pub question_state: Option<Arc<QuestionState>>,
    pub plan_branch_repo: Option<Arc<dyn PlanBranchRepository>>,
    // Tauri handle
    pub app_handle: Option<AppHandle<R>>,
    // Run chain correlation
    pub run_chain_id: Option<String>,
    // Run metadata
    pub is_retry_attempt: bool,
    pub user_message_content: Option<String>,
    pub conversation: Option<ChatConversation>,
    pub agent_name: Option<String>,
    pub team_mode: bool,
    // Cancellation
    pub cancellation_token: CancellationToken,
    // Team state
    pub team_service: Option<std::sync::Arc<crate::application::TeamService>>,
    // Streaming state cache for frontend hydration
    pub streaming_state_cache: StreamingStateCache,
    // Interactive process registry for stdin cleanup on process exit
    pub interactive_process_registry: Option<Arc<InteractiveProcessRegistry>>,
}

/// Returns true when `--resume` was used (stored is Some) AND the stream returned a different
/// session ID (new_id is Some and differs from stored). False in all other cases.
fn session_changed_after_resume(stored: Option<&str>, new_id: Option<&str>) -> bool {
    match (stored, new_id) {
        (Some(s), Some(n)) => s != n,
        _ => false,
    }
}

pub(super) async fn finalize_assistant_message<R: Runtime>(
    chat_message_repo: &Arc<dyn ChatMessageRepository>,
    app_handle: Option<&AppHandle<R>>,
    event_ctx: &EventContextPayload,
    message_id: &str,
    role: &str,
    content: &str,
    tool_calls_json: Option<&str>,
    content_blocks_json: Option<&str>,
) {
    let _ = chat_message_repo
        .update_content(
            &crate::domain::entities::ChatMessageId::from_string(message_id.to_string()),
            content,
            tool_calls_json,
            content_blocks_json,
        )
        .await;

    if let Some(handle) = app_handle {
        let _ = handle.emit(
            "agent:message_created",
            AgentMessageCreatedPayload {
                message_id: message_id.to_string(),
                conversation_id: event_ctx.conversation_id.clone(),
                context_type: event_ctx.context_type.clone(),
                context_id: event_ctx.context_id.clone(),
                role: role.to_string(),
                content: content.to_string(),
                created_at: None,
            },
        );
    }
}

/// Spawn background task to process agent run, handle stream, transitions, and queue.
///
/// This function encapsulates the entire tokio::spawn background logic from send_message.
/// It processes the agent run stream, handles task state transitions (for TaskExecution),
/// and processes any queued messages using --resume.
pub fn spawn_send_message_background<R: Runtime>(ctx: BackgroundRunContext<R>) {
    let span = tracing::info_span!(
        "agent_run",
        agent_run_id = %ctx.agent_run_id,
        run_chain_id = ctx.run_chain_id.as_deref().unwrap_or("none"),
        %ctx.context_type,
        context_id = %ctx.context_id,
        conversation_id = ctx.conversation_id.as_str(),
    );

    tokio::spawn(async move {
        let BackgroundRunContext {
            child,
            context_type,
            context_id,
            conversation_id,
            agent_run_id,
            stored_session_id,
            working_directory,
            cli_path,
            plugin_dir,
            repos,
            execution_state,
            question_state,
            plan_branch_repo,
            app_handle,
            run_chain_id,
            is_retry_attempt,
            user_message_content,
            conversation,
            agent_name,
            team_mode,
            cancellation_token,
            team_service,
            streaming_state_cache,
            interactive_process_registry,
        } = ctx;
        let BackgroundRunRepos {
            chat_message_repo,
            chat_attachment_repo,
            conversation_repo,
            agent_run_repo,
            task_repo,
            task_dependency_repo,
            project_repo,
            ideation_session_repo,
            task_proposal_repo,
            activity_event_repo,
            memory_event_repo,
            message_queue,
            running_agent_registry,
            task_step_repo,
            review_repo,
        } = repos;

        tracing::debug!("send_background start");
        let event_ctx = event_context(&conversation_id, &context_type, &context_id);

        // Pre-spawn cleanup: disband any stale teams for this context before the new run.
        // Handles mode-switch (team → solo) and crash-recovery re-execution scenarios.
        if let Some(ref service) = team_service {
            service.cleanup_stale_teams_for_context(&context_id).await;
        }

        // Resolve project ID for RALPHX_PROJECT_ID env var (used in queue processing)
        let resolved_project_id = chat_service_context::resolve_project_id(
            context_type,
            &context_id,
            Arc::clone(&task_repo),
            Arc::clone(&ideation_session_repo),
        )
        .await;
        let resolved_project_id_typed = resolved_project_id.as_ref().map(|s| crate::domain::entities::ProjectId::from_string(s.clone()));

        // Create key for unregistering
        let registry_key = RunningAgentKey::new(context_type.to_string(), &context_id);

        // Create empty assistant message BEFORE streaming starts (crash recovery)
        let pre_assistant_msg = chat_service_context::create_assistant_message(
            context_type, &context_id, "", conversation_id, &[], &[],
        );
        let pre_assistant_msg_id = pre_assistant_msg.id.as_str().to_string();
        let _ = chat_message_repo.create(pre_assistant_msg).await;

        tracing::debug!(
            conversation_id = conversation_id.as_str(),
            "send_background calling process_stream_background"
        );
        let result = process_stream_background(
            child,
            context_type,
            &context_id,
            &conversation_id,
            app_handle.clone(),
            Some(Arc::clone(&activity_event_repo)),
            Some(Arc::clone(&task_repo)),
            Some(Arc::clone(&chat_message_repo)),
            Some(pre_assistant_msg_id.clone()),
            question_state.clone(),
            cancellation_token.clone(),
            team_service.clone(),
            team_mode,
            streaming_state_cache.clone(),
            Some(Arc::clone(&running_agent_registry)),
            Some(Arc::clone(&agent_run_repo)),
            Some(agent_run_id.clone()),
            execution_state.clone(),
            Some(Arc::clone(&conversation_repo)),
        )
        .await;

        // Clean up team state when lead stream ends (success, error, or timeout)
        if team_mode {
            if let Some(ref service) = team_service {
                let teams = service.list_teams().await;
                for tn in &teams {
                    if let Ok(status) = service.get_team_status(tn).await {
                        if status.context_id == context_id {
                            // Disband the team via TeamService (stops teammates + persists + emits events)
                            let _ = service.disband_team(tn).await;
                        }
                    }
                }
            }
        }

        // Unregister the process when done (ownership check: only removes our own slot)
        running_agent_registry.unregister(&registry_key, &agent_run_id).await;

        // Remove interactive stdin handle so future messages trigger a new spawn.
        // EXCEPTION: if a team is still active for this context, keep the IPR entry
        // so teammate→lead nudges can still attempt delivery. The entry will be
        // cleaned up when the team is disbanded or the next send_message detects
        // a broken pipe and removes it.
        if let Some(ref ipr) = interactive_process_registry {
            let ipr_key = InteractiveProcessKey::new(
                context_type.to_string(),
                &context_id,
            );

            let team_still_active = if team_mode {
                if let Some(ref service) = team_service {
                    let teams = service.list_teams().await;
                    let mut found = false;
                    for tn in &teams {
                        if let Ok(status) = service.get_team_status(tn).await {
                            if status.context_id == context_id {
                                found = true;
                                break;
                            }
                        }
                    }
                    found
                } else {
                    false
                }
            } else {
                false
            };

            if team_still_active {
                tracing::warn!(
                    %context_type,
                    context_id = %context_id,
                    "[IPR_KEEP] Keeping interactive process stdin — team still active for context"
                );
            } else {
                ipr.remove(&ipr_key).await;
                tracing::info!(
                    %context_type,
                    context_id = %context_id,
                    "[IPR_REMOVE] Removed interactive process stdin on stream exit"
                );
            }
        }

        // Clean up interactive idle slot tracking
        if let Some(ref exec) = execution_state {
            let slot_key = format!("{}/{}", context_type, context_id);
            exec.remove_interactive_slot(&slot_key);
        }

        match result {
            Ok(outcome) => {
                let execution_slot_held = outcome.execution_slot_held;
                let response_text = outcome.response_text;
                let tool_calls = outcome.tool_calls;
                let content_blocks = outcome.content_blocks;
                let claude_session_id = outcome.session_id;
                let stderr_text = outcome.stderr_text;
                let turns_finalized = outcome.turns_finalized;
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
                    if let Err(e) = conversation_repo
                        .update_claude_session_id(&conversation_id, sess_id)
                        .await
                    {
                        tracing::error!(
                            error = %e,
                            conversation_id = conversation_id.as_str(),
                            session_id = %sess_id,
                            "[CHAT_SERVICE] Failed to persist claude_session_id — next resume attempt will use stale session ID"
                        );
                    }
                } else {
                    tracing::warn!("[CHAT_SERVICE] No claude_session_id captured from stream - queue processing will be skipped!");
                }

                // Detect resume failure: if --resume was used but Claude returned a different session ID,
                // it silently started a fresh session (original session likely expired).
                // Instead of just logging, trigger recovery: rebuild conversation history and
                // enqueue it as a priority message so Claude gets context before any pending user messages.
                if session_changed_after_resume(
                    stored_session_id.as_deref(),
                    claude_session_id.as_deref(),
                ) {
                    tracing::warn!(
                        stored_session_id = %stored_session_id.as_deref().unwrap_or(""),
                        new_session_id = %claude_session_id.as_deref().unwrap_or(""),
                        context_type = %context_type,
                        context_id = %context_id,
                        "[RESUME] Session ID changed after --resume — triggering context recovery"
                    );

                    // Build conversation replay to inject history into the new session
                    let replay_builder = super::chat_service_replay::ReplayBuilder::new(100_000);
                    match replay_builder.build_replay(&chat_message_repo, &conversation_id).await {
                        Ok(replay) if !replay.turns.is_empty() => {
                            let rehydration_prompt = super::chat_service_replay::build_rehydration_prompt(
                                &replay,
                                context_type,
                                &context_id,
                                "[System] Your session was silently restarted. The conversation history above has been restored. Briefly confirm you have this context, then wait for the next user message.",
                                None,
                            );

                            // Enqueue at front so history is sent before any pending user messages
                            message_queue.queue_front(
                                context_type,
                                &context_id,
                                rehydration_prompt,
                            );

                            tracing::info!(
                                replay_turns = replay.turns.len(),
                                estimated_tokens = replay.total_tokens,
                                "[RESUME] Enqueued conversation history replay for silent session swap recovery"
                            );
                        }
                        Ok(replay) => {
                            tracing::info!(
                                turns = replay.turns.len(),
                                "[RESUME] No conversation turns to replay, skipping history injection"
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                error = %e,
                                "[RESUME] Failed to build conversation replay for session swap recovery"
                            );
                        }
                    }

                    // Emit event to frontend so UI can show recovery banner
                    if let Some(ref handle) = app_handle {
                        let _ = handle.emit(
                            "agent:session_recovered",
                            serde_json::json!({
                                "conversation_id": conversation_id.as_str(),
                                "context_type": context_type.to_string(),
                                "context_id": context_id,
                                "message": "Session silently restarted — conversation history restored"
                            }),
                        );
                    }
                }

                // Update pre-created assistant message with final content.
                // When turns were finalized during interactive streaming, the original
                // pre_assistant_msg was already finalized in the TurnComplete handler.
                // The processor was reset, so response_text is empty. Skip overwriting.
                let has_output = has_meaningful_output(&response_text, tool_calls.len(), &stderr_text);
                let skip_post_loop_finalization = turns_finalized > 0 && !has_output;

                let assistant_role = get_assistant_role(&context_type).to_string();
                if skip_post_loop_finalization {
                    tracing::debug!(
                        turns_finalized,
                        "Skipping post-loop finalization — {} turn(s) already finalized in stream loop",
                        turns_finalized,
                    );
                } else if has_output {
                    let tool_calls_json = serde_json::to_string(&tool_calls).ok();
                    let content_blocks_json = serde_json::to_string(&content_blocks).ok();
                    finalize_assistant_message(
                        &chat_message_repo,
                        app_handle.as_ref(),
                        &event_ctx,
                        &pre_assistant_msg_id,
                        &assistant_role,
                        &response_text,
                        tool_calls_json.as_deref(),
                        content_blocks_json.as_deref(),
                    )
                    .await;
                } else {
                    // Stream completed with no content — update pre-created message so UI
                    // doesn't show "..." forever
                    let note = "[Agent completed with no output]";
                    finalize_assistant_message(
                        &chat_message_repo,
                        app_handle.as_ref(),
                        &event_ctx,
                        &pre_assistant_msg_id,
                        &assistant_role,
                        note,
                        None,
                        None,
                    )
                    .await;
                }

                // Treat zero-output runs as failed executions for autonomous task/review flows.
                // Note: when interactive turns were finalized, has_output is false (processor was reset)
                // but the run actually succeeded — override the flag for the run status check.
                let effective_has_output = has_output || turns_finalized > 0;
                // When turns were finalized in the stream loop, agent_run was already
                // completed in the TurnComplete handler — skip duplicate completion.
                if !skip_post_loop_finalization {
                    if !effective_has_output
                        && (context_type == ChatContextType::TaskExecution
                            || context_type == ChatContextType::Review)
                    {
                        let _ = agent_run_repo
                            .fail(
                                &AgentRunId::from_string(&agent_run_id),
                                "Agent completed with no output",
                            )
                            .await;
                    } else {
                        let _ = agent_run_repo
                            .complete(&AgentRunId::from_string(&agent_run_id))
                            .await;
                    }
                }

                // When TurnComplete freed the execution slot and the process exited
                // while idle, re-increment temporarily so that the state transition's
                // on_exit decrement produces the correct final count (net zero).
                if !execution_slot_held
                    && super::uses_execution_slot(context_type)
                    && !(outcome.silent_interactive_exit && context_type == ChatContextType::Ideation)
                {
                    if let Some(ref exec) = execution_state {
                        exec.increment_running();
                        tracing::debug!(
                            %context_type,
                            context_id = %context_id,
                            "Re-incremented before state transition to prevent double-decrement"
                        );
                    }
                }

                // Handle task state transitions and merge auto-completion
                super::chat_service_handlers::handle_stream_success(
                    context_type,
                    &context_id,
                    effective_has_output,
                    &execution_state,
                    &task_repo,
                    &task_dependency_repo,
                    &project_repo,
                    &chat_message_repo,
                    &chat_attachment_repo,
                    &conversation_repo,
                    &agent_run_repo,
                    &ideation_session_repo,
                    &activity_event_repo,
                    &message_queue,
                    &running_agent_registry,
                    &memory_event_repo,
                    &plan_branch_repo,
                    &task_step_repo,
                    &app_handle,
                    &interactive_process_registry,
                    &review_repo,
                )
                .await;

                // Staleness guard (defense-in-depth): drop stale queued messages before
                // processing on ANY process exit. Catches OOM/SIGKILL scenarios where
                // silent_interactive_exit flag cannot be set.
                let staleness_threshold_secs: u64 = std::env::var("QUEUE_STALENESS_THRESHOLD_SECS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(300);
                let stale_dropped =
                    message_queue.remove_stale(context_type, &context_id, staleness_threshold_secs);
                for msg in &stale_dropped {
                    tracing::warn!(
                        "[QUEUE] Dropped stale queued message (age > {}s) id={} for context {}:{}",
                        staleness_threshold_secs,
                        msg.id,
                        context_type,
                        context_id,
                    );
                }

                // Check if there are queued messages to process
                // If yes, DON'T emit run_completed yet - emit it after queue processing
                // Use the stream's session_id if available, otherwise fall back to stored session_id
                let effective_session_id = claude_session_id.clone().or(stored_session_id.clone());
                let initial_queue_count = message_queue.get_queued(context_type, &context_id).len();
                let has_session_for_queue = effective_session_id.is_some();
                let will_process_queue = initial_queue_count > 0 && has_session_for_queue && !outcome.silent_interactive_exit;

                if initial_queue_count > 0 && claude_session_id.is_none() && stored_session_id.is_some() {
                    tracing::info!(
                        "[QUEUE] Stream had no session_id, using stored session_id from conversation for queue processing"
                    );
                }

                // Only emit run_completed if there's no queue to process.
                // If there IS a queue, we'll emit run_completed after all queue messages are processed.
                // When turns were already finalized in the stream loop, skip the duplicate emission.
                if !will_process_queue {
                    // Clear streaming state cache - stream completed successfully
                    let conv_id_str = conversation_id.as_str();
                    streaming_state_cache.clear(&conv_id_str).await;

                    if !skip_post_loop_finalization || outcome.silent_interactive_exit {
                        if let Some(ref handle) = app_handle {
                            let _ = handle.emit(
                                "agent:run_completed",
                                AgentRunCompletedPayload {
                                    conversation_id: conversation_id.as_str().to_string(),
                                    context_type: context_type.to_string(),
                                    context_id: context_id.clone(),
                                    claude_session_id: effective_session_id.clone(),
                                    run_chain_id: run_chain_id.clone(),
                                },
                            );
                        }
                    }

                    // Trigger memory pipelines (no queue processing path)
                    trigger_memory_pipelines(
                        context_type,
                        &context_id,
                        &conversation_id,
                        resolved_project_id_typed.as_ref(),
                        agent_name.as_deref(),
                        &cli_path,
                        &plugin_dir,
                        &working_directory,
                        None,
                        Some(Arc::clone(&memory_event_repo)),
                    )
                    .await;
                } else {
                    tracing::info!(
                        "[QUEUE] Deferring run_completed: {} queued messages to process first",
                        initial_queue_count
                    );
                }

                // Process queued messages via extracted function
                if let Some(ref sess_id) = effective_session_id {
                    let total_processed = super::chat_service_queue::process_queued_messages(
                        context_type,
                        &context_id,
                        conversation_id,
                        sess_id,
                        &message_queue,
                        &chat_message_repo,
                        &chat_attachment_repo,
                        &activity_event_repo,
                        &task_repo,
                        &ideation_session_repo,
                        &cli_path,
                        &plugin_dir,
                        &working_directory,
                        question_state.clone(),
                        app_handle.clone(),
                        resolved_project_id.as_deref(),
                        team_mode,
                        cancellation_token.clone(),
                        run_chain_id.as_deref(),
                        Some(&agent_run_id),
                        streaming_state_cache.clone(),
                    )
                    .await;

                    // After ALL queue processing is done, emit the final run_completed
                    if total_processed > 0 {
                        tracing::info!("[QUEUE] Emitting final run_completed after processing {} queued messages", total_processed);

                        // Clear streaming state cache - queue processing completed
                        let conv_id_str = conversation_id.as_str();
                        streaming_state_cache.clear(&conv_id_str).await;

                        if let Some(ref handle) = app_handle {
                            let _ = handle.emit(
                                "agent:run_completed",
                                AgentRunCompletedPayload {
                                    conversation_id: conversation_id.as_str().to_string(),
                                    context_type: context_type.to_string(),
                                    context_id: context_id.clone(),
                                    claude_session_id: Some(sess_id.clone()),
                                    run_chain_id: run_chain_id.clone(),
                                },
                            );
                        }
                    }

                    // Trigger memory pipelines after queue processing completes
                    trigger_memory_pipelines(
                        context_type,
                        &context_id,
                        &conversation_id,
                        resolved_project_id_typed.as_ref(),
                        agent_name.as_deref(),
                        &cli_path,
                        &plugin_dir,
                        &working_directory,
                        None,
                        Some(Arc::clone(&memory_event_repo)),
                    )
                    .await;
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
                // Clear streaming state cache - stream errored
                let conv_id_str = conversation_id.as_str();
                streaming_state_cache.clear(&conv_id_str).await;

                // Delegate to error handler: classify, attempt recovery, fail run, emit events.
                // Returns true if recovery spawned a retry (no further action needed here
                // since the Err arm is the last statement in the async block).
                let error_string = e.to_string();
                let _recovery_spawned = super::chat_service_handlers::handle_stream_error(
                    &error_string,
                    Some(&e),
                    context_type,
                    &context_id,
                    conversation_id,
                    &agent_run_id,
                    &pre_assistant_msg_id,
                    &event_ctx,
                    stored_session_id.as_deref(),
                    is_retry_attempt,
                    user_message_content.as_deref(),
                    conversation.as_ref(),
                    resolved_project_id.clone(),
                    &cli_path,
                    &plugin_dir,
                    &working_directory,
                    &chat_message_repo,
                    &chat_attachment_repo,
                    &conversation_repo,
                    &agent_run_repo,
                    &task_repo,
                    &task_dependency_repo,
                    &project_repo,
                    &ideation_session_repo,
                    &task_proposal_repo,
                    &activity_event_repo,
                    &message_queue,
                    &running_agent_registry,
                    &memory_event_repo,
                    &execution_state,
                    &question_state,
                    &plan_branch_repo,
                    &app_handle,
                    agent_name.as_deref(),
                    team_mode,
                    run_chain_id.clone(),
                    &interactive_process_registry,
                    &review_repo,
                    &task_step_repo,
                )
                .await;
            }
        }
    }.instrument(span));
}

#[cfg(test)]
#[path = "chat_service_send_background_tests.rs"]
mod tests;
