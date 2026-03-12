//! PR startup recovery: restart pollers for Merging+PR tasks after app restart.
//!
//! On shutdown, pollers are killed without cleanup. On next startup,
//! this module scans for tasks that were actively polling (`pr_polling_active = true`)
//! and restarts their pollers with staggered jitter to avoid thundering herd.
//!
//! Called from `lib.rs` after dual-AppState block, inside the startup async task,
//! BEFORE `StartupJobRunner::run()` to ensure pollers exist before the reconciler
//! can re-enter on_enter(Merging) for PR-mode tasks.

use std::sync::Arc;

use crate::application::services::PrPollerRegistry;
use crate::application::TaskTransitionService;
use crate::domain::repositories::{PlanBranchRepository, TaskRepository};
use crate::domain::entities::InternalStatus;

/// Restart PR merge pollers for tasks that were polling when the app last shut down.
///
/// Scans `plan_branches` for rows with `pr_polling_active = 1`, verifies the
/// associated task is still in `Merging` status, then calls
/// `registry.start_polling()` for each — which applies its own staggered jitter
/// to prevent thundering herd. (AD9)
///
/// # Errors
/// Logs warnings on repo failures; never panics or returns an error to the caller.
pub async fn recover_pr_pollers(
    task_repo: Arc<dyn TaskRepository>,
    plan_branch_repo: Arc<dyn PlanBranchRepository>,
    pr_poller_registry: Arc<PrPollerRegistry>,
    project_repo: Arc<dyn crate::domain::repositories::ProjectRepository>,
    transition_service: Arc<TaskTransitionService<tauri::Wry>>,
) {
    let task_ids = match plan_branch_repo.find_pr_polling_task_ids().await {
        Ok(ids) => ids,
        Err(e) => {
            tracing::warn!(error = %e, "PR startup recovery: failed to query pr_polling task IDs");
            return;
        }
    };

    if task_ids.is_empty() {
        tracing::debug!("PR startup recovery: no tasks with pr_polling_active=true");
        return;
    }

    tracing::info!(count = task_ids.len(), "PR startup recovery: found tasks with active polling");

    for task_id in task_ids {
        // Verify task still in Merging status
        let task = match task_repo.get_by_id(&task_id).await {
            Ok(Some(t)) => t,
            Ok(None) => {
                tracing::debug!(
                    task_id = task_id.as_str(),
                    "PR startup recovery: task not found, skipping"
                );
                continue;
            }
            Err(e) => {
                tracing::warn!(
                    task_id = task_id.as_str(),
                    error = %e,
                    "PR startup recovery: failed to load task"
                );
                continue;
            }
        };

        if task.internal_status != InternalStatus::Merging {
            tracing::debug!(
                task_id = task_id.as_str(),
                status = ?task.internal_status,
                "PR startup recovery: task not in Merging, skipping"
            );
            continue;
        }

        // Load plan branch
        let plan_branch = match plan_branch_repo.get_by_merge_task_id(&task_id).await {
            Ok(Some(pb)) => pb,
            Ok(None) => {
                tracing::warn!(
                    task_id = task_id.as_str(),
                    "PR startup recovery: no plan branch found for task"
                );
                continue;
            }
            Err(e) => {
                tracing::warn!(
                    task_id = task_id.as_str(),
                    error = %e,
                    "PR startup recovery: failed to load plan branch"
                );
                continue;
            }
        };

        let pr_number = match plan_branch.pr_number {
            Some(n) => n,
            None => {
                tracing::debug!(
                    task_id = task_id.as_str(),
                    "PR startup recovery: no pr_number on plan branch, skipping"
                );
                continue;
            }
        };

        if !plan_branch.pr_eligible {
            tracing::debug!(
                task_id = task_id.as_str(),
                "PR startup recovery: pr_eligible=false, skipping"
            );
            continue;
        }

        // Load project for working_dir and base_branch
        let project = match project_repo.get_by_id(&plan_branch.project_id).await {
            Ok(Some(p)) => p,
            Ok(None) => {
                tracing::warn!(
                    task_id = task_id.as_str(),
                    "PR startup recovery: project not found"
                );
                continue;
            }
            Err(e) => {
                tracing::warn!(
                    task_id = task_id.as_str(),
                    error = %e,
                    "PR startup recovery: failed to load project"
                );
                continue;
            }
        };

        let working_dir = std::path::PathBuf::from(&project.working_directory);
        // source_branch = the base branch the plan was branched from (e.g. "main")
        let base_branch = plan_branch.source_branch.clone();

        tracing::info!(
            task_id = task_id.as_str(),
            pr_number = pr_number,
            "PR startup recovery: restarting poller (staggered jitter applied by registry)"
        );

        pr_poller_registry.start_polling(
            task_id,
            plan_branch.id,
            pr_number,
            working_dir,
            base_branch,
            Arc::clone(&transition_service),
        );
    }
}
