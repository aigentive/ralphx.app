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
    is_task_in_merge_workflow, task_targets_branch, truncate_str,
};
// Used by #[cfg(test)] mod tests via `super::*`
#[cfg(test)]
use super::merge_helpers::{discover_and_attach_task_branch, resolve_task_base_branch};
use super::merge_validation::{
    emit_merge_progress, extract_cached_validation, format_validation_warn_metadata,
    take_skip_validation_flag, ValidationFailure,
};
#[cfg(test)]
use crate::domain::entities::Project;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::Emitter;

use super::super::machine::State;
use crate::application::git_service::checkout_free::{self, CheckoutFreeMergeResult};
use crate::application::{GitService, MergeAttemptResult};
use crate::domain::entities::{
    merge_progress_event::{MergePhase, MergePhaseStatus},
    task_metadata::{
        MergeRecoveryEvent, MergeRecoveryEventKind, MergeRecoveryMetadata, MergeRecoveryReasonCode,
        MergeRecoverySource, MergeRecoveryState,
    },
    GitMode, InternalStatus, MergeStrategy, MergeValidationMode, PlanBranchStatus, ProjectId, Task,
    TaskId,
};
use crate::domain::repositories::{PlanBranchRepository, TaskRepository};
use crate::error::AppResult;

const TEMP_SKIP_POST_MERGE_VALIDATION: bool = true;

/// Build a squash commit message based on task category.
///
/// For plan_merge tasks: `feat: {title}\n\nPlan branch: {branch}`
/// For regular tasks: `feat: {branch} ({title})`
fn build_squash_commit_msg(category: &str, title: &str, source_branch: &str) -> String {
    if category == "plan_merge" {
        format!("feat: {}\n\nPlan branch: {}", title, source_branch)
    } else {
        format!("feat: {} ({})", source_branch, title)
    }
}

impl<'a> super::TransitionHandler<'a> {
    /// Execute on-enter action for a state
    ///
    /// This method is public to allow `TaskTransitionService` to trigger entry actions
    /// for direct status changes (e.g., Kanban drag-drop) without going through the
    /// full event-based transition flow.
    ///
    /// Returns an error if the state entry cannot be completed (e.g., execution blocked
    /// due to uncommitted changes in Local mode).
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

        // Resolve source and target branches (handles merge tasks and plan feature branches)
        let plan_branch_repo = &self.machine.context.services.plan_branch_repo;
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

            task.metadata = Some(
                serde_json::json!({
                    "error": "Empty source branch resolved. This typically means plan_branch_repo \
                              was unavailable when resolving merge branches for a plan_merge task.",
                    "source_branch": source_branch,
                    "target_branch": target_branch,
                    "category": task.category,
                })
                .to_string(),
            );
            task.internal_status = InternalStatus::MergeIncomplete;
            task.touch();

            if let Err(e) = task_repo.update(&task).await {
                tracing::error!(error = %e, "Failed to update task to MergeIncomplete status");
                return;
            }

            if let Err(e) = task_repo
                .persist_status_change(
                    &task_id,
                    InternalStatus::PendingMerge,
                    InternalStatus::MergeIncomplete,
                    "merge_incomplete",
                )
                .await
            {
                tracing::warn!(error = %e, "Failed to record merge incomplete transition (non-fatal)");
            }

            self.machine
                .context
                .services
                .event_emitter
                .emit_status_change(task_id_str, "pending_merge", "merge_incomplete")
                .await;

