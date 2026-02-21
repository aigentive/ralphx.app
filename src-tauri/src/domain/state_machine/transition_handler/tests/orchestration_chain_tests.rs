// Orchestration chain integration tests — real git + real DB + MockChatService
//
// These tests verify the full merge state machine path with real git repos and verify
// that the merger agent is spawned (or not) by checking MockChatService.call_count().
//
// Coverage:
//  A4 — plan_update_conflict: plan branch conflicts with main → Merging + agent spawn
//  A5 — plan_update_error → MergeIncomplete (no agent spawn)
//  A6 — source_update_conflict: task branch conflicts with target → Merging + agent spawn

use super::helpers::*;
use crate::domain::entities::{
    IdeationSessionId, InternalStatus, MergeStrategy, PlanBranchStatus,
};
use crate::domain::repositories::PlanBranchRepository;
use crate::domain::state_machine::{State, TransitionHandler};
use crate::infrastructure::memory::MemoryPlanBranchRepository;

// ─── A4 ─────────────────────────────────────────────────────────────────────

/// A4: plan_update_conflict path — plan branch has conflicting changes vs main.
///
/// Setup:
///   - Real git repo with `main` and a task branch
///   - Plan branch (`plan/feature-a4`) created from main with a conflicting change
///   - Main also commits a conflicting change to the same file → divergence
///   - Task has `ideation_session_id` linking it to the plan branch via PlanBranchRepository
///
/// Expected: `on_enter(PendingMerge)` routes through `update_plan_from_main()` →
///   detects conflict → persists `Merging` + calls `on_enter_dispatch(Merging)` →
///   `chat_service.send_message()` is called at least once (merger agent spawn).
#[tokio::test]
async fn plan_update_conflict_spawns_merger_agent() {
    // ── 1. Create real git repo ──────────────────────────────────────────────
    let git_repo = setup_real_git_repo();
    let path = git_repo.path();

    // ── 2. Create plan branch from main (before the conflicting commit) ──────
    let plan_branch = "plan/feature-a4";
    let _ = std::process::Command::new("git")
        .args(["branch", plan_branch])
        .current_dir(path)
        .output();

    // Add a conflicting change on main
    std::fs::write(path.join("shared.rs"), "// main version\nfn main_fn() {}").unwrap();
    let _ = std::process::Command::new("git")
        .args(["add", "shared.rs"])
        .current_dir(path)
        .output();
    let _ = std::process::Command::new("git")
        .args(["commit", "-m", "fix: main changes shared.rs"])
        .current_dir(path)
        .output();

    // Add a conflicting change on the plan branch (same file, different content)
    let _ = std::process::Command::new("git")
        .args(["checkout", plan_branch])
        .current_dir(path)
        .output();
    std::fs::write(path.join("shared.rs"), "// plan version\nfn plan_fn() {}").unwrap();
    let _ = std::process::Command::new("git")
        .args(["add", "shared.rs"])
        .current_dir(path)
        .output();
    let _ = std::process::Command::new("git")
        .args(["commit", "-m", "feat: plan changes shared.rs"])
        .current_dir(path)
        .output();

    // Back to main (merge will use worktree, needs main checked out)
    let _ = std::process::Command::new("git")
        .args(["checkout", "main"])
        .current_dir(path)
        .output();

    // ── 3. Set up repos: task with ideation_session_id + plan branch record ──
    let task_repo = Arc::new(MemoryTaskRepository::new());
    let project_repo = Arc::new(MemoryProjectRepository::new());
    let plan_branch_repo = Arc::new(MemoryPlanBranchRepository::new());

    let session_id_str = "sess-a4";
    let session_id = IdeationSessionId::from_string(session_id_str.to_string());
    let project_id = crate::domain::entities::ProjectId::from_string("proj-1".to_string());

    // Create task with ideation_session_id
    let mut task = crate::domain::entities::Task::new(project_id.clone(), "A4 plan conflict test".into());
    task.internal_status = InternalStatus::PendingMerge;
    task.task_branch = Some(git_repo.task_branch.clone());
    task.ideation_session_id = Some(session_id.clone());
    let task_id = task.id.clone();
    task_repo.create(task).await.unwrap();

    // Create project pointing at real git repo
    let mut project = crate::domain::entities::Project::new("test-project".into(), git_repo.path_string());
    project.id = project_id.clone();
    project.base_branch = Some("main".into());
    project.merge_strategy = MergeStrategy::Merge;
    project_repo.create(project).await.unwrap();

    // Register plan branch in repo (links session_id → plan/feature-a4)
    let pb = make_plan_branch(
        "artifact-a4",
        plan_branch,
        PlanBranchStatus::Active,
        None,
    );
    // Override session_id to match (make_plan_branch hard-codes "sess-1")
    let mut pb = pb;
    pb.session_id = session_id.clone();
    pb.project_id = project_id;
    plan_branch_repo.create(pb).await.unwrap();

    // ── 4. Wire MockChatService ───────────────────────────────────────────────
    let mock_chat = Arc::new(MockChatService::new());

    let services = TaskServices::new(
        Arc::new(MockAgentSpawner::new()) as Arc<dyn AgentSpawner>,
        Arc::new(MockEventEmitter::new()) as Arc<dyn EventEmitter>,
        Arc::new(MockNotifier::new()) as Arc<dyn Notifier>,
        Arc::new(MockDependencyManager::new()) as Arc<dyn DependencyManager>,
        Arc::new(MockReviewStarter::new()) as Arc<dyn ReviewStarter>,
        Arc::clone(&mock_chat) as Arc<dyn crate::application::ChatService>,
    )
    .with_task_repo(Arc::clone(&task_repo) as Arc<dyn crate::domain::repositories::TaskRepository>)
    .with_project_repo(Arc::clone(&project_repo) as Arc<dyn crate::domain::repositories::ProjectRepository>)
    .with_plan_branch_repo(Arc::clone(&plan_branch_repo) as Arc<dyn PlanBranchRepository>);

    let context = crate::domain::state_machine::context::TaskContext::new(
        task_id.as_str(),
        "proj-1",
        services,
    );
    let mut machine = crate::domain::state_machine::TaskStateMachine::new(context);

    // ── 5. Run the on_enter ──────────────────────────────────────────────────
    let handler = TransitionHandler::new(&mut machine);
    let _ = handler.on_enter(&State::PendingMerge).await;

    // ── 6. Assert: task in Merging, agent was spawned ────────────────────────
    let updated = task_repo.get_by_id(&task_id).await.unwrap().unwrap();
    assert_eq!(
        updated.internal_status,
        InternalStatus::Merging,
        "plan_update_conflict should route task to Merging, got {:?}. Metadata: {:?}",
        updated.internal_status,
        updated.metadata,
    );

    let meta: serde_json::Value =
        serde_json::from_str(updated.metadata.as_deref().unwrap_or("{}")).unwrap();
    assert_eq!(
        meta.get("plan_update_conflict"),
        Some(&serde_json::json!(true)),
        "Metadata should have plan_update_conflict=true. Metadata: {:?}",
        updated.metadata,
    );

    assert!(
        mock_chat.call_count() >= 1,
        "Merger agent should have been spawned (call_count >= 1), got {}",
        mock_chat.call_count(),
    );
}

