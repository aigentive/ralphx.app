// Integration tests for worktree restoration helpers and their integration points.
//
// Three tests covering the three layers where restore_task_worktree is used:
//
//   Test 1 (re_review_worktree_restore) — direct function call (L3, review_commands.rs path)
//     Calls restore_task_worktree() directly with a task whose worktree_path points to a
//     non-existent merge-{id} directory. The task branch exists in a real git repo so the
//     function should recreate the worktree via checkout_existing_branch_worktree.
//
//   Test 2 (merging_to_pending_review_worktree_path_reset) — L1 via TaskTransitionService
//     Simulates the exact sequence that L1 executes: task in Merging with freshness metadata
//     (reviewing origin + merge-prefixed worktree_path), transition to PendingReview via
//     TaskTransitionService. The transition service detects the stale merge-prefixed path and
//     calls restore_task_worktree, persisting the corrected path before the transition completes.
//
//   Test 3 (on_enter_reviewing_restores_merge_prefixed_worktree) — L2 via on_enter(Reviewing)
//     on_enter(Reviewing) in on_enter_states.rs detects that worktree_path has a merge-prefix,
//     calls restore_task_worktree, and updates the task in the repo with the correct path.

use super::helpers::*;
use crate::application::AppState;
use crate::application::chat_service::freshness_routing::{
    FreshnessRouteResult, freshness_return_route,
};
use crate::commands::ExecutionState;
use crate::domain::entities::{InternalStatus, Project, ProjectId, Task};
use crate::domain::services::{MemoryRunningAgentRegistry, MessageQueue};
use crate::domain::state_machine::services::TaskScheduler;
use crate::domain::state_machine::transition_handler::merge_helpers::{
    compute_task_worktree_path, is_merge_worktree_path, restore_task_worktree,
};
use crate::domain::state_machine::{State, TransitionHandler};

// ──────────────────────────────────────────────────────────────────────────────
// Shared helper: build TaskServices with retained MockChatService
// ──────────────────────────────────────────────────────────────────────────────

fn make_services_with_tracked_chat(
    task_repo: Arc<MemoryTaskRepository>,
    project_repo: Arc<MemoryProjectRepository>,
) -> (Arc<MockChatService>, TaskServices) {
    let chat_service = Arc::new(MockChatService::new());
    let services = TaskServices::new(
        Arc::new(MockAgentSpawner::new()) as Arc<dyn AgentSpawner>,
        Arc::new(MockEventEmitter::new()) as Arc<dyn EventEmitter>,
        Arc::new(MockNotifier::new()) as Arc<dyn Notifier>,
        Arc::new(MockDependencyManager::new()) as Arc<dyn DependencyManager>,
        Arc::new(MockReviewStarter::new()) as Arc<dyn ReviewStarter>,
        Arc::clone(&chat_service) as Arc<dyn ChatService>,
    )
    .with_task_scheduler(Arc::new(MockTaskScheduler::new()) as Arc<dyn TaskScheduler>)
    .with_task_repo(Arc::clone(&task_repo) as Arc<dyn TaskRepository>)
    .with_project_repo(Arc::clone(&project_repo) as Arc<dyn ProjectRepository>);
    (chat_service, services)
}

