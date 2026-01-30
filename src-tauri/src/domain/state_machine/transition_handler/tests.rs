use crate::domain::state_machine::{State, TaskEvent, TaskStateMachine, TransitionHandler, TransitionResult};
use crate::domain::state_machine::context::{TaskContext, TaskServices};
use crate::domain::state_machine::mocks::{
MockAgentSpawner, MockDependencyManager, MockEventEmitter, MockNotifier, MockReviewStarter,
};
use crate::domain::state_machine::services::{AgentSpawner, DependencyManager, EventEmitter, Notifier, ReviewStarter};
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
        Arc::clone(&dep_manager) as Arc<dyn DependencyManager>,
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
    let (_spawner, emitter, _notifier, _dep_manager, review_starter, services) = create_test_services();
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

    // NOTE: Reviewer is no longer spawned in PendingReview.
    // It's spawned in Reviewing state (via auto-transition).
    // See test_auto_transition_pending_review_to_reviewing for full flow.
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
        Arc::clone(&dep_manager) as Arc<dyn DependencyManager>,
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
        Arc::clone(&dep_manager) as Arc<dyn DependencyManager>,
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
    let (_spawner, emitter, _notifier, _dep_manager, review_starter, services) = create_test_services();
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

    // NOTE: Reviewer is no longer spawned in PendingReview.
    // It's spawned in Reviewing state (via auto-transition).

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
        Arc::clone(&dep_manager) as Arc<dyn DependencyManager>,
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
        Arc::clone(&dep_manager) as Arc<dyn DependencyManager>,
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

// ==================
// New review state entry action tests
// ==================

#[tokio::test]
async fn test_entering_reviewing_uses_chat_service() {
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
        Arc::clone(&dep_manager) as Arc<dyn DependencyManager>,
        Arc::clone(&review_starter) as Arc<dyn ReviewStarter>,
        chat_service.clone() as Arc<dyn ChatService>,
    );

    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);
    handler.on_enter(&State::Reviewing).await;

    // ChatService should have been called for reviewer with Review context
    assert!(chat_service.call_count() > 0, "ChatService should have been called");

    // Agent spawner should NOT have been called (we used ChatService instead)
    let spawner_calls = spawner.get_calls();
    assert!(
        !spawner_calls.iter().any(|c| c.method == "spawn" && c.args[0] == "reviewer"),
        "Agent spawner should not be called when ChatService is available"
    );
}

#[tokio::test]
async fn test_entering_review_passed_emits_event_and_notifies() {
    let (_spawner, emitter, notifier, _dep_manager, _review_starter, services) = create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);
    handler.on_enter(&State::ReviewPassed).await;

    // Should emit review:ai_approved event
    assert!(emitter.has_event("review:ai_approved"));

    // Should notify user
    assert!(notifier.has_notification("review:ai_approved"));
}

#[tokio::test]
async fn test_entering_re_executing_uses_chat_service() {
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
        Arc::clone(&dep_manager) as Arc<dyn DependencyManager>,
        Arc::clone(&review_starter) as Arc<dyn ReviewStarter>,
        chat_service.clone() as Arc<dyn ChatService>,
    );

    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);
    handler.on_enter(&State::ReExecuting).await;

    // ChatService should have been called for worker with revision context
    assert!(chat_service.call_count() > 0, "ChatService should have been called");

    // Agent spawner should NOT have been called (we used ChatService instead)
    let spawner_calls = spawner.get_calls();
    assert!(
        !spawner_calls.iter().any(|c| c.method == "spawn" && c.args[0] == "worker"),
        "Agent spawner should not be called when ChatService is available"
    );
}

#[tokio::test]
async fn test_exiting_reviewing_emits_event() {
    let (_spawner, emitter, _notifier, _dep_manager, _review_starter, services) = create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);
    handler.on_exit(&State::Reviewing, &State::ReviewPassed).await;

    // Should emit review:state_exited event
    assert!(emitter.has_event("review:state_exited"));
}

