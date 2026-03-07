// Execution, review, QA, and paused-task reconciliation handlers,
// plus orchestration (reconcile_task, reconcile_stuck_tasks) and apply_recovery_decision.

use std::sync::Arc;

use tauri::{Emitter, Runtime};
use tracing::{info, warn};

use crate::application::chat_service::{MergeAutoCompleteContext, reconcile_merge_auto_complete};
use crate::application::interactive_process_registry::InteractiveProcessKey;
use crate::application::GitService;
use crate::domain::entities::{
    ActivityEvent, ActivityEventType, AgentRunStatus, ChatContextType, ExecutionFailureSource,
    ExecutionRecoveryEvent, ExecutionRecoveryEventKind, ExecutionRecoveryMetadata,
    ExecutionRecoveryReasonCode, ExecutionRecoverySource, ExecutionRecoveryState, InternalStatus,
    ReviewNote, ReviewOutcome, ReviewerType, TaskId,
};
use crate::domain::services::RunningAgentKey;
use crate::domain::state_machine::transition_handler::set_trigger_origin;
use crate::infrastructure::agents::claude::reconciliation_config;

use super::super::policy::{
    RecoveryActionKind, RecoveryContext, RecoveryDecision, RecoveryEvidence, UserRecoveryAction,
};
use super::super::ReconciliationRunner;

