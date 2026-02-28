// Merge completion: finalize merge and cleanup branch/worktree
//
// Extracted from side_effects.rs — handles post-merge finalization and resource cleanup.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::{AppHandle, Emitter};

use crate::application::GitService;
use crate::infrastructure::agents::claude::git_runtime_config;
use crate::domain::entities::{
    merge_progress_event::{MergePhase, MergePhaseStatus},
    task_metadata::{
        MergeRecoveryEvent, MergeRecoveryEventKind, MergeRecoveryMetadata, MergeRecoveryReasonCode,
        MergeRecoverySource, MergeRecoveryState,
    },
    InternalStatus, Project, Task,
};
use crate::domain::repositories::TaskRepository;
use crate::error::{AppError, AppResult};

use super::merge_validation::emit_merge_progress;

/// Complete a merge operation by transitioning task to Merged and cleaning up.
///
/// This is shared logic used by:
/// - Programmatic merge success path (PendingMerge side effect)
/// - Merge auto-completion on agent exit (Phase 76)
/// - complete_merge HTTP handler (backwards compatibility)
///
/// # Arguments
/// * `task` - Mutable task to update (must be in appropriate state)
/// * `project` - Project for branch/worktree cleanup info
/// * `commit_sha` - The merge commit SHA (must be on target_branch)
/// * `target_branch` - The branch the merge was supposed to happen on
/// * `task_repo` - Repository to persist task changes
/// * `app_handle` - Optional Tauri handle for emitting events
///
/// # Side Effects
/// 1. Updates task.merge_commit_sha
/// 2. Updates task.internal_status to Merged
/// 3. Persists status change to history
/// 4. Deletes worktree (if Worktree mode)
/// 5. Deletes task branch
/// 6. Emits task:merged and task:status_changed events
///
/// # Errors
/// Returns `AppError::Validation` if the commit is not on the target branch.
/// Returns `AppError::GitOperation` if git verification itself fails (protects against
/// ghost merges — setting Merged status without confirmation is a data integrity error).
pub async fn complete_merge_internal<R: tauri::Runtime>(
    task: &mut Task,
    project: &Project,
    commit_sha: &str,
    target_branch: &str,
    task_repo: &Arc<dyn TaskRepository>,
    app_handle: Option<&AppHandle<R>>,
) -> AppResult<()> {
    // Clone task_id early to avoid borrow conflicts with mutable task
    let task_id = task.id.clone();
    let task_id_str = task_id.as_str();
    let old_status = task.internal_status.clone();
    let repo_path = Path::new(&project.working_directory);

    // VERIFY: Commit must be on target branch to prevent false merges
    match GitService::is_commit_on_branch(repo_path, commit_sha, target_branch).await {
        Ok(true) => {
            tracing::debug!(
                task_id = task_id_str,
                commit_sha = %commit_sha,
                target_branch = %target_branch,
                "complete_merge_internal: commit verified on target branch"
            );
        }
        Ok(false) => {
            tracing::error!(
                task_id = task_id_str,
                commit_sha = %commit_sha,
                target_branch = %target_branch,
                "complete_merge_internal: commit NOT on target branch - rejecting false merge"
            );
            return Err(AppError::Validation(format!(
                "Commit {} is not on target branch {} - merge verification failed",
                commit_sha, target_branch
            )));
        }
        Err(e) => {
            // Fatal: git verification failed — we cannot confirm the merge succeeded.
            // Setting Merged status without verification risks data corruption (ghost merge).
            // The caller will handle the error; reconciliation will retry the merge.
            tracing::error!(
                task_id = task_id_str,
                error = %e,
                commit_sha = %commit_sha,
                target_branch = %target_branch,
                "complete_merge_internal: git verification failed — rejecting Merged \
                 status to protect data integrity"
            );
            return Err(AppError::GitOperation(format!(
                "Cannot confirm merge: git verification of commit {} on branch {} failed: {}",
                commit_sha, target_branch, e
            )));
        }
    }

    tracing::info!(
        task_id = task_id_str,
        commit_sha = %commit_sha,
        old_status = ?old_status,
        "complete_merge_internal: completing merge"
    );

    // Emit finalize merge progress event
    emit_merge_progress(
        app_handle,
        task_id_str,
        MergePhase::finalize(),
        MergePhaseStatus::Started,
        "Finalizing merge and cleaning up".to_string(),
    );

    // 1. Append attempt_succeeded event to merge recovery metadata
    let mut recovery = MergeRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
        .unwrap_or(None)
        .unwrap_or_else(MergeRecoveryMetadata::new);

    // Count total retry attempts
    let attempt_count = recovery
        .events
        .iter()
        .filter(|e| matches!(e.kind, MergeRecoveryEventKind::AutoRetryTriggered))
        .count() as u32
        + 1;

    let success_event = MergeRecoveryEvent::new(
        MergeRecoveryEventKind::AttemptSucceeded,
        MergeRecoverySource::System,
        MergeRecoveryReasonCode::Unknown,
        format!("Merge completed successfully with commit {}", commit_sha),
    )
    .with_attempt(attempt_count);

    recovery.append_event_with_state(success_event, MergeRecoveryState::Succeeded);

    // Update task metadata
    if let Ok(updated_json) = recovery.update_task_metadata(task.metadata.as_deref()) {
        task.metadata = Some(updated_json);
    } else {
        tracing::warn!(
            task_id = task_id_str,
            "Failed to serialize merge recovery metadata on success (non-fatal)"
        );
    }

    // 2. Update task with merge commit SHA and status
    task.merge_commit_sha = Some(commit_sha.to_string());
    task.internal_status = InternalStatus::Merged;
    task.touch();

    task_repo.update(task).await.map_err(|e| {
        tracing::error!(error = %e, task_id = task_id_str, "Failed to update task with merge_commit_sha");
        e
    })?;

    // 2. Record status change in history
    if let Err(e) = task_repo
        .persist_status_change(
            &task_id,
            old_status.clone(),
            InternalStatus::Merged,
            "merge_success",
        )
        .await
    {
        tracing::warn!(error = %e, task_id = task_id_str, "Failed to record merge transition (non-fatal)");
    }

    // 3. Cleanup branch and worktree
    cleanup_branch_and_worktree_internal(task, project, task_repo).await;

    // 4. Emit events (intentional: no frontend listeners is OK)
    if let Some(handle) = app_handle {
        let _ = handle.emit(
            "task:merged",
            serde_json::json!({
                "task_id": task_id_str,
                "commit_sha": commit_sha,
            }),
        );
        let _ = handle.emit(
            "task:status_changed",
            serde_json::json!({
                "task_id": task_id_str,
                "old_status": old_status.as_str(),
                "new_status": "merged",
            }),
        );
        let _ = handle.emit(
            "merge:completed",
            serde_json::json!({
                "task_id": task_id_str,
                "commit_sha": commit_sha,
            }),
        );
    }

    // Emit finalize success merge progress event
    emit_merge_progress(
        app_handle,
        task_id_str,
        MergePhase::finalize(),
        MergePhaseStatus::Passed,
        format!("Merge finalized successfully: {}", commit_sha),
    );

    // Clean up in-memory merge progress hydration store
    crate::domain::entities::merge_progress_event::clear_merge_progress(task_id_str);

    // Clean up validation log files from disk
    super::merge_validation::cleanup_validation_logs(task_id_str);

    tracing::info!(
        task_id = task_id_str,
        commit_sha = %commit_sha,
        "complete_merge_internal: merge completed successfully"
    );

    Ok(())
}

