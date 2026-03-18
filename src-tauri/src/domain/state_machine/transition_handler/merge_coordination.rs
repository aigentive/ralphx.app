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
    InternalStatus, PlanBranchStatus, Task, TaskId,
};
use crate::domain::repositories::{PlanBranchRepository, TaskRepository};
use crate::infrastructure::agents::claude::{defer_merge_enabled, git_runtime_config};

use super::cleanup_helpers::{run_cleanup_step, CleanupStepResult};
use super::merge_helpers::{
    compute_merge_worktree_path, compute_plan_update_worktree_path, compute_rebase_worktree_path,
    compute_source_update_worktree_path, compute_task_worktree_path,
};
use super::merge_validation::emit_merge_progress;

/// Metadata keys that indicate a prior merge attempt has been made.
///
/// If any of these keys are present in `task.metadata`, the task has been through
/// a merge cycle before and cleanup must run (debris may exist).
const MERGE_DEBRIS_METADATA_KEYS: &[&str] = &[
    "merge_failure_source",
    "source_conflict_resolved",
    "plan_update_conflict",
    "merge_error",
    "conflict_type",
    "source_update_conflict",
    "conflict_files",
    "error",
    "merge_pipeline_active", // Legacy: pre-v53 stored as JSON metadata (evidence: side_effects.rs:142)
];

