use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::application::GitService;
use crate::domain::entities::{
    merge_progress_event::{MergePhase, MergePhaseStatus},
    InternalStatus, TaskId,
};
use crate::domain::repositories::TaskRepository;
use crate::infrastructure::agents::claude::git_runtime_config;

use super::cleanup_helpers::{run_cleanup_step, CleanupStepResult};


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
    pub(crate) async fn pre_merge_cleanup(
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
        let is_first = super::is_first_clean_attempt(task);
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
                        super::MERGE_DEBRIS_METADATA_KEYS
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
        super::emit_merge_progress(
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
            super::emit_merge_progress(
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
            super::emit_merge_progress(
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
                ("task", super::merge_helpers::compute_task_worktree_path(project, task_id_str)),
                ("merge", super::merge_helpers::compute_merge_worktree_path(project, task_id_str)),
                (
                    "rebase",
                    super::merge_helpers::compute_rebase_worktree_path(project, task_id_str),
                ),
                (
                    "plan-update",
                    super::merge_helpers::compute_plan_update_worktree_path(project, task_id_str),
                ),
                (
                    "source-update",
                    super::merge_helpers::compute_source_update_worktree_path(project, task_id_str),
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
