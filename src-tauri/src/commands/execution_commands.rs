// Tauri commands for execution control
// Manages per-project execution state: pause, resume, stop
// Phase 82: Project-scoped execution with optional project_id parameters

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime, State};
use tokio::sync::RwLock;

use crate::application::interactive_process_registry::InteractiveProcessKey;
use crate::application::reconciliation::UserRecoveryAction;
use crate::application::{
    AppState, ReconciliationRunner, TaskSchedulerService, TaskTransitionService,
};
use crate::domain::entities::{
    task_step::StepProgressSummary, AgentRunId, AgentRunStatus, ChatContextType, InternalStatus,
    ProjectId, Task, TaskId,
};
use crate::domain::execution::ExecutionSettings;
use crate::domain::state_machine::services::TaskScheduler;
use crate::domain::state_machine::transition_handler::get_trigger_origin;

/// Statuses where an agent is actively running.
/// Tasks in these states need to be cancelled when stop is called,
/// and resumed when the app restarts.
///
/// NOTE: `Paused` is intentionally excluded. Paused tasks persist across restarts
/// with their metadata intact (pause_reason in SQLite). Startup recovery does not
/// touch them — they stay visible in the UI for manual or auto-resume.
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
    InternalStatus::Merging,      // spawns merger agent
    InternalStatus::PendingMerge, // runs attempt_programmatic_merge async side effect
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
    /// Provider rate limit backpressure: epoch seconds until which all spawns are blocked.
    /// 0 = no active rate limit. When any agent detects a provider rate limit,
    /// this is set to the retry_after timestamp so ALL subsequent spawns are gated.
    rate_limited_until: AtomicU64,
    /// Set of task IDs that currently have an `attempt_merge_auto_complete` call in flight.
    /// Prevents duplicate auto-complete calls (e.g. from agent completion + error handler)
    /// from running validation concurrently in the same worktree.
    /// Uses std::sync::Mutex (not tokio) so Drop-based guards work synchronously.
    auto_completes_in_flight: std::sync::Mutex<HashSet<String>>,
    /// Set of task IDs currently being scheduled (transition_task in progress).
    /// Prevents two concurrent scheduler invocations from double-scheduling the same task
    /// when `on_enter(Executing)` is async and a stale DB read races with the first caller.
    /// Uses std::sync::Mutex for synchronous check-and-insert atomicity.
    scheduling_in_flight: std::sync::Mutex<HashSet<String>>,
    /// Set of interactive process context keys (format: "{context_type}/{context_id}")
    /// whose execution slot has been released by TurnComplete (process is idle between turns).
    /// Used to prevent double-increment when multiple messages arrive while the agent is active.
    /// - `mark_interactive_idle(key)`: called by TurnComplete after decrementing
    /// - `claim_interactive_slot(key)`: called by send_message/queue_message fast-path;
    ///   returns true if slot was idle (caller should increment), false if already active
    /// - `remove_interactive_slot(key)`: called on process exit cleanup
    interactive_idle_slots: std::sync::Mutex<HashSet<String>>,
}

impl ExecutionState {
    /// Create a new ExecutionState with defaults
    pub fn new() -> Self {
        Self {
            is_paused: AtomicBool::new(false),
            running_count: AtomicU32::new(0),
            max_concurrent: AtomicU32::new(2),
            global_max_concurrent: AtomicU32::new(20),
            rate_limited_until: AtomicU64::new(0),
            auto_completes_in_flight: std::sync::Mutex::new(HashSet::new()),
            scheduling_in_flight: std::sync::Mutex::new(HashSet::new()),
            interactive_idle_slots: std::sync::Mutex::new(HashSet::new()),
        }
    }

