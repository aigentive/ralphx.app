// Retry counters, SHA tracking, backoff delays for reconciliation.

use tauri::Runtime;
use tracing::warn;

use crate::application::GitService;
use crate::domain::entities::{
    task_metadata::RetryStrategy, ExecutionFailureSource, ExecutionRecoveryEvent,
    ExecutionRecoveryEventKind, ExecutionRecoveryMetadata, ExecutionRecoveryReasonCode,
    ExecutionRecoverySource, ExecutionRecoveryState, InternalStatus, MergeFailureSource,
    MergeRecoveryEventKind, MergeRecoveryMetadata, MergeRecoveryState, Task,
};
use crate::infrastructure::agents::claude::reconciliation_config;

use super::policy::ShaComparisonResult;
use super::ReconciliationRunner;

impl<R: Runtime> ReconciliationRunner<R> {
    /// Count `AttemptFailed` events in merge recovery metadata (Merging state retries).
    pub(crate) fn merging_auto_retry_count(task: &Task) -> u32 {
        MergeRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
            .ok()
            .flatten()
            .map(|meta| {
                meta.events
                    .iter()
                    .filter(|e| matches!(e.kind, MergeRecoveryEventKind::AttemptFailed))
                    .count() as u32
            })
            .unwrap_or(0)
    }

    /// Count auto-retry events for a given status in task metadata.
    /// Uses a metadata key like "auto_retry_count_{status}" to track retries.
    pub(crate) fn auto_retry_count_for_status(task: &Task, status: InternalStatus) -> u32 {
        let metadata = match task.metadata.as_deref() {
            Some(m) => m,
            None => return 0,
        };
        let json: serde_json::Value = match serde_json::from_str(metadata) {
            Ok(v) => v,
            Err(_) => return 0,
        };
        let key = format!("auto_retry_count_{}", status);
        json.get(&key).and_then(|v| v.as_u64()).unwrap_or(0) as u32
    }

    /// Record an auto-retry attempt in task metadata.
    pub(crate) async fn record_auto_retry_metadata(
        &self,
        task: &Task,
        status: InternalStatus,
        attempt: u32,
    ) -> Result<(), String> {
        let mut updated = task.clone();
        let mut json: serde_json::Value = updated
            .metadata
            .as_deref()
            .and_then(|m| serde_json::from_str(m).ok())
            .unwrap_or_else(|| serde_json::json!({}));

        let key = format!("auto_retry_count_{}", status);
        if let Some(obj) = json.as_object_mut() {
            obj.insert(key, serde_json::json!(attempt));
        }

        updated.metadata = Some(json.to_string());
        updated.touch();
        self.task_repo
            .update(&updated)
            .await
            .map_err(|e| e.to_string())
    }

    pub(crate) fn merge_incomplete_auto_retry_count(task: &Task) -> u32 {
        MergeRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
            .ok()
            .flatten()
            .map(|meta| {
                meta.events
                    .iter()
                    .filter(|e| matches!(e.kind, MergeRecoveryEventKind::AutoRetryTriggered))
                    .count() as u32
            })
            .unwrap_or(0)
    }

    pub(crate) fn merge_incomplete_retry_delay(retry_count: u32) -> chrono::Duration {
        use rand::Rng;
        let exponent = retry_count.min(6);
        let scaled = (reconciliation_config().merge_incomplete_retry_base_secs as i64)
            .saturating_mul(1_i64 << exponent);
        let base_delay = scaled.min(reconciliation_config().merge_incomplete_retry_max_secs as i64);
        let jitter = rand::thread_rng().gen_range(0..=base_delay / 4);
        chrono::Duration::seconds(base_delay + jitter)
    }

    pub(crate) fn merge_conflict_auto_retry_count(task: &Task) -> u32 {
        MergeRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
            .ok()
            .flatten()
            .map(|meta| {
                meta.events
                    .iter()
                    .filter(|e| matches!(e.kind, MergeRecoveryEventKind::AutoRetryTriggered))
                    .count() as u32
            })
            .unwrap_or(0)
    }

