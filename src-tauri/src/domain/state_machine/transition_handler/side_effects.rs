// State entry side effects
// This module contains the on_enter implementation that handles state-specific actions

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::{AppHandle, Emitter};

use super::super::machine::State;
use crate::application::{GitService, MergeAttemptResult};
use crate::domain::entities::{GitMode, InternalStatus, PlanBranchStatus, Project, Task, TaskId, ProjectId};
use crate::domain::repositories::{PlanBranchRepository, TaskRepository};
use crate::error::{AppError, AppResult};

/// Complete a merge operation by transitioning task to Merged and cleaning up.
///
/// This is shared logic used by:
/// - Programmatic merge success path (PendingMerge side effect)
/// - Merge auto-completion on agent exit (Phase 76)
/// - complete_merge HTTP handler (backwards compatibility)
///
/// # Arguments
/// * `task` - Mutable task to update (must be in appropriate state)
/// * `project` - Project for branch/worktree cleanup info
/// * `commit_sha` - The merge commit SHA
/// * `task_repo` - Repository to persist task changes
/// * `app_handle` - Optional Tauri handle for emitting events
///
/// # Side Effects
/// 1. Updates task.merge_commit_sha
/// 2. Updates task.internal_status to Merged
/// 3. Persists status change to history
/// 4. Deletes worktree (if Worktree mode)
/// 5. Deletes task branch
/// 6. Emits task:merged and task:status_changed events
pub async fn complete_merge_internal<R: tauri::Runtime>(
    task: &mut Task,
    project: &Project,
    commit_sha: &str,
    task_repo: &Arc<dyn TaskRepository>,
    app_handle: Option<&AppHandle<R>>,
) -> AppResult<()> {
    // Clone task_id early to avoid borrow conflicts with mutable task
    let task_id = task.id.clone();
    let task_id_str = task_id.as_str();
    let old_status = task.internal_status.clone();

    tracing::info!(
        task_id = task_id_str,
        commit_sha = %commit_sha,
        old_status = ?old_status,
        "complete_merge_internal: completing merge"
    );

    // 1. Update task with merge commit SHA and status
    task.merge_commit_sha = Some(commit_sha.to_string());
    task.internal_status = InternalStatus::Merged;
    task.touch();

    task_repo.update(task).await.map_err(|e| {
        tracing::error!(error = %e, task_id = task_id_str, "Failed to update task with merge_commit_sha");
        e
    })?;

    // 2. Record status change in history
    if let Err(e) = task_repo.persist_status_change(
        &task_id,
        old_status.clone(),
        InternalStatus::Merged,
        "merge_success",
    ).await {
        tracing::warn!(error = %e, task_id = task_id_str, "Failed to record merge transition (non-fatal)");
    }

    // 3. Cleanup branch and worktree
    cleanup_branch_and_worktree_internal(task, project);

    // 4. Emit events
    if let Some(handle) = app_handle {
        let _ = handle.emit(
            "task:merged",
            serde_json::json!({
                "task_id": task_id_str,
                "commit_sha": commit_sha,
            }),
        );
        let _ = handle.emit(
            "task:status_changed",
            serde_json::json!({
                "task_id": task_id_str,
                "old_status": old_status.as_str(),
                "new_status": "merged",
            }),
        );
        let _ = handle.emit(
            "merge:completed",
            serde_json::json!({
                "task_id": task_id_str,
                "commit_sha": commit_sha,
            }),
        );
    }

    tracing::info!(
        task_id = task_id_str,
        commit_sha = %commit_sha,
        "complete_merge_internal: merge completed successfully"
    );

    Ok(())
}