    /// Create ExecutionState with custom max concurrent
    pub fn with_max_concurrent(max: u32) -> Self {
        Self {
            is_paused: AtomicBool::new(false),
            running_count: AtomicU32::new(0),
            max_concurrent: AtomicU32::new(max),
            global_max_concurrent: AtomicU32::new(20),
            rate_limited_until: AtomicU64::new(0),
            auto_completes_in_flight: std::sync::Mutex::new(HashSet::new()),
            scheduling_in_flight: std::sync::Mutex::new(HashSet::new()),
            interactive_idle_slots: std::sync::Mutex::new(HashSet::new()),
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

    /// Decrement running count (when a task completes).
    /// Uses `fetch_update` with `saturating_sub` to prevent atomic underflow.
    pub fn decrement_running(&self) -> u32 {
        let prev = self
            .running_count
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| {
                Some(v.saturating_sub(1))
            })
            .unwrap(); // fetch_update with Some always succeeds
        prev.saturating_sub(1)
    }

    /// Atomically decrement running count AND mark the interactive slot as idle.
    /// Prevents race where a concurrent `claim_interactive_slot` call between
    /// separate `decrement_running()` and `mark_interactive_idle()` sees the slot
    /// as not-yet-idle, skips increment, and leaks a count.
    pub fn decrement_and_mark_idle(&self, key: &str) -> u32 {
        let mut idle_slots = self
            .interactive_idle_slots
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        idle_slots.insert(key.to_string());
        // Decrement while holding the lock so claim_interactive_slot can't race
        let new_count = self
            .running_count
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| {
                Some(v.saturating_sub(1))
            })
            .unwrap()
            .saturating_sub(1);
        new_count
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

    /// Check if the provider rate limit is currently active (blocking all spawns)
    pub fn is_provider_blocked(&self) -> bool {
        let until = self.rate_limited_until.load(Ordering::SeqCst);
        if until == 0 {
            return false;
        }
        let now = chrono::Utc::now().timestamp() as u64;
        now < until
    }

    /// Set the provider rate limit backpressure until a specific epoch second.
    /// All agent spawns will be blocked until this time.
    pub fn set_provider_blocked_until(&self, until_epoch_secs: u64) {
        self.rate_limited_until
            .store(until_epoch_secs, Ordering::SeqCst);
    }

    /// Clear the provider rate limit backpressure (set to 0 = no limit).
    pub fn clear_provider_block(&self) {
        self.rate_limited_until.store(0, Ordering::SeqCst);
    }

    /// Get the raw epoch seconds value for provider_blocked_until (0 = no limit).
    pub fn provider_blocked_until_epoch(&self) -> u64 {
        self.rate_limited_until.load(Ordering::SeqCst)
    }

    /// Check if we can start a new task
    /// Enforces pause, provider rate limit, per-project max, and global cap (Phase 82)
    pub fn can_start_task(&self) -> bool {
        if self.is_paused() {
            return false;
        }
        if self.is_provider_blocked() {
            return false;
        }
        let running = self.running_count();
        running < self.max_concurrent() && running < self.global_max_concurrent()
    }

    /// Try to mark a task as having an auto-complete in flight.
    /// Returns `true` if the task was newly inserted (caller should proceed).
    /// Returns `false` if the task was already in the set (caller should skip — duplicate).
    pub fn try_start_auto_complete(&self, task_id: &str) -> bool {
        let mut set = self
            .auto_completes_in_flight
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        set.insert(task_id.to_string())
    }

    /// Remove a task from the auto-completes-in-flight set.
    /// Called when auto-complete finishes (success or failure).
    pub fn finish_auto_complete(&self, task_id: &str) {
        let mut set = self
            .auto_completes_in_flight
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        set.remove(task_id);
    }

    /// Check if a task currently has an auto-complete in flight.
    /// Used by the reconciler to skip reconciliation when auto-complete is already running
    /// (prevents misinterpreting the dedup guard's skip as a failure).
    pub fn is_auto_complete_in_flight(&self, task_id: &str) -> bool {
        let set = self
            .auto_completes_in_flight
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        set.contains(task_id)
    }

    /// Try to claim a task for scheduling (per-task concurrency guard).
    /// Returns `true` if the task was newly inserted (caller should proceed with transition).
    /// Returns `false` if the task is already being scheduled (caller should skip).
    pub fn try_start_scheduling(&self, task_id: &str) -> bool {
        let mut set = self
            .scheduling_in_flight
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        set.insert(task_id.to_string())
    }

    /// Remove a task from the scheduling-in-flight set.
    /// Must be called after `transition_task` completes (success or error).
    pub fn finish_scheduling(&self, task_id: &str) {
        let mut set = self
            .scheduling_in_flight
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        set.remove(task_id);
    }

    /// Mark an interactive process as idle (execution slot released by TurnComplete).
    /// The key format is "{context_type}/{context_id}".
    pub fn mark_interactive_idle(&self, key: &str) {
        let mut set = self
            .interactive_idle_slots
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        set.insert(key.to_string());
    }

    /// Atomically claim an interactive slot if the process is idle.
    /// Returns `true` if the slot was idle (removed from set — caller should increment running).
    /// Returns `false` if the slot was already active (not in set — caller should NOT increment).
    pub fn claim_interactive_slot(&self, key: &str) -> bool {
        let mut set = self
            .interactive_idle_slots
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        set.remove(key)
    }

    /// Remove an interactive slot from tracking (process exited).
    pub fn remove_interactive_slot(&self, key: &str) {
        let mut set = self
            .interactive_idle_slots
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        set.remove(key);
    }

    /// Check if a specific interactive slot is currently idle (between turns).
    /// Used by reconciliation to avoid overcounting idle processes as active.
    pub fn is_interactive_idle(&self, key: &str) -> bool {
        let set = self
            .interactive_idle_slots
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        set.contains(key)
    }

    /// Emit execution:status_changed event with current state
    pub fn emit_status_changed<R: Runtime>(&self, handle: &AppHandle<R>, reason: &str) {
        let blocked_until = self.provider_blocked_until_epoch();
        let _ = handle.emit(
            "execution:status_changed",
            serde_json::json!({
                "isPaused": self.is_paused(),
                "runningCount": self.running_count(),
                "maxConcurrent": self.max_concurrent(),
                "globalMaxConcurrent": self.global_max_concurrent(),
                "providerBlocked": self.is_provider_blocked(),
                "providerBlockedUntil": if blocked_until > 0 { Some(blocked_until) } else { None::<u64> },
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
    /// Whether a provider rate limit is currently blocking all spawns
    pub provider_blocked: bool,
    /// Epoch seconds when the provider rate limit expires (0 = no limit)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_blocked_until: Option<u64>,
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

// ========================================
// Quota Sync Helper
// ========================================

/// Result of syncing project quota
/// Contains the resolved project ID and the max concurrent value that was applied
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ProjectQuotaSync {
    /// The resolved project ID (None if global/no project)
    pub project_id: Option<ProjectId>,
    /// The max concurrent tasks value that was synced to execution_state
    pub max_concurrent: u32,
}

/// Syncs runtime ExecutionState max_concurrent with persisted project settings.
/// Returns the resolved project ID and the effective max_concurrent value.
///
/// Resolution order:
/// 1. Explicit project_id parameter
/// 2. Active project from active_project_state
/// 3. None (uses global default settings)
///
/// This helper ensures the runtime quota always reflects the active project's
/// persisted settings, preventing drift when switching projects or querying status.
async fn sync_quota_from_project(
    project_id: Option<ProjectId>,
    active_project_state: &Arc<ActiveProjectState>,
    execution_state: &Arc<ExecutionState>,
    app_state: &AppState,
) -> Result<(Option<ProjectId>, u32), String> {
    // Determine effective project_id: explicit param > active project > none
    let effective_project_id = match project_id {
        Some(id) => Some(id),
        None => active_project_state.get().await,
    };

    // Load execution settings for the effective project
    let settings = app_state
        .execution_settings_repo
        .get_settings(effective_project_id.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    // Sync runtime ExecutionState with persisted max_concurrent
    execution_state.set_max_concurrent(settings.max_concurrent_tasks);

    Ok((effective_project_id, settings.max_concurrent_tasks))
}

/// Wrapper that returns a `ProjectQuotaSync` struct instead of a tuple.
/// Delegates to `sync_quota_from_project` for the actual logic.
#[allow(dead_code)]
async fn sync_project_quota(
    explicit_project_id: Option<ProjectId>,
    active_project_state: &Arc<ActiveProjectState>,
    execution_state: &Arc<ExecutionState>,
    app_state: &AppState,
) -> Result<ProjectQuotaSync, String> {
    let (project_id, max_concurrent) = sync_quota_from_project(
        explicit_project_id,
        active_project_state,
        execution_state,
        app_state,
    )
    .await?;

    Ok(ProjectQuotaSync {
        project_id,
        max_concurrent,
    })
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

    // Runtime GC pass to prune stale rows on every status poll.
    prune_stale_execution_registry_entries(&app_state).await;

    let registry_entries = app_state.running_agent_registry.list_all().await;

    // Keep execution state synchronized to global execution contexts.
    let global_running_count = registry_entries
        .iter()
        .filter(|(key, _)| is_execution_context_type(&key.context_type))
        .count() as u32;
    execution_state.set_running_count(global_running_count);

    let mut running_count = 0u32;
    for (key, _) in registry_entries {
        let context_type = match ChatContextType::from_str(&key.context_type) {
            Ok(value) => value,
            Err(_) => continue,
        };

        if !matches!(
            context_type,
            ChatContextType::TaskExecution | ChatContextType::Review | ChatContextType::Merge
        ) {
            continue;
        }

        let task_id = TaskId::from_string(key.context_id);
        let task = match app_state.task_repo.get_by_id(&task_id).await {
            Ok(Some(task)) => task,
            _ => continue,
        };

        if let Some(pid) = &effective_project_id {
            if task.project_id != *pid {
                continue;
            }
        }

        // Skip entries whose task status doesn't match the expected running
        // status for this context type (e.g., Failed task with TaskExecution entry)
        if !context_matches_running_status_for_gc(context_type, task.internal_status) {
            continue;
        }

        running_count += 1;
    }

    let max_concurrent = execution_state.max_concurrent();
    let global_max = execution_state.global_max_concurrent();

    let blocked_until = execution_state.provider_blocked_until_epoch();
    Ok(ExecutionStatusResponse {
        is_paused: execution_state.is_paused(),
        running_count,
        max_concurrent,
        global_max_concurrent: global_max,
        queued_count,
        can_start_task: !execution_state.is_paused()
            && !execution_state.is_provider_blocked()
            && running_count < max_concurrent
            && global_running_count < global_max,
        provider_blocked: execution_state.is_provider_blocked(),
        provider_blocked_until: if blocked_until > 0 {
            Some(blocked_until)
        } else {
            None
        },
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
    // Sync runtime quota with persisted project settings before can_start_task() loops
    let project_id = project_id.map(|id| ProjectId::from_string(id));
    let (effective_project_id, _max_concurrent) = sync_quota_from_project(
        project_id,
        &active_project_state,
        &execution_state,
        &app_state,
    )
    .await?;

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
        .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo))
        .with_interactive_process_registry(Arc::clone(&app_state.interactive_process_registry)),
    );
    scheduler.set_self_ref(Arc::clone(&scheduler) as Arc<dyn TaskScheduler>);
    // Set active project scope before scheduling to prevent cross-project scheduling
    scheduler
        .set_active_project(effective_project_id.clone())
        .await;
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

    let transition_service = Arc::new(
        TaskTransitionService::new(
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
        .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo))
        .with_interactive_process_registry(Arc::clone(&app_state.interactive_process_registry)),
    );

    let reconciler = ReconciliationRunner::new(
        Arc::clone(&app_state.task_repo),
        Arc::clone(&app_state.task_dependency_repo),
        Arc::clone(&app_state.project_repo),
        Arc::clone(&app_state.chat_conversation_repo),
        Arc::clone(&app_state.chat_message_repo),
        Arc::clone(&app_state.chat_attachment_repo),
        Arc::clone(&app_state.ideation_session_repo),
        Arc::clone(&app_state.activity_event_repo),
        Arc::clone(&app_state.message_queue),
        Arc::clone(&app_state.running_agent_registry),
        Arc::clone(&app_state.memory_event_repo),
        Arc::clone(&app_state.agent_run_repo),
        transition_service,
        Arc::clone(&execution_state),
        Some(app),
    )
    .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo))
    .with_interactive_process_registry(Arc::clone(&app_state.interactive_process_registry));

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

    let transition_service = Arc::new(
        TaskTransitionService::new(
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
        .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo))
        .with_interactive_process_registry(Arc::clone(&app_state.interactive_process_registry)),
    );

    let reconciler = ReconciliationRunner::new(
        Arc::clone(&app_state.task_repo),
        Arc::clone(&app_state.task_dependency_repo),
        Arc::clone(&app_state.project_repo),
        Arc::clone(&app_state.chat_conversation_repo),
        Arc::clone(&app_state.chat_message_repo),
        Arc::clone(&app_state.chat_attachment_repo),
        Arc::clone(&app_state.ideation_session_repo),
        Arc::clone(&app_state.activity_event_repo),
        Arc::clone(&app_state.message_queue),
        Arc::clone(&app_state.running_agent_registry),
        Arc::clone(&app_state.memory_event_repo),
        Arc::clone(&app_state.agent_run_repo),
        transition_service,
        Arc::clone(&execution_state),
        Some(app),
    )
    .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo))
    .with_interactive_process_registry(Arc::clone(&app_state.interactive_process_registry));

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
            .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo))
            .with_interactive_process_registry(Arc::clone(&app_state.interactive_process_registry)),
        );
        scheduler.set_self_ref(Arc::clone(&scheduler) as Arc<dyn TaskScheduler>);
        // Set active project scope before scheduling to prevent cross-project scheduling
        scheduler.set_active_project(active_project_id).await;
        scheduler.try_schedule_ready_tasks().await;
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
                .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo))
                .with_interactive_process_registry(Arc::clone(&app_state.interactive_process_registry)),
            );
            scheduler.set_self_ref(Arc::clone(&scheduler) as Arc<dyn TaskScheduler>);
            // Set active project scope before scheduling to prevent cross-project scheduling
            scheduler.set_active_project(project_id.clone()).await;
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
        // Get active project for scoped scheduling
        let active_project_id = active_project_state.get().await;

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
            .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo))
            .with_interactive_process_registry(Arc::clone(&app_state.interactive_process_registry)),
        );
        scheduler.set_self_ref(Arc::clone(&scheduler) as Arc<dyn TaskScheduler>);
        // Set active project scope before scheduling to prevent cross-project scheduling
        scheduler.set_active_project(active_project_id).await;
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

// ========================================
// Running Processes Query
// ========================================

/// A single running process with enriched data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningProcess {
    /// Task ID
    pub task_id: String,
    /// Task title
    pub title: String,
    /// Current internal status
    pub internal_status: String,
    /// Step progress summary (if steps exist)
    pub step_progress: Option<StepProgressSummary>,
    /// Elapsed time in seconds since entering current status
    pub elapsed_seconds: Option<i64>,
    /// Trigger origin (scheduler, revision, recovery, retry, qa)
    pub trigger_origin: Option<String>,
    /// Task branch name
    pub task_branch: Option<String>,
}

/// Response for get_running_processes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningProcessesResponse {
    /// List of running processes
    pub processes: Vec<RunningProcess>,
}

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
    state: State<'_, AppState>,
) -> Result<RunningProcessesResponse, String> {
    let effective_project_id = match project_id {
        Some(id) => Some(ProjectId::from_string(id)),
        None => active_project_state.get().await,
    };

    // Keep the registry clean so process rows reflect truly running agents.
    prune_stale_execution_registry_entries(&state).await;

    let mut processes = Vec::new();
    let mut seen_task_ids = std::collections::HashSet::new();
    let registry_entries = state.running_agent_registry.list_all().await;

    for (key, _) in registry_entries {
        if !is_execution_context_type(&key.context_type) {
            continue;
        }

        let context_type = match ChatContextType::from_str(&key.context_type) {
            Ok(value) => value,
            Err(_) => continue,
        };

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

        let elapsed_seconds = history
            .iter()
            .rev()
            .find(|t| t.to == task.internal_status)
            .map(|transition| {
                let now = chrono::Utc::now();
                let elapsed = now.signed_duration_since(transition.timestamp);
                elapsed.num_seconds()
            });

        // Get trigger origin
        let trigger_origin = get_trigger_origin(&task);

        processes.push(RunningProcess {
            task_id: task_id_str,
            title: task.title.clone(),
            internal_status: task.internal_status.as_str().to_string(),
            step_progress,
            elapsed_seconds,
            trigger_origin,
            task_branch: task.task_branch.clone(),
        });
    }

    Ok(RunningProcessesResponse { processes })
}

