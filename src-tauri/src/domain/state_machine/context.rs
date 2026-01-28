// TaskServices container and TaskContext for state machine
// These provide the shared context needed during state transitions

use super::mocks::{MockAgentSpawner, MockDependencyManager, MockEventEmitter, MockNotifier, MockReviewStarter};
use super::services::{AgentSpawner, DependencyManager, EventEmitter, Notifier, ReviewStarter};
use super::types::Blocker;
use crate::application::ChatService;
use crate::commands::ExecutionState;
use std::sync::Arc;
use std::any::Any;
use tauri::{AppHandle, Runtime, Wry};

/// Container for all services used by the state machine.
///
/// This is injected into the state machine and provides access to
/// external services like agent spawning, event emission, and notifications.
pub struct TaskServices {
    /// Service for spawning and managing AI agents
    pub agent_spawner: Arc<dyn AgentSpawner>,

    /// Service for emitting events to the frontend
    pub event_emitter: Arc<dyn EventEmitter>,

    /// Service for sending notifications to users
    pub notifier: Arc<dyn Notifier>,

    /// Service for managing task dependencies
    pub dependency_manager: Arc<dyn DependencyManager>,

    /// Service for starting reviews on tasks
    pub review_starter: Arc<dyn ReviewStarter>,

    /// Unified chat service for worker execution (handles TaskExecution context).
    /// Worker spawning uses this service to persist output to database.
    pub chat_service: Arc<dyn ChatService>,

    /// Global execution state for tracking running task count.
    /// Used by TransitionHandler to decrement running count when exiting agent-active states.
    pub execution_state: Option<Arc<ExecutionState>>,

    /// Tauri app handle for emitting events to frontend (optional).
    /// Used by TransitionHandler to emit execution:status_changed events.
    pub app_handle: Option<AppHandle<Wry>>,
}

impl TaskServices {
    /// Creates a new TaskServices with the given service implementations
    pub fn new(
        agent_spawner: Arc<dyn AgentSpawner>,
        event_emitter: Arc<dyn EventEmitter>,
        notifier: Arc<dyn Notifier>,
        dependency_manager: Arc<dyn DependencyManager>,
        review_starter: Arc<dyn ReviewStarter>,
        chat_service: Arc<dyn ChatService>,
    ) -> Self {
        Self {
            agent_spawner,
            event_emitter,
            notifier,
            dependency_manager,
            review_starter,
            chat_service,
            execution_state: None,
            app_handle: None,
        }
    }

    /// Set the execution state (builder pattern)
    pub fn with_execution_state(mut self, state: Arc<ExecutionState>) -> Self {
        self.execution_state = Some(state);
        self
    }

    /// Set the Tauri app handle for event emission (builder pattern)
    pub fn with_app_handle(mut self, handle: AppHandle<Wry>) -> Self {
        self.app_handle = Some(handle);
        self
    }

    /// Try to set the app handle from a generic Runtime type.
    /// Only sets the handle if R is Wry (the default Tauri runtime).
    /// Returns self for builder chaining.
    pub fn try_with_app_handle<R: Runtime + 'static>(mut self, handle: AppHandle<R>) -> Self {
        // Use type checking to only accept Wry handles
        let handle_any: Box<dyn Any> = Box::new(handle);
        if let Ok(wry_handle) = handle_any.downcast::<AppHandle<Wry>>() {
            self.app_handle = Some(*wry_handle);
        }
        self
    }

    /// Creates a TaskServices with all mock implementations for testing
    pub fn new_mock() -> Self {
        use crate::application::MockChatService;

        Self {
            agent_spawner: Arc::new(MockAgentSpawner::new()),
            event_emitter: Arc::new(MockEventEmitter::new()),
            notifier: Arc::new(MockNotifier::new()),
            dependency_manager: Arc::new(MockDependencyManager::new()),
            review_starter: Arc::new(MockReviewStarter::new()),
            chat_service: Arc::new(MockChatService::new()),
            execution_state: None,
            app_handle: None,
        }
    }
}

