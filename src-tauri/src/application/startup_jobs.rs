// Startup Job Runner
//
// Handles automatic task resumption when the app restarts.
// Tasks that were in agent-active states (Executing, QaRefining, QaTesting, Reviewing, ReExecuting)
// when the app shut down are automatically resumed on startup, respecting pause state and
// max_concurrent limits.
//
// Also cleans up orphaned agent runs that were left in "running" status from previous sessions.
//
// Usage:
// - Called once during app initialization after HTTP server is ready
// - Cleans up orphaned agent runs from previous sessions
// - Iterates all projects to find tasks in agent-active states
// - Re-executes entry actions to respawn agents
// - Stops early if max_concurrent is reached

use std::path::Path;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};
use tracing::{debug, info};

use crate::application::git_service::GitService;

use crate::application::ReconciliationRunner;
use crate::commands::execution_commands::{
    ActiveProjectState, ExecutionState, AGENT_ACTIVE_STATUSES, AUTO_TRANSITION_STATES,
};
use crate::domain::entities::InternalStatus;
use crate::domain::repositories::{
    AgentRunRepository, AppStateRepository, ChatConversationRepository,
    ExecutionSettingsRepository, ProjectRepository, TaskDependencyRepository, TaskRepository,
};
use crate::domain::state_machine::services::TaskScheduler;

use super::TaskTransitionService;

/// Environment variable that disables startup recovery mechanisms when present.
pub const RALPHX_DISABLE_STARTUP_RECOVERY_ENV: &str = "RALPHX_DISABLE_STARTUP_RECOVERY";

fn is_startup_recovery_disabled_var(value: Option<&std::ffi::OsStr>) -> bool {
    value.is_some()
}

/// Returns true when startup recovery should be skipped for this process.
/// Always returns false in test builds to avoid env-var leakage from outer processes.
pub fn is_startup_recovery_disabled() -> bool {
    #[cfg(test)]
    {
        false
    }
    #[cfg(not(test))]
    {
        is_startup_recovery_disabled_var(
            std::env::var_os(RALPHX_DISABLE_STARTUP_RECOVERY_ENV).as_deref(),
        )
    }
}

/// Runs startup jobs, primarily task resumption.
///
/// Finds all tasks that were in agent-active states when the app shut down
/// and re-triggers their entry actions to respawn worker agents.
/// Also cleans up orphaned agent runs from previous sessions.
/// Phase 82: Supports optional project scoping via `active_project_state`.
/// When active project is set, only tasks from that project will be resumed.
/// When no active project is set, resumption is skipped entirely.
pub struct StartupJobRunner<R: Runtime = tauri::Wry> {
    task_repo: Arc<dyn TaskRepository>,
    task_dep_repo: Arc<dyn TaskDependencyRepository>,
    project_repo: Arc<dyn ProjectRepository>,
    agent_run_repo: Arc<dyn AgentRunRepository>,
    transition_service: Arc<TaskTransitionService<R>>,
    execution_state: Arc<ExecutionState>,
    /// Phase 82: Active project state for per-project scoping
    active_project_state: Arc<ActiveProjectState>,
    /// Phase 90: App state repository for reading persisted active_project_id from DB
    app_state_repo: Arc<dyn AppStateRepository>,
    /// Execution settings repository for loading per-project max_concurrent quota
    execution_settings_repo: Arc<dyn ExecutionSettingsRepository>,
    /// Phase 105: Persisted agent registry for killing orphaned OS processes on restart
    running_agent_registry: Arc<dyn crate::domain::services::RunningAgentRegistry>,
    reconciler: ReconciliationRunner<R>,
    /// Optional task scheduler for auto-starting Ready tasks on startup.
    /// When provided, Ready tasks will be scheduled after resuming agent-active tasks.
    task_scheduler: Option<Arc<dyn TaskScheduler>>,
    /// Optional app handle for event emission
    app_handle: Option<AppHandle<R>>,
}

