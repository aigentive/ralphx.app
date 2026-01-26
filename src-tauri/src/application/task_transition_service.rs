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

use crate::application::{ClaudeExecutionChatService, ExecutionChatService};
use crate::domain::entities::{InternalStatus, Task, TaskId};
use crate::domain::repositories::{
    AgentRunRepository, ChatConversationRepository, ChatMessageRepository, TaskRepository,
};
use crate::domain::services::ExecutionMessageQueue;
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
                }),
            );
        }
    }
}

/// No-op Notifier - logs notifications but doesn't send them yet
pub struct LoggingNotifier;

#[async_trait]
impl Notifier for LoggingNotifier {
    async fn notify(&self, notification_type: &str, task_id: &str) {
        tracing::info!(
            notification_type = notification_type,
            task_id = task_id,
            "Notification triggered"
        );
    }

    async fn notify_with_message(&self, notification_type: &str, task_id: &str, message: &str) {
        tracing::info!(
            notification_type = notification_type,
            task_id = task_id,
            message = message,
            "Notification triggered with message"
        );
    }
}

/// No-op DependencyManager - placeholder until full implementation
pub struct NoOpDependencyManager;

#[async_trait]
impl DependencyManager for NoOpDependencyManager {
    async fn unblock_dependents(&self, _completed_task_id: &str) {
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
    execution_chat_service: Arc<dyn ExecutionChatService>,
    _app_handle: Option<AppHandle<R>>,
}

impl<R: Runtime> TaskTransitionService<R> {
    /// Create a new TaskTransitionService with all required dependencies.
    pub fn new(
        task_repo: Arc<dyn TaskRepository>,
        chat_message_repo: Arc<dyn ChatMessageRepository>,
        conversation_repo: Arc<dyn ChatConversationRepository>,
        agent_run_repo: Arc<dyn AgentRunRepository>,
        message_queue: Arc<ExecutionMessageQueue>,
        app_handle: Option<AppHandle<R>>,
    ) -> Self {
        // Create the agent client for spawning
        let agent_client = Arc::new(ClaudeCodeClient::new());

        // Create the agent spawner
        let agent_spawner: Arc<dyn AgentSpawner> = Arc::new(AgenticClientSpawner::new(agent_client));

        // Create the execution chat service for worker spawning
        let execution_chat_service: Arc<dyn ExecutionChatService> = {
            let mut service = ClaudeExecutionChatService::new(
                Arc::clone(&chat_message_repo),
                Arc::clone(&conversation_repo),
                Arc::clone(&agent_run_repo),
                Arc::clone(&task_repo),
                message_queue,
            );
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
            execution_chat_service,
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
    /// This is where the magic happens - when entering certain states,
    /// we trigger side effects like spawning agents.
    async fn execute_entry_actions(
        &self,
        task_id: &TaskId,
        task: &Task,
        status: InternalStatus,
    ) {
        let task_id_str = task_id.as_str();
        let project_id_str = task.project_id.as_str();

        match status {
            // ===== READY =====
            // When task becomes ready, optionally spawn QA prep agent
            InternalStatus::Ready => {
                // Check if QA is enabled for this task
                // For now, we don't auto-spawn QA prep - this can be added later
                tracing::debug!(task_id = task_id_str, "Task is now Ready");
            }

            // ===== EXECUTING =====
            // This is the key one - spawn the worker agent!
            InternalStatus::Executing => {
                println!(">>> execute_entry_actions: EXECUTING - about to spawn worker");

                // Build prompt for worker
                let prompt = format!(
                    "Execute the task. Task ID: {}. Use get_task_context to understand what needs to be done.",
                    task_id_str
                );
                println!(">>> execute_entry_actions: prompt = {}", prompt);

                // Spawn worker with persistence
                println!(">>> execute_entry_actions: calling spawn_with_persistence...");
                match self.execution_chat_service.spawn_with_persistence(task_id, &prompt).await {
                    Ok(result) => {
                        println!(
                            ">>> execute_entry_actions: Worker spawned! conversation_id={}, agent_run_id={}",
                            result.conversation_id.as_str(),
                            result.agent_run_id
                        );
                    }
                    Err(e) => {
                        println!(">>> execute_entry_actions: FAILED to spawn worker: {}", e);
                        // Emit error event
                        self.event_emitter
                            .emit_with_payload("task:spawn_error", task_id_str, &e.to_string())
                            .await;
                    }
                }
            }

            // ===== QA REFINING =====
            // Spawn QA refiner agent
            InternalStatus::QaRefining => {
                tracing::info!(task_id = task_id_str, "Spawning QA refiner agent");
                self.agent_spawner.spawn("qa-refiner", task_id_str).await;
            }

            // ===== QA TESTING =====
            // Spawn QA tester agent
            InternalStatus::QaTesting => {
                tracing::info!(task_id = task_id_str, "Spawning QA tester agent");
                self.agent_spawner.spawn("qa-tester", task_id_str).await;
            }

            // ===== QA PASSED =====
            // Emit event, could auto-transition to PendingReview
            InternalStatus::QaPassed => {
                tracing::info!(task_id = task_id_str, "QA tests passed");
                self.event_emitter.emit("task:qa_passed", task_id_str).await;
            }

            // ===== QA FAILED =====
            // Emit event, notify user
            InternalStatus::QaFailed => {
                tracing::info!(task_id = task_id_str, "QA tests failed");
                self.event_emitter.emit("task:qa_failed", task_id_str).await;
                self.notifier
                    .notify_with_message("qa_failed", task_id_str, "QA tests failed. Review needed.")
                    .await;
            }

            // ===== PENDING REVIEW =====
            // Start AI review
            InternalStatus::PendingReview => {
                tracing::info!(task_id = task_id_str, "Starting AI review");
                let result = self.review_starter.start_ai_review(task_id_str, project_id_str).await;
                match result {
                    ReviewStartResult::Started { review_id } => {
                        tracing::info!(
                            task_id = task_id_str,
                            review_id = %review_id,
                            "AI review started"
                        );
                    }
                    ReviewStartResult::Disabled => {
                        tracing::debug!(task_id = task_id_str, "AI review disabled");
                    }
                    ReviewStartResult::Error(e) => {
                        tracing::error!(
                            task_id = task_id_str,
                            error = %e,
                            "Failed to start AI review"
                        );
                    }
                }
            }

            // ===== REVISION NEEDED =====
            // Could auto-transition back to Executing
            InternalStatus::RevisionNeeded => {
                tracing::info!(task_id = task_id_str, "Revision needed, task will re-execute");
                self.event_emitter.emit("task:revision_needed", task_id_str).await;
            }

            // ===== APPROVED =====
            // Task complete! Unblock dependents, emit event
            InternalStatus::Approved => {
                tracing::info!(task_id = task_id_str, "Task approved!");
                self.event_emitter.emit("task:completed", task_id_str).await;
                self.dependency_manager.unblock_dependents(task_id_str).await;
                self.notifier
                    .notify_with_message("task_completed", task_id_str, "Task completed and approved!")
                    .await;
            }

            // ===== FAILED =====
            // Permanent failure
            InternalStatus::Failed => {
                tracing::warn!(task_id = task_id_str, "Task failed permanently");
                self.event_emitter.emit("task:failed", task_id_str).await;
                self.notifier
                    .notify_with_message("task_failed", task_id_str, "Task failed after max retries.")
                    .await;
            }

            // ===== CANCELLED =====
            InternalStatus::Cancelled => {
                tracing::info!(task_id = task_id_str, "Task cancelled");
                self.event_emitter.emit("task:cancelled", task_id_str).await;
            }

            // ===== Other states (no entry actions) =====
            // Note: ExecutionDone is handled in the worker completion handler
            // which auto-transitions to PendingReview (QA disabled by default)
            InternalStatus::Backlog
            | InternalStatus::Blocked
            | InternalStatus::ExecutionDone => {
                tracing::debug!(
                    task_id = task_id_str,
                    status = status.as_str(),
                    "No entry action for this status"
                );
            }
        }
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