// ─── A5 ─────────────────────────────────────────────────────────────────────

/// A5: plan_update_error path — base branch doesn't exist, update_plan_from_main errors.
///
/// Setup:
///   - Real git repo with a plan branch (target_branch != base_branch)
///   - Project has `base_branch = "nonexistent-base"` (no such git branch)
///
/// Expected: `update_plan_from_main()` returns `Error` →
///   `transition_to_merge_incomplete` is called →
///   task ends in `MergeIncomplete`, no agent spawn (call_count == 0).
#[tokio::test]
async fn plan_update_error_aborts_to_merge_incomplete() {
    // ── 1. Create real git repo with plan branch ─────────────────────────────
    let git_repo = setup_real_git_repo();
    let path = git_repo.path();

    let plan_branch = "plan/feature-a5";
    let _ = std::process::Command::new("git")
        .args(["branch", plan_branch])
        .current_dir(path)
        .output();

    // ── 2. Set up repos ──────────────────────────────────────────────────────
    let task_repo = Arc::new(MemoryTaskRepository::new());
    let project_repo = Arc::new(MemoryProjectRepository::new());
    let plan_branch_repo = Arc::new(MemoryPlanBranchRepository::new());

    let session_id_str = "sess-a5";
    let session_id = IdeationSessionId::from_string(session_id_str.to_string());
    let project_id = crate::domain::entities::ProjectId::from_string("proj-1".to_string());

    // Task with ideation_session_id (causes resolve_merge_branches to use plan branch)
    let mut task = crate::domain::entities::Task::new(project_id.clone(), "A5 plan error test".into());
    task.internal_status = InternalStatus::PendingMerge;
    task.task_branch = Some(git_repo.task_branch.clone());
    task.ideation_session_id = Some(session_id.clone());
    let task_id = task.id.clone();
    task_repo.create(task).await.unwrap();

    // Project with nonexistent base branch → update_plan_from_main cannot get SHA
    let mut project = crate::domain::entities::Project::new("test-project".into(), git_repo.path_string());
    project.id = project_id.clone();
    project.base_branch = Some("nonexistent-base".into()); // will cause Error result
    project.merge_strategy = MergeStrategy::Merge;
    project_repo.create(project).await.unwrap();

    // Plan branch record (target_branch = plan/feature-a5 != "nonexistent-base")
    let mut pb = make_plan_branch(
        "artifact-a5",
        plan_branch,
        PlanBranchStatus::Active,
        None,
    );
    pb.session_id = session_id;
    pb.project_id = project_id;
    plan_branch_repo.create(pb).await.unwrap();

    // ── 3. Wire MockChatService ───────────────────────────────────────────────
    let mock_chat = Arc::new(MockChatService::new());

    let services = TaskServices::new(
        Arc::new(MockAgentSpawner::new()) as Arc<dyn AgentSpawner>,
        Arc::new(MockEventEmitter::new()) as Arc<dyn EventEmitter>,
        Arc::new(MockNotifier::new()) as Arc<dyn Notifier>,
        Arc::new(MockDependencyManager::new()) as Arc<dyn DependencyManager>,
        Arc::new(MockReviewStarter::new()) as Arc<dyn ReviewStarter>,
        Arc::clone(&mock_chat) as Arc<dyn crate::application::ChatService>,
    )
    .with_task_repo(Arc::clone(&task_repo) as Arc<dyn crate::domain::repositories::TaskRepository>)
    .with_project_repo(Arc::clone(&project_repo) as Arc<dyn crate::domain::repositories::ProjectRepository>)
    .with_plan_branch_repo(Arc::clone(&plan_branch_repo) as Arc<dyn PlanBranchRepository>);

    let context = crate::domain::state_machine::context::TaskContext::new(
        task_id.as_str(),
        "proj-1",
        services,
    );
    let mut machine = crate::domain::state_machine::TaskStateMachine::new(context);

    // ── 4. Run the on_enter ──────────────────────────────────────────────────
    let handler = TransitionHandler::new(&mut machine);
    let _ = handler.on_enter(&State::PendingMerge).await;

    // ── 5. Assert: task in MergeIncomplete, no agent spawn ───────────────────
    let updated = task_repo.get_by_id(&task_id).await.unwrap().unwrap();
    assert_eq!(
        updated.internal_status,
        InternalStatus::MergeIncomplete,
        "plan_update_error should route task to MergeIncomplete, got {:?}. Metadata: {:?}",
        updated.internal_status,
        updated.metadata,
    );

    assert_eq!(
        mock_chat.call_count(),
        0,
        "No agent should be spawned on plan_update_error (call_count should be 0), got {}",
        mock_chat.call_count(),
    );
}

