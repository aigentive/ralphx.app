// Merge auto-completion logic extracted from chat_service_send_background.rs
//
// Contains functions that handle automatic merge completion when the merger agent exits.
// Checks git state to determine if merge succeeded, had conflicts, or is incomplete.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};

use crate::application::git_service::{GitService, StaleRebaseResult};
use crate::application::git_service::checkout_free::update_branch_ref;
use crate::application::task_scheduler_service::TaskSchedulerService;
use crate::application::task_transition_service::TaskTransitionService;
use crate::commands::ExecutionState;
use crate::domain::entities::{InternalStatus, TaskId};
use crate::domain::repositories::{
    ActivityEventRepository, AgentRunRepository, ChatAttachmentRepository,
    ChatConversationRepository, ChatMessageRepository, IdeationSessionRepository,
    MemoryEventRepository, PlanBranchRepository, ProjectRepository, TaskDependencyRepository,
    TaskRepository,
};
use crate::domain::services::{MessageQueue, RunningAgentRegistry};
use crate::domain::state_machine::resolve_merge_branches;
use crate::domain::state_machine::services::TaskScheduler;
use crate::domain::state_machine::transition_handler::complete_merge_internal;
use crate::infrastructure::agents::claude::scheduler_config;
use crate::domain::state_machine::transition_handler::{
    format_validation_error_metadata, merge_metadata_into, parse_metadata,
    run_validation_commands,
};

/// RAII guard that removes a task from ExecutionState::auto_completes_in_flight on drop.
/// Ensures cleanup on all return paths, including early returns and panics.
struct AutoCompleteGuard {
    execution_state: Arc<ExecutionState>,
    task_id: String,
}

impl Drop for AutoCompleteGuard {
    fn drop(&mut self) {
        self.execution_state.finish_auto_complete(&self.task_id);
    }
}

