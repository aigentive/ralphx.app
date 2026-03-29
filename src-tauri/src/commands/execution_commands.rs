// Tauri commands for execution control
// Manages per-project execution state: pause, resume, stop
// Phase 82: Project-scoped execution with optional project_id parameters

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use tauri::{AppHandle, Emitter, Runtime, State};
use tokio::sync::RwLock;

use crate::application::chat_service::{
    ChatService, ClaudeChatService, SendMessageOptions, uses_execution_slot,
};
use crate::application::reconciliation::UserRecoveryAction;
use crate::application::team_state_tracker::TeamStateTracker;
use crate::application::{
    AppState, ReconciliationRunner, TaskSchedulerService, TaskTransitionService,
};
use crate::domain::entities::{
    ChatContextType, IdeationSessionStatus, InternalStatus, ProjectId, Task, TaskId,
    app_state::ExecutionHaltMode, task_step::StepProgressSummary, types::IdeationSessionId,
};
use crate::domain::execution::ExecutionSettings;
use crate::domain::execution::{ScopedExecutionSubject, count_execution_status};
use crate::domain::execution::{
    build_running_ideation_session, build_running_process, elapsed_seconds_for_status,
};
use crate::domain::services::QueueKey;
use crate::domain::state_machine::services::TaskScheduler;
use crate::domain::state_machine::transition_handler::get_trigger_origin;

mod state;

pub use state::{
    AGENT_ACTIVE_STATUSES, AUTO_TRANSITION_STATES, ActiveProjectState, ExecutionCommandResponse,
    ExecutionSettingsResponse, ExecutionState, ExecutionStatusResponse,
    GlobalExecutionSettingsResponse, UpdateExecutionSettingsInput,
    UpdateGlobalExecutionSettingsInput,
};

use state::*;

mod control_helpers;

pub use control_helpers::count_active_ideation_slots;
pub use control_helpers::project_has_execution_capacity_for_state;
use control_helpers::*;

mod recovery;

pub use recovery::{
    CategorizedResume, RestartResult, ResumeCategory, ResumeValidationResult,
    ResumeValidationWarning, categorize_resume_state,
};
use recovery::{
    build_reconciler_for_recovery, build_transition_service_for_recovery, validate_resume,
};

mod running;

pub use running::{
    RunningIdeationSession, RunningProcess, RunningProcessesResponse,
    context_matches_running_status_for_gc,
};
use running::prune_stale_execution_registry_entries;

mod scheduling;
use scheduling::schedule_ready_tasks_for_project;

/// Get current execution status
/// Phase 82: Optional project_id for per-project scoping.
/// If project_id is None, falls back to active project or aggregates across all projects.
#[tauri::command]
pub async fn get_execution_status(
    project_id: Option<String>,
    active_project_state: State<'_, Arc<ActiveProjectState>>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app_state: State<'_, AppState>,
) -> Result<ExecutionStatusResponse, String> {
    // Sync runtime quota with persisted project settings before returning status
    let project_id = project_id.map(|id| ProjectId::from_string(id));
    let (effective_project_id, _max_concurrent) = sync_quota_from_project(
        project_id,
        &active_project_state,
        &execution_state,
        &app_state,
    )
    .await?;

    // Count queued tasks (tasks in Ready status)
    let mut queued_count = 0u32;

    if let Some(pid) = &effective_project_id {
        // Scoped to single project
        let tasks = app_state
            .task_repo
            .get_by_project(pid)
            .await
            .map_err(|e| e.to_string())?;

        queued_count = tasks
            .iter()
            .filter(|t| t.internal_status == InternalStatus::Ready)
            .count() as u32;
    } else {
        // Aggregate across all projects
        let all_projects = app_state
            .project_repo
            .get_all()
            .await
            .map_err(|e| e.to_string())?;

        for project in &all_projects {
            let tasks = app_state
                .task_repo
                .get_by_project(&project.id)
                .await
                .map_err(|e| e.to_string())?;

            queued_count += tasks
                .iter()
                .filter(|t| t.internal_status == InternalStatus::Ready)
                .count() as u32;
        }
    }

    let queued_message_count =
        count_slot_consuming_queued_messages(effective_project_id.as_ref(), &app_state).await?;

    // Runtime GC pass to prune stale rows on every status poll.
    prune_stale_execution_registry_entries(&app_state, &execution_state).await;

    let registry_entries = app_state.running_agent_registry.list_all().await;

    // Keep execution state synchronized to global execution contexts.
    // Subtract idle interactive slots (processes alive between turns that already
    // freed their execution slot via TurnComplete) to avoid re-inflating the counter.
    let total_with_slot = registry_entries
        .iter()
        .filter(|(key, _)| {
            ChatContextType::from_str(&key.context_type)
                .map(uses_execution_slot)
                .unwrap_or(false)
        })
        .count();
    let active_count =
        (total_with_slot.saturating_sub(execution_state.interactive_idle_count())) as u32;
    execution_state.set_running_count(active_count);
    let global_running_count = active_count;

    let mut scoped_subjects = Vec::new();
    for (key, _) in registry_entries {
        let context_type = match ChatContextType::from_str(&key.context_type) {
            Ok(value) => value,
            Err(_) => continue,
        };

        if !uses_execution_slot(context_type) {
            continue;
        }

        // Ideation uses session IDs (not task IDs) — look up session for project filtering.
        // Track active (generating) and idle (waiting_for_input) separately.
        if matches!(context_type, ChatContextType::Ideation) {
            let session_id = IdeationSessionId::from_string(key.context_id.clone());
            let session = match app_state
                .ideation_session_repo
                .get_by_id(&session_id)
                .await
            {
                Ok(Some(s)) => s,
                _ => continue, // orphaned registry entry — skip
            };
            if let Some(pid) = &effective_project_id {
                if session.project_id != *pid {
                    continue;
                }
            }
            let slot_key = format!("{}/{}", key.context_type, key.context_id);
            scoped_subjects.push(ScopedExecutionSubject::Ideation {
                project_id: session.project_id,
                is_idle: execution_state.is_interactive_idle(&slot_key),
            });
            continue;
        }

        let task_id = TaskId::from_string(key.context_id);
        let task = match app_state.task_repo.get_by_id(&task_id).await {
            Ok(Some(task)) => task,
            _ => continue,
        };

        scoped_subjects.push(ScopedExecutionSubject::Task {
            context_type,
            project_id: task.project_id,
            status: task.internal_status,
        });
    }
    let counts = count_execution_status(scoped_subjects, effective_project_id.as_ref());

    // Count sessions waiting for ideation capacity (have pending_initial_prompt set).
    let ideation_waiting = match &effective_project_id {
        Some(pid) => app_state
            .ideation_session_repo
            .count_pending_sessions_for_project(pid)
            .await
            .unwrap_or(0),
        None => 0,
    };

    let max_concurrent = execution_state.max_concurrent();
    let global_max = execution_state.global_max_concurrent();
    let halt_mode = load_execution_halt_mode(&app_state).await?;

    let blocked_until = execution_state.provider_blocked_until_epoch();
    Ok(ExecutionStatusResponse {
        is_paused: execution_state.is_paused(),
        halt_mode: execution_halt_mode_str(halt_mode).to_string(),
        running_count: counts.running_count,
        max_concurrent,
        global_max_concurrent: global_max,
        queued_count,
        queued_message_count,
        can_start_task: !execution_state.is_paused()
            && !execution_state.is_provider_blocked()
            && counts.total_project_active < max_concurrent
            && global_running_count < global_max,
        provider_blocked: execution_state.is_provider_blocked(),
        provider_blocked_until: if blocked_until > 0 {
            Some(blocked_until)
        } else {
            None
        },
        ideation_active: counts.ideation_active,
        ideation_idle: counts.ideation_idle,
        ideation_waiting,
        ideation_max_project: execution_state.project_ideation_max(),
        ideation_max_global: execution_state.global_ideation_max(),
    })
}

