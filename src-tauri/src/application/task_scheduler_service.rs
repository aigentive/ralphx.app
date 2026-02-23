// Task Scheduler Service
//
// Production implementation of the TaskScheduler trait for auto-scheduling Ready tasks.
// This service checks execution capacity and transitions the oldest Ready task to Executing
// when slots are available.
//
// Called from:
// - TransitionHandler::on_exit() when an agent-active task completes (slot freed)
// - TransitionHandler::on_enter(Ready) when a task becomes Ready
// - StartupJobRunner after resuming agent-active tasks
// - resume_execution and set_max_concurrent commands (future Phase 26 tasks)

use async_trait::async_trait;
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};
use tauri::{AppHandle, Runtime};
use tokio::sync::{Mutex as TokioMutex, RwLock};

use crate::infrastructure::agents::claude::scheduler_config;

use crate::commands::ExecutionState;
use crate::domain::entities::{
    task_metadata::{
        MergeRecoveryEvent, MergeRecoveryEventKind, MergeRecoveryMetadata, MergeRecoveryReasonCode,
        MergeRecoverySource, MergeRecoveryState,
    },
    InternalStatus, ProjectId, Task, TaskCategory,
};
use crate::domain::repositories::{
    ActivityEventRepository, AgentRunRepository, ChatAttachmentRepository,
    ChatConversationRepository, ChatMessageRepository, IdeationSessionRepository,
    MemoryEventRepository, PlanBranchRepository, ProjectRepository, TaskDependencyRepository,
    TaskRepository,
};
use crate::domain::services::{MessageQueue, RunningAgentRegistry};
use crate::domain::state_machine::services::TaskScheduler;

use super::TaskTransitionService;
use crate::domain::state_machine::transition_handler::{get_trigger_origin, set_trigger_origin};

/// Production implementation of TaskScheduler for auto-scheduling Ready tasks.
///
/// This service queries for the oldest Ready task across all projects and
/// transitions it to Executing when execution slots are available.
///
/// Phase 82: Supports optional project scoping via `active_project_id` filter.
/// When set, only tasks from that project will be scheduled.
pub struct TaskSchedulerService<R: Runtime = tauri::Wry> {
    execution_state: Arc<ExecutionState>,
    project_repo: Arc<dyn ProjectRepository>,
    task_repo: Arc<dyn TaskRepository>,
    task_dependency_repo: Arc<dyn TaskDependencyRepository>,
    chat_message_repo: Arc<dyn ChatMessageRepository>,
    chat_attachment_repo: Arc<dyn ChatAttachmentRepository>,
    conversation_repo: Arc<dyn ChatConversationRepository>,
    agent_run_repo: Arc<dyn AgentRunRepository>,
    ideation_session_repo: Arc<dyn IdeationSessionRepository>,
    activity_event_repo: Arc<dyn ActivityEventRepository>,
    message_queue: Arc<MessageQueue>,
    running_agent_registry: Arc<dyn RunningAgentRegistry>,
    memory_event_repo: Arc<dyn MemoryEventRepository>,
    app_handle: Option<AppHandle<R>>,
    /// Optional plan branch repository for feature branch resolution.
    plan_branch_repo: Option<Arc<dyn PlanBranchRepository>>,
    /// Self-reference for propagating scheduler through build_transition_service().
    /// Set after Arc-wrapping via set_self_ref(). Uses Mutex since it's written once at init.
    self_ref: Mutex<Option<Arc<dyn TaskScheduler>>>,
    /// Phase 82: Optional project ID to scope scheduling to a single project.
    /// When set, only Ready tasks from this project are considered.
    active_project_id: RwLock<Option<ProjectId>>,
    /// Guard to prevent concurrent scheduling from causing duplicate transitions.
    /// Multiple triggers can fire try_schedule_ready_tasks() simultaneously
    /// (e.g., on_enter(Ready) delayed tokio::spawn + on_exit(agent_state) direct call),
    /// leading to TOCTOU races where two invocations both find the same Ready task
    /// and both transition it to Executing, causing duplicate on_enter(Executing).
    scheduling_lock: TokioMutex<()>,
    /// Number of pending contention-retry spawns currently in flight.
    /// Wrapped in Arc so spawned retry closures can decrement it without downcasting.
    /// Bounded by scheduler_config().max_contention_retries.
    contention_retry_pending: Arc<AtomicU32>,
}

