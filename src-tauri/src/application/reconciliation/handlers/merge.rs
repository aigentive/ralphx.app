// Merge-specific reconciliation handlers: Merging, PendingMerge, MergeIncomplete, MergeConflict.

use tauri::Runtime;
use tracing::{debug, warn};

use crate::application::interactive_process_registry::InteractiveProcessKey;
use crate::domain::entities::{
    AgentRunStatus, ChatContextType, InternalStatus, MergeFailureSource,
};
use crate::domain::state_machine::transition_handler::has_branch_missing_metadata;
use crate::infrastructure::agents::claude::reconciliation_config;

use super::super::policy::{
    RecoveryActionKind, RecoveryContext, RecoveryDecision, RecoveryEvidence, ShaComparisonResult,
};
use super::super::ReconciliationRunner;

impl<R: Runtime> ReconciliationRunner<R> {
    pub(crate) async fn reconcile_merging_task(
        &self,
        task: &crate::domain::entities::Task,
        status: InternalStatus,
    ) -> bool {
        if status != InternalStatus::Merging {
            return false;
        }

        // Skip if there's an active interactive process — the agent is alive between turns
        if let Some(ref ipr) = self.interactive_process_registry {
            let ipr_key =
                InteractiveProcessKey::new(ChatContextType::Merge.to_string(), task.id.as_str());
            if ipr.has_process(&ipr_key).await {
                tracing::debug!(
                    task_id = task.id.as_str(),
                    "Skipping merge reconciliation: interactive process active"
                );
                return true;
            }
        }

        // Auto-complete in-flight guard: if attempt_merge_auto_complete is already running
        // for this task (e.g. triggered by agent completion), skip this reconciliation cycle.
        // Without this, the reconciler misinterprets the dedup guard's "skip" as a failure
        // and may incorrectly escalate (increment retry count, eventually → MergeIncomplete).
        if self
            .execution_state
            .is_auto_complete_in_flight(task.id.as_str())
        {
            tracing::debug!(
                task_id = task.id.as_str(),
                "Skipping Merging reconciliation — auto-complete already in flight"
            );
            return true;
        }

        let run = self
            .lookup_latest_run_for_task_context(task, ChatContextType::Merge)
            .await;
        let transition_age = match self.latest_status_transition_age(task, status).await {
            Some(age) => age,
            None => return false,
        };
        // Read last_active_at from registry to compute activity-based effective age.
        // Falls back to wall-clock transition age when no heartbeat exists.
        let merge_key = crate::domain::services::RunningAgentKey::new(
            ChatContextType::Merge.to_string(),
            task.id.as_str(),
        );
        let agent_info = self.running_agent_registry.get(&merge_key).await;
        let effective_age = agent_info
            .as_ref()
            .and_then(|info| info.last_active_at)
            .map(|last_active| chrono::Utc::now() - last_active)
            .unwrap_or(transition_age);
        let mut evidence = self
            .build_run_evidence(task, ChatContextType::Merge, run.as_ref())
            .await;
        evidence.is_stale = effective_age
            >= chrono::Duration::seconds(reconciliation_config().merger_timeout_secs as i64);

        // Agent is running, registered, and not stale — let it work
        if evidence.run_status == Some(AgentRunStatus::Running)
            && evidence.registry_running
            && !evidence.is_stale
        {
            return true;
        }

        if evidence.is_stale {
            self.record_merge_timeout_event(task, effective_age).await;
        }

        // Gap 1: Check retry count — escalate to MergeConflict after max retries.
        // Re-read task to get updated metadata after record_merge_timeout_event.
        let updated_task = match self.task_repo.get_by_id(&task.id).await {
            Ok(Some(t)) => t,
            Ok(None) => return false,
            Err(_) => return false,
        };
        let retry_count = Self::merging_auto_retry_count(&updated_task);
        if retry_count >= reconciliation_config().merging_max_retries as u32 {
            warn!(
                task_id = task.id.as_str(),
                retry_count = retry_count,
                max = reconciliation_config().merging_max_retries,
                "Merging retry limit reached — transitioning to MergeIncomplete for user retry"
            );
            // Use MergeIncomplete (not MergeConflict) because timeout indicates a hung agent,
            // not an explicit merge conflict. MergeIncomplete surfaces the task in the
            // needs_attention panel and allows auto-retry via reconcile_merge_incomplete_task.
            return self
                .apply_recovery_decision(
                    &updated_task,
                    status,
                    RecoveryContext::Merge,
                    RecoveryDecision {
                        action: RecoveryActionKind::Transition(InternalStatus::MergeIncomplete),
                        reason: Some(format!(
                            "Merger agent timed out {} times — transitioning to MergeIncomplete for retry",
                            retry_count
                        )),
                    },
                )
                .await;
        }

        let decision = self
            .policy
            .decide_reconciliation(RecoveryContext::Merge, evidence);

        // Auto-recover merge conflicts instead of prompting the user.
        // When DB run state disagrees with registry (has_conflict), the agent likely died
        // silently. Instead of bothering the user, auto-restart within the retry budget.
        let decision = if decision.action == RecoveryActionKind::Prompt {
            // Grace period: if the agent run was created recently, the PID may not
            // have been registered yet — skip this cycle and let registration catch up.
            let grace_period_secs = reconciliation_config().merge_registry_grace_period_secs as i64;
            let within_grace_period = run.as_ref().map_or(false, |r| {
                let age = chrono::Utc::now() - r.started_at;
                age < chrono::Duration::seconds(grace_period_secs)
            });

            if within_grace_period {
                tracing::debug!(
                    task_id = task.id.as_str(),
                    grace_period_secs = grace_period_secs,
                    "Merge conflict detection within grace period — skipping"
                );
                return false;
            }

            // Within retry budget: auto-restart the merger agent
            warn!(
                task_id = task.id.as_str(),
                retry_count = retry_count,
                "Auto-recovering merge conflict: restarting merger agent (run state vs registry mismatch)"
            );
            RecoveryDecision {
                action: RecoveryActionKind::ExecuteEntryActions,
                reason: Some(
                    "Auto-recovering merge run state conflict — restarting merger agent."
                        .to_string(),
                ),
            }
        } else {
            decision
        };

        // Gap 2: Don't re-spawn agent if one is still running in registry
        if decision.action == RecoveryActionKind::ExecuteEntryActions && evidence.registry_running {
            warn!(
                task_id = task.id.as_str(),
                "Skipping merger agent re-spawn — agent still running in registry"
            );
            return false;
        }

        // Gap 4: Don't retry if the merge worktree doesn't exist — retrying is futile
        // since the spawn will fail again with the same "no valid merge worktree" error.
        if decision.action == RecoveryActionKind::ExecuteEntryActions {
            if let Some(ref wt_path) = updated_task.worktree_path {
                let wt = std::path::PathBuf::from(wt_path);
                if !wt.exists() {
                    warn!(
                        task_id = task.id.as_str(),
                        worktree_path = %wt_path,
                        "Merge worktree does not exist — skipping futile retry, transitioning to MergeIncomplete"
                    );
                    return self
                        .apply_recovery_decision(
                            &updated_task,
                            status,
                            RecoveryContext::Merge,
                            RecoveryDecision {
                                action: RecoveryActionKind::Transition(InternalStatus::MergeIncomplete),
                                reason: Some(format!(
                                    "Merge worktree {} does not exist — cannot spawn merger agent",
                                    wt_path
                                )),
                            },
                        )
                        .await;
                }
            }
        }

        // Gap 3: After auto-complete, check if task is still stuck in Merging
        if decision.action == RecoveryActionKind::AttemptMergeAutoComplete {
            self.apply_recovery_decision(&updated_task, status, RecoveryContext::Merge, decision)
                .await;
            // Re-read to see if auto-complete transitioned the task
            if let Ok(Some(post_task)) = self.task_repo.get_by_id(&task.id).await {
                if post_task.internal_status == InternalStatus::Merging {
                    warn!(
                        task_id = task.id.as_str(),
                        "Auto-complete did not transition task out of Merging — will escalate on next timeout"
                    );
                }
            }
            return true;
        }

        self.apply_recovery_decision(&updated_task, status, RecoveryContext::Merge, decision)
            .await
    }