/// Pause execution (stops picking up new tasks and transitions running tasks to Paused)
/// This transitions all agent-active tasks to Paused status via TransitionHandler.
/// Paused is NOT terminal - tasks can be auto-restored on resume using status history.
/// The on_exit handlers will decrement the running count for each task.
/// Phase 82: Optional project_id for per-project scoping.
///
/// ## Pause contract
/// 1. `execution_state.pause()` — gates new task scheduling immediately.
/// 2. `running_agent_registry.stop_all()` — kills all agent processes (even if transition fails).
/// 3. For each task in `AGENT_ACTIVE_STATUSES`:
///    a. Write `PauseReason::UserInitiated { previous_status, paused_at, scope: "global" }`
///       to `task.metadata["pause_reason"]` — this is what `resume_execution` reads back.
///    b. Call `transition_task(id, Paused)` via `TransitionHandler` — triggers `on_exit`
///       which decrements `running_count` and emits `execution:status_changed`.
/// 4. `Paused` state machine rejects all events (resume is command-layer only).
///
/// ## What Pause does NOT affect
/// - `Blocked` tasks: already idle, not agent-active.
/// - `Stopped` tasks: already terminal.
/// - `Paused` tasks: already paused (idempotent — won't double-pause).
#[tauri::command]
pub async fn pause_execution(
    project_id: Option<String>,
    active_project_state: State<'_, Arc<ActiveProjectState>>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app_state: State<'_, AppState>,
) -> Result<ExecutionCommandResponse, String> {
    // Sync runtime quota with persisted project settings for consistency
    let project_id = project_id.map(|id| ProjectId::from_string(id));
    let (effective_project_id, _max_concurrent) = sync_quota_from_project(
        project_id,
        &active_project_state,
        &execution_state,
        &app_state,
    )
    .await?;

    // First pause to prevent new tasks from starting
    execution_state.pause();
    persist_execution_halt_mode(&app_state, ExecutionHaltMode::Paused).await?;

    // Kill all running agent processes immediately via registry
    // This ensures agents are terminated even if transition fails
    app_state.running_agent_registry.stop_all().await;
    // Also clear all interactive process entries — their stdin pipes are now dead
    app_state.interactive_process_registry.clear().await;

    // Build transition service for proper state machine transitions
    let transition_service = TaskTransitionService::new(
        Arc::clone(&app_state.task_repo),
        Arc::clone(&app_state.task_dependency_repo),
        Arc::clone(&app_state.project_repo),
        Arc::clone(&app_state.chat_message_repo),
        Arc::clone(&app_state.chat_attachment_repo),
        Arc::clone(&app_state.chat_conversation_repo),
        Arc::clone(&app_state.agent_run_repo),
        Arc::clone(&app_state.ideation_session_repo),
        Arc::clone(&app_state.activity_event_repo),
        Arc::clone(&app_state.message_queue),
        Arc::clone(&app_state.running_agent_registry),
        Arc::clone(&execution_state),
        app_state.app_handle.clone(),
        Arc::clone(&app_state.memory_event_repo),
    )
    .with_execution_settings_repo(Arc::clone(&app_state.execution_settings_repo))
    .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo))
    .with_interactive_process_registry(Arc::clone(&app_state.interactive_process_registry));

    // Find all tasks in agent-active states (scoped to project if specified)
    let projects_to_process = if let Some(ref pid) = effective_project_id {
        match app_state.project_repo.get_by_id(pid).await {
            Ok(Some(project)) => vec![project],
            Ok(None) => return Err(format!("Project not found: {}", pid.as_str())),
            Err(e) => return Err(e.to_string()),
        }
    } else {
        app_state
            .project_repo
            .get_all()
            .await
            .map_err(|e| e.to_string())?
    };

    for project in projects_to_process {
        let tasks = app_state
            .task_repo
            .get_by_project(&project.id)
            .await
            .map_err(|e| e.to_string())?;

        for task in tasks {
            // Check if task is in an agent-active state
            if AGENT_ACTIVE_STATUSES.contains(&task.internal_status) {
                // Store PauseReason::UserInitiated metadata before transitioning
                let pause_reason = crate::application::chat_service::PauseReason::UserInitiated {
                    previous_status: task.internal_status.to_string(),
                    paused_at: chrono::Utc::now().to_rfc3339(),
                    scope: "global".to_string(),
                };
                let mut updated_task = task.clone();
                updated_task.metadata =
                    Some(pause_reason.write_to_task_metadata(updated_task.metadata.as_deref()));
                updated_task.touch();
                let _ = app_state.task_repo.update(&updated_task).await;

                // Use TransitionHandler to transition to Paused
                // Paused is NOT terminal - can be restored on resume
                // This triggers on_exit handlers which decrement running count
                if let Err(e) = transition_service
                    .transition_task(&task.id, InternalStatus::Paused)
                    .await
                {
                    tracing::warn!(
                        task_id = task.id.as_str(),
                        error = %e,
                        "Failed to transition task to Paused during pause"
                    );
                }
            }
        }
    }

    // Note: running_count is decremented by on_exit handlers in TransitionHandler
    // No manual reset needed here

    // Emit status_changed event for real-time UI update with projectId
    // This reflects the final state after all tasks have been paused
    if let Some(ref handle) = app_state.app_handle {
        let _ = handle.emit(
            "execution:status_changed",
            serde_json::json!({
                "isPaused": execution_state.is_paused(),
                "haltMode": "paused",
                "runningCount": execution_state.running_count(),
                "maxConcurrent": execution_state.max_concurrent(),
                "reason": "paused",
                "projectId": effective_project_id.as_ref().map(|p| p.as_str()),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        );
    }

    // Get current status
    let status = get_execution_status(
        effective_project_id.map(|p| p.as_str().to_string()),
        active_project_state,
        execution_state,
        app_state,
    )
    .await?;

    Ok(ExecutionCommandResponse {
        success: true,
        status,
    })
}

/// Resume execution (restores Paused tasks and allows picking up new tasks)
/// This restores only Paused tasks (NOT Stopped) to their previous agent-active state.
/// Uses status history to find the pre-pause state and re-runs entry actions.
/// After restoring, triggers the scheduler to pick up waiting Ready tasks.
/// Phase 82: Optional project_id for per-project scoping.
///
/// ## Resume contract
/// - `Paused` → previous agent-active state (from `pause_reason.previous_status` metadata).
/// - Falls back to `status_history` if `pause_reason` metadata is absent.
/// - Skips tasks whose `previous_status` is not in `AGENT_ACTIVE_STATUSES` (defensive guard).
/// - Respects `execution_state.can_start_task()` — stops restoring once capacity is full.
/// - Calls `execute_entry_actions()` after transition to re-spawn the agent process.
///
/// ## Stopped vs Paused
/// `Stopped` tasks are intentionally excluded. `Stopped` is terminal — requires the user
/// to manually trigger `Retry` (→ `ready`) before the task can re-execute. Only `Paused`
/// (non-terminal) is auto-restored by resume.
#[tauri::command]
pub async fn resume_execution(
    project_id: Option<String>,
    active_project_state: State<'_, Arc<ActiveProjectState>>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app_state: State<'_, AppState>,
) -> Result<ExecutionCommandResponse, String> {
    ensure_resume_allowed(&app_state).await?;

    // Sync runtime quota with persisted project settings before can_start_task() loops
    let project_id = project_id.map(|id| ProjectId::from_string(id));
    let (effective_project_id, _max_concurrent) = sync_quota_from_project(
        project_id,
        &active_project_state,
        &execution_state,
        &app_state,
    )
    .await?;
    persist_execution_halt_mode(&app_state, ExecutionHaltMode::Running).await?;

    // Build transition service for proper state machine transitions
    let transition_service = TaskTransitionService::new(
        Arc::clone(&app_state.task_repo),
        Arc::clone(&app_state.task_dependency_repo),
        Arc::clone(&app_state.project_repo),
        Arc::clone(&app_state.chat_message_repo),
        Arc::clone(&app_state.chat_attachment_repo),
        Arc::clone(&app_state.chat_conversation_repo),
        Arc::clone(&app_state.agent_run_repo),
        Arc::clone(&app_state.ideation_session_repo),
        Arc::clone(&app_state.activity_event_repo),
        Arc::clone(&app_state.message_queue),
        Arc::clone(&app_state.running_agent_registry),
        Arc::clone(&execution_state),
        app_state.app_handle.clone(),
        Arc::clone(&app_state.memory_event_repo),
    )
    .with_execution_settings_repo(Arc::clone(&app_state.execution_settings_repo))
    .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo))
    .with_interactive_process_registry(Arc::clone(&app_state.interactive_process_registry));

    // Find all Paused tasks (scoped to project if specified) and restore them
    // Note: Stopped tasks are NOT restored - they require manual restart
    // Local counter tracks tasks queued for restoration during this loop.
    // We cannot use execution_state.can_start_task() here because: (a) the pause
    // flag is still set (cleared after the loop), and (b) running_count hasn't yet
    // been incremented for tasks whose entry actions fire asynchronously.
    let mut restoring_count: u32 = 0;
    let projects_to_process = if let Some(ref pid) = effective_project_id {
        match app_state.project_repo.get_by_id(pid).await {
            Ok(Some(project)) => vec![project],
            Ok(None) => return Err(format!("Project not found: {}", pid.as_str())),
            Err(e) => return Err(e.to_string()),
        }
    } else {
        app_state
            .project_repo
            .get_all()
            .await
            .map_err(|e| e.to_string())?
    };

    for project in projects_to_process {
        let tasks = app_state
            .task_repo
            .get_by_status(&project.id, InternalStatus::Paused)
            .await
            .map_err(|e| e.to_string())?;

        for task in tasks {
            // Determine restore status: prefer pause_reason metadata, fall back to status_history
            let restore_status = if let Some(reason) =
                crate::application::chat_service::PauseReason::from_task_metadata(
                    task.metadata.as_deref(),
                ) {
                match reason.previous_status().parse::<InternalStatus>() {
                    Ok(status) => status,
                    Err(_) => {
                        tracing::warn!(
                            task_id = task.id.as_str(),
                            previous_status = reason.previous_status(),
                            "Invalid previous_status in pause metadata, falling back to history"
                        );
                        InternalStatus::Executing // safe fallback
                    }
                }
            } else {
                // Fallback: find the pre-pause status from status history
                let status_history = match app_state.task_repo.get_status_history(&task.id).await {
                    Ok(history) => history,
                    Err(e) => {
                        tracing::warn!(
                            task_id = task.id.as_str(),
                            error = %e,
                            "Failed to get status history for resume"
                        );
                        continue;
                    }
                };

                let pause_transition = status_history
                    .iter()
                    .rev()
                    .find(|t| t.to == InternalStatus::Paused);

                match pause_transition {
                    Some(transition) => transition.from,
                    None => {
                        tracing::warn!(
                            task_id = task.id.as_str(),
                            "No pause transition found in history or metadata, cannot restore"
                        );
                        continue;
                    }
                }
            };

            // Validate that the restore status is a valid agent-active state
            if !AGENT_ACTIVE_STATUSES.contains(&restore_status) {
                tracing::warn!(
                    task_id = task.id.as_str(),
                    restore_status = ?restore_status,
                    "Pre-pause status is not agent-active, skipping restore"
                );
                continue;
            }

            // Check capacity using local counter (running_count not yet incremented
            // for tasks queued this loop; pause flag still set so can_start_task() → false).
            let current = execution_state.running_count() + restoring_count;
            if current >= execution_state.max_concurrent()
                || current >= execution_state.global_max_concurrent()
            {
                tracing::info!(
                    task_id = task.id.as_str(),
                    running = execution_state.running_count(),
                    restoring = restoring_count,
                    "Max concurrent reached, stopping pause recovery"
                );
                break;
            }

            // Transition back to the pre-pause status
            if let Err(e) = transition_service
                .transition_task(&task.id, restore_status)
                .await
            {
                tracing::warn!(
                    task_id = task.id.as_str(),
                    restore_status = ?restore_status,
                    error = %e,
                    "Failed to restore task from Paused"
                );
                continue;
            }

            // Re-run entry actions to respawn the agent
            // Fetch fresh task after transition
            if let Ok(Some(mut restored_task)) = app_state.task_repo.get_by_id(&task.id).await {
                // Clear pause_reason metadata on successful resume
                restored_task.metadata = Some(
                    crate::application::chat_service::PauseReason::clear_from_task_metadata(
                        restored_task.metadata.as_deref(),
                    ),
                );
                restored_task.touch();
                let _ = app_state.task_repo.update(&restored_task).await;

                transition_service
                    .execute_entry_actions(&task.id, &restored_task, restore_status)
                    .await;

                restoring_count += 1;
                tracing::info!(
                    task_id = task.id.as_str(),
                    restored_to = ?restore_status,
                    restoring_count,
                    "Successfully restored Paused task"
                );
            }
        }
    }

    // Clear the pause flag now that all paused tasks have been queued for restoration.
    // Doing this after the loop prevents the scheduler from racing with restoration
    // and prevents can_start_task() from returning false during the loop above.
    execution_state.resume();

    // Emit status_changed event for real-time UI update with projectId
    if let Some(ref handle) = app_state.app_handle {
        let _ = handle.emit(
            "execution:status_changed",
            serde_json::json!({
                "isPaused": execution_state.is_paused(),
                "haltMode": "running",
                "runningCount": execution_state.running_count(),
                "maxConcurrent": execution_state.max_concurrent(),
                "reason": "resumed",
                "projectId": effective_project_id.as_ref().map(|p| p.as_str()),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        );
    }

    // Trigger scheduler to pick up waiting Ready tasks
    let scheduler = Arc::new(
        TaskSchedulerService::new(
            Arc::clone(&execution_state),
            Arc::clone(&app_state.project_repo),
            Arc::clone(&app_state.task_repo),
            Arc::clone(&app_state.task_dependency_repo),
            Arc::clone(&app_state.chat_message_repo),
            Arc::clone(&app_state.chat_attachment_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.activity_event_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            Arc::clone(&app_state.memory_event_repo),
            app_state.app_handle.clone(),
        )
        .with_execution_settings_repo(Arc::clone(&app_state.execution_settings_repo))
        .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo))
        .with_interactive_process_registry(Arc::clone(&app_state.interactive_process_registry)),
    );
    scheduler.set_self_ref(Arc::clone(&scheduler) as Arc<dyn TaskScheduler>);
    // Set active project scope before scheduling to prevent cross-project scheduling
    scheduler
        .set_active_project(effective_project_id.clone())
        .await;
    scheduler.try_schedule_ready_tasks().await;

    if let Some(ref handle) = app_state.app_handle {
        let execution_state_arc = Arc::clone(execution_state.inner());
        let team_service = std::sync::Arc::new(crate::application::TeamService::new_with_repos(
            std::sync::Arc::new(TeamStateTracker::new()),
            handle.clone(),
            Arc::clone(&app_state.team_session_repo),
            Arc::clone(&app_state.team_message_repo),
        ));
        if let Err(error) = resume_paused_slot_consuming_queues_with_chat_service(
            effective_project_id.as_ref(),
            &app_state,
            &execution_state_arc,
            || {
                Arc::new(
                    ClaudeChatService::new(
                        Arc::clone(&app_state.chat_message_repo),
                        Arc::clone(&app_state.chat_attachment_repo),
                        Arc::clone(&app_state.artifact_repo),
                        Arc::clone(&app_state.chat_conversation_repo),
                        Arc::clone(&app_state.agent_run_repo),
                        Arc::clone(&app_state.project_repo),
                        Arc::clone(&app_state.task_repo),
                        Arc::clone(&app_state.task_dependency_repo),
                        Arc::clone(&app_state.ideation_session_repo),
                        Arc::clone(&app_state.activity_event_repo),
                        Arc::clone(&app_state.message_queue),
                        Arc::clone(&app_state.running_agent_registry),
                        Arc::clone(&app_state.memory_event_repo),
                    )
                    .with_app_handle(handle.clone())
                    .with_execution_state(Arc::clone(&execution_state_arc))
                    .with_execution_settings_repo(Arc::clone(&app_state.execution_settings_repo))
                    .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo))
                    .with_task_proposal_repo(Arc::clone(&app_state.task_proposal_repo))
                    .with_task_step_repo(Arc::clone(&app_state.task_step_repo))
                    .with_streaming_state_cache(app_state.streaming_state_cache.clone())
                    .with_interactive_process_registry(Arc::clone(
                        &app_state.interactive_process_registry,
                    ))
                    .with_review_repo(Arc::clone(&app_state.review_repo))
                    .with_team_service(Arc::clone(&team_service)),
                ) as Arc<dyn ChatService>
            },
        )
        .await
        {
            tracing::warn!(
                error = %error,
                "Failed to relaunch paused task/review/merge queues on resume"
            );
        }

        if let Err(error) = resume_paused_ideation_queues_with_chat_service(
            effective_project_id.as_ref(),
            &app_state,
            &execution_state_arc,
            |is_team_mode| {
                let mut service = ClaudeChatService::new(
                    Arc::clone(&app_state.chat_message_repo),
                    Arc::clone(&app_state.chat_attachment_repo),
                    Arc::clone(&app_state.artifact_repo),
                    Arc::clone(&app_state.chat_conversation_repo),
                    Arc::clone(&app_state.agent_run_repo),
                    Arc::clone(&app_state.project_repo),
                    Arc::clone(&app_state.task_repo),
                    Arc::clone(&app_state.task_dependency_repo),
                    Arc::clone(&app_state.ideation_session_repo),
                    Arc::clone(&app_state.activity_event_repo),
                    Arc::clone(&app_state.message_queue),
                    Arc::clone(&app_state.running_agent_registry),
                    Arc::clone(&app_state.memory_event_repo),
                )
                .with_app_handle(handle.clone())
                .with_execution_state(Arc::clone(&execution_state_arc))
                .with_execution_settings_repo(Arc::clone(&app_state.execution_settings_repo))
                .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo))
                .with_task_proposal_repo(Arc::clone(&app_state.task_proposal_repo))
                .with_task_step_repo(Arc::clone(&app_state.task_step_repo))
                .with_streaming_state_cache(app_state.streaming_state_cache.clone())
                .with_interactive_process_registry(Arc::clone(
                    &app_state.interactive_process_registry,
                ))
                .with_review_repo(Arc::clone(&app_state.review_repo))
                .with_team_service(Arc::clone(&team_service));
                if is_team_mode {
                    service = service.with_team_mode(true);
                }
                Arc::new(service) as Arc<dyn ChatService>
            },
        )
        .await
        {
            tracing::warn!(error = %error, "Failed to relaunch paused ideation queues on resume");
        }

        if let Err(error) = resume_paused_non_slot_chat_queues_with_chat_service(
            effective_project_id.as_ref(),
            &app_state,
            || {
                Arc::new(
                    ClaudeChatService::new(
                        Arc::clone(&app_state.chat_message_repo),
                        Arc::clone(&app_state.chat_attachment_repo),
                        Arc::clone(&app_state.artifact_repo),
                        Arc::clone(&app_state.chat_conversation_repo),
                        Arc::clone(&app_state.agent_run_repo),
                        Arc::clone(&app_state.project_repo),
                        Arc::clone(&app_state.task_repo),
                        Arc::clone(&app_state.task_dependency_repo),
                        Arc::clone(&app_state.ideation_session_repo),
                        Arc::clone(&app_state.activity_event_repo),
                        Arc::clone(&app_state.message_queue),
                        Arc::clone(&app_state.running_agent_registry),
                        Arc::clone(&app_state.memory_event_repo),
                    )
                    .with_app_handle(handle.clone())
                    .with_execution_state(Arc::clone(&execution_state_arc))
                    .with_execution_settings_repo(Arc::clone(&app_state.execution_settings_repo))
                    .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo))
                    .with_task_proposal_repo(Arc::clone(&app_state.task_proposal_repo))
                    .with_task_step_repo(Arc::clone(&app_state.task_step_repo))
                    .with_streaming_state_cache(app_state.streaming_state_cache.clone())
                    .with_interactive_process_registry(Arc::clone(
                        &app_state.interactive_process_registry,
                    ))
                    .with_review_repo(Arc::clone(&app_state.review_repo))
                    .with_team_service(Arc::clone(&team_service)),
                ) as Arc<dyn ChatService>
            },
        )
        .await
        {
            tracing::warn!(
                error = %error,
                "Failed to relaunch paused task/project chat queues on resume"
            );
        }
    }

    // Get current status
    let status = get_execution_status(
        effective_project_id.map(|p| p.as_str().to_string()),
        active_project_state,
        execution_state,
        app_state,
    )
    .await?;

    Ok(ExecutionCommandResponse {
        success: true,
        status,
    })
}

