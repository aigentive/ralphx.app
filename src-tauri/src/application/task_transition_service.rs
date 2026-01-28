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
    AgentRunRepository, ChatConversationRepository, ChatMessageRepository,
    IdeationSessionRepository, ProjectRepository, TaskRepository,
};
use crate::domain::services::{MessageQueue, RunningAgentRegistry};
use crate::domain::state_machine::services::{
    AgentSpawner, DependencyManager, EventEmitter, Notifier, ReviewStartResult, ReviewStarter,
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
pub struct NoOpDependencyManager;

#[async_trait]
impl DependencyManager for NoOpDependencyManager {
    async fn unblock_dependents(&self, _task_id: &str) {
        // TODO: Implement when task dependencies are fully wired
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
        }
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
        println!(">>> transition_task: task_id={}, new_status={}", task_id.as_str(), new_status.as_str());

        // 1. Fetch the task
        let mut task = self
            .task_repo
            .get_by_id(task_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Task not found: {}", task_id.as_str())))?;

        let old_status = task.internal_status;
        println!(">>> transition_task: old_status={}", old_status.as_str());

        // 2. If status is the same, no transition needed
        if old_status == new_status {
            println!(">>> transition_task: status unchanged, returning early");
            return Ok(task);
        }

        println!(">>> transition_task: {} -> {}", old_status.as_str(), new_status.as_str());

        // 3. Update the task status
        task.internal_status = new_status;
        task.touch();

        // 4. Persist the update first (so UI can see the change)
        self.task_repo.update(&task).await?;
        println!(">>> transition_task: task updated in database");

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
            println!(">>> transition_task: emitted task:event status_changed");
        }

        // 6. Execute entry actions for the new status
        println!(">>> transition_task: calling execute_entry_actions for status {}", new_status.as_str());
        self.execute_entry_actions(task_id, &task, new_status).await;

        println!(">>> transition_task: entry actions complete");

        Ok(task)
    }

    /// Execute entry actions for a given status.
    ///
    /// This method delegates to TransitionHandler::on_enter() to ensure we use
    /// the canonical entry action logic defined in the state machine module.
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
            machine::{State, TaskStateMachine},
            transition_handler::TransitionHandler,
        };

        // Convert InternalStatus to State
        let state = match status {
            InternalStatus::Backlog => State::Backlog,
            InternalStatus::Ready => State::Ready,
            InternalStatus::Blocked => State::Blocked,
            InternalStatus::Executing => State::Executing,
            InternalStatus::QaRefining => State::QaRefining,
            InternalStatus::QaTesting => State::QaTesting,
            InternalStatus::QaPassed => State::QaPassed,
            InternalStatus::QaFailed => State::QaFailed(Default::default()),
            InternalStatus::PendingReview => State::PendingReview,
            InternalStatus::Reviewing => State::PendingReview, // TODO: Will be replaced with State::Reviewing in next task
            InternalStatus::ReviewPassed => State::PendingReview, // TODO: Will be replaced with State::ReviewPassed in next task
            InternalStatus::RevisionNeeded => State::RevisionNeeded,
            InternalStatus::ReExecuting => State::Executing, // TODO: Will be replaced with State::ReExecuting in next task
            InternalStatus::Approved => State::Approved,
            InternalStatus::Failed => State::Failed(Default::default()),
            InternalStatus::Cancelled => State::Cancelled,
        };

        // Build TaskServices from our services
        let services = TaskServices::new(
            Arc::clone(&self.agent_spawner),
            Arc::clone(&self.event_emitter),
            Arc::clone(&self.notifier),
            Arc::clone(&self.dependency_manager),
            Arc::clone(&self.review_starter),
            Arc::clone(&self.chat_service),
        )
        .with_execution_state(Arc::clone(&self.execution_state));

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
        println!(">>> execute_entry_actions: calling TransitionHandler::on_enter for {:?}", state);
        handler.on_enter(&state).await;
        println!(">>> execute_entry_actions: TransitionHandler::on_enter complete");
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