/// Attempt to auto-complete a merge when the merger agent exits.
///
/// Called after process_stream_background returns for ChatContextType::Merge.
/// Checks if the task is still in Merging state (agent didn't explicitly transition)
/// and determines the appropriate transition based on git state:
/// - Rebase complete + no conflict markers → transition to Merged
/// - Rebase in progress or conflict markers → transition to MergeConflict
///
/// This enables "fire and forget" merge agents that don't need to call complete_merge.
#[allow(clippy::too_many_arguments)]
pub(super) async fn attempt_merge_auto_complete<R: Runtime>(
    task_id_str: &str,
    task_repo: &Arc<dyn TaskRepository>,
    task_dependency_repo: &Arc<dyn TaskDependencyRepository>,
    project_repo: &Arc<dyn ProjectRepository>,
    chat_message_repo: &Arc<dyn ChatMessageRepository>,
    chat_attachment_repo: &Arc<dyn ChatAttachmentRepository>,
    conversation_repo: &Arc<dyn ChatConversationRepository>,
    agent_run_repo: &Arc<dyn AgentRunRepository>,
    ideation_session_repo: &Arc<dyn IdeationSessionRepository>,
    activity_event_repo: &Arc<dyn ActivityEventRepository>,
    message_queue: &Arc<MessageQueue>,
    running_agent_registry: &Arc<dyn RunningAgentRegistry>,
    memory_event_repo: &Arc<dyn MemoryEventRepository>,
    execution_state: &Arc<ExecutionState>,
    plan_branch_repo: &Option<Arc<dyn PlanBranchRepository>>,
    app_handle: Option<&AppHandle<R>>,
) {
    let task_id = TaskId::from_string(task_id_str.to_string());

    // Dedup guard: prevent concurrent auto-complete calls for the same task.
    // Two trigger paths (agent completion + error handler) can fire ~9s apart,
    // causing concurrent validation in the same worktree → cargo file lock contention.
    if !execution_state.try_start_auto_complete(task_id_str) {
        tracing::info!(
            task_id = task_id_str,
            "attempt_merge_auto_complete: skipping — another auto-complete is already in flight for this task"
        );
        return;
    }
    // RAII guard: ensure we remove the task from the in-flight set on all exit paths.
    let _auto_complete_guard = AutoCompleteGuard {
        execution_state: Arc::clone(execution_state),
        task_id: task_id_str.to_string(),
    };

    // 1. Get task - if not in Merging state, agent already handled it
    let mut task = match task_repo.get_by_id(&task_id).await {
        Ok(Some(task)) => task,
        Ok(None) => {
            tracing::warn!(
                task_id = task_id_str,
                "attempt_merge_auto_complete: task not found"
            );
            return;
        }
        Err(e) => {
            tracing::error!(
                task_id = task_id_str,
                error = %e,
                "attempt_merge_auto_complete: failed to get task"
            );
            return;
        }
    };

    // If task is not in Merging state, agent already transitioned (called complete_merge or report_conflict)
    if task.internal_status != InternalStatus::Merging {
        tracing::info!(
            task_id = task_id_str,
            status = ?task.internal_status,
            "attempt_merge_auto_complete: task already transitioned, skipping"
        );
        // Defence-in-depth: if task reached Merged via a path that bypassed TransitionHandler
        // (e.g. programmatic merge in side_effects.rs), unblock_dependents may not have fired.
        // Calling it here is idempotent — blocked→ready only applies if the dependent is still Blocked.
        if task.internal_status == InternalStatus::Merged {
            use crate::application::task_transition_service::RepoBackedDependencyManager;
            use crate::domain::state_machine::services::DependencyManager;

            let dependency_manager = RepoBackedDependencyManager::new(
                Arc::clone(task_dependency_repo),
                Arc::clone(task_repo),
                app_handle.cloned(),
            );
            dependency_manager.unblock_dependents(task_id_str).await;
        }
        return;
    }

    // 2. Get project for resolving working path
    let project = match project_repo.get_by_id(&task.project_id).await {
        Ok(Some(project)) => project,
        Ok(None) => {
            tracing::error!(
                task_id = task_id_str,
                project_id = task.project_id.as_str(),
                "attempt_merge_auto_complete: project not found"
            );
            return;
        }
        Err(e) => {
            tracing::error!(
                task_id = task_id_str,
                error = %e,
                "attempt_merge_auto_complete: failed to get project"
            );
            return;
        }
    };

    // Resolve working path: MUST be a valid worktree or the main repo if that's where
    // the merge happened (checkout-free validation recovery sets merge_path = repo_path).
    // Never silently fall back to the main repo — that would run git operations in the
    // user's checkout and potentially switch their branch.
    let worktree_path = match &task.worktree_path {
        Some(wt) => {
            let path = PathBuf::from(wt);
            if path.exists() {
                path
            } else {
                tracing::error!(
                    task_id = task_id_str,
                    worktree_path = wt.as_str(),
                    "attempt_merge_auto_complete: worktree path was set but does not exist. \
                     Aborting to avoid running git operations in the user's main checkout."
                );
                return;
            }
        }
        None => {
            tracing::error!(
                task_id = task_id_str,
                "attempt_merge_auto_complete: task has no worktree_path. \
                 Cannot safely determine merge working directory."
            );
            return;
        }
    };

    let worktree = Path::new(&worktree_path);

    // 3. Check git state - try to complete stale rebase first
    match GitService::try_complete_stale_rebase(worktree).await {
        StaleRebaseResult::Completed => {
            tracing::info!(
                task_id = task_id_str,
                "attempt_merge_auto_complete: stale rebase completed successfully, continuing verification"
            );
            // Continue to remaining merge verification steps below
        }
        StaleRebaseResult::HasConflicts { files } => {
            tracing::info!(
                task_id = task_id_str,
                conflict_count = files.len(),
                "attempt_merge_auto_complete: stale rebase has real conflicts, transitioning to MergeConflict"
            );
            transition_to_merge_conflict(
                &task_id,
                "Stale rebase has unresolved conflicts",
                task_repo,
                task_dependency_repo,
                project_repo,
                chat_message_repo,
                chat_attachment_repo,
                conversation_repo,
                agent_run_repo,
                ideation_session_repo,
                activity_event_repo,
                message_queue,
                running_agent_registry,
                memory_event_repo,
                execution_state,
                plan_branch_repo,
                app_handle,
            )
            .await;
            return;
        }
        StaleRebaseResult::Failed { reason } => {
            tracing::info!(
                task_id = task_id_str,
                reason = &reason,
                "attempt_merge_auto_complete: stale rebase recovery failed, transitioning to MergeConflict"
            );
            transition_to_merge_conflict(
                &task_id,
                &format!("Stale rebase recovery failed: {}", reason),
                task_repo,
                task_dependency_repo,
                project_repo,
                chat_message_repo,
                chat_attachment_repo,
                conversation_repo,
                agent_run_repo,
                ideation_session_repo,
                activity_event_repo,
                message_queue,
                running_agent_registry,
                memory_event_repo,
                execution_state,
                plan_branch_repo,
                app_handle,
            )
            .await;
            return;
        }
        StaleRebaseResult::NoRebase => {
            // No rebase in progress, continue to next checks
        }
    }

    // Safety net: check if rebase is somehow still in progress after recovery attempt
    if GitService::is_rebase_in_progress(worktree) {
        tracing::info!(
            task_id = task_id_str,
            "attempt_merge_auto_complete: rebase still in progress after recovery attempt, transitioning to MergeConflict"
        );
        transition_to_merge_conflict(
            &task_id,
            "Rebase still in progress after recovery attempt",
            task_repo,
            task_dependency_repo,
            project_repo,
            chat_message_repo,
            chat_attachment_repo,
            conversation_repo,
            agent_run_repo,
            ideation_session_repo,
            activity_event_repo,
            message_queue,
            running_agent_registry,
            memory_event_repo,
            execution_state,
            plan_branch_repo,
            app_handle,
        )
        .await;
        return;
    }

    if GitService::is_merge_in_progress(worktree) {
        tracing::info!(
            task_id = task_id_str,
            "attempt_merge_auto_complete: merge in progress (MERGE_HEAD exists), transitioning to MergeConflict"
        );
        transition_to_merge_conflict(
            &task_id,
            "Agent exited with incomplete merge (MERGE_HEAD exists)",
            task_repo,
            task_dependency_repo,
            project_repo,
            chat_message_repo,
            chat_attachment_repo,
            conversation_repo,
            agent_run_repo,
            ideation_session_repo,
            activity_event_repo,
            message_queue,
            running_agent_registry,
            memory_event_repo,
            execution_state,
            plan_branch_repo,
            app_handle,
        )
        .await;
        return;
    }

    match GitService::has_conflict_markers(worktree).await {
        Ok(true) => {
            tracing::info!(
                task_id = task_id_str,
                "attempt_merge_auto_complete: conflict markers found, transitioning to MergeConflict"
            );
            transition_to_merge_conflict(
                &task_id,
                "Agent exited with unresolved conflict markers",
                task_repo,
                task_dependency_repo,
                project_repo,
                chat_message_repo,
                chat_attachment_repo,
                conversation_repo,
                agent_run_repo,
                ideation_session_repo,
                activity_event_repo,
                message_queue,
                running_agent_registry,
                memory_event_repo,
                execution_state,
                plan_branch_repo,
                app_handle,
            )
            .await;
            return;
        }
        Ok(false) => {
            // No conflicts - merge succeeded!
        }
        Err(e) => {
            tracing::error!(
                task_id = task_id_str,
                error = %e,
                "attempt_merge_auto_complete: failed to check conflict markers, transitioning to MergeIncomplete"
            );
            transition_to_merge_incomplete(
                &task_id,
                &format!("Auto-complete failed: {}", e),
                task_repo,
                task_dependency_repo,
                project_repo,
                chat_message_repo,
                chat_attachment_repo,
                conversation_repo,
                agent_run_repo,
                ideation_session_repo,
                activity_event_repo,
                message_queue,
                running_agent_registry,
                memory_event_repo,
                execution_state,
                plan_branch_repo,
                app_handle,
            )
            .await;
            return;
        }
    }

    // Get main repo path and resolve merge branches early (needed for verification)
    let main_repo_path = PathBuf::from(&project.working_directory);
    let (source_branch, mut target_branch) =
        resolve_merge_branches(&task, &project, plan_branch_repo).await;

    // Guard: source_branch should never be empty after resolve_merge_branches
    if source_branch.is_empty() {
        tracing::error!(
            task_id = task_id_str,
            "attempt_merge_auto_complete: source_branch is empty after resolve_merge_branches"
        );
        transition_to_merge_incomplete(
            &task_id,
            "Auto-complete failed: could not determine source branch name",
            task_repo,
            task_dependency_repo,
            project_repo,
            chat_message_repo,
            chat_attachment_repo,
            conversation_repo,
            agent_run_repo,
            ideation_session_repo,
            activity_event_repo,
            message_queue,
            running_agent_registry,
            memory_event_repo,
            execution_state,
            plan_branch_repo,
            app_handle,
        )
        .await;
        return;
    }

    // 3b. If this was a plan_update_conflict (plan←main had conflicts), check if the
    // merger agent successfully updated the plan branch from main. If so, clear the flag
    // and retry the task merge by transitioning back to PendingMerge.
    let task_meta_value: Option<serde_json::Value> = task
        .metadata
        .as_ref()
        .and_then(|m| serde_json::from_str(m).ok());
    let is_plan_update_conflict = task_meta_value
        .as_ref()
        .and_then(|v| v.get("plan_update_conflict")?.as_bool())
        .unwrap_or(false);

    // TOCTOU guard: use the target branch that was resolved when the merge was initiated,
    // not the re-resolved value which may differ if plan state changed since then.
    if let Some(stored) = task_meta_value
        .as_ref()
        .and_then(|v| v.get("merge_target_branch")?.as_str().map(String::from))
    {
        if stored != target_branch {
            tracing::info!(
                task_id = task_id_str,
                resolved = %target_branch,
                from_metadata = %stored,
                "Using target_branch from task metadata (TOCTOU guard)"
            );
            target_branch = stored;
        }
    }

    if is_plan_update_conflict {
        let base_branch = task_meta_value
            .as_ref()
            .and_then(|v| v.get("base_branch")?.as_str().map(String::from))
            .unwrap_or_else(|| "main".to_string());
        // Use target_branch from metadata: it was stored at conflict-detection time and is
        // more reliable than re-resolving via resolve_merge_branches, which can return the
        // base branch (e.g. "main") if the plan branch state changed between when the
        // conflict was detected and when this auto-complete runs.
        let plan_branch = task_meta_value
            .as_ref()
            .and_then(|v| v.get("target_branch")?.as_str().map(String::from))
            .unwrap_or_else(|| target_branch.clone());

        // Check if the plan branch is now up-to-date with base_branch
        let plan_up_to_date = match GitService::get_branch_sha(&main_repo_path, &base_branch).await {
            Ok(main_sha) => GitService::is_commit_on_branch(&main_repo_path, &main_sha, &plan_branch)
                .await
                .unwrap_or(false),
            Err(e) => {
                tracing::warn!(
                    task_id = task_id_str,
                    error = %e,
                    "attempt_merge_auto_complete: failed to get base branch SHA for plan_update_conflict check"
                );
                false
            }
        };

        if plan_up_to_date {
            tracing::info!(
                task_id = task_id_str,
                plan_branch = %plan_branch,
                base_branch = %base_branch,
                "attempt_merge_auto_complete: plan branch now up-to-date with base — retrying task merge"
            );
            // RC#12: Clean up the merge-{id} worktree left over from Phase 1 (plan_update)
            // before retrying the task merge. Without this, Phase 2 fails with
            // "fatal: '/path/merge-{id}' already exists" when source_update_conflict
            // tries to create the same worktree path.
            {
                use crate::domain::state_machine::transition_handler::compute_merge_worktree_path;
                let merge_wt_path = PathBuf::from(compute_merge_worktree_path(&project, task_id_str));
                if merge_wt_path.exists() {
                    if let Err(e) = GitService::delete_worktree(&main_repo_path, &merge_wt_path).await {
                        tracing::warn!(
                            task_id = task_id_str,
                            error = %e,
                            "attempt_merge_auto_complete: failed to clean merge worktree after plan update (non-fatal)"
                        );
                    }
                }
            }
            // Clear plan_update_conflict flag so the PendingMerge retry proceeds normally
            {
                let mut meta = task_meta_value.clone().unwrap_or_else(|| serde_json::json!({}));
                if let Some(obj) = meta.as_object_mut() {
                    obj.remove("plan_update_conflict");
                    obj.remove("conflict_files");
                    obj.remove("error");
                }
                task.metadata = Some(meta.to_string());
                task.touch();
                if let Err(e) = task_repo.update(&task).await {
                    tracing::warn!(
                        task_id = task_id_str,
                        error = %e,
                        "attempt_merge_auto_complete: failed to clear plan_update_conflict metadata"
                    );
                }
            }
            let transition_service = TaskTransitionService::new(
                Arc::clone(task_repo),
                Arc::clone(task_dependency_repo),
                Arc::clone(project_repo),
                Arc::clone(chat_message_repo),
                Arc::clone(chat_attachment_repo),
                Arc::clone(conversation_repo),
                Arc::clone(agent_run_repo),
                Arc::clone(ideation_session_repo),
                Arc::clone(activity_event_repo),
                Arc::clone(message_queue),
                Arc::clone(running_agent_registry),
                Arc::clone(execution_state),
                app_handle.cloned(),
                Arc::clone(memory_event_repo),
            );
            let transition_service = if let Some(ref repo) = plan_branch_repo {
                transition_service.with_plan_branch_repo(Arc::clone(repo))
            } else {
                transition_service
            };
            if let Err(e) = transition_service
                .transition_task(&task_id, InternalStatus::PendingMerge)
                .await
            {
                tracing::error!(
                    task_id = task_id_str,
                    error = %e,
                    "attempt_merge_auto_complete: failed to retry task merge via PendingMerge"
                );
            }
        } else {
            tracing::warn!(
                task_id = task_id_str,
                plan_branch = %plan_branch,
                base_branch = %base_branch,
                "attempt_merge_auto_complete: plan branch still not up-to-date — merger agent did not resolve plan←main conflict"
            );
            transition_to_merge_incomplete(
                &task_id,
                &format!(
                    "Merger agent exited but plan branch {} was not updated from {}",
                    plan_branch, base_branch
                ),
                task_repo,
                task_dependency_repo,
                project_repo,
                chat_message_repo,
                chat_attachment_repo,
                conversation_repo,
                agent_run_repo,
                ideation_session_repo,
                activity_event_repo,
                message_queue,
                running_agent_registry,
                memory_event_repo,
                execution_state,
                plan_branch_repo,
                app_handle,
            )
            .await;
        }
        return;
    }

    // 3b-2. Handle source_update_conflict: agent resolved source←target conflicts.
    // Verify source branch is now up-to-date with target, then retry the task merge.
    let is_source_update_conflict = task_meta_value
        .as_ref()
        .and_then(|v| v.get("source_update_conflict")?.as_bool())
        .unwrap_or(false);

    if is_source_update_conflict {
        // Use target_branch from metadata: same reasoning as plan_update_conflict above —
        // resolve_merge_branches may return the base branch if plan branch state changed.
        let target_branch = task_meta_value
            .as_ref()
            .and_then(|v| v.get("target_branch")?.as_str().map(String::from))
            .unwrap_or_else(|| target_branch.clone());
        let source_up_to_date = match GitService::get_branch_sha(&main_repo_path, &target_branch).await {
            Ok(target_sha) => GitService::is_commit_on_branch(&main_repo_path, &target_sha, &source_branch)
                .await
                .unwrap_or(false),
            Err(e) => {
                tracing::warn!(
                    task_id = task_id_str,
                    error = %e,
                    "attempt_merge_auto_complete: failed to get target branch SHA for source_update_conflict check"
                );
                false
            }
        };

        if source_up_to_date {
            tracing::info!(
                task_id = task_id_str,
                source_branch = %source_branch,
                target_branch = %target_branch,
                "attempt_merge_auto_complete: source branch now up-to-date with target — retrying task merge"
            );
            // Clear source_update_conflict flag so the PendingMerge retry proceeds normally
            {
                let mut meta = task_meta_value.clone().unwrap_or_else(|| serde_json::json!({}));
                if let Some(obj) = meta.as_object_mut() {
                    obj.remove("source_update_conflict");
                    obj.remove("conflict_files");
                    obj.remove("error");
                }
                task.metadata = Some(meta.to_string());
                task.touch();
                if let Err(e) = task_repo.update(&task).await {
                    tracing::warn!(
                        task_id = task_id_str,
                        error = %e,
                        "attempt_merge_auto_complete: failed to clear source_update_conflict metadata"
                    );
                }
            }
            let transition_service = TaskTransitionService::new(
                Arc::clone(task_repo),
                Arc::clone(task_dependency_repo),
                Arc::clone(project_repo),
                Arc::clone(chat_message_repo),
                Arc::clone(chat_attachment_repo),
                Arc::clone(conversation_repo),
                Arc::clone(agent_run_repo),
                Arc::clone(ideation_session_repo),
                Arc::clone(activity_event_repo),
                Arc::clone(message_queue),
                Arc::clone(running_agent_registry),
                Arc::clone(execution_state),
                app_handle.cloned(),
                Arc::clone(memory_event_repo),
            );
            let transition_service = if let Some(ref repo) = plan_branch_repo {
                transition_service.with_plan_branch_repo(Arc::clone(repo))
            } else {
                transition_service
            };
            if let Err(e) = transition_service
                .transition_task(&task_id, InternalStatus::PendingMerge)
                .await
            {
                tracing::error!(
                    task_id = task_id_str,
                    error = %e,
                    "attempt_merge_auto_complete: failed to retry task merge via PendingMerge (source_update_conflict)"
                );
            }
        } else {
            tracing::warn!(
                task_id = task_id_str,
                source_branch = %source_branch,
                target_branch = %target_branch,
                "attempt_merge_auto_complete: source branch still not up-to-date — merger agent did not resolve source←target conflict"
            );
            transition_to_merge_incomplete(
                &task_id,
                &format!(
                    "Merger agent exited but source branch {} is not yet up-to-date with {}",
                    source_branch, target_branch
                ),
                task_repo,
                task_dependency_repo,
                project_repo,
                chat_message_repo,
                chat_attachment_repo,
                conversation_repo,
                agent_run_repo,
                ideation_session_repo,
                activity_event_repo,
                message_queue,
                running_agent_registry,
                memory_event_repo,
                execution_state,
                plan_branch_repo,
                app_handle,
            )
            .await;
        }
        return;
    }

    // 3c. If this was a validation recovery (AutoFix mode), re-run validation before completing.
    // The agent may have fixed code and committed, but we must verify validation passes.
    let is_validation_recovery = task
        .metadata
        .as_ref()
        .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
        .and_then(|v| v.get("validation_recovery")?.as_bool())
        .unwrap_or(false);

    if is_validation_recovery {
        tracing::info!(
            task_id = task_id_str,
            "attempt_merge_auto_complete: validation recovery mode — re-running validation"
        );

        // Clear stale validation data and set revalidating flag so the UI shows re-validation state
        {
            let mut val = task
                .metadata
                .as_deref()
                .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
                .unwrap_or_else(|| serde_json::json!({}));
            if let Some(obj) = val.as_object_mut() {
                obj.remove("validation_log");
                obj.remove("validation_failures");
                obj.insert("revalidating".to_string(), serde_json::json!(true));
            }
            task.metadata = Some(val.to_string());
            task.touch();
            let _ = task_repo.update(&task).await;
        }

        // Emit validation_start event so the frontend clears stale live steps
        if let Some(handle) = app_handle {
            let _ = handle.emit("merge:validation_start", serde_json::json!({
                "task_id": task_id_str,
            }));
        }

        // Downcast generic app_handle to Wry for run_validation_commands
        let wry_handle: Option<tauri::AppHandle<tauri::Wry>> = app_handle.and_then(|h| {
            let any: Box<dyn std::any::Any> = Box::new(h.clone());
            any.downcast::<tauri::AppHandle<tauri::Wry>>().ok().map(|b| *b)
        });

        // Re-run validation commands on the merge path
        let validation_cancel = tokio_util::sync::CancellationToken::new();
        match run_validation_commands(&project, &task, worktree, task_id_str, wry_handle.as_ref(), None, &project.merge_validation_mode, &validation_cancel).await {
            Some(result) if !result.all_passed => {
                // Agent didn't fix it — revert and fall back to MergeIncomplete
                tracing::warn!(
                    task_id = task_id_str,
                    failure_count = result.failures.len(),
                    "attempt_merge_auto_complete: re-validation failed, reverting merge"
                );

                // Defence-in-depth: never reset_hard on the main repo — that would
                // destroy the user's checkout. With Fix 1 (fixer worktree isolation),
                // worktree should never equal main_repo_path, but guard against it.
                if worktree_path == main_repo_path {
                    tracing::error!(
                        task_id = task_id_str,
                        "attempt_merge_auto_complete: BUG — worktree_path equals main_repo_path, \
                         refusing to reset_hard on user's checkout"
                    );
                } else if let Err(e) = GitService::reset_hard(worktree, "HEAD~1").await {
                    tracing::error!(
                        task_id = task_id_str,
                        error = %e,
                        "attempt_merge_auto_complete: failed to revert merge after validation failure"
                    );
                }

                // Update task metadata with validation failure details,
                // preserving merge_recovery history and incrementing validation_revert_count
                let (source_branch, target_branch) =
                    resolve_merge_branches(&task, &project, plan_branch_repo).await;
                let error_metadata_str = format_validation_error_metadata(
                    &result.failures,
                    &result.log,
                    &source_branch,
                    &target_branch,
                );
                // Merge error metadata into existing metadata to preserve revert count and recovery history
                if let Ok(error_obj) = serde_json::from_str::<serde_json::Value>(&error_metadata_str) {
                    merge_metadata_into(&mut task, &error_obj);
                }
                // Increment validation_revert_count (tracks how many times we've reverted after validation failure)
                let prev_revert_count: u32 = parse_metadata(&task)
                    .and_then(|v| v.get("validation_revert_count")?.as_u64())
                    .unwrap_or(0) as u32;
                merge_metadata_into(&mut task, &serde_json::json!({
                    "validation_revert_count": prev_revert_count + 1,
                    "merge_failure_source": "ValidationFailed",
                }));
                // Remove revalidating flag (merge_metadata_into only inserts, so remove manually)
                {
                    let mut meta = parse_metadata(&task).unwrap_or_else(|| serde_json::json!({}));
                    if let Some(obj) = meta.as_object_mut() {
                        obj.remove("revalidating");
                    }
                    task.metadata = Some(meta.to_string());
                }
                task.touch();
                let _ = task_repo.update(&task).await;

                transition_to_merge_incomplete(
                    &task_id,
                    "Validation re-check failed after agent fix attempt",
                    task_repo,
                    task_dependency_repo,
                    project_repo,
                    chat_message_repo,
                    chat_attachment_repo,
                    conversation_repo,
                    agent_run_repo,
                    ideation_session_repo,
                    activity_event_repo,
                    message_queue,
                    running_agent_registry,
                    memory_event_repo,
                    execution_state,
                    plan_branch_repo,
                    app_handle,
                )
                .await;
                return;
            }
            Some(result) => {
                tracing::info!(
                    task_id = task_id_str,
                    "attempt_merge_auto_complete: re-validation passed — proceeding to complete merge"
                );
                // Update metadata with fresh validation_log so the UI shows the new results
                let mut merged_meta = task
                    .metadata
                    .as_deref()
                    .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
                    .unwrap_or_else(|| serde_json::json!({}));
                if let Some(obj) = merged_meta.as_object_mut() {
                    obj.insert("validation_log".to_string(), serde_json::json!(result.log));
                    obj.remove("revalidating");
                }
                task.metadata = Some(merged_meta.to_string());
                task.touch();
                let _ = task_repo.update(&task).await;
            }
            None => {
                // No validation commands configured — proceed normally
                tracing::info!(
                    task_id = task_id_str,
                    "attempt_merge_auto_complete: no validation commands found, proceeding"
                );
                // Clear revalidating flag
                let mut val = task
                    .metadata
                    .as_deref()
                    .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
                    .unwrap_or_else(|| serde_json::json!({}));
                if let Some(obj) = val.as_object_mut() {
                    obj.remove("revalidating");
                }
                task.metadata = Some(val.to_string());
                task.touch();
                let _ = task_repo.update(&task).await;
            }
        }
    }

    // 4. Verify merge actually happened on target branch.
    // For validation recovery, skip ancestry-based verification — the merge was already
    // verified during handle_outcome_success; the fixer agent only added fix commits on top.
    // Squash merges fail is_commit_on_branch because the source SHA isn't an ancestor of the
    // squash commit. Instead, just get the target branch HEAD SHA directly.
    let commit_sha = if is_validation_recovery {
        match GitService::get_branch_sha(&main_repo_path, &target_branch).await {
            Ok(sha) => {
                tracing::info!(
                    task_id = task_id_str,
                    target_sha = %sha,
                    "attempt_merge_auto_complete: validation recovery — skipping ancestry check, using target HEAD"
                );
                sha
            }
            Err(e) => {
                tracing::error!(
                    task_id = task_id_str,
                    error = %e,
                    target_branch = %target_branch,
                    "attempt_merge_auto_complete: validation recovery — failed to get target branch SHA"
                );
                transition_to_merge_incomplete(
                    &task_id,
                    &format!("Auto-complete failed: cannot resolve target branch {}: {}", target_branch, e),
                    task_repo,
                    task_dependency_repo,
                    project_repo,
                    chat_message_repo,
                    chat_attachment_repo,
                    conversation_repo,
                    agent_run_repo,
                    ideation_session_repo,
                    activity_event_repo,
                    message_queue,
                    running_agent_registry,
                    memory_event_repo,
                    execution_state,
                    plan_branch_repo,
                    app_handle,
                )
                .await;
                return;
            }
        }
    } else {
        // 4a. Fast-forward target branch from merge-resolve/{task_id} if it exists.
        // When checkout-free merge detects conflicts, merge_outcome_handler creates a
        // merge-resolve/{task_id} branch + worktree for agent conflict resolution.
        // After the agent resolves and commits, the merge commit lives on merge-resolve/,
        // NOT on the target branch. We must fast-forward the target branch before verification.
        let resolve_branch = format!("merge-resolve/{}", task_id_str);
        if let Ok(resolve_sha) = GitService::get_branch_sha(&main_repo_path, &resolve_branch).await {
            tracing::info!(
                task_id = task_id_str,
                resolve_branch = %resolve_branch,
                resolve_sha = %resolve_sha,
                target_branch = %target_branch,
                "attempt_merge_auto_complete: merge-resolve branch found, fast-forwarding target branch"
            );
            if let Err(e) = update_branch_ref(&main_repo_path, &target_branch, &resolve_sha).await {
                tracing::error!(
                    task_id = task_id_str,
                    error = %e,
                    target_branch = %target_branch,
                    resolve_sha = %resolve_sha,
                    "attempt_merge_auto_complete: failed to fast-forward target branch from merge-resolve"
                );
                transition_to_merge_incomplete(
                    &task_id,
                    &format!(
                        "Auto-complete failed: could not fast-forward {} to merge-resolve commit {}: {}",
                        target_branch, resolve_sha, e
                    ),
                    task_repo,
                    task_dependency_repo,
                    project_repo,
                    chat_message_repo,
                    chat_attachment_repo,
                    conversation_repo,
                    agent_run_repo,
                    ideation_session_repo,
                    activity_event_repo,
                    message_queue,
                    running_agent_registry,
                    memory_event_repo,
                    execution_state,
                    plan_branch_repo,
                    app_handle,
                )
                .await;
                return;
            }
            tracing::info!(
                task_id = task_id_str,
                target_branch = %target_branch,
                resolve_sha = %resolve_sha,
                "attempt_merge_auto_complete: target branch fast-forwarded successfully"
            );

            // Clean up: delete the merge-resolve worktree and branch
            let merge_wt_path = task.worktree_path.as_deref().map(PathBuf::from);
            if let Some(ref wt_path) = merge_wt_path {
                if wt_path.exists() {
                    if let Err(e) = GitService::delete_worktree(&main_repo_path, wt_path).await {
                        tracing::warn!(
                            task_id = task_id_str,
                            error = %e,
                            worktree = %wt_path.display(),
                            "attempt_merge_auto_complete: failed to delete merge-resolve worktree (non-fatal)"
                        );
                    }
                }
            }
            if let Err(e) = GitService::delete_branch(&main_repo_path, &resolve_branch, true).await {
                tracing::warn!(
                    task_id = task_id_str,
                    error = %e,
                    resolve_branch = %resolve_branch,
                    "attempt_merge_auto_complete: failed to delete merge-resolve branch (non-fatal)"
                );
            }
        }

        match verify_merge_on_target(&main_repo_path, &source_branch, &target_branch).await {
            MergeVerification::Merged(sha) => {
                // Task branch is merged - capture the merge commit SHA
                sha
            }
            MergeVerification::NotMerged => {
                tracing::warn!(
                    task_id = task_id_str,
                    source_branch = %source_branch,
                    target_branch = %target_branch,
                    "attempt_merge_auto_complete: task branch not merged to target, transitioning to MergeIncomplete"
                );
                transition_to_merge_incomplete(
                    &task_id,
                    &format!(
                        "Agent exited but task branch {} not merged to {}",
                        source_branch, target_branch
                    ),
                    task_repo,
                    task_dependency_repo,
                    project_repo,
                    chat_message_repo,
                    chat_attachment_repo,
                    conversation_repo,
                    agent_run_repo,
                    ideation_session_repo,
                    activity_event_repo,
                    message_queue,
                    running_agent_registry,
                    memory_event_repo,
                    execution_state,
                    plan_branch_repo,
                    app_handle,
                )
                .await;
                return;
            }
            MergeVerification::SourceBranchMissing => {
                tracing::error!(
                    task_id = task_id_str,
                    source_branch = %source_branch,
                    "attempt_merge_auto_complete: source branch does not exist or cannot be resolved"
                );
                transition_to_merge_incomplete(
                    &task_id,
                    &format!(
                        "Auto-complete failed: source branch {} does not exist or cannot be resolved",
                        source_branch
                    ),
                    task_repo,
                    task_dependency_repo,
                    project_repo,
                    chat_message_repo,
                    chat_attachment_repo,
                    conversation_repo,
                    agent_run_repo,
                    ideation_session_repo,
                    activity_event_repo,
                    message_queue,
                    running_agent_registry,
                    memory_event_repo,
                    execution_state,
                    plan_branch_repo,
                    app_handle,
                )
                .await;
                return;
            }
            MergeVerification::TargetBranchMissing => {
                tracing::error!(
                    task_id = task_id_str,
                    target_branch = %target_branch,
                    "attempt_merge_auto_complete: target branch does not exist or cannot be resolved"
                );
                transition_to_merge_incomplete(
                    &task_id,
                    &format!(
                        "Auto-complete failed: target branch {} does not exist or cannot be resolved",
                        target_branch
                    ),
                    task_repo,
                    task_dependency_repo,
                    project_repo,
                    chat_message_repo,
                    chat_attachment_repo,
                    conversation_repo,
                    agent_run_repo,
                    ideation_session_repo,
                    activity_event_repo,
                    message_queue,
                    running_agent_registry,
                    memory_event_repo,
                    execution_state,
                    plan_branch_repo,
                    app_handle,
                )
                .await;
                return;
            }
        }
    };

    // 7. Complete the merge using shared logic
    tracing::info!(
        task_id = task_id_str,
        commit_sha = %commit_sha,
        "attempt_merge_auto_complete: merge verified on target branch, completing"
    );

    if let Err(e) =
        complete_merge_internal(&mut task, &project, &commit_sha, &target_branch, task_repo, app_handle).await
    {
        tracing::error!(
            task_id = task_id_str,
            error = %e,
            "attempt_merge_auto_complete: complete_merge_internal failed — transitioning to MergeIncomplete"
        );
        // Clean up fixer worktree on failure (if it's not the main repo)
        if worktree_path != main_repo_path {
            if let Err(cleanup_err) = GitService::delete_worktree(&main_repo_path, worktree).await {
                tracing::warn!(task_id = task_id_str, error = %cleanup_err, "Failed to cleanup fixer worktree after merge failure (non-fatal)");
            }
        }
        // Transition via TaskTransitionService so on_exit(Merging) fires:
        // - decrements running_count (prevents concurrency limit leak)
        // - triggers try_retry_deferred_merges
        // - surfaces task in needs_attention panel
        transition_to_merge_incomplete(
            &task_id,
            &format!("Auto-complete failed: complete_merge_internal error: {}", e),
            task_repo,
            task_dependency_repo,
            project_repo,
            chat_message_repo,
            chat_attachment_repo,
            conversation_repo,
            agent_run_repo,
            ideation_session_repo,
            activity_event_repo,
            message_queue,
            running_agent_registry,
            memory_event_repo,
            execution_state,
            plan_branch_repo,
            app_handle,
        )
        .await;
    } else {
        // Auto-unblock tasks that were waiting on this task
        // (auto-complete merge path - on_enter(Merged) won't be triggered)
        use crate::application::task_transition_service::RepoBackedDependencyManager;
        use crate::domain::state_machine::services::DependencyManager;

        let dependency_manager = RepoBackedDependencyManager::new(
            Arc::clone(task_dependency_repo),
            Arc::clone(task_repo),
            app_handle.cloned(),
        );
        dependency_manager.unblock_dependents(task_id_str).await;

        // Clean up fixer worktree after successful merge (if it's not the main repo)
        if worktree_path != main_repo_path {
            if let Err(cleanup_err) = GitService::delete_worktree(&main_repo_path, worktree).await {
                tracing::warn!(task_id = task_id_str, error = %cleanup_err, "Failed to cleanup fixer worktree after merge success (non-fatal)");
            }
        }

        // Schedule newly-unblocked tasks (e.g. plan_merge tasks that just became Ready)
        let scheduler = TaskSchedulerService::new(
            Arc::clone(execution_state),
            Arc::clone(project_repo),
            Arc::clone(task_repo),
            Arc::clone(task_dependency_repo),
            Arc::clone(chat_message_repo),
            Arc::clone(chat_attachment_repo),
            Arc::clone(conversation_repo),
            Arc::clone(agent_run_repo),
            Arc::clone(ideation_session_repo),
            Arc::clone(activity_event_repo),
            Arc::clone(message_queue),
            Arc::clone(running_agent_registry),
            Arc::clone(memory_event_repo),
            app_handle.cloned(),
        );
        let scheduler = if let Some(ref repo) = plan_branch_repo {
            scheduler.with_plan_branch_repo(Arc::clone(repo))
        } else {
            scheduler
        };
        let scheduler = Arc::new(scheduler);
        scheduler.set_self_ref(Arc::clone(&scheduler) as Arc<dyn TaskScheduler>);
        // Auto-complete path is internal — no UI settle needed → merge_settle_ms
        let merge_settle_ms = scheduler_config().merge_settle_ms;
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(merge_settle_ms)).await;
            scheduler.try_schedule_ready_tasks().await;
        });
    }
}

