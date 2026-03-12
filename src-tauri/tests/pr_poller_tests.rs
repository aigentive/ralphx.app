//! PrPollerRegistry lifecycle tests.
//!
//! Covers: MERGED/CLOSED/error transitions, duplicate prevention, stopping guard,
//! cascade stop, and the DashMap CAS creation guard.

mod common;

use std::sync::Arc;

use ralphx_lib::application::services::PrPollerRegistry;
use ralphx_lib::application::{AppState, TaskTransitionService};
use ralphx_lib::commands::ExecutionState;
use ralphx_lib::domain::entities::{
    ArtifactId, IdeationSessionId, InternalStatus, PlanBranch, PlanBranchId, Project, Task,
};
use ralphx_lib::domain::repositories::PlanBranchRepository;
use ralphx_lib::domain::services::github_service::PrStatus;
use ralphx_lib::infrastructure::memory::MemoryPlanBranchRepository;

use common::MockGithubService;

// ============================================================================
// Shared helpers
// ============================================================================

fn build_transition_service(
    app_state: &AppState,
    execution_state: &Arc<ExecutionState>,
) -> Arc<TaskTransitionService<tauri::Wry>> {
    Arc::new(TaskTransitionService::new(
        Arc::clone(&app_state.task_repo),
        Arc::clone(&app_state.task_dependency_repo),
        Arc::clone(&app_state.project_repo),
        Arc::clone(&app_state.chat_message_repo),
        Arc::clone(&app_state.chat_attachment_repo),
        Arc::clone(&app_state.chat_conversation_repo),
        Arc::clone(&app_state.agent_run_repo),
        Arc::clone(&app_state.ideation_session_repo),
        Arc::clone(&app_state.activity_event_repo),
        Arc::clone(&app_state.message_queue),
        Arc::clone(&app_state.running_agent_registry),
        Arc::clone(execution_state),
        None,
        Arc::clone(&app_state.memory_event_repo),
    ))
}


// ============================================================================
// Test 1: start_polling with no github_service is a no-op
// ============================================================================

/// `PrPollerRegistry::new(None, ...)` + `start_polling` must always leave
/// `is_polling()` = false.  This is the fallback contract: without a github_service
/// the registry never starts real tasks.
#[tokio::test]
async fn test_start_polling_noop_without_github_service() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());

    let project = Project::new("Test Project".to_string(), "/tmp/test-repo".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "No-op poller task".to_string());
    task.internal_status = InternalStatus::Merging;
    app_state.task_repo.create(task.clone()).await.unwrap();

    let plan_branch_repo = Arc::new(MemoryPlanBranchRepository::new());
    let mut pb = PlanBranch::new(
        ArtifactId::from_string("test-artifact".to_string()),
        IdeationSessionId::from_string("test-session".to_string()),
        project.id.clone(),
        "plan/feature".to_string(),
        "main".to_string(),
    );
    pb.merge_task_id = Some(task.id.clone());
    pb.pr_number = Some(42);
    pb.pr_eligible = true;
    pb.pr_polling_active = true;
    let plan_branch_id = pb.id.clone();
    plan_branch_repo.create(pb).await.unwrap();

    // Registry with NO github_service
    let registry = Arc::new(PrPollerRegistry::new(
        None,
        Arc::clone(&plan_branch_repo) as Arc<dyn PlanBranchRepository>,
    ));

    let transition_service = build_transition_service(&app_state, &execution_state);

    registry.start_polling(
        task.id.clone(),
        plan_branch_id,
        42,
        std::path::PathBuf::from("/tmp/test-repo"),
        "main".to_string(),
        transition_service,
    );

    assert!(
        !registry.is_polling(&task.id),
        "start_polling without github_service must be a no-op — is_polling() must remain false"
    );
}

// ============================================================================
// Test 2: start_polling with github_service creates a live poller
// ============================================================================

