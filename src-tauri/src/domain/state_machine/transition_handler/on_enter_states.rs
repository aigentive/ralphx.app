// State entry dispatch — all `on_enter` match arms and helpers.
//
// Extracted from side_effects.rs for maintainability. The `on_enter` method
// signature stays in side_effects.rs and delegates here.

use std::path::Path;
use std::sync::Arc;

use chrono::Utc;

use super::super::machine::State;
use super::freshness::{self, FreshnessAction};
use super::merge_helpers::{
    compute_merge_worktree_path, compute_task_worktree_path,
    is_merge_worktree_path, resolve_task_base_branch, restore_task_worktree, slugify,
};
use super::metadata_builder::{build_failed_metadata, MetadataUpdate};
use crate::application::git_service::git_cmd::ENOENT_MARKER;
use crate::application::GitService;
use crate::domain::entities::task_metadata::GIT_ISOLATION_ERROR_PREFIX;
use crate::domain::entities::plan_branch::PlanBranchId;
use crate::domain::entities::{
    ExecutionFailureSource, ExecutionRecoveryEvent, ExecutionRecoveryEventKind,
    ExecutionRecoveryMetadata, ExecutionRecoveryReasonCode, ExecutionRecoverySource,
    ExecutionRecoveryState, MergeFailureSource, MergeRecoveryEvent, MergeRecoveryEventKind,
    MergeRecoveryMetadata, MergeRecoveryReasonCode, MergeRecoverySource, MergeRecoveryState,
    Project, ProjectId, Task, TaskCategory, TaskId, TaskStepStatus,
};
use crate::domain::repositories::{PlanBranchRepository, TaskRepository};
use crate::domain::services::github_service::GithubServiceTrait;
use crate::error::{AppError, AppResult};
use crate::infrastructure::agents::claude::{reconciliation_config, scheduler_config};

/// Get the plan branch name for a task by looking up via plan_branch_repo.
/// Returns None if task has no execution_plan_id or plan_branch can't be found,
/// or if the resolved branch equals the project base branch.
async fn get_task_plan_branch(
    task: &crate::domain::entities::Task,
    project: &crate::domain::entities::Project,
    plan_branch_repo: &Option<Arc<dyn crate::domain::repositories::PlanBranchRepository>>,
    task_repo: &Option<Arc<dyn TaskRepository>>,
) -> Option<String> {
    let resolved = resolve_task_base_branch(task, project, plan_branch_repo, task_repo, &None, &None).await;
    // resolve_task_base_branch returns the plan branch for plan tasks, or project base otherwise.
    // We only want the plan branch — check if it's different from project base.
    let project_base = project.base_branch.as_deref().unwrap_or("main");
    if resolved != project_base {
        Some(resolved)
    } else {
        None
    }
}

/// Handle the result of ensure_branches_fresh() for an entry point.
/// Returns Ok(()) if fresh or skipped, Err if needs routing or blocking.
async fn apply_freshness_result(
    result: Result<freshness::FreshnessMetadata, FreshnessAction>,
    task: &crate::domain::entities::Task,
    task_id_str: &str,
    task_repo: &Arc<dyn TaskRepository>,
) -> AppResult<()> {
    let task_id = TaskId::from_string(task_id_str.to_string());
    match result {
        Ok(updated_meta) => {
            // Persist updated freshness metadata (last_freshness_check_at, etc.)
            let mut task_metadata: serde_json::Value = task
                .metadata
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_else(|| serde_json::json!({}));
            updated_meta.merge_into(&mut task_metadata);
            if let Err(e) = task_repo
                .update_metadata(&task_id, Some(task_metadata.to_string()))
                .await
            {
                tracing::warn!(task_id = task_id_str, error = %e, "Failed to persist freshness metadata");
            }
            Ok(())
        }
        Err(FreshnessAction::RouteToMerging { mut freshness_metadata, .. }) => {
            // INVARIANT: freshness_count_incremented_by signals to the corrective handler
            // (task_transition_service.rs) that freshness_conflict_count was already
            // incremented by ensure_branches_fresh(). The conflict marker scan path does
            // NOT set this field, so the corrective handler will increment there instead.
            freshness_metadata.freshness_count_incremented_by =
                Some("ensure_branches_fresh".to_string());
            // Store freshness metadata on task for merger agent context
            let mut task_metadata: serde_json::Value = task
                .metadata
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_else(|| serde_json::json!({}));
            freshness_metadata.merge_into(&mut task_metadata);
            if let Err(e) = task_repo
                .update_metadata(&task_id, Some(task_metadata.to_string()))
                .await
            {
                tracing::warn!(task_id = task_id_str, error = %e, "Failed to persist freshness conflict metadata");
            }
            Err(AppError::BranchFreshnessConflict)
        }
        Err(FreshnessAction::ExecutionBlocked { reason }) => {
            Err(AppError::ExecutionBlocked(reason))
        }
    }
}

/// Create a fresh task branch and worktree for a task entering Executing state.
///
/// Resolves the base branch, computes the standard worktree path via
/// `compute_task_worktree_path`, cleans any stale worktree at that path, then
/// creates (or checks out an existing) branch into the worktree.
///
/// Does NOT update the database — the caller is responsible for persisting the
/// returned `(branch_name, worktree_path)` onto the task.
///
/// # Errors
/// Returns `AppError::ExecutionBlocked` if the git worktree cannot be created.
#[allow(clippy::too_many_arguments)]
async fn create_fresh_branch_and_worktree(
    task: &Task,
    project: &Project,
    task_id_str: &str,
    repo_path: &Path,
    plan_branch_repo: &Option<Arc<dyn PlanBranchRepository>>,
    task_repo_ref: &Option<Arc<dyn TaskRepository>>,
    pr_creation_guard: &Option<Arc<dashmap::DashMap<PlanBranchId, ()>>>,
    github_service: &Option<Arc<dyn GithubServiceTrait>>,
) -> AppResult<(String, std::path::PathBuf)> {
    let branch = format!(
        "ralphx/{}/task-{}",
        slugify(&project.name),
        task_id_str
    );
    let resolved_base = resolve_task_base_branch(
        task,
        project,
        plan_branch_repo,
        task_repo_ref,
        pr_creation_guard,
        github_service,
    )
    .await;
    let base_branch = resolved_base.as_str();

    // Use compute_task_worktree_path for consistent path computation
    let worktree_path_str = compute_task_worktree_path(project, task_id_str);
    let worktree_path_buf = std::path::PathBuf::from(&worktree_path_str);

    // Clean up stale task worktree from a prior execution attempt
    if worktree_path_buf.exists() {
        if let Err(e) = GitService::delete_worktree(repo_path, &worktree_path_buf).await {
            tracing::warn!(
                task_id = task_id_str,
                error = %e,
                "Failed to clean stale task worktree (non-fatal)"
            );
        }
    }

    // Check if branch already exists from a previous execution attempt
    let branch_exists = GitService::branch_exists(repo_path, &branch)
        .await
        .unwrap_or(false);

    // Create worktree — use existing branch if it exists, create new one otherwise
    let result = if branch_exists {
        tracing::info!(
            task_id = task_id_str,
            branch = %branch,
            "Branch already exists, checking out existing branch into worktree"
        );
        GitService::checkout_existing_branch_worktree(repo_path, &worktree_path_buf, &branch).await
    } else {
        GitService::create_worktree(repo_path, &worktree_path_buf, &branch, base_branch).await
    };

    match result {
        Ok(_) => Ok((branch, worktree_path_buf)),
        Err(e) => Err(AppError::ExecutionBlocked(format!(
            "{}: could not create worktree at '{}': {}",
            GIT_ISOLATION_ERROR_PREFIX,
            worktree_path_str,
            e
        ))),
    }
}

#[derive(Default)]
struct MergePromptContext {
    is_validation_recovery: bool,
    is_plan_update_conflict: bool,
    is_source_update_conflict: bool,
    freshness_conflict_count: u32,
    base_branch: Option<String>,
    source_branch: Option<String>,
    target_branch: Option<String>,
}

impl<'a> super::TransitionHandler<'a> {
    /// Check that the task's plan branch is still Active.
    /// Returns Err(ExecutionBlocked) if the branch is Merged or Abandoned.
    /// No-op for non-plan tasks or when repos are unavailable.
    /// Uses `execution_plan_id` (not `session_id`) to handle re-accept flows where
    /// multiple PlanBranch records exist for the same session.
    async fn check_plan_branch_active(&self, task_id_str: &str) -> Result<(), AppError> {
        use crate::domain::entities::PlanBranchStatus;

        let task_repo = match &self.machine.context.services.task_repo {
            Some(repo) => repo,
            None => return Ok(()),
        };
        let plan_branch_repo = match &self.machine.context.services.plan_branch_repo {
            Some(repo) => repo,
            None => return Ok(()),
        };

        let task_id = TaskId::from_string(task_id_str.to_string());
        let task = match task_repo.get_by_id(&task_id).await {
            Ok(Some(t)) => t,
            _ => return Ok(()), // Can't find task, don't block
        };

        let exec_plan_id = match &task.execution_plan_id {
            Some(id) => id,
            None => return Ok(()), // Non-plan task, no guard needed
        };

        if let Ok(Some(branch)) = plan_branch_repo.get_by_execution_plan_id(exec_plan_id).await {
            if !matches!(branch.status, PlanBranchStatus::Active) {
                return Err(AppError::ExecutionBlocked(format!(
                    "Plan branch '{}' is {} — cannot execute task on inactive branch",
                    branch.branch_name, branch.status
                )));
            }
        }

        Ok(())
    }

