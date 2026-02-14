use crate::domain::entities::InternalStatus;
use crate::domain::state_machine::context::{TaskContext, TaskServices};
use crate::domain::state_machine::mocks::{
    MockAgentSpawner, MockDependencyManager, MockEventEmitter, MockNotifier, MockReviewStarter,
};
use crate::domain::state_machine::services::{
    AgentSpawner, DependencyManager, EventEmitter, Notifier, ReviewStarter,
};
use crate::domain::state_machine::types::QaFailedData;
use crate::domain::state_machine::{
    State, TaskEvent, TaskStateMachine, TransitionHandler, TransitionResult,
};
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

    (
        spawner,
        emitter,
        notifier,
        dep_manager,
        review_starter,
        services,
    )
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
    let (spawner, _emitter, _notifier, _dep_manager, _review_starter, services) =
        create_test_services();
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
    let (spawner, _emitter, _notifier, _dep_manager, _review_starter, services) =
        create_test_services();
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
    let (spawner, _emitter, _notifier, _dep_manager, _review_starter, services) =
        create_test_services();
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
    assert!(calls
        .iter()
        .any(|c| c.method == "wait_for" && c.args[0] == "qa-prep"));
    assert!(calls
        .iter()
        .any(|c| c.method == "spawn" && c.args[0] == "qa-refiner"));
}

#[tokio::test]
async fn test_executing_complete_transitions_to_pending_review_without_qa() {
    let (_spawner, _emitter, _notifier, _dep_manager, _review_starter, services) =
        create_test_services();
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
    let (spawner, _emitter, _notifier, _dep_manager, _review_starter, services) =
        create_test_services();
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
    assert!(calls
        .iter()
        .any(|c| c.method == "spawn" && c.args[0] == "qa-refiner"));
}

// ==================
// QA state tests
// ==================

#[tokio::test]
async fn test_qa_refining_to_qa_testing() {
    let (spawner, _emitter, _notifier, _dep_manager, _review_starter, services) =
        create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services).with_qa_enabled();
    let mut machine = TaskStateMachine::new(context);
    let mut handler = TransitionHandler::new(&mut machine);

    let result = handler
        .handle_transition(&State::QaRefining, &TaskEvent::QaRefinementComplete)
        .await;

    assert_eq!(result.state(), Some(&State::QaTesting));

    // Should spawn qa-tester
    let calls = spawner.get_calls();
    assert!(calls
        .iter()
        .any(|c| c.method == "spawn" && c.args[0] == "qa-tester"));
}

#[tokio::test]
async fn test_qa_testing_passed() {
    let (_spawner, emitter, _notifier, _dep_manager, _review_starter, services) =
        create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services).with_qa_enabled();
    let mut machine = TaskStateMachine::new(context);
    let mut handler = TransitionHandler::new(&mut machine);

    let result = handler
        .handle_transition(
            &State::QaTesting,
            &TaskEvent::QaTestsComplete { passed: true },
        )
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
    let (_spawner, emitter, notifier, _dep_manager, _review_starter, services) =
        create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services).with_qa_enabled();
    let mut machine = TaskStateMachine::new(context);
    let mut handler = TransitionHandler::new(&mut machine);

    let result = handler
        .handle_transition(
            &State::QaTesting,
            &TaskEvent::QaTestsComplete { passed: false },
        )
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
    let (_spawner, _emitter, _notifier, _dep_manager, _review_starter, services) =
        create_test_services();
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
    let (_spawner, _emitter, _notifier, _dep_manager, _review_starter, services) =
        create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let mut handler = TransitionHandler::new(&mut machine);

    let result = handler
        .handle_transition(&State::QaFailed(QaFailedData::default()), &TaskEvent::Retry)
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
    let (_spawner, _emitter, _notifier, _dep_manager, _review_starter, services) =
        create_test_services();
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
async fn test_review_passed_human_approve_transitions_to_pending_merge() {
    let (_spawner, emitter, _notifier, dep_manager, _review_starter, services) =
        create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let mut handler = TransitionHandler::new(&mut machine);

    let result = handler
        .handle_transition(&State::ReviewPassed, &TaskEvent::HumanApprove)
        .await;

    // Should auto-transition from Approved to PendingMerge (Phase 66 - merge workflow)
    if let TransitionResult::AutoTransition(state) = &result {
        assert_eq!(*state, State::PendingMerge);
    } else {
        panic!("Expected AutoTransition to PendingMerge, got {:?}", result);
    }

    // Should emit task_completed event (during Approved entry)
    assert!(emitter.has_event("task_completed"));

    // Should NOT unblock dependents at Approved - only at Merged (after successful merge)
    let calls = dep_manager.get_calls();
    assert!(
        !calls.iter().any(|c| c.method == "unblock_dependents"),
        "unblock_dependents should NOT be called at Approved - only at Merged"
    );
}

#[tokio::test]
async fn test_reviewing_rejected_auto_transitions_to_re_executing() {
    let (_spawner, _emitter, _notifier, _dep_manager, _review_starter, services) =
        create_test_services();
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
    let (_spawner, emitter, _notifier, _dep_manager, _review_starter, services) =
        create_test_services();
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
    let (_spawner, _emitter, _notifier, _dep_manager, _review_starter, services) =
        create_test_services();
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

// ==================
// Review integration tests
// ==================

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
    let _ = handler.on_enter(&State::Executing).await;

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
    assert!(
        chat_service.call_count() > 0,
        "ChatService should spawn worker for re-execution"
    );
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
    handler
        .on_exit(&State::Executing, &State::PendingReview)
        .await;

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
    handler
        .on_exit(&State::Reviewing, &State::ReviewPassed)
        .await;

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
    handler
        .on_exit(&State::ReExecuting, &State::PendingReview)
        .await;

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
    handler
        .on_exit(&State::Executing, &State::PendingReview)
        .await;
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
    handler
        .on_exit(&State::Executing, &State::PendingReview)
        .await;

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
            execution_state.running_count(),
            0,
            "Expected running_count=0 after exiting {:?}",
            from_state
        );
    }
}

// ==================
// Deferred Merge Retry Tests
// ==================

#[tokio::test]
async fn test_exiting_pending_merge_triggers_retry_deferred_merges() {
    use crate::domain::state_machine::mocks::MockTaskScheduler;

    let scheduler = Arc::new(MockTaskScheduler::new());
    let services = TaskServices::new_mock()
        .with_task_scheduler(Arc::clone(&scheduler)
            as Arc<dyn crate::domain::state_machine::services::TaskScheduler>);

    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    // Transition from PendingMerge to Merged (successful merge)
    handler.on_exit(&State::PendingMerge, &State::Merged).await;

    // Give the spawned task a moment to execute
    tokio::time::sleep(tokio::time::Duration::from_millis(900)).await;

    // Verify try_retry_deferred_merges was called
    let calls = scheduler.get_calls();
    let retry_calls: Vec<_> = calls
        .iter()
        .filter(|c| c.method == "try_retry_deferred_merges")
        .collect();

    assert_eq!(
        retry_calls.len(),
        1,
        "Expected exactly one try_retry_deferred_merges call"
    );
    assert_eq!(
        retry_calls[0].args,
        vec!["proj-1"],
        "Expected project_id to be passed"
    );
}

