// Execution, review, QA, and paused-task reconciliation handlers,
// plus orchestration (reconcile_task, reconcile_stuck_tasks) and apply_recovery_decision.

use std::str::FromStr;

use tauri::{Emitter, Runtime};
use tracing::warn;

use crate::application::chat_service::{MergeAutoCompleteContext, reconcile_merge_auto_complete};
use crate::application::interactive_process_registry::InteractiveProcessKey;
use crate::commands::execution_commands::AGENT_ACTIVE_STATUSES;
use crate::domain::entities::{
    AgentRunId, AgentRunStatus, ChatContextType, InternalStatus, ReviewNote, ReviewOutcome,
    ReviewerType, TaskId,
};
use crate::domain::services::RunningAgentKey;
use crate::domain::state_machine::transition_handler::set_trigger_origin;
use crate::infrastructure::agents::claude::reconciliation_config;

use super::super::policy::{
    RecoveryActionKind, RecoveryContext, RecoveryDecision, RecoveryEvidence, UserRecoveryAction,
};
use super::super::ReconciliationRunner;

fn context_matches_task_status(context_type: ChatContextType, status: InternalStatus) -> bool {
    match context_type {
        ChatContextType::TaskExecution => {
            status == InternalStatus::Executing || status == InternalStatus::ReExecuting
        }
        ChatContextType::Review => status == InternalStatus::Reviewing,
        ChatContextType::Merge => status == InternalStatus::Merging,
        ChatContextType::Task | ChatContextType::Ideation | ChatContextType::Project => {
            AGENT_ACTIVE_STATUSES.contains(&status)
        }
    }
}