/// Stop execution (cancels current tasks and pauses)
/// This transitions all agent-active tasks to Stopped status via TransitionHandler.
/// Stopped is a terminal state requiring manual restart - tasks won't auto-resume.
/// The on_exit handlers will decrement the running count for each task.
/// Phase 82: Optional project_id for per-project scoping.
///
/// ## Stop vs Pause
/// | | `stop_execution` | `pause_execution` |
/// |---|---|---|
/// | Result state | `Stopped` (terminal) | `Paused` (non-terminal) |
/// | Auto-resume | ❌ No — user must retry | ✅ Yes — via `resume_execution` |
/// | Metadata written | None | `PauseReason::UserInitiated` |
/// | `running_count` | Decremented by `on_exit` | Decremented by `on_exit` |
/// | Restart path | `Retry` → `ready` → re-execute | `resume_execution` → previous state |
#[tauri::command]
pub async fn stop_execution(
    project_id: Option<String>,
    active_project_state: State<'_, Arc<ActiveProjectState>>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app_state: State<'_, AppState>,
) -> Result<ExecutionCommandResponse, String> {
    // Sync runtime quota with persisted project settings for consistency
    let project_id = project_id.map(|id| ProjectId::from_string(id));
    let (effective_project_id, _max_concurrent) = sync_quota_from_project(
        project_id,
        &active_project_state,
        &execution_state,
        &app_state,
    )
    .await?;

    // First pause to prevent new tasks from starting
    execution_state.pause();
    persist_execution_halt_mode(&app_state, ExecutionHaltMode::Stopped).await?;

    // Kill all running agent processes immediately via registry
    // This ensures agents are terminated even if transition fails
    app_state.running_agent_registry.stop_all().await;
    // Also clear all interactive process entries — their stdin pipes are now dead
    app_state.interactive_process_registry.clear().await;
    if let Err(error) = clear_paused_chat_queues(effective_project_id.as_ref(), &app_state).await {
        tracing::warn!(error = %error, "Failed to clear queued chat work during stop");
    }

    // Build transition service for proper state machine transitions
    let transition_service = TaskTransitionService::new(
        Arc::clone(&app_state.task_repo),
        Arc::clone(&app_state.task_dependency_repo),
        Arc::clone(&app_state.project_repo),
        Arc::clone(&app_state.chat_message_repo),
        Arc::clone(&app_state.chat_attachment_repo),
        Arc::clone(&app_state.chat_conversation_repo),
        Arc::clone(&app_state.agent_run_repo),
        Arc::clone(&app_state.ideation_session_repo),
        Arc::clone(&app_state.activity_event_repo),
        Arc::clone(&app_state.message_queue),
        Arc::clone(&app_state.running_agent_registry),
        Arc::clone(&execution_state),
        app_state.app_handle.clone(),
        Arc::clone(&app_state.memory_event_repo),
    )
    .with_execution_settings_repo(Arc::clone(&app_state.execution_settings_repo))
    .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo))
    .with_interactive_process_registry(Arc::clone(&app_state.interactive_process_registry));

    // Find all tasks in agent-active states (scoped to project if specified)
    let projects_to_process = if let Some(ref pid) = effective_project_id {
        match app_state.project_repo.get_by_id(pid).await {
            Ok(Some(project)) => vec![project],
            Ok(None) => return Err(format!("Project not found: {}", pid.as_str())),
            Err(e) => return Err(e.to_string()),
        }
    } else {
        app_state
            .project_repo
            .get_all()
            .await
            .map_err(|e| e.to_string())?
    };

    for project in projects_to_process {
        let tasks = app_state
            .task_repo
            .get_by_project(&project.id)
            .await
            .map_err(|e| e.to_string())?;

        for task in tasks {
            // Check if task is in an agent-active state
            if AGENT_ACTIVE_STATUSES.contains(&task.internal_status) {
                // Use TransitionHandler to transition to Stopped
                // Stopped is terminal - requires manual restart via Ready transition
                // This triggers on_exit handlers which decrement running count
                if let Err(e) = transition_service
                    .transition_task(&task.id, InternalStatus::Stopped)
                    .await
                {
                    tracing::warn!(
                        task_id = task.id.as_str(),
                        error = %e,
                        "Failed to transition task to Stopped during stop"
                    );
                }
            }
        }
    }

    // Note: running_count is decremented by on_exit handlers in TransitionHandler
    // No manual reset needed here

    // Emit status_changed event for real-time UI update with projectId
    // This reflects the final state after all tasks have been stopped
    if let Some(ref handle) = app_state.app_handle {
        let _ = handle.emit(
            "execution:status_changed",
            serde_json::json!({
                "isPaused": execution_state.is_paused(),
                "haltMode": "stopped",
                "runningCount": execution_state.running_count(),
                "maxConcurrent": execution_state.max_concurrent(),
                "reason": "stopped",
                "projectId": effective_project_id.as_ref().map(|p| p.as_str()),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        );
    }

    // Get current status
    let status = get_execution_status(
        effective_project_id.map(|p| p.as_str().to_string()),
        active_project_state,
        execution_state,
        app_state,
    )
    .await?;

    Ok(ExecutionCommandResponse {
        success: true,
        status,
    })
}

/// Recover a task execution after a stop request
///
/// Applies the recovery policy:
/// - If run completed → PendingReview
/// - Else → Ready
/// - If evidence conflicts → emit recovery:prompt
#[tauri::command]
pub async fn recover_task_execution(
    task_id: String,
    execution_state: State<'_, Arc<ExecutionState>>,
    app_state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<bool, String> {
    let task_id = crate::domain::entities::TaskId::from_string(task_id);
    let reconciler =
        build_reconciler_for_recovery(&app_state, Arc::clone(&execution_state), app);

    Ok(reconciler.recover_execution_stop(&task_id).await)
}

/// Resolve a recovery prompt by applying the selected action.
#[tauri::command]
pub async fn resolve_recovery_prompt(
    task_id: String,
    action: String,
    execution_state: State<'_, Arc<ExecutionState>>,
    app_state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<bool, String> {
    let task_id = crate::domain::entities::TaskId::from_string(task_id);
    let action = match action.as_str() {
        "restart" => UserRecoveryAction::Restart,
        "cancel" => UserRecoveryAction::Cancel,
        _ => return Err("Invalid recovery action".to_string()),
    };
    let reconciler =
        build_reconciler_for_recovery(&app_state, Arc::clone(&execution_state), app);

    let task = match app_state.task_repo.get_by_id(&task_id).await {
        Ok(Some(task)) => task,
        Ok(None) => return Ok(false),
        Err(e) => return Err(e.to_string()),
    };

    Ok(reconciler.apply_user_recovery_action(&task, action).await)
}

/// Set maximum concurrent tasks
/// When capacity increases, triggers the scheduler to pick up waiting Ready tasks.
#[tauri::command]
pub async fn set_max_concurrent(
    max: u32,
    active_project_state: State<'_, Arc<ActiveProjectState>>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app_state: State<'_, AppState>,
) -> Result<ExecutionCommandResponse, String> {
    let old_max = execution_state.max_concurrent();
    execution_state.set_max_concurrent(max);

    // Emit status_changed event for real-time UI update
    if let Some(ref handle) = app_state.app_handle {
        execution_state.emit_status_changed(handle, "max_concurrent_changed");
    }

    // If capacity increased, trigger scheduler to pick up waiting Ready tasks
    if max > old_max {
        // Get active project for scoped scheduling
        let active_project_id = active_project_state.get().await;
        schedule_ready_tasks_for_project(&app_state, Arc::clone(&execution_state), active_project_id)
            .await;
    }

    // Get current status
    let status =
        get_execution_status(None, active_project_state, execution_state, app_state).await?;

    Ok(ExecutionCommandResponse {
        success: true,
        status,
    })
}

/// Get execution settings from database
/// Phase 82: Optional project_id for per-project settings
/// - project_id = Some(id): returns settings for that project (falls back to global if none exist)
/// - project_id = None: returns global default settings
#[tauri::command]
pub async fn get_execution_settings(
    project_id: Option<String>,
    app_state: State<'_, AppState>,
) -> Result<ExecutionSettingsResponse, String> {
    let project_id = project_id.map(|id| ProjectId::from_string(id));
    let settings = app_state
        .execution_settings_repo
        .get_settings(project_id.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    Ok(ExecutionSettingsResponse::from(settings))
}

/// Update execution settings in database and sync ExecutionState
/// Phase 82: Optional project_id for per-project settings
/// - project_id = Some(id): updates/creates settings for that project
/// - project_id = None: updates global default settings
/// When max_concurrent_tasks changes:
/// - Updates the in-memory ExecutionState
/// - If capacity increased, triggers scheduler to pick up waiting Ready tasks
/// Emits settings:execution:updated event for UI updates
#[tauri::command]
pub async fn update_execution_settings(
    project_id: Option<String>,
    input: UpdateExecutionSettingsInput,
    execution_state: State<'_, Arc<ExecutionState>>,
    app_state: State<'_, AppState>,
) -> Result<ExecutionSettingsResponse, String> {
    let project_id = project_id.map(|id| ProjectId::from_string(id));
    let old_max = execution_state.max_concurrent();
    let new_max = input.max_concurrent_tasks;

    // Build domain settings from input
    let settings = ExecutionSettings {
        max_concurrent_tasks: input.max_concurrent_tasks,
        project_ideation_max: input.project_ideation_max,
        auto_commit: input.auto_commit,
        pause_on_failure: input.pause_on_failure,
    };

    // Persist to database
    let updated = app_state
        .execution_settings_repo
        .update_settings(project_id.as_ref(), &settings)
        .await
        .map_err(|e| e.to_string())?;

    // Sync ExecutionState if max_concurrent_tasks changed
    if new_max != old_max {
        execution_state.set_max_concurrent(new_max);

        // If capacity increased, trigger scheduler to pick up waiting Ready tasks
        if new_max > old_max {
            schedule_ready_tasks_for_project(
                &app_state,
                Arc::clone(&execution_state),
                project_id.clone(),
            )
            .await;
        }
    }

    // Emit settings:execution:updated event for UI updates (include projectId for per-project)
    if let Some(ref handle) = app_state.app_handle {
        let _ = handle.emit(
            "settings:execution:updated",
            serde_json::json!({
                "project_id": project_id.as_ref().map(|p| p.as_str()),
                "max_concurrent_tasks": updated.max_concurrent_tasks,
                "project_ideation_max": updated.project_ideation_max,
                "auto_commit": updated.auto_commit,
                "pause_on_failure": updated.pause_on_failure,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        );
    }

    Ok(ExecutionSettingsResponse::from(updated))
}

// ========================================
// Phase 82: Active Project Management
// ========================================

/// Set the active project for execution scoping.
/// Frontend should call this when switching projects.
/// Commands without explicit project_id will use this active project.
#[tauri::command]
pub async fn set_active_project(
    project_id: Option<String>,
    active_project_state: State<'_, Arc<ActiveProjectState>>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    let project_id = project_id.map(|id| ProjectId::from_string(id));

    // Validate project exists if ID provided
    if let Some(ref pid) = project_id {
        let exists = app_state
            .project_repo
            .get_by_id(pid)
            .await
            .map_err(|e| e.to_string())?
            .is_some();

        if !exists {
            return Err(format!("Project not found: {}", pid.as_str()));
        }
    }

    active_project_state.set(project_id.clone()).await;

    // Persist to DB so it survives app restarts
    app_state
        .app_state_repo
        .set_active_project(project_id.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    // Sync runtime quota immediately after switching active project
    let (_resolved_project_id, _max_concurrent) = sync_quota_from_project(
        project_id.clone(),
        &active_project_state,
        &execution_state,
        &app_state,
    )
    .await?;

    tracing::info!(
        project_id = ?project_id.as_ref().map(|p| p.as_str()),
        "Active project set (in-memory + DB)"
    );

    // Emit events for UI sync
    if let Some(ref handle) = app_state.app_handle {
        let _ = handle.emit(
            "execution:active_project_changed",
            serde_json::json!({
                "projectId": project_id.as_ref().map(|p| p.as_str()),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        );

        // Emit execution:status_changed after sync so UI updates quota instantly
        let _ = handle.emit(
            "execution:status_changed",
            serde_json::json!({
                "isPaused": execution_state.is_paused(),
                "runningCount": execution_state.running_count(),
                "maxConcurrent": execution_state.max_concurrent(),
                "globalMaxConcurrent": execution_state.global_max_concurrent(),
                "reason": "active_project_changed",
                "projectId": project_id.as_ref().map(|p| p.as_str()),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        );
    }

    Ok(())
}

/// Get the current active project ID.
/// Returns None if no project is active.
#[tauri::command]
pub async fn get_active_project(
    active_project_state: State<'_, Arc<ActiveProjectState>>,
) -> Result<Option<String>, String> {
    let project_id = active_project_state.get().await;
    Ok(project_id.map(|p| p.as_str().to_string()))
}

// ========================================
// Phase 82: Global Execution Settings
// ========================================

/// Get global execution settings (cross-project limits)
/// Returns the global_max_concurrent cap that limits total tasks across all projects
#[tauri::command]
pub async fn get_global_execution_settings(
    app_state: State<'_, AppState>,
) -> Result<GlobalExecutionSettingsResponse, String> {
    let settings = app_state
        .global_execution_settings_repo
        .get_settings()
        .await
        .map_err(|e| e.to_string())?;

    Ok(GlobalExecutionSettingsResponse::from(settings))
}

/// Update global execution settings (cross-project limits)
/// global_max_concurrent is capped at 50 (enforced by repository)
/// Syncs in-memory ExecutionState and triggers scheduler if capacity increased
#[tauri::command]
pub async fn update_global_execution_settings(
    input: UpdateGlobalExecutionSettingsInput,
    active_project_state: State<'_, Arc<ActiveProjectState>>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app_state: State<'_, AppState>,
) -> Result<GlobalExecutionSettingsResponse, String> {
    use crate::domain::execution::GlobalExecutionSettings;

    let old_global_max = execution_state.global_max_concurrent();

    let settings = GlobalExecutionSettings {
        global_max_concurrent: input.global_max_concurrent,
        global_ideation_max: input.global_ideation_max,
        allow_ideation_borrow_idle_execution: input.allow_ideation_borrow_idle_execution,
    };

    let updated = app_state
        .global_execution_settings_repo
        .update_settings(&settings)
        .await
        .map_err(|e| e.to_string())?;

    // Sync in-memory global cap
    execution_state.set_global_max_concurrent(updated.global_max_concurrent);
    execution_state.set_global_ideation_max(updated.global_ideation_max);
    execution_state
        .set_allow_ideation_borrow_idle_execution(updated.allow_ideation_borrow_idle_execution);

    // If global capacity increased, trigger scheduler to pick up waiting tasks
    if updated.global_max_concurrent > old_global_max {
        // Get active project for scoped scheduling
        let active_project_id = active_project_state.get().await;
        schedule_ready_tasks_for_project(&app_state, Arc::clone(&execution_state), active_project_id)
            .await;
    }

    // Emit event for UI sync
    if let Some(ref handle) = app_state.app_handle {
        let _ = handle.emit(
            "settings:global_execution:updated",
            serde_json::json!({
                "global_max_concurrent": updated.global_max_concurrent,
                "global_ideation_max": updated.global_ideation_max,
                "allow_ideation_borrow_idle_execution": updated.allow_ideation_borrow_idle_execution,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        );
    }

    Ok(GlobalExecutionSettingsResponse::from(updated))
}

// ========================================
// Running Processes Query
// ========================================

/// Get all currently running processes (tasks with active execution contexts)
///
/// Returns tasks found in the running agent registry (task_execution/review/merge)
/// with enriched data:
/// - Step progress via StepProgressSummary::from_steps()
/// - Elapsed time from task_state_history
/// - Trigger origin from metadata
/// - Branch name
#[tauri::command]
pub async fn get_running_processes(
    project_id: Option<String>,
    active_project_state: State<'_, Arc<ActiveProjectState>>,
    execution_state: State<'_, Arc<ExecutionState>>,
    state: State<'_, AppState>,
) -> Result<RunningProcessesResponse, String> {
    let effective_project_id = match project_id {
        Some(id) => Some(ProjectId::from_string(id)),
        None => active_project_state.get().await,
    };

    // Keep the registry clean so process rows reflect truly running agents.
    prune_stale_execution_registry_entries(&state, &execution_state).await;

    let mut processes = Vec::new();
    let mut ideation_sessions = Vec::new();
    let mut seen_task_ids = std::collections::HashSet::new();
    let mut seen_session_ids = std::collections::HashSet::new();
    let registry_entries = state.running_agent_registry.list_all().await;

    for (key, _) in registry_entries {
        let context_type = match ChatContextType::from_str(&key.context_type) {
            Ok(value) => value,
            Err(_) => continue,
        };

        // Collect ideation sessions separately
        if context_type == ChatContextType::Ideation {
            let session_id_str = key.context_id.clone();
            if !seen_session_ids.insert(session_id_str.clone()) {
                continue;
            }
            let session_id = IdeationSessionId(session_id_str.clone());
            if let Ok(Some(session)) = state.ideation_session_repo.get_by_id(&session_id).await {
                if let Some(pid) = &effective_project_id {
                    if session.project_id != *pid {
                        continue;
                    }
                }
                let now = chrono::Utc::now();
                let slot_key = format!("ideation/{}", session_id_str);
                let is_generating = !execution_state.is_interactive_idle(&slot_key);
                ideation_sessions.push(build_running_ideation_session(
                    session_id_str,
                    &session,
                    is_generating,
                    now,
                ));
            }
            continue;
        }

        // Only include task-based execution contexts in the process list
        if !matches!(
            context_type,
            ChatContextType::TaskExecution | ChatContextType::Review | ChatContextType::Merge
        ) {
            continue;
        }

        let task_id = TaskId::from_string(key.context_id);
        let task = match state.task_repo.get_by_id(&task_id).await {
            Ok(Some(task)) => task,
            _ => continue,
        };

        if let Some(pid) = &effective_project_id {
            if task.project_id != *pid {
                continue;
            }
        }

        // Extra guard against races between status transitions and registry updates.
        if !context_matches_running_status_for_gc(context_type, task.internal_status) {
            continue;
        }

        let task_id_str = task.id.as_str().to_string();
        if !seen_task_ids.insert(task_id_str.clone()) {
            continue;
        }

        // Get step progress
        let steps = state
            .task_step_repo
            .get_by_task(&task_id)
            .await
            .map_err(|e| e.to_string())?;

        let step_progress = if !steps.is_empty() {
            Some(StepProgressSummary::from_steps(&task_id, &steps))
        } else {
            None
        };

        // Get elapsed time from status history
        let history = state
            .task_repo
            .get_status_history(&task_id)
            .await
            .map_err(|e| e.to_string())?;

        let elapsed_seconds =
            elapsed_seconds_for_status(&history, task.internal_status, chrono::Utc::now());

        // Get trigger origin
        let trigger_origin = get_trigger_origin(&task);

        processes.push(build_running_process(
            &task,
            step_progress,
            elapsed_seconds,
            trigger_origin,
        ));
    }

    Ok(RunningProcessesResponse {
        processes,
        ideation_sessions,
    })
}


/// Smart resume for stopped tasks.
///
/// Restarts a task that was stopped mid-execution, using the captured stop metadata
/// to determine the appropriate resume behavior:
///
/// - **Direct**: Resume directly to the original state (Executing, ReExecuting, Reviewing, etc.)
/// - **Validated**: Validate git state before resuming (Merging, PendingMerge, etc.)
/// - **Redirect**: Resume to successor state (QaPassed→PendingReview, RevisionNeeded→ReExecuting)
///
/// # Arguments
/// * `task_id` - The ID of the task to restart
/// * `force` - If true, skip validation (use with caution)
///
/// # Returns
/// * `RestartResult::Success` - Task was restarted successfully
/// * `RestartResult::ValidationFailed` - Validation failed with warnings
#[tauri::command]
pub async fn restart_task(
    task_id: String,
    force: bool,
    note: Option<String>,
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
) -> Result<RestartResult, String> {
    use crate::domain::state_machine::transition_handler::metadata_builder::{
        build_restart_metadata, parse_stop_metadata,
    };

    let task_id = TaskId::from_string(task_id);

    // 1. Get the task
    let task = state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Task not found: {}", task_id.as_str()))?;

    // 2. Verify task is in Stopped status
    if task.internal_status != InternalStatus::Stopped {
        return Err(format!(
            "Task is not in Stopped status (current: {})",
            task.internal_status.as_str()
        ));
    }

    // 3. Parse stop metadata
    let stop_metadata = parse_stop_metadata(task.metadata.as_deref())
        .ok_or_else(|| "Task has no stop metadata - cannot smart resume".to_string())?;

    let stopped_from_status = stop_metadata.parse_from_status().ok_or_else(|| {
        format!(
            "Invalid stopped_from_status: {}",
            stop_metadata.stopped_from_status
        )
    })?;

    tracing::info!(
        task_id = task_id.as_str(),
        stopped_from = stopped_from_status.as_str(),
        reason = ?stop_metadata.stop_reason,
        "Smart restarting task"
    );

    // 4. Categorize the resume state
    let categorized = categorize_resume_state(stopped_from_status);

    // 5. For Validated category, run validation (unless forced)
    if categorized.category == ResumeCategory::Validated && !force {
        let validation_result = validate_resume(&task, &state).await;
        if !validation_result.passed {
            return Ok(RestartResult::ValidationFailed {
                warnings: validation_result.warnings,
                stopped_from_status: stopped_from_status.as_str().to_string(),
            });
        }
    }

    // 6. Build transition service
    let transition_service =
        build_transition_service_for_recovery(&state, Arc::clone(&execution_state));

    // 7. Transition to target status: clear stop metadata and optionally store restart_note
    let restart_metadata = build_restart_metadata(note.as_deref());
    let updated_task = transition_service
        .transition_task_with_metadata(&task_id, categorized.target_status, Some(restart_metadata))
        .await
        .map_err(|e| e.to_string())?;

    tracing::info!(
        task_id = task_id.as_str(),
        category = ?categorized.category,
        target = categorized.target_status.as_str(),
        "Task restarted successfully"
    );

    // 8. Emit lifecycle event
    if let Some(ref app) = state.app_handle {
        let _ = app.emit(
            "task:restarted",
            serde_json::json!({
                "taskId": updated_task.id.as_str(),
                "projectId": updated_task.project_id.as_str(),
                "resumedToStatus": categorized.target_status.as_str(),
                "stoppedFromStatus": stopped_from_status.as_str(),
                "category": categorized.category,
                "stopReason": stop_metadata.stop_reason,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        );
    }

    // 9. Return success result
    // Serialize task to JSON Value for flexible response
    let task_json = serde_json::to_value(&updated_task).map_err(|e| e.to_string())?;

    Ok(RestartResult::Success {
        task: task_json,
        category: categorized.category,
        resumed_to_status: categorized.target_status.as_str().to_string(),
    })
}


#[cfg(test)]
mod tests;