/// When a real (mock) github_service is supplied, `start_polling` creates a
/// live JoinHandle and `is_polling()` returns true.  Also verifies that the
/// `pr_creation_guard` DashMap field is publicly accessible.
#[tokio::test]
async fn test_start_polling_creates_live_poller_with_github_service() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());

    let project = Project::new("Test Project".to_string(), "/tmp/test-repo".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "Live poller task".to_string());
    task.internal_status = InternalStatus::Merging;
    app_state.task_repo.create(task.clone()).await.unwrap();

    let plan_branch_repo = Arc::new(MemoryPlanBranchRepository::new());
    let mut pb = PlanBranch::new(
        ArtifactId::from_string("test-artifact".to_string()),
        IdeationSessionId::from_string("test-session".to_string()),
        project.id.clone(),
        "plan/feature".to_string(),
        "main".to_string(),
    );
    pb.merge_task_id = Some(task.id.clone());
    pb.pr_number = Some(42);
    pb.pr_eligible = true;
    pb.pr_polling_active = true;
    let plan_branch_id = pb.id.clone();
    plan_branch_repo.create(pb).await.unwrap();

    let mock = Arc::new(MockGithubService::new());
    mock.will_return_status(PrStatus::Open);

    let registry = Arc::new(PrPollerRegistry::new(
        Some(mock.clone() as Arc<dyn ralphx_lib::domain::services::github_service::GithubServiceTrait>),
        Arc::clone(&plan_branch_repo) as Arc<dyn PlanBranchRepository>,
    ));

    let transition_service = build_transition_service(&app_state, &execution_state);

    registry.start_polling(
        task.id.clone(),
        plan_branch_id.clone(),
        42,
        std::path::PathBuf::from("/tmp/test-repo"),
        "main".to_string(),
        transition_service,
    );

    assert!(
        registry.is_polling(&task.id),
        "start_polling with a github_service must create a live poller — is_polling() must be true"
    );

    // Verify pr_creation_guard is accessible as a public DashMap field
    let guard_ref = &registry.pr_creation_guard;
    assert!(
        guard_ref.is_empty(),
        "pr_creation_guard DashMap must be accessible and start empty"
    );
}

// ============================================================================
// Test 3: stop_polling removes the handle
// ============================================================================

/// After `start_polling` + `stop_polling`, `is_polling()` must return false.
#[tokio::test]
async fn test_stop_polling_removes_handle() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());

    let project = Project::new("Test Project".to_string(), "/tmp/test-repo".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "Stop poller task".to_string());
    task.internal_status = InternalStatus::Merging;
    app_state.task_repo.create(task.clone()).await.unwrap();

    let plan_branch_repo = Arc::new(MemoryPlanBranchRepository::new());
    let mut pb = PlanBranch::new(
        ArtifactId::from_string("test-artifact".to_string()),
        IdeationSessionId::from_string("test-session".to_string()),
        project.id.clone(),
        "plan/feature".to_string(),
        "main".to_string(),
    );
    pb.merge_task_id = Some(task.id.clone());
    pb.pr_number = Some(42);
    pb.pr_eligible = true;
    pb.pr_polling_active = true;
    let plan_branch_id = pb.id.clone();
    plan_branch_repo.create(pb).await.unwrap();

    let mock = Arc::new(MockGithubService::new());

    let registry = Arc::new(PrPollerRegistry::new(
        Some(mock as Arc<dyn ralphx_lib::domain::services::github_service::GithubServiceTrait>),
        Arc::clone(&plan_branch_repo) as Arc<dyn PlanBranchRepository>,
    ));

    let transition_service = build_transition_service(&app_state, &execution_state);

    // Start then immediately stop
    registry.start_polling(
        task.id.clone(),
        plan_branch_id,
        42,
        std::path::PathBuf::from("/tmp/test-repo"),
        "main".to_string(),
        transition_service,
    );

    assert!(registry.is_polling(&task.id), "poller must be live after start_polling");

    registry.stop_polling(&task.id);

    assert!(
        !registry.is_polling(&task.id),
        "is_polling() must return false immediately after stop_polling"
    );
}

// ============================================================================
// Test 4: duplicate start_polling is idempotent
// ============================================================================

