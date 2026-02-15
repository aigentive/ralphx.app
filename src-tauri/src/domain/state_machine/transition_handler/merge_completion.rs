// Merge completion: finalize merge and cleanup branch/worktree
//
// Extracted from side_effects.rs — handles post-merge finalization and resource cleanup.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::{AppHandle, Emitter};

use crate::application::GitService;
use crate::domain::entities::{
    merge_progress_event::{MergePhase, MergePhaseStatus},
    task_metadata::{
        MergeRecoveryEvent, MergeRecoveryEventKind, MergeRecoveryMetadata, MergeRecoveryReasonCode,
        MergeRecoverySource, MergeRecoveryState,
    },
    GitMode, InternalStatus, Project, Task,
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
/// Returns `AppError::InvalidState` if the commit is not on the target branch.
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
    match GitService::is_commit_on_branch(repo_path, commit_sha, target_branch) {
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
            tracing::warn!(
                task_id = task_id_str,
                error = %e,
                commit_sha = %commit_sha,
                target_branch = %target_branch,
                "complete_merge_internal: failed to verify commit on target branch, proceeding (non-fatal)"
            );
            // Non-fatal: git verification failed, but we don't want to block legitimate merges
            // The caller has already verified the merge in most cases
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
        MergePhase::Finalize,
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
    cleanup_branch_and_worktree_internal(task, project);

    // 4. Emit events
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
        MergePhase::Finalize,
        MergePhaseStatus::Passed,
        format!("Merge finalized successfully: {}", commit_sha),
    );

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
pub(super) fn cleanup_branch_and_worktree_internal(task: &Task, project: &Project) {
    let task_id_str = task.id.as_str();

    let Some(ref task_branch) = task.task_branch else {
        tracing::debug!(task_id = task_id_str, "No branch to cleanup");
        return;
    };

    let repo_path = Path::new(&project.working_directory);

    match project.git_mode {
        GitMode::Local => {
            // For Local mode: already on base branch (from merge), just delete task branch
            match GitService::delete_branch(repo_path, task_branch, true) {
                Ok(_) => {
                    tracing::info!(
                        task_id = task_id_str,
                        branch = %task_branch,
                        "Deleted task branch after merge (Local mode)"
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
        }
        GitMode::Worktree => {
            // For Worktree mode: delete worktree first, then branch
            if let Some(ref worktree_path) = task.worktree_path {
                let worktree_path_buf = PathBuf::from(worktree_path);
                match GitService::delete_worktree(repo_path, &worktree_path_buf) {
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

            // Delete the branch from main repo.
            // The branch is no longer checked out in any worktree, so force-delete works
            // without needing to checkout a different branch in the main repo.
            match GitService::delete_branch(repo_path, task_branch, true) {
                Ok(_) => {
                    tracing::info!(
                        task_id = task_id_str,
                        branch = %task_branch,
                        "Deleted task branch after merge (Worktree mode)"
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
        }
    }
}
