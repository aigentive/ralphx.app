// State entry dispatch — all `on_enter` match arms and helpers.
//
// Extracted from side_effects.rs for maintainability. The `on_enter` method
// signature stays in side_effects.rs and delegates here.

use std::path::Path;
use std::sync::Arc;

use chrono::Utc;

use super::super::machine::State;
use super::merge_helpers::{
    compute_merge_worktree_path, expand_home, resolve_task_base_branch, slugify,
};
use super::metadata_builder::{build_failed_metadata, MetadataUpdate};
use crate::application::GitService;
use crate::domain::entities::{ProjectId, TaskId, TaskStepStatus};
use crate::error::{AppError, AppResult};
use crate::infrastructure::agents::claude::scheduler_config;

impl<'a> super::TransitionHandler<'a> {
    /// Run pre-execution setup (worktree_setup + install), store log in metadata.
    /// Returns Err if setup fails in Block/AutoFix mode.
    async fn run_and_store_pre_execution_setup(
        &self,
        task_id_str: &str,
        project_id_str: &str,
        context: &str,      // "execution" or "review"
        metadata_key: &str, // "execution_setup_log" or "review_setup_log"
    ) -> AppResult<()> {
        // Run pre-execution setup (worktree_setup + install) before spawning agent
        if let (Some(ref task_repo), Some(ref project_repo)) = (
            &self.machine.context.services.task_repo,
            &self.machine.context.services.project_repo,
        ) {
            let task_id = TaskId::from_string(task_id_str.to_string());
            let project_id = ProjectId::from_string(project_id_str.to_string());

            let task_result = task_repo.get_by_id(&task_id).await;
            let project_result = project_repo.get_by_id(&project_id).await;

            if let (Ok(Some(task)), Ok(Some(project))) = (task_result, project_result) {
                // Skip pre-exec setup if mode is Off
                use crate::domain::entities::MergeValidationMode;
                if project.merge_validation_mode != MergeValidationMode::Off {
                    // Determine execution directory (worktree_path or working_directory)
                    let exec_cwd = if let Some(ref wt_path) = task.worktree_path {
                        std::path::PathBuf::from(wt_path)
                    } else {
                        std::path::PathBuf::from(&project.working_directory)
                    };

                    // Only run pre-execution setup if exec_cwd exists
                    if !exec_cwd.exists() {
                        tracing::warn!(
                            task_id = task_id_str,
                            exec_cwd = %exec_cwd.display(),
                            "Execution directory does not exist, skipping pre-execution setup"
                        );
                    } else if let Some(setup_result) =
                        super::merge_validation::run_pre_execution_setup(
                            &project,
                            &task,
                            &exec_cwd,
                            task_id_str,
                            self.machine.context.services.app_handle.as_ref(),
                            context,
                        )
                        .await
                    {
                        // Store setup log in metadata (using update_metadata for targeted write)
                        if let Ok(Some(task_updated)) = task_repo.get_by_id(&task_id).await {
                            let log_json = serde_json::to_value(&setup_result.log)
                                .unwrap_or_else(|_| serde_json::Value::Array(Vec::new()));

                            let mut metadata_obj =
                                if let Some(json_str) = task_updated.metadata.as_ref() {
                                    serde_json::from_str::<serde_json::Value>(json_str)
                                        .unwrap_or_else(|_| serde_json::json!({}))
                                } else {
                                    serde_json::json!({})
                                };

                            if let Some(obj) = metadata_obj.as_object_mut() {
                                obj.insert(metadata_key.to_string(), log_json);
                            }

                            if let Ok(updated_metadata) = serde_json::to_string(&metadata_obj) {
                                let _ = task_repo
                                    .update_metadata(&task_id, Some(updated_metadata))
                                    .await;
                            }
                        }

                        // Handle setup failure based on merge_validation_mode
                        if !setup_result.success {
                            match project.merge_validation_mode {
                                MergeValidationMode::Block | MergeValidationMode::AutoFix => {
                                    tracing::error!(
                                        task_id = task_id_str,
                                        "Pre-execution setup failed (install command failed). Blocking execution."
                                    );
                                    return Err(AppError::ExecutionBlocked(
                                        format!("Pre-execution setup failed: install command(s) failed. Check {} in task metadata for details.", metadata_key)
                                    ));
                                }
                                MergeValidationMode::Warn => {
                                    tracing::warn!(
                                        task_id = task_id_str,
                                        "Pre-execution setup failed (install command failed). Proceeding with warning."
                                    );
                                    // Store warning in metadata but proceed (using update_metadata for targeted write)
                                    if let Ok(Some(task_updated)) =
                                        task_repo.get_by_id(&task_id).await
                                    {
                                        let mut metadata_obj = if let Some(json_str) =
                                            task_updated.metadata.as_ref()
                                        {
                                            serde_json::from_str::<serde_json::Value>(json_str)
                                                .unwrap_or_else(|_| serde_json::json!({}))
                                        } else {
                                            serde_json::json!({})
                                        };

                                        if let Some(obj) = metadata_obj.as_object_mut() {
                                            obj.insert(
                                                "execution_setup_warning".to_string(),
                                                serde_json::json!(true),
                                            );
                                        }

                                        if let Ok(updated_metadata) =
                                            serde_json::to_string(&metadata_obj)
                                        {
                                            let _ = task_repo
                                                .update_metadata(&task_id, Some(updated_metadata))
                                                .await;
                                        }
                                    }
                                }
                                MergeValidationMode::Off => {
                                    // Already skipped above, but for completeness
                                }
                            }
                        }
                    } // end if let Some(setup_result)
                } // end if project.merge_validation_mode != Off
            } // end if let (Ok(Some(task)), Ok(Some(project)))
        } // end if let (Some(ref task_repo), Some(ref project_repo))
        Ok(())
    }