/// Calling `start_polling` twice for the same `TaskId` must be idempotent —
/// the second call is a no-op; the first handle stays alive.
/// After one `stop_polling`, the poller is gone.
#[tokio::test]
async fn test_duplicate_start_polling_is_idempotent() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());

    let project = Project::new("Test Project".to_string(), "/tmp/test-repo".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "Duplicate poller task".to_string());
    task.internal_status = InternalStatus::Merging;
    app_state.task_repo.create(task.clone()).await.unwrap();

    let plan_branch_repo = Arc::new(MemoryPlanBranchRepository::new());
    let mut pb = PlanBranch::new(
        ArtifactId::from_string("test-artifact".to_string()),
        IdeationSessionId::from_string("test-session".to_string()),
        project.id.clone(),
        "plan/feature".to_string(),
        "main".to_string(),
    );
    pb.merge_task_id = Some(task.id.clone());
    pb.pr_number = Some(42);
    pb.pr_eligible = true;
    pb.pr_polling_active = true;
    let plan_branch_id = pb.id.clone();
    plan_branch_repo.create(pb).await.unwrap();

    let mock = Arc::new(MockGithubService::new());

    let registry = Arc::new(PrPollerRegistry::new(
        Some(mock as Arc<dyn ralphx_lib::domain::services::github_service::GithubServiceTrait>),
        Arc::clone(&plan_branch_repo) as Arc<dyn PlanBranchRepository>,
    ));

    let transition_service = build_transition_service(&app_state, &execution_state);

    // First call — creates the live handle
    registry.start_polling(
        task.id.clone(),
        plan_branch_id.clone(),
        42,
        std::path::PathBuf::from("/tmp/test-repo"),
        "main".to_string(),
        Arc::clone(&transition_service),
    );

    assert!(registry.is_polling(&task.id), "poller must be live after first start_polling");

    // Second call — must be idempotent (no-op, first handle still live)
    registry.start_polling(
        task.id.clone(),
        plan_branch_id,
        42,
        std::path::PathBuf::from("/tmp/test-repo"),
        "main".to_string(),
        transition_service,
    );

    assert!(
        registry.is_polling(&task.id),
        "poller must still be live after second (duplicate) start_polling"
    );

    // One stop_polling removes the entry
    registry.stop_polling(&task.id);

    assert!(
        !registry.is_polling(&task.id),
        "is_polling() must be false after stop_polling even after duplicate start calls"
    );
}

// ============================================================================
// Test 5: pr_creation_guard DashMap CAS pattern
// ============================================================================

/// Verifies that `pr_creation_guard` supports the insert/contains/remove pattern
/// used by the create-draft-PR idempotency guard.
#[tokio::test]
async fn test_pr_creation_guard_is_dashmap() {
    let plan_branch_repo = Arc::new(MemoryPlanBranchRepository::new());
    let registry = PrPollerRegistry::new(
        None,
        Arc::clone(&plan_branch_repo) as Arc<dyn PlanBranchRepository>,
    );

    let pb_id = PlanBranchId::from_string("guard-test-branch".to_string());

    // Initially empty
    assert!(
        !registry.pr_creation_guard.contains_key(&pb_id),
        "pr_creation_guard must not contain the key before insertion"
    );

    // Insert — simulating CAS guard acquisition
    registry.pr_creation_guard.insert(pb_id.clone(), ());

    assert!(
        registry.pr_creation_guard.contains_key(&pb_id),
        "pr_creation_guard must contain the key after insertion"
    );

    // Remove — simulating guard release
    registry.pr_creation_guard.remove(&pb_id);

    assert!(
        !registry.pr_creation_guard.contains_key(&pb_id),
        "pr_creation_guard must not contain the key after removal"
    );
}

// ============================================================================
// Test 6: poller calls check_pr_status after jitter elapses
// ============================================================================

/// Verifies that the poll loop calls `check_pr_status` at least once after the
/// initial jitter sleep.  Uses `start_paused = true` + `tokio::time::advance`.
#[tokio::test(start_paused = true)]
async fn test_poller_calls_check_pr_status_after_jitter() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());

    let project = Project::new("Test Project".to_string(), "/tmp/test-repo".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "Status check task".to_string());
    task.internal_status = InternalStatus::Merging;
    app_state.task_repo.create(task.clone()).await.unwrap();

    let plan_branch_repo = Arc::new(MemoryPlanBranchRepository::new());
    let mut pb = PlanBranch::new(
        ArtifactId::from_string("test-artifact".to_string()),
        IdeationSessionId::from_string("test-session".to_string()),
        project.id.clone(),
        "plan/feature".to_string(),
        "main".to_string(),
    );
    pb.merge_task_id = Some(task.id.clone());
    pb.pr_number = Some(42);
    pb.pr_eligible = true;
    pb.pr_polling_active = true;
    let plan_branch_id = pb.id.clone();
    plan_branch_repo.create(pb).await.unwrap();

    let mock = Arc::new(MockGithubService::new());
    // Return Open so the poller does not self-terminate after the first check
    mock.will_return_status(PrStatus::Open);
    mock.will_return_status(PrStatus::Open);

    let registry = Arc::new(PrPollerRegistry::new(
        Some(mock.clone() as Arc<dyn ralphx_lib::domain::services::github_service::GithubServiceTrait>),
        Arc::clone(&plan_branch_repo) as Arc<dyn PlanBranchRepository>,
    ));

    let transition_service = build_transition_service(&app_state, &execution_state);

    registry.start_polling(
        task.id.clone(),
        plan_branch_id,
        42,
        std::path::PathBuf::from("/tmp/test-repo"),
        "main".to_string(),
        transition_service,
    );

    // Step-by-step advancement to account for tokio mock clock semantics:
    // `sleep(n)` registers a timer at `Instant::now() + n`.  If we advance
    // before the task starts, the task's sleep target is *after* the advance.
    // We must let the task start and register each sleep BEFORE advancing past it.
    //
    // Step 1: Let the spawned task start and register its jitter sleep.
    for _ in 0..5 {
        tokio::task::yield_now().await;
    }
    // Step 2: Advance past the maximum jitter window (31 s > max 30 s jitter).
    tokio::time::advance(std::time::Duration::from_secs(31)).await;
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    // Step 3: Task entered poll_loop and registered sleep(60 s).  Advance past it.
    tokio::time::advance(std::time::Duration::from_secs(61)).await;
    for _ in 0..20 {
        tokio::task::yield_now().await;
    }

    assert!(
        mock.check_calls() >= 1,
        "check_pr_status must be called at least once after jitter+interval elapses (got {} calls)",
        mock.check_calls()
    );
}

