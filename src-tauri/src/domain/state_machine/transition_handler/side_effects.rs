// State entry side effects
// This module contains the on_enter implementation that handles state-specific actions
//
// Sibling modules (extracted for maintainability, declared in transition_handler/mod.rs):
// - merge_helpers: path computation, metadata parsing, branch resolution
// - merge_completion: finalize merge and cleanup branch/worktree
// - merge_validation: post-merge validation gate (setup + validate phases)
// - merge_orchestrator: sub-functions extracted from attempt_programmatic_merge

use super::merge_helpers::{
    resolve_merge_branches, truncate_str, validate_plan_merge_preconditions,
};
use super::merge_orchestrator::ConcurrentGuardResult;
use super::merge_validation::{format_validation_error_metadata, ValidationLogEntry};

use super::merge_validation::{emit_merge_progress, ValidationFailure};

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::Emitter;

use super::super::machine::State;
use crate::application::GitService;
use crate::infrastructure::agents::claude::{reconciliation_config, scheduler_config};
use crate::domain::entities::{
    merge_progress_event::{MergePhase, MergePhaseStatus},
    task_metadata::MergeFailureSource,
    InternalStatus, MergeValidationMode, Project,
    Task, TaskId,
};
use crate::domain::repositories::{
    PlanBranchRepository, TaskRepository,
};
use crate::error::AppResult;

impl<'a> super::TransitionHandler<'a> {
    /// Execute on-enter action for a state
    ///
    /// This method is public to allow `TaskTransitionService` to trigger entry actions
    /// for direct status changes (e.g., Kanban drag-drop) without going through the
    /// full event-based transition flow.
    ///
    /// Returns an error if the state entry cannot be completed (e.g., execution blocked
    /// due to blocked execution).
    pub async fn on_enter(&self, state: &State) -> AppResult<()> {
        self.on_enter_dispatch(state).await
    }

