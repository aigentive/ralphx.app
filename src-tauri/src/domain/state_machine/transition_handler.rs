// Transition handler - orchestrates side effects for state transitions
// This module wraps the state machine and handles entry/exit actions,
// especially for QA-related transitions.

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
                // Spawn worker agent
                self.machine
                    .context
                    .services
                    .agent_spawner
                    .spawn("worker", &self.machine.context.task_id)
                    .await;
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
                // Spawn reviewer agent
                self.machine
                    .context
                    .services
                    .agent_spawner
                    .spawn("reviewer", &self.machine.context.task_id)
                    .await;
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
        MockAgentSpawner, MockDependencyManager, MockEventEmitter, MockNotifier,
    };
    use crate::domain::state_machine::services::{AgentSpawner, EventEmitter, Notifier};
    use crate::domain::state_machine::types::QaFailedData;
    use std::sync::Arc;

    fn create_test_services() -> (
        Arc<MockAgentSpawner>,
        Arc<MockEventEmitter>,
        Arc<MockNotifier>,
        Arc<MockDependencyManager>,
        TaskServices,
    ) {
        let spawner = Arc::new(MockAgentSpawner::new());
        let emitter = Arc::new(MockEventEmitter::new());
        let notifier = Arc::new(MockNotifier::new());
        let dep_manager = Arc::new(MockDependencyManager::new());

        let services = TaskServices::new(
            Arc::clone(&spawner) as Arc<dyn AgentSpawner>,
            Arc::clone(&emitter) as Arc<dyn EventEmitter>,
            Arc::clone(&notifier) as Arc<dyn Notifier>,
            Arc::clone(&dep_manager) as Arc<dyn super::super::services::DependencyManager>,
        );

        (spawner, emitter, notifier, dep_manager, services)
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
        let (spawner, _emitter, _notifier, _dep_manager, services) = create_test_services();
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
        let (spawner, _emitter, _notifier, _dep_manager, services) = create_test_services();
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
        let (spawner, _emitter, _notifier, _dep_manager, services) = create_test_services();
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
        let (spawner, _emitter, _notifier, _dep_manager, services) = create_test_services();
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
        let (spawner, _emitter, _notifier, _dep_manager, services) = create_test_services();
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
        let (spawner, _emitter, _notifier, _dep_manager, services) = create_test_services();
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
        let (_spawner, emitter, _notifier, _dep_manager, services) = create_test_services();
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
        let (_spawner, emitter, notifier, _dep_manager, services) = create_test_services();
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
        let (spawner, _emitter, _notifier, _dep_manager, services) = create_test_services();
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
        let (_spawner, _emitter, _notifier, _dep_manager, services) = create_test_services();
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
        let (_spawner, emitter, _notifier, dep_manager, services) = create_test_services();
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
        let (spawner, _emitter, _notifier, _dep_manager, services) = create_test_services();
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
        let (_spawner, emitter, _notifier, _dep_manager, services) = create_test_services();
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
        let (_spawner, _emitter, _notifier, _dep_manager, services) = create_test_services();
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
        let (spawner, _emitter, _notifier, _dep_manager, services) = create_test_services();
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
}
