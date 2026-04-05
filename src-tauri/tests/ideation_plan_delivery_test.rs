// Integration tests for the ideation plan delivery flow.
//
// Verifies that `plan:delivered` external event fires correctly when all tasks
// in an ideation session reach `Merged` status (via the PlanMerge task's
// `on_enter(Merged)` handler path).
//
// Tests:
//   1. Happy path: all 3 tasks Merged → plan:delivered fires once, delivery_status=delivered
//   2. Partial merge guard: workers NOT yet Merged → no plan:delivered
//   3. Idempotency: on_enter(Merged) called twice → still exactly 1 plan:delivered

use std::sync::Arc;

use async_trait::async_trait;
use axum::extract::{Path, Query, State};
use axum::Json;
use dashmap::DashMap;
use ralphx_domain::entities::EventType;
use ralphx_lib::application::{AppState, TeamService, TeamStateTracker};
use ralphx_lib::commands::ExecutionState;
use ralphx_lib::domain::entities::{
    IdeationSessionBuilder, IdeationSessionId, InternalStatus, ProjectId, Task, TaskCategory,
};
use ralphx_lib::domain::repositories::{
    ExternalEventsRepository, IdeationSessionRepository, ProjectRepository, TaskRepository,
};
use ralphx_lib::domain::state_machine::{
    State as MachineState, TaskContext, TaskServices, TaskStateMachine, TransitionHandler,
};
use ralphx_lib::domain::state_machine::services::WebhookPublisher as WebhookPublisherTrait;
use ralphx_lib::http_server::handlers::{
    get_session_tasks_http, GetSessionTasksParams,
};
use ralphx_lib::http_server::project_scope::ProjectScope;
use ralphx_lib::http_server::types::HttpServerState;
use ralphx_lib::infrastructure::memory::{
    MemoryExternalEventsRepository, MemoryIdeationSessionRepository, MemoryProjectRepository,
    MemoryTaskRepository,
};
use std::sync::Mutex;
use tokio::sync::Mutex as TokioMutex;

// ============================================================================
// RecordingWebhookPublisher — captures publish() calls for assertion
// ============================================================================

#[derive(Default)]
struct RecordingWebhookPublisher {
    calls: Mutex<Vec<EventType>>,
}

impl RecordingWebhookPublisher {
    fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
        }
    }

    fn was_called_with(&self, event_type: EventType) -> bool {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .any(|et| *et == event_type)
    }

    fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}

#[async_trait]
impl WebhookPublisherTrait for RecordingWebhookPublisher {
    async fn publish(
        &self,
        event_type: EventType,
        _project_id: &str,
        _payload: serde_json::Value,
    ) {
        self.calls.lock().unwrap().push(event_type);
    }
}

// ============================================================================
// TestSetup — holds all shared repos and IDs for a test run
// ============================================================================

struct TestSetup {
    project_id: String,
    session_id: String,
    task_repo: Arc<MemoryTaskRepository>,
    project_repo: Arc<MemoryProjectRepository>,
    events_repo: Arc<MemoryExternalEventsRepository>,
    session_repo: Arc<MemoryIdeationSessionRepository>,
    webhook_publisher: Arc<RecordingWebhookPublisher>,
    session_merge_locks: Arc<DashMap<String, Arc<TokioMutex<()>>>>,
    plan_merge_task_id: String,
}

impl TestSetup {
    /// All 3 tasks (2 workers + 1 PlanMerge) are in Merged state.
    async fn new_all_merged() -> Self {
        Self::build(InternalStatus::Merged).await
    }

    /// Workers are in Backlog; only the PlanMerge task is set to Merged.
    async fn new_partial_merge() -> Self {
        Self::build(InternalStatus::Backlog).await
    }

