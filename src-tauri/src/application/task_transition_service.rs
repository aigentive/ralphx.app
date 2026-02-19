// Task Transition Service
//
// Orchestrates task status transitions with proper state machine entry/exit actions.
// This service bridges the gap between simple status updates and the full state machine
// that triggers side effects like spawning worker agents.
//
// Key responsibilities:
// - Build TaskServices from AppState dependencies
// - Handle status transitions with proper entry actions
// - Spawn workers when moving to Executing state
// - Emit events for UI updates

use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};

use crate::application::{ChatService, ClaudeChatService};
use crate::commands::ExecutionState;
use crate::domain::entities::{InternalStatus, Task, TaskId};
use crate::domain::repositories::{
    ActivityEventRepository, AgentRunRepository, ChatAttachmentRepository,
    ChatConversationRepository, ChatMessageRepository, IdeationSessionRepository,
    MemoryEventRepository, PlanBranchRepository, ProjectRepository, TaskDependencyRepository,
    TaskRepository, TaskStepRepository,
};
use crate::domain::services::{MessageQueue, RunningAgentRegistry};
use crate::domain::state_machine::services::{
    AgentSpawner, DependencyManager, EventEmitter, Notifier, ReviewStartResult, ReviewStarter,
    TaskScheduler,
};
use crate::domain::state_machine::transition_handler::metadata_builder::{
    build_stop_metadata, build_trigger_origin_metadata, MetadataUpdate,
};
use crate::domain::state_machine::transition_handler::set_trigger_origin;
use crate::error::{AppError, AppResult};
use crate::infrastructure::agents::spawner::AgenticClientSpawner;
use crate::infrastructure::ClaudeCodeClient;

// ============================================================================
// No-op service implementations (for services not yet fully implemented)
// ============================================================================

/// EventEmitter - emits events to Tauri app handle when available
pub struct TauriEventEmitter<R: Runtime = tauri::Wry> {
    app_handle: Option<AppHandle<R>>,
}

impl<R: Runtime> TauriEventEmitter<R> {
    pub fn new(app_handle: Option<AppHandle<R>>) -> Self {
        Self { app_handle }
    }
}

#[async_trait]
impl<R: Runtime> EventEmitter for TauriEventEmitter<R> {
    async fn emit(&self, event_type: &str, task_id: &str) {
        if let Some(ref handle) = self.app_handle {
            let _ = handle.emit(
                event_type,
                serde_json::json!({
                    "taskId": task_id,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                }),
            );
        }
    }

    async fn emit_with_payload(&self, event_type: &str, task_id: &str, payload: &str) {
        if let Some(ref handle) = self.app_handle {
            let _ = handle.emit(
                event_type,
                serde_json::json!({
                    "taskId": task_id,
                    "payload": payload,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                }),
            );
        }
    }

    async fn emit_status_change(&self, task_id: &str, old_status: &str, new_status: &str) {
        if let Some(ref handle) = self.app_handle {
            let _ = handle.emit(
                "task:status_changed",
                serde_json::json!({
                    "task_id": task_id,
                    "old_status": old_status,
                    "new_status": new_status,
                }),
            );
        }
    }
}

/// LoggingNotifier - logs notifications for debugging
pub struct LoggingNotifier;

#[async_trait]
impl Notifier for LoggingNotifier {
    async fn notify(&self, notification_type: &str, task_id: &str) {
        tracing::info!(
            task_id = task_id,
            notification_type = notification_type,
            "Notification"
        );
    }

    async fn notify_with_message(&self, notification_type: &str, task_id: &str, message: &str) {
        tracing::info!(
            task_id = task_id,
            notification_type = notification_type,
            message = message,
            "Notification with message"
        );
    }
}

/// Repository-backed DependencyManager for automatic task blocking/unblocking
///
/// When a task completes (enters Approved state), this manager:
/// 1. Finds all tasks that were blocked by the completed task
/// 2. For each blocked task, checks if ALL its blockers are now complete
/// 3. If all blockers complete, transitions the task from Blocked to Ready
/// 4. Emits task:unblocked event for UI updates
pub struct RepoBackedDependencyManager<R: Runtime = tauri::Wry> {
    task_dep_repo: Arc<dyn TaskDependencyRepository>,
    task_repo: Arc<dyn TaskRepository>,
    app_handle: Option<AppHandle<R>>,
}

impl<R: Runtime> RepoBackedDependencyManager<R> {
    pub fn new(
        task_dep_repo: Arc<dyn TaskDependencyRepository>,
        task_repo: Arc<dyn TaskRepository>,
        app_handle: Option<AppHandle<R>>,
    ) -> Self {
        Self {
            task_dep_repo,
            task_repo,
            app_handle,
        }
    }