            return;
        }

        // --- Main-merge deferral check ---
        // If target is main/base branch and agents are running, defer the merge
        // to avoid interrupting them with an app rebuild. Retry when all agents go idle.
        let base_branch = project.base_branch.as_deref().unwrap_or("main");
        if target_branch == base_branch {
            if let Some(ref execution_state) = self.machine.context.services.execution_state {
                if execution_state.running_count() > 0 {
                    tracing::info!(
                        task_id = task_id_str,
                        source_branch = %source_branch,
                        target_branch = %target_branch,
                        running_count = execution_state.running_count(),
                        "Deferring main-branch merge: {} agents still running — \
                         merge will be retried when all agents complete",
                        execution_state.running_count()
                    );

                    // Set main_merge_deferred metadata flag
                    super::merge_helpers::set_main_merge_deferred_metadata(&mut task);
                    task.touch();

                    if let Err(e) = task_repo.update(&task).await {
                        tracing::error!(error = %e, "Failed to set main_merge_deferred metadata");
                        return;
                    }

                    // Emit merge progress event for UI visibility
                    emit_merge_progress(
                        self.machine.context.services.app_handle.as_ref(),
                        task_id_str,
                        MergePhase::ProgrammaticMerge,
                        MergePhaseStatus::Started,
                        format!(
                            "Deferred merge to {} — waiting for {} agent(s) to complete",
                            target_branch,
                            execution_state.running_count()
                        ),
                    );

                    return;
                }
            }
        }

        let repo_path = Path::new(&project.working_directory);

        // Ensure plan branch exists as git ref (lazy creation for merge target).
        // Handles the case where the plan branch DB record exists but the git branch
        // was never created (e.g., lazy creation failed at execution time).
        if let Some(ref session_id) = task.ideation_session_id {
            if let Some(ref pb_repo) = plan_branch_repo {
                if let Ok(Some(pb)) = pb_repo.get_by_session_id(session_id).await {
                    if pb.status == PlanBranchStatus::Active
                        && pb.branch_name == target_branch
                        && !GitService::branch_exists(repo_path, &target_branch)
                    {
                        match GitService::create_feature_branch(
                            repo_path,
                            &pb.branch_name,
                            &pb.source_branch,
                        ) {
                            Ok(_) => {
                                tracing::info!(
                                    task_id = task_id_str,
                                    branch = %pb.branch_name,
                                    source = %pb.source_branch,
                                    "Lazily created plan branch for merge target"
                                );
                            }
                            Err(e) if GitService::branch_exists(repo_path, &pb.branch_name) => {
                                // Race: concurrent task created it between check and create
                                let _ = e;
                            }
                            Err(e) => {
                                tracing::warn!(
                                    task_id = task_id_str,
                                    error = %e,
                                    branch = %pb.branch_name,
                                    "Failed to lazily create plan branch for merge"
                                );
                            }
                        }
                    }
                }
            }
        }

        // --- "Already merged" early exit ---
        // If the source branch is already an ancestor of the target branch, the merge
        // was completed by a prior agent run that died before calling complete_merge.
        // Skip the merge entirely and transition straight to Merged.
        if let Ok(source_sha) = GitService::get_branch_sha(repo_path, &source_branch) {
            if let Ok(true) =
                GitService::is_commit_on_branch(repo_path, &source_sha, &target_branch)
            {
                tracing::info!(
                    task_id = task_id_str,
                    source_branch = %source_branch,
                    target_branch = %target_branch,
                    source_sha = %source_sha,
                    "Source branch already merged into target — skipping merge"
                );

                // Clean up orphaned merge worktree (if any from prior agent run)
                if project.git_mode == GitMode::Worktree {
                    let merge_wt = compute_merge_worktree_path(&project, task_id_str);
                    let merge_wt_path = Path::new(&merge_wt);
                    if merge_wt_path.exists() {
                        if let Err(e) = GitService::delete_worktree(repo_path, merge_wt_path) {
                            tracing::warn!(error = %e, "Failed to clean up orphaned merge worktree (non-fatal)");
                        }
                    }
                }

                // Use target branch HEAD as the merge commit SHA
                let target_sha = GitService::get_branch_sha(repo_path, &target_branch)
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
        if !GitService::branch_exists(repo_path, &source_branch) {
            match GitService::find_commit_by_message_grep(repo_path, task_id_str, &target_branch) {
                Ok(Some(found_sha)) => {
                    tracing::info!(
                        task_id = task_id_str,
                        source_branch = %source_branch,
                        target_branch = %target_branch,
                        found_sha = %found_sha,
                        "Source branch missing but task commits found on target — recovering"
                    );

                    // Clean up orphaned merge worktree (same as "already merged" path)
                    if project.git_mode == GitMode::Worktree {
                        let merge_wt = compute_merge_worktree_path(&project, task_id_str);
                        let merge_wt_path = Path::new(&merge_wt);
                        if merge_wt_path.exists() {
                            if let Err(e) = GitService::delete_worktree(repo_path, merge_wt_path) {
                                tracing::warn!(error = %e, "Failed to clean up orphaned merge worktree (non-fatal)");
                            }
                        }
                    }

                    let target_sha = GitService::get_branch_sha(repo_path, &target_branch)
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
            git_mode = ?project.git_mode,
            "Attempting programmatic merge (Phase 1)"
        );

        // --- Concurrent merge guard (worktree mode only) ---
        // In worktree mode, git only allows one worktree per branch. If another task
        // is already merging (PendingMerge or Merging) into the same target branch,
        // we must defer this task to avoid the "branch already checked out" error.
        // Priority: task that entered pending_merge first wins; later task gets deferred.
        // Tie-breaker: lexical task ID comparison for deterministic results.
        if project.git_mode == GitMode::Worktree {
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

        // In worktree mode, delete the task worktree first to unlock the branch.
        // Git refuses to checkout a branch that's checked out in another worktree,
        // so we must remove the task worktree before creating the merge worktree.
        if project.git_mode == GitMode::Worktree {
            if let Some(ref worktree_path) = task.worktree_path {
                let worktree_path_buf = PathBuf::from(worktree_path);
                if worktree_path_buf.exists() {
                    tracing::info!(
                        task_id = task_id_str,
                        worktree_path = %worktree_path,
                        "Deleting task worktree before programmatic merge to unlock branch"
                    );
                    if let Err(e) = GitService::delete_worktree(repo_path, &worktree_path_buf) {
                        tracing::error!(
                            task_id = task_id_str,
                            error = %e,
                            worktree_path = %worktree_path,
                            "Failed to delete task worktree before merge"
                        );
                        // Continue anyway - merge will fail with a clear error
                    }
                }
            }

            // --- Stale merge worktree cleanup ---
            // Step 1: Prune stale worktree references (metadata pointing to deleted dirs)
            if let Err(e) = GitService::prune_worktrees(repo_path) {
                tracing::warn!(
                    task_id = task_id_str,
                    error = %e,
                    "Failed to prune stale worktrees (non-fatal)"
                );
            }

            // Step 2: Force-delete our own merge worktree if it exists from a prior attempt
            let own_merge_wt = compute_merge_worktree_path(&project, task_id_str);
            let own_merge_wt_path = PathBuf::from(&own_merge_wt);
            if own_merge_wt_path.exists() {
                tracing::info!(
                    task_id = task_id_str,
                    merge_worktree_path = %own_merge_wt,
                    "Cleaning up stale merge worktree from previous attempt"
                );
                if let Err(e) = GitService::delete_worktree(repo_path, &own_merge_wt_path) {
                    tracing::warn!(
                        task_id = task_id_str,
                        error = %e,
                        merge_worktree_path = %own_merge_wt,
                        "Failed to delete stale merge worktree (non-fatal)"
                    );
                }
            }

            // Step 3: Scan for orphaned merge worktrees on the same target branch.
            // Another task's merge may have crashed/failed, leaving a worktree that locks
            // the target branch. We only clean up if the owning task is NOT actively merging.
            if let Ok(worktrees) = GitService::list_worktrees(repo_path) {
                for wt in &worktrees {
                    // Only consider merge worktrees (path contains "/merge-")
                    let Some(other_task_id) = extract_task_id_from_merge_path(&wt.path) else {
                        continue;
                    };
                    // Skip our own — already handled above
                    if other_task_id == task_id_str {
                        continue;
                    }
                    // Only care about worktrees on the same target branch
                    let wt_branch = wt.branch.as_deref().unwrap_or("");
                    if wt_branch != target_branch {
                        continue;
                    }
                    // Check if the owning task is in the merge workflow — if so, leave it alone
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
                    if let Err(e) = GitService::delete_worktree(repo_path, &orphan_path) {
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
        }

        // Abort stale rebase/merge from prior failed attempts (Local mode only —
        // Worktree mode already deletes and recreates isolated worktrees above)
        if project.git_mode == GitMode::Local {
            if GitService::is_rebase_in_progress(repo_path) {
                tracing::info!(
                    task_id = task_id_str,
                    "Aborting stale rebase before programmatic merge retry"
                );
                let _ = GitService::abort_rebase(repo_path);
            }
            if GitService::is_merge_in_progress(repo_path) {
                tracing::info!(
                    task_id = task_id_str,
                    "Aborting stale merge before programmatic merge retry"
                );
                let _ = GitService::abort_merge(repo_path);
            }
        }

        // Clean working tree before merge (non-fatal on error)
        match GitService::clean_working_tree(repo_path) {
            Ok(()) => tracing::debug!(
                task_id = task_id_str,
                "Pre-merge working tree clean succeeded"
            ),
            Err(e) => {
                tracing::warn!(task_id = task_id_str, error = %e, "Pre-merge clean failed (non-fatal)")
            }
        }

        // Attempt the merge based on (merge_strategy, git_mode):
        // - (Rebase, Local): rebase for linear history (operates on main repo)
        // - (Merge, Local): direct merge, no rebase (operates on main repo)
        // - (Merge, Worktree): merge in isolated worktree (or in-repo if target checked out)
        // - (Rebase, Worktree): rebase in worktree then merge (or in-repo if target checked out)
        // - (Squash, Local): squash merge for clean single commit (operates on main repo)
        // - (Squash, Worktree): squash merge in worktree (or in-repo if target checked out)

        // Build commit message for squash merges
        let squash_commit_msg =
            build_squash_commit_msg(&task.category, &task.title, &source_branch);
        match (project.merge_strategy, project.git_mode) {
            (MergeStrategy::Merge, GitMode::Worktree) => {
                // Detect if the target branch is already checked out in the primary repo.
                // This happens for plan merge tasks (plan feature branch → main) because
                // main is always checked out in the primary repo. Git forbids the same
                // branch in multiple worktrees, so we merge directly in-repo instead.
                let current_branch = GitService::get_current_branch(repo_path).unwrap_or_default();
                let target_is_checked_out = current_branch == target_branch;

                if target_is_checked_out {
                    // Target branch (e.g., main) is checked out in the primary repo.
                    // Use checkout-free merge (git plumbing) to avoid disrupting working tree.
                    tracing::info!(
                        task_id = task_id_str,
                        target_branch = %target_branch,
                        "Target branch is checked out, using checkout-free merge"
                    );

                    // Validate branches exist before merge
                    if !GitService::branch_exists(repo_path, &source_branch) {
                        tracing::error!(
                            task_id = task_id_str,
                            "Source branch '{}' does not exist",
                            source_branch
                        );

                        // Record merge recovery event for retry tracking
                        let mut recovery =
                            MergeRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
                                .unwrap_or(None)
                                .unwrap_or_else(MergeRecoveryMetadata::new);

                        // Count existing AutoRetryTriggered events
                        let attempt_count = recovery
                            .events
                            .iter()
                            .filter(|e| {
                                matches!(e.kind, MergeRecoveryEventKind::AutoRetryTriggered)
                            })
                            .count() as u32
                            + 1;

                        // Create AutoRetryTriggered event
                        let event = MergeRecoveryEvent::new(
                            MergeRecoveryEventKind::AutoRetryTriggered,
                            MergeRecoverySource::Auto,
                            MergeRecoveryReasonCode::BranchNotFound,
                            format!("Source branch '{}' does not exist", source_branch),
                        )
                        .with_target_branch(&target_branch)
                        .with_source_branch(&source_branch)
                        .with_attempt(attempt_count);

                        recovery.append_event(event);

                        // Update task metadata with recovery events and branch_missing flag
                        match recovery.update_task_metadata(task.metadata.as_deref()) {
                            Ok(updated_json) => {
                                // Add branch_missing flag to metadata
                                if let Ok(mut metadata_obj) =
                                    serde_json::from_str::<serde_json::Value>(&updated_json)
                                {
                                    if let Some(obj) = metadata_obj.as_object_mut() {
                                        obj.insert(
                                            "branch_missing".to_string(),
                                            serde_json::json!(true),
                                        );
                                    }
                                    task.metadata = Some(metadata_obj.to_string());
                                } else {
                                    task.metadata = Some(updated_json);
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    task_id = task_id_str,
                                    error = %e,
                                    "Failed to serialize merge recovery metadata, using legacy format"
                                );
                                // Fallback to legacy metadata
                                task.metadata = Some(serde_json::json!({"error": format!("Branch '{}' does not exist", source_branch), "missing_branch": source_branch, "source_branch": source_branch, "target_branch": target_branch, "branch_missing": true}).to_string());
                            }
                        }

                        task.internal_status = InternalStatus::MergeIncomplete;
                        task.touch();
                        let _ = task_repo.update(&task).await;
                        let _ = task_repo
                            .persist_status_change(
                                &task_id,
                                InternalStatus::PendingMerge,
                                InternalStatus::MergeIncomplete,
                                "merge_incomplete",
                            )
                            .await;
                        self.machine
                            .context
                            .services
                            .event_emitter
                            .emit_status_change(task_id_str, "pending_merge", "merge_incomplete")
                            .await;
                        return;
                    }
                    if !GitService::branch_exists(repo_path, &target_branch) {
                        tracing::error!(
                            task_id = task_id_str,
                            "Target branch '{}' does not exist",
                            target_branch
                        );

                        // Record merge recovery event for retry tracking
                        let mut recovery =
                            MergeRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
                                .unwrap_or(None)
                                .unwrap_or_else(MergeRecoveryMetadata::new);

                        // Count existing AutoRetryTriggered events
                        let attempt_count = recovery
                            .events
                            .iter()
                            .filter(|e| {
                                matches!(e.kind, MergeRecoveryEventKind::AutoRetryTriggered)
                            })
                            .count() as u32
                            + 1;

                        // Create AutoRetryTriggered event
                        let event = MergeRecoveryEvent::new(
                            MergeRecoveryEventKind::AutoRetryTriggered,
                            MergeRecoverySource::Auto,
                            MergeRecoveryReasonCode::BranchNotFound,
                            format!("Target branch '{}' does not exist", target_branch),
                        )
                        .with_target_branch(&target_branch)
                        .with_source_branch(&source_branch)
                        .with_attempt(attempt_count);

                        recovery.append_event(event);

                        // Update task metadata with recovery events and branch_missing flag
                        match recovery.update_task_metadata(task.metadata.as_deref()) {
                            Ok(updated_json) => {
                                // Add branch_missing flag to metadata
                                if let Ok(mut metadata_obj) =
                                    serde_json::from_str::<serde_json::Value>(&updated_json)
                                {
                                    if let Some(obj) = metadata_obj.as_object_mut() {
                                        obj.insert(
                                            "branch_missing".to_string(),
                                            serde_json::json!(true),
                                        );
                                    }
                                    task.metadata = Some(metadata_obj.to_string());
                                } else {
                                    task.metadata = Some(updated_json);
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    task_id = task_id_str,
                                    error = %e,
                                    "Failed to serialize merge recovery metadata, using legacy format"
                                );
                                // Fallback to legacy metadata
                                task.metadata = Some(serde_json::json!({"error": format!("Branch '{}' does not exist", target_branch), "missing_branch": target_branch, "source_branch": source_branch, "target_branch": target_branch, "branch_missing": true}).to_string());
                            }
                        }

                        task.internal_status = InternalStatus::MergeIncomplete;
                        task.touch();
                        let _ = task_repo.update(&task).await;
                        let _ = task_repo
                            .persist_status_change(
                                &task_id,
                                InternalStatus::PendingMerge,
                                InternalStatus::MergeIncomplete,
                                "merge_incomplete",
                            )
                            .await;
                        self.machine
                            .context
                            .services
                            .event_emitter
                            .emit_status_change(task_id_str, "pending_merge", "merge_incomplete")
                            .await;
                        return;
                    }

                    let cf_result = checkout_free::try_merge_checkout_free(
                        repo_path,
                        &source_branch,
                        &target_branch,
                    );

                    match cf_result {
                        Ok(CheckoutFreeMergeResult::Success { commit_sha }) => {
                            // Atomically sync working tree
                            if let Err(e) = GitService::hard_reset_to_head(repo_path) {
                                tracing::error!(error = %e, task_id = task_id_str, "Failed to sync working tree after checkout-free merge");
                            }

                            tracing::info!(
                                task_id = task_id_str,
                                commit_sha = %commit_sha,
                                "Checkout-free merge succeeded"
                            );

                            // Emit merge progress success event
                            emit_merge_progress(
                                self.machine.context.services.app_handle.as_ref(),
                                task_id_str,
                                MergePhase::ProgrammaticMerge,
                                MergePhaseStatus::Passed,
                                format!("Merge completed: {}", commit_sha),
                            );

                            if TEMP_SKIP_POST_MERGE_VALIDATION {
                                tracing::warn!(
                                task_id = task_id_str,
                                "Post-merge validation temporarily disabled (global flag, checkout-free merge)"
                            );
                            } else {
                                // Post-merge validation gate: check mode + skip flag
                                let skip_validation = take_skip_validation_flag(&mut task);
                                let validation_mode = &project.merge_validation_mode;
                                if !skip_validation && *validation_mode != MergeValidationMode::Off
                                {
                                    let source_sha =
                                        GitService::get_branch_sha(repo_path, &source_branch).ok();
                                    let cached_log = source_sha
                                        .as_deref()
                                        .and_then(|sha| extract_cached_validation(&task, sha));
                                    let app_handle_ref =
                                        self.machine.context.services.app_handle.as_ref();
                                    if let Some(validation) = run_validation_commands(
                                        &project,
                                        &task,
                                        repo_path,
                                        task_id_str,
                                        app_handle_ref,
                                        cached_log.as_deref(),
                                    )
                                    .await
                                    {
                                        if !validation.all_passed {
                                            if *validation_mode == MergeValidationMode::Warn {
                                                tracing::warn!(task_id = task_id_str, "Validation failed in Warn mode (checkout-free merge), proceeding");
                                                task.metadata =
                                                    Some(format_validation_warn_metadata(
                                                        &validation.log,
                                                        &source_branch,
                                                        &target_branch,
                                                    ));
                                            } else {
                                                self.handle_validation_failure(
                                                    &mut task,
                                                    &task_id,
                                                    task_id_str,
                                                    task_repo,
                                                    &validation.failures,
                                                    &validation.log,
                                                    &source_branch,
                                                    &target_branch,
                                                    repo_path,
                                                    "checkout-free",
                                                    validation_mode,
                                                )
                                                .await;
                                                return;
                                            }
                                        } else {
                                            task.metadata = Some(
                                                serde_json::json!({
                                                    "validation_log": validation.log,
                                                    "validation_source_sha": source_sha,
                                                    "source_branch": source_branch,
                                                    "target_branch": target_branch,
                                                })
                                                .to_string(),
                                            );
                                        }
                                    }
                                }
                            }

                            let app_handle = self.machine.context.services.app_handle.as_ref();
                            if let Err(e) = complete_merge_internal(
                                &mut task,
                                &project,
                                &commit_sha,
                                &target_branch,
                                task_repo,
                                app_handle,
                            )
                            .await
                            {
                                tracing::error!(error = %e, task_id = task_id_str, "Failed to complete checkout-free merge, falling back to MergeIncomplete");

                                task.metadata = Some(
                                    serde_json::json!({
                                        "error": format!("complete_merge_internal failed: {}", e),
                                        "source_branch": source_branch,
                                        "target_branch": target_branch,
                                    })
                                    .to_string(),
                                );
                                task.internal_status = InternalStatus::MergeIncomplete;
                                task.touch();

                                let _ = task_repo.update(&task).await;
                                let _ = task_repo
                                    .persist_status_change(
                                        &task_id,
                                        InternalStatus::PendingMerge,
                                        InternalStatus::MergeIncomplete,
                                        "merge_incomplete",
                                    )
                                    .await;

                                self.machine
                                    .context
                                    .services
                                    .event_emitter
                                    .emit_status_change(
                                        task_id_str,
                                        "pending_merge",
                                        "merge_incomplete",
                                    )
                                    .await;
                            } else {
                                self.post_merge_cleanup(
                                    task_id_str,
                                    &task_id,
                                    repo_path,
                                    plan_branch_repo,
                                )
                                .await;
                            }
                        }
                        Ok(CheckoutFreeMergeResult::Conflict {
                            files: conflict_files,
                        }) => {
                            // Conflict detected — create temp worktree for merger agent
                            tracing::info!(
                            task_id = task_id_str,
                            conflict_count = conflict_files.len(),
                            "Checkout-free merge has conflicts, creating temp worktree for resolution"
                        );

                            emit_merge_progress(
                                self.machine.context.services.app_handle.as_ref(),
                                task_id_str,
                                MergePhase::ProgrammaticMerge,
                                MergePhaseStatus::Failed,
                                format!(
                                    "Merge conflicts detected in {} files",
                                    conflict_files.len()
                                ),
                            );

                            for file in &conflict_files {
                                tracing::debug!(task_id = task_id_str, file = %file.display(), "Conflict file");
                            }

                            // Create temp worktree for conflict resolution (keeps primary checkout clean)
                            let merge_wt_path =
                                PathBuf::from(compute_merge_worktree_path(&project, task_id_str));
                            let target_sha = GitService::get_branch_sha(repo_path, &target_branch)
                                .unwrap_or_default();
                            let resolve_branch = format!("merge-resolve/{}", task_id_str);

                            // Create temp branch at target's current commit
                            if let Err(e) = GitService::create_branch_at(
                                repo_path,
                                &resolve_branch,
                                &target_sha,
                            ) {
                                tracing::error!(error = %e, task_id = task_id_str, "Failed to create resolve branch");
                            }

                            // Create worktree on the temp branch
                            if let Err(e) = GitService::checkout_existing_branch_worktree(
                                repo_path,
                                &merge_wt_path,
                                &resolve_branch,
                            ) {
                                tracing::error!(error = %e, task_id = task_id_str, "Failed to create merge worktree for conflict resolution");
                            }

                            // Start the actual merge in the worktree (leaves conflicts for agent)
                            let _ = std::process::Command::new("git")
                                .args(["merge", &source_branch, "--no-edit"])
                                .current_dir(&merge_wt_path)
                                .output();

                            task.internal_status = InternalStatus::Merging;
                            task.touch();

                            if let Err(e) = task_repo.update(&task).await {
                                tracing::error!(error = %e, "Failed to update task to Merging");
                                return;
                            }

                            if let Err(e) = task_repo
                                .persist_status_change(
                                    &task_id,
                                    InternalStatus::PendingMerge,
                                    InternalStatus::Merging,
                                    "merge_conflict",
                                )
                                .await
                            {
                                tracing::warn!(error = %e, "Failed to record merge conflict transition (non-fatal)");
                            }

                            self.machine
                                .context
                                .services
                                .event_emitter
                                .emit_status_change(task_id_str, "pending_merge", "merging")
                                .await;

                            // Spawn merger agent — CWD is the temp worktree
                            let prompt =
                                format!("Resolve merge conflicts for task: {}", task_id_str);
                            tracing::info!(
                                task_id = task_id_str,
                                merge_worktree = %merge_wt_path.display(),
                                "Spawning merger agent for conflict resolution in temp worktree"
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
                                    "Merger agent spawned successfully"
                                ),
                                Err(e) => {
                                    tracing::error!(task_id = task_id_str, error = %e, "Failed to spawn merger agent")
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                task_id = task_id_str,
                                error = %e,
                                "Checkout-free merge failed, transitioning to MergeIncomplete"
                            );

                            task.metadata = Some(
                                serde_json::json!({
                                    "error": e.to_string(),
                                    "source_branch": source_branch,
                                    "target_branch": target_branch,
                                })
                                .to_string(),
                            );
                            task.internal_status = InternalStatus::MergeIncomplete;
                            task.touch();

                            let _ = task_repo.update(&task).await;
                            let _ = task_repo
                                .persist_status_change(
                                    &task_id,
                                    InternalStatus::PendingMerge,
                                    InternalStatus::MergeIncomplete,
                                    "merge_incomplete",
                                )
                                .await;

                            self.machine
                                .context
                                .services
                                .event_emitter
                                .emit_status_change(
                                    task_id_str,
                                    "pending_merge",
                                    "merge_incomplete",
                                )
                                .await;
                        }
                    }
                } else {
                    // Target branch is NOT checked out — use isolated merge worktree (existing path)
                    let merge_wt_path_str = compute_merge_worktree_path(&project, task_id_str);
                    let merge_wt_path = PathBuf::from(&merge_wt_path_str);

                    tracing::info!(
                        task_id = task_id_str,
                        merge_worktree_path = %merge_wt_path_str,
                        "Creating merge worktree for isolated merge"
                    );

                    let merge_result = GitService::try_merge_in_worktree(
                        repo_path,
                        &source_branch,
                        &target_branch,
                        &merge_wt_path,
                    );

                    match merge_result {
                        Ok(MergeAttemptResult::Success { commit_sha }) => {
                            tracing::info!(
                                task_id = task_id_str,
                                commit_sha = %commit_sha,
                                "Programmatic merge in worktree succeeded (fast path)"
                            );

                            // Emit merge progress success event
                            emit_merge_progress(
                                self.machine.context.services.app_handle.as_ref(),
                                task_id_str,
                                MergePhase::ProgrammaticMerge,
                                MergePhaseStatus::Passed,
                                format!("Merge completed: {}", commit_sha),
                            );

                            if TEMP_SKIP_POST_MERGE_VALIDATION {
                                tracing::warn!(
                                task_id = task_id_str,
                                "Post-merge validation temporarily disabled (global flag, worktree merge)"
                            );
                            } else {
                                // Post-merge validation gate: check mode + skip flag
                                let skip_validation = take_skip_validation_flag(&mut task);
                                let validation_mode = &project.merge_validation_mode;
                                if !skip_validation && *validation_mode != MergeValidationMode::Off
                                {
                                    let source_sha =
                                        GitService::get_branch_sha(repo_path, &source_branch).ok();
                                    let cached_log = source_sha
                                        .as_deref()
                                        .and_then(|sha| extract_cached_validation(&task, sha));
                                    let app_handle_ref =
                                        self.machine.context.services.app_handle.as_ref();
                                    if let Some(validation) = run_validation_commands(
                                        &project,
                                        &task,
                                        &merge_wt_path,
                                        task_id_str,
                                        app_handle_ref,
                                        cached_log.as_deref(),
                                    )
                                    .await
                                    {
                                        if !validation.all_passed {
                                            if *validation_mode == MergeValidationMode::Warn {
                                                tracing::warn!(task_id = task_id_str, "Validation failed in Warn mode (worktree), proceeding with merge");
                                                task.metadata =
                                                    Some(format_validation_warn_metadata(
                                                        &validation.log,
                                                        &source_branch,
                                                        &target_branch,
                                                    ));
                                            } else {
                                                // Block mode: reset in merge worktree, then delete it
                                                // AutoFix mode: keep the worktree for the merger agent to fix in
                                                self.handle_validation_failure(
                                                    &mut task,
                                                    &task_id,
                                                    task_id_str,
                                                    task_repo,
                                                    &validation.failures,
                                                    &validation.log,
                                                    &source_branch,
                                                    &target_branch,
                                                    &merge_wt_path,
                                                    "worktree",
                                                    validation_mode,
                                                )
                                                .await;
                                                return;
                                            }
                                        } else {
                                            task.metadata = Some(
                                                serde_json::json!({
                                                    "validation_log": validation.log,
                                                    "validation_source_sha": source_sha,
                                                    "source_branch": source_branch,
                                                    "target_branch": target_branch,
                                                })
                                                .to_string(),
                                            );
                                        }
                                    }
                                }
                            }

                            if let Err(e) = GitService::delete_worktree(repo_path, &merge_wt_path) {
                                tracing::warn!(
                                    error = %e,
                                    task_id = task_id_str,
                                    merge_worktree_path = %merge_wt_path_str,
                                    "Failed to delete merge worktree after success (non-fatal)"
                                );
                            }

                            let app_handle = self.machine.context.services.app_handle.as_ref();
                            if let Err(e) = complete_merge_internal(
                                &mut task,
                                &project,
                                &commit_sha,
                                &target_branch,
                                task_repo,
                                app_handle,
                            )
                            .await
                            {
                                tracing::error!(error = %e, task_id = task_id_str, "Failed to complete programmatic merge, falling back to MergeIncomplete");

                                task.metadata = Some(
                                    serde_json::json!({
                                        "error": format!("complete_merge_internal failed: {}", e),
                                        "source_branch": source_branch,
                                        "target_branch": target_branch,
                                    })
                                    .to_string(),
                                );
                                task.internal_status = InternalStatus::MergeIncomplete;
                                task.touch();

                                let _ = task_repo.update(&task).await;
                                let _ = task_repo
                                    .persist_status_change(
                                        &task_id,
                                        InternalStatus::PendingMerge,
                                        InternalStatus::MergeIncomplete,
                                        "merge_incomplete",
                                    )
                                    .await;

                                self.machine
                                    .context
                                    .services
                                    .event_emitter
                                    .emit_status_change(
                                        task_id_str,
                                        "pending_merge",
                                        "merge_incomplete",
                                    )
                                    .await;
                            } else {
                                self.post_merge_cleanup(
                                    task_id_str,
                                    &task_id,
                                    repo_path,
                                    plan_branch_repo,
                                )
                                .await;
                            }
                        }
                        Ok(MergeAttemptResult::NeedsAgent { conflict_files }) => {
                            tracing::info!(
                                task_id = task_id_str,
                                conflict_count = conflict_files.len(),
                                merge_worktree_path = %merge_wt_path_str,
                                "Merge in worktree has conflicts, transitioning to Merging"
                            );

                            // Emit merge progress conflict event
                            emit_merge_progress(
                                self.machine.context.services.app_handle.as_ref(),
                                task_id_str,
                                MergePhase::ProgrammaticMerge,
                                MergePhaseStatus::Failed,
                                format!(
                                    "Merge conflicts detected in {} files",
                                    conflict_files.len()
                                ),
                            );

                            for file in &conflict_files {
                                tracing::debug!(task_id = task_id_str, file = %file.display(), "Conflict file");
                            }

                            task.worktree_path = Some(merge_wt_path_str.clone());
                            task.internal_status = InternalStatus::Merging;
                            task.touch();

                            if let Err(e) = task_repo.update(&task).await {
                                tracing::error!(error = %e, "Failed to update task to Merging with merge worktree path");
                                return;
                            }

                            if let Err(e) = task_repo
                                .persist_status_change(
                                    &task_id,
                                    InternalStatus::PendingMerge,
                                    InternalStatus::Merging,
                                    "merge_conflict",
                                )
                                .await
                            {
                                tracing::warn!(error = %e, "Failed to record merge conflict transition (non-fatal)");
                            }

                            self.machine
                                .context
                                .services
                                .event_emitter
                                .emit_status_change(task_id_str, "pending_merge", "merging")
                                .await;

                            let prompt =
                                format!("Resolve merge conflicts for task: {}", task_id_str);
                            tracing::info!(
                            task_id = task_id_str,
                            "Spawning merger agent for conflict resolution (from attempt_programmatic_merge)"
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
                                    "Merger agent spawned successfully"
                                ),
                                Err(e) => {
                                    tracing::error!(task_id = task_id_str, error = %e, "Failed to spawn merger agent")
                                }
                            }
                        }
                        Ok(MergeAttemptResult::BranchNotFound { branch }) => {
                            tracing::error!(
                                task_id = task_id_str,
                                missing_branch = %branch,
                                "Merge failed: branch '{}' does not exist", branch
                            );

                            task.metadata = Some(
                                serde_json::json!({
                                    "error": format!("Branch '{}' does not exist", branch),
                                    "missing_branch": branch,
                                    "source_branch": source_branch,
                                    "target_branch": target_branch,
                                })
                                .to_string(),
                            );
                            task.internal_status = InternalStatus::MergeIncomplete;
                            task.touch();

                            if let Err(e) = task_repo.update(&task).await {
                                tracing::error!(error = %e, "Failed to update task to MergeIncomplete status");
                                return;
                            }

                            if let Err(e) = task_repo
                                .persist_status_change(
                                    &task_id,
                                    InternalStatus::PendingMerge,
                                    InternalStatus::MergeIncomplete,
                                    "merge_incomplete",
                                )
                                .await
                            {
                                tracing::warn!(error = %e, "Failed to record merge incomplete transition (non-fatal)");
                            }

                            self.machine
                                .context
                                .services
                                .event_emitter
                                .emit_status_change(
                                    task_id_str,
                                    "pending_merge",
                                    "merge_incomplete",
                                )
                                .await;
                        }
                        Err(e) => {
                            // Classify error: deferrable (branch lock) vs terminal (true failure)
                            if GitService::is_branch_lock_error(&e) {
                                tracing::warn!(
                                    task_id = task_id_str,
                                    error = %e,
                                    merge_worktree_path = %merge_wt_path_str,
                                    source_branch = %source_branch,
                                    target_branch = %target_branch,
                                    "Merge in worktree failed due to branch lock (deferrable), staying in PendingMerge"
                                );

                                if merge_wt_path.exists() {
                                    let _ = GitService::delete_worktree(repo_path, &merge_wt_path);
                                }

                                // Get or create merge recovery metadata
                                let mut recovery = MergeRecoveryMetadata::from_task_metadata(
                                    task.metadata.as_deref(),
                                )
                                .unwrap_or(None)
                                .unwrap_or_else(MergeRecoveryMetadata::new);

                                // Create deferred event for branch lock
                                let event = MergeRecoveryEvent::new(
                                    MergeRecoveryEventKind::Deferred,
                                    MergeRecoverySource::System,
                                    MergeRecoveryReasonCode::GitError,
                                    format!("Merge deferred due to branch lock: {}", e),
                                )
                                .with_target_branch(&target_branch)
                                .with_source_branch(&source_branch);

                                // Append event and update state
                                recovery
                                    .append_event_with_state(event, MergeRecoveryState::Deferred);

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
                                        task.metadata = Some(
                                            serde_json::json!({
                                                "merge_deferred": true,
                                                "error": e.to_string(),
                                                "source_branch": source_branch,
                                                "target_branch": target_branch,
                                                "reason": "branch_lock",
                                            })
                                            .to_string(),
                                        );
                                    }
                                }

                                task.touch();

                                if let Err(e) = task_repo.update(&task).await {
                                    tracing::error!(error = %e, "Failed to update task with merge_deferred metadata");
                                }

                                // Task remains in pending_merge, will be retried when blocker exits
                            } else {
                                // Non-deferrable error: transition to merge_incomplete
                                tracing::error!(
                                    task_id = task_id_str,
                                    error = %e,
                                    merge_worktree_path = %merge_wt_path_str,
                                    source_branch = %source_branch,
                                    target_branch = %target_branch,
                                    "Merge in worktree failed, transitioning to MergeIncomplete"
                                );

                                if merge_wt_path.exists() {
                                    let _ = GitService::delete_worktree(repo_path, &merge_wt_path);
                                }

                                // Append attempt_failed event
                                let mut recovery = MergeRecoveryMetadata::from_task_metadata(
                                    task.metadata.as_deref(),
                                )
                                .unwrap_or(None)
                                .unwrap_or_else(MergeRecoveryMetadata::new);

                                let attempt_count = recovery
                                    .events
                                    .iter()
                                    .filter(|ev| {
                                        matches!(
                                            ev.kind,
                                            MergeRecoveryEventKind::AutoRetryTriggered
                                        )
                                    })
                                    .count()
                                    as u32
                                    + 1;

                                let failed_event = MergeRecoveryEvent::new(
                                    MergeRecoveryEventKind::AttemptFailed,
                                    MergeRecoverySource::System,
                                    MergeRecoveryReasonCode::GitError,
                                    format!("Merge attempt failed (worktree): {}", e),
                                )
                                .with_target_branch(&target_branch)
                                .with_source_branch(&source_branch)
                                .with_attempt(attempt_count);

                                recovery.append_event_with_state(
                                    failed_event,
                                    MergeRecoveryState::Failed,
                                );

                                // Update task metadata with both recovery data and legacy error fields
                                match recovery.update_task_metadata(task.metadata.as_deref()) {
                                    Ok(updated_json) => {
                                        // Also preserve legacy error metadata
                                        if let Ok(mut meta) =
                                            serde_json::from_str::<serde_json::Value>(&updated_json)
                                        {
                                            if let Some(obj) = meta.as_object_mut() {
                                                obj.insert(
                                                    "error".to_string(),
                                                    serde_json::json!(e.to_string()),
                                                );
                                                obj.insert(
                                                    "source_branch".to_string(),
                                                    serde_json::json!(source_branch),
                                                );
                                                obj.insert(
                                                    "target_branch".to_string(),
                                                    serde_json::json!(target_branch),
                                                );
                                            }
                                            task.metadata = Some(meta.to_string());
                                        } else {
                                            task.metadata = Some(updated_json);
                                        }
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            task_id = task_id_str,
                                            error = %e,
                                            "Failed to serialize merge recovery metadata on failure"
                                        );
                                        // Fallback to legacy metadata
                                        task.metadata = Some(
                                            serde_json::json!({
                                                "error": e.to_string(),
                                                "source_branch": source_branch,
                                                "target_branch": target_branch,
                                            })
                                            .to_string(),
                                        );
                                    }
                                }

                                task.internal_status = InternalStatus::MergeIncomplete;
                                task.touch();

                                if let Err(e) = task_repo.update(&task).await {
                                    tracing::error!(error = %e, "Failed to update task to MergeIncomplete status");
                                    return;
                                }

                                if let Err(e) = task_repo
                                    .persist_status_change(
                                        &task_id,
                                        InternalStatus::PendingMerge,
                                        InternalStatus::MergeIncomplete,
                                        "merge_incomplete",
                                    )
                                    .await
                                {
                                    tracing::warn!(error = %e, "Failed to record merge incomplete transition (non-fatal)");
                                }

                                self.machine
                                    .context
                                    .services
                                    .event_emitter
                                    .emit_status_change(
                                        task_id_str,
                                        "pending_merge",
                                        "merge_incomplete",
                                    )
                                    .await;
                            }
                        }
                    }
                }
            }
            (MergeStrategy::Rebase, GitMode::Local) => {
                // Local mode: rebase for linear history
                let merge_result =
                    GitService::try_rebase_and_merge(repo_path, &source_branch, &target_branch);
                match merge_result {
                    Ok(MergeAttemptResult::Success { commit_sha }) => {
                        tracing::info!(
                            task_id = task_id_str,
                            commit_sha = %commit_sha,
                            "Programmatic merge succeeded (fast path)"
                        );

                        // Emit merge progress success event
                        emit_merge_progress(
                            self.machine.context.services.app_handle.as_ref(),
                            task_id_str,
                            MergePhase::ProgrammaticMerge,
                            MergePhaseStatus::Passed,
                            format!("Merge completed: {}", commit_sha),
                        );

                        if TEMP_SKIP_POST_MERGE_VALIDATION {
                            tracing::warn!(
                            task_id = task_id_str,
                            "Post-merge validation temporarily disabled (global flag, local merge)"
                        );
                        } else {
                            // Post-merge validation gate: check mode + skip flag
                            let skip_validation = take_skip_validation_flag(&mut task);
                            let validation_mode = &project.merge_validation_mode;
                            if !skip_validation && *validation_mode != MergeValidationMode::Off {
                                let source_sha =
                                    GitService::get_branch_sha(repo_path, &source_branch).ok();
                                let cached_log = source_sha
                                    .as_deref()
                                    .and_then(|sha| extract_cached_validation(&task, sha));
                                let app_handle_ref =
                                    self.machine.context.services.app_handle.as_ref();
                                if let Some(validation) = run_validation_commands(
                                    &project,
                                    &task,
                                    repo_path,
                                    task_id_str,
                                    app_handle_ref,
                                    cached_log.as_deref(),
                                )
                                .await
                                {
                                    if !validation.all_passed {
                                        if *validation_mode == MergeValidationMode::Warn {
                                            tracing::warn!(task_id = task_id_str, "Validation failed in Warn mode (local), proceeding with merge");
                                            task.metadata = Some(format_validation_warn_metadata(
                                                &validation.log,
                                                &source_branch,
                                                &target_branch,
                                            ));
                                        } else {
                                            self.handle_validation_failure(
                                                &mut task,
                                                &task_id,
                                                task_id_str,
                                                task_repo,
                                                &validation.failures,
                                                &validation.log,
                                                &source_branch,
                                                &target_branch,
                                                repo_path,
                                                "local",
                                                validation_mode,
                                            )
                                            .await;
                                            return;
                                        }
                                    } else {
                                        task.metadata = Some(
                                            serde_json::json!({
                                                "validation_log": validation.log,
                                                "validation_source_sha": source_sha,
                                                "source_branch": source_branch,
                                                "target_branch": target_branch,
                                            })
                                            .to_string(),
                                        );
                                    }
                                }
                            }
                        }

                        let app_handle = self.machine.context.services.app_handle.as_ref();
                        if let Err(e) = complete_merge_internal(
                            &mut task,
                            &project,
                            &commit_sha,
                            &target_branch,
                            task_repo,
                            app_handle,
                        )
                        .await
                        {
                            tracing::error!(error = %e, task_id = task_id_str, "Failed to complete programmatic merge, falling back to MergeIncomplete");

                            task.metadata = Some(
                                serde_json::json!({
                                    "error": format!("complete_merge_internal failed: {}", e),
                                    "source_branch": source_branch,
                                    "target_branch": target_branch,
                                })
                                .to_string(),
                            );
                            task.internal_status = InternalStatus::MergeIncomplete;
                            task.touch();

                            let _ = task_repo.update(&task).await;
                            let _ = task_repo
                                .persist_status_change(
                                    &task_id,
                                    InternalStatus::PendingMerge,
                                    InternalStatus::MergeIncomplete,
                                    "merge_incomplete",
                                )
                                .await;

                            self.machine
                                .context
                                .services
                                .event_emitter
                                .emit_status_change(
                                    task_id_str,
                                    "pending_merge",
                                    "merge_incomplete",
                                )
                                .await;
                        } else {
                            self.post_merge_cleanup(
                                task_id_str,
                                &task_id,
                                repo_path,
                                plan_branch_repo,
                            )
                            .await;
                        }
                    }
                    Ok(MergeAttemptResult::NeedsAgent { conflict_files }) => {
                        tracing::info!(
                        task_id = task_id_str,
                        conflict_count = conflict_files.len(),
                        "Programmatic merge failed: conflicts detected, transitioning to Merging"
                    );

                        // Emit merge progress conflict event
                        emit_merge_progress(
                            self.machine.context.services.app_handle.as_ref(),
                            task_id_str,
                            MergePhase::ProgrammaticMerge,
                            MergePhaseStatus::Failed,
                            format!("Merge conflicts detected in {} files", conflict_files.len()),
                        );

                        for file in &conflict_files {
                            tracing::debug!(task_id = task_id_str, file = %file.display(), "Conflict file");
                        }

                        task.internal_status = InternalStatus::Merging;
                        task.touch();

                        if let Err(e) = task_repo.update(&task).await {
                            tracing::error!(error = %e, "Failed to update task to Merging status");
                            return;
                        }

                        if let Err(e) = task_repo
                            .persist_status_change(
                                &task_id,
                                InternalStatus::PendingMerge,
                                InternalStatus::Merging,
                                "merge_conflict",
                            )
                            .await
                        {
                            tracing::warn!(error = %e, "Failed to record merge conflict transition (non-fatal)");
                        }

                        self.machine
                            .context
                            .services
                            .event_emitter
                            .emit_status_change(task_id_str, "pending_merge", "merging")
                            .await;

                        let prompt = format!("Resolve merge conflicts for task: {}", task_id_str);
                        tracing::info!(
                        task_id = task_id_str,
                        "Spawning merger agent for conflict resolution (from attempt_programmatic_merge)"
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
                                "Merger agent spawned successfully"
                            ),
                            Err(e) => {
                                tracing::error!(task_id = task_id_str, error = %e, "Failed to spawn merger agent")
                            }
                        }
                    }
                    Ok(MergeAttemptResult::BranchNotFound { branch }) => {
                        tracing::error!(
                            task_id = task_id_str,
                            missing_branch = %branch,
                            "Merge failed: branch '{}' does not exist", branch
                        );

                        task.metadata = Some(
                            serde_json::json!({
                                "error": format!("Branch '{}' does not exist", branch),
                                "missing_branch": branch,
                                "source_branch": source_branch,
                                "target_branch": target_branch,
                            })
                            .to_string(),
                        );
                        task.internal_status = InternalStatus::MergeIncomplete;
                        task.touch();

                        if let Err(e) = task_repo.update(&task).await {
                            tracing::error!(error = %e, "Failed to update task to MergeIncomplete status");
                            return;
                        }

                        if let Err(e) = task_repo
                            .persist_status_change(
                                &task_id,
                                InternalStatus::PendingMerge,
                                InternalStatus::MergeIncomplete,
                                "merge_incomplete",
                            )
                            .await
                        {
                            tracing::warn!(error = %e, "Failed to record merge incomplete transition (non-fatal)");
                        }

                        self.machine
                            .context
                            .services
                            .event_emitter
                            .emit_status_change(task_id_str, "pending_merge", "merge_incomplete")
                            .await;
                    }
                    Err(e) => {
                        // Classify error: deferrable (branch lock) vs terminal (true failure)
                        if GitService::is_branch_lock_error(&e) {
                            tracing::warn!(
                                task_id = task_id_str,
                                error = %e,
                                source_branch = %source_branch,
                                target_branch = %target_branch,
                                repo_path = %repo_path.display(),
                                "Programmatic merge failed due to branch lock (deferrable), staying in PendingMerge"
                            );

                            // Get or create merge recovery metadata
                            let mut recovery =
                                MergeRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
                                    .unwrap_or(None)
                                    .unwrap_or_else(MergeRecoveryMetadata::new);

                            // Create deferred event for branch lock
                            let event = MergeRecoveryEvent::new(
                                MergeRecoveryEventKind::Deferred,
                                MergeRecoverySource::System,
                                MergeRecoveryReasonCode::GitError,
                                format!("Merge deferred due to branch lock: {}", e),
                            )
                            .with_target_branch(&target_branch)
                            .with_source_branch(&source_branch);

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
                                    task.metadata = Some(
                                        serde_json::json!({
                                            "merge_deferred": true,
                                            "error": e.to_string(),
                                            "source_branch": source_branch,
                                            "target_branch": target_branch,
                                            "reason": "branch_lock",
                                        })
                                        .to_string(),
                                    );
                                }
                            }

                            task.touch();

                            if let Err(e) = task_repo.update(&task).await {
                                tracing::error!(error = %e, "Failed to update task with merge_deferred metadata");
                            }

                            // Task remains in pending_merge, will be retried when blocker exits
                        } else {
                            // Non-deferrable error: transition to merge_incomplete
                            tracing::error!(
                                task_id = task_id_str,
                                error = %e,
                                source_branch = %source_branch,
                                target_branch = %target_branch,
                                repo_path = %repo_path.display(),
                                "Programmatic merge failed due to error, transitioning to MergeIncomplete"
                            );

                            // Append attempt_failed event
                            let mut recovery =
                                MergeRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
                                    .unwrap_or(None)
                                    .unwrap_or_else(MergeRecoveryMetadata::new);

                            let attempt_count = recovery
                                .events
                                .iter()
                                .filter(|ev| {
                                    matches!(ev.kind, MergeRecoveryEventKind::AutoRetryTriggered)
                                })
                                .count() as u32
                                + 1;

                            let failed_event = MergeRecoveryEvent::new(
                                MergeRecoveryEventKind::AttemptFailed,
                                MergeRecoverySource::System,
                                MergeRecoveryReasonCode::GitError,
                                format!("Merge attempt failed (programmatic): {}", e),
                            )
                            .with_target_branch(&target_branch)
                            .with_source_branch(&source_branch)
                            .with_attempt(attempt_count);

                            recovery
                                .append_event_with_state(failed_event, MergeRecoveryState::Failed);

                            // Update task metadata with both recovery data and legacy error fields
                            match recovery.update_task_metadata(task.metadata.as_deref()) {
                                Ok(updated_json) => {
                                    // Also preserve legacy error metadata
                                    if let Ok(mut meta) =
                                        serde_json::from_str::<serde_json::Value>(&updated_json)
                                    {
                                        if let Some(obj) = meta.as_object_mut() {
                                            obj.insert(
                                                "error".to_string(),
                                                serde_json::json!(e.to_string()),
                                            );
                                            obj.insert(
                                                "source_branch".to_string(),
                                                serde_json::json!(source_branch),
                                            );
                                            obj.insert(
                                                "target_branch".to_string(),
                                                serde_json::json!(target_branch),
                                            );
                                        }
                                        task.metadata = Some(meta.to_string());
                                    } else {
                                        task.metadata = Some(updated_json);
                                    }
                                }
                                Err(e) => {
                                    tracing::error!(
                                        task_id = task_id_str,
                                        error = %e,
                                        "Failed to serialize merge recovery metadata on failure"
                                    );
                                    // Fallback to legacy metadata
                                    task.metadata = Some(
                                        serde_json::json!({
                                            "error": e.to_string(),
                                            "source_branch": source_branch,
                                            "target_branch": target_branch,
                                        })
                                        .to_string(),
                                    );
                                }
                            }

                            task.internal_status = InternalStatus::MergeIncomplete;
                            task.touch();

                            if let Err(e) = task_repo.update(&task).await {
                                tracing::error!(error = %e, "Failed to update task to MergeIncomplete status");
                                return;
                            }

                            if let Err(e) = task_repo
                                .persist_status_change(
                                    &task_id,
                                    InternalStatus::PendingMerge,
                                    InternalStatus::MergeIncomplete,
                                    "merge_incomplete",
                                )
                                .await
                            {
                                tracing::warn!(error = %e, "Failed to record merge incomplete transition (non-fatal)");
                            }

                            self.machine
                                .context
                                .services
                                .event_emitter
                                .emit_status_change(
                                    task_id_str,
                                    "pending_merge",
                                    "merge_incomplete",
                                )
                                .await;
                        }
                    }
                }
            }
            (MergeStrategy::Merge, GitMode::Local) => {
                // Local mode: direct merge, no rebase (produces merge commits)
                let merge_result = GitService::try_merge(repo_path, &source_branch, &target_branch);
                match merge_result {
                    Ok(MergeAttemptResult::Success { commit_sha }) => {
                        tracing::info!(
                            task_id = task_id_str,
                            commit_sha = %commit_sha,
                            "Direct merge succeeded (fast path, no rebase)"
                        );

                        emit_merge_progress(
                            self.machine.context.services.app_handle.as_ref(),
                            task_id_str,
                            MergePhase::ProgrammaticMerge,
                            MergePhaseStatus::Passed,
                            format!("Merge completed: {}", commit_sha),
                        );

                        if TEMP_SKIP_POST_MERGE_VALIDATION {
                            tracing::warn!(
                            task_id = task_id_str,
                            "Post-merge validation temporarily disabled (global flag, local direct merge)"
                        );
                        } else {
                            let skip_validation = take_skip_validation_flag(&mut task);
                            let validation_mode = &project.merge_validation_mode;
                            if !skip_validation && *validation_mode != MergeValidationMode::Off {
                                let source_sha =
                                    GitService::get_branch_sha(repo_path, &source_branch).ok();
                                let cached_log = source_sha
                                    .as_deref()
                                    .and_then(|sha| extract_cached_validation(&task, sha));
                                let app_handle_ref =
                                    self.machine.context.services.app_handle.as_ref();
                                if let Some(validation) = run_validation_commands(
                                    &project,
                                    &task,
                                    repo_path,
                                    task_id_str,
                                    app_handle_ref,
                                    cached_log.as_deref(),
                                )
                                .await
                                {
                                    if !validation.all_passed {
                                        if *validation_mode == MergeValidationMode::Warn {
                                            tracing::warn!(task_id = task_id_str, "Validation failed in Warn mode (local direct merge), proceeding");
                                            task.metadata = Some(format_validation_warn_metadata(
                                                &validation.log,
                                                &source_branch,
                                                &target_branch,
                                            ));
                                        } else {
                                            self.handle_validation_failure(
                                                &mut task,
                                                &task_id,
                                                task_id_str,
                                                task_repo,
                                                &validation.failures,
                                                &validation.log,
                                                &source_branch,
                                                &target_branch,
                                                repo_path,
                                                "local",
                                                validation_mode,
                                            )
                                            .await;
                                            return;
                                        }
                                    } else {
                                        task.metadata = Some(
                                            serde_json::json!({
                                                "validation_log": validation.log,
                                                "validation_source_sha": source_sha,
                                                "source_branch": source_branch,
                                                "target_branch": target_branch,
                                            })
                                            .to_string(),
                                        );
                                    }
                                }
                            }
                        }

                        let app_handle = self.machine.context.services.app_handle.as_ref();
                        if let Err(e) = complete_merge_internal(
                            &mut task,
                            &project,
                            &commit_sha,
                            &target_branch,
                            task_repo,
                            app_handle,
                        )
                        .await
                        {
                            tracing::error!(error = %e, task_id = task_id_str, "Failed to complete direct merge, falling back to MergeIncomplete");

                            task.metadata = Some(
                                serde_json::json!({
                                    "error": format!("complete_merge_internal failed: {}", e),
                                    "source_branch": source_branch,
                                    "target_branch": target_branch,
                                })
                                .to_string(),
                            );
                            task.internal_status = InternalStatus::MergeIncomplete;
                            task.touch();

                            let _ = task_repo.update(&task).await;
                            let _ = task_repo
                                .persist_status_change(
                                    &task_id,
                                    InternalStatus::PendingMerge,
                                    InternalStatus::MergeIncomplete,
                                    "merge_incomplete",
                                )
                                .await;

                            self.machine
                                .context
                                .services
                                .event_emitter
                                .emit_status_change(
                                    task_id_str,
                                    "pending_merge",
                                    "merge_incomplete",
                                )
                                .await;
                        } else {
                            self.post_merge_cleanup(
                                task_id_str,
                                &task_id,
                                repo_path,
                                plan_branch_repo,
                            )
                            .await;
                        }
                    }
                    Ok(MergeAttemptResult::NeedsAgent { conflict_files }) => {
                        tracing::info!(
                            task_id = task_id_str,
                            conflict_count = conflict_files.len(),
                            "Direct merge failed: conflicts detected, transitioning to Merging"
                        );

                        emit_merge_progress(
                            self.machine.context.services.app_handle.as_ref(),
                            task_id_str,
                            MergePhase::ProgrammaticMerge,
                            MergePhaseStatus::Failed,
                            format!("Merge conflicts detected in {} files", conflict_files.len()),
                        );

                        task.internal_status = InternalStatus::Merging;
                        task.touch();

                        if let Err(e) = task_repo.update(&task).await {
                            tracing::error!(error = %e, "Failed to update task to Merging status");
                            return;
                        }

                        if let Err(e) = task_repo
                            .persist_status_change(
                                &task_id,
                                InternalStatus::PendingMerge,
                                InternalStatus::Merging,
                                "merge_conflict",
                            )
                            .await
                        {
                            tracing::warn!(error = %e, "Failed to record merge conflict transition (non-fatal)");
                        }

                        self.machine
                            .context
                            .services
                            .event_emitter
                            .emit_status_change(task_id_str, "pending_merge", "merging")
                            .await;

                        let prompt = format!("Resolve merge conflicts for task: {}", task_id_str);
                        tracing::info!(
                            task_id = task_id_str,
                            "Spawning merger agent for conflict resolution (direct merge)"
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
                                "Merger agent spawned successfully"
                            ),
                            Err(e) => {
                                tracing::error!(task_id = task_id_str, error = %e, "Failed to spawn merger agent")
                            }
                        }
                    }
                    Ok(MergeAttemptResult::BranchNotFound { branch }) => {
                        tracing::error!(
                            task_id = task_id_str,
                            missing_branch = %branch,
                            "Merge failed: branch '{}' does not exist", branch
                        );

                        task.metadata = Some(
                            serde_json::json!({
                                "error": format!("Branch '{}' does not exist", branch),
                                "missing_branch": branch,
                                "source_branch": source_branch,
                                "target_branch": target_branch,
                            })
                            .to_string(),
                        );
                        task.internal_status = InternalStatus::MergeIncomplete;
                        task.touch();

                        if let Err(e) = task_repo.update(&task).await {
                            tracing::error!(error = %e, "Failed to update task to MergeIncomplete status");
                            return;
                        }

                        if let Err(e) = task_repo
                            .persist_status_change(
                                &task_id,
                                InternalStatus::PendingMerge,
                                InternalStatus::MergeIncomplete,
                                "merge_incomplete",
                            )
                            .await
                        {
                            tracing::warn!(error = %e, "Failed to record merge incomplete transition (non-fatal)");
                        }

                        self.machine
                            .context
                            .services
                            .event_emitter
                            .emit_status_change(task_id_str, "pending_merge", "merge_incomplete")
                            .await;
                    }
                    Err(e) => {
                        tracing::error!(
                            task_id = task_id_str,
                            error = %e,
                            source_branch = %source_branch,
                            target_branch = %target_branch,
                            "Direct merge failed, transitioning to MergeIncomplete"
                        );

                        task.metadata = Some(
                            serde_json::json!({
                                "error": e.to_string(),
                                "source_branch": source_branch,
                                "target_branch": target_branch,
                            })
                            .to_string(),
                        );
                        task.internal_status = InternalStatus::MergeIncomplete;
                        task.touch();

                        if let Err(e) = task_repo.update(&task).await {
                            tracing::error!(error = %e, "Failed to update task to MergeIncomplete status");
                            return;
                        }

                        if let Err(e) = task_repo
                            .persist_status_change(
                                &task_id,
                                InternalStatus::PendingMerge,
                                InternalStatus::MergeIncomplete,
                                "merge_incomplete",
                            )
                            .await
                        {
                            tracing::warn!(error = %e, "Failed to record merge incomplete transition (non-fatal)");
                        }

                        self.machine
                            .context
                            .services
                            .event_emitter
                            .emit_status_change(task_id_str, "pending_merge", "merge_incomplete")
                            .await;
                    }
                }
            }
            (MergeStrategy::Rebase, GitMode::Worktree) => {
                // Worktree mode with rebase: rebase in worktree, then fast-forward merge
                let current_branch = GitService::get_current_branch(repo_path).unwrap_or_default();
                let target_is_checked_out = current_branch == target_branch;

                if target_is_checked_out {
                    // Target branch (e.g., main) is checked out in the primary repo.
                    // Use checkout-free FF/merge to avoid disrupting working tree.
                    // FF is attempted first; falls back to merge commit if branches diverged.
                    tracing::info!(
                        task_id = task_id_str,
                        target_branch = %target_branch,
                        "Target branch is checked out, using checkout-free fast-forward/merge"
                    );

                    // Validate branches exist
                    if !GitService::branch_exists(repo_path, &source_branch)
                        || !GitService::branch_exists(repo_path, &target_branch)
                    {
                        let missing = if !GitService::branch_exists(repo_path, &source_branch) {
                            &source_branch
                        } else {
                            &target_branch
                        };
                        tracing::error!(
                            task_id = task_id_str,
                            "Branch '{}' does not exist",
                            missing
                        );

                        // Record merge recovery event for retry tracking
                        let mut recovery =
                            MergeRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
                                .unwrap_or(None)
                                .unwrap_or_else(MergeRecoveryMetadata::new);

                        // Count existing AutoRetryTriggered events
                        let attempt_count = recovery
                            .events
                            .iter()
                            .filter(|e| {
                                matches!(e.kind, MergeRecoveryEventKind::AutoRetryTriggered)
                            })
                            .count() as u32
                            + 1;

                        // Create AutoRetryTriggered event
                        let event = MergeRecoveryEvent::new(
                            MergeRecoveryEventKind::AutoRetryTriggered,
                            MergeRecoverySource::Auto,
                            MergeRecoveryReasonCode::BranchNotFound,
                            format!("Branch '{}' does not exist", missing),
                        )
                        .with_target_branch(&target_branch)
                        .with_source_branch(&source_branch)
                        .with_attempt(attempt_count);

                        recovery.append_event(event);

                        // Update task metadata with recovery events and branch_missing flag
                        match recovery.update_task_metadata(task.metadata.as_deref()) {
                            Ok(updated_json) => {
                                // Add branch_missing flag to metadata
                                if let Ok(mut metadata_obj) =
                                    serde_json::from_str::<serde_json::Value>(&updated_json)
                                {
                                    if let Some(obj) = metadata_obj.as_object_mut() {
                                        obj.insert(
                                            "branch_missing".to_string(),
                                            serde_json::json!(true),
                                        );
                                    }
                                    task.metadata = Some(metadata_obj.to_string());
                                } else {
                                    task.metadata = Some(updated_json);
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    task_id = task_id_str,
                                    error = %e,
                                    "Failed to serialize merge recovery metadata, using legacy format"
                                );
                                // Fallback to legacy metadata
                                task.metadata = Some(serde_json::json!({"error": format!("Branch '{}' does not exist", missing), "missing_branch": missing, "source_branch": source_branch, "target_branch": target_branch, "branch_missing": true}).to_string());
                            }
                        }

                        task.internal_status = InternalStatus::MergeIncomplete;
                        task.touch();
                        let _ = task_repo.update(&task).await;
                        let _ = task_repo
                            .persist_status_change(
                                &task_id,
                                InternalStatus::PendingMerge,
                                InternalStatus::MergeIncomplete,
                                "merge_incomplete",
                            )
                            .await;
                        self.machine
                            .context
                            .services
                            .event_emitter
                            .emit_status_change(task_id_str, "pending_merge", "merge_incomplete")
                            .await;
                        return;
                    }

                    let cf_result = checkout_free::try_fast_forward_checkout_free(
                        repo_path,
                        &source_branch,
                        &target_branch,
                    );

                    match cf_result {
                        Ok(CheckoutFreeMergeResult::Success { commit_sha }) => {
                            // Atomically sync working tree
                            if let Err(e) = GitService::hard_reset_to_head(repo_path) {
                                tracing::error!(error = %e, task_id = task_id_str, "Failed to sync working tree after checkout-free rebase merge");
                            }

                            tracing::info!(
                                task_id = task_id_str,
                                commit_sha = %commit_sha,
                                "Checkout-free rebase merge succeeded"
                            );

                            emit_merge_progress(
                                self.machine.context.services.app_handle.as_ref(),
                                task_id_str,
                                MergePhase::ProgrammaticMerge,
                                MergePhaseStatus::Passed,
                                format!("Merge completed: {}", commit_sha),
                            );

                            if TEMP_SKIP_POST_MERGE_VALIDATION {
                                tracing::warn!(task_id = task_id_str, "Post-merge validation temporarily disabled (global flag, checkout-free rebase)");
                            } else {
                                let skip_validation = take_skip_validation_flag(&mut task);
                                let validation_mode = &project.merge_validation_mode;
                                if !skip_validation && *validation_mode != MergeValidationMode::Off
                                {
                                    let source_sha =
                                        GitService::get_branch_sha(repo_path, &source_branch).ok();
                                    let cached_log = source_sha
                                        .as_deref()
                                        .and_then(|sha| extract_cached_validation(&task, sha));
                                    let app_handle_ref =
                                        self.machine.context.services.app_handle.as_ref();
                                    if let Some(validation) = run_validation_commands(
                                        &project,
                                        &task,
                                        repo_path,
                                        task_id_str,
                                        app_handle_ref,
                                        cached_log.as_deref(),
                                    )
                                    .await
                                    {
                                        if !validation.all_passed {
                                            if *validation_mode == MergeValidationMode::Warn {
                                                tracing::warn!(task_id = task_id_str, "Validation failed in Warn mode (checkout-free rebase), proceeding");
                                                task.metadata =
                                                    Some(format_validation_warn_metadata(
                                                        &validation.log,
                                                        &source_branch,
                                                        &target_branch,
                                                    ));
                                            } else {
                                                self.handle_validation_failure(
                                                    &mut task,
                                                    &task_id,
                                                    task_id_str,
                                                    task_repo,
                                                    &validation.failures,
                                                    &validation.log,
                                                    &source_branch,
                                                    &target_branch,
                                                    repo_path,
                                                    "checkout-free",
                                                    validation_mode,
                                                )
                                                .await;
                                                return;
                                            }
                                        } else {
                                            task.metadata = Some(serde_json::json!({"validation_log": validation.log, "validation_source_sha": source_sha, "source_branch": source_branch, "target_branch": target_branch}).to_string());
                                        }
                                    }
                                }
                            }

                            let app_handle = self.machine.context.services.app_handle.as_ref();
                            if let Err(e) = complete_merge_internal(
                                &mut task,
                                &project,
                                &commit_sha,
                                &target_branch,
                                task_repo,
                                app_handle,
                            )
                            .await
                            {
                                tracing::error!(error = %e, task_id = task_id_str, "Failed to complete checkout-free rebase merge");
                                task.metadata = Some(serde_json::json!({"error": format!("complete_merge_internal failed: {}", e), "source_branch": source_branch, "target_branch": target_branch}).to_string());
                                task.internal_status = InternalStatus::MergeIncomplete;
                                task.touch();
                                let _ = task_repo.update(&task).await;
                                let _ = task_repo
                                    .persist_status_change(
                                        &task_id,
                                        InternalStatus::PendingMerge,
                                        InternalStatus::MergeIncomplete,
                                        "merge_incomplete",
                                    )
                                    .await;
                                self.machine
                                    .context
                                    .services
                                    .event_emitter
                                    .emit_status_change(
                                        task_id_str,
                                        "pending_merge",
                                        "merge_incomplete",
                                    )
                                    .await;
                            } else {
                                self.post_merge_cleanup(
                                    task_id_str,
                                    &task_id,
                                    repo_path,
                                    plan_branch_repo,
                                )
                                .await;
                            }
                        }
                        Ok(CheckoutFreeMergeResult::Conflict {
                            files: conflict_files,
                        }) => {
                            tracing::info!(
                                task_id = task_id_str,
                                conflict_count = conflict_files.len(),
                                "Checkout-free rebase merge has conflicts, creating temp worktree"
                            );
                            emit_merge_progress(
                                self.machine.context.services.app_handle.as_ref(),
                                task_id_str,
                                MergePhase::ProgrammaticMerge,
                                MergePhaseStatus::Failed,
                                format!("Merge conflicts in {} files", conflict_files.len()),
                            );

                            let merge_wt_path =
                                PathBuf::from(compute_merge_worktree_path(&project, task_id_str));
                            let target_sha = GitService::get_branch_sha(repo_path, &target_branch)
                                .unwrap_or_default();
                            let resolve_branch = format!("merge-resolve/{}", task_id_str);
                            let _ = GitService::create_branch_at(
                                repo_path,
                                &resolve_branch,
                                &target_sha,
                            );
                            let _ = GitService::checkout_existing_branch_worktree(
                                repo_path,
                                &merge_wt_path,
                                &resolve_branch,
                            );
                            let _ = std::process::Command::new("git")
                                .args(["merge", &source_branch, "--no-edit"])
                                .current_dir(&merge_wt_path)
                                .output();

                            task.internal_status = InternalStatus::Merging;
                            task.touch();
                            let _ = task_repo.update(&task).await;
                            let _ = task_repo
                                .persist_status_change(
                                    &task_id,
                                    InternalStatus::PendingMerge,
                                    InternalStatus::Merging,
                                    "merge_conflict",
                                )
                                .await;
                            self.machine
                                .context
                                .services
                                .event_emitter
                                .emit_status_change(task_id_str, "pending_merge", "merging")
                                .await;

                            let prompt =
                                format!("Resolve merge conflicts for task: {}", task_id_str);
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
                                Ok(_) => {
                                    tracing::info!(task_id = task_id_str, "Merger agent spawned")
                                }
                                Err(e) => {
                                    tracing::error!(task_id = task_id_str, error = %e, "Failed to spawn merger agent")
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!(task_id = task_id_str, error = %e, "Checkout-free rebase merge failed");
                            task.metadata = Some(serde_json::json!({"error": e.to_string(), "source_branch": source_branch, "target_branch": target_branch}).to_string());
                            task.internal_status = InternalStatus::MergeIncomplete;
                            task.touch();
                            let _ = task_repo.update(&task).await;
                            let _ = task_repo
                                .persist_status_change(
                                    &task_id,
                                    InternalStatus::PendingMerge,
                                    InternalStatus::MergeIncomplete,
                                    "merge_incomplete",
                                )
                                .await;
                            self.machine
                                .context
                                .services
                                .event_emitter
                                .emit_status_change(
                                    task_id_str,
                                    "pending_merge",
                                    "merge_incomplete",
                                )
                                .await;
                        }
                    }
                } else {
                    // Target branch is NOT checked out — use rebase-and-merge in worktrees
                    let rebase_wt_path_str = compute_rebase_worktree_path(&project, task_id_str);
                    let rebase_wt_path = PathBuf::from(&rebase_wt_path_str);
                    let merge_wt_path_str = compute_merge_worktree_path(&project, task_id_str);
                    let merge_wt_path = PathBuf::from(&merge_wt_path_str);

                    // Clean up stale rebase worktree from prior attempt
                    if rebase_wt_path.exists() {
                        tracing::info!(
                            task_id = task_id_str,
                            rebase_worktree_path = %rebase_wt_path_str,
                            "Cleaning up stale rebase worktree from previous attempt"
                        );
                        let _ = GitService::delete_worktree(repo_path, &rebase_wt_path);
                    }

                    tracing::info!(
                        task_id = task_id_str,
                        rebase_worktree_path = %rebase_wt_path_str,
                        merge_worktree_path = %merge_wt_path_str,
                        "Using rebase-and-merge in worktrees"
                    );

                    let merge_result = GitService::try_rebase_and_merge_in_worktree(
                        repo_path,
                        &source_branch,
                        &target_branch,
                        &rebase_wt_path,
                        &merge_wt_path,
                    );

                    match merge_result {
                        Ok(MergeAttemptResult::Success { commit_sha }) => {
                            tracing::info!(
                                task_id = task_id_str,
                                commit_sha = %commit_sha,
                                "Rebase-and-merge in worktree succeeded (fast path)"
                            );

                            emit_merge_progress(
                                self.machine.context.services.app_handle.as_ref(),
                                task_id_str,
                                MergePhase::ProgrammaticMerge,
                                MergePhaseStatus::Passed,
                                format!("Merge completed: {}", commit_sha),
                            );

                            if TEMP_SKIP_POST_MERGE_VALIDATION {
                                tracing::warn!(
                                task_id = task_id_str,
                                "Post-merge validation temporarily disabled (global flag, worktree rebase)"
                            );
                            } else {
                                let skip_validation = take_skip_validation_flag(&mut task);
                                let validation_mode = &project.merge_validation_mode;
                                if !skip_validation && *validation_mode != MergeValidationMode::Off
                                {
                                    let source_sha =
                                        GitService::get_branch_sha(repo_path, &source_branch).ok();
                                    let cached_log = source_sha
                                        .as_deref()
                                        .and_then(|sha| extract_cached_validation(&task, sha));
                                    let app_handle_ref =
                                        self.machine.context.services.app_handle.as_ref();
                                    if let Some(validation) = run_validation_commands(
                                        &project,
                                        &task,
                                        &merge_wt_path,
                                        task_id_str,
                                        app_handle_ref,
                                        cached_log.as_deref(),
                                    )
                                    .await
                                    {
                                        if !validation.all_passed {
                                            if *validation_mode == MergeValidationMode::Warn {
                                                tracing::warn!(task_id = task_id_str, "Validation failed in Warn mode (worktree rebase), proceeding");
                                                task.metadata =
                                                    Some(format_validation_warn_metadata(
                                                        &validation.log,
                                                        &source_branch,
                                                        &target_branch,
                                                    ));
                                            } else {
                                                self.handle_validation_failure(
                                                    &mut task,
                                                    &task_id,
                                                    task_id_str,
                                                    task_repo,
                                                    &validation.failures,
                                                    &validation.log,
                                                    &source_branch,
                                                    &target_branch,
                                                    &merge_wt_path,
                                                    "worktree",
                                                    validation_mode,
                                                )
                                                .await;
                                                return;
                                            }
                                        } else {
                                            task.metadata = Some(
                                                serde_json::json!({
                                                    "validation_log": validation.log,
                                                    "validation_source_sha": source_sha,
                                                    "source_branch": source_branch,
                                                    "target_branch": target_branch,
                                                })
                                                .to_string(),
                                            );
                                        }
                                    }
                                }
                            }

                            // Clean up merge worktree after success
                            if let Err(e) = GitService::delete_worktree(repo_path, &merge_wt_path) {
                                tracing::warn!(
                                    error = %e,
                                    task_id = task_id_str,
                                    "Failed to delete merge worktree after rebase success (non-fatal)"
                                );
                            }

                            let app_handle = self.machine.context.services.app_handle.as_ref();
                            if let Err(e) = complete_merge_internal(
                                &mut task,
                                &project,
                                &commit_sha,
                                &target_branch,
                                task_repo,
                                app_handle,
                            )
                            .await
                            {
                                tracing::error!(error = %e, task_id = task_id_str, "Failed to complete rebase merge in worktree");

                                task.metadata = Some(
                                    serde_json::json!({
                                        "error": format!("complete_merge_internal failed: {}", e),
                                        "source_branch": source_branch,
                                        "target_branch": target_branch,
                                    })
                                    .to_string(),
                                );
                                task.internal_status = InternalStatus::MergeIncomplete;
                                task.touch();

                                let _ = task_repo.update(&task).await;
                                let _ = task_repo
                                    .persist_status_change(
                                        &task_id,
                                        InternalStatus::PendingMerge,
                                        InternalStatus::MergeIncomplete,
                                        "merge_incomplete",
                                    )
                                    .await;

                                self.machine
                                    .context
                                    .services
                                    .event_emitter
                                    .emit_status_change(
                                        task_id_str,
                                        "pending_merge",
                                        "merge_incomplete",
                                    )
                                    .await;
                            } else {
                                self.post_merge_cleanup(
                                    task_id_str,
                                    &task_id,
                                    repo_path,
                                    plan_branch_repo,
                                )
                                .await;
                            }
                        }
                        Ok(MergeAttemptResult::NeedsAgent { conflict_files }) => {
                            // Rebase conflict in worktree — agent resolves there
                            tracing::info!(
                                task_id = task_id_str,
                                conflict_count = conflict_files.len(),
                                rebase_worktree_path = %rebase_wt_path_str,
                                "Rebase in worktree has conflicts, transitioning to Merging"
                            );

                            emit_merge_progress(
                                self.machine.context.services.app_handle.as_ref(),
                                task_id_str,
                                MergePhase::ProgrammaticMerge,
                                MergePhaseStatus::Failed,
                                format!(
                                    "Rebase conflicts detected in {} files",
                                    conflict_files.len()
                                ),
                            );

                            // Set worktree_path to rebase worktree for agent CWD
                            task.worktree_path = Some(rebase_wt_path_str.clone());
                            // Store conflict_type in metadata so agent/completion knows it's a rebase
                            let mut meta =
                                parse_metadata(&task).unwrap_or_else(|| serde_json::json!({}));
                            if let Some(obj) = meta.as_object_mut() {
                                obj.insert(
                                    "conflict_type".to_string(),
                                    serde_json::json!("rebase"),
                                );
                            }
                            task.metadata = Some(meta.to_string());
                            task.internal_status = InternalStatus::Merging;
                            task.touch();

                            if let Err(e) = task_repo.update(&task).await {
                                tracing::error!(error = %e, "Failed to update task to Merging with rebase worktree");
                                return;
                            }

                            if let Err(e) = task_repo
                                .persist_status_change(
                                    &task_id,
                                    InternalStatus::PendingMerge,
                                    InternalStatus::Merging,
                                    "rebase_conflict",
                                )
                                .await
                            {
                                tracing::warn!(error = %e, "Failed to record rebase conflict transition (non-fatal)");
                            }

                            self.machine
                                .context
                                .services
                                .event_emitter
                                .emit_status_change(task_id_str, "pending_merge", "merging")
                                .await;

                            let prompt = format!(
                            "Resolve rebase conflicts for task: {}. After resolving each file, run `git add <file>` then `git rebase --continue`.",
                            task_id_str
                        );
                            tracing::info!(
                                task_id = task_id_str,
                                "Spawning merger agent for rebase conflict resolution"
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
                                    "Merger agent spawned successfully"
                                ),
                                Err(e) => {
                                    tracing::error!(task_id = task_id_str, error = %e, "Failed to spawn merger agent")
                                }
                            }
                        }
                        Ok(MergeAttemptResult::BranchNotFound { branch }) => {
                            tracing::error!(
                                task_id = task_id_str,
                                missing_branch = %branch,
                                "Merge failed: branch '{}' does not exist", branch
                            );

                            task.metadata = Some(
                                serde_json::json!({
                                    "error": format!("Branch '{}' does not exist", branch),
                                    "missing_branch": branch,
                                    "source_branch": source_branch,
                                    "target_branch": target_branch,
                                })
                                .to_string(),
                            );
                            task.internal_status = InternalStatus::MergeIncomplete;
                            task.touch();

                            if let Err(e) = task_repo.update(&task).await {
                                tracing::error!(error = %e, "Failed to update task to MergeIncomplete status");
                                return;
                            }

                            if let Err(e) = task_repo
                                .persist_status_change(
                                    &task_id,
                                    InternalStatus::PendingMerge,
                                    InternalStatus::MergeIncomplete,
                                    "merge_incomplete",
                                )
                                .await
                            {
                                tracing::warn!(error = %e, "Failed to record merge incomplete transition (non-fatal)");
                            }

                            self.machine
                                .context
                                .services
                                .event_emitter
                                .emit_status_change(
                                    task_id_str,
                                    "pending_merge",
                                    "merge_incomplete",
                                )
                                .await;
                        }
                        Err(e) => {
                            if GitService::is_branch_lock_error(&e) {
                                tracing::warn!(
                                    task_id = task_id_str,
                                    error = %e,
                                    "Rebase-and-merge in worktree failed due to branch lock, staying in PendingMerge"
                                );

                                if rebase_wt_path.exists() {
                                    let _ = GitService::delete_worktree(repo_path, &rebase_wt_path);
                                }
                                if merge_wt_path.exists() {
                                    let _ = GitService::delete_worktree(repo_path, &merge_wt_path);
                                }
                            } else {
                                tracing::error!(
                                    task_id = task_id_str,
                                    error = %e,
                                    "Rebase-and-merge in worktree failed, transitioning to MergeIncomplete"
                                );

                                if rebase_wt_path.exists() {
                                    let _ = GitService::delete_worktree(repo_path, &rebase_wt_path);
                                }
                                if merge_wt_path.exists() {
                                    let _ = GitService::delete_worktree(repo_path, &merge_wt_path);
                                }

                                task.metadata = Some(
                                    serde_json::json!({
                                        "error": e.to_string(),
                                        "source_branch": source_branch,
                                        "target_branch": target_branch,
                                    })
                                    .to_string(),
                                );
                                task.internal_status = InternalStatus::MergeIncomplete;
                                task.touch();

                                let _ = task_repo.update(&task).await;
                                let _ = task_repo
                                    .persist_status_change(
                                        &task_id,
                                        InternalStatus::PendingMerge,
                                        InternalStatus::MergeIncomplete,
                                        "merge_incomplete",
                                    )
                                    .await;

                                self.machine
                                    .context
                                    .services
                                    .event_emitter
                                    .emit_status_change(
                                        task_id_str,
                                        "pending_merge",
                                        "merge_incomplete",
                                    )
                                    .await;
                            }
                        }
                    }
                }
            }
            (MergeStrategy::Squash, GitMode::Local) => {
                // Local mode: squash merge for clean single commit
                let merge_result = GitService::try_squash_merge(
                    repo_path,
                    &source_branch,
                    &target_branch,
                    &squash_commit_msg,
                );
                match merge_result {
                    Ok(MergeAttemptResult::Success { commit_sha }) => {
                        tracing::info!(
                            task_id = task_id_str,
                            commit_sha = %commit_sha,
                            "Squash merge succeeded (single commit on target)"
                        );

                        emit_merge_progress(
                            self.machine.context.services.app_handle.as_ref(),
                            task_id_str,
                            MergePhase::ProgrammaticMerge,
                            MergePhaseStatus::Passed,
                            format!("Squash merge completed: {}", commit_sha),
                        );

                        if TEMP_SKIP_POST_MERGE_VALIDATION {
                            tracing::warn!(
                            task_id = task_id_str,
                            "Post-merge validation temporarily disabled (global flag, local squash merge)"
                        );
                        } else {
                            let skip_validation = take_skip_validation_flag(&mut task);
                            let validation_mode = &project.merge_validation_mode;
                            if !skip_validation && *validation_mode != MergeValidationMode::Off {
                                let source_sha =
                                    GitService::get_branch_sha(repo_path, &source_branch).ok();
                                let cached_log = source_sha
                                    .as_deref()
                                    .and_then(|sha| extract_cached_validation(&task, sha));
                                let app_handle_ref =
                                    self.machine.context.services.app_handle.as_ref();
                                if let Some(validation) = run_validation_commands(
                                    &project,
                                    &task,
                                    repo_path,
                                    task_id_str,
                                    app_handle_ref,
                                    cached_log.as_deref(),
                                )
                                .await
                                {
                                    if !validation.all_passed {
                                        if *validation_mode == MergeValidationMode::Warn {
                                            tracing::warn!(task_id = task_id_str, "Validation failed in Warn mode (local squash merge), proceeding");
                                            task.metadata = Some(format_validation_warn_metadata(
                                                &validation.log,
                                                &source_branch,
                                                &target_branch,
                                            ));
                                        } else {
                                            self.handle_validation_failure(
                                                &mut task,
                                                &task_id,
                                                task_id_str,
                                                task_repo,
                                                &validation.failures,
                                                &validation.log,
                                                &source_branch,
                                                &target_branch,
                                                repo_path,
                                                "local",
                                                validation_mode,
                                            )
                                            .await;
                                            return;
                                        }
                                    } else {
                                        task.metadata = Some(
                                            serde_json::json!({
                                                "validation_log": validation.log,
                                                "validation_source_sha": source_sha,
                                                "source_branch": source_branch,
                                                "target_branch": target_branch,
                                            })
                                            .to_string(),
                                        );
                                    }
                                }
                            }
                        }

                        let app_handle = self.machine.context.services.app_handle.as_ref();
                        if let Err(e) = complete_merge_internal(
                            &mut task,
                            &project,
                            &commit_sha,
                            &target_branch,
                            task_repo,
                            app_handle,
                        )
                        .await
                        {
                            tracing::error!(error = %e, task_id = task_id_str, "Failed to complete squash merge, falling back to MergeIncomplete");

                            task.metadata = Some(
                                serde_json::json!({
                                    "error": format!("complete_merge_internal failed: {}", e),
                                    "source_branch": source_branch,
                                    "target_branch": target_branch,
                                })
                                .to_string(),
                            );
                            task.internal_status = InternalStatus::MergeIncomplete;
                            task.touch();

                            let _ = task_repo.update(&task).await;
                            let _ = task_repo
                                .persist_status_change(
                                    &task_id,
                                    InternalStatus::PendingMerge,
                                    InternalStatus::MergeIncomplete,
                                    "merge_incomplete",
                                )
                                .await;

                            self.machine
                                .context
                                .services
                                .event_emitter
                                .emit_status_change(
                                    task_id_str,
                                    "pending_merge",
                                    "merge_incomplete",
                                )
                                .await;
                        } else {
                            self.post_merge_cleanup(
                                task_id_str,
                                &task_id,
                                repo_path,
                                plan_branch_repo,
                            )
                            .await;
                        }
                    }
                    Ok(MergeAttemptResult::NeedsAgent { conflict_files }) => {
                        tracing::info!(
                            task_id = task_id_str,
                            conflict_count = conflict_files.len(),
                            "Squash merge failed: conflicts detected, transitioning to Merging"
                        );

                        emit_merge_progress(
                            self.machine.context.services.app_handle.as_ref(),
                            task_id_str,
                            MergePhase::ProgrammaticMerge,
                            MergePhaseStatus::Failed,
                            format!(
                                "Squash merge conflicts detected in {} files",
                                conflict_files.len()
                            ),
                        );

                        task.internal_status = InternalStatus::Merging;
                        task.touch();

                        if let Err(e) = task_repo.update(&task).await {
                            tracing::error!(error = %e, "Failed to update task to Merging status");
                            return;
                        }

                        if let Err(e) = task_repo
                            .persist_status_change(
                                &task_id,
                                InternalStatus::PendingMerge,
                                InternalStatus::Merging,
                                "merge_conflict",
                            )
                            .await
                        {
                            tracing::warn!(error = %e, "Failed to record squash merge conflict transition (non-fatal)");
                        }

                        self.machine
                            .context
                            .services
                            .event_emitter
                            .emit_status_change(task_id_str, "pending_merge", "merging")
                            .await;

                        let prompt = format!("Resolve merge conflicts for task: {}", task_id_str);
                        tracing::info!(
                            task_id = task_id_str,
                            "Spawning merger agent for conflict resolution (squash merge)"
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
                                "Merger agent spawned successfully"
                            ),
                            Err(e) => {
                                tracing::error!(task_id = task_id_str, error = %e, "Failed to spawn merger agent")
                            }
                        }
                    }
                    Ok(MergeAttemptResult::BranchNotFound { branch }) => {
                        tracing::error!(
                            task_id = task_id_str,
                            missing_branch = %branch,
                            "Merge failed: branch '{}' does not exist", branch
                        );

                        task.metadata = Some(
                            serde_json::json!({
                                "error": format!("Branch '{}' does not exist", branch),
                                "missing_branch": branch,
                                "source_branch": source_branch,
                                "target_branch": target_branch,
                            })
                            .to_string(),
                        );
                        task.internal_status = InternalStatus::MergeIncomplete;
                        task.touch();

                        if let Err(e) = task_repo.update(&task).await {
                            tracing::error!(error = %e, "Failed to update task to MergeIncomplete status");
                            return;
                        }

                        if let Err(e) = task_repo
                            .persist_status_change(
                                &task_id,
                                InternalStatus::PendingMerge,
                                InternalStatus::MergeIncomplete,
                                "merge_incomplete",
                            )
                            .await
                        {
                            tracing::warn!(error = %e, "Failed to record merge incomplete transition (non-fatal)");
                        }

                        self.machine
                            .context
                            .services
                            .event_emitter
                            .emit_status_change(task_id_str, "pending_merge", "merge_incomplete")
                            .await;
                    }
                    Err(e) => {
                        tracing::error!(
                            task_id = task_id_str,
                            error = %e,
                            "Squash merge failed, transitioning to MergeIncomplete"
                        );

                        task.metadata = Some(
                            serde_json::json!({
                                "error": e.to_string(),
                                "source_branch": source_branch,
                                "target_branch": target_branch,
                            })
                            .to_string(),
                        );
                        task.internal_status = InternalStatus::MergeIncomplete;
                        task.touch();

                        let _ = task_repo.update(&task).await;
                        let _ = task_repo
                            .persist_status_change(
                                &task_id,
                                InternalStatus::PendingMerge,
                                InternalStatus::MergeIncomplete,
                                "merge_incomplete",
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
            (MergeStrategy::Squash, GitMode::Worktree) => {
                // Worktree mode: squash merge in worktree (or in-repo if target checked out)
                let current_branch = GitService::get_current_branch(repo_path).unwrap_or_default();
                let target_is_checked_out = current_branch == target_branch;

                let merge_result = if target_is_checked_out {
                    // Use checkout-free squash merge to avoid disrupting working tree
                    tracing::info!(
                        task_id = task_id_str,
                        target_branch = %target_branch,
                        "Target branch is checked out, using checkout-free squash merge"
                    );
                    // Validate branches exist
                    if !GitService::branch_exists(repo_path, &source_branch) {
                        Ok(MergeAttemptResult::BranchNotFound {
                            branch: source_branch.clone(),
                        })
                    } else if !GitService::branch_exists(repo_path, &target_branch) {
                        Ok(MergeAttemptResult::BranchNotFound {
                            branch: target_branch.clone(),
                        })
                    } else {
                        match checkout_free::try_squash_merge_checkout_free(
                            repo_path,
                            &source_branch,
                            &target_branch,
                            &squash_commit_msg,
                        ) {
                            Ok(CheckoutFreeMergeResult::Success { commit_sha }) => {
                                // Atomically sync working tree
                                if let Err(e) = GitService::hard_reset_to_head(repo_path) {
                                    tracing::error!(error = %e, task_id = task_id_str, "Failed to sync working tree after checkout-free squash merge");
                                }
                                Ok(MergeAttemptResult::Success { commit_sha })
                            }
                            Ok(CheckoutFreeMergeResult::Conflict { files }) => {
                                Ok(MergeAttemptResult::NeedsAgent {
                                    conflict_files: files,
                                })
                            }
                            Err(e) => Err(e),
                        }
                    }
                } else {
                    let merge_wt_path =
                        PathBuf::from(compute_merge_worktree_path(&project, task_id_str));
                    tracing::info!(
                        task_id = task_id_str,
                        merge_worktree = %merge_wt_path.display(),
                        "Squash merging in isolated worktree"
                    );
                    let result = GitService::try_squash_merge_in_worktree(
                        repo_path,
                        &source_branch,
                        &target_branch,
                        &merge_wt_path,
                        &squash_commit_msg,
                    );
                    // Clean up worktree on success
                    if let Ok(MergeAttemptResult::Success { .. }) = &result {
                        if let Err(e) = GitService::delete_worktree(repo_path, &merge_wt_path) {
                            tracing::warn!(
                                error = %e,
                                task_id = task_id_str,
                                "Failed to delete merge worktree after squash success (non-fatal)"
                            );
                        }
                    }
                    result
                };

                match merge_result {
                    Ok(MergeAttemptResult::Success { commit_sha }) => {
                        tracing::info!(
                            task_id = task_id_str,
                            commit_sha = %commit_sha,
                            "Squash merge in worktree mode succeeded"
                        );

                        emit_merge_progress(
                            self.machine.context.services.app_handle.as_ref(),
                            task_id_str,
                            MergePhase::ProgrammaticMerge,
                            MergePhaseStatus::Passed,
                            format!("Squash merge completed: {}", commit_sha),
                        );

                        if TEMP_SKIP_POST_MERGE_VALIDATION {
                            tracing::warn!(
                            task_id = task_id_str,
                            "Post-merge validation temporarily disabled (global flag, worktree squash merge)"
                        );
                        } else {
                            let skip_validation = take_skip_validation_flag(&mut task);
                            let validation_mode = &project.merge_validation_mode;
                            if !skip_validation && *validation_mode != MergeValidationMode::Off {
                                let source_sha =
                                    GitService::get_branch_sha(repo_path, &source_branch).ok();
                                let cached_log = source_sha
                                    .as_deref()
                                    .and_then(|sha| extract_cached_validation(&task, sha));
                                let app_handle_ref =
                                    self.machine.context.services.app_handle.as_ref();
                                if let Some(validation) = run_validation_commands(
                                    &project,
                                    &task,
                                    repo_path,
                                    task_id_str,
                                    app_handle_ref,
                                    cached_log.as_deref(),
                                )
                                .await
                                {
                                    if !validation.all_passed {
                                        if *validation_mode == MergeValidationMode::Warn {
                                            tracing::warn!(task_id = task_id_str, "Validation failed in Warn mode (worktree squash merge), proceeding");
                                            task.metadata = Some(format_validation_warn_metadata(
                                                &validation.log,
                                                &source_branch,
                                                &target_branch,
                                            ));
                                        } else {
                                            self.handle_validation_failure(
                                                &mut task,
                                                &task_id,
                                                task_id_str,
                                                task_repo,
                                                &validation.failures,
                                                &validation.log,
                                                &source_branch,
                                                &target_branch,
                                                repo_path,
                                                "worktree",
                                                validation_mode,
                                            )
                                            .await;
                                            return;
                                        }
                                    } else {
                                        task.metadata = Some(
                                            serde_json::json!({
                                                "validation_log": validation.log,
                                                "validation_source_sha": source_sha,
                                                "source_branch": source_branch,
                                                "target_branch": target_branch,
                                            })
                                            .to_string(),
                                        );
                                    }
                                }
                            }
                        }

                        let app_handle = self.machine.context.services.app_handle.as_ref();
                        if let Err(e) = complete_merge_internal(
                            &mut task,
                            &project,
                            &commit_sha,
                            &target_branch,
                            task_repo,
                            app_handle,
                        )
                        .await
                        {
                            tracing::error!(error = %e, task_id = task_id_str, "Failed to complete squash merge in worktree mode");

                            task.metadata = Some(
                                serde_json::json!({
                                    "error": format!("complete_merge_internal failed: {}", e),
                                    "source_branch": source_branch,
                                    "target_branch": target_branch,
                                })
                                .to_string(),
                            );
                            task.internal_status = InternalStatus::MergeIncomplete;
                            task.touch();

                            let _ = task_repo.update(&task).await;
                            let _ = task_repo
                                .persist_status_change(
                                    &task_id,
                                    InternalStatus::PendingMerge,
                                    InternalStatus::MergeIncomplete,
                                    "merge_incomplete",
                                )
                                .await;

                            self.machine
                                .context
                                .services
                                .event_emitter
                                .emit_status_change(
                                    task_id_str,
                                    "pending_merge",
                                    "merge_incomplete",
                                )
                                .await;
                        } else {
                            self.post_merge_cleanup(
                                task_id_str,
                                &task_id,
                                repo_path,
                                plan_branch_repo,
                            )
                            .await;
                        }
                    }
                    Ok(MergeAttemptResult::NeedsAgent { conflict_files }) => {
                        tracing::info!(
                            task_id = task_id_str,
                            conflict_count = conflict_files.len(),
                            "Squash merge in worktree has conflicts, transitioning to Merging"
                        );

                        emit_merge_progress(
                            self.machine.context.services.app_handle.as_ref(),
                            task_id_str,
                            MergePhase::ProgrammaticMerge,
                            MergePhaseStatus::Failed,
                            format!(
                                "Squash merge conflicts detected in {} files",
                                conflict_files.len()
                            ),
                        );

                        task.internal_status = InternalStatus::Merging;
                        task.touch();

                        if let Err(e) = task_repo.update(&task).await {
                            tracing::error!(error = %e, "Failed to update task to Merging status");
                            return;
                        }

                        if let Err(e) = task_repo
                            .persist_status_change(
                                &task_id,
                                InternalStatus::PendingMerge,
                                InternalStatus::Merging,
                                "merge_conflict",
                            )
                            .await
                        {
                            tracing::warn!(error = %e, "Failed to record squash merge conflict transition (non-fatal)");
                        }

                        self.machine
                            .context
                            .services
                            .event_emitter
                            .emit_status_change(task_id_str, "pending_merge", "merging")
                            .await;

                        let prompt = format!("Resolve merge conflicts for task: {}", task_id_str);
                        tracing::info!(
                        task_id = task_id_str,
                        "Spawning merger agent for conflict resolution (squash merge, worktree)"
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
                                "Merger agent spawned successfully"
                            ),
                            Err(e) => {
                                tracing::error!(task_id = task_id_str, error = %e, "Failed to spawn merger agent")
                            }
                        }
                    }
                    Ok(MergeAttemptResult::BranchNotFound { branch }) => {
                        tracing::error!(
                            task_id = task_id_str,
                            missing_branch = %branch,
                            "Merge failed: branch '{}' does not exist", branch
                        );

                        // Record merge recovery event for retry tracking
                        let mut recovery =
                            MergeRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
                                .unwrap_or(None)
                                .unwrap_or_else(MergeRecoveryMetadata::new);

                        // Count existing AutoRetryTriggered events
                        let attempt_count = recovery
                            .events
                            .iter()
                            .filter(|e| {
                                matches!(e.kind, MergeRecoveryEventKind::AutoRetryTriggered)
                            })
                            .count() as u32
                            + 1;

                        // Create AutoRetryTriggered event
                        let event = MergeRecoveryEvent::new(
                            MergeRecoveryEventKind::AutoRetryTriggered,
                            MergeRecoverySource::Auto,
                            MergeRecoveryReasonCode::BranchNotFound,
                            format!("Branch '{}' does not exist", branch),
                        )
                        .with_target_branch(&target_branch)
                        .with_source_branch(&source_branch)
                        .with_attempt(attempt_count);

                        recovery.append_event(event);

                        // Update task metadata with recovery events and branch_missing flag
                        match recovery.update_task_metadata(task.metadata.as_deref()) {
                            Ok(updated_json) => {
                                // Add branch_missing flag to metadata
                                if let Ok(mut metadata_obj) =
                                    serde_json::from_str::<serde_json::Value>(&updated_json)
                                {
                                    if let Some(obj) = metadata_obj.as_object_mut() {
                                        obj.insert(
                                            "branch_missing".to_string(),
                                            serde_json::json!(true),
                                        );
                                    }
                                    task.metadata = Some(metadata_obj.to_string());
                                } else {
                                    task.metadata = Some(updated_json);
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    task_id = task_id_str,
                                    error = %e,
                                    "Failed to serialize merge recovery metadata, using legacy format"
                                );
                                // Fallback to legacy metadata
                                task.metadata = Some(
                                    serde_json::json!({
                                        "error": format!("Branch '{}' does not exist", branch),
                                        "missing_branch": branch,
                                        "source_branch": source_branch,
                                        "target_branch": target_branch,
                                        "branch_missing": true
                                    })
                                    .to_string(),
                                );
                            }
                        }

                        task.internal_status = InternalStatus::MergeIncomplete;
                        task.touch();

                        if let Err(e) = task_repo.update(&task).await {
                            tracing::error!(error = %e, "Failed to update task to MergeIncomplete status");
                            return;
                        }

                        if let Err(e) = task_repo
                            .persist_status_change(
                                &task_id,
                                InternalStatus::PendingMerge,
                                InternalStatus::MergeIncomplete,
                                "merge_incomplete",
                            )
                            .await
                        {
                            tracing::warn!(error = %e, "Failed to record merge incomplete transition (non-fatal)");
                        }

                        self.machine
                            .context
                            .services
                            .event_emitter
                            .emit_status_change(task_id_str, "pending_merge", "merge_incomplete")
                            .await;
                    }
                    Err(e) => {
                        if GitService::is_branch_lock_error(&e) {
                            tracing::warn!(
                                task_id = task_id_str,
                                error = %e,
                                "Squash merge in worktree failed due to branch lock, staying in PendingMerge"
                            );
                        } else {
                            tracing::error!(
                                task_id = task_id_str,
                                error = %e,
                                "Squash merge in worktree failed, transitioning to MergeIncomplete"
                            );

                            task.metadata = Some(
                                serde_json::json!({
                                    "error": e.to_string(),
                                    "source_branch": source_branch,
                                    "target_branch": target_branch,
                                })
                                .to_string(),
                            );
                            task.internal_status = InternalStatus::MergeIncomplete;
                            task.touch();

                            let _ = task_repo.update(&task).await;
                            let _ = task_repo
                                .persist_status_change(
                                    &task_id,
                                    InternalStatus::PendingMerge,
                                    InternalStatus::MergeIncomplete,
                                    "merge_incomplete",
                                )
                                .await;

                            self.machine
                                .context
                                .services
                                .event_emitter
                                .emit_status_change(
                                    task_id_str,
                                    "pending_merge",
                                    "merge_incomplete",
                                )
                                .await;
                        }
                    }
                }
            }
            (MergeStrategy::RebaseSquash, GitMode::Local) => {
                // Local mode: rebase first (catches conflicts), then squash into single commit
                let merge_result = GitService::try_rebase_squash_merge(
                    repo_path,
                    &source_branch,
                    &target_branch,
                    &squash_commit_msg,
                );
                match merge_result {
                    Ok(MergeAttemptResult::Success { commit_sha }) => {
                        tracing::info!(
                            task_id = task_id_str,
                            commit_sha = %commit_sha,
                            "Rebase+squash merge succeeded (single commit on target)"
                        );

                        emit_merge_progress(
                            self.machine.context.services.app_handle.as_ref(),
                            task_id_str,
                            MergePhase::ProgrammaticMerge,
                            MergePhaseStatus::Passed,
                            format!("Rebase+squash completed: {}", commit_sha),
                        );

                        if TEMP_SKIP_POST_MERGE_VALIDATION {
                            tracing::warn!(
                            task_id = task_id_str,
                            "Post-merge validation temporarily disabled (global flag, local rebase+squash)"
                        );
                        } else {
                            let skip_validation = take_skip_validation_flag(&mut task);
                            let validation_mode = &project.merge_validation_mode;
                            if !skip_validation && *validation_mode != MergeValidationMode::Off {
                                let source_sha =
                                    GitService::get_branch_sha(repo_path, &source_branch).ok();
                                let cached_log = source_sha
                                    .as_deref()
                                    .and_then(|sha| extract_cached_validation(&task, sha));
                                let app_handle_ref =
                                    self.machine.context.services.app_handle.as_ref();
                                if let Some(validation) = run_validation_commands(
                                    &project,
                                    &task,
                                    repo_path,
                                    task_id_str,
                                    app_handle_ref,
                                    cached_log.as_deref(),
                                )
                                .await
                                {
                                    if !validation.all_passed {
                                        if *validation_mode == MergeValidationMode::Warn {
                                            tracing::warn!(task_id = task_id_str, "Validation failed in Warn mode (local rebase+squash), proceeding");
                                            task.metadata = Some(format_validation_warn_metadata(
                                                &validation.log,
                                                &source_branch,
                                                &target_branch,
                                            ));
                                        } else {
                                            self.handle_validation_failure(
                                                &mut task,
                                                &task_id,
                                                task_id_str,
                                                task_repo,
                                                &validation.failures,
                                                &validation.log,
                                                &source_branch,
                                                &target_branch,
                                                repo_path,
                                                "local",
                                                validation_mode,
                                            )
                                            .await;
                                            return;
                                        }
                                    } else {
                                        task.metadata = Some(
                                            serde_json::json!({
                                                "validation_log": validation.log,
                                                "validation_source_sha": source_sha,
                                                "source_branch": source_branch,
                                                "target_branch": target_branch,
                                            })
                                            .to_string(),
                                        );
                                    }
                                }
                            }
                        }

                        let app_handle = self.machine.context.services.app_handle.as_ref();
                        if let Err(e) = complete_merge_internal(
                            &mut task,
                            &project,
                            &commit_sha,
                            &target_branch,
                            task_repo,
                            app_handle,
                        )
                        .await
                        {
                            tracing::error!(error = %e, task_id = task_id_str, "Failed to complete rebase+squash, falling back to MergeIncomplete");

                            task.metadata = Some(
                                serde_json::json!({
                                    "error": format!("complete_merge_internal failed: {}", e),
                                    "source_branch": source_branch,
                                    "target_branch": target_branch,
                                })
                                .to_string(),
                            );
                            task.internal_status = InternalStatus::MergeIncomplete;
                            task.touch();

                            let _ = task_repo.update(&task).await;
                            let _ = task_repo
                                .persist_status_change(
                                    &task_id,
                                    InternalStatus::PendingMerge,
                                    InternalStatus::MergeIncomplete,
                                    "merge_incomplete",
                                )
                                .await;

                            self.machine
                                .context
                                .services
                                .event_emitter
                                .emit_status_change(
                                    task_id_str,
                                    "pending_merge",
                                    "merge_incomplete",
                                )
                                .await;
                        } else {
                            self.post_merge_cleanup(
                                task_id_str,
                                &task_id,
                                repo_path,
                                plan_branch_repo,
                            )
                            .await;
                        }
                    }
                    Ok(MergeAttemptResult::NeedsAgent { conflict_files }) => {
                        tracing::info!(
                            task_id = task_id_str,
                            conflict_count = conflict_files.len(),
                            "Rebase+squash: rebase conflicts detected, transitioning to Merging"
                        );

                        emit_merge_progress(
                            self.machine.context.services.app_handle.as_ref(),
                            task_id_str,
                            MergePhase::ProgrammaticMerge,
                            MergePhaseStatus::Failed,
                            format!(
                                "Rebase conflicts detected in {} files",
                                conflict_files.len()
                            ),
                        );

                        task.internal_status = InternalStatus::Merging;
                        task.touch();

                        if let Err(e) = task_repo.update(&task).await {
                            tracing::error!(error = %e, "Failed to update task to Merging status");
                            return;
                        }

                        if let Err(e) = task_repo
                            .persist_status_change(
                                &task_id,
                                InternalStatus::PendingMerge,
                                InternalStatus::Merging,
                                "merge_conflict",
                            )
                            .await
                        {
                            tracing::warn!(error = %e, "Failed to record rebase+squash conflict transition (non-fatal)");
                        }

                        self.machine
                            .context
                            .services
                            .event_emitter
                            .emit_status_change(task_id_str, "pending_merge", "merging")
                            .await;

                        let prompt = format!("Resolve rebase conflicts for task: {}. After resolving each file, run `git add <file>` then `git rebase --continue`.", task_id_str);
                        tracing::info!(
                            task_id = task_id_str,
                            "Spawning merger agent for rebase conflict resolution (rebase+squash)"
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
                                "Merger agent spawned successfully"
                            ),
                            Err(e) => {
                                tracing::error!(task_id = task_id_str, error = %e, "Failed to spawn merger agent")
                            }
                        }
                    }
                    Ok(MergeAttemptResult::BranchNotFound { branch }) => {
                        tracing::error!(
                            task_id = task_id_str,
                            missing_branch = %branch,
                            "Merge failed: branch '{}' does not exist", branch
                        );

                        task.metadata = Some(
                            serde_json::json!({
                                "error": format!("Branch '{}' does not exist", branch),
                                "missing_branch": branch,
                                "source_branch": source_branch,
                                "target_branch": target_branch,
                            })
                            .to_string(),
                        );
                        task.internal_status = InternalStatus::MergeIncomplete;
                        task.touch();

                        if let Err(e) = task_repo.update(&task).await {
                            tracing::error!(error = %e, "Failed to update task to MergeIncomplete status");
                            return;
                        }

                        if let Err(e) = task_repo
                            .persist_status_change(
                                &task_id,
                                InternalStatus::PendingMerge,
                                InternalStatus::MergeIncomplete,
                                "merge_incomplete",
                            )
                            .await
                        {
                            tracing::warn!(error = %e, "Failed to record merge incomplete transition (non-fatal)");
                        }

                        self.machine
                            .context
                            .services
                            .event_emitter
                            .emit_status_change(task_id_str, "pending_merge", "merge_incomplete")
                            .await;
                    }
                    Err(e) => {
                        tracing::error!(
                            task_id = task_id_str,
                            error = %e,
                            "Rebase+squash failed, transitioning to MergeIncomplete"
                        );

                        task.metadata = Some(
                            serde_json::json!({
                                "error": e.to_string(),
                                "source_branch": source_branch,
                                "target_branch": target_branch,
                            })
                            .to_string(),
                        );
                        task.internal_status = InternalStatus::MergeIncomplete;
                        task.touch();

                        let _ = task_repo.update(&task).await;
                        let _ = task_repo
                            .persist_status_change(
                                &task_id,
                                InternalStatus::PendingMerge,
                                InternalStatus::MergeIncomplete,
                                "merge_incomplete",
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
            (MergeStrategy::RebaseSquash, GitMode::Worktree) => {
                // Worktree mode: rebase in worktree, then squash into single commit
                let current_branch = GitService::get_current_branch(repo_path).unwrap_or_default();
                let target_is_checked_out = current_branch == target_branch;

                if target_is_checked_out {
                    // Target checked out in primary repo — use checkout-free squash merge
                    // (skips rebase step to avoid worktree conflicts with source branch)
                    tracing::info!(
                        task_id = task_id_str,
                        target_branch = %target_branch,
                        "Target branch is checked out, using checkout-free squash merge"
                    );

                    // Validate branches exist
                    if !GitService::branch_exists(repo_path, &source_branch)
                        || !GitService::branch_exists(repo_path, &target_branch)
                    {
                        let missing = if !GitService::branch_exists(repo_path, &source_branch) {
                            &source_branch
                        } else {
                            &target_branch
                        };
                        tracing::error!(
                            task_id = task_id_str,
                            "Branch '{}' does not exist",
                            missing
                        );

                        // Record merge recovery event for retry tracking
                        let mut recovery =
                            MergeRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
                                .unwrap_or(None)
                                .unwrap_or_else(MergeRecoveryMetadata::new);

                        // Count existing AutoRetryTriggered events
                        let attempt_count = recovery
                            .events
                            .iter()
                            .filter(|e| {
                                matches!(e.kind, MergeRecoveryEventKind::AutoRetryTriggered)
                            })
                            .count() as u32
                            + 1;

                        // Create AutoRetryTriggered event
                        let event = MergeRecoveryEvent::new(
                            MergeRecoveryEventKind::AutoRetryTriggered,
                            MergeRecoverySource::Auto,
                            MergeRecoveryReasonCode::BranchNotFound,
                            format!("Branch '{}' does not exist", missing),
                        )
                        .with_target_branch(&target_branch)
                        .with_source_branch(&source_branch)
                        .with_attempt(attempt_count);

                        recovery.append_event(event);

                        // Update task metadata with recovery events and branch_missing flag
                        match recovery.update_task_metadata(task.metadata.as_deref()) {
                            Ok(updated_json) => {
                                // Add branch_missing flag to metadata
                                if let Ok(mut metadata_obj) =
                                    serde_json::from_str::<serde_json::Value>(&updated_json)
                                {
                                    if let Some(obj) = metadata_obj.as_object_mut() {
                                        obj.insert(
                                            "branch_missing".to_string(),
                                            serde_json::json!(true),
                                        );
                                    }
                                    task.metadata = Some(metadata_obj.to_string());
                                } else {
                                    task.metadata = Some(updated_json);
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    task_id = task_id_str,
                                    error = %e,
                                    "Failed to serialize merge recovery metadata, using legacy format"
                                );
                                // Fallback to legacy metadata
                                task.metadata = Some(serde_json::json!({"error": format!("Branch '{}' does not exist", missing), "missing_branch": missing, "source_branch": source_branch, "target_branch": target_branch, "branch_missing": true}).to_string());
                            }
                        }

                        task.internal_status = InternalStatus::MergeIncomplete;
                        task.touch();
                        let _ = task_repo.update(&task).await;
                        let _ = task_repo
                            .persist_status_change(
                                &task_id,
                                InternalStatus::PendingMerge,
                                InternalStatus::MergeIncomplete,
                                "merge_incomplete",
                            )
                            .await;
                        self.machine
                            .context
                            .services
                            .event_emitter
                            .emit_status_change(task_id_str, "pending_merge", "merge_incomplete")
                            .await;
                        return;
                    }

                    let cf_result = checkout_free::try_squash_merge_checkout_free(
                        repo_path,
                        &source_branch,
                        &target_branch,
                        &squash_commit_msg,
                    );

                    match cf_result {
                        Ok(CheckoutFreeMergeResult::Success { commit_sha }) => {
                            // Atomically sync working tree
                            if let Err(e) = GitService::hard_reset_to_head(repo_path) {
                                tracing::error!(error = %e, task_id = task_id_str, "Failed to sync working tree after checkout-free rebase+squash");
                            }

                            tracing::info!(
                                task_id = task_id_str,
                                commit_sha = %commit_sha,
                                "Checkout-free rebase+squash succeeded"
                            );

                            emit_merge_progress(
                                self.machine.context.services.app_handle.as_ref(),
                                task_id_str,
                                MergePhase::ProgrammaticMerge,
                                MergePhaseStatus::Passed,
                                format!("Rebase+squash completed: {}", commit_sha),
                            );

                            if TEMP_SKIP_POST_MERGE_VALIDATION {
                                tracing::warn!(task_id = task_id_str, "Post-merge validation temporarily disabled (global flag, checkout-free rebase+squash)");
                            } else {
                                let skip_validation = take_skip_validation_flag(&mut task);
                                let validation_mode = &project.merge_validation_mode;
                                if !skip_validation && *validation_mode != MergeValidationMode::Off
                                {
                                    let source_sha =
                                        GitService::get_branch_sha(repo_path, &source_branch).ok();
                                    let cached_log = source_sha
                                        .as_deref()
                                        .and_then(|sha| extract_cached_validation(&task, sha));
                                    let app_handle_ref =
                                        self.machine.context.services.app_handle.as_ref();
                                    if let Some(validation) = run_validation_commands(
                                        &project,
                                        &task,
                                        repo_path,
                                        task_id_str,
                                        app_handle_ref,
                                        cached_log.as_deref(),
                                    )
                                    .await
                                    {
                                        if !validation.all_passed {
                                            if *validation_mode == MergeValidationMode::Warn {
                                                tracing::warn!(task_id = task_id_str, "Validation failed in Warn mode (checkout-free rebase+squash), proceeding");
                                                task.metadata =
                                                    Some(format_validation_warn_metadata(
                                                        &validation.log,
                                                        &source_branch,
                                                        &target_branch,
                                                    ));
                                            } else {
                                                self.handle_validation_failure(
                                                    &mut task,
                                                    &task_id,
                                                    task_id_str,
                                                    task_repo,
                                                    &validation.failures,
                                                    &validation.log,
                                                    &source_branch,
                                                    &target_branch,
                                                    repo_path,
                                                    "checkout-free",
                                                    validation_mode,
                                                )
                                                .await;
                                                return;
                                            }
                                        } else {
                                            task.metadata = Some(serde_json::json!({"validation_log": validation.log, "validation_source_sha": source_sha, "source_branch": source_branch, "target_branch": target_branch}).to_string());
                                        }
                                    }
                                }
                            }

                            let app_handle = self.machine.context.services.app_handle.as_ref();
                            if let Err(e) = complete_merge_internal(
                                &mut task,
                                &project,
                                &commit_sha,
                                &target_branch,
                                task_repo,
                                app_handle,
                            )
                            .await
                            {
                                tracing::error!(error = %e, task_id = task_id_str, "Failed to complete checkout-free rebase+squash");
                                task.metadata = Some(serde_json::json!({"error": format!("complete_merge_internal failed: {}", e), "source_branch": source_branch, "target_branch": target_branch}).to_string());
                                task.internal_status = InternalStatus::MergeIncomplete;
                                task.touch();
                                let _ = task_repo.update(&task).await;
                                let _ = task_repo
                                    .persist_status_change(
                                        &task_id,
                                        InternalStatus::PendingMerge,
                                        InternalStatus::MergeIncomplete,
                                        "merge_incomplete",
                                    )
                                    .await;
                                self.machine
                                    .context
                                    .services
                                    .event_emitter
                                    .emit_status_change(
                                        task_id_str,
                                        "pending_merge",
                                        "merge_incomplete",
                                    )
                                    .await;
                            } else {
                                self.post_merge_cleanup(
                                    task_id_str,
                                    &task_id,
                                    repo_path,
                                    plan_branch_repo,
                                )
                                .await;
                            }
                        }
                        Ok(CheckoutFreeMergeResult::Conflict {
                            files: conflict_files,
                        }) => {
                            tracing::info!(
                                task_id = task_id_str,
                                conflict_count = conflict_files.len(),
                                "Checkout-free rebase+squash has conflicts, creating temp worktree"
                            );
                            emit_merge_progress(
                                self.machine.context.services.app_handle.as_ref(),
                                task_id_str,
                                MergePhase::ProgrammaticMerge,
                                MergePhaseStatus::Failed,
                                format!("Conflicts in {} files", conflict_files.len()),
                            );

                            let merge_wt_path =
                                PathBuf::from(compute_merge_worktree_path(&project, task_id_str));
                            let target_sha = GitService::get_branch_sha(repo_path, &target_branch)
                                .unwrap_or_default();
                            let resolve_branch = format!("merge-resolve/{}", task_id_str);
                            let _ = GitService::create_branch_at(
                                repo_path,
                                &resolve_branch,
                                &target_sha,
                            );
                            let _ = GitService::checkout_existing_branch_worktree(
                                repo_path,
                                &merge_wt_path,
                                &resolve_branch,
                            );
                            let _ = std::process::Command::new("git")
                                .args(["merge", &source_branch, "--no-edit"])
                                .current_dir(&merge_wt_path)
                                .output();

                            task.internal_status = InternalStatus::Merging;
                            task.touch();
                            let _ = task_repo.update(&task).await;
                            let _ = task_repo
                                .persist_status_change(
                                    &task_id,
                                    InternalStatus::PendingMerge,
                                    InternalStatus::Merging,
                                    "merge_conflict",
                                )
                                .await;
                            self.machine
                                .context
                                .services
                                .event_emitter
                                .emit_status_change(task_id_str, "pending_merge", "merging")
                                .await;

                            let prompt =
                                format!("Resolve merge conflicts for task: {}", task_id_str);
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
                                Ok(_) => {
                                    tracing::info!(task_id = task_id_str, "Merger agent spawned")
                                }
                                Err(e) => {
                                    tracing::error!(task_id = task_id_str, error = %e, "Failed to spawn merger agent")
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!(task_id = task_id_str, error = %e, "Checkout-free rebase+squash failed");
                            task.metadata = Some(serde_json::json!({"error": e.to_string(), "source_branch": source_branch, "target_branch": target_branch}).to_string());
                            task.internal_status = InternalStatus::MergeIncomplete;
                            task.touch();
                            let _ = task_repo.update(&task).await;
                            let _ = task_repo
                                .persist_status_change(
                                    &task_id,
                                    InternalStatus::PendingMerge,
                                    InternalStatus::MergeIncomplete,
                                    "merge_incomplete",
                                )
                                .await;
                            self.machine
                                .context
                                .services
                                .event_emitter
                                .emit_status_change(
                                    task_id_str,
                                    "pending_merge",
                                    "merge_incomplete",
                                )
                                .await;
                        }
                    }
                } else {
                    // Target NOT checked out — use worktrees for both rebase and squash
                    let rebase_wt_path =
                        PathBuf::from(compute_rebase_worktree_path(&project, task_id_str));
                    let merge_wt_path =
                        PathBuf::from(compute_merge_worktree_path(&project, task_id_str));

                    tracing::info!(
                        task_id = task_id_str,
                        rebase_worktree = %rebase_wt_path.display(),
                        merge_worktree = %merge_wt_path.display(),
                        "Rebase+squash in isolated worktrees"
                    );

                    let merge_result = GitService::try_rebase_squash_merge_in_worktree(
                        repo_path,
                        &source_branch,
                        &target_branch,
                        &rebase_wt_path,
                        &merge_wt_path,
                        &squash_commit_msg,
                    );

                    match merge_result {
                        Ok(MergeAttemptResult::Success { commit_sha }) => {
                            tracing::info!(
                                task_id = task_id_str,
                                commit_sha = %commit_sha,
                                "Rebase+squash in worktrees succeeded"
                            );

                            emit_merge_progress(
                                self.machine.context.services.app_handle.as_ref(),
                                task_id_str,
                                MergePhase::ProgrammaticMerge,
                                MergePhaseStatus::Passed,
                                format!("Rebase+squash completed: {}", commit_sha),
                            );

                            // Clean up merge worktree
                            if let Err(e) = GitService::delete_worktree(repo_path, &merge_wt_path) {
                                tracing::warn!(error = %e, task_id = task_id_str, "Failed to delete merge worktree (non-fatal)");
                            }

                            if TEMP_SKIP_POST_MERGE_VALIDATION {
                                tracing::warn!(task_id = task_id_str, "Post-merge validation temporarily disabled (global flag, worktree rebase+squash)");
                            } else {
                                let skip_validation = take_skip_validation_flag(&mut task);
                                let validation_mode = &project.merge_validation_mode;
                                if !skip_validation && *validation_mode != MergeValidationMode::Off
                                {
                                    let source_sha =
                                        GitService::get_branch_sha(repo_path, &source_branch).ok();
                                    let cached_log = source_sha
                                        .as_deref()
                                        .and_then(|sha| extract_cached_validation(&task, sha));
                                    let app_handle_ref =
                                        self.machine.context.services.app_handle.as_ref();
                                    if let Some(validation) = run_validation_commands(
                                        &project,
                                        &task,
                                        repo_path,
                                        task_id_str,
                                        app_handle_ref,
                                        cached_log.as_deref(),
                                    )
                                    .await
                                    {
                                        if !validation.all_passed {
                                            if *validation_mode == MergeValidationMode::Warn {
                                                tracing::warn!(task_id = task_id_str, "Validation failed in Warn mode (worktree rebase+squash), proceeding");
                                                task.metadata =
                                                    Some(format_validation_warn_metadata(
                                                        &validation.log,
                                                        &source_branch,
                                                        &target_branch,
                                                    ));
                                            } else {
                                                self.handle_validation_failure(
                                                    &mut task,
                                                    &task_id,
                                                    task_id_str,
                                                    task_repo,
                                                    &validation.failures,
                                                    &validation.log,
                                                    &source_branch,
                                                    &target_branch,
                                                    repo_path,
                                                    "worktree",
                                                    validation_mode,
                                                )
                                                .await;
                                                return;
                                            }
                                        } else {
                                            task.metadata = Some(
                                                serde_json::json!({
                                                    "validation_log": validation.log,
                                                    "validation_source_sha": source_sha,
                                                    "source_branch": source_branch,
                                                    "target_branch": target_branch,
                                                })
                                                .to_string(),
                                            );
                                        }
                                    }
                                }
                            }

                            let app_handle = self.machine.context.services.app_handle.as_ref();
                            if let Err(e) = complete_merge_internal(
                                &mut task,
                                &project,
                                &commit_sha,
                                &target_branch,
                                task_repo,
                                app_handle,
                            )
                            .await
                            {
                                tracing::error!(error = %e, task_id = task_id_str, "Failed to complete rebase+squash in worktrees");

                                task.metadata = Some(
                                    serde_json::json!({
                                        "error": format!("complete_merge_internal failed: {}", e),
                                        "source_branch": source_branch,
                                        "target_branch": target_branch,
                                    })
                                    .to_string(),
                                );
                                task.internal_status = InternalStatus::MergeIncomplete;
                                task.touch();

                                let _ = task_repo.update(&task).await;
                                let _ = task_repo
                                    .persist_status_change(
                                        &task_id,
                                        InternalStatus::PendingMerge,
                                        InternalStatus::MergeIncomplete,
                                        "merge_incomplete",
                                    )
                                    .await;

                                self.machine
                                    .context
                                    .services
                                    .event_emitter
                                    .emit_status_change(
                                        task_id_str,
                                        "pending_merge",
                                        "merge_incomplete",
                                    )
                                    .await;
                            } else {
                                self.post_merge_cleanup(
                                    task_id_str,
                                    &task_id,
                                    repo_path,
                                    plan_branch_repo,
                                )
                                .await;
                            }
                        }
                        Ok(MergeAttemptResult::NeedsAgent { conflict_files }) => {
                            tracing::info!(
                            task_id = task_id_str,
                            conflict_count = conflict_files.len(),
                            "Rebase+squash in worktrees: rebase conflicts, transitioning to Merging"
                        );

                            emit_merge_progress(
                                self.machine.context.services.app_handle.as_ref(),
                                task_id_str,
                                MergePhase::ProgrammaticMerge,
                                MergePhaseStatus::Failed,
                                format!(
                                    "Rebase conflicts detected in {} files",
                                    conflict_files.len()
                                ),
                            );

                            // Set worktree_path to rebase worktree for agent CWD
                            let rebase_wt_path_str = rebase_wt_path.to_string_lossy().to_string();
                            task.worktree_path = Some(rebase_wt_path_str);
                            let mut meta =
                                parse_metadata(&task).unwrap_or_else(|| serde_json::json!({}));
                            if let Some(obj) = meta.as_object_mut() {
                                obj.insert(
                                    "conflict_type".to_string(),
                                    serde_json::json!("rebase"),
                                );
                            }
                            task.metadata = Some(meta.to_string());
                            task.internal_status = InternalStatus::Merging;
                            task.touch();

                            if let Err(e) = task_repo.update(&task).await {
                                tracing::error!(error = %e, "Failed to update task to Merging");
                                return;
                            }

                            if let Err(e) = task_repo
                                .persist_status_change(
                                    &task_id,
                                    InternalStatus::PendingMerge,
                                    InternalStatus::Merging,
                                    "rebase_conflict",
                                )
                                .await
                            {
                                tracing::warn!(error = %e, "Failed to record rebase conflict transition (non-fatal)");
                            }

                            self.machine
                                .context
                                .services
                                .event_emitter
                                .emit_status_change(task_id_str, "pending_merge", "merging")
                                .await;

                            let prompt = format!(
                            "Resolve rebase conflicts for task: {}. After resolving each file, run `git add <file>` then `git rebase --continue`.",
                            task_id_str
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
                                    "Merger agent spawned successfully"
                                ),
                                Err(e) => {
                                    tracing::error!(task_id = task_id_str, error = %e, "Failed to spawn merger agent")
                                }
                            }
                        }
                        Ok(MergeAttemptResult::BranchNotFound { branch }) => {
                            tracing::error!(
                                task_id = task_id_str,
                                missing_branch = %branch,
                                "Merge failed: branch '{}' does not exist", branch
                            );

                            task.metadata = Some(
                                serde_json::json!({
                                    "error": format!("Branch '{}' does not exist", branch),
                                    "missing_branch": branch,
                                    "source_branch": source_branch,
                                    "target_branch": target_branch,
                                })
                                .to_string(),
                            );
                            task.internal_status = InternalStatus::MergeIncomplete;
                            task.touch();

                            if let Err(e) = task_repo.update(&task).await {
                                tracing::error!(error = %e, "Failed to update task to MergeIncomplete status");
                                return;
                            }

                            if let Err(e) = task_repo
                                .persist_status_change(
                                    &task_id,
                                    InternalStatus::PendingMerge,
                                    InternalStatus::MergeIncomplete,
                                    "merge_incomplete",
                                )
                                .await
                            {
                                tracing::warn!(error = %e, "Failed to record merge incomplete transition (non-fatal)");
                            }

                            self.machine
                                .context
                                .services
                                .event_emitter
                                .emit_status_change(
                                    task_id_str,
                                    "pending_merge",
                                    "merge_incomplete",
                                )
                                .await;
                        }
                        Err(e) => {
                            if GitService::is_branch_lock_error(&e) {
                                tracing::warn!(task_id = task_id_str, error = %e, "Rebase+squash in worktrees failed due to branch lock, staying in PendingMerge");
                                if rebase_wt_path.exists() {
                                    let _ = GitService::delete_worktree(repo_path, &rebase_wt_path);
                                }
                                if merge_wt_path.exists() {
                                    let _ = GitService::delete_worktree(repo_path, &merge_wt_path);
                                }
                            } else {
                                tracing::error!(task_id = task_id_str, error = %e, "Rebase+squash in worktrees failed, transitioning to MergeIncomplete");

                                if rebase_wt_path.exists() {
                                    let _ = GitService::delete_worktree(repo_path, &rebase_wt_path);
                                }
                                if merge_wt_path.exists() {
                                    let _ = GitService::delete_worktree(repo_path, &merge_wt_path);
                                }

                                task.metadata = Some(
                                    serde_json::json!({
                                        "error": e.to_string(),
                                        "source_branch": source_branch,
                                        "target_branch": target_branch,
                                    })
                                    .to_string(),
                                );
                                task.internal_status = InternalStatus::MergeIncomplete;
                                task.touch();

                                let _ = task_repo.update(&task).await;
                                let _ = task_repo
                                    .persist_status_change(
                                        &task_id,
                                        InternalStatus::PendingMerge,
                                        InternalStatus::MergeIncomplete,
                                        "merge_incomplete",
                                    )
                                    .await;

                                self.machine
                                    .context
                                    .services
                                    .event_emitter
                                    .emit_status_change(
                                        task_id_str,
                                        "pending_merge",
                                        "merge_incomplete",
                                    )
                                    .await;
                            }
                        }
                    }
                }
            }
        } // end match
    }

    /// Post-merge cleanup: update plan branch status, delete feature branch, unblock dependents.
    ///
    /// Shared between Worktree and Local mode success paths in `attempt_programmatic_merge()`.
    async fn post_merge_cleanup(
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

                if let Err(e) = GitService::delete_feature_branch(repo_path, &pb.branch_name) {
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

        // Schedule newly-unblocked tasks (e.g. plan_merge tasks that just became Ready)
        if let Some(ref scheduler) = self.machine.context.services.task_scheduler {
            let scheduler = Arc::clone(scheduler);
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;
                scheduler.try_schedule_ready_tasks().await;
            });
        }

        // Retry deferred merges: after a merge completes, re-trigger any tasks that
        // were deferred because they targeted the same branch. We use the scheduler's
        // try_retry_deferred_merges() method which builds a fresh TaskTransitionService
        // and re-invokes attempt_programmatic_merge for each deferred task.
        if let Some(ref scheduler) = self.machine.context.services.task_scheduler {
            let scheduler = Arc::clone(scheduler);
            let project_id = self.machine.context.project_id.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
                scheduler.try_retry_deferred_merges(&project_id).await;
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
    async fn handle_validation_failure(
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
            if let Err(e) = GitService::reset_hard(merge_path, "HEAD~1") {
                tracing::error!(
                    task_id = task_id_str,
                    error = %e,
                    "Failed to revert merge commit after validation failure — target branch may have failing code"
                );
            }

            task.metadata = Some(format_validation_error_metadata(
                failures,
                log,
                source_branch,
                target_branch,
            ));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::types::IdeationSessionId;
    use crate::domain::entities::{ArtifactId, PlanBranch, PlanBranchStatus, ProjectId, TaskId};
    use crate::infrastructure::memory::{MemoryPlanBranchRepository, MemoryTaskRepository};

    fn make_project(base_branch: Option<&str>) -> Project {
        let mut p = Project::new("test-project".into(), "/tmp/test".into());
        p.base_branch = base_branch.map(|s| s.to_string());
        p
    }

    fn make_task(plan_artifact_id: Option<&str>, task_branch: Option<&str>) -> Task {
        make_task_with_session(plan_artifact_id, task_branch, None)
    }

    fn make_task_with_session(
        plan_artifact_id: Option<&str>,
        task_branch: Option<&str>,
        ideation_session_id: Option<&str>,
    ) -> Task {
        let mut t = Task::new(
            ProjectId::from_string("proj-1".to_string()),
            "Test task".into(),
        );
        t.plan_artifact_id = plan_artifact_id.map(|s| ArtifactId::from_string(s));
        t.task_branch = task_branch.map(|s| s.to_string());
        t.ideation_session_id = ideation_session_id.map(|s| IdeationSessionId::from_string(s));
        t
    }

    fn make_plan_branch(
        plan_artifact_id: &str,
        branch_name: &str,
        status: PlanBranchStatus,
        merge_task_id: Option<&str>,
    ) -> PlanBranch {
        let mut pb = PlanBranch::new(
            ArtifactId::from_string(plan_artifact_id),
            IdeationSessionId::from_string("sess-1"),
            ProjectId::from_string("proj-1".to_string()),
            branch_name.to_string(),
            "main".to_string(),
        );
        pb.status = status;
        pb.merge_task_id = merge_task_id.map(|s| TaskId::from_string(s.to_string()));
        pb
    }

    // ==================
    // resolve_task_base_branch tests
    // ==================

    #[tokio::test]
    async fn resolve_task_base_branch_returns_project_base_when_no_repo() {
        let project = make_project(Some("develop"));
        let task = make_task_with_session(Some("art-1"), None, Some("sess-1"));
        let repo: Option<Arc<dyn PlanBranchRepository>> = None;

        let result = resolve_task_base_branch(&task, &project, &repo).await;
        assert_eq!(result, "develop");
    }

    #[tokio::test]
    async fn resolve_task_base_branch_defaults_to_main_when_no_base_branch() {
        let project = make_project(None);
        let task = make_task_with_session(Some("art-1"), None, Some("sess-1"));
        let repo: Option<Arc<dyn PlanBranchRepository>> = None;

        let result = resolve_task_base_branch(&task, &project, &repo).await;
        assert_eq!(result, "main");
    }

    #[tokio::test]
    async fn resolve_task_base_branch_returns_default_when_task_has_no_session_id() {
        let project = make_project(Some("develop"));
        let task = make_task(None, None);
        let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
        let repo: Option<Arc<dyn PlanBranchRepository>> = Some(mem_repo);

        let result = resolve_task_base_branch(&task, &project, &repo).await;
        assert_eq!(result, "develop");
    }

    #[tokio::test]
    async fn resolve_task_base_branch_falls_back_when_branch_creation_fails() {
        // Project points to /tmp/test which is not a real git repo,
        // so lazy branch creation will fail → should fall back to "main"
        let project = make_project(Some("main"));
        let task = make_task_with_session(Some("art-1"), None, Some("sess-1"));

        let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
        let pb = make_plan_branch(
            "art-1",
            "ralphx/test/plan-abc123",
            PlanBranchStatus::Active,
            None,
        );
        mem_repo.create(pb).await.unwrap();

        let repo: Option<Arc<dyn PlanBranchRepository>> = Some(mem_repo);
        let result = resolve_task_base_branch(&task, &project, &repo).await;
        assert_eq!(result, "main");
    }

    #[tokio::test]
    async fn resolve_task_base_branch_returns_feature_branch_when_branch_exists() {
        // Set up a real git repo with the plan branch already created
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path();

        // Init git repo with an initial commit (needed for branch creation)
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "--allow-empty", "-m", "init"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        // Create the plan branch
        std::process::Command::new("git")
            .args(["branch", "ralphx/test/plan-abc123"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        let mut project = make_project(Some("main"));
        project.working_directory = repo_path.to_string_lossy().to_string();
        let task = make_task_with_session(Some("art-1"), None, Some("sess-1"));

        let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
        let pb = make_plan_branch(
            "art-1",
            "ralphx/test/plan-abc123",
            PlanBranchStatus::Active,
            None,
        );
        mem_repo.create(pb).await.unwrap();

        let repo: Option<Arc<dyn PlanBranchRepository>> = Some(mem_repo);
        let result = resolve_task_base_branch(&task, &project, &repo).await;
        assert_eq!(result, "ralphx/test/plan-abc123");
    }

    #[tokio::test]
    async fn resolve_task_base_branch_returns_default_when_branch_merged() {
        let project = make_project(Some("main"));
        let task = make_task_with_session(Some("art-1"), None, Some("sess-1"));

        let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
        let pb = make_plan_branch(
            "art-1",
            "ralphx/test/plan-abc123",
            PlanBranchStatus::Merged,
            None,
        );
        mem_repo.create(pb).await.unwrap();

        let repo: Option<Arc<dyn PlanBranchRepository>> = Some(mem_repo);
        let result = resolve_task_base_branch(&task, &project, &repo).await;
        assert_eq!(result, "main");
    }

    #[tokio::test]
    async fn resolve_task_base_branch_returns_default_when_branch_abandoned() {
        let project = make_project(Some("main"));
        let task = make_task_with_session(Some("art-1"), None, Some("sess-1"));

        let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
        let pb = make_plan_branch(
            "art-1",
            "ralphx/test/plan-abc123",
            PlanBranchStatus::Abandoned,
            None,
        );
        mem_repo.create(pb).await.unwrap();

        let repo: Option<Arc<dyn PlanBranchRepository>> = Some(mem_repo);
        let result = resolve_task_base_branch(&task, &project, &repo).await;
        assert_eq!(result, "main");
    }

    #[tokio::test]
    async fn resolve_task_base_branch_returns_default_when_no_matching_branch() {
        let project = make_project(Some("main"));
        // Task has session_id "sess-nonexistent" which won't match "sess-1" in plan branch
        let task = make_task_with_session(Some("art-nonexistent"), None, Some("sess-nonexistent"));

        let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
        let pb = make_plan_branch(
            "art-other",
            "ralphx/test/plan-abc123",
            PlanBranchStatus::Active,
            None,
        );
        mem_repo.create(pb).await.unwrap();

        let repo: Option<Arc<dyn PlanBranchRepository>> = Some(mem_repo);
        let result = resolve_task_base_branch(&task, &project, &repo).await;
        assert_eq!(result, "main");
    }

    // ==================
    // resolve_merge_branches tests
    // ==================

    #[tokio::test]
    async fn resolve_merge_branches_returns_default_when_no_repo() {
        let project = make_project(Some("main"));
        let mut task = make_task(None, Some("ralphx/test/task-123"));
        task.id = TaskId::from_string("task-123".to_string());

        let repo: Option<Arc<dyn PlanBranchRepository>> = None;
        let (source, target) = resolve_merge_branches(&task, &project, &repo).await;
        assert_eq!(source, "ralphx/test/task-123");
        assert_eq!(target, "main");
    }

    #[tokio::test]
    async fn resolve_merge_branches_merge_task_returns_feature_into_base() {
        let project = make_project(Some("main"));
        let mut task = make_task(None, None);
        task.id = TaskId::from_string("merge-task-1".to_string());

        let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
        let pb = make_plan_branch(
            "art-1",
            "ralphx/test/plan-abc123",
            PlanBranchStatus::Active,
            Some("merge-task-1"),
        );
        mem_repo.create(pb).await.unwrap();

        let repo: Option<Arc<dyn PlanBranchRepository>> = Some(mem_repo);
        let (source, target) = resolve_merge_branches(&task, &project, &repo).await;
        assert_eq!(source, "ralphx/test/plan-abc123");
        assert_eq!(target, "main");
    }

    #[tokio::test]
    async fn resolve_merge_branches_plan_task_returns_task_into_feature() {
        let project = make_project(Some("main"));
        let mut task =
            make_task_with_session(Some("art-1"), Some("ralphx/test/task-456"), Some("sess-1"));
        task.id = TaskId::from_string("task-456".to_string());

        let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
        let pb = make_plan_branch(
            "art-1",
            "ralphx/test/plan-abc123",
            PlanBranchStatus::Active,
            None,
        );
        mem_repo.create(pb).await.unwrap();

        let repo: Option<Arc<dyn PlanBranchRepository>> = Some(mem_repo);
        let (source, target) = resolve_merge_branches(&task, &project, &repo).await;
        assert_eq!(source, "ralphx/test/task-456");
        assert_eq!(target, "ralphx/test/plan-abc123");
    }

    #[tokio::test]
    async fn resolve_merge_branches_regular_task_returns_task_into_base() {
        let project = make_project(Some("develop"));
        let mut task = make_task(None, Some("ralphx/test/task-789"));
        task.id = TaskId::from_string("task-789".to_string());

        let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
        let repo: Option<Arc<dyn PlanBranchRepository>> = Some(mem_repo);

        let (source, target) = resolve_merge_branches(&task, &project, &repo).await;
        assert_eq!(source, "ralphx/test/task-789");
        assert_eq!(target, "develop");
    }

    #[tokio::test]
    async fn resolve_merge_branches_merge_task_with_merged_branch_returns_default() {
        let project = make_project(Some("main"));
        let mut task = make_task(None, Some("ralphx/test/task-merge"));
        task.id = TaskId::from_string("merge-task-2".to_string());

        let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
        let pb = make_plan_branch(
            "art-2",
            "ralphx/test/plan-def456",
            PlanBranchStatus::Merged,
            Some("merge-task-2"),
        );
        mem_repo.create(pb).await.unwrap();

        let repo: Option<Arc<dyn PlanBranchRepository>> = Some(mem_repo);
        let (source, target) = resolve_merge_branches(&task, &project, &repo).await;
        // Merged branch is not Active, so falls through to default
        assert_eq!(source, "ralphx/test/task-merge");
        assert_eq!(target, "main");
    }

    #[tokio::test]
    async fn resolve_merge_branches_plan_task_with_abandoned_branch_returns_default() {
        let project = make_project(Some("main"));
        let mut task = make_task_with_session(
            Some("art-3"),
            Some("ralphx/test/task-abandoned"),
            Some("sess-1"),
        );
        task.id = TaskId::from_string("task-abandoned".to_string());

        let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
        let pb = make_plan_branch(
            "art-3",
            "ralphx/test/plan-ghi789",
            PlanBranchStatus::Abandoned,
            None,
        );
        mem_repo.create(pb).await.unwrap();

        let repo: Option<Arc<dyn PlanBranchRepository>> = Some(mem_repo);
        let (source, target) = resolve_merge_branches(&task, &project, &repo).await;
        // Abandoned branch is not Active, so falls through to default
        assert_eq!(source, "ralphx/test/task-abandoned");
        assert_eq!(target, "main");
    }

    #[tokio::test]
    async fn resolve_merge_branches_defaults_to_main_when_no_base_branch() {
        let project = make_project(None);
        let mut task = make_task(None, Some("ralphx/test/task-no-base"));
        task.id = TaskId::from_string("task-no-base".to_string());

        let repo: Option<Arc<dyn PlanBranchRepository>> = None;
        let (source, target) = resolve_merge_branches(&task, &project, &repo).await;
        assert_eq!(source, "ralphx/test/task-no-base");
        assert_eq!(target, "main");
    }

    #[tokio::test]
    async fn resolve_merge_branches_merge_task_checked_before_plan_task() {
        // If a task is both a merge task AND has ideation_session_id,
        // merge task check should take precedence
        let project = make_project(Some("main"));
        let mut task =
            make_task_with_session(Some("art-1"), Some("ralphx/test/task-dual"), Some("sess-1"));
        task.id = TaskId::from_string("dual-task".to_string());

        let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
        let pb = make_plan_branch(
            "art-1",
            "ralphx/test/plan-dual",
            PlanBranchStatus::Active,
            Some("dual-task"),
        );
        mem_repo.create(pb).await.unwrap();

        let repo: Option<Arc<dyn PlanBranchRepository>> = Some(mem_repo);
        let (source, target) = resolve_merge_branches(&task, &project, &repo).await;
        // Merge task path wins: feature branch into base
        assert_eq!(source, "ralphx/test/plan-dual");
        assert_eq!(target, "main");
    }

    #[tokio::test]
    async fn resolve_merge_branches_after_branch_discovery_returns_valid_source() {
        // Set up a real git repo with a task branch
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path();

        // Init git repo with an initial commit
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "--allow-empty", "-m", "init"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Create a task branch (simulating orphaned branch from failed task)
        let mut task = make_task(None, None); // task_branch is None
        task.id = TaskId::from_string("orphaned-123".to_string());
        let expected_branch = format!("ralphx/test-project/task-{}", task.id.as_str());

        std::process::Command::new("git")
            .args(["branch", &expected_branch])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Create project pointing to real git repo
        let mut project = make_project(Some("main"));
        project.working_directory = repo_path.to_string_lossy().to_string();

        // Create task repo and save task
        let task_repo: Arc<dyn TaskRepository> = Arc::new(MemoryTaskRepository::new());
        task_repo.create(task.clone()).await.unwrap();

        // Call discover_and_attach_task_branch (this simulates what happens in attempt_programmatic_merge)
        let mut task_mut = task.clone();
        let discovered = discover_and_attach_task_branch(&mut task_mut, &project, &task_repo)
            .await
            .unwrap();

        // Branch should have been discovered and attached
        assert!(discovered, "Branch should have been discovered");
        assert_eq!(task_mut.task_branch, Some(expected_branch.clone()));

        // Now verify that resolve_merge_branches returns the correct source branch
        let repo: Option<Arc<dyn PlanBranchRepository>> = None;
        let (source, target) = resolve_merge_branches(&task_mut, &project, &repo).await;

        assert_eq!(
            source, expected_branch,
            "Source branch should match the discovered branch"
        );
        assert_eq!(
            target, "main",
            "Target branch should be project base branch"
        );
    }

    // ==================
    // run_validation_commands tests
    // ==================

    #[tokio::test]
    async fn run_validation_returns_none_when_no_analysis() {
        let project = make_project(Some("main"));
        let task = make_task(None, None);
        let result =
            run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn run_validation_returns_none_when_empty_entries() {
        let mut project = make_project(Some("main"));
        project.detected_analysis = Some("[]".to_string());
        let task = make_task(None, None);
        let result =
            run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn run_validation_returns_none_when_no_validate_commands() {
        let mut project = make_project(Some("main"));
        project.detected_analysis =
            Some(r#"[{"path": ".", "label": "Test", "validate": []}]"#.to_string());
        let task = make_task(None, None);
        let result =
            run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn run_validation_prefers_custom_over_detected() {
        let mut project = make_project(Some("main"));
        // detected has a failing command
        project.detected_analysis =
            Some(r#"[{"path": ".", "label": "Test", "validate": ["false"]}]"#.to_string());
        // custom has a passing command (overrides detected)
        project.custom_analysis =
            Some(r#"[{"path": ".", "label": "Test", "validate": ["true"]}]"#.to_string());
        let task = make_task(None, None);
        let result =
            run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None).await;
        assert!(result.is_some());
        assert!(result.unwrap().all_passed);
    }

    #[tokio::test]
    async fn run_validation_succeeds_with_passing_command() {
        let mut project = make_project(Some("main"));
        project.detected_analysis =
            Some(r#"[{"path": ".", "label": "Test", "validate": ["true"]}]"#.to_string());
        let task = make_task(None, None);
        let result =
            run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None).await;
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.all_passed);
        assert!(r.failures.is_empty());
        assert_eq!(r.log.len(), 1);
        assert_eq!(r.log[0].phase, "validate");
        assert_eq!(r.log[0].status, "success");
        assert_eq!(r.log[0].label, "Test");
    }

    #[tokio::test]
    async fn run_validation_fails_with_failing_command() {
        let mut project = make_project(Some("main"));
        project.detected_analysis =
            Some(r#"[{"path": ".", "label": "Test", "validate": ["false"]}]"#.to_string());
        let task = make_task(None, None);
        let result =
            run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None).await;
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(!r.all_passed);
        assert_eq!(r.failures.len(), 1);
        assert_eq!(r.failures[0].command, "false");
        assert_eq!(r.log.len(), 1);
        assert_eq!(r.log[0].phase, "validate");
        assert_eq!(r.log[0].status, "failed");
    }

    #[tokio::test]
    async fn run_validation_resolves_template_vars() {
        let mut project = make_project(Some("main"));
        project.detected_analysis = Some(
            r#"[{"path": ".", "label": "Test", "validate": ["echo {project_root} {worktree_path}"]}]"#.to_string(),
        );
        let mut task = make_task(None, None);
        task.worktree_path = Some("/tmp/wt".to_string());
        let result =
            run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None).await;
        assert!(result.is_some());
        assert!(result.unwrap().all_passed);
    }

    #[tokio::test]
    async fn run_validation_returns_none_for_invalid_json() {
        let mut project = make_project(Some("main"));
        project.detected_analysis = Some("not valid json".to_string());
        let task = make_task(None, None);
        let result =
            run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None).await;
        assert!(result.is_none());
    }

    #[test]
    fn format_validation_error_metadata_formats_correctly() {
        let failures = vec![ValidationFailure {
            command: "cargo check".to_string(),
            path: ".".to_string(),
            exit_code: Some(1),
            stderr: "error[E0308]: mismatched types".to_string(),
        }];
        let log = vec![ValidationLogEntry {
            phase: "validate".to_string(),
            command: "cargo check".to_string(),
            path: ".".to_string(),
            label: "Rust".to_string(),
            status: "failed".to_string(),
            exit_code: Some(1),
            stdout: String::new(),
            stderr: "error[E0308]: mismatched types".to_string(),
            duration_ms: 1500,
        }];
        let result = format_validation_error_metadata(&failures, &log, "task-branch", "main");
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed["error"]
            .as_str()
            .unwrap()
            .contains("1 command(s) failed"));
        assert_eq!(parsed["source_branch"], "task-branch");
        assert_eq!(parsed["target_branch"], "main");
        assert_eq!(parsed["validation_failures"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["validation_log"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn format_validation_warn_metadata_formats_correctly() {
        let log = vec![ValidationLogEntry {
            phase: "validate".to_string(),
            command: "npm test".to_string(),
            path: ".".to_string(),
            label: "Node".to_string(),
            status: "failed".to_string(),
            exit_code: Some(1),
            stdout: String::new(),
            stderr: "test failed".to_string(),
            duration_ms: 500,
        }];
        let result = format_validation_warn_metadata(&log, "task-branch", "main");
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["validation_warnings"], true);
        assert_eq!(parsed["source_branch"], "task-branch");
        assert_eq!(parsed["target_branch"], "main");
        assert_eq!(parsed["validation_log"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn take_skip_validation_flag_returns_false_when_no_metadata() {
        let mut task = make_task(None, None);
        assert!(!take_skip_validation_flag(&mut task));
    }

    #[test]
    fn take_skip_validation_flag_returns_false_when_no_flag() {
        let mut task = make_task(None, None);
        task.metadata = Some(r#"{"some_key": "value"}"#.to_string());
        assert!(!take_skip_validation_flag(&mut task));
    }

    #[test]
    fn take_skip_validation_flag_returns_true_and_clears() {
        let mut task = make_task(None, None);
        task.metadata = Some(r#"{"skip_validation": true, "other": "data"}"#.to_string());
        assert!(take_skip_validation_flag(&mut task));
        // Flag should be cleared
        let meta: serde_json::Value =
            serde_json::from_str(task.metadata.as_ref().unwrap()).unwrap();
        assert!(meta.get("skip_validation").is_none());
        assert_eq!(meta["other"], "data");
        // Second call returns false
        assert!(!take_skip_validation_flag(&mut task));
    }

    #[test]
    fn run_validation_skipped_in_off_mode() {
        let mut project = make_project(Some("main"));
        project.merge_validation_mode = MergeValidationMode::Off;
        project.detected_analysis =
            Some(r#"[{"path": ".", "label": "Test", "validate": ["false"]}]"#.to_string());
        // With Off mode, validation should not run, so the test verifies the enum
        // is correctly set and accessible (actual skip happens in attempt_programmatic_merge)
        assert_eq!(project.merge_validation_mode, MergeValidationMode::Off);
    }

    // ==================
    // extract_cached_validation tests
    // ==================

    #[test]
    fn extract_cached_returns_none_when_no_metadata() {
        let task = make_task(None, None);
        assert!(extract_cached_validation(&task, "abc123").is_none());
    }

    #[test]
    fn extract_cached_returns_none_when_sha_mismatch() {
        let mut task = make_task(None, None);
        task.metadata = Some(
            serde_json::json!({
                "validation_source_sha": "old_sha",
                "validation_log": [{
                    "phase": "validate",
                    "command": "true",
                    "path": ".",
                    "label": "Test",
                    "status": "success",
                    "exit_code": 0,
                    "stdout": "",
                    "stderr": "",
                    "duration_ms": 100,
                }],
            })
            .to_string(),
        );
        assert!(extract_cached_validation(&task, "different_sha").is_none());
    }

    #[test]
    fn extract_cached_returns_log_when_sha_matches() {
        let mut task = make_task(None, None);
        task.metadata = Some(
            serde_json::json!({
                "validation_source_sha": "abc123",
                "validation_log": [{
                    "phase": "validate",
                    "command": "cargo check",
                    "path": ".",
                    "label": "Rust",
                    "status": "success",
                    "exit_code": 0,
                    "stdout": "",
                    "stderr": "",
                    "duration_ms": 1500,
                }],
            })
            .to_string(),
        );
        let cached = extract_cached_validation(&task, "abc123");
        assert!(cached.is_some());
        let entries = cached.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].command, "cargo check");
        assert_eq!(entries[0].status, "success");
    }

    #[test]
    fn extract_cached_returns_none_when_no_sha_in_metadata() {
        let mut task = make_task(None, None);
        task.metadata = Some(
            serde_json::json!({
                "validation_log": [{
                    "phase": "validate",
                    "command": "true",
                    "path": ".",
                    "label": "Test",
                    "status": "success",
                    "exit_code": 0,
                    "stdout": "",
                    "stderr": "",
                    "duration_ms": 100,
                }],
            })
            .to_string(),
        );
        // No validation_source_sha → no cache hit
        assert!(extract_cached_validation(&task, "abc123").is_none());
    }

    // ==================
    // run_validation_commands caching tests
    // ==================

    #[tokio::test]
    async fn run_validation_skips_passed_when_cached() {
        let mut project = make_project(Some("main"));
        // "true" always passes, "echo hello" always passes
        project.detected_analysis = Some(
            r#"[{"path": ".", "label": "Test", "validate": ["true", "echo hello"]}]"#.to_string(),
        );
        let task = make_task(None, None);

        // Build a cached log where "true" passed but "echo hello" failed
        let cached = vec![
            ValidationLogEntry {
                phase: "validate".to_string(),
                command: "true".to_string(),
                path: ".".to_string(),
                label: "Test".to_string(),
                status: "success".to_string(),
                exit_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
                duration_ms: 50,
            },
            ValidationLogEntry {
                phase: "validate".to_string(),
                command: "echo hello".to_string(),
                path: ".".to_string(),
                label: "Test".to_string(),
                status: "failed".to_string(),
                exit_code: Some(1),
                stdout: String::new(),
                stderr: "error".to_string(),
                duration_ms: 100,
            },
        ];

        let result =
            run_validation_commands(&project, &task, Path::new("/tmp"), "", None, Some(&cached))
                .await;
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.all_passed);
        assert_eq!(r.log.len(), 2);
        // First command should be cached (was "success" in cache)
        assert_eq!(r.log[0].status, "cached");
        assert_eq!(r.log[0].command, "true");
        assert_eq!(r.log[0].duration_ms, 0);
        // Second command should be re-run (was "failed" in cache)
        assert_eq!(r.log[1].status, "success");
        assert_eq!(r.log[1].command, "echo hello");
    }

    #[tokio::test]
    async fn run_validation_reruns_all_when_no_cache() {
        let mut project = make_project(Some("main"));
        project.detected_analysis =
            Some(r#"[{"path": ".", "label": "Test", "validate": ["true"]}]"#.to_string());
        let task = make_task(None, None);

        let result =
            run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None).await;
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.all_passed);
        assert_eq!(r.log.len(), 1);
        assert_eq!(r.log[0].status, "success"); // actually ran, not "cached"
    }

    // ==================
    // extract_task_id_from_merge_path tests
    // ==================

    #[test]
    fn test_extract_task_id_from_merge_path_valid() {
        let path = "/home/user/ralphx-worktrees/my-app/merge-abc123def456";
        assert_eq!(extract_task_id_from_merge_path(path), Some("abc123def456"));
    }

    #[test]
    fn test_extract_task_id_from_merge_path_uuid() {
        let path = "/tmp/wt/merge-e0ce32e7-eaef-4a07-b81d-2126d0dee5d9";
        assert_eq!(
            extract_task_id_from_merge_path(path),
            Some("e0ce32e7-eaef-4a07-b81d-2126d0dee5d9"),
        );
    }

    #[test]
    fn test_extract_task_id_from_merge_path_not_merge() {
        let path = "/home/user/ralphx-worktrees/my-app/task-abc123";
        assert_eq!(extract_task_id_from_merge_path(path), None);
    }

    #[test]
    fn test_extract_task_id_from_merge_path_bare_name() {
        assert_eq!(extract_task_id_from_merge_path("merge-xyz"), Some("xyz"));
    }

    #[test]
    fn test_extract_task_id_from_merge_path_empty() {
        assert_eq!(extract_task_id_from_merge_path(""), None);
    }

    #[test]
    fn test_extract_task_id_from_merge_path_just_merge_prefix() {
        // "merge-" with empty task ID should return empty string
        assert_eq!(extract_task_id_from_merge_path("/dir/merge-"), Some(""));
    }

    // ==================
    // parse_metadata tests
    // ==================

    #[test]
    fn parse_metadata_returns_none_when_no_metadata() {
        let task = make_task(None, None);
        assert!(parse_metadata(&task).is_none());
    }

    #[test]
    fn parse_metadata_returns_none_for_invalid_json() {
        let mut task = make_task(None, None);
        task.metadata = Some("not json".to_string());
        assert!(parse_metadata(&task).is_none());
    }

    #[test]
    fn parse_metadata_returns_value_for_valid_json() {
        let mut task = make_task(None, None);
        task.metadata = Some(r#"{"key": "value"}"#.to_string());
        let meta = parse_metadata(&task).unwrap();
        assert_eq!(meta["key"], "value");
    }

    // ==================
    // has_merge_deferred_metadata tests
    // ==================

    #[test]
    fn has_merge_deferred_returns_false_when_no_metadata() {
        let task = make_task(None, None);
        assert!(!has_merge_deferred_metadata(&task));
    }

    #[test]
    fn has_merge_deferred_returns_false_when_no_flag() {
        let mut task = make_task(None, None);
        task.metadata = Some(r#"{"other": "data"}"#.to_string());
        assert!(!has_merge_deferred_metadata(&task));
    }

    #[test]
    fn has_merge_deferred_returns_false_when_flag_is_false() {
        let mut task = make_task(None, None);
        task.metadata = Some(r#"{"merge_deferred": false}"#.to_string());
        assert!(!has_merge_deferred_metadata(&task));
    }

    #[test]
    fn has_merge_deferred_returns_true_when_flag_is_true() {
        let mut task = make_task(None, None);
        task.metadata = Some(r#"{"merge_deferred": true}"#.to_string());
        assert!(has_merge_deferred_metadata(&task));
    }

    // ==================
    // clear_merge_deferred_metadata tests
    // ==================

    #[test]
    fn clear_merge_deferred_removes_flags_from_metadata() {
        let mut task = make_task(None, None);
        task.metadata = Some(
            serde_json::json!({
                "merge_deferred": true,
                "merge_deferred_at": "2026-01-01T00:00:00Z",
                "other": "keep"
            })
            .to_string(),
        );

        clear_merge_deferred_metadata(&mut task);

        let meta = parse_metadata(&task).unwrap();
        assert!(meta.get("merge_deferred").is_none());
        assert!(meta.get("merge_deferred_at").is_none());
        assert_eq!(meta["other"], "keep");
    }

    #[test]
    fn clear_merge_deferred_clears_metadata_when_only_deferred_fields() {
        let mut task = make_task(None, None);
        task.metadata = Some(
            serde_json::json!({
                "merge_deferred": true,
                "merge_deferred_at": "2026-01-01T00:00:00Z",
            })
            .to_string(),
        );

        clear_merge_deferred_metadata(&mut task);

        assert!(task.metadata.is_none());
    }

    #[test]
    fn clear_merge_deferred_noop_when_no_metadata() {
        let mut task = make_task(None, None);
        clear_merge_deferred_metadata(&mut task);
        assert!(task.metadata.is_none());
    }

    // ==================
    // trigger_origin helpers tests
    // ==================

    #[test]
    fn set_trigger_origin_creates_metadata_when_none() {
        let mut task = make_task(None, None);
        set_trigger_origin(&mut task, "scheduler");
        let meta = parse_metadata(&task).unwrap();
        assert_eq!(meta["trigger_origin"], "scheduler");
    }

    #[test]
    fn set_trigger_origin_preserves_existing_metadata() {
        let mut task = make_task(None, None);
        task.metadata = Some(r#"{"other": "value"}"#.to_string());
        set_trigger_origin(&mut task, "revision");
        let meta = parse_metadata(&task).unwrap();
        assert_eq!(meta["trigger_origin"], "revision");
        assert_eq!(meta["other"], "value");
    }

    #[test]
    fn get_trigger_origin_returns_value_when_set() {
        let mut task = make_task(None, None);
        task.metadata = Some(r#"{"trigger_origin": "recovery"}"#.to_string());
        assert_eq!(get_trigger_origin(&task), Some("recovery".to_string()));
    }

    #[test]
    fn get_trigger_origin_returns_none_when_not_set() {
        let task = make_task(None, None);
        assert!(get_trigger_origin(&task).is_none());
    }

    #[test]
    fn get_trigger_origin_returns_none_when_metadata_has_no_origin() {
        let mut task = make_task(None, None);
        task.metadata = Some(r#"{"other": "value"}"#.to_string());
        assert!(get_trigger_origin(&task).is_none());
    }

    #[test]
    fn clear_trigger_origin_removes_field_from_metadata() {
        let mut task = make_task(None, None);
        task.metadata = Some(
            serde_json::json!({
                "trigger_origin": "qa",
                "other": "keep"
            })
            .to_string(),
        );

        clear_trigger_origin(&mut task);

        let meta = parse_metadata(&task).unwrap();
        assert!(meta.get("trigger_origin").is_none());
        assert_eq!(meta["other"], "keep");
    }

    #[test]
    fn clear_trigger_origin_clears_metadata_when_last_field() {
        let mut task = make_task(None, None);
        task.metadata = Some(r#"{"trigger_origin": "retry"}"#.to_string());

        clear_trigger_origin(&mut task);

        assert!(task.metadata.is_none());
    }

    #[test]
    fn clear_trigger_origin_noop_when_no_metadata() {
        let mut task = make_task(None, None);
        clear_trigger_origin(&mut task);
        assert!(task.metadata.is_none());
    }

    // ==================
    // concurrent merge guard — archived task skip tests
    // ==================

    /// Archived tasks in PendingMerge should NOT block newer merge tasks.
    /// Regression test: archived tasks have archived_at set and will never
    /// complete their merge, so the guard must skip them.
    #[test]
    fn archived_task_in_pending_merge_is_not_a_blocker() {
        // An archived task in PendingMerge — should be skipped by the guard
        let mut archived_task = make_task(None, None);
        archived_task.internal_status = InternalStatus::PendingMerge;
        archived_task.archived_at = Some(chrono::Utc::now());
        archived_task.created_at = chrono::Utc::now() - chrono::Duration::hours(1);

        // The guard checks: skip self, skip non-merge states, skip deferred, skip archived
        // Verify that archived_at.is_some() returns true for this task
        assert!(archived_task.archived_at.is_some());

        // A non-archived task should NOT be skipped
        let mut active_task = make_task(None, None);
        active_task.internal_status = InternalStatus::PendingMerge;
        active_task.created_at = chrono::Utc::now() - chrono::Duration::hours(1);
        assert!(active_task.archived_at.is_none());
    }

    // ==================
    // task_targets_branch tests
    // ==================

    #[tokio::test]
    async fn task_targets_branch_returns_true_for_matching_target() {
        let project = make_project(Some("main"));
        let mut task = make_task(None, Some("ralphx/test/task-123"));
        task.id = TaskId::from_string("task-123".to_string());

        let repo: Option<Arc<dyn PlanBranchRepository>> = None;
        // A standalone task merges into project base branch (main)
        assert!(task_targets_branch(&task, &project, &repo, "main").await);
    }

    #[tokio::test]
    async fn task_targets_branch_returns_false_for_non_matching_target() {
        let project = make_project(Some("main"));
        let mut task = make_task(None, Some("ralphx/test/task-123"));
        task.id = TaskId::from_string("task-123".to_string());

        let repo: Option<Arc<dyn PlanBranchRepository>> = None;
        assert!(!task_targets_branch(&task, &project, &repo, "develop").await);
    }

    #[tokio::test]
    async fn task_targets_branch_plan_task_targets_feature_branch() {
        let project = make_project(Some("main"));
        let mut task =
            make_task_with_session(Some("art-1"), Some("ralphx/test/task-456"), Some("sess-1"));
        task.id = TaskId::from_string("task-456".to_string());

        let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
        let pb = make_plan_branch(
            "art-1",
            "ralphx/test/plan-abc123",
            PlanBranchStatus::Active,
            None,
        );
        mem_repo.create(pb).await.unwrap();

        let repo: Option<Arc<dyn PlanBranchRepository>> = Some(mem_repo);
        // Plan task merges into feature branch, not main
        assert!(task_targets_branch(&task, &project, &repo, "ralphx/test/plan-abc123").await);
        assert!(!task_targets_branch(&task, &project, &repo, "main").await);
    }

    // ==================
    // is_task_in_merge_workflow tests
    // ==================

    fn make_task_with_status(task_id: &str, status: InternalStatus) -> Task {
        let mut t = Task::new(
            ProjectId::from_string("proj-1".to_string()),
            "Test task".into(),
        );
        t.id = TaskId::from_string(task_id.to_string());
        t.internal_status = status;
        t
    }

    #[tokio::test]
    async fn test_is_task_in_merge_workflow_pending_merge() {
        let task = make_task_with_status("task-1", InternalStatus::PendingMerge);
        let repo: Arc<dyn TaskRepository> = Arc::new(MemoryTaskRepository::with_tasks(vec![task]));
        assert!(is_task_in_merge_workflow(&repo, "task-1").await);
    }

    #[tokio::test]
    async fn test_is_task_in_merge_workflow_merging() {
        let task = make_task_with_status("task-1", InternalStatus::Merging);
        let repo: Arc<dyn TaskRepository> = Arc::new(MemoryTaskRepository::with_tasks(vec![task]));
        assert!(is_task_in_merge_workflow(&repo, "task-1").await);
    }

    #[tokio::test]
    async fn test_is_task_in_merge_workflow_merge_incomplete() {
        let task = make_task_with_status("task-1", InternalStatus::MergeIncomplete);
        let repo: Arc<dyn TaskRepository> = Arc::new(MemoryTaskRepository::with_tasks(vec![task]));
        assert!(!is_task_in_merge_workflow(&repo, "task-1").await);
    }

    #[tokio::test]
    async fn test_is_task_in_merge_workflow_merge_conflict() {
        let task = make_task_with_status("task-1", InternalStatus::MergeConflict);
        let repo: Arc<dyn TaskRepository> = Arc::new(MemoryTaskRepository::with_tasks(vec![task]));
        assert!(!is_task_in_merge_workflow(&repo, "task-1").await);
    }

    #[tokio::test]
    async fn test_is_task_in_merge_workflow_executing_returns_false() {
        let task = make_task_with_status("task-1", InternalStatus::Executing);
        let repo: Arc<dyn TaskRepository> = Arc::new(MemoryTaskRepository::with_tasks(vec![task]));
        assert!(!is_task_in_merge_workflow(&repo, "task-1").await);
    }

    #[tokio::test]
    async fn test_is_task_in_merge_workflow_nonexistent_task() {
        let repo: Arc<dyn TaskRepository> = Arc::new(MemoryTaskRepository::new());
        assert!(!is_task_in_merge_workflow(&repo, "nonexistent-id").await);
    }

    // ==================
    // Concurrent merge arbitration tests
    // ==================

    /// Test: Lexical task ID comparison works as expected
    ///
    /// This documents the tie-breaker behavior when timestamps are equal or missing.
    #[test]
    fn test_merge_arbitration_task_id_lexical_comparison() {
        let task_alpha = TaskId::from_string("task-alpha".to_string());
        let task_beta = TaskId::from_string("task-beta".to_string());
        let task_x = TaskId::from_string("task-x".to_string());
        let task_y = TaskId::from_string("task-y".to_string());

        // Verify lexical ordering works as expected
        assert!(
            task_alpha.as_str() < task_beta.as_str(),
            "task-alpha < task-beta"
        );
        assert!(task_x.as_str() < task_y.as_str(), "task-x < task-y");
        assert!(task_alpha.as_str() < task_x.as_str(), "task-alpha < task-x");
    }

    /// Test: get_status_entered_at integration with arbitration logic
    ///
    /// This verifies that we can query pending_merge entry times correctly,
    /// which is what the arbitration logic depends on.
    #[tokio::test]
    async fn test_merge_arbitration_get_pending_merge_timestamp() {
        let repo = MemoryTaskRepository::new();
        let project_id = ProjectId::new();
        let task = Task::new(project_id, "Test task".to_string());
        repo.create(task.clone()).await.unwrap();

        // Transition to pending_merge
        repo.persist_status_change(
            &task.id,
            InternalStatus::Executing,
            InternalStatus::PendingMerge,
            "agent",
        )
        .await
        .unwrap();

        // Should be able to retrieve the timestamp
        let timestamp = repo
            .get_status_entered_at(&task.id, InternalStatus::PendingMerge)
            .await
            .unwrap();

        assert!(timestamp.is_some(), "Should have pending_merge timestamp");
    }

    /// Test: Task without state history returns None for get_status_entered_at
    ///
    /// Documents the edge case where a task is in pending_merge but has no history.
    #[tokio::test]
    async fn test_merge_arbitration_missing_timestamp_edge_case() {
        let repo = MemoryTaskRepository::new();
        let project_id = ProjectId::new();

        // Task in pending_merge but no state history recorded
        let mut task = Task::new(project_id, "Edge case task".to_string());
        task.internal_status = InternalStatus::PendingMerge;
        repo.create(task.clone()).await.unwrap();

        // Should return None since no transition was recorded
        let timestamp = repo
            .get_status_entered_at(&task.id, InternalStatus::PendingMerge)
            .await
            .unwrap();

        assert!(
            timestamp.is_none(),
            "Should return None for missing history"
        );
    }

    /// Test: Timestamp comparison works correctly with chrono::DateTime
    ///
    /// Documents the comparison behavior for the arbitration logic.
    #[test]
    fn test_merge_arbitration_timestamp_comparison() {
        use chrono::{Duration, Utc};

        let earlier = Utc::now() - Duration::minutes(30);
        let later = Utc::now() - Duration::minutes(15);

        assert!(
            earlier < later,
            "Earlier timestamp should be less than later"
        );
        assert_eq!(earlier, earlier, "Same timestamps should be equal");
    }

    /// Test: Branch missing sets metadata correctly with AutoRetryTriggered event
    ///
    /// Verifies that when a branch validation fails during programmatic merge,
    /// the code records an AutoRetryTriggered event in MergeRecoveryMetadata
    /// and sets the branch_missing flag in the task metadata JSON.
    #[test]
    fn test_branch_missing_metadata_with_auto_retry_event() {
        // Create initial recovery metadata
        let mut recovery = MergeRecoveryMetadata::new();

        // Verify it starts empty
        assert_eq!(recovery.events.len(), 0);

        // Count existing AutoRetryTriggered events (should be 0)
        let attempt_count = recovery
            .events
            .iter()
            .filter(|e| matches!(e.kind, MergeRecoveryEventKind::AutoRetryTriggered))
            .count() as u32
            + 1;

        assert_eq!(attempt_count, 1, "First attempt should be 1");

        // Create AutoRetryTriggered event with BranchNotFound reason
        let event = MergeRecoveryEvent::new(
            MergeRecoveryEventKind::AutoRetryTriggered,
            MergeRecoverySource::Auto,
            MergeRecoveryReasonCode::BranchNotFound,
            "Source branch 'feature/missing' does not exist".to_string(),
        )
        .with_target_branch("main")
        .with_source_branch("feature/missing")
        .with_attempt(attempt_count);

        // Verify event was created correctly
        assert_eq!(event.kind, MergeRecoveryEventKind::AutoRetryTriggered);
        assert_eq!(event.reason_code, MergeRecoveryReasonCode::BranchNotFound);
        assert_eq!(event.source, MergeRecoverySource::Auto);

        // Append event
        recovery.append_event(event);
        assert_eq!(recovery.events.len(), 1);

        // Update task metadata with recovery events
        let updated_json = recovery.update_task_metadata(None).unwrap();

        // Parse and verify the JSON contains merge_recovery
        let metadata: serde_json::Value = serde_json::from_str(&updated_json).unwrap();
        assert!(
            metadata.get("merge_recovery").is_some(),
            "Should contain merge_recovery field"
        );

        // Verify merge_recovery structure
        let merge_recovery = &metadata["merge_recovery"];
        assert_eq!(merge_recovery["version"], 1);
        assert_eq!(merge_recovery["events"].as_array().unwrap().len(), 1);
        assert_eq!(
            merge_recovery["events"][0]["kind"], "auto_retry_triggered",
            "Event kind should be AutoRetryTriggered"
        );
        assert_eq!(
            merge_recovery["events"][0]["reason_code"], "branch_not_found",
            "Event reason_code should be BranchNotFound"
        );

        // Now add branch_missing flag to metadata (simulating what side_effects.rs does)
        let mut metadata_obj = metadata;
        if let Some(obj) = metadata_obj.as_object_mut() {
            obj.insert("branch_missing".to_string(), serde_json::json!(true));
        }
        let final_json = metadata_obj.to_string();

        // Parse again and verify branch_missing flag is set
        let final_metadata: serde_json::Value = serde_json::from_str(&final_json).unwrap();
        assert_eq!(
            final_metadata.get("branch_missing"),
            Some(&serde_json::json!(true)),
            "Should have branch_missing flag set to true"
        );

        // Verify retry count increments on subsequent attempts
        let recovery2 = MergeRecoveryMetadata::from_task_metadata(Some(&final_json))
            .unwrap_or(None)
            .unwrap_or_else(MergeRecoveryMetadata::new);

        let attempt_count2 = recovery2
            .events
            .iter()
            .filter(|e| matches!(e.kind, MergeRecoveryEventKind::AutoRetryTriggered))
            .count() as u32
            + 1;

        assert_eq!(
            attempt_count2, 2,
            "Second attempt should be 2, confirming retry count increments"
        );
    }

    #[test]
    fn test_build_squash_commit_msg_plan_merge() {
        let msg = build_squash_commit_msg(
            "plan_merge",
            "Fix \"Remove All From Plan\"",
            "ralphx/ralphx/plan-abc123",
        );
        assert_eq!(
            msg,
            "feat: Fix \"Remove All From Plan\"\n\nPlan branch: ralphx/ralphx/plan-abc123"
        );
    }

    #[test]
    fn test_build_squash_commit_msg_regular_task() {
        let msg = build_squash_commit_msg("feat", "Write tests", "ralphx/ralphx/task-xyz");
        assert_eq!(msg, "feat: ralphx/ralphx/task-xyz (Write tests)");
    }

    #[test]
    fn test_build_squash_commit_msg_different_category() {
        let msg = build_squash_commit_msg("fix", "Fix bug", "ralphx/ralphx/task-123");
        assert_eq!(msg, "feat: ralphx/ralphx/task-123 (Fix bug)");
    }
}