    pub(crate) async fn reconcile_pending_merge_task(
        &self,
        task: &crate::domain::entities::Task,
        status: InternalStatus,
    ) -> bool {
        use crate::domain::state_machine::transition_handler::{
            has_main_merge_deferred_metadata, has_merge_deferred_metadata,
        };

        if status != InternalStatus::PendingMerge {
            return false;
        }

        // Merge-pipeline-active guard: if attempt_programmatic_merge is actively running
        // (set at start, cleared at end), skip reconciliation to prevent the reconciler
        // from killing the merge mid-pipeline (cleanup 60s + freshness 60s > stale 2min).
        if Self::has_merge_pipeline_active(task) {
            tracing::info!(
                task_id = task.id.as_str(),
                "Skipping PendingMerge reconciliation — merge pipeline active"
            );
            return true;
        }

        // Validation-in-progress guard: if validation commands are actively running
        // (set before run_validation_commands, cleared after), skip reconciliation to
        // prevent the reconciler from re-triggering merge while cargo test etc. runs.
        if Self::has_validation_in_progress(task) {
            tracing::debug!(
                task_id = task.id.as_str(),
                "Skipping PendingMerge reconciliation — validation in progress"
            );
            return true;
        }

        // Phase 4: Handle main-merge-deferred tasks (deferred because agents were running)
        // If no agents are running now, retry the merge. If agents are still running, skip.
        if has_main_merge_deferred_metadata(task) {
            let running_count = self.execution_state.running_count();
            if running_count == 0 {
                tracing::info!(
                    task_id = task.id.as_str(),
                    "Main-merge-deferred task ready to retry: all agents completed"
                );
                return self
                    .apply_recovery_decision(
                        task,
                        status,
                        RecoveryContext::PendingMerge,
                        RecoveryDecision {
                            action: RecoveryActionKind::ExecuteEntryActions,
                            reason: Some(
                                "Main merge deferred while agents running — now retrying."
                                    .to_string(),
                            ),
                        },
                    )
                    .await;
            } else {
                tracing::debug!(
                    task_id = task.id.as_str(),
                    running_count = running_count,
                    "Main-merge-deferred task waiting for global idle: agents still running"
                );
                // Task is correctly deferred, not orphaned — skip reconciliation
                return true;
            }
        }

        let age = match self.latest_status_transition_age(task, status).await {
            Some(age) => age,
            None => return false,
        };

        let is_deferred = has_merge_deferred_metadata(task);

        // Deferred-orphan watchdog: if the recorded blocker is no longer active
        // (missing, archived, or no longer in merge workflow), immediately re-trigger
        // entry actions instead of waiting for stale timeout.
        if is_deferred {
            if let Some(blocker_id) = self.latest_deferred_blocker_id(task) {
                if !self.deferred_blocker_is_active(&blocker_id).await {
                    return self
                        .apply_recovery_decision(
                            task,
                            status,
                            RecoveryContext::PendingMerge,
                            RecoveryDecision {
                                action: RecoveryActionKind::ExecuteEntryActions,
                                reason: Some(
                                    "Deferred merge blocker is no longer active — re-triggering."
                                        .to_string(),
                                ),
                            },
                        )
                        .await;
                }
            }
        }

        let evidence = RecoveryEvidence {
            run_status: None,
            registry_running: false,
            can_start: true,
            is_stale: age
                >= chrono::Duration::minutes(
                    reconciliation_config().pending_merge_stale_minutes as i64,
                ),
            is_deferred,
        };
        let decision = self
            .policy
            .decide_reconciliation(RecoveryContext::PendingMerge, evidence);

        self.apply_recovery_decision(task, status, RecoveryContext::PendingMerge, decision)
            .await
    }