// ─── A6 ─────────────────────────────────────────────────────────────────────

/// A6: source_update_conflict path — task branch conflicts with main (target branch).
///
/// Setup:
///   - Real git repo with `main` and a task branch that both modify `feature.rs`
///   - Main has a commit (after task branch creation) that conflicts with task branch
///   - No plan branch involved (target = main = base_branch — standard task merge)
///
/// Expected: `update_source_from_target()` returns `Conflicts` →
///   worktree created for source branch → task persisted as `Merging` →
///   `on_enter_dispatch(Merging)` → `chat_service.send_message()` called (agent spawn).
#[tokio::test]
async fn source_update_conflict_spawns_merger_agent() {
    // ── 1. Create real git repo ──────────────────────────────────────────────
    let git_repo = setup_real_git_repo();
    let path = git_repo.path();

    // Add a conflicting change on main AFTER task branch was created
    // (task branch already has feature.rs from setup_real_git_repo)
    std::fs::write(
        path.join("feature.rs"),
        "// main conflicting version\nfn main_feature() {}",
    )
    .unwrap();
    let _ = std::process::Command::new("git")
        .args(["add", "feature.rs"])
        .current_dir(path)
        .output();
    let _ = std::process::Command::new("git")
        .args(["commit", "-m", "fix: main modifies feature.rs (conflict)"])
        .current_dir(path)
        .output();

    // ── 2. Set up repos ──────────────────────────────────────────────────────
    let task_repo = Arc::new(MemoryTaskRepository::new());
    let project_repo = Arc::new(MemoryProjectRepository::new());

    let project_id = crate::domain::entities::ProjectId::from_string("proj-1".to_string());

    // Regular task (no ideation_session_id → target = main = base_branch)
    let mut task = crate::domain::entities::Task::new(project_id.clone(), "A6 source conflict test".into());
    task.internal_status = InternalStatus::PendingMerge;
    task.task_branch = Some(git_repo.task_branch.clone());
    let task_id = task.id.clone();
    task_repo.create(task).await.unwrap();

    // Project pointing at real git repo
    let mut project = crate::domain::entities::Project::new("test-project".into(), git_repo.path_string());
    project.id = project_id;
    project.base_branch = Some("main".into());
    project.merge_strategy = MergeStrategy::Merge;
    project_repo.create(project).await.unwrap();

    // ── 3. Wire MockChatService ───────────────────────────────────────────────
    let mock_chat = Arc::new(MockChatService::new());

    let services = TaskServices::new(
        Arc::new(MockAgentSpawner::new()) as Arc<dyn AgentSpawner>,
        Arc::new(MockEventEmitter::new()) as Arc<dyn EventEmitter>,
        Arc::new(MockNotifier::new()) as Arc<dyn Notifier>,
        Arc::new(MockDependencyManager::new()) as Arc<dyn DependencyManager>,
        Arc::new(MockReviewStarter::new()) as Arc<dyn ReviewStarter>,
        Arc::clone(&mock_chat) as Arc<dyn crate::application::ChatService>,
    )
    .with_task_repo(Arc::clone(&task_repo) as Arc<dyn crate::domain::repositories::TaskRepository>)
    .with_project_repo(Arc::clone(&project_repo) as Arc<dyn crate::domain::repositories::ProjectRepository>);

    let context = crate::domain::state_machine::context::TaskContext::new(
        task_id.as_str(),
        "proj-1",
        services,
    );
    let mut machine = crate::domain::state_machine::TaskStateMachine::new(context);

    // ── 4. Run the on_enter ──────────────────────────────────────────────────
    let handler = TransitionHandler::new(&mut machine);
    let _ = handler.on_enter(&State::PendingMerge).await;

    // ── 5. Assert: task in Merging, agent was spawned ────────────────────────
    let updated = task_repo.get_by_id(&task_id).await.unwrap().unwrap();
    assert_eq!(
        updated.internal_status,
        InternalStatus::Merging,
        "source_update_conflict should route task to Merging, got {:?}. Metadata: {:?}",
        updated.internal_status,
        updated.metadata,
    );

    let meta: serde_json::Value =
        serde_json::from_str(updated.metadata.as_deref().unwrap_or("{}")).unwrap();
    assert_eq!(
        meta.get("source_update_conflict"),
        Some(&serde_json::json!(true)),
        "Metadata should have source_update_conflict=true. Metadata: {:?}",
        updated.metadata,
    );

    assert!(
        mock_chat.call_count() >= 1,
        "Merger agent should have been spawned (call_count >= 1), got {}",
        mock_chat.call_count(),
    );
}
