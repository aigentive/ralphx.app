// State entry side effects
// This module contains the on_enter implementation that handles state-specific actions
//
// Sibling modules (extracted for maintainability, declared in transition_handler/mod.rs):
// - merge_helpers: path computation, metadata parsing, branch resolution
// - merge_completion: finalize merge and cleanup branch/worktree
// - merge_validation: post-merge validation gate (setup + validate phases)

// Re-export public/crate items so transition_handler/mod.rs re-exports still resolve
pub use super::merge_completion::complete_merge_internal;
pub use super::merge_helpers::resolve_merge_branches;

pub(crate) use super::merge_helpers::{
    clear_merge_deferred_metadata, clear_trigger_origin,
    get_trigger_origin, has_branch_missing_metadata,
    has_merge_deferred_metadata, parse_metadata, set_trigger_origin,
};
pub(crate) use super::merge_validation::{
    format_validation_error_metadata, run_validation_commands, ValidationLogEntry,
};

// Internal imports used by code remaining in this file
use super::merge_helpers::{
    compute_merge_worktree_path, compute_rebase_worktree_path, extract_task_id_from_merge_path,
    is_task_in_merge_workflow, task_targets_branch, truncate_str, validate_plan_merge_preconditions,
};
use super::merge_outcome_handler::MergeHandlerOptions;
use super::merge_validation::{emit_merge_progress, ValidationFailure};

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::Emitter;

use super::super::machine::State;
use crate::application::GitService;
use crate::infrastructure::agents::claude::{git_runtime_config, reconciliation_config, scheduler_config};
use crate::domain::entities::{
    merge_progress_event::{MergePhase, MergePhaseStatus},
    task_metadata::{
        MergeFailureSource, MergeRecoveryEvent, MergeRecoveryEventKind, MergeRecoveryMetadata,
        MergeRecoveryReasonCode, MergeRecoverySource, MergeRecoveryState,
    },
    InternalStatus, MergeStrategy, MergeValidationMode,
    ProjectId, Task, TaskCategory, TaskId,
};
use crate::domain::repositories::{
    PlanBranchRepository, TaskRepository,
};
use crate::error::AppResult;
pub(super) const TEMP_SKIP_POST_MERGE_VALIDATION: bool = true;

/// Seconds to wait after SIGTERM for process tree cleanup before worktree deletion.
/// Prevents TOCTOU race where git operations fail because agent processes still hold files.
const AGENT_KILL_SETTLE_SECS: u64 = 1;

/// Timeout in seconds for deleting the task worktree (step 2 of pre_merge_cleanup).
const CLEANUP_TASK_WORKTREE_TIMEOUT_SECS: u64 = 10;

/// Timeout in seconds for merge/rebase worktree deletion and git clean (steps 4, 5, 6).
const CLEANUP_GIT_OP_TIMEOUT_SECS: u64 = 30;

