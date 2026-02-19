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
use super::merge_validation::{
    emit_merge_progress, extract_cached_validation, format_validation_warn_metadata,
    take_skip_validation_flag, ValidationFailure,
};

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::Emitter;

use super::super::machine::State;
use crate::application::git_service::checkout_free::{self, CheckoutFreeMergeResult};
use crate::application::git_service::git_cmd;
use crate::application::{GitService, MergeAttemptResult};
use crate::domain::entities::{
    merge_progress_event::{MergePhase, MergePhaseStatus},
    task_metadata::{
        MergeFailureSource, MergeRecoveryEvent, MergeRecoveryEventKind, MergeRecoveryMetadata,
        MergeRecoveryReasonCode, MergeRecoverySource, MergeRecoveryState,
    },
    IdeationSessionId, InternalStatus, MergeStrategy, MergeValidationMode,
    PlanBranchStatus, ProjectId, Task, TaskCategory, TaskId,
};
use crate::domain::repositories::{
    IdeationSessionRepository, PlanBranchRepository, TaskRepository,
};
use crate::error::AppResult;
use crate::infrastructure::agents::claude::defer_merge_enabled;

const TEMP_SKIP_POST_MERGE_VALIDATION: bool = true;

/// Map a TaskCategory to its conventional commit type prefix.
///
/// | Category | Commit Type |
/// |---|---|
/// | Regular | `feat` |
/// | PlanMerge | `feat` |
pub(super) fn category_to_commit_type(category: &TaskCategory) -> &'static str {
    match category {
        TaskCategory::Regular => "feat",
        TaskCategory::PlanMerge => "feat",
    }
}

/// Derive the conventional commit type via majority-wins across task categories.
///
/// Maps each task's category to a commit type, counts votes, and returns the type
/// with the most votes. Ties are broken by variant priority:
/// feat > fix > refactor > docs > test > perf > chore.
/// Falls back to `"feat"` if the task list is empty.
pub(super) fn derive_commit_type(tasks: &[crate::domain::entities::Task]) -> &'static str {
    use std::collections::HashMap;

    // Priority order for tie-breaking (lower index = higher priority)
    const PRIORITY: &[&str] = &[
        "feat", "fix", "refactor", "docs", "test", "perf", "chore",
    ];

    let mut votes: HashMap<&'static str, usize> = HashMap::new();
    for task in tasks {
        let commit_type = category_to_commit_type(&task.category);
        *votes.entry(commit_type).or_insert(0) += 1;
    }

    if votes.is_empty() {
        return "feat";
    }

    let max_votes = *votes.values().max().unwrap_or(&0);

    // Among types with max votes, pick the highest-priority one
    PRIORITY
        .iter()
        .find(|&&t| votes.get(t).copied().unwrap_or(0) == max_votes)
        .copied()
        .unwrap_or("feat")
}

/// Build a descriptive squash commit message for plan merge tasks.
///
/// Fetches the live session title and sibling tasks to construct:
/// `$derived_type: $session_title\n\nPlan branch: {branch}\nTasks ({n}):\n- ...`
///
/// Fallback chain for subject:
/// 1. `session.title` (live fetch) — set by session-namer or user rename
/// 2. First sibling task title — if session title is NULL
/// 3. `"Merge plan into {base_branch}"` — no session title, no tasks
///
/// Task list is capped at 20 entries with `(+N more)` overflow.
pub(super) async fn build_plan_merge_commit_msg(
    ideation_session_id: &IdeationSessionId,
    source_branch: &str,
    task_repo: &dyn TaskRepository,
    session_repo: &dyn IdeationSessionRepository,
) -> String {
    // Fetch sibling tasks for this ideation session
    let sibling_tasks = task_repo
        .get_by_ideation_session(ideation_session_id)
        .await
        .unwrap_or_default();

    // Fetch live session title
    let session_title = session_repo
        .get_by_id(ideation_session_id)
        .await
        .ok()
        .flatten()
        .and_then(|s| s.title);

    // Derive commit type from sibling task categories
    let commit_type = derive_commit_type(&sibling_tasks);

    // Determine subject with fallback chain
    let subject = session_title
        .as_deref()
        .map(str::to_owned)
        .or_else(|| sibling_tasks.first().map(|t| t.title.clone()))
        .unwrap_or_else(|| "Merge plan into main".to_string());

    // Build task list body (capped at 20)
    let task_count = sibling_tasks.len();
    let mut body = format!("Plan branch: {}", source_branch);

    if task_count > 0 {
        body.push_str(&format!("\nTasks ({}):", task_count));
        let display_count = task_count.min(20);
        for task in sibling_tasks.iter().take(display_count) {
            body.push_str(&format!("\n- {}", task.title));
        }
        if task_count > 20 {
            body.push_str(&format!("\n(+{} more)", task_count - 20));
        }
    }

    format!("{}: {}\n\n{}", commit_type, subject, body)
}