#[tokio::test]
async fn test_auto_transition_pending_review_to_reviewing() {
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
        Arc::clone(&dep_manager) as Arc<dyn DependencyManager>,
        Arc::clone(&review_starter) as Arc<dyn ReviewStarter>,
        chat_service.clone() as Arc<dyn ChatService>,
    );

    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let mut handler = TransitionHandler::new(&mut machine);

    // Manually transition to PendingReview (simulating ExecutionComplete)
    let result = handler
        .handle_transition(&State::Executing, &TaskEvent::ExecutionComplete)
        .await;

    // Should auto-transition to Reviewing
    if let TransitionResult::AutoTransition(state) = &result {
        assert_eq!(*state, State::Reviewing);
    } else {
        panic!("Expected AutoTransition to Reviewing, got {:?}", result);
    }

    // Review should be started
    assert!(review_starter.has_review_for_task("task-1"));

    // ChatService should have been called for reviewer with Review context
    assert!(chat_service.call_count() > 0, "ChatService should have been called");

    // Agent spawner should NOT have been called (we used ChatService instead)
    let spawner_calls = spawner.get_calls();
    assert!(
        !spawner_calls.iter().any(|c| c.method == "spawn" && c.args[0] == "reviewer"),
        "Agent spawner should not be called when ChatService is available"
    );
}

#[tokio::test]
async fn test_auto_transition_revision_needed_to_re_executing() {
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
        Arc::clone(&dep_manager) as Arc<dyn DependencyManager>,
        Arc::clone(&review_starter) as Arc<dyn ReviewStarter>,
        chat_service.clone() as Arc<dyn ChatService>,
    );

    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let mut handler = TransitionHandler::new(&mut machine);

    // Transition from Reviewing to RevisionNeeded (AI requested changes)
    let result = handler
        .handle_transition(
            &State::Reviewing,
            &TaskEvent::ReviewComplete {
                approved: false,
                feedback: Some("Needs more tests".to_string()),
            },
        )
        .await;

    // Should auto-transition to ReExecuting
    if let TransitionResult::AutoTransition(state) = &result {
        assert_eq!(*state, State::ReExecuting);
    } else {
        panic!("Expected AutoTransition to ReExecuting, got {:?}", result);
    }

    // Worker should be spawned via ChatService for re-execution
    assert!(chat_service.call_count() > 0, "ChatService should spawn worker for re-execution");
}

// ==================
// ExecutionState decrement on exit tests
// ==================

#[tokio::test]
async fn test_exiting_executing_decrements_running_count() {
    use crate::commands::ExecutionState;

    let execution_state = Arc::new(ExecutionState::new());
    // Simulate task already running
    execution_state.increment_running();
    assert_eq!(execution_state.running_count(), 1);

    let services = TaskServices::new_mock().with_execution_state(Arc::clone(&execution_state));
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);
    handler.on_exit(&State::Executing, &State::PendingReview).await;

    // Running count should be decremented
    assert_eq!(execution_state.running_count(), 0);
}

#[tokio::test]
async fn test_exiting_reviewing_decrements_running_count() {
    use crate::commands::ExecutionState;

    let execution_state = Arc::new(ExecutionState::new());
    execution_state.increment_running();
    assert_eq!(execution_state.running_count(), 1);

    let services = TaskServices::new_mock().with_execution_state(Arc::clone(&execution_state));
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);
    handler.on_exit(&State::Reviewing, &State::ReviewPassed).await;

    assert_eq!(execution_state.running_count(), 0);
}

#[tokio::test]
async fn test_exiting_qa_refining_decrements_running_count() {
    use crate::commands::ExecutionState;

    let execution_state = Arc::new(ExecutionState::new());
    execution_state.increment_running();

    let services = TaskServices::new_mock().with_execution_state(Arc::clone(&execution_state));
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);
    handler.on_exit(&State::QaRefining, &State::QaTesting).await;

    assert_eq!(execution_state.running_count(), 0);
}