/// Check whether this is a first clean merge attempt with no prior debris.
///
/// Returns `true` when the task has never been through a merge failure cycle —
/// meaning there's no debris from prior attempts that needs cleaning up.
/// When `true`, `pre_merge_cleanup` can skip all cleanup steps (Phase 1 GUARD fast-path).
///
/// Uses a 3-tier check:
/// 1. Dedicated `merge_pipeline_active` DB column — set when pipeline starts, cleared on success.
///    Non-null means a prior run crashed mid-pipeline.
/// 2. JSON metadata debris keys (including legacy `merge_pipeline_active` JSON key from pre-v53).
/// 3. Disk-presence check — if `worktree_path` is set AND the directory still exists on disk,
///    treat as potential debris (process may have crashed before writing metadata).
pub(crate) fn is_first_clean_attempt(task: &Task) -> bool {
    // Tier 1: Dedicated DB column — crash-mid-pipeline detection.
    // Set by set_merge_pipeline_active() in side_effects.rs, cleared after successful run.
    let pipeline_active = task.merge_pipeline_active.is_some();
    if pipeline_active {
        return false;
    }

    // Tier 2: JSON metadata debris keys (includes legacy merge_pipeline_active JSON key).
    let has_debris_metadata = match task.metadata.as_ref() {
        None => false,
        Some(metadata_str) => {
            match serde_json::from_str::<serde_json::Value>(metadata_str) {
                // Malformed metadata — conservative: treat as debris
                Err(_) => true,
                Ok(metadata) => match metadata.as_object() {
                    None => false,
                    Some(obj) => MERGE_DEBRIS_METADATA_KEYS
                        .iter()
                        .any(|key| obj.contains_key(*key)),
                },
            }
        }
    };
    if has_debris_metadata {
        return false;
    }

    // Tier 3: Disk-presence check — crash-before-metadata scenario.
    // If worktree_path is set AND the directory exists on disk, treat as potential debris.
    // Path::exists() is a blocking stat() — acceptable for local ~/ralphx-worktrees/ paths (<1ms).
    let disk_exists = task
        .worktree_path
        .as_ref()
        .map_or(false, |p| std::path::Path::new(p).exists());
    if disk_exists {
        return false;
    }

    true
}

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
        || GitService::branch_exists(repo_path, target_branch).await.unwrap_or(false)
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
        Err(_) if GitService::branch_exists(repo_path, &pb.branch_name).await.unwrap_or(false) => {
            // Intentional: race condition — concurrent task already created the branch
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

/// Result of attempting to update a plan branch from main.
#[derive(Debug)]
pub(super) enum PlanUpdateResult {
    /// Plan branch was already up-to-date with main (no action needed).
    AlreadyUpToDate,
    /// Plan branch was updated (fast-forward or merge commit created).
    Updated,
    /// Plan branch is behind main but the target is main itself — skip update.
    NotPlanBranch,
    /// Merge main into plan branch produced conflicts — needs agent resolution.
    Conflicts { conflict_files: Vec<PathBuf> },
    /// Git error during the update attempt.
    Error(String),
}

/// Update a plan branch from main before a task→plan merge.
///
/// When a task merges into a plan branch (not main), the plan branch may be behind
/// main if fixes were committed to main after the plan branch was created. This causes
/// false validation failures because the plan branch code doesn't have those fixes.
///
/// This function checks if the plan branch is behind main and, if so, merges main
/// into the plan branch using an isolated worktree. On conflict, returns
/// `PlanUpdateResult::Conflicts` so the caller can route to the merger agent.
///
/// Only runs when `target_branch != base_branch` (i.e., target is a plan branch).
pub(super) async fn update_plan_from_main(
    repo_path: &Path,
    target_branch: &str,
    base_branch: &str,
    project: &crate::domain::entities::Project,
    task_id_str: &str,
    app_handle: Option<&tauri::AppHandle>,
) -> PlanUpdateResult {
    // Only update when merging to a plan branch (not main)
    if target_branch == base_branch {
        return PlanUpdateResult::NotPlanBranch;
    }

    // Check if main's HEAD is already an ancestor of the plan branch
    // (i.e., the plan branch already has all of main's changes)
    let main_sha = match GitService::get_branch_sha(repo_path, base_branch).await {
        Ok(sha) => sha,
        Err(e) => {
            tracing::warn!(
                task_id = task_id_str,
                error = %e,
                base_branch = %base_branch,
                "Failed to get SHA for base branch — skipping plan branch update"
            );
            return PlanUpdateResult::Error(format!(
                "Failed to get SHA for {}: {}",
                base_branch, e
            ));
        }
    };

    match GitService::is_commit_on_branch(repo_path, &main_sha, target_branch).await {
        Ok(true) => {
            tracing::debug!(
                task_id = task_id_str,
                target_branch = %target_branch,
                base_branch = %base_branch,
                "Plan branch is up-to-date with main — no update needed"
            );
            return PlanUpdateResult::AlreadyUpToDate;
        }
        Ok(false) => {
            tracing::info!(
                task_id = task_id_str,
                target_branch = %target_branch,
                base_branch = %base_branch,
                main_sha = %main_sha,
                "Plan branch is behind main — updating before task merge"
            );
        }
        Err(e) => {
            tracing::warn!(
                task_id = task_id_str,
                error = %e,
                "Failed to check if plan branch is up-to-date — skipping update"
            );
            return PlanUpdateResult::Error(format!("is_commit_on_branch check failed: {}", e));
        }
    }

    emit_merge_progress(
        app_handle,
        task_id_str,
        MergePhase::programmatic_merge(),
        MergePhaseStatus::Started,
        format!(
            "Updating {} from {} before merge",
            target_branch, base_branch
        ),
    );

    // Use checkout-free merge if target is already checked out
    let current_branch = GitService::get_current_branch(repo_path)
        .await
        .unwrap_or_default();
    if current_branch == target_branch {
        // Target is checked out in main repo — merge main directly
        match GitService::merge_branch(repo_path, base_branch, target_branch).await {
            Ok(result) => {
                let sha = match &result {
                    crate::application::MergeResult::Success { commit_sha }
                    | crate::application::MergeResult::FastForward { commit_sha } => {
                        commit_sha.clone()
                    }
                    crate::application::MergeResult::Conflict { files } => {
                        // Abort the in-progress merge so the working tree is clean
                        let _ = GitService::abort_merge(repo_path).await;
                        tracing::warn!(
                            task_id = task_id_str,
                            conflict_count = files.len(),
                            "Conflicts detected updating plan branch from main (checkout-free)"
                        );
                        return PlanUpdateResult::Conflicts {
                            conflict_files: files.iter().map(PathBuf::from).collect(),
                        };
                    }
                };
                tracing::info!(
                    task_id = task_id_str,
                    commit_sha = %sha,
                    "Plan branch updated from main (checkout-free)"
                );
                return PlanUpdateResult::Updated;
            }
            Err(e) => {
                return PlanUpdateResult::Error(format!("checkout-free merge failed: {}", e));
            }
        }
    }

    // Fallback: if the plan branch is already checked out in an existing worktree
    // (e.g., merge worktree from a prior attempt), merge main directly there instead
    // of trying to create a new worktree (which would fail with "already used by worktree").
    if let Ok(worktrees) = GitService::list_worktrees(repo_path).await {
        if let Some(wt) = worktrees
            .iter()
            .find(|w| w.branch.as_deref() == Some(target_branch))
        {
            let wt_path = PathBuf::from(&wt.path);
            if !wt_path.exists() {
                tracing::warn!(
                    task_id = task_id_str,
                    path = %wt_path.display(),
                    "Stale worktree entry — path deleted, pruning before fresh creation"
                );
                // Prune the stale entry so fresh worktree creation won't hit "already checked out"
                let _ = GitService::delete_worktree(repo_path, &wt_path).await;
                super::cleanup_helpers::git_worktree_prune(repo_path).await;
                // Fall through to fresh worktree creation below
            } else {
            tracing::info!(
                task_id = task_id_str,
                target_branch = %target_branch,
                worktree_path = %wt_path.display(),
                "Plan branch already checked out in existing worktree — merging main there"
            );
            match GitService::merge_branch(&wt_path, base_branch, target_branch).await {
                Ok(result) => {
                    let sha = match &result {
                        crate::application::MergeResult::Success { commit_sha }
                        | crate::application::MergeResult::FastForward { commit_sha } => {
                            commit_sha.clone()
                        }
                        crate::application::MergeResult::Conflict { files } => {
                            let _ = GitService::abort_merge(&wt_path).await;
                            tracing::warn!(
                                task_id = task_id_str,
                                conflict_count = files.len(),
                                "Conflicts detected updating plan branch from main (existing worktree)"
                            );
                            return PlanUpdateResult::Conflicts {
                                conflict_files: files.iter().map(PathBuf::from).collect(),
                            };
                        }
                    };
                    tracing::info!(
                        task_id = task_id_str,
                        commit_sha = %sha,
                        "Plan branch updated from main (existing worktree)"
                    );
                    return PlanUpdateResult::Updated;
                }
                Err(e) => {
                    // Check if the error is because the worktree already has a merge in
                    // progress from a prior attempt. When git sees MERGE_HEAD it refuses
                    // to start a new merge and returns "You have not concluded your merge"
                    // — no "CONFLICT" in stderr, so merge_branch returns Err instead of
                    // Ok(MergeResult::Conflict). Route to Conflicts so the agent can
                    // resolve the existing conflict markers without discarding them.
                    if GitService::is_merge_in_progress(&wt_path) {
                        let conflict_files = GitService::get_conflict_files(&wt_path)
                            .await
                            .unwrap_or_default();
                        tracing::warn!(
                            task_id = task_id_str,
                            conflict_count = conflict_files.len(),
                            worktree_path = %wt_path.display(),
                            "Plan branch already in merge conflict state in existing worktree \
                             (prior attempt) — routing to merger agent without aborting"
                        );
                        return PlanUpdateResult::Conflicts { conflict_files };
                    }
                    // Not a conflict — abort any partial state and return as error.
                    let _ = GitService::abort_merge(&wt_path).await;
                    return PlanUpdateResult::Error(format!(
                        "merge in existing worktree failed: {}",
                        e
                    ));
                }
            }
            } // end else (wt_path.exists())
        }
    }

    // Target not checked out anywhere — use isolated worktree
    // try_merge_in_worktree merges base_branch (main) into target_branch (plan)
    let wt_path_str = super::merge_helpers::compute_plan_update_worktree_path(project, task_id_str);
    let wt_path = PathBuf::from(&wt_path_str);

    // Clean up any stale worktree from a prior attempt
    super::merge_helpers::pre_delete_worktree(repo_path, &wt_path, task_id_str).await;

    let result = match GitService::try_merge_in_worktree(
        repo_path,
        base_branch,   // source = main
        target_branch, // target = plan branch
        &wt_path,
    )
    .await
    {
        Ok(crate::application::MergeAttemptResult::Success { commit_sha }) => {
            tracing::info!(
                task_id = task_id_str,
                target_branch = %target_branch,
                base_branch = %base_branch,
                commit_sha = %commit_sha,
                "Plan branch updated from main via worktree"
            );
            PlanUpdateResult::Updated
        }
        Ok(crate::application::MergeAttemptResult::NeedsAgent { conflict_files }) => {
            tracing::warn!(
                task_id = task_id_str,
                conflict_count = conflict_files.len(),
                "Conflicts detected updating plan branch from main via worktree"
            );
            PlanUpdateResult::Conflicts { conflict_files }
        }
        Ok(crate::application::MergeAttemptResult::BranchNotFound { branch }) => {
            PlanUpdateResult::Error(format!("Branch not found during plan update: {}", branch))
        }
        Err(e) => PlanUpdateResult::Error(format!("Plan update merge failed: {}", e)),
    };

    // Clean up worktree (always, regardless of outcome)
    super::merge_helpers::pre_delete_worktree(repo_path, &wt_path, task_id_str).await;

    result
}

/// Result of attempting to update a source (task/feature) branch from its target branch.
#[derive(Debug)]
pub(super) enum SourceUpdateResult {
    /// Source branch already contains all of target's changes.
    AlreadyUpToDate,
    /// Source branch was updated (merge commit created).
    Updated,
    /// Merge target into source produced conflicts — needs agent resolution.
    Conflicts { conflict_files: Vec<PathBuf> },
    /// Git error during the update attempt.
    Error(String),
}

/// Update the source (task/feature) branch from the target branch before merging.
///
/// When a task branch merges into a target (plan or main), the task branch may be behind
/// the target if other tasks were merged into it since this task branched off. This causes
/// false validation failures because the merged code doesn't include those changes.
///
/// This function checks if the target branch's HEAD is an ancestor of the source branch.
/// If not, it merges the target into the source using an isolated worktree to bring it
/// up-to-date. On conflict, returns `SourceUpdateResult::Conflicts` so the caller can
/// route to the merger agent.
pub(super) async fn update_source_from_target(
    repo_path: &Path,
    source_branch: &str,
    target_branch: &str,
    project: &crate::domain::entities::Project,
    task_id_str: &str,
    app_handle: Option<&tauri::AppHandle>,
) -> SourceUpdateResult {
    // Check if target's HEAD is already an ancestor of source
    // (i.e., source already has all of target's changes)
    let target_sha = match GitService::get_branch_sha(repo_path, target_branch).await {
        Ok(sha) => sha,
        Err(e) => {
            tracing::warn!(
                task_id = task_id_str,
                error = %e,
                target_branch = %target_branch,
                "Failed to get SHA for target branch — skipping source update"
            );
            return SourceUpdateResult::Error(format!(
                "Failed to get SHA for {}: {}",
                target_branch, e
            ));
        }
    };

    match GitService::is_commit_on_branch(repo_path, &target_sha, source_branch).await {
        Ok(true) => {
            tracing::debug!(
                task_id = task_id_str,
                source_branch = %source_branch,
                target_branch = %target_branch,
                "Source branch is up-to-date with target — no update needed"
            );
            return SourceUpdateResult::AlreadyUpToDate;
        }
        Ok(false) => {
            tracing::info!(
                task_id = task_id_str,
                source_branch = %source_branch,
                target_branch = %target_branch,
                target_sha = %target_sha,
                "Source branch is behind target — updating before merge"
            );
        }
        Err(e) => {
            tracing::warn!(
                task_id = task_id_str,
                error = %e,
                "Failed to check if source branch is up-to-date — skipping update"
            );
            return SourceUpdateResult::Error(format!("is_commit_on_branch check failed: {}", e));
        }
    }

    emit_merge_progress(
        app_handle,
        task_id_str,
        MergePhase::programmatic_merge(),
        MergePhaseStatus::Started,
        format!(
            "Updating {} from {} before merge",
            source_branch, target_branch
        ),
    );

    // Fallback: if the source branch is already checked out in an existing worktree
    // (e.g., from a prior task execution), merge target directly there instead of
    // trying to create a new worktree (which would fail with "already used by worktree").
    if let Ok(worktrees) = GitService::list_worktrees(repo_path).await {
        if let Some(wt) = worktrees.iter().find(|w| w.branch.as_deref() == Some(source_branch)) {
            let wt_path = PathBuf::from(&wt.path);
            if !wt_path.exists() {
                tracing::warn!(
                    task_id = task_id_str,
                    path = %wt_path.display(),
                    "Stale worktree entry — path deleted, pruning before fresh creation"
                );
                // Prune the stale entry so fresh worktree creation won't hit "already checked out"
                let _ = GitService::delete_worktree(repo_path, &wt_path).await;
                super::cleanup_helpers::git_worktree_prune(repo_path).await;
                // Fall through to fresh worktree creation below
            } else {
            tracing::info!(
                task_id = task_id_str,
                source_branch = %source_branch,
                worktree_path = %wt_path.display(),
                "Source branch already checked out in existing worktree — merging target there"
            );
            match GitService::merge_branch(&wt_path, target_branch, source_branch).await {
                Ok(result) => {
                    let sha = match &result {
                        crate::application::MergeResult::Success { commit_sha }
                        | crate::application::MergeResult::FastForward { commit_sha } => commit_sha.clone(),
                        crate::application::MergeResult::Conflict { files } => {
                            let _ = GitService::abort_merge(&wt_path).await;
                            tracing::warn!(
                                task_id = task_id_str,
                                conflict_count = files.len(),
                                "Conflicts detected updating source branch from target (existing worktree)"
                            );
                            return SourceUpdateResult::Conflicts {
                                conflict_files: files.iter().map(PathBuf::from).collect(),
                            };
                        }
                    };
                    tracing::info!(
                        task_id = task_id_str,
                        commit_sha = %sha,
                        "Source branch updated from target (existing worktree)"
                    );
                    return SourceUpdateResult::Updated;
                }
                Err(e) => {
                    // Check if the error is because the worktree already has a merge in
                    // progress from a prior attempt.
                    if GitService::is_merge_in_progress(&wt_path) {
                        let conflict_files = GitService::get_conflict_files(&wt_path)
                            .await
                            .unwrap_or_default();
                        tracing::warn!(
                            task_id = task_id_str,
                            conflict_count = conflict_files.len(),
                            worktree_path = %wt_path.display(),
                            "Source branch already in merge conflict state in existing worktree \
                             (prior attempt) — routing to merger agent without aborting"
                        );
                        return SourceUpdateResult::Conflicts { conflict_files };
                    }
                    // Not a conflict — abort any partial state and return as error.
                    let _ = GitService::abort_merge(&wt_path).await;
                    return SourceUpdateResult::Error(format!(
                        "merge in existing worktree failed: {}", e
                    ));
                }
            }
            } // end else (wt_path.exists())
        }
    }

    // Use isolated worktree to merge target into source
    let wt_path_str = compute_source_update_worktree_path(project, task_id_str);
    let wt_path = PathBuf::from(&wt_path_str);

    // Clean up any stale worktree from a prior attempt
    super::merge_helpers::pre_delete_worktree(repo_path, &wt_path, task_id_str).await;

    // try_merge_in_worktree merges target_branch into source_branch
    let result = match GitService::try_merge_in_worktree(
        repo_path,
        target_branch, // source of changes = target branch
        source_branch, // branch to update = source/task branch
        &wt_path,
    )
    .await
    {
        Ok(crate::application::MergeAttemptResult::Success { commit_sha }) => {
            tracing::info!(
                task_id = task_id_str,
                source_branch = %source_branch,
                target_branch = %target_branch,
                commit_sha = %commit_sha,
                "Source branch updated from target via worktree"
            );
            SourceUpdateResult::Updated
        }
        Ok(crate::application::MergeAttemptResult::NeedsAgent { conflict_files }) => {
            tracing::warn!(
                task_id = task_id_str,
                conflict_count = conflict_files.len(),
                "Conflicts detected updating source branch from target via worktree"
            );
            SourceUpdateResult::Conflicts { conflict_files }
        }
        Ok(crate::application::MergeAttemptResult::BranchNotFound { branch }) => {
            SourceUpdateResult::Error(format!("Branch not found during source update: {}", branch))
        }
        Err(e) => SourceUpdateResult::Error(format!("Source update merge failed: {}", e)),
    };

    // Clean up worktree (always, regardless of outcome)
    super::merge_helpers::pre_delete_worktree(repo_path, &wt_path, task_id_str).await;

    result
}

/// Check if a main-branch merge should be deferred.
///
/// Returns `true` if the merge was deferred (caller should return early).
/// Defers when target is the base branch AND either:
/// 1. Sibling plan tasks are not all terminal
/// 2. Agents are still running (running_agent_count > 0)
pub(super) async fn check_main_merge_deferral(
    tc: super::TaskCore<'_>,
    bp: super::BranchPair<'_>,
    base_branch: &str,
    running_agent_count: Option<u32>,
    app_handle: Option<&tauri::AppHandle>,
) -> bool {
    let task = tc.task;
    let task_id_str = tc.task_id_str;
    let task_repo = tc.task_repo;
    let (source_branch, target_branch) = (bp.source_branch, bp.target_branch);
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
            t.id == task.id || t.internal_status == InternalStatus::PendingMerge || t.is_terminal()
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
                MergePhase::programmatic_merge(),
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
                MergePhase::programmatic_merge(),
                MergePhaseStatus::Started,
                format!(
                    "Deferred merge to {} — waiting for {} agent(s) to complete",
                    target_branch, count
                ),
            );

            return true;
        }
    }

    tracing::debug!(
        task_id = task_id_str,
        running_count = running_agent_count.unwrap_or(0),
        proceeding = true,
        "check_main_merge_deferral: all guards passed — proceeding with merge"
    );
    false
}