    /// Run pre-execution setup (worktree_setup + install), store log in metadata.
    /// Returns Err if setup fails in Block/AutoFix mode.
    async fn run_and_store_pre_execution_setup(
        &self,
        task_id_str: &str,
        project_id_str: &str,
        context: &str,      // "execution" or "review"
        metadata_key: &str, // "execution_setup_log" or "review_setup_log"
    ) -> AppResult<()> {
        // Run pre-execution setup (worktree_setup + install) before spawning agent
        if let (Some(ref task_repo), Some(ref project_repo)) = (
            &self.machine.context.services.task_repo,
            &self.machine.context.services.project_repo,
        ) {
            let task_id = TaskId::from_string(task_id_str.to_string());
            let project_id = ProjectId::from_string(project_id_str.to_string());

            let task_result = task_repo.get_by_id(&task_id).await;
            let project_result = project_repo.get_by_id(&project_id).await;

            if let (Ok(Some(task)), Ok(Some(project))) = (task_result, project_result) {
                // Skip pre-exec setup if mode is Off
                use crate::domain::entities::MergeValidationMode;
                if project.merge_validation_mode != MergeValidationMode::Off {
                    // Determine execution directory — MUST be a worktree, never the main repo.
                    // Falling back to working_directory would run install commands in the
                    // user's checkout, potentially disrupting their work.
                    let exec_cwd = if let Some(ref wt_path) = task.worktree_path {
                        std::path::PathBuf::from(wt_path)
                    } else {
                        tracing::warn!(
                            task_id = task_id_str,
                            "Skipping pre-execution setup: task has no worktree_path. \
                             Running install commands in the main repo is not safe."
                        );
                        return Ok(());
                    };

                    // Only run pre-execution setup if exec_cwd exists
                    if !exec_cwd.exists() {
                        tracing::warn!(
                            task_id = task_id_str,
                            exec_cwd = %exec_cwd.display(),
                            "Execution directory does not exist, skipping pre-execution setup"
                        );
                    } else if let Some(setup_result) =
                        super::merge_validation::run_pre_execution_setup(
                            &project,
                            &task,
                            &exec_cwd,
                            task_id_str,
                            self.machine.context.services.app_handle.as_ref(),
                            context,
                            &tokio_util::sync::CancellationToken::new(),
                        )
                        .await
                    {
                        // Store setup log in metadata (using update_metadata for targeted write)
                        if let Ok(Some(task_updated)) = task_repo.get_by_id(&task_id).await {
                            let log_json = serde_json::to_value(&setup_result.log)
                                .unwrap_or_else(|_| serde_json::Value::Array(Vec::new()));

                            let mut metadata_obj =
                                if let Some(json_str) = task_updated.metadata.as_ref() {
                                    serde_json::from_str::<serde_json::Value>(json_str)
                                        .unwrap_or_else(|_| serde_json::json!({}))
                                } else {
                                    serde_json::json!({})
                                };

                            if let Some(obj) = metadata_obj.as_object_mut() {
                                obj.insert(metadata_key.to_string(), log_json);
                            }

                            if let Ok(updated_metadata) = serde_json::to_string(&metadata_obj) {
                                if let Err(e) = task_repo
                                    .update_metadata(&task_id, Some(updated_metadata))
                                    .await
                                {
                                    tracing::warn!(task_id = %task_id, error = %e, "Failed to persist setup log metadata");
                                }
                            }
                        }

                        // Handle setup failure based on merge_validation_mode
                        if !setup_result.success {
                            match project.merge_validation_mode {
                                MergeValidationMode::Block | MergeValidationMode::AutoFix => {
                                    tracing::error!(
                                        task_id = task_id_str,
                                        "Pre-execution setup failed (install command failed). Blocking execution."
                                    );
                                    return Err(AppError::ExecutionBlocked(
                                        format!("Pre-execution setup failed: install command(s) failed. Check {} in task metadata for details.", metadata_key)
                                    ));
                                }
                                MergeValidationMode::Warn => {
                                    tracing::warn!(
                                        task_id = task_id_str,
                                        "Pre-execution setup failed (install command failed). Proceeding with warning."
                                    );
                                    // Store warning in metadata but proceed (using update_metadata for targeted write)
                                    if let Ok(Some(task_updated)) =
                                        task_repo.get_by_id(&task_id).await
                                    {
                                        let mut metadata_obj = if let Some(json_str) =
                                            task_updated.metadata.as_ref()
                                        {
                                            serde_json::from_str::<serde_json::Value>(json_str)
                                                .unwrap_or_else(|_| serde_json::json!({}))
                                        } else {
                                            serde_json::json!({})
                                        };

                                        if let Some(obj) = metadata_obj.as_object_mut() {
                                            obj.insert(
                                                "execution_setup_warning".to_string(),
                                                serde_json::json!(true),
                                            );
                                        }

                                        if let Ok(updated_metadata) =
                                            serde_json::to_string(&metadata_obj)
                                        {
                                            if let Err(e) = task_repo
                                                .update_metadata(&task_id, Some(updated_metadata))
                                                .await
                                            {
                                                tracing::warn!(task_id = %task_id, error = %e, "Failed to persist setup warning metadata");
                                            }
                                        }
                                    }
                                }
                                MergeValidationMode::Off => {
                                    // Already skipped above, but for completeness
                                }
                            }
                        }
                    } // end if let Some(setup_result)
                } // end if project.merge_validation_mode != Off
            } // end if let (Ok(Some(task)), Ok(Some(project)))
        } // end if let (Some(ref task_repo), Some(ref project_repo))
        Ok(())
    }

