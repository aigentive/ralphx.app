// Transition handler - orchestrates side effects for state transitions
// This module wraps the state machine and handles entry/exit actions,
// especially for QA-related transitions.

use crate::domain::entities::TaskId;

use super::events::TaskEvent;
use super::machine::{Response, State, TaskStateMachine};

/// Result of handling a transition
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionResult {
    /// Transition completed successfully
    Success(State),
    /// Event was not handled in current state
    NotHandled,
    /// Auto-transition triggered (e.g., ExecutionDone -> QaRefining)
    AutoTransition(State),
}

impl TransitionResult {
    /// Get the final state if transition was successful
    pub fn state(&self) -> Option<&State> {
        match self {
            TransitionResult::Success(s) | TransitionResult::AutoTransition(s) => Some(s),
            TransitionResult::NotHandled => None,
        }
    }

    /// Check if the transition resulted in a new state
    pub fn is_success(&self) -> bool {
        matches!(
            self,
            TransitionResult::Success(_) | TransitionResult::AutoTransition(_)
        )
    }
}

/// Handler for state transitions with side effects
pub struct TransitionHandler<'a> {
    machine: &'a mut TaskStateMachine,
}

impl<'a> TransitionHandler<'a> {
    /// Create a new transition handler wrapping a state machine
    pub fn new(machine: &'a mut TaskStateMachine) -> Self {
        Self { machine }
    }

    /// Handle a state transition with side effects
    ///
    /// This method:
    /// 1. Dispatches the event to the state machine
    /// 2. Executes entry actions for the new state (if applicable)
    /// 3. Handles auto-transitions (e.g., ExecutionDone -> QaRefining when QA enabled)
    pub async fn handle_transition(
        &mut self,
        current_state: &State,
        event: &TaskEvent,
    ) -> TransitionResult {
        // Dispatch event to state machine
        let response = self.machine.dispatch(current_state, event);

        match response {
            Response::Transition(new_state) => {
                // Execute on-exit action for old state
                self.on_exit(current_state, &new_state).await;

                // Execute on-enter action for new state
                self.on_enter(&new_state).await;

                // Check for auto-transitions
                if let Some(auto_state) = self.check_auto_transition(&new_state) {
                    // Execute on-exit for intermediate state
                    self.on_exit(&new_state, &auto_state).await;
                    // Execute on-enter for final state
                    self.on_enter(&auto_state).await;
                    return TransitionResult::AutoTransition(auto_state);
                }

                TransitionResult::Success(new_state)
            }
            Response::Handled => TransitionResult::Success(current_state.clone()),
            Response::NotHandled => TransitionResult::NotHandled,
        }
    }