impl<R: Runtime> TaskSchedulerService<R> {
    /// Create a new TaskSchedulerService with all required dependencies.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        execution_state: Arc<ExecutionState>,
        project_repo: Arc<dyn ProjectRepository>,
        task_repo: Arc<dyn TaskRepository>,
        task_dependency_repo: Arc<dyn TaskDependencyRepository>,
        chat_message_repo: Arc<dyn ChatMessageRepository>,
        chat_attachment_repo: Arc<dyn ChatAttachmentRepository>,
        conversation_repo: Arc<dyn ChatConversationRepository>,
        agent_run_repo: Arc<dyn AgentRunRepository>,
        ideation_session_repo: Arc<dyn IdeationSessionRepository>,
        activity_event_repo: Arc<dyn ActivityEventRepository>,
        message_queue: Arc<MessageQueue>,
        running_agent_registry: Arc<dyn RunningAgentRegistry>,
        memory_event_repo: Arc<dyn MemoryEventRepository>,
        app_handle: Option<AppHandle<R>>,
    ) -> Self {
        Self {
            execution_state,
            project_repo,
            task_repo,
            task_dependency_repo,
            chat_message_repo,
            chat_attachment_repo,
            conversation_repo,
            agent_run_repo,
            ideation_session_repo,
            activity_event_repo,
            message_queue,
            running_agent_registry,
            memory_event_repo,
            app_handle,
            plan_branch_repo: None,
            self_ref: Mutex::new(None),
            active_project_id: RwLock::new(None),
            scheduling_lock: TokioMutex::new(()),
            contention_retry_pending: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Set the plan branch repository for feature branch resolution (builder pattern).
    pub fn with_plan_branch_repo(mut self, repo: Arc<dyn PlanBranchRepository>) -> Self {
        self.plan_branch_repo = Some(repo);
        self
    }

    /// Set the self-reference after wrapping in Arc.
    /// This allows build_transition_service() to propagate the scheduler.
    /// Must be called after Arc::new(scheduler) at each construction site.
    pub fn set_self_ref(&self, scheduler: Arc<dyn TaskScheduler>) {
        *self.self_ref.lock().unwrap() = Some(scheduler);
    }

    /// Set the active project ID for scoped scheduling (Phase 82).
    /// When set, only Ready tasks from this project will be scheduled.
    /// Set to None to schedule across all projects.
    pub async fn set_active_project(&self, project_id: Option<ProjectId>) {
        *self.active_project_id.write().await = project_id;
    }

    /// Get the current active project ID, if any.
    pub async fn get_active_project(&self) -> Option<ProjectId> {
        self.active_project_id.read().await.clone()
    }

    /// Find the oldest schedulable task across all projects (or scoped to active project).
    ///
    /// Phase 82: When active_project_id is set, only tasks from that project are considered.
    /// For Worktree-mode projects, any Ready task is schedulable.
    /// For Local-mode projects, a task is only schedulable if no other task
    /// in the same project is in a "running" state (Executing, ReExecuting,
    /// Reviewing, or Merging).
    ///
    /// Returns None if no schedulable tasks exist or if there's an error querying.
    async fn find_oldest_schedulable_task(&self) -> Option<Task> {
        // Phase 82: Get active project filter
        let active_project = self.active_project_id.read().await.clone();

        // Get a batch of oldest Ready tasks to evaluate
        let ready_tasks = match self.task_repo.get_oldest_ready_tasks(50).await {
            Ok(tasks) => tasks,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to get Ready tasks for scheduling");
                return None;
            }
        };

        for task in ready_tasks {
            // Phase 82: If active project is set, skip tasks from other projects
            if let Some(ref active_pid) = active_project {
                if task.project_id != *active_pid {
                    tracing::debug!(
                        task_id = task.id.as_str(),
                        task_project = task.project_id.as_str(),
                        active_project = active_pid.as_str(),
                        "Skipping task: not in active project"
                    );
                    continue;
                }
            }

            // Verify the project exists
            match self.project_repo.get_by_id(&task.project_id).await {
                Ok(Some(_)) => {}
                Ok(None) => {
                    tracing::warn!(
                        task_id = task.id.as_str(),
                        project_id = task.project_id.as_str(),
                        "Task has non-existent project, skipping"
                    );
                    continue;
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        task_id = task.id.as_str(),
                        "Failed to get project for task, skipping"
                    );
                    continue;
                }
            }

            // Dependency gate: skip tasks whose blockers are not satisfied.
            // Re-block the task so it doesn't sit in Ready with unsatisfied deps.
            if self.has_unsatisfied_dependencies(&task).await {
                self.reblock_task(&task).await;
                continue;
            }

            // This task is schedulable
            return Some(task);
        }

        None
    }

    /// Check if a task has any blocker whose status is not dependency-satisfied.
    /// Returns true if the task should NOT be scheduled. Fail-open on errors.
    async fn has_unsatisfied_dependencies(&self, task: &Task) -> bool {
        let blocker_ids = match self.task_dependency_repo.get_blockers(&task.id).await {
            Ok(ids) => ids,
            Err(_) => return false,
        };
        if blocker_ids.is_empty() {
            return false;
        }
        for blocker_id in &blocker_ids {
            match self.task_repo.get_by_id(blocker_id).await {
                Ok(Some(blocker)) => {
                    if !blocker.internal_status.is_dependency_satisfied() {
                        return true;
                    }
                }
                Ok(None) => {} // deleted blocker = satisfied
                Err(_) => {}   // fail-open
            }
        }
        false
    }

    /// Re-block a Ready task that has unsatisfied dependencies.
    /// Sets status to Blocked with a descriptive reason listing unsatisfied blocker titles.
    async fn reblock_task(&self, task: &Task) {
        let blocker_ids = self
            .task_dependency_repo
            .get_blockers(&task.id)
            .await
            .unwrap_or_default();

        let mut reasons = Vec::new();
        for bid in &blocker_ids {
            if let Ok(Some(b)) = self.task_repo.get_by_id(bid).await {
                if !b.internal_status.is_dependency_satisfied() {
                    let label = if b.internal_status == InternalStatus::Failed {
                        format!("\"{}\" (failed)", b.title)
                    } else {
                        format!("\"{}\" ({})", b.title, b.internal_status)
                    };
                    reasons.push(label);
                }
            }
        }

        let mut updated = task.clone();
        updated.internal_status = InternalStatus::Blocked;
        updated.blocked_reason = if reasons.is_empty() {
            Some("Dependency check failed".to_string())
        } else {
            Some(format!("Waiting for: {}", reasons.join(", ")))
        };
        updated.touch();

        // Use optimistic lock — if task already moved out of Ready, this is a no-op
        match self
            .task_repo
            .update_with_expected_status(&updated, InternalStatus::Ready)
            .await
        {
            Ok(true) => {
                tracing::warn!(
                    task_id = task.id.as_str(),
                    "Scheduler: re-blocked Ready task with unsatisfied dependencies"
                );
                let _ = self
                    .task_repo
                    .persist_status_change(
                        &task.id,
                        InternalStatus::Ready,
                        InternalStatus::Blocked,
                        "scheduler_dep_gate",
                    )
                    .await;
            }
            Ok(false) => {
                tracing::debug!(
                    task_id = task.id.as_str(),
                    "Scheduler: task already moved from Ready, skipping re-block"
                );
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    task_id = task.id.as_str(),
                    "Failed to re-block task with unsatisfied dependencies"
                );
            }
        }
    }

    /// Build a TaskTransitionService for transitioning tasks.
    ///
    /// Creates a fresh instance to avoid circular dependency issues when
    /// the scheduler is called from within TransitionHandler.
    fn build_transition_service(&self) -> TaskTransitionService<R>
    where
        R: Runtime,
    {
        let mut service = TaskTransitionService::new(
            Arc::clone(&self.task_repo),
            Arc::clone(&self.task_dependency_repo),
            Arc::clone(&self.project_repo),
            Arc::clone(&self.chat_message_repo),
            Arc::clone(&self.chat_attachment_repo),
            Arc::clone(&self.conversation_repo),
            Arc::clone(&self.agent_run_repo),
            Arc::clone(&self.ideation_session_repo),
            Arc::clone(&self.activity_event_repo),
            Arc::clone(&self.message_queue),
            Arc::clone(&self.running_agent_registry),
            Arc::clone(&self.execution_state),
            self.app_handle.clone(),
            Arc::clone(&self.memory_event_repo),
        );
        if let Some(ref repo) = self.plan_branch_repo {
            service = service.with_plan_branch_repo(Arc::clone(repo));
        }
        if let Some(ref sched) = *self.self_ref.lock().unwrap() {
            service = service.with_task_scheduler(Arc::clone(sched));
        }
        service
    }
}