    /// Execute on-enter dispatch for all state arms.
    ///
    /// Called by `on_enter` in side_effects.rs.
    pub(super) async fn on_enter_dispatch(&self, state: &State) -> AppResult<()> {
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
                // before it potentially moves to Executing (user-visible → ready_settle_ms)
                if let Some(ref scheduler) = self.machine.context.services.task_scheduler {
                    let scheduler = Arc::clone(scheduler);
                    let ready_settle_ms = scheduler_config().ready_settle_ms;
                    tokio::spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_millis(ready_settle_ms))
                            .await;
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
                            let branch =
                                format!("ralphx/{}/task-{}", slugify(&project.name), task_id_str);
                            // Resolve base branch: feature branch for plan tasks, project base otherwise
                            let plan_branch_repo = &self.machine.context.services.plan_branch_repo;
                            let resolved_base =
                                resolve_task_base_branch(&task, &project, plan_branch_repo).await;
                            let base_branch = resolved_base.as_str();
                            let repo_path = Path::new(&project.working_directory);

                            // Attempt branch/worktree setup. Git isolation failures MUST
                            // block execution to prevent agents from writing to main branch.
                            // All git errors return ExecutionBlocked to fail the task.
                            let git_result: AppResult<Option<(String, Option<String>)>> = {
                                // Build worktree path
                                let worktree_parent = project
                                    .worktree_parent_directory
                                    .as_deref()
                                    .unwrap_or("~/ralphx-worktrees");
                                let expanded_parent = expand_home(worktree_parent);

                                let worktree_path = format!(
                                    "{}/{}/task-{}",
                                    expanded_parent,
                                    slugify(&project.name),
                                    task_id_str
                                );
                                let worktree_path_buf =
                                    std::path::PathBuf::from(&worktree_path);

                                // Check if branch already exists from a previous execution attempt
                                let branch_exists =
                                    GitService::branch_exists(repo_path, &branch).await;

                                // Create worktree - use existing branch if it exists, create new one otherwise
                                let result = if branch_exists {
                                    tracing::info!(
                                        task_id = task_id_str,
                                        branch = %branch,
                                        "Branch already exists, checking out existing branch into worktree"
                                    );
                                    GitService::checkout_existing_branch_worktree(
                                        repo_path,
                                        &worktree_path_buf,
                                        &branch,
                                    )
                                    .await
                                } else {
                                    GitService::create_worktree(
                                        repo_path,
                                        &worktree_path_buf,
                                        &branch,
                                        base_branch,
                                    )
                                    .await
                                };

                                match result {
                                    Ok(_) => {
                                        Ok(Some((branch.clone(), Some(worktree_path))))
                                    }
                                    Err(e) => {
                                        return Err(AppError::ExecutionBlocked(
                                            format!("Git isolation failed: could not create worktree at '{}': {}", worktree_path, e)
                                        ));
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
                                        "Created worktree with task branch"
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

                // Run pre-execution setup (worktree_setup + install) before spawning agent
                self.run_and_store_pre_execution_setup(
                    task_id_str,
                    project_id_str,
                    "execution",
                    "execution_setup_log",
                )
                .await?;

                // Use ChatService for persistent worker execution (Phase 15B)
                // Read restart_note from metadata (one-shot: append to prompt, then clear)
                let mut prompt = format!("Execute task: {}", task_id_str);
                if let Some(ref task_repo) = self.machine.context.services.task_repo {
                    let task_id_typed = TaskId::from_string(task_id_str.clone());
                    if let Ok(Some(task)) = task_repo.get_by_id(&task_id_typed).await {
                        if let Some(note) = extract_restart_note(task.metadata.as_deref()) {
                            prompt = format!("{}\n\nUser note: {}", prompt, note);
                            // Clear restart_note from metadata (one-shot consumption)
                            let cleared = MetadataUpdate::new()
                                .with_null("restart_note")
                                .merge_into(task.metadata.as_deref());
                            if let Err(e) = task_repo
                                .update_metadata(&task_id_typed, Some(cleared))
                                .await
                            {
                                tracing::warn!(
                                    task_id = task_id_str,
                                    error = %e,
                                    "Failed to clear restart_note from metadata"
                                );
                            }
                        }
                    }
                }
                tracing::debug!(
                    task_id = task_id_str,
                    prompt_len = prompt.len(),
                    "Transition handler sending task_execution message"
                );

                // send_message handles:
                // 1. Creating chat_conversation (context_type: 'task_execution')
                // 2. Creating agent_run (status: 'running')
                // 3. Spawning Claude CLI with --agent worker
                // 4. Persisting stream output to chat_messages
                // 5. Processing queued messages on completion
                if let Err(e) = self
                    .machine
                    .context
                    .services
                    .chat_service
                    .send_message(
                        crate::domain::entities::ChatContextType::TaskExecution,
                        task_id_str,
                        &prompt,
                    )
                    .await
                {
                    tracing::error!(
                        task_id = task_id_str,
                        error = %e,
                        "Failed to send task execution message — agent not started"
                    );
                    return Err(AppError::ExecutionBlocked(format!(
                        "Failed to start agent: {}",
                        e
                    )));
                }
            }
            State::QaRefining => {
                // Set trigger_origin="qa" for QA cycle (skip if already set by transition_task_with_metadata)
                if let Some(ref task_repo) = self.machine.context.services.task_repo {
                    let task_id = TaskId::from_string(self.machine.context.task_id.clone());
                    if let Ok(Some(task)) = task_repo.get_by_id(&task_id).await {
                        if !MetadataUpdate::key_exists_in(
                            "trigger_origin",
                            task.metadata.as_deref(),
                        ) {
                            // Fallback: metadata not pre-computed, write it now for backward compatibility
                            let metadata_update =
                                super::metadata_builder::build_trigger_origin_metadata("qa");
                            let merged_metadata =
                                metadata_update.merge_into(task.metadata.as_deref());

                            if let Err(e) = task_repo
                                .update_metadata(&task_id, Some(merged_metadata))
                                .await
                            {
                                tracing::error!(
                                    task_id = %self.machine.context.task_id,
                                    error = %e,
                                    "Failed to set trigger_origin=qa in metadata"
                                );
                            }
                        } else {
                            tracing::debug!(
                                task_id = %self.machine.context.task_id,
                                "Skipping metadata write - trigger_origin already present (pre-computed)"
                            );
                        }
                    }
                }

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
                // Set trigger_origin="qa" for QA cycle (skip if already set by transition_task_with_metadata)
                if let Some(ref task_repo) = self.machine.context.services.task_repo {
                    let task_id = TaskId::from_string(self.machine.context.task_id.clone());
                    if let Ok(Some(task)) = task_repo.get_by_id(&task_id).await {
                        if !MetadataUpdate::key_exists_in(
                            "trigger_origin",
                            task.metadata.as_deref(),
                        ) {
                            // Fallback: metadata not pre-computed, write it now for backward compatibility
                            let metadata_update =
                                super::metadata_builder::build_trigger_origin_metadata("qa");
                            let merged_metadata =
                                metadata_update.merge_into(task.metadata.as_deref());

                            if let Err(e) = task_repo
                                .update_metadata(&task_id, Some(merged_metadata))
                                .await
                            {
                                tracing::error!(
                                    task_id = %self.machine.context.task_id,
                                    error = %e,
                                    "Failed to set trigger_origin=qa in metadata"
                                );
                            }
                        } else {
                            tracing::debug!(
                                task_id = %self.machine.context.task_id,
                                "Skipping metadata write - trigger_origin already present (pre-computed)"
                            );
                        }
                    }
                }

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
                    let message = format!("QA tests failed: {} failure(s)", data.failure_count());
                    self.machine
                        .context
                        .services
                        .notifier
                        .notify_with_message("qa_failed", &self.machine.context.task_id, &message)
                        .await;
                }
            }
            State::PendingReview => {
                // Start AI review via ReviewStarter
                let review_result = self
                    .machine
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
                            .notify_with_message("review_error", &self.machine.context.task_id, msg)
                            .await;
                    }
                }
            }
            State::Reviewing => {
                // Run pre-execution setup before reviewing
                let project_id_str = &self.machine.context.project_id;
                let task_id = &self.machine.context.task_id;
                self.run_and_store_pre_execution_setup(
                    task_id,
                    project_id_str,
                    "review",
                    "review_setup_log",
                )
                .await?;

                // Spawn reviewer agent via ChatService with Review context
                let prompt = format!("Review task: {}", task_id);

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
                        tracing::info!(task_id = task_id, "Reviewer agent spawned successfully");
                    }
                    Err(e) => {
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
                // Run pre-execution setup before re-executing
                let task_id_str = &self.machine.context.task_id;
                let project_id_str = &self.machine.context.project_id;
                self.run_and_store_pre_execution_setup(
                    task_id_str,
                    project_id_str,
                    "execution",
                    "execution_setup_log",
                )
                .await?;

                // Spawn worker agent with revision context via ChatService
                let task_id = &self.machine.context.task_id;
                // Read restart_note from metadata (one-shot: append to prompt, then clear)
                let mut prompt = format!("Re-execute task (revision): {}", task_id);
                if let Some(ref task_repo) = self.machine.context.services.task_repo {
                    let task_id_typed = TaskId::from_string(task_id.clone());
                    if let Ok(Some(task)) = task_repo.get_by_id(&task_id_typed).await {
                        if let Some(note) = extract_restart_note(task.metadata.as_deref()) {
                            prompt = format!("{}\n\nUser note: {}", prompt, note);
                            // Clear restart_note from metadata (one-shot consumption)
                            let cleared = MetadataUpdate::new()
                                .with_null("restart_note")
                                .merge_into(task.metadata.as_deref());
                            if let Err(e) = task_repo
                                .update_metadata(&task_id_typed, Some(cleared))
                                .await
                            {
                                tracing::warn!(
                                    task_id = task_id,
                                    error = %e,
                                    "Failed to clear restart_note from metadata"
                                );
                            }
                        }
                    }
                }

                if let Err(e) = self
                    .machine
                    .context
                    .services
                    .chat_service
                    .send_message(
                        crate::domain::entities::ChatContextType::TaskExecution,
                        task_id,
                        &prompt,
                    )
                    .await
                {
                    tracing::error!(
                        task_id = task_id,
                        error = %e,
                        "Failed to send re-execution message — agent not started"
                    );
                    return Err(AppError::ExecutionBlocked(format!(
                        "Failed to start agent: {}",
                        e
                    )));
                }
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
            State::Failed(data) => {
                let task_id = &self.machine.context.task_id;

                // Store failure reason in task metadata for frontend access
                if let Some(ref task_repo) = self.machine.context.services.task_repo {
                    let task_id_typed = TaskId::from_string(task_id.clone());

                    // Skip guard: check if metadata was already pre-computed (e.g., by transition_task_with_metadata)
                    match task_repo.get_by_id(&task_id_typed).await {
                        Ok(Some(task)) => {
                            // Read auto_retry_count_executing from task metadata for observability
                            let attempt_count = task
                                .metadata
                                .as_deref()
                                .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
                                .and_then(|v| {
                                    v.get("auto_retry_count_executing")
                                        .and_then(|c| c.as_u64())
                                })
                                .unwrap_or(0) as u32;

                            if MetadataUpdate::key_exists_in(
                                "failure_error",
                                task.metadata.as_deref(),
                            ) {
                                tracing::debug!(
                                    task_id = task_id,
                                    attempt_count = attempt_count,
                                    "failure_error already present (pre-computed); writing attempt_count only"
                                );
                                // Write attempt_count even when other failure metadata was pre-computed
                                let metadata_update =
                                    MetadataUpdate::new().with_u32("attempt_count", attempt_count);
                                let merged_metadata =
                                    metadata_update.merge_into(task.metadata.as_deref());
                                if let Err(e) = task_repo
                                    .update_metadata(&task_id_typed, Some(merged_metadata))
                                    .await
                                {
                                    tracing::error!(
                                        task_id = task_id,
                                        error = %e,
                                        "Failed to write attempt_count to failure metadata"
                                    );
                                }
                            } else {
                                // Fallback: metadata not pre-computed, write it now for backward compatibility
                                let enriched_data =
                                    data.clone().with_attempt_count(attempt_count);
                                let metadata_update = build_failed_metadata(&enriched_data);
                                let merged_metadata =
                                    metadata_update.merge_into(task.metadata.as_deref());

                                if let Err(e) = task_repo
                                    .update_metadata(&task_id_typed, Some(merged_metadata))
                                    .await
                                {
                                    tracing::error!(
                                        task_id = task_id,
                                        error = %e,
                                        "Failed to update task with failure metadata"
                                    );
                                }
                            }
                        }
                        Ok(None) => {
                            tracing::error!(
                                task_id = task_id,
                                "Task not found when storing failure metadata"
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                task_id = task_id,
                                error = %e,
                                "Error retrieving task for failure metadata"
                            );
                        }
                    }
                }

                // Fail any in-progress steps (Bug 2: agent was terminated, won't call fail_step)
                if let Some(ref step_repo) = self.machine.context.services.step_repo {
                    let task_id_typed = TaskId::from_string(task_id.clone());
                    match step_repo.get_by_task(&task_id_typed).await {
                        Ok(steps) => {
                            for step in steps
                                .iter()
                                .filter(|s| s.status == TaskStepStatus::InProgress)
                            {
                                let mut failed_step = step.clone();
                                failed_step.status = TaskStepStatus::Failed;
                                failed_step.completion_note =
                                    Some("Task execution failed".to_string());
                                failed_step.completed_at = Some(Utc::now());

                                if let Err(e) = step_repo.update(&failed_step).await {
                                    tracing::error!(
                                        task_id = task_id,
                                        step_id = %step.id,
                                        error = %e,
                                        "Failed to update in-progress step to failed status"
                                    );
                                } else {
                                    // Emit step updated event
                                    self.machine
                                        .context
                                        .services
                                        .event_emitter
                                        .emit("step:updated", &format!("{}", step.id))
                                        .await;
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                task_id = task_id,
                                error = %e,
                                "Failed to retrieve steps for failure handling"
                            );
                        }
                    }
                }

                // Emit task failed event
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("task_failed", task_id)
                    .await;
            }
            State::PendingMerge => {
                // Phase 1 of merge workflow: Attempt programmatic rebase and merge
                // This is the "fast path" - if successful, skip agent entirely
                self.attempt_programmatic_merge().await;
            }
            State::Merging => {
                // Phase 2 of merge workflow: Spawn merger agent for conflict resolution
                // This state is reached when Phase 1 (programmatic merge) failed due to conflicts,
                // OR when AutoFix validation mode detected validation failures (Phase 113)
                let task_id = &self.machine.context.task_id;

                // Clean up merge worktree before spawning merger agent.
                // - Symlink removal: ALWAYS (symlinks cause false conflicts for the agent)
                // - Git abort: only on recovery re-entry (stale rebase/merge from prior attempt)
                if let (Some(ref _task_repo), Some(ref project_repo)) = (
                    &self.machine.context.services.task_repo,
                    &self.machine.context.services.project_repo,
                ) {
                    let project_id =
                        ProjectId::from_string(self.machine.context.project_id.clone());
                    if let Ok(Some(project)) = project_repo.get_by_id(&project_id).await {
                        let wt_path = std::path::PathBuf::from(compute_merge_worktree_path(
                            &project, task_id,
                        ));
                        if wt_path.exists() {
                            // Abort stale rebase/merge from prior attempt (recovery or retry)
                            if GitService::is_rebase_in_progress(&wt_path) {
                                tracing::info!(
                                    task_id = task_id,
                                    "on_enter(Merging): Aborting stale rebase before agent spawn"
                                );
                                let _ = GitService::abort_rebase(&wt_path).await;
                            }
                            if GitService::is_merge_in_progress(&wt_path) {
                                tracing::info!(
                                    task_id = task_id,
                                    "on_enter(Merging): Aborting stale merge before agent spawn"
                                );
                                let _ = GitService::abort_merge(&wt_path).await;
                            }

                            // Always: remove worktree symlinks that cause false conflicts.
                            // The merger agent's validation step re-creates them via worktree_setup.
                            for rel in &[
                                "node_modules",
                                "src-tauri/target",
                                "ralphx-plugin/ralphx-mcp-server/node_modules",
                            ] {
                                let sym = wt_path.join(rel);
                                if sym.is_symlink() {
                                    tracing::info!(
                                        task_id = task_id,
                                        path = %sym.display(),
                                        "on_enter(Merging): Removing worktree symlink"
                                    );
                                    let _ = std::fs::remove_file(&sym);
                                }
                            }
                        }
                    }
                }

                // Check task metadata for validation_recovery flag (Phase 113: AutoFix mode)
                let is_validation_recovery =
                    if let Some(ref task_repo) = self.machine.context.services.task_repo {
                        let tid = TaskId::from_string(task_id.clone());
                        if let Ok(Some(task)) = task_repo.get_by_id(&tid).await {
                            task.metadata
                                .as_ref()
                                .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
                                .and_then(|v| v.get("validation_recovery")?.as_bool())
                                .unwrap_or(false)
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                let prompt = if is_validation_recovery {
                    format!(
                        "Fix validation failures for task: {}. The merge succeeded but post-merge \
                         validation commands failed. The failing code is on the target branch. \
                         Read the validation failures from task context, fix the code, run validation \
                         to confirm, then commit your fixes.",
                        task_id
                    )
                } else {
                    format!("Resolve merge conflicts for task: {}", task_id)
                };

                tracing::info!(
                    task_id = task_id,
                    is_validation_recovery = is_validation_recovery,
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

                // Schedule newly-unblocked tasks (e.g. plan_merge tasks that just became Ready)
                // Internal transition — no UI settle needed → merge_settle_ms
                if let Some(ref scheduler) = self.machine.context.services.task_scheduler {
                    let scheduler = Arc::clone(scheduler);
                    let merge_settle_ms = scheduler_config().merge_settle_ms;
                    tokio::spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_millis(merge_settle_ms))
                            .await;
                        scheduler.try_schedule_ready_tasks().await;
                    });
                } else {
                    tracing::warn!(
                        task_id = self.machine.context.task_id.as_str(),
                        "task_scheduler not wired — Ready tasks will not be auto-scheduled after Merged"
                    );
                }

                // Retry deferred merges — covers the HTTP handler path (e.g. ConflictResolved)
                // where on_enter(Merged) is called directly without going through
                // post_merge_cleanup(). No sleep needed: scheduling_lock mutex in
                // task_scheduler_service.rs serializes concurrent calls via try_lock(), and
                // has_merge_deferred_metadata is the actual safety guard.
                if let Some(ref scheduler) = self.machine.context.services.task_scheduler {
                    let scheduler = Arc::clone(scheduler);
                    let project_id = self.machine.context.project_id.clone();
                    tokio::spawn(async move {
                        scheduler.try_retry_deferred_merges(&project_id).await;
                    });
                }
            }
            _ => {}
        }
        Ok(())
    }

}

/// Extract `restart_note` from task metadata JSON.
/// Returns `Some(note)` if the key exists and is a non-empty string, `None` otherwise.
fn extract_restart_note(metadata: Option<&str>) -> Option<String> {
    let metadata_str = metadata?;
    let obj = serde_json::from_str::<serde_json::Value>(metadata_str).ok()?;
    let note = obj.get("restart_note")?.as_str()?;
    if note.is_empty() {
        None
    } else {
        Some(note.to_string())
    }
}