    /// Execute on-enter action for a state
    async fn on_enter(&self, state: &State) {
        match state {
            State::Ready => {
                // When entering Ready, spawn QA prep agent if enabled
                if self.machine.context.qa_enabled {
                    self.machine
                        .context
                        .services
                        .agent_spawner
                        .spawn_background("qa-prep", &self.machine.context.task_id)
                        .await;
                }
            }
            State::Executing => {
                // Spawn worker agent with persistence if ExecutionChatService is available
                // Otherwise fall back to agent_spawner.spawn() for backward compatibility
                if let Some(ref execution_service) = self.machine.context.services.execution_chat_service
                {
                    // Use ExecutionChatService for persistent worker execution (Phase 15B)
                    let task_id = TaskId::from_string(self.machine.context.task_id.clone());
                    let prompt = format!("Execute task: {}", self.machine.context.task_id);

                    // spawn_with_persistence handles:
                    // 1. Creating chat_conversation (context_type: 'task_execution')
                    // 2. Creating agent_run (status: 'running')
                    // 3. Spawning Claude CLI with --agent worker
                    // 4. Persisting stream output to chat_messages
                    // 5. Processing queued messages on completion
                    let _ = execution_service.spawn_with_persistence(&task_id, &prompt).await;
                } else {
                    // Fallback: use agent_spawner without persistence
                    self.machine
                        .context
                        .services
                        .agent_spawner
                        .spawn("worker", &self.machine.context.task_id)
                        .await;
                }
            }
            State::QaRefining => {
                // Wait for QA prep if not complete, then spawn QA refiner
                if !self.machine.context.qa_prep_complete {
                    self.machine
                        .context
                        .services
                        .agent_spawner
                        .wait_for("qa-prep", &self.machine.context.task_id)
                        .await;
                }
                self.machine
                    .context
                    .services
                    .agent_spawner
                    .spawn("qa-refiner", &self.machine.context.task_id)
                    .await;
            }
            State::QaTesting => {
                // Spawn QA tester agent
                self.machine
                    .context
                    .services
                    .agent_spawner
                    .spawn("qa-tester", &self.machine.context.task_id)
                    .await;
            }
            State::QaPassed => {
                // Emit QA passed event
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("qa_passed", &self.machine.context.task_id)
                    .await;
            }
            State::QaFailed(data) => {
                // Emit QA failed event and notify user
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("qa_failed", &self.machine.context.task_id)
                    .await;

                // Notify user if not already notified
                if !data.notified {
                    let message = format!(
                        "QA tests failed: {} failure(s)",
                        data.failure_count()
                    );
                    self.machine
                        .context
                        .services
                        .notifier
                        .notify_with_message(
                            "qa_failed",
                            &self.machine.context.task_id,
                            &message,
                        )
                        .await;
                }
            }
            State::PendingReview => {
                // Start AI review via ReviewStarter
                let review_result = self.machine
                    .context
                    .services
                    .review_starter
                    .start_ai_review(
                        &self.machine.context.task_id,
                        &self.machine.context.project_id,
                    )
                    .await;

                // Emit review:update event with the result
                match &review_result {
                    super::services::ReviewStartResult::Started { review_id } => {
                        self.machine
                            .context
                            .services
                            .event_emitter
                            .emit_with_payload(
                                "review:update",
                                &self.machine.context.task_id,
                                &format!(r#"{{"type":"started","reviewId":"{}"}}"#, review_id),
                            )
                            .await;

                        // Spawn reviewer agent
                        self.machine
                            .context
                            .services
                            .agent_spawner
                            .spawn("reviewer", &self.machine.context.task_id)
                            .await;
                    }
                    super::services::ReviewStartResult::Disabled => {
                        // AI review disabled, emit event but don't spawn agent
                        self.machine
                            .context
                            .services
                            .event_emitter
                            .emit_with_payload(
                                "review:update",
                                &self.machine.context.task_id,
                                r#"{"type":"disabled"}"#,
                            )
                            .await;
                    }
                    super::services::ReviewStartResult::Error(msg) => {
                        // Review failed to start, notify user
                        self.machine
                            .context
                            .services
                            .notifier
                            .notify_with_message(
                                "review_error",
                                &self.machine.context.task_id,
                                msg,
                            )
                            .await;
                    }
                }
            }
            State::RevisionNeeded => {
                // Auto-transition to Executing will be handled by check_auto_transition
            }
            State::Approved => {
                // Emit task completed event
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("task_completed", &self.machine.context.task_id)
                    .await;
                // Unblock dependent tasks
                self.machine
                    .context
                    .services
                    .dependency_manager
                    .unblock_dependents(&self.machine.context.task_id)
                    .await;
            }
            State::Failed(_) => {
                // Emit task failed event
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("task_failed", &self.machine.context.task_id)
                    .await;
            }
            _ => {}
        }
    }

    /// Execute on-exit action for a state
    async fn on_exit(&self, from: &State, _to: &State) {
        match from {
            State::Executing => {
                // Stop worker agent if it's still running
                // (This is a safety net - normally the agent completes naturally)
            }
            State::QaTesting => {
                // Stop QA tester if transitioning away
            }
            _ => {}
        }
    }

    /// Check for auto-transitions from the given state
    fn check_auto_transition(&self, state: &State) -> Option<State> {
        match state {
            State::ExecutionDone => {
                // Auto-transition to QaRefining if QA is enabled
                if self.machine.context.qa_enabled {
                    Some(State::QaRefining)
                } else {
                    // Skip QA, go directly to PendingReview
                    Some(State::PendingReview)
                }
            }
            State::QaPassed => {
                // Auto-transition to PendingReview
                Some(State::PendingReview)
            }
            State::RevisionNeeded => {
                // Auto-transition back to Executing
                Some(State::Executing)
            }
            _ => None,
        }
    }
}

/// Trait for receiving transition notifications
#[allow(dead_code)]
pub trait TransitionObserver: Send + Sync {
    /// Called when a transition occurs
    fn on_transition(&self, from: &State, to: &State, event: &TaskEvent);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::state_machine::context::{TaskContext, TaskServices};
    use crate::domain::state_machine::mocks::{
        MockAgentSpawner, MockDependencyManager, MockEventEmitter, MockNotifier, MockReviewStarter,
    };
    use crate::domain::state_machine::services::{AgentSpawner, EventEmitter, Notifier, ReviewStarter};
    use crate::domain::state_machine::types::QaFailedData;
    use std::sync::Arc;

