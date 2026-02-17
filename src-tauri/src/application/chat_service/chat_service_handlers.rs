// Success/error handler logic for background send processing.
//
// Extracted from chat_service_send_background.rs to reduce file size.
// Contains:
// - handle_stream_success: task transitions (TaskExecution → PendingReview/Failed)
//   and merge auto-completion after successful stream processing
// - handle_stream_error: error classification, stale session recovery retry,
//   agent run failure recording, message finalization, and fallback task transitions

use std::path::Path;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};

use crate::application::question_state::QuestionState;
use crate::application::task_scheduler_service::TaskSchedulerService;
use crate::application::task_transition_service::TaskTransitionService;
use crate::commands::ExecutionState;
use crate::domain::entities::{
    AgentRunId, ChatContextType, ChatConversation, ChatConversationId, ChatMessageId,
    InternalStatus, TaskId,
};
use crate::domain::repositories::{
    ActivityEventRepository, AgentRunRepository, ChatAttachmentRepository,
    ChatConversationRepository, ChatMessageRepository, IdeationSessionRepository,
    MemoryEventRepository, PlanBranchRepository, ProjectRepository, TaskDependencyRepository,
    TaskRepository,
};
use crate::domain::services::{MessageQueue, RunningAgentRegistry};
use crate::domain::state_machine::services::TaskScheduler;
use crate::error::AppError;

use super::chat_service_context;
use super::chat_service_errors::{classify_agent_error, StreamError};
use super::chat_service_helpers::get_assistant_role;
use super::chat_service_types::AgentErrorPayload;
use super::EventContextPayload;

/// Read existing message content and tool_calls from the database.
///
/// Used before error finalization to preserve any content that was flushed
/// during streaming, so the error note is appended rather than overwriting.
async fn read_existing_message_content(
    chat_message_repo: &Arc<dyn ChatMessageRepository>,
    message_id: &str,
) -> (String, Option<String>) {
    match chat_message_repo
        .get_by_id(&ChatMessageId::from_string(message_id.to_string()))
        .await
    {
        Ok(Some(msg)) => (msg.content, msg.tool_calls),
        _ => (String::new(), None),
    }
}