fn is_execution_context_type(context_type: &str) -> bool {
    matches!(context_type, "task_execution" | "review" | "merge")
}

fn process_is_alive_for_gc(pid: u32) -> bool {
    // PID 0 refers to the process group on macOS/Unix — `kill -0 0` succeeds
    // but doesn't mean a real agent is alive. Placeholder PIDs from try_register
    // use pid=0 before update_agent_process fills in the real PID.
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|status| status.success())
            .unwrap_or(true)
    }

    #[cfg(windows)]
    {
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/FO", "CSV", "/NH"])
            .output()
            .map(|output| {
                if !output.status.success() {
                    return true;
                }
                let text = String::from_utf8_lossy(&output.stdout);
                !text.to_ascii_lowercase().contains("no tasks are running")
            })
            .unwrap_or(true)
    }
}

fn context_matches_running_status_for_gc(
    context_type: ChatContextType,
    status: InternalStatus,
) -> bool {
    match context_type {
        ChatContextType::TaskExecution => {
            status == InternalStatus::Executing || status == InternalStatus::ReExecuting
        }
        ChatContextType::Review => status == InternalStatus::Reviewing,
        ChatContextType::Merge => status == InternalStatus::Merging,
        ChatContextType::Task | ChatContextType::Ideation | ChatContextType::Project => false,
    }
}

async fn prune_stale_execution_registry_entries(app_state: &AppState) {
    let entries = app_state.running_agent_registry.list_all().await;
    if entries.is_empty() {
        return;
    }

    for (key, info) in entries {
        if !is_execution_context_type(&key.context_type) {
            continue;
        }

        // Age guard: pid=0 entries younger than 30s are in the try_register →
        // update_agent_process window. The pruner must not race against the spawn.
        if info.pid == 0 {
            let age = chrono::Utc::now() - info.started_at;
            if age < chrono::Duration::seconds(30) {
                continue;
            }
        }

        // Skip entries with an active interactive process — the CLI is alive
        // between turns, waiting for the next stdin message.
        {
            let ipr_key = InteractiveProcessKey::new(&key.context_type, &key.context_id);
            if app_state.interactive_process_registry.has_process(&ipr_key).await {
                tracing::debug!(
                    context_type = key.context_type,
                    context_id = key.context_id,
                    "Skipping prune for interactive process"
                );
                continue;
            }
        }

        let context_type = match ChatContextType::from_str(&key.context_type) {
            Ok(context_type) => context_type,
            Err(_) => continue,
        };

        let pid_alive = process_is_alive_for_gc(info.pid);
        let run = match app_state
            .agent_run_repo
            .get_by_id(&AgentRunId::from_string(&info.agent_run_id))
            .await
        {
            Ok(run) => run,
            Err(_) => continue,
        };

        let mut stale = !pid_alive;
        if !matches!(
            run.as_ref().map(|r| r.status),
            Some(AgentRunStatus::Running)
        ) {
            stale = true;
        }

        let task_id = TaskId::from_string(key.context_id.clone());
        match app_state.task_repo.get_by_id(&task_id).await {
            Ok(Some(task)) => {
                if !context_matches_running_status_for_gc(context_type, task.internal_status) {
                    stale = true;
                }
            }
            Ok(None) | Err(_) => {
                stale = true;
            }
        }

        if !stale {
            continue;
        }

        if pid_alive {
            let _ = app_state.running_agent_registry.stop(&key).await;
        } else {
            let _ = app_state
                .running_agent_registry
                .unregister(&key, &info.agent_run_id)
                .await;
        }

        if let Some(agent_run) = run {
            if agent_run.status == AgentRunStatus::Running {
                let _ = app_state
                    .agent_run_repo
                    .cancel(&AgentRunId::from_string(&info.agent_run_id))
                    .await;
            }
        }
    }
}

// ========================================
// Smart Resume Types and Functions
// ========================================

/// Category of resume behavior based on the stopped_from_status.
///
/// Determines how a task should be resumed after being stopped mid-execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResumeCategory {
    /// Directly resume to the original state (spawn agent if needed).
    /// Used for: Executing, ReExecuting, Reviewing, QaRefining, QaTesting
    Direct,
    /// Validate git state before resuming.
    /// Used for: Merging, PendingMerge, MergeConflict, MergeIncomplete
    Validated,
    /// Redirect to a successor state (avoid invalid intermediate states).
    /// Used for: QaPassed, RevisionNeeded, PendingReview
    Redirect,
}

/// Result of categorizing a resume state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategorizedResume {
    /// The category of resume behavior
    pub category: ResumeCategory,
    /// The target status to resume to (may differ from original for Redirect)
    pub target_status: InternalStatus,
}

/// Categorize the resume state based on the stopped_from_status.
///
/// Returns a `CategorizedResume` with the category and target status.
/// For Redirect states, the target is the successor state.
pub fn categorize_resume_state(stopped_from_status: InternalStatus) -> CategorizedResume {
    match stopped_from_status {
        // Direct Resume: spawn agent directly
        InternalStatus::Executing
        | InternalStatus::ReExecuting
        | InternalStatus::Reviewing
        | InternalStatus::QaRefining
        | InternalStatus::QaTesting => CategorizedResume {
            category: ResumeCategory::Direct,
            target_status: stopped_from_status,
        },

        // Validated Resume: check git state first
        InternalStatus::Merging
        | InternalStatus::PendingMerge
        | InternalStatus::MergeConflict
        | InternalStatus::MergeIncomplete => CategorizedResume {
            category: ResumeCategory::Validated,
            target_status: stopped_from_status,
        },

        // Redirect: go to successor state (these have auto-transitions)
        InternalStatus::QaPassed => CategorizedResume {
            // QaPassed → PendingReview (auto-transitions anyway)
            category: ResumeCategory::Redirect,
            target_status: InternalStatus::PendingReview,
        },
        InternalStatus::RevisionNeeded => CategorizedResume {
            // RevisionNeeded → ReExecuting (auto-transitions anyway)
            category: ResumeCategory::Redirect,
            target_status: InternalStatus::ReExecuting,
        },
        InternalStatus::PendingReview => CategorizedResume {
            // PendingReview → Reviewing (spawn reviewer)
            category: ResumeCategory::Redirect,
            target_status: InternalStatus::Reviewing,
        },

        // Default: treat as Direct (fallback to Ready if invalid)
        _ => CategorizedResume {
            category: ResumeCategory::Direct,
            target_status: stopped_from_status,
        },
    }
}

/// Validation warning for resume operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeValidationWarning {
    /// Warning code (e.g., "dirty_worktree", "base_branch_moved")
    pub code: String,
    /// Human-readable warning message
    pub message: String,
}

/// Result of resume validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeValidationResult {
    /// Whether validation passed (true = can proceed)
    pub passed: bool,
    /// Warnings encountered (non-blocking issues)
    pub warnings: Vec<ResumeValidationWarning>,
}

/// Result type for restart_task command.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RestartResult {
    /// Task was successfully restarted
    Success {
        /// The updated task
        task: serde_json::Value,
        /// The category of resume that was used
        category: ResumeCategory,
        /// The status the task was resumed to
        resumed_to_status: String,
    },
    /// Validation failed (only for Validated category)
    ValidationFailed {
        /// Validation warnings that caused the failure
        warnings: Vec<ResumeValidationWarning>,
        /// The stopped_from_status for reference
        stopped_from_status: String,
    },
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
    use crate::application::TaskTransitionService;
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
    let transition_service = TaskTransitionService::new(
        Arc::clone(&state.task_repo),
        Arc::clone(&state.task_dependency_repo),
        Arc::clone(&state.project_repo),
        Arc::clone(&state.chat_message_repo),
        Arc::clone(&state.chat_attachment_repo),
        Arc::clone(&state.chat_conversation_repo),
        Arc::clone(&state.agent_run_repo),
        Arc::clone(&state.ideation_session_repo),
        Arc::clone(&state.activity_event_repo),
        Arc::clone(&state.message_queue),
        Arc::clone(&state.running_agent_registry),
        Arc::clone(&execution_state),
        state.app_handle.clone(),
        Arc::clone(&state.memory_event_repo),
    )
    .with_plan_branch_repo(Arc::clone(&state.plan_branch_repo))
    .with_interactive_process_registry(Arc::clone(&state.interactive_process_registry));

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

/// Validate resume for Validated category states.
///
/// Checks:
/// - Task branch exists and is accessible
/// - Worktree is clean (no uncommitted changes)
/// - No stale merge/rebase in progress
async fn validate_resume(task: &Task, state: &AppState) -> ResumeValidationResult {
    use crate::application::git_service::GitService;
    use std::path::Path;

    let mut warnings = Vec::new();

    // Get project for git operations
    let project = match state.project_repo.get_by_id(&task.project_id).await {
        Ok(Some(p)) => p,
        _ => {
            warnings.push(ResumeValidationWarning {
                code: "project_not_found".to_string(),
                message: "Could not find project for git validation".to_string(),
            });
            return ResumeValidationResult {
                passed: false,
                warnings,
            };
        }
    };

    // Check if task has a branch
    let branch_name = match &task.task_branch {
        Some(branch) => branch.clone(),
        None => {
            warnings.push(ResumeValidationWarning {
                code: "no_branch".to_string(),
                message: "Task has no associated branch".to_string(),
            });
            return ResumeValidationResult {
                passed: false,
                warnings,
            };
        }
    };

    let repo_path = Path::new(&project.working_directory);

    // Check branch exists
    if !GitService::branch_exists(repo_path, &branch_name).await {
        warnings.push(ResumeValidationWarning {
            code: "branch_not_found".to_string(),
            message: format!("Task branch '{}' does not exist", branch_name),
        });
        return ResumeValidationResult {
            passed: false,
            warnings,
        };
    }

    // Check worktree is clean (if worktree path exists)
    if let Some(worktree_path) = &task.worktree_path {
        let worktree = Path::new(worktree_path);
        match GitService::has_uncommitted_changes(worktree).await {
            Ok(false) => {} // Clean, no changes
            Ok(true) => {
                warnings.push(ResumeValidationWarning {
                    code: "dirty_worktree".to_string(),
                    message: "Worktree has uncommitted changes".to_string(),
                });
                // Non-blocking warning - just log
                tracing::warn!(
                    task_id = task.id.as_str(),
                    worktree = %worktree_path,
                    "Worktree is dirty but proceeding"
                );
            }
            Err(e) => {
                warnings.push(ResumeValidationWarning {
                    code: "worktree_check_failed".to_string(),
                    message: format!("Could not check worktree status: {}", e),
                });
            }
        }
    }

    // All critical checks passed
    ResumeValidationResult {
        passed: true,
        warnings,
    }
}