// ──────────────────────────────────────────────────────────────────────────────
// Test 1: restore_task_worktree() direct call (L3 path)
//
// Precondition: task.worktree_path = "/tmp/nonexistent/merge-{id}" (does not exist)
//               task.task_branch exists in the real git repo
// Expected:     task.worktree_path updated to task-{id} path after the call
//               returned PathBuf exists on disk (worktree was created)
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn re_review_worktree_restore() {
    let git_repo = setup_real_git_repo();
    let repo_path = git_repo.path();

    // Build a minimal Project pointing at the real git repo.
    // We use the repo path as working_directory; no worktree_parent_directory so
    // compute_task_worktree_path will use the default ~/ralphx-worktrees convention.
    // However, to keep the worktree under the temp dir (for reliable cleanup), we
    // set worktree_parent_directory explicitly to repo_path.
    let project_id = ProjectId::from_string("proj-wt-restore-1".to_string());
    let mut project = Project::new(
        "wt-restore-project".to_string(),
        repo_path.to_string_lossy().to_string(),
    );
    project.id = project_id.clone();
    project.base_branch = Some("main".to_string());
    // Place worktrees under the temp dir so they are auto-cleaned on drop.
    project.worktree_parent_directory = Some(repo_path.to_string_lossy().to_string());

    // Build a task with a stale merge-prefixed worktree_path.
    let mut task = Task::new(project_id.clone(), "WTR restore test".to_string());
    let task_id_str = task.id.as_str().to_string();
    task.task_branch = Some(git_repo.task_branch.clone());
    task.worktree_path = Some(format!("/tmp/nonexistent/merge-{}", task_id_str));

    // Precondition: the stale path must not exist.
    assert!(
        !std::path::Path::new(task.worktree_path.as_deref().unwrap()).exists(),
        "Precondition: merge-prefixed worktree path must not exist before restoration"
    );

    // Compute what the expected restored path should be.
    let expected_path_str = compute_task_worktree_path(&project, &task_id_str);

    // Call restore_task_worktree — it should detect the task branch and create the worktree.
    let result = restore_task_worktree(&mut task, &project, repo_path).await;

    assert!(
        result.is_ok(),
        "restore_task_worktree should succeed when task branch exists. Got: {:?}",
        result.err()
    );

    let returned_path = result.unwrap();

    // The returned PathBuf must exist on disk.
    assert!(
        returned_path.exists(),
        "Restored worktree path must exist on disk after restoration. Path: {}",
        returned_path.display()
    );

    // task.worktree_path must have been updated in memory.
    assert_eq!(
        task.worktree_path.as_deref(),
        Some(expected_path_str.as_str()),
        "task.worktree_path must be updated to the task-{{id}} path after restoration"
    );

    // Must no longer be a merge-prefixed path.
    assert!(
        !is_merge_worktree_path(task.worktree_path.as_deref().unwrap()),
        "Restored worktree_path must not have a merge-pipeline prefix. Got: {:?}",
        task.worktree_path
    );

    // Cleanup: remove the git worktree that was created so the TempDir can be removed cleanly.
    let _ = std::process::Command::new("git")
        .args([
            "worktree",
            "remove",
            "--force",
            returned_path.to_str().unwrap(),
        ])
        .current_dir(repo_path)
        .output();
}

// ──────────────────────────────────────────────────────────────────────────────
// Shared helper: build a TaskTransitionService from AppState (mirrors freshness_return_path tests)
// ──────────────────────────────────────────────────────────────────────────────

fn build_transition_service(
    app_state: &AppState,
) -> crate::application::TaskTransitionService<tauri::Wry> {
    let execution_state = Arc::new(ExecutionState::new());
    let message_queue = Arc::new(MessageQueue::new());
    let running_registry = Arc::new(MemoryRunningAgentRegistry::new());

    crate::application::TaskTransitionService::new(
        Arc::clone(&app_state.task_repo),
        Arc::clone(&app_state.task_dependency_repo),
        Arc::clone(&app_state.project_repo),
        Arc::clone(&app_state.chat_message_repo),
        Arc::clone(&app_state.chat_attachment_repo),
        Arc::clone(&app_state.chat_conversation_repo),
        Arc::clone(&app_state.agent_run_repo),
        Arc::clone(&app_state.ideation_session_repo),
        Arc::clone(&app_state.activity_event_repo),
        message_queue,
        running_registry,
        execution_state,
        None,
        Arc::clone(&app_state.memory_event_repo),
    )
}