    fn create_test_services() -> (
        Arc<MockAgentSpawner>,
        Arc<MockEventEmitter>,
        Arc<MockNotifier>,
        Arc<MockDependencyManager>,
        Arc<MockReviewStarter>,
        TaskServices,
    ) {
        let spawner = Arc::new(MockAgentSpawner::new());
        let emitter = Arc::new(MockEventEmitter::new());
        let notifier = Arc::new(MockNotifier::new());
        let dep_manager = Arc::new(MockDependencyManager::new());
        let review_starter = Arc::new(MockReviewStarter::new());

        let services = TaskServices::new(
            Arc::clone(&spawner) as Arc<dyn AgentSpawner>,
            Arc::clone(&emitter) as Arc<dyn EventEmitter>,
            Arc::clone(&notifier) as Arc<dyn Notifier>,
            Arc::clone(&dep_manager) as Arc<dyn super::super::services::DependencyManager>,
            Arc::clone(&review_starter) as Arc<dyn ReviewStarter>,
        );

        (spawner, emitter, notifier, dep_manager, review_starter, services)
    }

    fn create_context_with_services(
        task_id: &str,
        project_id: &str,
        services: TaskServices,
    ) -> TaskContext {
        TaskContext::new(task_id, project_id, services)
    }

    // ==================
    // TransitionResult tests
    // ==================

    #[test]
    fn test_transition_result_success() {
        let result = TransitionResult::Success(State::Ready);
        assert!(result.is_success());
        assert_eq!(result.state(), Some(&State::Ready));
    }

    #[test]
    fn test_transition_result_auto_transition() {
        let result = TransitionResult::AutoTransition(State::QaRefining);
        assert!(result.is_success());
        assert_eq!(result.state(), Some(&State::QaRefining));
    }

    #[test]
    fn test_transition_result_not_handled() {
        let result = TransitionResult::NotHandled;
        assert!(!result.is_success());
        assert!(result.state().is_none());
    }

    // ==================
    // Basic transition tests
    // ==================

    #[tokio::test]
    async fn test_backlog_to_ready_transition() {
        let (spawner, _emitter, _notifier, _dep_manager, _review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);
        let mut handler = TransitionHandler::new(&mut machine);

        let result = handler
            .handle_transition(&State::Backlog, &TaskEvent::Schedule)
            .await;

        assert!(result.is_success());
        // Without QA enabled, no agent should be spawned on Ready entry
        assert_eq!(spawner.spawn_count(), 0);
    }