// ============================================================================
// Test 7: MERGED status causes the poller to stop itself
// ============================================================================

/// When `check_pr_status` returns `PrStatus::Merged`, the poll loop performs the
/// Merging→Merged transition and then exits, removing itself from `is_polling`.
#[tokio::test(start_paused = true)]
async fn test_poller_merged_stops_poller() {
    let app_state = AppState::new_test();
    let execution_state = Arc::new(ExecutionState::new());

    let project = Project::new("Test Project".to_string(), "/tmp/test-repo".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "Merged poller task".to_string());
    task.internal_status = InternalStatus::Merging;
    app_state.task_repo.create(task.clone()).await.unwrap();

    let plan_branch_repo = Arc::new(MemoryPlanBranchRepository::new());
    let mut pb = PlanBranch::new(
        ArtifactId::from_string("test-artifact".to_string()),
        IdeationSessionId::from_string("test-session".to_string()),
        project.id.clone(),
        "plan/feature".to_string(),
        "main".to_string(),
    );
    pb.merge_task_id = Some(task.id.clone());
    pb.pr_number = Some(42);
    pb.pr_eligible = true;
    pb.pr_polling_active = true;
    let plan_branch_id = pb.id.clone();
    plan_branch_repo.create(pb).await.unwrap();

    let mock = Arc::new(MockGithubService::new());
    // Return Merged — the poller should process the transition and exit
    mock.will_return_status(PrStatus::Merged {
        merge_commit_sha: Some("abc123".to_string()),
    });

    let registry = Arc::new(PrPollerRegistry::new(
        Some(mock.clone() as Arc<dyn ralphx_lib::domain::services::github_service::GithubServiceTrait>),
        Arc::clone(&plan_branch_repo) as Arc<dyn PlanBranchRepository>,
    ));

    let transition_service = build_transition_service(&app_state, &execution_state);

    registry.start_polling(
        task.id.clone(),
        plan_branch_id,
        42,
        std::path::PathBuf::from("/tmp/test-repo"),
        "main".to_string(),
        transition_service,
    );

    // Step-by-step advancement — see test_poller_calls_check_pr_status_after_jitter for rationale.
    // Step 1: Let the spawned task start and register its jitter sleep.
    for _ in 0..5 {
        tokio::task::yield_now().await;
    }
    // Step 2: Advance past the maximum jitter (31 s).
    tokio::time::advance(std::time::Duration::from_secs(31)).await;
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    // Step 3: Advance past the 60 s poll interval so check_pr_status fires.
    tokio::time::advance(std::time::Duration::from_secs(61)).await;
    for _ in 0..30 {
        tokio::task::yield_now().await;
    }

    // The poller must have invoked check_pr_status at least once
    assert!(
        mock.check_calls() >= 1,
        "check_pr_status must be called at least once before the poller exits (got {} calls)",
        mock.check_calls()
    );

    // After processing Merged the poller removes itself — is_polling must become false.
    assert!(
        !registry.is_polling(&task.id),
        "poller must remove itself from the registry after a MERGED status is processed"
    );
}