// ──────────────────────────────────────────────────────────────────────────────
// Test 2: L1 — merging_to_pending_review_worktree_path_reset
//
// The L1 fix in task_transition_service.rs runs inside apply_corrective_transition
// on the reviewing_origin path. It detects a stale merge-prefixed worktree_path and
// calls restore_task_worktree before the transition to PendingReview completes.
//
// This test exercises the exact same code path by:
//   1. Seeding a task in Merging with freshness metadata (origin = "reviewing") and a
//      stale merge-prefixed worktree_path. The reviewing_origin flag is derived from
//      freshness_origin_state == "reviewing" in apply_corrective_transition.
//   2. Simulating what handle_freshness_return_routing does: clear freshness metadata,
//      then call transition_task(PendingReview) via the real TaskTransitionService.
//   3. After the transition, asserting that worktree_path is no longer merge-prefixed.
//
// Uses a real git repo so restore_task_worktree can actually recreate the worktree.
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn merging_to_pending_review_worktree_path_reset() {
    let git_repo = setup_real_git_repo();
    let path = git_repo.path();

    let app_state = AppState::new_test();

    // Wire a real project pointing at the git repo.
    // Place worktrees under the temp dir so they are cleaned up on drop.
    let mut project = Project::new(
        "wt-restore-l1-project".to_string(),
        path.to_string_lossy().to_string(),
    );
    project.base_branch = Some("main".to_string());
    project.worktree_parent_directory = Some(path.to_string_lossy().to_string());
    let project_id = project.id.clone();
    app_state.project_repo.create(project).await.unwrap();

    // Build a task in Merging state with:
    //   - freshness metadata: reviewing origin + source_update_conflict = true
    //   - stale merge-prefixed worktree_path
    let meta = serde_json::json!({
        "branch_freshness_conflict": true,
        "freshness_origin_state": "reviewing",
        "freshness_conflict_count": 1,
        "source_update_conflict": true,
        "source_branch": &git_repo.task_branch,
        "target_branch": "main",
    });
    let mut task = Task::new(project_id.clone(), "L1 worktree restore test".to_string());
    let task_id = task.id.clone();
    let task_id_str = task_id.as_str().to_string();
    task.internal_status = InternalStatus::Merging;
    task.task_branch = Some(git_repo.task_branch.clone());
    task.worktree_path = Some(format!("/nonexistent/merge-{}", task_id_str));
    task.metadata = Some(meta.to_string());
    app_state.task_repo.create(task).await.unwrap();

    // Step 1: Simulate the metadata-clear step that handle_freshness_return_routing does.
    // (The L1 restore happens DURING apply_corrective_transition, not before the transition
    //  call — but the freshness metadata must be cleared so the auto-transition to Reviewing
    //  doesn't re-enter the freshness loop.)
    let mut stored = app_state
        .task_repo
        .get_by_id(&task_id)
        .await
        .unwrap()
        .expect("task must exist");
    let mut meta_val: serde_json::Value =
        serde_json::from_str(stored.metadata.as_deref().unwrap_or("{}")).unwrap();
    crate::domain::state_machine::transition_handler::freshness::FreshnessMetadata::clear_from(
        &mut meta_val,
    );
    stored.metadata = Some(meta_val.to_string());
    stored.touch();
    app_state.task_repo.update(&stored).await.unwrap();

    // Step 2: Call transition_task(PendingReview) via TaskTransitionService.
    // This exercises the L1 restore path in apply_corrective_transition.
    let service = build_transition_service(&app_state);
    let result = service
        .transition_task(&task_id, InternalStatus::PendingReview)
        .await;

    assert!(
        result.is_ok(),
        "transition_task to PendingReview must succeed: {:?}",
        result.err()
    );

    // Step 3: Verify the task's worktree_path is no longer merge-prefixed.
    let updated = app_state
        .task_repo
        .get_by_id(&task_id)
        .await
        .unwrap()
        .expect("Task must still exist");

    if let Some(ref wt) = updated.worktree_path {
        assert!(
            !is_merge_worktree_path(wt),
            "After L1 transition, worktree_path must not have a merge-pipeline prefix. Got: {}",
            wt
        );
        assert!(
            wt.contains(&format!("task-{}", task_id_str)),
            "Restored worktree_path should reference task-{{id}} pattern. Got: {}",
            wt
        );
    }
    // If None: ReviewWorktreeMissing was hit (branch/worktree absent) and the caller set it to None.
    // Both None and a non-merge path are acceptable postconditions for L1.
}