    pub(crate) fn merge_conflict_retry_delay(retry_count: u32) -> chrono::Duration {
        use rand::Rng;
        let exponent = retry_count.min(6);
        let scaled = (reconciliation_config().merge_conflict_retry_base_secs as i64)
            .saturating_mul(1_i64 << exponent);
        let base_delay = scaled.min(reconciliation_config().merge_conflict_retry_max_secs as i64);
        let jitter = rand::thread_rng().gen_range(0..=base_delay / 4);
        chrono::Duration::seconds(base_delay + jitter)
    }

    /// Returns true if the task's failure was explicitly reported by the merger agent
    /// (via report_conflict or report_incomplete endpoints). These should NOT be auto-retried
    /// because the agent made a deliberate decision requiring human intervention.
    pub(crate) fn is_agent_reported_failure(task: &Task) -> bool {
        task.metadata
            .as_deref()
            .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
            .and_then(|v| v.get("merge_failure_source").cloned())
            .and_then(|v| serde_json::from_value::<MergeFailureSource>(v).ok())
            .map(|source| matches!(source, MergeFailureSource::AgentReported))
            .unwrap_or(false)
    }

    /// Returns true if a user-initiated merge retry is currently in progress.
    /// The flag is a RFC3339 timestamp; it auto-expires after 60 s to prevent stuck state
    /// if the background task panics before clearing the guard.
    pub(crate) fn has_merge_retry_in_progress(task: &Task) -> bool {
        task.metadata
            .as_deref()
            .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
            .and_then(|v| {
                let val = v.get("merge_retry_in_progress")?;
                // Only valid timestamps < 60s old count as active.
                // Legacy boolean `true` or other non-string values have no timestamp
                // and cannot be verified as fresh — treat as stale.
                let ts = val.as_str()?;
                let started = chrono::DateTime::parse_from_rfc3339(ts).ok()?;
                let age = chrono::Utc::now() - started.with_timezone(&chrono::Utc);
                Some(age < chrono::Duration::seconds(60))
            })
            .unwrap_or(false)
    }

    /// Returns true if post-merge validation commands are currently running.
    /// The flag is an RFC3339 timestamp; it auto-expires after validation_deadline_secs
    /// to prevent stuck state if the validation pipeline crashes without clearing the flag.
    pub(crate) fn has_validation_in_progress(task: &Task) -> bool {
        task.metadata
            .as_deref()
            .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
            .and_then(|v| {
                let ts = v.get("validation_in_progress")?.as_str()?;
                let started = chrono::DateTime::parse_from_rfc3339(ts).ok()?;
                let age = chrono::Utc::now() - started.with_timezone(&chrono::Utc);
                // Use validation_deadline_secs as staleness threshold (same timeout
                // that governs the validation pipeline itself)
                let deadline_secs = reconciliation_config().validation_deadline_secs;
                Some(age < chrono::Duration::seconds(deadline_secs as i64))
            })
            .unwrap_or(false)
    }

    /// Returns true if the merge pipeline (attempt_programmatic_merge) is actively running.
    /// Reads from the dedicated `merge_pipeline_active` column (RFC3339 timestamp).
    /// Auto-expires after attempt_merge_deadline_secs as a crash safety net.
    pub(crate) fn has_merge_pipeline_active(task: &Task) -> bool {
        task.merge_pipeline_active
            .as_deref()
            .and_then(|ts| {
                let started = chrono::DateTime::parse_from_rfc3339(ts).ok()?;
                let age = chrono::Utc::now() - started.with_timezone(&chrono::Utc);
                let deadline_secs = reconciliation_config().attempt_merge_deadline_secs;
                Some(age < chrono::Duration::seconds(deadline_secs as i64))
            })
            .unwrap_or(false)
    }

    /// Returns the number of times post-merge validation has reverted the merge commit.
    /// Used to break validation→revert→retry loops.
    pub(crate) fn validation_revert_count(task: &Task) -> u32 {
        task.metadata
            .as_deref()
            .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
            .and_then(|v| {
                v.get("validation_revert_count")
                    .and_then(|c| c.as_u64())
                    .map(|c| c as u32)
            })
            .unwrap_or(0)
    }

    /// Returns true if the task's failure was caused by post-merge validation.
    pub(crate) fn is_validation_failure(task: &Task) -> bool {
        task.metadata
            .as_deref()
            .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
            .and_then(|v| v.get("merge_failure_source").cloned())
            .and_then(|v| serde_json::from_value::<MergeFailureSource>(v).ok())
            .map(|source| matches!(source, MergeFailureSource::ValidationFailed))
            .unwrap_or(false)
    }