    /// Check if a blocking task is complete (no longer blocking dependents).
    /// Complete states: Merged, Cancelled, Stopped, MergeIncomplete.
    /// Note: Approved is NOT complete — task still needs to merge successfully.
    /// Paused is NOT complete — task may resume.
    /// Failed is NOT complete — dependents stay Blocked to prevent cascade
    /// execution against broken output. Users must manually unblock.
    async fn is_blocker_complete(&self, blocker_id: &TaskId) -> bool {
        if let Ok(Some(task)) = self.task_repo.get_by_id(blocker_id).await {
            matches!(
                task.internal_status,
                InternalStatus::Merged
                    | InternalStatus::Cancelled
                    | InternalStatus::Stopped
                    | InternalStatus::MergeIncomplete
            )
        } else {
            // If task doesn't exist, consider it "complete" (not blocking)
            true
        }
    }

    /// Get names of incomplete blockers for a task (for blocked_reason message).
    /// Returns (waiting_names, failed_names) so callers can produce specific messages.
    async fn get_incomplete_blocker_names(
        &self,
        task_id: &TaskId,
    ) -> (Vec<String>, Vec<String>) {
        let blockers = match self.task_dep_repo.get_blockers(task_id).await {
            Ok(b) => b,
            Err(_) => return (Vec::new(), Vec::new()),
        };

        let mut waiting_names = Vec::new();
        let mut failed_names = Vec::new();
        for blocker_id in blockers {
            if let Ok(Some(task)) = self.task_repo.get_by_id(&blocker_id).await {
                match task.internal_status {
                    InternalStatus::Merged
                    | InternalStatus::Cancelled
                    | InternalStatus::Stopped
                    | InternalStatus::MergeIncomplete => {
                        // complete — not included
                    }
                    InternalStatus::Failed => {
                        failed_names.push(task.title);
                    }
                    _ => {
                        waiting_names.push(task.title);
                    }
                }
            }
        }
        (waiting_names, failed_names)
    }
}

#[async_trait]
impl<R: Runtime> DependencyManager for RepoBackedDependencyManager<R> {
    async fn unblock_dependents(&self, completed_task_id: &str) {
        let task_id = TaskId::from_string(completed_task_id.to_string());

        // Find all tasks that depend on the completed task
        let dependents = match self.task_dep_repo.get_blocked_by(&task_id).await {
            Ok(deps) => deps,
            Err(e) => {
                tracing::error!(error = %e, task_id = completed_task_id, "Failed to get dependents");
                return;
            }
        };

        tracing::info!(
            completed_task_id = completed_task_id,
            dependent_count = dependents.len(),
            "Checking dependents for unblocking"
        );

        for dependent_id in dependents {
            // Get all blockers for this dependent task
            let blockers = match self.task_dep_repo.get_blockers(&dependent_id).await {
                Ok(b) => b,
                Err(_) => continue,
            };

            // Check if ALL blockers are now complete
            let mut all_complete = true;
            for blocker_id in &blockers {
                if !self.is_blocker_complete(blocker_id).await {
                    all_complete = false;
                    break;
                }
            }

            // Get the dependent task
            let mut dependent_task = match self.task_repo.get_by_id(&dependent_id).await {
                Ok(Some(t)) => t,
                _ => continue,
            };

            if all_complete {
                // All blockers complete — transition Blocked → Ready using a direct DB update
                // (not TransitionHandler) to avoid recursive re-entry: we are already inside
                // on_enter(Merged) → unblock_dependents(). TransitionHandler would call
                // on_enter(Ready) which could trigger try_schedule_ready_tasks() mid-loop and
                // interact with the scheduler before all dependents are processed.
                //
                // Scheduling guarantee: on_enter(Merged) calls try_schedule_ready_tasks() via
                // tokio::spawn (600ms delay) AFTER this unblock_dependents() call completes,
                // so every task set to Ready here is guaranteed to be picked up by the
                // scheduler. See on_enter_states.rs State::Merged branch.
                if dependent_task.internal_status == InternalStatus::Blocked {
                    dependent_task.internal_status = InternalStatus::Ready;
                    dependent_task.blocked_reason = None;
                    dependent_task.touch();

                    if let Err(e) = self.task_repo.update(&dependent_task).await {
                        tracing::error!(error = %e, task_id = %dependent_id, "Failed to unblock task");
                        continue;
                    }

                    // Record state transition history for timeline visibility
                    if let Err(e) = self
                        .task_repo
                        .persist_status_change(
                            &dependent_id,
                            InternalStatus::Blocked,
                            InternalStatus::Ready,
                            "blockers_resolved",
                        )
                        .await
                    {
                        tracing::warn!(error = %e, task_id = %dependent_id, "Failed to record unblock transition (non-fatal)");
                    }

                    tracing::info!(
                        task_id = %dependent_id,
                        task_title = %dependent_task.title,
                        "Task unblocked - all blockers complete"
                    );

                    // Emit task:unblocked event for UI update
                    if let Some(ref handle) = self.app_handle {
                        let _ = handle.emit(
                            "task:unblocked",
                            serde_json::json!({
                                "taskId": dependent_id.as_str(),
                                "taskTitle": dependent_task.title,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            }),
                        );
                    }
                }
            } else {
                // Some blockers still incomplete - update blocked_reason with remaining names
                let (waiting_names, failed_names) =
                    self.get_incomplete_blocker_names(&dependent_id).await;
                let new_reason = if !failed_names.is_empty() && waiting_names.is_empty() {
                    // All remaining blockers have failed
                    let names = failed_names
                        .iter()
                        .map(|n| format!("\"{}\"", n))
                        .collect::<Vec<_>>()
                        .join(", ");
                    if failed_names.len() == 1 {
                        format!("Dependency {} failed", names)
                    } else {
                        format!("Dependencies {} failed", names)
                    }
                } else if !failed_names.is_empty() {
                    // Mix of failed and still-running blockers
                    let failed = failed_names
                        .iter()
                        .map(|n| format!("\"{}\" (failed)", n))
                        .collect::<Vec<_>>();
                    let waiting = waiting_names
                        .iter()
                        .map(|n| format!("\"{}\"", n))
                        .collect::<Vec<_>>();
                    let all: Vec<String> = failed.into_iter().chain(waiting).collect();
                    format!("Waiting for: {}", all.join(", "))
                } else if !waiting_names.is_empty() {
                    format!("Waiting for: {}", waiting_names.join(", "))
                } else {
                    return;
                };
                if dependent_task.blocked_reason.as_ref() != Some(&new_reason) {
                    dependent_task.blocked_reason = Some(new_reason);
                    dependent_task.touch();
                    let _ = self.task_repo.update(&dependent_task).await;
                }
            }
        }
    }

