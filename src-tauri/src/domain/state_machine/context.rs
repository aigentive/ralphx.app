// TaskServices container and TaskContext for state machine
// These provide the shared context needed during state transitions

use super::mocks::{MockAgentSpawner, MockDependencyManager, MockEventEmitter, MockNotifier, MockReviewStarter};
use super::services::{AgentSpawner, DependencyManager, EventEmitter, Notifier, ReviewStarter};
use super::types::Blocker;
use crate::application::ExecutionChatService;
use std::sync::Arc;

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

    /// Optional service for persistent worker execution (Phase 15B).
    /// When available, worker spawning uses this service to persist output to database.
    /// When None, falls back to agent_spawner.spawn() (backward compatibility).
    pub execution_chat_service: Option<Arc<dyn ExecutionChatService>>,
}

impl TaskServices {
    /// Creates a new TaskServices with the given service implementations
    pub fn new(
        agent_spawner: Arc<dyn AgentSpawner>,
        event_emitter: Arc<dyn EventEmitter>,
        notifier: Arc<dyn Notifier>,
        dependency_manager: Arc<dyn DependencyManager>,
        review_starter: Arc<dyn ReviewStarter>,
    ) -> Self {
        Self {
            agent_spawner,
            event_emitter,
            notifier,
            dependency_manager,
            review_starter,
            execution_chat_service: None,
        }
    }

    /// Creates a new TaskServices with all service implementations including ExecutionChatService
    pub fn new_with_execution_chat(
        agent_spawner: Arc<dyn AgentSpawner>,
        event_emitter: Arc<dyn EventEmitter>,
        notifier: Arc<dyn Notifier>,
        dependency_manager: Arc<dyn DependencyManager>,
        review_starter: Arc<dyn ReviewStarter>,
        execution_chat_service: Arc<dyn ExecutionChatService>,
    ) -> Self {
        Self {
            agent_spawner,
            event_emitter,
            notifier,
            dependency_manager,
            review_starter,
            execution_chat_service: Some(execution_chat_service),
        }
    }

    /// Sets the execution chat service (builder pattern)
    pub fn with_execution_chat_service(
        mut self,
        service: Arc<dyn ExecutionChatService>,
    ) -> Self {
        self.execution_chat_service = Some(service);
        self
    }

    /// Creates a TaskServices with all mock implementations for testing
    pub fn new_mock() -> Self {
        Self {
            agent_spawner: Arc::new(MockAgentSpawner::new()),
            event_emitter: Arc::new(MockEventEmitter::new()),
            notifier: Arc::new(MockNotifier::new()),
            dependency_manager: Arc::new(MockDependencyManager::new()),
            review_starter: Arc::new(MockReviewStarter::new()),
            execution_chat_service: None,
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
            .field(
                "execution_chat_service",
                &self.execution_chat_service.as_ref().map(|_| "<ExecutionChatService>"),
            )
            .finish()
    }
}

/// Context passed to the state machine during transitions.
///
/// Contains all the information needed to make transition decisions
/// and execute side effects.
#[derive(Debug)]
pub struct TaskContext {
    /// The ID of the task being processed
    pub task_id: String,

    /// The ID of the project the task belongs to
    pub project_id: String,

    /// Whether QA is enabled for this task
    pub qa_enabled: bool,

    /// Whether QA prep has completed
    pub qa_prep_complete: bool,

    /// Current blockers for this task
    pub blockers: Vec<Blocker>,

    /// Feedback from the last review, if any
    pub review_feedback: Option<String>,

    /// Error message if in Failed state
    pub error: Option<String>,

    /// Services for executing side effects
    pub services: TaskServices,
}

impl TaskContext {
    /// Creates a new TaskContext with the given task and project IDs
    pub fn new(
        task_id: impl Into<String>,
        project_id: impl Into<String>,
        services: TaskServices,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            project_id: project_id.into(),
            qa_enabled: false,
            qa_prep_complete: false,
            blockers: Vec::new(),
            review_feedback: None,
            error: None,
            services,
        }
    }

    /// Creates a TaskContext with mock services for testing
    pub fn new_test(task_id: impl Into<String>, project_id: impl Into<String>) -> Self {
        Self::new(task_id, project_id, TaskServices::new_mock())
    }

    /// Enables QA for this task
    pub fn with_qa_enabled(mut self) -> Self {
        self.qa_enabled = true;
        self
    }

    /// Sets the QA prep complete flag
    pub fn with_qa_prep_complete(mut self) -> Self {
        self.qa_prep_complete = true;
        self
    }

    /// Adds blockers to this context
    pub fn with_blockers(mut self, blockers: Vec<Blocker>) -> Self {
        self.blockers = blockers;
        self
    }