    /// Count consecutive validation failures (stored in task metadata).
    pub(crate) fn consecutive_validation_failures(task: &Task) -> u32 {
        task.metadata
            .as_deref()
            .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
            .and_then(|v| v.get("consecutive_validation_failures").and_then(|c| c.as_u64()).map(|c| c as u32))
            .unwrap_or(0)
    }

    /// Get the last_retried_at timestamp from task metadata, if set.
    pub(crate) fn last_retried_at(task: &Task) -> Option<chrono::DateTime<chrono::Utc>> {
        task.metadata
            .as_deref()
            .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
            .and_then(|v| v.get("last_retried_at")?.as_str().map(String::from))
            .and_then(|ts| chrono::DateTime::parse_from_rfc3339(&ts).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
    }

    /// Record last_retried_at timestamp and increment consecutive_validation_failures if applicable.
    pub(crate) async fn record_retry_metadata(
        &self,
        task: &Task,
        is_validation: bool,
    ) -> Result<(), String> {
        let mut updated = task.clone();
        let mut json: serde_json::Value = updated
            .metadata
            .as_deref()
            .and_then(|m| serde_json::from_str(m).ok())
            .unwrap_or_else(|| serde_json::json!({}));

        if let Some(obj) = json.as_object_mut() {
            obj.insert(
                "last_retried_at".to_string(),
                serde_json::json!(chrono::Utc::now().to_rfc3339()),
            );
            if is_validation {
                let prev = obj
                    .get("consecutive_validation_failures")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                obj.insert(
                    "consecutive_validation_failures".to_string(),
                    serde_json::json!(prev + 1),
                );
            } else {
                // Non-validation retry resets the counter
                obj.insert(
                    "consecutive_validation_failures".to_string(),
                    serde_json::json!(0),
                );
            }
        }

        updated.metadata = Some(json.to_string());
        updated.touch();
        self.task_repo
            .update(&updated)
            .await
            .map_err(|e| e.to_string())
    }

    /// Returns true if the task has mode_switch=true in metadata (AD12).
    /// Set by handle_pr_mode_switch when toggling PR→push-to-main mid-Merging.
    /// Used by reconcile_merge_incomplete_task to bypass all guards and retry immediately.
    pub(crate) fn is_mode_switch(task: &Task) -> bool {
        task.metadata
            .as_deref()
            .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
            .and_then(|v| v.get("mode_switch").and_then(|v| v.as_bool()))
            .unwrap_or(false)
    }

    /// Returns true if the circuit breaker has been triggered, preventing auto-retry.
    /// The circuit breaker is cleared when the user manually retries.
    pub(crate) fn is_circuit_breaker_active(task: &Task) -> bool {
        MergeRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
            .ok()
            .flatten()
            .map(|meta| meta.circuit_breaker_active)
            .unwrap_or(false)
    }

    /// Check if the circuit breaker should fire based on recent failure patterns.
    /// Returns Some(reason) if threshold+ of the last window failure events share the same source.
    /// Returns None if the circuit breaker should not fire.
    pub(crate) fn should_circuit_break(
        task: &Task,
        threshold: usize,
        window: usize,
    ) -> Option<String> {
        let metadata = MergeRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
            .ok()
            .flatten()?;

        // Filter to failure-type events with classified failure_source that would be auto-retried.
        // Only auto-retryable sources (e.g. WorktreeMissing, TransientGit) can cause infinite loops;
        // NoAutomaticRetry sources (AgentReported, ValidationFailed) already stop themselves.
        let failure_events: Vec<_> = metadata
            .events
            .iter()
            .filter(|e| {
                matches!(
                    e.kind,
                    MergeRecoveryEventKind::AttemptFailed
                        | MergeRecoveryEventKind::AutoRetryTriggered
                        | MergeRecoveryEventKind::Deferred
                ) && e
                    .failure_source
                    .as_ref()
                    .map(|s| s.retry_strategy() == RetryStrategy::AutoRetry)
                    .unwrap_or(false)
            })
            .collect();

        // Take the last `window` failure events
        let recent = if failure_events.len() > window {
            &failure_events[failure_events.len() - window..]
        } else {
            &failure_events[..]
        };

        if recent.len() < threshold {
            return None; // Not enough classified events
        }

        // Count occurrences of each failure_source variant
        use std::collections::HashMap;
        let mut counts: HashMap<String, usize> = HashMap::new();
        for event in recent {
            if let Some(source) = &event.failure_source {
                let key =
                    serde_json::to_string(source).unwrap_or_else(|_| "unknown".to_string());
                *counts.entry(key).or_insert(0) += 1;
            }
        }

        // Check if any variant hits the threshold
        for (source_key, count) in &counts {
            if *count >= threshold {
                return Some(format!(
                    "Circuit breaker: {}/{} recent failures share the same source ({})",
                    count, window, source_key
                ));
            }
        }

        None
    }

    /// Update task metadata to set circuit_breaker_active=true and reason.
    /// Uses the MergeRecoveryMetadata structured update path.
    pub(crate) async fn update_circuit_breaker_metadata(
        &self,
        task: &Task,
        reason: &str,
    ) -> Result<(), String> {
        let mut recovery = MergeRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
            .unwrap_or(None)
            .unwrap_or_default();

        recovery.circuit_breaker_active = true;
        recovery.circuit_breaker_reason = Some(reason.to_string());

        let updated_metadata = recovery
            .update_task_metadata(task.metadata.as_deref())
            .map_err(|e| e.to_string())?;

        self.task_repo
            .update_metadata(&task.id, Some(updated_metadata))
            .await
            .map_err(|e| e.to_string())
    }

    /// Get rate_limit_retry_after from merge recovery metadata, if set.
    pub(crate) fn get_rate_limit_retry_after(task: &Task) -> Option<String> {
        MergeRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
            .ok()
            .flatten()
            .and_then(|meta| meta.rate_limit_retry_after)
    }

    /// Clear the rate_limit_retry_after field from merge recovery metadata (after expiry).
    pub(crate) async fn clear_rate_limit_retry_after(&self, task: &Task) -> Result<(), String> {
        let mut updated = task.clone();
        let mut recovery = MergeRecoveryMetadata::from_task_metadata(updated.metadata.as_deref())
            .unwrap_or(None)
            .unwrap_or_default();

        recovery.rate_limit_retry_after = None;
        if recovery.last_state == MergeRecoveryState::RateLimited {
            recovery.last_state = MergeRecoveryState::Retrying;
        }

        updated.metadata = Some(
            recovery
                .update_task_metadata(updated.metadata.as_deref())
                .map_err(|e| e.to_string())?,
        );
        updated.touch();

        self.task_repo
            .update(&updated)
            .await
            .map_err(|e| e.to_string())
    }

    /// Get the last stored source branch SHA from merge recovery events.
    pub(crate) fn last_stored_source_sha(task: &Task) -> Option<String> {
        MergeRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
            .ok()
            .flatten()
            .and_then(|meta| meta.events.iter().rev().find_map(|e| e.source_sha.clone()))
    }

    /// Get the current HEAD SHA of the task's source branch using GitService.
    pub(crate) async fn get_current_source_sha(&self, task: &Task) -> Option<String> {
        let branch = task.task_branch.as_deref()?;
        let project_data = self.project_repo.get_by_id(&task.project_id).await.ok()??;
        let repo_path = std::path::Path::new(&project_data.working_directory);
        match GitService::get_branch_sha(repo_path, branch).await {
            Ok(sha) => Some(sha),
            Err(e) => {
                warn!(
                    task_id = task.id.as_str(),
                    branch = branch,
                    error = %e,
                    "Failed to get current source branch SHA for comparison guard"
                );
                None
            }
        }
    }

    /// Compare the source branch SHA at last failure time with the current SHA.
    pub(crate) async fn check_source_sha_changed(&self, task: &Task) -> ShaComparisonResult {
        let last_sha = match Self::last_stored_source_sha(task) {
            Some(sha) => sha,
            None => return ShaComparisonResult::NoStoredSha,
        };

        let current_sha = match self.get_current_source_sha(task).await {
            Some(sha) => sha,
            None => return ShaComparisonResult::GitError,
        };

        if current_sha == last_sha {
            ShaComparisonResult::Unchanged(current_sha)
        } else {
            ShaComparisonResult::Changed {
                old_sha: last_sha,
                new_sha: current_sha,
            }
        }
    }

}

// ── Execution Recovery Helpers ────────────────────────────────────────────────
// Called by reconcile_failed_execution_task() (Wave 3 handler in execution.rs).
impl<R: Runtime> ReconciliationRunner<R> {
    /// Count `AutoRetryTriggered` events in execution recovery metadata.
    pub(crate) fn execution_failed_auto_retry_count(task: &Task) -> u32 {
        ExecutionRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
            .ok()
            .flatten()
            .map(|meta| {
                meta.events
                    .iter()
                    .filter(|e| matches!(e.kind, ExecutionRecoveryEventKind::AutoRetryTriggered))
                    .count() as u32
            })
            .unwrap_or(0)
    }