    /// Attempt programmatic rebase and merge (Phase 1 of merge workflow).
    ///
    /// This is the "fast path" - try to rebase task branch onto base and merge.
    /// If successful, transition directly to Merged and cleanup branch/worktree.
    /// If conflicts occur, transition to Merging for agent-assisted resolution.
    pub(super) async fn attempt_programmatic_merge(&self) {
        let task_id_str = &self.machine.context.task_id;
        let project_id_str = &self.machine.context.project_id;

        // --- Self-dedup guard ---
        if !self.try_acquire_in_flight_guard(task_id_str) {
            return;
        }
        let _in_flight_guard = InFlightGuard {
            set: std::sync::Arc::clone(&self.machine.context.services.merges_in_flight),
            id: task_id_str.clone(),
        };

        // Load task and project from repos
        let Some(inputs) = self.fetch_merge_context(task_id_str, project_id_str).await else {
            return;
        };
        let mut task = inputs.task;
        let project = inputs.project;

        // Only proceed if task_repo is available (guaranteed by fetch_merge_context)
        let task_repo = self.machine.context.services.task_repo.as_ref().unwrap();

        let app_handle = self.machine.context.services.app_handle.as_ref();

        // Emit early phase list so the frontend can show pre-merge phases immediately
        // (validation start emits the full list including dynamic validation phases later)
        if let Some(handle) = app_handle {
            let _ = handle.emit(
                "task:merge_phases",
                serde_json::json!({
                    "task_id": task_id_str,
                    "phases": [
                        { "id": MergePhase::MERGE_PREPARATION, "label": "Preparation" },
                        { "id": MergePhase::PRECONDITION_CHECK, "label": "Preconditions" },
                        { "id": MergePhase::BRANCH_FRESHNESS, "label": "Branch Freshness" },
                        { "id": MergePhase::MERGE_CLEANUP, "label": "Cleanup" },
                        { "id": MergePhase::WORKTREE_SETUP, "label": "Worktree Setup" },
                        { "id": MergePhase::PROGRAMMATIC_MERGE, "label": "Merge" },
                        { "id": MergePhase::FINALIZE, "label": "Finalize" },
                    ],
                }),
            );
        }

        // Signal that merge preparation has started
        emit_merge_progress(
            app_handle,
            task_id_str,
            MergePhase::new(MergePhase::MERGE_PREPARATION),
            MergePhaseStatus::Started,
            "Preparing merge...".to_string(),
        );

        // Attempt to discover and re-attach orphaned task branch
        self.log_branch_discovery(&mut task, &project, task_repo, task_id_str)
            .await;

        // Preparation complete (branch discovery + context loaded)
        emit_merge_progress(
            app_handle,
            task_id_str,
            MergePhase::new(MergePhase::MERGE_PREPARATION),
            MergePhaseStatus::Passed,
            "Merge context loaded".to_string(),
        );

        // Pre-merge validation for plan_merge tasks
        emit_merge_progress(
            app_handle,
            task_id_str,
            MergePhase::new(MergePhase::PRECONDITION_CHECK),
            MergePhaseStatus::Started,
            "Validating merge preconditions...".to_string(),
        );
        let plan_branch_repo = &self.machine.context.services.plan_branch_repo;
        let task_id = TaskId::from_string(task_id_str.clone());
        if let Err(validation_err) =
            validate_plan_merge_preconditions(&task, &project, plan_branch_repo).await
        {
            let error_msg = validation_err.message();
            let error_code = validation_err.error_code();
            tracing::warn!(
                task_id = task_id_str,
                error_code = error_code,
                error = %error_msg,
                "Pre-merge validation failed for plan_merge task — transitioning to MergeIncomplete"
            );
            let metadata = serde_json::json!({
                "error": error_msg,
                "error_code": error_code,
                "category": task.category,
            });
            self.transition_to_merge_incomplete(
                &mut task, &task_id, task_id_str, metadata, task_repo, false,
            ).await;
            return;
        }

        // Resolve source and target branches
        let (source_branch, target_branch) =
            resolve_merge_branches(&task, &project, plan_branch_repo).await;

        // Ensure we have a source branch to merge
        if source_branch.is_empty() {
            tracing::error!(
                task_id = task_id_str,
                category = %task.category,
                task_branch = ?task.task_branch,
                "Programmatic merge failed: empty source branch resolved — \
                 transitioning to MergeIncomplete"
            );
            let metadata = serde_json::json!({
                "error": "Empty source branch resolved. This typically means plan_branch_repo \
                          was unavailable when resolving merge branches for a plan_merge task.",
                "source_branch": source_branch,
                "target_branch": target_branch,
                "category": task.category,
            });
            self.transition_to_merge_incomplete(
                &mut task, &task_id, task_id_str, metadata, task_repo, true,
            ).await;
            return;
        }

        // Main-merge deferral check
        let base_branch = project.base_branch.as_deref().unwrap_or("main");
        let running_count = self.machine.context.services.execution_state
            .as_ref()
            .map(|s| s.running_count());
        if super::merge_coordination::check_main_merge_deferral(
            &mut task, task_id_str, &source_branch, &target_branch, base_branch,
            task_repo, running_count,
            self.machine.context.services.app_handle.as_ref(),
        ).await {
            return;
        }

        // Preconditions validated, branches resolved
        emit_merge_progress(
            app_handle,
            task_id_str,
            MergePhase::new(MergePhase::PRECONDITION_CHECK),
            MergePhaseStatus::Passed,
            "Preconditions met".to_string(),
        );

        let repo_path = Path::new(&project.working_directory);

        // Branch freshness: ensure plan branch, update from main, update source from target
        emit_merge_progress(
            app_handle,
            task_id_str,
            MergePhase::new(MergePhase::BRANCH_FRESHNESS),
            MergePhaseStatus::Started,
            "Checking branch freshness...".to_string(),
        );

        // Ensure plan branch exists as git ref (lazy creation for merge target)
        super::merge_coordination::ensure_plan_branch_exists(
            &task, repo_path, &target_branch, plan_branch_repo,
        ).await;

        // Update plan branch from main if behind (prevents false validation failures)
        match super::merge_coordination::update_plan_from_main(
            repo_path,
            &target_branch,
            base_branch,
            &project,
            task_id_str,
            self.machine.context.services.app_handle.as_ref(),
        ).await {
            super::merge_coordination::PlanUpdateResult::AlreadyUpToDate
            | super::merge_coordination::PlanUpdateResult::Updated
            | super::merge_coordination::PlanUpdateResult::NotPlanBranch => {
                // Continue with merge
            }
            super::merge_coordination::PlanUpdateResult::Conflicts { conflict_files } => {
                tracing::warn!(
                    task_id = task_id_str,
                    conflict_count = conflict_files.len(),
                    "Plan branch update from main produced conflicts — routing to merger agent"
                );
                let metadata = serde_json::json!({
                    "error": "Conflicts detected while updating plan branch from main. Merger agent needed.",
                    "conflict_files": conflict_files.iter().map(|f| f.display().to_string()).collect::<Vec<_>>(),
                    "source_branch": source_branch,
                    "target_branch": target_branch,
                    "plan_update_conflict": true,
                });
                task.metadata = Some(metadata.to_string());
                task.internal_status = InternalStatus::Merging;
                self.persist_merge_transition(
                    &mut task, &task_id, task_id_str,
                    InternalStatus::PendingMerge, InternalStatus::Merging,
                    "plan_update_conflict", task_repo,
                ).await;
                return;
            }
            super::merge_coordination::PlanUpdateResult::Error(err) => {
                tracing::error!(
                    task_id = task_id_str,
                    error = %err,
                    "Plan branch update from main failed — aborting merge to prevent stale branch"
                );
                // Fatal: abort merge. Proceeding with a stale plan branch causes validation
                // failures that the fixer agent cannot resolve (missing code from main).
                let metadata = serde_json::json!({
                    "error": format!("Plan branch update failed: {}", err),
                    "source_branch": source_branch,
                    "target_branch": target_branch,
                    "merge_failure_source": "PlanUpdateFailed",
                });
                task.metadata = Some(metadata.to_string());
                task.internal_status = InternalStatus::MergeIncomplete;
                self.persist_merge_transition(
                    &mut task, &task_id, task_id_str,
                    InternalStatus::PendingMerge, InternalStatus::MergeIncomplete,
                    "plan_update_failed", task_repo,
                ).await;
                return;
            }
        }

        // Update source branch from target if behind (prevents validation failures from stale code)
        match super::merge_coordination::update_source_from_target(
            repo_path,
            &source_branch,
            &target_branch,
            &project,
            task_id_str,
            self.machine.context.services.app_handle.as_ref(),
        ).await {
            super::merge_coordination::SourceUpdateResult::AlreadyUpToDate
            | super::merge_coordination::SourceUpdateResult::Updated => {
                // Continue with merge
            }
            super::merge_coordination::SourceUpdateResult::Conflicts { conflict_files } => {
                tracing::warn!(
                    task_id = task_id_str,
                    conflict_count = conflict_files.len(),
                    "Source branch update from target produced conflicts — routing to merger agent"
                );
                let metadata = serde_json::json!({
                    "error": "Conflicts detected while updating source branch from target. Merger agent needed.",
                    "conflict_files": conflict_files.iter().map(|f| f.display().to_string()).collect::<Vec<_>>(),
                    "source_branch": source_branch,
                    "target_branch": target_branch,
                    "source_update_conflict": true,
                });
                task.metadata = Some(metadata.to_string());
                task.internal_status = InternalStatus::Merging;
                self.persist_merge_transition(
                    &mut task, &task_id, task_id_str,
                    InternalStatus::PendingMerge, InternalStatus::Merging,
                    "source_update_conflict", task_repo,
                ).await;
                return;
            }
            super::merge_coordination::SourceUpdateResult::Error(err) => {
                tracing::warn!(
                    task_id = task_id_str,
                    error = %err,
                    "Source branch update from target failed (non-fatal) — proceeding with merge"
                );
                // Non-fatal: continue with merge anyway. The source branch may still merge cleanly.
            }
        }

        // Branch freshness checks complete
        emit_merge_progress(
            app_handle,
            task_id_str,
            MergePhase::new(MergePhase::BRANCH_FRESHNESS),
            MergePhaseStatus::Passed,
            "Branches are up to date".to_string(),
        );

        // "Already merged" early exit
        if self.check_already_merged(
            &mut task, &task_id, task_id_str, &project, repo_path,
            &source_branch, &target_branch, task_repo, plan_branch_repo,
        ).await {
            return;
        }

        // "Deleted source branch" recovery
        if self.recover_deleted_source_branch(
            &mut task, &task_id, task_id_str, &project, repo_path,
            &source_branch, &target_branch, task_repo, plan_branch_repo,
        ).await {
            return;
        }

        // Emit merge progress event
        emit_merge_progress(
            app_handle,
            task_id_str,
            MergePhase::programmatic_merge(),
            MergePhaseStatus::Started,
            format!("Merging {} into {}", source_branch, target_branch),
        );

        tracing::info!(
            task_id = task_id_str,
            source_branch = %source_branch,
            target_branch = %target_branch,
            "Attempting programmatic merge (Phase 1)"
        );

        // Concurrent merge guard (TOCTOU-safe deferral under merge_lock)
        if matches!(
            self.run_concurrent_merge_guard(
                &mut task, task_id_str, &target_branch, &project, task_repo, plan_branch_repo,
            ).await,
            ConcurrentGuardResult::Deferred
        ) {
            return;
        }

        // Overall merge deadline
        let deadline_secs = reconciliation_config().attempt_merge_deadline_secs;
        let merge_deadline = tokio::time::Instant::now()
            + std::time::Duration::from_secs(deadline_secs);

        // Pre-merge cleanup
        emit_merge_progress(
            app_handle,
            task_id_str,
            MergePhase::new(MergePhase::MERGE_CLEANUP),
            MergePhaseStatus::Started,
            "Cleaning up previous merge artifacts...".to_string(),
        );
        self.pre_merge_cleanup(
            task_id_str, &task, &project, repo_path, &target_branch, task_repo,
        ).await;
        emit_merge_progress(
            app_handle,
            task_id_str,
            MergePhase::new(MergePhase::MERGE_CLEANUP),
            MergePhaseStatus::Passed,
            "Cleanup complete".to_string(),
        );

        // Check deadline after cleanup
        if tokio::time::Instant::now() >= merge_deadline {
            tracing::error!(
                task_id = task_id_str,
                deadline_secs = deadline_secs,
                "Programmatic merge exceeded deadline during cleanup — transitioning to MergeIncomplete"
            );
            let metadata = serde_json::json!({
                "error": format!("Merge attempt timed out after {}s (cleanup phase exceeded deadline)", deadline_secs),
                "source_branch": source_branch,
                "target_branch": target_branch,
            });
            self.transition_to_merge_incomplete(
                &mut task, &task_id, task_id_str, metadata, task_repo, true,
            ).await;
            return;
        }

        // Build squash commit message
        let squash_commit_msg = self
            .build_squash_commit_message(&task, task_id_str, &source_branch)
            .await;

        // Dispatch merge strategy with timeout
        let remaining = merge_deadline.saturating_duration_since(tokio::time::Instant::now());
        self.dispatch_merge_strategy(
            &mut task, &task_id, task_id_str, &project, repo_path,
            &source_branch, &target_branch, &squash_commit_msg,
            task_repo, plan_branch_repo, remaining, deadline_secs,
        ).await;
    }