    /// Sets review feedback
    pub fn with_review_feedback(mut self, feedback: impl Into<String>) -> Self {
        self.review_feedback = Some(feedback.into());
        self
    }

    /// Sets the error message
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// Returns true if this task has any unresolved blockers
    pub fn has_unresolved_blockers(&self) -> bool {
        self.blockers.iter().any(|b| !b.resolved)
    }

    /// Returns the count of unresolved blockers
    pub fn unresolved_blocker_count(&self) -> usize {
        self.blockers.iter().filter(|b| !b.resolved).count()
    }

    /// Adds a blocker to this task
    pub fn add_blocker(&mut self, blocker: Blocker) {
        self.blockers.push(blocker);
    }

    /// Resolves a blocker by ID
    pub fn resolve_blocker(&mut self, blocker_id: &str) {
        for blocker in &mut self.blockers {
            if blocker.id == blocker_id {
                blocker.resolved = true;
            }
        }
    }

    /// Resolves all blockers
    pub fn resolve_all_blockers(&mut self) {
        for blocker in &mut self.blockers {
            blocker.resolved = true;
        }
    }

    /// Clears the error message
    pub fn clear_error(&mut self) {
        self.error = None;
    }

    /// Clears review feedback
    pub fn clear_review_feedback(&mut self) {
        self.review_feedback = None;
    }

    /// Returns true if the task can proceed to execution
    /// (no unresolved blockers and not in error state)
    pub fn can_execute(&self) -> bool {
        !self.has_unresolved_blockers() && self.error.is_none()
    }

