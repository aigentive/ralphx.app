// Tauri commands for execution control
// Manages per-project execution state: pause, resume, stop
// Phase 82: Project-scoped execution with optional project_id parameters

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime, State};
use tokio::sync::RwLock;

use crate::application::{
    AppState, ReconciliationRunner, TaskSchedulerService, TaskTransitionService,
};
use crate::application::reconciliation::UserRecoveryAction;
use crate::domain::entities::{InternalStatus, ProjectId};
use crate::domain::execution::ExecutionSettings;
use crate::domain::state_machine::services::TaskScheduler;

/// Statuses where an agent is actively running.
/// Tasks in these states need to be cancelled when stop is called,
/// and resumed when the app restarts.
///
/// Used by:
/// - `stop_execution` command to find tasks to cancel
/// - `StartupJobRunner` to find tasks to resume on app restart
pub const AGENT_ACTIVE_STATUSES: &[InternalStatus] = &[
    InternalStatus::Executing,
    InternalStatus::QaRefining,
    InternalStatus::QaTesting,
    InternalStatus::Reviewing,
    InternalStatus::ReExecuting,
    InternalStatus::Merging, // spawns merger agent
];

/// States that have automatic transitions on entry.
/// Tasks stuck in these states on startup should have their entry actions
/// re-triggered to complete the auto-transition.
///
/// Used by:
/// - `StartupJobRunner` to find tasks needing auto-transition recovery
pub const AUTO_TRANSITION_STATES: &[InternalStatus] = &[
    InternalStatus::QaPassed,       // → PendingReview
    InternalStatus::PendingReview,  // → Reviewing (spawns reviewer)
    InternalStatus::RevisionNeeded, // → ReExecuting (spawns worker)
    InternalStatus::Approved,       // → PendingMerge (programmatic merge)
    InternalStatus::PendingMerge,   // attempt_programmatic_merge() (→ Merged or → Merging)
];

// ========================================
// Phase 82: Active Project State
// ========================================

/// Tracks the currently active project for execution scoping.
/// Commands without explicit project_id use the active project.
/// Phase 90: Simple RwLock — DB persistence eliminates the startup race condition.
pub struct ActiveProjectState {
    /// The currently active project, if any
    current: RwLock<Option<ProjectId>>,
}

impl std::fmt::Debug for ActiveProjectState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActiveProjectState")
            .field("current", &self.current)
            .finish()
    }
}

impl Default for ActiveProjectState {
    fn default() -> Self {
        Self::new()
    }
}

impl ActiveProjectState {
    /// Create a new ActiveProjectState with no active project
    pub fn new() -> Self {
        Self {
            current: RwLock::new(None),
        }
    }

    /// Get the current active project ID
    pub async fn get(&self) -> Option<ProjectId> {
        self.current.read().await.clone()
    }

    /// Set the active project
    pub async fn set(&self, project_id: Option<ProjectId>) {
        *self.current.write().await = project_id;
    }
}

/// Global execution state managed atomically for thread safety
pub struct ExecutionState {
    /// Whether execution is paused (stops picking up new tasks)
    is_paused: AtomicBool,
    /// Number of currently running tasks
    running_count: AtomicU32,
    /// Maximum concurrent tasks allowed (per-project)
    max_concurrent: AtomicU32,
    /// Global maximum concurrent tasks across ALL projects (Phase 82)
    /// Default 20, hard cap 50. Enforced alongside per-project max.
    global_max_concurrent: AtomicU32,
}

impl ExecutionState {
    /// Create a new ExecutionState with defaults
    pub fn new() -> Self {
        Self {
            is_paused: AtomicBool::new(false),
            running_count: AtomicU32::new(0),
            max_concurrent: AtomicU32::new(2),
            global_max_concurrent: AtomicU32::new(20),
        }
    }

    /// Create ExecutionState with custom max concurrent
    pub fn with_max_concurrent(max: u32) -> Self {
        Self {
            is_paused: AtomicBool::new(false),
            running_count: AtomicU32::new(0),
            max_concurrent: AtomicU32::new(max),
            global_max_concurrent: AtomicU32::new(20),
        }
    }

    /// Check if execution is paused
    pub fn is_paused(&self) -> bool {
        self.is_paused.load(Ordering::SeqCst)
    }

    /// Pause execution (stops picking up new tasks)
    pub fn pause(&self) {
        self.is_paused.store(true, Ordering::SeqCst);
    }

    /// Resume execution
    pub fn resume(&self) {
        self.is_paused.store(false, Ordering::SeqCst);
    }

    /// Get current running task count
    pub fn running_count(&self) -> u32 {
        self.running_count.load(Ordering::SeqCst)
    }