use super::cleanup_helpers::run_cleanup_step;
use super::commit_messages::{build_plan_merge_commit_msg, build_squash_commit_msg};

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
        // Prevent two concurrent `attempt_programmatic_merge` calls for the same task
        // (e.g., double-click retry or reconciliation racing with on_enter(PendingMerge)).
        // Uses std::sync::Mutex for synchronous insert/remove (safe from async context).
        {
            let mut in_flight = self
                .machine
                .context
                .services
                .merges_in_flight
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            if !in_flight.insert(task_id_str.clone()) {
                tracing::info!(
                    task_id = task_id_str,
                    "Merge attempt skipped — already in flight for this task (self-dedup guard)"
                );
                return;
            }
        }
        // Register a cleanup guard so we always remove the task from `merges_in_flight`
        // when this function returns (success, error, or early return).
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
        let _in_flight_guard = InFlightGuard {
            set: std::sync::Arc::clone(&self.machine.context.services.merges_in_flight),
            id: task_id_str.clone(),
        };

        // Only proceed if repos are available
        let (Some(ref task_repo), Some(ref project_repo)) = (
            &self.machine.context.services.task_repo,
            &self.machine.context.services.project_repo,
        ) else {
            tracing::error!(
                task_id = task_id_str,
                project_id = project_id_str,
                task_repo_available = self.machine.context.services.task_repo.is_some(),
                project_repo_available = self.machine.context.services.project_repo.is_some(),
                "Programmatic merge BLOCKED: repos not available — \
                 task will remain stuck in PendingMerge"
            );
            // Cannot write MergeIncomplete to DB without repos, but call on_exit so
            // deferred merge retries for other tasks are not blocked by this one.
            self.on_exit(&State::PendingMerge, &State::MergeIncomplete)
                .await;
            return;
        };

        let task_id = TaskId::from_string(task_id_str.clone());
        let project_id = ProjectId::from_string(project_id_str.clone());

        // Fetch task and project
        let task_result = task_repo.get_by_id(&task_id).await;
        let project_result = project_repo.get_by_id(&project_id).await;

        let (Ok(Some(mut task)), Ok(Some(project))) = (task_result, project_result) else {
            tracing::error!(
                task_id = task_id_str,
                project_id = project_id_str,
                "Programmatic merge BLOCKED: failed to fetch task or project from database — \
                 task will remain stuck in PendingMerge"
            );
            return;
        };

        // Attempt to discover and re-attach orphaned task branch
        // (handles recovery from Failed/Critical states where task_branch was cleared)
        match super::merge_helpers::discover_and_attach_task_branch(&mut task, &project, task_repo)
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

        // Pre-merge validation: for plan_merge tasks, check preconditions upfront
        // before attempting any git operations. This surfaces actionable errors
        // (S9: repo not wired, S10/S12: branch not active, S13: feature branch deleted)
        // as clear MergeIncomplete transitions instead of silent failures deep in the merge flow.
        let plan_branch_repo = &self.machine.context.services.plan_branch_repo;
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

        // Resolve source and target branches (handles merge tasks and plan feature branches)
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

        // --- Main-merge deferral check ---
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

        let repo_path = Path::new(&project.working_directory);

        // Ensure plan branch exists as git ref (lazy creation for merge target)
        super::merge_coordination::ensure_plan_branch_exists(
            &task, repo_path, &target_branch, plan_branch_repo,
        ).await;

        // --- "Already merged" early exit ---
        // If the source branch is already an ancestor of the target branch, the merge
        // was completed by a prior agent run that died before calling complete_merge.
        // Skip the merge entirely and transition straight to Merged.
        if let Ok(source_sha) = GitService::get_branch_sha(repo_path, &source_branch).await {
            if let Ok(true) =
                GitService::is_commit_on_branch(repo_path, &source_sha, &target_branch).await
            {
                tracing::info!(
                    task_id = task_id_str,
                    source_branch = %source_branch,
                    target_branch = %target_branch,
                    source_sha = %source_sha,
                    "Source branch already merged into target — skipping merge"
                );

                // Clean up orphaned merge worktree (if any from prior agent run)
                let merge_wt = compute_merge_worktree_path(&project, task_id_str);
                let merge_wt_path = Path::new(&merge_wt);
                if merge_wt_path.exists() {
                    if let Err(e) = GitService::delete_worktree(repo_path, merge_wt_path).await {
                        tracing::warn!(error = %e, "Failed to clean up orphaned merge worktree (non-fatal)");
                    }
                }

                // Use target branch HEAD as the merge commit SHA
                let target_sha = GitService::get_branch_sha(repo_path, &target_branch).await
                    .unwrap_or_else(|_| source_sha.clone());

                if let Err(e) = complete_merge_internal(
                    &mut task,
                    &project,
                    &target_sha,
                    &target_branch,
                    task_repo,
                    self.machine.context.services.app_handle.as_ref(),
                )
                .await
                {
                    tracing::error!(error = %e, "Failed to complete already-merged task");
                } else {
                    self.post_merge_cleanup(task_id_str, &task_id, repo_path, plan_branch_repo)
                        .await;
                }
                return;
            }
        }

        // --- "Deleted source branch" recovery ---
        // If the source branch ref is gone but the task's commits are already on
        // the target branch (e.g. detached HEAD, premature cleanup), recover
        // by completing the merge instead of falling through to MergeIncomplete.
        if !GitService::branch_exists(repo_path, &source_branch).await {
            match GitService::find_commit_by_message_grep(repo_path, task_id_str, &target_branch).await {
                Ok(Some(found_sha)) => {
                    tracing::info!(
                        task_id = task_id_str,
                        source_branch = %source_branch,
                        target_branch = %target_branch,
                        found_sha = %found_sha,
                        "Source branch missing but task commits found on target — recovering"
                    );

                    // Clean up orphaned merge worktree (same as "already merged" path)
                    let merge_wt = compute_merge_worktree_path(&project, task_id_str);
                    let merge_wt_path = Path::new(&merge_wt);
                    if merge_wt_path.exists() {
                        if let Err(e) = GitService::delete_worktree(repo_path, merge_wt_path).await {
                            tracing::warn!(error = %e, "Failed to clean up orphaned merge worktree (non-fatal)");
                        }
                    }

                    let target_sha = GitService::get_branch_sha(repo_path, &target_branch).await
                        .unwrap_or_else(|_| found_sha.clone());

                    if let Err(e) = complete_merge_internal(
                        &mut task,
                        &project,
                        &target_sha,
                        &target_branch,
                        task_repo,
                        self.machine.context.services.app_handle.as_ref(),
                    )
                    .await
                    {
                        tracing::error!(error = %e, "Failed to complete merge for recovered task");
                    } else {
                        self.post_merge_cleanup(task_id_str, &task_id, repo_path, plan_branch_repo)
                            .await;
                    }
                    return;
                }
                Ok(None) => {
                    tracing::debug!(
                        task_id = task_id_str,
                        source_branch = %source_branch,
                        "Source branch missing, no task commits on target — falling through"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        task_id = task_id_str,
                        error = %e,
                        "Failed to search for task commits on target branch"
                    );
                }
            }
        }

        // Emit merge progress event
        emit_merge_progress(
            self.machine.context.services.app_handle.as_ref(),
            task_id_str,
            MergePhase::ProgrammaticMerge,
            MergePhaseStatus::Started,
            format!("Merging {} into {}", source_branch, target_branch),
        );

        tracing::info!(
            task_id = task_id_str,
            source_branch = %source_branch,
            target_branch = %target_branch,
            "Attempting programmatic merge (Phase 1)"
        );

        // --- Concurrent merge guard (worktree mode only) ---
        // In worktree mode, git only allows one worktree per branch. If another task
        // is already merging (PendingMerge or Merging) into the same target branch,
        // we must defer this task to avoid the "branch already checked out" error.
        // Priority: task that entered pending_merge first wins; later task gets deferred.
        // Tie-breaker: lexical task ID comparison for deterministic results.
        //
        // TOCTOU fix: acquire merge_lock before the check-and-set so two tasks
        // cannot both read "no blocker" simultaneously and both proceed to merge.
        // The guard is held until the deferred metadata is written (or cleared),
        // then dropped — either explicitly at the `return` site or at block end.
        {
            let _merge_guard = self.machine.context.services.merge_lock.lock().await;
            let all_tasks = task_repo
                .get_by_project(&project.id)
                .await
                .unwrap_or_default();
            let merge_states = [InternalStatus::PendingMerge, InternalStatus::Merging];

            // Get this task's pending_merge entry timestamp
            let this_pending_merge_at = task_repo
                .get_status_entered_at(&task.id, InternalStatus::PendingMerge)
                .await
                .unwrap_or(None);

            let blocking_task_info = {
                let mut blocker: Option<TaskId> = None;
                for other in &all_tasks {
                    // Skip self
                    if other.id == task.id {
                        continue;
                    }
                    // Only consider tasks in merge states
                    if !merge_states.contains(&other.internal_status) {
                        continue;
                    }
                    // Skip tasks that are themselves deferred
                    if has_merge_deferred_metadata(other) {
                        continue;
                    }
                    // Skip archived tasks — they are dead, will never complete
                    if other.archived_at.is_some() {
                        continue;
                    }
                    // Check if targeting the same branch
                    if !task_targets_branch(other, &project, plan_branch_repo, &target_branch).await
                    {
                        continue;
                    }

                    // Get other task's pending_merge entry timestamp
                    let other_pending_merge_at = task_repo
                        .get_status_entered_at(&other.id, InternalStatus::PendingMerge)
                        .await
                        .unwrap_or(None);

                    // Determine priority: earliest pending_merge entry wins
                    let should_defer = match (other_pending_merge_at, this_pending_merge_at) {
                        (Some(other_time), Some(this_time)) => {
                            // Both have timestamps - compare them
                            use std::cmp::Ordering;
                            match other_time.cmp(&this_time) {
                                Ordering::Less => true,
                                Ordering::Equal => {
                                    // Tie-breaker: lexical task ID comparison
                                    other.id.as_str() < task.id.as_str()
                                }
                                Ordering::Greater => false,
                            }
                        }
                        (Some(_), None) => {
                            // Other has timestamp, this doesn't - other wins
                            true
                        }
                        (None, Some(_)) => {
                            // This has timestamp, other doesn't - this wins
                            false
                        }
                        (None, None) => {
                            // Neither has timestamp - fallback to lexical ID comparison
                            other.id.as_str() < task.id.as_str()
                        }
                    };

                    if should_defer {
                        // Determine arbitration reason for structured logging
                        let reason = match (other_pending_merge_at, this_pending_merge_at) {
                            (Some(_), Some(_)) => "earlier_pending_merge_timestamp",
                            (Some(_), None) => "other_has_timestamp_this_missing",
                            (None, None) => "lexical_task_id_tiebreaker",
                            _ => "unknown",
                        };

                        tracing::info!(
                            event = "merge_arbitration_decision",
                            winner_task_id = other.id.as_str(),
                            loser_task_id = task_id_str,
                            winner_pending_merge_at = ?other_pending_merge_at,
                            loser_pending_merge_at = ?this_pending_merge_at,
                            target_branch = %target_branch,
                            reason = reason,
                            "Merge arbitration: deferring loser task"
                        );
                        blocker = Some(other.id.clone());
                        break;
                    }
                }
                blocker
            };

            let has_older_merge = blocking_task_info.is_some();

            if has_older_merge {
                // Set merge_deferred metadata and return early — task stays in PendingMerge
                let now = chrono::Utc::now().to_rfc3339();

                // Capture blocking task ID string for logging before move
                let blocking_task_id_str = blocking_task_info
                    .as_ref()
                    .map(|id| id.as_str().to_string());

                // Get or create merge recovery metadata
                let mut recovery =
                    MergeRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
                        .unwrap_or(None)
                        .unwrap_or_else(MergeRecoveryMetadata::new);

                // Create deferred event with blocker info
                let mut event = MergeRecoveryEvent::new(
                    MergeRecoveryEventKind::Deferred,
                    MergeRecoverySource::System,
                    MergeRecoveryReasonCode::TargetBranchBusy,
                    format!(
                        "Merge deferred: another task is merging to {}",
                        target_branch
                    ),
                )
                .with_target_branch(&target_branch)
                .with_source_branch(task.task_branch.as_deref().unwrap_or("unknown"));

                // Add blocking task ID if available
                if let Some(blocker_id) = blocking_task_info {
                    event = event.with_blocking_task(blocker_id);
                }

                // Append event and update state
                recovery.append_event_with_state(event, MergeRecoveryState::Deferred);

                // Update task metadata
                match recovery.update_task_metadata(task.metadata.as_deref()) {
                    Ok(updated_json) => {
                        task.metadata = Some(updated_json);
                    }
                    Err(e) => {
                        tracing::error!(
                            task_id = task_id_str,
                            error = %e,
                            "Failed to serialize merge recovery metadata, falling back to legacy"
                        );
                        // Fallback to legacy metadata
                        let mut meta =
                            parse_metadata(&task).unwrap_or_else(|| serde_json::json!({}));
                        if let Some(obj) = meta.as_object_mut() {
                            obj.insert("merge_deferred".to_string(), serde_json::json!(true));
                            obj.insert("merge_deferred_at".to_string(), serde_json::json!(now));
                        }
                        task.metadata = Some(meta.to_string());
                    }
                }

                task.touch();

                if let Err(e) = task_repo.update(&task).await {
                    tracing::error!(
                        task_id = task_id_str,
                        error = %e,
                        "Failed to update task with merge_deferred metadata"
                    );
                }

                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit_status_change(task_id_str, "pending_merge", "pending_merge")
                    .await;

                // Structured deferral event log
                tracing::info!(
                    event = "merge_deferred",
                    deferred_task_id = task_id_str,
                    blocking_task_id = blocking_task_id_str.as_deref().unwrap_or("unknown"),
                    target_branch = %target_branch,
                    reason_code = "target_branch_busy",
                    deferred_at = %now,
                    "Task merge deferred due to competing merge on same target branch"
                );

                return;
            }

            // If this task was previously deferred, log attempt_started and clear the flag
            if has_merge_deferred_metadata(&task) {
                // Get or create merge recovery metadata
                let mut recovery =
                    MergeRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
                        .unwrap_or(None)
                        .unwrap_or_else(MergeRecoveryMetadata::new);

                // Count previous attempts
                let attempt_count = recovery
                    .events
                    .iter()
                    .filter(|e| matches!(e.kind, MergeRecoveryEventKind::AttemptStarted))
                    .count() as u32
                    + 1;

                // Create attempt_started event
                let event = MergeRecoveryEvent::new(
                    MergeRecoveryEventKind::AttemptStarted,
                    MergeRecoverySource::Auto,
                    MergeRecoveryReasonCode::TargetBranchBusy,
                    format!("Starting merge attempt {} after deferral", attempt_count),
                )
                .with_target_branch(&target_branch)
                .with_source_branch(task.task_branch.as_deref().unwrap_or("unknown"))
                .with_attempt(attempt_count);

                // Append event (keeping state as Retrying)
                recovery.append_event(event);

                // Update task metadata
                match recovery.update_task_metadata(task.metadata.as_deref()) {
                    Ok(updated_json) => {
                        task.metadata = Some(updated_json);
                    }
                    Err(e) => {
                        tracing::error!(
                            task_id = task_id_str,
                            error = %e,
                            "Failed to serialize merge recovery metadata for attempt_started"
                        );
                    }
                }

                clear_merge_deferred_metadata(&mut task);
                task.touch();
                let _ = task_repo.update(&task).await;

                tracing::info!(
                    event = "merge_arbitration_winner_retry",
                    task_id = task_id_str,
                    target_branch = %target_branch,
                    attempt = attempt_count,
                    "Recorded attempt_started event for retrying merge"
                );
            }
        }

        // --- Overall merge deadline ---
        // Track wall-clock time so we can bail to MergeIncomplete if the entire
        // cleanup + strategy dispatch exceeds the configured deadline, rather than
        // leaving the task silently stuck in PendingMerge for 5+ minutes.
        let deadline_secs = reconciliation_config().attempt_merge_deadline_secs;
        let merge_deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(deadline_secs);

        // Run pre-merge cleanup unconditionally on every attempt (first or retry).
        // Removes stale worktrees, locks, and in-progress git operations from prior runs.
        self.pre_merge_cleanup(
            task_id_str,
            &task,
            &project,
            repo_path,
            &target_branch,
            task_repo,
        )
        .await;

        // Check deadline after cleanup (cleanup can take time if agents are being stopped)
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

        // Attempt the merge based on merge_strategy:
        // - Merge: merge in isolated worktree (or in-repo if target checked out)
        // - Rebase: rebase in worktree then merge (or in-repo if target checked out)
        // - Squash: squash merge in worktree (or in-repo if target checked out)
        // - RebaseSquash: rebase in worktree, then squash into single commit

        // Build commit message for squash merges.
        // For plan_merge tasks: use live session title + task enumeration.
        // For regular tasks: use category-derived commit type.
        let squash_commit_msg = if task.category == TaskCategory::PlanMerge {
            if let (Some(session_id), Some(task_repo), Some(session_repo)) = (
                task.ideation_session_id.as_ref(),
                self.machine.context.services.task_repo.as_deref(),
                self.machine.context.services.ideation_session_repo.as_deref(),
            ) {
                build_plan_merge_commit_msg(
                    session_id,
                    &source_branch,
                    task_repo,
                    session_repo,
                )
                .await
            } else {
                // Fallback: repos unavailable, use generic message
                tracing::warn!(
                    task_id = task_id_str,
                    has_session_id = task.ideation_session_id.is_some(),
                    has_task_repo = self.machine.context.services.task_repo.is_some(),
                    has_session_repo = self.machine.context.services.ideation_session_repo.is_some(),
                    "build_plan_merge_commit_msg: repos unavailable, using generic message"
                );
                format!("feat: {}\n\nPlan branch: {}", task.title, source_branch)
            }
        } else {
            build_squash_commit_msg(&task.category, &task.title, &source_branch)
        };
        tracing::info!(
            task_id = task_id_str,
            strategy = ?project.merge_strategy,
            source_branch = %source_branch,
            target_branch = %target_branch,
            "Dispatching merge strategy"
        );

        // Compute remaining time for the strategy dispatch
        let remaining = merge_deadline.saturating_duration_since(tokio::time::Instant::now());

        // Wrap strategy dispatch in a timeout using the remaining deadline budget.
        // If the merge strategy hangs, we transition to MergeIncomplete immediately
        // instead of leaving the task stuck in PendingMerge for the 5-minute stale threshold.
        let strategy_completed = tokio::time::timeout(remaining, async {
            match project.merge_strategy {
                MergeStrategy::Merge => {
                    let outcome = self.merge_worktree_strategy(
                        repo_path, &source_branch, &target_branch, &project, task_id_str,
                    ).await;
                    let opts = MergeHandlerOptions::merge();
                    self.handle_merge_outcome(
                        outcome, &mut task, &task_id, task_id_str,
                        &project, repo_path, &source_branch, &target_branch,
                        task_repo, plan_branch_repo, &opts,
                    ).await;
                }
                MergeStrategy::Rebase => {
                    let outcome = self.rebase_worktree_strategy(
                        repo_path, &source_branch, &target_branch, &project, task_id_str,
                    ).await;
                    let opts = MergeHandlerOptions::rebase();
                    self.handle_merge_outcome(
                        outcome, &mut task, &task_id, task_id_str,
                        &project, repo_path, &source_branch, &target_branch,
                        task_repo, plan_branch_repo, &opts,
                    ).await;
                }
                MergeStrategy::Squash => {
                    let outcome = self.squash_worktree_strategy(
                        repo_path, &source_branch, &target_branch, &squash_commit_msg, &project, task_id_str,
                    ).await;
                    let opts = MergeHandlerOptions::squash();
                    self.handle_merge_outcome(
                        outcome, &mut task, &task_id, task_id_str,
                        &project, repo_path, &source_branch, &target_branch,
                        task_repo, plan_branch_repo, &opts,
                    ).await;
                }
                MergeStrategy::RebaseSquash => {
                    let outcome = self.rebase_squash_worktree_strategy(
                        repo_path, &source_branch, &target_branch, &squash_commit_msg, &project, task_id_str,
                    ).await;
                    let opts = MergeHandlerOptions::rebase_squash();
                    self.handle_merge_outcome(
                        outcome, &mut task, &task_id, task_id_str,
                        &project, repo_path, &source_branch, &target_branch,
                        task_repo, plan_branch_repo, &opts,
                    ).await;
                }
            }
        }).await;

        if strategy_completed.is_err() {
            tracing::error!(
                task_id = task_id_str,
                deadline_secs = deadline_secs,
                "Programmatic merge exceeded deadline during strategy dispatch — transitioning to MergeIncomplete"
            );
            let metadata = serde_json::json!({
                "error": format!("Merge attempt timed out after {}s (strategy dispatch exceeded deadline)", deadline_secs),
                "source_branch": source_branch,
                "target_branch": target_branch,
                "strategy": format!("{:?}", project.merge_strategy),
            });
            self.transition_to_merge_incomplete(
                &mut task, &task_id, task_id_str, metadata, task_repo, true,
            ).await;
        }
    }

    /// Transition a task to MergeIncomplete with the given metadata JSON.
    ///
    /// Handles the full transition: update metadata -> persist status change -> emit event.
    /// Optionally triggers on_exit (needed when the caller wants deferred-merge retry).
    async fn transition_to_merge_incomplete(
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
        task.touch();

        if let Err(e) = task_repo.update(task).await {
            tracing::error!(error = %e, "Failed to update task to MergeIncomplete status");
            return;
        }

        if let Err(e) = task_repo
            .persist_status_change(
                task_id,
                InternalStatus::PendingMerge,
                InternalStatus::MergeIncomplete,
                "merge_incomplete",
            )
            .await
        {
            tracing::warn!(error = %e, "Failed to record merge_incomplete transition (non-fatal)");
        }

        self.machine
            .context
            .services
            .event_emitter
            .emit_status_change(task_id_str, "pending_merge", "merge_incomplete")
            .await;

        if trigger_on_exit {
            self.on_exit(&State::PendingMerge, &State::MergeIncomplete)
                .await;
        }
    }

    /// Pre-merge cleanup: remove debris from any prior failed attempts and stale locks.
    ///
    /// Runs unconditionally on EVERY merge attempt (first or retry) so that transient
    /// failures from a previous run never block the current one.
    ///
    /// Steps:
    ///   0. Stop any running agents (review/merge) and kill worktree processes
    ///   1. Remove stale `.git/index.lock`
    ///   2. Delete the task worktree to unlock the task branch
    ///   3. Prune stale worktree references
    ///   4. Delete own merge/rebase worktrees from a prior attempt
    ///   5. Scan and remove orphaned merge worktrees targeting the same branch
    ///   6. Clean the working tree (git clean)
    async fn pre_merge_cleanup(
        &self,
        task_id_str: &str,
        task: &crate::domain::entities::Task,
        project: &crate::domain::entities::Project,
        repo_path: &Path,
        target_branch: &str,
        task_repo: &Arc<dyn TaskRepository>,
    ) {
        // --- Step 0: Stop running agents and kill worktree processes ---
        // The reviewer (or merger from a prior attempt) may still be running IN the
        // task worktree. We must stop it BEFORE attempting worktree deletion to avoid
        // git lock contention that causes the 5+ minute hang (see merge-hang RCA).
        tracing::info!(task_id = task_id_str, "pre_merge_cleanup: step 0 — stopping any running agents");
        for ctx_type in [
            crate::domain::entities::ChatContextType::Review,
            crate::domain::entities::ChatContextType::Merge,
        ] {
            match self
                .machine
                .context
                .services
                .chat_service
                .stop_agent(ctx_type, task_id_str)
                .await
            {
                Ok(true) => {
                    tracing::info!(
                        task_id = task_id_str,
                        context_type = ?ctx_type,
                        "Stopped running agent before merge cleanup"
                    );
                }
                Ok(false) => {}
                Err(e) => {
                    tracing::warn!(
                        task_id = task_id_str,
                        context_type = ?ctx_type,
                        error = %e,
                        "Failed to stop agent (non-fatal)"
                    );
                }
            }
        }
        // Kill any lingering processes with files open in the task worktree
        if let Some(ref worktree_path) = task.worktree_path {
            let worktree_path_buf = PathBuf::from(worktree_path);
            if worktree_path_buf.exists() {
                crate::domain::services::kill_worktree_processes(&worktree_path_buf);
            }
        }
        // Brief settle time for process tree cleanup after SIGTERM
        tokio::time::sleep(std::time::Duration::from_secs(AGENT_KILL_SETTLE_SECS)).await;

        // --- Step 1: Remove stale index.lock ---
        tracing::info!(task_id = task_id_str, "pre_merge_cleanup: step 1 — removing stale index.lock");
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

        {
            // --- Step 2: Delete task worktree ---
            tracing::info!(task_id = task_id_str, "pre_merge_cleanup: step 2 — deleting task worktree");
            if let Some(ref worktree_path) = task.worktree_path {
                let worktree_path_buf = PathBuf::from(worktree_path);
                if worktree_path_buf.exists() {
                    tracing::info!(
                        task_id = task_id_str,
                        worktree_path = %worktree_path,
                        "Deleting task worktree before programmatic merge to unlock branch"
                    );
                    run_cleanup_step(
                        "step 2 task worktree deletion",
                        CLEANUP_TASK_WORKTREE_TIMEOUT_SECS,
                        task_id_str,
                        GitService::delete_worktree(repo_path, &worktree_path_buf),
                    )
                    .await;
                }
            }

            // --- Step 3: Prune stale worktree refs ---
            tracing::info!(task_id = task_id_str, "pre_merge_cleanup: step 3 — pruning stale worktree refs");
            if let Err(e) = GitService::prune_worktrees(repo_path).await {
                tracing::warn!(
                    task_id = task_id_str,
                    error = %e,
                    "Failed to prune stale worktrees (non-fatal)"
                );
            }

            // --- Step 4: Delete own stale merge/rebase worktrees ---
            tracing::info!(task_id = task_id_str, "pre_merge_cleanup: step 4 — deleting own stale merge/rebase worktrees");
            for (wt_label, own_wt) in [
                ("merge", compute_merge_worktree_path(project, task_id_str)),
                ("rebase", compute_rebase_worktree_path(project, task_id_str)),
            ] {
                let own_wt_path = PathBuf::from(&own_wt);
                if own_wt_path.exists() {
                    tracing::info!(
                        task_id = task_id_str,
                        worktree_path = %own_wt,
                        "Cleaning up stale {} worktree from previous attempt",
                        wt_label
                    );
                    run_cleanup_step(
                        &format!("step 4 {} worktree deletion", wt_label),
                        CLEANUP_GIT_OP_TIMEOUT_SECS,
                        task_id_str,
                        GitService::delete_worktree(repo_path, &own_wt_path),
                    )
                    .await;
                }
            }

            // --- Step 5: Scan for orphaned merge worktrees ---
            tracing::info!(task_id = task_id_str, "pre_merge_cleanup: step 5 — scanning for orphaned merge worktrees");
            let worktrees_result = tokio::time::timeout(
                std::time::Duration::from_secs(CLEANUP_GIT_OP_TIMEOUT_SECS),
                GitService::list_worktrees(repo_path),
            )
            .await;
            match worktrees_result {
                Ok(Ok(worktrees)) => {
                    for wt in &worktrees {
                        let Some(other_task_id) = extract_task_id_from_merge_path(&wt.path) else {
                            continue;
                        };
                        if other_task_id == task_id_str {
                            continue;
                        }
                        let wt_branch = wt.branch.as_deref().unwrap_or("");
                        if wt_branch != target_branch {
                            continue;
                        }
                        if is_task_in_merge_workflow(task_repo, other_task_id).await {
                            tracing::info!(
                                task_id = task_id_str,
                                other_task_id = other_task_id,
                                worktree_path = %wt.path,
                                "Skipping merge worktree cleanup — owning task is still in merge workflow"
                            );
                            continue;
                        }
                        tracing::info!(
                            task_id = task_id_str,
                            other_task_id = other_task_id,
                            worktree_path = %wt.path,
                            target_branch = %target_branch,
                            "Cleaning up orphaned merge worktree from non-active task"
                        );
                        let orphan_path = PathBuf::from(&wt.path);
                        if let Err(e) = GitService::delete_worktree(repo_path, &orphan_path).await {
                            tracing::warn!(
                                task_id = task_id_str,
                                other_task_id = other_task_id,
                                error = %e,
                                worktree_path = %wt.path,
                                "Failed to delete orphaned merge worktree (non-fatal)"
                            );
                        }
                    }
                }
                Ok(Err(e)) => {
                    tracing::warn!(
                        task_id = task_id_str,
                        error = %e,
                        "Failed to list worktrees for orphan scan (non-fatal)"
                    );
                }
                Err(_elapsed) => {
                    tracing::warn!(
                        task_id = task_id_str,
                        timeout_secs = CLEANUP_GIT_OP_TIMEOUT_SECS,
                        "pre_merge_cleanup: step 5 worktree list timed out (non-fatal)"
                    );
                }
            }
        }

        // --- Step 6: Clean working tree ---
        tracing::info!(task_id = task_id_str, "pre_merge_cleanup: step 6 — cleaning working tree (git clean)");
        run_cleanup_step(
            "step 6 git clean",
            CLEANUP_GIT_OP_TIMEOUT_SECS,
            task_id_str,
            GitService::clean_working_tree(repo_path),
        )
        .await;
        tracing::info!(task_id = task_id_str, "pre_merge_cleanup: complete");
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

        // Unblock tasks that were waiting on this task to merge.
        // complete_merge_internal bypasses TransitionHandler (raw task_repo.update),
        // so on_enter(Merged) never fires and unblock_dependents is never called.
        // This is the canonical unblock call for the programmatic merge path.
        self.machine
            .context
            .services
            .dependency_manager
            .unblock_dependents(task_id_str)
            .await;

        // Schedule newly-unblocked tasks (e.g. plan_merge tasks that just became Ready).
        // Without this, unblocked tasks rely on the ReadyWatchdog (60s interval) to be
        // scheduled, causing up to 90s delay. This mirrors on_enter(Merged) in on_enter_states.rs.
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
    /// The merge commit has already landed on the target branch. We must revert it
    /// before transitioning so that failing code doesn't remain on the target branch.
    ///
    /// # Arguments
    /// * `task` - Mutable task to update
    /// * `task_id` - Task ID for persistence
    /// * `task_id_str` - Task ID string for logging
    /// * `task_repo` - Repository for persisting status change
    /// * `failures` - Validation failures to include in metadata
    /// * `source_branch` / `target_branch` - For metadata
    /// * `merge_path` - Path where the merge happened (for git reset)
    /// * `mode_label` - Label for log messages (e.g., "in-repo", "worktree", "local")
    /// * `validation_mode` - Current validation mode (AutoFix spawns agent, Block reverts)
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

            task.metadata = Some(
                serde_json::json!({
                    "validation_recovery": true,
                    "validation_failures": failure_details,
                    "validation_log": log,
                    "source_branch": source_branch,
                    "target_branch": target_branch,
                })
                .to_string(),
            );
            // Set worktree_path to the merge worktree so the merger agent CWD resolves correctly
            task.worktree_path = Some(merge_path.to_string_lossy().to_string());
            task.internal_status = InternalStatus::Merging;
            task.touch();

            let _ = task_repo.update(task).await;
            let _ = task_repo
                .persist_status_change(
                    task_id,
                    InternalStatus::PendingMerge,
                    InternalStatus::Merging,
                    "validation_auto_fix",
                )
                .await;

            self.machine
                .context
                .services
                .event_emitter
                .emit_status_change(task_id_str, "pending_merge", "merging")
                .await;

            // Spawn merger agent to attempt fix (same pattern as conflict resolution)
            let prompt = format!("Fix validation failures for task: {}", task_id_str);
            tracing::info!(
                task_id = task_id_str,
                "Spawning merger agent for validation recovery"
            );

            let result = self
                .machine
                .context
                .services
                .chat_service
                .send_message(
                    crate::domain::entities::ChatContextType::Merge,
                    task_id_str,
                    &prompt,
                )
                .await;

            match &result {
                Ok(_) => tracing::info!(
                    task_id = task_id_str,
                    "Merger agent spawned for validation recovery"
                ),
                Err(e) => {
                    tracing::error!(task_id = task_id_str, error = %e, "Failed to spawn merger agent for validation recovery")
                }
            }
        } else {
            // Block mode: revert merge and transition to MergeIncomplete
            tracing::warn!(
                task_id = task_id_str,
                failure_count = failures.len(),
                "Post-merge validation failed ({}), reverting merge and transitioning to MergeIncomplete",
                mode_label,
            );

            // Revert the merge commit so failing code doesn't remain on the target branch
            if let Err(e) = GitService::reset_hard(merge_path, "HEAD~1").await {
                tracing::error!(
                    task_id = task_id_str,
                    error = %e,
                    "Failed to revert merge commit after validation failure — target branch may have failing code"
                );
            }

            // Track revert count for loop-breaking: increment existing counter.
            // After >2 reverts due to validation failure, reconciler will stop auto-retrying.
            let prev_revert_count: u32 = task
                .metadata
                .as_deref()
                .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
                .and_then(|v| v.get("validation_revert_count").and_then(|c| c.as_u64()).map(|c| c as u32))
                .unwrap_or(0);
            let revert_count = prev_revert_count + 1;

            let base_metadata = format_validation_error_metadata(failures, log, source_branch, target_branch);
            // Merge base metadata with revert tracking fields
            let final_metadata = if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&base_metadata) {
                if let Some(obj) = v.as_object_mut() {
                    obj.insert(
                        "merge_failure_source".to_string(),
                        serde_json::to_value(MergeFailureSource::ValidationFailed)
                            .unwrap_or(serde_json::json!("validation_failed")),
                    );
                    obj.insert("validation_revert_count".to_string(), serde_json::json!(revert_count));
                }
                v.to_string()
            } else {
                base_metadata
            };

            task.metadata = Some(final_metadata);
            task.internal_status = InternalStatus::MergeIncomplete;
            task.touch();

            let _ = task_repo.update(task).await;
            let _ = task_repo
                .persist_status_change(
                    task_id,
                    InternalStatus::PendingMerge,
                    InternalStatus::MergeIncomplete,
                    "validation_failed",
                )
                .await;

            self.machine
                .context
                .services
                .event_emitter
                .emit_status_change(task_id_str, "pending_merge", "merge_incomplete")
                .await;
        }
    }
}