impl<'a> super::TransitionHandler<'a> {
    /// Phase 1 GUARD: fast pre-merge cleanup with first-attempt skip optimization.
    ///
    /// On first clean attempt (no prior failure metadata, no running agents),
    /// skips cleanup entirely — returns in microseconds.
    ///
    /// On retry attempts or when agents are running, executes targeted cleanup:
    ///   0a. Cancel in-flight validation tokens (instant)
    ///   0b. Stop running agents — uses SIGKILL immediate (no SIGTERM grace period)
    ///   1.  Remove stale `.git/index.lock`
    ///   2.  Delete the task worktree to unlock the task branch
    ///   3.  Prune stale worktree references
    ///   4.  Delete own merge/rebase/plan-update/source-update worktrees (PARALLEL)
    ///
    /// Step 5 (orphaned worktree scan) has been moved to Phase 3 deferred cleanup —
    /// it's not critical for merge success and is the slowest step.
    pub(super) async fn pre_merge_cleanup(
        &self,
        task_id_str: &str,
        task: &crate::domain::entities::Task,
        project: &crate::domain::entities::Project,
        repo_path: &Path,
        target_branch: &str,
        task_repo: &Arc<dyn TaskRepository>,
    ) {
        let cleanup_start = std::time::Instant::now();
        let app_handle = self.machine.context.services.app_handle.as_ref();

        // --- Phase 1 GUARD: first-attempt skip optimization (ROOT CAUSE #3) ---
        // If this is the first merge attempt AND no agents are running for this task,
        // skip all cleanup steps — there's no debris to clean.
        let is_first = is_first_clean_attempt(task);
        if is_first {
            // Quick agent check: are review/merge agents currently running?
            let review_running = self
                .machine
                .context
                .services
                .chat_service
                .is_agent_running(
                    crate::domain::entities::ChatContextType::Review,
                    task_id_str,
                )
                .await;
            let merge_running = self
                .machine
                .context
                .services
                .chat_service
                .is_agent_running(
                    crate::domain::entities::ChatContextType::Merge,
                    task_id_str,
                )
                .await;

            if !review_running && !merge_running {
                tracing::info!(
                    task_id = task_id_str,
                    elapsed_us = cleanup_start.elapsed().as_micros() as u64,
                    "pre_merge_cleanup: GUARD fast-path — first clean attempt, no agents running, skipping all cleanup"
                );
                return;
            }
            tracing::info!(
                task_id = task_id_str,
                review_running,
                merge_running,
                "pre_merge_cleanup: first attempt but agents running — proceeding with cleanup"
            );
        } else {
            let pipeline_active = task.merge_pipeline_active.is_some();
            let has_debris_metadata = task.metadata.as_ref().map_or(false, |s| {
                serde_json::from_str::<serde_json::Value>(s)
                    .ok()
                    .and_then(|v| v.as_object().cloned())
                    .map_or(true, |obj| {
                        MERGE_DEBRIS_METADATA_KEYS
                            .iter()
                            .any(|key| obj.contains_key(*key))
                    })
            });
            let disk_exists = task
                .worktree_path
                .as_ref()
                .map_or(false, |p| std::path::Path::new(p).exists());
            tracing::info!(
                task_id = task_id_str,
                pipeline_active,
                has_debris_metadata,
                disk_exists,
                "pre_merge_cleanup: retry attempt (debris detected — pipeline active flag, metadata, or stale worktree on disk) — running full cleanup"
            );
        }

        // --- Step 0a: Cancel in-flight validation for this task ---
        if let Some((_, token)) = self
            .machine
            .context
            .services
            .validation_tokens
            .remove(task_id_str)
        {
            token.cancel();
            tracing::info!(
                task_id = task_id_str,
                "pre_merge_cleanup: cancelled in-flight validation"
            );
        }

        // --- Step 0b: Stop running agents (SIGKILL immediate for merge cleanup) ---
        let step_start = std::time::Instant::now();
        emit_merge_progress(
            app_handle,
            task_id_str,
            MergePhase::new(MergePhase::MERGE_CLEANUP),
            MergePhaseStatus::Started,
            "Stopping running agents...".to_string(),
        );
        let agent_stop_timeout_secs = git_runtime_config().agent_stop_timeout_secs;
        let mut any_agent_was_running = false;
        for ctx_type in [
            crate::domain::entities::ChatContextType::Review,
            crate::domain::entities::ChatContextType::Merge,
        ] {
            // Defense-in-depth: if this is the Review agent context and the task has already
            // transitioned past Reviewing (e.g., to PendingMerge), skip stop_agent. The review
            // agent's job is done; stopping it here would kill the TCP connection that owns the
            // complete_review HTTP handler and cancel the entire inline merge pipeline.
            // This guard fires even if early-unregister in the complete_review handler missed
            // a timing edge (e.g., a different transition path).
            if ctx_type == crate::domain::entities::ChatContextType::Review
                && task.internal_status != crate::domain::entities::InternalStatus::Reviewing
            {
                tracing::info!(
                    task_id = task_id_str,
                    context_type = ?ctx_type,
                    task_status = ?task.internal_status,
                    "pre_merge_cleanup: skipping stop_agent for Review context — task already past Reviewing (self-sabotage guard)"
                );
                continue;
            }

            let stop_result = tokio::time::timeout(
                std::time::Duration::from_secs(agent_stop_timeout_secs),
                self.machine
                    .context
                    .services
                    .chat_service
                    .stop_agent(ctx_type, task_id_str),
            )
            .await;
            match stop_result {
                Ok(Ok(true)) => {
                    any_agent_was_running = true;
                    tracing::info!(
                        task_id = task_id_str,
                        context_type = ?ctx_type,
                        "Stopped running agent before merge cleanup"
                    );
                }
                Ok(Ok(false)) => {}
                Ok(Err(e)) => {
                    any_agent_was_running = true;
                    tracing::warn!(
                        task_id = task_id_str,
                        context_type = ?ctx_type,
                        error = %e,
                        "Failed to stop agent (non-fatal)"
                    );
                }
                Err(_elapsed) => {
                    any_agent_was_running = true;
                    tracing::warn!(
                        task_id = task_id_str,
                        context_type = ?ctx_type,
                        timeout_secs = agent_stop_timeout_secs,
                        "stop_agent timed out (non-fatal)"
                    );
                }
            }
        }
        // Scan for OS-level processes still holding worktree files open — only if agents were running
        if any_agent_was_running {
            emit_merge_progress(
                app_handle,
                task_id_str,
                MergePhase::new(MergePhase::MERGE_CLEANUP),
                MergePhaseStatus::Started,
                "Scanning worktree for orphaned processes...".to_string(),
            );
            if let Some(ref worktree_path) = task.worktree_path {
                let worktree_path_buf = PathBuf::from(worktree_path);
                if worktree_path_buf.exists() {
                    let lsof_timeout = git_runtime_config().worktree_lsof_timeout_secs;
                    let step_0b_timeout_secs = git_runtime_config().step_0b_kill_timeout_secs;
                    match super::cleanup_helpers::os_thread_timeout(
                        std::time::Duration::from_secs(step_0b_timeout_secs),
                        crate::domain::services::kill_worktree_processes_async(
                            &worktree_path_buf,
                            lsof_timeout,
                            true, // merge cleanup: SIGKILL immediately
                        ),
                    )
                    .await
                    {
                        Ok(()) => {}
                        Err(_os_elapsed) => {
                            tracing::warn!(
                                task_id = %task_id_str,
                                worktree = %worktree_path,
                                step_0b_timeout_secs,
                                "pre_merge_cleanup step 0b: kill_worktree_processes_async timed out — proceeding"
                            );
                        }
                    }
                }
            }
            // Conditional settle sleep — only when agents were actually killed
            let agent_kill_settle_secs = git_runtime_config().agent_kill_settle_secs;
            if agent_kill_settle_secs > 0 {
                let settle_secs = agent_kill_settle_secs;
                tracing::info!(
                    task_id = task_id_str,
                    settle_secs,
                    "pre_merge_cleanup: agents were killed, waiting {}s for process tree cleanup",
                    settle_secs,
                );
                // Always use os_thread_timeout — immune to tokio timer-driver starvation.
                // One dormant OS thread per merge (settle_secs + 1s grace) is acceptable.
                match super::cleanup_helpers::os_thread_timeout(
                    std::time::Duration::from_secs(settle_secs + 1),
                    tokio::time::sleep(std::time::Duration::from_secs(settle_secs)),
                )
                .await
                {
                    Ok(_) => {}
                    Err(_elapsed) => {
                        tracing::warn!(
                            task_id = %task_id_str,
                            settle_secs,
                            "settle sleep watchdog fired — possible tokio timer starvation"
                        );
                    }
                }
            }
        } else {
            tracing::info!(
                task_id = task_id_str,
                "pre_merge_cleanup: no agents running — skipping process scan and settle sleep"
            );
        }
        tracing::info!(
            task_id = task_id_str,
            elapsed_ms = step_start.elapsed().as_millis() as u64,
            "pre_merge_cleanup: step 0b complete"
        );

        // --- Step 1: Remove stale index.lock ---
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

        // --- Step 2: Delete task worktree ---
        {
            emit_merge_progress(
                app_handle,
                task_id_str,
                MergePhase::new(MergePhase::MERGE_CLEANUP),
                MergePhaseStatus::Started,
                "Removing stale worktrees...".to_string(),
            );
            if let Some(ref worktree_path) = task.worktree_path {
                let worktree_path_buf = PathBuf::from(worktree_path);
                if worktree_path_buf == repo_path {
                    tracing::warn!(
                        task_id = task_id_str,
                        worktree_path = %worktree_path,
                        "Skipping task worktree deletion — path is the main working tree"
                    );
                } else if worktree_path_buf.exists() {
                    // Step 2 TOCTOU guard: re-read status from DB before deleting.
                    // A concurrent handle_outcome_needs_agent may have set Merging
                    // and written this worktree_path as the merge agent's working dir.
                    let task_repo_step2 = Arc::clone(task_repo);
                    let task_id_for_step2 = task.id.clone();
                    let should_skip_step2 =
                        match task_repo_step2.get_by_id(&task_id_for_step2).await {
                            Ok(Some(ref fresh_task))
                                if matches!(
                                    fresh_task.internal_status,
                                    InternalStatus::Merging
                                ) =>
                            {
                                true
                            }
                            // Error or None: proceed with deletion (safe default)
                            _ => false,
                        };

                    if should_skip_step2 {
                        tracing::info!(
                            task_id = task_id_str,
                            worktree_path = %worktree_path,
                            "Skipping task worktree deletion — task is actively merging"
                        );
                    } else {
                        super::merge_helpers::clean_stale_git_state(
                            &worktree_path_buf,
                            task_id_str,
                        )
                        .await;
                        let deletion_start = std::time::Instant::now();
                        match run_cleanup_step(
                            "step 2 task worktree deletion (fast)",
                            git_runtime_config().cleanup_worktree_timeout_secs,
                            task_id_str,
                            super::cleanup_helpers::remove_worktree_fast(
                                &worktree_path_buf,
                                repo_path,
                            ),
                        )
                        .await
                        {
                            CleanupStepResult::Ok => {
                                tracing::info!(
                                    task_id = task_id_str,
                                    elapsed_ms = deletion_start.elapsed().as_millis() as u64,
                                    "Task worktree deletion succeeded"
                                );
                            }
                            CleanupStepResult::TimedOut { elapsed } => {
                                tracing::warn!(
                                    task_id = task_id_str,
                                    elapsed_ms = elapsed.as_millis() as u64,
                                    "Task worktree deletion timed out — branch may still be locked"
                                );
                                // Stale path cleanup: clear worktree_path from DB since deletion
                                // timed out and the path is no longer valid. Race guard inside
                                // prevents clearing when task is actively Merging.
                                clear_stale_worktree_path_on_timeout(
                                    &task_id_for_step2,
                                    task_id_str,
                                    task_repo,
                                )
                                .await;
                            }
                            CleanupStepResult::Error { ref message } => {
                                tracing::warn!(
                                    task_id = task_id_str,
                                    error = %message,
                                    "Task worktree deletion failed — branch may still be locked"
                                );
                            }
                        }
                    }
                }
            }

            // --- Step 3: Prune stale worktree refs ---
            run_cleanup_step(
                "prune_worktrees",
                git_runtime_config().cleanup_git_op_timeout_secs,
                task_id_str,
                GitService::prune_worktrees(repo_path),
            )
            .await;

            // --- Step 4: Delete own stale merge/rebase worktrees (PARALLEL) ---
            let step_start = std::time::Instant::now();
            tracing::info!(
                task_id = task_id_str,
                "pre_merge_cleanup: step 4 starting — parallel deletion of stale worktrees"
            );
            let worktree_specs: Vec<(&str, String)> = vec![
                ("task", compute_task_worktree_path(project, task_id_str)),
                ("merge", compute_merge_worktree_path(project, task_id_str)),
                (
                    "rebase",
                    compute_rebase_worktree_path(project, task_id_str),
                ),
                (
                    "plan-update",
                    compute_plan_update_worktree_path(project, task_id_str),
                ),
                (
                    "source-update",
                    compute_source_update_worktree_path(project, task_id_str),
                ),
            ];

            // Filter to only existing worktrees, then delete in parallel
            let existing_worktrees: Vec<(&str, PathBuf)> = worktree_specs
                .iter()
                .filter_map(|(label, path_str)| {
                    let path = PathBuf::from(path_str);
                    if path.exists() {
                        tracing::info!(
                            task_id = task_id_str,
                            worktree_path = %path_str,
                            wt_type = *label,
                            "Cleaning up stale {} worktree from previous attempt",
                            label
                        );
                        Some((*label, path))
                    } else {
                        None
                    }
                })
                .collect();

            if !existing_worktrees.is_empty() {
                let cleanup_timeout = git_runtime_config().cleanup_git_op_timeout_secs;
                // Pre-allocate step labels so the borrow checker is happy
                let step_labels: Vec<String> = existing_worktrees
                    .iter()
                    .map(|(label, _)| format!("step 4 {} worktree deletion (fast)", label))
                    .collect();
                // Use remove_worktree_fast (unlock + double-force + rm-rf + prune) in parallel.
                // remove_worktree_fast handles locked worktrees via unlock + -f -f before removal.
                // Step 4 TOCTOU guard: for "merge" worktrees, check DB status INSIDE the
                // async future body (not in filter_map) to close the race window where
                // handle_outcome_needs_agent sets Merging after filter_map but before join_all.
                let task_id_for_step4 = task.id.clone();
                let task_id_str_owned = task_id_str.to_string();
                let repo_path_owned = repo_path.to_path_buf();
                let futs: Vec<_> = existing_worktrees
                    .iter()
                    .zip(step_labels.iter())
                    .map(|((label, wt_path), step_label)| {
                        let label_owned = label.to_string();
                        let wt_path_owned = wt_path.clone();
                        let step_label_owned = step_label.clone();
                        let task_id_guard = task_id_for_step4.clone();
                        let task_repo_step4 = Arc::clone(task_repo);
                        let task_id_log = task_id_str_owned.clone();
                        let repo_path_step4 = repo_path_owned.clone();
                        async move {
                            // Guard applies only to "merge" label — these are the worktrees
                            // used by merge agents. Other labels (task/rebase/plan-update/
                            // source-update) are never needed by an active merge agent.
                            if label_owned == "merge" {
                                match task_repo_step4.get_by_id(&task_id_guard).await {
                                    Ok(Some(ref fresh_task))
                                        if matches!(
                                            fresh_task.internal_status,
                                            InternalStatus::Merging
                                        ) =>
                                    {
                                        tracing::info!(
                                            task_id = %task_id_log,
                                            worktree_path = %wt_path_owned.display(),
                                            "Skipping merge worktree deletion — task is actively merging"
                                        );
                                        return CleanupStepResult::Ok;
                                    }
                                    // Error or None: proceed with deletion (safe default)
                                    _ => {}
                                }
                            }
                            run_cleanup_step(
                                &step_label_owned,
                                cleanup_timeout,
                                &task_id_log,
                                super::cleanup_helpers::remove_worktree_fast(
                                    &wt_path_owned,
                                    &repo_path_step4,
                                ),
                            )
                            .await
                        }
                    })
                    .collect();

                let results = futures::future::join_all(futs).await;
                for (i, result) in results.iter().enumerate() {
                    let (label, wt_path) = &existing_worktrees[i];
                    match result {
                        CleanupStepResult::Ok => {}
                        CleanupStepResult::TimedOut { elapsed } => {
                            tracing::warn!(
                                task_id = task_id_str,
                                worktree_path = %wt_path.display(),
                                wt_type = *label,
                                elapsed_ms = elapsed.as_millis() as u64,
                                "Stale {} worktree deletion timed out",
                                label
                            );
                        }
                        CleanupStepResult::Error { ref message } => {
                            tracing::warn!(
                                task_id = task_id_str,
                                worktree_path = %wt_path.display(),
                                wt_type = *label,
                                error = %message,
                                "Stale {} worktree deletion failed",
                                label
                            );
                        }
                    }
                }
            }
            tracing::info!(
                task_id = task_id_str,
                elapsed_ms = step_start.elapsed().as_millis() as u64,
                deleted_count = existing_worktrees.len(),
                "pre_merge_cleanup: step 4 complete (parallel worktree deletion)"
            );

            // Step 5 DEFERRED: orphaned merge worktree scan moved to Phase 3 (fire-and-forget).
            // The scan is not critical for merge success — it's a hygiene operation that
            // lists all worktrees and checks each against the task repo, which is slow.
            // TODO(Phase 3): Move to deferred cleanup after merge completion.
        }

        tracing::info!(
            task_id = task_id_str,
            total_elapsed_ms = cleanup_start.elapsed().as_millis() as u64,
            is_first_attempt = is_first,
            target_branch = target_branch,
            "pre_merge_cleanup: complete"
        );
    }
}

