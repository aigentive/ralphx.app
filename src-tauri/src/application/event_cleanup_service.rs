// Event Cleanup Service
//
// Background service that periodically removes stale entries from the
// `external_events` table:
//   - entries older than 24 hours
//   - entries beyond the 10 000-row high-water mark per project
//
// Run via `EventCleanupService::run_loop()` in a `tokio::spawn` task.

use std::sync::Arc;
use std::time::Duration;

use crate::domain::repositories::ExternalEventsRepository;

/// Background service for pruning stale external_events rows.
pub struct EventCleanupService {
    repo: Arc<dyn ExternalEventsRepository>,
    /// How often to run the cleanup (default: 1 hour).
    interval: Duration,
}

impl EventCleanupService {
    pub fn new(repo: Arc<dyn ExternalEventsRepository>) -> Self {
        Self {
            repo,
            interval: Duration::from_secs(3600),
        }
    }

    /// Override the cleanup interval (useful for tests).
    #[allow(dead_code)]
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Run the cleanup loop indefinitely.
    ///
    /// Sleeps for `interval` between runs.  Logs errors but never panics.
    pub async fn run_loop(self) {
        loop {
            tokio::time::sleep(self.interval).await;
            match self.repo.cleanup_old_events().await {
                Ok(deleted) => {
                    if deleted > 0 {
                        tracing::info!(
                            deleted = deleted,
                            "EventCleanupService: pruned stale external_events rows"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "EventCleanupService: cleanup_old_events failed (non-fatal)"
                    );
                }
            }
        }
    }
}