/// Reconcile merge state when agent run finished but status is still Merging.
pub(crate) async fn reconcile_merge_auto_complete<R: Runtime>(
    task_id_str: &str,
    task_repo: &Arc<dyn TaskRepository>,
    task_dependency_repo: &Arc<dyn TaskDependencyRepository>,
    project_repo: &Arc<dyn ProjectRepository>,
    chat_message_repo: &Arc<dyn ChatMessageRepository>,
    chat_attachment_repo: &Arc<dyn ChatAttachmentRepository>,
    conversation_repo: &Arc<dyn ChatConversationRepository>,
    agent_run_repo: &Arc<dyn AgentRunRepository>,
    ideation_session_repo: &Arc<dyn IdeationSessionRepository>,
    activity_event_repo: &Arc<dyn ActivityEventRepository>,
    message_queue: &Arc<MessageQueue>,
    running_agent_registry: &Arc<dyn RunningAgentRegistry>,
    memory_event_repo: &Arc<dyn MemoryEventRepository>,
    execution_state: &Arc<ExecutionState>,
    plan_branch_repo: &Option<Arc<dyn PlanBranchRepository>>,
    app_handle: Option<&AppHandle<R>>,
) {
    attempt_merge_auto_complete(
        task_id_str,
        task_repo,
        task_dependency_repo,
        project_repo,
        chat_message_repo,
        chat_attachment_repo,
        conversation_repo,
        agent_run_repo,
        ideation_session_repo,
        activity_event_repo,
        message_queue,
        running_agent_registry,
        memory_event_repo,
        execution_state,
        plan_branch_repo,
        app_handle,
    )
    .await;
}