/// Handle successful stream completion: task state transitions and merge auto-completion.
///
/// For TaskExecution context:
/// - If agent produced output → transition to PendingReview
/// - If agent produced no output → transition to Failed
///
/// For Merge context:
/// - Attempts merge auto-completion via git state inspection
#[allow(clippy::too_many_arguments)]
pub(super) async fn handle_stream_success<R: Runtime>(
    context_type: ChatContextType,
    context_id: &str,
    has_output: bool,
    execution_state: &Option<Arc<ExecutionState>>,
    task_repo: &Arc<dyn TaskRepository>,
    task_dependency_repo: &Arc<dyn TaskDependencyRepository>,
    project_repo: &Arc<dyn ProjectRepository>,
    chat_message_repo: &Arc<dyn ChatMessageRepository>,
    chat_attachment_repo: &Arc<dyn ChatAttachmentRepository>,
    conversation_repo: &Arc<dyn ChatConversationRepository>,
    agent_run_repo: &Arc<dyn AgentRunRepository>,
    ideation_session_repo: &Arc<dyn IdeationSessionRepository>,
    activity_event_repo: &Arc<dyn ActivityEventRepository>,
    message_queue: &Arc<MessageQueue>,
    running_agent_registry: &Arc<dyn RunningAgentRegistry>,
    memory_event_repo: &Arc<dyn MemoryEventRepository>,
    plan_branch_repo: &Option<Arc<dyn PlanBranchRepository>>,
    app_handle: &Option<AppHandle<R>>,
) {
    // Handle task state transition (only for TaskExecution)
    if context_type == ChatContextType::TaskExecution {
        if let Some(ref exec_state) = execution_state {
            let task_id = TaskId::from_string(context_id.to_string());
            if let Ok(Some(task)) = task_repo.get_by_id(&task_id).await {
                if task.internal_status == InternalStatus::Executing
                    || task.internal_status == InternalStatus::ReExecuting
                {
                    // Create scheduler for auto-scheduling next Ready task
                    let mut scheduler_svc = TaskSchedulerService::new(
                        Arc::clone(exec_state),
                        Arc::clone(project_repo),
                        Arc::clone(task_repo),
                        Arc::clone(task_dependency_repo),
                        Arc::clone(chat_message_repo),
                        Arc::clone(chat_attachment_repo),
                        Arc::clone(conversation_repo),
                        Arc::clone(agent_run_repo),
                        Arc::clone(ideation_session_repo),
                        Arc::clone(activity_event_repo),
                        Arc::clone(message_queue),
                        Arc::clone(running_agent_registry),
                        Arc::clone(memory_event_repo),
                        app_handle.clone(),
                    );
                    if let Some(ref repo) = plan_branch_repo {
                        scheduler_svc = scheduler_svc.with_plan_branch_repo(Arc::clone(repo));
                    }
                    let scheduler_concrete = Arc::new(scheduler_svc);
                    scheduler_concrete
                        .set_self_ref(Arc::clone(&scheduler_concrete) as Arc<dyn TaskScheduler>);
                    let task_scheduler: Arc<dyn TaskScheduler> = scheduler_concrete;

                    let transition_service = TaskTransitionService::new(
                        Arc::clone(task_repo),
                        Arc::clone(task_dependency_repo),
                        Arc::clone(project_repo),
                        Arc::clone(chat_message_repo),
                        Arc::clone(chat_attachment_repo),
                        Arc::clone(conversation_repo),
                        Arc::clone(agent_run_repo),
                        Arc::clone(ideation_session_repo),
                        Arc::clone(activity_event_repo),
                        Arc::clone(message_queue),
                        Arc::clone(running_agent_registry),
                        Arc::clone(exec_state),
                        app_handle.clone(),
                        Arc::clone(memory_event_repo),
                    )
                    .with_task_scheduler(task_scheduler);
                    let transition_service = if let Some(ref repo) = plan_branch_repo {
                        transition_service.with_plan_branch_repo(Arc::clone(repo))
                    } else {
                        transition_service
                    };
                    if has_output {
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
                    } else if let Err(e) = transition_service
                        .transition_task(&task_id, InternalStatus::Failed)
                        .await
                    {
                        tracing::error!(
                            "Failed to transition empty-output task {} to Failed: {}",
                            task_id.as_str(),
                            e
                        );
                    } else {
                        tracing::warn!(
                            task_id = task_id.as_str(),
                            "Task execution produced no output; transitioned to Failed"
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

    // Handle merge auto-completion (only for Merge context)
    if context_type == ChatContextType::Merge {
        if let Some(ref exec_state) = execution_state {
            super::chat_service_merge::attempt_merge_auto_complete(
                context_id,
                task_repo,
                task_dependency_repo,
                project_repo,
                chat_message_repo,
                chat_attachment_repo,
                conversation_repo,
                agent_run_repo,
                ideation_session_repo,
                activity_event_repo,
                message_queue,
                running_agent_registry,
                memory_event_repo,
                exec_state,
                plan_branch_repo,
                app_handle.as_ref(),
            )
            .await;
        } else {
            tracing::warn!(
                "Cannot auto-complete merge for task {} - no execution_state available",
                context_id
            );
        }
    }
}

/// Handle stream error: classify error, attempt stale session recovery,
/// fail agent run, finalize message, emit error event, and transition task to Failed.
///
/// Accepts both the typed `StreamError` (for precise matching) and a pre-formatted
/// error string (for backward-compatible logging and message storage).
///
/// Returns `true` if recovery was successful and a retry was spawned (caller should return early).
/// Returns `false` if normal error handling was performed.
#[allow(clippy::too_many_arguments)]
pub(super) async fn handle_stream_error<R: Runtime + 'static>(
    error: &str,
    stream_error: Option<&StreamError>,
    context_type: ChatContextType,
    context_id: &str,
    conversation_id: ChatConversationId,
    agent_run_id: &str,
    pre_assistant_msg_id: &str,
    event_ctx: &EventContextPayload,
    stored_session_id: Option<&str>,
    is_retry_attempt: bool,
    user_message_content: Option<&str>,
    conversation: Option<&ChatConversation>,
    resolved_project_id: Option<String>,
    cli_path: &Path,
    plugin_dir: &Path,
    working_directory: &Path,
    chat_message_repo: &Arc<dyn ChatMessageRepository>,
    chat_attachment_repo: &Arc<dyn ChatAttachmentRepository>,
    conversation_repo: &Arc<dyn ChatConversationRepository>,
    agent_run_repo: &Arc<dyn AgentRunRepository>,
    task_repo: &Arc<dyn TaskRepository>,
    task_dependency_repo: &Arc<dyn TaskDependencyRepository>,
    project_repo: &Arc<dyn ProjectRepository>,
    ideation_session_repo: &Arc<dyn IdeationSessionRepository>,
    activity_event_repo: &Arc<dyn ActivityEventRepository>,
    message_queue: &Arc<MessageQueue>,
    running_agent_registry: &Arc<dyn RunningAgentRegistry>,
    memory_event_repo: &Arc<dyn MemoryEventRepository>,
    execution_state: &Option<Arc<ExecutionState>>,
    question_state: &Option<Arc<QuestionState>>,
    plan_branch_repo: &Option<Arc<dyn PlanBranchRepository>>,
    app_handle: &Option<AppHandle<R>>,
    agent_name: Option<&str>,
    team_mode: bool,
    run_chain_id: Option<String>,
) -> bool {
    // Handle cancellation: skip all recovery/transitions, just mark as stopped
    if error == "Cancelled" {
        tracing::info!(
            conversation_id = conversation_id.as_str(),
            context_type = %context_type,
            context_id,
            "Stream cancelled — skipping error recovery and fallback transitions"
        );
        let _ = agent_run_repo
            .fail(
                &AgentRunId::from_string(agent_run_id),
                "Agent stopped by user",
            )
            .await;

        // Update pre-created message — append stop note to any content already flushed
        let (existing_content, existing_tool_calls) = read_existing_message_content(
            chat_message_repo, pre_assistant_msg_id,
        ).await;
        let stop_note = if existing_content.is_empty() {
            "[Agent stopped]".to_string()
        } else {
            format!("{}\n\n[Agent stopped]", existing_content)
        };
        super::chat_service_send_background::finalize_assistant_message(
            chat_message_repo,
            app_handle.as_ref(),
            event_ctx,
            pre_assistant_msg_id,
            &get_assistant_role(&context_type).to_string(),
            &stop_note,
            existing_tool_calls.as_deref(),
            None,
        )
        .await;

        if let Some(ref handle) = app_handle {
            let _ = handle.emit(
                "agent:stopped",
                serde_json::json!({
                    "conversation_id": conversation_id.as_str(),
                    "agent_run_id": agent_run_id,
                    "context_type": context_type.to_string(),
                    "context_id": context_id,
                }),
            );
        }
        return false;
    }

    // Classify error to detect stale session
    let classified_error = classify_agent_error(error, &conversation_id, stored_session_id);

    match classified_error {
        AppError::StaleSession { session_id, .. } => {
            tracing::warn!(
                event = "stale_session_detected",
                session_id = %session_id,
                conversation_id = conversation_id.as_str(),
                context_type = %context_type,
                context_id = %context_id,
                "Detected stale Claude session"
            );

            // Feature flag check
            let recovery_enabled = std::env::var("ENABLE_SESSION_RECOVERY")
                .map(|v| v.to_lowercase() == "true")
                .unwrap_or(false);

            // Check retry flag (prevent infinite loop)
            if is_retry_attempt {
                tracing::error!(
                    conversation_id = conversation_id.as_str(),
                    "Session recovery failed on retry, aborting"
                );
                // Fall through to normal error handling below
            } else if !recovery_enabled {
                tracing::info!(
                    "Session recovery disabled by ENABLE_SESSION_RECOVERY flag, falling back to clear"
                );
                // Fall through to clear session
            } else if let (Some(msg), Some(conv)) = (user_message_content, conversation) {
                // Attempt recovery
                match super::chat_service_recovery::attempt_session_recovery(
                    &conversation_id,
                    conv,
                    context_type,
                    context_id,
                    msg,
                    cli_path,
                    plugin_dir,
                    working_directory,
                    resolved_project_id.clone(),
                    team_mode,
                    Arc::clone(chat_message_repo),
                    Arc::clone(conversation_repo),
                    Arc::clone(chat_attachment_repo),
                    &session_id,
                )
                .await
                {
                    Ok(new_session_id) => {
                        tracing::info!(
                            event = "rehydrate_success",
                            old_session = %session_id,
                            new_session = %new_session_id,
                            "Session recovery successful, retrying send"
                        );

                        // Emit non-blocking banner event
                        if let Some(ref handle) = app_handle {
                            let _ = handle.emit(
                                "agent:session_recovered",
                                serde_json::json!({
                                    "conversation_id": conversation_id.as_str(),
                                    "message": "Session restored from local history"
                                }),
                            );
                        }

                        // Retry send with fresh session (set is_retry=true)
                        let mut retry_conv = conv.clone();
                        retry_conv.claude_session_id = Some(new_session_id.clone());

                        // Build command for retry
                        if let Ok(spawnable) = chat_service_context::build_command(
                            cli_path,
                            plugin_dir,
                            &retry_conv,
                            msg,
                            working_directory,
                            None,
                            resolved_project_id.as_deref(),
                            team_mode,
                            Arc::clone(chat_attachment_repo),
                        )
                        .await
                        {
                            if let Ok(retry_child) = spawnable.spawn().await {
                                use super::chat_service_send_background::{
                                    BackgroundRunContext, BackgroundRunRepos,
                                };
                                // Recursive call with is_retry_attempt=true
                                super::chat_service_send_background::spawn_send_message_background(
                                    BackgroundRunContext {
                                        child: retry_child,
                                        context_type,
                                        context_id: context_id.to_string(),
                                        conversation_id,
                                        agent_run_id: agent_run_id.to_string(),
                                        stored_session_id: Some(new_session_id),
                                        working_directory: working_directory.to_path_buf(),
                                        cli_path: cli_path.to_path_buf(),
                                        plugin_dir: plugin_dir.to_path_buf(),
                                        repos: BackgroundRunRepos {
                                            chat_message_repo: Arc::clone(chat_message_repo),
                                            chat_attachment_repo: Arc::clone(chat_attachment_repo),
                                            conversation_repo: Arc::clone(conversation_repo),
                                            agent_run_repo: Arc::clone(agent_run_repo),
                                            task_repo: Arc::clone(task_repo),
                                            task_dependency_repo: Arc::clone(task_dependency_repo),
                                            project_repo: Arc::clone(project_repo),
                                            ideation_session_repo: Arc::clone(
                                                ideation_session_repo,
                                            ),
                                            activity_event_repo: Arc::clone(activity_event_repo),
                                            memory_event_repo: Arc::clone(memory_event_repo),
                                            message_queue: Arc::clone(message_queue),
                                            running_agent_registry: Arc::clone(
                                                running_agent_registry,
                                            ),
                                        },
                                        execution_state: execution_state.clone(),
                                        question_state: question_state.clone(),
                                        plan_branch_repo: plan_branch_repo.clone(),
                                        app_handle: app_handle.clone(),
                                        run_chain_id: run_chain_id.clone(),
                                        is_retry_attempt: true,
                                        user_message_content: user_message_content
                                            .map(|s| s.to_string()),
                                        conversation: Some(retry_conv),
                                        agent_name: agent_name.map(|s| s.to_string()),
                                        team_mode,
                                        cancellation_token:
                                            tokio_util::sync::CancellationToken::new(),
                                        team_service: None, // Recovery retries don't need team events
                                        streaming_state_cache: super::StreamingStateCache::new(), // Fresh cache for retry
                                    },
                                );

                                return true; // Recovery spawned retry, caller should return early
                            }
                        }

                        tracing::error!("Failed to spawn retry after recovery");
                        // Fall through to error handling
                    }
                    Err(recovery_err) => {
                        tracing::error!(
                            error = %recovery_err,
                            "Session recovery failed, falling back to clear"
                        );
                        // Fall through to normal error handling
                    }
                }
            }

            // Clear stale session ID as fallback
            let _ = conversation_repo
                .clear_claude_session_id(&conversation_id)
                .await;
        }
        _ => {
            // Non-stale-session errors: clear session if typed error requires it
            if let Some(se) = stream_error {
                if se.requires_session_clear() {
                    tracing::info!(
                        conversation_id = conversation_id.as_str(),
                        error_type = %se,
                        "Clearing session ID due to stream error requiring session reset"
                    );
                    let _ = conversation_repo
                        .clear_claude_session_id(&conversation_id)
                        .await;
                }
            }
        }
    }

    // Standard error handling (reached if recovery not attempted or failed)
    // Fail the agent run
    let _ = agent_run_repo
        .fail(&AgentRunId::from_string(agent_run_id), error)
        .await;

    // Read existing content before overwriting — append error to any content already flushed
    let (existing_content, existing_tool_calls) = read_existing_message_content(
        chat_message_repo, pre_assistant_msg_id,
    ).await;
    let error_note = if existing_content.is_empty() {
        format!("[Agent error: {}]", error)
    } else {
        format!("{}\n\n[Agent error: {}]", existing_content, error)
    };
    super::chat_service_send_background::finalize_assistant_message(
        chat_message_repo,
        app_handle.as_ref(),
        event_ctx,
        pre_assistant_msg_id,
        &get_assistant_role(&context_type).to_string(),
        &error_note,
        existing_tool_calls.as_deref(),
        None,
    )
    .await;

    // Emit error event
    if let Some(ref handle) = app_handle {
        let _ = handle.emit(
            "agent:error",
            AgentErrorPayload {
                conversation_id: Some(conversation_id.as_str().to_string()),
                context_type: context_type.to_string(),
                context_id: context_id.to_string(),
                error: error.to_string(),
                stderr: Some(error.to_string()),
            },
        );
    }

    // For worker execution failures, transition task out of active execution
    // Use StreamError::suggested_task_status() for precise transition when available
    // For ProviderErrors, store metadata and pause instead of failing
    if context_type == ChatContextType::TaskExecution {
        if let Some(ref exec_state) = execution_state {
            let task_id = TaskId::from_string(context_id.to_string());
            let target_status = stream_error
                .and_then(|se| se.suggested_task_status())
                .unwrap_or(InternalStatus::Failed);
            match task_repo.get_by_id(&task_id).await {
                Ok(Some(task))
                    if task.internal_status == InternalStatus::Executing
                        || task.internal_status == InternalStatus::ReExecuting =>
                {
                    // If this is a provider error → store metadata before pausing
                    if let Some(se) = stream_error {
                        if se.is_provider_error() {
                            if let Some(mut meta) =
                                se.provider_error_metadata(task.internal_status)
                            {
                                // Carry forward resume_attempts from existing metadata
                                // so the MAX_RESUME_ATTEMPTS limit works across re-pause cycles
                                if let Some(existing) = super::PauseReason::from_task_metadata(
                                    task.metadata.as_deref(),
                                ) {
                                    if let super::PauseReason::ProviderError { resume_attempts, .. } = existing {
                                        meta.resume_attempts = resume_attempts;
                                    }
                                } else if let Some(existing) = super::ProviderErrorMetadata::from_task_metadata(
                                    task.metadata.as_deref(),
                                ) {
                                    meta.resume_attempts = existing.resume_attempts;
                                }

                                // Write both legacy provider_error and new pause_reason keys
                                let pause_reason = super::PauseReason::ProviderError {
                                    category: meta.category.clone(),
                                    message: meta.message.clone(),
                                    retry_after: meta.retry_after.clone(),
                                    previous_status: meta.previous_status.clone(),
                                    paused_at: meta.paused_at.clone(),
                                    auto_resumable: meta.auto_resumable,
                                    resume_attempts: meta.resume_attempts,
                                };
                                let mut updated_task = task.clone();
                                let with_legacy = meta.write_to_task_metadata(
                                    updated_task.metadata.as_deref(),
                                );
                                updated_task.metadata = Some(
                                    pause_reason.write_to_task_metadata(Some(&with_legacy)),
                                );
                                updated_task.touch();
                                if let Err(e) = task_repo.update(&updated_task).await {
                                    tracing::error!(
                                        task_id = task_id.as_str(),
                                        error = %e,
                                        "Failed to store provider error metadata"
                                    );
                                } else {
                                    tracing::info!(
                                        task_id = task_id.as_str(),
                                        category = %meta.category,
                                        retry_after = ?meta.retry_after,
                                        "Stored provider error metadata, will pause task"
                                    );
                                }

                                // Emit provider error event for frontend
                                if let Some(ref handle) = app_handle {
                                    let _ = handle.emit(
                                        "task:provider_error_paused",
                                        serde_json::json!({
                                            "task_id": task_id.as_str(),
                                            "category": meta.category.to_string(),
                                            "message": meta.message,
                                            "retry_after": meta.retry_after,
                                            "previous_status": meta.previous_status,
                                            "auto_resumable": meta.auto_resumable,
                                        }),
                                    );
                                }
                            }
                        }
                    }

                    let transition_service = TaskTransitionService::new(
                        Arc::clone(task_repo),
                        Arc::clone(task_dependency_repo),
                        Arc::clone(project_repo),
                        Arc::clone(chat_message_repo),
                        Arc::clone(chat_attachment_repo),
                        Arc::clone(conversation_repo),
                        Arc::clone(agent_run_repo),
                        Arc::clone(ideation_session_repo),
                        Arc::clone(activity_event_repo),
                        Arc::clone(message_queue),
                        Arc::clone(running_agent_registry),
                        Arc::clone(exec_state),
                        app_handle.clone(),
                        Arc::clone(memory_event_repo),
                    );
                    let transition_service = if let Some(ref repo) = plan_branch_repo {
                        transition_service.with_plan_branch_repo(Arc::clone(repo))
                    } else {
                        transition_service
                    };

                    if let Err(transition_err) = transition_service
                        .transition_task(&task_id, target_status)
                        .await
                    {
                        tracing::warn!(
                            task_id = task_id.as_str(),
                            original_error = %error,
                            transition_error = %transition_err,
                            target_status = %target_status,
                            "Worker failed and fallback transition also failed — retrying after 500ms"
                        );
                        // D4: Retry once after 500ms delay
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        if let Err(retry_err) = transition_service
                            .transition_task(&task_id, target_status)
                            .await
                        {
                            tracing::error!(
                                task_id = task_id.as_str(),
                                original_error = %error,
                                retry_error = %retry_err,
                                target_status = %target_status,
                                "Worker failed and fallback transition retry also failed — task may be stuck"
                            );
                            // Emit event so reconciliation can pick it up
                            if let Some(ref handle) = app_handle {
                                let _ = handle.emit(
                                    "task:recovery_failed",
                                    serde_json::json!({
                                        "task_id": task_id.as_str(),
                                        "original_error": error,
                                        "transition_error": retry_err.to_string(),
                                        "target_status": target_status.to_string(),
                                    }),
                                );
                            }
                        }
                    } else {
                        tracing::warn!(
                            task_id = task_id.as_str(),
                            error = %error,
                            target_status = %target_status,
                            "Worker failed; transitioned task"
                        );
                    }
                }
                Ok(Some(_)) => {}
                Ok(None) => {
                    tracing::warn!(
                        task_id = context_id,
                        error = %error,
                        "Worker failed but task was not found for fallback transition"
                    );
                }
                Err(repo_err) => {
                    tracing::error!(
                        task_id = context_id,
                        error = %error,
                        repo_error = %repo_err,
                        "Worker failed and task lookup failed for fallback transition"
                    );
                }
            }
        } else {
            tracing::warn!(
                task_id = context_id,
                error = %error,
                "Worker failed but no execution_state available for fallback transition"
            );
        }
    }

    // Handle merge auto-completion even on agent error
    if context_type == ChatContextType::Merge {
        if let Some(ref exec_state) = execution_state {
            super::chat_service_merge::attempt_merge_auto_complete(
                context_id,
                task_repo,
                task_dependency_repo,
                project_repo,
                chat_message_repo,
                chat_attachment_repo,
                conversation_repo,
                agent_run_repo,
                ideation_session_repo,
                activity_event_repo,
                message_queue,
                running_agent_registry,
                memory_event_repo,
                exec_state,
                plan_branch_repo,
                app_handle.as_ref(),
            )
            .await;
        } else {
            tracing::warn!(
                "Cannot auto-complete merge for task {} on error - no execution_state available",
                context_id
            );
        }
    }

    false // Normal error handling performed, no retry spawned
}
