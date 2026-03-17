// State entry side effects
// This module contains the on_enter implementation that handles state-specific actions
//
// Sibling modules (extracted for maintainability, declared in transition_handler/mod.rs):
// - merge_helpers: path computation, metadata parsing, branch resolution
// - merge_completion: finalize merge and cleanup branch/worktree
// - merge_validation: post-merge validation gate (setup + validate phases)
// - merge_orchestrator: sub-functions extracted from attempt_programmatic_merge

use super::merge_helpers::{resolve_merge_branches, validate_plan_merge_preconditions};
use crate::utils::truncate_str;
use super::merge_orchestrator::ConcurrentGuardResult;
use super::merge_validation::{format_validation_error_metadata, ValidationLogEntry};

use super::merge_validation::{emit_merge_progress, ValidationFailure};
use crate::domain::entities::{ActivityEvent, ActivityEventRole, ActivityEventType};

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::Emitter;

use super::super::machine::State;
use crate::application::GitService;
use crate::infrastructure::agents::claude::{reconciliation_config, scheduler_config};
use crate::domain::entities::{
    merge_progress_event::{MergePhase, MergePhaseStatus},
    task_metadata::{CleanupPhase, MergeFailureSource},
    InternalStatus, MergeValidationMode, PlanBranch, Project,
    Task, TaskId,
};
use crate::domain::repositories::{
    PlanBranchRepository, TaskRepository,
};
use crate::domain::services::github_service::GithubServiceTrait;
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
        let attempt_start = std::time::Instant::now();

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

        // === PR MODE FORK (AD17) ===
        // Check if this is a plan merge task in PR mode. If so, use the PR path.
        let plan_branch_repo = &self.machine.context.services.plan_branch_repo;
        let github_service = self.machine.context.services.github_service.clone();
        let task_id_for_fork = TaskId::from_string(task_id_str.to_string());

        if let Some(ref pbr) = plan_branch_repo {
            if let Ok(Some(plan_branch)) = pbr.get_by_merge_task_id(&task_id_for_fork).await {
                let pr_mode = plan_branch.pr_eligible && github_service.is_some();
                if pr_mode {
                    let github = github_service.expect("pr_mode implies github_service is Some");
                    let pbr_arc = Arc::clone(pbr);
                    Box::pin(self.run_pr_mode_pending_merge(
                        &mut task, &project, plan_branch, task_id_for_fork, task_id_str, task_repo, &github, &pbr_arc,
                    )).await;
                    return;
                }
            }
        }

        // Push-to-main path (existing, unchanged)
        // Set merge_pipeline_active flag so the reconciler skips this task
        // while the pipeline is running (cleanup+freshness can exceed stale threshold).
        // Auto-expires after attempt_merge_deadline_secs as crash safety net.
        set_merge_pipeline_active(&mut task, task_id_str, task_repo).await;

        // Run the full pipeline body — ALL early returns stay inside the body,
        // so the clear below always executes.
        // heap-allocate to prevent stack overflow from large inlined future
        Box::pin(self.run_merge_pipeline_body(
            &mut task, &project, task_id_str, task_repo, attempt_start,
        )).await;

        // Always clear the flag — reloads from DB to avoid clobbering concurrent changes.
        clear_merge_pipeline_active_from_db(task_id_str, task_repo).await;
    }

    /// PR-mode PendingMerge path (AD17).
    ///
    /// Called when pr_eligible=true AND github_service is available.
    /// Pushes the plan branch and marks the PR ready, then transitions to Merging.
    /// The Merging entry point (on_enter_states.rs) will start the poller.
    #[allow(clippy::too_many_arguments)]
    async fn run_pr_mode_pending_merge(
        &self,
        task: &mut Task,
        project: &Project,
        plan_branch: PlanBranch,
        task_id: TaskId,
        task_id_str: &str,
        task_repo: &Arc<dyn TaskRepository>,
        github_service: &Arc<dyn GithubServiceTrait>,
        plan_branch_repo: &Arc<dyn PlanBranchRepository>,
    ) {
        tracing::info!(task_id = task_id_str, pr_number = ?plan_branch.pr_number, "PR-mode PendingMerge: starting PR path");

        // 1. Clear stale local-mode metadata (defense-in-depth for prior failed local attempts)
        {
            let mut changed = false;
            if let Some(ref meta_str) = task.metadata.clone() {
                if let Ok(mut meta_json) = serde_json::from_str::<serde_json::Value>(meta_str) {
                    if let Some(obj) = meta_json.as_object_mut() {
                        changed = obj.remove("merge_pipeline_active").is_some()
                            | obj.remove("validation_in_progress").is_some()
                            | obj.remove("merge_retry_in_progress").is_some();
                    }
                    if changed {
                        task.metadata = Some(meta_json.to_string());
                        task.touch();
                        let _ = task_repo.update(task).await;
                    }
                }
            }
        }

        // 2. Run concurrent merge guard with PR exclusion (AD14)
        // PR-polling tasks are excluded from the blocking check (they wait for GitHub, not local pipeline)
        let pb_repo_opt: Option<Arc<dyn PlanBranchRepository>> = Some(Arc::clone(plan_branch_repo));
        let target_branch = plan_branch.source_branch.clone(); // source_branch = the base branch (e.g. "main")
        if matches!(
            self.run_concurrent_merge_guard(task, task_id_str, &target_branch, project, task_repo, &pb_repo_opt).await,
            ConcurrentGuardResult::Deferred
        ) {
            return;
        }

        // 3. Re-entry guard: if already polling and poller is alive, return (no-op)
        if plan_branch.pr_polling_active {
            if let Some(ref registry) = self.machine.context.services.pr_poller_registry {
                if registry.is_polling(&task_id) {
                    tracing::info!(
                        task_id = task_id_str,
                        "PR-mode PendingMerge: re-entry guard — already polling, skipping"
                    );
                    return;
                }
            }
        }

        // 4. Set pr_polling_active = true in DB
        // update_last_polled_at also sets pr_polling_active = 1 in the same SQL statement
        if let Err(e) = plan_branch_repo.update_last_polled_at(&plan_branch.id, chrono::Utc::now()).await {
            tracing::warn!(task_id = task_id_str, error = %e, "PR-mode: failed to set pr_polling_active (non-fatal)");
        }

        // 5. Get working directory
        let working_dir = std::path::PathBuf::from(&project.working_directory);
        let branch_name = plan_branch.branch_name.clone();

        // 6. Perform PR operation: push branch and mark PR ready (or create PR if missing)
        let pr_op_result: Result<i64, crate::error::AppError> = if let Some(existing_pr_number) = plan_branch.pr_number {
            // Has PR: push latest commits then mark ready
            tracing::info!(task_id = task_id_str, pr_number = existing_pr_number, "PR-mode: pushing branch and marking PR ready");
            if let Err(e) = github_service.push_branch(&working_dir, &branch_name).await {
                tracing::warn!(task_id = task_id_str, error = %e, "PR-mode: push failed (proceeding to mark_pr_ready anyway)");
            }
            github_service.mark_pr_ready(&working_dir, existing_pr_number).await
                .map(|_| existing_pr_number)
        } else {
            // No PR yet: wait for CAS guard to clear (AD15), re-read, then create if still missing
            tracing::info!(task_id = task_id_str, "PR-mode: no pr_number, waiting for CAS guard (AD15)");
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while std::time::Instant::now() < deadline {
                if let Some(ref registry) = self.machine.context.services.pr_poller_registry {
                    if !registry.pr_creation_guard.contains_key(&plan_branch.id) {
                        break; // guard cleared — draft PR may have been created concurrently
                    }
                } else {
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }

            // Re-read pr_number from DB
            let refreshed_pr_number = plan_branch_repo
                .get_by_merge_task_id(&task_id)
                .await
                .ok()
                .flatten()
                .and_then(|pb| pb.pr_number);

            if let Some(new_pr_number) = refreshed_pr_number {
                // Concurrent creation succeeded: push + mark ready
                tracing::info!(task_id = task_id_str, pr_number = new_pr_number, "PR-mode: found PR from concurrent creation");
                let _ = github_service.push_branch(&working_dir, &branch_name).await;
                github_service.mark_pr_ready(&working_dir, new_pr_number).await
                    .map(|_| new_pr_number)
            } else {
                // Still no PR: push + create non-draft PR directly
                tracing::info!(task_id = task_id_str, "PR-mode: creating non-draft PR directly");
                match github_service.push_branch(&working_dir, &branch_name).await {
                    Err(e) => Err(e),
                    Ok(()) => {
                        let title = format!("Merge {}", branch_name);
                        let body = format!("Plan merge PR for `{}`.\n\nGenerated by RalphX.", branch_name);
                        let body_path = std::env::temp_dir().join(format!("ralphx_pr_body_{}.md", task_id_str));
                        match std::fs::write(&body_path, &body) {
                            Err(e) => Err(crate::error::AppError::Infrastructure(format!("write body file: {}", e))),
                            Ok(()) => {
                                let result = match github_service.create_draft_pr(
                                    &working_dir, &target_branch, &branch_name, &title, &body_path,
                                ).await {
                                    Err(e) => Err(e),
                                    Ok((new_pr_number, pr_url)) => {
                                        // Store PR info
                                        let _ = plan_branch_repo.update_pr_info(
                                            &plan_branch.id,
                                            new_pr_number,
                                            pr_url,
                                            crate::domain::entities::plan_branch::PrStatus::Open,
                                            false,
                                        ).await;
                                        // Mark ready immediately
                                        github_service.mark_pr_ready(&working_dir, new_pr_number).await
                                            .map(|_| new_pr_number)
                                    }
                                };
                                let _ = std::fs::remove_file(&body_path);
                                result
                            }
                        }
                    }
                }
            }
        };

        // 7. Handle result
        match pr_op_result {
            Ok(_pr_number) => {
                // Success: transition PendingMerge → Merging
                // on_enter(Merging) will check pr_eligible+pr_number and start the poller
                tracing::info!(task_id = task_id_str, "PR-mode: success, transitioning to Merging");
                task.internal_status = InternalStatus::Merging;
                if self.persist_merge_transition(
                    super::TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
                    InternalStatus::PendingMerge, InternalStatus::Merging,
                    "pr_mode_mark_ready",
                ).await {
                    // Trigger on_enter(Merging) to start the PR poller
                    if let Err(e) = Box::pin(self.on_enter_dispatch(&super::super::machine::State::Merging)).await {
                        tracing::error!(task_id = task_id_str, error = %e, "on_enter(Merging) failed in PR mode");
                    }
                }
            }
            Err(e) => {
                // Failure: clear pr_polling_active, transition to MergeIncomplete
                tracing::warn!(task_id = task_id_str, error = %e, "PR-mode: operation failed, transitioning to MergeIncomplete");
                let _ = plan_branch_repo.clear_polling_active_by_task(&task_id).await;
                let metadata = serde_json::json!({
                    "error": format!("PR operation failed: {}", e),
                    "error_code": "pr_operation_failed",
                });
                self.transition_to_merge_incomplete(
                    super::TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
                    metadata, true,
                ).await;
            }
        }
    }

    /// Inner body of `attempt_programmatic_merge`. Extracted so the outer wrapper
    /// can guarantee the `merge_pipeline_active` flag is always cleared on exit.
    async fn run_merge_pipeline_body(
        &self,
        task: &mut Task,
        project: &Project,
        task_id_str: &str,
        task_repo: &Arc<dyn TaskRepository>,
        attempt_start: std::time::Instant,
    ) {
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
        self.log_branch_discovery(task, project, task_repo, task_id_str)
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
        let task_id = TaskId::from_string(task_id_str.to_string());
        if let Err(validation_err) =
            validate_plan_merge_preconditions(task, project, plan_branch_repo).await
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
                super::TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
                metadata, false,
            ).await;
            return;
        }

        // Resolve source and target branches
        let (source_branch, target_branch) =
            resolve_merge_branches(task, project, plan_branch_repo).await;

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
                super::TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
                metadata, true,
            ).await;
            return;
        }

        // Cache resolved branches in task metadata so auto-complete uses the same target
        // branch (TOCTOU guard: plan state can change between merge start and auto-complete)
        {
            let mut meta: serde_json::Value = task.metadata
                .as_ref()
                .and_then(|m| serde_json::from_str(m).ok())
                .unwrap_or_else(|| serde_json::json!({}));
            meta["merge_source_branch"] = serde_json::json!(source_branch);
            meta["merge_target_branch"] = serde_json::json!(target_branch);
            task.metadata = Some(meta.to_string());
            if let Err(e) = task_repo.update(task).await {
                tracing::warn!(
                    task_id = task_id_str,
                    error = %e,
                    "Failed to cache merge branches in task metadata"
                );
            }
        }

        // Main-merge deferral check
        let base_branch = project.base_branch.as_deref().unwrap_or("main");
        let running_count = self.machine.context.services.execution_state
            .as_ref()
            .map(|s| s.running_count());
        if super::merge_coordination::check_main_merge_deferral(
            super::TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
            super::BranchPair { source_branch: &source_branch, target_branch: &target_branch },
            base_branch, running_count,
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
        self.emit_merge_activity_event(
            task_id_str,
            "Merge pipeline: preconditions validated",
            MergePhase::PRECONDITION_CHECK,
            "passed",
        ).await;

        let repo_path = Path::new(&project.working_directory);

        // Pre-merge cleanup: runs before freshness checks so all worktrees are
        // cleaned before freshness checks try to create new ones.
        emit_merge_progress(
            app_handle,
            task_id_str,
            MergePhase::new(MergePhase::MERGE_CLEANUP),
            MergePhaseStatus::Started,
            "Cleaning up previous merge artifacts...".to_string(),
        );
        let cleanup_timeout_secs = reconciliation_config().pre_merge_cleanup_timeout_secs;
        match super::cleanup_helpers::os_thread_timeout(
            std::time::Duration::from_secs(cleanup_timeout_secs),
            self.pre_merge_cleanup(
                task_id_str, task, project, repo_path, &target_branch, task_repo,
            ),
        ).await {
            Ok(()) => {
                emit_merge_progress(
                    app_handle,
                    task_id_str,
                    MergePhase::new(MergePhase::MERGE_CLEANUP),
                    MergePhaseStatus::Passed,
                    "Cleanup complete".to_string(),
                );
                self.emit_merge_activity_event(
                    task_id_str,
                    "Merge pipeline: cleanup complete",
                    MergePhase::MERGE_CLEANUP,
                    "passed",
                ).await;
            }
            Err(_os_elapsed) => {
                tracing::warn!(
                    task_id = %task_id_str,
                    cleanup_timeout_secs,
                    "pre_merge_cleanup timed out (OS-thread timeout) — proceeding to merge anyway (cleanup is best-effort)"
                );
                // Set debris metadata so GUARD knows this is a retry on next attempt
                // (prevents is_first_clean_attempt from skipping cleanup when stale worktree remains)
                super::merge_helpers::merge_metadata_into(task, &serde_json::json!({
                    "merge_failure_source": serde_json::to_value(MergeFailureSource::CleanupTimeout).unwrap_or_default(),
                    "cleanup_phase": serde_json::to_value(CleanupPhase::PreMergeWorktreeScan).unwrap_or_default(),
                }));
                if let Err(e) = task_repo.update(task).await {
                    tracing::warn!(
                        task_id = %task_id_str,
                        error = %e,
                        "Failed to persist cleanup_timeout debris metadata"
                    );
                }
                emit_merge_progress(
                    app_handle,
                    task_id_str,
                    MergePhase::new(MergePhase::MERGE_CLEANUP),
                    MergePhaseStatus::Passed,
                    format!("Cleanup timed out after {cleanup_timeout_secs}s — proceeding"),
                );
                self.emit_merge_activity_event(
                    task_id_str,
                    "Merge pipeline: cleanup timed out (best-effort, proceeding)",
                    MergePhase::MERGE_CLEANUP,
                    "warning",
                ).await;
            }
        }

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
            task, repo_path, &target_branch, plan_branch_repo,
        ).await;

        // Update plan branch from main if behind (prevents false validation failures)
        emit_merge_progress(
            app_handle,
            task_id_str,
            MergePhase::new(MergePhase::BRANCH_FRESHNESS),
            MergePhaseStatus::Started,
            "Updating plan branch from main...".to_string(),
        );
        let freshness_timeout = std::time::Duration::from_secs(
            reconciliation_config().branch_freshness_timeout_secs,
        );
        let plan_update_start = std::time::Instant::now();
        let plan_update_result = tokio::time::timeout(
            freshness_timeout,
            super::merge_coordination::update_plan_from_main(
                repo_path,
                &target_branch,
                base_branch,
                project,
                task_id_str,
                self.machine.context.services.app_handle.as_ref(),
            ),
        ).await;

        let plan_update_elapsed = plan_update_start.elapsed();
        match plan_update_result {
            Err(_elapsed) => {
                tracing::error!(
                    task_id = task_id_str,
                    timeout_secs = reconciliation_config().branch_freshness_timeout_secs,
                    elapsed_ms = plan_update_elapsed.as_millis() as u64,
                    "update_plan_from_main timed out — aborting merge"
                );
                let metadata = serde_json::json!({
                    "error": format!(
                        "update_plan_from_main timed out after {}s (limit: {}s)",
                        plan_update_elapsed.as_secs(),
                        reconciliation_config().branch_freshness_timeout_secs,
                    ),
                    "source_branch": source_branch,
                    "target_branch": target_branch,
                    "merge_failure_source": serde_json::to_value(MergeFailureSource::TransientGit).unwrap_or_default(),
                });
                self.transition_to_merge_incomplete(
                    super::TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
                    metadata, true,
                ).await;
                return;
            }
            Ok(super::merge_coordination::PlanUpdateResult::AlreadyUpToDate)
            | Ok(super::merge_coordination::PlanUpdateResult::Updated)
            | Ok(super::merge_coordination::PlanUpdateResult::NotPlanBranch) => {
                tracing::info!(
                    task_id = task_id_str,
                    elapsed_ms = plan_update_elapsed.as_millis() as u64,
                    "update_plan_from_main completed"
                );
                // Continue with merge
            }
            Ok(super::merge_coordination::PlanUpdateResult::Conflicts { conflict_files }) => {
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
                    "base_branch": base_branch,
                    "plan_update_conflict": true,
                });
                super::merge_helpers::merge_metadata_into(task, &metadata);
                task.internal_status = InternalStatus::Merging;
                // Create a merge worktree with the plan branch (target) checked out.
                // The merger agent will run `git merge main` to reproduce and resolve conflicts.
                // First, clean up any stale plan-update worktree that holds the plan branch —
                // git won't allow the same branch in two worktrees simultaneously.
                let plan_update_wt = super::merge_helpers::compute_plan_update_worktree_path(project, task_id_str);
                let plan_update_wt_path = std::path::PathBuf::from(&plan_update_wt);
                super::merge_helpers::pre_delete_worktree(repo_path, &plan_update_wt_path, task_id_str).await;
                let merge_wt = super::merge_helpers::compute_merge_worktree_path(project, task_id_str);
                let merge_wt_path = std::path::PathBuf::from(&merge_wt);
                if let Err(e) = GitService::checkout_existing_branch_worktree(
                    repo_path, &merge_wt_path, &target_branch,
                ).await {
                    tracing::error!(
                        task_id = task_id_str,
                        error = %e,
                        target_branch = %target_branch,
                        "Failed to create merge worktree for plan_update_conflict — falling back to MergeIncomplete"
                    );
                    self.transition_to_merge_incomplete(
                        super::TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
                        serde_json::json!({
                            "error": format!("Failed to create merge worktree for plan update conflict: {}", e),
                            "source_branch": source_branch,
                            "target_branch": target_branch,
                        }),
                        true,
                    ).await;
                    return;
                }
                task.worktree_path = Some(merge_wt);
                self.persist_merge_transition(
                    super::TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
                    InternalStatus::PendingMerge, InternalStatus::Merging,
                    "plan_update_conflict",
                ).await;
                // Spawn the merger agent — mirrors handle_validation_failure's AutoFix path.
                // Without this call, the task sits in Merging with no agent and
                // attempt_merge_auto_complete transitions it straight to MergeIncomplete.
                if let Err(e) = Box::pin(self.on_enter_dispatch(&State::Merging)).await {
                    tracing::error!(
                        task_id = task_id_str,
                        error = %e,
                        "on_enter(Merging) failed during plan_update_conflict routing"
                    );
                }
                return;
            }
            Ok(super::merge_coordination::PlanUpdateResult::Error(err)) => {
                tracing::error!(
                    task_id = task_id_str,
                    error = %err,
                    elapsed_ms = plan_update_elapsed.as_millis() as u64,
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
                self.transition_to_merge_incomplete(
                    super::TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
                    metadata, true,
                ).await;
                return;
            }
        }

        // Update source branch from target if behind (prevents validation failures from stale code)
        emit_merge_progress(
            app_handle,
            task_id_str,
            MergePhase::new(MergePhase::BRANCH_FRESHNESS),
            MergePhaseStatus::Started,
            "Updating source branch from target...".to_string(),
        );
        let source_update_start = std::time::Instant::now();
        let source_update_result = tokio::time::timeout(
            freshness_timeout,
            super::merge_coordination::update_source_from_target(
                repo_path,
                &source_branch,
                &target_branch,
                project,
                task_id_str,
                self.machine.context.services.app_handle.as_ref(),
            ),
        ).await;

        let source_update_elapsed = source_update_start.elapsed();
        match source_update_result {
            Err(_elapsed) => {
                tracing::error!(
                    task_id = task_id_str,
                    timeout_secs = reconciliation_config().branch_freshness_timeout_secs,
                    elapsed_ms = source_update_elapsed.as_millis() as u64,
                    "update_source_from_target timed out — aborting merge"
                );
                let metadata = serde_json::json!({
                    "error": format!(
                        "update_source_from_target timed out after {}s (limit: {}s)",
                        source_update_elapsed.as_secs(),
                        reconciliation_config().branch_freshness_timeout_secs,
                    ),
                    "source_branch": source_branch,
                    "target_branch": target_branch,
                    "merge_failure_source": serde_json::to_value(MergeFailureSource::TransientGit).unwrap_or_default(),
                });
                self.transition_to_merge_incomplete(
                    super::TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
                    metadata, true,
                ).await;
                return;
            }
            Ok(super::merge_coordination::SourceUpdateResult::AlreadyUpToDate)
            | Ok(super::merge_coordination::SourceUpdateResult::Updated) => {
                tracing::info!(
                    task_id = task_id_str,
                    elapsed_ms = source_update_elapsed.as_millis() as u64,
                    "update_source_from_target completed"
                );
                // Continue with merge
            }
            Ok(super::merge_coordination::SourceUpdateResult::Conflicts { conflict_files }) => {
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
                super::merge_helpers::merge_metadata_into(task, &metadata);
                task.internal_status = InternalStatus::Merging;
                // Create a merge worktree with source branch checked out so the merger
                // agent can resolve the conflict in an isolated directory. Mirrors the
                // plan_update_conflict path which sets worktree_path before persist.
                let merge_wt = super::merge_helpers::compute_merge_worktree_path(project, task_id_str);
                let merge_wt_path = std::path::PathBuf::from(&merge_wt);
                // RC#13: Clean up any stale merge worktree from a prior phase before
                // creating a fresh one. Without this, checkout_existing_branch_worktree
                // fails with "fatal: '/path/merge-{id}' already exists".
                super::merge_helpers::pre_delete_worktree(repo_path, &merge_wt_path, task_id_str).await;
                if let Err(e) = GitService::checkout_existing_branch_worktree(
                    repo_path, &merge_wt_path, &source_branch,
                ).await {
                    tracing::error!(
                        task_id = task_id_str,
                        error = %e,
                        source_branch = %source_branch,
                        "Failed to create merge worktree for source_update_conflict — falling back to MergeIncomplete"
                    );
                    self.transition_to_merge_incomplete(
                        super::TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
                        serde_json::json!({
                            "error": format!("Failed to create merge worktree for source update conflict: {}", e),
                            "source_branch": source_branch,
                            "target_branch": target_branch,
                        }),
                        true,
                    ).await;
                    return;
                }
                task.worktree_path = Some(merge_wt);
                self.persist_merge_transition(
                    super::TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
                    InternalStatus::PendingMerge, InternalStatus::Merging,
                    "source_update_conflict",
                ).await;
                // Spawn the merger agent — mirrors handle_validation_failure's AutoFix path.
                // Without this call, the task sits in Merging with no agent and
                // attempt_merge_auto_complete transitions it straight to MergeIncomplete.
                if let Err(e) = Box::pin(self.on_enter_dispatch(&State::Merging)).await {
                    tracing::error!(
                        task_id = task_id_str,
                        error = %e,
                        "on_enter(Merging) failed during source_update_conflict routing"
                    );
                }
                return;
            }
            Ok(super::merge_coordination::SourceUpdateResult::Error(err)) => {
                tracing::warn!(
                    task_id = task_id_str,
                    error = %err,
                    elapsed_ms = source_update_elapsed.as_millis() as u64,
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
        self.emit_merge_activity_event(
            task_id_str,
            "Merge pipeline: branch freshness check passed",
            MergePhase::BRANCH_FRESHNESS,
            "passed",
        ).await;

        // "Already merged" early exit
        if self.check_already_merged(
            super::TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
            super::BranchPair { source_branch: &source_branch, target_branch: &target_branch },
            super::ProjectCtx { project, repo_path },
            plan_branch_repo,
        ).await {
            return;
        }

        // "Deleted source branch" recovery
        if self.recover_deleted_source_branch(
            super::TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
            super::BranchPair { source_branch: &source_branch, target_branch: &target_branch },
            super::ProjectCtx { project, repo_path },
            plan_branch_repo,
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
                task, task_id_str, &target_branch, project, task_repo, plan_branch_repo,
            ).await,
            ConcurrentGuardResult::Deferred
        ) {
            return;
        }

        // Overall merge deadline — computed from function start to bound the full pipeline
        // (cleanup + freshness + dispatch). Previously this was a NOP because the deadline
        // was created and checked at the same instant (always passed).
        let deadline_secs = reconciliation_config().attempt_merge_deadline_secs;
        let deadline_duration = std::time::Duration::from_secs(deadline_secs);

        // Check deadline after cleanup+freshness (using attempt_start from function top)
        if attempt_start.elapsed() >= deadline_duration {
            tracing::error!(
                task_id = task_id_str,
                deadline_secs = deadline_secs,
                elapsed_ms = attempt_start.elapsed().as_millis() as u64,
                "Programmatic merge exceeded deadline during cleanup — transitioning to MergeIncomplete"
            );
            // Guard will clear merge_pipeline_active flag on return
            let metadata = serde_json::json!({
                "error": format!("Merge attempt timed out after {}s (cleanup phase exceeded deadline)", deadline_secs),
                "source_branch": source_branch,
                "target_branch": target_branch,
            });
            self.transition_to_merge_incomplete(
                super::TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
                metadata, true,
            ).await;
            return;
        }

        // Build squash commit message
        let squash_commit_msg = self
            .build_squash_commit_message(task, task_id_str, &source_branch)
            .await;

        // Dispatch merge strategy with timeout — remaining time computed from function start
        let remaining = deadline_duration.saturating_sub(attempt_start.elapsed());
        tracing::info!(
            task_id = task_id_str,
            elapsed_ms = attempt_start.elapsed().as_millis() as u64,
            remaining_ms = remaining.as_millis() as u64,
            deadline_secs = deadline_secs,
            "Merge pipeline: cleanup + freshness complete, dispatching strategy"
        );
        self.emit_merge_activity_event(
            task_id_str,
            format!("Merge pipeline: merging {} into {}", source_branch, target_branch),
            MergePhase::PROGRAMMATIC_MERGE,
            "started",
        ).await;
        self.dispatch_merge_strategy(
            super::TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
            super::BranchPair { source_branch: &source_branch, target_branch: &target_branch },
            super::ProjectCtx { project, repo_path },
            &squash_commit_msg, plan_branch_repo, remaining, deadline_secs,
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
        tc: super::TaskCore<'_>,
        from_status: InternalStatus,
        to_status: InternalStatus,
        persist_label: &str,
    ) -> bool {
        let (task, task_id, task_id_str, task_repo) = (tc.task, tc.task_id, tc.task_id_str, tc.task_repo);
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
    /// Merges `metadata` INTO the task's existing metadata so that recovery history
    /// (e.g., `merge_recovery` events, `validation_revert_count`, attempt counters)
    /// is preserved across MergeIncomplete->PendingMerge retry cycles.
    ///
    /// Handles the full transition: merge metadata -> persist status change -> emit event.
    /// Optionally triggers on_exit (needed when the caller wants deferred-merge retry).
    pub(super) async fn transition_to_merge_incomplete(
        &self,
        tc: super::TaskCore<'_>,
        metadata: serde_json::Value,
        trigger_on_exit: bool,
    ) {
        let (task, task_id, task_id_str, task_repo) = (tc.task, tc.task_id, tc.task_id_str, tc.task_repo);
        // Merge new metadata INTO existing metadata to preserve recovery history
        super::merge_helpers::merge_metadata_into(task, &metadata);
        task.internal_status = InternalStatus::MergeIncomplete;

        if !self.persist_merge_transition(
            super::TaskCore { task: &mut *task, task_id, task_id_str, task_repo },
            InternalStatus::PendingMerge, InternalStatus::MergeIncomplete,
            "merge_incomplete",
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
                // Idempotency guard: if already merged, return early (AD20)
                if pb.status == crate::domain::entities::PlanBranchStatus::Merged {
                    tracing::debug!(
                        task_id = task_id_str,
                        "post_merge_cleanup: plan branch already merged, skipping (idempotency guard)"
                    );
                    return;
                }
                if let Err(e) = plan_branch_repo.set_merged(&pb.id).await {
                    tracing::warn!(
                        error = %e,
                        task_id = task_id_str,
                        plan_branch_id = pb.id.as_str(),
                        "Failed to mark plan branch as merged (non-fatal)"
                    );
                }

                // PR mode: GitHub may have auto-deleted the plan branch; use delete_remote_branch
                // (idempotent — already-deleted is treated as no-op). Push-to-main: delete local.
                let pr_mode = pb.pr_eligible
                    && pb.pr_number.is_some()
                    && self.machine.context.services.github_service.is_some();

                if pr_mode {
                    if let Some(ref github) = self.machine.context.services.github_service {
                        if let Err(e) = github.delete_remote_branch(repo_path, &pb.branch_name).await {
                            tracing::warn!(
                                error = %e,
                                task_id = task_id_str,
                                branch = %pb.branch_name,
                                "PR mode: failed to delete remote plan branch (non-fatal)"
                            );
                        } else {
                            tracing::info!(
                                task_id = task_id_str,
                                branch = %pb.branch_name,
                                "PR mode: deleted remote plan branch"
                            );
                        }
                    }
                } else if let Err(e) = GitService::delete_feature_branch(repo_path, &pb.branch_name).await {
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

                // Cascade stop: cancel/stop sibling tasks in the same plan
                self.cascade_stop_sibling_tasks(task_id, task_id_str, &pb)
                    .await;

                // PR mode: delete remote task branches for all plan siblings
                if pr_mode {
                    if let (Some(ref github), Some(ref task_repo)) = (
                        &self.machine.context.services.github_service,
                        &self.machine.context.services.task_repo,
                    ) {
                        self.delete_remote_task_branches(
                            task_id,
                            task_id_str,
                            &pb,
                            task_repo,
                            github.as_ref(),
                            repo_path,
                        )
                        .await;
                    }
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

    /// Cascade stop sibling tasks after a plan merge completes.
    ///
    /// Uses `execution_plan_id` from the PlanBranch to find siblings (precise).
    /// Falls back to `ideation_session_id` if `execution_plan_id` is not set.
    /// Transitions non-terminal siblings to Stopped or Cancelled.
    /// The merge task itself is excluded from cascade.
    ///
    /// For states that don't have a valid transition to Stopped or Cancelled
    /// (e.g., QaPassed, PendingReview, ReviewPassed, Escalated, Approved),
    /// we force the transition via `persist_status_change` — this is an
    /// emergency cascade, not normal state machine flow.
    pub(super) async fn cascade_stop_sibling_tasks(
        &self,
        merge_task_id: &TaskId,
        merge_task_id_str: &str,
        plan_branch: &PlanBranch,
    ) {
        let Some(ref task_repo) = self.machine.context.services.task_repo else {
            return;
        };

        // Query siblings by execution_plan_id (precise) or fall back to session_id
        let siblings = if let Some(ref ep_id) = plan_branch.execution_plan_id {
            match task_repo
                .list_paginated(
                    &plan_branch.project_id,
                    None,  // all statuses
                    0,
                    10_000, // large limit to get all
                    false,  // exclude archived
                    None,   // no session filter
                    Some(ep_id.as_str()),
                    None,   // no category filter
                )
                .await
            {
                Ok(tasks) => tasks,
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        merge_task_id = merge_task_id_str,
                        execution_plan_id = ep_id.as_str(),
                        "Failed to query sibling tasks by execution_plan_id for cascade stop"
                    );
                    return;
                }
            }
        } else {
            match task_repo
                .get_by_ideation_session(&plan_branch.session_id)
                .await
            {
                Ok(tasks) => tasks,
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        merge_task_id = merge_task_id_str,
                        session_id = plan_branch.session_id.as_str(),
                        "Failed to query sibling tasks by session_id for cascade stop"
                    );
                    return;
                }
            }
        };

        let mut stopped_count = 0u32;
        for sibling in &siblings {
            // Skip the merge task itself
            if sibling.id == *merge_task_id {
                continue;
            }

            // Skip already-terminal tasks
            if sibling.internal_status.is_terminal() {
                continue;
            }

            let from = sibling.internal_status;

            // Choose target state:
            // - States with valid → Stopped: use Stopped
            // - States with valid → Cancelled but not Stopped: use Cancelled
            // - States with neither (QaPassed, PendingReview, ReviewPassed, etc.):
            //   force Stopped via persist_status_change (emergency cascade)
            let to = if from.can_transition_to(InternalStatus::Stopped) {
                InternalStatus::Stopped
            } else if from.can_transition_to(InternalStatus::Cancelled) {
                InternalStatus::Cancelled
            } else {
                // Force stop — these states (QaPassed, PendingReview, ReviewPassed,
                // Escalated, Approved, QaFailed) don't have valid exit transitions
                // to Stopped/Cancelled, but we must stop them to prevent execution
                // on a merged branch.
                tracing::warn!(
                    task_id = sibling.id.as_str(),
                    from = %from,
                    merge_task_id = merge_task_id_str,
                    "Force-stopping sibling in state with no valid Stopped/Cancelled transition"
                );
                InternalStatus::Stopped
            };

            // Stop poller BEFORE status change (AD11: TOCTOU prevention).
            // stop_polling is synchronous fire-and-forget — abort never panics.
            if let Some(ref registry) = self.machine.context.services.pr_poller_registry {
                registry.stop_polling(&sibling.id);
                tracing::debug!(
                    task_id = sibling.id.as_str(),
                    "cascade_stop: stopped poller for sibling before persist_status_change"
                );
            }

            // PR-mode running count fix: cascade_stop uses persist_status_change (bypasses
            // on_exit hooks). For PR-mode Merging siblings, increment_running() was called in
            // on_enter(Merging) — decrement explicitly here to prevent u32::MAX underflow (AD11).
            if from == InternalStatus::Merging && plan_branch.pr_number.is_some() {
                if let Some(ref exec) = self.machine.context.services.execution_state {
                    exec.decrement_running();
                    tracing::debug!(
                        task_id = sibling.id.as_str(),
                        "cascade_stop: decremented running count for PR-mode Merging sibling"
                    );
                }
            }

            if let Err(e) = task_repo
                .persist_status_change(
                    &sibling.id,
                    from,
                    to,
                    "post_merge_cascade_stop",
                )
                .await
            {
                tracing::warn!(
                    error = %e,
                    task_id = sibling.id.as_str(),
                    from = %from,
                    to = %to,
                    "Failed to cascade-stop sibling task"
                );
            } else {
                stopped_count += 1;
                tracing::info!(
                    task_id = sibling.id.as_str(),
                    from = %from,
                    to = %to,
                    merge_task_id = merge_task_id_str,
                    "Cascade-stopped sibling task after plan merge"
                );
            }
        }

        if stopped_count > 0 {
            tracing::info!(
                merge_task_id = merge_task_id_str,
                stopped_count,
                "Post-merge cascade stop complete — agents may still be running for stopped tasks"
            );
        }
    }

    /// Delete remote task branches for all siblings in a plan after PR-mode merge.
    ///
    /// Called from `post_merge_cleanup` in PR mode only.
    /// Each task has a `task_branch` field (e.g. `ralphx/ralphx/task-xxx`).
    /// Failures are non-fatal: logged as warn, cleanup continues for remaining branches.
    async fn delete_remote_task_branches(
        &self,
        merge_task_id: &TaskId,
        merge_task_id_str: &str,
        plan_branch: &PlanBranch,
        task_repo: &Arc<dyn crate::domain::repositories::TaskRepository>,
        github: &dyn GithubServiceTrait,
        repo_path: &Path,
    ) {
        let siblings = if let Some(ref ep_id) = plan_branch.execution_plan_id {
            match task_repo
                .list_paginated(
                    &plan_branch.project_id,
                    None,
                    0,
                    10_000,
                    false,
                    None,
                    Some(ep_id.as_str()),
                    None,
                )
                .await
            {
                Ok(tasks) => tasks,
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        merge_task_id = merge_task_id_str,
                        "PR mode: failed to query siblings for task branch cleanup"
                    );
                    return;
                }
            }
        } else {
            match task_repo.get_by_ideation_session(&plan_branch.session_id).await {
                Ok(tasks) => tasks,
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        merge_task_id = merge_task_id_str,
                        "PR mode: failed to query siblings by session for task branch cleanup"
                    );
                    return;
                }
            }
        };

        for sibling in &siblings {
            if sibling.id == *merge_task_id {
                continue;
            }
            let Some(ref branch) = sibling.task_branch else {
                continue;
            };
            if let Err(e) = github.delete_remote_branch(repo_path, branch).await {
                tracing::warn!(
                    error = %e,
                    task_id = sibling.id.as_str(),
                    branch = %branch,
                    merge_task_id = merge_task_id_str,
                    "PR mode: failed to delete remote task branch (non-fatal)"
                );
            } else {
                tracing::info!(
                    task_id = sibling.id.as_str(),
                    branch = %branch,
                    merge_task_id = merge_task_id_str,
                    "PR mode: deleted remote task branch"
                );
            }
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
        tc: super::TaskCore<'_>,
        bp: super::BranchPair<'_>,
        pc: super::ProjectCtx<'_>,
        failures: &[ValidationFailure],
        log: &[ValidationLogEntry],
        merge_path: &Path,
        mode_label: &str,
        validation_mode: &MergeValidationMode,
    ) {
        let (task, task_id, task_id_str, task_repo) = (tc.task, tc.task_id, tc.task_id_str, tc.task_repo);
        let (source_branch, target_branch) = (bp.source_branch, bp.target_branch);
        let (project, repo_path) = (pc.project, pc.repo_path);
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
                super::merge_helpers::pre_delete_worktree(repo_path, &wt_path, task_id_str).await;

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
                            super::TaskCore { task: &mut *task, task_id, task_id_str, task_repo },
                            metadata, true,
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
            super::merge_helpers::merge_metadata_into(task, &serde_json::json!({
                "validation_recovery": true,
                "validation_failures": failure_details,
                "validation_log": log,
                "source_branch": source_branch,
                "target_branch": target_branch,
            }));
            task.worktree_path = Some(fixer_worktree_path.to_string_lossy().to_string());
            task.internal_status = InternalStatus::Merging;

            self.persist_merge_transition(
                super::TaskCore { task: &mut *task, task_id, task_id_str, task_repo },
                InternalStatus::PendingMerge, InternalStatus::Merging,
                "validation_auto_fix",
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
            let prev_revert_count: u32 = super::merge_helpers::parse_metadata(task)
                .and_then(|v| v.get("validation_revert_count")?.as_u64())
                .unwrap_or(0) as u32;
            let revert_count = prev_revert_count + 1;

            let error_metadata_str = format_validation_error_metadata(failures, log, source_branch, target_branch);
            if let Ok(error_obj) = serde_json::from_str::<serde_json::Value>(&error_metadata_str) {
                super::merge_helpers::merge_metadata_into(task, &error_obj);
            }
            let mut extra = serde_json::json!({
                "merge_failure_source": serde_json::to_value(MergeFailureSource::ValidationFailed)
                    .unwrap_or(serde_json::json!("validation_failed")),
                "validation_revert_count": revert_count,
            });
            // Flag unrevertable merge commits so check_already_merged() doesn't
            // fast-path to completion with a failing merge commit on the target.
            if revert_failed {
                extra["merge_commit_unrevertable"] = serde_json::json!(true);
                if let Some(ref sha) = merge_head_sha {
                    extra["unrevertable_commit_sha"] = serde_json::json!(sha);
                }
            }
            super::merge_helpers::merge_metadata_into(task, &extra);
            task.internal_status = InternalStatus::MergeIncomplete;

            self.persist_merge_transition(
                super::TaskCore { task: &mut *task, task_id, task_id_str, task_repo },
                InternalStatus::PendingMerge, InternalStatus::MergeIncomplete,
                "validation_failed",
            ).await;
        }
    }

    /// Emit a system ActivityEvent for a merge pipeline phase.
    ///
    /// Non-fatal: if the repo is unavailable or the save fails, logs a warning and continues.
    pub(super) async fn emit_merge_activity_event(
        &self,
        task_id_str: &str,
        content: impl Into<String>,
        phase_id: &str,
        result: &str,
    ) {
        let Some(ref repo) = self.machine.context.services.activity_event_repo else {
            return;
        };
        let tid = TaskId::from_string(task_id_str.to_string());
        let metadata = serde_json::json!({ "phase_id": phase_id, "result": result }).to_string();
        let event = ActivityEvent::new_task_event(tid, ActivityEventType::System, content)
            .with_role(ActivityEventRole::System)
            .with_metadata(metadata);
        if let Err(e) = repo.save(event).await {
            tracing::warn!(
                task_id = task_id_str,
                phase_id = phase_id,
                error = %e,
                "Failed to save merge activity event (non-fatal)"
            );
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

/// Clear `merge_pipeline_active` dedicated column.
/// Reloads from DB to avoid clobbering concurrent changes to other fields.
async fn clear_merge_pipeline_active_from_db(
    task_id_str: &str,
    task_repo: &Arc<dyn TaskRepository>,
) {
    let task_id = TaskId(task_id_str.to_string());
    let mut task = match task_repo.get_by_id(&task_id).await {
        Ok(Some(t)) => t,
        _ => return,
    };
    if task.merge_pipeline_active.is_none() {
        return; // Already cleared — skip redundant update
    }
    task.merge_pipeline_active = None;
    task.touch();
    if let Err(e) = task_repo.update(&task).await {
        tracing::warn!(
            task_id = task_id_str,
            error = %e,
            "Failed to clear merge_pipeline_active flag (non-fatal, auto-expires)"
        );
    } else {
        tracing::info!(task_id = task_id_str, "merge_pipeline_active flag cleared");
    }
}

/// Set `merge_pipeline_active` dedicated column so the reconciler skips the task
/// while the merge pipeline is running. Auto-expires after `attempt_merge_deadline_secs`
/// as a crash safety net. Uses a dedicated column (not JSON metadata) to prevent
/// race-condition clobbers by concurrent metadata writers.
async fn set_merge_pipeline_active(
    task: &mut Task,
    task_id_str: &str,
    task_repo: &Arc<dyn TaskRepository>,
) {
    task.merge_pipeline_active = Some(chrono::Utc::now().to_rfc3339());
    task.touch();
    if let Err(e) = task_repo.update(task).await {
        tracing::warn!(
            task_id = task_id_str,
            error = %e,
            "Failed to persist merge_pipeline_active flag (non-fatal)"
        );
    } else {
        tracing::info!(task_id = task_id_str, "merge_pipeline_active flag set");
    }
}