/// Build a squash commit message for regular (non-plan-merge) tasks.
///
/// Format: `$category_commit_type: {branch} ({title})`
pub(super) fn build_squash_commit_msg(category: &TaskCategory, title: &str, source_branch: &str) -> String {
    let commit_type = category_to_commit_type(category);
    format!("{}: {} ({})", commit_type, source_branch, title)
}

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

            task.metadata = Some(
                serde_json::json!({
                    "error": error_msg,
                    "error_code": error_code,
                    "category": task.category,
                })
                .to_string(),
            );
            task.internal_status = InternalStatus::MergeIncomplete;
            task.touch();

            if let Err(e) = task_repo.update(&task).await {
                tracing::error!(error = %e, "Failed to update task to MergeIncomplete after pre-merge validation failure");
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
                tracing::warn!(error = %e, "Failed to record merge_incomplete transition (non-fatal)");
            }

            self.machine
                .context
                .services
                .event_emitter
                .emit_status_change(task_id_str, "pending_merge", "merge_incomplete")
                .await;

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

            // Route through TransitionHandler exit to trigger deferred merge retry
            // and ensure all side effects (e.g. try_retry_deferred_merges) fire.
            self.on_exit(&State::PendingMerge, &State::MergeIncomplete)
                .await;

            return;
        }

        // --- Main-merge deferral check ---
        // If target is main/base branch, defer unless ALL sibling plan tasks are terminal
        // AND no agents are running. Prevents premature retry while siblings still active.
        // Skip this entire check if defer_merge_enabled is false.
        let base_branch = project.base_branch.as_deref().unwrap_or("main");
        if target_branch == base_branch && defer_merge_enabled() {
            // Plan-level guard: all sibling tasks must be terminal before merging to main
            if let Some(ref session_id) = task.ideation_session_id {
                let siblings = task_repo
                    .get_by_ideation_session(session_id)
                    .await
                    .unwrap_or_default();
                let all_siblings_terminal = siblings.iter().all(|t| {
                    t.id == task.id
                        || t.internal_status == InternalStatus::PendingMerge
                        || t.is_terminal()
                });
                if !all_siblings_terminal {
                    tracing::info!(
                        task_id = task_id_str,
                        session_id = %session_id,
                        "Deferring main-branch merge: sibling plan tasks not yet terminal"
                    );

                    super::merge_helpers::set_main_merge_deferred_metadata(&mut task);
                    task.touch();

                    if let Err(e) = task_repo.update(&task).await {
                        tracing::error!(error = %e, "Failed to set main_merge_deferred metadata");
                        return;
                    }

                    emit_merge_progress(
                        self.machine.context.services.app_handle.as_ref(),
                        task_id_str,
                        MergePhase::ProgrammaticMerge,
                        MergePhaseStatus::Started,
                        format!(
                            "Deferred merge to {} — waiting for sibling tasks to complete",
                            target_branch,
                        ),
                    );

                    return;
                }
            }

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
                        && !GitService::branch_exists(repo_path, &target_branch).await
                    {
                        match GitService::create_feature_branch(
                            repo_path,
                            &pb.branch_name,
                            &pb.source_branch,
                        ).await {
                            Ok(_) => {
                                tracing::info!(
                                    task_id = task_id_str,
                                    branch = %pb.branch_name,
                                    source = %pb.source_branch,
                                    "Lazily created plan branch for merge target"
                                );
                            }
                            Err(e) if GitService::branch_exists(repo_path, &pb.branch_name).await => {
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
        match project.merge_strategy {
            MergeStrategy::Merge => {
                // Detect if the target branch is already checked out in the primary repo.
                // This happens for plan merge tasks (plan feature branch → main) because
                // main is always checked out in the primary repo. Git forbids the same
                // branch in multiple worktrees, so we merge directly in-repo instead.
                let current_branch = GitService::get_current_branch(repo_path).await.unwrap_or_default();
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
                    if !GitService::branch_exists(repo_path, &source_branch).await {
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
                    if !GitService::branch_exists(repo_path, &target_branch).await {
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
                    ).await;

                    match cf_result {
                        Ok(CheckoutFreeMergeResult::Success { commit_sha }) => {
                            // Atomically sync working tree
                            if let Err(e) = GitService::hard_reset_to_head(repo_path).await {
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
                                        GitService::get_branch_sha(repo_path, &source_branch).await.ok();
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

                            // Persist conflict metadata for historical navigation
                            let conflict_file_strings: Vec<String> = conflict_files
                                .iter()
                                .map(|p| p.to_string_lossy().to_string())
                                .collect();
                            super::merge_helpers::set_conflict_metadata(
                                &mut task,
                                &conflict_file_strings,
                                "programmatic",
                            );

                            // Create temp worktree for conflict resolution (keeps primary checkout clean)
                            let merge_wt_path =
                                PathBuf::from(compute_merge_worktree_path(&project, task_id_str));
                            let target_sha = GitService::get_branch_sha(repo_path, &target_branch).await
                                .unwrap_or_default();
                            let resolve_branch = format!("merge-resolve/{}", task_id_str);

                            // Create temp branch at target's current commit
                            if let Err(e) = GitService::create_branch_at(
                                repo_path,
                                &resolve_branch,
                                &target_sha,
                            ).await {
                                tracing::error!(error = %e, task_id = task_id_str, "Failed to create resolve branch");
                            }

                            // Create worktree on the temp branch
                            if let Err(e) = GitService::checkout_existing_branch_worktree(
                                repo_path,
                                &merge_wt_path,
                                &resolve_branch,
                            ).await {
                                tracing::error!(error = %e, task_id = task_id_str, "Failed to create merge worktree for conflict resolution");
                            }

                            // Start the actual merge in the worktree (leaves conflicts for agent)
                            let _ = git_cmd::run(&["merge", &source_branch, "--no-edit"], &merge_wt_path).await;

                            task.worktree_path = Some(merge_wt_path.to_string_lossy().to_string());
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
                    ).await;

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
                                        GitService::get_branch_sha(repo_path, &source_branch).await.ok();
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

                            if let Err(e) = GitService::delete_worktree(repo_path, &merge_wt_path).await {
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

                            // Persist conflict metadata for historical navigation
                            let conflict_file_strings: Vec<String> = conflict_files
                                .iter()
                                .map(|p| p.to_string_lossy().to_string())
                                .collect();
                            super::merge_helpers::set_conflict_metadata(
                                &mut task,
                                &conflict_file_strings,
                                "programmatic",
                            );

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
                                    let _ = GitService::delete_worktree(repo_path, &merge_wt_path).await;
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
                                    let _ = GitService::delete_worktree(repo_path, &merge_wt_path).await;
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
            MergeStrategy::Rebase => {
                let current_branch = GitService::get_current_branch(repo_path).await.unwrap_or_default();
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
                    if !GitService::branch_exists(repo_path, &source_branch).await
                        || !GitService::branch_exists(repo_path, &target_branch).await
                    {
                        let missing = if !GitService::branch_exists(repo_path, &source_branch).await {
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
                    ).await;

                    match cf_result {
                        Ok(CheckoutFreeMergeResult::Success { commit_sha }) => {
                            // Atomically sync working tree
                            if let Err(e) = GitService::hard_reset_to_head(repo_path).await {
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
                                        GitService::get_branch_sha(repo_path, &source_branch).await.ok();
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
                            let target_sha = GitService::get_branch_sha(repo_path, &target_branch).await
                                .unwrap_or_default();
                            let resolve_branch = format!("merge-resolve/{}", task_id_str);
                            let _ = GitService::create_branch_at(
                                repo_path,
                                &resolve_branch,
                                &target_sha,
                            ).await;
                            let _ = GitService::checkout_existing_branch_worktree(
                                repo_path,
                                &merge_wt_path,
                                &resolve_branch,
                            ).await;
                            let _ = git_cmd::run(&["merge", &source_branch, "--no-edit"], &merge_wt_path).await;

                            task.worktree_path = Some(merge_wt_path.to_string_lossy().to_string());
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
                        let _ = GitService::delete_worktree(repo_path, &rebase_wt_path).await;
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
                    ).await;

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
                                        GitService::get_branch_sha(repo_path, &source_branch).await.ok();
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
                            if let Err(e) = GitService::delete_worktree(repo_path, &merge_wt_path).await {
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

                            // Persist conflict metadata for historical navigation
                            let conflict_file_strings: Vec<String> = conflict_files
                                .iter()
                                .map(|p| p.to_string_lossy().to_string())
                                .collect();
                            super::merge_helpers::set_conflict_metadata(
                                &mut task,
                                &conflict_file_strings,
                                "programmatic",
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
                                    let _ = GitService::delete_worktree(repo_path, &rebase_wt_path).await;
                                }
                                if merge_wt_path.exists() {
                                    let _ = GitService::delete_worktree(repo_path, &merge_wt_path).await;
                                }
                            } else {
                                tracing::error!(
                                    task_id = task_id_str,
                                    error = %e,
                                    "Rebase-and-merge in worktree failed, transitioning to MergeIncomplete"
                                );

                                if rebase_wt_path.exists() {
                                    let _ = GitService::delete_worktree(repo_path, &rebase_wt_path).await;
                                }
                                if merge_wt_path.exists() {
                                    let _ = GitService::delete_worktree(repo_path, &merge_wt_path).await;
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
            MergeStrategy::Squash => {
                let current_branch = GitService::get_current_branch(repo_path).await.unwrap_or_default();
                let target_is_checked_out = current_branch == target_branch;

                let merge_result = if target_is_checked_out {
                    // Use checkout-free squash merge to avoid disrupting working tree
                    tracing::info!(
                        task_id = task_id_str,
                        target_branch = %target_branch,
                        "Target branch is checked out, using checkout-free squash merge"
                    );
                    // Validate branches exist
                    if !GitService::branch_exists(repo_path, &source_branch).await {
                        Ok(MergeAttemptResult::BranchNotFound {
                            branch: source_branch.clone(),
                        })
                    } else if !GitService::branch_exists(repo_path, &target_branch).await {
                        Ok(MergeAttemptResult::BranchNotFound {
                            branch: target_branch.clone(),
                        })
                    } else {
                        match checkout_free::try_squash_merge_checkout_free(
                            repo_path,
                            &source_branch,
                            &target_branch,
                            &squash_commit_msg,
                        ).await {
                            Ok(CheckoutFreeMergeResult::Success { commit_sha }) => {
                                // Atomically sync working tree
                                if let Err(e) = GitService::hard_reset_to_head(repo_path).await {
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
                    ).await;
                    // Clean up worktree on success
                    if let Ok(MergeAttemptResult::Success { .. }) = &result {
                        if let Err(e) = GitService::delete_worktree(repo_path, &merge_wt_path).await {
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
                                    GitService::get_branch_sha(repo_path, &source_branch).await.ok();
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

                        // Persist conflict metadata for historical navigation
                        let conflict_file_strings: Vec<String> = conflict_files
                            .iter()
                            .map(|p| p.to_string_lossy().to_string())
                            .collect();
                        super::merge_helpers::set_conflict_metadata(
                            &mut task,
                            &conflict_file_strings,
                            "programmatic",
                        );

                        // Create temp worktree for conflict resolution
                        let merge_wt_path = PathBuf::from(compute_merge_worktree_path(&project, task_id_str));
                        let target_sha = GitService::get_branch_sha(repo_path, &target_branch).await.unwrap_or_default();
                        let resolve_branch = format!("merge-resolve/{}", task_id_str);
                        let _ = GitService::create_branch_at(repo_path, &resolve_branch, &target_sha).await;
                        let _ = GitService::checkout_existing_branch_worktree(repo_path, &merge_wt_path, &resolve_branch).await;
                        let _ = git_cmd::run(&["merge", &source_branch, "--no-edit"], &merge_wt_path).await;

                        task.worktree_path = Some(merge_wt_path.to_string_lossy().to_string());
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
            MergeStrategy::RebaseSquash => {
                let current_branch = GitService::get_current_branch(repo_path).await.unwrap_or_default();
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
                    if !GitService::branch_exists(repo_path, &source_branch).await
                        || !GitService::branch_exists(repo_path, &target_branch).await
                    {
                        let missing = if !GitService::branch_exists(repo_path, &source_branch).await {
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
                    ).await;

                    match cf_result {
                        Ok(CheckoutFreeMergeResult::Success { commit_sha }) => {
                            // Atomically sync working tree
                            if let Err(e) = GitService::hard_reset_to_head(repo_path).await {
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
                                        GitService::get_branch_sha(repo_path, &source_branch).await.ok();
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
                            let target_sha = GitService::get_branch_sha(repo_path, &target_branch).await
                                .unwrap_or_default();
                            let resolve_branch = format!("merge-resolve/{}", task_id_str);
                            let _ = GitService::create_branch_at(
                                repo_path,
                                &resolve_branch,
                                &target_sha,
                            ).await;
                            let _ = GitService::checkout_existing_branch_worktree(
                                repo_path,
                                &merge_wt_path,
                                &resolve_branch,
                            ).await;
                            let _ = git_cmd::run(&["merge", &source_branch, "--no-edit"], &merge_wt_path).await;

                            task.worktree_path = Some(merge_wt_path.to_string_lossy().to_string());
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
                    ).await;

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
                            if let Err(e) = GitService::delete_worktree(repo_path, &merge_wt_path).await {
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
                                        GitService::get_branch_sha(repo_path, &source_branch).await.ok();
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

                            // Persist conflict metadata for historical navigation
                            let conflict_file_strings: Vec<String> = conflict_files
                                .iter()
                                .map(|p| p.to_string_lossy().to_string())
                                .collect();
                            super::merge_helpers::set_conflict_metadata(
                                &mut task,
                                &conflict_file_strings,
                                "programmatic",
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
                                    let _ = GitService::delete_worktree(repo_path, &rebase_wt_path).await;
                                }
                                if merge_wt_path.exists() {
                                    let _ = GitService::delete_worktree(repo_path, &merge_wt_path).await;
                                }
                            } else {
                                tracing::error!(task_id = task_id_str, error = %e, "Rebase+squash in worktrees failed, transitioning to MergeIncomplete");

                                if rebase_wt_path.exists() {
                                    let _ = GitService::delete_worktree(repo_path, &rebase_wt_path).await;
                                }
                                if merge_wt_path.exists() {
                                    let _ = GitService::delete_worktree(repo_path, &merge_wt_path).await;
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

    /// Pre-merge cleanup: remove debris from any prior failed attempts and stale locks.
    ///
    /// Runs unconditionally on EVERY merge attempt (first or retry) so that transient
    /// failures from a previous run never block the current one.
    ///
    /// Worktree mode:
    ///   1. Delete the task worktree to unlock the task branch
    ///   2. Prune stale worktree references
    ///   3. Delete own merge worktree from a prior attempt
    ///   4. Scan and remove orphaned merge worktrees targeting the same branch
    ///
    /// Also:
    ///   - Remove `.git/index.lock` if it is older than 5 seconds (stale lock from
    ///     a crashed git process).
    ///   - Clean the working tree (reset uncommitted changes)
    async fn pre_merge_cleanup(
        &self,
        task_id_str: &str,
        task: &crate::domain::entities::Task,
        project: &crate::domain::entities::Project,
        repo_path: &Path,
        target_branch: &str,
        task_repo: &Arc<dyn TaskRepository>,
    ) {
        // --- index.lock removal ---
        // Remove a stale .git/index.lock left by a crashed git process.
        // Age threshold: 5 seconds — any lock older than that is definitely stale.
        const INDEX_LOCK_STALE_SECS: u64 = 5;
        match GitService::remove_stale_index_lock(repo_path, INDEX_LOCK_STALE_SECS) {
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
            // Step 1: Delete task worktree to unlock branch for merge worktree creation
            if let Some(ref worktree_path) = task.worktree_path {
                let worktree_path_buf = PathBuf::from(worktree_path);
                if worktree_path_buf.exists() {
                    tracing::info!(
                        task_id = task_id_str,
                        worktree_path = %worktree_path,
                        "Deleting task worktree before programmatic merge to unlock branch"
                    );
                    if let Err(e) =
                        GitService::delete_worktree(repo_path, &worktree_path_buf).await
                    {
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

            // Step 2: Prune stale worktree references (metadata pointing to deleted dirs)
            if let Err(e) = GitService::prune_worktrees(repo_path).await {
                tracing::warn!(
                    task_id = task_id_str,
                    error = %e,
                    "Failed to prune stale worktrees (non-fatal)"
                );
            }

            // Step 3: Force-delete our own merge and rebase worktrees from prior attempts
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
                    if let Err(e) = GitService::delete_worktree(repo_path, &own_wt_path).await {
                        tracing::warn!(
                            task_id = task_id_str,
                            error = %e,
                            worktree_path = %own_wt,
                            "Failed to delete stale {} worktree (non-fatal)",
                            wt_label
                        );
                    }
                }
            }

            // Step 4: Scan for orphaned merge worktrees on the same target branch.
            // Another task's merge may have crashed/failed, leaving a worktree that locks
            // the target branch. We only clean up if the owning task is NOT actively merging.
            if let Ok(worktrees) = GitService::list_worktrees(repo_path).await {
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
        }

        // Clean working tree before merge (non-fatal on error)
        match GitService::clean_working_tree(repo_path).await {
            Ok(()) => tracing::debug!(
                task_id = task_id_str,
                "Pre-merge working tree clean succeeded"
            ),
            Err(e) => {
                tracing::warn!(task_id = task_id_str, error = %e, "Pre-merge clean failed (non-fatal)")
            }
        }
    }

    /// Post-merge cleanup: update plan branch status, delete feature branch, unblock dependents.
    ///
    /// Shared between all merge strategy success paths in `attempt_programmatic_merge()`.
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
