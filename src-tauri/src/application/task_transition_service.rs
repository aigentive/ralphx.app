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

use std::sync::Arc;
use async_trait::async_trait;
use tauri::{AppHandle, Emitter, Runtime};

use crate::application::{ChatService, ClaudeChatService};
use crate::commands::ExecutionState;
use crate::domain::entities::{InternalStatus, Task, TaskId};
use crate::domain::repositories::{
    ActivityEventRepository, AgentRunRepository, ChatConversationRepository, ChatMessageRepository,
    IdeationSessionRepository, ProjectRepository, TaskRepository,
};
use crate::domain::services::{MessageQueue, RunningAgentRegistry};
use crate::domain::state_machine::services::{
    AgentSpawner, DependencyManager, EventEmitter, Notifier, ReviewStartResult, ReviewStarter,
    TaskScheduler,
};
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

/// No-op DependencyManager - placeholder until dependencies are fully wired
///
/// Note: Task dependency repositories exist (TaskDependencyRepository), but are not yet
/// integrated into the state machine. This stub allows the transition system to function
/// without blocking on dependency implementation. When ready to wire dependencies:
/// 1. Create RepoBackedDependencyManager struct with TaskDependencyRepository + TaskRepository
/// 2. Implement unblock_dependents to query dependencies and update dependent task states
/// 3. Implement has_unresolved_blockers to check for blocking tasks
/// 4. Wire into TaskTransitionService initialization
pub struct NoOpDependencyManager;

#[async_trait]
impl DependencyManager for NoOpDependencyManager {
    async fn unblock_dependents(&self, _task_id: &str) {
        // No-op: dependency resolution not yet wired to state machine
    }

    async fn has_unresolved_blockers(&self, _task_id: &str) -> bool {
        false
    }

