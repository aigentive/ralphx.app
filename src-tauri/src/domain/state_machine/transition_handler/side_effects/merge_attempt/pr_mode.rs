use super::*;
use crate::domain::services::{PlanPrPublisher, PrReviewState};
use crate::domain::state_machine::{State, TransitionHandler};
use crate::domain::state_machine::transition_handler::{
    resolve_plan_branch_pr_base, TaskCore,
};

impl<'a> TransitionHandler<'a> {
    /// PR-mode PendingMerge path (AD17).
    ///
    /// Called when pr_eligible=true AND github_service is available.
    /// Pushes the plan branch and marks the PR ready, then transitions to Merging.
    /// The Merging entry point (on_enter_states.rs) will start the poller.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::domain::state_machine::transition_handler::side_effects) async fn run_pr_mode_pending_merge(
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
        let target_branch = resolve_plan_branch_pr_base(project, &plan_branch);
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
            let publisher = PlanPrPublisher::new(
                github_service,
                self.machine.context.services.ideation_session_repo.as_ref(),
                self.machine.context.services.artifact_repo.as_ref(),
            );
            if let Err(e) = publisher
                .sync_existing_pr(task, project, &plan_branch, PrReviewState::Ready)
                .await
            {
                tracing::warn!(
                    task_id = task_id_str,
                    pr_number = existing_pr_number,
                    error = %e,
                    "PR-mode: failed to refresh PR details before marking ready"
                );
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
                let mut refreshed_plan_branch = plan_branch.clone();
                refreshed_plan_branch.pr_number = Some(new_pr_number);
                let publisher = PlanPrPublisher::new(
                    github_service,
                    self.machine.context.services.ideation_session_repo.as_ref(),
                    self.machine.context.services.artifact_repo.as_ref(),
                );
                if let Err(e) = publisher
                    .sync_existing_pr(task, project, &refreshed_plan_branch, PrReviewState::Ready)
                    .await
                {
                    tracing::warn!(
                        task_id = task_id_str,
                        pr_number = new_pr_number,
                        error = %e,
                        "PR-mode: failed to refresh concurrently-created PR details before marking ready"
                    );
                }
                github_service.mark_pr_ready(&working_dir, new_pr_number).await
                    .map(|_| new_pr_number)
            } else {
                // Still no PR: push + create non-draft PR directly
                tracing::info!(task_id = task_id_str, "PR-mode: creating non-draft PR directly");
                match github_service.push_branch(&working_dir, &branch_name).await {
                    Err(e) => Err(e),
                    Ok(()) => {
                        let publisher = PlanPrPublisher::new(
                            github_service,
                            self.machine.context.services.ideation_session_repo.as_ref(),
                            self.machine.context.services.artifact_repo.as_ref(),
                        );
                        match publisher.create_draft_pr(task, project, &plan_branch).await {
                            Err(e) => Err(e),
                            Ok((new_pr_number, pr_url)) => {
                                let _ = plan_branch_repo.update_pr_info(
                                    &plan_branch.id,
                                    new_pr_number,
                                    pr_url,
                                    crate::domain::entities::plan_branch::PrStatus::Open,
                                    true,
                                ).await;

                                let mut ready_plan_branch = plan_branch.clone();
                                ready_plan_branch.pr_number = Some(new_pr_number);
                                if let Err(e) = publisher
                                    .sync_existing_pr(task, project, &ready_plan_branch, PrReviewState::Ready)
                                    .await
                                {
                                    tracing::warn!(
                                        task_id = task_id_str,
                                        pr_number = new_pr_number,
                                        error = %e,
                                        "PR-mode: failed to refresh newly-created PR details before marking ready"
                                    );
                                }
                                github_service.mark_pr_ready(&working_dir, new_pr_number).await
                                    .map(|_| new_pr_number)
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
}
