use super::*;
use crate::domain::entities::ReviewScopeMetadata;
use crate::domain::review::{evaluate_merge_scope_backstop, MergeScopeBackstopViolation};
use crate::domain::state_machine::transition_handler::TaskCore;

impl<'a> TransitionHandler<'a> {
    pub(super) async fn evaluate_merge_scope_backstop(
        &self,
        task: &Task,
        project: &Project,
        target_branch: &str,
    ) -> AppResult<Option<MergeScopeBackstopViolation>> {
        let Some(review_scope) =
            ReviewScopeMetadata::from_task_metadata(task.metadata.as_deref())
                .map_err(|e| crate::error::AppError::Validation(e.to_string()))?
        else {
            return Ok(None);
        };
        if review_scope.planned_paths.is_empty() {
            return Ok(None);
        }

        let repo_path = task
            .worktree_path
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(&project.working_directory));

        let Some(source_branch) = task.task_branch.as_deref() else {
            return Ok(None);
        };

        let diff = GitService::get_diff_stats_between(&repo_path, target_branch, source_branch).await?;
        Ok(evaluate_merge_scope_backstop(&review_scope, &diff.changed_files))
    }

    pub(super) async fn route_merge_scope_violation_to_revision(
        &self,
        tc: TaskCore<'_>,
        metadata: serde_json::Value,
    ) -> bool {
        let (task, task_id, task_id_str, task_repo) =
            (tc.task, tc.task_id, tc.task_id_str, tc.task_repo);
        merge_helpers::merge_metadata_into(task, &metadata);
        task.touch();
        if let Err(e) = task_repo.update(task).await {
            tracing::warn!(
                task_id = task_id_str,
                error = %e,
                "Failed to persist merge scope guard metadata before retrying revision"
            );
            return false;
        }

        let Some(transition_service) = &self.machine.context.services.transition_service else {
            tracing::warn!(
                task_id = task_id_str,
                "transition_service unavailable; cannot route merge scope guard back to revision"
            );
            return false;
        };

        match transition_service
            .transition_task(task_id, InternalStatus::RevisionNeeded)
            .await
        {
            Ok(_) => {
                tracing::info!(
                    task_id = task_id_str,
                    "Merge scope backstop routed task back to RevisionNeeded"
                );
                true
            }
            Err(e) => {
                tracing::warn!(
                    task_id = task_id_str,
                    error = %e,
                    "Failed to route merge scope guard back to RevisionNeeded"
                );
                false
            }
        }
    }
}