/// Cleanup task branch and worktree after successful merge (standalone version).
///
/// This is the standalone version that can be called from `complete_merge_internal`.
/// For use within TransitionHandler, use the async method which has access to services.
fn cleanup_branch_and_worktree_internal(task: &Task, project: &Project) {
    let task_id_str = task.id.as_str();

    let Some(ref task_branch) = task.task_branch else {
        tracing::debug!(task_id = task_id_str, "No branch to cleanup");
        return;
    };

    let base_branch = project.base_branch.as_deref().unwrap_or("main");
    let repo_path = Path::new(&project.working_directory);

    match project.git_mode {
        GitMode::Local => {
            // For Local mode: already on base branch (from merge), just delete task branch
            match GitService::delete_branch(repo_path, task_branch, true) {
                Ok(_) => {
                    tracing::info!(
                        task_id = task_id_str,
                        branch = %task_branch,
                        "Deleted task branch after merge (Local mode)"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        task_id = task_id_str,
                        branch = %task_branch,
                        "Failed to delete task branch (non-fatal)"
                    );
                }
            }
        }
        GitMode::Worktree => {
            // For Worktree mode: delete worktree first, then branch
            if let Some(ref worktree_path) = task.worktree_path {
                let worktree_path_buf = PathBuf::from(worktree_path);
                match GitService::delete_worktree(repo_path, &worktree_path_buf) {
                    Ok(_) => {
                        tracing::info!(
                            task_id = task_id_str,
                            worktree = %worktree_path,
                            "Deleted worktree after merge"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            task_id = task_id_str,
                            worktree = %worktree_path,
                            "Failed to delete worktree (non-fatal)"
                        );
                    }
                }
            }

            // Delete the branch from main repo
            // We need to be on base branch to delete the task branch
            let _ = GitService::checkout_branch(repo_path, base_branch);
            match GitService::delete_branch(repo_path, task_branch, true) {
                Ok(_) => {
                    tracing::info!(
                        task_id = task_id_str,
                        branch = %task_branch,
                        "Deleted task branch after merge (Worktree mode)"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        task_id = task_id_str,
                        branch = %task_branch,
                        "Failed to delete task branch (non-fatal)"
                    );
                }
            }
        }
    }
}

/// Convert project name to a URL-safe slug for branch naming
fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

/// Resolve the base branch for a task's working branch.
///
/// If the task belongs to a plan with an active feature branch, returns the feature
/// branch name so the task branch is created from it. Otherwise falls back to the
/// project's base branch.
async fn resolve_task_base_branch(
    task: &Task,
    project: &Project,
    plan_branch_repo: &Option<Arc<dyn PlanBranchRepository>>,
) -> String {
    let default = project.base_branch.as_deref().unwrap_or("main").to_string();

    let Some(ref plan_branch_repo) = plan_branch_repo else {
        return default;
    };
    let Some(ref session_id) = task.ideation_session_id else {
        return default;
    };

    match plan_branch_repo.get_by_session_id(session_id).await {
        Ok(Some(pb)) if pb.status == PlanBranchStatus::Active => {
            tracing::info!(
                task_id = task.id.as_str(),
                feature_branch = %pb.branch_name,
                "Resolved task base branch to plan feature branch"
            );
            pb.branch_name
        }
        _ => default,
    }
}

