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
use crate::domain::entities::{InternalStatus, PlanBranchStatus};
use crate::domain::repositories::{
    ArtifactRepository, IdeationSessionRepository, PlanBranchRepository, ProjectRepository,
    TaskRepository,
};
use crate::domain::services::GithubServiceTrait;
use crate::domain::state_machine::transition_handler::{
    create_draft_pr_if_needed, plan_branch_has_reviewable_diff, sync_plan_branch_pr_if_needed,
};

/// Re-create draft PRs that should already exist for active PR-mode plans.
///
/// This runs once on startup to repair the gap where an executing plan branch was
/// marked `pr_eligible=true` but never persisted a `pr_number` because early PR
/// creation failed before app shutdown/restart. The helper reuses the same
/// duplicate-safe `create_draft_pr_if_needed` flow used during normal execution.
///
/// # Errors
/// Logs warnings on repo failures; never panics or returns an error to the caller.
pub async fn recover_missing_draft_prs(
    task_repo: Arc<dyn TaskRepository>,
    plan_branch_repo: Arc<dyn PlanBranchRepository>,
    project_repo: Arc<dyn ProjectRepository>,
    ideation_session_repo: Arc<dyn IdeationSessionRepository>,
    artifact_repo: Arc<dyn ArtifactRepository>,
    github_service: Arc<dyn GithubServiceTrait>,
) {
    let pr_creation_guard = Arc::new(dashmap::DashMap::new());

    let projects = match project_repo.get_all().await {
        Ok(projects) => projects,
        Err(e) => {
            tracing::warn!(error = %e, "PR startup recovery: failed to list projects");
            return;
        }
    };

    for project in projects {
        let plan_branches = match plan_branch_repo.get_by_project_id(&project.id).await {
            Ok(branches) => branches,
            Err(e) => {
                tracing::warn!(
                    project_id = project.id.as_str(),
                    error = %e,
                    "PR startup recovery: failed to load plan branches for project"
                );
                continue;
            }
        };

        for plan_branch in plan_branches {
            if !plan_branch.pr_eligible || plan_branch.status != PlanBranchStatus::Active {
                continue;
            }

            let Some(merge_task_id) = plan_branch.merge_task_id.as_ref() else {
                tracing::debug!(
                    branch_id = plan_branch.id.as_str(),
                    branch = %plan_branch.branch_name,
                    "PR startup recovery: active PR-eligible plan branch has no merge task"
                );
                continue;
            };

            let merge_task = match task_repo.get_by_id(merge_task_id).await {
                Ok(Some(task)) => task,
                Ok(None) => {
                    tracing::debug!(
                        branch_id = plan_branch.id.as_str(),
                        branch = %plan_branch.branch_name,
                        merge_task_id = merge_task_id.as_str(),
                        "PR startup recovery: merge task not found for PR-eligible plan branch"
                    );
                    continue;
                }
                Err(e) => {
                    tracing::warn!(
                        branch_id = plan_branch.id.as_str(),
                        branch = %plan_branch.branch_name,
                        merge_task_id = merge_task_id.as_str(),
                        error = %e,
                        "PR startup recovery: failed to load merge task for PR-eligible plan branch"
                    );
                    continue;
                }
            };

            if merge_task.is_terminal() {
                tracing::debug!(
                    branch_id = plan_branch.id.as_str(),
                    branch = %plan_branch.branch_name,
                    merge_task_id = merge_task.id.as_str(),
                    status = ?merge_task.internal_status,
                    "PR startup recovery: skipping terminal merge task with no PR"
                );
                continue;
            }

            let branch_has_reviewable_diff =
                match plan_branch_has_reviewable_diff(&project, &plan_branch).await {
                    Ok(has_diff) => has_diff,
                    Err(e) => {
                        tracing::warn!(
                            branch_id = plan_branch.id.as_str(),
                            branch = %plan_branch.branch_name,
                            merge_task_id = merge_task.id.as_str(),
                            error = %e,
                            "PR startup recovery: failed to determine whether the active plan branch is ahead of base"
                        );
                        false
                    }
                };
            if !branch_has_reviewable_diff {
                tracing::debug!(
                    branch_id = plan_branch.id.as_str(),
                    branch = %plan_branch.branch_name,
                    merge_task_id = merge_task.id.as_str(),
                    status = ?merge_task.internal_status,
                    "PR startup recovery: skipping active plan branch with no reviewable diff"
                );
                continue;
            }

            if plan_branch.pr_number.is_none() {
                tracing::info!(
                    branch_id = plan_branch.id.as_str(),
                    branch = %plan_branch.branch_name,
                    merge_task_id = merge_task.id.as_str(),
                    status = ?merge_task.internal_status,
                    "PR startup recovery: repairing missing draft PR for active plan branch"
                );

                create_draft_pr_if_needed(
                    &merge_task,
                    &project,
                    &plan_branch,
                    &pr_creation_guard,
                    &github_service,
                    &plan_branch_repo,
                    Some(&ideation_session_repo),
                    Some(&artifact_repo),
                )
                .await;
                continue;
            }

            if !matches!(
                plan_branch.pr_push_status,
                crate::domain::entities::plan_branch::PrPushStatus::Pushed
            ) {
                tracing::info!(
                    branch_id = plan_branch.id.as_str(),
                    branch = %plan_branch.branch_name,
                    merge_task_id = merge_task.id.as_str(),
                    status = ?merge_task.internal_status,
                    push_status = %plan_branch.pr_push_status,
                    "PR startup recovery: syncing pending PR branch push for active plan branch"
                );
                sync_plan_branch_pr_if_needed(
                    &project,
                    &plan_branch,
                    &github_service,
                    &plan_branch_repo,
                )
                .await;
            }
        }
    }
}

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
    project_repo: Arc<dyn ProjectRepository>,
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

    tracing::info!(
        count = task_ids.len(),
        "PR startup recovery: found tasks with active polling"
    );

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
