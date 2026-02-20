// Reusable helpers for pre-merge cleanup steps.
//
// - run_cleanup_step: timeout-wrapped async operation with standardized logging

use std::time::Duration;

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
