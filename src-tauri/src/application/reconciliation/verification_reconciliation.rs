// Verification reconciliation service
//
// Detects and resets sessions stuck in `verification_in_progress=1` after a configurable
// timeout (default: 90 min, D14). Runs on startup and periodically every 5 min.
//
// Stuck sessions occur when:
// - The orchestrator agent crashes mid-verification loop
// - Session recovery fails after a verification crash
// - The verification agent hangs indefinitely
//
// The reconciler FORCE-resets stuck sessions via `update_verification_state()` (unconditional),
// NOT via `reset_verification()` which guards on `in_progress=false` and is only for
// conditional resets triggered by plan artifact updates.

use std::sync::Arc;

use chrono::Utc;

use crate::domain::entities::VerificationStatus;
use crate::domain::repositories::IdeationSessionRepository;

/// Configuration for the verification reconciliation service.
#[derive(Debug, Clone, Copy)]
pub struct VerificationReconciliationConfig {
    /// Sessions stuck in `verification_in_progress=1` for longer than this are reset.
    pub stale_after_secs: u64,
    /// How often to scan for stuck sessions (seconds).
    pub interval_secs: u64,
}

impl Default for VerificationReconciliationConfig {
    fn default() -> Self {
        Self {
            stale_after_secs: 5400, // 90 minutes (D14)
            interval_secs: 300,     // 5 minutes
        }
    }
}

/// Service that detects and resets stuck verification sessions.
pub struct VerificationReconciliationService {
    ideation_session_repo: Arc<dyn IdeationSessionRepository>,
    config: VerificationReconciliationConfig,
}

impl VerificationReconciliationService {
    pub fn new(
        ideation_session_repo: Arc<dyn IdeationSessionRepository>,
        config: VerificationReconciliationConfig,
    ) -> Self {
        Self {
            ideation_session_repo,
            config,
        }
    }

    /// Scan for stuck sessions and reset them. Called on startup and periodically.
    ///
    /// Returns the number of sessions reset.
    pub async fn scan_and_reset(&self) -> u32 {
        let stale_before = Utc::now()
            - chrono::Duration::seconds(self.config.stale_after_secs as i64);

        let stale_sessions = match self
            .ideation_session_repo
            .get_stale_in_progress_sessions(stale_before)
            .await
        {
            Ok(sessions) => sessions,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "Failed to query stale verification sessions"
                );
                return 0;
            }
        };

        let mut reset_count = 0u32;
        for session in &stale_sessions {
            // Force-reset via update_verification_state (unconditional).
            // reset_verification() guards on in_progress=false and is only for
            // conditional resets on plan artifact updates — not for crash recovery.
            match self
                .ideation_session_repo
                .update_verification_state(
                    &session.id,
                    VerificationStatus::Unverified,
                    false,
                    None,
                )
                .await
            {
                Ok(()) => {
                    tracing::info!(
                        session_id = %session.id.as_str(),
                        stale_after_secs = self.config.stale_after_secs,
                        "Reconciliation reset stuck verification"
                    );
                    reset_count += 1;
                }
                Err(e) => {
                    tracing::warn!(
                        session_id = %session.id.as_str(),
                        error = %e,
                        "Failed to reset stuck verification session"
                    );
                }
            }
        }

        if reset_count > 0 {
            tracing::info!(
                count = reset_count,
                "Verification reconciliation: reset stuck sessions"
            );
        }

        reset_count
    }

    /// Run periodic reconciliation loop. Never returns (runs until task is cancelled).
    pub async fn run_periodic(self: Arc<Self>) {
        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(self.config.interval_secs));
        interval.tick().await; // skip immediate first tick (startup_scan handles it)

        loop {
            interval.tick().await;
            self.scan_and_reset().await;
        }
    }

    /// Startup scan — run once at boot before the periodic loop begins.
    pub async fn startup_scan(&self) {
        tracing::info!("Running verification startup scan...");
        let count = self.scan_and_reset().await;
        tracing::info!(count, "Verification startup scan complete");
    }
}

#[cfg(test)]
#[path = "verification_reconciliation_tests.rs"]
mod tests;
