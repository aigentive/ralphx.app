// Reusable helpers for cleanup steps and post-merge scheduling.
//
// - run_cleanup_step: timeout-wrapped async operation with standardized logging
// - spawn_schedule_after_settle: delayed task scheduling after merge/state settle

use std::sync::Arc;
use std::time::Duration;

use crate::domain::state_machine::services::TaskScheduler;

/// Run a cleanup step with a timeout, logging success/error/timeout uniformly.
///
/// Returns `true` if the operation succeeded, `false` on error or timeout.
/// All failures are non-fatal (logged as warn) to allow cleanup to continue.
pub(crate) async fn run_cleanup_step<F, E>(
    label: &str,
    timeout_secs: u64,
    task_id: &str,
    fut: F,
) -> bool
where
    F: std::future::Future<Output = Result<(), E>>,
    E: std::fmt::Display,
{
    match tokio::time::timeout(Duration::from_secs(timeout_secs), fut).await {
        Ok(Ok(())) => true,
        Ok(Err(e)) => {
            tracing::warn!(
                task_id = task_id,
                error = %e,
                step = label,
                "pre_merge_cleanup: {} failed (non-fatal, continuing)",
                label,
            );
            false
        }
        Err(_elapsed) => {
            tracing::warn!(
                task_id = task_id,
                step = label,
                timeout_secs = timeout_secs,
                "pre_merge_cleanup: {} timed out after {}s (non-fatal, continuing)",
                label,
                timeout_secs,
            );
            false
        }
    }
}

/// Spawn a delayed `try_schedule_ready_tasks` call after a settle period.
///
/// Used after merges and state transitions to give the system time to stabilize
/// before scheduling the next batch of ready tasks.
pub(crate) fn spawn_schedule_after_settle(scheduler: Arc<dyn TaskScheduler>, settle_ms: u64) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(settle_ms)).await;
        scheduler.try_schedule_ready_tasks().await;
    });
}
