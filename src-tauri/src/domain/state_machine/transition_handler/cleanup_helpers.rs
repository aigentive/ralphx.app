// Reusable helpers for pre-merge cleanup steps.
//
// - run_cleanup_step: timeout-wrapped async operation with standardized logging
// - CleanupStepResult: typed outcome replacing bare bool

use std::time::Duration;

/// Outcome of a single cleanup step — richer than a bare `bool`.
#[derive(Debug)]
pub(crate) enum CleanupStepResult {
    /// The operation completed successfully.
    Ok,
    /// The operation exceeded its deadline.
    TimedOut { elapsed: Duration },
    /// The operation returned an error.
    Error { message: String },
}

impl CleanupStepResult {
    /// Convenience predicate matching the old `bool` semantics.
    #[allow(dead_code)]
    pub fn is_ok(&self) -> bool {
        matches!(self, CleanupStepResult::Ok)
    }
}

/// Run a cleanup step with a timeout, logging success/error/timeout uniformly.
///
/// Returns a [`CleanupStepResult`] describing the outcome.
/// All failures are non-fatal (logged as warn) to allow cleanup to continue.
pub(crate) async fn run_cleanup_step<F, E>(
    label: &str,
    timeout_secs: u64,
    task_id: &str,
    fut: F,
) -> CleanupStepResult
where
    F: std::future::Future<Output = Result<(), E>>,
    E: std::fmt::Display,
{
    let deadline = Duration::from_secs(timeout_secs);
    match tokio::time::timeout(deadline, fut).await {
        Ok(Ok(())) => CleanupStepResult::Ok,
        Ok(Err(e)) => {
            let message = e.to_string();
            tracing::warn!(
                task_id = task_id,
                error = %message,
                step = label,
                "pre_merge_cleanup: {} failed (non-fatal, continuing)",
                label,
            );
            CleanupStepResult::Error { message }
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
            CleanupStepResult::TimedOut { elapsed: deadline }
        }
    }
}
