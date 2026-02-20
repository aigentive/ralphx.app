// Shared merge outcome handler: uniform post-merge logic for all strategy arms
//
// Processes MergeOutcome variants (from merge_strategies.rs) and performs the
// appropriate side effects: success → validate → complete, conflict → spawn agent,
// branch-not-found → MergeIncomplete, error → defer or MergeIncomplete.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::application::git_service::git_cmd;
use crate::application::GitService;
use crate::domain::entities::{
    merge_progress_event::{MergePhase, MergePhaseStatus},
    task_metadata::{
        MergeRecoveryEvent, MergeRecoveryEventKind, MergeRecoveryMetadata,
        MergeRecoveryReasonCode, MergeRecoverySource, MergeRecoveryState,
    },
    InternalStatus, MergeValidationMode, Project, Task, TaskId,
};
use crate::domain::repositories::{PlanBranchRepository, TaskRepository};

use super::merge_completion::complete_merge_internal;
use super::merge_helpers::{compute_merge_worktree_path, parse_metadata};
use super::merge_strategies::MergeOutcome;
use super::merge_validation::{
    emit_merge_progress, extract_cached_validation, format_validation_warn_metadata,
    run_validation_commands, take_skip_validation_flag,
};

/// Per-arm options that vary between merge strategies.
pub(super) struct MergeHandlerOptions {
    pub strategy_label: &'static str,
    pub conflict_reason: &'static str,
    pub conflict_type: Option<&'static str>,
    pub agent_prompt_suffix: &'static str,
}

impl MergeHandlerOptions {
    pub fn merge() -> Self {
        Self { strategy_label: "merge", conflict_reason: "merge_conflict", conflict_type: None, agent_prompt_suffix: "" }
    }
    pub fn rebase() -> Self {
        Self { strategy_label: "rebase", conflict_reason: "rebase_conflict", conflict_type: Some("rebase"),
            agent_prompt_suffix: ". After resolving each file, run `git add <file>` then `git rebase --continue`" }
    }
    pub fn squash() -> Self {
        Self { strategy_label: "squash", conflict_reason: "merge_conflict", conflict_type: None, agent_prompt_suffix: "" }
    }
    pub fn rebase_squash() -> Self {
        Self { strategy_label: "rebase+squash", conflict_reason: "rebase_conflict", conflict_type: Some("rebase"),
            agent_prompt_suffix: ". After resolving each file, run `git add <file>` then `git rebase --continue`" }
    }
}

// ===== Shared helpers for repeated patterns =====

/// Load or create MergeRecoveryMetadata from task metadata.
fn get_or_create_recovery(task: &Task) -> MergeRecoveryMetadata {
    MergeRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
        .unwrap_or(None)
        .unwrap_or_else(MergeRecoveryMetadata::new)
}

/// Count existing AutoRetryTriggered events in recovery metadata.
fn retry_attempt_count(recovery: &MergeRecoveryMetadata) -> u32 {
    recovery.events.iter()
        .filter(|e| matches!(e.kind, MergeRecoveryEventKind::AutoRetryTriggered))
        .count() as u32 + 1
}

/// Transition task to MergeIncomplete, persist status change, and emit event.
async fn transition_to_merge_incomplete(
    task: &mut Task,
    task_id: &TaskId,
    task_id_str: &str,
    task_repo: &Arc<dyn TaskRepository>,
    event_emitter: &Arc<dyn super::super::services::EventEmitter>,
) {
    task.internal_status = InternalStatus::MergeIncomplete;
    task.touch();

    if let Err(e) = task_repo.update(task).await {
        tracing::error!(error = %e, "Failed to update task to MergeIncomplete status");
        return;
    }
    if let Err(e) = task_repo
        .persist_status_change(task_id, InternalStatus::PendingMerge, InternalStatus::MergeIncomplete, "merge_incomplete")
        .await
    {
        tracing::warn!(error = %e, "Failed to record merge incomplete transition (non-fatal)");
    }
    event_emitter.emit_status_change(task_id_str, "pending_merge", "merge_incomplete").await;
}