    async fn get_blocking_tasks(&self, _task_id: &str) -> Vec<String> {
        Vec::new()
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
fn internal_status_to_state(status: InternalStatus) -> crate::domain::state_machine::machine::State {
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
        InternalStatus::Failed => State::Failed(Default::default()),
        InternalStatus::Cancelled => State::Cancelled,
    }
}

/// Convert state machine State to InternalStatus.
/// Used for persisting auto-transitions to the database.
fn state_to_internal_status(state: &crate::domain::state_machine::machine::State) -> InternalStatus {
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
        State::Failed(_) => InternalStatus::Failed,
        State::Cancelled => InternalStatus::Cancelled,
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
}

impl<R: Runtime> TaskTransitionService<R> {
    /// Create a new TaskTransitionService with all required dependencies.
    pub fn new(
        task_repo: Arc<dyn TaskRepository>,
        project_repo: Arc<dyn ProjectRepository>,
        chat_message_repo: Arc<dyn ChatMessageRepository>,
        conversation_repo: Arc<dyn ChatConversationRepository>,
        agent_run_repo: Arc<dyn AgentRunRepository>,
        ideation_session_repo: Arc<dyn IdeationSessionRepository>,
        activity_event_repo: Arc<dyn ActivityEventRepository>,
        message_queue: Arc<MessageQueue>,
        running_agent_registry: Arc<RunningAgentRegistry>,
        execution_state: Arc<ExecutionState>,
        app_handle: Option<AppHandle<R>>,
    ) -> Self {
        // Create the agent client for spawning
        let agent_client = Arc::new(ClaudeCodeClient::new());

        // Create the agent spawner with execution state for spawn gating
        let agent_spawner: Arc<dyn AgentSpawner> = Arc::new(
            AgenticClientSpawner::new(agent_client)
                .with_execution_state(Arc::clone(&execution_state)),
        );

        // Create the unified chat service for worker spawning
        let chat_service: Arc<dyn ChatService> = {
            let mut service = ClaudeChatService::new(
                Arc::clone(&chat_message_repo),
                Arc::clone(&conversation_repo),
                Arc::clone(&agent_run_repo),
                Arc::clone(&project_repo),
                Arc::clone(&task_repo),
                Arc::clone(&ideation_session_repo),
                activity_event_repo,
                message_queue,
                running_agent_registry,
            )
            .with_execution_state(Arc::clone(&execution_state));
            if let Some(ref handle) = app_handle {
                service = service.with_app_handle(handle.clone());
            }
            Arc::new(service)
        };

        // Create other services (no-ops for now)
        let event_emitter: Arc<dyn EventEmitter> = Arc::new(TauriEventEmitter::new(app_handle.clone()));
        let notifier: Arc<dyn Notifier> = Arc::new(LoggingNotifier);
        let dependency_manager: Arc<dyn DependencyManager> = Arc::new(NoOpDependencyManager);
        let review_starter: Arc<dyn ReviewStarter> = Arc::new(NoOpReviewStarter);

        Self {
            task_repo,
            agent_spawner,
            event_emitter,
            notifier,
            dependency_manager,
            review_starter,
            chat_service,
            execution_state,
            _app_handle: app_handle,
            task_scheduler: None,
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

    /// Transition a task to a new status, triggering appropriate entry actions.
    ///
    /// This is the main entry point for status changes that should trigger side effects
    /// like spawning worker agents.
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
        tracing::debug!(
            task_id = task_id.as_str(),
            new_status = new_status.as_str(),
            "Starting task transition"
        );

        // 1. Fetch the task
        let mut task = self
            .task_repo
            .get_by_id(task_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Task not found: {}", task_id.as_str())))?;

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

        // 4. Persist the update and record history (so UI can see the change)
        self.task_repo.update(&task).await?;

        // 4.1 Record state transition history for time-travel feature
        if let Err(e) = self.task_repo.persist_status_change(task_id, old_status, new_status, "system").await {
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
        self.execute_exit_actions(task_id, &task, old_status, new_status).await;

        // 7. Execute entry actions for the new status
        tracing::debug!(
            new_status = new_status.as_str(),
            "Executing entry actions for new status"
        );
        self.execute_entry_actions(task_id, &task, new_status).await;

        tracing::debug!("Task transition complete");

        Ok(task)
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

        // Build TaskServices from our services
        let mut services = TaskServices::new(
            Arc::clone(&self.agent_spawner),
            Arc::clone(&self.event_emitter),
            Arc::clone(&self.notifier),
            Arc::clone(&self.dependency_manager),
            Arc::clone(&self.review_starter),
            Arc::clone(&self.chat_service),
        )
        .with_execution_state(Arc::clone(&self.execution_state));

        // Pass app_handle for event emission (uses try_with_app_handle for generic R)
        if let Some(ref handle) = self._app_handle {
            services = services.try_with_app_handle(handle.clone());
        }

        // Pass task scheduler for auto-scheduling Ready tasks
        if let Some(ref scheduler) = self.task_scheduler {
            services = services.with_task_scheduler(Arc::clone(scheduler));
        }

        // Create TaskContext
        let context = TaskContext::new(
            task_id.as_str(),
            task.project_id.as_str(),
            services,
        );

        // Create state machine and handler
        let mut machine = TaskStateMachine::new(context);
        let handler = TransitionHandler::new(&mut machine);

        // Execute entry action via TransitionHandler
        eprintln!("[ENTRY_ACTION] Calling on_enter for state: {:?}", state);
        tracing::debug!(?state, "Calling TransitionHandler::on_enter");
        handler.on_enter(&state).await;
        eprintln!("[ENTRY_ACTION] on_enter complete");
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
                updated_task.touch();
                if let Err(e) = self.task_repo.update(&updated_task).await {
                    tracing::error!(error = %e, "Failed to persist auto-transition");
                }
                // Record auto-transition in history
                if let Err(e) = self.task_repo.persist_status_change(task_id, from_status, auto_status, "auto").await {
                    tracing::warn!(error = %e, "Failed to record auto-transition history (non-fatal)");
                }
            }

            // Execute on_enter for the auto-transition target state
            handler.on_enter(&auto_state).await;
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
        .with_execution_state(Arc::clone(&self.execution_state));

        // Pass app_handle for event emission (uses try_with_app_handle for generic R)
        if let Some(ref handle) = self._app_handle {
            services = services.try_with_app_handle(handle.clone());
        }

        // Pass task scheduler for auto-scheduling Ready tasks
        if let Some(ref scheduler) = self.task_scheduler {
            services = services.with_task_scheduler(Arc::clone(scheduler));
        }

        // Create TaskContext
        let context = TaskContext::new(
            task_id.as_str(),
            task.project_id.as_str(),
            services,
        );

        // Create state machine and handler
        let mut machine = TaskStateMachine::new(context);
        let handler = TransitionHandler::new(&mut machine);

        // Execute exit action via TransitionHandler
        tracing::debug!(?from_state, ?to_state, "Calling TransitionHandler::on_exit");
        handler.on_exit(&from_state, &to_state).await;
        tracing::debug!("TransitionHandler::on_exit complete");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tauri_event_emitter_creation() {
        let emitter: TauriEventEmitter<tauri::Wry> = TauriEventEmitter::new(None);
        assert!(emitter.app_handle.is_none());
    }

    #[test]
    fn test_logging_notifier() {
        let _notifier = LoggingNotifier;
        // Just verify it can be created
    }

    #[test]
    fn test_no_op_dependency_manager() {
        let _manager = NoOpDependencyManager;
        // Just verify it can be created
    }

    #[test]
    fn test_no_op_review_starter() {
        let _starter = NoOpReviewStarter;
        // Just verify it can be created
    }
}