impl<R: Runtime> ReconciliationRunner<R> {
    /// Check if an IPR entry for a context is backed by a live process.
    ///
    /// The IPR (InteractiveProcessRegistry) stores `ChildStdin` handles keyed by
    /// (context_type, context_id). An entry existing does NOT mean the process is alive —
    /// the cleanup in `spawn_send_message_background` can be skipped (team mode, panic,
    /// cancellation), leaving a stale entry that blocks reconciliation forever.
    ///
    /// This method cross-references the IPR entry against the running_agent_registry PID:
    /// - If IPR has no entry → returns false (no interactive process)
    /// - If IPR has entry AND registry has entry with alive PID → returns true
    /// - If IPR has entry BUT no registry entry or PID is dead → removes stale IPR entry, returns false
    pub(crate) async fn is_ipr_process_alive(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> bool {
        let ipr = match self.interactive_process_registry {
            Some(ref ipr) => ipr,
            None => return false,
        };

        let ipr_key = InteractiveProcessKey::new(context_type.to_string(), context_id);
        if !ipr.has_process(&ipr_key).await {
            return false;
        }

        // IPR has an entry — verify the underlying process is still alive
        let registry_key = RunningAgentKey::new(context_type.to_string(), context_id);
        let pid_alive = match self.running_agent_registry.get(&registry_key).await {
            Some(info) => crate::domain::services::is_process_alive(info.pid),
            None => false, // Registry cleaned but IPR wasn't → stale
        };

        if !pid_alive {
            warn!(
                context_type = %context_type,
                context_id,
                "Removing stale IPR entry — process no longer alive"
            );
            ipr.remove(&ipr_key).await;
            return false;
        }

        true
    }

    /// Startup-only recovery: re-queue Failed tasks that failed due to transient timeouts.
    ///
    /// Runs once at app startup (not in the recurring loop). Handles two formats (GAP M2):
    /// - **Legacy**: `is_timeout: true` in flat metadata (old format)
    /// - **New**: `execution_recovery` present with `last_state == Retrying`
    ///
    /// Legacy tasks are migrated to the new `ExecutionRecoveryMetadata` format during recovery.
    /// All startup-processed tasks get an `AutoRetryTriggered` event with `Startup` source,
    /// which acts as a sentinel to prevent the reconciler loop from double-processing
    /// the same task within 60 seconds (GAP M5).
    pub async fn recover_timeout_failures(&self) {
        let projects = match self.project_repo.get_all().await {
            Ok(projects) => projects,
            Err(e) => {
                warn!(error = %e, "recover_timeout_failures: failed to get projects");
                return;
            }
        };

        let max_retries = crate::infrastructure::agents::claude::reconciliation_config()
            .execution_failed_max_retries as u32;

        for project in &projects {
            let failed_tasks = match self
                .task_repo
                .get_by_status(&project.id, InternalStatus::Failed)
                .await
            {
                Ok(tasks) => tasks,
                Err(e) => {
                    warn!(
                        project_id = project.id.as_str(),
                        error = %e,
                        "recover_timeout_failures: failed to get Failed tasks"
                    );
                    continue;
                }
            };

            for task in failed_tasks {
                // Determine eligibility using BOTH legacy and new metadata formats (GAP M2).
                let is_legacy_timeout = self.task_is_timeout_failure(&task);
                let new_recovery =
                    ExecutionRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
                        .ok()
                        .flatten();

                let is_new_retrying = new_recovery
                    .as_ref()
                    .map(|r| r.last_state == ExecutionRecoveryState::Retrying && !r.stop_retrying)
                    .unwrap_or(false);

                if !is_legacy_timeout && !is_new_retrying {
                    continue;
                }

                // Determine current attempt count from whichever format is active.
                // Legacy uses the flat counter; new uses event log.
                let attempt_count = if is_new_retrying {
                    Self::execution_failed_auto_retry_count(&task)
                } else {
                    Self::auto_retry_count_for_status(&task, InternalStatus::Executing)
                };

                if attempt_count >= max_retries {
                    tracing::debug!(
                        task_id = task.id.as_str(),
                        attempt_count = attempt_count,
                        max_retries = max_retries,
                        "Startup recovery: skipping task — max retries reached"
                    );
                    continue;
                }

                info!(
                    task_id = task.id.as_str(),
                    attempt_count = attempt_count,
                    is_legacy = is_legacy_timeout,
                    "Startup recovery: re-queuing timeout-failed task"
                );

                // Record structured AutoRetryTriggered event with Startup source (GAP M5 sentinel).
                // For legacy tasks, this also creates the initial ExecutionRecoveryMetadata
                // with a Failed event, migrating them to the new format (GAP M2).
                if let Err(e) = self
                    .record_execution_startup_retry_event(&task, attempt_count + 1)
                    .await
                {
                    warn!(
                        task_id = task.id.as_str(),
                        error = %e,
                        "Startup recovery: failed to record startup retry event (proceeding anyway)"
                    );
                }

                match self
                    .transition_service
                    .transition_task(&task.id, InternalStatus::Ready)
                    .await
                {
                    Ok(_) => {
                        info!(
                            task_id = task.id.as_str(),
                            "Startup recovery: task transitioned Failed -> Ready"
                        );
                    }
                    Err(e) => {
                        warn!(
                            task_id = task.id.as_str(),
                            error = %e,
                            "Startup recovery: failed to transition task to Ready"
                        );
                    }
                }
            }
        }
    }

    /// Returns true if the task's metadata indicates it failed due to a timeout.
    fn task_is_timeout_failure(&self, task: &crate::domain::entities::Task) -> bool {
        let metadata = match task.metadata.as_deref() {
            Some(m) => m,
            None => return false,
        };
        let json: serde_json::Value = match serde_json::from_str(metadata) {
            Ok(v) => v,
            Err(_) => return false,
        };
        json.get("is_timeout")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    pub async fn reconcile_stuck_tasks(&self) {
        self.prune_stale_running_registry_entries().await;

        if self.execution_state.is_paused() {
            return;
        }

        let projects = match self.project_repo.get_all().await {
            Ok(projects) => projects,
            Err(e) => {
                warn!(error = %e, "Failed to get projects for reconciliation");
                return;
            }
        };

        for project in &projects {
            for status in [
                InternalStatus::Executing,
                InternalStatus::ReExecuting,
                InternalStatus::Reviewing,
                InternalStatus::Merging,
                InternalStatus::PendingMerge,
                InternalStatus::MergeIncomplete,
                InternalStatus::MergeConflict,
                InternalStatus::QaRefining,
                InternalStatus::QaTesting,
                InternalStatus::Paused,
                InternalStatus::Failed, // GAP B2: auto-retry eligible Failed executions
            ] {
                let tasks = match self.task_repo.get_by_status(&project.id, status).await {
                    Ok(tasks) => tasks,
                    Err(e) => {
                        warn!(
                            project_id = project.id.as_str(),
                            status = ?status,
                            error = %e,
                            "Failed to get tasks by status for reconciliation"
                        );
                        continue;
                    }
                };

                for task in tasks {
                    let _ = self.reconcile_task(&task, status).await;
                }
            }
        }
    }

    pub(crate) async fn prune_stale_running_registry_entries(&self) {
        let entries = self.running_agent_registry.list_all().await;
        if entries.is_empty() {
            self.execution_state.set_running_count(0);
            return;
        }

        let engine = crate::application::PruneEngine::new(
            Arc::clone(&self.running_agent_registry),
            Arc::clone(&self.agent_run_repo),
            Arc::clone(&self.task_repo),
            self.interactive_process_registry.clone(),
        );

        let mut removed = 0u32;

        for (key, info) in &entries {
            // Skip in-flight registrations: try_register inserts pid=0/empty agent_run_id as
            // placeholder; update_agent_process fills real values ~40ms later.
            if info.agent_run_id.is_empty() {
                tracing::debug!(
                    context_type = key.context_type,
                    context_id = key.context_id,
                    "Skipping in-flight registry entry (no agent_run_id yet)"
                );
                continue;
            }

            // Age guard: pid=0 entries younger than 30s are in the try_register →
            // update_agent_process window. The pruner must not race against the spawn.
            if info.pid == 0 {
                let age = chrono::Utc::now() - info.started_at;
                if age < chrono::Duration::seconds(30) {
                    tracing::debug!(
                        context_type = key.context_type,
                        context_id = key.context_id,
                        age_secs = age.num_seconds(),
                        "Skipping young pid=0 registry entry (age < 30s)"
                    );
                    continue;
                }
            }

            // Compute pid liveness once; both the IPR check and staleness evaluation use it.
            let pid_alive = crate::domain::services::is_process_alive(info.pid);

            // PID-verified IPR check: skip if interactive process is alive; remove stale
            // IPR entries (PID dead) so reconciliation is not blocked forever.
            if engine.check_ipr_skip(key, pid_alive).await {
                continue;
            }

            if engine.evaluate_and_prune(key, info, pid_alive).await {
                removed += 1;
            }
        }

        // Always recalculate running count from remaining entries.
        // Subtract idle interactive processes that already freed their execution slot
        // via TurnComplete but still have a registry entry (process alive, waiting for stdin).
        let remaining_entries = self.running_agent_registry.list_all().await;
        let registry_count = remaining_entries.len() as u32;
        let idle_count = remaining_entries
            .iter()
            .filter(|(key, _)| {
                let slot_key = format!("{}/{}", key.context_type, key.context_id);
                self.execution_state.is_interactive_idle(&slot_key)
            })
            .count() as u32;
        self.execution_state
            .set_running_count(registry_count.saturating_sub(idle_count));
        if removed > 0 {
            if let Some(handle) = self.app_handle.as_ref() {
                self.execution_state
                    .emit_status_changed(handle, "runtime_registry_gc");
            }
        }
    }

    pub async fn reconcile_task(
        &self,
        task: &crate::domain::entities::Task,
        status: InternalStatus,
    ) -> bool {
        match status {
            InternalStatus::Executing | InternalStatus::ReExecuting => {
                self.reconcile_completed_execution(task, status).await
            }
            InternalStatus::Reviewing => self.reconcile_reviewing_task(task, status).await,
            InternalStatus::Merging => self.reconcile_merging_task(task, status).await,
            InternalStatus::PendingMerge => self.reconcile_pending_merge_task(task, status).await,
            InternalStatus::MergeIncomplete => {
                self.reconcile_merge_incomplete_task(task, status).await
            }
            InternalStatus::MergeConflict => self.reconcile_merge_conflict_task(task, status).await,
            InternalStatus::QaRefining | InternalStatus::QaTesting => {
                self.reconcile_qa_task(task, status).await
            }
            InternalStatus::Paused => self.reconcile_paused_provider_error(task).await,
            // GAP B3: Auto-retry Failed executions via reconcile_failed_execution_task
            InternalStatus::Failed => self.reconcile_failed_execution_task(task, status).await,
            _ => false,
        }
    }

    pub(crate) async fn reconcile_completed_execution(
        &self,
        task: &crate::domain::entities::Task,
        status: InternalStatus,
    ) -> bool {
        if status != InternalStatus::Executing && status != InternalStatus::ReExecuting {
            return false;
        }

        // Skip if there's a live interactive process — the agent is alive between turns.
        // Cross-references IPR against registry PID to detect stale entries.
        if self
            .is_ipr_process_alive(ChatContextType::TaskExecution, task.id.as_str())
            .await
        {
            tracing::debug!(
                task_id = task.id.as_str(),
                "Skipping execution reconciliation: interactive process active"
            );
            return true;
        }

        let run = self.load_execution_run(task, status).await;
        let evidence = self
            .build_run_evidence(task, ChatContextType::TaskExecution, run.as_ref())
            .await;
        if evidence.run_status == Some(AgentRunStatus::Running) && evidence.registry_running {
            return true;
        }

        // C5: Wall-clock timeout for long-running executions
        if let Some(age) = self.latest_status_transition_age(task, status).await {
            let max_minutes = reconciliation_config().executing_max_wall_clock_minutes as i64;
            if age >= chrono::Duration::minutes(max_minutes) {
                warn!(
                    task_id = task.id.as_str(),
                    age_minutes = age.num_minutes(),
                    max_minutes = max_minutes,
                    "Execution wall-clock timeout exceeded"
                );
                // GAP H1: Tag with WallClockTimeout so reconciler never retries these.
                // Wall-clock failures are hard limits — retrying causes infinite C5→Failed→retry→C5 loop.
                let mut recovery =
                    ExecutionRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
                        .unwrap_or(None)
                        .unwrap_or_default();
                let wc_event = ExecutionRecoveryEvent::new(
                    ExecutionRecoveryEventKind::Failed,
                    ExecutionRecoverySource::System,
                    ExecutionRecoveryReasonCode::WallClockExceeded,
                    format!(
                        "Wall-clock timeout exceeded ({} minutes)",
                        age.num_minutes()
                    ),
                )
                .with_failure_source(ExecutionFailureSource::WallClockTimeout);
                recovery.append_event_with_state(wc_event, ExecutionRecoveryState::Failed);
                recovery.stop_retrying = true;
                if let Ok(updated_metadata) =
                    recovery.update_task_metadata(task.metadata.as_deref())
                {
                    if let Err(e) = self
                        .task_repo
                        .update_metadata(&task.id, Some(updated_metadata))
                        .await
                    {
                        warn!(
                            task_id = task.id.as_str(),
                            error = %e,
                            "Failed to write wall-clock timeout metadata (H1)"
                        );
                    }
                }
                return self
                    .apply_recovery_decision(
                        task,
                        status,
                        RecoveryContext::Execution,
                        RecoveryDecision {
                            action: RecoveryActionKind::Transition(InternalStatus::Failed),
                            reason: Some(format!(
                                "Execution timed out after {} minutes (wall-clock limit: {}m)",
                                age.num_minutes(),
                                max_minutes
                            )),
                        },
                    )
                    .await;
            }
        }

        let decision = self
            .policy
            .decide_reconciliation(RecoveryContext::Execution, evidence);

        // Auto-recover execution conflicts instead of prompting the user.
        // When DB run state disagrees with registry (has_conflict), the agent likely died
        // silently. Instead of bothering the user, auto-restart within the retry budget.
        let decision = if decision.action == RecoveryActionKind::Prompt {
            // Grace period: if the agent run was created < 30s ago, the PID may not
            // have been registered yet — skip this cycle and let registration catch up.
            let within_grace_period = run.as_ref().map_or(false, |r| {
                let age = chrono::Utc::now() - r.started_at;
                age < chrono::Duration::seconds(30)
            });

            if within_grace_period {
                tracing::debug!(
                    task_id = task.id.as_str(),
                    "Execution conflict detection within 30s grace period — skipping"
                );
                return false;
            }

            warn!(
                task_id = task.id.as_str(),
                "Auto-recovering execution conflict: restarting agent (run state vs registry mismatch)"
            );
            RecoveryDecision {
                action: RecoveryActionKind::ExecuteEntryActions,
                reason: Some(
                    "Auto-recovering execution run state conflict — restarting agent.".to_string(),
                ),
            }
        } else {
            decision
        };

        // E7: Enforce retry limit for execution re-spawns
        if decision.action == RecoveryActionKind::ExecuteEntryActions {
            let retry_count = Self::auto_retry_count_for_status(task, status);
            if retry_count >= reconciliation_config().executing_max_retries as u32 {
                warn!(
                    task_id = task.id.as_str(),
                    retry_count = retry_count,
                    max = reconciliation_config().executing_max_retries,
                    "Execution retry limit reached — escalating to Failed"
                );
                return self
                    .apply_recovery_decision(
                        task,
                        status,
                        RecoveryContext::Execution,
                        RecoveryDecision {
                            action: RecoveryActionKind::Transition(InternalStatus::Failed),
                            reason: Some(format!(
                                "Execution failed {} times — escalating to Failed",
                                retry_count
                            )),
                        },
                    )
                    .await;
            }
            // Record the retry attempt
            if let Err(e) = self
                .record_auto_retry_metadata(task, status, retry_count + 1)
                .await
            {
                warn!(task_id = task.id.as_str(), error = %e, "Failed to record execution retry metadata");
            }
        }

        self.apply_recovery_decision(task, status, RecoveryContext::Execution, decision)
            .await
    }

    /// Core handler for auto-retrying Failed execution tasks (Wave 3 — GAPs B2, B3, B5, B6, B7,
    /// B8, H1, H8, H10, M4, M6, M9).
    ///
    /// Decision tree:
    /// 1. Re-fetch from DB (M4: staleness guard)
    /// 2. Skip if no execution_recovery metadata (legacy tasks — not opted in)
    /// 3. Skip if stop_retrying (user or system halted retries)
    /// 4. Skip if last_state == Failed (permanent failure)
    /// 5. Skip if last failure was WallClockTimeout (H1: C5 hard limit — would loop)
    /// 6. Skip if retry_count >= max_retries (record permanent failure)
    /// 7. Skip if backoff not elapsed (M6, M8: computed from last retry event timestamp)
    /// 8. Skip if can_start_task() == false (B6: concurrency guard)
    /// 9. Full git cleanup: delete worktree → delete branch → clear DB fields (B5, B8, H6, H8, M9)
    /// 10. Clear stale flat metadata (B7)
    /// 11. Emit ActivityEvent for visibility (H10)
    /// 12. Record AutoRetryTriggered event in recovery metadata
    /// 13. Dispatch TaskEvent::Retry → Failed → Ready
    pub(crate) async fn reconcile_failed_execution_task(
        &self,
        task: &crate::domain::entities::Task,
        _status: InternalStatus,
    ) -> bool {
        // GAP M4: Re-fetch task from DB — reconciler query can be up to 30s stale
        let task = match self.task_repo.get_by_id(&task.id).await {
            Ok(Some(t)) => t,
            Ok(None) => {
                warn!(
                    task_id = task.id.as_str(),
                    "Task not found in DB during failed execution reconciliation"
                );
                return false;
            }
            Err(e) => {
                warn!(
                    task_id = task.id.as_str(),
                    error = %e,
                    "Failed to re-fetch task for failed execution reconciliation"
                );
                return false;
            }
        };

        // Skip if not actually Failed (could have been transitioned by another path)
        if task.internal_status != InternalStatus::Failed {
            return false;
        }

        // Parse execution recovery metadata — skip if absent (legacy tasks, not opted in)
        let recovery =
            match ExecutionRecoveryMetadata::from_task_metadata(task.metadata.as_deref()) {
                Ok(Some(r)) => r,
                Ok(None) => {
                    tracing::debug!(
                        task_id = task.id.as_str(),
                        "Skipping failed execution reconciliation: no execution_recovery metadata (legacy)"
                    );
                    return false;
                }
                Err(e) => {
                    warn!(
                        task_id = task.id.as_str(),
                        error = %e,
                        "Failed to parse execution recovery metadata"
                    );
                    return false;
                }
            };

        // Skip if user or system halted retries
        if recovery.stop_retrying {
            tracing::debug!(
                task_id = task.id.as_str(),
                "Skipping failed execution reconciliation: stop_retrying=true"
            );
            return false;
        }

        // GAP M5: Skip if startup recovery handled this task within the last 60s —
        // prevents double-processing race between recover_timeout_failures() and the loop.
        if Self::has_recent_startup_recovery(&task) {
            tracing::debug!(
                task_id = task.id.as_str(),
                "Skipping failed execution reconciliation: startup recovery sentinel active (< 60s)"
            );
            return false;
        }

        // Skip if recovery has reached permanent failure state
        if recovery.last_state == ExecutionRecoveryState::Failed {
            tracing::debug!(
                task_id = task.id.as_str(),
                "Skipping failed execution reconciliation: last_state=Failed (permanent)"
            );
            return false;
        }

        // GAP H1: Skip if last failure was wall-clock timeout — retrying causes infinite C5 loop
        let last_is_wall_clock = recovery
            .events
            .last()
            .and_then(|e| e.failure_source.as_ref())
            .map(|s| matches!(s, ExecutionFailureSource::WallClockTimeout))
            .unwrap_or(false);
        if last_is_wall_clock {
            tracing::debug!(
                task_id = task.id.as_str(),
                "Skipping failed execution reconciliation: wall-clock timeout (C5) — non-retryable"
            );
            return false;
        }

        // Check max retries — record permanent failure if budget exhausted
        let retry_count = Self::execution_failed_auto_retry_count(&task);
        let max_retries = reconciliation_config().execution_failed_max_retries as u32;
        if retry_count >= max_retries {
            warn!(
                task_id = task.id.as_str(),
                retry_count = retry_count,
                max_retries = max_retries,
                "Failed execution max retries exceeded — marking permanent failure"
            );
            if let Err(e) = self.set_execution_stop_retrying(&task).await {
                warn!(
                    task_id = task.id.as_str(),
                    error = %e,
                    "Failed to set stop_retrying flag after max retries"
                );
            }
            return false;
        }

        // GAPs M6, M8: Check backoff elapsed — computed dynamically from last retry event
        if let Some(next_retry_at) = Self::execution_next_retry_at(&task) {
            if chrono::Utc::now() < next_retry_at {
                tracing::debug!(
                    task_id = task.id.as_str(),
                    next_retry_at = %next_retry_at,
                    "Skipping failed execution reconciliation: backoff not elapsed"
                );
                return false;
            }
        }

        // GAP B6: Concurrency guard — skip if at max_concurrent capacity
        if !self.execution_state.can_start_task() {
            tracing::debug!(
                task_id = task.id.as_str(),
                "Skipping failed execution reconciliation: at max concurrency — will try next cycle"
            );
            return false;
        }

        // ── Git cleanup (GAPs B5, B8, H6, H8, M9) ───────────────────────────────────────────────
        let project_data = match self.project_repo.get_by_id(&task.project_id).await {
            Ok(Some(p)) => p,
            Ok(None) => {
                warn!(
                    task_id = task.id.as_str(),
                    "Project not found for task — skipping git cleanup"
                );
                return false;
            }
            Err(e) => {
                warn!(
                    task_id = task.id.as_str(),
                    error = %e,
                    "Failed to get project for git cleanup during execution recovery"
                );
                return false;
            }
        };
        let repo_path = std::path::Path::new(&project_data.working_directory);

        // (1) Delete worktree FIRST — must happen before branch deletion (GAP H8)
        //     git refuses to delete a branch checked out in a worktree.
        if let Some(worktree_path_str) = task.worktree_path.as_deref() {
            let worktree_path = std::path::Path::new(worktree_path_str);
            if worktree_path.exists() {
                if let Err(e) = GitService::delete_worktree(repo_path, worktree_path).await {
                    warn!(
                        task_id = task.id.as_str(),
                        worktree = worktree_path_str,
                        error = %e,
                        "Failed to delete worktree during execution recovery — continuing"
                    );
                }
            }
        }

        // (2) Delete branch — log warn and continue if it fails (GAP M9)
        //     on_enter_executing() handles branch_exists=true as fallback
        if let Some(branch) = task.task_branch.as_deref() {
            if let Err(e) = GitService::delete_branch(repo_path, branch, true).await {
                warn!(
                    task_id = task.id.as_str(),
                    branch = branch,
                    error = %e,
                    "Failed to delete task branch during execution recovery — on_enter_executing handles branch_exists (M9)"
                );
                // Non-fatal: on_enter_executing will check out existing branch as fallback
            }
        }

        // (3) Clear task_branch + worktree_path in DB so on_enter_executing gets a fresh start
        let mut updated_task = task.clone();
        updated_task.task_branch = None;
        updated_task.worktree_path = None;
        updated_task.touch();
        if let Err(e) = self.task_repo.update(&updated_task).await {
            warn!(
                task_id = task.id.as_str(),
                error = %e,
                "Failed to clear task_branch/worktree_path in DB during execution recovery"
            );
            // Continue — transition will still work, on_enter_executing recomputes branch name
        }

        // (4) GAP B7: Clear stale flat metadata (is_timeout, failure_error) to prevent
        //     misclassification on subsequent attempts
        if let Err(e) = self.clear_execution_flat_metadata(&task).await {
            warn!(
                task_id = task.id.as_str(),
                error = %e,
                "Failed to clear execution flat metadata (B7)"
            );
        }

        // Re-fetch task after metadata cleanup so record_execution_auto_retry_event uses the
        // clean DB state as base. Without this, both clear_execution_flat_metadata and
        // record_execution_auto_retry_event use the same stale in-memory snapshot — the latter
        // call re-introduces is_timeout/failure_error by merging from the stale snapshot,
        // causing task_is_timeout_failure() to misclassify subsequent attempts.
        let task = match self.task_repo.get_by_id(&task.id).await {
            Ok(Some(t)) => t,
            Ok(None) => {
                warn!(
                    task_id = task.id.as_str(),
                    "Task not found after metadata cleanup — skipping auto-retry"
                );
                return false;
            }
            Err(e) => {
                warn!(
                    task_id = task.id.as_str(),
                    error = %e,
                    "Failed to re-fetch task after metadata cleanup — using stale snapshot"
                );
                task
            }
        };

        // (5) GAP H10: Emit ActivityEvent so auto-retry is visible in task feed
        let attempt_num = retry_count + 1;
        let failure_reason = recovery
            .events
            .last()
            .map(|e| format!("{:?}", e.reason_code))
            .unwrap_or_else(|| "Unknown".to_string());
        let activity_message = format!(
            "Auto-retrying execution (attempt {}/{}) — previous failure: {}",
            attempt_num, max_retries, failure_reason
        );
        let activity_event = ActivityEvent::new_task_event(
            task.id.clone(),
            ActivityEventType::Text,
            &activity_message,
        );
        if let Err(e) = self.activity_event_repo.save(activity_event).await {
            warn!(
                task_id = task.id.as_str(),
                error = %e,
                "Failed to emit auto-retry activity event (H10)"
            );
        }

        // (6) Record AutoRetryTriggered in execution recovery metadata (uses update_metadata() path — H7)
        let failure_source = recovery
            .events
            .last()
            .and_then(|e| e.failure_source)
            .unwrap_or(ExecutionFailureSource::Unknown);
        if let Err(e) = self
            .record_execution_auto_retry_event(&task, attempt_num, failure_source, &activity_message)
            .await
        {
            warn!(
                task_id = task.id.as_str(),
                error = %e,
                "Failed to record auto-retry event in execution recovery metadata"
            );
        }

        // (7) Dispatch TaskEvent::Retry → Failed → Ready
        info!(
            task_id = task.id.as_str(),
            attempt = attempt_num,
            max_retries = max_retries,
            "Auto-retrying failed execution task"
        );
        if let Err(e) = self
            .transition_service
            .transition_task(&task.id, InternalStatus::Ready)
            .await
        {
            tracing::error!(
                task_id = task.id.as_str(),
                error = %e,
                "Failed to transition task from Failed to Ready during auto-retry"
            );
            return false;
        }

        true
    }

    pub(crate) async fn reconcile_reviewing_task(
        &self,
        task: &crate::domain::entities::Task,
        status: InternalStatus,
    ) -> bool {
        if status != InternalStatus::Reviewing {
            return false;
        }

        // Skip if there's a live interactive process — the agent is alive between turns.
        // Cross-references IPR against registry PID to detect stale entries.
        if self
            .is_ipr_process_alive(ChatContextType::Review, task.id.as_str())
            .await
        {
            tracing::debug!(
                task_id = task.id.as_str(),
                "Skipping review reconciliation: interactive process active"
            );
            return true;
        }

        let run = self
            .lookup_latest_run_for_task_context(task, ChatContextType::Review)
            .await;
        let evidence = self
            .build_run_evidence(task, ChatContextType::Review, run.as_ref())
            .await;
        if evidence.run_status == Some(AgentRunStatus::Running) && evidence.registry_running {
            return true;
        }

        // C5: Wall-clock timeout for long-running reviews
        if let Some(age) = self.latest_status_transition_age(task, status).await {
            let max_minutes = reconciliation_config().reviewing_max_wall_clock_minutes as i64;
            if age >= chrono::Duration::minutes(max_minutes) {
                warn!(
                    task_id = task.id.as_str(),
                    age_minutes = age.num_minutes(),
                    max_minutes = max_minutes,
                    "Review wall-clock timeout exceeded"
                );
                return self
                    .apply_recovery_decision(
                        task,
                        status,
                        RecoveryContext::Review,
                        RecoveryDecision {
                            action: RecoveryActionKind::Transition(InternalStatus::Escalated),
                            reason: Some(format!(
                                "Review timed out after {} minutes (wall-clock limit: {}m)",
                                age.num_minutes(),
                                max_minutes
                            )),
                        },
                    )
                    .await;
            }
        }

        let decision = self
            .policy
            .decide_reconciliation(RecoveryContext::Review, evidence);

        // Auto-recover review conflicts instead of prompting the user.
        // When DB run state disagrees with registry (has_conflict), the agent likely died
        // silently. Instead of bothering the user, auto-restart within the retry budget.
        let decision = if decision.action == RecoveryActionKind::Prompt {
            // Grace period: if the agent run was created < 30s ago, the PID may not
            // have been registered yet — skip this cycle and let registration catch up.
            let within_grace_period = run.as_ref().map_or(false, |r| {
                let age = chrono::Utc::now() - r.started_at;
                age < chrono::Duration::seconds(30)
            });

            if within_grace_period {
                tracing::debug!(
                    task_id = task.id.as_str(),
                    "Review conflict detection within 30s grace period — skipping"
                );
                return false;
            }

            warn!(
                task_id = task.id.as_str(),
                "Auto-recovering review conflict: restarting agent (run state vs registry mismatch)"
            );
            RecoveryDecision {
                action: RecoveryActionKind::ExecuteEntryActions,
                reason: Some(
                    "Auto-recovering review run state conflict — restarting agent.".to_string(),
                ),
            }
        } else {
            decision
        };

        // E7: Enforce retry limit for review re-spawns
        if decision.action == RecoveryActionKind::ExecuteEntryActions {
            let retry_count = Self::auto_retry_count_for_status(task, status);
            if retry_count >= reconciliation_config().reviewing_max_retries as u32 {
                warn!(
                    task_id = task.id.as_str(),
                    retry_count = retry_count,
                    max = reconciliation_config().reviewing_max_retries,
                    "Review retry limit reached — escalating to Escalated"
                );
                return self
                    .apply_recovery_decision(
                        task,
                        status,
                        RecoveryContext::Review,
                        RecoveryDecision {
                            action: RecoveryActionKind::Transition(InternalStatus::Escalated),
                            reason: Some(format!(
                                "Review failed {} times — escalating for manual review",
                                retry_count
                            )),
                        },
                    )
                    .await;
            }
            if let Err(e) = self
                .record_auto_retry_metadata(task, status, retry_count + 1)
                .await
            {
                warn!(task_id = task.id.as_str(), error = %e, "Failed to record review retry metadata");
            }
        }

        self.apply_recovery_decision(task, status, RecoveryContext::Review, decision)
            .await
    }

    pub(crate) async fn reconcile_qa_task(
        &self,
        task: &crate::domain::entities::Task,
        status: InternalStatus,
    ) -> bool {
        if status != InternalStatus::QaRefining && status != InternalStatus::QaTesting {
            return false;
        }

        let age = match self.latest_status_transition_age(task, status).await {
            Some(age) => age,
            None => return false,
        };

        let context = if status == InternalStatus::QaRefining {
            RecoveryContext::QaRefining
        } else {
            RecoveryContext::QaTesting
        };

        // C5: Wall-clock timeout for long-running QA
        let max_qa_minutes = reconciliation_config().qa_max_wall_clock_minutes as i64;
        if age >= chrono::Duration::minutes(max_qa_minutes) {
            warn!(
                task_id = task.id.as_str(),
                age_minutes = age.num_minutes(),
                max_minutes = max_qa_minutes,
                "QA wall-clock timeout exceeded"
            );
            return self
                .apply_recovery_decision(
                    task,
                    status,
                    context,
                    RecoveryDecision {
                        action: RecoveryActionKind::Transition(InternalStatus::QaFailed),
                        reason: Some(format!(
                            "QA timed out after {} minutes (wall-clock limit: {}m)",
                            age.num_minutes(),
                            max_qa_minutes
                        )),
                    },
                )
                .await;
        }

        let evidence = RecoveryEvidence {
            run_status: None,
            registry_running: false,
            can_start: self.execution_state.can_start_task(),
            is_stale: age
                >= chrono::Duration::minutes(reconciliation_config().qa_stale_minutes as i64),
            is_deferred: false,
        };
        let decision = self.policy.decide_reconciliation(context, evidence);

        // E7: Enforce retry limit for QA re-spawns
        if decision.action == RecoveryActionKind::ExecuteEntryActions {
            let retry_count = Self::auto_retry_count_for_status(task, status);
            if retry_count >= reconciliation_config().qa_max_retries as u32 {
                warn!(
                    task_id = task.id.as_str(),
                    retry_count = retry_count,
                    max = reconciliation_config().qa_max_retries,
                    "QA retry limit reached — escalating to QaFailed"
                );
                return self
                    .apply_recovery_decision(
                        task,
                        status,
                        context,
                        RecoveryDecision {
                            action: RecoveryActionKind::Transition(InternalStatus::QaFailed),
                            reason: Some(format!(
                                "QA failed {} times — escalating to QaFailed",
                                retry_count
                            )),
                        },
                    )
                    .await;
            }
            if let Err(e) = self
                .record_auto_retry_metadata(task, status, retry_count + 1)
                .await
            {
                warn!(task_id = task.id.as_str(), error = %e, "Failed to record QA retry metadata");
            }
        }

        self.apply_recovery_decision(task, status, context, decision)
            .await
    }

    /// Auto-resume paused tasks that were paused due to recoverable provider errors.
    ///
    /// Checks pause_reason metadata in task.metadata (with backward compat for provider_error):
    /// - UserInitiated → skip (user-paused tasks stay paused until user resumes)
    /// - ProviderError with retry_after passed and resume_attempts < MAX → resume via entry actions
    /// - ProviderError with max attempts exceeded → transition to Failed
    /// - No metadata → skip (user-paused task)
    pub(crate) async fn reconcile_paused_provider_error(
        &self,
        task: &crate::domain::entities::Task,
    ) -> bool {
        use crate::application::chat_service::{PauseReason, ProviderErrorMetadata};

        // Read from new pause_reason key (with backward compat for old provider_error key)
        let pause_reason = match PauseReason::from_task_metadata(task.metadata.as_deref()) {
            Some(reason) => reason,
            None => {
                // Also try legacy ProviderErrorMetadata directly
                match ProviderErrorMetadata::from_task_metadata(task.metadata.as_deref()) {
                    Some(_) => {
                        // Fall through to legacy handling below
                        return self.reconcile_paused_provider_error_legacy(task).await;
                    }
                    None => return false, // No metadata — user-paused, skip
                }
            }
        };

        // UserInitiated pauses should NOT be auto-resumed
        let (category, message, retry_after, previous_status, auto_resumable, resume_attempts) =
            match pause_reason {
                PauseReason::UserInitiated { .. } => return false,
                PauseReason::ProviderError {
                    category,
                    message,
                    retry_after,
                    previous_status,
                    auto_resumable,
                    resume_attempts,
                    ..
                } => (
                    category,
                    message,
                    retry_after,
                    previous_status,
                    auto_resumable,
                    resume_attempts,
                ),
            };

        if !auto_resumable {
            return false;
        }

        // Check if max resume attempts exceeded → transition to Failed
        if resume_attempts >= ProviderErrorMetadata::max_resume_attempts() {
            warn!(
                task_id = task.id.as_str(),
                attempts = resume_attempts,
                max = ProviderErrorMetadata::max_resume_attempts(),
                category = %category,
                "Provider error auto-resume limit reached — transitioning to Failed"
            );
            // Clear pause metadata and transition to Failed
            let mut updated = task.clone();
            updated.metadata = Some(PauseReason::clear_from_task_metadata(
                updated.metadata.as_deref(),
            ));
            updated.touch();
            let _ = self.task_repo.update(&updated).await;

            let _ = self
                .transition_service
                .transition_task(&task.id, InternalStatus::Failed)
                .await;
            return true;
        }

        // Build a ProviderErrorMetadata to check retry eligibility
        let meta = ProviderErrorMetadata {
            category: category.clone(),
            message: message.clone(),
            retry_after: retry_after.clone(),
            previous_status: previous_status.clone(),
            paused_at: String::new(),
            auto_resumable,
            resume_attempts,
        };

        // Check if retry_after time has passed
        if !meta.is_retry_eligible() {
            return false; // Not eligible yet — wait for retry_after
        }

        // Can we start a task right now?
        if !self.execution_state.can_start_task() {
            return false; // At max concurrency — retry on next reconciliation cycle
        }

        // Increment resume attempts
        let updated_reason = PauseReason::ProviderError {
            category: category.clone(),
            message,
            retry_after,
            previous_status: previous_status.clone(),
            paused_at: chrono::Utc::now().to_rfc3339(),
            auto_resumable,
            resume_attempts: resume_attempts + 1,
        };
        let mut updated_task = task.clone();
        updated_task.metadata =
            Some(updated_reason.write_to_task_metadata(updated_task.metadata.as_deref()));
        updated_task.touch();
        if let Err(e) = self.task_repo.update(&updated_task).await {
            warn!(
                task_id = task.id.as_str(),
                error = %e,
                "Failed to update resume attempt count"
            );
            return false;
        }

        // Determine the target state to resume to (from previous_status)
        let resume_status = match previous_status.as_str() {
            "executing" => InternalStatus::Executing,
            "re_executing" => InternalStatus::ReExecuting,
            "qa_refining" => InternalStatus::QaRefining,
            "qa_testing" => InternalStatus::QaTesting,
            "reviewing" => InternalStatus::Reviewing,
            "merging" => InternalStatus::Merging,
            _ => InternalStatus::Executing, // Safe default
        };

        tracing::info!(
            task_id = task.id.as_str(),
            category = %category,
            resume_status = %resume_status,
            attempt = resume_attempts + 1,
            "Auto-resuming provider-error-paused task"
        );

        // Emit event for frontend
        if let Some(ref handle) = self.app_handle {
            let _ = handle.emit(
                "task:provider_error_resuming",
                serde_json::json!({
                    "task_id": task.id.as_str(),
                    "category": category.to_string(),
                    "resume_status": resume_status.to_string(),
                    "attempt": resume_attempts + 1,
                }),
            );
        }

        // Transition back to the previous active state
        // The TransitionHandler's on_enter will re-spawn the agent
        match self
            .transition_service
            .transition_task(&task.id, resume_status)
            .await
        {
            Ok(_) => {
                // Clear pause metadata on successful resume
                let mut cleared_task = match self.task_repo.get_by_id(&task.id).await {
                    Ok(Some(t)) => t,
                    _ => return true,
                };
                cleared_task.metadata = Some(PauseReason::clear_from_task_metadata(
                    cleared_task.metadata.as_deref(),
                ));
                cleared_task.touch();
                let _ = self.task_repo.update(&cleared_task).await;
                true
            }
            Err(e) => {
                warn!(
                    task_id = task.id.as_str(),
                    error = %e,
                    resume_status = %resume_status,
                    "Failed to auto-resume provider-error-paused task"
                );
                false
            }
        }
    }

    /// Legacy handler for tasks with old provider_error key (backward compat).
    pub(crate) async fn reconcile_paused_provider_error_legacy(
        &self,
        task: &crate::domain::entities::Task,
    ) -> bool {
        use crate::application::chat_service::ProviderErrorMetadata;

        let meta = match ProviderErrorMetadata::from_task_metadata(task.metadata.as_deref()) {
            Some(meta) => meta,
            None => return false,
        };

        if !meta.auto_resumable {
            return false;
        }

        if meta.resume_attempts >= ProviderErrorMetadata::max_resume_attempts() {
            warn!(
                task_id = task.id.as_str(),
                attempts = meta.resume_attempts,
                "Legacy provider error auto-resume limit reached — transitioning to Failed"
            );
            let mut updated = task.clone();
            updated.metadata = Some(ProviderErrorMetadata::clear_from_task_metadata(
                updated.metadata.as_deref(),
            ));
            updated.touch();
            let _ = self.task_repo.update(&updated).await;
            let _ = self
                .transition_service
                .transition_task(&task.id, InternalStatus::Failed)
                .await;
            return true;
        }

        if !meta.is_retry_eligible() {
            return false;
        }

        if !self.execution_state.can_start_task() {
            return false;
        }

        let mut updated_meta = meta.clone();
        updated_meta.resume_attempts += 1;
        let mut updated_task = task.clone();
        updated_task.metadata =
            Some(updated_meta.write_to_task_metadata(updated_task.metadata.as_deref()));
        updated_task.touch();
        if let Err(e) = self.task_repo.update(&updated_task).await {
            warn!(task_id = task.id.as_str(), error = %e, "Failed to update legacy resume attempts");
            return false;
        }

        let resume_status = match meta.previous_status.as_str() {
            "executing" => InternalStatus::Executing,
            "re_executing" => InternalStatus::ReExecuting,
            "qa_refining" => InternalStatus::QaRefining,
            "qa_testing" => InternalStatus::QaTesting,
            "reviewing" => InternalStatus::Reviewing,
            "merging" => InternalStatus::Merging,
            _ => InternalStatus::Executing,
        };

        match self
            .transition_service
            .transition_task(&task.id, resume_status)
            .await
        {
            Ok(_) => {
                let mut cleared = match self.task_repo.get_by_id(&task.id).await {
                    Ok(Some(t)) => t,
                    _ => return true,
                };
                cleared.metadata = Some(ProviderErrorMetadata::clear_from_task_metadata(
                    cleared.metadata.as_deref(),
                ));
                cleared.touch();
                let _ = self.task_repo.update(&cleared).await;
                true
            }
            Err(e) => {
                warn!(task_id = task.id.as_str(), error = %e, "Failed legacy auto-resume");
                false
            }
        }
    }

    pub async fn recover_execution_stop(&self, task_id: &TaskId) -> bool {
        let task = match self.task_repo.get_by_id(task_id).await {
            Ok(Some(task)) => task,
            Ok(None) => return false,
            Err(e) => {
                warn!(task_id = task_id.as_str(), error = %e, "Failed to load task for stop recovery");
                return false;
            }
        };

        if task.internal_status != InternalStatus::Executing
            && task.internal_status != InternalStatus::ReExecuting
        {
            return false;
        }

        let key = crate::domain::services::RunningAgentKey::new(
            ChatContextType::TaskExecution.to_string(),
            task.id.as_str(),
        );

        // Skip recovery if the process has a live interactive slot — it's alive
        // between turns, waiting for the next stdin message. Stopping it would kill
        // a healthy interactive agent. Cross-references PID liveness to avoid stale entries.
        if self
            .is_ipr_process_alive(ChatContextType::TaskExecution, task.id.as_str())
            .await
        {
            tracing::debug!(
                task_id = task_id.as_str(),
                "Skipping recovery stop — active interactive process"
            );
            return false;
        }

        let registry_running = self.running_agent_registry.is_running(&key).await;
        if registry_running {
            let _ = self.running_agent_registry.stop(&key).await;
        }

        let run = self.load_execution_run(&task, task.internal_status).await;
        let evidence = RecoveryEvidence {
            run_status: run.as_ref().map(|r| r.status),
            registry_running,
            can_start: self.execution_state.can_start_task(),
            is_stale: false,
            is_deferred: false,
        };
        let decision = self.policy.decide_execution_stop(evidence);

        self.apply_recovery_decision(
            &task,
            task.internal_status,
            RecoveryContext::Execution,
            decision,
        )
        .await
    }

    pub async fn apply_user_recovery_action(
        &self,
        task: &crate::domain::entities::Task,
        action: UserRecoveryAction,
    ) -> bool {
        let status = task.internal_status;

        // GAP H2: Failed state has custom recovery logic — delegate to dedicated handler
        if status == InternalStatus::Failed {
            return self.apply_failed_user_recovery_action(task, action).await;
        }

        let decision = match status {
            InternalStatus::Executing | InternalStatus::ReExecuting => match action {
                UserRecoveryAction::Restart => RecoveryDecision {
                    action: RecoveryActionKind::ExecuteEntryActions,
                    reason: None,
                },
                UserRecoveryAction::Cancel => RecoveryDecision {
                    action: RecoveryActionKind::Transition(InternalStatus::Ready),
                    reason: None,
                },
            },
            InternalStatus::PendingMerge => match action {
                UserRecoveryAction::Restart => RecoveryDecision {
                    action: RecoveryActionKind::ExecuteEntryActions,
                    reason: None,
                },
                UserRecoveryAction::Cancel => RecoveryDecision {
                    action: RecoveryActionKind::Transition(InternalStatus::MergeIncomplete),
                    reason: None,
                },
            },
            InternalStatus::Reviewing
            | InternalStatus::Merging
            | InternalStatus::QaRefining
            | InternalStatus::QaTesting => match action {
                UserRecoveryAction::Restart => RecoveryDecision {
                    action: RecoveryActionKind::ExecuteEntryActions,
                    reason: None,
                },
                UserRecoveryAction::Cancel => {
                    let next_status = match status {
                        InternalStatus::Reviewing => InternalStatus::Escalated,
                        InternalStatus::Merging => InternalStatus::MergeConflict,
                        InternalStatus::QaRefining | InternalStatus::QaTesting => {
                            InternalStatus::QaFailed
                        }
                        _ => InternalStatus::Escalated,
                    };
                    RecoveryDecision {
                        action: RecoveryActionKind::Transition(next_status),
                        reason: None,
                    }
                }
            },
            _ => return false,
        };

        self.clear_prompt_marker(task.id.as_str(), status).await;

        let context = match status {
            InternalStatus::Executing | InternalStatus::ReExecuting => RecoveryContext::Execution,
            InternalStatus::Reviewing => RecoveryContext::Review,
            InternalStatus::Merging => RecoveryContext::Merge,
            InternalStatus::PendingMerge => RecoveryContext::PendingMerge,
            InternalStatus::QaRefining => RecoveryContext::QaRefining,
            InternalStatus::QaTesting => RecoveryContext::QaTesting,
            _ => return false,
        };

        self.apply_recovery_decision(task, status, context, decision)
            .await
    }

    /// Handle user recovery actions for tasks in Failed state (GAP H2).
    ///
    /// - `Restart` → reset retry budget (H9) + full git cleanup + ManualRetry event → Ready
    /// - `Cancel`  → `stop_retrying=true` → task stays Failed permanently
    async fn apply_failed_user_recovery_action(
        &self,
        task: &crate::domain::entities::Task,
        action: UserRecoveryAction,
    ) -> bool {
        match action {
            UserRecoveryAction::Cancel => {
                // User explicitly cancelled auto-retry — task remains Failed permanently
                if let Err(e) = self.stop_execution_retrying_by_user(task).await {
                    warn!(
                        task_id = task.id.as_str(),
                        error = %e,
                        "Failed to set stop_retrying for user Cancel on Failed task"
                    );
                }
                true
            }
            UserRecoveryAction::Restart => {
                // Full git cleanup — same sequence as reconcile_failed_execution_task() (B5, B8, H8)
                match self.project_repo.get_by_id(&task.project_id).await {
                    Ok(Some(project)) => {
                        let repo_path = std::path::Path::new(&project.working_directory);

                        // (1) Delete worktree FIRST — must happen before branch deletion (GAP H8)
                        if let Some(worktree_path_str) = task.worktree_path.as_deref() {
                            let worktree_path = std::path::Path::new(worktree_path_str);
                            if worktree_path.exists() {
                                if let Err(e) =
                                    GitService::delete_worktree(repo_path, worktree_path).await
                                {
                                    warn!(
                                        task_id = task.id.as_str(),
                                        worktree = worktree_path_str,
                                        error = %e,
                                        "Failed to delete worktree during manual restart git cleanup"
                                    );
                                }
                            }
                        }

                        // (2) Delete branch — log warn and continue if fails (GAP M9)
                        if let Some(branch) = task.task_branch.as_deref() {
                            if let Err(e) =
                                GitService::delete_branch(repo_path, branch, true).await
                            {
                                warn!(
                                    task_id = task.id.as_str(),
                                    branch = branch,
                                    error = %e,
                                    "Failed to delete task branch during manual restart git cleanup"
                                );
                            }
                        }
                    }
                    Ok(None) => {
                        warn!(
                            task_id = task.id.as_str(),
                            "Project not found for task — skipping git cleanup in manual restart"
                        );
                    }
                    Err(e) => {
                        warn!(
                            task_id = task.id.as_str(),
                            error = %e,
                            "Failed to get project for manual restart git cleanup"
                        );
                    }
                }

                // (3) Clear task_branch + worktree_path in DB
                let mut updated_task = task.clone();
                updated_task.task_branch = None;
                updated_task.worktree_path = None;
                updated_task.touch();
                if let Err(e) = self.task_repo.update(&updated_task).await {
                    warn!(
                        task_id = task.id.as_str(),
                        error = %e,
                        "Failed to clear task_branch/worktree_path in DB during manual restart"
                    );
                }

                // (GAP H9) Reset recovery metadata AFTER task_repo.update() so it wins —
                // update() carries the stale task.metadata (with events + flat keys) and would
                // overwrite any earlier metadata write. Resetting last ensures a clean slate.
                // reset_execution_recovery_metadata() also removes is_timeout/failure_error (GAP B7).
                if let Err(e) = self.reset_execution_recovery_metadata(task).await {
                    warn!(
                        task_id = task.id.as_str(),
                        error = %e,
                        "Failed to reset execution recovery metadata for manual restart"
                    );
                }

                // Re-fetch task with clean metadata before recording ManualRetry event
                let task = match self.task_repo.get_by_id(&task.id).await {
                    Ok(Some(t)) => t,
                    Ok(None) => {
                        warn!(
                            task_id = task.id.as_str(),
                            "Task not found after metadata cleanup in manual restart"
                        );
                        return false;
                    }
                    Err(e) => {
                        warn!(
                            task_id = task.id.as_str(),
                            error = %e,
                            "Failed to re-fetch task after metadata cleanup in manual restart"
                        );
                        task.clone()
                    }
                };

                // Record ManualRetry event in execution recovery metadata
                if let Err(e) = self.record_execution_manual_retry_event(&task).await {
                    warn!(
                        task_id = task.id.as_str(),
                        error = %e,
                        "Failed to record ManualRetry event for manual restart"
                    );
                }

                // Transition Failed → Ready
                info!(task_id = task.id.as_str(), "User manually restarting failed execution task");
                if let Err(e) = self
                    .transition_service
                    .transition_task(&task.id, InternalStatus::Ready)
                    .await
                {
                    tracing::error!(
                        task_id = task.id.as_str(),
                        error = %e,
                        "Failed to transition task from Failed to Ready during manual restart"
                    );
                    return false;
                }

                true
            }
        }
    }

    pub(crate) async fn apply_recovery_decision(
        &self,
        task: &crate::domain::entities::Task,
        status: InternalStatus,
        context: RecoveryContext,
        decision: RecoveryDecision,
    ) -> bool {
        match decision.action {
            RecoveryActionKind::None => {
                tracing::debug!(
                    task_id = task.id.as_str(),
                    status = ?status,
                    context = ?context,
                    "Reconciliation policy returned None — no recovery action taken"
                );
                false
            }
            RecoveryActionKind::ExecuteEntryActions => {
                // Clean stale registry entry before re-spawning agent.
                // Without this, try_register fails (old slot occupied), the message
                // gets queued, and the conflict never resolves → infinite thrash loop.
                let chat_context = match context {
                    RecoveryContext::Execution => ChatContextType::TaskExecution,
                    RecoveryContext::Review => ChatContextType::Review,
                    RecoveryContext::Merge | RecoveryContext::PendingMerge => {
                        ChatContextType::Merge
                    }
                    RecoveryContext::QaRefining | RecoveryContext::QaTesting => {
                        ChatContextType::TaskExecution
                    }
                };
                let registry_key = crate::domain::services::RunningAgentKey::new(
                    chat_context.to_string(),
                    task.id.as_str(),
                );

                // Skip recovery re-spawn if the process has a live interactive
                // slot — the agent is alive between turns and doesn't need respawning.
                // Cross-references PID liveness to avoid stale entries.
                if self
                    .is_ipr_process_alive(chat_context, task.id.as_str())
                    .await
                {
                    tracing::debug!(
                        task_id = task.id.as_str(),
                        context_type = %chat_context,
                        "Skipping recovery re-spawn — active interactive process"
                    );
                    return false;
                }

                if self.running_agent_registry.is_running(&registry_key).await {
                    tracing::info!(
                        task_id = task.id.as_str(),
                        context_type = %chat_context,
                        "Clearing stale registry entry before recovery re-spawn"
                    );
                    let _ = self.running_agent_registry.stop(&registry_key).await;
                }

                // Set trigger_origin="recovery" before resuming agent
                let mut task_mut = task.clone();
                set_trigger_origin(&mut task_mut, "recovery");
                if let Err(e) = self.task_repo.update(&task_mut).await {
                    tracing::error!(
                        task_id = task.id.as_str(),
                        error = %e,
                        "Failed to set trigger_origin=recovery in metadata"
                    );
                }

                self.transition_service
                    .execute_entry_actions(&task.id, task, status)
                    .await;
                true
            }
            RecoveryActionKind::Transition(next_status) => {
                // Store escalation reason before transitioning to Escalated from a Review context
                if matches!(context, RecoveryContext::Review)
                    && matches!(next_status, InternalStatus::Escalated)
                {
                    if let Some(ref repo) = self.review_repo {
                        let reason = decision
                            .reason
                            .as_deref()
                            .unwrap_or("Review recovery: task escalated by reconciler");
                        let note = ReviewNote::with_notes(
                            task.id.clone(),
                            ReviewerType::Ai,
                            ReviewOutcome::Rejected,
                            reason.to_string(),
                        );
                        if let Err(e) = repo.add_note(&note).await {
                            tracing::warn!(
                                task_id = task.id.as_str(),
                                error = %e,
                                "Failed to store escalation ReviewNote during recovery"
                            );
                        }
                    }
                }
                if let Err(e) = self
                    .transition_service
                    .transition_task(&task.id, next_status)
                    .await
                {
                    tracing::error!(
                        task_id = task.id.as_str(),
                        error = %e,
                        "Failed to transition task during recovery"
                    );
                    return false;
                }
                true
            }
            RecoveryActionKind::AttemptMergeAutoComplete => {
                let merge_ctx = MergeAutoCompleteContext {
                    task_id_str: task.id.as_str(),
                    task_id: task.id.clone(),
                    task_repo: &self.task_repo,
                    task_dependency_repo: &self.task_dep_repo,
                    project_repo: &self.project_repo,
                    chat_message_repo: &self.chat_message_repo,
                    chat_attachment_repo: &self.chat_attachment_repo,
                    conversation_repo: &self.chat_conversation_repo,
                    agent_run_repo: &self.agent_run_repo,
                    ideation_session_repo: &self.ideation_session_repo,
                    activity_event_repo: &self.activity_event_repo,
                    message_queue: &self.message_queue,
                    running_agent_registry: &self.running_agent_registry,
                    memory_event_repo: &self.memory_event_repo,
                    execution_state: &self.execution_state,
                    plan_branch_repo: &self.plan_branch_repo,
                    app_handle: self.app_handle.as_ref(),
                    interactive_process_registry: &None,
                };
                reconcile_merge_auto_complete(&merge_ctx).await;
                true
            }
            RecoveryActionKind::Prompt => {
                let reason = decision
                    .reason
                    .unwrap_or_else(|| "Recovery decision requires user input.".to_string());
                self.emit_recovery_prompt(task, status, context, reason)
                    .await
            }
        }
    }
}