    async fn has_unresolved_blockers(&self, task_id: &str) -> bool {
        let task_id = TaskId::from_string(task_id.to_string());
        let blockers = match self.task_dep_repo.get_blockers(&task_id).await {
            Ok(b) => b,
            Err(_) => return false,
        };

        for blocker_id in blockers {
            if !self.is_blocker_complete(&blocker_id).await {
                return true;
            }
        }
        false
    }

    async fn get_blocking_tasks(&self, task_id: &str) -> Vec<String> {
        let task_id = TaskId::from_string(task_id.to_string());
        match self.task_dep_repo.get_blockers(&task_id).await {
            Ok(blockers) => blockers
                .into_iter()
                .map(|id| id.as_str().to_string())
                .collect(),
            Err(_) => Vec::new(),
        }
    }
}

/// No-op ReviewStarter - placeholder until review system is wired
pub struct NoOpReviewStarter;

#[async_trait]
impl ReviewStarter for NoOpReviewStarter {
    async fn start_ai_review(&self, task_id: &str, _project_id: &str) -> ReviewStartResult {
        tracing::info!(task_id = task_id, "AI review would start here");
        // Return disabled for now - review system not fully wired
        ReviewStartResult::Disabled
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert InternalStatus to state machine State.
/// Used by execute_entry_actions and execute_exit_actions.
fn internal_status_to_state(
    status: InternalStatus,
) -> crate::domain::state_machine::machine::State {
    use crate::domain::state_machine::machine::State;
    match status {
        InternalStatus::Backlog => State::Backlog,
        InternalStatus::Ready => State::Ready,
        InternalStatus::Blocked => State::Blocked,
        InternalStatus::Executing => State::Executing,
        InternalStatus::QaRefining => State::QaRefining,
        InternalStatus::QaTesting => State::QaTesting,
        InternalStatus::QaPassed => State::QaPassed,
        InternalStatus::QaFailed => State::QaFailed(Default::default()),
        InternalStatus::PendingReview => State::PendingReview,
        InternalStatus::Reviewing => State::Reviewing,
        InternalStatus::ReviewPassed => State::ReviewPassed,
        InternalStatus::Escalated => State::Escalated,
        InternalStatus::RevisionNeeded => State::RevisionNeeded,
        InternalStatus::ReExecuting => State::ReExecuting,
        InternalStatus::Approved => State::Approved,
        InternalStatus::PendingMerge => State::PendingMerge,
        InternalStatus::Merging => State::Merging,
        InternalStatus::MergeIncomplete => State::MergeIncomplete,
        InternalStatus::MergeConflict => State::MergeConflict,
        InternalStatus::Merged => State::Merged,
        InternalStatus::Failed => State::Failed(Default::default()),
        InternalStatus::Cancelled => State::Cancelled,
        InternalStatus::Paused => State::Paused,
        InternalStatus::Stopped => State::Stopped,
    }
}

/// Convert state machine State to InternalStatus.
/// Used for persisting auto-transitions to the database.
fn state_to_internal_status(
    state: &crate::domain::state_machine::machine::State,
) -> InternalStatus {
    use crate::domain::state_machine::machine::State;
    match state {
        State::Backlog => InternalStatus::Backlog,
        State::Ready => InternalStatus::Ready,
        State::Blocked => InternalStatus::Blocked,
        State::Executing => InternalStatus::Executing,
        State::QaRefining => InternalStatus::QaRefining,
        State::QaTesting => InternalStatus::QaTesting,
        State::QaPassed => InternalStatus::QaPassed,
        State::QaFailed(_) => InternalStatus::QaFailed,
        State::PendingReview => InternalStatus::PendingReview,
        State::Reviewing => InternalStatus::Reviewing,
        State::ReviewPassed => InternalStatus::ReviewPassed,
        State::Escalated => InternalStatus::Escalated,
        State::RevisionNeeded => InternalStatus::RevisionNeeded,
        State::ReExecuting => InternalStatus::ReExecuting,
        State::Approved => InternalStatus::Approved,
        State::PendingMerge => InternalStatus::PendingMerge,
        State::Merging => InternalStatus::Merging,
        State::MergeIncomplete => InternalStatus::MergeIncomplete,
        State::MergeConflict => InternalStatus::MergeConflict,
        State::Merged => InternalStatus::Merged,
        State::Failed(_) => InternalStatus::Failed,
        State::Cancelled => InternalStatus::Cancelled,
        State::Paused => InternalStatus::Paused,
        State::Stopped => InternalStatus::Stopped,
    }
}

// ============================================================================
// TaskTransitionService
// ============================================================================

/// Service for orchestrating task status transitions with proper entry actions.
///
/// This service ensures that when a task's status changes (e.g., via Kanban drag-drop),
/// the appropriate side effects are triggered (e.g., spawning worker agents).
pub struct TaskTransitionService<R: Runtime = tauri::Wry> {
    task_repo: Arc<dyn TaskRepository>,
    project_repo: Arc<dyn ProjectRepository>,
    agent_spawner: Arc<dyn AgentSpawner>,
    event_emitter: Arc<dyn EventEmitter>,
    notifier: Arc<dyn Notifier>,
    dependency_manager: Arc<dyn DependencyManager>,
    review_starter: Arc<dyn ReviewStarter>,
    chat_service: Arc<dyn ChatService>,
    execution_state: Arc<ExecutionState>,
    _app_handle: Option<AppHandle<R>>,
    /// Task scheduler for auto-scheduling Ready tasks when slots are available.
    /// Passed to TaskServices so TransitionHandler can trigger scheduling on
    /// state exits and Ready state entry.
    task_scheduler: Option<Arc<dyn TaskScheduler>>,
    /// Plan branch repository for resolving feature branch targets.
    /// Passed to TaskServices so TransitionHandler can override merge targets.
    plan_branch_repo: Option<Arc<dyn PlanBranchRepository>>,

    /// Task step repository for updating step statuses.
    /// Passed to TaskServices so TransitionHandler can fail in-progress steps.
    step_repo: Option<Arc<dyn TaskStepRepository>>,

    /// Ideation session repository for fetching live session titles.
    /// Passed to TaskServices so TransitionHandler can build descriptive plan merge commit messages.
    ideation_session_repo: Option<Arc<dyn IdeationSessionRepository>>,

    /// Per-task team mode override. When `Some(true)`, the chat service uses
    /// team-mode agent names (e.g., orchestrator-execution instead of worker).
    /// `Some(false)` means solo was explicitly chosen (skip metadata fallback).
    /// `None` means unset — fall back to task metadata `agent_variant`.
    team_mode: Option<bool>,

    /// Shared tokio mutex for the concurrent merge guard critical section.
    /// Serializes the check-and-set in the worktree-mode merge guard so two tasks
    /// cannot both read "no blocker" simultaneously (eliminates TOCTOU race).
    /// Shared across all `execute_entry_actions` calls from this service instance.
    merge_lock: Arc<tokio::sync::Mutex<()>>,

    /// Shared set of task IDs currently undergoing `attempt_programmatic_merge`.
    /// Prevents double-click / duplicate reconciliation from spawning two concurrent
    /// merge attempts for the same task (self-dedup).
    merges_in_flight: Arc<std::sync::Mutex<HashSet<String>>>,
}

impl<R: Runtime> TaskTransitionService<R> {
    /// Create a new TaskTransitionService with all required dependencies.
    pub fn new(
        task_repo: Arc<dyn TaskRepository>,
        task_dep_repo: Arc<dyn TaskDependencyRepository>,
        project_repo: Arc<dyn ProjectRepository>,
        chat_message_repo: Arc<dyn ChatMessageRepository>,
        chat_attachment_repo: Arc<dyn ChatAttachmentRepository>,
        conversation_repo: Arc<dyn ChatConversationRepository>,
        agent_run_repo: Arc<dyn AgentRunRepository>,
        ideation_session_repo: Arc<dyn IdeationSessionRepository>,
        activity_event_repo: Arc<dyn ActivityEventRepository>,
        message_queue: Arc<MessageQueue>,
        running_agent_registry: Arc<dyn RunningAgentRegistry>,
        execution_state: Arc<ExecutionState>,
        app_handle: Option<AppHandle<R>>,
        memory_event_repo: Arc<dyn MemoryEventRepository>,
    ) -> Self {
        // Create the agent client for spawning
        let agent_client = Arc::new(ClaudeCodeClient::new());

        // Create the agent spawner with execution state for spawn gating
        // and task/project repos for per-task CWD resolution (worktree-aware)
        let agent_spawner: Arc<dyn AgentSpawner> = Arc::new(
            AgenticClientSpawner::new(agent_client)
                .with_repos(Arc::clone(&task_repo), Arc::clone(&project_repo))
                .with_execution_state(Arc::clone(&execution_state)),
        );

        // Create the unified chat service for worker spawning
        let chat_service: Arc<dyn ChatService> = {
            let mut service = ClaudeChatService::new(
                Arc::clone(&chat_message_repo),
                Arc::clone(&chat_attachment_repo),
                Arc::clone(&conversation_repo),
                Arc::clone(&agent_run_repo),
                Arc::clone(&project_repo),
                Arc::clone(&task_repo),
                Arc::clone(&task_dep_repo),
                Arc::clone(&ideation_session_repo),
                activity_event_repo,
                message_queue,
                running_agent_registry,
                memory_event_repo,
            )
            .with_execution_state(Arc::clone(&execution_state));
            if let Some(ref handle) = app_handle {
                service = service.with_app_handle(handle.clone());
            }
            // Global env var override: RALPHX_PROCESS_VARIANT_EXECUTION=team
            use crate::infrastructure::agents::claude::env_variant_override;
            if env_variant_override("execution").as_deref() == Some("team") {
                service = service.with_team_mode(true);
            }
            Arc::new(service)
        };

        // Create other services
        let event_emitter: Arc<dyn EventEmitter> =
            Arc::new(TauriEventEmitter::new(app_handle.clone()));
        let notifier: Arc<dyn Notifier> = Arc::new(LoggingNotifier);
        // Use real dependency manager for automatic blocking/unblocking based on dependency graph
        let dependency_manager: Arc<dyn DependencyManager> =
            Arc::new(RepoBackedDependencyManager::new(
                task_dep_repo,
                Arc::clone(&task_repo),
                app_handle.clone(),
            ));
        let review_starter: Arc<dyn ReviewStarter> = Arc::new(NoOpReviewStarter);

        Self {
            task_repo,
            project_repo,
            agent_spawner,
            event_emitter,
            notifier,
            dependency_manager,
            review_starter,
            chat_service,
            execution_state,
            _app_handle: app_handle,
            task_scheduler: None,
            plan_branch_repo: None,
            step_repo: None,
            ideation_session_repo: Some(ideation_session_repo),
            team_mode: None,
            merge_lock: Arc::new(tokio::sync::Mutex::new(())),
            merges_in_flight: Arc::new(std::sync::Mutex::new(HashSet::new())),
        }
    }

    /// Set the task scheduler for auto-scheduling Ready tasks (builder pattern).
    ///
    /// When set, the scheduler is passed to TaskServices so that TransitionHandler
    /// can trigger scheduling when tasks exit agent-active states or enter Ready state.
    pub fn with_task_scheduler(mut self, scheduler: Arc<dyn TaskScheduler>) -> Self {
        self.task_scheduler = Some(scheduler);
        self
    }

    /// Set the plan branch repository for feature branch resolution (builder pattern).
    pub fn with_plan_branch_repo(mut self, repo: Arc<dyn PlanBranchRepository>) -> Self {
        self.plan_branch_repo = Some(repo);
        self
    }

    /// Set the task step repository (builder pattern).
    pub fn with_step_repo(mut self, repo: Arc<dyn TaskStepRepository>) -> Self {
        self.step_repo = Some(repo);
        self
    }

    /// Enable team mode for agent spawning (builder pattern).
    ///
    /// When enabled, the chat service resolves to team-mode agent names
    /// (e.g., orchestrator-execution instead of worker).
    pub fn with_team_mode(mut self, team_mode: bool) -> Self {
        self.team_mode = Some(team_mode);
        self
    }

    /// Transition a task to a new status, triggering appropriate entry actions.
    ///
    /// This is a backward-compatible wrapper around transition_task_with_metadata
    /// that passes None for the metadata parameter.
    ///
    /// # Arguments
    /// * `task_id` - The ID of the task to transition
    /// * `new_status` - The target status
    ///
    /// # Returns
    /// * `Ok(Task)` - The updated task with new status
    /// * `Err(AppError)` - If the task is not found or transition is invalid
    pub async fn transition_task(
        &self,
        task_id: &TaskId,
        new_status: InternalStatus,
    ) -> AppResult<Task> {
        self.transition_task_with_metadata(task_id, new_status, None)
            .await
    }

    /// Transition a task to a new status with optional metadata update.
    ///
    /// This is the main entry point for status changes that should trigger side effects
    /// like spawning worker agents. Metadata updates are merged atomically with the
    /// status change.
    ///
    /// # Arguments
    /// * `task_id` - The ID of the task to transition
    /// * `new_status` - The target status
    /// * `metadata_update` - Optional metadata to merge into task.metadata
    ///
    /// # Returns
    /// * `Ok(Task)` - The updated task with new status and merged metadata
    /// * `Err(AppError)` - If the task is not found or transition is invalid
    pub async fn transition_task_with_metadata(
        &self,
        task_id: &TaskId,
        new_status: InternalStatus,
        metadata_update: Option<MetadataUpdate>,
    ) -> AppResult<Task> {
        tracing::debug!(
            task_id = task_id.as_str(),
            new_status = new_status.as_str(),
            "Starting task transition"
        );

        // 1. Fetch the task
        let mut task =
            self.task_repo.get_by_id(task_id).await?.ok_or_else(|| {
                AppError::NotFound(format!("Task not found: {}", task_id.as_str()))
            })?;

        let old_status = task.internal_status;
        tracing::debug!(
            old_status = old_status.as_str(),
            "Found task with current status"
        );

        // 2. If status is the same, no transition needed
        if old_status == new_status {
            tracing::debug!("Status unchanged, skipping transition");
            return Ok(task);
        }

        tracing::debug!(
            from = old_status.as_str(),
            to = new_status.as_str(),
            "Transitioning task status"
        );

        // 3. Update the task status
        task.internal_status = new_status;
        task.touch();

        // 3.1. Compute auto-metadata for QA transitions
        let auto_metadata = auto_metadata_for_status(new_status);

        // 3.2. Merge metadata updates (auto + explicit)
        if metadata_update.is_some() || auto_metadata.is_some() {
            // Prioritize explicit update, fallback to auto
            let final_update = metadata_update.or(auto_metadata);
            if let Some(update) = final_update {
                task.metadata = Some(update.merge_into(task.metadata.as_deref()));
            }
        }

        // 4. Persist the update and record history (so UI can see the change)
        self.task_repo.update(&task).await?;

        // 4.1 Record state transition history for time-travel feature
        if let Err(e) = self
            .task_repo
            .persist_status_change(task_id, old_status, new_status, "system")
            .await
        {
            tracing::warn!(error = %e, "Failed to record state history (non-fatal)");
        }
        tracing::debug!("Task status persisted to database");

        // 5. Emit event for UI update
        if let Some(ref handle) = self._app_handle {
            let _ = handle.emit(
                "task:event",
                serde_json::json!({
                    "type": "status_changed",
                    "taskId": task_id.as_str(),
                    "from": old_status.as_str(),
                    "to": new_status.as_str(),
                    "changedBy": "user",
                }),
            );
            tracing::debug!("Emitted task:event status_changed");
        }

        // 6. Execute exit actions for the old status (e.g., decrement running count)
        tracing::debug!(
            old_status = old_status.as_str(),
            "Executing exit actions for old status"
        );
        self.execute_exit_actions(task_id, &task, old_status, new_status)
            .await;

        // 7. Execute entry actions for the new status
        tracing::debug!(
            new_status = new_status.as_str(),
            "Executing entry actions for new status"
        );
        self.execute_entry_actions(task_id, &task, new_status).await;

        tracing::debug!("Task transition complete");

        Ok(task)
    }

    /// Transition a task to Stopped status with context capture for smart resume.
    ///
    /// This method is specifically for stopping tasks mid-execution. It captures
    /// the current status and optional reason in the task's metadata, enabling
    /// the "smart resume" feature to restore context when the task is restarted.
    ///
    /// # Arguments
    /// * `task_id` - The ID of the task to stop
    /// * `from_status` - The status the task was in when stopped (captured for resume)
    /// * `reason` - Optional reason for stopping (captured for resume)
    ///
    /// # Returns
    /// * `Ok(Task)` - The stopped task with stop metadata
    /// * `Err(AppError)` - If the task is not found or transition is invalid
    pub async fn transition_to_stopped_with_context(
        &self,
        task_id: &TaskId,
        from_status: InternalStatus,
        reason: Option<String>,
    ) -> AppResult<Task> {
        tracing::info!(
            task_id = task_id.as_str(),
            from_status = from_status.as_str(),
            reason = ?reason,
            "Stopping task with context capture"
        );

        // Build stop metadata
        let stop_metadata = build_stop_metadata(from_status, reason);

        // Transition to Stopped with metadata
        self.transition_task_with_metadata(task_id, InternalStatus::Stopped, Some(stop_metadata))
            .await
    }

    /// Execute entry actions for a given status, including auto-transitions.
    ///
    /// This method delegates to TransitionHandler::on_enter() to ensure we use
    /// the canonical entry action logic defined in the state machine module.
    /// It also handles auto-transitions (e.g., PendingReview → Reviewing).
    ///
    /// Public so that StartupJobRunner can re-trigger entry actions on app restart
    /// for tasks that were in agent-active states when the app shut down.
    pub async fn execute_entry_actions(
        &self,
        task_id: &TaskId,
        task: &Task,
        status: InternalStatus,
    ) {
        use crate::domain::state_machine::{
            context::{TaskContext, TaskServices},
            machine::TaskStateMachine,
            transition_handler::TransitionHandler,
        };

        let state = internal_status_to_state(status);

        // Per-task team_mode override: check builder flag OR task metadata.
        // Some(true/false) = explicitly set by caller → use directly, skip metadata.
        // None = unset → fall back to task metadata agent_variant.
        match self.team_mode {
            Some(explicit) => {
                self.chat_service.set_team_mode(explicit);
            }
            None => {
                // No explicit choice — fall back to task metadata.
                // Always set team_mode explicitly to prevent AtomicBool contamination
                // from previous tasks sharing the same Arc<ChatService>.
                let is_team = task.metadata.as_ref()
                    .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
                    .and_then(|meta| meta.get("agent_variant").and_then(|v| v.as_str()).map(|s| s == "team"))
                    .unwrap_or(false);
                self.chat_service.set_team_mode(is_team);
            }
        }

        // Build TaskServices from our services
        let mut services = TaskServices::new(
            Arc::clone(&self.agent_spawner),
            Arc::clone(&self.event_emitter),
            Arc::clone(&self.notifier),
            Arc::clone(&self.dependency_manager),
            Arc::clone(&self.review_starter),
            Arc::clone(&self.chat_service),
        )
        .with_execution_state(Arc::clone(&self.execution_state))
        .with_task_repo(Arc::clone(&self.task_repo))
        .with_project_repo(Arc::clone(&self.project_repo));

        // Pass app_handle for event emission (uses try_with_app_handle for generic R)
        if let Some(ref handle) = self._app_handle {
            services = services.try_with_app_handle(handle.clone());
        }

        // Pass task scheduler for auto-scheduling Ready tasks
        if let Some(ref scheduler) = self.task_scheduler {
            services = services.with_task_scheduler(Arc::clone(scheduler));
        }

        // Pass plan branch repository for feature branch resolution
        if let Some(ref plan_branch_repo) = self.plan_branch_repo {
            services = services.with_plan_branch_repo(Arc::clone(plan_branch_repo));
        }

        // Pass step repository for updating step statuses on task failure
        if let Some(ref step_repo) = self.step_repo {
            services = services.with_step_repo(Arc::clone(step_repo));
        }

        // Pass shared merge lock for TOCTOU-safe concurrent merge guard
        services = services.with_merge_lock(Arc::clone(&self.merge_lock));

        // Pass shared merges_in_flight set for self-dedup across concurrent calls
        services = services.with_merges_in_flight(Arc::clone(&self.merges_in_flight));

        // Pass ideation session repository for plan merge commit message generation
        if let Some(ref session_repo) = self.ideation_session_repo {
            services = services.with_ideation_session_repo(Arc::clone(session_repo));
        }

        // Create TaskContext
        let context = TaskContext::new(task_id.as_str(), task.project_id.as_str(), services);

        // Create state machine and handler
        let mut machine = TaskStateMachine::new(context);
        let handler = TransitionHandler::new(&mut machine);

        // Execute entry action via TransitionHandler
        tracing::debug!(?state, "Calling TransitionHandler::on_enter");
        if let Err(e) = handler.on_enter(&state).await {
            tracing::error!(error = %e, "on_enter failed");

            // If execution was blocked (e.g., git isolation failure), transition task to Failed
            if matches!(&e, AppError::ExecutionBlocked(_)) {
                tracing::warn!(
                    task_id = task_id.as_str(),
                    error = %e,
                    "ExecutionBlocked during on_enter — transitioning task to Failed"
                );
                if let Ok(Some(mut failed_task)) = self.task_repo.get_by_id(task_id).await {
                    let from_status = failed_task.internal_status;
                    failed_task.internal_status = InternalStatus::Failed;
                    failed_task.blocked_reason = Some(e.to_string());
                    failed_task.touch();
                    if let Err(update_err) = self.task_repo.update(&failed_task).await {
                        tracing::error!(error = %update_err, "Failed to persist Failed status after ExecutionBlocked");
                    } else {
                        // Record state history
                        let _ = self
                            .task_repo
                            .persist_status_change(
                                task_id,
                                from_status,
                                InternalStatus::Failed,
                                "system",
                            )
                            .await;
                        // Emit event for UI
                        if let Some(ref handle) = self._app_handle {
                            let _ = handle.emit(
                                "task:event",
                                serde_json::json!({
                                    "type": "status_changed",
                                    "taskId": task_id.as_str(),
                                    "from": from_status.as_str(),
                                    "to": "failed",
                                    "changedBy": "system",
                                    "reason": e.to_string(),
                                }),
                            );
                        }
                    }
                }
            }
        }
        tracing::debug!("TransitionHandler::on_enter complete");

        // Check for auto-transitions (e.g., PendingReview → Reviewing, RevisionNeeded → ReExecuting)
        // This is critical for states that should immediately transition to spawn an agent
        if let Some(auto_state) = handler.check_auto_transition(&state) {
            let auto_status = state_to_internal_status(&auto_state);
            tracing::info!(
                from = status.as_str(),
                to = auto_status.as_str(),
                "Auto-transition triggered"
            );

            // Execute on_exit for the intermediate state
            handler.on_exit(&state, &auto_state).await;

            // Persist the auto-transition to the database
            if let Ok(Some(mut updated_task)) = self.task_repo.get_by_id(task_id).await {
                let from_status = updated_task.internal_status;
                updated_task.internal_status = auto_status;

                // Set trigger_origin for RevisionNeeded → ReExecuting transition
                if from_status == InternalStatus::RevisionNeeded
                    && auto_status == InternalStatus::ReExecuting
                {
                    set_trigger_origin(&mut updated_task, "revision");
                }

                updated_task.touch();
                if let Err(e) = self.task_repo.update(&updated_task).await {
                    tracing::error!(error = %e, "Failed to persist auto-transition");
                }
                // Record auto-transition in history
                if let Err(e) = self
                    .task_repo
                    .persist_status_change(task_id, from_status, auto_status, "auto")
                    .await
                {
                    tracing::warn!(error = %e, "Failed to record auto-transition history (non-fatal)");
                }
            }

            // Emit task:event for auto-transition so UI updates in real time
            if let Some(ref handle) = self._app_handle {
                let _ = handle.emit(
                    "task:event",
                    serde_json::json!({
                        "type": "status_changed",
                        "taskId": task_id.as_str(),
                        "from": status.as_str(),
                        "to": auto_status.as_str(),
                        "changedBy": "auto",
                    }),
                );
                tracing::debug!("Emitted task:event for auto-transition");
            }

            // Execute on_enter for the auto-transition target state
            if let Err(e) = handler.on_enter(&auto_state).await {
                tracing::error!(error = %e, "on_enter failed for auto-transition state {:?}", auto_state);
            }
            tracing::debug!(?auto_state, "Auto-transition on_enter complete");
        }
    }

    /// Execute exit actions for a status transition.
    ///
    /// This method delegates to TransitionHandler::on_exit() to ensure we use
    /// the canonical exit action logic defined in the state machine module.
    /// This is critical for decrementing running count when tasks exit agent-active states.
    async fn execute_exit_actions(
        &self,
        task_id: &TaskId,
        task: &Task,
        from_status: InternalStatus,
        to_status: InternalStatus,
    ) {
        use crate::domain::state_machine::{
            context::{TaskContext, TaskServices},
            machine::TaskStateMachine,
            transition_handler::TransitionHandler,
        };

        let from_state = internal_status_to_state(from_status);
        let to_state = internal_status_to_state(to_status);

        // Build TaskServices from our services
        let mut services = TaskServices::new(
            Arc::clone(&self.agent_spawner),
            Arc::clone(&self.event_emitter),
            Arc::clone(&self.notifier),
            Arc::clone(&self.dependency_manager),
            Arc::clone(&self.review_starter),
            Arc::clone(&self.chat_service),
        )
        .with_execution_state(Arc::clone(&self.execution_state))
        .with_task_repo(Arc::clone(&self.task_repo))
        .with_project_repo(Arc::clone(&self.project_repo));

        // Pass app_handle for event emission (uses try_with_app_handle for generic R)
        if let Some(ref handle) = self._app_handle {
            services = services.try_with_app_handle(handle.clone());
        }

        // Pass task scheduler for auto-scheduling Ready tasks
        if let Some(ref scheduler) = self.task_scheduler {
            services = services.with_task_scheduler(Arc::clone(scheduler));
        }

        // Pass plan branch repository for feature branch resolution
        if let Some(ref plan_branch_repo) = self.plan_branch_repo {
            services = services.with_plan_branch_repo(Arc::clone(plan_branch_repo));
        }

        // Pass step repository for updating step statuses on task failure
        if let Some(ref step_repo) = self.step_repo {
            services = services.with_step_repo(Arc::clone(step_repo));
        }

        // Pass ideation session repository for plan merge commit message generation
        if let Some(ref session_repo) = self.ideation_session_repo {
            services = services.with_ideation_session_repo(Arc::clone(session_repo));
        }

        // Create TaskContext
        let context = TaskContext::new(task_id.as_str(), task.project_id.as_str(), services);

        // Create state machine and handler
        let mut machine = TaskStateMachine::new(context);
        let handler = TransitionHandler::new(&mut machine);

        // Execute exit action via TransitionHandler
        tracing::debug!(?from_state, ?to_state, "Calling TransitionHandler::on_exit");
        handler.on_exit(&from_state, &to_state).await;
        tracing::debug!("TransitionHandler::on_exit complete");
    }
}

/// Auto-compute metadata for specific status transitions.
///
/// Returns metadata updates that should be automatically applied when transitioning
/// to certain statuses (e.g., QaRefining/QaTesting get trigger_origin=qa).
///
/// # Arguments
/// * `status` - The target status being transitioned to
///
/// # Returns
/// * `Some(MetadataUpdate)` if auto-metadata applies to this status
/// * `None` if no auto-metadata for this status
fn auto_metadata_for_status(status: InternalStatus) -> Option<MetadataUpdate> {
    match status {
        InternalStatus::QaRefining | InternalStatus::QaTesting => {
            Some(build_trigger_origin_metadata("qa"))
        }
        _ => None,
    }
}

/// TaskStopper implementation — delegates to transition_task for graceful stop.
#[async_trait]
impl<R: Runtime> crate::application::TaskStopper for TaskTransitionService<R> {
    async fn transition_to_stopped(&self, task_id: &TaskId) -> AppResult<()> {
        self.transition_task(task_id, InternalStatus::Stopped)
            .await
            .map(|_| ())
    }

    async fn transition_to_stopped_with_context(
        &self,
        task_id: &TaskId,
        from_status: InternalStatus,
        reason: Option<String>,
    ) -> AppResult<()> {
        self.transition_to_stopped_with_context(task_id, from_status, reason)
            .await
            .map(|_| ())
    }
}

#[cfg(test)]
mod tests;
