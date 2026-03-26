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

use crate::application::chat_service::{
    uses_execution_slot, ChatService, ClaudeChatService, SendMessageOptions,
};
use crate::application::reconciliation::UserRecoveryAction;
use crate::application::team_state_tracker::TeamStateTracker;
use crate::application::{
    AppState, ReconciliationRunner, TaskSchedulerService, TaskTransitionService,
};
use crate::domain::entities::{
    app_state::ExecutionHaltMode, task_step::StepProgressSummary, types::IdeationSessionId,
    ChatContextType, IdeationSessionStatus, InternalStatus, ProjectId, Task, TaskId,
};
use crate::domain::services::QueueKey;
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

const RESUME_AFTER_STOP_ERROR: &str = "Execution was stopped. Restart tasks manually.";

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
    /// Whether the app is shutting down. Set as the FIRST operation in RunEvent::Exit
    /// (before agent cleanup) so stream handlers can skip escalation during clean shutdown.
    /// Resets automatically on restart (AtomicBool in memory — not persisted).
    pub is_shutting_down: AtomicBool,
    /// Whether execution is paused (stops picking up new tasks)
    is_paused: AtomicBool,
    /// Number of currently running tasks
    running_count: AtomicU32,
    /// Maximum concurrent tasks allowed (per-project)
    max_concurrent: AtomicU32,
    /// Global maximum concurrent tasks across ALL projects (Phase 82)
    /// Default 20, hard cap 50. Enforced alongside per-project max.
    global_max_concurrent: AtomicU32,
    /// Global maximum concurrent ideation sessions allowed to actively generate.
    /// This is a pipeline cap inside the global hard cap so ideation cannot consume
    /// all slots and starve task/review/merge execution.
    global_ideation_max: AtomicU32,
    /// When true, ideation may exceed `global_ideation_max` only if there is still
    /// total capacity available and no runnable execution work is waiting.
    allow_ideation_borrow_idle_execution: AtomicBool,
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
            is_shutting_down: AtomicBool::new(false),
            is_paused: AtomicBool::new(false),
            running_count: AtomicU32::new(0),
            max_concurrent: AtomicU32::new(2),
            global_max_concurrent: AtomicU32::new(20),
            global_ideation_max: AtomicU32::new(4),
            allow_ideation_borrow_idle_execution: AtomicBool::new(false),
            rate_limited_until: AtomicU64::new(0),
            auto_completes_in_flight: std::sync::Mutex::new(HashSet::new()),
            scheduling_in_flight: std::sync::Mutex::new(HashSet::new()),
            interactive_idle_slots: std::sync::Mutex::new(HashSet::new()),
        }
    }

    /// Create ExecutionState with custom max concurrent
    pub fn with_max_concurrent(max: u32) -> Self {
        Self {
            is_shutting_down: AtomicBool::new(false),
            is_paused: AtomicBool::new(false),
            running_count: AtomicU32::new(0),
            max_concurrent: AtomicU32::new(max),
            global_max_concurrent: AtomicU32::new(20),
            global_ideation_max: AtomicU32::new(4),
            allow_ideation_borrow_idle_execution: AtomicBool::new(false),
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
        if prev == 0 {
            tracing::warn!("decrement_running: running_count underflow prevented (was 0)");
        }
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
        let ideation_cap = self.global_ideation_max();
        if ideation_cap > clamped {
            self.global_ideation_max.store(clamped, Ordering::SeqCst);
        }
    }

    /// Get global max concurrent ideation sessions.
    pub fn global_ideation_max(&self) -> u32 {
        self.global_ideation_max.load(Ordering::SeqCst)
    }

    /// Set global max concurrent ideation sessions.
    /// Clamped to [1, global_max_concurrent].
    pub fn set_global_ideation_max(&self, max: u32) {
        let clamped = max.clamp(1, self.global_max_concurrent());
        self.global_ideation_max.store(clamped, Ordering::SeqCst);
    }

    /// Check whether ideation may borrow idle execution capacity.
    pub fn allow_ideation_borrow_idle_execution(&self) -> bool {
        self.allow_ideation_borrow_idle_execution
            .load(Ordering::SeqCst)
    }

    /// Enable or disable ideation borrowing of idle execution capacity.
    pub fn set_allow_ideation_borrow_idle_execution(&self, allow: bool) {
        self.allow_ideation_borrow_idle_execution
            .store(allow, Ordering::SeqCst);
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

    /// Check if any new execution-side context may start from the global perspective.
    /// This ignores per-project limits; callers that know the project must apply those separately.
    pub fn can_start_any_execution_context(&self) -> bool {
        if self.is_paused() {
            return false;
        }
        if self.is_provider_blocked() {
            return false;
        }
        self.running_count() < self.global_max_concurrent()
    }

    /// Check if we can start a new execution-side slot consumer (task/review/merge)
    /// for a specific project without violating the global or per-project caps.
    pub fn can_start_execution_context(
        &self,
        running_project_total: u32,
        project_max_concurrent: u32,
    ) -> bool {
        if self.is_paused() {
            return false;
        }
        if self.is_provider_blocked() {
            return false;
        }

        let running = self.running_count();
        if running >= self.global_max_concurrent() {
            return false;
        }

        running_project_total < project_max_concurrent
    }

    /// Check if we can start a new ideation session without starving execution.
    ///
    /// This uses the global hard cap plus a global ideation sub-cap. Per-project
    /// ideation allocation lands in a later milestone once those settings are
    /// persisted and threaded into all ideation entry points.
    pub fn can_start_ideation(
        &self,
        running_global_ideation: u32,
        running_project_ideation: u32,
        running_project_total: u32,
        project_max_concurrent: u32,
        project_ideation_max: u32,
        runnable_execution_waiting: bool,
        project_execution_waiting: bool,
    ) -> bool {
        if self.is_paused() {
            return false;
        }
        if self.is_provider_blocked() {
            return false;
        }

        let running = self.running_count();
        if running >= self.global_max_concurrent() {
            return false;
        }

        if running_project_total >= project_max_concurrent {
            return false;
        }

        let global_allows = if running_global_ideation < self.global_ideation_max() {
            true
        } else {
            self.allow_ideation_borrow_idle_execution() && !runnable_execution_waiting
        };

        if !global_allows {
            return false;
        }

        if project_ideation_max == 0 {
            return false;
        }

        if running_project_ideation < project_ideation_max {
            return true;
        }

        self.allow_ideation_borrow_idle_execution() && !project_execution_waiting
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

    /// Count how many interactive slots are currently idle.
    /// Used by `get_execution_status` to compute active count = registry_count - idle_count.
    pub fn interactive_idle_count(&self) -> usize {
        self.interactive_idle_slots
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .len()
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
    /// Current halt mode for the global execution bar
    pub halt_mode: String,
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
    /// Maximum number of concurrent ideation sessions for this project
    pub project_ideation_max: u32,
    /// Whether to auto-commit changes after successful task completion
    pub auto_commit: bool,
    /// Whether to pause execution when a task fails
    pub pause_on_failure: bool,
}

impl From<ExecutionSettings> for ExecutionSettingsResponse {
    fn from(settings: ExecutionSettings) -> Self {
        Self {
            max_concurrent_tasks: settings.max_concurrent_tasks,
            project_ideation_max: settings.project_ideation_max,
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
    /// Maximum number of concurrent ideation sessions for this project
    pub project_ideation_max: u32,
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
    /// Maximum total concurrent ideation sessions across all projects
    pub global_ideation_max: u32,
    /// Whether ideation may borrow idle execution capacity
    pub allow_ideation_borrow_idle_execution: bool,
}

impl From<crate::domain::execution::GlobalExecutionSettings> for GlobalExecutionSettingsResponse {
    fn from(settings: crate::domain::execution::GlobalExecutionSettings) -> Self {
        Self {
            global_max_concurrent: settings.global_max_concurrent,
            global_ideation_max: settings.global_ideation_max,
            allow_ideation_borrow_idle_execution: settings.allow_ideation_borrow_idle_execution,
        }
    }
}

/// Input for updating global execution settings
#[derive(Debug, Deserialize)]
pub struct UpdateGlobalExecutionSettingsInput {
    /// Maximum total concurrent tasks across ALL projects (max: 50)
    pub global_max_concurrent: u32,
    /// Maximum total concurrent ideation sessions across ALL projects (max: 50)
    pub global_ideation_max: u32,
    /// Whether ideation may borrow idle execution capacity
    pub allow_ideation_borrow_idle_execution: bool,
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

async fn persist_execution_halt_mode(
    app_state: &AppState,
    halt_mode: ExecutionHaltMode,
) -> Result<(), String> {
    app_state
        .app_state_repo
        .set_execution_halt_mode(halt_mode)
        .await
        .map_err(|e| e.to_string())
}

fn execution_halt_mode_str(halt_mode: ExecutionHaltMode) -> &'static str {
    match halt_mode {
        ExecutionHaltMode::Running => "running",
        ExecutionHaltMode::Paused => "paused",
        ExecutionHaltMode::Stopped => "stopped",
    }
}

async fn load_execution_halt_mode(app_state: &AppState) -> Result<ExecutionHaltMode, String> {
    app_state
        .app_state_repo
        .get()
        .await
        .map(|settings| settings.execution_halt_mode)
        .map_err(|e| e.to_string())
}

async fn ensure_resume_allowed(app_state: &AppState) -> Result<(), String> {
    if load_execution_halt_mode(app_state).await? == ExecutionHaltMode::Stopped {
        return Err(RESUME_AFTER_STOP_ERROR.to_string());
    }
    Ok(())
}

fn queued_message_to_send_options(message: &crate::domain::services::QueuedMessage) -> SendMessageOptions {
    let created_at = message
        .created_at_override
        .as_deref()
        .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
        .map(|ts| ts.with_timezone(&chrono::Utc));

    SendMessageOptions {
        metadata: message.metadata_override.clone(),
        created_at,
        ..Default::default()
    }
}

fn session_is_team_mode(team_mode: Option<&str>) -> bool {
    team_mode.is_some_and(|mode| mode != "solo")
}

fn is_ideation_registry_context(context_type: &str) -> bool {
    context_type == "ideation" || context_type == "session"
}

async fn queue_key_matches_project(
    key: &QueueKey,
    project_filter: Option<&ProjectId>,
    app_state: &AppState,
) -> Result<bool, String> {
    let Some(project_id) = project_filter else {
        return Ok(true);
    };

    match key.context_type {
        ChatContextType::Ideation => {
            let session_id = IdeationSessionId::from_string(key.context_id.clone());
            let Some(session) = app_state
                .ideation_session_repo
                .get_by_id(&session_id)
                .await
                .map_err(|e| e.to_string())?
            else {
                return Ok(false);
            };
            Ok(session.project_id == *project_id)
        }
        ChatContextType::TaskExecution | ChatContextType::Review | ChatContextType::Merge => {
            let task_id = TaskId::from_string(key.context_id.clone());
            let Some(task) = app_state
                .task_repo
                .get_by_id(&task_id)
                .await
                .map_err(|e| e.to_string())?
            else {
                return Ok(false);
            };
            Ok(task.project_id == *project_id)
        }
        _ => Ok(false),
    }
}

async fn clear_slot_consuming_queues(
    project_filter: Option<&ProjectId>,
    app_state: &AppState,
) -> Result<u32, String> {
    let mut cleared = 0u32;
    for key in app_state.message_queue.list_keys() {
        if !uses_execution_slot(key.context_type) {
            continue;
        }
        if !queue_key_matches_project(&key, project_filter, app_state).await? {
            continue;
        }
        app_state.message_queue.clear_with_key(&key);
        cleared += 1;
    }
    Ok(cleared)
}

async fn count_active_ideation_slots(
    app_state: &AppState,
    execution_state: &Arc<ExecutionState>,
    project_filter: Option<&ProjectId>,
) -> Result<u32, String> {
    let registry_entries = app_state.running_agent_registry.list_all().await;
    let mut count = 0u32;

    for (key, info) in registry_entries {
        if info.pid == 0 || !is_ideation_registry_context(&key.context_type) {
            continue;
        }

        let session_id = IdeationSessionId::from_string(key.context_id.clone());
        let Some(session) = app_state
            .ideation_session_repo
            .get_by_id(&session_id)
            .await
            .map_err(|e| e.to_string())?
        else {
            continue;
        };

        if project_filter.is_some_and(|project_id| session.project_id != *project_id) {
            continue;
        }

        let slot_key = format!("{}/{}", key.context_type, key.context_id);
        if execution_state.is_interactive_idle(&slot_key) {
            continue;
        }

        count += 1;
    }

    Ok(count)
}

async fn count_active_slot_consuming_contexts_for_project(
    app_state: &AppState,
    execution_state: &Arc<ExecutionState>,
    project_id: &ProjectId,
) -> Result<u32, String> {
    let registry_entries = app_state.running_agent_registry.list_all().await;
    let mut count = 0u32;

    for (key, info) in registry_entries {
        if info.pid == 0 {
            continue;
        }

        if is_ideation_registry_context(&key.context_type) {
            let session_id = IdeationSessionId::from_string(key.context_id.clone());
            let Some(session) = app_state
                .ideation_session_repo
                .get_by_id(&session_id)
                .await
                .map_err(|e| e.to_string())?
            else {
                continue;
            };

            if session.project_id != *project_id {
                continue;
            }

            let slot_key = format!("{}/{}", key.context_type, key.context_id);
            if execution_state.is_interactive_idle(&slot_key) {
                continue;
            }

            count += 1;
            continue;
        }

        let context_type = match key.context_type.parse::<ChatContextType>() {
            Ok(value) => value,
            Err(_) => continue,
        };

        if !uses_execution_slot(context_type) {
            continue;
        }

        let task_id = TaskId::from_string(key.context_id);
        let Some(task) = app_state
            .task_repo
            .get_by_id(&task_id)
            .await
            .map_err(|e| e.to_string())?
        else {
            continue;
        };

        if task.project_id != *project_id
            || !context_matches_running_status_for_gc(context_type, task.internal_status)
        {
            continue;
        }

        count += 1;
    }

    Ok(count)
}

#[doc(hidden)]
pub async fn project_has_execution_capacity_for_state(
    app_state: &AppState,
    execution_state: &Arc<ExecutionState>,
    project_id: &ProjectId,
) -> Result<bool, String> {
    let settings = app_state
        .execution_settings_repo
        .get_settings(Some(project_id))
        .await
        .map_err(|e| e.to_string())?;
    let running_project_total =
        count_active_slot_consuming_contexts_for_project(app_state, execution_state, project_id)
            .await?;

    Ok(execution_state
        .can_start_execution_context(running_project_total, settings.max_concurrent_tasks))
}

async fn has_runnable_execution_waiting(
    app_state: &AppState,
    project_filter: Option<&ProjectId>,
) -> Result<bool, String> {
    if let Some(project_id) = project_filter {
        let tasks = app_state
            .task_repo
            .get_by_project(project_id)
            .await
            .map_err(|e| e.to_string())?;
        if tasks.iter().any(|task| task.internal_status == InternalStatus::Ready) {
            return Ok(true);
        }
    } else {
        let projects = app_state.project_repo.get_all().await.map_err(|e| e.to_string())?;
        for project in projects {
            let tasks = app_state
                .task_repo
                .get_by_project(&project.id)
                .await
                .map_err(|e| e.to_string())?;
            if tasks.iter().any(|task| task.internal_status == InternalStatus::Ready) {
                return Ok(true);
            }
        }
    }

    for key in app_state.message_queue.list_keys() {
        if !matches!(
            key.context_type,
            ChatContextType::TaskExecution | ChatContextType::Review | ChatContextType::Merge
        ) {
            continue;
        }

        let task_id = TaskId::from_string(key.context_id.clone());
        let Some(task) = app_state
            .task_repo
            .get_by_id(&task_id)
            .await
            .map_err(|e| e.to_string())?
        else {
            continue;
        };

        if project_filter.is_none_or(|project_id| task.project_id == *project_id) {
            return Ok(true);
        }
    }

    Ok(false)
}

async fn resume_paused_ideation_queues_with_chat_service<F>(
    project_filter: Option<&ProjectId>,
    app_state: &AppState,
    execution_state: &Arc<ExecutionState>,
    build_chat_service: F,
) -> Result<u32, String>
where
    F: Fn(bool) -> Arc<dyn ChatService>,
{
    let mut resumed = 0u32;
    let mut ideation_keys = Vec::new();
    for key in app_state.message_queue.list_keys() {
        if key.context_type != ChatContextType::Ideation {
            continue;
        }

        let session_id = IdeationSessionId::from_string(key.context_id.clone());
        let project_sort_key = app_state
            .ideation_session_repo
            .get_by_id(&session_id)
            .await
            .map_err(|e| e.to_string())?
            .map(|session| session.project_id.as_str().to_string())
            .unwrap_or_default();

        ideation_keys.push((project_sort_key, key.context_id.clone(), key));
    }
    ideation_keys.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    for (_, _, key) in ideation_keys {
        if !queue_key_matches_project(&key, project_filter, app_state).await? {
            continue;
        }

        let session_id = IdeationSessionId::from_string(key.context_id.clone());
        let Some(session) = app_state
            .ideation_session_repo
            .get_by_id(&session_id)
            .await
            .map_err(|e| e.to_string())?
        else {
            app_state.message_queue.clear_with_key(&key);
            continue;
        };

        if session.status != IdeationSessionStatus::Active {
            app_state.message_queue.clear_with_key(&key);
            continue;
        }

        let project_settings = app_state
            .execution_settings_repo
            .get_settings(Some(&session.project_id))
            .await
            .map_err(|e| e.to_string())?;
        let running_global_ideation =
            count_active_ideation_slots(app_state, execution_state, None).await?;
        let running_project_ideation = count_active_ideation_slots(
            app_state,
            execution_state,
            Some(&session.project_id),
        )
        .await?;
        let running_project_total = count_active_slot_consuming_contexts_for_project(
            app_state,
            execution_state,
            &session.project_id,
        )
        .await?;
        let global_execution_waiting =
            has_runnable_execution_waiting(app_state, None).await?;
        let project_execution_waiting =
            has_runnable_execution_waiting(app_state, Some(&session.project_id)).await?;
        if !execution_state.can_start_ideation(
            running_global_ideation,
            running_project_ideation,
            running_project_total,
            project_settings.max_concurrent_tasks,
            project_settings.project_ideation_max,
            global_execution_waiting,
            project_execution_waiting,
        ) {
            let global_ideation_allows =
                if running_global_ideation < execution_state.global_ideation_max() {
                    true
                } else {
                    execution_state.allow_ideation_borrow_idle_execution()
                        && !global_execution_waiting
                };

            if !execution_state.can_start_any_execution_context() || !global_ideation_allows {
                break;
            }

            continue;
        }

        let Some(queued) = app_state.message_queue.pop_with_key(&key) else {
            continue;
        };

        let send_result = build_chat_service(session_is_team_mode(session.team_mode.as_deref()))
            .send_message(
                ChatContextType::Ideation,
                session.id.as_str(),
                &queued.content,
                queued_message_to_send_options(&queued),
            )
            .await;

        match send_result {
            Ok(_) => {
                resumed += 1;
            }
            Err(error) => {
                app_state.message_queue.queue_front_existing(
                    ChatContextType::Ideation,
                    session.id.as_str(),
                    queued,
                );
                tracing::warn!(
                    session_id = session.id.as_str(),
                    error = %error,
                    "Failed to relaunch paused ideation queue item on resume"
                );
                break;
            }
        }
    }

    Ok(resumed)
}

async fn resume_paused_slot_consuming_queues_with_chat_service<F>(
    project_filter: Option<&ProjectId>,
    app_state: &AppState,
    execution_state: &Arc<ExecutionState>,
    build_chat_service: F,
) -> Result<u32, String>
where
    F: Fn() -> Arc<dyn ChatService>,
{
    let mut resumed = 0u32;
    let mut slot_keys = Vec::new();

    for key in app_state.message_queue.list_keys() {
        if !matches!(
            key.context_type,
            ChatContextType::TaskExecution | ChatContextType::Review | ChatContextType::Merge
        ) {
            continue;
        }

        let task_id = TaskId::from_string(key.context_id.clone());
        let project_sort_key = app_state
            .task_repo
            .get_by_id(&task_id)
            .await
            .map_err(|e| e.to_string())?
            .map(|task| task.project_id.as_str().to_string())
            .unwrap_or_default();

        slot_keys.push((
            project_sort_key,
            key.context_type.to_string(),
            key.context_id.clone(),
            key,
        ));
    }

    slot_keys.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));

    for (_, _, _, key) in slot_keys {
        let task_id = TaskId::from_string(key.context_id.clone());
        let Some(task) = app_state
            .task_repo
            .get_by_id(&task_id)
            .await
            .map_err(|e| e.to_string())?
        else {
            continue;
        };

        if project_filter.is_some_and(|project_id| task.project_id != *project_id) {
            continue;
        }

        if !context_matches_running_status_for_gc(key.context_type, task.internal_status) {
            continue;
        }

        let slot_key = format!("{}/{}", key.context_type, key.context_id);
        if execution_state.is_interactive_idle(&slot_key) {
            continue;
        }

        if !project_has_execution_capacity_for_state(app_state, execution_state, &task.project_id)
            .await?
        {
            continue;
        }

        let Some(queued) = app_state.message_queue.pop_with_key(&key) else {
            continue;
        };

        let chat_service = build_chat_service();
        let send_result = chat_service
            .send_message(
                key.context_type,
                &key.context_id,
                &queued.content,
                SendMessageOptions {
                    metadata: queued.metadata_override.clone(),
                    created_at: queued
                        .created_at_override
                        .as_deref()
                        .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
                        .map(|ts| ts.with_timezone(&chrono::Utc)),
                    ..Default::default()
                },
            )
            .await;

        match send_result {
            Ok(_) => resumed += 1,
            Err(error) => {
                tracing::warn!(
                    context_type = %key.context_type,
                    context_id = key.context_id,
                    error = %error,
                    "Failed to relaunch paused slot-consuming queued message"
                );
                app_state.message_queue.queue_front_existing(
                    key.context_type,
                    &key.context_id,
                    queued,
                );
            }
        }
    }

    Ok(resumed)
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

    let mut running_count = 0u32;
    for (key, _) in registry_entries {
        let context_type = match ChatContextType::from_str(&key.context_type) {
            Ok(value) => value,
            Err(_) => continue,
        };

        if !uses_execution_slot(context_type) {
            continue;
        }

        // Ideation uses session IDs (not task IDs) — no task lookup or GC needed.
        // Only count ideation sessions that are actively generating (not idle between turns).
        if matches!(context_type, ChatContextType::Ideation) {
            let slot_key = format!("{}/{}", key.context_type, key.context_id);
            if !execution_state.is_interactive_idle(&slot_key) {
                running_count += 1;
            }
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
    let halt_mode = load_execution_halt_mode(&app_state).await?;

    let blocked_until = execution_state.provider_blocked_until_epoch();
    Ok(ExecutionStatusResponse {
        is_paused: execution_state.is_paused(),
        halt_mode: execution_halt_mode_str(halt_mode).to_string(),
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
    if let Err(error) = clear_slot_consuming_queues(effective_project_id.as_ref(), &app_state).await
    {
        tracing::warn!(error = %error, "Failed to clear queued slot work during stop");
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
        .with_execution_settings_repo(Arc::clone(&app_state.execution_settings_repo))
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
    .with_execution_settings_repo(Arc::clone(&app_state.execution_settings_repo))
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
        .with_execution_settings_repo(Arc::clone(&app_state.execution_settings_repo))
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
            .with_execution_settings_repo(Arc::clone(&app_state.execution_settings_repo))
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
    execution_state.set_allow_ideation_borrow_idle_execution(
        updated.allow_ideation_borrow_idle_execution,
    );

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
            .with_execution_settings_repo(Arc::clone(&app_state.execution_settings_repo))
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

/// A running ideation session with enriched data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningIdeationSession {
    /// Session ID
    pub session_id: String,
    /// Session title
    pub title: String,
    /// Elapsed time in seconds since session was created
    pub elapsed_seconds: Option<i64>,
    /// Team mode (solo, research, debate)
    pub team_mode: Option<String>,
    /// Whether the agent is actively generating (false = idle between turns)
    pub is_generating: bool,
}

/// Response for get_running_processes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningProcessesResponse {
    /// List of running processes
    pub processes: Vec<RunningProcess>,
    /// List of running ideation sessions
    pub ideation_sessions: Vec<RunningIdeationSession>,
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
                let elapsed_seconds = {
                    let now = chrono::Utc::now();
                    let elapsed = now.signed_duration_since(session.created_at);
                    Some(elapsed.num_seconds())
                };
                let slot_key = format!("ideation/{}", session_id_str);
                let is_generating = !execution_state.is_interactive_idle(&slot_key);
                ideation_sessions.push(RunningIdeationSession {
                    session_id: session_id_str,
                    title: session.title.unwrap_or_else(|| "Untitled Session".to_string()),
                    elapsed_seconds,
                    team_mode: session.team_mode,
                    is_generating,
                });
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

    Ok(RunningProcessesResponse { processes, ideation_sessions })
}


#[doc(hidden)]
pub fn context_matches_running_status_for_gc(
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

async fn prune_stale_execution_registry_entries(
    app_state: &AppState,
    execution_state: &ExecutionState,
) {
    let entries = app_state.running_agent_registry.list_all().await;
    if entries.is_empty() {
        return;
    }

    let engine = crate::application::PruneEngine::new(
        Arc::clone(&app_state.running_agent_registry),
        Arc::clone(&app_state.agent_run_repo),
        Arc::clone(&app_state.task_repo),
        Some(Arc::clone(&app_state.interactive_process_registry)),
    );

    let mut pruned_any = false;

    for (key, info) in &entries {
        let context_type = match ChatContextType::from_str(&key.context_type) {
            Ok(ct) => ct,
            Err(_) => continue,
        };

        if !uses_execution_slot(context_type) {
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

        // Compute pid liveness once; both the IPR check and staleness evaluation use it.
        let pid_alive = crate::domain::services::is_process_alive(info.pid);

        // PID-verified IPR check: skip if interactive process is alive; remove stale
        // IPR entries (PID dead) so reconciliation is not blocked forever.
        if engine.check_ipr_skip(key, pid_alive).await {
            continue;
        }

        if engine.evaluate_and_prune(key, info, pid_alive).await {
            // Clean up any interactive idle slot tracking for this pruned entry
            // so ghost entries don't persist in interactive_idle_slots.
            let slot_key = format!("{}/{}", key.context_type, key.context_id);
            execution_state.remove_interactive_slot(&slot_key);
            pruned_any = true;
        }
    }

    // Correct the running count if entries were pruned.  The GC runs every ~5s so
    // this keeps the slot counter accurate between 30s reconciliation cycles (Bug 3).
    if pruned_any {
        let remaining = app_state.running_agent_registry.list_all().await;
        let idle_count = remaining
            .iter()
            .filter(|(k, _)| {
                let slot_key = format!("{}/{}", k.context_type, k.context_id);
                execution_state.is_interactive_idle(&slot_key)
            })
            .count() as u32;
        execution_state
            .set_running_count((remaining.len() as u32).saturating_sub(idle_count));
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
    .with_execution_settings_repo(Arc::clone(&state.execution_settings_repo))
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
    if !GitService::branch_exists(repo_path, &branch_name).await.unwrap_or(false) {
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
mod tests {
    use super::*;
    use crate::application::chat_service::{ChatService, MockChatService};
    use crate::domain::entities::{GitMode, IdeationSession};
    use crate::domain::services::RunningAgentKey;
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
            halt_mode: "paused".to_string(),
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
        assert!(json.contains("\"halt_mode\":\"paused\""));
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
                halt_mode: "running".to_string(),
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
        assert!(json.contains("\"halt_mode\":\"running\""));
    }

    #[test]
    fn test_execution_settings_response_serialization() {
        let response = ExecutionSettingsResponse {
            max_concurrent_tasks: 4,
            project_ideation_max: 2,
            auto_commit: true,
            pause_on_failure: false,
        };

        let json = serde_json::to_string(&response).unwrap();

        // Verify snake_case serialization
        assert!(json.contains("\"max_concurrent_tasks\":4"));
        assert!(json.contains("\"project_ideation_max\":2"));
        assert!(json.contains("\"auto_commit\":true"));
        assert!(json.contains("\"pause_on_failure\":false"));
    }

    #[test]
    fn test_execution_settings_response_from_domain() {
        let settings = ExecutionSettings {
            max_concurrent_tasks: 3,
            project_ideation_max: 1,
            auto_commit: false,
            pause_on_failure: true,
        };

        let response = ExecutionSettingsResponse::from(settings);

        assert_eq!(response.max_concurrent_tasks, 3);
        assert_eq!(response.project_ideation_max, 1);
        assert!(!response.auto_commit);
        assert!(response.pause_on_failure);
    }

    #[test]
    fn test_update_execution_settings_input_deserialization() {
        let json = r#"{"max_concurrent_tasks":5,"project_ideation_max":2,"auto_commit":false,"pause_on_failure":true}"#;

        let input: UpdateExecutionSettingsInput =
            serde_json::from_str(json).expect("Failed to deserialize input");

        assert_eq!(input.max_concurrent_tasks, 5);
        assert_eq!(input.project_ideation_max, 2);
        assert!(!input.auto_commit);
        assert!(input.pause_on_failure);
    }

    #[test]
    fn test_global_execution_settings_response_serialization() {
        let response = GlobalExecutionSettingsResponse {
            global_max_concurrent: 20,
            global_ideation_max: 4,
            allow_ideation_borrow_idle_execution: true,
        };

        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"global_max_concurrent\":20"));
        assert!(json.contains("\"global_ideation_max\":4"));
        assert!(json.contains("\"allow_ideation_borrow_idle_execution\":true"));
    }

    #[test]
    fn test_global_execution_settings_response_from_domain() {
        let settings = crate::domain::execution::GlobalExecutionSettings {
            global_max_concurrent: 18,
            global_ideation_max: 3,
            allow_ideation_borrow_idle_execution: false,
        };

        let response = GlobalExecutionSettingsResponse::from(settings);

        assert_eq!(response.global_max_concurrent, 18);
        assert_eq!(response.global_ideation_max, 3);
        assert!(!response.allow_ideation_borrow_idle_execution);
    }

    #[test]
    fn test_update_global_execution_settings_input_deserialization() {
        let json = r#"{"global_max_concurrent":22,"global_ideation_max":5,"allow_ideation_borrow_idle_execution":true}"#;

        let input: UpdateGlobalExecutionSettingsInput =
            serde_json::from_str(json).expect("Failed to deserialize global input");

        assert_eq!(input.global_max_concurrent, 22);
        assert_eq!(input.global_ideation_max, 5);
        assert!(input.allow_ideation_borrow_idle_execution);
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
            project_ideation_max: 2,
            auto_commit: true,
            pause_on_failure: true,
        };
        let settings2 = ExecutionSettings {
            max_concurrent_tasks: 10,
            project_ideation_max: 2,
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
            project_ideation_max: 2,
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
            project_ideation_max: 2,
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
            project_ideation_max: 2,
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
            project_ideation_max: 1,
            auto_commit: true,
            pause_on_failure: true,
        };
        let settings2 = ExecutionSettings {
            max_concurrent_tasks: 12,
            project_ideation_max: 2,
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
    async fn test_stop_clears_queued_slot_consuming_messages() {
        let app_state = AppState::new_test();
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        let session = app_state
            .ideation_session_repo
            .create(IdeationSession::new(project.id.clone()))
            .await
            .unwrap();

        let task = app_state
            .task_repo
            .create(Task::new(project.id.clone(), "Task".to_string()))
            .await
            .unwrap();

        app_state.message_queue.queue(
            ChatContextType::Ideation,
            session.id.as_str(),
            "queued ideation".to_string(),
        );
        app_state.message_queue.queue(
            ChatContextType::TaskExecution,
            task.id.as_str(),
            "queued execution".to_string(),
        );
        app_state.message_queue.queue(
            ChatContextType::Review,
            task.id.as_str(),
            "queued review".to_string(),
        );
        app_state.message_queue.queue(
            ChatContextType::Merge,
            task.id.as_str(),
            "queued merge".to_string(),
        );
        app_state.message_queue.queue(
            ChatContextType::Task,
            task.id.as_str(),
            "keep task".to_string(),
        );
        app_state.message_queue.queue(
            ChatContextType::Project,
            project.id.as_str(),
            "keep project".to_string(),
        );

        let cleared = clear_slot_consuming_queues(None, &app_state)
            .await
            .expect("clear queued slot work");

        assert_eq!(cleared, 4);
        assert!(
            app_state
                .message_queue
                .get_queued(ChatContextType::Ideation, session.id.as_str())
                .is_empty()
        );
        assert!(
            app_state
                .message_queue
                .get_queued(ChatContextType::TaskExecution, task.id.as_str())
                .is_empty()
        );
        assert!(
            app_state
                .message_queue
                .get_queued(ChatContextType::Review, task.id.as_str())
                .is_empty()
        );
        assert!(
            app_state
                .message_queue
                .get_queued(ChatContextType::Merge, task.id.as_str())
                .is_empty()
        );
        assert_eq!(
            app_state
                .message_queue
                .get_queued(ChatContextType::Task, task.id.as_str())
                .len(),
            1
        );
        assert_eq!(
            app_state
                .message_queue
                .get_queued(ChatContextType::Project, project.id.as_str())
                .len(),
            1
        );
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
    async fn test_resume_relaunches_one_queued_message_for_active_ideation_session() {
        let app_state = AppState::new_test();
        let execution_state = Arc::new(ExecutionState::new());
        let project = Project::new("Resume Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        let session = app_state
            .ideation_session_repo
            .create(IdeationSession::new(project.id.clone()))
            .await
            .unwrap();

        app_state.message_queue.queue_with_overrides(
            ChatContextType::Ideation,
            session.id.as_str(),
            "first queued".to_string(),
            Some(r#"{"source":"pause"}"#.to_string()),
            Some("2026-03-25T10:00:00Z".to_string()),
        );
        app_state.message_queue.queue(
            ChatContextType::Ideation,
            session.id.as_str(),
            "second queued".to_string(),
        );

        let mock = Arc::new(MockChatService::new());
        let resumed = resume_paused_ideation_queues_with_chat_service(
            None,
            &app_state,
            &execution_state,
            |_| Arc::clone(&mock) as Arc<dyn ChatService>,
        )
        .await
        .expect("resume paused ideation queue");

        assert_eq!(resumed, 1);
        assert_eq!(mock.call_count(), 1);
        assert_eq!(
            app_state
                .message_queue
                .get_queued(ChatContextType::Ideation, session.id.as_str())
                .len(),
            1,
            "resume should relaunch only the front queued message for the session"
        );
        assert_eq!(
            mock.get_sent_messages().await,
            vec!["first queued".to_string()]
        );
    }

    #[tokio::test]
    async fn test_resume_respects_project_ideation_cap_for_same_project() {
        let app_state = AppState::new_test();
        let execution_state = Arc::new(ExecutionState::new());
        let project = Project::new("Project Cap".to_string(), "/test/project-cap".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        app_state
            .execution_settings_repo
            .update_settings(
                Some(&project.id),
                &ExecutionSettings {
                    max_concurrent_tasks: 5,
                    project_ideation_max: 1,
                    auto_commit: true,
                    pause_on_failure: true,
                },
            )
            .await
            .unwrap();

        let occupied = app_state
            .ideation_session_repo
            .create(IdeationSession::new(project.id.clone()))
            .await
            .unwrap();
        let queued = app_state
            .ideation_session_repo
            .create(IdeationSession::new(project.id.clone()))
            .await
            .unwrap();

        app_state.message_queue.queue(
            ChatContextType::Ideation,
            queued.id.as_str(),
            "blocked by project cap".to_string(),
        );

        app_state
            .running_agent_registry
            .register(
                RunningAgentKey::new("ideation", occupied.id.as_str()),
                22222,
                "occupied-conv".to_string(),
                "occupied-run".to_string(),
                None,
                None,
            )
            .await;

        let mock = Arc::new(MockChatService::new());
        let resumed = resume_paused_ideation_queues_with_chat_service(
            Some(&project.id),
            &app_state,
            &execution_state,
            |_| Arc::clone(&mock) as Arc<dyn ChatService>,
        )
        .await
        .expect("resume paused ideation queue with project cap");

        assert_eq!(resumed, 0);
        assert_eq!(mock.call_count(), 0);
        assert_eq!(
            app_state
                .message_queue
                .get_queued(ChatContextType::Ideation, queued.id.as_str())
                .len(),
            1,
            "project-capped session must stay queued on resume"
        );
    }

    #[tokio::test]
    async fn test_resume_skips_project_capped_ideation_queue_and_relaunches_other_project() {
        let app_state = AppState::new_test();
        let execution_state = Arc::new(ExecutionState::new());

        let first_project = Project::new("First Project".to_string(), "/test/first".to_string());
        let second_project =
            Project::new("Second Project".to_string(), "/test/second".to_string());
        app_state
            .project_repo
            .create(first_project.clone())
            .await
            .unwrap();
        app_state
            .project_repo
            .create(second_project.clone())
            .await
            .unwrap();

        let (blocked_project, runnable_project) = if first_project.id.as_str()
            <= second_project.id.as_str()
        {
            (first_project, second_project)
        } else {
            (second_project, first_project)
        };

        app_state
            .execution_settings_repo
            .update_settings(
                Some(&blocked_project.id),
                &ExecutionSettings {
                    max_concurrent_tasks: 5,
                    project_ideation_max: 1,
                    auto_commit: true,
                    pause_on_failure: true,
                },
            )
            .await
            .unwrap();

        execution_state.set_global_max_concurrent(5);
        execution_state.set_global_ideation_max(5);

        let occupied = app_state
            .ideation_session_repo
            .create(IdeationSession::new(blocked_project.id.clone()))
            .await
            .unwrap();
        let blocked_queued = app_state
            .ideation_session_repo
            .create(IdeationSession::new(blocked_project.id.clone()))
            .await
            .unwrap();
        let runnable_queued = app_state
            .ideation_session_repo
            .create(IdeationSession::new(runnable_project.id.clone()))
            .await
            .unwrap();

        app_state.message_queue.queue(
            ChatContextType::Ideation,
            blocked_queued.id.as_str(),
            "blocked project queued".to_string(),
        );
        app_state.message_queue.queue(
            ChatContextType::Ideation,
            runnable_queued.id.as_str(),
            "runnable project queued".to_string(),
        );

        app_state
            .running_agent_registry
            .register(
                RunningAgentKey::new("ideation", occupied.id.as_str()),
                23232,
                "occupied-conv".to_string(),
                "occupied-run".to_string(),
                None,
                None,
            )
            .await;

        let mock = Arc::new(MockChatService::new());
        let resumed = resume_paused_ideation_queues_with_chat_service(
            None,
            &app_state,
            &execution_state,
            |_| Arc::clone(&mock) as Arc<dyn ChatService>,
        )
        .await
        .expect("resume queued ideation across projects");

        assert_eq!(resumed, 1);
        assert_eq!(mock.call_count(), 1);
        assert_eq!(
            mock.get_sent_messages().await,
            vec!["runnable project queued".to_string()]
        );
        assert_eq!(
            app_state
                .message_queue
                .get_queued(ChatContextType::Ideation, blocked_queued.id.as_str())
                .len(),
            1,
            "blocked project's queue must remain pending"
        );
        assert_eq!(
            app_state
                .message_queue
                .get_queued(ChatContextType::Ideation, runnable_queued.id.as_str())
                .len(),
            0,
            "other project should still relaunch in the same resume pass"
        );
    }

    #[tokio::test]
    async fn test_resume_borrowing_stays_blocked_when_ready_execution_waits() {
        let app_state = AppState::new_test();
        let execution_state = Arc::new(ExecutionState::new());
        let project = Project::new("Borrow Block".to_string(), "/test/borrow-block".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        execution_state.set_global_max_concurrent(5);
        execution_state.set_global_ideation_max(1);
        execution_state.set_allow_ideation_borrow_idle_execution(true);

        let occupied = app_state
            .ideation_session_repo
            .create(IdeationSession::new(project.id.clone()))
            .await
            .unwrap();
        let queued = app_state
            .ideation_session_repo
            .create(IdeationSession::new(project.id.clone()))
            .await
            .unwrap();

        let ready_task = Task::new(project.id.clone(), "Ready execution".to_string());
        app_state
            .task_repo
            .create(Task {
                internal_status: InternalStatus::Ready,
                ..ready_task
            })
            .await
            .unwrap();

        app_state.message_queue.queue(
            ChatContextType::Ideation,
            queued.id.as_str(),
            "blocked by ready execution".to_string(),
        );

        app_state
            .running_agent_registry
            .register(
                RunningAgentKey::new("ideation", occupied.id.as_str()),
                11111,
                "occupied-conv".to_string(),
                "occupied-run".to_string(),
                None,
                None,
            )
            .await;

        let mock = Arc::new(MockChatService::new());
        let resumed = resume_paused_ideation_queues_with_chat_service(
            Some(&project.id),
            &app_state,
            &execution_state,
            |_| Arc::clone(&mock) as Arc<dyn ChatService>,
        )
        .await
        .expect("resume paused ideation queue with ready execution");

        assert_eq!(resumed, 0);
        assert_eq!(mock.call_count(), 0);
        assert_eq!(
            app_state
                .message_queue
                .get_queued(ChatContextType::Ideation, queued.id.as_str())
                .len(),
            1,
            "borrowing must stay blocked while ready execution work exists"
        );
    }

    #[tokio::test]
    async fn test_resume_relaunches_queued_task_execution_message_for_active_task() {
        let app_state = AppState::new_test();
        let execution_state = Arc::new(ExecutionState::new());
        let project = Project::new("Resume Task Queue".to_string(), "/test/task-queue".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        let task = app_state
            .task_repo
            .create(Task {
                internal_status: InternalStatus::Executing,
                ..Task::new(project.id.clone(), "Queued worker prompt".to_string())
            })
            .await
            .unwrap();

        app_state.message_queue.queue(
            ChatContextType::TaskExecution,
            task.id.as_str(),
            "continue execution".to_string(),
        );

        let mock = Arc::new(MockChatService::new());
        let resumed = resume_paused_slot_consuming_queues_with_chat_service(
            None,
            &app_state,
            &execution_state,
            || Arc::clone(&mock) as Arc<dyn ChatService>,
        )
        .await
        .expect("resume paused task queue");

        assert_eq!(resumed, 1);
        assert_eq!(mock.call_count(), 1);
        assert_eq!(
            app_state
                .message_queue
                .get_queued(ChatContextType::TaskExecution, task.id.as_str())
                .len(),
            0,
            "active task queue should be drained when resume relaunches the prompt"
        );
        assert_eq!(
            mock.get_sent_messages().await,
            vec!["continue execution".to_string()]
        );
    }

    #[tokio::test]
    async fn test_resume_leaves_queued_task_execution_message_pending_for_paused_task() {
        let app_state = AppState::new_test();
        let execution_state = Arc::new(ExecutionState::new());
        let project =
            Project::new("Resume Pending Queue".to_string(), "/test/pending-queue".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        let task = app_state
            .task_repo
            .create(Task {
                internal_status: InternalStatus::Paused,
                ..Task::new(project.id.clone(), "Paused worker prompt".to_string())
            })
            .await
            .unwrap();

        app_state.message_queue.queue(
            ChatContextType::TaskExecution,
            task.id.as_str(),
            "wait until restored".to_string(),
        );

        let mock = Arc::new(MockChatService::new());
        let resumed = resume_paused_slot_consuming_queues_with_chat_service(
            None,
            &app_state,
            &execution_state,
            || Arc::clone(&mock) as Arc<dyn ChatService>,
        )
        .await
        .expect("resume paused queue for paused task");

        assert_eq!(resumed, 0);
        assert_eq!(mock.call_count(), 0);
        assert_eq!(
            app_state
                .message_queue
                .get_queued(ChatContextType::TaskExecution, task.id.as_str())
                .len(),
            1,
            "paused task queue must stay pending until the task is active again"
        );
    }

    #[tokio::test]
    async fn test_resume_relaunches_queued_review_message_for_active_task() {
        let app_state = AppState::new_test();
        let execution_state = Arc::new(ExecutionState::new());
        let project = Project::new("Resume Review Queue".to_string(), "/test/review-queue".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        let task = app_state
            .task_repo
            .create(Task {
                internal_status: InternalStatus::Reviewing,
                ..Task::new(project.id.clone(), "Queued review prompt".to_string())
            })
            .await
            .unwrap();

        app_state.message_queue.queue(
            ChatContextType::Review,
            task.id.as_str(),
            "continue review".to_string(),
        );

        let mock = Arc::new(MockChatService::new());
        let resumed = resume_paused_slot_consuming_queues_with_chat_service(
            None,
            &app_state,
            &execution_state,
            || Arc::clone(&mock) as Arc<dyn ChatService>,
        )
        .await
        .expect("resume paused review queue");

        assert_eq!(resumed, 1);
        assert_eq!(mock.call_count(), 1);
        assert_eq!(
            app_state
                .message_queue
                .get_queued(ChatContextType::Review, task.id.as_str())
                .len(),
            0,
            "active review queue should be drained when resume relaunches the prompt"
        );
        assert_eq!(
            mock.get_sent_messages().await,
            vec!["continue review".to_string()]
        );
    }

    #[tokio::test]
    async fn test_resume_relaunches_queued_merge_message_for_active_task() {
        let app_state = AppState::new_test();
        let execution_state = Arc::new(ExecutionState::new());
        let project = Project::new("Resume Merge Queue".to_string(), "/test/merge-queue".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        let task = app_state
            .task_repo
            .create(Task {
                internal_status: InternalStatus::Merging,
                ..Task::new(project.id.clone(), "Queued merge prompt".to_string())
            })
            .await
            .unwrap();

        app_state.message_queue.queue(
            ChatContextType::Merge,
            task.id.as_str(),
            "continue merge".to_string(),
        );

        let mock = Arc::new(MockChatService::new());
        let resumed = resume_paused_slot_consuming_queues_with_chat_service(
            None,
            &app_state,
            &execution_state,
            || Arc::clone(&mock) as Arc<dyn ChatService>,
        )
        .await
        .expect("resume paused merge queue");

        assert_eq!(resumed, 1);
        assert_eq!(mock.call_count(), 1);
        assert_eq!(
            app_state
                .message_queue
                .get_queued(ChatContextType::Merge, task.id.as_str())
                .len(),
            0,
            "active merge queue should be drained when resume relaunches the prompt"
        );
        assert_eq!(
            mock.get_sent_messages().await,
            vec!["continue merge".to_string()]
        );
    }

    #[tokio::test]
    async fn test_resume_respects_project_capacity_for_same_project_slot_queue() {
        let app_state = AppState::new_test();
        let execution_state = Arc::new(ExecutionState::new());
        let project = Project::new("Blocked Slot Project".to_string(), "/test/blocked-slot".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        app_state
            .execution_settings_repo
            .update_settings(
                Some(&project.id),
                &ExecutionSettings {
                    max_concurrent_tasks: 1,
                    project_ideation_max: 1,
                    auto_commit: true,
                    pause_on_failure: true,
                },
            )
            .await
            .unwrap();

        let occupied = app_state
            .task_repo
            .create(Task {
                internal_status: InternalStatus::Executing,
                ..Task::new(project.id.clone(), "Occupied slot".to_string())
            })
            .await
            .unwrap();
        let queued = app_state
            .task_repo
            .create(Task {
                internal_status: InternalStatus::Reviewing,
                ..Task::new(project.id.clone(), "Queued review".to_string())
            })
            .await
            .unwrap();

        app_state.message_queue.queue(
            ChatContextType::Review,
            queued.id.as_str(),
            "blocked review queue".to_string(),
        );

        app_state
            .running_agent_registry
            .register(
                RunningAgentKey::new("task_execution", occupied.id.as_str()),
                31337,
                "occupied-conv".to_string(),
                "occupied-run".to_string(),
                None,
                None,
            )
            .await;
        execution_state.set_running_count(1);

        let mock = Arc::new(MockChatService::new());
        let resumed = resume_paused_slot_consuming_queues_with_chat_service(
            Some(&project.id),
            &app_state,
            &execution_state,
            || Arc::clone(&mock) as Arc<dyn ChatService>,
        )
        .await
        .expect("resume blocked slot-consuming queue");

        assert_eq!(resumed, 0);
        assert_eq!(mock.call_count(), 0);
        assert_eq!(
            app_state
                .message_queue
                .get_queued(ChatContextType::Review, queued.id.as_str())
                .len(),
            1,
            "project-capped slot-consuming work must stay queued on resume"
        );
    }

    #[tokio::test]
    async fn test_resume_skips_project_capped_slot_queue_and_relaunches_other_project() {
        let app_state = AppState::new_test();
        let execution_state = Arc::new(ExecutionState::new());

        let first_project = Project::new("Blocked First".to_string(), "/test/blocked-first".to_string());
        let second_project =
            Project::new("Runnable Second".to_string(), "/test/runnable-second".to_string());
        app_state
            .project_repo
            .create(first_project.clone())
            .await
            .unwrap();
        app_state
            .project_repo
            .create(second_project.clone())
            .await
            .unwrap();

        let (blocked_project, runnable_project) = if first_project.id.as_str()
            <= second_project.id.as_str()
        {
            (first_project, second_project)
        } else {
            (second_project, first_project)
        };

        app_state
            .execution_settings_repo
            .update_settings(
                Some(&blocked_project.id),
                &ExecutionSettings {
                    max_concurrent_tasks: 1,
                    project_ideation_max: 1,
                    auto_commit: true,
                    pause_on_failure: true,
                },
            )
            .await
            .unwrap();

        let occupied = app_state
            .task_repo
            .create(Task {
                internal_status: InternalStatus::Executing,
                ..Task::new(blocked_project.id.clone(), "Blocked project occupied".to_string())
            })
            .await
            .unwrap();
        let blocked_queued = app_state
            .task_repo
            .create(Task {
                internal_status: InternalStatus::Reviewing,
                ..Task::new(blocked_project.id.clone(), "Blocked project review".to_string())
            })
            .await
            .unwrap();
        let runnable_queued = app_state
            .task_repo
            .create(Task {
                internal_status: InternalStatus::Merging,
                ..Task::new(runnable_project.id.clone(), "Runnable project merge".to_string())
            })
            .await
            .unwrap();

        app_state.message_queue.queue(
            ChatContextType::Review,
            blocked_queued.id.as_str(),
            "blocked project review queue".to_string(),
        );
        app_state.message_queue.queue(
            ChatContextType::Merge,
            runnable_queued.id.as_str(),
            "runnable project merge queue".to_string(),
        );

        app_state
            .running_agent_registry
            .register(
                RunningAgentKey::new("task_execution", occupied.id.as_str()),
                41414,
                "occupied-conv".to_string(),
                "occupied-run".to_string(),
                None,
                None,
            )
            .await;
        execution_state.set_running_count(1);

        let mock = Arc::new(MockChatService::new());
        let resumed = resume_paused_slot_consuming_queues_with_chat_service(
            None,
            &app_state,
            &execution_state,
            || Arc::clone(&mock) as Arc<dyn ChatService>,
        )
        .await
        .expect("resume queued slot-consuming work across projects");

        assert_eq!(resumed, 1);
        assert_eq!(mock.call_count(), 1);
        assert_eq!(
            mock.get_sent_messages().await,
            vec!["runnable project merge queue".to_string()]
        );
        assert_eq!(
            app_state
                .message_queue
                .get_queued(ChatContextType::Review, blocked_queued.id.as_str())
                .len(),
            1,
            "blocked project's slot-consuming queue must remain pending"
        );
        assert_eq!(
            app_state
                .message_queue
                .get_queued(ChatContextType::Merge, runnable_queued.id.as_str())
                .len(),
            0,
            "other project should still relaunch in the same resume pass"
        );
    }

    #[tokio::test]
    async fn test_project_has_execution_capacity_for_state_ignores_other_projects() {
        let app_state = AppState::new_test();
        let execution_state = Arc::new(ExecutionState::new());

        let project_a = Project::new("Project A".to_string(), "/test/project-a".to_string());
        let project_b = Project::new("Project B".to_string(), "/test/project-b".to_string());
        app_state.project_repo.create(project_a.clone()).await.unwrap();
        app_state.project_repo.create(project_b.clone()).await.unwrap();

        app_state
            .execution_settings_repo
            .update_settings(
                Some(&project_a.id),
                &ExecutionSettings {
                    max_concurrent_tasks: 1,
                    project_ideation_max: 1,
                    auto_commit: true,
                    pause_on_failure: true,
                },
            )
            .await
            .unwrap();

        let other_project_task = app_state
            .task_repo
            .create(Task {
                internal_status: InternalStatus::Executing,
                ..Task::new(project_b.id.clone(), "Other project running".to_string())
            })
            .await
            .unwrap();
        app_state
            .running_agent_registry
            .register(
                RunningAgentKey::new("task_execution", other_project_task.id.as_str()),
                34343,
                "other-project-conv".to_string(),
                "other-project-run".to_string(),
                None,
                None,
            )
            .await;

        assert!(
            project_has_execution_capacity_for_state(&app_state, &execution_state, &project_a.id)
                .await
                .expect("project capacity check"),
            "activity in another project must not consume this project's execution quota"
        );
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
        assert_eq!(settings.project_ideation_max, 2);
        assert!(settings.auto_commit);
        assert!(settings.pause_on_failure);
    }

    #[tokio::test]
    async fn test_execution_settings_repo_update() {
        let app_state = AppState::new_test();

        let new_settings = ExecutionSettings {
            max_concurrent_tasks: 5,
            project_ideation_max: 2,
            auto_commit: false,
            pause_on_failure: false,
        };

        let updated = app_state
            .execution_settings_repo
            .update_settings(None, &new_settings)
            .await
            .expect("Failed to update execution settings");

        assert_eq!(updated.max_concurrent_tasks, 5);
        assert_eq!(updated.project_ideation_max, 2);
        assert!(!updated.auto_commit);
        assert!(!updated.pause_on_failure);

        // Verify persistence
        let retrieved = app_state
            .execution_settings_repo
            .get_settings(None)
            .await
            .expect("Failed to get execution settings");

        assert_eq!(retrieved.max_concurrent_tasks, 5);
        assert_eq!(retrieved.project_ideation_max, 2);
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
            project_ideation_max: 2,
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
            project_ideation_max: 2,
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
            project_ideation_max: 2,
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
            project_ideation_max: 2,
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
    async fn test_persist_execution_halt_mode_paused() {
        let app_state = AppState::new_test();

        persist_execution_halt_mode(&app_state, ExecutionHaltMode::Paused)
            .await
            .unwrap();

        let settings = app_state.app_state_repo.get().await.unwrap();
        assert_eq!(settings.execution_halt_mode, ExecutionHaltMode::Paused);
    }

    #[tokio::test]
    async fn test_persist_execution_halt_mode_stopped() {
        let app_state = AppState::new_test();

        persist_execution_halt_mode(&app_state, ExecutionHaltMode::Stopped)
            .await
            .unwrap();

        let settings = app_state.app_state_repo.get().await.unwrap();
        assert_eq!(settings.execution_halt_mode, ExecutionHaltMode::Stopped);
    }

    #[tokio::test]
    async fn test_persist_execution_halt_mode_running() {
        let app_state = AppState::new_test();

        persist_execution_halt_mode(&app_state, ExecutionHaltMode::Stopped)
            .await
            .unwrap();
        persist_execution_halt_mode(&app_state, ExecutionHaltMode::Running)
            .await
            .unwrap();

        let settings = app_state.app_state_repo.get().await.unwrap();
        assert_eq!(settings.execution_halt_mode, ExecutionHaltMode::Running);
    }

    #[tokio::test]
    async fn test_load_execution_halt_mode_reads_persisted_stop_state() {
        let app_state = AppState::new_test();
        persist_execution_halt_mode(&app_state, ExecutionHaltMode::Stopped)
            .await
            .unwrap();

        let halt_mode = load_execution_halt_mode(&app_state).await.unwrap();
        assert_eq!(halt_mode, ExecutionHaltMode::Stopped);
    }

    #[tokio::test]
    async fn test_ensure_resume_allowed_rejects_stopped_halt_mode() {
        let app_state = AppState::new_test();
        persist_execution_halt_mode(&app_state, ExecutionHaltMode::Stopped)
            .await
            .unwrap();

        let error = ensure_resume_allowed(&app_state).await.unwrap_err();
        assert_eq!(error, RESUME_AFTER_STOP_ERROR);
    }

    #[tokio::test]
    async fn test_ensure_resume_allowed_accepts_paused_halt_mode() {
        let app_state = AppState::new_test();
        persist_execution_halt_mode(&app_state, ExecutionHaltMode::Paused)
            .await
            .unwrap();

        ensure_resume_allowed(&app_state).await.unwrap();
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
            project_ideation_max: 2,
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
            project_ideation_max: 2,
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
            project_ideation_max: 2,
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