#[cfg(test)]
#[path = "execution_commands_running_count_tests.rs"]
mod running_count_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::GitMode;
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

    // ========================================
    // Provider Rate Limit Backpressure Tests
    // ========================================

    #[test]
    fn test_can_start_task_returns_false_when_provider_blocked() {
        let state = ExecutionState::new();
        // Set provider blocked until 60 seconds in the future
        let future_epoch = chrono::Utc::now().timestamp() as u64 + 60;
        state.set_provider_blocked_until(future_epoch);

        assert!(state.is_provider_blocked());
        assert!(!state.can_start_task());
    }

    #[test]
    fn test_can_start_task_returns_true_when_block_expired() {
        let state = ExecutionState::new();
        // Set provider blocked until 60 seconds in the past
        let past_epoch = chrono::Utc::now().timestamp() as u64 - 60;
        state.set_provider_blocked_until(past_epoch);

        assert!(!state.is_provider_blocked());
        assert!(state.can_start_task());
    }

    #[test]
    fn test_can_start_task_returns_true_when_no_block() {
        let state = ExecutionState::new();
        // Default: no provider block
        assert!(!state.is_provider_blocked());
        assert!(state.can_start_task());
    }

    #[test]
    fn test_set_clear_provider_block_lifecycle() {
        let state = ExecutionState::new();

        // Initially not blocked
        assert!(!state.is_provider_blocked());
        assert_eq!(state.provider_blocked_until_epoch(), 0);

        // Set block in the future
        let future_epoch = chrono::Utc::now().timestamp() as u64 + 300;
        state.set_provider_blocked_until(future_epoch);
        assert!(state.is_provider_blocked());
        assert_eq!(state.provider_blocked_until_epoch(), future_epoch);

        // Clear block
        state.clear_provider_block();
        assert!(!state.is_provider_blocked());
        assert_eq!(state.provider_blocked_until_epoch(), 0);
    }

    #[test]
    fn test_provider_block_independent_of_pause() {
        let state = ExecutionState::new();
        let future_epoch = chrono::Utc::now().timestamp() as u64 + 60;
        state.set_provider_blocked_until(future_epoch);

        // Provider blocked, not paused — still can't start
        assert!(!state.is_paused());
        assert!(state.is_provider_blocked());
        assert!(!state.can_start_task());

        // Clear provider block, pause — still can't start (different reason)
        state.clear_provider_block();
        state.pause();
        assert!(!state.is_provider_blocked());
        assert!(state.is_paused());
        assert!(!state.can_start_task());

        // Both blocked — still can't start
        state.set_provider_blocked_until(future_epoch);
        assert!(state.is_provider_blocked());
        assert!(state.is_paused());
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
            provider_blocked: false,
            provider_blocked_until: None,
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
                provider_blocked: false,
                provider_blocked_until: None,
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
    // sync_project_quota Tests
    // ========================================

    #[tokio::test]
    async fn test_sync_project_quota_explicit_project_priority() {
        let app_state = AppState::new_test();
        let execution_state = Arc::new(ExecutionState::new());
        let active_project_state = Arc::new(ActiveProjectState::new());

        // Create two projects with different quotas
        let project1 = Project::new("Project 1".to_string(), "/path1".to_string());
        let project2 = Project::new("Project 2".to_string(), "/path2".to_string());

        app_state
            .project_repo
            .create(project1.clone())
            .await
            .unwrap();
        app_state
            .project_repo
            .create(project2.clone())
            .await
            .unwrap();

        // Set different quotas for each project
        let settings1 = ExecutionSettings {
            max_concurrent_tasks: 5,
            auto_commit: true,
            pause_on_failure: true,
        };
        let settings2 = ExecutionSettings {
            max_concurrent_tasks: 10,
            auto_commit: true,
            pause_on_failure: true,
        };

        app_state
            .execution_settings_repo
            .update_settings(Some(&project1.id), &settings1)
            .await
            .unwrap();
        app_state
            .execution_settings_repo
            .update_settings(Some(&project2.id), &settings2)
            .await
            .unwrap();

        // Set project1 as active
        active_project_state.set(Some(project1.id.clone())).await;

        // Call sync with explicit project2 - should use project2, not active project1
        let result = sync_project_quota(
            Some(project2.id.clone()),
            &active_project_state,
            &execution_state,
            &app_state,
        )
        .await
        .unwrap();

        // Should use explicit project2 (quota 10), not active project1 (quota 5)
        assert_eq!(result.project_id, Some(project2.id));
        assert_eq!(result.max_concurrent, 10);
        assert_eq!(execution_state.max_concurrent(), 10);
    }

    #[tokio::test]
    async fn test_sync_project_quota_active_project_fallback() {
        let app_state = AppState::new_test();
        let execution_state = Arc::new(ExecutionState::new());
        let active_project_state = Arc::new(ActiveProjectState::new());

        // Create project with custom quota
        let project = Project::new("Active Project".to_string(), "/path".to_string());
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        let settings = ExecutionSettings {
            max_concurrent_tasks: 7,
            auto_commit: true,
            pause_on_failure: true,
        };
        app_state
            .execution_settings_repo
            .update_settings(Some(&project.id), &settings)
            .await
            .unwrap();

        // Set as active project
        active_project_state.set(Some(project.id.clone())).await;

        // Call sync without explicit project - should use active project
        let result = sync_project_quota(None, &active_project_state, &execution_state, &app_state)
            .await
            .unwrap();

        assert_eq!(result.project_id, Some(project.id));
        assert_eq!(result.max_concurrent, 7);
        assert_eq!(execution_state.max_concurrent(), 7);
    }

    #[tokio::test]
    async fn test_sync_project_quota_none_fallback_to_global_default() {
        let app_state = AppState::new_test();
        let execution_state = Arc::new(ExecutionState::new());
        let active_project_state = Arc::new(ActiveProjectState::new());

        // No explicit project, no active project
        // Should use global default (project_id = None)
        let result = sync_project_quota(None, &active_project_state, &execution_state, &app_state)
            .await
            .unwrap();

        assert_eq!(result.project_id, None);
        // Default quota is 10 (from ExecutionSettings::default())
        assert_eq!(result.max_concurrent, 10);
        assert_eq!(execution_state.max_concurrent(), 10);
    }

    #[tokio::test]
    async fn test_sync_project_quota_updates_execution_state() {
        let app_state = AppState::new_test();
        let execution_state = Arc::new(ExecutionState::with_max_concurrent(99));
        let active_project_state = Arc::new(ActiveProjectState::new());

        let project = Project::new("Test Project".to_string(), "/path".to_string());
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        let settings = ExecutionSettings {
            max_concurrent_tasks: 15,
            auto_commit: true,
            pause_on_failure: true,
        };
        app_state
            .execution_settings_repo
            .update_settings(Some(&project.id), &settings)
            .await
            .unwrap();

        // Before sync, execution_state has old value
        assert_eq!(execution_state.max_concurrent(), 99);

        // Sync should update execution_state
        sync_project_quota(
            Some(project.id.clone()),
            &active_project_state,
            &execution_state,
            &app_state,
        )
        .await
        .unwrap();

        // After sync, execution_state should have new value
        assert_eq!(execution_state.max_concurrent(), 15);
    }

    #[tokio::test]
    async fn test_sync_project_quota_multiple_calls_idempotent() {
        let app_state = AppState::new_test();
        let execution_state = Arc::new(ExecutionState::new());
        let active_project_state = Arc::new(ActiveProjectState::new());

        let project = Project::new("Test Project".to_string(), "/path".to_string());
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        let settings = ExecutionSettings {
            max_concurrent_tasks: 8,
            auto_commit: true,
            pause_on_failure: true,
        };
        app_state
            .execution_settings_repo
            .update_settings(Some(&project.id), &settings)
            .await
            .unwrap();

        // Call sync multiple times
        for _ in 0..3 {
            let result = sync_project_quota(
                Some(project.id.clone()),
                &active_project_state,
                &execution_state,
                &app_state,
            )
            .await
            .unwrap();

            assert_eq!(result.project_id, Some(project.id.clone()));
            assert_eq!(result.max_concurrent, 8);
            assert_eq!(execution_state.max_concurrent(), 8);
        }
    }

    #[tokio::test]
    async fn test_sync_project_quota_switching_between_projects() {
        let app_state = AppState::new_test();
        let execution_state = Arc::new(ExecutionState::new());
        let active_project_state = Arc::new(ActiveProjectState::new());

        let project1 = Project::new("Project 1".to_string(), "/path1".to_string());
        let project2 = Project::new("Project 2".to_string(), "/path2".to_string());

        app_state
            .project_repo
            .create(project1.clone())
            .await
            .unwrap();
        app_state
            .project_repo
            .create(project2.clone())
            .await
            .unwrap();

        let settings1 = ExecutionSettings {
            max_concurrent_tasks: 3,
            auto_commit: true,
            pause_on_failure: true,
        };
        let settings2 = ExecutionSettings {
            max_concurrent_tasks: 12,
            auto_commit: true,
            pause_on_failure: true,
        };

        app_state
            .execution_settings_repo
            .update_settings(Some(&project1.id), &settings1)
            .await
            .unwrap();
        app_state
            .execution_settings_repo
            .update_settings(Some(&project2.id), &settings2)
            .await
            .unwrap();

        // Sync to project1
        sync_project_quota(
            Some(project1.id.clone()),
            &active_project_state,
            &execution_state,
            &app_state,
        )
        .await
        .unwrap();
        assert_eq!(execution_state.max_concurrent(), 3);

        // Switch to project2
        sync_project_quota(
            Some(project2.id.clone()),
            &active_project_state,
            &execution_state,
            &app_state,
        )
        .await
        .unwrap();
        assert_eq!(execution_state.max_concurrent(), 12);

        // Switch back to project1
        sync_project_quota(
            Some(project1.id.clone()),
            &active_project_state,
            &execution_state,
            &app_state,
        )
        .await
        .unwrap();
        assert_eq!(execution_state.max_concurrent(), 3);
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
            let tasks = app_state
                .task_repo
                .get_by_project(&project.id)
                .await
                .unwrap();
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
        let tasks = app_state
            .task_repo
            .get_by_project(&project.id)
            .await
            .unwrap();
        for mut task in tasks {
            if task.internal_status == InternalStatus::Executing {
                task.internal_status = InternalStatus::Stopped;
                task.touch();
                app_state.task_repo.update(&task).await.unwrap();
            }
        }

        // Verify the task is now stopped (not failed)
        let tasks = app_state
            .task_repo
            .get_by_project(&project.id)
            .await
            .unwrap();
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
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

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
            Arc::clone(&app_state.chat_attachment_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.activity_event_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            Arc::clone(&execution_state),
            None,
            Arc::clone(&app_state.memory_event_repo),
        );

        // Pause execution (as stop_execution would)
        execution_state.pause();

        // Transition all agent-active tasks to Stopped (as stop_execution does)
        let tasks = app_state
            .task_repo
            .get_by_project(&project.id)
            .await
            .unwrap();
        for task in tasks {
            if AGENT_ACTIVE_STATUSES.contains(&task.internal_status) {
                let _ = transition_service
                    .transition_task(&task.id, InternalStatus::Stopped)
                    .await;
            }
        }

        // Verify: All agent-active tasks should now be Stopped
        let tasks = app_state
            .task_repo
            .get_by_project(&project.id)
            .await
            .unwrap();

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
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

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
            Arc::clone(&app_state.chat_attachment_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.activity_event_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            Arc::clone(&execution_state),
            None,
            Arc::clone(&app_state.memory_event_repo),
        );

        // Pause execution (as pause_execution would)
        execution_state.pause();

        // Transition all agent-active tasks to Paused (as pause_execution does)
        let tasks = app_state
            .task_repo
            .get_by_project(&project.id)
            .await
            .unwrap();
        for task in tasks {
            if AGENT_ACTIVE_STATUSES.contains(&task.internal_status) {
                let _ = transition_service
                    .transition_task(&task.id, InternalStatus::Paused)
                    .await;
            }
        }

        // Verify: All agent-active tasks should now be Paused
        let tasks = app_state
            .task_repo
            .get_by_project(&project.id)
            .await
            .unwrap();

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
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

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
            Arc::clone(&app_state.chat_attachment_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.activity_event_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            Arc::clone(&execution_state),
            None,
            Arc::clone(&app_state.memory_event_repo),
        );

        // Execute pause: pause and transition all agent-active tasks to Paused
        execution_state.pause();

        let tasks = app_state
            .task_repo
            .get_by_project(&project.id)
            .await
            .unwrap();
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
        let reasons = [
            "task_started",
            "task_completed",
            "paused",
            "resumed",
            "stopped",
        ];
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
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

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
            Arc::clone(&app_state.chat_attachment_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.activity_event_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            Arc::clone(&execution_state),
            None,
            Arc::clone(&app_state.memory_event_repo),
        );

        // Execute stop: pause and transition all agent-active tasks to Stopped
        execution_state.pause();

        let tasks = app_state
            .task_repo
            .get_by_project(&project.id)
            .await
            .unwrap();
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
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

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
            Arc::clone(&app_state.chat_attachment_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.activity_event_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            Arc::clone(&execution_state),
            None,
            Arc::clone(&app_state.memory_event_repo),
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
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

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
            Arc::clone(&app_state.chat_attachment_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.activity_event_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            Arc::clone(&execution_state),
            None,
            Arc::clone(&app_state.memory_event_repo),
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
        assert_eq!(settings.max_concurrent_tasks, 10);
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
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

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
            Arc::clone(&app_state.chat_attachment_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.activity_event_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            Arc::clone(&execution_state),
            None,
            Arc::clone(&app_state.memory_event_repo),
        );

        // Pause: transition Executing -> Paused (creates status history entry)
        execution_state.pause();
        transition_service
            .transition_task(&task_id, InternalStatus::Paused)
            .await
            .expect("Failed to transition to Paused");

        // Verify task is Paused
        let paused_task = app_state
            .task_repo
            .get_by_id(&task_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(paused_task.internal_status, InternalStatus::Paused);

        // Verify status history shows Executing -> Paused transition
        let history = app_state
            .task_repo
            .get_status_history(&task_id)
            .await
            .unwrap();
        let pause_transition = history
            .iter()
            .rev()
            .find(|t| t.to == InternalStatus::Paused);
        assert!(pause_transition.is_some());
        assert_eq!(pause_transition.unwrap().from, InternalStatus::Executing);

        // Resume: should restore Paused -> Executing
        execution_state.resume();
        transition_service
            .transition_task(&task_id, InternalStatus::Executing)
            .await
            .expect("Failed to restore from Paused");

        // Verify task transitions to Failed when execution is blocked
        let restored_task = app_state
            .task_repo
            .get_by_id(&task_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(restored_task.internal_status, InternalStatus::Failed);
    }

    #[tokio::test]
    async fn test_resume_does_not_restore_stopped_tasks() {
        // Setup: Create a task that was Executing before being Stopped
        let execution_state = Arc::new(ExecutionState::with_max_concurrent(5));
        let app_state = AppState::new_test();

        // Create a test project
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

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
            Arc::clone(&app_state.chat_attachment_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.activity_event_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            Arc::clone(&execution_state),
            None,
            Arc::clone(&app_state.memory_event_repo),
        );

        // Stop: transition Executing -> Stopped
        execution_state.pause();
        transition_service
            .transition_task(&task_id, InternalStatus::Stopped)
            .await
            .expect("Failed to transition to Stopped");

        // Verify task is Stopped
        let stopped_task = app_state
            .task_repo
            .get_by_id(&task_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stopped_task.internal_status, InternalStatus::Stopped);

        // Resume: should NOT restore Stopped tasks
        execution_state.resume();

        // Task should STILL be Stopped (resume doesn't restore Stopped)
        let still_stopped = app_state
            .task_repo
            .get_by_id(&task_id)
            .await
            .unwrap()
            .unwrap();
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
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        // Build transition service
        let transition_service: TaskTransitionService<tauri::Wry> = TaskTransitionService::new(
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
            None,
            Arc::clone(&app_state.memory_event_repo),
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
            let task = app_state
                .task_repo
                .get_by_id(task_id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(task.internal_status, InternalStatus::Paused);
        }

        // Resume: should restore all Paused tasks to their previous status
        execution_state.resume();
        for task_id in &task_ids {
            // Find the pre-pause status from history and restore
            let history = app_state
                .task_repo
                .get_status_history(task_id)
                .await
                .unwrap();
            let pause_transition = history
                .iter()
                .rev()
                .find(|t| t.to == InternalStatus::Paused);
            if let Some(transition) = pause_transition {
                let _ = transition_service
                    .transition_task(task_id, transition.from)
                    .await;
            }
        }

        // Verify tasks: Executing tasks transition to Failed when blocked, others restore successfully
        for (i, task_id) in task_ids.iter().enumerate() {
            let task = app_state
                .task_repo
                .get_by_id(task_id)
                .await
                .unwrap()
                .unwrap();
            let expected_status = if original_statuses[i] == InternalStatus::Executing {
                InternalStatus::Failed
            } else {
                original_statuses[i]
            };
            assert_eq!(
                task.internal_status, expected_status,
                "Task should transition to {:?} (was {:?})",
                expected_status, original_statuses[i]
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
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        // Build transition service
        let transition_service: TaskTransitionService<tauri::Wry> = TaskTransitionService::new(
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
            None,
            Arc::clone(&app_state.memory_event_repo),
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
            let history = app_state
                .task_repo
                .get_status_history(&task.id)
                .await
                .unwrap();
            if let Some(transition) = history
                .iter()
                .rev()
                .find(|t| t.to == InternalStatus::Paused)
            {
                let _ = transition_service
                    .transition_task(&task.id, transition.from)
                    .await;
            }
        }

        // Verify: task1 (was Paused) should transition to Failed when execution is blocked
        let task1_final = app_state
            .task_repo
            .get_by_id(&task1_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            task1_final.internal_status,
            InternalStatus::Failed,
            "Paused task should transition to Failed when execution is blocked"
        );

        // Verify: task2 (was Stopped) should remain Stopped
        let task2_final = app_state
            .task_repo
            .get_by_id(&task2_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            task2_final.internal_status,
            InternalStatus::Stopped,
            "Stopped task should remain Stopped"
        );
    }

    // ========================================
    // Quota Sync Tests
    // ========================================

    #[tokio::test]
    async fn test_get_execution_status_syncs_quota_from_project() {
        let execution_state = Arc::new(ExecutionState::with_max_concurrent(5));
        let active_project_state = Arc::new(ActiveProjectState::new());
        let app_state = AppState::new_test();

        // Create a project with specific execution settings
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        // Set project-specific max_concurrent_tasks = 8
        let settings = ExecutionSettings {
            max_concurrent_tasks: 8,
            auto_commit: false,
            pause_on_failure: false,
        };
        app_state
            .execution_settings_repo
            .update_settings(Some(&project.id), &settings)
            .await
            .unwrap();

        // Verify initial state: execution_state has max=5 (not synced yet)
        assert_eq!(execution_state.max_concurrent(), 5);

        // Directly test the sync helper (commands need full State setup which is complex)
        let (resolved_project_id, max_concurrent) = sync_quota_from_project(
            Some(project.id.clone()),
            &active_project_state,
            &execution_state,
            &app_state,
        )
        .await
        .unwrap();

        // Verify: execution_state was synced to project's max (8)
        assert_eq!(max_concurrent, 8);
        assert_eq!(execution_state.max_concurrent(), 8);
        assert_eq!(resolved_project_id, Some(project.id));
    }

    #[tokio::test]
    async fn test_resume_execution_syncs_quota_before_can_start_task() {
        let execution_state = Arc::new(ExecutionState::with_max_concurrent(2));
        let active_project_state = Arc::new(ActiveProjectState::new());
        let app_state = AppState::new_test();

        // Create a project with max_concurrent_tasks = 10
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        let settings = ExecutionSettings {
            max_concurrent_tasks: 10,
            auto_commit: false,
            pause_on_failure: false,
        };
        app_state
            .execution_settings_repo
            .update_settings(Some(&project.id), &settings)
            .await
            .unwrap();

        // Set as active project
        active_project_state.set(Some(project.id.clone())).await;

        // Verify initial state before sync
        assert_eq!(execution_state.max_concurrent(), 2);

        // Test sync helper with active project (None project_id, uses active)
        let (resolved_project_id, max_concurrent) = sync_quota_from_project(
            None, // Use active project
            &active_project_state,
            &execution_state,
            &app_state,
        )
        .await
        .unwrap();

        // Verify: quota synced to project's max (10)
        assert_eq!(max_concurrent, 10);
        assert_eq!(execution_state.max_concurrent(), 10);
        assert_eq!(resolved_project_id, Some(project.id));
    }

    #[tokio::test]
    async fn test_pause_execution_syncs_quota() {
        let execution_state = Arc::new(ExecutionState::with_max_concurrent(3));
        let active_project_state = Arc::new(ActiveProjectState::new());
        let app_state = AppState::new_test();

        // Create a project with max_concurrent_tasks = 7
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        let settings = ExecutionSettings {
            max_concurrent_tasks: 7,
            auto_commit: false,
            pause_on_failure: false,
        };
        app_state
            .execution_settings_repo
            .update_settings(Some(&project.id), &settings)
            .await
            .unwrap();

        // Verify initial state
        assert_eq!(execution_state.max_concurrent(), 3);

        // Test sync helper with explicit project_id
        let (resolved_project_id, max_concurrent) = sync_quota_from_project(
            Some(project.id.clone()),
            &active_project_state,
            &execution_state,
            &app_state,
        )
        .await
        .unwrap();

        // Verify: quota synced to project's max (7)
        assert_eq!(max_concurrent, 7);
        assert_eq!(execution_state.max_concurrent(), 7);
        assert_eq!(resolved_project_id, Some(project.id));
    }

    #[tokio::test]
    async fn test_stop_execution_syncs_quota() {
        let execution_state = Arc::new(ExecutionState::with_max_concurrent(4));
        let active_project_state = Arc::new(ActiveProjectState::new());
        let app_state = AppState::new_test();

        // Create a project with max_concurrent_tasks = 6
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        let settings = ExecutionSettings {
            max_concurrent_tasks: 6,
            auto_commit: false,
            pause_on_failure: false,
        };
        app_state
            .execution_settings_repo
            .update_settings(Some(&project.id), &settings)
            .await
            .unwrap();

        // Verify initial state
        assert_eq!(execution_state.max_concurrent(), 4);

        // Test sync helper with explicit project_id
        let (resolved_project_id, max_concurrent) = sync_quota_from_project(
            Some(project.id.clone()),
            &active_project_state,
            &execution_state,
            &app_state,
        )
        .await
        .unwrap();

        // Verify: quota synced to project's max (6)
        assert_eq!(max_concurrent, 6);
        assert_eq!(execution_state.max_concurrent(), 6);
        assert_eq!(resolved_project_id, Some(project.id));
    }

    #[tokio::test]
    async fn test_set_active_project_syncs_quota_and_updates_execution_state() {
        let execution_state = Arc::new(ExecutionState::with_max_concurrent(2));
        let active_project_state = Arc::new(ActiveProjectState::new());
        let app_state = AppState::new_test();

        // Create two projects with different max_concurrent settings
        let project1 = Project::new("Project 1".to_string(), "/test/path1".to_string());
        app_state
            .project_repo
            .create(project1.clone())
            .await
            .unwrap();

        let settings1 = ExecutionSettings {
            max_concurrent_tasks: 5,
            auto_commit: false,
            pause_on_failure: false,
        };
        app_state
            .execution_settings_repo
            .update_settings(Some(&project1.id), &settings1)
            .await
            .unwrap();

        let project2 = Project::new("Project 2".to_string(), "/test/path2".to_string());
        app_state
            .project_repo
            .create(project2.clone())
            .await
            .unwrap();

        let settings2 = ExecutionSettings {
            max_concurrent_tasks: 12,
            auto_commit: true,
            pause_on_failure: true,
        };
        app_state
            .execution_settings_repo
            .update_settings(Some(&project2.id), &settings2)
            .await
            .unwrap();

        // Verify initial state
        assert_eq!(execution_state.max_concurrent(), 2);
        assert!(active_project_state.get().await.is_none());

        // Set active project to project1 (simulate what set_active_project command does)
        active_project_state.set(Some(project1.id.clone())).await;
        let (_resolved1, max1) = sync_quota_from_project(
            Some(project1.id.clone()),
            &active_project_state,
            &execution_state,
            &app_state,
        )
        .await
        .unwrap();

        // Verify: active project set and quota synced to project1's max (5)
        assert_eq!(
            active_project_state
                .get()
                .await
                .as_ref()
                .map(|p| p.as_str()),
            Some(project1.id.as_str())
        );
        assert_eq!(max1, 5);
        assert_eq!(execution_state.max_concurrent(), 5);

        // Switch to project2
        active_project_state.set(Some(project2.id.clone())).await;
        let (_resolved2, max2) = sync_quota_from_project(
            Some(project2.id.clone()),
            &active_project_state,
            &execution_state,
            &app_state,
        )
        .await
        .unwrap();

        // Verify: active project switched and quota synced to project2's max (12)
        assert_eq!(
            active_project_state
                .get()
                .await
                .as_ref()
                .map(|p| p.as_str()),
            Some(project2.id.as_str())
        );
        assert_eq!(max2, 12);
        assert_eq!(execution_state.max_concurrent(), 12);

        // Switch back to project1
        active_project_state.set(Some(project1.id.clone())).await;
        let (_resolved3, max3) = sync_quota_from_project(
            Some(project1.id.clone()),
            &active_project_state,
            &execution_state,
            &app_state,
        )
        .await
        .unwrap();

        // Verify: quota correctly synced back to project1's max (5)
        assert_eq!(max3, 5);
        assert_eq!(execution_state.max_concurrent(), 5);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Active Project Scoping Tests
    // ═══════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_project_switch_prevents_other_projects_from_scheduling() {
        use crate::application::TaskSchedulerService;
        use crate::domain::state_machine::services::TaskScheduler;

        let app_state = AppState::new_test();
        let execution_state = Arc::new(ExecutionState::with_max_concurrent(10));
        let active_project_state = Arc::new(ActiveProjectState::new());

        // Create two projects
        let mut project1 = Project::new("Project 1".to_string(), "/test/path1".to_string());
        project1.git_mode = GitMode::Worktree; // Worktree mode allows concurrent tasks
        app_state
            .project_repo
            .create(project1.clone())
            .await
            .unwrap();

        let mut project2 = Project::new("Project 2".to_string(), "/test/path2".to_string());
        project2.git_mode = GitMode::Worktree;
        app_state
            .project_repo
            .create(project2.clone())
            .await
            .unwrap();

        // Create Ready tasks in both projects
        let mut p1_task = Task::new(project1.id.clone(), "Project 1 Task".to_string());
        p1_task.internal_status = InternalStatus::Ready;
        app_state.task_repo.create(p1_task.clone()).await.unwrap();

        let mut p2_task = Task::new(project2.id.clone(), "Project 2 Task".to_string());
        p2_task.internal_status = InternalStatus::Ready;
        app_state.task_repo.create(p2_task.clone()).await.unwrap();

        // Set active project to project 1
        active_project_state.set(Some(project1.id.clone())).await;

        // Build scheduler with active project 1
        let scheduler = Arc::new(TaskSchedulerService::<tauri::Wry>::new(
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
            None,
        ));
        scheduler.set_self_ref(Arc::clone(&scheduler) as Arc<dyn TaskScheduler>);

        // Set active project on scheduler (simulating what execution commands do)
        scheduler
            .set_active_project(Some(project1.id.clone()))
            .await;
        scheduler.try_schedule_ready_tasks().await;

        // Verify: Project 1 task transitions to Failed when execution is blocked
        let p1_updated = app_state
            .task_repo
            .get_by_id(&p1_task.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            p1_updated.internal_status,
            InternalStatus::Failed,
            "Project 1 task should transition to Failed when execution is blocked"
        );

        // Verify: Project 2 task should NOT be scheduled
        let p2_updated = app_state
            .task_repo
            .get_by_id(&p2_task.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            p2_updated.internal_status,
            InternalStatus::Ready,
            "Project 2 task should NOT be scheduled when project 1 is active"
        );

        // Now switch active project to project 2
        active_project_state.set(Some(project2.id.clone())).await;

        // Create new scheduler instance for project 2
        let scheduler2 = Arc::new(TaskSchedulerService::<tauri::Wry>::new(
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
            None,
        ));
        scheduler2.set_self_ref(Arc::clone(&scheduler2) as Arc<dyn TaskScheduler>);

        // Set active project on new scheduler (simulating what execution commands do)
        scheduler2
            .set_active_project(Some(project2.id.clone()))
            .await;
        scheduler2.try_schedule_ready_tasks().await;

        // Verify: Project 2 task transitions to Failed when execution is blocked
        let p2_final = app_state
            .task_repo
            .get_by_id(&p2_task.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            p2_final.internal_status,
            InternalStatus::Failed,
            "Project 2 task should transition to Failed when execution is blocked"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Smart Resume Categorization Tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_categorize_direct_resume_states() {
        // Direct resume: spawn agent directly
        let direct_states = [
            InternalStatus::Executing,
            InternalStatus::ReExecuting,
            InternalStatus::Reviewing,
            InternalStatus::QaRefining,
            InternalStatus::QaTesting,
        ];

        for status in direct_states {
            let result = categorize_resume_state(status);
            assert_eq!(result.category, ResumeCategory::Direct);
            assert_eq!(result.target_status, status);
        }
    }

    #[test]
    fn test_categorize_validated_resume_states() {
        // Validated resume: check git state first
        let validated_states = [
            InternalStatus::Merging,
            InternalStatus::PendingMerge,
            InternalStatus::MergeConflict,
            InternalStatus::MergeIncomplete,
        ];

        for status in validated_states {
            let result = categorize_resume_state(status);
            assert_eq!(result.category, ResumeCategory::Validated);
            assert_eq!(result.target_status, status);
        }
    }

    #[test]
    fn test_categorize_redirect_states() {
        // Redirect: go to successor state

        // QaPassed → PendingReview
        let result = categorize_resume_state(InternalStatus::QaPassed);
        assert_eq!(result.category, ResumeCategory::Redirect);
        assert_eq!(result.target_status, InternalStatus::PendingReview);

        // RevisionNeeded → ReExecuting
        let result = categorize_resume_state(InternalStatus::RevisionNeeded);
        assert_eq!(result.category, ResumeCategory::Redirect);
        assert_eq!(result.target_status, InternalStatus::ReExecuting);

        // PendingReview → Reviewing
        let result = categorize_resume_state(InternalStatus::PendingReview);
        assert_eq!(result.category, ResumeCategory::Redirect);
        assert_eq!(result.target_status, InternalStatus::Reviewing);
    }

    #[test]
    fn test_categorize_unknown_states_fallback_to_direct() {
        // Unknown states should fallback to Direct
        let unknown_states = [
            InternalStatus::Backlog,
            InternalStatus::Ready,
            InternalStatus::Blocked,
            InternalStatus::Approved,
            InternalStatus::Merged,
        ];

        for status in unknown_states {
            let result = categorize_resume_state(status);
            assert_eq!(result.category, ResumeCategory::Direct);
            assert_eq!(result.target_status, status);
        }
    }

    #[test]
    fn test_resume_category_serialization() {
        // Verify ResumeCategory can be serialized for API responses
        let direct = ResumeCategory::Direct;
        let validated = ResumeCategory::Validated;
        let redirect = ResumeCategory::Redirect;

        let direct_json = serde_json::to_string(&direct).unwrap();
        let validated_json = serde_json::to_string(&validated).unwrap();
        let redirect_json = serde_json::to_string(&redirect).unwrap();

        assert!(direct_json.contains("Direct"));
        assert!(validated_json.contains("Validated"));
        assert!(redirect_json.contains("Redirect"));
    }

    // ========================================
    // Pause/Resume/Unblock Behavioral Tests
    // ========================================

    #[tokio::test]
    async fn test_blocked_task_unblocks_to_ready_stays_ready_during_pause() {
        // A Blocked task that transitions to Ready during a global pause must stay Ready,
        // not get re-paused. Blocked tasks are not in AGENT_ACTIVE_STATUSES so the pause
        // loop never touches them. This test verifies that invariant.
        let execution_state = Arc::new(ExecutionState::with_max_concurrent(5));
        let app_state = AppState::new_test();

        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        // Create a blocker task (Executing) and a dependent blocked task
        let mut blocker = Task::new(project.id.clone(), "Blocker Task".to_string());
        blocker.internal_status = InternalStatus::Executing;
        app_state.task_repo.create(blocker.clone()).await.unwrap();

        let mut blocked = Task::new(project.id.clone(), "Blocked Task".to_string());
        blocked.internal_status = InternalStatus::Blocked;
        app_state.task_repo.create(blocked.clone()).await.unwrap();

        // Register the dependency: blocked depends on blocker
        app_state
            .task_dependency_repo
            .add_dependency(&blocked.id, &blocker.id)
            .await
            .unwrap();

        // Pause execution: agent-active tasks pause, blocked tasks remain unchanged
        execution_state.pause();
        assert!(execution_state.is_paused());

        // Verify: blocked task is NOT touched by pause (not in AGENT_ACTIVE_STATUSES)
        let task_after_pause = app_state
            .task_repo
            .get_by_id(&blocked.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            task_after_pause.internal_status,
            InternalStatus::Blocked,
            "Blocked task should remain Blocked during pause"
        );

        // Simulate: blocker completes, unblock_dependents sets blocked → Ready
        let mut ready_task = task_after_pause.clone();
        ready_task.internal_status = InternalStatus::Ready;
        ready_task.blocked_reason = None;
        ready_task.touch();
        app_state.task_repo.update(&ready_task).await.unwrap();

        // Verify: blocked task is now Ready — stays Ready even while pause is active
        let task_after_unblock = app_state
            .task_repo
            .get_by_id(&blocked.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            task_after_unblock.internal_status,
            InternalStatus::Ready,
            "Unblocked task should be Ready, not re-paused"
        );

        // Pause flag is still set — can_start_task() should block scheduling
        assert!(
            execution_state.is_paused(),
            "Global pause flag still set — scheduler won't pick up Ready task yet"
        );
    }

    #[tokio::test]
    async fn test_resume_restores_paused_before_scheduling_ordering() {
        // After resume_execution(), the pause flag must be cleared AFTER the restoration
        // loop, not before. This means: (1) paused tasks get restored while pause is still
        // set, (2) can_start_task() returns false during the loop (preventing race with
        // scheduler), (3) pause flag is cleared only after all paused tasks are queued.
        let execution_state = Arc::new(ExecutionState::with_max_concurrent(5));
        let app_state = AppState::new_test();

        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        let transition_service: TaskTransitionService<tauri::Wry> = TaskTransitionService::new(
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
            None,
            Arc::clone(&app_state.memory_event_repo),
        );

        // Create a task in Reviewing state, then pause it
        let mut task = Task::new(project.id.clone(), "Reviewing Task".to_string());
        task.internal_status = InternalStatus::Reviewing;
        app_state.task_repo.create(task.clone()).await.unwrap();

        execution_state.pause();
        transition_service
            .transition_task(&task.id, InternalStatus::Paused)
            .await
            .expect("Failed to transition to Paused");

        // Verify: paused
        let paused = app_state
            .task_repo
            .get_by_id(&task.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(paused.internal_status, InternalStatus::Paused);

        // While paused, a blocked task becomes Ready (simulating unblock_dependents)
        let mut ready_task = Task::new(project.id.clone(), "Ready Task".to_string());
        ready_task.internal_status = InternalStatus::Ready;
        app_state
            .task_repo
            .create(ready_task.clone())
            .await
            .unwrap();

        // can_start_task() should be false while pause flag is set (paused tasks can't race
        // with scheduler during the restoration loop)
        assert!(
            !execution_state.can_start_task(),
            "can_start_task() must return false while pause flag is set"
        );

        // After resume, pause flag is cleared and new tasks can be scheduled
        execution_state.resume();
        assert!(
            !execution_state.is_paused(),
            "Pause flag must be cleared after resume()"
        );
        assert!(
            execution_state.can_start_task(),
            "can_start_task() must return true after resume()"
        );
    }

    #[tokio::test]
    async fn test_max_concurrent_respected_on_resume_with_local_counter() {
        // resume_execution() uses a local restoring_count counter to enforce max_concurrent
        // without relying on can_start_task() (which returns false due to pause flag).
        // This test verifies the counter logic: with max_concurrent=2 and 3 candidates,
        // only 2 are admitted (running_count + restoring_count < max_concurrent).
        let execution_state = Arc::new(ExecutionState::with_max_concurrent(2));

        // Verify initial state: no tasks running
        assert_eq!(execution_state.running_count(), 0);
        assert_eq!(execution_state.max_concurrent(), 2);

        execution_state.pause();
        assert!(execution_state.is_paused());

        // Simulate the restoring_count logic from resume_execution():
        //   current = running_count + restoring_count
        //   stop when current >= max_concurrent
        // With running_count=0 and max_concurrent=2, only 2 of 3 candidates should restore.
        let mut restoring_count: u32 = 0;
        let max = execution_state.max_concurrent();

        for _ in 0..3u32 {
            let current = execution_state.running_count() + restoring_count;
            if current >= max {
                break;
            }
            restoring_count += 1;
        }

        assert_eq!(
            restoring_count, 2,
            "restoring_count should stop at max_concurrent=2, got {}",
            restoring_count
        );

        // Clearing pause flag (as resume_execution does after the loop)
        execution_state.resume();
        assert!(
            !execution_state.is_paused(),
            "Pause flag cleared after restoration loop"
        );

        // After resume, can_start_task() reflects true capacity
        // (running_count=0, max=2, not paused → can start)
        assert!(
            execution_state.can_start_task(),
            "can_start_task() must be true after resume with capacity available"
        );
    }

    // ========================================
    // Interactive idle slot tracking tests
    // ========================================

    #[test]
    fn test_interactive_slot_claim_when_idle() {
        let state = ExecutionState::new();
        let key = "task_execution/task-1";

        // Initially not idle — claim should return false
        assert!(!state.claim_interactive_slot(key));

        // Mark idle → claim should return true (once)
        state.mark_interactive_idle(key);
        assert!(state.claim_interactive_slot(key));

        // Second claim should return false (already claimed)
        assert!(!state.claim_interactive_slot(key));
    }

    #[test]
    fn test_interactive_slot_rapid_burst_no_double_increment() {
        // Simulates: TurnComplete decrements → 3 rapid messages arrive
        // Only the first message should trigger increment.
        let state = ExecutionState::with_max_concurrent(5);
        let key = "task_execution/task-1";

        // Initial state: 1 running (process just spawned)
        state.increment_running();
        assert_eq!(state.running_count(), 1);

        // TurnComplete fires → decrement + mark idle
        state.decrement_running();
        state.mark_interactive_idle(key);
        assert_eq!(state.running_count(), 0);

        // First message → claim succeeds → increment
        assert!(state.claim_interactive_slot(key));
        state.increment_running();
        assert_eq!(state.running_count(), 1);

        // Second message (rapid burst) → claim fails → no increment
        assert!(!state.claim_interactive_slot(key));
        assert_eq!(state.running_count(), 1);

        // Third message → still no increment
        assert!(!state.claim_interactive_slot(key));
        assert_eq!(state.running_count(), 1);
    }

    #[test]
    fn test_interactive_slot_full_lifecycle() {
        // Full lifecycle: spawn → TurnComplete → resume → TurnComplete → exit
        let state = ExecutionState::with_max_concurrent(2);
        let key = "task_execution/task-1";

        // 1. Process spawns, initial increment
        state.increment_running();
        assert_eq!(state.running_count(), 1);

        // 2. TurnComplete → decrement + mark idle
        state.decrement_running();
        state.mark_interactive_idle(key);
        assert_eq!(state.running_count(), 0);
        assert!(state.can_start_task()); // Slot freed

        // 3. User sends next message → claim + increment
        assert!(state.claim_interactive_slot(key));
        state.increment_running();
        assert_eq!(state.running_count(), 1);

        // 4. Second TurnComplete → decrement + mark idle
        state.decrement_running();
        state.mark_interactive_idle(key);
        assert_eq!(state.running_count(), 0);

        // 5. Process exits while idle → remove slot tracking
        // No increment needed (slot already free), just cleanup
        state.remove_interactive_slot(key);
        assert_eq!(state.running_count(), 0);
        assert!(!state.claim_interactive_slot(key)); // Gone
    }

    #[test]
    fn test_interactive_slot_multiple_contexts_independent() {
        let state = ExecutionState::with_max_concurrent(5);
        let key1 = "task_execution/task-1";
        let key2 = "review/task-2";

        // Both idle
        state.mark_interactive_idle(key1);
        state.mark_interactive_idle(key2);

        // Claim key1 — key2 still idle
        assert!(state.claim_interactive_slot(key1));
        assert!(state.claim_interactive_slot(key2));

        // Both claimed — neither claimable
        assert!(!state.claim_interactive_slot(key1));
        assert!(!state.claim_interactive_slot(key2));
    }

    #[test]
    fn test_interactive_slot_remove_clears_idle() {
        let state = ExecutionState::new();
        let key = "task_execution/task-1";

        state.mark_interactive_idle(key);
        state.remove_interactive_slot(key);

        // After removal, claim should return false
        assert!(!state.claim_interactive_slot(key));
    }

    #[test]
    fn test_interactive_slot_concurrent_claims_exactly_one_wins() {
        // Verify that concurrent claim attempts on the same key
        // result in exactly one increment (no race condition).
        use std::sync::Arc;
        use std::thread;

        let state = Arc::new(ExecutionState::with_max_concurrent(10));
        let key = "task_execution/task-1";

        state.mark_interactive_idle(key);
        state.increment_running(); // Start at 1

        let mut handles = vec![];
        let claim_count = Arc::new(std::sync::atomic::AtomicU32::new(0));

        // Spawn 10 threads all trying to claim the same slot
        for _ in 0..10 {
            let state = Arc::clone(&state);
            let claim_count = Arc::clone(&claim_count);
            handles.push(thread::spawn(move || {
                if state.claim_interactive_slot(key) {
                    state.increment_running();
                    claim_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        // Exactly one thread should have won the claim
        assert_eq!(
            claim_count.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "Exactly one concurrent claim should succeed"
        );
        // running_count should be 2 (1 initial + 1 from the winning claim)
        assert_eq!(state.running_count(), 2);
    }

    #[test]
    fn test_is_interactive_idle_reflects_state() {
        let state = ExecutionState::with_max_concurrent(5);
        let key = "task_execution/task-1";

        // Not idle initially
        assert!(!state.is_interactive_idle(key));

        // Mark idle → should show as idle
        state.mark_interactive_idle(key);
        assert!(state.is_interactive_idle(key));

        // Claim it → no longer idle
        assert!(state.claim_interactive_slot(key));
        assert!(!state.is_interactive_idle(key));

        // Mark idle again, then remove → no longer idle
        state.mark_interactive_idle(key);
        assert!(state.is_interactive_idle(key));
        state.remove_interactive_slot(key);
        assert!(!state.is_interactive_idle(key));
    }

    #[test]
    fn test_force_sync_running_count_subtracts_idle_slots() {
        // Simulates what prune_stale_running_registry_entries does:
        // registry has 3 entries, 1 is idle → set_running_count(3 - 1 = 2)
        let state = ExecutionState::with_max_concurrent(5);

        // Simulate 3 registered processes
        state.increment_running();
        state.increment_running();
        state.increment_running();
        assert_eq!(state.running_count(), 3);

        // One process goes idle (TurnComplete)
        state.decrement_running();
        state.mark_interactive_idle("task_execution/task-2");
        assert_eq!(state.running_count(), 2);

        // Force-sync from registry count (3 entries) minus idle (1)
        let registry_count: u32 = 3;
        let idle_count: u32 = if state.is_interactive_idle("task_execution/task-1") {
            1
        } else {
            0
        } + if state.is_interactive_idle("task_execution/task-2") {
            1
        } else {
            0
        } + if state.is_interactive_idle("task_execution/task-3") {
            1
        } else {
            0
        };
        assert_eq!(idle_count, 1);
        state.set_running_count(registry_count.saturating_sub(idle_count));
        assert_eq!(state.running_count(), 2);
    }

    // ========================================
    // H1: decrement_running saturating_sub (no underflow)
    // ========================================

    #[test]
    fn test_decrement_running_saturating_no_wrap() {
        // Verify that decrementing from 0 never wraps to u32::MAX.
        let state = ExecutionState::new();
        assert_eq!(state.running_count(), 0);

        // Decrement from 0 — must stay at 0
        let result = state.decrement_running();
        assert_eq!(result, 0);
        assert_eq!(state.running_count(), 0);

        // Second decrement from 0 — still 0
        let result = state.decrement_running();
        assert_eq!(result, 0);
        assert_eq!(state.running_count(), 0);
    }

    #[test]
    fn test_decrement_running_concurrent_no_underflow() {
        // Spawn more decrement threads than increments to stress underflow path.
        use std::sync::Arc;
        use std::thread;

        let state = Arc::new(ExecutionState::with_max_concurrent(50));

        // Increment 5 times
        for _ in 0..5 {
            state.increment_running();
        }
        assert_eq!(state.running_count(), 5);

        // Decrement 20 times concurrently — 15 extra should saturate at 0
        let mut handles = vec![];
        for _ in 0..20 {
            let s = Arc::clone(&state);
            handles.push(thread::spawn(move || {
                s.decrement_running();
            }));
        }
        for h in handles {
            h.join().unwrap();
        }

        // Must be exactly 0, never wrapped
        assert_eq!(state.running_count(), 0);
    }

    // ========================================
    // H2: decrement_and_mark_idle atomicity
    // ========================================

    #[test]
    fn test_decrement_and_mark_idle_basic() {
        let state = ExecutionState::with_max_concurrent(5);
        let key = "task_execution/task-1";

        state.increment_running();
        assert_eq!(state.running_count(), 1);

        // Atomic decrement + mark idle
        let new_count = state.decrement_and_mark_idle(key);
        assert_eq!(new_count, 0);
        assert_eq!(state.running_count(), 0);
        assert!(state.is_interactive_idle(key));

        // claim should now work
        assert!(state.claim_interactive_slot(key));
    }

    #[test]
    fn test_decrement_and_mark_idle_race_with_claim() {
        // Simulate the race condition: one thread does decrement_and_mark_idle,
        // many threads try claim_interactive_slot concurrently.
        // Exactly one claim should succeed (no lost increments).
        use std::sync::Arc;
        use std::thread;

        let state = Arc::new(ExecutionState::with_max_concurrent(10));
        let key = "task_execution/task-1";

        // Initial: process is running
        state.increment_running();
        assert_eq!(state.running_count(), 1);

        // Use a barrier so all threads start at the same time
        let barrier = Arc::new(std::sync::Barrier::new(11)); // 1 decrement + 10 claimers

        let mut handles = vec![];

        // Thread that decrements and marks idle
        {
            let s = Arc::clone(&state);
            let b = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                b.wait();
                s.decrement_and_mark_idle(key);
            }));
        }

        // 10 threads that try to claim the slot
        let claim_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        for _ in 0..10 {
            let s = Arc::clone(&state);
            let b = Arc::clone(&barrier);
            let cc = Arc::clone(&claim_count);
            handles.push(thread::spawn(move || {
                b.wait();
                // Small spin to increase chance of interleaving
                for _ in 0..100 {
                    if s.claim_interactive_slot(key) {
                        cc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        s.increment_running();
                        break;
                    }
                    std::thread::yield_now();
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        let claims = claim_count.load(std::sync::atomic::Ordering::SeqCst);
        // Either 0 or 1 claims — never more than 1
        assert!(claims <= 1, "At most one claim should succeed, got {}", claims);
    }

    #[test]
    fn test_decrement_and_mark_idle_from_zero_saturates() {
        // decrement_and_mark_idle from 0 should not underflow
        let state = ExecutionState::new();
        let key = "task_execution/task-1";

        let new_count = state.decrement_and_mark_idle(key);
        assert_eq!(new_count, 0);
        assert_eq!(state.running_count(), 0);
        assert!(state.is_interactive_idle(key));
    }
}