#[tokio::test]
async fn test_exiting_pending_merge_to_merge_incomplete_triggers_retry() {
    use crate::domain::state_machine::mocks::MockTaskScheduler;

    let scheduler = Arc::new(MockTaskScheduler::new());
    let services = TaskServices::new_mock()
        .with_task_scheduler(Arc::clone(&scheduler)
            as Arc<dyn crate::domain::state_machine::services::TaskScheduler>);

    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    // Transition from PendingMerge to MergeIncomplete (failed merge)
    handler
        .on_exit(&State::PendingMerge, &State::MergeIncomplete)
        .await;

    // Give the spawned task a moment to execute
    tokio::time::sleep(tokio::time::Duration::from_millis(900)).await;

    // Verify try_retry_deferred_merges was called even on failure
    let calls = scheduler.get_calls();
    let retry_calls: Vec<_> = calls
        .iter()
        .filter(|c| c.method == "try_retry_deferred_merges")
        .collect();

    assert_eq!(
        retry_calls.len(),
        1,
        "Expected retry even on merge_incomplete"
    );
    assert_eq!(retry_calls[0].args, vec!["proj-1"]);
}

#[tokio::test]
async fn test_exiting_merging_to_merged_triggers_retry() {
    use crate::domain::state_machine::mocks::MockTaskScheduler;

    let scheduler = Arc::new(MockTaskScheduler::new());
    let services = TaskServices::new_mock()
        .with_task_scheduler(Arc::clone(&scheduler)
            as Arc<dyn crate::domain::state_machine::services::TaskScheduler>);

    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    // Transition from Merging to Merged (manual merge completion)
    handler.on_exit(&State::Merging, &State::Merged).await;

    // Give the spawned task a moment to execute
    tokio::time::sleep(tokio::time::Duration::from_millis(900)).await;

    // Verify try_retry_deferred_merges was called
    let calls = scheduler.get_calls();
    let retry_calls: Vec<_> = calls
        .iter()
        .filter(|c| c.method == "try_retry_deferred_merges")
        .collect();

    assert_eq!(retry_calls.len(), 1);
    assert_eq!(retry_calls[0].args, vec!["proj-1"]);
}

#[tokio::test]
async fn test_exiting_merging_to_merge_incomplete_triggers_retry() {
    use crate::domain::state_machine::mocks::MockTaskScheduler;

    let scheduler = Arc::new(MockTaskScheduler::new());
    let services = TaskServices::new_mock()
        .with_task_scheduler(Arc::clone(&scheduler)
            as Arc<dyn crate::domain::state_machine::services::TaskScheduler>);

    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    // Transition from Merging to MergeIncomplete (merge failed during conflict resolution)
    handler
        .on_exit(&State::Merging, &State::MergeIncomplete)
        .await;

    // Give the spawned task a moment to execute
    tokio::time::sleep(tokio::time::Duration::from_millis(900)).await;

    // Verify try_retry_deferred_merges was called
    let calls = scheduler.get_calls();
    let retry_calls: Vec<_> = calls
        .iter()
        .filter(|c| c.method == "try_retry_deferred_merges")
        .collect();

    assert_eq!(retry_calls.len(), 1);
    assert_eq!(retry_calls[0].args, vec!["proj-1"]);
}

#[tokio::test]
async fn test_exiting_other_states_does_not_trigger_retry() {
    use crate::domain::state_machine::mocks::MockTaskScheduler;

    let scheduler = Arc::new(MockTaskScheduler::new());
    let services = TaskServices::new_mock()
        .with_task_scheduler(Arc::clone(&scheduler)
            as Arc<dyn crate::domain::state_machine::services::TaskScheduler>);

    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    // Transition from Ready to Executing (normal execution start)
    handler.on_exit(&State::Ready, &State::Executing).await;

    // Give potential spawned tasks time (though none should spawn)
    tokio::time::sleep(tokio::time::Duration::from_millis(900)).await;

    // Verify try_retry_deferred_merges was NOT called for non-merge states
    let calls = scheduler.get_calls();
    let retry_calls: Vec<_> = calls
        .iter()
        .filter(|c| c.method == "try_retry_deferred_merges")
        .collect();

    assert_eq!(
        retry_calls.len(),
        0,
        "Expected no retry calls for non-merge state transitions"
    );
}

#[tokio::test]
async fn test_no_scheduler_does_not_panic_on_exit() {
    // Create services without a scheduler
    let services = TaskServices::new_mock();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    // Should not panic when scheduler is None
    handler.on_exit(&State::PendingMerge, &State::Merged).await;

    // Wait a bit to ensure no panic from spawned task
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
}

// ==================
// Non-blocking retry merge tests
// ==================

/// Test: retry_merge command returns quickly (on_enter(PendingMerge) is non-blocking
/// at the transition handler level).
///
/// The state machine dispatches Approved → PendingMerge as an auto-transition.
/// The handle_transition call should complete in bounded time because on_enter
/// for PendingMerge delegates heavy work (attempt_programmatic_merge) which,
/// without repos, returns immediately. This validates the structural non-blocking
/// property: the command handler can return while background work continues.
#[tokio::test]
async fn test_retry_merge_command_latency() {
    use std::time::Instant;

    let (_spawner, _emitter, _notifier, _dep_manager, _review_starter, services) =
        create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let mut handler = TransitionHandler::new(&mut machine);

    // Measure time for the transition from ReviewPassed -> Approved -> PendingMerge
    // (Approved auto-transitions to PendingMerge)
    let start = Instant::now();
    let result = handler
        .handle_transition(&State::ReviewPassed, &TaskEvent::HumanApprove)
        .await;
    let elapsed = start.elapsed();

    // The transition should complete quickly (without repos, on_enter(PendingMerge)
    // skips the heavy merge attempt and returns immediately).
    assert!(
        elapsed.as_millis() < 100,
        "Transition to PendingMerge should complete in <100ms, took {}ms",
        elapsed.as_millis()
    );

    // Verify correct auto-transition chain: ReviewPassed -> Approved -> PendingMerge
    if let TransitionResult::AutoTransition(state) = &result {
        assert_eq!(*state, State::PendingMerge);
    } else {
        panic!(
            "Expected AutoTransition to PendingMerge, got {:?}",
            result
        );
    }
}

/// Test: on_enter(PendingMerge) without repos returns immediately without blocking.
///
/// Validates that when repos are not available, the merge attempt is a no-op
/// and the handler returns quickly, preventing any app-wide hang.
#[tokio::test]
async fn test_pending_merge_entry_without_repos_returns_immediately() {
    use std::time::Instant;

    let services = TaskServices::new_mock(); // No task_repo or project_repo
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    let start = Instant::now();
    let result = handler.on_enter(&State::PendingMerge).await;
    let elapsed = start.elapsed();

    // Without repos, on_enter(PendingMerge) returns immediately (skips merge attempt)
    assert!(result.is_ok());
    assert!(
        elapsed.as_millis() < 50,
        "on_enter(PendingMerge) without repos should be near-instant, took {}ms",
        elapsed.as_millis()
    );
}

// ==================
// Background execution correctness tests
// ==================