    /// Increment running count (when a task starts)
    pub fn increment_running(&self) -> u32 {
        self.running_count.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Decrement running count (when a task completes)
    pub fn decrement_running(&self) -> u32 {
        let prev = self.running_count.fetch_sub(1, Ordering::SeqCst);
        if prev == 0 {
            // Prevent underflow
            self.running_count.store(0, Ordering::SeqCst);
            0
        } else {
            prev - 1
        }
    }

    /// Force-set running count (used for reconciliation with registry)
    pub fn set_running_count(&self, count: u32) {
        self.running_count.store(count, Ordering::SeqCst);
    }

    /// Get max concurrent tasks (per-project)
    pub fn max_concurrent(&self) -> u32 {
        self.max_concurrent.load(Ordering::SeqCst)
    }

    /// Set max concurrent tasks (per-project)
    pub fn set_max_concurrent(&self, max: u32) {
        self.max_concurrent.store(max, Ordering::SeqCst);
    }

    /// Get global max concurrent tasks (Phase 82)
    pub fn global_max_concurrent(&self) -> u32 {
        self.global_max_concurrent.load(Ordering::SeqCst)
    }

    /// Set global max concurrent tasks (Phase 82)
    /// Clamped to [1, 50] range
    pub fn set_global_max_concurrent(&self, max: u32) {
        let clamped = max.clamp(1, 50);
        self.global_max_concurrent.store(clamped, Ordering::SeqCst);
    }

    /// Check if we can start a new task
    /// Enforces both per-project max and global cap (Phase 82)
    pub fn can_start_task(&self) -> bool {
        if self.is_paused() {
            return false;
        }
        let running = self.running_count();
        running < self.max_concurrent() && running < self.global_max_concurrent()
    }

    /// Emit execution:status_changed event with current state
    pub fn emit_status_changed<R: Runtime>(&self, handle: &AppHandle<R>, reason: &str) {
        let _ = handle.emit(
            "execution:status_changed",
            serde_json::json!({
                "isPaused": self.is_paused(),
                "runningCount": self.running_count(),
                "maxConcurrent": self.max_concurrent(),
                "globalMaxConcurrent": self.global_max_concurrent(),
                "reason": reason,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        );
    }
}

impl Default for ExecutionState {
    fn default() -> Self {
        Self::new()
    }
}

/// Response for execution status queries
#[derive(Debug, Serialize)]
pub struct ExecutionStatusResponse {
    /// Whether execution is paused
    pub is_paused: bool,
    /// Number of currently running tasks
    pub running_count: u32,
    /// Maximum concurrent tasks allowed (per-project)
    pub max_concurrent: u32,
    /// Global maximum concurrent tasks across all projects (Phase 82)
    pub global_max_concurrent: u32,
    /// Number of tasks queued (ready to execute)
    pub queued_count: u32,
    /// Whether new tasks can be started
    pub can_start_task: bool,
}

/// Response for pause/resume/stop commands
#[derive(Debug, Serialize)]
pub struct ExecutionCommandResponse {
    /// Whether the command succeeded
    pub success: bool,
    /// Current execution status after the command
    pub status: ExecutionStatusResponse,
}

/// Response for execution settings queries
#[derive(Debug, Serialize)]
pub struct ExecutionSettingsResponse {
    /// Maximum number of concurrent tasks
    pub max_concurrent_tasks: u32,
    /// Whether to auto-commit changes after successful task completion
    pub auto_commit: bool,
    /// Whether to pause execution when a task fails
    pub pause_on_failure: bool,
}

impl From<ExecutionSettings> for ExecutionSettingsResponse {
    fn from(settings: ExecutionSettings) -> Self {
        Self {
            max_concurrent_tasks: settings.max_concurrent_tasks,
            auto_commit: settings.auto_commit,
            pause_on_failure: settings.pause_on_failure,
        }
    }
}

/// Input for updating execution settings
#[derive(Debug, Deserialize)]
pub struct UpdateExecutionSettingsInput {
    /// Maximum number of concurrent tasks
    pub max_concurrent_tasks: u32,
    /// Whether to auto-commit changes after successful task completion
    pub auto_commit: bool,
    /// Whether to pause execution when a task fails
    pub pause_on_failure: bool,
}

/// Response for global execution settings queries
/// Phase 82: Global concurrency cap across all projects
#[derive(Debug, Serialize)]
pub struct GlobalExecutionSettingsResponse {
    /// Maximum total concurrent tasks across ALL projects
    pub global_max_concurrent: u32,
}

impl From<crate::domain::execution::GlobalExecutionSettings> for GlobalExecutionSettingsResponse {
    fn from(settings: crate::domain::execution::GlobalExecutionSettings) -> Self {
        Self {
            global_max_concurrent: settings.global_max_concurrent,
        }
    }
}

/// Input for updating global execution settings
#[derive(Debug, Deserialize)]
pub struct UpdateGlobalExecutionSettingsInput {
    /// Maximum total concurrent tasks across ALL projects (max: 50)
    pub global_max_concurrent: u32,
}

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
    // Determine effective project_id: explicit param > active project > all projects
    let effective_project_id = match project_id {
        Some(id) => Some(ProjectId::from_string(id)),
        None => active_project_state.get().await,
    };

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

    // Use running agent registry as source of truth for active processes.
    // This avoids inflated counts from stuck task statuses.
    let registry_count = app_state.running_agent_registry.list_all().await.len() as u32;

    // Sync in-memory count with registry so downstream logic stays consistent.
    execution_state.set_running_count(registry_count);

    let running_count = registry_count;

    let max_concurrent = execution_state.max_concurrent();
    let global_max = execution_state.global_max_concurrent();

    Ok(ExecutionStatusResponse {
        is_paused: execution_state.is_paused(),
        running_count,
        max_concurrent,
        global_max_concurrent: global_max,
        queued_count,
        can_start_task: !execution_state.is_paused() && running_count < max_concurrent && running_count < global_max,
    })
}

/// Pause execution (stops picking up new tasks and transitions running tasks to Paused)
/// This transitions all agent-active tasks to Paused status via TransitionHandler.
/// Paused is NOT terminal - tasks can be auto-restored on resume using status history.
/// The on_exit handlers will decrement the running count for each task.
/// Phase 82: Optional project_id for per-project scoping.
#[tauri::command]
pub async fn pause_execution(
    project_id: Option<String>,
    active_project_state: State<'_, Arc<ActiveProjectState>>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app_state: State<'_, AppState>,
) -> Result<ExecutionCommandResponse, String> {
    // Determine effective project_id: explicit param > active project > all projects
    let effective_project_id = match project_id {
        Some(id) => Some(ProjectId::from_string(id)),
        None => active_project_state.get().await,
    };

    // First pause to prevent new tasks from starting
    execution_state.pause();

    // Kill all running agent processes immediately via registry
    // This ensures agents are terminated even if transition fails
    app_state.running_agent_registry.stop_all().await;

    // Build transition service for proper state machine transitions
    let transition_service = TaskTransitionService::new(
        Arc::clone(&app_state.task_repo),
        Arc::clone(&app_state.task_dependency_repo),
        Arc::clone(&app_state.project_repo),
        Arc::clone(&app_state.chat_message_repo),
        Arc::clone(&app_state.chat_conversation_repo),
        Arc::clone(&app_state.agent_run_repo),
        Arc::clone(&app_state.ideation_session_repo),
        Arc::clone(&app_state.activity_event_repo),
        Arc::clone(&app_state.message_queue),
        Arc::clone(&app_state.running_agent_registry),
        Arc::clone(&execution_state),
        app_state.app_handle.clone(),
    )
    .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo));

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
#[tauri::command]
pub async fn resume_execution(
    project_id: Option<String>,
    active_project_state: State<'_, Arc<ActiveProjectState>>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app_state: State<'_, AppState>,
) -> Result<ExecutionCommandResponse, String> {
    // Determine effective project_id: explicit param > active project > all projects
    let effective_project_id = match project_id {
        Some(id) => Some(ProjectId::from_string(id)),
        None => active_project_state.get().await,
    };

    // Clear the pause flag first
    execution_state.resume();

    // Build transition service for proper state machine transitions
    let transition_service = TaskTransitionService::new(
        Arc::clone(&app_state.task_repo),
        Arc::clone(&app_state.task_dependency_repo),
        Arc::clone(&app_state.project_repo),
        Arc::clone(&app_state.chat_message_repo),
        Arc::clone(&app_state.chat_conversation_repo),
        Arc::clone(&app_state.agent_run_repo),
        Arc::clone(&app_state.ideation_session_repo),
        Arc::clone(&app_state.activity_event_repo),
        Arc::clone(&app_state.message_queue),
        Arc::clone(&app_state.running_agent_registry),
        Arc::clone(&execution_state),
        app_state.app_handle.clone(),
    )
    .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo));

    // Find all Paused tasks (scoped to project if specified) and restore them
    // Note: Stopped tasks are NOT restored - they require manual restart
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
            // Find the pre-pause status from status history
            // Look for the last transition where to == Paused, restore to `from` status
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

            // Find the transition that moved to Paused (most recent)
            let pause_transition = status_history
                .iter()
                .rev()
                .find(|t| t.to == InternalStatus::Paused);

            let restore_status = match pause_transition {
                Some(transition) => transition.from,
                None => {
                    tracing::warn!(
                        task_id = task.id.as_str(),
                        "No pause transition found in history, cannot restore"
                    );
                    continue;
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

            // Check if we can start another task (respect max_concurrent)
            if !execution_state.can_start_task() {
                tracing::info!(
                    task_id = task.id.as_str(),
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
            if let Ok(Some(restored_task)) = app_state.task_repo.get_by_id(&task.id).await {
                transition_service
                    .execute_entry_actions(&task.id, &restored_task, restore_status)
                    .await;

                tracing::info!(
                    task_id = task.id.as_str(),
                    restored_to = ?restore_status,
                    "Successfully restored Paused task"
                );
            }
        }
    }

    // Emit status_changed event for real-time UI update with projectId
    if let Some(ref handle) = app_state.app_handle {
        let _ = handle.emit(
            "execution:status_changed",
            serde_json::json!({
                "isPaused": execution_state.is_paused(),
                "runningCount": execution_state.running_count(),
                "maxConcurrent": execution_state.max_concurrent(),
                "reason": "resumed",
                "projectId": effective_project_id.as_ref().map(|p| p.as_str()),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        );
    }

    // Trigger scheduler to pick up waiting Ready tasks
    let scheduler = Arc::new(TaskSchedulerService::new(
        Arc::clone(&execution_state),
        Arc::clone(&app_state.project_repo),
        Arc::clone(&app_state.task_repo),
        Arc::clone(&app_state.task_dependency_repo),
        Arc::clone(&app_state.chat_message_repo),
        Arc::clone(&app_state.chat_conversation_repo),
        Arc::clone(&app_state.agent_run_repo),
        Arc::clone(&app_state.ideation_session_repo),
        Arc::clone(&app_state.activity_event_repo),
        Arc::clone(&app_state.message_queue),
        Arc::clone(&app_state.running_agent_registry),
        app_state.app_handle.clone(),
    )
    .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo)));
    scheduler.set_self_ref(Arc::clone(&scheduler) as Arc<dyn TaskScheduler>);
    scheduler.try_schedule_ready_tasks().await;

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
#[tauri::command]
pub async fn stop_execution(
    project_id: Option<String>,
    active_project_state: State<'_, Arc<ActiveProjectState>>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app_state: State<'_, AppState>,
) -> Result<ExecutionCommandResponse, String> {
    // Determine effective project_id: explicit param > active project > all projects
    let effective_project_id = match project_id {
        Some(id) => Some(ProjectId::from_string(id)),
        None => active_project_state.get().await,
    };

    // First pause to prevent new tasks from starting
    execution_state.pause();

    // Kill all running agent processes immediately via registry
    // This ensures agents are terminated even if transition fails
    app_state.running_agent_registry.stop_all().await;

    // Build transition service for proper state machine transitions
    let transition_service = TaskTransitionService::new(
        Arc::clone(&app_state.task_repo),
        Arc::clone(&app_state.task_dependency_repo),
        Arc::clone(&app_state.project_repo),
        Arc::clone(&app_state.chat_message_repo),
        Arc::clone(&app_state.chat_conversation_repo),
        Arc::clone(&app_state.agent_run_repo),
        Arc::clone(&app_state.ideation_session_repo),
        Arc::clone(&app_state.activity_event_repo),
        Arc::clone(&app_state.message_queue),
        Arc::clone(&app_state.running_agent_registry),
        Arc::clone(&execution_state),
        app_state.app_handle.clone(),
    )
    .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo));

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

    let transition_service = Arc::new(TaskTransitionService::new(
        Arc::clone(&app_state.task_repo),
        Arc::clone(&app_state.task_dependency_repo),
        Arc::clone(&app_state.project_repo),
        Arc::clone(&app_state.chat_message_repo),
        Arc::clone(&app_state.chat_conversation_repo),
        Arc::clone(&app_state.agent_run_repo),
        Arc::clone(&app_state.ideation_session_repo),
        Arc::clone(&app_state.activity_event_repo),
        Arc::clone(&app_state.message_queue),
        Arc::clone(&app_state.running_agent_registry),
        Arc::clone(&execution_state),
        app_state.app_handle.clone(),
    )
    .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo)));

    let reconciler = ReconciliationRunner::new(
        Arc::clone(&app_state.task_repo),
        Arc::clone(&app_state.task_dependency_repo),
        Arc::clone(&app_state.project_repo),
        Arc::clone(&app_state.chat_conversation_repo),
        Arc::clone(&app_state.chat_message_repo),
        Arc::clone(&app_state.ideation_session_repo),
        Arc::clone(&app_state.activity_event_repo),
        Arc::clone(&app_state.message_queue),
        Arc::clone(&app_state.running_agent_registry),
        Arc::clone(&app_state.agent_run_repo),
        transition_service,
        Arc::clone(&execution_state),
        Some(app),
    )
    .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo));

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

    let transition_service = Arc::new(TaskTransitionService::new(
        Arc::clone(&app_state.task_repo),
        Arc::clone(&app_state.task_dependency_repo),
        Arc::clone(&app_state.project_repo),
        Arc::clone(&app_state.chat_message_repo),
        Arc::clone(&app_state.chat_conversation_repo),
        Arc::clone(&app_state.agent_run_repo),
        Arc::clone(&app_state.ideation_session_repo),
        Arc::clone(&app_state.activity_event_repo),
        Arc::clone(&app_state.message_queue),
        Arc::clone(&app_state.running_agent_registry),
        Arc::clone(&execution_state),
        app_state.app_handle.clone(),
    )
    .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo)));

    let reconciler = ReconciliationRunner::new(
        Arc::clone(&app_state.task_repo),
        Arc::clone(&app_state.task_dependency_repo),
        Arc::clone(&app_state.project_repo),
        Arc::clone(&app_state.chat_conversation_repo),
        Arc::clone(&app_state.chat_message_repo),
        Arc::clone(&app_state.ideation_session_repo),
        Arc::clone(&app_state.activity_event_repo),
        Arc::clone(&app_state.message_queue),
        Arc::clone(&app_state.running_agent_registry),
        Arc::clone(&app_state.agent_run_repo),
        transition_service,
        Arc::clone(&execution_state),
        Some(app),
    )
    .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo));

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
        let scheduler = Arc::new(TaskSchedulerService::new(
            Arc::clone(&execution_state),
            Arc::clone(&app_state.project_repo),
            Arc::clone(&app_state.task_repo),
            Arc::clone(&app_state.task_dependency_repo),
            Arc::clone(&app_state.chat_message_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.activity_event_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            app_state.app_handle.clone(),
        )
        .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo)));
        scheduler.set_self_ref(Arc::clone(&scheduler) as Arc<dyn TaskScheduler>);
        scheduler.try_schedule_ready_tasks().await;
    }

    // Get current status
    let status = get_execution_status(None, active_project_state, execution_state, app_state).await?;

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
            let scheduler = Arc::new(TaskSchedulerService::new(
                Arc::clone(&execution_state),
                Arc::clone(&app_state.project_repo),
                Arc::clone(&app_state.task_repo),
                Arc::clone(&app_state.task_dependency_repo),
                Arc::clone(&app_state.chat_message_repo),
                Arc::clone(&app_state.chat_conversation_repo),
                Arc::clone(&app_state.agent_run_repo),
                Arc::clone(&app_state.ideation_session_repo),
                Arc::clone(&app_state.activity_event_repo),
                Arc::clone(&app_state.message_queue),
                Arc::clone(&app_state.running_agent_registry),
                app_state.app_handle.clone(),
            )
            .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo)));
            scheduler.set_self_ref(Arc::clone(&scheduler) as Arc<dyn TaskScheduler>);
            scheduler.try_schedule_ready_tasks().await;
        }
    }

    // Emit settings:execution:updated event for UI updates (include projectId for per-project)
    if let Some(ref handle) = app_state.app_handle {
        let _ = handle.emit(
            "settings:execution:updated",
            serde_json::json!({
                "project_id": project_id.as_ref().map(|p| p.as_str()),
                "max_concurrent_tasks": updated.max_concurrent_tasks,
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

    tracing::info!(
        project_id = ?project_id.as_ref().map(|p| p.as_str()),
        "Active project set (in-memory + DB)"
    );

    // Emit event for UI sync
    if let Some(ref handle) = app_state.app_handle {
        let _ = handle.emit(
            "execution:active_project_changed",
            serde_json::json!({
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
    execution_state: State<'_, Arc<ExecutionState>>,
    app_state: State<'_, AppState>,
) -> Result<GlobalExecutionSettingsResponse, String> {
    use crate::domain::execution::GlobalExecutionSettings;

    let old_global_max = execution_state.global_max_concurrent();

    let settings = GlobalExecutionSettings {
        global_max_concurrent: input.global_max_concurrent,
    };

    let updated = app_state
        .global_execution_settings_repo
        .update_settings(&settings)
        .await
        .map_err(|e| e.to_string())?;

    // Sync in-memory global cap
    execution_state.set_global_max_concurrent(updated.global_max_concurrent);

    // If global capacity increased, trigger scheduler to pick up waiting tasks
    if updated.global_max_concurrent > old_global_max {
        let scheduler = Arc::new(TaskSchedulerService::new(
            Arc::clone(&execution_state),
            Arc::clone(&app_state.project_repo),
            Arc::clone(&app_state.task_repo),
            Arc::clone(&app_state.task_dependency_repo),
            Arc::clone(&app_state.chat_message_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.activity_event_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            app_state.app_handle.clone(),
        )
        .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo)));
        scheduler.set_self_ref(Arc::clone(&scheduler) as Arc<dyn TaskScheduler>);
        scheduler.try_schedule_ready_tasks().await;
    }

    // Emit event for UI sync
    if let Some(ref handle) = app_state.app_handle {
        let _ = handle.emit(
            "settings:global_execution:updated",
            serde_json::json!({
                "global_max_concurrent": updated.global_max_concurrent,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        );
    }

    Ok(GlobalExecutionSettingsResponse::from(updated))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // ========================================
    // ExecutionState Unit Tests
    // ========================================

    #[test]
    fn test_execution_state_new() {
        let state = ExecutionState::new();
        assert!(!state.is_paused());
        assert_eq!(state.running_count(), 0);
        assert_eq!(state.max_concurrent(), 2);
    }

    #[test]
    fn test_execution_state_with_max_concurrent() {
        let state = ExecutionState::with_max_concurrent(5);
        assert_eq!(state.max_concurrent(), 5);
    }

    #[test]
    fn test_execution_state_pause_resume() {
        let state = ExecutionState::new();

        assert!(!state.is_paused());

        state.pause();
        assert!(state.is_paused());

        state.resume();
        assert!(!state.is_paused());
    }

    #[test]
    fn test_execution_state_running_count() {
        let state = ExecutionState::new();

        assert_eq!(state.running_count(), 0);

        let count = state.increment_running();
        assert_eq!(count, 1);
        assert_eq!(state.running_count(), 1);

        let count = state.increment_running();
        assert_eq!(count, 2);
        assert_eq!(state.running_count(), 2);

        let count = state.decrement_running();
        assert_eq!(count, 1);
        assert_eq!(state.running_count(), 1);
    }

    #[test]
    fn test_execution_state_decrement_no_underflow() {
        let state = ExecutionState::new();

        // Should not underflow
        let count = state.decrement_running();
        assert_eq!(count, 0);
        assert_eq!(state.running_count(), 0);
    }

    #[test]
    fn test_execution_state_set_max_concurrent() {
        let state = ExecutionState::new();

        state.set_max_concurrent(10);
        assert_eq!(state.max_concurrent(), 10);
    }

    #[test]
    fn test_execution_state_can_start_task() {
        let state = ExecutionState::with_max_concurrent(2);

        // Initially can start
        assert!(state.can_start_task());

        // After pausing, cannot start
        state.pause();
        assert!(!state.can_start_task());

        // After resuming, can start again
        state.resume();
        assert!(state.can_start_task());

        // Fill up to max concurrent
        state.increment_running();
        state.increment_running();
        assert!(!state.can_start_task());

        // After one completes, can start again
        state.decrement_running();
        assert!(state.can_start_task());
    }

    #[test]
    fn test_execution_state_global_max_concurrent() {
        let state = ExecutionState::new();

        // Default global max is 20
        assert_eq!(state.global_max_concurrent(), 20);

        // Set global max
        state.set_global_max_concurrent(10);
        assert_eq!(state.global_max_concurrent(), 10);

        // Clamped to max 50
        state.set_global_max_concurrent(100);
        assert_eq!(state.global_max_concurrent(), 50);

        // Clamped to min 1
        state.set_global_max_concurrent(0);
        assert_eq!(state.global_max_concurrent(), 1);
    }

    #[test]
    fn test_execution_state_can_start_task_respects_global_cap() {
        let state = ExecutionState::with_max_concurrent(10);
        // Set global cap lower than per-project max
        state.set_global_max_concurrent(3);

        assert!(state.can_start_task());

        // Fill up to global cap
        state.increment_running();
        state.increment_running();
        state.increment_running();

        // At global cap (3), per-project max (10) still has room, but global blocks
        assert!(!state.can_start_task());

        // Free a slot
        state.decrement_running();
        assert!(state.can_start_task());
    }

    #[test]
    fn test_execution_state_can_start_task_per_project_cap_lower() {
        let state = ExecutionState::with_max_concurrent(2);
        // Global cap is higher than per-project max
        state.set_global_max_concurrent(20);

        state.increment_running();
        state.increment_running();

        // At per-project cap (2), global cap (20) still has room, but per-project blocks
        assert!(!state.can_start_task());
    }

    #[test]
    fn test_execution_state_thread_safe() {
        use std::thread;

        let state = Arc::new(ExecutionState::new());
        let mut handles = vec![];

        // Spawn threads that increment and decrement
        for _ in 0..10 {
            let state_clone = Arc::clone(&state);
            handles.push(thread::spawn(move || {
                state_clone.increment_running();
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(state.running_count(), 10);

        let mut handles = vec![];
        for _ in 0..10 {
            let state_clone = Arc::clone(&state);
            handles.push(thread::spawn(move || {
                state_clone.decrement_running();
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(state.running_count(), 0);
    }

    // ========================================
    // Response Serialization Tests
    // ========================================

    #[test]
    fn test_execution_status_response_serialization() {
        let response = ExecutionStatusResponse {
            is_paused: true,
            running_count: 1,
            max_concurrent: 2,
            global_max_concurrent: 20,
            queued_count: 5,
            can_start_task: false,
        };

        let json = serde_json::to_string(&response).unwrap();

        // Verify snake_case serialization (Rust default, frontend transform handles conversion)
        assert!(json.contains("\"is_paused\":true"));
        assert!(json.contains("\"running_count\":1"));
        assert!(json.contains("\"max_concurrent\":2"));
        assert!(json.contains("\"global_max_concurrent\":20"));
        assert!(json.contains("\"queued_count\":5"));
        assert!(json.contains("\"can_start_task\":false"));
    }

    #[test]
    fn test_execution_command_response_serialization() {
        let response = ExecutionCommandResponse {
            success: true,
            status: ExecutionStatusResponse {
                is_paused: false,
                running_count: 0,
                max_concurrent: 2,
                global_max_concurrent: 20,
                queued_count: 3,
                can_start_task: true,
            },
        };

        let json = serde_json::to_string(&response).unwrap();

        // Verify snake_case serialization (Rust default, frontend transform handles conversion)
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"status\":"));
        assert!(json.contains("\"is_paused\":false"));
    }

    #[test]
    fn test_execution_settings_response_serialization() {
        let response = ExecutionSettingsResponse {
            max_concurrent_tasks: 4,
            auto_commit: true,
            pause_on_failure: false,
        };

        let json = serde_json::to_string(&response).unwrap();

        // Verify snake_case serialization
        assert!(json.contains("\"max_concurrent_tasks\":4"));
        assert!(json.contains("\"auto_commit\":true"));
        assert!(json.contains("\"pause_on_failure\":false"));
    }

    #[test]
    fn test_execution_settings_response_from_domain() {
        let settings = ExecutionSettings {
            max_concurrent_tasks: 3,
            auto_commit: false,
            pause_on_failure: true,
        };

        let response = ExecutionSettingsResponse::from(settings);

        assert_eq!(response.max_concurrent_tasks, 3);
        assert!(!response.auto_commit);
        assert!(response.pause_on_failure);
    }

    #[test]
    fn test_update_execution_settings_input_deserialization() {
        let json = r#"{"max_concurrent_tasks":5,"auto_commit":false,"pause_on_failure":true}"#;

        let input: UpdateExecutionSettingsInput =
            serde_json::from_str(json).expect("Failed to deserialize input");

        assert_eq!(input.max_concurrent_tasks, 5);
        assert!(!input.auto_commit);
        assert!(input.pause_on_failure);
    }

    // ========================================
    // Integration Tests with AppState
    // ========================================

    use crate::domain::entities::{Project, Task};
    use crate::domain::repositories::{ProjectRepository, TaskRepository};
    use crate::infrastructure::memory::{MemoryProjectRepository, MemoryTaskRepository};

    async fn setup_test_state() -> (Arc<ExecutionState>, AppState) {
        let execution_state = Arc::new(ExecutionState::new());
        let task_repo = Arc::new(MemoryTaskRepository::new());
        let project_repo = Arc::new(MemoryProjectRepository::new());

        // Create a test project with tasks
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        project_repo
            .create(project.clone())
            .await
            .expect("Failed to create test project");

        // Create tasks in various statuses
        let mut task1 = Task::new(project.id.clone(), "Ready Task 1".to_string());
        task1.internal_status = InternalStatus::Ready;
        task_repo
            .create(task1)
            .await
            .expect("Failed to create Ready task 1");

        let mut task2 = Task::new(project.id.clone(), "Ready Task 2".to_string());
        task2.internal_status = InternalStatus::Ready;
        task_repo
            .create(task2)
            .await
            .expect("Failed to create Ready task 2");

        let mut task3 = Task::new(project.id.clone(), "Executing Task".to_string());
        task3.internal_status = InternalStatus::Executing;
        task_repo
            .create(task3)
            .await
            .expect("Failed to create Executing task");

        let mut task4 = Task::new(project.id.clone(), "Backlog Task".to_string());
        task4.internal_status = InternalStatus::Backlog;
        task_repo
            .create(task4)
            .await
            .expect("Failed to create Backlog task");

        let app_state = AppState::with_repos(task_repo, project_repo);

        (execution_state, app_state)
    }

    #[tokio::test]
    async fn test_get_execution_status_counts_ready_tasks() {
        let (execution_state, app_state) = setup_test_state().await;

        // Simulate the command by directly calling the logic
        let all_projects = app_state.project_repo.get_all().await.unwrap();

        let mut queued_count = 0u32;
        for project in all_projects {
            let tasks = app_state.task_repo.get_by_project(&project.id).await.unwrap();
            queued_count += tasks
                .iter()
                .filter(|t| t.internal_status == InternalStatus::Ready)
                .count() as u32;
        }

        // We created 2 ready tasks
        assert_eq!(queued_count, 2);
        assert!(!execution_state.is_paused());
        assert_eq!(execution_state.running_count(), 0);
    }

    #[tokio::test]
    async fn test_pause_sets_paused_flag() {
        let (execution_state, _app_state) = setup_test_state().await;

        assert!(!execution_state.is_paused());
        execution_state.pause();
        assert!(execution_state.is_paused());
    }

    #[tokio::test]
    async fn test_resume_clears_paused_flag() {
        let (execution_state, _app_state) = setup_test_state().await;

        execution_state.pause();
        assert!(execution_state.is_paused());

        execution_state.resume();
        assert!(!execution_state.is_paused());
    }

    #[tokio::test]
    async fn test_stop_cancels_executing_tasks() {
        let (_execution_state, app_state) = setup_test_state().await;

        // Get the project
        let projects = app_state.project_repo.get_all().await.unwrap();
        let project = &projects[0];

        // Find the executing task and stop it (simulating stop_execution behavior)
        let tasks = app_state.task_repo.get_by_project(&project.id).await.unwrap();
        for mut task in tasks {
            if task.internal_status == InternalStatus::Executing {
                task.internal_status = InternalStatus::Stopped;
                task.touch();
                app_state.task_repo.update(&task).await.unwrap();
            }
        }

        // Verify the task is now stopped (not failed)
        let tasks = app_state.task_repo.get_by_project(&project.id).await.unwrap();
        let executing_count = tasks
            .iter()
            .filter(|t| t.internal_status == InternalStatus::Executing)
            .count();
        let stopped_count = tasks
            .iter()
            .filter(|t| t.internal_status == InternalStatus::Stopped)
            .count();

        assert_eq!(executing_count, 0);
        assert_eq!(stopped_count, 1);
    }

    #[tokio::test]
    async fn test_stop_cancels_multiple_agent_active_tasks() {
        // Setup: Create tasks in various agent-active states
        let execution_state = Arc::new(ExecutionState::new());
        let app_state = AppState::new_test();

        // Create a test project
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        // Create tasks in all agent-active statuses
        let mut task1 = Task::new(project.id.clone(), "Executing Task".to_string());
        task1.internal_status = InternalStatus::Executing;
        app_state.task_repo.create(task1.clone()).await.unwrap();

        let mut task2 = Task::new(project.id.clone(), "QaRefining Task".to_string());
        task2.internal_status = InternalStatus::QaRefining;
        app_state.task_repo.create(task2.clone()).await.unwrap();

        let mut task3 = Task::new(project.id.clone(), "Reviewing Task".to_string());
        task3.internal_status = InternalStatus::Reviewing;
        app_state.task_repo.create(task3.clone()).await.unwrap();

        // Create a task NOT in agent-active state (should not be affected)
        let mut task4 = Task::new(project.id.clone(), "Ready Task".to_string());
        task4.internal_status = InternalStatus::Ready;
        app_state.task_repo.create(task4.clone()).await.unwrap();

        // Build transition service (same as stop_execution does)
        let transition_service: TaskTransitionService<tauri::Wry> = TaskTransitionService::new(
            Arc::clone(&app_state.task_repo),
            Arc::clone(&app_state.task_dependency_repo),
            Arc::clone(&app_state.project_repo),
            Arc::clone(&app_state.chat_message_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.activity_event_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            Arc::clone(&execution_state),
            None,
        );

        // Pause execution (as stop_execution would)
        execution_state.pause();

        // Transition all agent-active tasks to Stopped (as stop_execution does)
        let tasks = app_state.task_repo.get_by_project(&project.id).await.unwrap();
        for task in tasks {
            if AGENT_ACTIVE_STATUSES.contains(&task.internal_status) {
                let _ = transition_service
                    .transition_task(&task.id, InternalStatus::Stopped)
                    .await;
            }
        }

        // Verify: All agent-active tasks should now be Stopped
        let tasks = app_state.task_repo.get_by_project(&project.id).await.unwrap();

        let stopped_count = tasks
            .iter()
            .filter(|t| t.internal_status == InternalStatus::Stopped)
            .count();

        let ready_count = tasks
            .iter()
            .filter(|t| t.internal_status == InternalStatus::Ready)
            .count();

        // 3 agent-active tasks should be Stopped
        assert_eq!(stopped_count, 3);
        // 1 Ready task should remain Ready
        assert_eq!(ready_count, 1);
        // Execution should be paused
        assert!(execution_state.is_paused());
    }

    #[tokio::test]
    async fn test_pause_transitions_agent_active_tasks_to_paused() {
        // Setup: Create tasks in various agent-active states
        let execution_state = Arc::new(ExecutionState::new());
        let app_state = AppState::new_test();

        // Create a test project
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        // Create tasks in all agent-active statuses
        let mut task1 = Task::new(project.id.clone(), "Executing Task".to_string());
        task1.internal_status = InternalStatus::Executing;
        app_state.task_repo.create(task1.clone()).await.unwrap();

        let mut task2 = Task::new(project.id.clone(), "QaRefining Task".to_string());
        task2.internal_status = InternalStatus::QaRefining;
        app_state.task_repo.create(task2.clone()).await.unwrap();

        let mut task3 = Task::new(project.id.clone(), "Reviewing Task".to_string());
        task3.internal_status = InternalStatus::Reviewing;
        app_state.task_repo.create(task3.clone()).await.unwrap();

        // Create a task NOT in agent-active state (should not be affected)
        let mut task4 = Task::new(project.id.clone(), "Ready Task".to_string());
        task4.internal_status = InternalStatus::Ready;
        app_state.task_repo.create(task4.clone()).await.unwrap();

        // Build transition service (same as pause_execution does)
        let transition_service: TaskTransitionService<tauri::Wry> = TaskTransitionService::new(
            Arc::clone(&app_state.task_repo),
            Arc::clone(&app_state.task_dependency_repo),
            Arc::clone(&app_state.project_repo),
            Arc::clone(&app_state.chat_message_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.activity_event_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            Arc::clone(&execution_state),
            None,
        );

        // Pause execution (as pause_execution would)
        execution_state.pause();

        // Transition all agent-active tasks to Paused (as pause_execution does)
        let tasks = app_state.task_repo.get_by_project(&project.id).await.unwrap();
        for task in tasks {
            if AGENT_ACTIVE_STATUSES.contains(&task.internal_status) {
                let _ = transition_service
                    .transition_task(&task.id, InternalStatus::Paused)
                    .await;
            }
        }

        // Verify: All agent-active tasks should now be Paused
        let tasks = app_state.task_repo.get_by_project(&project.id).await.unwrap();

        let paused_count = tasks
            .iter()
            .filter(|t| t.internal_status == InternalStatus::Paused)
            .count();

        let ready_count = tasks
            .iter()
            .filter(|t| t.internal_status == InternalStatus::Ready)
            .count();

        // 3 agent-active tasks should be Paused
        assert_eq!(paused_count, 3);
        // 1 Ready task should remain Ready
        assert_eq!(ready_count, 1);
        // Execution should be paused
        assert!(execution_state.is_paused());
    }

    #[tokio::test]
    async fn test_pause_resets_running_count() {
        // Setup: Create tasks in agent-active states and simulate running count
        let execution_state = Arc::new(ExecutionState::with_max_concurrent(5));
        let app_state = AppState::new_test();

        // Create a test project
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        // Create tasks in agent-active statuses
        let mut task1 = Task::new(project.id.clone(), "Executing Task 1".to_string());
        task1.internal_status = InternalStatus::Executing;
        app_state.task_repo.create(task1.clone()).await.unwrap();

        let mut task2 = Task::new(project.id.clone(), "Executing Task 2".to_string());
        task2.internal_status = InternalStatus::Executing;
        app_state.task_repo.create(task2.clone()).await.unwrap();

        let mut task3 = Task::new(project.id.clone(), "Reviewing Task".to_string());
        task3.internal_status = InternalStatus::Reviewing;
        app_state.task_repo.create(task3.clone()).await.unwrap();

        // Simulate that running count matches agent-active tasks
        execution_state.increment_running(); // task1
        execution_state.increment_running(); // task2
        execution_state.increment_running(); // task3
        assert_eq!(execution_state.running_count(), 3);

        // Build transition service
        let transition_service: TaskTransitionService<tauri::Wry> = TaskTransitionService::new(
            Arc::clone(&app_state.task_repo),
            Arc::clone(&app_state.task_dependency_repo),
            Arc::clone(&app_state.project_repo),
            Arc::clone(&app_state.chat_message_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.activity_event_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            Arc::clone(&execution_state),
            None,
        );

        // Execute pause: pause and transition all agent-active tasks to Paused
        execution_state.pause();

        let tasks = app_state.task_repo.get_by_project(&project.id).await.unwrap();
        for task in tasks {
            if AGENT_ACTIVE_STATUSES.contains(&task.internal_status) {
                let _ = transition_service
                    .transition_task(&task.id, InternalStatus::Paused)
                    .await;
            }
        }

        // Verify: Running count should be 0 after all tasks transitioned to Paused
        // (on_exit handlers decrement for each agent-active state exit)
        assert_eq!(
            execution_state.running_count(),
            0,
            "Running count should be 0 after pause transitions all tasks to Paused"
        );

        // Verify execution is paused
        assert!(execution_state.is_paused());
    }

    #[test]
    fn test_agent_active_statuses_constant() {
        // Verify the constant includes all expected statuses
        assert!(AGENT_ACTIVE_STATUSES.contains(&InternalStatus::Executing));
        assert!(AGENT_ACTIVE_STATUSES.contains(&InternalStatus::QaRefining));
        assert!(AGENT_ACTIVE_STATUSES.contains(&InternalStatus::QaTesting));
        assert!(AGENT_ACTIVE_STATUSES.contains(&InternalStatus::Reviewing));
        assert!(AGENT_ACTIVE_STATUSES.contains(&InternalStatus::ReExecuting));
        assert!(AGENT_ACTIVE_STATUSES.contains(&InternalStatus::Merging));

        // Non-agent-active statuses should not be included
        assert!(!AGENT_ACTIVE_STATUSES.contains(&InternalStatus::Ready));
        assert!(!AGENT_ACTIVE_STATUSES.contains(&InternalStatus::Backlog));
        assert!(!AGENT_ACTIVE_STATUSES.contains(&InternalStatus::Failed));
        assert!(!AGENT_ACTIVE_STATUSES.contains(&InternalStatus::Stopped));
        assert!(!AGENT_ACTIVE_STATUSES.contains(&InternalStatus::Paused));
    }

    #[test]
    fn test_default_trait() {
        let state = ExecutionState::default();
        assert!(!state.is_paused());
        assert_eq!(state.running_count(), 0);
        assert_eq!(state.max_concurrent(), 2);
    }

    // ========================================
    // Event Emission Tests
    // ========================================

    #[test]
    fn test_emit_status_changed_does_not_panic() {
        let state = ExecutionState::new();
        state.increment_running();

        let handle = crate::testing::create_mock_app_handle();
        // Should not panic even with mock runtime
        state.emit_status_changed(&handle, "task_started");
    }

    #[test]
    fn test_emit_status_changed_reflects_current_state() {
        let state = ExecutionState::with_max_concurrent(4);
        state.increment_running();
        state.increment_running();
        state.pause();

        let handle = crate::testing::create_mock_app_handle();
        // Verify the method reads current state correctly
        // (emit itself is fire-and-forget, but we can verify state is consistent)
        assert!(state.is_paused());
        assert_eq!(state.running_count(), 2);
        assert_eq!(state.max_concurrent(), 4);
        state.emit_status_changed(&handle, "paused");
    }

    #[test]
    fn test_emit_status_changed_with_various_reasons() {
        let state = ExecutionState::new();
        let handle = crate::testing::create_mock_app_handle();

        // All valid reason strings should work without panic
        let reasons = ["task_started", "task_completed", "paused", "resumed", "stopped"];
        for reason in &reasons {
            state.emit_status_changed(&handle, reason);
        }
    }

    // ========================================
    // Integration Tests - Stop Execution
    // ========================================

    #[tokio::test]
    async fn test_stop_resets_running_count() {
        // Setup: Create tasks in agent-active states and simulate running count
        let execution_state = Arc::new(ExecutionState::with_max_concurrent(5));
        let app_state = AppState::new_test();

        // Create a test project
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        // Create tasks in agent-active statuses
        let mut task1 = Task::new(project.id.clone(), "Executing Task 1".to_string());
        task1.internal_status = InternalStatus::Executing;
        app_state.task_repo.create(task1.clone()).await.unwrap();

        let mut task2 = Task::new(project.id.clone(), "Executing Task 2".to_string());
        task2.internal_status = InternalStatus::Executing;
        app_state.task_repo.create(task2.clone()).await.unwrap();

        let mut task3 = Task::new(project.id.clone(), "Reviewing Task".to_string());
        task3.internal_status = InternalStatus::Reviewing;
        app_state.task_repo.create(task3.clone()).await.unwrap();

        // Simulate that running count matches agent-active tasks
        // (In real usage, spawner increments this when starting each task)
        execution_state.increment_running(); // task1
        execution_state.increment_running(); // task2
        execution_state.increment_running(); // task3
        assert_eq!(execution_state.running_count(), 3);

        // Build transition service
        let transition_service: TaskTransitionService<tauri::Wry> = TaskTransitionService::new(
            Arc::clone(&app_state.task_repo),
            Arc::clone(&app_state.task_dependency_repo),
            Arc::clone(&app_state.project_repo),
            Arc::clone(&app_state.chat_message_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.activity_event_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            Arc::clone(&execution_state),
            None,
        );

        // Execute stop: pause and transition all agent-active tasks to Stopped
        execution_state.pause();

        let tasks = app_state.task_repo.get_by_project(&project.id).await.unwrap();
        for task in tasks {
            if AGENT_ACTIVE_STATUSES.contains(&task.internal_status) {
                let _ = transition_service
                    .transition_task(&task.id, InternalStatus::Stopped)
                    .await;
            }
        }

        // Verify: Running count should be 0 after all tasks transitioned to Stopped
        // (on_exit handlers decrement for each agent-active state exit)
        assert_eq!(
            execution_state.running_count(),
            0,
            "Running count should be 0 after stop transitions all tasks to Stopped"
        );

        // Verify execution is paused
        assert!(execution_state.is_paused());
    }

    #[tokio::test]
    async fn test_running_count_decrements_on_task_completion() {
        // Setup: Create a task in Executing state
        let execution_state = Arc::new(ExecutionState::with_max_concurrent(5));
        let app_state = AppState::new_test();

        // Create a test project
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        // Create a task in Executing status
        let mut task = Task::new(project.id.clone(), "Executing Task".to_string());
        task.internal_status = InternalStatus::Executing;
        app_state.task_repo.create(task.clone()).await.unwrap();

        // Simulate that running count was incremented when task started
        execution_state.increment_running();
        assert_eq!(execution_state.running_count(), 1);

        // Build transition service with execution state
        let transition_service: TaskTransitionService<tauri::Wry> = TaskTransitionService::new(
            Arc::clone(&app_state.task_repo),
            Arc::clone(&app_state.task_dependency_repo),
            Arc::clone(&app_state.project_repo),
            Arc::clone(&app_state.chat_message_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.activity_event_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            Arc::clone(&execution_state),
            None,
        );

        // Transition task from Executing to Failed (simulating task cancellation)
        // Note: In real usage, task might go through QaRefining -> QaTesting -> QaPassed,
        // but for testing the decrement behavior, any exit from Executing is sufficient.
        let _ = transition_service
            .transition_task(&task.id, InternalStatus::Failed)
            .await;

        // Verify: Running count should have decremented
        // (on_exit handler for Executing state decrements)
        assert_eq!(
            execution_state.running_count(),
            0,
            "Running count should decrement when task exits Executing state"
        );
    }

    #[tokio::test]
    async fn test_running_count_decrements_for_all_agent_active_states() {
        // Test that decrement works for all agent-active states, not just Executing
        let execution_state = Arc::new(ExecutionState::with_max_concurrent(10));
        let app_state = AppState::new_test();

        // Create a test project
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        // Create tasks in different agent-active states
        let test_cases = [
            (InternalStatus::Executing, "Executing Task"),
            (InternalStatus::QaRefining, "QaRefining Task"),
            (InternalStatus::QaTesting, "QaTesting Task"),
            (InternalStatus::Reviewing, "Reviewing Task"),
            (InternalStatus::ReExecuting, "ReExecuting Task"),
        ];

        // Create all tasks and increment running count for each
        let mut task_ids = Vec::new();
        for (status, title) in &test_cases {
            let mut task = Task::new(project.id.clone(), title.to_string());
            task.internal_status = *status;
            app_state.task_repo.create(task.clone()).await.unwrap();
            task_ids.push(task.id);
            execution_state.increment_running();
        }

        assert_eq!(execution_state.running_count(), 5);

        // Build transition service
        let transition_service: TaskTransitionService<tauri::Wry> = TaskTransitionService::new(
            Arc::clone(&app_state.task_repo),
            Arc::clone(&app_state.task_dependency_repo),
            Arc::clone(&app_state.project_repo),
            Arc::clone(&app_state.chat_message_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.activity_event_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            Arc::clone(&execution_state),
            None,
        );

        // Transition each task to Failed (all should decrement running count)
        for task_id in &task_ids {
            let _ = transition_service
                .transition_task(task_id, InternalStatus::Failed)
                .await;
        }

        // Verify: Running count should be 0 after all tasks transitioned
        assert_eq!(
            execution_state.running_count(),
            0,
            "Running count should be 0 after all agent-active tasks exit their states"
        );
    }

    // ========================================
    // Integration Tests - Pause Prevents Spawns
    // ========================================
    // Note: Detailed spawn blocking tests are in spawner.rs:
    // - test_spawn_blocked_when_paused
    // - test_spawn_blocked_at_max_concurrent
    // - test_spawn_increments_running_count
    // These tests verify the ExecutionState integration with the spawner.

    // ========================================
    // set_max_concurrent Tests
    // ========================================

    #[test]
    fn test_set_max_concurrent_updates_value() {
        let state = ExecutionState::new();
        assert_eq!(state.max_concurrent(), 2); // default

        state.set_max_concurrent(5);
        assert_eq!(state.max_concurrent(), 5);

        state.set_max_concurrent(1);
        assert_eq!(state.max_concurrent(), 1);
    }

    #[test]
    fn test_can_start_task_respects_max_concurrent() {
        let state = ExecutionState::with_max_concurrent(2);

        // Initially can start
        assert!(state.can_start_task());

        // Add one running
        state.increment_running();
        assert!(state.can_start_task());

        // At max
        state.increment_running();
        assert!(!state.can_start_task());

        // Increase max - now can start again
        state.set_max_concurrent(3);
        assert!(state.can_start_task());
    }

    #[tokio::test]
    async fn test_resume_clears_pause_and_allows_tasks() {
        let state = ExecutionState::with_max_concurrent(2);

        // Pause
        state.pause();
        assert!(!state.can_start_task());

        // Resume
        state.resume();
        assert!(state.can_start_task());
    }

    // ========================================
    // Execution Settings Tests
    // ========================================

    #[tokio::test]
    async fn test_execution_settings_repo_get_default() {
        let app_state = AppState::new_test();

        let settings = app_state
            .execution_settings_repo
            .get_settings(None)
            .await
            .expect("Failed to get execution settings");

        // Default values
        assert_eq!(settings.max_concurrent_tasks, 2);
        assert!(settings.auto_commit);
        assert!(settings.pause_on_failure);
    }

    #[tokio::test]
    async fn test_execution_settings_repo_update() {
        let app_state = AppState::new_test();

        let new_settings = ExecutionSettings {
            max_concurrent_tasks: 5,
            auto_commit: false,
            pause_on_failure: false,
        };

        let updated = app_state
            .execution_settings_repo
            .update_settings(None, &new_settings)
            .await
            .expect("Failed to update execution settings");

        assert_eq!(updated.max_concurrent_tasks, 5);
        assert!(!updated.auto_commit);
        assert!(!updated.pause_on_failure);

        // Verify persistence
        let retrieved = app_state
            .execution_settings_repo
            .get_settings(None)
            .await
            .expect("Failed to get execution settings");

        assert_eq!(retrieved.max_concurrent_tasks, 5);
        assert!(!retrieved.auto_commit);
        assert!(!retrieved.pause_on_failure);
    }

    #[tokio::test]
    async fn test_execution_settings_update_syncs_execution_state() {
        let execution_state = Arc::new(ExecutionState::new());
        let app_state = AppState::new_test();

        // Initial state
        assert_eq!(execution_state.max_concurrent(), 2);

        // Update settings
        let new_settings = ExecutionSettings {
            max_concurrent_tasks: 8,
            auto_commit: true,
            pause_on_failure: true,
        };

        app_state
            .execution_settings_repo
            .update_settings(None, &new_settings)
            .await
            .expect("Failed to update execution settings");

        // Simulate what update_execution_settings command does
        execution_state.set_max_concurrent(8);

        // ExecutionState should be updated
        assert_eq!(execution_state.max_concurrent(), 8);
    }

    // ========================================
    // Resume Execution Tests (Phase 80 Task 4)
    // ========================================

    #[tokio::test]
    async fn test_resume_restores_paused_tasks_to_previous_status() {
        // Setup: Create a task that was Executing before being Paused
        let execution_state = Arc::new(ExecutionState::with_max_concurrent(5));
        let app_state = AppState::new_test();

        // Create a test project
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        // Create a task in Executing state
        let mut task = Task::new(project.id.clone(), "Executing Task".to_string());
        task.internal_status = InternalStatus::Executing;
        let task_id = task.id.clone();
        app_state.task_repo.create(task.clone()).await.unwrap();

        // Build transition service
        let transition_service: TaskTransitionService<tauri::Wry> = TaskTransitionService::new(
            Arc::clone(&app_state.task_repo),
            Arc::clone(&app_state.task_dependency_repo),
            Arc::clone(&app_state.project_repo),
            Arc::clone(&app_state.chat_message_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.activity_event_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            Arc::clone(&execution_state),
            None,
        );

        // Pause: transition Executing -> Paused (creates status history entry)
        execution_state.pause();
        transition_service
            .transition_task(&task_id, InternalStatus::Paused)
            .await
            .expect("Failed to transition to Paused");

        // Verify task is Paused
        let paused_task = app_state.task_repo.get_by_id(&task_id).await.unwrap().unwrap();
        assert_eq!(paused_task.internal_status, InternalStatus::Paused);

        // Verify status history shows Executing -> Paused transition
        let history = app_state.task_repo.get_status_history(&task_id).await.unwrap();
        let pause_transition = history.iter().rev().find(|t| t.to == InternalStatus::Paused);
        assert!(pause_transition.is_some());
        assert_eq!(pause_transition.unwrap().from, InternalStatus::Executing);

        // Resume: should restore Paused -> Executing
        execution_state.resume();
        transition_service
            .transition_task(&task_id, InternalStatus::Executing)
            .await
            .expect("Failed to restore from Paused");

        // Verify task is back to Executing
        let restored_task = app_state.task_repo.get_by_id(&task_id).await.unwrap().unwrap();
        assert_eq!(restored_task.internal_status, InternalStatus::Executing);
    }

    #[tokio::test]
    async fn test_resume_does_not_restore_stopped_tasks() {
        // Setup: Create a task that was Executing before being Stopped
        let execution_state = Arc::new(ExecutionState::with_max_concurrent(5));
        let app_state = AppState::new_test();

        // Create a test project
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        // Create a task and transition it to Stopped
        let mut task = Task::new(project.id.clone(), "Stopped Task".to_string());
        task.internal_status = InternalStatus::Executing;
        let task_id = task.id.clone();
        app_state.task_repo.create(task.clone()).await.unwrap();

        // Build transition service
        let transition_service: TaskTransitionService<tauri::Wry> = TaskTransitionService::new(
            Arc::clone(&app_state.task_repo),
            Arc::clone(&app_state.task_dependency_repo),
            Arc::clone(&app_state.project_repo),
            Arc::clone(&app_state.chat_message_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.activity_event_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            Arc::clone(&execution_state),
            None,
        );

        // Stop: transition Executing -> Stopped
        execution_state.pause();
        transition_service
            .transition_task(&task_id, InternalStatus::Stopped)
            .await
            .expect("Failed to transition to Stopped");

        // Verify task is Stopped
        let stopped_task = app_state.task_repo.get_by_id(&task_id).await.unwrap().unwrap();
        assert_eq!(stopped_task.internal_status, InternalStatus::Stopped);

        // Resume: should NOT restore Stopped tasks
        execution_state.resume();

        // Task should STILL be Stopped (resume doesn't restore Stopped)
        let still_stopped = app_state.task_repo.get_by_id(&task_id).await.unwrap().unwrap();
        assert_eq!(
            still_stopped.internal_status,
            InternalStatus::Stopped,
            "Stopped tasks should NOT be automatically restored on resume"
        );
    }

    #[tokio::test]
    async fn test_resume_restores_multiple_paused_tasks() {
        // Setup: Create multiple tasks in different agent-active states, pause them, then resume
        let execution_state = Arc::new(ExecutionState::with_max_concurrent(10));
        let app_state = AppState::new_test();

        // Create a test project
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        // Build transition service
        let transition_service: TaskTransitionService<tauri::Wry> = TaskTransitionService::new(
            Arc::clone(&app_state.task_repo),
            Arc::clone(&app_state.task_dependency_repo),
            Arc::clone(&app_state.project_repo),
            Arc::clone(&app_state.chat_message_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.activity_event_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            Arc::clone(&execution_state),
            None,
        );

        // Create tasks in different agent-active states
        let test_cases = [
            (InternalStatus::Executing, "Executing Task"),
            (InternalStatus::Reviewing, "Reviewing Task"),
            (InternalStatus::QaRefining, "QaRefining Task"),
        ];

        let mut task_ids = Vec::new();
        let mut original_statuses = Vec::new();
        for (status, title) in &test_cases {
            let mut task = Task::new(project.id.clone(), title.to_string());
            task.internal_status = *status;
            app_state.task_repo.create(task.clone()).await.unwrap();
            task_ids.push(task.id);
            original_statuses.push(*status);
        }

        // Pause all tasks
        execution_state.pause();
        for task_id in &task_ids {
            let _ = transition_service
                .transition_task(task_id, InternalStatus::Paused)
                .await;
        }

        // Verify all are Paused
        for task_id in &task_ids {
            let task = app_state.task_repo.get_by_id(task_id).await.unwrap().unwrap();
            assert_eq!(task.internal_status, InternalStatus::Paused);
        }

        // Resume: should restore all Paused tasks to their previous status
        execution_state.resume();
        for task_id in &task_ids {
            // Find the pre-pause status from history and restore
            let history = app_state.task_repo.get_status_history(task_id).await.unwrap();
            let pause_transition = history.iter().rev().find(|t| t.to == InternalStatus::Paused);
            if let Some(transition) = pause_transition {
                let _ = transition_service
                    .transition_task(task_id, transition.from)
                    .await;
            }
        }

        // Verify all tasks are restored to their original statuses
        for (i, task_id) in task_ids.iter().enumerate() {
            let task = app_state.task_repo.get_by_id(task_id).await.unwrap().unwrap();
            assert_eq!(
                task.internal_status, original_statuses[i],
                "Task should be restored to original status {:?}",
                original_statuses[i]
            );
        }
    }

    #[tokio::test]
    async fn test_resume_with_mixed_paused_and_stopped_tasks() {
        // Setup: Some tasks Paused, some Stopped - only Paused should be restored
        let execution_state = Arc::new(ExecutionState::with_max_concurrent(10));
        let app_state = AppState::new_test();

        // Create a test project
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        // Build transition service
        let transition_service: TaskTransitionService<tauri::Wry> = TaskTransitionService::new(
            Arc::clone(&app_state.task_repo),
            Arc::clone(&app_state.task_dependency_repo),
            Arc::clone(&app_state.project_repo),
            Arc::clone(&app_state.chat_message_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.activity_event_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            Arc::clone(&execution_state),
            None,
        );

        // Create two Executing tasks
        let mut task1 = Task::new(project.id.clone(), "To Be Paused".to_string());
        task1.internal_status = InternalStatus::Executing;
        let task1_id = task1.id.clone();
        app_state.task_repo.create(task1).await.unwrap();

        let mut task2 = Task::new(project.id.clone(), "To Be Stopped".to_string());
        task2.internal_status = InternalStatus::Executing;
        let task2_id = task2.id.clone();
        app_state.task_repo.create(task2).await.unwrap();

        execution_state.pause();

        // Transition task1 to Paused, task2 to Stopped
        transition_service
            .transition_task(&task1_id, InternalStatus::Paused)
            .await
            .expect("Failed to pause task1");
        transition_service
            .transition_task(&task2_id, InternalStatus::Stopped)
            .await
            .expect("Failed to stop task2");

        // Resume
        execution_state.resume();

        // Restore only Paused task (simulating resume_execution logic)
        let paused_tasks = app_state
            .task_repo
            .get_by_status(&project.id, InternalStatus::Paused)
            .await
            .unwrap();
        for task in paused_tasks {
            let history = app_state.task_repo.get_status_history(&task.id).await.unwrap();
            if let Some(transition) = history.iter().rev().find(|t| t.to == InternalStatus::Paused) {
                let _ = transition_service
                    .transition_task(&task.id, transition.from)
                    .await;
            }
        }

        // Verify: task1 (was Paused) should be restored to Executing
        let task1_final = app_state.task_repo.get_by_id(&task1_id).await.unwrap().unwrap();
        assert_eq!(
            task1_final.internal_status,
            InternalStatus::Executing,
            "Paused task should be restored to Executing"
        );

        // Verify: task2 (was Stopped) should remain Stopped
        let task2_final = app_state.task_repo.get_by_id(&task2_id).await.unwrap().unwrap();
        assert_eq!(
            task2_final.internal_status,
            InternalStatus::Stopped,
            "Stopped task should remain Stopped"
        );
    }
}
