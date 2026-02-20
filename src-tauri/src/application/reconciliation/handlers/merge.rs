// Merge-specific reconciliation handlers: Merging, PendingMerge, MergeIncomplete, MergeConflict.

use tauri::Runtime;
use tracing::warn;

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
        evidence.is_stale = effective_age >= chrono::Duration::seconds(reconciliation_config().merger_timeout_secs as i64);

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

        // Gap 2: Don't re-spawn agent if one is still running in registry
        if decision.action == RecoveryActionKind::ExecuteEntryActions && evidence.registry_running {
            warn!(
                task_id = task.id.as_str(),
                "Skipping merger agent re-spawn — agent still running in registry"
            );
            return false;
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
                                "Main merge deferred while agents running — now retrying.".to_string(),
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
            is_stale: age >= chrono::Duration::minutes(reconciliation_config().pending_merge_stale_minutes as i64),
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

        // Rate limit guard: if a provider rate limit is active, skip retry until it expires.
        // Rate-limited skips do NOT count toward max retries — the retry budget is preserved.
        if let Some(retry_after) = Self::get_rate_limit_retry_after(task) {
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
            }
        }

        // Smart retry guard: if the incomplete was explicitly reported by the agent,
        // it was a deliberate decision — do NOT auto-retry without human intervention.
        if Self::is_agent_reported_failure(task) {
            warn!(
                task_id = task.id.as_str(),
                "Skipping auto-retry of MergeIncomplete — agent explicitly reported this failure (AgentReported)"
            );
            return false;
        }

        // Loop-breaking guard: if validation has reverted the merge more than the configured max
        // times, stop auto-retrying and surface to user — the code changes must fix the failures.
        let revert_count = Self::validation_revert_count(task);
        if revert_count > reconciliation_config().validation_revert_max_count as u32 {
            warn!(
                task_id = task.id.as_str(),
                revert_count = revert_count,
                max = reconciliation_config().validation_revert_max_count,
                "Stopping auto-retry of MergeIncomplete — validation revert loop detected (ValidationFailed)"
            );
            return false;
        }

        let age = match self.latest_status_transition_age(task, status).await {
            Some(age) => age,
            None => return false,
        };

        let retry_count = Self::merge_incomplete_auto_retry_count(task);
        if retry_count >= reconciliation_config().merge_incomplete_max_retries as u32 {
            return false;
        }

        let retry_delay = Self::merge_incomplete_retry_delay(retry_count);
        if age < retry_delay {
            return false;
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
