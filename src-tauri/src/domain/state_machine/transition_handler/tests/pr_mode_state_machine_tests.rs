// PR-mode state machine integration tests
//
// Tests for Phase 3 PR integration: PendingMerge + Merging + Merged paths
// when pr_eligible=true and GithubServiceTrait is wired.
//
// Covered scenarios:
//   1. PR-mode with existing pr_number: push_branch + mark_pr_ready, no create_draft_pr
//   2. PR-mode without pr_number: create_draft_pr + mark_pr_ready
//   3. pr_eligible=false: skips PR path entirely (no github calls)
//   4. Re-entry guard: pr_polling_active=true, no registry → proceeds normally
//   5. AD14: PR-polling task in Merging does not block a second PendingMerge task
//   6. post_merge_cleanup idempotency: plan_branch.status == Merged → early return

use super::helpers::*;
use crate::domain::entities::{
    types::IdeationSessionId, ArtifactId, InternalStatus, PlanBranch, PlanBranchStatus, Project,
    ProjectId, Task, TaskCategory, TaskId,
};
use crate::domain::repositories::{PlanBranchRepository, ProjectRepository, TaskRepository};
use crate::domain::services::github_service::GithubServiceTrait;
use crate::domain::state_machine::{State, TransitionHandler};
use crate::infrastructure::memory::{
    MemoryPlanBranchRepository, MemoryProjectRepository, MemoryTaskRepository,
};
use crate::tests::mock_github_service::MockGithubService;

// ─────────────────────────────────────────────────────────────────────────────
// Shared helpers
// ─────────────────────────────────────────────────────────────────────────────

fn make_pr_eligible_plan_branch(
    task_id: &TaskId,
    pr_number: Option<i64>,
    pr_polling_active: bool,
) -> PlanBranch {
    let mut pb = PlanBranch::new(
        ArtifactId::from_string("artifact-1".to_string()),
        IdeationSessionId::from_string("sess-1".to_string()),
        ProjectId::from_string("proj-1".to_string()),
        "plan/feature-branch".to_string(),
        "main".to_string(), // source_branch = base branch (PR target)
    );
    pb.merge_task_id = Some(task_id.clone());
    pb.pr_eligible = true;
    pb.pr_number = pr_number;
    pb.pr_polling_active = pr_polling_active;
    pb
}

async fn setup_project(project_repo: &MemoryProjectRepository) {
    let mut project =
        Project::new("test-project".to_string(), "/tmp/pr-mode-test".to_string());
    project.id = ProjectId::from_string("proj-1".to_string());
    project.base_branch = Some("main".to_string());
    project_repo.create(project).await.unwrap();
}

