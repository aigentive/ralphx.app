use super::*;
use crate::domain::state_machine::{State, TransitionHandler};
use crate::domain::state_machine::transition_handler::{BranchPair, ProjectCtx, TaskCore, cleanup_helpers, merge_coordination, merge_helpers};

impl<'a> TransitionHandler<'a> {
    /// PR-mode PendingMerge path (AD17).
    ///
    /// Called when pr_eligible=true AND github_service is available.
    /// Pushes the plan branch and marks the PR ready, then transitions to Merging.
    /// The Merging entry point (on_enter_states.rs) will start the poller.
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn run_pr_mode_pending_merge(
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
                    TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
                    InternalStatus::PendingMerge, InternalStatus::Merging,
                    "pr_mode_mark_ready",
                ).await {
                    // Trigger on_enter(Merging) to start the PR poller
                    if let Err(e) = Box::pin(self.on_enter_dispatch(&State::Merging)).await {
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
                    TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
                    metadata, true,
                ).await;
            }
        }
    }

    /// Inner body of `attempt_programmatic_merge`. Extracted so the outer wrapper
    /// can guarantee the `merge_pipeline_active` flag is always cleared on exit.
    pub(super) async fn run_merge_pipeline_body(
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
                TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
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
                TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
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
        if merge_coordination::check_main_merge_deferral(
            TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
            BranchPair { source_branch: &source_branch, target_branch: &target_branch },
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
        match cleanup_helpers::os_thread_timeout(
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
                merge_helpers::merge_metadata_into(task, &serde_json::json!({
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
        merge_coordination::ensure_plan_branch_exists(
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
            merge_coordination::update_plan_from_main(
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
                    TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
                    metadata, true,
                ).await;
                return;
            }
            Ok(merge_coordination::PlanUpdateResult::AlreadyUpToDate)
            | Ok(merge_coordination::PlanUpdateResult::Updated)
            | Ok(merge_coordination::PlanUpdateResult::NotPlanBranch) => {
                tracing::info!(
                    task_id = task_id_str,
                    elapsed_ms = plan_update_elapsed.as_millis() as u64,
                    "update_plan_from_main completed"
                );
                // Continue with merge
            }
            Ok(merge_coordination::PlanUpdateResult::Conflicts { conflict_files }) => {
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
                merge_helpers::merge_metadata_into(task, &metadata);
                task.internal_status = InternalStatus::Merging;
                // Create a merge worktree with the plan branch (target) checked out.
                // The merger agent will run `git merge main` to reproduce and resolve conflicts.
                // First, clean up any stale plan-update worktree that holds the plan branch —
                // git won't allow the same branch in two worktrees simultaneously.
                let plan_update_wt = merge_helpers::compute_plan_update_worktree_path(project, task_id_str);
                let plan_update_wt_path = std::path::PathBuf::from(&plan_update_wt);
                merge_helpers::pre_delete_worktree(repo_path, &plan_update_wt_path, task_id_str).await;
                let merge_wt = merge_helpers::compute_merge_worktree_path(project, task_id_str);
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
                        TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
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
                    TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
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
            Ok(merge_coordination::PlanUpdateResult::Error(err)) => {
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
                    TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
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
            merge_coordination::update_source_from_target(
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
                    TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
                    metadata, true,
                ).await;
                return;
            }
            Ok(merge_coordination::SourceUpdateResult::AlreadyUpToDate)
            | Ok(merge_coordination::SourceUpdateResult::Updated) => {
                tracing::info!(
                    task_id = task_id_str,
                    elapsed_ms = source_update_elapsed.as_millis() as u64,
                    "update_source_from_target completed"
                );
                // Continue with merge
            }
            Ok(merge_coordination::SourceUpdateResult::Conflicts { conflict_files }) => {
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
                merge_helpers::merge_metadata_into(task, &metadata);
                task.internal_status = InternalStatus::Merging;
                // Create a merge worktree with source branch checked out so the merger
                // agent can resolve the conflict in an isolated directory. Mirrors the
                // plan_update_conflict path which sets worktree_path before persist.
                let merge_wt = merge_helpers::compute_merge_worktree_path(project, task_id_str);
                let merge_wt_path = std::path::PathBuf::from(&merge_wt);
                // RC#13: Clean up any stale merge worktree from a prior phase before
                // creating a fresh one. Without this, checkout_existing_branch_worktree
                // fails with "fatal: '/path/merge-{id}' already exists".
                merge_helpers::pre_delete_worktree(repo_path, &merge_wt_path, task_id_str).await;
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
                        TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
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
                    TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
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
            Ok(merge_coordination::SourceUpdateResult::Error(err)) => {
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
            TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
            BranchPair { source_branch: &source_branch, target_branch: &target_branch },
            ProjectCtx { project, repo_path },
            plan_branch_repo,
        ).await {
            return;
        }

        // "Deleted source branch" recovery
        if self.recover_deleted_source_branch(
            TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
            BranchPair { source_branch: &source_branch, target_branch: &target_branch },
            ProjectCtx { project, repo_path },
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
                TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
                metadata, true,
            ).await;
            return;
        }

        // Build squash commit message
        let squash_commit_msg = self
            .build_squash_commit_message(task, task_id_str, &source_branch, &target_branch)
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
            TaskCore { task: &mut *task, task_id: &task_id, task_id_str, task_repo },
            BranchPair { source_branch: &source_branch, target_branch: &target_branch },
            ProjectCtx { project, repo_path },
            &squash_commit_msg, plan_branch_repo, remaining, deadline_secs,
        ).await;
    }

    /// Try to insert this task into the in-flight set. Returns false if already in flight.
    pub(super) fn try_acquire_in_flight_guard(&self, task_id_str: &str) -> bool {
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
    pub(super) async fn log_branch_discovery(
        &self,
        task: &mut Task,
        project: &crate::domain::entities::Project,
        task_repo: &Arc<dyn TaskRepository>,
        task_id_str: &str,
    ) {
        match merge_helpers::discover_and_attach_task_branch(task, project, task_repo)
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
}