/// Cleanup task branch and worktree after successful merge (standalone version).
///
/// This is the standalone version that can be called from `complete_merge_internal`.
/// For use within TransitionHandler, use the async method which has access to services.
///
/// After deleting the git branch and worktree, clears `task.task_branch` and
/// `task.worktree_path` to `None` and persists the update. This prevents a
/// reopened task from seeing stale (now-deleted) branch/worktree values and
/// skipping branch setup on re-execution.
pub(super) async fn cleanup_branch_and_worktree_internal(
    task: &mut Task,
    project: &Project,
    task_repo: &Arc<dyn TaskRepository>,
) {
    let task_id_str = task.id.as_str().to_string();

    let Some(ref task_branch) = task.task_branch.clone() else {
        tracing::debug!(task_id = task_id_str, "No branch to cleanup");
        return;
    };

    let repo_path = Path::new(&project.working_directory);

    // Delete worktree first, then branch
    if let Some(ref worktree_path) = task.worktree_path.clone() {
        let worktree_path_buf = PathBuf::from(worktree_path);

        // Defence-in-depth: if the worktree is still in active use (e.g. validation
        // running via a WorktreePermit), log a warning. The kill_worktree_processes
        // call below + CancellationToken (Fix 4) should have already stopped the work,
        // but if not, this warning surfaces a potential race condition.
        if crate::domain::services::is_worktree_in_use(&worktree_path_buf) {
            tracing::warn!(
                task_id = task_id_str,
                worktree = %worktree_path,
                "Worktree is still marked as in-use (WorktreePermit held) — proceeding with cleanup"
            );
        }

        // Kill any lingering processes with files open in the worktree
        // (prevents race where a validation subprocess from a prior attempt
        // still holds files, causing delete_worktree to fail or corrupt state).
        // Matches pre_merge_cleanup step 0 pattern (merge_coordination.rs).
        if worktree_path_buf.exists() {
            let lsof_timeout = git_runtime_config().worktree_lsof_timeout_secs;
            crate::domain::services::kill_worktree_processes_async(
                &worktree_path_buf,
                lsof_timeout,
            )
            .await;
            // Brief settle time for process tree cleanup after SIGTERM
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        if !worktree_path_buf.exists() {
            tracing::info!(
                task_id = task_id_str,
                worktree = %worktree_path,
                "worktree already removed, skipping deletion"
            );
        } else {
            match GitService::delete_worktree(repo_path, &worktree_path_buf).await {
                Ok(_) => {
                    tracing::info!(
                        task_id = task_id_str,
                        worktree = %worktree_path,
                        "Deleted worktree after merge"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        task_id = task_id_str,
                        worktree = %worktree_path,
                        "Failed to delete worktree (non-fatal)"
                    );
                }
            }
        }
    }

    // Delete the branch from main repo.
    // The branch is no longer checked out in any worktree, so force-delete works
    // without needing to checkout a different branch in the main repo.
    match GitService::delete_branch(repo_path, task_branch, true).await {
        Ok(_) => {
            tracing::info!(
                task_id = task_id_str,
                branch = %task_branch,
                "Deleted task branch after merge"
            );
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                task_id = task_id_str,
                branch = %task_branch,
                "Failed to delete task branch (non-fatal)"
            );
        }
    }

    // Clear stale branch/worktree fields so a reopened task doesn't see deleted values.
    // on_enter(Executing) checks task.task_branch.is_some() to decide whether to skip
    // branch setup — if these are left set, re-execution will try to use a deleted branch.
    task.task_branch = None;
    task.worktree_path = None;
    task.touch();
    if let Err(e) = task_repo.update(task).await {
        tracing::warn!(
            error = %e,
            task_id = task_id_str,
            "Failed to clear task_branch/worktree_path after cleanup (non-fatal)"
        );
    } else {
        tracing::info!(
            task_id = task_id_str,
            "Cleared task_branch and worktree_path after merge cleanup"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{InternalStatus, MergeStrategy, Project, ProjectId, Task};
    use crate::infrastructure::memory::MemoryTaskRepository;

    fn make_test_repo() -> (tempfile::TempDir, String) {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let path = dir.path();
        for args in [
            vec!["init", "-b", "main"],
            vec!["config", "user.email", "t@t.com"],
            vec!["config", "user.name", "T"],
        ] {
            let _ = std::process::Command::new("git")
                .args(&args)
                .current_dir(path)
                .output();
        }
        std::fs::write(path.join("README.md"), "# test").unwrap();
        for args in [vec!["add", "."], vec!["commit", "-m", "init"]] {
            let _ = std::process::Command::new("git")
                .args(&args)
                .current_dir(path)
                .output();
        }
        let path_str = path.to_string_lossy().to_string();
        (dir, path_str)
    }

    /// V2 fix: complete_merge_internal must return Err when git verification fails,
    /// NOT fall through to set Merged status.
    ///
    /// Before the fix: Err from is_commit_on_branch was treated as non-fatal, and
    /// the function proceeded to set task.internal_status = Merged. This allowed
    /// ghost merges when git verification was unavailable or errored.
    ///
    /// After the fix: Err returns AppError::GitOperation — task stays in its prior
    /// state, reconciliation retries, data integrity is preserved.
    #[tokio::test]
    async fn complete_merge_internal_returns_err_when_git_verification_fails() {
        let (_dir, repo_path_str) = make_test_repo();

        let task_repo = Arc::new(MemoryTaskRepository::new());
        let project_id = ProjectId::from_string("proj-v2".to_string());

        let mut task = Task::new(project_id.clone(), "V2 test task".to_string());
        task.internal_status = InternalStatus::PendingMerge;
        let _task_id = task.id.clone();
        task_repo.create(task.clone()).await.unwrap();

        let mut project = Project::new("v2-project".to_string(), repo_path_str);
        project.id = project_id;
        project.base_branch = Some("main".to_string());
        project.merge_strategy = MergeStrategy::Merge;

        // Pass an INVALID commit SHA — git verification will return Err (not Ok(false)),
        // because `git merge-base --is-ancestor invalid_sha main` exits with a non-0/1 code.
        let invalid_sha = "0000000000000000000000000000000000000000";
        let task_repo_arc: Arc<dyn TaskRepository> = Arc::new(MemoryTaskRepository::new());
        task_repo_arc.create(task.clone()).await.unwrap();

        let result = complete_merge_internal::<tauri::Wry>(
            &mut task,
            &project,
            invalid_sha,
            "main",
            &task_repo_arc,
            None,
        )
        .await;

        // Must return Err — git verification failed
        assert!(
            result.is_err(),
            "complete_merge_internal must return Err when git verification fails (V2 fix). \
             Got Ok(()) which means Merged status was set without confirmation."
        );

        // Task status must NOT have been updated to Merged
        assert_ne!(
            task.internal_status,
            InternalStatus::Merged,
            "Task internal_status must NOT be Merged when git verification fails. \
             Got {:?}",
            task.internal_status,
        );
    }

    /// Fix #3: When pre_merge_cleanup already deleted the worktree,
    /// cleanup_branch_and_worktree_internal should skip worktree deletion
    /// but still delete the branch and clear task fields.
    #[tokio::test]
    async fn test_cleanup_branch_and_worktree_skips_already_deleted_worktree() {
        let (_dir, repo_path_str) = make_test_repo();
        let repo_path = Path::new(&repo_path_str);

        // Create a task branch, then return to main so force-delete works
        let branch_name = "task/fix3-test";
        let _ = std::process::Command::new("git")
            .args(["checkout", "-b", branch_name])
            .current_dir(repo_path)
            .output();
        let _ = std::process::Command::new("git")
            .args(["checkout", "main"])
            .current_dir(repo_path)
            .output();

        // Verify branch exists before cleanup
        let branch_check = std::process::Command::new("git")
            .args(["branch", "--list", branch_name])
            .current_dir(repo_path)
            .output()
            .unwrap();
        assert!(
            String::from_utf8_lossy(&branch_check.stdout).contains(branch_name),
            "Branch should exist before cleanup"
        );

        let task_repo: Arc<dyn TaskRepository> = Arc::new(MemoryTaskRepository::new());
        let project_id = ProjectId::from_string("proj-fix3".to_string());

        let mut task = Task::new(project_id.clone(), "Fix3 test".to_string());
        task.internal_status = InternalStatus::Merged;
        task.task_branch = Some(branch_name.to_string());
        // Non-existent worktree path — simulates pre_merge_cleanup already deleted it
        task.worktree_path = Some("/tmp/nonexistent-worktree-fix3-test".to_string());
        task_repo.create(task.clone()).await.unwrap();

        let mut project = Project::new("fix3-project".to_string(), repo_path_str.clone());
        project.id = project_id;
        project.base_branch = Some("main".to_string());

        cleanup_branch_and_worktree_internal(&mut task, &project, &task_repo).await;

        // Both fields must be cleared after cleanup
        assert!(
            task.worktree_path.is_none(),
            "worktree_path should be None after cleanup (was already deleted)"
        );
        assert!(
            task.task_branch.is_none(),
            "task_branch should be None after cleanup"
        );

        // Branch must be deleted from the real git repo
        let branch_check_after = std::process::Command::new("git")
            .args(["branch", "--list", branch_name])
            .current_dir(repo_path)
            .output()
            .unwrap();
        assert!(
            !String::from_utf8_lossy(&branch_check_after.stdout).contains(branch_name),
            "Branch should be deleted from git after cleanup"
        );
    }
}
