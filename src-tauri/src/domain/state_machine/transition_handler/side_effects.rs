// State entry side effects
// This module contains the on_enter implementation that handles state-specific actions

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::{AppHandle, Emitter};

use super::super::machine::State;
use crate::application::{GitService, MergeAttemptResult};
use crate::domain::entities::{GitMode, InternalStatus, Project, Task, TaskId, ProjectId};
use crate::domain::repositories::TaskRepository;
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
                            let base_branch = project.base_branch.as_deref().unwrap_or("main");
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
                // Unblock dependent tasks
                self.machine
                    .context
                    .services
                    .dependency_manager
                    .unblock_dependents(&self.machine.context.task_id)
                    .await;
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

        // Ensure task has a branch to merge
        let Some(ref task_branch) = task.task_branch else {
            tracing::warn!(
                task_id = task_id_str,
                "Skipping programmatic merge: task has no branch"
            );
            return;
        };

        let base_branch = project.base_branch.as_deref().unwrap_or("main");
        let repo_path = Path::new(&project.working_directory);

        tracing::info!(
            task_id = task_id_str,
            task_branch = %task_branch,
            base_branch = %base_branch,
            "Attempting programmatic rebase and merge (Phase 1)"
        );

        // Attempt the rebase and merge
        match GitService::try_rebase_and_merge(repo_path, task_branch, base_branch) {
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
                    "Programmatic merge failed due to error, transitioning to Merging"
                );

                // Still transition to Merging - agent might be able to handle it
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
                    "merge_error",
                ).await {
                    tracing::warn!(error = %e, "Failed to record merge error transition (non-fatal)");
                }

                // Spawn merger agent directly (same as conflict case)
                let prompt = format!("Resolve merge conflicts for task: {}", task_id_str);
                tracing::info!(
                    task_id = task_id_str,
                    "Spawning merger agent for error recovery (from attempt_programmatic_merge)"
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
        }
    }

}