/// Helper to transition task to MergeConflict state
#[allow(clippy::too_many_arguments)]
async fn transition_to_merge_conflict<R: Runtime>(
    task_id: &TaskId,
    reason: &str,
    task_repo: &Arc<dyn TaskRepository>,
    task_dependency_repo: &Arc<dyn TaskDependencyRepository>,
    project_repo: &Arc<dyn ProjectRepository>,
    chat_message_repo: &Arc<dyn ChatMessageRepository>,
    chat_attachment_repo: &Arc<dyn ChatAttachmentRepository>,
    conversation_repo: &Arc<dyn ChatConversationRepository>,
    agent_run_repo: &Arc<dyn AgentRunRepository>,
    ideation_session_repo: &Arc<dyn IdeationSessionRepository>,
    activity_event_repo: &Arc<dyn ActivityEventRepository>,
    message_queue: &Arc<MessageQueue>,
    running_agent_registry: &Arc<dyn RunningAgentRegistry>,
    memory_event_repo: &Arc<dyn MemoryEventRepository>,
    execution_state: &Arc<ExecutionState>,
    plan_branch_repo: &Option<Arc<dyn PlanBranchRepository>>,
    app_handle: Option<&AppHandle<R>>,
) {
    tracing::info!(
        task_id = task_id.as_str(),
        reason = reason,
        "transition_to_merge_conflict: transitioning task"
    );

    let transition_service = TaskTransitionService::new(
        Arc::clone(task_repo),
        Arc::clone(task_dependency_repo),
        Arc::clone(project_repo),
        Arc::clone(chat_message_repo),
        Arc::clone(chat_attachment_repo),
        Arc::clone(conversation_repo),
        Arc::clone(agent_run_repo),
        Arc::clone(ideation_session_repo),
        Arc::clone(activity_event_repo),
        Arc::clone(message_queue),
        Arc::clone(running_agent_registry),
        Arc::clone(execution_state),
        app_handle.cloned(),
        Arc::clone(memory_event_repo),
    );
    let transition_service = if let Some(ref repo) = plan_branch_repo {
        transition_service.with_plan_branch_repo(Arc::clone(repo))
    } else {
        transition_service
    };

    if let Err(e) = transition_service
        .transition_task(task_id, InternalStatus::MergeConflict)
        .await
    {
        tracing::error!(
            task_id = task_id.as_str(),
            error = %e,
            "transition_to_merge_conflict: failed to transition"
        );
    }
}