    pub(crate) async fn reconcile_merge_incomplete_task(
        &self,
        task: &crate::domain::entities::Task,
        status: InternalStatus,
    ) -> bool {
        if status != InternalStatus::MergeIncomplete {
            return false;
        }

        // Skip retry when branch_missing flag is set - surface to user instead
        if has_branch_missing_metadata(task) {
            return false;
        }

        // Validation-in-progress guard: same as PendingMerge guard — validation may
        // still be running when the task transitions to MergeIncomplete (e.g., revert
        // completes but subprocess lingers). Skip until validation flag expires.
        if Self::has_validation_in_progress(task) {
            tracing::debug!(
                task_id = task.id.as_str(),
                "Skipping MergeIncomplete reconciliation — validation in progress"
            );
            return true;
        }

        // User-initiated retry guard: if retry_merge set the in-flight flag, the background
        // task will handle the transition — skip reconciliation to prevent stale-data races.
        if Self::has_merge_retry_in_progress(task) {
            tracing::debug!(
                task_id = task.id.as_str(),
                "Skipping MergeIncomplete reconciliation — user retry in progress"
            );
            return true;
        }

        // Rate limit guard: if a provider rate limit is active, skip retry until it expires.
        // Rate-limited skips do NOT count toward max retries — the retry budget is preserved.
        //
        // When the rate limit expires, we clear it from DB via clear_rate_limit_retry_after.
        // To prevent subsequent metadata writes (record_retry_metadata, record_merge_auto_retry_event)
        // from overwriting the cleared field using the stale `task` reference, we refresh the
        // task's metadata in-memory. We preserve the original updated_at to avoid resetting
        // the age used by the retry delay check.
        let rate_limit_cleared;
        let refreshed_task;
        let task = if let Some(retry_after) = Self::get_rate_limit_retry_after(task) {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&retry_after) {
                if chrono::Utc::now() < dt {
                    tracing::debug!(
                        task_id = task.id.as_str(),
                        retry_after = %retry_after,
                        "Skipping MergeIncomplete retry — provider rate limit active"
                    );
                    return false; // Don't retry, don't count toward budget
                }
                // Rate limit expired — clear it and proceed with normal retry logic
                if let Err(e) = self.clear_rate_limit_retry_after(task).await {
                    warn!(
                        task_id = task.id.as_str(),
                        error = %e,
                        "Failed to clear expired rate_limit_retry_after"
                    );
                }
                // Refresh metadata from DB so subsequent writes don't re-introduce the field,
                // but keep the original updated_at so age calculations remain stable.
                rate_limit_cleared = match self.task_repo.get_by_id(&task.id).await {
                    Ok(Some(mut refreshed)) => {
                        refreshed.updated_at = task.updated_at;
                        refreshed
                    }
                    _ => task.clone(),
                };
                &rate_limit_cleared
            } else {
                refreshed_task = task.clone();
                &refreshed_task
            }
        } else {
            task
        };

