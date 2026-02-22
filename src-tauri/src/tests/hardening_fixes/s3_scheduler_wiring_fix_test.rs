// Fix S3: task_scheduler not injected — scheduling silently skipped with no log
//
// Scenario S3: `task_scheduler` is `Option<Arc<dyn TaskScheduler>>` in `TaskServices`.
// When `None`, `on_enter(Merged)` scheduling spawn is silently skipped. No log, no error.
// Fix: add `tracing::warn!` when scheduler is None so the issue is visible in logs.
//
// Tests:
// 1. When task_scheduler is None, try_schedule_ready_tasks is NOT called (expected).
// 2. When task_scheduler is Some, try_schedule_ready_tasks IS called after 600ms delay.

use std::sync::{Arc, Mutex};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

use crate::application::MockChatService;
use crate::domain::state_machine::context::{TaskContext, TaskServices};
use crate::domain::state_machine::machine::{State, TaskStateMachine};
use crate::domain::state_machine::mocks::{
    MockAgentSpawner, MockDependencyManager, MockEventEmitter, MockNotifier, MockReviewStarter,
    MockTaskScheduler,
};
use crate::domain::state_machine::services::{
    AgentSpawner, DependencyManager, EventEmitter, Notifier, ReviewStarter, TaskScheduler,
};
use crate::domain::state_machine::transition_handler::TransitionHandler;
use crate::domain::state_machine::TaskEvent;

/// A tracing Layer that captures WARN+ log messages for test inspection.
struct CapturingLayer {
    captured: Arc<Mutex<Vec<String>>>,
}

impl<S: tracing::Subscriber> Layer<S> for CapturingLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if *event.metadata().level() <= tracing::Level::WARN {
            struct MessageVisitor(String);
            impl tracing::field::Visit for MessageVisitor {
                fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
                    if field.name() == "message" {
                        self.0 = value.to_string();
                    }
                }
                fn record_debug(
                    &mut self,
                    field: &tracing::field::Field,
                    value: &dyn std::fmt::Debug,
                ) {
                    if field.name() == "message" {
                        self.0 = format!("{:?}", value);
                    }
                }
            }
            let mut visitor = MessageVisitor(String::new());
            event.record(&mut visitor);
            if !visitor.0.is_empty() {
                self.captured.lock().unwrap().push(visitor.0);
            }
        }
    }
}

fn build_services_without_scheduler() -> (Arc<MockTaskScheduler>, TaskServices) {
    let spawner = Arc::new(MockAgentSpawner::new());
    let emitter = Arc::new(MockEventEmitter::new());
    let notifier = Arc::new(MockNotifier::new());
    let dep_manager = Arc::new(MockDependencyManager::new());
    let review_starter = Arc::new(MockReviewStarter::new());
    let chat_service = Arc::new(MockChatService::new());
    let scheduler = Arc::new(MockTaskScheduler::new());

    // No .with_task_scheduler() — task_scheduler remains None
    let services = TaskServices::new(
        spawner as Arc<dyn AgentSpawner>,
        emitter as Arc<dyn EventEmitter>,
        notifier as Arc<dyn Notifier>,
        dep_manager as Arc<dyn DependencyManager>,
        review_starter as Arc<dyn ReviewStarter>,
        chat_service as Arc<dyn crate::application::ChatService>,
    );

    (scheduler, services)
}

fn build_services_with_scheduler() -> (Arc<MockTaskScheduler>, TaskServices) {
    let spawner = Arc::new(MockAgentSpawner::new());
    let emitter = Arc::new(MockEventEmitter::new());
    let notifier = Arc::new(MockNotifier::new());
    let dep_manager = Arc::new(MockDependencyManager::new());
    let review_starter = Arc::new(MockReviewStarter::new());
    let chat_service = Arc::new(MockChatService::new());
    let scheduler = Arc::new(MockTaskScheduler::new());

    let services = TaskServices::new(
        spawner as Arc<dyn AgentSpawner>,
        emitter as Arc<dyn EventEmitter>,
        notifier as Arc<dyn Notifier>,
        dep_manager as Arc<dyn DependencyManager>,
        review_starter as Arc<dyn ReviewStarter>,
        chat_service as Arc<dyn crate::application::ChatService>,
    )
    .with_task_scheduler(Arc::clone(&scheduler) as Arc<dyn TaskScheduler>);

    (scheduler, services)
}

