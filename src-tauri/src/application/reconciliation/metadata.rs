// Retry counters, SHA tracking, backoff delays for reconciliation.

use tauri::Runtime;
use tracing::warn;

use crate::application::GitService;
use crate::domain::entities::{
    InternalStatus, MergeFailureSource, MergeRecoveryEventKind, MergeRecoveryMetadata,
    MergeRecoveryState, Task,
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