/// Resolve the source and target branches for a merge operation.
///
/// Returns `(source_branch, target_branch)`:
/// - **Merge task** (task is `plan_branches.merge_task_id`): merge feature branch into project base
/// - **Plan task with feature branch**: merge task branch into feature branch
/// - **Regular task**: merge task branch into project base branch
pub async fn resolve_merge_branches(
    task: &Task,
    project: &Project,
    plan_branch_repo: &Option<Arc<dyn PlanBranchRepository>>,
) -> (String, String) {
    let base_branch = project.base_branch.as_deref().unwrap_or("main").to_string();
    let task_branch = task.task_branch.clone().unwrap_or_default();

    let Some(ref plan_branch_repo) = plan_branch_repo else {
        return (task_branch, base_branch);
    };

    // Check if this task IS the merge task for a plan branch
    if let Ok(Some(pb)) = plan_branch_repo.get_by_merge_task_id(&task.id).await {
        if pb.status == PlanBranchStatus::Active {
            tracing::info!(
                task_id = task.id.as_str(),
                feature_branch = %pb.branch_name,
                base_branch = %base_branch,
                "Merge task: merging feature branch into base"
            );
            return (pb.branch_name, base_branch);
        }
    }

    // Check if this task belongs to a plan with a feature branch
    if let Some(ref session_id) = task.ideation_session_id {
        if let Ok(Some(pb)) = plan_branch_repo.get_by_session_id(session_id).await {
            if pb.status == PlanBranchStatus::Active {
                tracing::info!(
                    task_id = task.id.as_str(),
                    task_branch = %task_branch,
                    feature_branch = %pb.branch_name,
                    "Plan task: merging task branch into feature branch"
                );
                return (task_branch, pb.branch_name);
            }
        }
    }

    (task_branch, base_branch)
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
        match state {
            State::Ready => {
                // When entering Ready, spawn QA prep agent if enabled
                if self.machine.context.qa_enabled {
                    self.machine
                        .context
                        .services
                        .agent_spawner
                        .spawn_background("qa-prep", &self.machine.context.task_id)
                        .await;
                }

                // Delay auto-scheduling so UI sees task "settle" in Ready column
                // before it potentially moves to Executing (600ms matches common UI feedback timing)
                if let Some(ref scheduler) = self.machine.context.services.task_scheduler {
                    let scheduler = Arc::clone(scheduler);
                    tokio::spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;
                        scheduler.try_schedule_ready_tasks().await;
                    });
                }
            }
            State::Executing => {
                let task_id_str = &self.machine.context.task_id;
                let project_id_str = &self.machine.context.project_id;

                // Setup branch/worktree for task isolation (Phase 66)
                // Only setup if task_repo and project_repo are available
                if let (Some(ref task_repo), Some(ref project_repo)) = (
                    &self.machine.context.services.task_repo,
                    &self.machine.context.services.project_repo,
                ) {
                    let task_id = TaskId::from_string(task_id_str.clone());
                    let project_id = ProjectId::from_string(project_id_str.clone());

                    // Fetch task and project
                    let task_result = task_repo.get_by_id(&task_id).await;
                    let project_result = project_repo.get_by_id(&project_id).await;

                    if let (Ok(Some(mut task)), Ok(Some(project))) = (task_result, project_result) {
                        // Only setup if task doesn't already have a branch
                        if task.task_branch.is_none() {
                            let branch = format!(
                                "ralphx/{}/task-{}",
                                slugify(&project.name),
                                task_id_str
                            );
                            // Resolve base branch: feature branch for plan tasks, project base otherwise
                            let plan_branch_repo = &self.machine.context.services.plan_branch_repo;
                            let resolved_base = resolve_task_base_branch(&task, &project, plan_branch_repo).await;
                            let base_branch = resolved_base.as_str();
                            let repo_path = Path::new(&project.working_directory);

                            // Attempt branch/worktree setup. Only ExecutionBlocked errors
                            // should prevent task execution (uncommitted changes in Local mode).
                            // Other git errors (missing repo, invalid path) are logged but
                            // don't block - the agent can still work in the project directory.
                            let git_result: AppResult<Option<(String, Option<String>)>> = match project.git_mode {
                                GitMode::Local => {
                                    // Block if uncommitted changes exist
                                    match GitService::has_uncommitted_changes(repo_path) {
                                        Ok(true) => {
                                            return Err(AppError::ExecutionBlocked(
                                                "Cannot execute task: uncommitted changes in working directory. \
                                                 Please commit or stash your changes first.".to_string()
                                            ));
                                        }
                                        Ok(false) => {
                                            // Create and checkout branch in main repo
                                            match GitService::create_branch(repo_path, &branch, base_branch)
                                                .and_then(|_| GitService::checkout_branch(repo_path, &branch))
                                            {
                                                Ok(_) => Ok(Some((branch.clone(), None))),
                                                Err(e) => {
                                                    tracing::warn!(
                                                        error = %e,
                                                        task_id = task_id_str,
                                                        "Failed to create/checkout task branch (Local mode), continuing without isolation"
                                                    );
                                                    Ok(None)
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tracing::warn!(
                                                error = %e,
                                                task_id = task_id_str,
                                                "Failed to check uncommitted changes, continuing without isolation"
                                            );
                                            Ok(None)
                                        }
                                    }
                                }
                                GitMode::Worktree => {
                                    // Build worktree path
                                    let worktree_parent = project
                                        .worktree_parent_directory
                                        .as_deref()
                                        .unwrap_or("~/ralphx-worktrees");
                                    // Expand ~ to home directory
                                    let worktree_parent = if let Some(stripped) = worktree_parent.strip_prefix("~/") {
                                        if let Ok(home) = std::env::var("HOME") {
                                            format!("{}/{}", home, stripped)
                                        } else {
                                            worktree_parent.to_string()
                                        }
                                    } else {
                                        worktree_parent.to_string()
                                    };

                                    let worktree_path = format!(
                                        "{}/{}/task-{}",
                                        worktree_parent,
                                        slugify(&project.name),
                                        task_id_str
                                    );
                                    let worktree_path_buf = std::path::PathBuf::from(&worktree_path);

                                    // Create worktree with new branch
                                    match GitService::create_worktree(
                                        repo_path,
                                        &worktree_path_buf,
                                        &branch,
                                        base_branch,
                                    ) {
                                        Ok(_) => Ok(Some((branch.clone(), Some(worktree_path)))),
                                        Err(e) => {
                                            tracing::warn!(
                                                error = %e,
                                                task_id = task_id_str,
                                                "Failed to create worktree (Worktree mode), continuing without isolation"
                                            );
                                            Ok(None)
                                        }
                                    }
                                }
                            };

                            // If git setup succeeded, persist the branch info
                            if let Ok(Some((branch_name, worktree_path_opt))) = git_result {
                                task.task_branch = Some(branch_name.clone());
                                if let Some(wt_path) = worktree_path_opt {
                                    task.worktree_path = Some(wt_path.clone());
                                    tracing::info!(
                                        task_id = task_id_str,
                                        branch = %branch_name,
                                        worktree_path = %wt_path,
                                        "Created worktree with task branch (Worktree mode)"
                                    );
                                } else {
                                    tracing::info!(
                                        task_id = task_id_str,
                                        branch = %branch_name,
                                        "Created and checked out task branch (Local mode)"
                                    );
                                }
                                task.touch();
                                if let Err(e) = task_repo.update(&task).await {
                                    tracing::error!(error = %e, "Failed to persist task branch info");
                                }
                            }
                        }
                    }
                }

                // Use ChatService for persistent worker execution (Phase 15B)
                let prompt = format!("Execute task: {}", task_id_str);
                eprintln!(
                    "[STREAM_DEBUG] transition handler sending task_execution message (task_id={}, prompt_len={})",
                    task_id_str,
                    prompt.len()
                );

                // send_message handles:
                // 1. Creating chat_conversation (context_type: 'task_execution')
                // 2. Creating agent_run (status: 'running')
                // 3. Spawning Claude CLI with --agent worker
                // 4. Persisting stream output to chat_messages
                // 5. Processing queued messages on completion
                let _ = self
                    .machine
                    .context
                    .services
                    .chat_service
                    .send_message(
                        crate::domain::entities::ChatContextType::TaskExecution,
                        task_id_str,
                        &prompt,
                    )
                    .await;
            }
            State::QaRefining => {
                // Wait for QA prep if not complete, then spawn QA refiner
                if !self.machine.context.qa_prep_complete {
                    self.machine
                        .context
                        .services
                        .agent_spawner
                        .wait_for("qa-prep", &self.machine.context.task_id)
                        .await;
                }
                self.machine
                    .context
                    .services
                    .agent_spawner
                    .spawn("qa-refiner", &self.machine.context.task_id)
                    .await;
            }
            State::QaTesting => {
                // Spawn QA tester agent
                self.machine
                    .context
                    .services
                    .agent_spawner
                    .spawn("qa-tester", &self.machine.context.task_id)
                    .await;
            }
            State::QaPassed => {
                // Emit QA passed event
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("qa_passed", &self.machine.context.task_id)
                    .await;
            }
            State::QaFailed(data) => {
                // Emit QA failed event and notify user
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("qa_failed", &self.machine.context.task_id)
                    .await;

                // Notify user if not already notified
                if !data.notified {
                    let message = format!(
                        "QA tests failed: {} failure(s)",
                        data.failure_count()
                    );
                    self.machine
                        .context
                        .services
                        .notifier
                        .notify_with_message(
                            "qa_failed",
                            &self.machine.context.task_id,
                            &message,
                        )
                        .await;
                }
            }
            State::PendingReview => {
                // Start AI review via ReviewStarter
                let review_result = self.machine
                    .context
                    .services
                    .review_starter
                    .start_ai_review(
                        &self.machine.context.task_id,
                        &self.machine.context.project_id,
                    )
                    .await;

                // Emit review:update event with the result
                match &review_result {
                    super::super::services::ReviewStartResult::Started { review_id } => {
                        self.machine
                            .context
                            .services
                            .event_emitter
                            .emit_with_payload(
                                "review:update",
                                &self.machine.context.task_id,
                                &format!(r#"{{"type":"started","reviewId":"{}"}}"#, review_id),
                            )
                            .await;
                    }
                    super::super::services::ReviewStartResult::Disabled => {
                        // AI review disabled, emit event but don't spawn agent
                        self.machine
                            .context
                            .services
                            .event_emitter
                            .emit_with_payload(
                                "review:update",
                                &self.machine.context.task_id,
                                r#"{"type":"disabled"}"#,
                            )
                            .await;
                    }
                    super::super::services::ReviewStartResult::Error(msg) => {
                        // Review failed to start, notify user
                        self.machine
                            .context
                            .services
                            .notifier
                            .notify_with_message(
                                "review_error",
                                &self.machine.context.task_id,
                                msg,
                            )
                            .await;
                    }
                }
            }
            State::Reviewing => {
                // For Local mode: checkout task branch before spawning reviewer
                // (Worktree mode already has isolated directory)
                self.checkout_task_branch_if_needed("Reviewing").await;

                // Spawn reviewer agent via ChatService with Review context
                let task_id = &self.machine.context.task_id;
                let prompt = format!("Review task: {}", task_id);

                eprintln!("[REVIEWING] on_enter(Reviewing) called for task: {}", task_id);
                tracing::info!(
                    task_id = task_id,
                    "on_enter(Reviewing): Spawning reviewer agent via ChatService"
                );

                let result = self
                    .machine
                    .context
                    .services
                    .chat_service
                    .send_message(
                        crate::domain::entities::ChatContextType::Review,
                        task_id,
                        &prompt,
                    )
                    .await;

                match &result {
                    Ok(_) => {
                        eprintln!("[REVIEWING] Reviewer agent spawned successfully for task: {}", task_id);
                        tracing::info!(task_id = task_id, "Reviewer agent spawned successfully");
                    }
                    Err(e) => {
                        eprintln!("[REVIEWING] FAILED to spawn reviewer agent: {}", e);
                        tracing::error!(task_id = task_id, error = %e, "Failed to spawn reviewer agent");
                    }
                }
            }
            State::ReviewPassed => {
                // Emit 'review:ai_approved' event
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("review:ai_approved", &self.machine.context.task_id)
                    .await;

                // Notify user that review passed and awaits approval
                self.machine
                    .context
                    .services
                    .notifier
                    .notify_with_message(
                        "review:ai_approved",
                        &self.machine.context.task_id,
                        "AI review passed. Please review and approve.",
                    )
                    .await;
            }
            State::Escalated => {
                // Emit 'review:escalated' event
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("review:escalated", &self.machine.context.task_id)
                    .await;

                // Notify user that AI escalated review
                self.machine
                    .context
                    .services
                    .notifier
                    .notify_with_message(
                        "review:escalated",
                        &self.machine.context.task_id,
                        "AI review escalated. Please review and decide.",
                    )
                    .await;
            }
            State::ReExecuting => {
                // For Local mode: checkout task branch before spawning worker
                // (Worktree mode already has isolated directory)
                self.checkout_task_branch_if_needed("ReExecuting").await;

                // Spawn worker agent with revision context via ChatService
                let task_id = &self.machine.context.task_id;
                let prompt = format!("Re-execute task (revision): {}", task_id);

                let _ = self
                    .machine
                    .context
                    .services
                    .chat_service
                    .send_message(
                        crate::domain::entities::ChatContextType::TaskExecution,
                        task_id,
                        &prompt,
                    )
                    .await;
            }
            State::RevisionNeeded => {
                // Auto-transition to ReExecuting will be handled by check_auto_transition
            }
            State::Approved => {
                // Emit task completed event
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("task_completed", &self.machine.context.task_id)
                    .await;
                // NOTE: Do NOT unblock dependents here. Approved auto-transitions to
                // PendingMerge (Phase 66). Unblocking happens at on_enter(Merged) after
                // the task's work is actually on main.
            }
            State::Failed(_) => {
                // Emit task failed event
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("task_failed", &self.machine.context.task_id)
                    .await;
            }
            State::PendingMerge => {
                // Phase 1 of merge workflow: Attempt programmatic rebase and merge
                // This is the "fast path" - if successful, skip agent entirely
                self.attempt_programmatic_merge().await;
            }
            State::Merging => {
                // Phase 2 of merge workflow: Spawn merger agent for conflict resolution
                // This state is reached when Phase 1 (programmatic merge) failed due to conflicts
                let task_id = &self.machine.context.task_id;
                let prompt = format!("Resolve merge conflicts for task: {}", task_id);

                tracing::info!(
                    task_id = task_id,
                    "on_enter(Merging): Spawning merger agent via ChatService"
                );

                // Use ChatService with Merge context type for the merger agent
                let result = self
                    .machine
                    .context
                    .services
                    .chat_service
                    .send_message(
                        crate::domain::entities::ChatContextType::Merge,
                        task_id,
                        &prompt,
                    )
                    .await;

                match &result {
                    Ok(_) => {
                        tracing::info!(task_id = task_id, "Merger agent spawned successfully");
                    }
                    Err(e) => {
                        tracing::error!(task_id = task_id, error = %e, "Failed to spawn merger agent");
                    }
                }
            }
            State::Merged => {
                // Auto-unblock tasks that were waiting on this task
                // This handles the HTTP handler path where transition_task triggers on_enter
                self.machine
                    .context
                    .services
                    .dependency_manager
                    .unblock_dependents(&self.machine.context.task_id)
                    .await;
            }
            _ => {}
        }
        Ok(())
    }

    /// For Local mode: checkout task branch if current branch differs.
    /// This is needed when re-entering execution states (ReExecuting, Reviewing)
    /// where the task already has a branch but we may be on a different branch.
    /// Worktree mode doesn't need this as each task has its own isolated directory.
    async fn checkout_task_branch_if_needed(&self, state_name: &str) {
        let task_id_str = &self.machine.context.task_id;
        let project_id_str = &self.machine.context.project_id;

        if let (Some(ref task_repo), Some(ref project_repo)) = (
            &self.machine.context.services.task_repo,
            &self.machine.context.services.project_repo,
        ) {
            let task_id = TaskId::from_string(task_id_str.clone());
            let project_id = ProjectId::from_string(project_id_str.clone());

            let task_result = task_repo.get_by_id(&task_id).await;
            let project_result = project_repo.get_by_id(&project_id).await;

            if let (Ok(Some(task)), Ok(Some(project))) = (task_result, project_result) {
                // Only checkout for Local mode - Worktree mode already has isolated directory
                if project.git_mode == GitMode::Local {
                    if let Some(branch) = &task.task_branch {
                        let repo_path = Path::new(&project.working_directory);
                        match GitService::get_current_branch(repo_path) {
                            Ok(current) if current != *branch => {
                                match GitService::checkout_branch(repo_path, branch) {
                                    Ok(_) => {
                                        tracing::info!(
                                            task_id = task_id_str,
                                            branch = %branch,
                                            from_branch = %current,
                                            state = state_name,
                                            "Checked out task branch (Local mode)"
                                        );
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            error = %e,
                                            task_id = task_id_str,
                                            branch = %branch,
                                            state = state_name,
                                            "Failed to checkout task branch (Local mode)"
                                        );
                                    }
                                }
                            }
                            Ok(_) => {
                                // Already on correct branch
                                tracing::debug!(
                                    task_id = task_id_str,
                                    branch = %branch,
                                    state = state_name,
                                    "Already on task branch (Local mode)"
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    error = %e,
                                    task_id = task_id_str,
                                    state = state_name,
                                    "Failed to get current branch"
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// Attempt programmatic rebase and merge (Phase 1 of merge workflow).
    ///
    /// This is the "fast path" - try to rebase task branch onto base and merge.
    /// If successful, transition directly to Merged and cleanup branch/worktree.
    /// If conflicts occur, transition to Merging for agent-assisted resolution.
    async fn attempt_programmatic_merge(&self) {
        let task_id_str = &self.machine.context.task_id;
        let project_id_str = &self.machine.context.project_id;

        // Only proceed if repos are available
        let (Some(ref task_repo), Some(ref project_repo)) = (
            &self.machine.context.services.task_repo,
            &self.machine.context.services.project_repo,
        ) else {
            tracing::warn!(
                task_id = task_id_str,
                "Skipping programmatic merge: repos not available"
            );
            return;
        };

        let task_id = TaskId::from_string(task_id_str.clone());
        let project_id = ProjectId::from_string(project_id_str.clone());

        // Fetch task and project
        let task_result = task_repo.get_by_id(&task_id).await;
        let project_result = project_repo.get_by_id(&project_id).await;

        let (Ok(Some(mut task)), Ok(Some(project))) = (task_result, project_result) else {
            tracing::warn!(
                task_id = task_id_str,
                "Skipping programmatic merge: failed to fetch task or project"
            );
            return;
        };

        // Resolve source and target branches (handles merge tasks and plan feature branches)
        let plan_branch_repo = &self.machine.context.services.plan_branch_repo;
        let (source_branch, target_branch) = resolve_merge_branches(&task, &project, plan_branch_repo).await;

        // Ensure we have a source branch to merge
        if source_branch.is_empty() {
            // For regular tasks, source_branch comes from task_branch which may be None
            if task.task_branch.is_none() {
                tracing::warn!(
                    task_id = task_id_str,
                    "Skipping programmatic merge: task has no branch"
                );
                return;
            }
        }

        let repo_path = Path::new(&project.working_directory);

        tracing::info!(
            task_id = task_id_str,
            source_branch = %source_branch,
            target_branch = %target_branch,
            git_mode = ?project.git_mode,
            "Attempting programmatic merge (Phase 1)"
        );

        // In worktree mode, delete the worktree first to unlock the branch.
        // Git refuses to checkout a branch that's checked out in another worktree,
        // so we must remove the worktree before the merge can checkout the task branch.
        if project.git_mode == GitMode::Worktree {
            if let Some(ref worktree_path) = task.worktree_path {
                let worktree_path_buf = PathBuf::from(worktree_path);
                if worktree_path_buf.exists() {
                    tracing::info!(
                        task_id = task_id_str,
                        worktree_path = %worktree_path,
                        "Deleting worktree before programmatic merge to unlock branch"
                    );
                    if let Err(e) = GitService::delete_worktree(repo_path, &worktree_path_buf) {
                        tracing::error!(
                            task_id = task_id_str,
                            error = %e,
                            worktree_path = %worktree_path,
                            "Failed to delete worktree before merge"
                        );
                        // Continue anyway - merge will fail with a clear error
                    }
                }
            }
        }

        // Attempt the merge: worktree mode uses git merge (no rebase) to avoid
        // failing when the main repo has unrelated unstaged changes.
        // Local mode uses rebase for linear history.
        let merge_result = if project.git_mode == GitMode::Worktree {
            GitService::try_merge(repo_path, &source_branch, &target_branch)
        } else {
            GitService::try_rebase_and_merge(repo_path, &source_branch, &target_branch)
        };
        match merge_result {
            Ok(MergeAttemptResult::Success { commit_sha }) => {
                // Fast path success: merge completed without conflicts
                tracing::info!(
                    task_id = task_id_str,
                    commit_sha = %commit_sha,
                    "Programmatic merge succeeded (fast path)"
                );

                // Use shared merge completion logic
                let app_handle = self.machine.context.services.app_handle.as_ref();
                if let Err(e) = complete_merge_internal(
                    &mut task,
                    &project,
                    &commit_sha,
                    task_repo,
                    app_handle,
                ).await {
                    tracing::error!(error = %e, task_id = task_id_str, "Failed to complete programmatic merge");
                } else {
                    // Post-merge cleanup for merge tasks: update plan_branch status,
                    // delete feature branch, emit event
                    if let Some(ref plan_branch_repo) = plan_branch_repo {
                        if let Ok(Some(pb)) = plan_branch_repo.get_by_merge_task_id(&task_id).await {
                            // Mark plan branch as merged
                            if let Err(e) = plan_branch_repo.set_merged(&pb.id).await {
                                tracing::warn!(
                                    error = %e,
                                    task_id = task_id_str,
                                    plan_branch_id = pb.id.as_str(),
                                    "Failed to mark plan branch as merged (non-fatal)"
                                );
                            }

                            // Delete the feature branch from git
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

                            // Emit plan-merge-complete event
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

                    // Auto-unblock tasks that were waiting on this task
                    // (programmatic merge path - on_enter(Merged) won't be triggered)
                    self.machine
                        .context
                        .services
                        .dependency_manager
                        .unblock_dependents(task_id_str)
                        .await;
                }
            }
            Ok(MergeAttemptResult::NeedsAgent { conflict_files }) => {
                // Conflict detected: transition to Merging for agent resolution
                tracing::info!(
                    task_id = task_id_str,
                    conflict_count = conflict_files.len(),
                    "Programmatic merge failed: conflicts detected, transitioning to Merging"
                );

                // Log conflict files for debugging
                for file in &conflict_files {
                    tracing::debug!(
                        task_id = task_id_str,
                        file = %file.display(),
                        "Conflict file"
                    );
                }

                // Update task status to Merging (agent phase)
                task.internal_status = InternalStatus::Merging;
                task.touch();

                if let Err(e) = task_repo.update(&task).await {
                    tracing::error!(error = %e, "Failed to update task to Merging status");
                    return;
                }

                // Record status change in history
                if let Err(e) = task_repo.persist_status_change(
                    &task_id,
                    InternalStatus::PendingMerge,
                    InternalStatus::Merging,
                    "merge_conflict",
                ).await {
                    tracing::warn!(error = %e, "Failed to record merge conflict transition (non-fatal)");
                }

                // Emit event for UI update
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("task:merge_conflict", task_id_str)
                    .await;

                // Spawn merger agent directly (on_enter(Merging) won't be called automatically
                // when we update status directly - TaskTransitionService only triggers entry
                // actions via transition_task() or execute_entry_actions())
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
                    Ok(_) => {
                        tracing::info!(task_id = task_id_str, "Merger agent spawned successfully");
                    }
                    Err(e) => {
                        tracing::error!(task_id = task_id_str, error = %e, "Failed to spawn merger agent");
                    }
                }
            }
            Err(e) => {
                // Git operation failed (not a conflict, but an error)
                tracing::error!(
                    task_id = task_id_str,
                    error = %e,
                    worktree_path = ?task.worktree_path,
                    source_branch = %source_branch,
                    target_branch = %target_branch,
                    repo_path = %repo_path.display(),
                    "Programmatic merge failed due to error, transitioning to MergeIncomplete"
                );

                // Transition to MergeIncomplete - distinguishes non-conflict errors from actual conflicts
                task.internal_status = InternalStatus::MergeIncomplete;
                task.touch();

                if let Err(e) = task_repo.update(&task).await {
                    tracing::error!(error = %e, "Failed to update task to MergeIncomplete status");
                    return;
                }

                // Record status change in history
                if let Err(e) = task_repo.persist_status_change(
                    &task_id,
                    InternalStatus::PendingMerge,
                    InternalStatus::MergeIncomplete,
                    "merge_incomplete",
                ).await {
                    tracing::warn!(error = %e, "Failed to record merge incomplete transition (non-fatal)");
                }

                // Emit event for UI update — MergeIncomplete is a human-waiting state
                // (user clicks "Retry" which transitions to Merging where agent spawns correctly)
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("task:status_changed", task_id_str)
                    .await;
            }
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{
        ArtifactId, PlanBranch, PlanBranchStatus, ProjectId, TaskId,
    };
    use crate::domain::entities::types::IdeationSessionId;
    use crate::infrastructure::memory::MemoryPlanBranchRepository;

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
        let mut t = Task::new(ProjectId::from_string("proj-1".to_string()), "Test task".into());
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
    async fn resolve_task_base_branch_returns_feature_branch_when_active() {
        let project = make_project(Some("main"));
        let task = make_task_with_session(Some("art-1"), None, Some("sess-1"));

        let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
        let pb = make_plan_branch("art-1", "ralphx/test/plan-abc123", PlanBranchStatus::Active, None);
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
        let pb = make_plan_branch("art-1", "ralphx/test/plan-abc123", PlanBranchStatus::Merged, None);
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
        let pb = make_plan_branch("art-1", "ralphx/test/plan-abc123", PlanBranchStatus::Abandoned, None);
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
        let pb = make_plan_branch("art-other", "ralphx/test/plan-abc123", PlanBranchStatus::Active, None);
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
        let mut task = make_task_with_session(Some("art-1"), Some("ralphx/test/task-456"), Some("sess-1"));
        task.id = TaskId::from_string("task-456".to_string());

        let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
        let pb = make_plan_branch("art-1", "ralphx/test/plan-abc123", PlanBranchStatus::Active, None);
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
        let mut task = make_task_with_session(Some("art-3"), Some("ralphx/test/task-abandoned"), Some("sess-1"));
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
        let mut task = make_task_with_session(Some("art-1"), Some("ralphx/test/task-dual"), Some("sess-1"));
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
}