impl<R: Runtime> StartupJobRunner<R> {
    /// Create a new StartupJobRunner with all required dependencies.
    /// Phase 82: Now requires active_project_state for per-project scoping.
    /// Phase 90: Now requires app_state_repo for reading persisted active project from DB.
    pub fn new(
        task_repo: Arc<dyn TaskRepository>,
        task_dep_repo: Arc<dyn TaskDependencyRepository>,
        project_repo: Arc<dyn ProjectRepository>,
        chat_conversation_repo: Arc<dyn ChatConversationRepository>,
        chat_message_repo: Arc<dyn crate::domain::repositories::ChatMessageRepository>,
        chat_attachment_repo: Arc<dyn crate::domain::repositories::ChatAttachmentRepository>,
        ideation_session_repo: Arc<dyn crate::domain::repositories::IdeationSessionRepository>,
        activity_event_repo: Arc<dyn crate::domain::repositories::ActivityEventRepository>,
        message_queue: Arc<crate::domain::services::MessageQueue>,
        running_agent_registry: Arc<dyn crate::domain::services::RunningAgentRegistry>,
        memory_event_repo: Arc<dyn crate::domain::repositories::MemoryEventRepository>,
        agent_run_repo: Arc<dyn AgentRunRepository>,
        transition_service: Arc<TaskTransitionService<R>>,
        execution_state: Arc<ExecutionState>,
        active_project_state: Arc<ActiveProjectState>,
        app_state_repo: Arc<dyn AppStateRepository>,
        execution_settings_repo: Arc<dyn ExecutionSettingsRepository>,
    ) -> Self {
        let reconciler = ReconciliationRunner::new(
            Arc::clone(&task_repo),
            Arc::clone(&task_dep_repo),
            Arc::clone(&project_repo),
            Arc::clone(&chat_conversation_repo),
            Arc::clone(&chat_message_repo),
            Arc::clone(&chat_attachment_repo),
            Arc::clone(&ideation_session_repo),
            Arc::clone(&activity_event_repo),
            Arc::clone(&message_queue),
            Arc::clone(&running_agent_registry),
            Arc::clone(&memory_event_repo),
            Arc::clone(&agent_run_repo),
            Arc::clone(&transition_service),
            Arc::clone(&execution_state),
            None,
        );

        Self {
            task_repo,
            task_dep_repo,
            project_repo,
            agent_run_repo,
            transition_service,
            execution_state,
            active_project_state,
            app_state_repo,
            execution_settings_repo,
            running_agent_registry,
            reconciler,
            task_scheduler: None,
            app_handle: None,
        }
    }

    /// Set the task scheduler for auto-starting Ready tasks (builder pattern).
    ///
    /// When set, the runner will call try_schedule_ready_tasks() after resuming
    /// agent-active tasks, allowing queued Ready tasks to start execution.
    pub fn with_task_scheduler(mut self, scheduler: Arc<dyn TaskScheduler>) -> Self {
        self.task_scheduler = Some(scheduler);
        self
    }

    /// Set the app handle for event emission (builder pattern).
    pub fn with_app_handle(mut self, app_handle: AppHandle<R>) -> Self {
        self.app_handle = Some(app_handle.clone());
        self.reconciler = self.reconciler.with_app_handle(app_handle);
        self
    }