    /// Try to insert this task into the in-flight set. Returns false if already in flight.
    fn try_acquire_in_flight_guard(&self, task_id_str: &str) -> bool {
        let mut in_flight = self
            .machine
            .context
            .services
            .merges_in_flight
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        if !in_flight.insert(task_id_str.to_string()) {
            tracing::info!(
                task_id = task_id_str,
                "Merge attempt skipped — already in flight for this task (self-dedup guard)"
            );
            return false;
        }
        true
    }

    /// Log the result of branch discovery (orphaned task branch re-attach).
    async fn log_branch_discovery(
        &self,
        task: &mut Task,
        project: &crate::domain::entities::Project,
        task_repo: &Arc<dyn TaskRepository>,
        task_id_str: &str,
    ) {
        match super::merge_helpers::discover_and_attach_task_branch(task, project, task_repo)
            .await
        {
            Ok(true) => {
                tracing::info!(
                    task_id = task_id_str,
                    branch = ?task.task_branch,
                    "Successfully recovered orphaned task branch"
                );
            }
            Ok(false) => {
                tracing::debug!(
                    task_id = task_id_str,
                    "No orphaned branch to recover (branch already set or doesn't exist)"
                );
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    task_id = task_id_str,
                    "Failed to discover orphaned task branch — continuing with existing flow"
                );
            }
        }
    }

    /// Persist a merge status transition: touch -> update -> persist_status_change -> emit.
    ///
    /// Callers must set `task.metadata` and `task.internal_status` before calling.
    /// Returns `false` if the update failed (caller should return early).
    pub(super) async fn persist_merge_transition(
        &self,
        task: &mut Task,
        task_id: &TaskId,
        task_id_str: &str,
        from_status: InternalStatus,
        to_status: InternalStatus,
        persist_label: &str,
        task_repo: &Arc<dyn TaskRepository>,
    ) -> bool {
        task.touch();

        if let Err(e) = task_repo.update(task).await {
            tracing::error!(error = %e, "Failed to update task to {:?} status", to_status);
            return false;
        }

        let from_str = serde_json::to_value(from_status)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| format!("{:?}", from_status));
        let to_str = serde_json::to_value(to_status)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| format!("{:?}", to_status));

        if let Err(e) = task_repo
            .persist_status_change(task_id, from_status, to_status, persist_label)
            .await
        {
            tracing::warn!(error = %e, "Failed to record {} transition (non-fatal)", persist_label);
        }

        self.machine
            .context
            .services
            .event_emitter
            .emit_status_change(task_id_str, &from_str, &to_str)
            .await;

        true
    }

    /// Transition a task to MergeIncomplete with the given metadata JSON.
    ///
    /// Handles the full transition: update metadata -> persist status change -> emit event.
    /// Optionally triggers on_exit (needed when the caller wants deferred-merge retry).
    pub(super) async fn transition_to_merge_incomplete(
        &self,
        task: &mut Task,
        task_id: &TaskId,
        task_id_str: &str,
        metadata: serde_json::Value,
        task_repo: &Arc<dyn TaskRepository>,
        trigger_on_exit: bool,
    ) {
        task.metadata = Some(metadata.to_string());
        task.internal_status = InternalStatus::MergeIncomplete;

        if !self.persist_merge_transition(
            task, task_id, task_id_str,
            InternalStatus::PendingMerge, InternalStatus::MergeIncomplete,
            "merge_incomplete", task_repo,
        ).await {
            return;
        }

        if trigger_on_exit {
            self.on_exit(&State::PendingMerge, &State::MergeIncomplete)
                .await;
        }
    }

    /// Post-merge cleanup: update plan branch status, delete feature branch, unblock dependents.
    ///
    /// Shared between all merge strategy success paths in `attempt_programmatic_merge()`.
    pub(super) async fn post_merge_cleanup(
        &self,
        task_id_str: &str,
        task_id: &TaskId,
        repo_path: &Path,
        plan_branch_repo: &Option<Arc<dyn PlanBranchRepository>>,
    ) {
        let app_handle = self.machine.context.services.app_handle.as_ref();

        if let Some(ref plan_branch_repo) = plan_branch_repo {
            if let Ok(Some(pb)) = plan_branch_repo.get_by_merge_task_id(task_id).await {
                if let Err(e) = plan_branch_repo.set_merged(&pb.id).await {
                    tracing::warn!(
                        error = %e,
                        task_id = task_id_str,
                        plan_branch_id = pb.id.as_str(),
                        "Failed to mark plan branch as merged (non-fatal)"
                    );
                }

                if let Err(e) = GitService::delete_feature_branch(repo_path, &pb.branch_name).await {
                    tracing::warn!(
                        error = %e,
                        task_id = task_id_str,
                        branch = %pb.branch_name,
                        "Failed to delete feature branch after merge (non-fatal)"
                    );
                } else {
                    tracing::info!(
                        task_id = task_id_str,
                        branch = %pb.branch_name,
                        "Deleted feature branch after plan merge"
                    );
                }

                if let Some(handle) = app_handle {
                    let _ = handle.emit(
                        "plan:merge_complete",
                        serde_json::json!({
                            "plan_artifact_id": pb.plan_artifact_id.as_str(),
                            "plan_branch_id": pb.id.as_str(),
                            "merge_task_id": task_id_str,
                            "branch_name": pb.branch_name,
                        }),
                    );
                }
            }
        }

        self.machine
            .context
            .services
            .dependency_manager
            .unblock_dependents(task_id_str)
            .await;

        if let Some(ref scheduler) = self.machine.context.services.task_scheduler {
            let scheduler = Arc::clone(scheduler);
            let merge_settle_ms = scheduler_config().merge_settle_ms;
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(merge_settle_ms)).await;
                scheduler.try_schedule_ready_tasks().await;
            });
        }
    }

    /// Handle post-merge validation failure: revert the merge commit, then transition
    /// to MergeIncomplete with error metadata.
    ///
    /// `repo_path` and `project` are needed in AutoFix mode to create a dedicated
    /// merge worktree when the merge was checkout-free (merge_path == repo_path).
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn handle_validation_failure(
        &self,
        task: &mut Task,
        task_id: &TaskId,
        task_id_str: &str,
        task_repo: &Arc<dyn TaskRepository>,
        failures: &[ValidationFailure],
        log: &[ValidationLogEntry],
        source_branch: &str,
        target_branch: &str,
        merge_path: &Path,
        mode_label: &str,
        validation_mode: &MergeValidationMode,
        repo_path: &Path,
        project: &Project,
    ) {
        if *validation_mode == MergeValidationMode::AutoFix {
            // AutoFix: DON'T revert — keep the merged (failing) code for the agent to fix
            tracing::info!(
                task_id = task_id_str,
                failure_count = failures.len(),
                "Validation failed (AutoFix mode, {}), spawning merger agent to attempt fix",
                mode_label,
            );

            let failure_details: Vec<serde_json::Value> = failures
                .iter()
                .map(|f| {
                    serde_json::json!({
                        "command": f.command,
                        "path": f.path,
                        "exit_code": f.exit_code,
                        "stderr": truncate_str(&f.stderr, 2000),
                    })
                })
                .collect();

            // If merge was checkout-free (merge_path == repo_path), create a dedicated
            // worktree so the fixer agent doesn't run in the user's main checkout.
            let fixer_worktree_path: PathBuf = if merge_path == repo_path {
                let wt_path = PathBuf::from(
                    super::merge_helpers::compute_merge_worktree_path(project, task_id_str),
                );

                // Pre-delete stale worktree if it exists from a previous attempt
                if wt_path.exists() {
                    if let Err(e) = GitService::delete_worktree(repo_path, &wt_path).await {
                        tracing::warn!(
                            task_id = task_id_str,
                            error = %e,
                            "Failed to delete stale fixer worktree (non-fatal)"
                        );
                    }
                }

                // Create worktree on the target branch (which has the merged+failing code)
                match GitService::checkout_existing_branch_worktree(repo_path, &wt_path, target_branch).await {
                    Ok(()) => {
                        tracing::info!(
                            task_id = task_id_str,
                            worktree = %wt_path.display(),
                            "Created dedicated fixer worktree for validation recovery"
                        );
                        wt_path
                    }
                    Err(e) => {
                        tracing::error!(
                            task_id = task_id_str,
                            error = %e,
                            "Failed to create fixer worktree — reverting merge and transitioning to MergeIncomplete"
                        );
                        // Revert the merge since we can't safely spawn a fixer
                        if let Err(revert_err) = GitService::reset_hard(repo_path, "HEAD~1").await {
                            tracing::error!(
                                task_id = task_id_str,
                                error = %revert_err,
                                "Failed to revert merge after worktree creation failure"
                            );
                        }
                        let metadata = serde_json::json!({
                            "error": format!("Failed to create fixer worktree: {}", e),
                            "source_branch": source_branch,
                            "target_branch": target_branch,
                        });
                        self.transition_to_merge_incomplete(
                            task, task_id, task_id_str, metadata, task_repo, true,
                        ).await;
                        return;
                    }
                }
            } else {
                // Worktree-based merge — use the existing merge worktree
                merge_path.to_path_buf()
            };

            // Merge new validation recovery metadata into existing metadata
            // to preserve merge_recovery history and validation_revert_count
            let mut metadata_obj = task
                .metadata
                .as_deref()
                .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
                .unwrap_or_else(|| serde_json::json!({}));
            if let Some(obj) = metadata_obj.as_object_mut() {
                obj.insert("validation_recovery".to_string(), serde_json::json!(true));
                obj.insert("validation_failures".to_string(), serde_json::json!(failure_details));
                obj.insert("validation_log".to_string(), serde_json::json!(log));
                obj.insert("source_branch".to_string(), serde_json::json!(source_branch));
                obj.insert("target_branch".to_string(), serde_json::json!(target_branch));
            }
            task.metadata = Some(metadata_obj.to_string());
            task.worktree_path = Some(fixer_worktree_path.to_string_lossy().to_string());
            task.internal_status = InternalStatus::Merging;

            self.persist_merge_transition(
                task, task_id, task_id_str,
                InternalStatus::PendingMerge, InternalStatus::Merging,
                "validation_auto_fix", task_repo,
            ).await;

            // Delegate to on_enter(Merging) which handles symlink cleanup, stale
            // rebase/merge abort, and spawns the merger agent with the correct prompt
            // (reads validation_recovery from metadata). This avoids a dual spawn path
            // where handle_validation_failure spawns one agent and reconciler spawns another.
            tracing::info!(
                task_id = task_id_str,
                "Delegating to on_enter(Merging) for validation recovery agent spawn"
            );
            if let Err(e) = Box::pin(self.on_enter_dispatch(&State::Merging)).await {
                tracing::error!(
                    task_id = task_id_str,
                    error = %e,
                    "on_enter(Merging) failed during validation recovery"
                );
            }
        } else {
            // Block mode: revert merge and transition to MergeIncomplete
            tracing::warn!(
                task_id = task_id_str,
                failure_count = failures.len(),
                "Post-merge validation failed ({}), reverting merge and transitioning to MergeIncomplete",
                mode_label,
            );

            // Capture the merge commit SHA before attempting revert (needed for
            // unrevertable flag if reset_hard fails).
            let merge_head_sha = GitService::get_branch_sha(merge_path, "HEAD")
                .await
                .ok();

            let revert_failed = match GitService::reset_hard(merge_path, "HEAD~1").await {
                Ok(_) => {
                    tracing::info!(
                        task_id = task_id_str,
                        "Successfully reverted merge commit after validation failure"
                    );
                    false
                }
                Err(e) => {
                    tracing::error!(
                        task_id = task_id_str,
                        error = %e,
                        merge_sha = ?merge_head_sha,
                        "Failed to revert merge commit after validation failure — target branch has failing code"
                    );
                    true
                }
            };

            // Merge error metadata INTO existing metadata to preserve all existing keys
            // (merge_retry_in_progress, merge_recovery, etc.) instead of replacing.
            let mut merged = task
                .metadata
                .as_deref()
                .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
                .unwrap_or_else(|| serde_json::json!({}));
            let prev_revert_count: u32 = merged
                .get("validation_revert_count")
                .and_then(|c| c.as_u64())
                .unwrap_or(0) as u32;
            let revert_count = prev_revert_count + 1;

            let error_metadata_str = format_validation_error_metadata(failures, log, source_branch, target_branch);
            if let Ok(error_obj) = serde_json::from_str::<serde_json::Value>(&error_metadata_str) {
                if let (Some(target), Some(source)) = (merged.as_object_mut(), error_obj.as_object()) {
                    for (k, v) in source {
                        target.insert(k.clone(), v.clone());
                    }
                }
            }
            if let Some(obj) = merged.as_object_mut() {
                obj.insert(
                    "merge_failure_source".to_string(),
                    serde_json::to_value(MergeFailureSource::ValidationFailed)
                        .unwrap_or(serde_json::json!("validation_failed")),
                );
                obj.insert("validation_revert_count".to_string(), serde_json::json!(revert_count));

                // Flag unrevertable merge commits so check_already_merged() doesn't
                // fast-path to completion with a failing merge commit on the target.
                if revert_failed {
                    obj.insert("merge_commit_unrevertable".to_string(), serde_json::json!(true));
                    if let Some(ref sha) = merge_head_sha {
                        obj.insert("unrevertable_commit_sha".to_string(), serde_json::json!(sha));
                    }
                }
            }

            task.metadata = Some(merged.to_string());
            task.internal_status = InternalStatus::MergeIncomplete;

            self.persist_merge_transition(
                task, task_id, task_id_str,
                InternalStatus::PendingMerge, InternalStatus::MergeIncomplete,
                "validation_failed", task_repo,
            ).await;
        }
    }
}

/// RAII guard that removes a task ID from the `merges_in_flight` set on drop.
struct InFlightGuard {
    set: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
    id: String,
}

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.set.lock() {
            guard.remove(&self.id);
        }
    }
}