    async fn enter_pending_review_state(&self) {
        let review_result = self
            .machine
            .context
            .services
            .review_starter
            .start_ai_review(
                &self.machine.context.task_id,
                &self.machine.context.project_id,
            )
            .await;

        match &review_result {
            super::super::services::ReviewStartResult::Started { review_id } => {
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit_with_payload(
                        "review:update",
                        &self.machine.context.task_id,
                        &format!(r#"{{"type":"started","reviewId":"{}"}}"#, review_id),
                    )
                    .await;
            }
            super::super::services::ReviewStartResult::Disabled => {
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit_with_payload(
                        "review:update",
                        &self.machine.context.task_id,
                        r#"{"type":"disabled"}"#,
                    )
                    .await;
            }
            super::super::services::ReviewStartResult::Error(msg) => {
                self.machine
                    .context
                    .services
                    .notifier
                    .notify_with_message("review_error", &self.machine.context.task_id, msg)
                    .await;
            }
        }

        if let Some(ref publisher) = self.machine.context.services.webhook_publisher {
            let payload = serde_json::json!({
                "task_id": self.machine.context.task_id,
                "project_id": self.machine.context.project_id,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });
            publisher
                .publish(
                    ralphx_domain::entities::EventType::ReviewReady,
                    &self.machine.context.project_id,
                    payload,
                )
                .await;
        }
    }

    async fn run_reviewing_freshness_check(
        &self,
        task_id_str: &str,
        project_id_str: &str,
    ) -> AppResult<()> {
        if let (Some(ref task_repo), Some(ref project_repo)) = (
            &self.machine.context.services.task_repo,
            &self.machine.context.services.project_repo,
        ) {
            let task_id_typed = TaskId::from_string(task_id_str.to_string());
            let project_id_typed = ProjectId::from_string(project_id_str.to_string());
            if let (Ok(Some(task)), Ok(Some(project))) = (
                task_repo.get_by_id(&task_id_typed).await,
                project_repo.get_by_id(&project_id_typed).await,
            ) {
                let repo_path = Path::new(&project.working_directory);
                let plan_branch = get_task_plan_branch(
                    &task,
                    &project,
                    &self.machine.context.services.plan_branch_repo,
                    &self.machine.context.services.task_repo,
                )
                .await;
                let config = reconciliation_config();
                let app_handle = self.machine.context.services.app_handle.as_ref();
                let activity_event_repo = self.machine.context.services.activity_event_repo.as_ref();
                let freshness_result = freshness::ensure_branches_fresh(
                    repo_path,
                    &task,
                    &project,
                    task_id_str,
                    plan_branch.as_deref(),
                    app_handle,
                    activity_event_repo,
                    "reviewing",
                    config,
                )
                .await;
                apply_freshness_result(freshness_result, &task, task_id_str, task_repo).await?;
            }
        }

        Ok(())
    }

    async fn ensure_review_worktree_ready(&self, task_id_str: &str) -> AppResult<()> {
        if let (Some(ref task_repo), Some(ref project_repo)) = (
            &self.machine.context.services.task_repo,
            &self.machine.context.services.project_repo,
        ) {
            let task_id_typed = TaskId::from_string(task_id_str.to_string());
            if let Ok(Some(mut task)) = task_repo.get_by_id(&task_id_typed).await {
                if task
                    .worktree_path
                    .as_deref()
                    .map(is_merge_worktree_path)
                    .unwrap_or(false)
                {
                    match project_repo.get_by_id(&task.project_id).await {
                        Ok(Some(project)) => {
                            let repo_path = Path::new(&project.working_directory);
                            match restore_task_worktree(&mut task, &project, repo_path).await {
                                Ok(restored) => {
                                    tracing::info!(
                                        task_id = task_id_str,
                                        restored_path = %restored.display(),
                                        "L2: restored merge-prefixed worktree_path before reviewer spawn"
                                    );
                                    if let Err(e) = task_repo.update(&task).await {
                                        tracing::warn!(
                                            task_id = task_id_str,
                                            error = %e,
                                            "L2: failed to persist restored worktree_path"
                                        );
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        task_id = task_id_str,
                                        error = %e,
                                        "L2: failed to restore task worktree in Reviewing entry — will fail as ReviewWorktreeMissing"
                                    );
                                }
                            }
                        }
                        Ok(None) => {
                            tracing::warn!(
                                task_id = task_id_str,
                                "L2: project not found for worktree restoration"
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                task_id = task_id_str,
                                error = %e,
                                "L2: failed to fetch project for worktree restoration"
                            );
                        }
                    }
                }

                if let Some(ref wt_path_str) = task.worktree_path {
                    let wt_path = std::path::Path::new(wt_path_str);
                    if wt_path.exists() {
                        match crate::application::git_service::GitService::has_conflict_markers(wt_path).await {
                            Ok(true) => {
                                tracing::warn!(
                                    task_id = task_id_str,
                                    worktree = %wt_path.display(),
                                    "Conflict markers detected in worktree before reviewer spawn — routing to merge pipeline"
                                );
                                let mut task_metadata: serde_json::Value = task
                                    .metadata
                                    .as_deref()
                                    .and_then(|s| serde_json::from_str(s).ok())
                                    .unwrap_or_else(|| serde_json::json!({}));
                                task_metadata["conflict_markers_detected"] = serde_json::json!(true);
                                task_metadata["branch_freshness_conflict"] = serde_json::json!(true);
                                task_metadata["freshness_origin_state"] =
                                    serde_json::json!("reviewing");
                                if let Err(e) = task_repo
                                    .update_metadata(
                                        &task_id_typed,
                                        Some(task_metadata.to_string()),
                                    )
                                    .await
                                {
                                    tracing::warn!(
                                        task_id = task_id_str,
                                        error = %e,
                                        "Failed to persist conflict marker metadata"
                                    );
                                }
                                return Err(AppError::BranchFreshnessConflict);
                            }
                            Ok(false) => {
                                tracing::debug!(
                                    task_id = task_id_str,
                                    worktree = %wt_path.display(),
                                    "Conflict marker scan passed — worktree is clean"
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    task_id = task_id_str,
                                    error = %e,
                                    "Conflict marker scan failed — proceeding with review anyway"
                                );
                            }
                        }
                    } else {
                        tracing::error!(
                            task_id = task_id_str,
                            worktree = %wt_path.display(),
                            "Reviewer spawn blocked: worktree directory does not exist"
                        );
                        let mut task_meta: serde_json::Value = task
                            .metadata
                            .as_deref()
                            .and_then(|s| serde_json::from_str(s).ok())
                            .unwrap_or_else(|| serde_json::json!({}));
                        task_meta["worktree_missing_at_review"] = serde_json::json!(true);
                        if let Err(me) = task_repo
                            .update_metadata(&task_id_typed, Some(task_meta.to_string()))
                            .await
                        {
                            tracing::warn!(
                                task_id = task_id_str,
                                error = %me,
                                "Failed to persist worktree_missing_at_review metadata"
                            );
                        }
                        return Err(crate::error::AppError::ReviewWorktreeMissing);
                    }
                } else {
                    tracing::error!(
                        task_id = task_id_str,
                        "Reviewer spawn blocked: task has no worktree_path set"
                    );
                    return Err(crate::error::AppError::ReviewWorktreeMissing);
                }
            }
        }

        Ok(())
    }

    async fn spawn_reviewer_agent(&self, task_id: &str) {
        let prompt = format!("Review task: {}", task_id);

        tracing::info!(
            task_id = task_id,
            "on_enter(Reviewing): Spawning reviewer agent via ChatService"
        );

        let result = self
            .machine
            .context
            .services
            .chat_service
            .send_message(
                crate::domain::entities::ChatContextType::Review,
                task_id,
                &prompt,
                Default::default(),
            )
            .await;

        match result {
            Ok(result) if result.was_queued => {
                tracing::info!(
                    task_id = task_id,
                    "Agent already running for this task — treating on_enter(Reviewing) as no-op"
                );
            }
            Ok(_) => {
                tracing::info!(task_id = task_id, "Reviewer agent spawned successfully");
            }
            Err(e) => {
                tracing::error!(task_id = task_id, error = %e, "Failed to spawn reviewer agent");
                record_reviewer_spawn_failure(
                    &self.machine.context.services.task_repo,
                    task_id,
                    &e.to_string(),
                )
                .await;
            }
        }
    }

    async fn enter_reviewing_state(&self) -> AppResult<()> {
        let task_id_str = self.machine.context.task_id.as_str();
        let project_id_str = self.machine.context.project_id.as_str();

        self.run_reviewing_freshness_check(task_id_str, project_id_str)
            .await?;
        self.ensure_review_worktree_ready(task_id_str).await?;
        self.run_and_store_pre_execution_setup(
            task_id_str,
            project_id_str,
            "review",
            "review_setup_log",
        )
        .await?;
        self.spawn_reviewer_agent(task_id_str).await;

        Ok(())
    }

    async fn reset_stale_steps_on_entry(&self, task_id_str: &str) {
        if let Some(ref step_repo) = self.machine.context.services.step_repo {
            let task_id_typed = TaskId::from_string(task_id_str.to_string());
            match step_repo.reset_all_to_pending(&task_id_typed).await {
                Ok(count) if count > 0 => {
                    tracing::info!(
                        task_id = task_id_str,
                        count,
                        "Reset stale steps to Pending on re-entry"
                    );
                    self.machine
                        .context
                        .services
                        .event_emitter
                        .emit("step:updated", task_id_str)
                        .await;
                }
                Err(e) => {
                    tracing::warn!(
                        task_id = task_id_str,
                        error = %e,
                        "Failed to reset steps on re-entry"
                    );
                }
                _ => {}
            }
        }
    }

    async fn run_execution_freshness_check(
        &self,
        task_id_str: &str,
        project_id_str: &str,
        stage: &'static str,
    ) -> AppResult<()> {
        if let (Some(ref task_repo), Some(ref project_repo)) = (
            &self.machine.context.services.task_repo,
            &self.machine.context.services.project_repo,
        ) {
            let task_id_typed = TaskId::from_string(task_id_str.to_string());
            let project_id_typed = ProjectId::from_string(project_id_str.to_string());
            if let (Ok(Some(task)), Ok(Some(project))) = (
                task_repo.get_by_id(&task_id_typed).await,
                project_repo.get_by_id(&project_id_typed).await,
            ) {
                let repo_path = Path::new(&project.working_directory);
                let plan_branch = get_task_plan_branch(
                    &task,
                    &project,
                    &self.machine.context.services.plan_branch_repo,
                    &self.machine.context.services.task_repo,
                )
                .await;
                let config = reconciliation_config();
                let app_handle = self.machine.context.services.app_handle.as_ref();
                let activity_event_repo = self.machine.context.services.activity_event_repo.as_ref();
                let freshness_result = freshness::ensure_branches_fresh(
                    repo_path,
                    &task,
                    &project,
                    task_id_str,
                    plan_branch.as_deref(),
                    app_handle,
                    activity_event_repo,
                    stage,
                    config,
                )
                .await;
                apply_freshness_result(freshness_result, &task, task_id_str, task_repo).await?;
            }
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn ensure_executing_branch_and_worktree(
        &self,
        task_id_str: &str,
        project_id_str: &str,
    ) -> AppResult<()> {
        if let (Some(ref task_repo), Some(ref project_repo)) = (
            &self.machine.context.services.task_repo,
            &self.machine.context.services.project_repo,
        ) {
            let task_id = TaskId::from_string(task_id_str.to_string());
            let project_id = ProjectId::from_string(project_id_str.to_string());

            let task_result = task_repo.get_by_id(&task_id).await;
            let project_result = project_repo.get_by_id(&project_id).await;

            if let (Ok(Some(mut task)), Ok(Some(project))) = (task_result, project_result) {
                let repo_path = Path::new(&project.working_directory);
                let plan_branch_repo = &self.machine.context.services.plan_branch_repo;
                let task_repo_ref = &self.machine.context.services.task_repo;
                let pr_creation_guard_ref = &self.machine.context.services.pr_creation_guard;
                let github_service_ref = &self.machine.context.services.github_service;

                let mut branch_self_healed = false;
                if let Some(ref branch) = task.task_branch.clone() {
                    let branch_exists = GitService::branch_exists(repo_path, branch)
                        .await
                        .unwrap_or(false);
                    if !branch_exists {
                        tracing::warn!(
                            task_id = task_id_str,
                            branch = %branch,
                            "Stale task_branch detected — branch deleted, self-healing by creating fresh branch"
                        );
                        if let Some(ref stored_wt) = task.worktree_path.clone() {
                            let stored = std::path::PathBuf::from(stored_wt);
                            if stored.exists() {
                                let _ = GitService::delete_worktree(repo_path, &stored).await;
                            }
                        }
                        let expected_wt_path_str = compute_task_worktree_path(&project, task_id_str);
                        let expected_wt_path = std::path::PathBuf::from(&expected_wt_path_str);
                        if expected_wt_path.exists() {
                            let _ = GitService::delete_worktree(repo_path, &expected_wt_path).await;
                        }
                        task.task_branch = None;
                        task.worktree_path = None;
                        task.merge_commit_sha = None;
                        task.touch();
                        if let Err(e) = task_repo.update(&task).await {
                            tracing::error!(
                                task_id = task_id_str,
                                error = %e,
                                "Failed to clear stale git refs during self-heal"
                            );
                        }
                        match create_fresh_branch_and_worktree(
                            &task,
                            &project,
                            task_id_str,
                            repo_path,
                            plan_branch_repo,
                            task_repo_ref,
                            pr_creation_guard_ref,
                            github_service_ref,
                        )
                        .await
                        {
                            Ok((new_branch, new_worktree)) => {
                                task.task_branch = Some(new_branch.clone());
                                task.worktree_path = Some(new_worktree.to_string_lossy().to_string());
                                task.touch();
                                tracing::info!(
                                    task_id = task_id_str,
                                    branch = %new_branch,
                                    worktree_path = %new_worktree.display(),
                                    "Self-healed: created fresh branch and worktree for deleted branch"
                                );
                                if let Err(e) = task_repo.update(&task).await {
                                    tracing::error!(
                                        task_id = task_id_str,
                                        error = %e,
                                        "Failed to persist self-healed branch info"
                                    );
                                }
                                branch_self_healed = true;
                            }
                            Err(e) => return Err(e),
                        }
                    }
                }

                if !branch_self_healed {
                    if task.task_branch.is_none() {
                        match create_fresh_branch_and_worktree(
                            &task,
                            &project,
                            task_id_str,
                            repo_path,
                            plan_branch_repo,
                            task_repo_ref,
                            pr_creation_guard_ref,
                            github_service_ref,
                        )
                        .await
                        {
                            Ok((branch_name, worktree_path)) => {
                                tracing::info!(
                                    task_id = task_id_str,
                                    branch = %branch_name,
                                    worktree_path = %worktree_path.display(),
                                    "Created worktree with task branch"
                                );
                                task.task_branch = Some(branch_name);
                                task.worktree_path =
                                    Some(worktree_path.to_string_lossy().to_string());
                                task.touch();
                                if let Err(e) = task_repo.update(&task).await {
                                    tracing::error!(error = %e, "Failed to persist task branch info");
                                }
                            }
                            Err(e) => return Err(e),
                        }
                    }

                    if let Ok(Some(mut task)) = task_repo.get_by_id(&task_id).await {
                        if let Some(ref branch) = task.task_branch.clone() {
                            let expected_wt_path = compute_task_worktree_path(&project, task_id_str);
                            let expected_wt_buf = std::path::PathBuf::from(&expected_wt_path);
                            let stored_path_exists = task
                                .worktree_path
                                .as_ref()
                                .map(|p| std::path::PathBuf::from(p).exists())
                                .unwrap_or(false);
                            let expected_path_exists = expected_wt_buf.exists();
                            if !stored_path_exists && !expected_path_exists {
                                let branch_exists = GitService::branch_exists(repo_path, branch)
                                    .await
                                    .unwrap_or(false);
                                if !branch_exists {
                                    return Err(AppError::ExecutionBlocked(format!(
                                        "{}: branch '{}' no longer exists (deleted during prior merge cleanup). Task needs manual recovery or reset to Ready.",
                                        GIT_ISOLATION_ERROR_PREFIX,
                                        branch
                                    )));
                                }
                                tracing::info!(
                                    task_id = task_id_str,
                                    branch = %branch,
                                    expected_wt = %expected_wt_path,
                                    "Worktree missing for task with existing branch — re-creating"
                                );
                                match GitService::checkout_existing_branch_worktree(
                                    repo_path,
                                    &expected_wt_buf,
                                    branch,
                                )
                                .await
                                {
                                    Ok(_) => {
                                        task.worktree_path = Some(expected_wt_path);
                                        task.touch();
                                        if let Err(e) = task_repo.update(&task).await {
                                            tracing::error!(
                                                error = %e,
                                                "Failed to persist re-created worktree_path"
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        return Err(AppError::ExecutionBlocked(format!(
                                            "{}: could not re-create missing worktree for task with existing branch: {}",
                                            GIT_ISOLATION_ERROR_PREFIX,
                                            e
                                        )));
                                    }
                                }
                            } else if !stored_path_exists && expected_path_exists {
                                task.worktree_path = Some(expected_wt_path);
                                task.touch();
                                if let Err(e) = task_repo.update(&task).await {
                                    tracing::error!(
                                        error = %e,
                                        "Failed to update stale worktree_path in DB"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn build_execution_prompt(
        &self,
        task_id_str: &str,
        base_prompt: String,
    ) -> String {
        let mut prompt = base_prompt;
        if let Some(ref task_repo) = self.machine.context.services.task_repo {
            let task_id_typed = TaskId::from_string(task_id_str.to_string());
            if let Ok(Some(task)) = task_repo.get_by_id(&task_id_typed).await {
                if let Some(note) = extract_restart_note(task.metadata.as_deref()) {
                    prompt = format!("{}\n\nUser note: {}", prompt, note);
                    let cleared = MetadataUpdate::new()
                        .with_null("restart_note")
                        .merge_into(task.metadata.as_deref());
                    if let Err(e) = task_repo
                        .update_metadata(&task_id_typed, Some(cleared))
                        .await
                    {
                        tracing::warn!(
                            task_id = task_id_str,
                            error = %e,
                            "Failed to clear restart_note from metadata"
                        );
                    }
                }
            }
        }
        prompt
    }

    async fn send_task_execution_message(
        &self,
        task_id_str: &str,
        prompt: &str,
        failure_log: &str,
    ) -> AppResult<()> {
        match self
            .machine
            .context
            .services
            .chat_service
            .send_message(
                crate::domain::entities::ChatContextType::TaskExecution,
                task_id_str,
                prompt,
                Default::default(),
            )
            .await
        {
            Ok(result) if result.was_queued => {
                tracing::info!(
                    task_id = task_id_str,
                    "Agent already running for this task — treating on_enter as no-op"
                );
                Ok(())
            }
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::error!(
                    task_id = task_id_str,
                    error = %e,
                    "{}",
                    failure_log
                );
                Err(AppError::ExecutionBlocked(format!(
                    "Failed to start agent: {}",
                    e
                )))
            }
        }
    }

    async fn enter_executing_state(&self) -> AppResult<()> {
        let task_id_str = self.machine.context.task_id.as_str();
        let project_id_str = self.machine.context.project_id.as_str();

        self.check_plan_branch_active(task_id_str).await?;
        self.reset_stale_steps_on_entry(task_id_str).await;
        self.ensure_executing_branch_and_worktree(task_id_str, project_id_str)
            .await?;
        self.run_execution_freshness_check(task_id_str, project_id_str, "executing")
            .await?;
        self.run_and_store_pre_execution_setup(
            task_id_str,
            project_id_str,
            "execution",
            "execution_setup_log",
        )
        .await?;

        let prompt = self
            .build_execution_prompt(task_id_str, format!("Execute task: {}", task_id_str))
            .await;
        tracing::debug!(
            task_id = task_id_str,
            prompt_len = prompt.len(),
            "Transition handler sending task_execution message"
        );
        self.send_task_execution_message(
            task_id_str,
            &prompt,
            "Failed to send task execution message — agent not started",
        )
        .await
    }

    async fn enter_reexecuting_state(&self) -> AppResult<()> {
        let task_id_str = self.machine.context.task_id.as_str();
        let project_id_str = self.machine.context.project_id.as_str();

        self.check_plan_branch_active(task_id_str).await?;
        self.reset_stale_steps_on_entry(task_id_str).await;
        self.run_execution_freshness_check(task_id_str, project_id_str, "re_executing")
            .await?;
        self.run_and_store_pre_execution_setup(
            task_id_str,
            project_id_str,
            "execution",
            "execution_setup_log",
        )
        .await?;

        let prompt = self
            .build_execution_prompt(
                task_id_str,
                format!("Re-execute task (revision): {}", task_id_str),
            )
            .await;
        self.send_task_execution_message(
            task_id_str,
            &prompt,
            "Failed to send re-execution message — agent not started",
        )
        .await
    }

    async fn load_merge_prompt_context(&self, task_id: &str) -> MergePromptContext {
        let Some(task_repo) = &self.machine.context.services.task_repo else {
            return MergePromptContext::default();
        };

        let tid = TaskId::from_string(task_id.to_string());
        let Ok(Some(task)) = task_repo.get_by_id(&tid).await else {
            return MergePromptContext::default();
        };

        let meta = task
            .metadata
            .as_ref()
            .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok());

        MergePromptContext {
            is_validation_recovery: meta
                .as_ref()
                .and_then(|v| v.get("validation_recovery")?.as_bool())
                .unwrap_or(false),
            is_plan_update_conflict: meta
                .as_ref()
                .and_then(|v| v.get("plan_update_conflict")?.as_bool())
                .unwrap_or(false),
            is_source_update_conflict: meta
                .as_ref()
                .and_then(|v| v.get("source_update_conflict")?.as_bool())
                .unwrap_or(false),
            freshness_conflict_count: meta
                .as_ref()
                .and_then(|v| v.get("freshness_conflict_count")?.as_u64())
                .unwrap_or(0) as u32,
            base_branch: meta
                .as_ref()
                .and_then(|v| v.get("base_branch")?.as_str().map(String::from)),
            source_branch: meta
                .as_ref()
                .and_then(|v| v.get("source_branch")?.as_str().map(String::from)),
            target_branch: meta
                .as_ref()
                .and_then(|v| v.get("target_branch")?.as_str().map(String::from)),
        }
    }

    fn build_merge_prompt(&self, task_id: &str, context: &MergePromptContext) -> String {
        let prompt = if context.is_validation_recovery {
            format!(
                "Fix validation failures for task: {}. The merge succeeded but post-merge \
                 validation commands failed. The failing code is on the target branch. \
                 Read the validation failures from task context, fix the code, run validation \
                 to confirm, then commit your fixes.",
                task_id
            )
        } else if context.is_plan_update_conflict {
            let base_branch = context
                .base_branch
                .clone()
                .unwrap_or_else(|| "main".to_string());
            let plan_branch = context.target_branch.clone().unwrap_or_default();
            format!(
                "Resolve the plan branch update conflict for task {task_id}.\n\n\
                 The plan branch ({plan_branch}) needs to be updated from {base_branch} \
                 before the task can merge, but there are merge conflicts.\n\n\
                 Your working directory is the merge worktree where the plan branch is \
                 already checked out. DO NOT merge the task branch — the system handles \
                 that automatically after you finish.\n\n\
                 Steps:\n\
                 1. Run `git status` to confirm you are on the plan branch ({plan_branch})\n\
                 2. Run `git merge {base_branch}` to trigger the merge and expose conflicts\n\
                 3. Resolve all conflict markers in the conflicted files\n\
                 4. Stage resolved files: `git add <files>`\n\
                 5. Commit: `git commit --no-edit`\n\
                 6. Exit — the system will automatically retry the task merge\n\n\
                 If the conflict is too complex, call report_incomplete with a description.",
                task_id = task_id,
                base_branch = base_branch,
                plan_branch = plan_branch,
            )
        } else if context.is_source_update_conflict {
            let source_branch = context.source_branch.clone().unwrap_or_default();
            let target_branch = context.target_branch.clone().unwrap_or_default();
            format!(
                "Resolve the source branch update conflict for task {task_id}.\n\n\
                 The task branch ({source_branch}) needs to incorporate changes from \
                 {target_branch} before it can be merged, but there are conflicts.\n\n\
                 Your working directory is the merge worktree with the task branch checked out.\n\n\
                 Steps:\n\
                 1. Run `git status` to confirm you are on the task branch ({source_branch})\n\
                 2. Run `git merge {target_branch}` to trigger the merge and expose conflicts\n\
                 3. Resolve all conflict markers in the conflicted files\n\
                 4. Stage resolved files: `git add <files>`\n\
                 5. Commit: `git commit --no-edit`\n\
                 6. Exit — the system will automatically retry the task merge\n\n\
                 If the conflict is too complex, call report_incomplete with a description.",
                task_id = task_id,
                source_branch = source_branch,
                target_branch = target_branch,
            )
        } else {
            format!("Resolve merge conflicts for task: {}", task_id)
        };

        if context.freshness_conflict_count > 1
            && (context.is_plan_update_conflict || context.is_source_update_conflict)
        {
            let config = reconciliation_config();
            format!(
                "{}\n\nIMPORTANT: This is retry {} of {}. Previous resolution \
                 attempts did not fully resolve the staleness. Take extra care to \
                 resolve ALL conflicts completely. If you cannot resolve cleanly, \
                 call report_incomplete rather than committing a partial resolution.",
                prompt,
                context.freshness_conflict_count,
                config.freshness_max_conflict_retries
            )
        } else {
            prompt
        }
    }

    /// Execute on-enter dispatch for all state arms.
    ///
    /// Called by `on_enter` in side_effects.rs.
    async fn enter_merging_state(&self) -> AppResult<()> {
        // Phase 2 of merge workflow: Spawn merger agent for conflict resolution.
        // Keep this on a separate boxed future so recovery paths do not overflow the
        // default thread stack in debug/test builds.
        let task_id = &self.machine.context.task_id;

        // === PR-MODE GUARD (AD17) ===
        // If this task is in PR mode (pr_eligible=true, pr_number IS NOT NULL),
        // skip the worktree setup and merger agent spawn entirely.
        // The PR poller handles merge detection; on_exit(Merging) decrements the slot.
        if let (Some(ref plan_branch_repo), Some(ref project_repo)) = (
            &self.machine.context.services.plan_branch_repo,
            &self.machine.context.services.project_repo,
        ) {
            let tid = TaskId::from_string(task_id.clone());
            let project_id = ProjectId::from_string(self.machine.context.project_id.clone());
            if let (Ok(Some(plan_branch)), Ok(Some(_project))) = (
                plan_branch_repo.get_by_merge_task_id(&tid).await,
                project_repo.get_by_id(&project_id).await,
            ) {
                if let (true, Some(pr_number)) = (plan_branch.pr_eligible, plan_branch.pr_number) {
                    tracing::info!(
                        task_id = task_id.as_str(),
                        pr_number = pr_number,
                        "on_enter(Merging): PR mode — skipping merger agent, starting poller"
                    );

                    // PR-mode execution slot: increment running count.
                    // Guard: only increment on first entry (re-entry from reconciler
                    // should not double-increment if poller is already running).
                    let already_polling = self
                        .machine
                        .context
                        .services
                        .pr_poller_registry
                        .as_ref()
                        .map(|r| r.is_polling(&tid))
                        .unwrap_or(false);

                    if !already_polling {
                        if let Some(ref execution_state) = self.machine.context.services.execution_state
                        {
                            execution_state.increment_running();
                            tracing::debug!(
                                task_id = task_id.as_str(),
                                "PR-mode Merging: incremented execution slot"
                            );
                        }
                    }

                    // Start the PR merge poller.
                    if let Some(ref registry) = self.machine.context.services.pr_poller_registry {
                        if let Ok(Some(project_for_poller)) =
                            project_repo.get_by_id(&project_id).await
                        {
                            let working_dir =
                                std::path::PathBuf::from(&project_for_poller.working_directory);
                            // source_branch = the base branch the plan branch was created from (e.g. "main")
                            let base_branch = plan_branch.source_branch.clone();
                            if let Some(ref ts) = self.machine.context.services.transition_service {
                                registry.start_polling(
                                    tid.clone(),
                                    plan_branch.id.clone(),
                                    pr_number,
                                    working_dir,
                                    base_branch,
                                    Arc::clone(ts),
                                );
                                tracing::info!(
                                    task_id = task_id.as_str(),
                                    pr_number = pr_number,
                                    "on_enter(Merging): started PR merge poller"
                                );
                            } else {
                                tracing::warn!(
                                    task_id = task_id.as_str(),
                                    pr_number = pr_number,
                                    "on_enter(Merging): PR mode but transition_service not wired — poller not started"
                                );
                            }
                        }
                    }

                    return Ok(());
                }
            }
        }
        // === END PR-MODE GUARD ===

        // Clean up merge worktree before spawning merger agent.
        // - Symlink removal: ALWAYS (symlinks cause false conflicts for the agent)
        // - Git abort: only on recovery re-entry (stale rebase/merge from prior attempt)
        // - Worktree creation: when missing (BranchFreshnessConflict path from
        //   on_enter(Executing/ReExecuting/Reviewing) — the freshness path sets metadata
        //   flags and returns BranchFreshnessConflict; task_transition_service transitions
        //   to Merging and calls on_enter(Merging) without creating a merge worktree)
        if let (Some(ref task_repo), Some(ref project_repo)) = (
            &self.machine.context.services.task_repo,
            &self.machine.context.services.project_repo,
        ) {
            let project_id = ProjectId::from_string(self.machine.context.project_id.clone());
            if let Ok(Some(project)) = project_repo.get_by_id(&project_id).await {
                let wt_path =
                    std::path::PathBuf::from(compute_merge_worktree_path(&project, task_id));

                // --- Create merge worktree if missing (BranchFreshnessConflict path) ---
                // The normal merge pipeline path (side_effects.rs) always creates the
                // worktree before on_enter(Merging), so this block is a no-op on that
                // path. It only runs when on_enter(Merging) is reached via
                // BranchFreshnessConflict handling (mod.rs handle_transition or
                // task_transition_service auto-transition), where no merge worktree was
                // created.
                //
                // Guard: only attempt if the project working directory exists as a valid
                // git repo. Tests use nonexistent paths intentionally; skipping worktree
                // creation there preserves test behavior.
                let repo_path = std::path::Path::new(&project.working_directory);
                if !wt_path.exists() && repo_path.exists() {
                    let tid = TaskId::from_string(task_id.clone());
                    if let Ok(Some(task)) = task_repo.get_by_id(&tid).await {
                        let meta = task
                            .metadata
                            .as_ref()
                            .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok());
                        let is_plan_conflict = meta
                            .as_ref()
                            .and_then(|v| v.get("plan_update_conflict")?.as_bool())
                            .unwrap_or(false);
                        let is_source_conflict = meta
                            .as_ref()
                            .and_then(|v| v.get("source_update_conflict")?.as_bool())
                            .unwrap_or(false);
                        let meta_source_branch = meta.as_ref().and_then(|v| {
                            v.get("source_branch")?.as_str().map(String::from)
                        });
                        let meta_target_branch = meta.as_ref().and_then(|v| {
                            v.get("target_branch")?.as_str().map(String::from)
                        });

                        // Determine which branch to checkout in the merge worktree.
                        // plan_update_conflict: checkout target (plan branch) so agent runs `git merge base_branch`
                        // source_update_conflict: checkout source (task branch) so agent runs `git merge target_branch`
                        // conflict_markers_detected (from Reviewing): checkout task branch so agent resolves markers
                        // fallback: use task's task_branch (regular freshness-conflict path)
                        let checkout_branch: Option<String> = if is_plan_conflict {
                            meta_target_branch.or_else(|| project.base_branch.clone())
                        } else if is_source_conflict {
                            meta_source_branch.or_else(|| task.task_branch.clone())
                        } else {
                            // conflict_markers_detected or generic freshness conflict:
                            // use the task branch (the branch with conflict markers)
                            task.task_branch.clone()
                        };

                        if let Some(ref branch) = checkout_branch {
                            // Pre-delete task worktree: git rejects two worktrees on the
                            // same branch (source_update_conflict and conflict_markers
                            // cases both check out the task branch).
                            let task_wt_str =
                                super::merge_helpers::compute_task_worktree_path(&project, task_id);
                            let task_wt_path = std::path::PathBuf::from(&task_wt_str);
                            super::merge_helpers::pre_delete_worktree(
                                repo_path,
                                &task_wt_path,
                                task_id,
                            )
                            .await;

                            // Pre-delete plan-update worktree (plan_update_conflict case:
                            // plan branch may still be checked out in plan-update worktree).
                            let plan_update_wt_str =
                                super::merge_helpers::compute_plan_update_worktree_path(
                                    &project, task_id,
                                );
                            let plan_update_wt_path =
                                std::path::PathBuf::from(&plan_update_wt_str);
                            super::merge_helpers::pre_delete_worktree(
                                repo_path,
                                &plan_update_wt_path,
                                task_id,
                            )
                            .await;

                            // Create the merge worktree with the appropriate branch.
                            match GitService::checkout_existing_branch_worktree(
                                repo_path, &wt_path, branch,
                            )
                            .await
                            {
                                Ok(_) => {
                                    tracing::info!(
                                        task_id = task_id,
                                        branch = %branch,
                                        worktree = %wt_path.display(),
                                        is_plan_conflict = is_plan_conflict,
                                        is_source_conflict = is_source_conflict,
                                        "on_enter(Merging): Created merge worktree for freshness-conflict path"
                                    );
                                    // Persist worktree_path to DB so resolve_working_directory
                                    // (called inside send_message) can find the merge worktree.
                                    // Symlink removal runs in the wt_path.exists() block below.
                                    if let Ok(Some(mut fresh_task)) = task_repo.get_by_id(&tid).await
                                    {
                                        fresh_task.worktree_path =
                                            Some(wt_path.to_string_lossy().to_string());
                                        fresh_task.touch();
                                        if let Err(e) = task_repo.update(&fresh_task).await {
                                            tracing::warn!(
                                                task_id = task_id,
                                                error = %e,
                                                "on_enter(Merging): Failed to persist worktree_path — cleaning up orphan"
                                            );
                                            let _ =
                                                GitService::delete_worktree(repo_path, &wt_path).await;
                                            // Fall through: let the agent spawn attempt run;
                                            // it will fail with a clear error and reconciler will retry.
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        task_id = task_id,
                                        branch = %branch,
                                        error = %e,
                                        "on_enter(Merging): Failed to create merge worktree on freshness path"
                                    );
                                    // Fall through: let the agent spawn attempt proceed;
                                    // if it fails, record_merger_spawn_failure will run
                                    // in the spawn error handler below.
                                }
                            }
                        } else {
                            tracing::warn!(
                                task_id = task_id,
                                "on_enter(Merging): No merge worktree and no branch to checkout from metadata"
                            );
                            // Fall through: let the agent spawn attempt proceed.
                        }
                    }
                }
                // --- END: Create merge worktree if missing ---

                if wt_path.exists() {
                    // Abort stale rebase/merge from prior attempt (recovery or retry)
                    super::merge_helpers::clean_stale_git_state(&wt_path, task_id).await;

                    // Always: remove worktree symlinks that cause false conflicts.
                    // The merger agent's validation step re-creates them via worktree_setup.
                    for rel in &[
                        "node_modules",
                        "src-tauri/target",
                        "ralphx-plugin/ralphx-mcp-server/node_modules",
                    ] {
                        let sym = wt_path.join(rel);
                        if sym.is_symlink() {
                            tracing::info!(
                                task_id = task_id,
                                path = %sym.display(),
                                "on_enter(Merging): Removing worktree symlink"
                            );
                            if let Err(e) = std::fs::remove_file(&sym) {
                                tracing::warn!(task_id = task_id, path = %sym.display(), error = %e, "Failed to remove worktree symlink");
                            }
                        }
                    }
                }
            }
        }

        let prompt_context = self.load_merge_prompt_context(task_id).await;
        let prompt = self.build_merge_prompt(task_id, &prompt_context);

        tracing::info!(
            task_id = task_id,
            is_validation_recovery = prompt_context.is_validation_recovery,
            is_plan_update_conflict = prompt_context.is_plan_update_conflict,
            is_source_update_conflict = prompt_context.is_source_update_conflict,
            freshness_conflict_count = prompt_context.freshness_conflict_count,
            "on_enter(Merging): Spawning merger agent via ChatService"
        );

        // Use ChatService with Merge context type for the merger agent
        let result = self
            .machine
            .context
            .services
            .chat_service
            .send_message(
                crate::domain::entities::ChatContextType::Merge,
                task_id,
                &prompt,
                Default::default(),
            )
            .await;

        match result {
            Ok(result) if result.was_queued => {
                tracing::info!(
                    task_id = task_id,
                    "Agent already running for this task — treating on_enter(Merging) as no-op"
                );
                Ok(())
            }
            Ok(_) => {
                tracing::info!(task_id = task_id, "Merger agent spawned successfully");
                Ok(())
            }
            Err(e) => {
                tracing::error!(task_id = task_id, error = %e, "Failed to spawn merger agent");
                record_merger_spawn_failure(
                    &self.machine.context.services.task_repo,
                    task_id,
                    &e.to_string(),
                )
                .await;
                Ok(())
            }
        }
    }

    pub(super) async fn on_enter_dispatch(&self, state: &State) -> AppResult<()> {
        match state {
            State::Ready => {
                // When entering Ready, spawn QA prep agent if enabled
                if self.machine.context.qa_enabled {
                    self.machine
                        .context
                        .services
                        .agent_spawner
                        .spawn_background("qa-prep", &self.machine.context.task_id)
                        .await;
                }

                // Delay auto-scheduling so UI sees task "settle" in Ready column
                // before it potentially moves to Executing (user-visible → ready_settle_ms)
                if let Some(ref scheduler) = self.machine.context.services.task_scheduler {
                    let scheduler = Arc::clone(scheduler);
                    let ready_settle_ms = scheduler_config().ready_settle_ms;
                    tokio::spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_millis(ready_settle_ms))
                            .await;
                        scheduler.try_schedule_ready_tasks().await;
                    });
                }
            }
            State::Executing => {
                self.enter_executing_state().await?;
            }
            State::QaRefining => {
                // Set trigger_origin="qa" for QA cycle (skip if already set by transition_task_with_metadata)
                if let Some(ref task_repo) = self.machine.context.services.task_repo {
                    let task_id = TaskId::from_string(self.machine.context.task_id.clone());
                    if let Ok(Some(task)) = task_repo.get_by_id(&task_id).await {
                        if !MetadataUpdate::key_exists_in(
                            "trigger_origin",
                            task.metadata.as_deref(),
                        ) {
                            // Fallback: metadata not pre-computed, write it now for backward compatibility
                            let metadata_update =
                                super::metadata_builder::build_trigger_origin_metadata("qa");
                            let merged_metadata =
                                metadata_update.merge_into(task.metadata.as_deref());

                            if let Err(e) = task_repo
                                .update_metadata(&task_id, Some(merged_metadata))
                                .await
                            {
                                tracing::error!(
                                    task_id = %self.machine.context.task_id,
                                    error = %e,
                                    "Failed to set trigger_origin=qa in metadata"
                                );
                            }
                        } else {
                            tracing::debug!(
                                task_id = %self.machine.context.task_id,
                                "Skipping metadata write - trigger_origin already present (pre-computed)"
                            );
                        }
                    }
                }

                // Wait for QA prep if not complete, then spawn QA refiner
                if !self.machine.context.qa_prep_complete {
                    self.machine
                        .context
                        .services
                        .agent_spawner
                        .wait_for("qa-prep", &self.machine.context.task_id)
                        .await;
                }
                self.machine
                    .context
                    .services
                    .agent_spawner
                    .spawn("qa-refiner", &self.machine.context.task_id)
                    .await;
            }
            State::QaTesting => {
                // Set trigger_origin="qa" for QA cycle (skip if already set by transition_task_with_metadata)
                if let Some(ref task_repo) = self.machine.context.services.task_repo {
                    let task_id = TaskId::from_string(self.machine.context.task_id.clone());
                    if let Ok(Some(task)) = task_repo.get_by_id(&task_id).await {
                        if !MetadataUpdate::key_exists_in(
                            "trigger_origin",
                            task.metadata.as_deref(),
                        ) {
                            // Fallback: metadata not pre-computed, write it now for backward compatibility
                            let metadata_update =
                                super::metadata_builder::build_trigger_origin_metadata("qa");
                            let merged_metadata =
                                metadata_update.merge_into(task.metadata.as_deref());

                            if let Err(e) = task_repo
                                .update_metadata(&task_id, Some(merged_metadata))
                                .await
                            {
                                tracing::error!(
                                    task_id = %self.machine.context.task_id,
                                    error = %e,
                                    "Failed to set trigger_origin=qa in metadata"
                                );
                            }
                        } else {
                            tracing::debug!(
                                task_id = %self.machine.context.task_id,
                                "Skipping metadata write - trigger_origin already present (pre-computed)"
                            );
                        }
                    }
                }

                // Spawn QA tester agent
                self.machine
                    .context
                    .services
                    .agent_spawner
                    .spawn("qa-tester", &self.machine.context.task_id)
                    .await;
            }
            State::QaPassed => {
                // Emit QA passed event
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("qa_passed", &self.machine.context.task_id)
                    .await;
            }
            State::QaFailed(data) => {
                // Emit QA failed event and notify user
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("qa_failed", &self.machine.context.task_id)
                    .await;

                // Notify user if not already notified
                if !data.notified {
                    let message = format!("QA tests failed: {} failure(s)", data.failure_count());
                    self.machine
                        .context
                        .services
                        .notifier
                        .notify_with_message("qa_failed", &self.machine.context.task_id, &message)
                        .await;
                }
            }
            State::PendingReview => {
                self.enter_pending_review_state().await;
            }
            State::Reviewing => {
                self.enter_reviewing_state().await?;
            }
            State::ReviewPassed => {
                // Clear stale freshness routing metadata so freshness_routing.rs defense-in-depth
                // is not confused if this task later reaches Merging via a different path.
                if let Some(task_repo) = &self.machine.context.services.task_repo {
                    let task_id_typed = TaskId::from_string(self.machine.context.task_id.clone());
                    if let Ok(Some(task)) = task_repo.get_by_id(&task_id_typed).await {
                        let mut meta: serde_json::Value = task
                            .metadata
                            .as_deref()
                            .and_then(|s| serde_json::from_str(s).ok())
                            .unwrap_or_else(|| serde_json::json!({}));
                        freshness::FreshnessMetadata::cleanup(
                            freshness::FreshnessCleanupScope::RoutingOnly,
                            &mut meta,
                        );
                        if let Err(e) = task_repo
                            .update_metadata(&task_id_typed, Some(meta.to_string()))
                            .await
                        {
                            tracing::warn!(
                                task_id = %self.machine.context.task_id,
                                error = %e,
                                "Failed to clear freshness routing metadata on ReviewPassed"
                            );
                        }
                    }
                }

                // Emit 'review:ai_approved' event
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("review:ai_approved", &self.machine.context.task_id)
                    .await;

                // Notify user that review passed and awaits approval
                self.machine
                    .context
                    .services
                    .notifier
                    .notify_with_message(
                        "review:ai_approved",
                        &self.machine.context.task_id,
                        "AI review passed. Please review and approve.",
                    )
                    .await;

                // Emit review:approved webhook event
                if let Some(ref publisher) = self.machine.context.services.webhook_publisher {
                    let payload = serde_json::json!({
                        "task_id": self.machine.context.task_id,
                        "project_id": self.machine.context.project_id,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    });
                    publisher.publish(
                        ralphx_domain::entities::EventType::ReviewApproved,
                        &self.machine.context.project_id,
                        payload,
                    ).await;
                }
            }
            State::Escalated => {
                // Emit 'review:escalated' event
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("review:escalated", &self.machine.context.task_id)
                    .await;

                // Notify user that AI escalated review
                self.machine
                    .context
                    .services
                    .notifier
                    .notify_with_message(
                        "review:escalated",
                        &self.machine.context.task_id,
                        "AI review escalated. Please review and decide.",
                    )
                    .await;

                // Also emit via webhook publisher
                if let Some(ref publisher) = self.machine.context.services.webhook_publisher {
                    let payload = serde_json::json!({
                        "task_id": self.machine.context.task_id,
                        "project_id": self.machine.context.project_id,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    });
                    publisher.publish(
                        ralphx_domain::entities::EventType::ReviewEscalated,
                        &self.machine.context.project_id,
                        payload,
                    ).await;
                }
            }
            State::ReExecuting => {
                self.enter_reexecuting_state().await?;
            }
            State::RevisionNeeded => {
                // Auto-transition to ReExecuting will be handled by check_auto_transition

                // Emit review:changes_requested webhook event
                if let Some(ref publisher) = self.machine.context.services.webhook_publisher {
                    let payload = serde_json::json!({
                        "task_id": self.machine.context.task_id,
                        "project_id": self.machine.context.project_id,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    });
                    publisher.publish(
                        ralphx_domain::entities::EventType::ReviewChangesRequested,
                        &self.machine.context.project_id,
                        payload,
                    ).await;
                }
            }
            State::Approved => {
                // Emit task completed event
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("task_completed", &self.machine.context.task_id)
                    .await;
                // NOTE: Do NOT unblock dependents here. Approved auto-transitions to
                // PendingMerge (Phase 66). Unblocking happens at on_enter(Merged) after
                // the task's work is actually on main.
            }
            State::Failed(data) => {
                let task_id = &self.machine.context.task_id;

                // Store failure reason in task metadata for frontend access
                if let Some(ref task_repo) = self.machine.context.services.task_repo {
                    let task_id_typed = TaskId::from_string(task_id.clone());

                    // Skip guard: check if metadata was already pre-computed (e.g., by transition_task_with_metadata)
                    match task_repo.get_by_id(&task_id_typed).await {
                        Ok(Some(task)) => {
                            // Read auto_retry_count_executing from task metadata for observability
                            let attempt_count =
                                task.metadata
                                    .as_deref()
                                    .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
                                    .and_then(|v| {
                                        v.get("auto_retry_count_executing").and_then(|c| c.as_u64())
                                    })
                                    .unwrap_or(0) as u32;

                            // Compute base metadata (without persisting yet — fallback check runs after)
                            let merged_metadata: String = if MetadataUpdate::key_exists_in(
                                "failure_error",
                                task.metadata.as_deref(),
                            ) {
                                tracing::debug!(
                                    task_id = task_id,
                                    attempt_count = attempt_count,
                                    "failure_error already present (pre-computed); writing attempt_count only"
                                );
                                // Write attempt_count even when other failure metadata was pre-computed
                                MetadataUpdate::new()
                                    .with_u32("attempt_count", attempt_count)
                                    .merge_into(task.metadata.as_deref())
                            } else {
                                // Fallback: metadata not pre-computed, write it now for backward compatibility
                                let enriched_data = data.clone().with_attempt_count(attempt_count);
                                build_failed_metadata(&enriched_data)
                                    .merge_into(task.metadata.as_deref())
                            };

                            // Fallback safety net: ensure execution_recovery exists.
                            // Terminal paths (E7, wall-clock, paths 8-9) pre-write execution_recovery
                            // with stop_retrying=true before transition, so is_none() check skips them.
                            // This catches path #5 (empty output) and any future unknown paths.
                            let mut metadata_obj: serde_json::Map<String, serde_json::Value> =
                                serde_json::from_str(&merged_metadata).unwrap_or_default();

                            if ExecutionRecoveryMetadata::from_task_metadata(Some(&merged_metadata))
                                .unwrap_or(None)
                                .is_none()
                            {
                                let mut recovery = ExecutionRecoveryMetadata::new();
                                recovery.append_event_with_state(
                                    ExecutionRecoveryEvent::new(
                                        ExecutionRecoveryEventKind::Failed,
                                        ExecutionRecoverySource::System,
                                        ExecutionRecoveryReasonCode::Unknown,
                                        "Failed without pre-written recovery metadata (fallback)",
                                    )
                                    .with_failure_source(ExecutionFailureSource::Unknown),
                                    ExecutionRecoveryState::Retrying,
                                );
                                // stop_retrying stays false (default) — conservative, gives task
                                // a recovery chance via reconciler's reconcile_failed_execution_task
                                if let Ok(recovery_value) = serde_json::to_value(&recovery) {
                                    metadata_obj
                                        .insert("execution_recovery".to_string(), recovery_value);
                                }
                            }

                            // Add failed_at if absent (merge-safe, used for staleness tracking)
                            if !metadata_obj.contains_key("failed_at") {
                                metadata_obj.insert(
                                    "failed_at".to_string(),
                                    serde_json::json!(Utc::now().to_rfc3339()),
                                );
                            }

                            let final_metadata = serde_json::to_string(
                                &serde_json::Value::Object(metadata_obj),
                            )
                            .unwrap_or(merged_metadata);

                            if let Err(e) = task_repo
                                .update_metadata(&task_id_typed, Some(final_metadata))
                                .await
                            {
                                tracing::error!(
                                    task_id = task_id,
                                    error = %e,
                                    "Failed to update task failure metadata"
                                );
                            }
                        }
                        Ok(None) => {
                            tracing::error!(
                                task_id = task_id,
                                "Task not found when storing failure metadata"
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                task_id = task_id,
                                error = %e,
                                "Error retrieving task for failure metadata"
                            );
                        }
                    }
                }

                // Fail any in-progress steps (Bug 2: agent was terminated, won't call fail_step)
                if let Some(ref step_repo) = self.machine.context.services.step_repo {
                    let task_id_typed = TaskId::from_string(task_id.clone());
                    match step_repo.get_by_task(&task_id_typed).await {
                        Ok(steps) => {
                            for step in steps
                                .iter()
                                .filter(|s| s.status == TaskStepStatus::InProgress)
                            {
                                let mut failed_step = step.clone();
                                failed_step.status = TaskStepStatus::Failed;
                                failed_step.completion_note =
                                    Some("Task execution failed".to_string());
                                failed_step.completed_at = Some(Utc::now());

                                if let Err(e) = step_repo.update(&failed_step).await {
                                    tracing::error!(
                                        task_id = task_id,
                                        step_id = %step.id,
                                        error = %e,
                                        "Failed to update in-progress step to failed status"
                                    );
                                } else {
                                    // Emit step updated event
                                    self.machine
                                        .context
                                        .services
                                        .event_emitter
                                        .emit("step:updated", &format!("{}", step.id))
                                        .await;
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                task_id = task_id,
                                error = %e,
                                "Failed to retrieve steps for failure handling"
                            );
                        }
                    }
                }

                // Emit task failed event
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("task_failed", task_id)
                    .await;
            }
            State::PendingMerge => {
                // Phase 1 of merge workflow: Attempt programmatic rebase and merge
                // This is the "fast path" - if successful, skip agent entirely
                // heap-allocate to prevent stack overflow from large inlined future
                Box::pin(self.attempt_programmatic_merge()).await;
            }
            State::Merging => {
                Box::pin(self.enter_merging_state()).await?;
            }
            State::Merged => {
                // For plan merge tasks: run post_merge_cleanup to update plan branch status,
                // delete feature branch, emit plan:merge_complete event, and cascade-stop siblings.
                // Idempotency guard in post_merge_cleanup prevents double-execution if already
                // called during push-to-main pipeline (plan_branch.status == Merged → early return).
                // This call is the PRIMARY path for PR-mode merges (poller triggers Merging→Merged).
                let task_id_str = &self.machine.context.task_id.clone();
                let task_id = TaskId::from_string(task_id_str.clone());
                let plan_branch_repo = &self.machine.context.services.plan_branch_repo;

                if let (Some(ref task_repo), Some(ref project_repo)) = (
                    &self.machine.context.services.task_repo,
                    &self.machine.context.services.project_repo,
                ) {
                    if let Ok(Some(task)) = task_repo.get_by_id(&task_id).await {
                        if task.category == TaskCategory::PlanMerge {
                            let project_id = ProjectId::from_string(self.machine.context.project_id.clone());
                            if let Ok(Some(project)) = project_repo.get_by_id(&project_id).await {
                                let repo_path = std::path::PathBuf::from(&project.working_directory);
                                self.post_merge_cleanup(
                                    task_id_str,
                                    &task_id,
                                    &repo_path,
                                    plan_branch_repo,
                                ).await;
                            }
                        }
                    }
                }

                // Auto-unblock tasks that were waiting on this task
                // This handles the HTTP handler path where transition_task triggers on_enter
                self.machine
                    .context
                    .services
                    .dependency_manager
                    .unblock_dependents(&self.machine.context.task_id)
                    .await;

                // Schedule newly-unblocked tasks (e.g. plan_merge tasks that just became Ready)
                // Internal transition — no UI settle needed → merge_settle_ms
                if let Some(ref scheduler) = self.machine.context.services.task_scheduler {
                    let scheduler = Arc::clone(scheduler);
                    let merge_settle_ms = scheduler_config().merge_settle_ms;
                    tokio::spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_millis(merge_settle_ms))
                            .await;
                        scheduler.try_schedule_ready_tasks().await;
                    });
                } else {
                    tracing::warn!(
                        task_id = self.machine.context.task_id.as_str(),
                        "task_scheduler not wired — Ready tasks will not be auto-scheduled after Merged"
                    );
                }

                // Retry deferred merges — covers the HTTP handler path (e.g. ConflictResolved)
                // where on_enter(Merged) is called directly without going through
                // post_merge_cleanup(). No sleep needed: scheduling_lock mutex in
                // task_scheduler_service.rs serializes concurrent calls via try_lock(), and
                // has_merge_deferred_metadata is the actual safety guard.
                if let Some(ref scheduler) = self.machine.context.services.task_scheduler {
                    let scheduler = Arc::clone(scheduler);
                    let project_id = self.machine.context.project_id.clone();
                    tokio::spawn(async move {
                        scheduler.try_retry_deferred_merges(&project_id).await;
                    });
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// Record a merger agent spawn failure as an `AttemptFailed` event in task metadata.
///
/// Each spawn failure consumes one slot of the reconciler's retry budget
/// (`merging_max_retries`). Once the budget is exhausted the reconciler
/// transitions the task to `MergeIncomplete` on the next cycle (≤30 s).
async fn record_merger_spawn_failure(
    task_repo: &Option<std::sync::Arc<dyn crate::domain::repositories::TaskRepository>>,
    task_id: &str,
    error: &str,
) {
    let Some(repo) = task_repo else { return };
    let tid = TaskId::from_string(task_id.to_string());
    let Ok(Some(mut task)) = repo.get_by_id(&tid).await else {
        return;
    };

    let mut recovery = MergeRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
        .unwrap_or(None)
        .unwrap_or_default();

    let spawn_failure_count = recovery
        .events
        .iter()
        .filter(|ev| {
            ev.kind == MergeRecoveryEventKind::AttemptFailed
                && ev.message.contains("failed to spawn")
        })
        .count() as u32
        + 1; // +1 for the event we're about to record

    let error_lower = error.to_lowercase();
    let spawn_failure_source = if error_lower.contains(ENOENT_MARKER)
        || error_lower.contains("no such file")
    {
        MergeFailureSource::SpawnFailure
    } else {
        MergeFailureSource::TransientGit
    };
    let event = MergeRecoveryEvent::new(
        MergeRecoveryEventKind::AttemptFailed,
        MergeRecoverySource::System,
        MergeRecoveryReasonCode::GitError,
        format!("Merger agent failed to spawn: {}", error),
    )
    .with_failure_source(spawn_failure_source);

    recovery.append_event_with_state(event, MergeRecoveryState::Failed);

    let max_retries = reconciliation_config().merging_max_retries as u32;
    if let Ok(updated_meta) = recovery.update_task_metadata(task.metadata.as_deref()) {
        task.metadata = Some(updated_meta);
        super::set_trigger_origin(&mut task, "recovery");
        task.touch();
        if let Err(e) = repo.update(&task).await {
            tracing::warn!(
                task_id = task_id,
                error = %e,
                "Failed to persist merger spawn failure metadata"
            );
        } else {
            tracing::warn!(
                task_id = task_id,
                spawn_failure_count = spawn_failure_count,
                max_retries = max_retries,
                "Recorded merger spawn failure ({}/{}); reconciler will transition to \
                 MergeIncomplete when retry budget is exhausted",
                spawn_failure_count,
                max_retries,
            );
        }
    }
}

/// Record a reviewer agent spawn failure in task metadata.
///
/// Uses flat JSON fields: reviewer_spawn_failure_count, last_reviewer_spawn_error,
/// reviewer_spawn_failed_at. The reconciler reads reviewer_spawn_failure_count
/// to detect when the retry budget is exhausted and escalate.
async fn record_reviewer_spawn_failure(
    task_repo: &Option<std::sync::Arc<dyn crate::domain::repositories::TaskRepository>>,
    task_id: &str,
    error: &str,
) {
    let Some(repo) = task_repo else { return };
    let tid = crate::domain::entities::TaskId::from_string(task_id.to_string());
    let Ok(Some(task)) = repo.get_by_id(&tid).await else {
        return;
    };

    let mut meta: serde_json::Value = task
        .metadata
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_else(|| serde_json::json!({}));

    let current_count = meta
        .get("reviewer_spawn_failure_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let new_count = current_count + 1;

    meta["reviewer_spawn_failure_count"] = serde_json::json!(new_count);
    meta["last_reviewer_spawn_error"] = serde_json::json!(error);
    meta["reviewer_spawn_failed_at"] =
        serde_json::json!(chrono::Utc::now().to_rfc3339());

    let tid2 = crate::domain::entities::TaskId::from_string(task_id.to_string());
    if let Err(e) = repo.update_metadata(&tid2, Some(meta.to_string())).await {
        tracing::warn!(
            task_id = task_id,
            error = %e,
            "Failed to persist reviewer spawn failure metadata"
        );
    } else {
        tracing::warn!(
            task_id = task_id,
            count = new_count,
            "Recorded reviewer spawn failure ({}); reconciler will escalate when retry budget is exhausted",
            new_count,
        );
    }
}

/// Extract `restart_note` from task metadata JSON.
/// Returns `Some(note)` if the key exists and is a non-empty string, `None` otherwise.
fn extract_restart_note(metadata: Option<&str>) -> Option<String> {
    let metadata_str = metadata?;
    let obj = serde_json::from_str::<serde_json::Value>(metadata_str).ok()?;
    let note = obj.get("restart_note")?.as_str()?;
    if note.is_empty() {
        None
    } else {
        Some(note.to_string())
    }
}