    /// Exponential backoff with jitter for failed execution retries.
    /// Formula: `min(2^retry_count * base_secs, max_secs) + random jitter (0–25%)`.
    /// Mirrors `merge_incomplete_retry_delay()`.
    pub(crate) fn execution_failed_retry_delay(retry_count: u32) -> chrono::Duration {
        use rand::Rng;
        let exponent = retry_count.min(6);
        let scaled = (reconciliation_config().execution_failed_retry_base_secs as i64)
            .saturating_mul(1_i64 << exponent);
        let base_delay =
            scaled.min(reconciliation_config().execution_failed_retry_max_secs as i64);
        let jitter = rand::thread_rng().gen_range(0..=base_delay / 4);
        chrono::Duration::seconds(base_delay + jitter)
    }

    /// Returns true if the task's execution recovery has `stop_retrying` set.
    #[allow(dead_code)]
    pub(crate) fn has_execution_stop_retrying(task: &Task) -> bool {
        ExecutionRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
            .ok()
            .flatten()
            .map(|meta| meta.stop_retrying)
            .unwrap_or(false)
    }

    /// Append an `AutoRetryTriggered` event to the task's execution recovery metadata.
    /// Uses targeted `update_metadata()` SQL path to prevent metadata write races (GAP H7).
    pub(crate) async fn record_execution_auto_retry_event(
        &self,
        task: &Task,
        attempt: u32,
        failure_source: ExecutionFailureSource,
        message: impl Into<String>,
    ) -> Result<(), String> {
        let mut recovery =
            ExecutionRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
                .unwrap_or(None)
                .unwrap_or_default();

        let reason_code = Self::failure_source_to_reason_code(failure_source);
        let event = ExecutionRecoveryEvent::new(
            ExecutionRecoveryEventKind::AutoRetryTriggered,
            ExecutionRecoverySource::Auto,
            reason_code,
            message,
        )
        .with_attempt(attempt)
        .with_failure_source(failure_source);

        recovery.append_event_with_state(event, ExecutionRecoveryState::Retrying);

        let updated_metadata = recovery
            .update_task_metadata(task.metadata.as_deref())
            .map_err(|e| e.to_string())?;

        self.task_repo
            .update_metadata(&task.id, Some(updated_metadata))
            .await
            .map_err(|e| e.to_string())
    }