        // Smart retry guard: if the incomplete was explicitly reported by the agent,
        // it was a deliberate decision — do NOT auto-retry without human intervention.
        if Self::is_agent_reported_failure(task) {
            debug!(
                task_id = task.id.as_str(),
                "Skipping auto-retry of MergeIncomplete — agent explicitly reported this failure (AgentReported)"
            );
            return false;
        }

        // Loop-breaking guard: if validation has reverted the merge more than the configured max
        // times, stop auto-retrying and surface to user — the code changes must fix the failures.
        let revert_count = Self::validation_revert_count(task);
        if revert_count >= reconciliation_config().validation_revert_max_count as u32 {
            debug!(
                task_id = task.id.as_str(),
                revert_count = revert_count,
                max = reconciliation_config().validation_revert_max_count,
                "Stopping auto-retry of MergeIncomplete — validation revert loop detected (ValidationFailed)"
            );
            return false;
        }

        let is_validation = Self::is_validation_failure(task);

        // RC#4: Validation failure circuit breaker — after N consecutive validation failures,
        // stop auto-retrying entirely and leave for human intervention.
        if is_validation {
            let consecutive = Self::consecutive_validation_failures(task);
            if consecutive >= reconciliation_config().validation_failure_circuit_breaker_count as u32 {
                debug!(
                    task_id = task.id.as_str(),
                    consecutive = consecutive,
                    max = reconciliation_config().validation_failure_circuit_breaker_count,
                    "Circuit breaker: stopping auto-retry after consecutive validation failures"
                );
                return false;
            }
        }

        let age = match self.latest_status_transition_age(task, status).await {
            Some(age) => age,
            None => return false,
        };

        // RC#4: Validation failure cooldown — enforce minimum wait before retrying
        // after a validation failure to prevent rapid retry loops.
        if is_validation {
            let cooldown = chrono::Duration::seconds(
                reconciliation_config().validation_retry_min_cooldown_secs as i64,
            );
            if age < cooldown {
                debug!(
                    task_id = task.id.as_str(),
                    age_secs = age.num_seconds(),
                    cooldown_secs = reconciliation_config().validation_retry_min_cooldown_secs,
                    "Skipping MergeIncomplete retry — validation failure cooldown not elapsed"
                );
                return false;
            }
        }

        // RC#5: Starvation guard — if this task was retried recently, skip it this cycle
        // to give other MergeIncomplete tasks a turn.
        if let Some(last_retry) = Self::last_retried_at(task) {
            let since_retry = chrono::Utc::now() - last_retry;
            let guard_secs = reconciliation_config().merge_starvation_guard_secs as i64;
            if since_retry < chrono::Duration::seconds(guard_secs) {
                debug!(
                    task_id = task.id.as_str(),
                    since_retry_secs = since_retry.num_seconds(),
                    guard_secs = guard_secs,
                    "Skipping MergeIncomplete retry — starvation guard (recently retried)"
                );
                return false;
            }
        }

        let retry_count = Self::merge_incomplete_auto_retry_count(task);
        if retry_count >= reconciliation_config().merge_incomplete_max_retries as u32 {
            return false;
        }

        let retry_delay = Self::merge_incomplete_retry_delay(retry_count);
        if age < retry_delay {
            return false;
        }

        // Record retry metadata (last_retried_at + consecutive_validation_failures tracking)
        if let Err(e) = self.record_retry_metadata(task, is_validation).await {
            warn!(
                task_id = task.id.as_str(),
                error = %e,
                "Failed to record retry metadata (last_retried_at)"
            );
        }