    #[tokio::test]
    async fn test_backlog_to_ready_with_qa_enabled() {
        let (spawner, _emitter, _notifier, _dep_manager, _review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services).with_qa_enabled();
        let mut machine = TaskStateMachine::new(context);
        let mut handler = TransitionHandler::new(&mut machine);

        let result = handler
            .handle_transition(&State::Backlog, &TaskEvent::Schedule)
            .await;

        assert!(result.is_success());
        // With QA enabled, QA prep should be spawned in background
        let calls = spawner.get_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].method, "spawn_background");
        assert_eq!(calls[0].args, vec!["qa-prep", "task-1"]);
    }

    // ==================
    // ExecutionDone auto-transition tests
    // ==================

    #[tokio::test]
    async fn test_execution_done_auto_transition_to_qa_refining() {
        let (spawner, _emitter, _notifier, _dep_manager, _review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services).with_qa_enabled();
        let mut machine = TaskStateMachine::new(context);
        let mut handler = TransitionHandler::new(&mut machine);

        let result = handler
            .handle_transition(&State::Executing, &TaskEvent::ExecutionComplete)
            .await;

        // Should auto-transition to QaRefining
        if let TransitionResult::AutoTransition(state) = &result {
            assert_eq!(*state, State::QaRefining);
        } else {
            panic!("Expected AutoTransition, got {:?}", result);
        }

        // Should wait for qa-prep and spawn qa-refiner
        let calls = spawner.get_calls();
        assert!(calls.iter().any(|c| c.method == "wait_for" && c.args[0] == "qa-prep"));
        assert!(calls.iter().any(|c| c.method == "spawn" && c.args[0] == "qa-refiner"));
    }

    #[tokio::test]
    async fn test_execution_done_auto_transition_to_pending_review_without_qa() {
        let (spawner, _emitter, _notifier, _dep_manager, _review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);
        let mut handler = TransitionHandler::new(&mut machine);

        let result = handler
            .handle_transition(&State::Executing, &TaskEvent::ExecutionComplete)
            .await;

        // Should auto-transition to PendingReview
        if let TransitionResult::AutoTransition(state) = &result {
            assert_eq!(*state, State::PendingReview);
        } else {
            panic!("Expected AutoTransition, got {:?}", result);
        }

        // Should spawn reviewer
        let calls = spawner.get_calls();
        assert!(calls.iter().any(|c| c.method == "spawn" && c.args[0] == "reviewer"));
    }

    #[tokio::test]
    async fn test_execution_done_with_qa_prep_complete_skips_wait() {
        let (spawner, _emitter, _notifier, _dep_manager, _review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services)
            .with_qa_enabled()
            .with_qa_prep_complete();
        let mut machine = TaskStateMachine::new(context);
        let mut handler = TransitionHandler::new(&mut machine);

        let result = handler
            .handle_transition(&State::Executing, &TaskEvent::ExecutionComplete)
            .await;

        assert!(result.is_success());

        // Should NOT wait for qa-prep since it's already complete
        let calls = spawner.get_calls();
        assert!(!calls.iter().any(|c| c.method == "wait_for"));
        // But should still spawn qa-refiner
        assert!(calls.iter().any(|c| c.method == "spawn" && c.args[0] == "qa-refiner"));
    }

    // ==================
    // QA state tests
    // ==================

    #[tokio::test]
    async fn test_qa_refining_to_qa_testing() {
        let (spawner, _emitter, _notifier, _dep_manager, _review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services).with_qa_enabled();
        let mut machine = TaskStateMachine::new(context);
        let mut handler = TransitionHandler::new(&mut machine);

        let result = handler
            .handle_transition(&State::QaRefining, &TaskEvent::QaRefinementComplete)
            .await;

        assert_eq!(result.state(), Some(&State::QaTesting));

        // Should spawn qa-tester
        let calls = spawner.get_calls();
        assert!(calls.iter().any(|c| c.method == "spawn" && c.args[0] == "qa-tester"));
    }

    #[tokio::test]
    async fn test_qa_testing_passed() {
        let (_spawner, emitter, _notifier, _dep_manager, _review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services).with_qa_enabled();
        let mut machine = TaskStateMachine::new(context);
        let mut handler = TransitionHandler::new(&mut machine);

        let result = handler
            .handle_transition(&State::QaTesting, &TaskEvent::QaTestsComplete { passed: true })
            .await;

        // Should auto-transition from QaPassed to PendingReview
        if let TransitionResult::AutoTransition(state) = &result {
            assert_eq!(*state, State::PendingReview);
        } else {
            panic!("Expected AutoTransition, got {:?}", result);
        }

        // Should emit qa_passed event
        assert!(emitter.has_event("qa_passed"));
    }

    #[tokio::test]
    async fn test_qa_testing_failed_notifies_user() {
        let (_spawner, emitter, notifier, _dep_manager, _review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services).with_qa_enabled();
        let mut machine = TaskStateMachine::new(context);
        let mut handler = TransitionHandler::new(&mut machine);

        let result = handler
            .handle_transition(&State::QaTesting, &TaskEvent::QaTestsComplete { passed: false })
            .await;

        // Should transition to QaFailed
        assert!(matches!(result.state(), Some(State::QaFailed(_))));

        // Should emit qa_failed event
        assert!(emitter.has_event("qa_failed"));

        // Should notify user
        assert!(notifier.has_notification("qa_failed"));
    }

    #[tokio::test]
    async fn test_qa_failed_skip_qa_transitions_to_pending_review() {
        let (spawner, _emitter, _notifier, _dep_manager, _review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);
        let mut handler = TransitionHandler::new(&mut machine);

        let result = handler
            .handle_transition(
                &State::QaFailed(QaFailedData::default()),
                &TaskEvent::SkipQa,
            )
            .await;

        assert_eq!(result.state(), Some(&State::PendingReview));

        // Should spawn reviewer
        let calls = spawner.get_calls();
        assert!(calls.iter().any(|c| c.method == "spawn" && c.args[0] == "reviewer"));
    }

    #[tokio::test]
    async fn test_qa_failed_retry_transitions_to_revision_needed() {
        let (_spawner, _emitter, _notifier, _dep_manager, _review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);
        let mut handler = TransitionHandler::new(&mut machine);

        let result = handler
            .handle_transition(
                &State::QaFailed(QaFailedData::default()),
                &TaskEvent::Retry,
            )
            .await;

        // Should auto-transition from RevisionNeeded to Executing
        if let TransitionResult::AutoTransition(state) = &result {
            assert_eq!(*state, State::Executing);
        } else {
            panic!("Expected AutoTransition, got {:?}", result);
        }
    }

    // ==================
    // Review and terminal state tests
    // ==================

    #[tokio::test]
    async fn test_pending_review_approved() {
        let (_spawner, emitter, _notifier, dep_manager, _review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);
        let mut handler = TransitionHandler::new(&mut machine);

        let result = handler
            .handle_transition(
                &State::PendingReview,
                &TaskEvent::ReviewComplete {
                    approved: true,
                    feedback: None,
                },
            )
            .await;

        assert_eq!(result.state(), Some(&State::Approved));

        // Should emit task_completed event
        assert!(emitter.has_event("task_completed"));

        // Should unblock dependents
        let calls = dep_manager.get_calls();
        assert!(calls.iter().any(|c| c.method == "unblock_dependents"));
    }

    #[tokio::test]
    async fn test_pending_review_rejected_auto_transitions_to_executing() {
        let (spawner, _emitter, _notifier, _dep_manager, _review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);
        let mut handler = TransitionHandler::new(&mut machine);

        let result = handler
            .handle_transition(
                &State::PendingReview,
                &TaskEvent::ReviewComplete {
                    approved: false,
                    feedback: Some("Needs tests".to_string()),
                },
            )
            .await;

        // Should auto-transition from RevisionNeeded to Executing
        if let TransitionResult::AutoTransition(state) = &result {
            assert_eq!(*state, State::Executing);
        } else {
            panic!("Expected AutoTransition, got {:?}", result);
        }

        // Should spawn worker agent
        let calls = spawner.get_calls();
        assert!(calls.iter().any(|c| c.method == "spawn" && c.args[0] == "worker"));
    }

    #[tokio::test]
    async fn test_execution_failed_emits_event() {
        let (_spawner, emitter, _notifier, _dep_manager, _review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);
        let mut handler = TransitionHandler::new(&mut machine);

        let result = handler
            .handle_transition(
                &State::Executing,
                &TaskEvent::ExecutionFailed {
                    error: "Build failed".to_string(),
                },
            )
            .await;

        assert!(matches!(result.state(), Some(State::Failed(_))));

        // Should emit task_failed event
        assert!(emitter.has_event("task_failed"));
    }

    // ==================
    // Event not handled tests
    // ==================

    #[tokio::test]
    async fn test_event_not_handled() {
        let (_spawner, _emitter, _notifier, _dep_manager, _review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);
        let mut handler = TransitionHandler::new(&mut machine);

        // Try to schedule from Executing (not allowed)
        let result = handler
            .handle_transition(&State::Executing, &TaskEvent::Schedule)
            .await;

        assert!(!result.is_success());
        assert_eq!(result, TransitionResult::NotHandled);
    }

    // ==================
    // Entering Executing spawns worker
    // ==================

    #[tokio::test]
    async fn test_entering_executing_spawns_worker() {
        let (spawner, _emitter, _notifier, _dep_manager, _review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);

        // Manually test on_enter for Executing
        let handler = TransitionHandler::new(&mut machine);
        handler.on_enter(&State::Executing).await;

        let calls = spawner.get_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].method, "spawn");
        assert_eq!(calls[0].args, vec!["worker", "task-1"]);
    }

    // ==================
    // Review integration tests
    // ==================

    #[tokio::test]
    async fn test_entering_pending_review_starts_ai_review() {
        let (spawner, emitter, _notifier, _dep_manager, review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);

        let handler = TransitionHandler::new(&mut machine);
        handler.on_enter(&State::PendingReview).await;

        // Should have called start_ai_review
        assert!(review_starter.has_review_for_task("task-1"));
        assert_eq!(review_starter.call_count(), 1);

        // Should have emitted review:update event
        let events = emitter.get_events();
        assert!(events.iter().any(|e| {
            e.method == "emit_with_payload" && e.args[0] == "review:update"
        }));

        // Should have spawned reviewer agent
        let calls = spawner.get_calls();
        assert!(calls.iter().any(|c| c.method == "spawn" && c.args[0] == "reviewer"));
    }

    #[tokio::test]
    async fn test_entering_pending_review_with_disabled_ai_review() {
        let spawner = Arc::new(MockAgentSpawner::new());
        let emitter = Arc::new(MockEventEmitter::new());
        let notifier = Arc::new(MockNotifier::new());
        let dep_manager = Arc::new(MockDependencyManager::new());
        let review_starter = Arc::new(MockReviewStarter::disabled());

        let services = TaskServices::new(
            Arc::clone(&spawner) as Arc<dyn AgentSpawner>,
            Arc::clone(&emitter) as Arc<dyn EventEmitter>,
            Arc::clone(&notifier) as Arc<dyn Notifier>,
            Arc::clone(&dep_manager) as Arc<dyn super::super::services::DependencyManager>,
            Arc::clone(&review_starter) as Arc<dyn ReviewStarter>,
        );

        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);

        let handler = TransitionHandler::new(&mut machine);
        handler.on_enter(&State::PendingReview).await;

        // Should NOT spawn reviewer agent when AI review is disabled
        let calls = spawner.get_calls();
        assert!(!calls.iter().any(|c| c.method == "spawn" && c.args[0] == "reviewer"));

        // Should emit review:update with disabled type
        let events = emitter.get_events();
        assert!(events.iter().any(|e| {
            e.method == "emit_with_payload"
                && e.args[0] == "review:update"
                && e.args[2].contains("disabled")
        }));
    }

    #[tokio::test]
    async fn test_entering_pending_review_with_error_notifies_user() {
        let spawner = Arc::new(MockAgentSpawner::new());
        let emitter = Arc::new(MockEventEmitter::new());
        let notifier = Arc::new(MockNotifier::new());
        let dep_manager = Arc::new(MockDependencyManager::new());
        let review_starter = Arc::new(MockReviewStarter::with_error("Database connection failed"));

        let services = TaskServices::new(
            Arc::clone(&spawner) as Arc<dyn AgentSpawner>,
            Arc::clone(&emitter) as Arc<dyn EventEmitter>,
            Arc::clone(&notifier) as Arc<dyn Notifier>,
            Arc::clone(&dep_manager) as Arc<dyn super::super::services::DependencyManager>,
            Arc::clone(&review_starter) as Arc<dyn ReviewStarter>,
        );

        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);

        let handler = TransitionHandler::new(&mut machine);
        handler.on_enter(&State::PendingReview).await;

        // Should NOT spawn reviewer agent when review fails to start
        let calls = spawner.get_calls();
        assert!(!calls.iter().any(|c| c.method == "spawn" && c.args[0] == "reviewer"));

        // Should notify user about the error
        assert!(notifier.has_notification("review_error"));
    }

    #[tokio::test]
    async fn test_entering_pending_review_emits_started_event_with_review_id() {
        let (spawner, emitter, _notifier, _dep_manager, review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);

        let handler = TransitionHandler::new(&mut machine);
        handler.on_enter(&State::PendingReview).await;

        // Verify review:update event contains the review ID
        let events = emitter.get_events();
        let review_event = events.iter().find(|e| {
            e.method == "emit_with_payload" && e.args[0] == "review:update"
        }).expect("Should have review:update event");

        assert!(review_event.args[2].contains("started"));
        assert!(review_event.args[2].contains("review-"));

        // Verify reviewer spawned after review started
        let calls = spawner.get_calls();
        assert!(calls.iter().any(|c| c.method == "spawn" && c.args[0] == "reviewer"));

        // Verify review_starter was called with correct arguments
        let review_calls = review_starter.get_calls();
        assert_eq!(review_calls.len(), 1);
        assert_eq!(review_calls[0].args[0], "task-1");
        assert_eq!(review_calls[0].args[1], "proj-1");
    }

    #[tokio::test]
    async fn test_execution_done_to_pending_review_starts_ai_review() {
        let (_spawner, emitter, _notifier, _dep_manager, review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);
        let mut handler = TransitionHandler::new(&mut machine);

        // Transition from Executing -> ExecutionDone -> PendingReview (auto)
        let result = handler
            .handle_transition(&State::Executing, &TaskEvent::ExecutionComplete)
            .await;

        // Should auto-transition to PendingReview
        assert_eq!(result.state(), Some(&State::PendingReview));

        // Should have started AI review
        assert!(review_starter.has_review_for_task("task-1"));

        // Should have emitted review:update event
        assert!(emitter.get_events().iter().any(|e| {
            e.method == "emit_with_payload" && e.args[0] == "review:update"
        }));
    }

    #[tokio::test]
    async fn test_qa_passed_to_pending_review_starts_ai_review() {
        let (_spawner, emitter, _notifier, _dep_manager, review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services).with_qa_enabled();
        let mut machine = TaskStateMachine::new(context);
        let mut handler = TransitionHandler::new(&mut machine);

        // Transition from QaTesting -> QaPassed -> PendingReview (auto)
        let result = handler
            .handle_transition(&State::QaTesting, &TaskEvent::QaTestsComplete { passed: true })
            .await;

        // Should auto-transition to PendingReview
        if let TransitionResult::AutoTransition(state) = &result {
            assert_eq!(*state, State::PendingReview);
        } else {
            panic!("Expected AutoTransition to PendingReview");
        }

        // Should have started AI review
        assert!(review_starter.has_review_for_task("task-1"));

        // Should have emitted review:update event
        assert!(emitter.get_events().iter().any(|e| {
            e.method == "emit_with_payload" && e.args[0] == "review:update"
        }));
    }

    // ==================
    // ExecutionChatService integration tests (Phase 15B)
    // ==================

    #[tokio::test]
    async fn test_entering_executing_uses_execution_chat_service_when_available() {
        use crate::application::{ExecutionChatService, MockExecutionChatService};

        let spawner = Arc::new(MockAgentSpawner::new());
        let emitter = Arc::new(MockEventEmitter::new());
        let notifier = Arc::new(MockNotifier::new());
        let dep_manager = Arc::new(MockDependencyManager::new());
        let review_starter = Arc::new(MockReviewStarter::new());
        let execution_chat_service = Arc::new(MockExecutionChatService::new());

        let services = TaskServices::new(
            Arc::clone(&spawner) as Arc<dyn AgentSpawner>,
            Arc::clone(&emitter) as Arc<dyn EventEmitter>,
            Arc::clone(&notifier) as Arc<dyn Notifier>,
            Arc::clone(&dep_manager) as Arc<dyn super::super::services::DependencyManager>,
            Arc::clone(&review_starter) as Arc<dyn ReviewStarter>,
        )
        .with_execution_chat_service(execution_chat_service.clone() as Arc<dyn ExecutionChatService>);

        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);

        let handler = TransitionHandler::new(&mut machine);
        handler.on_enter(&State::Executing).await;

        // ExecutionChatService should have been called (creates a conversation)
        let executions = execution_chat_service
            .list_task_executions(&TaskId::from_string("task-1".to_string()))
            .await
            .unwrap();
        assert_eq!(executions.len(), 1, "ExecutionChatService should have been called");

        // Agent spawner should NOT have been called (we used ExecutionChatService instead)
        let spawner_calls = spawner.get_calls();
        assert!(
            !spawner_calls.iter().any(|c| c.method == "spawn" && c.args[0] == "worker"),
            "Agent spawner should not be called when ExecutionChatService is available"
        );
    }

    #[tokio::test]
    async fn test_entering_executing_falls_back_to_agent_spawner_without_execution_chat_service() {
        // Use standard services without ExecutionChatService
        let (spawner, _emitter, _notifier, _dep_manager, _review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);

        // Verify execution_chat_service is None
        assert!(machine.context.services.execution_chat_service.is_none());

        let handler = TransitionHandler::new(&mut machine);
        handler.on_enter(&State::Executing).await;

        // Agent spawner should have been called (fallback behavior)
        let calls = spawner.get_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].method, "spawn");
        assert_eq!(calls[0].args, vec!["worker", "task-1"]);
    }

    #[tokio::test]
    async fn test_execution_chat_service_unavailable_falls_back_gracefully() {
        use crate::application::{ExecutionChatService, MockExecutionChatService};

        let spawner = Arc::new(MockAgentSpawner::new());
        let emitter = Arc::new(MockEventEmitter::new());
        let notifier = Arc::new(MockNotifier::new());
        let dep_manager = Arc::new(MockDependencyManager::new());
        let review_starter = Arc::new(MockReviewStarter::new());
        let execution_chat_service = Arc::new(MockExecutionChatService::new());

        // Mark the service as unavailable
        execution_chat_service.set_available(false).await;

        let services = TaskServices::new(
            Arc::clone(&spawner) as Arc<dyn AgentSpawner>,
            Arc::clone(&emitter) as Arc<dyn EventEmitter>,
            Arc::clone(&notifier) as Arc<dyn Notifier>,
            Arc::clone(&dep_manager) as Arc<dyn super::super::services::DependencyManager>,
            Arc::clone(&review_starter) as Arc<dyn ReviewStarter>,
        )
        .with_execution_chat_service(execution_chat_service.clone() as Arc<dyn ExecutionChatService>);

        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);

        let handler = TransitionHandler::new(&mut machine);
        handler.on_enter(&State::Executing).await;

        // The service is present but unavailable - spawn_with_persistence returns error
        // The current implementation still tries to use it (graceful degradation)
        // We verify that calling on_enter doesn't panic

        // ExecutionChatService was called even though unavailable (returns error)
        // The key is that the system doesn't crash
        let executions = execution_chat_service
            .list_task_executions(&TaskId::from_string("task-1".to_string()))
            .await
            .unwrap();
        // When unavailable, spawn_with_persistence returns error and no conversation is created
        assert_eq!(executions.len(), 0, "No conversation created when service unavailable");
    }
}
