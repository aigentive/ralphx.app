// Merge coordination helpers — deferral logic, plan branch management, and pre-merge cleanup.
//
// Extracted from side_effects.rs for maintainability.
// - ensure_plan_branch_exists: lazy git ref creation for plan merge targets
// - check_main_merge_deferral: defer main-branch merges until siblings terminal / agents idle
// - pre_merge_cleanup: remove debris from prior failed attempts before merge

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::application::GitService;
use crate::domain::entities::{
    merge_progress_event::{MergePhase, MergePhaseStatus},
    InternalStatus, PlanBranchStatus, Task,
};
use crate::domain::repositories::{PlanBranchRepository, TaskRepository};
use crate::infrastructure::agents::claude::{defer_merge_enabled, git_runtime_config};

use super::cleanup_helpers::run_cleanup_step;
use super::merge_helpers::{
    compute_merge_worktree_path, compute_rebase_worktree_path,
    extract_task_id_from_merge_path, is_task_in_merge_workflow,
};
use super::merge_validation::emit_merge_progress;

/// Ensure the plan branch exists as a git ref (lazy creation for merge target).
///
/// Handles the case where the plan branch DB record exists but the git branch
/// was never created (e.g., lazy creation failed at execution time).
pub(super) async fn ensure_plan_branch_exists(
    task: &Task,
    repo_path: &Path,
    target_branch: &str,
    plan_branch_repo: &Option<Arc<dyn PlanBranchRepository>>,
) {
    let Some(ref session_id) = task.ideation_session_id else {
        return;
    };
    let Some(ref pb_repo) = plan_branch_repo else {
        return;
    };
    let Ok(Some(pb)) = pb_repo.get_by_session_id(session_id).await else {
        return;
    };
    if pb.status != PlanBranchStatus::Active
        || pb.branch_name != target_branch
        || GitService::branch_exists(repo_path, target_branch).await
    {
        return;
    }

    let task_id_str = task.id.as_str();
    match GitService::create_feature_branch(repo_path, &pb.branch_name, &pb.source_branch).await {
        Ok(_) => {
            tracing::info!(
                task_id = task_id_str,
                branch = %pb.branch_name,
                source = %pb.source_branch,
                "Lazily created plan branch for merge target"
            );
        }
        Err(e) if GitService::branch_exists(repo_path, &pb.branch_name).await => {
            // Race: concurrent task created it between check and create
            let _ = e;
        }
        Err(e) => {
            tracing::warn!(
                task_id = task_id_str,
                error = %e,
                branch = %pb.branch_name,
                "Failed to lazily create plan branch for merge"
            );
        }
    }
}

/// Check if a main-branch merge should be deferred.
///
/// Returns `true` if the merge was deferred (caller should return early).
/// Defers when target is the base branch AND either:
/// 1. Sibling plan tasks are not all terminal
/// 2. Agents are still running (running_agent_count > 0)
pub(super) async fn check_main_merge_deferral(
    task: &mut Task,
    task_id_str: &str,
    source_branch: &str,
    target_branch: &str,
    base_branch: &str,
    task_repo: &Arc<dyn TaskRepository>,
    running_agent_count: Option<u32>,
    app_handle: Option<&tauri::AppHandle>,
) -> bool {
    if target_branch != base_branch || !defer_merge_enabled() {
        return false;
    }

    // Plan-level guard: all sibling tasks must be terminal before merging to main
    if let Some(ref session_id) = task.ideation_session_id {
        let siblings = task_repo
            .get_by_ideation_session(session_id)
            .await
            .unwrap_or_default();
        let all_siblings_terminal = siblings.iter().all(|t| {
            t.id == task.id
                || t.internal_status == InternalStatus::PendingMerge
                || t.is_terminal()
        });
        if !all_siblings_terminal {
            tracing::info!(
                task_id = task_id_str,
                session_id = %session_id,
                "Deferring main-branch merge: sibling plan tasks not yet terminal"
            );

            super::merge_helpers::set_main_merge_deferred_metadata(task);
            task.touch();

            if let Err(e) = task_repo.update(task).await {
                tracing::error!(error = %e, "Failed to set main_merge_deferred metadata");
                return true;
            }

            emit_merge_progress(
                app_handle,
                task_id_str,
                MergePhase::ProgrammaticMerge,
                MergePhaseStatus::Started,
                format!(
                    "Deferred merge to {} — waiting for sibling tasks to complete",
                    target_branch,
                ),
            );

            return true;
        }
    }

    if let Some(count) = running_agent_count {
        if count > 0 {
            tracing::info!(
                task_id = task_id_str,
                source_branch = %source_branch,
                target_branch = %target_branch,
                running_count = count,
                "Deferring main-branch merge: {} agents still running — \
                 merge will be retried when all agents complete",
                count
            );

            super::merge_helpers::set_main_merge_deferred_metadata(task);
            task.touch();

            if let Err(e) = task_repo.update(task).await {
                tracing::error!(error = %e, "Failed to set main_merge_deferred metadata");
                return true;
            }

            emit_merge_progress(
                app_handle,
                task_id_str,
                MergePhase::ProgrammaticMerge,
                MergePhaseStatus::Started,
                format!(
                    "Deferred merge to {} — waiting for {} agent(s) to complete",
                    target_branch, count
                ),
            );

            return true;
        }
    }

    false
}