        let attempt = retry_count + 1;
        if let Err(e) = self
            .record_merge_auto_retry_event(
                task,
                attempt,
                MergeFailureSource::TransientGit,
                "MergeIncomplete auto-retry — transient git failure",
            )
            .await
        {
            warn!(
                task_id = task.id.as_str(),
                attempt = attempt,
                error = %e,
                "Failed to record merge auto-retry metadata"
            );
        }

        warn!(
            task_id = task.id.as_str(),
            attempt = attempt,
            failure_source = "transient_git",
            "Auto-retrying MergeIncomplete — transient git failure, transitioning to PendingMerge"
        );

        match self
            .transition_service
            .transition_task(&task.id, InternalStatus::PendingMerge)
            .await
        {
            Ok(_) => true,
            Err(e) => {
                warn!(
                    task_id = task.id.as_str(),
                    error = %e,
                    "Failed to transition MergeIncomplete -> PendingMerge during recovery"
                );
                false
            }
        }
    }

    pub(crate) async fn reconcile_merge_conflict_task(
        &self,
        task: &crate::domain::entities::Task,
        status: InternalStatus,
    ) -> bool {
        if status != InternalStatus::MergeConflict {
            return false;
        }

        // Skip retry when branch_missing flag is set - surface to user instead
        if has_branch_missing_metadata(task) {
            return false;
        }

        // User-initiated retry guard: skip reconciliation while background retry is running.
        if Self::has_merge_retry_in_progress(task) {
            tracing::debug!(
                task_id = task.id.as_str(),
                "Skipping MergeConflict reconciliation — user retry in progress"
            );
            return true;
        }

        // Smart retry guard: if the conflict was explicitly reported by the agent,
        // the agent made a deliberate decision — do NOT auto-retry without human intervention.
        if Self::is_agent_reported_failure(task) {
            warn!(
                task_id = task.id.as_str(),
                "Skipping auto-retry of MergeConflict — agent explicitly reported this conflict (AgentReported)"
            );
            return false;
        }

        let age = match self.latest_status_transition_age(task, status).await {
            Some(age) => age,
            None => return false,
        };

        let retry_count = Self::merge_conflict_auto_retry_count(task);
        if retry_count >= reconciliation_config().merge_conflict_max_retries as u32 {
            return false;
        }

        let retry_delay = Self::merge_conflict_retry_delay(retry_count);
        if age < retry_delay {
            return false;
        }

        // SHA comparison guard: if source branch hasn't changed since last failure,
        // retrying the same commit will produce the same conflict — skip.
        let sha_changed = self.check_source_sha_changed(task).await;
        match sha_changed {
            ShaComparisonResult::Unchanged(sha) => {
                warn!(
                    task_id = task.id.as_str(),
                    source_sha = %sha,
                    "Skipping MergeConflict auto-retry — source branch SHA unchanged since last failure"
                );
                return false;
            }
            ShaComparisonResult::Changed { old_sha, new_sha } => {
                warn!(
                    task_id = task.id.as_str(),
                    old_sha = %old_sha,
                    new_sha = %new_sha,
                    "Source branch SHA changed since last failure — proceeding with retry"
                );
            }
            ShaComparisonResult::NoStoredSha | ShaComparisonResult::GitError => {
                // No SHA stored (first attempt) or git error — allow retry
            }
        }

        // Get current source SHA to record with the retry event
        let current_sha = self.get_current_source_sha(task).await;

        let attempt = retry_count + 1;
        let failure_source = MergeFailureSource::SystemDetected;
        if let Err(e) = self
            .record_merge_auto_retry_event_with_sha(
                task,
                attempt,
                failure_source,
                "MergeConflict auto-retry — system-detected conflict",
                current_sha.as_deref(),
            )
            .await
        {
            warn!(
                task_id = task.id.as_str(),
                attempt = attempt,
                error = %e,
                "Failed to record merge auto-retry metadata"
            );
        }

        warn!(
            task_id = task.id.as_str(),
            attempt = attempt,
            failure_source = "system_detected",
            source_sha = current_sha.as_deref().unwrap_or("unknown"),
            "Auto-retrying MergeConflict — system-detected conflict, transitioning to PendingMerge"
        );

        match self
            .transition_service
            .transition_task(&task.id, InternalStatus::PendingMerge)
            .await
        {
            Ok(_) => true,
            Err(e) => {
                warn!(
                    task_id = task.id.as_str(),
                    error = %e,
                    "Failed to transition MergeConflict -> PendingMerge during recovery"
                );
                false
            }
        }
    }
}
