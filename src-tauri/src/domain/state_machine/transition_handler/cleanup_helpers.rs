// Reusable helpers for pre-merge cleanup steps.
//
// - os_thread_timeout: OS-thread-based timeout immune to tokio timer driver starvation
// - run_cleanup_step: timeout-wrapped async operation with standardized logging
// - CleanupStepResult: typed outcome replacing bare bool

use std::time::Duration;

/// Returned when an [`os_thread_timeout`] expires.
#[derive(Debug)]
pub(crate) struct OsTimeoutElapsed;

/// Race `fut` against an OS-thread-based deadline.
///
/// Unlike `tokio::time::timeout`, this is immune to tokio timer driver
/// starvation. The deadline runs on a real OS thread via `std::thread::sleep`,
/// signalling back through a `oneshot` channel (which uses `Waker`, not the
/// timer driver).
///
/// When the future wins, the OS thread remains sleeping until its duration
/// expires (then exits cleanly). This is acceptable for our timeout durations.
pub(crate) async fn os_thread_timeout<F, T>(
    duration: Duration,
    fut: F,
) -> Result<T, OsTimeoutElapsed>
where
    F: std::future::Future<Output = T>,
{
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();

    std::thread::spawn(move || {
        std::thread::sleep(duration);
        let _ = tx.send(());
    });

    tokio::pin!(fut);

    tokio::select! {
        result = &mut fut => Ok(result),
        _ = rx => Err(OsTimeoutElapsed),
    }
}

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
/// Uses [`os_thread_timeout`] internally — immune to tokio timer driver starvation.
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
    match os_thread_timeout(deadline, fut).await {
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
        Err(OsTimeoutElapsed) => {
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