#[async_trait]
impl<R: Runtime> TaskScheduler for TaskSchedulerService<R> {
    /// Try to schedule Ready tasks if execution slots are available.
    ///
    /// This method loops to fill all available execution slots:
    /// 1. Checks if execution is paused or at capacity
    /// 2. Finds the oldest Ready task across all projects
    /// 3. Transitions it to Executing state via the state machine
    /// 4. Repeats until no more slots or no more schedulable tasks
    async fn try_schedule_ready_tasks(&self) {
        // Prevent concurrent scheduling to avoid TOCTOU race where two invocations
        // both find the same Ready task and both transition it to Executing.
        // Use try_lock: if another scheduling is already in progress, queue a 200ms retry
        // so the caller's scheduling intent is not silently lost (S6 fix).
        let _guard = match self.scheduling_lock.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                // Limit concurrent retry spawns to avoid cascading if lock is persistently held.
                let sched_cfg = scheduler_config();
                let max_retries = sched_cfg.max_contention_retries as u32;
                let retry_delay_ms = sched_cfg.contention_retry_delay_ms;
                let pending = self.contention_retry_pending.load(Ordering::Relaxed);
                if pending >= max_retries {
                    tracing::debug!(
                        pending_retries = pending,
                        "Scheduling already in progress; retry limit reached, dropping attempt"
                    );
                    return;
                }
                if let Some(scheduler) = self.self_ref.lock().unwrap().clone() {
                    self.contention_retry_pending
                        .fetch_add(1, Ordering::Relaxed);
                    tracing::debug!(
                        pending_retries = pending + 1,
                        "Scheduling lock contention detected; queuing retry in {retry_delay_ms}ms"
                    );
                    let retry_counter = Arc::clone(&self.contention_retry_pending);
                    tokio::spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_millis(retry_delay_ms))
                            .await;
                        // Decrement before the retry attempt so the slot is freed
                        // regardless of whether the retry succeeds or skips.
                        retry_counter.fetch_sub(1, Ordering::Relaxed);
                        scheduler.try_schedule_ready_tasks().await;
                    });
                } else {
                    tracing::debug!(
                        "Scheduling already in progress, skipping concurrent attempt (no self_ref)"
                    );
                }
                return;
            }
        };

        loop {
            // Check capacity on each iteration
            if !self.execution_state.can_start_task() {
                tracing::debug!(
                    is_paused = self.execution_state.is_paused(),
                    running_count = self.execution_state.running_count(),
                    max_concurrent = self.execution_state.max_concurrent(),
                    "Cannot schedule more: at capacity or paused"
                );
                break;
            }

            // Find next schedulable task (accounting for Local-mode constraints)
            let Some(task) = self.find_oldest_schedulable_task().await else {
                tracing::debug!("No more schedulable tasks");
                break;
            };

            tracing::info!(
                task_id = task.id.as_str(),
                task_title = task.title.as_str(),
                created_at = %task.created_at,
                "Scheduling Ready task for execution"
            );

            // Determine target status: plan_merge tasks skip execution and go directly to merge
            let target_status = if task.category == TaskCategory::PlanMerge {
                tracing::info!(
                    task_id = task.id.as_str(),
                    "Plan merge task: routing to PendingMerge (skip execution)"
                );
                InternalStatus::PendingMerge
            } else {
                InternalStatus::Executing
            };

            // Set trigger_origin to "scheduler" if not already set (preserves retry/recovery origins)
            if get_trigger_origin(&task).is_none() {
                let mut task_mut = task.clone();
                set_trigger_origin(&mut task_mut, "scheduler");
                if let Err(e) = self.task_repo.update(&task_mut).await {
                    tracing::error!(
                        task_id = task.id.as_str(),
                        error = %e,
                        "Failed to set trigger_origin=scheduler in metadata"
                    );
                }
            }

            // Per-task concurrency guard: prevent two concurrent scheduler invocations
            // from both scheduling the same task (TOCTOU race on async on_enter_Executing).
            if !self.execution_state.try_start_scheduling(task.id.as_str()) {
                tracing::debug!(
                    task_id = task.id.as_str(),
                    "Scheduler: task already being scheduled by another caller, skipping"
                );
                continue;
            }

            // Transition the task to the target status
            // For Executing: triggers on_enter(Executing) which spawns worker agent
            // For PendingMerge: triggers on_enter(PendingMerge) which runs attempt_programmatic_merge()
            let transition_service = self.build_transition_service();

            let transition_result = transition_service
                .transition_task(&task.id, target_status)
                .await;

            // Always release the per-task guard after transition completes
            self.execution_state.finish_scheduling(task.id.as_str());

            if let Err(e) = transition_result {
                tracing::error!(
                    task_id = task.id.as_str(),
                    error = %e,
                    target = ?target_status,
                    "Failed to transition Ready task"
                );
                // Stop on error to avoid infinite loop on persistent failures
                break;
            }

            // Continue loop - try to fill next slot
        }
    }

    /// Re-trigger deferred merges for a project after a competing merge completes.
    ///
    /// Finds tasks in PendingMerge with `merge_deferred` metadata, clears the flag,
    /// and re-invokes their entry actions so `attempt_programmatic_merge()` runs again.
    async fn try_retry_deferred_merges(&self, project_id: &str) {
        use crate::domain::state_machine::transition_handler::{
            clear_merge_deferred_metadata, has_merge_deferred_metadata,
            is_merge_deferred_timed_out, DEFERRED_MERGE_TIMEOUT_SECONDS,
        };

        let pid = ProjectId::from_string(project_id.to_string());
        let all_tasks = match self.task_repo.get_by_project(&pid).await {
            Ok(tasks) => tasks,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    project_id = project_id,
                    "Failed to fetch tasks for deferred merge retry"
                );
                return;
            }
        };

        // Count deferred tasks for logging
        let deferred_tasks: Vec<_> = all_tasks
            .iter()
            .filter(|t| {
                t.internal_status == InternalStatus::PendingMerge && has_merge_deferred_metadata(t)
            })
            .collect();

        let deferred_count = deferred_tasks.len();

        if deferred_count == 0 {
            tracing::debug!(project_id = project_id, "No deferred merges to retry");
            return;
        }

        tracing::info!(
            project_id = project_id,
            deferred_count = deferred_count,
            "Found deferred merges to retry (will retry one at a time)"
        );

        for task in deferred_tasks {
            // Extract metadata for logging
            let (target_branch, blocking_task_id) = task
                .metadata
                .as_ref()
                .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
                .map(|val| {
                    let target = val
                        .get("target_branch")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let blocker = val.get("blocking_task_id").and_then(|v| v.as_str());
                    (target.to_string(), blocker.map(|s| s.to_string()))
                })
                .unwrap_or_else(|| ("unknown".to_string(), None));

            // Warn if the merge has been deferred longer than the configured timeout.
            // This is a diagnostic indicator; the retry proceeds regardless (blocker just completed).
            if is_merge_deferred_timed_out(task) {
                tracing::warn!(
                    event = "deferred_merge_timeout_exceeded",
                    task_id = task.id.as_str(),
                    project_id = project_id,
                    target_branch = %target_branch,
                    timeout_seconds = DEFERRED_MERGE_TIMEOUT_SECONDS,
                    "Deferred merge exceeded timeout — retry was delayed beyond expected window"
                );
            }

            // Structured retry attempt event
            tracing::info!(
                event = "merge_retry_attempt",
                task_id = task.id.as_str(),
                project_id = project_id,
                target_branch = %target_branch,
                blocking_task_id = blocking_task_id.as_deref().unwrap_or("unknown"),
                remaining_deferred = deferred_count,
                "Re-triggering deferred merge attempt"
            );

            // Append auto_retry_triggered event before clearing deferred flag
            let mut updated = task.clone();

            // Get or create merge recovery metadata
            let mut recovery =
                MergeRecoveryMetadata::from_task_metadata(updated.metadata.as_deref())
                    .unwrap_or(None)
                    .unwrap_or_else(MergeRecoveryMetadata::new);

            // Count previous retry attempts from events
            let attempt_count = recovery
                .events
                .iter()
                .filter(|e| matches!(e.kind, MergeRecoveryEventKind::AutoRetryTriggered))
                .count() as u32
                + 1;

            // Create auto_retry_triggered event
            let auto_retry_event = MergeRecoveryEvent::new(
                MergeRecoveryEventKind::AutoRetryTriggered,
                MergeRecoverySource::Auto,
                MergeRecoveryReasonCode::TargetBranchBusy,
                format!(
                    "Automatic retry attempt {}: blocker task completed or exited merge workflow",
                    attempt_count
                ),
            )
            .with_target_branch(&target_branch)
            .with_attempt(attempt_count);

            // Append event and update state to Retrying
            recovery.append_event_with_state(auto_retry_event, MergeRecoveryState::Retrying);

            // Update task metadata
            match recovery.update_task_metadata(updated.metadata.as_deref()) {
                Ok(updated_json) => {
                    updated.metadata = Some(updated_json);
                }
                Err(e) => {
                    tracing::error!(
                        task_id = task.id.as_str(),
                        error = %e,
                        "Failed to serialize merge recovery metadata during retry"
                    );
                }
            }

            // Clear the legacy deferred flag
            clear_merge_deferred_metadata(&mut updated);
            updated.touch();

            if let Err(e) = self.task_repo.update(&updated).await {
                tracing::warn!(
                    event = "merge_retry_failed",
                    error = %e,
                    task_id = task.id.as_str(),
                    reason = "metadata_update_failed",
                    "Failed to update task metadata with retry event, skipping retry"
                );
                continue;
            }

            tracing::info!(
                task_id = task.id.as_str(),
                attempt = attempt_count,
                "Appended auto_retry_triggered event, re-invoking merge attempt"
            );

            // Re-invoke entry actions for PendingMerge to re-run attempt_programmatic_merge
            let transition_service = self.build_transition_service();
            transition_service
                .execute_entry_actions(&task.id, &updated, InternalStatus::PendingMerge)
                .await;

            // Only retry one deferred merge at a time to serialize them properly
            break;
        }
    }

    /// Retry main-branch merges that were deferred because agents were running.
    ///
    /// Called when the global running_count transitions to 0 (all agents idle).
    /// Finds tasks in PendingMerge with `main_merge_deferred` metadata, clears the flag,
    /// and re-invokes their entry actions to retry the main-branch merge.
    async fn try_retry_main_merges(&self) {
        use crate::domain::state_machine::transition_handler::{
            clear_main_merge_deferred_metadata, has_main_merge_deferred_metadata,
            is_main_merge_deferred_timed_out, DEFERRED_MERGE_TIMEOUT_SECONDS,
        };

        // Query all projects for main-merge-deferred tasks
        let projects = match self.project_repo.get_all().await {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to fetch projects for main merge retry"
                );
                return;
            }
        };

        let mut deferred_tasks: Vec<Task> = Vec::new();

        for project in &projects {
            let tasks = match self.task_repo.get_by_project(&project.id).await {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        project_id = project.id.as_str(),
                        "Failed to fetch tasks for main merge retry"
                    );
                    continue;
                }
            };

            for task in tasks {
                if task.internal_status == InternalStatus::PendingMerge
                    && has_main_merge_deferred_metadata(&task)
                {
                    deferred_tasks.push(task);
                }
            }
        }

        let deferred_count = deferred_tasks.len();

        if deferred_count == 0 {
            tracing::debug!("No main-merge-deferred tasks to retry");
            return;
        }

        tracing::info!(
            deferred_count = deferred_count,
            "Found main-merge-deferred tasks to retry (all agents now idle)"
        );

        for task in deferred_tasks {
            // Check if this deferred merge has exceeded the configured timeout.
            // If so, bypass the sibling guard and force a retry with a warning.
            let timed_out = is_main_merge_deferred_timed_out(&task);

            // Plan-level guard: skip retry if sibling tasks are not all terminal.
            // Bypassed when the deferred merge has exceeded DEFERRED_MERGE_TIMEOUT_SECONDS.
            if !timed_out {
                if let Some(ref session_id) = task.ideation_session_id {
                    match self.task_repo.get_by_ideation_session(session_id).await {
                        Ok(siblings) => {
                            let all_siblings_terminal = siblings.iter().all(|t| {
                                t.id == task.id
                                    || t.internal_status == InternalStatus::PendingMerge
                                    || t.is_terminal()
                            });
                            if !all_siblings_terminal {
                                tracing::info!(
                                    task_id = task.id.as_str(),
                                    session_id = %session_id,
                                    "Skipping main merge retry: sibling plan tasks not yet terminal"
                                );
                                continue;
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                task_id = task.id.as_str(),
                                "Failed to fetch siblings for plan-level merge guard, skipping retry"
                            );
                            continue;
                        }
                    }
                }
            } else {
                tracing::warn!(
                    event = "deferred_merge_timeout_forced_retry",
                    task_id = task.id.as_str(),
                    project_id = task.project_id.as_str(),
                    timeout_seconds = DEFERRED_MERGE_TIMEOUT_SECONDS,
                    "Deferred main merge has exceeded timeout — forcing retry regardless of sibling state"
                );
            }

            tracing::info!(
                event = "main_merge_retry_attempt",
                task_id = task.id.as_str(),
                project_id = task.project_id.as_str(),
                timed_out = timed_out,
                "Retrying deferred main merge (agents now idle)"
            );

            // Append main_merge_retry event before clearing deferred flag
            let mut updated = task.clone();

            // Get or create merge recovery metadata
            let mut recovery =
                MergeRecoveryMetadata::from_task_metadata(updated.metadata.as_deref())
                    .unwrap_or(None)
                    .unwrap_or_else(MergeRecoveryMetadata::new);

            // Count previous main merge retry attempts from events
            let attempt_count = recovery
                .events
                .iter()
                .filter(|e| matches!(e.kind, MergeRecoveryEventKind::MainMergeRetry))
                .count() as u32
                + 1;

            // Create main_merge_retry event
            // Extract target_branch from metadata if available
            let target_branch = updated
                .metadata
                .as_ref()
                .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
                .and_then(|v| {
                    v.get("target_branch")
                        .and_then(|t| t.as_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| "main".to_string());

            let (reason_code, retry_message) = if timed_out {
                (
                    MergeRecoveryReasonCode::DeferredTimeout,
                    format!(
                        "Main merge retry attempt {} (forced): deferred for >{}s, bypassing sibling guard",
                        attempt_count, DEFERRED_MERGE_TIMEOUT_SECONDS
                    ),
                )
            } else {
                (
                    MergeRecoveryReasonCode::AgentsRunning,
                    format!(
                        "Main merge retry attempt {}: all agents now idle",
                        attempt_count
                    ),
                )
            };

            let retry_event = MergeRecoveryEvent::new(
                MergeRecoveryEventKind::MainMergeRetry,
                MergeRecoverySource::Auto,
                reason_code,
                retry_message,
            )
            .with_target_branch(&target_branch)
            .with_attempt(attempt_count);

            // Append event and update state to Retrying
            recovery.append_event_with_state(retry_event, MergeRecoveryState::Retrying);

            // Update task metadata
            match recovery.update_task_metadata(updated.metadata.as_deref()) {
                Ok(updated_json) => {
                    updated.metadata = Some(updated_json);
                }
                Err(e) => {
                    tracing::error!(
                        task_id = task.id.as_str(),
                        error = %e,
                        "Failed to serialize merge recovery metadata during main merge retry"
                    );
                }
            }

            // Clear the main_merge_deferred flag
            clear_main_merge_deferred_metadata(&mut updated);
            updated.touch();

            if let Err(e) = self.task_repo.update(&updated).await {
                tracing::warn!(
                    event = "main_merge_retry_failed",
                    error = %e,
                    task_id = task.id.as_str(),
                    reason = "metadata_update_failed",
                    "Failed to update task metadata, skipping main merge retry"
                );
                continue;
            }

            tracing::info!(
                task_id = task.id.as_str(),
                attempt = attempt_count,
                "Appended main_merge_retry event, re-invoking merge attempt"
            );

            // Re-invoke entry actions for PendingMerge to re-run attempt_programmatic_merge
            let transition_service = self.build_transition_service();
            transition_service
                .execute_entry_actions(&task.id, &updated, InternalStatus::PendingMerge)
                .await;

            // Only retry one main merge at a time to serialize them properly
            break;
        }
    }
}