async fn create_pending_merge_task(
    task_repo: &MemoryTaskRepository,
    task_id_str: &str,
) -> TaskId {
    let mut task = Task::new(ProjectId::from_string("proj-1".to_string()), "PR merge task".to_string());
    task.id = TaskId::from_string(task_id_str.to_string());
    task.internal_status = InternalStatus::PendingMerge;
    task.category = TaskCategory::PlanMerge;
    let task_id = task.id.clone();
    task_repo.create(task).await.unwrap();
    task_id
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 1: PR-mode with existing pr_number → push_branch + mark_pr_ready
// ─────────────────────────────────────────────────────────────────────────────

/// PR-mode: plan_branch has pr_number=42.
/// Expected: push_branch(42) + mark_pr_ready(42) called, create_draft_pr NOT called.
#[tokio::test]
async fn test_pr_mode_with_existing_pr_number_calls_push_and_mark_ready() {
    let task_repo = Arc::new(MemoryTaskRepository::new());
    let project_repo = Arc::new(MemoryProjectRepository::new());
    let plan_branch_repo = Arc::new(MemoryPlanBranchRepository::new());

    setup_project(&project_repo).await;
    let task_id = create_pending_merge_task(&task_repo, "task-pr-existing").await;

    let pb = make_pr_eligible_plan_branch(&task_id, Some(42), false);
    plan_branch_repo.create(pb).await.unwrap();

    let mock_github = Arc::new(MockGithubService::new());

    let services = TaskServices::new_mock()
        .with_task_repo(Arc::clone(&task_repo) as Arc<dyn TaskRepository>)
        .with_project_repo(Arc::clone(&project_repo) as Arc<dyn ProjectRepository>)
        .with_plan_branch_repo(
            Arc::clone(&plan_branch_repo) as Arc<dyn PlanBranchRepository>
        )
        .with_github_service(
            Arc::clone(&mock_github) as Arc<dyn GithubServiceTrait>
        );

    let context = TaskContext::new(task_id.as_str(), "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    let result = handler.on_enter(&State::PendingMerge).await;
    assert!(result.is_ok(), "on_enter(PendingMerge) should succeed: {:?}", result);

    let state = mock_github.state();
    assert_eq!(state.push_branch_calls, 1, "push_branch should be called once");
    assert_eq!(
        state.mark_pr_ready_calls, 1,
        "mark_pr_ready should be called once"
    );
    assert_eq!(
        state.create_draft_pr_calls, 0,
        "create_draft_pr should NOT be called when pr_number already exists"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 2: PR-mode without pr_number → creates new PR
// ─────────────────────────────────────────────────────────────────────────────

/// PR-mode: plan_branch has no pr_number yet.
/// Expected: push_branch called, create_draft_pr called (returns pr#99), mark_pr_ready called.
#[tokio::test]
async fn test_pr_mode_without_pr_number_creates_new_pr() {
    let task_repo = Arc::new(MemoryTaskRepository::new());
    let project_repo = Arc::new(MemoryProjectRepository::new());
    let plan_branch_repo = Arc::new(MemoryPlanBranchRepository::new());

    setup_project(&project_repo).await;
    let task_id = create_pending_merge_task(&task_repo, "task-pr-new").await;

    // No pr_number — should trigger PR creation path
    let pb = make_pr_eligible_plan_branch(&task_id, None, false);
    plan_branch_repo.create(pb).await.unwrap();

    let mock_github = Arc::new(MockGithubService::new());
    mock_github.will_create_pr(99, "https://github.com/owner/repo/pull/99");

    let services = TaskServices::new_mock()
        .with_task_repo(Arc::clone(&task_repo) as Arc<dyn TaskRepository>)
        .with_project_repo(Arc::clone(&project_repo) as Arc<dyn ProjectRepository>)
        .with_plan_branch_repo(
            Arc::clone(&plan_branch_repo) as Arc<dyn PlanBranchRepository>
        )
        .with_github_service(
            Arc::clone(&mock_github) as Arc<dyn GithubServiceTrait>
        );

    let context = TaskContext::new(task_id.as_str(), "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    let result = handler.on_enter(&State::PendingMerge).await;
    assert!(result.is_ok(), "on_enter(PendingMerge) should succeed: {:?}", result);

    {
        let state = mock_github.state();
        assert_eq!(state.push_branch_calls, 1, "push_branch should be called once");
        assert_eq!(
            state.create_draft_pr_calls, 1,
            "create_draft_pr should be called when pr_number is absent"
        );
        assert_eq!(
            state.mark_pr_ready_calls, 1,
            "mark_pr_ready should be called after creation"
        );
    } // drop MutexGuard before await

    // Verify pr_info was stored in plan branch repo
    let updated_pb = plan_branch_repo
        .get_by_merge_task_id(&task_id)
        .await
        .unwrap()
        .expect("plan branch should still exist");
    assert_eq!(
        updated_pb.pr_number,
        Some(99),
        "pr_number should be persisted after creation"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 3: pr_eligible=false → skips PR path, no github calls
// ─────────────────────────────────────────────────────────────────────────────

/// When pr_eligible=false, the PR fork is not taken.
/// The push-to-main path runs instead (fails fast on nonexistent dir).
/// No GitHub service calls should be made.
#[tokio::test]
async fn test_pr_eligible_false_skips_pr_path() {
    let task_repo = Arc::new(MemoryTaskRepository::new());
    let project_repo = Arc::new(MemoryProjectRepository::new());
    let plan_branch_repo = Arc::new(MemoryPlanBranchRepository::new());

    setup_project(&project_repo).await;
    let task_id = create_pending_merge_task(&task_repo, "task-push-to-main").await;

    // pr_eligible = false → should NOT trigger PR path
    let mut pb = make_pr_eligible_plan_branch(&task_id, Some(42), false);
    pb.pr_eligible = false;
    plan_branch_repo.create(pb).await.unwrap();

    let mock_github = Arc::new(MockGithubService::new());

    let services = TaskServices::new_mock()
        .with_task_repo(Arc::clone(&task_repo) as Arc<dyn TaskRepository>)
        .with_project_repo(Arc::clone(&project_repo) as Arc<dyn ProjectRepository>)
        .with_plan_branch_repo(
            Arc::clone(&plan_branch_repo) as Arc<dyn PlanBranchRepository>
        )
        .with_github_service(
            Arc::clone(&mock_github) as Arc<dyn GithubServiceTrait>
        );

    let context = TaskContext::new(task_id.as_str(), "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    let result = handler.on_enter(&State::PendingMerge).await;
    assert!(result.is_ok(), "on_enter(PendingMerge) should succeed: {:?}", result);

    let state = mock_github.state();
    assert_eq!(
        state.push_branch_calls, 0,
        "push_branch should NOT be called when pr_eligible=false"
    );
    assert_eq!(
        state.mark_pr_ready_calls, 0,
        "mark_pr_ready should NOT be called when pr_eligible=false"
    );
    assert_eq!(
        state.create_draft_pr_calls, 0,
        "create_draft_pr should NOT be called when pr_eligible=false"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 4: Re-entry guard — pr_polling_active=true, no registry → proceeds
// ─────────────────────────────────────────────────────────────────────────────

/// pr_polling_active=true but no PrPollerRegistry wired.
/// The re-entry guard only triggers when BOTH flags are set AND registry.is_polling().
/// Without a registry, guard is skipped → PR operations proceed normally.
#[tokio::test]
async fn test_pr_mode_reentry_guard_no_registry_proceeds() {
    let task_repo = Arc::new(MemoryTaskRepository::new());
    let project_repo = Arc::new(MemoryProjectRepository::new());
    let plan_branch_repo = Arc::new(MemoryPlanBranchRepository::new());

    setup_project(&project_repo).await;
    let task_id = create_pending_merge_task(&task_repo, "task-reentry").await;

    // pr_polling_active=true simulates a previous run that set the flag.
    // Without a registry, is_polling() can't be checked → guard bypassed.
    let pb = make_pr_eligible_plan_branch(&task_id, Some(77), true);
    plan_branch_repo.create(pb).await.unwrap();

    let mock_github = Arc::new(MockGithubService::new());

    let services = TaskServices::new_mock()
        .with_task_repo(Arc::clone(&task_repo) as Arc<dyn TaskRepository>)
        .with_project_repo(Arc::clone(&project_repo) as Arc<dyn ProjectRepository>)
        .with_plan_branch_repo(
            Arc::clone(&plan_branch_repo) as Arc<dyn PlanBranchRepository>
        )
        .with_github_service(
            Arc::clone(&mock_github) as Arc<dyn GithubServiceTrait>
        );
    // NOTE: no .with_pr_poller_registry() — guard must be bypassed

    let context = TaskContext::new(task_id.as_str(), "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    let result = handler.on_enter(&State::PendingMerge).await;
    assert!(result.is_ok(), "on_enter(PendingMerge) should succeed: {:?}", result);

    // With no registry, re-entry guard doesn't fire → operations proceed
    let state = mock_github.state();
    assert_eq!(
        state.push_branch_calls, 1,
        "push_branch should be called when no registry prevents re-entry"
    );
    assert_eq!(
        state.mark_pr_ready_calls, 1,
        "mark_pr_ready should be called when no registry prevents re-entry"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 5: AD14 — PR-polling task in Merging does not block second PendingMerge task
// ─────────────────────────────────────────────────────────────────────────────

/// AD14: find_blocking_merge_task skips tasks whose merge_task_id is in pr_polling_ids.
///
/// Setup:
///   - Task A: in Merging, plan_branch has pr_polling_active=true
///   - Task B: in PendingMerge (pr_eligible=true, pr_number=55), test subject
///
/// Without AD14, Task A (Merging) would block Task B (PendingMerge) from proceeding.
/// With AD14, Task A is excluded from blocking → Task B proceeds → push_branch called.
#[tokio::test]
async fn test_ad14_pr_polling_task_does_not_block_pending_merge() {
    let task_repo = Arc::new(MemoryTaskRepository::new());
    let project_repo = Arc::new(MemoryProjectRepository::new());
    let plan_branch_repo = Arc::new(MemoryPlanBranchRepository::new());

    setup_project(&project_repo).await;

    // Task A: in Merging, is a PR-polling task
    let mut task_a = Task::new(ProjectId::from_string("proj-1".to_string()), "Task A (merging)".to_string());
    task_a.id = TaskId::from_string("task-a-merging".to_string());
    task_a.internal_status = InternalStatus::Merging;
    task_a.category = TaskCategory::PlanMerge;
    let task_a_id = task_a.id.clone();
    task_repo.create(task_a).await.unwrap();

    // Plan branch for Task A: pr_polling_active=true makes it excluded by AD14
    let mut pb_a = PlanBranch::new(
        ArtifactId::from_string("artifact-a".to_string()),
        IdeationSessionId::from_string("sess-1".to_string()),
        ProjectId::from_string("proj-1".to_string()),
        "plan/branch-a".to_string(),
        "main".to_string(),
    );
    pb_a.merge_task_id = Some(task_a_id.clone());
    pb_a.pr_eligible = true;
    pb_a.pr_number = Some(10);
    pb_a.pr_polling_active = true;
    plan_branch_repo.create(pb_a).await.unwrap();

    // Task B: in PendingMerge — this is what we're testing
    let task_b_id = create_pending_merge_task(&task_repo, "task-b-pending").await;

    // Plan branch for Task B: pr_eligible=true, pr_number=55
    let pb_b = make_pr_eligible_plan_branch(&task_b_id, Some(55), false);
    plan_branch_repo.create(pb_b).await.unwrap();

    let mock_github = Arc::new(MockGithubService::new());

    let services = TaskServices::new_mock()
        .with_task_repo(Arc::clone(&task_repo) as Arc<dyn TaskRepository>)
        .with_project_repo(Arc::clone(&project_repo) as Arc<dyn ProjectRepository>)
        .with_plan_branch_repo(
            Arc::clone(&plan_branch_repo) as Arc<dyn PlanBranchRepository>
        )
        .with_github_service(
            Arc::clone(&mock_github) as Arc<dyn GithubServiceTrait>
        );

    let context = TaskContext::new(task_b_id.as_str(), "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    let result = handler.on_enter(&State::PendingMerge).await;
    assert!(result.is_ok(), "on_enter(PendingMerge) should succeed: {:?}", result);

    // Task B should proceed (not deferred by Task A) and call push_branch
    let state = mock_github.state();
    assert_eq!(
        state.push_branch_calls, 1,
        "Task B should proceed despite Task A being in Merging (AD14 excludes PR-polling tasks)"
    );
    assert_eq!(
        state.mark_pr_ready_calls, 1,
        "Task B should mark PR ready after proceeding"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 6: post_merge_cleanup idempotency — Merged plan branch returns early
// ─────────────────────────────────────────────────────────────────────────────

/// on_enter(Merged) calls post_merge_cleanup for PlanMerge tasks.
/// If plan_branch.status is already Merged, the cleanup returns early (idempotency guard).
/// Test verifies: no error, no infinite loop, expected guard branch executes.
#[tokio::test]
async fn test_post_merge_cleanup_idempotency_already_merged_plan_branch() {
    let task_repo = Arc::new(MemoryTaskRepository::new());
    let project_repo = Arc::new(MemoryProjectRepository::new());
    let plan_branch_repo = Arc::new(MemoryPlanBranchRepository::new());

    setup_project(&project_repo).await;

    // Task in Merged state (simulating successful merge)
    let mut task = Task::new(ProjectId::from_string("proj-1".to_string()), "Merged task".to_string());
    task.id = TaskId::from_string("task-already-merged".to_string());
    task.internal_status = InternalStatus::Merged;
    task.category = TaskCategory::PlanMerge;
    let task_id = task.id.clone();
    task_repo.create(task).await.unwrap();

    // Plan branch already in Merged status — idempotency guard should trigger
    let mut pb = make_pr_eligible_plan_branch(&task_id, Some(88), false);
    pb.status = PlanBranchStatus::Merged;
    plan_branch_repo.create(pb).await.unwrap();

    let services = TaskServices::new_mock()
        .with_task_repo(Arc::clone(&task_repo) as Arc<dyn TaskRepository>)
        .with_project_repo(Arc::clone(&project_repo) as Arc<dyn ProjectRepository>)
        .with_plan_branch_repo(
            Arc::clone(&plan_branch_repo) as Arc<dyn PlanBranchRepository>
        );

    let context = TaskContext::new(task_id.as_str(), "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    // on_enter(Merged) → post_merge_cleanup → idempotency guard → early return
    let result = handler.on_enter(&State::Merged).await;
    assert!(
        result.is_ok(),
        "on_enter(Merged) with already-merged plan branch should succeed without error: {:?}",
        result
    );

    // Plan branch status should remain Merged (not double-transitioned)
    let pb_after = plan_branch_repo
        .get_by_merge_task_id(&task_id)
        .await
        .unwrap()
        .expect("plan branch should still exist");
    assert_eq!(
        pb_after.status,
        PlanBranchStatus::Merged,
        "plan branch should still be Merged after idempotent cleanup"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 7: No github_service → falls through to push-to-main path
// ─────────────────────────────────────────────────────────────────────────────

/// pr_eligible=true but no GithubServiceTrait wired.
/// pr_mode = pr_eligible && github_service.is_some() → false.
/// Falls through to push-to-main path (no PR calls).
#[tokio::test]
async fn test_pr_eligible_true_but_no_github_service_falls_through() {
    let task_repo = Arc::new(MemoryTaskRepository::new());
    let project_repo = Arc::new(MemoryProjectRepository::new());
    let plan_branch_repo = Arc::new(MemoryPlanBranchRepository::new());

    setup_project(&project_repo).await;
    let task_id = create_pending_merge_task(&task_repo, "task-no-github-svc").await;

    let pb = make_pr_eligible_plan_branch(&task_id, Some(99), false);
    plan_branch_repo.create(pb).await.unwrap();

    // No github_service wired → pr_mode = false
    let services = TaskServices::new_mock()
        .with_task_repo(Arc::clone(&task_repo) as Arc<dyn TaskRepository>)
        .with_project_repo(Arc::clone(&project_repo) as Arc<dyn ProjectRepository>)
        .with_plan_branch_repo(
            Arc::clone(&plan_branch_repo) as Arc<dyn PlanBranchRepository>
        );
    // NOTE: no .with_github_service()

    let context = TaskContext::new(task_id.as_str(), "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    // Should run the push-to-main path (fails fast on nonexistent git dir) without PR calls
    let result = handler.on_enter(&State::PendingMerge).await;
    assert!(
        result.is_ok(),
        "on_enter(PendingMerge) without github_service should fall through gracefully: {:?}",
        result
    );
    // No assertions on MockGithubService since it was never wired
}