#[tokio::test]
async fn test_exiting_qa_testing_decrements_running_count() {
    use crate::commands::ExecutionState;

    let execution_state = Arc::new(ExecutionState::new());
    execution_state.increment_running();

    let services = TaskServices::new_mock().with_execution_state(Arc::clone(&execution_state));
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);
    handler.on_exit(&State::QaTesting, &State::QaPassed).await;

    assert_eq!(execution_state.running_count(), 0);
}

#[tokio::test]
async fn test_exiting_re_executing_decrements_running_count() {
    use crate::commands::ExecutionState;

    let execution_state = Arc::new(ExecutionState::new());
    execution_state.increment_running();

    let services = TaskServices::new_mock().with_execution_state(Arc::clone(&execution_state));
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);
    handler.on_exit(&State::ReExecuting, &State::PendingReview).await;

    assert_eq!(execution_state.running_count(), 0);
}

#[tokio::test]
async fn test_exiting_non_agent_state_does_not_decrement() {
    use crate::commands::ExecutionState;

    let execution_state = Arc::new(ExecutionState::new());
    execution_state.increment_running();
    assert_eq!(execution_state.running_count(), 1);

    let services = TaskServices::new_mock().with_execution_state(Arc::clone(&execution_state));
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);
    // Exiting Ready (not agent-active) should NOT decrement
    handler.on_exit(&State::Ready, &State::Executing).await;

    // Running count should remain unchanged
    assert_eq!(execution_state.running_count(), 1);
}

#[tokio::test]
async fn test_exiting_without_execution_state_does_not_panic() {
    // Services without execution_state
    let services = TaskServices::new_mock();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);
    // Should not panic even without execution_state
    handler.on_exit(&State::Executing, &State::PendingReview).await;
}

#[tokio::test]
async fn test_on_exit_with_execution_state_no_app_handle_does_not_panic() {
    // Verifies that on_exit() handles the case where execution_state is Some
    // but app_handle is None (the emit_status_changed call is skipped gracefully).
    // Note: Actual event emission with app_handle requires a real Wry runtime,
    // which is tested via integration tests and execution_commands.rs emit tests.
    use crate::commands::ExecutionState;

    let execution_state = Arc::new(ExecutionState::new());
    execution_state.increment_running();
    assert_eq!(execution_state.running_count(), 1);

    // Services with execution_state but no app_handle
    let services = TaskServices::new_mock().with_execution_state(Arc::clone(&execution_state));
    // Note: app_handle is None by default in new_mock()
    assert!(services.app_handle.is_none());

    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);
    // Should not panic - emit_status_changed is skipped when app_handle is None
    handler.on_exit(&State::Executing, &State::PendingReview).await;

    // Running count should still be decremented
    assert_eq!(execution_state.running_count(), 0);
}

#[tokio::test]
async fn test_on_exit_emits_for_all_agent_active_states() {
    // Verifies that on_exit() attempts to emit status_changed for all agent-active states
    // The emit logic is the same for all states that decrement running count
    use crate::commands::ExecutionState;
    use crate::domain::state_machine::types::FailedData;

    let agent_active_states = [
        State::Executing,
        State::QaRefining,
        State::QaTesting,
        State::Reviewing,
        State::ReExecuting,
    ];

    for from_state in &agent_active_states {
        let execution_state = Arc::new(ExecutionState::new());
        execution_state.increment_running();

        let services = TaskServices::new_mock().with_execution_state(Arc::clone(&execution_state));
        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);

        let handler = TransitionHandler::new(&mut machine);
        // Each agent-active state should trigger decrement (and emit if app_handle present)
        let to_state = State::Failed(FailedData::default());
        handler.on_exit(from_state, &to_state).await;

        assert_eq!(
            execution_state.running_count(), 0,
            "Expected running_count=0 after exiting {:?}", from_state
        );
    }
}