/// Helper to transition task to MergeIncomplete state (non-conflict failures)
#[allow(clippy::too_many_arguments)]
async fn transition_to_merge_incomplete<R: Runtime>(
    task_id: &TaskId,
    reason: &str,
    task_repo: &Arc<dyn TaskRepository>,
    task_dependency_repo: &Arc<dyn TaskDependencyRepository>,
    project_repo: &Arc<dyn ProjectRepository>,
    chat_message_repo: &Arc<dyn ChatMessageRepository>,
    chat_attachment_repo: &Arc<dyn ChatAttachmentRepository>,
    conversation_repo: &Arc<dyn ChatConversationRepository>,
    agent_run_repo: &Arc<dyn AgentRunRepository>,
    ideation_session_repo: &Arc<dyn IdeationSessionRepository>,
    activity_event_repo: &Arc<dyn ActivityEventRepository>,
    message_queue: &Arc<MessageQueue>,
    running_agent_registry: &Arc<dyn RunningAgentRegistry>,
    memory_event_repo: &Arc<dyn MemoryEventRepository>,
    execution_state: &Arc<ExecutionState>,
    plan_branch_repo: &Option<Arc<dyn PlanBranchRepository>>,
    app_handle: Option<&AppHandle<R>>,
) {
    tracing::info!(
        task_id = task_id.as_str(),
        reason = reason,
        "transition_to_merge_incomplete: transitioning task"
    );

    let transition_service = TaskTransitionService::new(
        Arc::clone(task_repo),
        Arc::clone(task_dependency_repo),
        Arc::clone(project_repo),
        Arc::clone(chat_message_repo),
        Arc::clone(chat_attachment_repo),
        Arc::clone(conversation_repo),
        Arc::clone(agent_run_repo),
        Arc::clone(ideation_session_repo),
        Arc::clone(activity_event_repo),
        Arc::clone(message_queue),
        Arc::clone(running_agent_registry),
        Arc::clone(execution_state),
        app_handle.cloned(),
        Arc::clone(memory_event_repo),
    );
    let transition_service = if let Some(ref repo) = plan_branch_repo {
        transition_service.with_plan_branch_repo(Arc::clone(repo))
    } else {
        transition_service
    };

    if let Err(e) = transition_service
        .transition_task(task_id, InternalStatus::MergeIncomplete)
        .await
    {
        tracing::error!(
            task_id = task_id.as_str(),
            error = %e,
            "transition_to_merge_incomplete: failed to transition"
        );
    }
}