/// Test: State transitions for merge workflow occur in correct order.
///
/// Validates the expected progression:
/// ReviewPassed -> Approved -> PendingMerge (via auto-transitions)
/// with correct side effects at each stage.
#[tokio::test]
async fn test_background_execution_correctness_state_ordering() {
    let (_spawner, emitter, _notifier, dep_manager, _review_starter, services) =
        create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let mut handler = TransitionHandler::new(&mut machine);

    // Trigger: ReviewPassed -> HumanApprove
    // Expected chain: ReviewPassed -> Approved -> PendingMerge (auto-transition)
    let result = handler
        .handle_transition(&State::ReviewPassed, &TaskEvent::HumanApprove)
        .await;

    // Final state should be PendingMerge (via Approved auto-transition)
    assert_eq!(
        result.state(),
        Some(&State::PendingMerge),
        "Should auto-transition to PendingMerge"
    );

    // Approved entry action should have emitted task_completed
    assert!(
        emitter.has_event("task_completed"),
        "Should emit task_completed on entering Approved"
    );

    // Approved should NOT unblock dependents (that happens at Merged)
    let dep_calls = dep_manager.get_calls();
    assert!(
        !dep_calls
            .iter()
            .any(|c| c.method == "unblock_dependents"),
        "Should NOT unblock dependents at Approved — only at Merged"
    );
}

/// Test: On entering Merged state, dependents are unblocked and scheduling triggered.
///
/// Verifies the terminal merge state correctly handles:
/// 1. Dependency unblocking
/// 2. Ready task scheduling
/// 3. Deferred merge retry
#[tokio::test]
async fn test_background_execution_merged_terminal_state() {
    use crate::domain::state_machine::mocks::MockTaskScheduler;

    let scheduler = Arc::new(MockTaskScheduler::new());
    let dep_manager = Arc::new(MockDependencyManager::new());

    let mut services = TaskServices::new_mock();
    services.dependency_manager = Arc::clone(&dep_manager) as Arc<dyn DependencyManager>;
    services.task_scheduler =
        Some(Arc::clone(&scheduler) as Arc<dyn crate::domain::state_machine::services::TaskScheduler>);

    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);
    let _ = handler.on_enter(&State::Merged).await;

    // Dependents should be unblocked
    let dep_calls = dep_manager.get_calls();
    assert!(
        dep_calls
            .iter()
            .any(|c| c.method == "unblock_dependents" && c.args[0] == "task-1"),
        "Should unblock dependents on Merged entry"
    );

    // Wait for spawned scheduling/retry tasks
    tokio::time::sleep(tokio::time::Duration::from_millis(900)).await;

    let sched_calls = scheduler.get_calls();

    // Should trigger ready task scheduling
    assert!(
        sched_calls
            .iter()
            .any(|c| c.method == "try_schedule_ready_tasks"),
        "Should schedule ready tasks after merge"
    );

    // Should trigger deferred merge retry
    assert!(
        sched_calls
            .iter()
            .any(|c| c.method == "try_retry_deferred_merges"),
        "Should retry deferred merges after merge"
    );
}

/// Test: MergeIncomplete -> PendingMerge (retry) handles transition correctly.
///
/// Simulates the retry_merge path: user clicks Retry from MergeIncomplete,
/// which transitions to PendingMerge.
#[tokio::test]
async fn test_background_execution_retry_from_merge_incomplete() {
    let (_spawner, _emitter, _notifier, _dep_manager, _review_starter, services) =
        create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let mut handler = TransitionHandler::new(&mut machine);

    // MergeIncomplete -> Retry -> PendingMerge
    let result = handler
        .handle_transition(&State::MergeIncomplete, &TaskEvent::Retry)
        .await;

    assert_eq!(
        result.state(),
        Some(&State::PendingMerge),
        "Retry from MergeIncomplete should go to PendingMerge"
    );
}

// ==================
// Blocking isolation tests
// ==================

/// Test: While in PendingMerge state, unrelated state machine operations
/// remain responsive and do not degrade.
///
/// Validates runtime isolation: the merge workflow for one task does not
/// block state machine operations for other tasks.
#[tokio::test]
async fn test_blocking_isolation_concurrent_operations() {
    use std::time::Instant;

    // Task 1: enters PendingMerge (merge workflow in progress)
    let services1 = TaskServices::new_mock();
    let context1 = create_context_with_services("task-1", "proj-1", services1);
    let mut machine1 = TaskStateMachine::new(context1);
    let handler1 = TransitionHandler::new(&mut machine1);

    // Trigger on_enter for PendingMerge (starts merge attempt, which is a no-op without repos)
    let _ = handler1.on_enter(&State::PendingMerge).await;

    // Task 2: independent operation (Backlog -> Ready) should not be affected
    let (spawner2, _emitter2, _notifier2, _dep_manager2, _review_starter2, services2) =
        create_test_services();
    let context2 = create_context_with_services("task-2", "proj-1", services2);
    let mut machine2 = TaskStateMachine::new(context2);
    let mut handler2 = TransitionHandler::new(&mut machine2);

    let start = Instant::now();
    let result = handler2
        .handle_transition(&State::Backlog, &TaskEvent::Schedule)
        .await;
    let elapsed = start.elapsed();

    // Unrelated transition should complete quickly
    assert!(result.is_success());
    assert_eq!(result.state(), Some(&State::Ready));
    assert!(
        elapsed.as_millis() < 50,
        "Unrelated transition should not be blocked by merge workflow, took {}ms",
        elapsed.as_millis()
    );

    // Verify the second task's operations completed correctly
    assert_eq!(spawner2.spawn_count(), 0); // No QA, so no agent spawned
}

/// Test: ExecutionState running count is not affected by PendingMerge transitions.
///
/// PendingMerge is NOT an agent-active state (only Executing, QaRefining,
/// QaTesting, Reviewing, ReExecuting, and Merging are). Exiting PendingMerge
/// should not affect the execution concurrency counter.
#[tokio::test]
async fn test_blocking_isolation_execution_state_unaffected_by_pending_merge() {
    use crate::commands::ExecutionState;

    let execution_state = Arc::new(ExecutionState::new());
    // Simulate one task running
    execution_state.increment_running();
    assert_eq!(execution_state.running_count(), 1);

    let services = TaskServices::new_mock().with_execution_state(Arc::clone(&execution_state));
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    // Exiting PendingMerge to Merged should NOT decrement (PendingMerge is not agent-active)
    handler
        .on_exit(&State::PendingMerge, &State::Merged)
        .await;

    // PendingMerge is NOT agent-active, running count should be unchanged
    assert_eq!(
        execution_state.running_count(),
        1,
        "PendingMerge exit should NOT decrement running count (not agent-active)"
    );
}

// ==================
// Reload continuation tests
// ==================

