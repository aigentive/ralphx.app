// TaskServices container and TaskContext for state machine
// These provide the shared context needed during state transitions

use super::mocks::{
    MockAgentSpawner, MockDependencyManager, MockEventEmitter, MockNotifier, MockReviewStarter,
    MockTaskScheduler,
};
use super::services::{
    AgentSpawner, DependencyManager, EventEmitter, Notifier, ReviewStarter, TaskScheduler,
};
use super::types::Blocker;
use crate::application::ChatService;
use crate::commands::ExecutionState;
use crate::domain::repositories::{
    IdeationSessionRepository, PlanBranchRepository, ProjectRepository, TaskRepository,
    TaskStepRepository,
};
use std::any::Any;
use std::collections::HashSet;
use std::sync::Arc;
use dashmap::DashMap;
use tauri::{AppHandle, Runtime, Wry};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

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

    /// Task scheduler for auto-scheduling Ready tasks when slots are available.
    /// Used by TransitionHandler to trigger scheduling on slot free and on enter Ready.
    pub task_scheduler: Option<Arc<dyn TaskScheduler>>,

    /// Task repository for fetching and updating tasks during state transitions.
    /// Used by TransitionHandler to set task_branch and worktree_path on Executing entry.
    pub task_repo: Option<Arc<dyn TaskRepository>>,

    /// Project repository for fetching project settings during state transitions.
    /// Used by TransitionHandler to get git_mode and worktree_parent_directory.
    pub project_repo: Option<Arc<dyn ProjectRepository>>,

    /// Plan branch repository for resolving feature branch targets during state transitions.
    /// Used by TransitionHandler to check if a task belongs to a plan with a feature branch.
    pub plan_branch_repo: Option<Arc<dyn PlanBranchRepository>>,

    /// Task step repository for updating step statuses during state transitions.
    /// Used by TransitionHandler to fail in-progress steps when task fails.
    pub step_repo: Option<Arc<dyn TaskStepRepository>>,

    /// Application-level mutex for the concurrent merge guard critical section.
    /// Serializes the check-and-set in `try_programmatic_merge` so two tasks
    /// cannot both read "no blocker" and both proceed to merge simultaneously.
    /// Eliminates the TOCTOU race in the worktree-mode merge guard.
    pub merge_lock: Arc<Mutex<()>>,

    /// Set of task IDs that currently have an `attempt_programmatic_merge` call in flight.
    /// Prevents double-click / double-trigger from spawning two merge attempts for the same task.
    /// Uses std::sync::Mutex (not tokio) so Drop impls can clean up synchronously.
    pub merges_in_flight: Arc<std::sync::Mutex<HashSet<String>>>,

    /// Ideation session repository for fetching live session titles.
    /// Used by TransitionHandler to build descriptive plan merge commit messages.
    pub ideation_session_repo: Option<Arc<dyn IdeationSessionRepository>>,

    /// Task-keyed CancellationTokens for in-flight post-merge validations.
    /// Inserted in handle_outcome_success before validation, cancelled in
    /// pre_merge_cleanup when a new merge attempt starts for the same task.
    pub validation_tokens: Arc<DashMap<String, CancellationToken>>,
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
            task_scheduler: None,
            task_repo: None,
            project_repo: None,
            plan_branch_repo: None,
            step_repo: None,
            merge_lock: Arc::new(Mutex::new(())),
            merges_in_flight: Arc::new(std::sync::Mutex::new(HashSet::new())),
            ideation_session_repo: None,
            validation_tokens: Arc::new(DashMap::new()),
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

    /// Set the task scheduler (builder pattern)
    pub fn with_task_scheduler(mut self, scheduler: Arc<dyn TaskScheduler>) -> Self {
        self.task_scheduler = Some(scheduler);
        self
    }

    /// Set the task repository (builder pattern)
    pub fn with_task_repo(mut self, repo: Arc<dyn TaskRepository>) -> Self {
        self.task_repo = Some(repo);
        self
    }

    /// Set the project repository (builder pattern)
    pub fn with_project_repo(mut self, repo: Arc<dyn ProjectRepository>) -> Self {
        self.project_repo = Some(repo);
        self
    }

    /// Set the plan branch repository (builder pattern)
    pub fn with_plan_branch_repo(mut self, repo: Arc<dyn PlanBranchRepository>) -> Self {
        self.plan_branch_repo = Some(repo);
        self
    }

    /// Set the step repository (builder pattern)
    pub fn with_step_repo(mut self, repo: Arc<dyn TaskStepRepository>) -> Self {
        self.step_repo = Some(repo);
        self
    }

    /// Set a shared merge lock (builder pattern).
    /// Use this to share the same mutex across multiple TaskServices instances
    /// (e.g., when two tasks run concurrently in the same process).
    pub fn with_merge_lock(mut self, lock: Arc<Mutex<()>>) -> Self {
        self.merge_lock = lock;
        self
    }

    /// Set a shared merges_in_flight set (builder pattern).
    /// Use this to share the same dedup set across multiple TaskServices instances.
    pub fn with_merges_in_flight(
        mut self,
        set: Arc<std::sync::Mutex<HashSet<String>>>,
    ) -> Self {
        self.merges_in_flight = set;
        self
    }

    /// Set the ideation session repository (builder pattern)
    pub fn with_ideation_session_repo(mut self, repo: Arc<dyn IdeationSessionRepository>) -> Self {
        self.ideation_session_repo = Some(repo);
        self
    }

    /// Set shared validation tokens DashMap (builder pattern).
    /// Use this to share tokens across multiple TaskServices instances.
    pub fn with_validation_tokens(mut self, tokens: Arc<DashMap<String, CancellationToken>>) -> Self {
        self.validation_tokens = tokens;
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
            task_scheduler: Some(Arc::new(MockTaskScheduler::new())),
            task_repo: None,
            project_repo: None,
            plan_branch_repo: None,
            step_repo: None,
            merge_lock: Arc::new(Mutex::new(())),
            merges_in_flight: Arc::new(std::sync::Mutex::new(HashSet::new())),
            ideation_session_repo: None,
            validation_tokens: Arc::new(DashMap::new()),
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
            .field(
                "execution_state",
                &self.execution_state.as_ref().map(|_| "<ExecutionState>"),
            )
            .field(
                "app_handle",
                &self.app_handle.as_ref().map(|_| "<AppHandle>"),
            )
            .field(
                "task_scheduler",
                &self.task_scheduler.as_ref().map(|_| "<TaskScheduler>"),
            )
            .field(
                "task_repo",
                &self.task_repo.as_ref().map(|_| "<TaskRepository>"),
            )
            .field(
                "project_repo",
                &self.project_repo.as_ref().map(|_| "<ProjectRepository>"),
            )
            .field(
                "plan_branch_repo",
                &self
                    .plan_branch_repo
                    .as_ref()
                    .map(|_| "<PlanBranchRepository>"),
            )
            .field("merge_lock", &"<Mutex<()>>")
            .field("merges_in_flight", &"<Mutex<HashSet<String>>>")
            .field("validation_tokens", &format!("<DashMap len={}>", self.validation_tokens.len()))
            .field(
                "ideation_session_repo",
                &self
                    .ideation_session_repo
                    .as_ref()
                    .map(|_| "<IdeationSessionRepository>"),
            )
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
#[path = "context_tests.rs"]
mod tests;
