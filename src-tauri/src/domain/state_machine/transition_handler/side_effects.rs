// State entry side effects
// This module contains the on_enter implementation that handles state-specific actions

use std::path::Path;
use std::sync::Arc;

use super::super::machine::State;
use crate::application::GitService;
use crate::domain::entities::{GitMode, TaskId, ProjectId};
use crate::error::{AppError, AppResult};

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
}