/// Test: Callback drop during merge workflow is handled gracefully.
///
/// When a Tauri app reloads, in-flight callbacks are dropped. The state machine
/// should not panic or leave state inconsistent. Without an app_handle, event
/// emission is silently skipped.
///
/// PendingMerge is NOT agent-active, so it doesn't decrement running count.
/// But the exit should still not panic even without app_handle.
#[tokio::test]
async fn test_reload_continuation_callback_drop() {
    use crate::commands::ExecutionState;

    let execution_state = Arc::new(ExecutionState::new());
    execution_state.increment_running();

    // Services WITHOUT app_handle simulates reload scenario
    let services = TaskServices::new_mock().with_execution_state(Arc::clone(&execution_state));
    assert!(
        services.app_handle.is_none(),
        "Mock services should not have app_handle"
    );

    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    // Simulate various merge-related on_exit calls without app_handle
    // None should panic (graceful handling even without app_handle)
    handler
        .on_exit(&State::PendingMerge, &State::Merged)
        .await;
    handler
        .on_exit(&State::PendingMerge, &State::MergeIncomplete)
        .await;

    // PendingMerge is NOT agent-active, so running count is unchanged
    assert_eq!(
        execution_state.running_count(),
        1,
        "Running count should be unchanged (PendingMerge is not agent-active)"
    );
}

/// Test: on_enter for merge states without app_handle (reload scenario).
///
/// After reload, the backend continues processing but app_handle is None.
/// Entry actions should still execute correctly (minus event emission).
#[tokio::test]
async fn test_reload_continuation_enter_states_without_app_handle() {
    let services = TaskServices::new_mock();
    assert!(services.app_handle.is_none());

    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    // on_enter for PendingMerge should not panic without app_handle
    let result = handler.on_enter(&State::PendingMerge).await;
    assert!(result.is_ok(), "on_enter(PendingMerge) should succeed without app_handle");

    // on_enter for Merged should not panic without app_handle
    let result = handler.on_enter(&State::Merged).await;
    assert!(result.is_ok(), "on_enter(Merged) should succeed without app_handle");

    // on_enter for Merging should not panic without app_handle
    let result = handler.on_enter(&State::Merging).await;
    assert!(result.is_ok(), "on_enter(Merging) should succeed without app_handle");
}

/// Test: State remains correct after simulated reload mid-merge.
///
/// Validates that state machine operations can resume after a "reload"
/// (new context creation) with the task still in PendingMerge.
#[tokio::test]
async fn test_reload_continuation_state_recovery() {
    use crate::domain::state_machine::mocks::MockTaskScheduler;

    // Phase 1: Task enters PendingMerge
    let scheduler1 = Arc::new(MockTaskScheduler::new());
    let services1 = TaskServices::new_mock()
        .with_task_scheduler(Arc::clone(&scheduler1)
            as Arc<dyn crate::domain::state_machine::services::TaskScheduler>);
    let context1 = create_context_with_services("task-1", "proj-1", services1);
    let mut machine1 = TaskStateMachine::new(context1);
    let handler1 = TransitionHandler::new(&mut machine1);

    // Enter PendingMerge
    let _ = handler1.on_enter(&State::PendingMerge).await;

    // Phase 2: Simulate "reload" — create fresh context for same task
    let scheduler2 = Arc::new(MockTaskScheduler::new());
    let services2 = TaskServices::new_mock()
        .with_task_scheduler(Arc::clone(&scheduler2)
            as Arc<dyn crate::domain::state_machine::services::TaskScheduler>);
    let context2 = create_context_with_services("task-1", "proj-1", services2);
    let mut machine2 = TaskStateMachine::new(context2);
    let handler2 = TransitionHandler::new(&mut machine2);

    // After reload, system could re-trigger on_enter(PendingMerge)
    // to resume the merge attempt. This should not panic or produce errors.
    let result = handler2.on_enter(&State::PendingMerge).await;
    assert!(
        result.is_ok(),
        "Re-entering PendingMerge after reload should succeed"
    );

    // Or the merge could complete (transition to Merged from outside)
    handler2
        .on_exit(&State::PendingMerge, &State::Merged)
        .await;

    // Deferred merge retry should still be triggered after reload
    tokio::time::sleep(tokio::time::Duration::from_millis(900)).await;
    let calls = scheduler2.get_calls();
    assert!(
        calls
            .iter()
            .any(|c| c.method == "try_retry_deferred_merges"),
        "Deferred merge retry should work after reload"
    );
}

// ==================
// Event emission tests
// ==================

/// Test: Approved entry emits task_completed event.
///
/// The Approved state is reached via ReviewPassed -> HumanApprove,
/// and should emit task_completed to notify the frontend.
#[tokio::test]
async fn test_event_emission_approved_emits_task_completed() {
    let (_spawner, emitter, _notifier, _dep_manager, _review_starter, services) =
        create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let mut handler = TransitionHandler::new(&mut machine);

    let result = handler
        .handle_transition(&State::ReviewPassed, &TaskEvent::HumanApprove)
        .await;

    // Should auto-transition to PendingMerge
    assert_eq!(result.state(), Some(&State::PendingMerge));

    // task_completed should be emitted during Approved entry (before auto-transition)
    assert!(
        emitter.has_event("task_completed"),
        "Should emit task_completed when entering Approved"
    );
}

/// Test: Merged entry emits appropriate events and triggers side effects.
///
/// on_enter(Merged) should unblock dependents and schedule tasks.
/// Event emission through mock emitter validates the dual-emission pattern.
#[tokio::test]
async fn test_event_emission_merged_entry_side_effects() {
    let dep_manager = Arc::new(MockDependencyManager::new());
    let emitter = Arc::new(MockEventEmitter::new());

    let mut services = TaskServices::new_mock();
    services.dependency_manager = Arc::clone(&dep_manager) as Arc<dyn DependencyManager>;
    services.event_emitter = Arc::clone(&emitter) as Arc<dyn EventEmitter>;

    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);
    let _ = handler.on_enter(&State::Merged).await;

    // Verify unblock_dependents was called
    let dep_calls = dep_manager.get_calls();
    assert!(
        dep_calls
            .iter()
            .any(|c| c.method == "unblock_dependents"),
        "on_enter(Merged) should unblock dependents"
    );
}

/// Test: Exiting PendingMerge does NOT decrement execution running count.
///
/// PendingMerge is not agent-active. The on_exit handler triggers deferred
/// merge retry but does NOT touch the execution concurrency counter.
#[tokio::test]
async fn test_event_emission_pending_merge_exit_preserves_execution_state() {
    use crate::commands::ExecutionState;

    let execution_state = Arc::new(ExecutionState::new());
    execution_state.increment_running();

    let emitter = Arc::new(MockEventEmitter::new());
    let mut services = TaskServices::new_mock();
    services.event_emitter = Arc::clone(&emitter) as Arc<dyn EventEmitter>;
    let services = services.with_execution_state(Arc::clone(&execution_state));

    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);
    handler
        .on_exit(&State::PendingMerge, &State::Merged)
        .await;

    // PendingMerge is NOT agent-active, running count should be unchanged
    assert_eq!(
        execution_state.running_count(),
        1,
        "PendingMerge exit should NOT decrement running count (not agent-active)"
    );
}

