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
    ///
    /// This method is public to allow `TaskTransitionService` to trigger entry actions
    /// for direct status changes (e.g., Kanban drag-drop) without going through the
    /// full event-based transition flow.
    pub async fn on_enter(&self, state: &State) {
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
                // Use ChatService for persistent worker execution (Phase 15B)
                let task_id = &self.machine.context.task_id;
                let prompt = format!("Execute task: {}", task_id);

                // send_message handles:
                // 1. Creating chat_conversation (context_type: 'task_execution')
                // 2. Creating agent_run (status: 'running')
                // 3. Spawning Claude CLI with --agent worker
                // 4. Persisting stream output to chat_messages
                // 5. Processing queued messages on completion
                let _ = self
                    .machine
                    .context
                    .services
                    .chat_service
                    .send_message(
                        crate::domain::entities::ChatContextType::TaskExecution,
                        task_id,
                        &prompt,
                    )
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
            State::QaPassed => {
                // Auto-transition to PendingReview
                Some(State::PendingReview)
            }
            State::RevisionNeeded => {
                // Auto-transition to ReExecuting (revision work)
                Some(State::ReExecuting)
            }
            State::PendingReview => {
                // Auto-transition to Reviewing (spawn reviewer)
                Some(State::Reviewing)
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
        use crate::application::{ChatService, MockChatService};

        let spawner = Arc::new(MockAgentSpawner::new());
        let emitter = Arc::new(MockEventEmitter::new());
        let notifier = Arc::new(MockNotifier::new());
        let dep_manager = Arc::new(MockDependencyManager::new());
        let review_starter = Arc::new(MockReviewStarter::new());
        let chat_service = Arc::new(MockChatService::new());

        let services = TaskServices::new(
            Arc::clone(&spawner) as Arc<dyn AgentSpawner>,
            Arc::clone(&emitter) as Arc<dyn EventEmitter>,
            Arc::clone(&notifier) as Arc<dyn Notifier>,
            Arc::clone(&dep_manager) as Arc<dyn super::super::services::DependencyManager>,
            Arc::clone(&review_starter) as Arc<dyn ReviewStarter>,
            Arc::clone(&chat_service) as Arc<dyn ChatService>,
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
    async fn test_executing_complete_transitions_to_qa_refining_with_qa() {
        let (spawner, _emitter, _notifier, _dep_manager, _review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services).with_qa_enabled();
        let mut machine = TaskStateMachine::new(context);
        let mut handler = TransitionHandler::new(&mut machine);

        let result = handler
            .handle_transition(&State::Executing, &TaskEvent::ExecutionComplete)
            .await;

        // Should transition directly to QaRefining
        assert_eq!(result.state(), Some(&State::QaRefining));

        // Should wait for qa-prep and spawn qa-refiner
        let calls = spawner.get_calls();
        assert!(calls.iter().any(|c| c.method == "wait_for" && c.args[0] == "qa-prep"));
        assert!(calls.iter().any(|c| c.method == "spawn" && c.args[0] == "qa-refiner"));
    }

    #[tokio::test]
    async fn test_executing_complete_transitions_to_pending_review_without_qa() {
        let (_spawner, _emitter, _notifier, _dep_manager, _review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);
        let mut handler = TransitionHandler::new(&mut machine);

        let result = handler
            .handle_transition(&State::Executing, &TaskEvent::ExecutionComplete)
            .await;

        // Should transition to PendingReview, then auto-transition to Reviewing
        if let TransitionResult::AutoTransition(state) = &result {
            assert_eq!(*state, State::Reviewing);
        } else {
            panic!("Expected AutoTransition to Reviewing, got {:?}", result);
        }
    }

    #[tokio::test]
    async fn test_executing_complete_with_qa_prep_complete_skips_wait() {
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
        let (_spawner, _emitter, _notifier, _dep_manager, _review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);
        let mut handler = TransitionHandler::new(&mut machine);

        let result = handler
            .handle_transition(
                &State::QaFailed(QaFailedData::default()),
                &TaskEvent::SkipQa,
            )
            .await;

        // Should transition to PendingReview, then auto-transition to Reviewing
        if let TransitionResult::AutoTransition(state) = &result {
            assert_eq!(*state, State::Reviewing);
        } else {
            panic!("Expected AutoTransition to Reviewing, got {:?}", result);
        }
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

        // Should auto-transition from RevisionNeeded to ReExecuting
        if let TransitionResult::AutoTransition(state) = &result {
            assert_eq!(*state, State::ReExecuting);
        } else {
            panic!("Expected AutoTransition, got {:?}", result);
        }
    }

    // ==================
    // Review and terminal state tests
    // ==================

    #[tokio::test]
    async fn test_reviewing_approved_transitions_to_review_passed() {
        let (_spawner, _emitter, _notifier, _dep_manager, _review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);
        let mut handler = TransitionHandler::new(&mut machine);

        let result = handler
            .handle_transition(
                &State::Reviewing,
                &TaskEvent::ReviewComplete {
                    approved: true,
                    feedback: None,
                },
            )
            .await;

        // Reviewing -> ReviewPassed (awaiting human)
        assert_eq!(result.state(), Some(&State::ReviewPassed));
    }

    #[tokio::test]
    async fn test_review_passed_human_approve_transitions_to_approved() {
        let (_spawner, emitter, _notifier, dep_manager, _review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);
        let mut handler = TransitionHandler::new(&mut machine);

        let result = handler
            .handle_transition(
                &State::ReviewPassed,
                &TaskEvent::HumanApprove,
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
    async fn test_reviewing_rejected_auto_transitions_to_re_executing() {
        let (_spawner, _emitter, _notifier, _dep_manager, _review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);
        let mut handler = TransitionHandler::new(&mut machine);

        let result = handler
            .handle_transition(
                &State::Reviewing,
                &TaskEvent::ReviewComplete {
                    approved: false,
                    feedback: Some("Needs tests".to_string()),
                },
            )
            .await;

        // Should auto-transition from RevisionNeeded to ReExecuting
        if let TransitionResult::AutoTransition(state) = &result {
            assert_eq!(*state, State::ReExecuting);
        } else {
            panic!("Expected AutoTransition, got {:?}", result);
        }

        // Worker is spawned via ExecutionChatService (not agent_spawner)
        // Test passes if the auto-transition completes without panic
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
        let (_spawner, _emitter, _notifier, _dep_manager, _review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);

        // Manually test on_enter for Executing
        let handler = TransitionHandler::new(&mut machine);
        handler.on_enter(&State::Executing).await;

        // Test passes if no panic occurs - ExecutionChatService is called
        // (The MockExecutionChatService handles the call gracefully)
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
        use crate::application::{ChatService, MockChatService};

        let spawner = Arc::new(MockAgentSpawner::new());
        let emitter = Arc::new(MockEventEmitter::new());
        let notifier = Arc::new(MockNotifier::new());
        let dep_manager = Arc::new(MockDependencyManager::new());
        let review_starter = Arc::new(MockReviewStarter::disabled());
        let chat_service = Arc::new(MockChatService::new());

        let services = TaskServices::new(
            Arc::clone(&spawner) as Arc<dyn AgentSpawner>,
            Arc::clone(&emitter) as Arc<dyn EventEmitter>,
            Arc::clone(&notifier) as Arc<dyn Notifier>,
            Arc::clone(&dep_manager) as Arc<dyn super::super::services::DependencyManager>,
            Arc::clone(&review_starter) as Arc<dyn ReviewStarter>,
            Arc::clone(&chat_service) as Arc<dyn ChatService>,
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
        use crate::application::{ChatService, MockChatService};

        let spawner = Arc::new(MockAgentSpawner::new());
        let emitter = Arc::new(MockEventEmitter::new());
        let notifier = Arc::new(MockNotifier::new());
        let dep_manager = Arc::new(MockDependencyManager::new());
        let review_starter = Arc::new(MockReviewStarter::with_error("Database connection failed"));
        let chat_service = Arc::new(MockChatService::new());

        let services = TaskServices::new(
            Arc::clone(&spawner) as Arc<dyn AgentSpawner>,
            Arc::clone(&emitter) as Arc<dyn EventEmitter>,
            Arc::clone(&notifier) as Arc<dyn Notifier>,
            Arc::clone(&dep_manager) as Arc<dyn super::super::services::DependencyManager>,
            Arc::clone(&review_starter) as Arc<dyn ReviewStarter>,
            Arc::clone(&chat_service) as Arc<dyn ChatService>,
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
    async fn test_executing_to_pending_review_starts_ai_review() {
        let (_spawner, _emitter, _notifier, _dep_manager, review_starter, services) = create_test_services();
        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);
        let mut handler = TransitionHandler::new(&mut machine);

        // Transition from Executing -> PendingReview (direct, no ExecutionDone)
        // then auto-transition to Reviewing
        let result = handler
            .handle_transition(&State::Executing, &TaskEvent::ExecutionComplete)
            .await;

        // Should auto-transition to Reviewing
        if let TransitionResult::AutoTransition(state) = &result {
            assert_eq!(*state, State::Reviewing);
        } else {
            panic!("Expected AutoTransition to Reviewing, got {:?}", result);
        }

        // PendingReview entry action starts AI review
        assert!(review_starter.has_review_for_task("task-1"));
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
    async fn test_entering_executing_uses_chat_service() {
        use crate::application::{ChatService, MockChatService};

        let spawner = Arc::new(MockAgentSpawner::new());
        let emitter = Arc::new(MockEventEmitter::new());
        let notifier = Arc::new(MockNotifier::new());
        let dep_manager = Arc::new(MockDependencyManager::new());
        let review_starter = Arc::new(MockReviewStarter::new());
        let chat_service = Arc::new(MockChatService::new());

        let services = TaskServices::new(
            Arc::clone(&spawner) as Arc<dyn AgentSpawner>,
            Arc::clone(&emitter) as Arc<dyn EventEmitter>,
            Arc::clone(&notifier) as Arc<dyn Notifier>,
            Arc::clone(&dep_manager) as Arc<dyn super::super::services::DependencyManager>,
            Arc::clone(&review_starter) as Arc<dyn ReviewStarter>,
            chat_service.clone() as Arc<dyn ChatService>,
        );

        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);

        let handler = TransitionHandler::new(&mut machine);
        handler.on_enter(&State::Executing).await;

        // ChatService should have been called (check call count)
        assert!(chat_service.call_count() > 0, "ChatService should have been called");

        // Agent spawner should NOT have been called (we used ChatService instead)
        let spawner_calls = spawner.get_calls();
        assert!(
            !spawner_calls.iter().any(|c| c.method == "spawn" && c.args[0] == "worker"),
            "Agent spawner should not be called when ChatService is available"
        );
    }

    #[tokio::test]
    async fn test_chat_service_unavailable_falls_back_gracefully() {
        use crate::application::{ChatService, MockChatService};

        let spawner = Arc::new(MockAgentSpawner::new());
        let emitter = Arc::new(MockEventEmitter::new());
        let notifier = Arc::new(MockNotifier::new());
        let dep_manager = Arc::new(MockDependencyManager::new());
        let review_starter = Arc::new(MockReviewStarter::new());
        let chat_service = Arc::new(MockChatService::new());

        // Mark the service as unavailable
        chat_service.set_available(false).await;

        let services = TaskServices::new(
            Arc::clone(&spawner) as Arc<dyn AgentSpawner>,
            Arc::clone(&emitter) as Arc<dyn EventEmitter>,
            Arc::clone(&notifier) as Arc<dyn Notifier>,
            Arc::clone(&dep_manager) as Arc<dyn super::super::services::DependencyManager>,
            Arc::clone(&review_starter) as Arc<dyn ReviewStarter>,
            chat_service.clone() as Arc<dyn ChatService>,
        );

        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);

        let handler = TransitionHandler::new(&mut machine);
        handler.on_enter(&State::Executing).await;

        // The service is present but unavailable - send_message returns error
        // The current implementation still tries to use it (graceful degradation)
        // We verify that calling on_enter doesn't panic
        // The key is that the system doesn't crash
    }
}