impl<'a> super::TransitionHandler<'a> {
    /// Handle a MergeOutcome uniformly for all merge strategy arms.
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn handle_merge_outcome(
        &self,
        outcome: MergeOutcome,
        task: &mut Task,
        task_id: &TaskId,
        task_id_str: &str,
        project: &Project,
        repo_path: &Path,
        source_branch: &str,
        target_branch: &str,
        task_repo: &Arc<dyn TaskRepository>,
        plan_branch_repo: &Option<Arc<dyn PlanBranchRepository>>,
        opts: &MergeHandlerOptions,
    ) {
        match outcome {
            MergeOutcome::Success { commit_sha, merge_path } => {
                self.handle_outcome_success(
                    task, task_id, task_id_str, project, repo_path,
                    source_branch, target_branch, task_repo, plan_branch_repo,
                    &commit_sha, &merge_path, opts,
                ).await;
            }
            MergeOutcome::NeedsAgent { conflict_files, merge_worktree } => {
                self.handle_outcome_needs_agent(
                    task, task_id, task_id_str, project, repo_path,
                    source_branch, target_branch, task_repo,
                    &conflict_files, merge_worktree.as_deref(), opts,
                ).await;
            }
            MergeOutcome::BranchNotFound { branch } => {
                self.handle_outcome_branch_not_found(
                    task, task_id, task_id_str, source_branch, target_branch, task_repo, &branch,
                ).await;
            }
            MergeOutcome::Deferred { reason } => {
                self.handle_outcome_deferred(task, task_id_str, source_branch, target_branch, task_repo, &reason).await;
            }
            MergeOutcome::GitError(e) => {
                self.handle_outcome_git_error(
                    task, task_id, task_id_str, source_branch, target_branch, task_repo, e, opts,
                ).await;
            }
            MergeOutcome::AlreadyHandled => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn handle_outcome_success(
        &self,
        task: &mut Task,
        task_id: &TaskId,
        task_id_str: &str,
        project: &Project,
        repo_path: &Path,
        source_branch: &str,
        target_branch: &str,
        task_repo: &Arc<dyn TaskRepository>,
        plan_branch_repo: &Option<Arc<dyn PlanBranchRepository>>,
        commit_sha: &str,
        _merge_path: &Path,
        opts: &MergeHandlerOptions,
    ) {
        tracing::info!(task_id = task_id_str, commit_sha = %commit_sha, strategy = opts.strategy_label, "Merge succeeded");

        emit_merge_progress(
            self.machine.context.services.app_handle.as_ref(), task_id_str,
            MergePhase::ProgrammaticMerge, MergePhaseStatus::Passed,
            format!("{} completed: {}", capitalize(opts.strategy_label), commit_sha),
        );

        // Post-merge validation gate
        let skip_validation = take_skip_validation_flag(task);
        let validation_mode = &project.merge_validation_mode;
        if !skip_validation && *validation_mode != MergeValidationMode::Off {
            let source_sha = GitService::get_branch_sha(repo_path, source_branch).await.ok();
            let cached_log = source_sha.as_deref().and_then(|sha| extract_cached_validation(task, sha));
            if let Some(validation) = run_validation_commands(
                project, task, repo_path, task_id_str,
                self.machine.context.services.app_handle.as_ref(), cached_log.as_deref(),
            ).await {
                if !validation.all_passed {
                    if *validation_mode == MergeValidationMode::Warn {
                        tracing::warn!(task_id = task_id_str, "Validation failed in Warn mode, proceeding");
                        task.metadata = Some(format_validation_warn_metadata(&validation.log, source_branch, target_branch));
                    } else {
                        self.handle_validation_failure(
                            task, task_id, task_id_str, task_repo, &validation.failures, &validation.log,
                            source_branch, target_branch, repo_path, opts.strategy_label, validation_mode,
                        ).await;
                        return;
                    }
                } else {
                    task.metadata = Some(serde_json::json!({
                        "validation_log": validation.log, "validation_source_sha": source_sha,
                        "source_branch": source_branch, "target_branch": target_branch,
                    }).to_string());
                }
            }
        }

        // Complete merge
        let app_handle = self.machine.context.services.app_handle.as_ref();
        if let Err(e) = complete_merge_internal(task, project, commit_sha, target_branch, task_repo, app_handle).await {
            tracing::error!(error = %e, task_id = task_id_str, strategy = opts.strategy_label, "Failed to complete merge");
            task.metadata = Some(serde_json::json!({
                "error": format!("complete_merge_internal failed: {}", e),
                "source_branch": source_branch, "target_branch": target_branch,
            }).to_string());
            transition_to_merge_incomplete(
                task, task_id, task_id_str, task_repo, &self.machine.context.services.event_emitter,
            ).await;
        } else {
            self.post_merge_cleanup(task_id_str, task_id, repo_path, plan_branch_repo).await;
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn handle_outcome_needs_agent(
        &self,
        task: &mut Task,
        task_id: &TaskId,
        task_id_str: &str,
        project: &Project,
        repo_path: &Path,
        source_branch: &str,
        target_branch: &str,
        task_repo: &Arc<dyn TaskRepository>,
        conflict_files: &[PathBuf],
        merge_worktree: Option<&Path>,
        opts: &MergeHandlerOptions,
    ) {
        tracing::info!(task_id = task_id_str, conflict_count = conflict_files.len(), strategy = opts.strategy_label, "Conflicts detected");

        emit_merge_progress(
            self.machine.context.services.app_handle.as_ref(), task_id_str,
            MergePhase::ProgrammaticMerge, MergePhaseStatus::Failed,
            format!("{} conflicts in {} files", capitalize(opts.strategy_label), conflict_files.len()),
        );

        for file in conflict_files {
            tracing::debug!(task_id = task_id_str, file = %file.display(), "Conflict file");
        }

        // Persist conflict metadata
        let conflict_file_strings: Vec<String> = conflict_files.iter().map(|p| p.to_string_lossy().to_string()).collect();
        super::merge_helpers::set_conflict_metadata(task, &conflict_file_strings, "programmatic");

        // Determine worktree path for agent
        let agent_wt = if let Some(wt) = merge_worktree {
            wt.to_path_buf()
        } else {
            // Checkout-free: create temp worktree for conflict resolution
            let wt_path = PathBuf::from(compute_merge_worktree_path(project, task_id_str));
            let target_sha = GitService::get_branch_sha(repo_path, target_branch).await.unwrap_or_default();
            let resolve_branch = format!("merge-resolve/{}", task_id_str);
            if let Err(e) = GitService::create_branch_at(repo_path, &resolve_branch, &target_sha).await {
                tracing::error!(error = %e, task_id = task_id_str, "Failed to create resolve branch");
            }
            if let Err(e) = GitService::checkout_existing_branch_worktree(repo_path, &wt_path, &resolve_branch).await {
                tracing::error!(error = %e, task_id = task_id_str, "Failed to create merge worktree");
            }
            let _ = git_cmd::run(&["merge", source_branch, "--no-edit"], &wt_path).await;
            wt_path
        };

        task.worktree_path = Some(agent_wt.to_string_lossy().to_string());

        // Set conflict_type metadata for rebase strategies
        if let Some(ct) = opts.conflict_type {
            let mut meta = parse_metadata(task).unwrap_or_else(|| serde_json::json!({}));
            if let Some(obj) = meta.as_object_mut() {
                obj.insert("conflict_type".to_string(), serde_json::json!(ct));
            }
            task.metadata = Some(meta.to_string());
        }

        task.internal_status = InternalStatus::Merging;
        task.touch();
        if let Err(e) = task_repo.update(task).await {
            tracing::error!(error = %e, "Failed to update task to Merging");
            return;
        }
        if let Err(e) = task_repo
            .persist_status_change(task_id, InternalStatus::PendingMerge, InternalStatus::Merging, opts.conflict_reason)
            .await
        {
            tracing::warn!(error = %e, "Failed to record {} transition (non-fatal)", opts.conflict_reason);
        }
        self.machine.context.services.event_emitter.emit_status_change(task_id_str, "pending_merge", "merging").await;

        // Spawn merger agent
        let prompt = format!("Resolve {} conflicts for task: {}{}", opts.strategy_label, task_id_str, opts.agent_prompt_suffix);
        tracing::info!(task_id = task_id_str, strategy = opts.strategy_label, "Spawning merger agent");
        let result = self.machine.context.services.chat_service
            .send_message(crate::domain::entities::ChatContextType::Merge, task_id_str, &prompt).await;
        match &result {
            Ok(_) => tracing::info!(task_id = task_id_str, "Merger agent spawned"),
            Err(e) => tracing::error!(task_id = task_id_str, error = %e, "Failed to spawn merger agent"),
        }
    }

    async fn handle_outcome_branch_not_found(
        &self,
        task: &mut Task,
        task_id: &TaskId,
        task_id_str: &str,
        source_branch: &str,
        target_branch: &str,
        task_repo: &Arc<dyn TaskRepository>,
        missing_branch: &str,
    ) {
        tracing::error!(task_id = task_id_str, missing_branch = %missing_branch, "Branch does not exist");

        let mut recovery = get_or_create_recovery(task);
        let attempt = retry_attempt_count(&recovery);
        let event = MergeRecoveryEvent::new(
            MergeRecoveryEventKind::AutoRetryTriggered, MergeRecoverySource::Auto,
            MergeRecoveryReasonCode::BranchNotFound, format!("Branch '{}' does not exist", missing_branch),
        ).with_target_branch(target_branch).with_source_branch(source_branch).with_attempt(attempt);
        recovery.append_event(event);

        match recovery.update_task_metadata(task.metadata.as_deref()) {
            Ok(updated_json) => {
                if let Ok(mut obj) = serde_json::from_str::<serde_json::Value>(&updated_json) {
                    if let Some(m) = obj.as_object_mut() {
                        m.insert("branch_missing".to_string(), serde_json::json!(true));
                    }
                    task.metadata = Some(obj.to_string());
                } else {
                    task.metadata = Some(updated_json);
                }
            }
            Err(e) => {
                tracing::error!(task_id = task_id_str, error = %e, "Failed to serialize recovery metadata");
                task.metadata = Some(serde_json::json!({
                    "error": format!("Branch '{}' does not exist", missing_branch),
                    "missing_branch": missing_branch, "source_branch": source_branch,
                    "target_branch": target_branch, "branch_missing": true
                }).to_string());
            }
        }

        transition_to_merge_incomplete(
            task, task_id, task_id_str, task_repo, &self.machine.context.services.event_emitter,
        ).await;
    }

    async fn handle_outcome_deferred(
        &self,
        task: &mut Task,
        task_id_str: &str,
        source_branch: &str,
        target_branch: &str,
        task_repo: &Arc<dyn TaskRepository>,
        reason: &str,
    ) {
        tracing::warn!(task_id = task_id_str, reason = %reason, "Merge deferred, staying in PendingMerge");

        let mut recovery = get_or_create_recovery(task);
        let event = MergeRecoveryEvent::new(
            MergeRecoveryEventKind::Deferred, MergeRecoverySource::System,
            MergeRecoveryReasonCode::GitError, format!("Merge deferred: {}", reason),
        ).with_target_branch(target_branch).with_source_branch(source_branch);
        recovery.append_event_with_state(event, MergeRecoveryState::Deferred);

        match recovery.update_task_metadata(task.metadata.as_deref()) {
            Ok(updated_json) => { task.metadata = Some(updated_json); }
            Err(e) => {
                tracing::error!(task_id = task_id_str, error = %e, "Failed to serialize recovery metadata");
                task.metadata = Some(serde_json::json!({
                    "merge_deferred": true, "error": reason,
                    "source_branch": source_branch, "target_branch": target_branch,
                }).to_string());
            }
        }

        task.touch();
        if let Err(e) = task_repo.update(task).await {
            tracing::error!(error = %e, "Failed to update task with merge_deferred metadata");
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn handle_outcome_git_error(
        &self,
        task: &mut Task,
        task_id: &TaskId,
        task_id_str: &str,
        source_branch: &str,
        target_branch: &str,
        task_repo: &Arc<dyn TaskRepository>,
        error: crate::error::AppError,
        opts: &MergeHandlerOptions,
    ) {
        if GitService::is_branch_lock_error(&error) {
            tracing::warn!(task_id = task_id_str, error = %error, strategy = opts.strategy_label, "Branch lock, deferring");
            self.handle_outcome_deferred(task, task_id_str, source_branch, target_branch, task_repo, &format!("branch lock: {}", error)).await;
            return;
        }

        tracing::error!(task_id = task_id_str, error = %error, strategy = opts.strategy_label, "Merge failed → MergeIncomplete");

        let mut recovery = get_or_create_recovery(task);
        let attempt = retry_attempt_count(&recovery);
        let failed_event = MergeRecoveryEvent::new(
            MergeRecoveryEventKind::AttemptFailed, MergeRecoverySource::System,
            MergeRecoveryReasonCode::GitError, format!("Merge failed ({}): {}", opts.strategy_label, error),
        ).with_target_branch(target_branch).with_source_branch(source_branch).with_attempt(attempt);
        recovery.append_event_with_state(failed_event, MergeRecoveryState::Failed);

        match recovery.update_task_metadata(task.metadata.as_deref()) {
            Ok(updated_json) => {
                if let Ok(mut meta) = serde_json::from_str::<serde_json::Value>(&updated_json) {
                    if let Some(obj) = meta.as_object_mut() {
                        obj.insert("error".to_string(), serde_json::json!(error.to_string()));
                        obj.insert("source_branch".to_string(), serde_json::json!(source_branch));
                        obj.insert("target_branch".to_string(), serde_json::json!(target_branch));
                    }
                    task.metadata = Some(meta.to_string());
                } else {
                    task.metadata = Some(updated_json);
                }
            }
            Err(e) => {
                tracing::error!(task_id = task_id_str, error = %e, "Failed to serialize recovery metadata");
                task.metadata = Some(serde_json::json!({
                    "error": error.to_string(), "source_branch": source_branch, "target_branch": target_branch,
                }).to_string());
            }
        }

        transition_to_merge_incomplete(
            task, task_id, task_id_str, task_repo, &self.machine.context.services.event_emitter,
        ).await;
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}