impl std::fmt::Debug for TaskServices {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskServices")
            .field("agent_spawner", &"<AgentSpawner>")
            .field("event_emitter", &"<EventEmitter>")
            .field("notifier", &"<Notifier>")
            .field("dependency_manager", &"<DependencyManager>")
            .field("review_starter", &"<ReviewStarter>")
            .field("chat_service", &"<ChatService>")
            .field("execution_state", &self.execution_state.as_ref().map(|_| "<ExecutionState>"))
            .field("app_handle", &self.app_handle.as_ref().map(|_| "<AppHandle>"))
            .finish()
    }
}

// ============================================================================
// TaskContext - Container for task-specific state machine context
// ============================================================================

/// Runtime context for a specific task's state machine.
///
/// Each task has its own TaskContext that holds:
/// - The task's ID and project ID for database operations
/// - Shared services for performing actions
/// - Task-specific state like blockers
pub struct TaskContext {
    /// ID of the task being processed
    pub task_id: String,

    /// ID of the project this task belongs to
    pub project_id: String,

    /// Shared services for state machine actions
    pub services: TaskServices,

    /// Current blockers preventing task progress
    pub blockers: Vec<Blocker>,

    /// Whether QA is enabled for this task's workflow
    pub qa_enabled: bool,

    /// Whether QA prep has completed (used to skip wait_for in QaRefining)
    pub qa_prep_complete: bool,

    /// Feedback from review (used when transitioning to RevisionNeeded)
    pub review_feedback: Option<String>,

    /// Error message from failed execution or QA
    pub error: Option<String>,
}

impl TaskContext {
    /// Creates a new TaskContext for a task
    pub fn new(task_id: &str, project_id: &str, services: TaskServices) -> Self {
        Self {
            task_id: task_id.to_string(),
            project_id: project_id.to_string(),
            services,
            blockers: Vec::new(),
            qa_enabled: false,
            qa_prep_complete: false,
            review_feedback: None,
            error: None,
        }
    }

    /// Creates a TaskContext with mock services for testing
    pub fn new_test(task_id: &str, project_id: &str) -> Self {
        Self::new(task_id, project_id, TaskServices::new_mock())
    }

    /// Enable QA for this context (builder pattern for tests)
    pub fn with_qa_enabled(mut self) -> Self {
        self.qa_enabled = true;
        self
    }

    /// Mark QA prep as complete (builder pattern for tests)
    pub fn with_qa_prep_complete(mut self) -> Self {
        self.qa_prep_complete = true;
        self
    }

    /// Add a blocker to this context
    pub fn add_blocker(&mut self, blocker: Blocker) {
        self.blockers.push(blocker);
    }

    /// Clear all blockers
    pub fn clear_blockers(&mut self) {
        self.blockers.clear();
    }

    /// Resolve all blockers (alias for clear_blockers)
    pub fn resolve_all_blockers(&mut self) {
        self.blockers.clear();
    }

    /// Check if there are any blockers
    pub fn has_blockers(&self) -> bool {
        !self.blockers.is_empty()
    }

    /// Clear the review feedback
    pub fn clear_review_feedback(&mut self) {
        self.review_feedback = None;
    }

    /// Clear the error message
    pub fn clear_error(&mut self) {
        self.error = None;
    }
}

impl std::fmt::Debug for TaskContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskContext")
            .field("task_id", &self.task_id)
            .field("project_id", &self.project_id)
            .field("services", &self.services)
            .field("blockers", &self.blockers)
            .field("qa_enabled", &self.qa_enabled)
            .field("qa_prep_complete", &self.qa_prep_complete)
            .field("review_feedback", &self.review_feedback)
            .field("error", &self.error)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_context_creation() {
        let ctx = TaskContext::new_test("task-1", "proj-1");
        assert_eq!(ctx.task_id, "task-1");
        assert_eq!(ctx.project_id, "proj-1");
        assert!(!ctx.has_blockers());
    }

    #[test]
    fn test_task_context_blockers() {
        let mut ctx = TaskContext::new_test("task-1", "proj-1");
        assert!(!ctx.has_blockers());

        ctx.add_blocker(Blocker::new("task-2"));
        assert!(ctx.has_blockers());
        assert_eq!(ctx.blockers.len(), 1);

        ctx.clear_blockers();
        assert!(!ctx.has_blockers());
    }

    #[test]
    fn test_task_services_mock() {
        let services = TaskServices::new_mock();
        // Just verify it creates without panicking
        let _ = format!("{:?}", services);
    }
}