    /// Run startup jobs, resuming tasks in agent-active states.
    ///
    /// Skips if execution is paused. Stops early if max_concurrent is reached.
    /// For each task in an agent-active state, re-executes entry actions to
    /// respawn the appropriate agent.
    pub async fn run(&self) {
        debug!("StartupJobRunner::run() called");

        if is_startup_recovery_disabled() {
            info!(
                env_var = RALPHX_DISABLE_STARTUP_RECOVERY_ENV,
                "Startup recovery disabled via environment; skipping startup jobs"
            );
            return;
        }

        // Kill orphaned MCP server node processes from previous session.
        // Pattern-based cleanup catches leaked processes that escaped PID tracking
        // (e.g. app crashed before registering PID, or child survived parent kill).
        let mcp_killed =
            crate::domain::services::running_agent_registry::kill_orphaned_mcp_servers();
        if mcp_killed > 0 {
            info!(count = mcp_killed, "Killed orphaned MCP server processes");
        }

        // Phase 105: Kill orphaned agent OS processes from previous session.
        // The SQLite-backed registry persists PIDs across restarts, so we can
        // SIGTERM old processes before spawning new ones.
        // Now uses process-tree kill (children first, then parent).
        let killed = self.running_agent_registry.stop_all().await;
        if !killed.is_empty() {
            info!(
                count = killed.len(),
                "Killed orphaned agent processes from previous session"
            );
        }

        // Clean up orphaned agent runs from previous sessions
        // These are runs that were left in "running" status when the app was closed/crashed
        match self.agent_run_repo.cancel_all_running().await {
            Ok(count) if count > 0 => {
                info!(
                    count = count,
                    "Cancelled orphaned agent runs from previous session"
                );
            }
            Ok(_) => {
                // No orphaned runs, nothing to log
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to clean up orphaned agent runs");
            }
        }

        // Unblock tasks that got stuck due to app crash (safety net)
        // This runs before pause check since unblocking doesn't spawn agents
        self.unblock_ready_tasks().await;

        // Check if execution is paused - skip resumption if so
        if self.execution_state.is_paused() {
            info!("Execution paused, skipping task resumption");
            return;
        }
        debug!("Execution NOT paused, continuing...");

        // Phase 90: Read active project from DB (persisted from last session)
        // No waiting needed — DB has the value from the previous session.
        debug!("Reading active project from DB...");
        let active_project_id = {
            let db_result = self.app_state_repo.get().await;
            match db_result {
                Ok(settings) => settings.active_project_id,
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to read app_state from DB");
                    None
                }
            }
        };
        if let Some(ref pid) = active_project_id {
            // Set in-memory state from DB value so other commands can use it immediately
            self.active_project_state.set(Some(pid.clone())).await;
            info!(project_id = pid.as_str(), "Active project loaded from DB");

            // Load execution settings for this project and sync runtime quota
            match self.execution_settings_repo.get_settings(Some(pid)).await {
                Ok(settings) => {
                    let old_max = self.execution_state.max_concurrent();
                    self.execution_state
                        .set_max_concurrent(settings.max_concurrent_tasks);
                    info!(
                        project_id = pid.as_str(),
                        old_max = old_max,
                        new_max = settings.max_concurrent_tasks,
                        "Updated max_concurrent from persisted project settings"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        project_id = pid.as_str(),
                        "Failed to load execution settings for active project, keeping current quota"
                    );
                }
            }
        }
        if active_project_id.is_none() {
            info!("No active project in DB, skipping task resumption");
            // Still try to schedule Ready tasks if scheduler is set
            if let Some(ref scheduler) = self.task_scheduler {
                info!("Scheduling Ready tasks (no resumption)");
                scheduler.try_schedule_ready_tasks().await;
            }
            return;
        }

        // Get projects to process (scoped to active project in Phase 82)
        let projects = if let Some(ref active_pid) = active_project_id {
            // Scope to active project only
            match self.project_repo.get_by_id(active_pid).await {
                Ok(Some(project)) => vec![project],
                Ok(None) => {
                    tracing::warn!(
                        project_id = active_pid.as_str(),
                        "Active project not found, skipping resumption"
                    );
                    return;
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to get active project for startup resumption");
                    return;
                }
            }
        } else {
            // Fallback to all projects (shouldn't reach here due to check above)
            match self.project_repo.get_all().await {
                Ok(projects) => projects,
                Err(e) => {
                    tracing::error!(error = %e, "Failed to get projects for startup resumption");
                    return;
                }
            }
        };

        let mut resumed = 0u32;

        debug!(
            count = projects.len(),
            active_project = ?active_project_id.as_ref().map(|p| p.as_str()),
            "Found projects for startup resumption"
        );

        // Phase 0: Clean up stale git state before any task recovery
        self.cleanup_stale_git_state(&projects).await;

        // Phase 1: Merge-first recovery — process PendingMerge and Merging tasks
        // before spawning other agents. This ensures main branch is in a clean state
        // before worker/reviewer agents start. PendingMerge first so fast-path
        // programmatic merges complete before agent-based merges.
        const MERGE_RECOVERY_STATES: &[InternalStatus] = &[
            InternalStatus::PendingMerge,
            InternalStatus::Merging,
        ];

        info!("Phase 1: Merge-first recovery — processing merge states before agent spawning");