fn process_is_alive(pid: u32) -> bool {
    // PID 0 refers to the process group on macOS/Unix — `kill -0 0` succeeds
    // but doesn't mean a real agent is alive. Placeholder PIDs from try_register
    // use pid=0 before update_agent_process fills in the real PID.
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|status| status.success())
            .unwrap_or(true)
    }

    #[cfg(windows)]
    {
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/FO", "CSV", "/NH"])
            .output()
            .map(|output| {
                if !output.status.success() {
                    return true;
                }
                let text = String::from_utf8_lossy(&output.stdout);
                !text.to_ascii_lowercase().contains("no tasks are running")
            })
            .unwrap_or(true)
    }
}

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
            Some(info) => process_is_alive(info.pid),
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
    /// Runs once at app startup (not in the recurring loop). Finds tasks in Failed state where
    /// `is_timeout: true` (in metadata) and `auto_retry_count_executing < 3`, then transitions
    /// them back to Ready so they can be picked up on the next scheduling cycle.
    ///
    /// Cap at 3 attempts prevents infinite retry on persistent failures; timeout-only filter
    /// ensures we only recover transient overnight stalls, not logic errors.
    pub async fn recover_timeout_failures(&self) {
        let projects = match self.project_repo.get_all().await {
            Ok(projects) => projects,
            Err(e) => {
                warn!(error = %e, "recover_timeout_failures: failed to get projects");
                return;
            }
        };

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
                let is_timeout = self.task_is_timeout_failure(&task);
                let attempt_count =
                    Self::auto_retry_count_for_status(&task, InternalStatus::Executing);

                if !is_timeout || attempt_count >= 3 {
                    continue;
                }

                tracing::info!(
                    task_id = task.id.as_str(),
                    attempt_count = attempt_count,
                    "Startup recovery: re-queuing timeout-failed task"
                );

                // Increment attempt count before transitioning so the count persists
                // across recovery cycles and the cap is enforced correctly next startup.
                if let Err(e) = self
                    .record_auto_retry_metadata(&task, InternalStatus::Executing, attempt_count + 1)
                    .await
                {
                    warn!(
                        task_id = task.id.as_str(),
                        error = %e,
                        "Startup recovery: failed to increment attempt count (proceeding anyway)"
                    );
                }

                match self
                    .transition_service
                    .transition_task(&task.id, InternalStatus::Ready)
                    .await
                {
                    Ok(_) => {
                        tracing::info!(
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

        let mut removed = 0u32;

        for (key, info) in entries {
            let context_type = ChatContextType::from_str(&key.context_type).ok();
            let pid_alive = process_is_alive(info.pid);

            // Skip in-flight registrations: try_register inserts pid=0/empty agent_run_id as
            // placeholder; update_agent_process fills real values ~40ms later. Pruning during
            // this window would incorrectly delete a valid registration.
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

            // Skip entries with an active interactive process AND a live PID.
            // If IPR has an entry but the PID is dead, the entry is stale — remove
            // it so reconciliation can proceed instead of being blocked forever.
            if let Some(ref ipr) = self.interactive_process_registry {
                let ipr_key =
                    InteractiveProcessKey::new(&key.context_type, &key.context_id);
                if ipr.has_process(&ipr_key).await {
                    if pid_alive {
                        tracing::debug!(
                            context_type = key.context_type,
                            context_id = key.context_id,
                            "Skipping prune for interactive process"
                        );
                        continue;
                    }
                    // PID is dead but IPR still has entry → stale, remove it
                    warn!(
                        context_type = key.context_type,
                        context_id = key.context_id,
                        pid = info.pid,
                        "Removing stale IPR entry during prune — PID no longer alive"
                    );
                    ipr.remove(&ipr_key).await;
                }
            }

            let run = match self
                .agent_run_repo
                .get_by_id(&AgentRunId::from_string(&info.agent_run_id))
                .await
            {
                Ok(run) => run,
                Err(e) => {
                    warn!(
                        context_type = key.context_type,
                        context_id = key.context_id,
                        run_id = info.agent_run_id,
                        error = %e,
                        "Failed to load agent_run while pruning running registry; keeping entry"
                    );
                    continue;
                }
            };

            let mut stale_reasons: Vec<&str> = Vec::new();

            if !pid_alive {
                stale_reasons.push("pid_missing");
            }

            match run.as_ref() {
                Some(agent_run) if agent_run.status != AgentRunStatus::Running => {
                    stale_reasons.push("run_not_running");
                }
                None => {
                    stale_reasons.push("run_missing");
                }
                _ => {}
            }

            if let Some(ctx) = context_type {
                if matches!(
                    ctx,
                    ChatContextType::TaskExecution
                        | ChatContextType::Review
                        | ChatContextType::Merge
                ) {
                    let task_id = TaskId::from_string(key.context_id.clone());
                    match self.task_repo.get_by_id(&task_id).await {
                        Ok(Some(task)) => {
                            if !context_matches_task_status(ctx, task.internal_status) {
                                stale_reasons.push("task_status_mismatch");
                            }
                        }
                        Ok(None) => stale_reasons.push("task_missing"),
                        Err(e) => {
                            warn!(
                                context_type = key.context_type,
                                context_id = key.context_id,
                                error = %e,
                                "Failed to load task while pruning running registry; keeping entry"
                            );
                            continue;
                        }
                    }
                }
            }

            if stale_reasons.is_empty() {
                continue;
            }

            if pid_alive {
                let _ = self.running_agent_registry.stop(&key).await;
            } else {
                let _ = self
                    .running_agent_registry
                    .unregister(&key, &info.agent_run_id)
                    .await;
            }
            removed += 1;

            if let Some(agent_run) = run {
                if agent_run.status == AgentRunStatus::Running {
                    let _ = self
                        .agent_run_repo
                        .cancel(&AgentRunId::from_string(&info.agent_run_id))
                        .await;
                }
            }

            warn!(
                context_type = key.context_type,
                context_id = key.context_id,
                pid = info.pid,
                run_id = info.agent_run_id,
                reasons = stale_reasons.join(","),
                "Pruned stale running agent registry entry"
            );
        }

        let remaining_entries = self.running_agent_registry.list_all().await;
        let registry_count = remaining_entries.len() as u32;
        // Subtract idle interactive processes that already freed their execution slot
        // via TurnComplete but still have a registry entry (process alive, waiting for stdin).
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