    async fn build(worker_status: InternalStatus) -> Self {
        let project_id = "proj-plan-delivery-test".to_string();

        // Build a session and get its ID
        let session = IdeationSessionBuilder::new()
            .project_id(ProjectId::from_string(project_id.clone()))
            .build();
        let session_id = session.id.as_str().to_string();

        let task_repo = Arc::new(MemoryTaskRepository::new());
        let project_repo = Arc::new(MemoryProjectRepository::new());
        let events_repo = Arc::new(MemoryExternalEventsRepository::new());
        let session_repo = Arc::new(MemoryIdeationSessionRepository::new());
        let webhook_publisher = Arc::new(RecordingWebhookPublisher::new());
        let session_merge_locks = Arc::new(DashMap::new());

        // Persist the session so the HTTP handler can load it
        session_repo.create(session).await.expect("create session");

        // Worker task 1
        let mut worker1 = Task::new(
            ProjectId::from_string(project_id.clone()),
            "Worker task 1".to_string(),
        );
        worker1.ideation_session_id =
            Some(IdeationSessionId::from_string(session_id.clone()));
        worker1.internal_status = worker_status.clone();
        task_repo.create(worker1).await.expect("create worker1");

        // Worker task 2
        let mut worker2 = Task::new(
            ProjectId::from_string(project_id.clone()),
            "Worker task 2".to_string(),
        );
        worker2.ideation_session_id =
            Some(IdeationSessionId::from_string(session_id.clone()));
        worker2.internal_status = worker_status;
        task_repo.create(worker2).await.expect("create worker2");

        // PlanMerge task — category=PlanMerge, always Merged (it just entered this state)
        let mut plan_merge = Task::new(
            ProjectId::from_string(project_id.clone()),
            "Plan merge task".to_string(),
        );
        plan_merge.ideation_session_id =
            Some(IdeationSessionId::from_string(session_id.clone()));
        plan_merge.category = TaskCategory::PlanMerge;
        plan_merge.internal_status = InternalStatus::Merged;
        let plan_merge_task_id = plan_merge.id.as_str().to_string();
        task_repo.create(plan_merge).await.expect("create plan_merge");

        Self {
            project_id,
            session_id,
            task_repo,
            project_repo,
            events_repo,
            session_repo,
            webhook_publisher,
            session_merge_locks,
            plan_merge_task_id,
        }
    }

    /// Build TaskServices wired with this test's repos.
    fn build_task_services(&self) -> TaskServices {
        TaskServices::new_mock()
            .with_task_repo(
                Arc::clone(&self.task_repo) as Arc<dyn TaskRepository>,
            )
            .with_project_repo(
                Arc::clone(&self.project_repo) as Arc<dyn ProjectRepository>,
            )
            .with_external_events_repo(
                Arc::clone(&self.events_repo) as Arc<dyn ExternalEventsRepository>,
            )
            .with_webhook_publisher(
                Arc::clone(&self.webhook_publisher) as Arc<dyn WebhookPublisherTrait>,
            )
            .with_session_merge_locks(Arc::clone(&self.session_merge_locks))
    }
}

// ============================================================================
// Test helpers
// ============================================================================

/// Trigger `on_enter(Merged)` for the PlanMerge task using the setup's repos.
async fn call_on_enter_merged(setup: &TestSetup) {
    let services = setup.build_task_services();
    let context = TaskContext::new(&setup.plan_merge_task_id, &setup.project_id, services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);
    handler
        .on_enter(&MachineState::Merged)
        .await
        .expect("on_enter(Merged) should not return an error");
}

/// Count `plan:delivered` events for this test's project in the events repo.
async fn count_plan_delivered(setup: &TestSetup) -> usize {
    setup
        .events_repo
        .get_events_after_cursor(&[setup.project_id.clone()], 0, 1000)
        .await
        .expect("get_events_after_cursor")
        .into_iter()
        .filter(|e| e.event_type == "plan:delivered")
        .count()
}

/// Build an HttpServerState backed by this setup's task and session repos.
fn build_http_state(setup: &TestSetup) -> HttpServerState {
    let mut app_state = AppState::new_test();
    app_state.task_repo = Arc::clone(&setup.task_repo) as Arc<dyn TaskRepository>;
    app_state.ideation_session_repo =
        Arc::clone(&setup.session_repo) as Arc<dyn IdeationSessionRepository>;
    let app_state = Arc::new(app_state);
    let execution_state = Arc::new(ExecutionState::new());
    let tracker = TeamStateTracker::new();
    let team_service = Arc::new(TeamService::new_without_events(Arc::new(tracker.clone())));
    HttpServerState {
        app_state,
        execution_state,
        team_tracker: tracker,
        team_service,
    }
}

