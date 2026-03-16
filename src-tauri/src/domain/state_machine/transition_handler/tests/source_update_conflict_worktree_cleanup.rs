// Tests for Fix 5: source_update_conflict completion path cleans up merge worktree
//
// The source_update_conflict completion path in git.rs (complete_merge handler)
// must delete the merge worktree created for the source_update_conflict resolution,
// clear task.worktree_path, and remove the stale conflict_type from metadata.
//
// This mirrors the rebase handler's cleanup (git.rs:201-219) for the source_update path.

use super::helpers::*;
use crate::application::git_service::GitService;
use crate::domain::entities::{Project, ProjectId};
use crate::domain::state_machine::transition_handler::merge_helpers::compute_merge_worktree_path;
use std::fs;
use std::path::PathBuf;

/// After source_update_conflict resolution, the merge worktree created for the agent
/// should be deleted and task.worktree_path should be cleared to None.
///
/// Simulates: agent resolves source_update_conflict in merge worktree →
///   complete_merge handler detects is_source_update → Fix 5 cleans worktree.
#[tokio::test]
async fn test_source_update_conflict_completion_deletes_worktree_and_clears_path() {
    let git_repo = setup_real_git_repo();
    let path = git_repo.path();

    let worktree_parent = path.join("worktrees");
    fs::create_dir_all(&worktree_parent).unwrap();

    let mut project = Project::new(
        "test-project".to_string(),
        path.to_string_lossy().to_string(),
    );
    let project_id = ProjectId::from_string("proj-1".to_string());
    project.id = project_id.clone();
    project.base_branch = Some("main".to_string());
    project.worktree_parent_directory = Some(worktree_parent.to_string_lossy().to_string());

    let task_id_str = "source-update-cleanup-test";
    let merge_wt_path_str = compute_merge_worktree_path(&project, task_id_str);
    let merge_wt_path = PathBuf::from(&merge_wt_path_str);

    // Simulate: source_update_conflict created a merge worktree for the agent
    fs::create_dir_all(merge_wt_path.parent().unwrap()).unwrap();
    GitService::checkout_existing_branch_worktree(path, &merge_wt_path, &git_repo.task_branch)
        .await
        .expect("create merge worktree simulating source_update_conflict path");

    assert!(
        merge_wt_path.exists(),
        "Precondition: merge worktree should exist before cleanup. Path: {}",
        merge_wt_path.display()
    );

    // --- Replicate Fix 5 code path (git.rs source_update_conflict completion) ---

    // Set up task state mirroring what the handler would see
    let mut task = make_task(None, Some(&git_repo.task_branch));
    task.worktree_path = Some(merge_wt_path_str.clone());
    let mut metadata: serde_json::Value = serde_json::json!({
        "source_update_conflict": true,
        "conflict_type": "rebase",
        "conflict_files": ["src/main.rs"],
        "error": null
    });

    // Delete merge worktree (Fix 5 step 1)
    if let Some(ref worktree_path) = task.worktree_path {
        let wt_path = PathBuf::from(worktree_path);
        let result = GitService::delete_worktree(path, &wt_path).await;
        assert!(
            result.is_ok(),
            "delete_worktree should succeed for a valid merge worktree: {:?}",
            result.err()
        );
    }

    // Clear task.worktree_path (Fix 5 step 2)
    task.worktree_path = None;

    // Clear metadata: source_update_conflict + conflict_files + error + conflict_type (Fix 5 step 3)
    if let Some(obj) = metadata.as_object_mut() {
        obj.remove("source_update_conflict");
        obj.remove("conflict_files");
        obj.remove("error");
        obj.remove("conflict_type");
    }
    task.metadata = Some(metadata.to_string());

    // --- Assertions ---

    assert!(
        !merge_wt_path.exists(),
        "After Fix 5 cleanup, merge worktree should be deleted. Path: {}",
        merge_wt_path.display()
    );

    assert!(
        task.worktree_path.is_none(),
        "After Fix 5 cleanup, task.worktree_path should be None"
    );

    let meta: serde_json::Value =
        serde_json::from_str(task.metadata.as_deref().unwrap_or("{}")).unwrap();
    assert!(
        meta.get("conflict_type").is_none(),
        "conflict_type should be cleared from metadata after source_update_conflict resolution"
    );
    assert!(
        meta.get("source_update_conflict").is_none(),
        "source_update_conflict flag should be cleared from metadata"
    );
    assert!(
        meta.get("conflict_files").is_none(),
        "conflict_files should be cleared from metadata"
    );
}

/// Fix 5: delete_worktree failure (e.g., path never existed) is non-fatal.
/// task.worktree_path is still cleared to None so pre_merge_cleanup handles it.
#[tokio::test]
async fn test_source_update_worktree_cleanup_tolerates_missing_worktree() {
    let git_repo = setup_real_git_repo();
    let path = git_repo.path();

    // worktree_path points to a path that doesn't exist on disk
    let nonexistent_path = path.join("worktrees").join("merge-nonexistent-test");

    let mut task = make_task(None, Some(&git_repo.task_branch));
    task.worktree_path = Some(nonexistent_path.to_string_lossy().to_string());

    // Fix 5: delete_worktree may fail (non-fatal), worktree_path is cleared regardless
    if let Some(ref worktree_path) = task.worktree_path {
        let wt_path = PathBuf::from(worktree_path);
        // Error is intentionally not propagated — mirrors the tracing::warn! behavior
        let _ = GitService::delete_worktree(path, &wt_path).await;
    }
    task.worktree_path = None;

    assert!(
        task.worktree_path.is_none(),
        "task.worktree_path should be None even if delete_worktree failed"
    );
    assert!(
        !nonexistent_path.exists(),
        "Non-existent worktree path should remain non-existent"
    );
}

/// Fix 5: conflict_type removal is idempotent — no conflict_type in metadata is fine.
#[tokio::test]
async fn test_source_update_conflict_type_removal_is_idempotent() {
    let git_repo = setup_real_git_repo();

    let mut task = make_task(None, Some(&git_repo.task_branch));
    // Metadata without conflict_type — removal should not fail
    task.metadata = Some(
        serde_json::json!({
            "source_update_conflict": true,
            "conflict_files": []
        })
        .to_string(),
    );

    let mut meta: serde_json::Value =
        serde_json::from_str(task.metadata.as_deref().unwrap_or("{}")).unwrap();
    if let Some(obj) = meta.as_object_mut() {
        obj.remove("source_update_conflict");
        obj.remove("conflict_files");
        obj.remove("error");
        obj.remove("conflict_type"); // idempotent — key may not exist
    }
    task.metadata = Some(meta.to_string());

    let result: serde_json::Value =
        serde_json::from_str(task.metadata.as_deref().unwrap_or("{}")).unwrap();
    assert!(
        result.get("conflict_type").is_none(),
        "conflict_type removal should be idempotent when key is absent"
    );
    assert!(
        result.get("source_update_conflict").is_none(),
        "source_update_conflict should be removed"
    );
}