/// Result of merge verification on target branch
#[derive(Debug, PartialEq)]
pub(crate) enum MergeVerification {
    /// Source branch was successfully merged to target (includes merge commit SHA)
    Merged(String),
    /// Source branch exists but is not merged to target
    NotMerged,
    /// Source branch does not exist or is empty
    SourceBranchMissing,
    /// Target branch does not exist
    TargetBranchMissing,
}

/// Verify if source branch has been merged to target branch.
///
/// Uses git operations from main repo to avoid race conditions with worktree deletion.
/// Returns:
/// - `Merged(sha)` if source branch tip is on target branch (includes target HEAD SHA)
/// - `NotMerged` if source exists but is not on target
/// - `SourceBranchMissing` if source branch doesn't exist or can't be resolved
/// - `TargetBranchMissing` if target branch doesn't exist or can't be resolved
pub(crate) async fn verify_merge_on_target(
    main_repo: &Path,
    source_branch: &str,
    target_branch: &str,
) -> MergeVerification {
    // Get source branch SHA
    let source_sha = match GitService::get_branch_sha(main_repo, source_branch).await {
        Ok(sha) => sha,
        Err(_) => return MergeVerification::SourceBranchMissing,
    };

    // Get target branch SHA
    let target_sha = match GitService::get_branch_sha(main_repo, target_branch).await {
        Ok(sha) => sha,
        Err(_) => return MergeVerification::TargetBranchMissing,
    };

    // Check if source commit is on target branch
    match GitService::is_commit_on_branch(main_repo, &source_sha, target_branch).await {
        Ok(true) => MergeVerification::Merged(target_sha),
        Ok(false) => MergeVerification::NotMerged,
        Err(_) => MergeVerification::TargetBranchMissing,
    }
}

#[cfg(test)]
#[path = "chat_service_merge_tests.rs"]
mod chat_service_merge_tests;
