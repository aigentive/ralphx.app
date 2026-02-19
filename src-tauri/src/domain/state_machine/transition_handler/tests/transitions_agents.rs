use super::helpers::{create_context_with_services, create_test_services};
use crate::application::{ChatService, MockChatService};
use crate::domain::state_machine::context::TaskServices;
use crate::domain::state_machine::mocks::{
    MockAgentSpawner, MockDependencyManager, MockEventEmitter, MockNotifier, MockReviewStarter,
};
use crate::domain::state_machine::services::{
    AgentSpawner, DependencyManager, EventEmitter, Notifier, ReviewStarter,
};
use crate::domain::state_machine::{
    State, TaskEvent, TaskStateMachine, TransitionHandler, TransitionResult,
};
use std::sync::Arc;

#[tokio::test]
async fn test_entering_executing_spawns_worker() {
    let (_spawner, _emitter, _notifier, _dep_manager, _review_starter, services) =
        create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    // Manually test on_enter for Executing
    let handler = TransitionHandler::new(&mut machine);
    let _ = handler.on_enter(&State::Executing).await;

    // Test passes if no panic occurs - ExecutionChatService is called
    // (The MockExecutionChatService handles the call gracefully)
}

#[tokio::test]
async fn test_entering_pending_review_starts_ai_review() {
    let (_spawner, emitter, _notifier, _dep_manager, review_starter, services) =
        create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);
    let _ = handler.on_enter(&State::PendingReview).await;

    // Should have called start_ai_review
    assert!(review_starter.has_review_for_task("task-1"));
    assert_eq!(review_starter.call_count(), 1);

    // Should have emitted review:update event
    let events = emitter.get_events();
    assert!(events
        .iter()
        .any(|e| { e.method == "emit_with_payload" && e.args[0] == "review:update" }));

    // NOTE: Reviewer is no longer spawned in PendingReview.
    // It's spawned in Reviewing state (via auto-transition).
    // See test_auto_transition_pending_review_to_reviewing for full flow.
}

#[tokio::test]
async fn test_entering_pending_review_with_disabled_ai_review() {
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
    let _ = handler.on_enter(&State::PendingReview).await;

    // Should NOT spawn reviewer agent when AI review is disabled
    let calls = spawner.get_calls();
    assert!(!calls
        .iter()
        .any(|c| c.method == "spawn" && c.args[0] == "reviewer"));

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
    let _ = handler.on_enter(&State::PendingReview).await;

    // Should NOT spawn reviewer agent when review fails to start
    let calls = spawner.get_calls();
    assert!(!calls
        .iter()
        .any(|c| c.method == "spawn" && c.args[0] == "reviewer"));

    // Should notify user about the error
    assert!(notifier.has_notification("review_error"));
}

#[tokio::test]
async fn test_entering_pending_review_emits_started_event_with_review_id() {
    let (_spawner, emitter, _notifier, _dep_manager, review_starter, services) =
        create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);
    let _ = handler.on_enter(&State::PendingReview).await;

    // Verify review:update event contains the review ID
    let events = emitter.get_events();
    let review_event = events
        .iter()
        .find(|e| e.method == "emit_with_payload" && e.args[0] == "review:update")
        .expect("Should have review:update event");

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
    let (_spawner, _emitter, _notifier, _dep_manager, review_starter, services) =
        create_test_services();
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
    let (_spawner, emitter, _notifier, _dep_manager, review_starter, services) =
        create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services).with_qa_enabled();
    let mut machine = TaskStateMachine::new(context);
    let mut handler = TransitionHandler::new(&mut machine);

    // Transition from QaTesting -> QaPassed -> PendingReview (auto)
    let result = handler
        .handle_transition(
            &State::QaTesting,
            &TaskEvent::QaTestsComplete { passed: true },
        )
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
    assert!(emitter
        .get_events()
        .iter()
        .any(|e| { e.method == "emit_with_payload" && e.args[0] == "review:update" }));
}

#[tokio::test]
async fn test_entering_executing_uses_chat_service() {
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
    let _ = handler.on_enter(&State::Executing).await;

    // ChatService should have been called (check call count)
    assert!(
        chat_service.call_count() > 0,
        "ChatService should have been called"
    );

    // Agent spawner should NOT have been called (we used ChatService instead)
    let spawner_calls = spawner.get_calls();
    assert!(
        !spawner_calls
            .iter()
            .any(|c| c.method == "spawn" && c.args[0] == "worker"),
        "Agent spawner should not be called when ChatService is available"
    );
}

#[tokio::test]
async fn test_chat_service_unavailable_falls_back_gracefully() {
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
    let _ = handler.on_enter(&State::Executing).await;

    // The service is present but unavailable - send_message returns error
    // The current implementation still tries to use it (graceful degradation)
    // We verify that calling on_enter doesn't panic
    // The key is that the system doesn't crash
}

#[tokio::test]
async fn test_entering_reviewing_uses_chat_service() {
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
    let _ = handler.on_enter(&State::Reviewing).await;

    // ChatService should have been called for reviewer with Review context
    assert!(
        chat_service.call_count() > 0,
        "ChatService should have been called"
    );

    // Agent spawner should NOT have been called (we used ChatService instead)
    let spawner_calls = spawner.get_calls();
    assert!(
        !spawner_calls
            .iter()
            .any(|c| c.method == "spawn" && c.args[0] == "reviewer"),
        "Agent spawner should not be called when ChatService is available"
    );
}

#[tokio::test]
async fn test_entering_review_passed_emits_event_and_notifies() {
    let (_spawner, emitter, notifier, _dep_manager, _review_starter, services) =
        create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);
    let _ = handler.on_enter(&State::ReviewPassed).await;

    // Should emit review:ai_approved event
    assert!(emitter.has_event("review:ai_approved"));

    // Should notify user
    assert!(notifier.has_notification("review:ai_approved"));
}

#[tokio::test]
async fn test_entering_re_executing_uses_chat_service() {
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
    let _ = handler.on_enter(&State::ReExecuting).await;

    // ChatService should have been called for worker with revision context
    assert!(
        chat_service.call_count() > 0,
        "ChatService should have been called"
    );

    // Agent spawner should NOT have been called (we used ChatService instead)
    let spawner_calls = spawner.get_calls();
    assert!(
        !spawner_calls
            .iter()
            .any(|c| c.method == "spawn" && c.args[0] == "worker"),
        "Agent spawner should not be called when ChatService is available"
    );
}

#[tokio::test]
async fn test_exiting_reviewing_emits_event() {
    let (_spawner, emitter, _notifier, _dep_manager, _review_starter, services) =
        create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);
    handler
        .on_exit(&State::Reviewing, &State::ReviewPassed)
        .await;

    // Should emit review:state_exited event
    assert!(emitter.has_event("review:state_exited"));
}

#[tokio::test]
async fn test_auto_transition_pending_review_to_reviewing() {
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
    assert!(
        chat_service.call_count() > 0,
        "ChatService should have been called"
    );

    // Agent spawner should NOT have been called (we used ChatService instead)
    let spawner_calls = spawner.get_calls();
    assert!(
        !spawner_calls
            .iter()
            .any(|c| c.method == "spawn" && c.args[0] == "reviewer"),
        "Agent spawner should not be called when ChatService is available"
    );
}

#[tokio::test]
async fn test_auto_transition_revision_needed_to_re_executing() {
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
    assert!(
        chat_service.call_count() > 0,
        "ChatService should spawn worker for re-execution"
    );
}
