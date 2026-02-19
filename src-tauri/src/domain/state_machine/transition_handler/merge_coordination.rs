// Merge coordination helpers — deferral logic and plan branch management.
//
// Extracted from side_effects.rs for maintainability.
// - ensure_plan_branch_exists: lazy git ref creation for plan merge targets
// - check_main_merge_deferral: defer main-branch merges until siblings terminal / agents idle

use std::path::Path;
use std::sync::Arc;

use crate::application::GitService;
use crate::domain::entities::{
    merge_progress_event::{MergePhase, MergePhaseStatus},
    InternalStatus, PlanBranchStatus, Task,
};
use crate::domain::repositories::{PlanBranchRepository, TaskRepository};
use crate::infrastructure::agents::claude::defer_merge_enabled;

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