// ============================================================================
// Test 1 — Happy path: all tasks merged → plan:delivered fires once
// ============================================================================

#[tokio::test]
async fn test_plan_delivered_fires_after_all_tasks_merged() {
    let setup = TestSetup::new_all_merged().await;

    // Trigger: PlanMerge task enters Merged state (all workers already Merged)
    call_on_enter_merged(&setup).await;

    // Assert: exactly 1 plan:delivered event in external_events
    let count = count_plan_delivered(&setup).await;
    assert_eq!(count, 1, "Expected exactly one plan:delivered event");

    // Assert: the event payload contains the correct session_id
    let events = setup
        .events_repo
        .get_events_after_cursor(&[setup.project_id.clone()], 0, 1000)
        .await
        .unwrap();
    let delivered_event = events
        .iter()
        .find(|e| e.event_type == "plan:delivered")
        .expect("plan:delivered event must exist");
    assert!(
        delivered_event.payload.contains(&setup.session_id),
        "plan:delivered payload must contain session_id"
    );

    // Assert: webhook publisher was called with PlanDelivered
    assert!(
        setup
            .webhook_publisher
            .was_called_with(EventType::PlanDelivered),
        "webhook publisher should have received PlanDelivered"
    );
    assert_eq!(
        setup.webhook_publisher.call_count(),
        1,
        "webhook publisher should be called exactly once"
    );

    // Assert: delivery_status=delivered via the session tasks HTTP handler
    let http_state = build_http_state(&setup);
    let result = get_session_tasks_http(
        State(http_state),
        ProjectScope(None),
        Path(setup.session_id.clone()),
        Query(GetSessionTasksParams { changed_since: None }),
    )
    .await
    .expect("get_session_tasks_http should succeed");
    let Json(body) = result;
    assert_eq!(
        body.delivery_status, "delivered",
        "delivery_status must be 'delivered' when all tasks are Merged"
    );
    assert_eq!(body.task_count, 3, "session should have 3 tasks");
}

// ============================================================================
// Test 2 — Partial merge guard: workers NOT yet Merged → no plan:delivered
// ============================================================================

#[tokio::test]
async fn test_plan_delivered_not_fired_on_partial_merge() {
    let setup = TestSetup::new_partial_merge().await;

    // Trigger: PlanMerge task enters Merged, but workers are still in Backlog
    call_on_enter_merged(&setup).await;

    // Assert: no plan:delivered event
    let count = count_plan_delivered(&setup).await;
    assert_eq!(
        count, 0,
        "plan:delivered must NOT fire when worker tasks are not yet Merged"
    );

    // Assert: webhook publisher was not called
    assert_eq!(
        setup.webhook_publisher.call_count(),
        0,
        "webhook publisher must not be called on partial merge"
    );
}

// ============================================================================
// Test 3 — Idempotency: calling on_enter(Merged) twice → only 1 plan:delivered
// ============================================================================

#[tokio::test]
async fn test_plan_delivered_idempotent() {
    let setup = TestSetup::new_all_merged().await;

    // First trigger: inserts the plan:delivered event
    call_on_enter_merged(&setup).await;

    // Second trigger: simulates duplicate / concurrent call (e.g. reconciler re-fires)
    // Must be blocked by the idempotency guard (event_exists check)
    call_on_enter_merged(&setup).await;

    // Assert: still exactly 1 plan:delivered — not 2
    let count = count_plan_delivered(&setup).await;
    assert_eq!(
        count, 1,
        "plan:delivered must fire only once even if on_enter(Merged) is called twice"
    );

    // Assert: webhook publisher was called exactly once (not twice)
    assert_eq!(
        setup.webhook_publisher.call_count(),
        1,
        "webhook publisher must be called only once (idempotency guard)"
    );
}