impl<'a> super::TransitionHandler<'a> {
    /// Pre-merge cleanup: remove debris from any prior failed attempts and stale locks.
    ///
    /// Runs unconditionally on EVERY merge attempt (first or retry) so that transient
    /// failures from a previous run never block the current one.
    ///
    /// Steps:
    ///   0. Stop any running agents (review/merge) and kill worktree processes
    ///   1. Remove stale `.git/index.lock`
    ///   2. Delete the task worktree to unlock the task branch
    ///   3. Prune stale worktree references
    ///   4. Delete own merge/rebase worktrees from a prior attempt
    ///   5. Scan and remove orphaned merge worktrees targeting the same branch
    ///   6. Clean the working tree (git clean)
    pub(super) async fn pre_merge_cleanup(
        &self,
        task_id_str: &str,
        task: &crate::domain::entities::Task,
        project: &crate::domain::entities::Project,
        repo_path: &Path,
        target_branch: &str,
        task_repo: &Arc<dyn TaskRepository>,
    ) {
        // --- Step 0: Stop running agents and kill worktree processes ---
        // The reviewer (or merger from a prior attempt) may still be running IN the
        // task worktree. We must stop it BEFORE attempting worktree deletion to avoid
        // git lock contention that causes the 5+ minute hang (see merge-hang RCA).
        tracing::info!(task_id = task_id_str, "pre_merge_cleanup: step 0 — stopping any running agents");
        for ctx_type in [
            crate::domain::entities::ChatContextType::Review,
            crate::domain::entities::ChatContextType::Merge,
        ] {
            match self
                .machine
                .context
                .services
                .chat_service
                .stop_agent(ctx_type, task_id_str)
                .await
            {
                Ok(true) => {
                    tracing::info!(
                        task_id = task_id_str,
                        context_type = ?ctx_type,
                        "Stopped running agent before merge cleanup"
                    );
                }
                Ok(false) => {}
                Err(e) => {
                    tracing::warn!(
                        task_id = task_id_str,
                        context_type = ?ctx_type,
                        error = %e,
                        "Failed to stop agent (non-fatal)"
                    );
                }
            }
        }
        // Kill any lingering processes with files open in the task worktree
        if let Some(ref worktree_path) = task.worktree_path {
            let worktree_path_buf = PathBuf::from(worktree_path);
            if worktree_path_buf.exists() {
                crate::domain::services::kill_worktree_processes(&worktree_path_buf);
            }
        }
        // Brief settle time for process tree cleanup after SIGTERM
        let agent_kill_settle_secs = git_runtime_config().agent_kill_settle_secs;
        tokio::time::sleep(std::time::Duration::from_secs(agent_kill_settle_secs)).await;

        // --- Step 1: Remove stale index.lock ---
        tracing::info!(task_id = task_id_str, "pre_merge_cleanup: step 1 — removing stale index.lock");
        let index_lock_stale_secs = git_runtime_config().index_lock_stale_secs;
        match GitService::remove_stale_index_lock(repo_path, index_lock_stale_secs) {
            Ok(true) => {
                tracing::info!(
                    task_id = task_id_str,
                    "Removed stale index.lock before merge attempt"
                );
            }
            Ok(false) => {}
            Err(e) => {
                tracing::warn!(
                    task_id = task_id_str,
                    error = %e,
                    "Failed to remove stale index.lock (non-fatal)"
                );
            }
        }

        {
            // --- Step 2: Delete task worktree ---
            tracing::info!(task_id = task_id_str, "pre_merge_cleanup: step 2 — deleting task worktree");
            if let Some(ref worktree_path) = task.worktree_path {
                let worktree_path_buf = PathBuf::from(worktree_path);
                if worktree_path_buf.exists() {
                    tracing::info!(
                        task_id = task_id_str,
                        worktree_path = %worktree_path,
                        "Deleting task worktree before programmatic merge to unlock branch"
                    );
                    run_cleanup_step(
                        "step 2 task worktree deletion",
                        git_runtime_config().cleanup_worktree_timeout_secs,
                        task_id_str,
                        GitService::delete_worktree(repo_path, &worktree_path_buf),
                    )
                    .await;
                }
            }

            // --- Step 3: Prune stale worktree refs ---
            tracing::info!(task_id = task_id_str, "pre_merge_cleanup: step 3 — pruning stale worktree refs");
            if let Err(e) = GitService::prune_worktrees(repo_path).await {
                tracing::warn!(
                    task_id = task_id_str,
                    error = %e,
                    "Failed to prune stale worktrees (non-fatal)"
                );
            }

            // --- Step 4: Delete own stale merge/rebase worktrees ---
            tracing::info!(task_id = task_id_str, "pre_merge_cleanup: step 4 — deleting own stale merge/rebase worktrees");
            for (wt_label, own_wt) in [
                ("merge", compute_merge_worktree_path(project, task_id_str)),
                ("rebase", compute_rebase_worktree_path(project, task_id_str)),
            ] {
                let own_wt_path = PathBuf::from(&own_wt);
                if own_wt_path.exists() {
                    tracing::info!(
                        task_id = task_id_str,
                        worktree_path = %own_wt,
                        "Cleaning up stale {} worktree from previous attempt",
                        wt_label
                    );
                    run_cleanup_step(
                        &format!("step 4 {} worktree deletion", wt_label),
                        git_runtime_config().cleanup_git_op_timeout_secs,
                        task_id_str,
                        GitService::delete_worktree(repo_path, &own_wt_path),
                    )
                    .await;
                }
            }

            // --- Step 5: Scan for orphaned merge worktrees ---
            tracing::info!(task_id = task_id_str, "pre_merge_cleanup: step 5 — scanning for orphaned merge worktrees");
            let worktrees_result = tokio::time::timeout(
                std::time::Duration::from_secs(git_runtime_config().cleanup_git_op_timeout_secs),
                GitService::list_worktrees(repo_path),
            )
            .await;
            match worktrees_result {
                Ok(Ok(worktrees)) => {
                    for wt in &worktrees {
                        let Some(other_task_id) = extract_task_id_from_merge_path(&wt.path) else {
                            continue;
                        };
                        if other_task_id == task_id_str {
                            continue;
                        }
                        let wt_branch = wt.branch.as_deref().unwrap_or("");
                        if wt_branch != target_branch {
                            continue;
                        }
                        if is_task_in_merge_workflow(task_repo, other_task_id).await {
                            tracing::info!(
                                task_id = task_id_str,
                                other_task_id = other_task_id,
                                worktree_path = %wt.path,
                                "Skipping merge worktree cleanup — owning task is still in merge workflow"
                            );
                            continue;
                        }
                        tracing::info!(
                            task_id = task_id_str,
                            other_task_id = other_task_id,
                            worktree_path = %wt.path,
                            target_branch = %target_branch,
                            "Cleaning up orphaned merge worktree from non-active task"
                        );
                        let orphan_path = PathBuf::from(&wt.path);
                        if let Err(e) = GitService::delete_worktree(repo_path, &orphan_path).await {
                            tracing::warn!(
                                task_id = task_id_str,
                                other_task_id = other_task_id,
                                error = %e,
                                worktree_path = %wt.path,
                                "Failed to delete orphaned merge worktree (non-fatal)"
                            );
                        }
                    }
                }
                Ok(Err(e)) => {
                    tracing::warn!(
                        task_id = task_id_str,
                        error = %e,
                        "Failed to list worktrees for orphan scan (non-fatal)"
                    );
                }
                Err(_elapsed) => {
                    tracing::warn!(
                        task_id = task_id_str,
                        timeout_secs = git_runtime_config().cleanup_git_op_timeout_secs,
                        "pre_merge_cleanup: step 5 worktree list timed out (non-fatal)"
                    );
                }
            }
        }

        // --- Step 6: Clean working tree ---
        tracing::info!(task_id = task_id_str, "pre_merge_cleanup: step 6 — cleaning working tree (git clean)");
        run_cleanup_step(
            "step 6 git clean",
            git_runtime_config().cleanup_git_op_timeout_secs,
            task_id_str,
            GitService::clean_working_tree(repo_path),
        )
        .await;
        tracing::info!(task_id = task_id_str, "pre_merge_cleanup: complete");
    }
}