    /// Set `stop_retrying = true` in the task's execution recovery metadata.
    /// Appends a `StopRetrying` event and transitions state to `Failed`.
    /// Uses targeted `update_metadata()` SQL path (GAP H7).
    pub(crate) async fn set_execution_stop_retrying(&self, task: &Task) -> Result<(), String> {
        let mut recovery =
            ExecutionRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
                .unwrap_or(None)
                .unwrap_or_default();

        let event = ExecutionRecoveryEvent::new(
            ExecutionRecoveryEventKind::StopRetrying,
            ExecutionRecoverySource::System,
            ExecutionRecoveryReasonCode::MaxRetriesExceeded,
            "Max retries exceeded — stopping auto-retry",
        );

        recovery.stop_retrying = true;
        recovery.append_event_with_state(event, ExecutionRecoveryState::Failed);

        let updated_metadata = recovery
            .update_task_metadata(task.metadata.as_deref())
            .map_err(|e| e.to_string())?;

        self.task_repo
            .update_metadata(&task.id, Some(updated_metadata))
            .await
            .map_err(|e| e.to_string())
    }

    /// Compute the earliest time the next retry should be attempted.
    /// Returns the timestamp of the last `AutoRetryTriggered` event plus the
    /// backoff delay for the current retry count. Returns `None` if no retry
    /// events exist yet (GAPs M6, M8).
    pub(crate) fn execution_next_retry_at(
        task: &Task,
    ) -> Option<chrono::DateTime<chrono::Utc>> {
        let recovery = ExecutionRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
            .ok()
            .flatten()?;

        let last_retry_event = recovery
            .events
            .iter()
            .rev()
            .find(|e| matches!(e.kind, ExecutionRecoveryEventKind::AutoRetryTriggered))?;

        let retry_count = recovery
            .events
            .iter()
            .filter(|e| matches!(e.kind, ExecutionRecoveryEventKind::AutoRetryTriggered))
            .count() as u32;

        let delay = Self::execution_failed_retry_delay(retry_count);
        Some(last_retry_event.at + delay)
    }