    /// Returns true if QA should be run after execution
    pub fn should_run_qa(&self) -> bool {
        self.qa_enabled && self.qa_prep_complete
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================
    // TaskServices tests
    // ==================

    #[test]
    fn test_task_services_new_mock() {
        let services = TaskServices::new_mock();
        // Just verify it creates without error
        let debug_str = format!("{:?}", services);
        assert!(debug_str.contains("TaskServices"));
    }

    #[test]
    fn test_task_services_debug() {
        let services = TaskServices::new_mock();
        let debug_str = format!("{:?}", services);
        assert!(debug_str.contains("agent_spawner"));
        assert!(debug_str.contains("event_emitter"));
        assert!(debug_str.contains("notifier"));
        assert!(debug_str.contains("dependency_manager"));
        assert!(debug_str.contains("review_starter"));
        assert!(debug_str.contains("execution_chat_service"));
    }

    // ==================
    // TaskContext tests
    // ==================

    #[test]
    fn test_task_context_new() {
        let ctx = TaskContext::new_test("task-123", "project-456");
        assert_eq!(ctx.task_id, "task-123");
        assert_eq!(ctx.project_id, "project-456");
        assert!(!ctx.qa_enabled);
        assert!(!ctx.qa_prep_complete);
        assert!(ctx.blockers.is_empty());
        assert!(ctx.review_feedback.is_none());
        assert!(ctx.error.is_none());
    }

    #[test]
    fn test_task_context_with_qa_enabled() {
        let ctx = TaskContext::new_test("task-1", "proj-1").with_qa_enabled();
        assert!(ctx.qa_enabled);
    }

    #[test]
    fn test_task_context_with_qa_prep_complete() {
        let ctx = TaskContext::new_test("task-1", "proj-1").with_qa_prep_complete();
        assert!(ctx.qa_prep_complete);
    }

    #[test]
    fn test_task_context_with_blockers() {
        let blockers = vec![Blocker::new("task-2"), Blocker::new("task-3")];
        let ctx = TaskContext::new_test("task-1", "proj-1").with_blockers(blockers);
        assert_eq!(ctx.blockers.len(), 2);
    }

    #[test]
    fn test_task_context_with_review_feedback() {
        let ctx =
            TaskContext::new_test("task-1", "proj-1").with_review_feedback("Needs more tests");
        assert_eq!(ctx.review_feedback, Some("Needs more tests".to_string()));
    }

    #[test]
    fn test_task_context_with_error() {
        let ctx = TaskContext::new_test("task-1", "proj-1").with_error("Build failed");
        assert_eq!(ctx.error, Some("Build failed".to_string()));
    }

    #[test]
    fn test_task_context_has_unresolved_blockers_false_when_empty() {
        let ctx = TaskContext::new_test("task-1", "proj-1");
        assert!(!ctx.has_unresolved_blockers());
    }

    #[test]
    fn test_task_context_has_unresolved_blockers_true_when_present() {
        let blockers = vec![Blocker::new("task-2")];
        let ctx = TaskContext::new_test("task-1", "proj-1").with_blockers(blockers);
        assert!(ctx.has_unresolved_blockers());
    }

    #[test]
    fn test_task_context_has_unresolved_blockers_false_when_all_resolved() {
        let blockers = vec![Blocker::new("task-2").as_resolved()];
        let ctx = TaskContext::new_test("task-1", "proj-1").with_blockers(blockers);
        assert!(!ctx.has_unresolved_blockers());
    }

    #[test]
    fn test_task_context_unresolved_blocker_count() {
        let blockers = vec![
            Blocker::new("task-2"),
            Blocker::new("task-3").as_resolved(),
            Blocker::new("task-4"),
        ];
        let ctx = TaskContext::new_test("task-1", "proj-1").with_blockers(blockers);
        assert_eq!(ctx.unresolved_blocker_count(), 2);
    }

    #[test]
    fn test_task_context_add_blocker() {
        let mut ctx = TaskContext::new_test("task-1", "proj-1");
        assert_eq!(ctx.blockers.len(), 0);
        ctx.add_blocker(Blocker::new("task-2"));
        assert_eq!(ctx.blockers.len(), 1);
    }

    #[test]
    fn test_task_context_resolve_blocker() {
        let blockers = vec![Blocker::new("task-2"), Blocker::new("task-3")];
        let mut ctx = TaskContext::new_test("task-1", "proj-1").with_blockers(blockers);
        assert_eq!(ctx.unresolved_blocker_count(), 2);

        ctx.resolve_blocker("task-2");
        assert_eq!(ctx.unresolved_blocker_count(), 1);
        assert!(ctx.blockers[0].resolved);
    }

    #[test]
    fn test_task_context_resolve_all_blockers() {
        let blockers = vec![Blocker::new("task-2"), Blocker::new("task-3")];
        let mut ctx = TaskContext::new_test("task-1", "proj-1").with_blockers(blockers);
        assert_eq!(ctx.unresolved_blocker_count(), 2);

        ctx.resolve_all_blockers();
        assert_eq!(ctx.unresolved_blocker_count(), 0);
    }

    #[test]
    fn test_task_context_clear_error() {
        let mut ctx = TaskContext::new_test("task-1", "proj-1").with_error("Error");
        assert!(ctx.error.is_some());
        ctx.clear_error();
        assert!(ctx.error.is_none());
    }

    #[test]
    fn test_task_context_clear_review_feedback() {
        let mut ctx = TaskContext::new_test("task-1", "proj-1").with_review_feedback("Feedback");
        assert!(ctx.review_feedback.is_some());
        ctx.clear_review_feedback();
        assert!(ctx.review_feedback.is_none());
    }

    #[test]
    fn test_task_context_can_execute() {
        let ctx = TaskContext::new_test("task-1", "proj-1");
        assert!(ctx.can_execute());
    }

    #[test]
    fn test_task_context_can_execute_false_with_blockers() {
        let blockers = vec![Blocker::new("task-2")];
        let ctx = TaskContext::new_test("task-1", "proj-1").with_blockers(blockers);
        assert!(!ctx.can_execute());
    }

    #[test]
    fn test_task_context_can_execute_false_with_error() {
        let ctx = TaskContext::new_test("task-1", "proj-1").with_error("Error");
        assert!(!ctx.can_execute());
    }

    #[test]
    fn test_task_context_should_run_qa() {
        let ctx = TaskContext::new_test("task-1", "proj-1")
            .with_qa_enabled()
            .with_qa_prep_complete();
        assert!(ctx.should_run_qa());
    }

    #[test]
    fn test_task_context_should_run_qa_false_without_qa_enabled() {
        let ctx = TaskContext::new_test("task-1", "proj-1").with_qa_prep_complete();
        assert!(!ctx.should_run_qa());
    }

    #[test]
    fn test_task_context_should_run_qa_false_without_prep_complete() {
        let ctx = TaskContext::new_test("task-1", "proj-1").with_qa_enabled();
        assert!(!ctx.should_run_qa());
    }

    #[test]
    fn test_task_context_debug() {
        let ctx = TaskContext::new_test("task-1", "proj-1");
        let debug_str = format!("{:?}", ctx);
        assert!(debug_str.contains("TaskContext"));
        assert!(debug_str.contains("task-1"));
    }

    // ==================
    // Builder pattern chain tests
    // ==================

    #[test]
    fn test_task_context_builder_chain() {
        let blockers = vec![Blocker::new("dep-1")];
        let ctx = TaskContext::new_test("task-1", "proj-1")
            .with_qa_enabled()
            .with_qa_prep_complete()
            .with_blockers(blockers)
            .with_review_feedback("LGTM")
            .with_error("Test error");

        assert!(ctx.qa_enabled);
        assert!(ctx.qa_prep_complete);
        assert_eq!(ctx.blockers.len(), 1);
        assert_eq!(ctx.review_feedback, Some("LGTM".to_string()));
        assert_eq!(ctx.error, Some("Test error".to_string()));
    }
}