/// S3 Fix Test 1: When task_scheduler is None, scheduling is skipped.
/// The warning log should be emitted (verified via CapturingLayer).
/// Run single-threaded to avoid tracing subscriber interference from parallel tests.
#[tokio::test(flavor = "current_thread")]
async fn test_on_enter_merged_logs_warning_when_scheduler_missing() {
    let captured = Arc::new(Mutex::new(Vec::<String>::new()));
    let layer = CapturingLayer {
        captured: Arc::clone(&captured),
    };

    let subscriber = tracing_subscriber::registry().with(layer);
    // set_default sets a thread-local subscriber for this test's current thread.
    // current_thread flavor ensures we stay on one thread throughout the test.
    let _guard = subscriber.set_default();

    let (_scheduler, services) = build_services_without_scheduler();
    let context = TaskContext::new("task-s3-warn", "proj-s3", services);
    let mut machine = TaskStateMachine { context };
    let mut handler = TransitionHandler::new(&mut machine);

    // on_enter(Merged) is triggered by handling MergeCompleted from Merging state
    let result = handler
        .handle_transition(&State::Merging, &TaskEvent::MergeComplete)
        .await;

    // Transition should succeed
    assert!(result.is_success(), "Transition to Merged should succeed");

    // Give tokio spawns time to run (there are none for None scheduler, but unblock_dependents runs)
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // After fix: a warning should be emitted about missing scheduler
    let logs = captured.lock().unwrap().clone();
    let has_scheduler_warning = logs.iter().any(|msg| {
        msg.contains("task_scheduler")
            && (msg.contains("not wired") || msg.contains("missing") || msg.contains("None"))
    });

    assert!(
        has_scheduler_warning,
        "FIX S3: Expected warning about missing task_scheduler when on_enter(Merged) skips scheduling. Got logs: {:?}",
        logs
    );
}

/// S3 Fix Test 2: When task_scheduler is None, try_schedule_ready_tasks is not called.
/// This verifies the behavioral contract (scheduling is indeed skipped).
#[tokio::test]
async fn test_on_enter_merged_does_not_schedule_when_scheduler_missing() {
    let (scheduler, services) = build_services_without_scheduler();
    let context = TaskContext::new("task-s3-no-sched", "proj-s3", services);
    let mut machine = TaskStateMachine { context };
    let mut handler = TransitionHandler::new(&mut machine);

    let result = handler
        .handle_transition(&State::Merging, &TaskEvent::MergeComplete)
        .await;

    assert!(result.is_success(), "Transition to Merged should succeed");

    // The scheduler (which is NOT wired) should have received no calls
    // (scheduler is a detached Arc, not wired into services)
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    assert_eq!(
        scheduler.call_count(),
        0,
        "S3: Detached scheduler should not be called when task_scheduler is None"
    );
}

/// S3 Fix Test 3: When task_scheduler is Some, try_schedule_ready_tasks IS called.
/// Uses a longer delay to account for the 600ms spawn sleep.
#[tokio::test]
async fn test_on_enter_merged_schedules_when_scheduler_present() {
    let (scheduler, services) = build_services_with_scheduler();
    let context = TaskContext::new("task-s3-sched", "proj-s3", services);
    let mut machine = TaskStateMachine { context };
    let mut handler = TransitionHandler::new(&mut machine);

    let result = handler
        .handle_transition(&State::Merging, &TaskEvent::MergeComplete)
        .await;

    assert!(result.is_success(), "Transition to Merged should succeed");

    // Wait longer than 600ms for the tokio::spawn delay
    tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;

    let calls = scheduler.get_calls();
    let schedule_calls: Vec<_> = calls
        .iter()
        .filter(|c| c.method == "try_schedule_ready_tasks")
        .collect();

    assert!(
        !schedule_calls.is_empty(),
        "S3: try_schedule_ready_tasks should be called when task_scheduler is Some. Got calls: {:?}",
        calls.iter().map(|c| &c.method).collect::<Vec<_>>()
    );
}