/// Clear `worktree_path` from the DB after a Step 2 deletion timeout.
///
/// Race guard: only clears if the task's current status is NOT [`InternalStatus::Merging`].
/// When the task is actively merging, the worktree is still needed by the merge agent
/// and must not be cleared.
pub(crate) async fn clear_stale_worktree_path_on_timeout(
    task_id: &TaskId,
    task_id_str: &str,
    task_repo: &Arc<dyn TaskRepository>,
) {
    match task_repo.get_by_id(task_id).await {
        Ok(Some(mut fresh_task))
            if !matches!(fresh_task.internal_status, InternalStatus::Merging) =>
        {
            fresh_task.worktree_path = None;
            if let Err(e) = task_repo.update(&fresh_task).await {
                tracing::warn!(
                    task_id = task_id_str,
                    error = %e,
                    "Failed to clear stale worktree_path from DB after timeout (non-fatal)"
                );
            } else {
                tracing::info!(
                    task_id = task_id_str,
                    "Cleared stale worktree_path from DB after deletion timeout"
                );
            }
        }
        Ok(Some(_)) => {
            tracing::info!(
                task_id = task_id_str,
                "Skipping worktree_path clear — task is actively merging"
            );
        }
        // DB error or task not found: skip silently (non-fatal)
        _ => {}
    }
}