// ──────────────────────────────────────────────────────────────────────────────
// Test 3: Freshness return to execution restores stale merge-prefixed worktree_path
//
// Covers the execution-origin path:
//   Merging + plan_update_conflict + freshness_origin_state="executing"
//   → freshness_return_route() → Ready
//
// Regression: without this repair, the task could return to Ready with
// worktree_path still pointing at merge-{id}, and the next Executing spawn would
// fail with "context points to merge worktree".
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn freshness_return_to_ready_restores_merge_prefixed_worktree_path() {
    let git_repo = setup_real_git_repo();
    let path = git_repo.path();

    let app_state = AppState::new_test();

    let mut project = Project::new(
        "wt-restore-execution-origin-project".to_string(),
        path.to_string_lossy().to_string(),
    );
    project.base_branch = Some("main".to_string());
    project.worktree_parent_directory = Some(path.to_string_lossy().to_string());
    let project_id = project.id.clone();
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let mut task = Task::new(
        project_id.clone(),
        "Execution-origin worktree restore test".to_string(),
    );
    let task_id = task.id.clone();
    let task_id_str = task_id.as_str().to_string();
    task.internal_status = InternalStatus::Merging;
    task.task_branch = Some(git_repo.task_branch.clone());
    task.worktree_path = Some(format!("/nonexistent/merge-{}", task_id_str));
    task.metadata = Some(
        serde_json::json!({
            "plan_update_conflict": true,
            "branch_freshness_conflict": true,
            "freshness_origin_state": "executing",
            "freshness_conflict_count": 1,
        })
        .to_string(),
    );
    app_state.task_repo.create(task).await.unwrap();

    let service = build_transition_service(&app_state);
    let current_task = app_state
        .task_repo
        .get_by_id(&task_id)
        .await
        .unwrap()
        .expect("task must exist");

    let result = freshness_return_route(
        &current_task,
        Arc::clone(&app_state.task_repo),
        &service,
        &project,
        None,
    )
    .await
    .expect("freshness return to execution must succeed");

    match result {
        FreshnessRouteResult::FreshnessRouted(state) => assert_eq!(state, "executing"),
        FreshnessRouteResult::NormalMerge => panic!("Expected FreshnessRouted"),
    }

    let updated = app_state
        .task_repo
        .get_by_id(&task_id)
        .await
        .unwrap()
        .expect("task must exist after routing");
    assert_eq!(updated.internal_status, InternalStatus::Ready);

    let restored_wt = updated
        .worktree_path
        .as_deref()
        .expect("execution-origin routing should restore task worktree_path");
    assert!(
        !is_merge_worktree_path(restored_wt),
        "Execution-origin routing must not leave a merge-prefixed worktree_path. Got: {}",
        restored_wt
    );
    assert!(
        restored_wt.contains(&format!("task-{}", task_id_str)),
        "Restored execution worktree_path should reference task-{{id}}. Got: {}",
        restored_wt
    );
    assert!(
        std::path::Path::new(restored_wt).exists(),
        "Restored execution worktree should exist on disk. Path: {}",
        restored_wt
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Test 3: L2 — on_enter_reviewing_restores_merge_prefixed_worktree
//
// on_enter(Reviewing) in on_enter_states.rs detects that worktree_path has a
// merge-prefix, calls restore_task_worktree, and persists the corrected path to
// the task repo before the reviewer spawn attempt.
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn on_enter_reviewing_restores_merge_prefixed_worktree() {
    let git_repo = setup_real_git_repo();
    let path = git_repo.path();

    let task_repo = Arc::new(MemoryTaskRepository::new());
    let project_repo = Arc::new(MemoryProjectRepository::new());

    let project_id = ProjectId::from_string("proj-1".to_string());

    // Build a task in PendingReview with a stale merge-prefixed worktree_path.
    let mut task = Task::new(project_id.clone(), "L2 reviewing restore test".to_string());
    let task_id = task.id.clone();
    let task_id_str = task_id.as_str().to_string();
    task.internal_status = InternalStatus::PendingReview;
    task.task_branch = Some(git_repo.task_branch.clone());
    task.worktree_path = Some(format!("/nonexistent/merge-{}", task_id_str));
    task_repo.create(task).await.unwrap();

    let mut project = Project::new(
        "test-project".to_string(),
        path.to_string_lossy().to_string(),
    );
    project.id = project_id;
    project.base_branch = Some("main".to_string());
    // Place worktrees under the temp dir for cleanup.
    project.worktree_parent_directory = Some(path.to_string_lossy().to_string());
    project_repo.create(project).await.unwrap();

    let (_, services) =
        make_services_with_tracked_chat(Arc::clone(&task_repo), Arc::clone(&project_repo));
    let context = crate::domain::state_machine::context::TaskContext::new(
        task_id_str.as_str(),
        "proj-1",
        services,
    );
    let mut machine = crate::domain::state_machine::TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    // Call on_enter(Reviewing) — the guard detects the merge-prefix and calls
    // restore_task_worktree before the reviewer spawn attempt.
    let _ = handler.on_enter(&State::Reviewing).await;

    // Allow the async path to write the restored path back to the repo.
    let task_repo_poll = Arc::clone(&task_repo);
    let task_id_poll = task_id.clone();
    let settled = wait_for_condition(
        || {
            let r = Arc::clone(&task_repo_poll);
            let id = task_id_poll.clone();
            async move {
                r.get_by_id(&id)
                    .await
                    .ok()
                    .flatten()
                    .map(|t| {
                        // Condition: worktree_path no longer has a merge-pipeline prefix.
                        t.worktree_path
                            .as_deref()
                            .map(|wt| !is_merge_worktree_path(wt))
                            .unwrap_or(true) // None is also acceptable (missing worktree cleared it)
                    })
                    .unwrap_or(false)
            }
        },
        5000,
    )
    .await;

    assert!(
        settled,
        "Task worktree_path should no longer have a merge-pipeline prefix within 5s"
    );

    let updated = task_repo
        .get_by_id(&task_id)
        .await
        .unwrap()
        .expect("Task must still exist");

    // Validate the path is no longer merge-prefixed.
    if let Some(ref wt) = updated.worktree_path {
        assert!(
            !is_merge_worktree_path(wt),
            "After on_enter(Reviewing), worktree_path must not have a merge-pipeline prefix. Got: {}",
            wt
        );
    }
    // If worktree_path is None, ReviewWorktreeMissing was returned and the caller cleared it — acceptable.
}
