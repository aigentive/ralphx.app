// Merge auto-completion logic extracted from chat_service_send_background.rs
//
// Contains functions that handle automatic merge completion when the merger agent exits.
// Checks git state to determine if merge succeeded, had conflicts, or is incomplete.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Runtime};

use crate::application::git_service::{GitService, StaleRebaseResult};
use crate::application::task_transition_service::TaskTransitionService;
use crate::application::task_scheduler_service::TaskSchedulerService;
use crate::domain::state_machine::services::TaskScheduler;
use crate::domain::state_machine::resolve_merge_branches;
use crate::domain::state_machine::transition_handler::complete_merge_internal;
use crate::domain::state_machine::transition_handler::{
    format_validation_error_metadata, run_validation_commands,
};
use crate::commands::ExecutionState;
use crate::domain::entities::{InternalStatus, TaskId};
use crate::domain::repositories::{
    ActivityEventRepository, AgentRunRepository, ChatAttachmentRepository,
    ChatConversationRepository, ChatMessageRepository, IdeationSessionRepository,
    MemoryEventRepository, PlanBranchRepository, ProjectRepository, TaskDependencyRepository,
    TaskRepository,
};
use crate::domain::services::{MessageQueue, RunningAgentRegistry};

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

    // Resolve working path: prefer worktree if it exists, else fall back to project repo
    let worktree_path = task
        .worktree_path
        .as_ref()
        .map(PathBuf::from)
        .filter(|path| path.exists())
        .unwrap_or_else(|| PathBuf::from(&project.working_directory));

    let worktree = Path::new(&worktree_path);

    // 3. Check git state - try to complete stale rebase first
    match GitService::try_complete_stale_rebase(worktree) {
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

    match GitService::has_conflict_markers(worktree) {
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

    // 3b. If this was a validation recovery (AutoFix mode), re-run validation before completing.
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

        // Re-run validation commands on the merge path
        match run_validation_commands(&project, &task, worktree, task_id_str, None, None).await {
            Some(result) if !result.all_passed => {
                // Agent didn't fix it — revert and fall back to MergeIncomplete
                tracing::warn!(
                    task_id = task_id_str,
                    failure_count = result.failures.len(),
                    "attempt_merge_auto_complete: re-validation failed, reverting merge"
                );

                if let Err(e) = GitService::reset_hard(worktree, "HEAD~1") {
                    tracing::error!(
                        task_id = task_id_str,
                        error = %e,
                        "attempt_merge_auto_complete: failed to revert merge after validation failure"
                    );
                }

                // Update task metadata with validation failure details
                let (source_branch, target_branch) =
                    resolve_merge_branches(&task, &project, plan_branch_repo).await;
                task.metadata = Some(format_validation_error_metadata(
                    &result.failures,
                    &result.log,
                    &source_branch,
                    &target_branch,
                ));
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
            Some(_) => {
                tracing::info!(
                    task_id = task_id_str,
                    "attempt_merge_auto_complete: re-validation passed — proceeding to complete merge"
                );
            }
            None => {
                // No validation commands configured — proceed normally
                tracing::info!(
                    task_id = task_id_str,
                    "attempt_merge_auto_complete: no validation commands found, proceeding"
                );
            }
        }
    }

    // 4. Verify merge actually happened on main branch
    // Get the task branch HEAD SHA from worktree
    let task_branch_head = match GitService::get_head_sha(worktree) {
        Ok(sha) => sha,
        Err(e) => {
            tracing::error!(
                task_id = task_id_str,
                error = %e,
                "attempt_merge_auto_complete: failed to get task branch HEAD SHA"
            );
            transition_to_merge_incomplete(
                &task_id,
                &format!("Auto-complete failed: could not get task branch HEAD SHA: {}", e),
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
    };

    // Get main repo path and resolve correct merge target (plan branch or base branch)
    let main_repo_path = PathBuf::from(&project.working_directory);
    let (_, target_branch) = resolve_merge_branches(&task, &project, plan_branch_repo).await;

    // Verify task branch commit is merged into target branch
    match GitService::is_commit_on_branch(&main_repo_path, &task_branch_head, &target_branch) {
        Ok(true) => {
            // Task branch is merged - good to proceed
        }
        Ok(false) => {
            tracing::warn!(
                task_id = task_id_str,
                task_branch_head = %task_branch_head,
                target_branch = %target_branch,
                "attempt_merge_auto_complete: task branch not merged to target, transitioning to MergeIncomplete"
            );
            transition_to_merge_incomplete(
                &task_id,
                &format!("Agent exited but task branch {} not merged to {}", task_branch_head, target_branch),
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
        Err(e) => {
            tracing::error!(
                task_id = task_id_str,
                error = %e,
                "attempt_merge_auto_complete: failed to verify merge on target branch"
            );
            transition_to_merge_incomplete(
                &task_id,
                &format!("Auto-complete failed: could not verify merge: {}", e),
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

    // 6. Get merge commit SHA from main branch HEAD (not worktree)
    let commit_sha = match GitService::get_head_sha(&main_repo_path) {
        Ok(sha) => sha,
        Err(e) => {
            tracing::error!(
                task_id = task_id_str,
                error = %e,
                "attempt_merge_auto_complete: failed to get main branch HEAD SHA"
            );
            transition_to_merge_incomplete(
                &task_id,
                &format!("Auto-complete failed: could not get main branch HEAD SHA: {}", e),
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
    };

    // 7. Complete the merge using shared logic
    tracing::info!(
        task_id = task_id_str,
        commit_sha = %commit_sha,
        "attempt_merge_auto_complete: merge verified on target branch, completing"
    );

    if let Err(e) = complete_merge_internal(
        &mut task,
        &project,
        &commit_sha,
        task_repo,
        app_handle,
    )
    .await
    {
        tracing::error!(
            task_id = task_id_str,
            error = %e,
            "attempt_merge_auto_complete: complete_merge_internal failed"
        );
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
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;
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