/// Periodic watchdog that detects tasks stuck in Ready state and reschedules them.
///
/// Safety net for scenarios S5, S6, S7, S8 where the primary scheduling trigger
/// (on_enter(Ready) or on_exit completion) may have been missed due to:
/// - Lock contention in try_lock()
/// - Scheduler unavailable (None) when task became Ready
/// - Timing races with the 600ms spawn delay
/// - Max concurrent capacity temporarily blocking schedule
///
/// The watchdog scans for Ready tasks older than `stale_threshold_secs` every
/// `interval_secs` and calls `try_schedule_ready_tasks()` to reschedule them.
pub struct ReadyWatchdog {
    scheduler: Arc<dyn TaskScheduler>,
    task_repo: Arc<dyn crate::domain::repositories::TaskRepository>,
    /// How often to run the watchdog scan.
    interval_secs: u64,
    /// How long a task must be in Ready state before being considered stale.
    stale_threshold_secs: u64,
}

impl ReadyWatchdog {
    /// Create a new ReadyWatchdog with configuration from scheduler_config().
    pub fn new(
        scheduler: Arc<dyn TaskScheduler>,
        task_repo: Arc<dyn crate::domain::repositories::TaskRepository>,
    ) -> Self {
        let sched_cfg = scheduler_config();
        Self {
            scheduler,
            task_repo,
            interval_secs: sched_cfg.watchdog_interval_secs,
            stale_threshold_secs: sched_cfg.watchdog_stale_threshold_secs,
        }
    }