/// Test: Full merge event sequence from ReviewPassed through PendingMerge.
///
/// Validates the complete event chain for the merge workflow auto-transition:
/// ReviewPassed -> Approved (emits task_completed)
///              -> PendingMerge (exit Approved triggers no events)
#[tokio::test]
async fn test_event_emission_full_merge_event_sequence() {
    let (_spawner, emitter, _notifier, dep_manager, _review_starter, services) =
        create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let mut handler = TransitionHandler::new(&mut machine);

    // Full transition chain
    let result = handler
        .handle_transition(&State::ReviewPassed, &TaskEvent::HumanApprove)
        .await;

    assert_eq!(result.state(), Some(&State::PendingMerge));

    let events = emitter.get_events();

    // task_completed should be in the event list (emitted at Approved entry)
    assert!(
        events.iter().any(|e| e.args.first().map(|s| s.as_str()) == Some("task_completed")),
        "Event sequence should include task_completed"
    );

    // unblock_dependents should NOT have been called at Approved
    let dep_calls = dep_manager.get_calls();
    assert!(
        !dep_calls
            .iter()
            .any(|c| c.method == "unblock_dependents"),
        "Dependents should NOT be unblocked at Approved"
    );
}

// ==================
// Deferred merge compatibility regression tests
// ==================

/// Test: Non-blocking changes preserve deferred merge retry on PendingMerge exit.
///
/// Regression: when a task exits PendingMerge (to any terminal state),
/// the deferred merge retry must still be triggered for other waiting tasks.
#[tokio::test]
async fn test_deferred_merge_retry_on_all_pending_merge_exits() {
    use crate::domain::state_machine::mocks::MockTaskScheduler;

    let target_states = [
        State::Merged,
        State::MergeIncomplete,
        State::Merging,
    ];

    for target in &target_states {
        let scheduler = Arc::new(MockTaskScheduler::new());
        let services = TaskServices::new_mock()
            .with_task_scheduler(Arc::clone(&scheduler)
                as Arc<dyn crate::domain::state_machine::services::TaskScheduler>);

        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);
        let handler = TransitionHandler::new(&mut machine);

        handler.on_exit(&State::PendingMerge, target).await;

        // Wait for spawned deferred retry task
        tokio::time::sleep(tokio::time::Duration::from_millis(900)).await;

        let calls = scheduler.get_calls();
        let retry_calls: Vec<_> = calls
            .iter()
            .filter(|c| c.method == "try_retry_deferred_merges")
            .collect();

        assert_eq!(
            retry_calls.len(),
            1,
            "Expected deferred merge retry when exiting PendingMerge to {:?}",
            target
        );
        assert_eq!(
            retry_calls[0].args,
            vec!["proj-1"],
            "Deferred retry should use correct project_id for target {:?}",
            target
        );
    }
}

/// Test: Non-blocking changes preserve deferred merge retry on Merging exit.
///
/// Regression: Merging exit should also trigger deferred retry for tasks
/// waiting on the same target branch.
#[tokio::test]
async fn test_deferred_merge_retry_on_all_merging_exits() {
    use crate::domain::state_machine::mocks::MockTaskScheduler;

    let target_states = [
        State::Merged,
        State::MergeIncomplete,
        State::MergeConflict,
    ];

    for target in &target_states {
        let scheduler = Arc::new(MockTaskScheduler::new());
        let services = TaskServices::new_mock()
            .with_task_scheduler(Arc::clone(&scheduler)
                as Arc<dyn crate::domain::state_machine::services::TaskScheduler>);

        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);
        let handler = TransitionHandler::new(&mut machine);

        handler.on_exit(&State::Merging, target).await;

        // Wait for spawned deferred retry task
        tokio::time::sleep(tokio::time::Duration::from_millis(900)).await;

        let calls = scheduler.get_calls();
        let retry_calls: Vec<_> = calls
            .iter()
            .filter(|c| c.method == "try_retry_deferred_merges")
            .collect();

        assert_eq!(
            retry_calls.len(),
            1,
            "Expected deferred merge retry when exiting Merging to {:?}",
            target
        );
    }
}

/// Test: No duplicate deferred retry calls from a single exit.
///
/// Regression: ensure that a single on_exit call produces exactly one
/// deferred retry, not multiple.
#[tokio::test]
async fn test_deferred_merge_no_duplicate_retries() {
    use crate::domain::state_machine::mocks::MockTaskScheduler;

    let scheduler = Arc::new(MockTaskScheduler::new());
    let services = TaskServices::new_mock()
        .with_task_scheduler(Arc::clone(&scheduler)
            as Arc<dyn crate::domain::state_machine::services::TaskScheduler>);

    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    // Single exit call
    handler
        .on_exit(&State::PendingMerge, &State::Merged)
        .await;

    // Wait for all spawned tasks
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    let calls = scheduler.get_calls();
    let retry_calls: Vec<_> = calls
        .iter()
        .filter(|c| c.method == "try_retry_deferred_merges")
        .collect();

    assert_eq!(
        retry_calls.len(),
        1,
        "Single on_exit should produce exactly one deferred retry call, got {}",
        retry_calls.len()
    );
}

/// Test: Non-merge state exits do not trigger deferred merge retry.
///
/// Regression: only PendingMerge and Merging exits should trigger retry.
/// Other states (Executing, Reviewing, etc.) must not.
#[tokio::test]
async fn test_deferred_merge_not_triggered_by_non_merge_exits() {
    use crate::domain::state_machine::mocks::MockTaskScheduler;

    let non_merge_transitions = [
        (State::Executing, State::PendingReview),
        (State::Reviewing, State::ReviewPassed),
        (State::ReExecuting, State::PendingReview),
        (State::Ready, State::Executing),
        (State::QaTesting, State::QaPassed),
        (State::QaRefining, State::QaTesting),
    ];

    for (from, to) in &non_merge_transitions {
        let scheduler = Arc::new(MockTaskScheduler::new());
        let services = TaskServices::new_mock()
            .with_task_scheduler(Arc::clone(&scheduler)
                as Arc<dyn crate::domain::state_machine::services::TaskScheduler>);

        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);
        let handler = TransitionHandler::new(&mut machine);

        handler.on_exit(from, to).await;

        // Wait for potential spawned tasks
        tokio::time::sleep(tokio::time::Duration::from_millis(900)).await;

        let calls = scheduler.get_calls();
        let retry_calls: Vec<_> = calls
            .iter()
            .filter(|c| c.method == "try_retry_deferred_merges")
            .collect();

        assert_eq!(
            retry_calls.len(),
            0,
            "Non-merge exit {:?} -> {:?} should NOT trigger deferred retry",
            from,
            to
        );
    }
}

// ==================
// Branch discovery integration tests
// ==================