        'merge_recovery: for project in &projects {
            for status in MERGE_RECOVERY_STATES {
                let tasks = match self.task_repo.get_by_status(&project.id, *status).await {
                    Ok(tasks) => tasks,
                    Err(e) => {
                        tracing::warn!(
                            project_id = project.id.as_str(),
                            status = ?status,
                            error = %e,
                            "Failed to get tasks by status for merge-first recovery"
                        );
                        continue;
                    }
                };

                debug!(count = tasks.len(), ?status, "Phase 1: Found tasks in merge state");
                for task in tasks {
                    if task.archived_at.is_some() {
                        debug!(task_id = task.id.as_str(), title = %task.title, "Skipping archived task");
                        continue;
                    }

                    // Skip tasks that already have main_merge_deferred=true — they need
                    // serialized one-at-a-time retry via try_retry_main_merges(), not
                    // direct execute_entry_actions() which could race if multiple siblings
                    // are all in PendingMerge with the flag set.
                    {
                        use crate::domain::state_machine::transition_handler::has_main_merge_deferred_metadata;
                        if has_main_merge_deferred_metadata(&task) {
                            debug!(
                                task_id = task.id.as_str(),
                                "Phase 1: skipping main_merge_deferred task — will be handled by try_retry_main_merges at startup end"
                            );
                            continue;
                        }
                    }

                    let reconciled = self.reconciler.reconcile_task(&task, *status).await;

                    if reconciled {
                        continue;
                    }

                    if !self.execution_state.can_start_task() {
                        info!(
                            max_concurrent = self.execution_state.max_concurrent(),
                            running_count = self.execution_state.running_count(),
                            "Phase 1: Max concurrent reached, stopping merge-first recovery"
                        );
                        break 'merge_recovery;
                    }

                    info!(
                        task_id = task.id.as_str(),
                        status = ?status,
                        "Phase 1: Resuming merge task"
                    );

                    self.transition_service
                        .execute_entry_actions(&task.id, &task, *status)
                        .await;

                    resumed += 1;
                }
            }
        }

        // Iterate through projects and their tasks in agent-active states
        for project in &projects {
            debug!(
                project_id = project.id.as_str(),
                "Checking project for resumable tasks"
            );
            for status in AGENT_ACTIVE_STATUSES {
                // Skip merge states — already handled in Phase 1 merge-first recovery
                if *status == InternalStatus::Merging || *status == InternalStatus::PendingMerge {
                    continue;
                }

                // Get tasks in this status for this project
                let tasks = match self.task_repo.get_by_status(&project.id, *status).await {
                    Ok(tasks) => tasks,
                    Err(e) => {
                        tracing::warn!(
                            project_id = project.id.as_str(),
                            status = ?status,
                            error = %e,
                            "Failed to get tasks by status"
                        );
                        continue;
                    }
                };

                debug!(count = tasks.len(), ?status, "Found tasks in status");
                for task in tasks {
                    // Phase 106: Defense-in-depth — skip archived tasks even if query returns them
                    if task.archived_at.is_some() {
                        debug!(task_id = task.id.as_str(), title = %task.title, "Skipping archived task");
                        continue;
                    }

                    // Skip main-merge-deferred tasks when agents are still running.
                    // These are correctly deferred, not orphaned — reconciliation will retry when agents complete.
                    if Self::is_waiting_for_global_idle(&task, self.execution_state.running_count()) {
                        debug!(
                            task_id = task.id.as_str(),
                            running_count = self.execution_state.running_count(),
                            "Skipping main-merge-deferred task: agents still running"
                        );
                        continue;
                    }

                    let reconciled = self.reconciler.reconcile_task(&task, *status).await;

                    if reconciled {
                        continue;
                    }

                    // Check if we can start another task
                    if !self.execution_state.can_start_task() {
                        info!(
                            max_concurrent = self.execution_state.max_concurrent(),
                            running_count = self.execution_state.running_count(),
                            "Max concurrent reached, stopping resumption"
                        );
                        info!(count = resumed, "Task resumption complete (partial)");
                        return;
                    }

                    info!(
                        task_id = task.id.as_str(),
                        status = ?status,
                        "Resuming task"
                    );

                    // Re-execute entry actions to respawn the agent
                    self.transition_service
                        .execute_entry_actions(&task.id, &task, *status)
                        .await;

                    resumed += 1;
                }
            }
        }

        info!(count = resumed, "Task resumption complete");

        // Re-trigger auto-transition states that may have been interrupted mid-transition
        // These states have on_enter side effects that trigger auto-transitions to spawn agents
        for project in &projects {
            for status in AUTO_TRANSITION_STATES {
                // Skip PendingMerge — already handled in Phase 1 merge-first recovery
                if *status == InternalStatus::PendingMerge {
                    continue;
                }

                let tasks = match self.task_repo.get_by_status(&project.id, *status).await {
                    Ok(tasks) => tasks,
                    Err(e) => {
                        tracing::warn!(
                            project_id = project.id.as_str(),
                            status = ?status,
                            error = %e,
                            "Failed to get tasks by status for auto-transition"
                        );
                        continue;
                    }
                };

                debug!(
                    count = tasks.len(),
                    ?status,
                    "Found tasks in auto-transition status"
                );
                for task in tasks {
                    // Phase 106: Defense-in-depth — skip archived tasks even if query returns them
                    if task.archived_at.is_some() {
                        debug!(task_id = task.id.as_str(), title = %task.title, "Skipping archived task");
                        continue;
                    }

                    // Skip main-merge-deferred tasks when agents are still running.
                    // These tasks are correctly deferred and will be retried when all agents
                    // complete (via try_retry_main_merges on global idle).
                    if Self::is_waiting_for_global_idle(&task, self.execution_state.running_count()) {
                        debug!(
                            task_id = task.id.as_str(),
                            running_count = self.execution_state.running_count(),
                            "Skipping main-merge-deferred task in auto-transition recovery: agents still running"
                        );
                        continue;
                    }

                    // Check max_concurrent before triggering (auto-transitions may spawn agents)
                    if !self.execution_state.can_start_task() {
                        info!(
                            max_concurrent = self.execution_state.max_concurrent(),
                            running_count = self.execution_state.running_count(),
                            "Max concurrent reached, stopping auto-transition recovery"
                        );
                        return;
                    }

                    info!(
                        task_id = task.id.as_str(),
                        status = ?status,
                        "Re-triggering auto-transition for stuck task"
                    );

                    // Re-execute entry actions - this will trigger check_auto_transition()
                    self.transition_service
                        .execute_entry_actions(&task.id, &task, *status)
                        .await;
                }
            }
        }

        // After resuming agent-active tasks, try to schedule any Ready tasks
        // that may be waiting in the queue (if scheduler is configured)
        if let Some(ref scheduler) = self.task_scheduler {
            info!("Scheduling Ready tasks after resumption");
            scheduler.try_schedule_ready_tasks().await;
        }

        // Boot recovery: if no agents were spawned during startup (quiescent boot),
        // invoke try_retry_main_merges() now. Normally this is called from the
        // agent-exit path in transition_handler when running_count transitions to 0,
        // but at boot there may be no agents to exit and thus no future trigger.
        // Without this call, PendingMerge tasks with main_merge_deferred=true would
        // sit stuck indefinitely after a reboot.
        if self.execution_state.running_count() == 0 {
            if let Some(ref scheduler) = self.task_scheduler {
                info!("Boot recovery: invoking try_retry_main_merges for deferred main-branch merges (running_count == 0)");
                scheduler.try_retry_main_merges().await;
            }
        }
    }

    /// Unblock tasks whose blockers are all complete.
    ///
    /// This is a safety net for cases where the app crashed before real-time
    /// unblocking could run (e.g., when a task merged but the app closed before
    /// dependent tasks were unblocked).
    ///
    /// Scans all Blocked tasks across all projects and transitions those
    /// whose blockers are all in terminal states (Approved, Merged, Failed, Cancelled)
    /// to Ready status.
    async fn unblock_ready_tasks(&self) {
        // Get all projects to scan for blocked tasks
        let projects = match self.project_repo.get_all().await {
            Ok(projects) => projects,
            Err(e) => {
                tracing::error!(error = %e, "Failed to fetch projects for startup unblock");
                return;
            }
        };

        let mut unblocked_count = 0u32;

        for project in projects {
            // Get all blocked tasks for this project
            let blocked_tasks = match self
                .task_repo
                .get_by_status(&project.id, InternalStatus::Blocked)
                .await
            {
                Ok(tasks) => tasks,
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        project_id = project.id.as_str(),
                        "Failed to fetch blocked tasks for project"
                    );
                    continue;
                }
            };

            if blocked_tasks.is_empty() {
                continue;
            }

            tracing::debug!(
                project_id = project.id.as_str(),
                count = blocked_tasks.len(),
                "Checking blocked tasks for startup unblock"
            );

            for mut task in blocked_tasks {
                // Phase 106: Defense-in-depth — skip archived tasks even if query returns them
                if task.archived_at.is_some() {
                    debug!(task_id = task.id.as_str(), title = %task.title, "Skipping archived task in unblock check");
                    continue;
                }

                // Get blockers for this task
                let blockers = match self.task_dep_repo.get_blockers(&task.id).await {
                    Ok(b) => b,
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            task_id = task.id.as_str(),
                            "Failed to get blockers for task"
                        );
                        continue;
                    }
                };

                // Check if all blockers are complete
                if !self.all_blockers_complete(&blockers).await {
                    continue;
                }

                // All blockers complete - transition to Ready
                task.internal_status = InternalStatus::Ready;
                task.blocked_reason = None;
                task.touch();

                if let Err(e) = self.task_repo.update(&task).await {
                    tracing::error!(
                        error = %e,
                        task_id = task.id.as_str(),
                        "Failed to unblock task on startup"
                    );
                    continue;
                }

                tracing::info!(
                    task_id = task.id.as_str(),
                    task_title = %task.title,
                    "Task unblocked on startup - all blockers complete"
                );

                // Emit task:unblocked event for UI update
                if let Some(ref handle) = self.app_handle {
                    let _ = handle.emit(
                        "task:unblocked",
                        serde_json::json!({
                            "taskId": task.id.as_str(),
                            "taskTitle": task.title,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        }),
                    );
                }

                unblocked_count += 1;
            }
        }

        if unblocked_count > 0 {
            info!(count = unblocked_count, "Unblocked tasks on startup");
        } else {
            tracing::debug!("No blocked tasks needed unblocking on startup");
        }
    }

    /// Check if all blocker tasks are in a terminal state.
    /// Delegates to InternalStatus::is_terminal() as the single source of truth.
    /// Terminal states: Merged, Failed, Cancelled, Stopped, MergeIncomplete.
    /// If a blocker doesn't exist (was deleted), it's considered complete.
    async fn all_blockers_complete(&self, blocker_ids: &[crate::domain::entities::TaskId]) -> bool {
        for blocker_id in blocker_ids {
            match self.task_repo.get_by_id(blocker_id).await {
                Ok(Some(task)) => {
                    if !task.internal_status.is_terminal() {
                        return false;
                    }
                }
                Ok(None) => {
                    // Blocker was deleted - consider it complete (not blocking)
                    continue;
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        blocker_id = blocker_id.as_str(),
                        "Failed to fetch blocker task, assuming incomplete"
                    );
                    return false;
                }
            }
        }
        true
    }

    /// Abort stale rebase/merge operations on project repos.
    /// Called before any task recovery to ensure clean git state.
    async fn cleanup_stale_git_state(&self, projects: &[crate::domain::entities::Project]) {
        for project in projects {
            let repo_path = Path::new(&project.working_directory);
            if !repo_path.exists() {
                continue;
            }
            if GitService::is_rebase_in_progress(repo_path) {
                info!(
                    project_id = project.id.as_str(),
                    "Phase 0: Aborting stale rebase on main repo before startup recovery"
                );
                let _ = GitService::abort_rebase(repo_path).await;
            }
            if GitService::is_merge_in_progress(repo_path) {
                info!(
                    project_id = project.id.as_str(),
                    "Phase 0: Aborting stale merge on main repo before startup recovery"
                );
                let _ = GitService::abort_merge(repo_path).await;
            }
        }
    }

    /// Check if a task is waiting for global idle (no agents running) before retrying.
    ///
    /// Used for main-merge-deferred tasks that should not be resumed on startup
    /// when agents are still running. Returns true only if:
    /// - Task has `main_merge_deferred` metadata flag set
    /// - There are agents currently running (running_count > 0)
    fn is_waiting_for_global_idle(task: &crate::domain::entities::Task, running_count: u32) -> bool {
        use crate::domain::state_machine::transition_handler::has_main_merge_deferred_metadata;

        has_main_merge_deferred_metadata(task) && running_count > 0
    }
}

#[cfg(test)]
mod tests;