    /// Override the scan interval (builder pattern).
    pub fn with_interval_secs(mut self, interval_secs: u64) -> Self {
        self.interval_secs = interval_secs;
        self
    }

    /// Override the staleness threshold (builder pattern).
    pub fn with_stale_threshold_secs(mut self, threshold_secs: u64) -> Self {
        self.stale_threshold_secs = threshold_secs;
        self
    }

    /// Run one watchdog cycle: scan for stale Ready tasks and reschedule if any are found.
    ///
    /// Returns the number of stale tasks found (0 means no action was taken).
    pub async fn run_once(&self) -> usize {
        match self
            .task_repo
            .get_stale_ready_tasks(self.stale_threshold_secs)
            .await
        {
            Ok(stale_tasks) => {
                let count = stale_tasks.len();
                if count > 0 {
                    tracing::warn!(
                        stale_count = count,
                        threshold_secs = self.stale_threshold_secs,
                        "Watchdog: found stale Ready tasks, triggering reschedule"
                    );
                    self.scheduler.try_schedule_ready_tasks().await;
                } else {
                    tracing::debug!("Watchdog: no stale Ready tasks found");
                }
                count
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "Watchdog: failed to query stale Ready tasks"
                );
                0
            }
        }
    }

    /// Run the watchdog loop indefinitely, sleeping `interval_secs` between cycles.
    ///
    /// This is intended to be spawned as a background task at application startup.
    pub async fn run_loop(&self) {
        let interval = std::time::Duration::from_secs(self.interval_secs);
        loop {
            tokio::time::sleep(interval).await;
            self.run_once().await;
        }
    }
}

#[cfg(test)]
#[path = "task_scheduler_service_tests.rs"]
mod tests;