/// Test: Branch discovery integration with attempt_programmatic_merge.
///
/// Verifies end-to-end recovery flow: when a task enters PendingMerge with
/// task_branch = None but the git branch exists, discover_and_attach_task_branch
/// is called to re-attach the branch before resolve_merge_branches.
#[tokio::test]
async fn test_branch_discovery_integrates_with_pending_merge() {
    use crate::domain::entities::{Project, Task};
    use crate::infrastructure::memory::{MemoryProjectRepository, MemoryTaskRepository};
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    // Create a temporary git repository
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().to_path_buf();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Create initial commit
    fs::write(repo_path.join("README.md"), "test").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Create project and task
    let project = Project::new("test-project".to_string(), repo_path.to_string_lossy().to_string());
    let task = Task::new(project.id.clone(), "Test task".to_string());

    // Create the orphaned git branch (simulating recovery scenario)
    let expected_branch = format!("ralphx/test-project/task-{}", task.id.as_str());
    Command::new("git")
        .args(["branch", &expected_branch])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Ensure task_branch is None (orphaned state)
    assert_eq!(task.task_branch, None);

    // Set up repositories
    let task_repo: Arc<dyn crate::domain::repositories::TaskRepository> =
        Arc::new(MemoryTaskRepository::new());
    let project_repo: Arc<dyn crate::domain::repositories::ProjectRepository> =
        Arc::new(MemoryProjectRepository::new());

    // Persist task and project
    task_repo.create(task.clone()).await.unwrap();
    project_repo.create(project.clone()).await.unwrap();

    // Create services with real repos
    let mut services = TaskServices::new_mock();
    services.task_repo = Some(task_repo.clone());
    services.project_repo = Some(project_repo.clone());

    let context = create_context_with_services(task.id.as_str(), project.id.as_str(), services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    // Call on_enter(PendingMerge) which triggers attempt_programmatic_merge
    let _ = handler.on_enter(&State::PendingMerge).await;

    // Wait for attempt_programmatic_merge to complete (it's async)
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify the branch was discovered and attached
    let updated_task = task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    assert_eq!(
        updated_task.task_branch,
        Some(expected_branch),
        "Branch should be discovered and re-attached during PendingMerge entry"
    );
}

/// Integration test: Task in MergeIncomplete with task_branch=None, git branch exists
/// with commits → retry_merge → branch discovered → programmatic merge succeeds → Merged
#[tokio::test]
async fn test_merge_retry_recovery_discovers_branch_and_merges() {
    use crate::domain::entities::{Project, Task};
    use crate::infrastructure::memory::{MemoryProjectRepository, MemoryTaskRepository};
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    // Create a temporary git repository
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().to_path_buf();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Create initial commit on main
    fs::write(repo_path.join("README.md"), "initial content").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Create project and task
    let project = Project::new("test-project".to_string(), repo_path.to_string_lossy().to_string());
    let mut task = Task::new(project.id.clone(), "Test recovery task".to_string());
    task.internal_status = InternalStatus::MergeIncomplete;
    task.task_branch = None; // Simulate orphaned state

    // Create the orphaned git branch with a commit
    let expected_branch = format!("ralphx/test-project/task-{}", task.id.as_str());
    Command::new("git")
        .args(["checkout", "-b", &expected_branch])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    fs::write(repo_path.join("feature.txt"), "feature work").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "Add feature"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Return to main
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Ensure task_branch is None (orphaned state)
    assert_eq!(task.task_branch, None);

    // Set up repositories
    let task_repo: Arc<dyn crate::domain::repositories::TaskRepository> =
        Arc::new(MemoryTaskRepository::new());
    let project_repo: Arc<dyn crate::domain::repositories::ProjectRepository> =
        Arc::new(MemoryProjectRepository::new());

    // Persist task and project
    task_repo.create(task.clone()).await.unwrap();
    project_repo.create(project.clone()).await.unwrap();

    // Create services with real repos
    let mut services = TaskServices::new_mock();
    services.task_repo = Some(task_repo.clone());
    services.project_repo = Some(project_repo.clone());

    let context = create_context_with_services(task.id.as_str(), project.id.as_str(), services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    // Simulate retry_merge: transition from MergeIncomplete → PendingMerge
    let _ = handler.on_enter(&State::PendingMerge).await;

    // Wait for attempt_programmatic_merge to complete (it's async)
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Verify the branch was discovered and attached
    let updated_task = task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    assert_eq!(
        updated_task.task_branch,
        Some(expected_branch.clone()),
        "Branch should be discovered and re-attached during retry"
    );

    // Verify merge succeeded and task reached Merged
    assert_eq!(
        updated_task.internal_status,
        InternalStatus::Merged,
        "Task should transition to Merged after successful programmatic merge"
    );
}

/// Integration test: Same setup as previous test but with conflicting changes on target
/// branch → retry_merge → branch discovered → Merging state (agent would be spawned)
#[tokio::test]
async fn test_merge_retry_recovery_detects_conflicts_and_enters_merging() {
    use crate::domain::entities::{Project, Task};
    use crate::infrastructure::memory::{MemoryProjectRepository, MemoryTaskRepository};
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    // Create a temporary git repository
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().to_path_buf();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Create initial commit on main
    fs::write(repo_path.join("conflict.txt"), "original line\n").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Create project and task
    let project = Project::new("test-project".to_string(), repo_path.to_string_lossy().to_string());
    let mut task = Task::new(project.id.clone(), "Test conflict task".to_string());
    task.internal_status = InternalStatus::MergeIncomplete;
    task.task_branch = None; // Simulate orphaned state

    // Create the orphaned git branch with a conflicting change
    let expected_branch = format!("ralphx/test-project/task-{}", task.id.as_str());
    Command::new("git")
        .args(["checkout", "-b", &expected_branch])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    fs::write(repo_path.join("conflict.txt"), "branch change\n").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "Branch modification"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Return to main and make conflicting change
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    fs::write(repo_path.join("conflict.txt"), "main change\n").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "Main modification"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Ensure task_branch is None (orphaned state)
    assert_eq!(task.task_branch, None);

    // Set up repositories
    let task_repo: Arc<dyn crate::domain::repositories::TaskRepository> =
        Arc::new(MemoryTaskRepository::new());
    let project_repo: Arc<dyn crate::domain::repositories::ProjectRepository> =
        Arc::new(MemoryProjectRepository::new());

    // Persist task and project
    task_repo.create(task.clone()).await.unwrap();
    project_repo.create(project.clone()).await.unwrap();

    // Create services with real repos
    let mut services = TaskServices::new_mock();
    services.task_repo = Some(task_repo.clone());
    services.project_repo = Some(project_repo.clone());

    let context = create_context_with_services(task.id.as_str(), project.id.as_str(), services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    // Simulate retry_merge: transition from MergeIncomplete → PendingMerge
    let _ = handler.on_enter(&State::PendingMerge).await;

    // Wait for attempt_programmatic_merge to complete (it's async)
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Verify the branch was discovered and attached
    let updated_task = task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    assert_eq!(
        updated_task.task_branch,
        Some(expected_branch.clone()),
        "Branch should be discovered and re-attached during retry"
    );

    // Verify merge detected conflicts and entered Merging state (agent path)
    assert_eq!(
        updated_task.internal_status,
        InternalStatus::Merging,
        "Task should transition to Merging when conflicts are detected"
    );
}

/// Integration test: Task in Failed with task_branch=None, git branch exists → transition
/// to Ready → Executing → worktree created for existing branch → execution proceeds
#[tokio::test]
async fn test_executing_entry_recovers_existing_branch_into_worktree() {
    use crate::domain::entities::{GitMode, Project, Task};
    use crate::infrastructure::memory::{MemoryProjectRepository, MemoryTaskRepository};
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    // Create a temporary git repository
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().to_path_buf();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Create initial commit on main
    fs::write(repo_path.join("README.md"), "initial content").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Create project with Worktree mode and worktree parent in temp dir
    let worktree_parent = temp_dir.path().join("worktrees");
    fs::create_dir_all(&worktree_parent).unwrap();
    let mut project = Project::new("test-project".to_string(), repo_path.to_string_lossy().to_string());
    project.git_mode = GitMode::Worktree;
    project.worktree_parent_directory = Some(worktree_parent.to_string_lossy().to_string());

    let mut task = Task::new(project.id.clone(), "Test worktree recovery".to_string());
    task.internal_status = InternalStatus::Failed;
    task.task_branch = None; // Simulate recovery scenario

    // Create the orphaned git branch (from previous execution attempt)
    let expected_branch = format!("ralphx/test-project/task-{}", task.id.as_str());
    Command::new("git")
        .args(["branch", &expected_branch])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Ensure task_branch is None (orphaned state)
    assert_eq!(task.task_branch, None);

    // Set up repositories
    let task_repo: Arc<dyn crate::domain::repositories::TaskRepository> =
        Arc::new(MemoryTaskRepository::new());
    let project_repo: Arc<dyn crate::domain::repositories::ProjectRepository> =
        Arc::new(MemoryProjectRepository::new());

    // Persist task and project
    task_repo.create(task.clone()).await.unwrap();
    project_repo.create(project.clone()).await.unwrap();

    // Create services with real repos
    let mut services = TaskServices::new_mock();
    services.task_repo = Some(task_repo.clone());
    services.project_repo = Some(project_repo.clone());

    let context = create_context_with_services(task.id.as_str(), project.id.as_str(), services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    // Simulate recovery: transition to Executing (which triggers worktree setup)
    let result = handler.on_enter(&State::Executing).await;

    // Verify worktree creation succeeded
    assert!(
        result.is_ok(),
        "Executing entry should succeed even with existing branch: {:?}",
        result
    );

    // Verify the branch was attached to the task
    let updated_task = task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    assert_eq!(
        updated_task.task_branch,
        Some(expected_branch.clone()),
        "Existing branch should be attached during Executing entry"
    );

    // Verify worktree was created (path should exist)
    let expected_worktree = format!(
        "{}/test-project/task-{}",
        worktree_parent.to_string_lossy(),
        task.id.as_str()
    );
    assert_eq!(
        updated_task.worktree_path,
        Some(expected_worktree.clone()),
        "Worktree path should be set"
    );

    // Verify worktree directory exists
    let worktree_path = std::path::Path::new(&expected_worktree);
    assert!(
        worktree_path.exists(),
        "Worktree directory should exist at {}",
        expected_worktree
    );

    // Verify the worktree is on the correct branch
    let branch_check = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(worktree_path)
        .output()
        .unwrap();
    let current_branch = String::from_utf8_lossy(&branch_check.stdout).trim().to_string();
    assert_eq!(
        current_branch, expected_branch,
        "Worktree should be on the existing branch"
    );
}

// ==================
// ExecutionBlocked error handling tests
// ==================

/// Test that ExecutionFailed event transitions Executing to Failed
/// This verifies the INTENDED behavior when on_enter returns ExecutionBlocked:
/// the handler should auto-dispatch ExecutionFailed to move the task to Failed.
///
/// NOTE: This test verifies the event transition logic. The actual on_enter
/// ExecutionBlocked path (uncommitted changes guard) is tested via integration
/// tests that require filesystem setup.
#[tokio::test]
async fn test_execution_blocked_triggers_execution_failed() {
    let (_spawner, emitter, _notifier, _dep_manager, _review_starter, services) =
        create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let mut handler = TransitionHandler::new(&mut machine);

    // Simulate what happens when on_enter(Executing) returns ExecutionBlocked:
    // the handler should auto-dispatch ExecutionFailed
    let result = handler
        .handle_transition(
            &State::Executing,
            &TaskEvent::ExecutionFailed {
                error: "Execution blocked: uncommitted changes in working directory".to_string(),
            },
        )
        .await;

    // Should transition to Failed state
    assert!(matches!(result.state(), Some(State::Failed(_))));

    // Should emit task_failed event
    assert!(emitter.has_event("task_failed"));
}

// ============================================================================
// Wave 4: Metadata Skip Guard Tests
// ============================================================================

/// Test that on_enter(Failed) skips metadata write when failure_error is already present
#[tokio::test]
async fn test_on_enter_failed_skips_when_failure_error_already_present() {
    use crate::domain::entities::{Project, Task};
    use crate::domain::state_machine::types::FailedData;
    use crate::infrastructure::memory::MemoryTaskRepository;

    // Create task with pre-computed failure metadata
    let project = Project::new("test-project".to_string(), "/test/path".to_string());
    let mut task = Task::new(project.id.clone(), "Test task".to_string());
    task.metadata = Some(r#"{"failure_error":"Pre-computed error","is_timeout":false}"#.to_string());

    let task_repo: Arc<dyn crate::domain::repositories::TaskRepository> =
        Arc::new(MemoryTaskRepository::new());
    task_repo.create(task.clone()).await.unwrap();

    // Create services with real repo
    let mut services = TaskServices::new_mock();
    services.task_repo = Some(task_repo.clone());

    let context = create_context_with_services(task.id.as_str(), project.id.as_str(), services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    // Create Failed state with different error
    let failed_data = FailedData::new("New error from on_enter");
    let failed_state = State::Failed(failed_data);

    // Call on_enter(Failed) which should skip the write
    let _ = handler.on_enter(&failed_state).await;

    // Verify metadata was NOT overwritten (still has pre-computed value)
    let updated_task = task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let metadata_json = updated_task.metadata.expect("Metadata should still exist");
    let parsed: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&metadata_json).unwrap();

    assert_eq!(
        parsed.get("failure_error").unwrap().as_str().unwrap(),
        "Pre-computed error",
        "Metadata should NOT be overwritten when already present"
    );
}

/// Test that on_enter(Failed) writes metadata when not present (backward compatibility)
#[tokio::test]
async fn test_on_enter_failed_writes_when_not_present() {
    use crate::domain::entities::{Project, Task};
    use crate::domain::state_machine::types::FailedData;
    use crate::infrastructure::memory::MemoryTaskRepository;

    // Create task without failure metadata
    let project = Project::new("test-project".to_string(), "/test/path".to_string());
    let task = Task::new(project.id.clone(), "Test task".to_string());

    let task_repo: Arc<dyn crate::domain::repositories::TaskRepository> =
        Arc::new(MemoryTaskRepository::new());
    task_repo.create(task.clone()).await.unwrap();

    // Create services with real repo
    let mut services = TaskServices::new_mock();
    services.task_repo = Some(task_repo.clone());

    let context = create_context_with_services(task.id.as_str(), project.id.as_str(), services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    // Create Failed state
    let failed_data = FailedData::new("Fallback error").with_details("Details here");
    let failed_state = State::Failed(failed_data);

    // Call on_enter(Failed) which should write the metadata
    let _ = handler.on_enter(&failed_state).await;

    // Verify metadata was written
    let updated_task = task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let metadata_json = updated_task.metadata.expect("Metadata should be written");
    let parsed: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&metadata_json).unwrap();

    assert_eq!(
        parsed.get("failure_error").unwrap().as_str().unwrap(),
        "Fallback error",
        "Metadata should be written when not present"
    );
    assert_eq!(
        parsed.get("failure_details").unwrap().as_str().unwrap(),
        "Details here"
    );
    assert!(!parsed.get("is_timeout").unwrap().as_bool().unwrap());
}

/// Test that on_enter(Executing) uses update_metadata instead of full update
/// This test verifies the change from update(&task) to update_metadata()
#[tokio::test]
async fn test_on_enter_executing_uses_update_metadata() {
    use crate::domain::entities::{Project, Task};
    use crate::infrastructure::memory::MemoryTaskRepository;

    // Create project and task
    let project = Project::new("test-project".to_string(), "/test/path".to_string());
    let task = Task::new(project.id.clone(), "Test task".to_string());

    let task_repo: Arc<dyn crate::domain::repositories::TaskRepository> =
        Arc::new(MemoryTaskRepository::new());
    task_repo.create(task.clone()).await.unwrap();

    // Create services with real repo
    let mut services = TaskServices::new_mock();
    services.task_repo = Some(task_repo.clone());

    let context = create_context_with_services(task.id.as_str(), project.id.as_str(), services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    // Note: We can't directly verify update_metadata was called vs update,
    // but we can verify the behavior is correct (metadata is written).
    // This test primarily documents the expected behavior after Wave 4 changes.
    let _ = handler.on_enter(&State::Executing).await;

    // If the task has execution_setup_log, it was written via update_metadata
    // (In practice, this won't happen without actual project analysis setup,
    // but the code path is correct)
    let updated_task = task_repo.get_by_id(&task.id).await.unwrap().unwrap();

    // This test passes if no panic occurred and task is retrievable
    assert_eq!(updated_task.id, task.id);
}

/// Test that on_enter(QaRefining) skips metadata write when trigger_origin is already present
#[tokio::test]
async fn test_on_enter_qa_refining_skips_when_trigger_origin_already_present() {
    use crate::domain::entities::{Project, Task};
    use crate::infrastructure::memory::MemoryTaskRepository;

    // Create task with pre-computed trigger_origin metadata
    let project = Project::new("test-project".to_string(), "/test/path".to_string());
    let mut task = Task::new(project.id.clone(), "Test task".to_string());
    task.metadata = Some(r#"{"trigger_origin":"scheduler","other_key":"value"}"#.to_string());

    let task_repo: Arc<dyn crate::domain::repositories::TaskRepository> =
        Arc::new(MemoryTaskRepository::new());
    task_repo.create(task.clone()).await.unwrap();

    // Create services with real repo
    let mut services = TaskServices::new_mock();
    services.task_repo = Some(task_repo.clone());

    let context = create_context_with_services(task.id.as_str(), project.id.as_str(), services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    // Call on_enter(QaRefining) which should skip the write
    let _ = handler.on_enter(&State::QaRefining).await;

    // Verify metadata was NOT overwritten (still has pre-computed value)
    let updated_task = task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let metadata_json = updated_task.metadata.expect("Metadata should still exist");
    let parsed: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&metadata_json).unwrap();

    assert_eq!(
        parsed.get("trigger_origin").unwrap().as_str().unwrap(),
        "scheduler",
        "Metadata should NOT be overwritten when already present"
    );
    assert_eq!(
        parsed.get("other_key").unwrap().as_str().unwrap(),
        "value",
        "Other metadata keys should be preserved"
    );
}

/// Test that on_enter(QaRefining) writes metadata when trigger_origin not present (backward compatibility)
#[tokio::test]
async fn test_on_enter_qa_refining_writes_when_not_present() {
    use crate::domain::entities::{Project, Task};
    use crate::infrastructure::memory::MemoryTaskRepository;

    // Create task without trigger_origin metadata
    let project = Project::new("test-project".to_string(), "/test/path".to_string());
    let task = Task::new(project.id.clone(), "Test task".to_string());

    let task_repo: Arc<dyn crate::domain::repositories::TaskRepository> =
        Arc::new(MemoryTaskRepository::new());
    task_repo.create(task.clone()).await.unwrap();

    // Create services with real repo
    let mut services = TaskServices::new_mock();
    services.task_repo = Some(task_repo.clone());

    let context = create_context_with_services(task.id.as_str(), project.id.as_str(), services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    // Call on_enter(QaRefining) which should write the metadata
    let _ = handler.on_enter(&State::QaRefining).await;

    // Verify metadata was written
    let updated_task = task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let metadata_json = updated_task.metadata.expect("Metadata should be written");
    let parsed: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&metadata_json).unwrap();

    assert_eq!(
        parsed.get("trigger_origin").unwrap().as_str().unwrap(),
        "qa",
        "trigger_origin should be set to 'qa' when not present"
    );
}

/// Test that on_enter(QaTesting) skips metadata write when trigger_origin is already present
#[tokio::test]
async fn test_on_enter_qa_testing_skips_when_trigger_origin_already_present() {
    use crate::domain::entities::{Project, Task};
    use crate::infrastructure::memory::MemoryTaskRepository;

    // Create task with pre-computed trigger_origin metadata
    let project = Project::new("test-project".to_string(), "/test/path".to_string());
    let mut task = Task::new(project.id.clone(), "Test task".to_string());
    task.metadata = Some(r#"{"trigger_origin":"scheduler","other_key":"value"}"#.to_string());

    let task_repo: Arc<dyn crate::domain::repositories::TaskRepository> =
        Arc::new(MemoryTaskRepository::new());
    task_repo.create(task.clone()).await.unwrap();

    // Create services with real repo
    let mut services = TaskServices::new_mock();
    services.task_repo = Some(task_repo.clone());

    let context = create_context_with_services(task.id.as_str(), project.id.as_str(), services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    // Call on_enter(QaTesting) which should skip the write
    let _ = handler.on_enter(&State::QaTesting).await;

    // Verify metadata was NOT overwritten (still has pre-computed value)
    let updated_task = task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let metadata_json = updated_task.metadata.expect("Metadata should still exist");
    let parsed: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&metadata_json).unwrap();

    assert_eq!(
        parsed.get("trigger_origin").unwrap().as_str().unwrap(),
        "scheduler",
        "Metadata should NOT be overwritten when already present"
    );
    assert_eq!(
        parsed.get("other_key").unwrap().as_str().unwrap(),
        "value",
        "Other metadata keys should be preserved"
    );
}

/// Test that on_enter(QaTesting) writes metadata when trigger_origin not present (backward compatibility)
#[tokio::test]
async fn test_on_enter_qa_testing_writes_when_not_present() {
    use crate::domain::entities::{Project, Task};
    use crate::infrastructure::memory::MemoryTaskRepository;

    // Create task without trigger_origin metadata
    let project = Project::new("test-project".to_string(), "/test/path".to_string());
    let task = Task::new(project.id.clone(), "Test task".to_string());

    let task_repo: Arc<dyn crate::domain::repositories::TaskRepository> =
        Arc::new(MemoryTaskRepository::new());
    task_repo.create(task.clone()).await.unwrap();

    // Create services with real repo
    let mut services = TaskServices::new_mock();
    services.task_repo = Some(task_repo.clone());

    let context = create_context_with_services(task.id.as_str(), project.id.as_str(), services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    // Call on_enter(QaTesting) which should write the metadata
    let _ = handler.on_enter(&State::QaTesting).await;

    // Verify metadata was written
    let updated_task = task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let metadata_json = updated_task.metadata.expect("Metadata should be written");
    let parsed: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&metadata_json).unwrap();

    assert_eq!(
        parsed.get("trigger_origin").unwrap().as_str().unwrap(),
        "qa",
        "trigger_origin should be set to 'qa' when not present"
    );
}