    /// Remove stale flat failure metadata keys (`is_timeout`, `failure_error`) from task metadata.
    /// Preserves structured `execution_recovery` metadata intact.
    /// Uses targeted `update_metadata()` SQL path (GAPs B7, H7).
    pub(crate) async fn clear_execution_flat_metadata(&self, task: &Task) -> Result<(), String> {
        let mut json: serde_json::Value = task
            .metadata
            .as_deref()
            .and_then(|m| serde_json::from_str(m).ok())
            .unwrap_or_else(|| serde_json::json!({}));

        if let Some(obj) = json.as_object_mut() {
            obj.remove("is_timeout");
            obj.remove("failure_error");
        }

        self.task_repo
            .update_metadata(&task.id, Some(json.to_string()))
            .await
            .map_err(|e| e.to_string())
    }

    /// Reset execution recovery metadata for a manual restart.
    /// Clears event log, sets `last_state: Retrying`, sets `stop_retrying: false`.
    /// Gives the user a fresh retry budget after manual intervention (GAP H9).
    /// Uses targeted `update_metadata()` SQL path (GAP H7).
    /// Used by Wave 4 (apply_user_recovery_action).
    pub(crate) async fn reset_execution_recovery_metadata(
        &self,
        task: &Task,
    ) -> Result<(), String> {
        let mut recovery =
            ExecutionRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
                .unwrap_or(None)
                .unwrap_or_default();

        recovery.events.clear();
        recovery.last_state = ExecutionRecoveryState::Retrying;
        recovery.stop_retrying = false;

        let updated_metadata = recovery
            .update_task_metadata(task.metadata.as_deref())
            .map_err(|e| e.to_string())?;

        // Also clear stale flat keys (is_timeout, failure_error) — prevents
        // a subsequent clear_execution_flat_metadata call from re-reading the
        // stale task object and overwriting this write (GAP B7).
        let mut json: serde_json::Value = serde_json::from_str(&updated_metadata)
            .unwrap_or_else(|_| serde_json::json!({}));
        if let Some(obj) = json.as_object_mut() {
            obj.remove("is_timeout");
            obj.remove("failure_error");
        }

        self.task_repo
            .update_metadata(&task.id, Some(json.to_string()))
            .await
            .map_err(|e| e.to_string())
    }

    /// Set `stop_retrying = true` with User source — user explicitly cancelled auto-retry.
    /// Task remains Failed permanently (GAP H2, Cancel action).
    /// Uses targeted `update_metadata()` SQL path (GAP H7).
    pub(crate) async fn stop_execution_retrying_by_user(&self, task: &Task) -> Result<(), String> {
        let mut recovery =
            ExecutionRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
                .unwrap_or(None)
                .unwrap_or_default();

        let event = ExecutionRecoveryEvent::new(
            ExecutionRecoveryEventKind::StopRetrying,
            ExecutionRecoverySource::User,
            ExecutionRecoveryReasonCode::UserStopped,
            "User cancelled auto-retry — task will remain Failed",
        );

        recovery.stop_retrying = true;
        recovery.append_event_with_state(event, ExecutionRecoveryState::Failed);

        let updated_metadata = recovery
            .update_task_metadata(task.metadata.as_deref())
            .map_err(|e| e.to_string())?;

        self.task_repo
            .update_metadata(&task.id, Some(updated_metadata))
            .await
            .map_err(|e| e.to_string())
    }

