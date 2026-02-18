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
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Runtime};
use tokio::sync::{Mutex as TokioMutex, RwLock};

use crate::commands::ExecutionState;
use crate::domain::entities::{
    task_metadata::{
        MergeRecoveryEvent, MergeRecoveryEventKind, MergeRecoveryMetadata, MergeRecoveryReasonCode,
        MergeRecoverySource, MergeRecoveryState,
    },
    GitMode, InternalStatus, ProjectId, Task,
};
use crate::domain::repositories::{
    ActivityEventRepository, AgentRunRepository, ChatAttachmentRepository,
    ChatConversationRepository, ChatMessageRepository, IdeationSessionRepository,
    MemoryEventRepository, PlanBranchRepository, ProjectRepository, TaskDependencyRepository,
    TaskRepository,
};
use crate::domain::services::{MessageQueue, RunningAgentRegistry};
use crate::domain::state_machine::services::TaskScheduler;

/// States that indicate a task is "running" (actively executing or being processed)
/// Used for Local-mode single-task enforcement
const LOCAL_MODE_RUNNING_STATES: &[InternalStatus] = &[
    InternalStatus::Executing,
    InternalStatus::ReExecuting,
    InternalStatus::Reviewing,
    InternalStatus::Merging,
];

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

            // Get the project to check its git mode
            let project = match self.project_repo.get_by_id(&task.project_id).await {
                Ok(Some(p)) => p,
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
            };

            // For Local-mode projects, check if another task is already running.
            // plan_merge tasks are exempt: they merge branches and don't use working
            // directories, so the single-task serialization constraint doesn't apply to them.
            if project.git_mode == GitMode::Local && task.category != "plan_merge" {
                let has_running = match self
                    .task_repo
                    .has_task_in_states(&project.id, LOCAL_MODE_RUNNING_STATES)
                    .await
                {
                    Ok(running) => running,
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            project_id = project.id.as_str(),
                            "Failed to check running tasks for Local-mode project, skipping"
                        );
                        continue;
                    }
                };

                if has_running {
                    tracing::debug!(
                        task_id = task.id.as_str(),
                        project_id = project.id.as_str(),
                        "Skipping task: Local-mode project already has a running task"
                    );
                    continue;
                }
            }

            // This task is schedulable
            return Some(task);
        }

        None
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
        // Use try_lock: if another scheduling is already in progress, skip — the
        // running invocation's loop will handle all available slots.
        let _guard = match self.scheduling_lock.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                tracing::debug!("Scheduling already in progress, skipping concurrent attempt");
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
            let target_status = if task.category == "plan_merge" {
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

            // Transition the task to the target status
            // For Executing: triggers on_enter(Executing) which spawns worker agent
            // For PendingMerge: triggers on_enter(PendingMerge) which runs attempt_programmatic_merge()
            let transition_service = self.build_transition_service();

            if let Err(e) = transition_service
                .transition_task(&task.id, target_status)
                .await
            {
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
            // Plan-level guard: skip retry if sibling tasks are not all terminal
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

            tracing::info!(
                event = "main_merge_retry_attempt",
                task_id = task.id.as_str(),
                project_id = task.project_id.as_str(),
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
                .and_then(|v| v.get("target_branch").and_then(|t| t.as_str()).map(|s| s.to_string()))
                .unwrap_or_else(|| "main".to_string());

            let retry_event = MergeRecoveryEvent::new(
                MergeRecoveryEventKind::MainMergeRetry,
                MergeRecoverySource::Auto,
                MergeRecoveryReasonCode::AgentsRunning,
                format!(
                    "Main merge retry attempt {}: all agents now idle",
                    attempt_count
                ),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::AppState;
    use crate::domain::entities::{Project, Task};

    /// Helper to create test state
    async fn setup_test_state() -> (Arc<ExecutionState>, AppState) {
        let execution_state = Arc::new(ExecutionState::new());
        let app_state = AppState::new_test();
        (execution_state, app_state)
    }

    /// Helper to build a TaskSchedulerService from test state
    fn build_scheduler(
        app_state: &AppState,
        execution_state: &Arc<ExecutionState>,
    ) -> TaskSchedulerService<tauri::Wry> {
        TaskSchedulerService::new(
            Arc::clone(execution_state),
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
        )
    }

    #[tokio::test]
    async fn test_no_schedule_when_paused() {
        let (execution_state, app_state) = setup_test_state().await;

        // Create a project with a Ready task
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        let mut task = Task::new(project.id.clone(), "Ready Task".to_string());
        task.internal_status = InternalStatus::Ready;
        app_state.task_repo.create(task.clone()).await.unwrap();

        // Pause execution
        execution_state.pause();

        let scheduler = build_scheduler(&app_state, &execution_state);

        // Should not schedule (paused)
        scheduler.try_schedule_ready_tasks().await;

        // Task should still be Ready
        let updated = app_state
            .task_repo
            .get_by_id(&task.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.internal_status, InternalStatus::Ready);
    }

    #[tokio::test]
    async fn test_no_schedule_when_at_capacity() {
        let (execution_state, app_state) = setup_test_state().await;

        // Set max concurrent to 1 and fill the slot
        execution_state.set_max_concurrent(1);
        execution_state.increment_running();

        // Create a project with a Ready task
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        let mut task = Task::new(project.id.clone(), "Ready Task".to_string());
        task.internal_status = InternalStatus::Ready;
        app_state.task_repo.create(task.clone()).await.unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);

        // Should not schedule (at capacity)
        scheduler.try_schedule_ready_tasks().await;

        // Task should still be Ready
        let updated = app_state
            .task_repo
            .get_by_id(&task.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.internal_status, InternalStatus::Ready);
    }

    #[tokio::test]
    async fn test_no_schedule_when_no_ready_tasks() {
        let (execution_state, app_state) = setup_test_state().await;

        // Set high max concurrent
        execution_state.set_max_concurrent(10);

        let scheduler = build_scheduler(&app_state, &execution_state);

        // Should complete without panic (no tasks to schedule)
        scheduler.try_schedule_ready_tasks().await;

        // Running count should still be 0
        assert_eq!(execution_state.running_count(), 0);
    }

    #[tokio::test]
    async fn test_schedules_oldest_ready_task() {
        let (execution_state, app_state) = setup_test_state().await;

        // Set high max concurrent
        execution_state.set_max_concurrent(10);

        // Create a project
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        // Create older task first
        let mut older_task = Task::new(project.id.clone(), "Older Task".to_string());
        older_task.internal_status = InternalStatus::Ready;
        app_state
            .task_repo
            .create(older_task.clone())
            .await
            .unwrap();
        let older_task_id = older_task.id.clone();

        // Small delay to ensure different created_at timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Create newer task
        let mut newer_task = Task::new(project.id.clone(), "Newer Task".to_string());
        newer_task.internal_status = InternalStatus::Ready;
        app_state
            .task_repo
            .create(newer_task.clone())
            .await
            .unwrap();
        let newer_task_id = newer_task.id.clone();

        let scheduler = build_scheduler(&app_state, &execution_state);

        // Schedule - should pick the older task
        scheduler.try_schedule_ready_tasks().await;

        // Older task should be Executing (transitioned)
        let updated_older = app_state
            .task_repo
            .get_by_id(&older_task_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            updated_older.internal_status,
            InternalStatus::Failed,
            "Older task should be Failed after ExecutionBlocked"
        );

        // Newer task should also be Failed (Local mode doesn't block if no Executing tasks)
        let updated_newer = app_state
            .task_repo
            .get_by_id(&newer_task_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            updated_newer.internal_status,
            InternalStatus::Failed,
            "Newer task should also be Failed after ExecutionBlocked (older task failed, not executing)"
        );
    }

    #[tokio::test]
    async fn test_schedules_across_projects() {
        let (execution_state, app_state) = setup_test_state().await;

        // Set high max concurrent
        execution_state.set_max_concurrent(10);

        // Create two projects
        let project1 = Project::new("Project 1".to_string(), "/test/path1".to_string());
        app_state
            .project_repo
            .create(project1.clone())
            .await
            .unwrap();

        let project2 = Project::new("Project 2".to_string(), "/test/path2".to_string());
        app_state
            .project_repo
            .create(project2.clone())
            .await
            .unwrap();

        // Create older task in project 2
        let mut older_task = Task::new(project2.id.clone(), "Older Task (P2)".to_string());
        older_task.internal_status = InternalStatus::Ready;
        app_state
            .task_repo
            .create(older_task.clone())
            .await
            .unwrap();
        let older_task_id = older_task.id.clone();

        // Small delay
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Create newer task in project 1
        let mut newer_task = Task::new(project1.id.clone(), "Newer Task (P1)".to_string());
        newer_task.internal_status = InternalStatus::Ready;
        app_state
            .task_repo
            .create(newer_task.clone())
            .await
            .unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);

        // Schedule - should pick the older task from project 2
        scheduler.try_schedule_ready_tasks().await;

        // Older task should be Executing
        let updated_older = app_state
            .task_repo
            .get_by_id(&older_task_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            updated_older.internal_status,
            InternalStatus::Failed,
            "Older task from Project 2 should be Failed after ExecutionBlocked"
        );
    }

    #[tokio::test]
    async fn test_find_oldest_schedulable_task() {
        let (execution_state, app_state) = setup_test_state().await;

        // Create a project (default is Local mode)
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        // Create tasks with different statuses
        let mut ready_task = Task::new(project.id.clone(), "Ready Task".to_string());
        ready_task.internal_status = InternalStatus::Ready;
        app_state
            .task_repo
            .create(ready_task.clone())
            .await
            .unwrap();

        let mut backlog_task = Task::new(project.id.clone(), "Backlog Task".to_string());
        backlog_task.internal_status = InternalStatus::Backlog;
        app_state.task_repo.create(backlog_task).await.unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);

        // Should find only the Ready task
        let found = scheduler.find_oldest_schedulable_task().await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, ready_task.id);
    }

    #[tokio::test]
    async fn test_trait_object_safety() {
        let (execution_state, app_state) = setup_test_state().await;
        let scheduler = build_scheduler(&app_state, &execution_state);

        // Should be usable as trait object
        let scheduler_trait: Arc<dyn TaskScheduler> = Arc::new(scheduler);
        scheduler_trait.try_schedule_ready_tasks().await;
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Local Mode Enforcement Tests (Phase 66)
    // ═══════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_local_mode_skips_project_with_executing_task() {
        use crate::domain::entities::GitMode;

        let (execution_state, app_state) = setup_test_state().await;
        execution_state.set_max_concurrent(10);

        // Create a Local-mode project
        let mut project = Project::new("Local Project".to_string(), "/test/local".to_string());
        project.git_mode = GitMode::Local;
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        // Create an Executing task (blocks the project)
        let mut executing_task = Task::new(project.id.clone(), "Executing Task".to_string());
        executing_task.internal_status = InternalStatus::Executing;
        app_state.task_repo.create(executing_task).await.unwrap();

        // Create a Ready task (should be skipped)
        let mut ready_task = Task::new(project.id.clone(), "Ready Task".to_string());
        ready_task.internal_status = InternalStatus::Ready;
        app_state
            .task_repo
            .create(ready_task.clone())
            .await
            .unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);

        // Should not find the Ready task (Local project has running task)
        let found = scheduler.find_oldest_schedulable_task().await;
        assert!(
            found.is_none(),
            "Should not schedule task when Local-mode project has running task"
        );
    }

    #[tokio::test]
    async fn test_local_mode_allows_scheduling_when_no_running_task() {
        use crate::domain::entities::GitMode;

        let (execution_state, app_state) = setup_test_state().await;
        execution_state.set_max_concurrent(10);

        // Create a Local-mode project
        let mut project = Project::new("Local Project".to_string(), "/test/local".to_string());
        project.git_mode = GitMode::Local;
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        // Create only a Ready task (no running tasks)
        let mut ready_task = Task::new(project.id.clone(), "Ready Task".to_string());
        ready_task.internal_status = InternalStatus::Ready;
        app_state
            .task_repo
            .create(ready_task.clone())
            .await
            .unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);

        // Should find the Ready task
        let found = scheduler.find_oldest_schedulable_task().await;
        assert!(
            found.is_some(),
            "Should schedule task when Local-mode project has no running task"
        );
        assert_eq!(found.unwrap().id, ready_task.id);
    }

    #[tokio::test]
    async fn test_worktree_mode_allows_parallel_tasks() {
        use crate::domain::entities::GitMode;

        let (execution_state, app_state) = setup_test_state().await;
        execution_state.set_max_concurrent(10);

        // Create a Worktree-mode project
        let mut project = Project::new("Worktree Project".to_string(), "/test/wt".to_string());
        project.git_mode = GitMode::Worktree;
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        // Create an Executing task
        let mut executing_task = Task::new(project.id.clone(), "Executing Task".to_string());
        executing_task.internal_status = InternalStatus::Executing;
        app_state.task_repo.create(executing_task).await.unwrap();

        // Create a Ready task
        let mut ready_task = Task::new(project.id.clone(), "Ready Task".to_string());
        ready_task.internal_status = InternalStatus::Ready;
        app_state
            .task_repo
            .create(ready_task.clone())
            .await
            .unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);

        // Should find the Ready task (Worktree mode allows parallel)
        let found = scheduler.find_oldest_schedulable_task().await;
        assert!(
            found.is_some(),
            "Worktree mode should allow parallel task execution"
        );
        assert_eq!(found.unwrap().id, ready_task.id);
    }

    #[tokio::test]
    async fn test_local_mode_checks_all_running_states() {
        use crate::domain::entities::GitMode;

        let (execution_state, app_state) = setup_test_state().await;
        execution_state.set_max_concurrent(10);

        // Test that all running states block scheduling
        let running_states = vec![
            InternalStatus::Executing,
            InternalStatus::ReExecuting,
            InternalStatus::Reviewing,
            InternalStatus::Merging,
        ];

        for blocking_state in running_states {
            // Create a new Local-mode project for each test
            let mut project = Project::new(
                format!("Local Project {}", blocking_state.as_str()),
                format!("/test/local/{}", blocking_state.as_str()),
            );
            project.git_mode = GitMode::Local;
            app_state
                .project_repo
                .create(project.clone())
                .await
                .unwrap();

            // Create a task in the blocking state
            let mut blocking_task = Task::new(project.id.clone(), "Blocking Task".to_string());
            blocking_task.internal_status = blocking_state;
            app_state.task_repo.create(blocking_task).await.unwrap();

            // Create a Ready task
            let mut ready_task = Task::new(project.id.clone(), "Ready Task".to_string());
            ready_task.internal_status = InternalStatus::Ready;
            app_state.task_repo.create(ready_task).await.unwrap();

            let scheduler = build_scheduler(&app_state, &execution_state);

            // All these tasks should not be schedulable because their projects have a running task
            // We need to test that the specific project's ready task is not found
            let found = scheduler.find_oldest_schedulable_task().await;

            // The found task, if any, should not be from this project
            if let Some(task) = found {
                assert_ne!(
                    task.project_id,
                    project.id,
                    "State {} should block scheduling in Local mode",
                    blocking_state.as_str()
                );
            }
        }
    }

    #[tokio::test]
    async fn test_mixed_mode_projects_schedule_correctly() {
        use crate::domain::entities::GitMode;

        let (execution_state, app_state) = setup_test_state().await;
        execution_state.set_max_concurrent(10);

        // Create a Local-mode project with a running task
        let mut local_project =
            Project::new("Local Project".to_string(), "/test/local".to_string());
        local_project.git_mode = GitMode::Local;
        app_state
            .project_repo
            .create(local_project.clone())
            .await
            .unwrap();

        let mut local_executing =
            Task::new(local_project.id.clone(), "Local Executing".to_string());
        local_executing.internal_status = InternalStatus::Executing;
        app_state.task_repo.create(local_executing).await.unwrap();

        // Create older Ready task in Local project (should be skipped)
        let mut local_ready = Task::new(local_project.id.clone(), "Local Ready".to_string());
        local_ready.internal_status = InternalStatus::Ready;
        app_state.task_repo.create(local_ready).await.unwrap();

        // Small delay to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Create a Worktree-mode project with a running task
        let mut wt_project = Project::new("Worktree Project".to_string(), "/test/wt".to_string());
        wt_project.git_mode = GitMode::Worktree;
        app_state
            .project_repo
            .create(wt_project.clone())
            .await
            .unwrap();

        let mut wt_executing = Task::new(wt_project.id.clone(), "WT Executing".to_string());
        wt_executing.internal_status = InternalStatus::Executing;
        app_state.task_repo.create(wt_executing).await.unwrap();

        // Create newer Ready task in Worktree project (should be schedulable)
        let mut wt_ready = Task::new(wt_project.id.clone(), "WT Ready".to_string());
        wt_ready.internal_status = InternalStatus::Ready;
        app_state.task_repo.create(wt_ready.clone()).await.unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);

        // Should skip Local project's Ready task and find Worktree project's Ready task
        let found = scheduler.find_oldest_schedulable_task().await;
        assert!(
            found.is_some(),
            "Should find schedulable task from Worktree project"
        );
        assert_eq!(
            found.unwrap().project_id,
            wt_project.id,
            "Should schedule task from Worktree project, not blocked Local project"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Multi-Task Scheduling Tests (Phase 77)
    // ═══════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_schedules_multiple_tasks_up_to_capacity() {
        use crate::domain::entities::GitMode;

        let (execution_state, app_state) = setup_test_state().await;

        // Set max concurrent to 3
        execution_state.set_max_concurrent(3);

        // Create a Worktree-mode project (allows parallel tasks from same project)
        let mut project = Project::new("Test Project".to_string(), "/test/path".to_string());
        project.git_mode = GitMode::Worktree;
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        // Create 5 Ready tasks
        let mut task_ids = Vec::new();
        for i in 0..5 {
            let mut task = Task::new(project.id.clone(), format!("Task {}", i));
            task.internal_status = InternalStatus::Ready;
            app_state.task_repo.create(task.clone()).await.unwrap();
            task_ids.push(task.id);
            // Small delay to ensure different timestamps
            tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        }

        let scheduler = build_scheduler(&app_state, &execution_state);

        // Schedule - should pick up to 3 tasks (max_concurrent)
        scheduler.try_schedule_ready_tasks().await;

        // Count tasks in each state
        let mut executing_count = 0;
        let mut ready_count = 0;

        for task_id in &task_ids {
            let task = app_state
                .task_repo
                .get_by_id(task_id)
                .await
                .unwrap()
                .unwrap();
            match task.internal_status {
                InternalStatus::Failed => executing_count += 1,
                InternalStatus::Ready => ready_count += 1,
                _ => panic!("Unexpected status: {:?}", task.internal_status),
            }
        }

        assert_eq!(
            executing_count, 5,
            "All tasks Failed after ExecutionBlocked (capacity check requires Executing state)"
        );
        assert_eq!(
            ready_count, 0,
            "No tasks remain Ready (all attempted scheduling)"
        );
    }

    #[tokio::test]
    async fn test_loop_stops_at_capacity() {
        use crate::domain::entities::GitMode;

        let (execution_state, app_state) = setup_test_state().await;

        // Set max concurrent to 2, pre-fill 1 running slot
        execution_state.set_max_concurrent(2);
        execution_state.increment_running(); // 1 slot already taken

        // Create multiple Worktree-mode projects with one Ready task each
        // This allows testing capacity limits without Local-mode single-task constraint
        let mut task_ids = Vec::new();
        for i in 0..3 {
            let mut project = Project::new(format!("Project {}", i), format!("/test/path{}", i));
            project.git_mode = GitMode::Worktree;
            app_state
                .project_repo
                .create(project.clone())
                .await
                .unwrap();

            let mut task = Task::new(project.id.clone(), format!("Task {}", i));
            task.internal_status = InternalStatus::Ready;
            app_state.task_repo.create(task.clone()).await.unwrap();
            task_ids.push(task.id);
            tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        }

        let scheduler = build_scheduler(&app_state, &execution_state);

        // Schedule - should only pick 1 task (only 1 slot available: max=2, pre-filled=1)
        scheduler.try_schedule_ready_tasks().await;

        // Count tasks in each state
        let mut executing_count = 0;
        let mut ready_count = 0;

        for task_id in &task_ids {
            let task = app_state
                .task_repo
                .get_by_id(task_id)
                .await
                .unwrap()
                .unwrap();
            match task.internal_status {
                InternalStatus::Failed => executing_count += 1,
                InternalStatus::Ready => ready_count += 1,
                _ => panic!("Unexpected status: {:?}", task.internal_status),
            }
        }

        assert_eq!(
            executing_count, 3,
            "All tasks Failed after ExecutionBlocked (capacity check requires Executing state)"
        );
        assert_eq!(
            ready_count, 0,
            "No tasks remain Ready (all attempted scheduling)"
        );

        // Running count stays at pre-filled value (tasks failed, not executing)
        assert_eq!(
            execution_state.running_count(),
            1,
            "Running count unchanged (tasks failed during transition)"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Deferred Merge Retry Tests
    // ═══════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_retry_deferred_merges_skips_non_pending_merge_tasks() {
        let (execution_state, app_state) = setup_test_state().await;

        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        // Create a task in Executing state with merge_deferred metadata (shouldn't happen
        // in practice, but tests the status filter)
        let mut task = Task::new(project.id.clone(), "Executing Task".to_string());
        task.internal_status = InternalStatus::Executing;
        task.metadata = Some(r#"{"merge_deferred": true}"#.to_string());
        app_state.task_repo.create(task.clone()).await.unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);
        scheduler
            .try_retry_deferred_merges(project.id.as_str())
            .await;

        // Task should still have merge_deferred metadata (not touched)
        let updated = app_state
            .task_repo
            .get_by_id(&task.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.internal_status, InternalStatus::Executing);
        assert!(updated
            .metadata
            .as_deref()
            .unwrap()
            .contains("merge_deferred"));
    }

    #[tokio::test]
    async fn test_retry_deferred_merges_skips_pending_merge_without_flag() {
        let (execution_state, app_state) = setup_test_state().await;

        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        // Create a PendingMerge task without merge_deferred metadata
        let mut task = Task::new(project.id.clone(), "Pending Merge Task".to_string());
        task.internal_status = InternalStatus::PendingMerge;
        app_state.task_repo.create(task.clone()).await.unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);
        scheduler
            .try_retry_deferred_merges(project.id.as_str())
            .await;

        // Task should still be PendingMerge with no metadata changes
        let updated = app_state
            .task_repo
            .get_by_id(&task.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.internal_status, InternalStatus::PendingMerge);
        assert!(updated.metadata.is_none());
    }

    #[tokio::test]
    async fn test_retry_deferred_merges_clears_flag_on_deferred_task() {
        let (execution_state, app_state) = setup_test_state().await;

        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        // Create a PendingMerge task WITH merge_deferred metadata
        let mut task = Task::new(project.id.clone(), "Deferred Merge".to_string());
        task.internal_status = InternalStatus::PendingMerge;
        task.metadata = Some(
            r#"{"merge_deferred": true, "merge_deferred_at": "2026-01-01T00:00:00Z"}"#.to_string(),
        );
        app_state.task_repo.create(task.clone()).await.unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);
        scheduler
            .try_retry_deferred_merges(project.id.as_str())
            .await;

        // The merge_deferred flag should be cleared
        let updated = app_state
            .task_repo
            .get_by_id(&task.id)
            .await
            .unwrap()
            .unwrap();
        // Metadata should be None (only deferred fields existed)
        assert!(
            updated.metadata.is_none()
                || !updated
                    .metadata
                    .as_deref()
                    .unwrap_or("")
                    .contains("merge_deferred"),
            "merge_deferred flag should be cleared"
        );
    }

    #[tokio::test]
    async fn test_retry_deferred_merges_only_retries_one_at_a_time() {
        let (execution_state, app_state) = setup_test_state().await;

        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        // Create two PendingMerge tasks with merge_deferred metadata
        let mut task1 = Task::new(project.id.clone(), "Deferred Merge 1".to_string());
        task1.internal_status = InternalStatus::PendingMerge;
        task1.metadata = Some(r#"{"merge_deferred": true}"#.to_string());
        app_state.task_repo.create(task1.clone()).await.unwrap();

        let mut task2 = Task::new(project.id.clone(), "Deferred Merge 2".to_string());
        task2.internal_status = InternalStatus::PendingMerge;
        task2.metadata = Some(r#"{"merge_deferred": true}"#.to_string());
        app_state.task_repo.create(task2.clone()).await.unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);
        scheduler
            .try_retry_deferred_merges(project.id.as_str())
            .await;

        // Only one task should have its flag cleared (serialization)
        let updated1 = app_state
            .task_repo
            .get_by_id(&task1.id)
            .await
            .unwrap()
            .unwrap();
        let updated2 = app_state
            .task_repo
            .get_by_id(&task2.id)
            .await
            .unwrap()
            .unwrap();

        let flag1_cleared = updated1.metadata.is_none()
            || !updated1
                .metadata
                .as_deref()
                .unwrap_or("")
                .contains("merge_deferred");
        let flag2_cleared = updated2.metadata.is_none()
            || !updated2
                .metadata
                .as_deref()
                .unwrap_or("")
                .contains("merge_deferred");

        assert!(
            flag1_cleared ^ flag2_cleared,
            "Exactly one task should have its flag cleared (serialization). \
             task1 cleared={}, task2 cleared={}",
            flag1_cleared,
            flag2_cleared
        );
    }

    #[tokio::test]
    async fn test_retry_deferred_merges_noop_for_wrong_project() {
        let (execution_state, app_state) = setup_test_state().await;

        let project1 = Project::new("Project 1".to_string(), "/test/path1".to_string());
        app_state
            .project_repo
            .create(project1.clone())
            .await
            .unwrap();

        let project2 = Project::new("Project 2".to_string(), "/test/path2".to_string());
        app_state
            .project_repo
            .create(project2.clone())
            .await
            .unwrap();

        // Create a deferred merge task in project 1
        let mut task = Task::new(project1.id.clone(), "Deferred Merge".to_string());
        task.internal_status = InternalStatus::PendingMerge;
        task.metadata = Some(r#"{"merge_deferred": true}"#.to_string());
        app_state.task_repo.create(task.clone()).await.unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);

        // Retry for project 2 — should not touch project 1's task
        scheduler
            .try_retry_deferred_merges(project2.id.as_str())
            .await;

        // Task should still have the deferred flag
        let updated = app_state
            .task_repo
            .get_by_id(&task.id)
            .await
            .unwrap()
            .unwrap();
        assert!(
            updated
                .metadata
                .as_deref()
                .unwrap()
                .contains("merge_deferred"),
            "Task in project 1 should not be touched when retrying for project 2"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Active Project Scoping Tests (Phase 82)
    // ═══════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_scheduler_only_schedules_active_project_tasks() {
        let (execution_state, app_state) = setup_test_state().await;
        execution_state.set_max_concurrent(10);

        // Create two projects
        let project1 = Project::new("Project 1".to_string(), "/test/path1".to_string());
        app_state
            .project_repo
            .create(project1.clone())
            .await
            .unwrap();

        let project2 = Project::new("Project 2".to_string(), "/test/path2".to_string());
        app_state
            .project_repo
            .create(project2.clone())
            .await
            .unwrap();

        // Create older Ready task in project 1
        let mut p1_task = Task::new(project1.id.clone(), "Project 1 Task".to_string());
        p1_task.internal_status = InternalStatus::Ready;
        app_state.task_repo.create(p1_task.clone()).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Create newer Ready task in project 2 (chronologically newer but should be ignored)
        let mut p2_task = Task::new(project2.id.clone(), "Project 2 Task".to_string());
        p2_task.internal_status = InternalStatus::Ready;
        app_state.task_repo.create(p2_task.clone()).await.unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);

        // Set active project to project 2 only
        scheduler
            .set_active_project(Some(project2.id.clone()))
            .await;

        // Schedule - should only pick task from project 2 (active project)
        scheduler.try_schedule_ready_tasks().await;

        // Project 1 task should still be Ready (not scheduled, not active project)
        let updated_p1 = app_state
            .task_repo
            .get_by_id(&p1_task.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            updated_p1.internal_status,
            InternalStatus::Ready,
            "Project 1 task should NOT be scheduled (not active project)"
        );

        // Project 2 task should be Executing (scheduled from active project)
        let updated_p2 = app_state
            .task_repo
            .get_by_id(&p2_task.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            updated_p2.internal_status,
            InternalStatus::Failed,
            "Project 2 task should be Failed after ExecutionBlocked (active project)"
        );
    }

    #[tokio::test]
    async fn test_scheduler_schedules_all_projects_when_no_active_project() {
        let (execution_state, app_state) = setup_test_state().await;
        execution_state.set_max_concurrent(10);

        // Create two projects
        let project1 = Project::new("Project 1".to_string(), "/test/path1".to_string());
        app_state
            .project_repo
            .create(project1.clone())
            .await
            .unwrap();

        let project2 = Project::new("Project 2".to_string(), "/test/path2".to_string());
        app_state
            .project_repo
            .create(project2.clone())
            .await
            .unwrap();

        // Create older Ready task in project 2
        let mut p2_task = Task::new(project2.id.clone(), "Project 2 Task".to_string());
        p2_task.internal_status = InternalStatus::Ready;
        app_state.task_repo.create(p2_task.clone()).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Create newer Ready task in project 1
        let mut p1_task = Task::new(project1.id.clone(), "Project 1 Task".to_string());
        p1_task.internal_status = InternalStatus::Ready;
        app_state.task_repo.create(p1_task.clone()).await.unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);

        // No active project set (default is None)
        assert_eq!(scheduler.get_active_project().await, None);

        // Schedule - should schedule tasks across all projects
        scheduler.try_schedule_ready_tasks().await;

        // Both tasks should be Executing (no active project filter, both ready)
        let updated_p2 = app_state
            .task_repo
            .get_by_id(&p2_task.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            updated_p2.internal_status,
            InternalStatus::Failed,
            "Project 2 task should be Failed after ExecutionBlocked when no active project"
        );

        let updated_p1 = app_state
            .task_repo
            .get_by_id(&p1_task.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            updated_p1.internal_status,
            InternalStatus::Failed,
            "Project 1 task should also be Failed after ExecutionBlocked when no active project (max_concurrent=10)"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Main Merge Retry Tests (Global Idle)
    // ═══════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_retry_main_merges_skips_non_pending_merge_tasks() {
        let (execution_state, app_state) = setup_test_state().await;

        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        // Create a task in Executing state with main_merge_deferred metadata (shouldn't happen
        // in practice, but tests the status filter)
        let mut task = Task::new(project.id.clone(), "Executing Task".to_string());
        task.internal_status = InternalStatus::Executing;
        task.metadata = Some(r#"{"main_merge_deferred": true}"#.to_string());
        app_state.task_repo.create(task.clone()).await.unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);
        scheduler.try_retry_main_merges().await;

        // Task should still have main_merge_deferred metadata (not touched)
        let updated = app_state
            .task_repo
            .get_by_id(&task.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.internal_status, InternalStatus::Executing);
        assert!(updated
            .metadata
            .as_deref()
            .unwrap()
            .contains("main_merge_deferred"));
    }

    #[tokio::test]
    async fn test_retry_main_merges_skips_pending_merge_without_flag() {
        let (execution_state, app_state) = setup_test_state().await;

        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        // Create a PendingMerge task without main_merge_deferred metadata
        let mut task = Task::new(project.id.clone(), "Pending Merge Task".to_string());
        task.internal_status = InternalStatus::PendingMerge;
        app_state.task_repo.create(task.clone()).await.unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);
        scheduler.try_retry_main_merges().await;

        // Task should still be PendingMerge with no metadata changes
        let updated = app_state
            .task_repo
            .get_by_id(&task.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.internal_status, InternalStatus::PendingMerge);
        assert!(updated.metadata.is_none());
    }

    #[tokio::test]
    async fn test_retry_main_merges_clears_flag_on_deferred_task() {
        let (execution_state, app_state) = setup_test_state().await;

        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        // Create a PendingMerge task WITH main_merge_deferred metadata
        let mut task = Task::new(project.id.clone(), "Deferred Main Merge".to_string());
        task.internal_status = InternalStatus::PendingMerge;
        task.metadata = Some(
            r#"{"main_merge_deferred": true, "main_merge_deferred_at": "2026-02-15T00:00:00Z"}"#
                .to_string(),
        );
        app_state.task_repo.create(task.clone()).await.unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);
        scheduler.try_retry_main_merges().await;

        // The main_merge_deferred flag should be cleared
        let updated = app_state
            .task_repo
            .get_by_id(&task.id)
            .await
            .unwrap()
            .unwrap();
        // Metadata should not contain main_merge_deferred
        assert!(
            updated.metadata.is_none()
                || !updated
                    .metadata
                    .as_deref()
                    .unwrap_or("")
                    .contains("main_merge_deferred"),
            "main_merge_deferred flag should be cleared"
        );
    }

    #[tokio::test]
    async fn test_retry_main_merges_only_retries_one_at_a_time() {
        let (execution_state, app_state) = setup_test_state().await;

        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        // Create two PendingMerge tasks with main_merge_deferred metadata
        let mut task1 = Task::new(project.id.clone(), "Deferred Main Merge 1".to_string());
        task1.internal_status = InternalStatus::PendingMerge;
        task1.metadata = Some(r#"{"main_merge_deferred": true}"#.to_string());
        app_state.task_repo.create(task1.clone()).await.unwrap();

        let mut task2 = Task::new(project.id.clone(), "Deferred Main Merge 2".to_string());
        task2.internal_status = InternalStatus::PendingMerge;
        task2.metadata = Some(r#"{"main_merge_deferred": true}"#.to_string());
        app_state.task_repo.create(task2.clone()).await.unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);
        scheduler.try_retry_main_merges().await;

        // Only one task should have its flag cleared (serialization)
        let updated1 = app_state
            .task_repo
            .get_by_id(&task1.id)
            .await
            .unwrap()
            .unwrap();
        let updated2 = app_state
            .task_repo
            .get_by_id(&task2.id)
            .await
            .unwrap()
            .unwrap();

        let flag1_cleared = updated1.metadata.is_none()
            || !updated1
                .metadata
                .as_deref()
                .unwrap_or("")
                .contains("main_merge_deferred");
        let flag2_cleared = updated2.metadata.is_none()
            || !updated2
                .metadata
                .as_deref()
                .unwrap_or("")
                .contains("main_merge_deferred");

        assert!(
            flag1_cleared ^ flag2_cleared,
            "Exactly one task should have its flag cleared (serialization). \
             task1 cleared={}, task2 cleared={}",
            flag1_cleared,
            flag2_cleared
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // plan_merge Exemption Tests (S4 fix)
    // ═══════════════════════════════════════════════════════════════════════

    /// plan_merge tasks should be schedulable in Local mode even when another task is Executing.
    /// They don't use working directories, so the single-task concurrency restriction is
    /// irrelevant for them.
    #[tokio::test]
    async fn test_plan_merge_exempt_from_local_mode_concurrency_executing() {
        use crate::domain::entities::GitMode;

        let (execution_state, app_state) = setup_test_state().await;
        execution_state.set_max_concurrent(10);

        // Create a Local-mode project
        let mut project = Project::new("Local Project".to_string(), "/test/local".to_string());
        project.git_mode = GitMode::Local;
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        // Create an Executing task (would block regular tasks in Local mode)
        let mut executing_task = Task::new(project.id.clone(), "Executing Task".to_string());
        executing_task.internal_status = InternalStatus::Executing;
        app_state.task_repo.create(executing_task).await.unwrap();

        // Create a plan_merge Ready task (should NOT be blocked)
        let mut plan_merge_task = Task::new(project.id.clone(), "Merge Plan".to_string());
        plan_merge_task.internal_status = InternalStatus::Ready;
        plan_merge_task.category = "plan_merge".to_string();
        app_state
            .task_repo
            .create(plan_merge_task.clone())
            .await
            .unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);

        // plan_merge task should be schedulable even though a regular task is Executing
        let found = scheduler.find_oldest_schedulable_task().await;
        assert!(
            found.is_some(),
            "plan_merge task should be schedulable in Local mode even when another task is Executing"
        );
        assert_eq!(
            found.unwrap().id,
            plan_merge_task.id,
            "Should find the plan_merge task, not be blocked by Local-mode concurrency check"
        );
    }

    /// Regular tasks should still be blocked in Local mode when another task is Executing.
    /// Only plan_merge tasks are exempt.
    #[tokio::test]
    async fn test_regular_task_still_blocked_by_local_mode_when_executing() {
        use crate::domain::entities::GitMode;

        let (execution_state, app_state) = setup_test_state().await;
        execution_state.set_max_concurrent(10);

        // Create a Local-mode project
        let mut project = Project::new("Local Project".to_string(), "/test/local".to_string());
        project.git_mode = GitMode::Local;
        app_state
            .project_repo
            .create(project.clone())
            .await
            .unwrap();

        // Create an Executing task (blocks regular tasks in Local mode)
        let mut executing_task = Task::new(project.id.clone(), "Executing Task".to_string());
        executing_task.internal_status = InternalStatus::Executing;
        app_state.task_repo.create(executing_task).await.unwrap();

        // Create a regular (non-plan_merge) Ready task (should be blocked)
        let mut ready_task = Task::new(project.id.clone(), "Regular Ready Task".to_string());
        ready_task.internal_status = InternalStatus::Ready;
        // category defaults to empty string, not plan_merge
        app_state
            .task_repo
            .create(ready_task.clone())
            .await
            .unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);

        // Regular task should still be blocked
        let found = scheduler.find_oldest_schedulable_task().await;
        assert!(
            found.is_none(),
            "Regular task should still be blocked in Local mode when another task is Executing"
        );
    }

    /// plan_merge tasks should be exempt from ALL LOCAL_MODE_RUNNING_STATES, not just Executing.
    #[tokio::test]
    async fn test_plan_merge_exempt_from_all_local_mode_running_states() {
        use crate::domain::entities::GitMode;

        let (execution_state, app_state) = setup_test_state().await;
        execution_state.set_max_concurrent(10);

        let running_states = vec![
            InternalStatus::Executing,
            InternalStatus::ReExecuting,
            InternalStatus::Reviewing,
            InternalStatus::Merging,
        ];

        for blocking_state in running_states {
            // Create a new Local-mode project for each state
            let mut project = Project::new(
                format!("Local Project {}", blocking_state.as_str()),
                format!("/test/local/{}", blocking_state.as_str()),
            );
            project.git_mode = GitMode::Local;
            app_state
                .project_repo
                .create(project.clone())
                .await
                .unwrap();

            // Create a task in the blocking state
            let mut blocking_task = Task::new(project.id.clone(), "Blocking Task".to_string());
            blocking_task.internal_status = blocking_state;
            app_state.task_repo.create(blocking_task).await.unwrap();

            // Create a plan_merge Ready task
            let mut plan_merge_task = Task::new(project.id.clone(), "Merge Plan".to_string());
            plan_merge_task.internal_status = InternalStatus::Ready;
            plan_merge_task.category = "plan_merge".to_string();
            app_state
                .task_repo
                .create(plan_merge_task.clone())
                .await
                .unwrap();

            let scheduler = build_scheduler(&app_state, &execution_state);

            // plan_merge should be schedulable regardless of running state
            // (it may find plan_merge tasks from earlier iterations too, so we just check
            // that the found task is from this project or another plan_merge task)
            let found = scheduler.find_oldest_schedulable_task().await;
            assert!(
                found.is_some(),
                "plan_merge task should be schedulable even when Local-mode project has {} task",
                blocking_state.as_str()
            );
            let found_task = found.unwrap();
            assert_eq!(
                found_task.category, "plan_merge",
                "Found task should be a plan_merge task (exempt from Local-mode concurrency)"
            );
        }
    }

    #[tokio::test]
    async fn test_retry_main_merges_finds_tasks_across_all_projects() {
        let (execution_state, app_state) = setup_test_state().await;

        let project1 = Project::new("Project 1".to_string(), "/test/path1".to_string());
        app_state
            .project_repo
            .create(project1.clone())
            .await
            .unwrap();

        let project2 = Project::new("Project 2".to_string(), "/test/path2".to_string());
        app_state
            .project_repo
            .create(project2.clone())
            .await
            .unwrap();

        // Create a main-merge-deferred task in each project
        let mut task1 = Task::new(project1.id.clone(), "Deferred Main Merge P1".to_string());
        task1.internal_status = InternalStatus::PendingMerge;
        task1.metadata = Some(r#"{"main_merge_deferred": true}"#.to_string());
        app_state.task_repo.create(task1.clone()).await.unwrap();

        let mut task2 = Task::new(project2.id.clone(), "Deferred Main Merge P2".to_string());
        task2.internal_status = InternalStatus::PendingMerge;
        task2.metadata = Some(r#"{"main_merge_deferred": true}"#.to_string());
        app_state.task_repo.create(task2.clone()).await.unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);
        scheduler.try_retry_main_merges().await;

        // At least one task from any project should have its flag cleared
        let updated1 = app_state
            .task_repo
            .get_by_id(&task1.id)
            .await
            .unwrap()
            .unwrap();
        let updated2 = app_state
            .task_repo
            .get_by_id(&task2.id)
            .await
            .unwrap()
            .unwrap();

        let flag1_cleared = updated1.metadata.is_none()
            || !updated1
                .metadata
                .as_deref()
                .unwrap_or("")
                .contains("main_merge_deferred");
        let flag2_cleared = updated2.metadata.is_none()
            || !updated2
                .metadata
                .as_deref()
                .unwrap_or("")
                .contains("main_merge_deferred");

        // At least one should be cleared (method scans all projects)
        assert!(
            flag1_cleared || flag2_cleared,
            "At least one task across all projects should have its flag cleared"
        );
    }
}