    /// Record a `ManualRetry` event in execution recovery metadata.
    /// Used when the user manually restarts a Failed task (GAP H2, Restart action).
    /// Uses targeted `update_metadata()` SQL path (GAP H7).
    pub(crate) async fn record_execution_manual_retry_event(
        &self,
        task: &Task,
    ) -> Result<(), String> {
        let mut recovery =
            ExecutionRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
                .unwrap_or(None)
                .unwrap_or_default();

        let event = ExecutionRecoveryEvent::new(
            ExecutionRecoveryEventKind::ManualRetry,
            ExecutionRecoverySource::User,
            ExecutionRecoveryReasonCode::Unknown,
            "User manually restarted task execution",
        );

        recovery.append_event_with_state(event, ExecutionRecoveryState::Retrying);

        let updated_metadata = recovery
            .update_task_metadata(task.metadata.as_deref())
            .map_err(|e| e.to_string())?;

        self.task_repo
            .update_metadata(&task.id, Some(updated_metadata))
            .await
            .map_err(|e| e.to_string())
    }

    /// Returns true if `recover_timeout_failures()` processed this task within the last 60s.
    /// Used by the reconciler loop to skip tasks already handled at startup (GAP M5 sentinel).
    pub(crate) fn has_recent_startup_recovery(task: &Task) -> bool {
        let recovery =
            match ExecutionRecoveryMetadata::from_task_metadata(task.metadata.as_deref()) {
                Ok(Some(r)) => r,
                _ => return false,
            };
        let threshold = chrono::Utc::now() - chrono::Duration::seconds(60);
        recovery.events.iter().any(|e| {
            matches!(e.source, ExecutionRecoverySource::Startup) && e.at >= threshold
        })
    }

    /// Append an `AutoRetryTriggered` event with `Startup` source to the task's execution recovery metadata.
    /// Also creates an initial `Failed` event if no recovery metadata exists yet (legacy migration — GAP M2).
    /// Uses targeted `update_metadata()` SQL path to prevent metadata write races (GAP H7).
    pub(crate) async fn record_execution_startup_retry_event(
        &self,
        task: &Task,
        attempt: u32,
    ) -> Result<(), String> {
        let mut recovery =
            ExecutionRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
                .unwrap_or(None)
                .unwrap_or_default();

        // For legacy tasks (no prior execution_recovery), create an initial Failed event
        // to record the historical failure before migrating to the new format.
        if recovery.events.is_empty() {
            let failed_event = ExecutionRecoveryEvent::new(
                ExecutionRecoveryEventKind::Failed,
                ExecutionRecoverySource::System,
                ExecutionRecoveryReasonCode::Timeout,
                "Legacy timeout failure — migrated to structured recovery metadata",
            )
            .with_failure_source(ExecutionFailureSource::TransientTimeout);
            recovery.append_event(failed_event);
        }

        let retry_event = ExecutionRecoveryEvent::new(
            ExecutionRecoveryEventKind::AutoRetryTriggered,
            ExecutionRecoverySource::Startup,
            ExecutionRecoveryReasonCode::Timeout,
            format!("Startup recovery: re-queuing timeout-failed task (attempt {attempt})"),
        )
        .with_attempt(attempt)
        .with_failure_source(ExecutionFailureSource::TransientTimeout);

        recovery.append_event_with_state(retry_event, ExecutionRecoveryState::Retrying);

        let updated_metadata = recovery
            .update_task_metadata(task.metadata.as_deref())
            .map_err(|e| e.to_string())?;

        self.task_repo
            .update_metadata(&task.id, Some(updated_metadata))
            .await
            .map_err(|e| e.to_string())
    }

    /// Map `ExecutionFailureSource` to the corresponding `ExecutionRecoveryReasonCode`.
    fn failure_source_to_reason_code(
        source: ExecutionFailureSource,
    ) -> ExecutionRecoveryReasonCode {
        match source {
            ExecutionFailureSource::TransientTimeout => ExecutionRecoveryReasonCode::Timeout,
            ExecutionFailureSource::ParseStall => ExecutionRecoveryReasonCode::ParseStall,
            ExecutionFailureSource::AgentCrash => ExecutionRecoveryReasonCode::AgentExit,
            ExecutionFailureSource::ProviderError => ExecutionRecoveryReasonCode::ProviderError,
            ExecutionFailureSource::WallClockTimeout => {
                ExecutionRecoveryReasonCode::WallClockExceeded
            }
            ExecutionFailureSource::Unknown => ExecutionRecoveryReasonCode::Unknown,
        }
    }
}
