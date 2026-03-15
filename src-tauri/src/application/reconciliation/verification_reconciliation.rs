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

use crate::domain::entities::{IdeationSessionStatus, VerificationStatus};
use crate::domain::repositories::IdeationSessionRepository;
use crate::domain::services::emit_verification_status_changed;

/// Configuration for the verification reconciliation service.
#[derive(Debug, Clone, Copy)]
pub struct VerificationReconciliationConfig {
    /// Sessions stuck in `verification_in_progress=1` for longer than this are reset (manual verify).
    pub stale_after_secs: u64,
    /// Shorter stale threshold for auto-verify sessions (generation > 0).
    pub auto_verify_stale_secs: u64,
    /// How often to scan for stuck sessions (seconds).
    pub interval_secs: u64,
}

impl Default for VerificationReconciliationConfig {
    fn default() -> Self {
        Self {
            stale_after_secs: 5400,       // 90 minutes for manual verify (D14)
            auto_verify_stale_secs: 600,  // 10 minutes for auto-verify
            interval_secs: 300,           // 5 minutes
        }
    }
}

/// Service that detects and resets stuck verification sessions.
pub struct VerificationReconciliationService {
    ideation_session_repo: Arc<dyn IdeationSessionRepository>,
    config: VerificationReconciliationConfig,
    /// AppHandle for emitting UI events after reconciliation resets.
    /// `None` in tests (no Tauri runtime available).
    app_handle: Option<tauri::AppHandle>,
}

impl VerificationReconciliationService {
    pub fn new(
        ideation_session_repo: Arc<dyn IdeationSessionRepository>,
        config: VerificationReconciliationConfig,
    ) -> Self {
        Self {
            ideation_session_repo,
            config,
            app_handle: None,
        }
    }

    /// Attach an AppHandle so the service can emit UI events after resetting stuck sessions.
    pub fn with_app_handle(mut self, app_handle: tauri::AppHandle) -> Self {
        self.app_handle = Some(app_handle);
        self
    }

    /// Scan for stuck sessions and reset them. Called on startup and periodically.
    ///
    /// Uses dual thresholds:
    /// - Auto-verify sessions (`verification_generation > 0`): reset after `auto_verify_stale_secs`
    /// - Manual verify sessions (`verification_generation == 0`): reset after `stale_after_secs`
    ///
    /// Returns the number of sessions reset.
    pub async fn scan_and_reset(&self) -> u32 {
        // Query with the shorter auto-verify threshold to get all candidates.
        // Manual sessions that haven't passed the longer threshold will be skipped below.
        let auto_stale_before = Utc::now()
            - chrono::Duration::seconds(self.config.auto_verify_stale_secs as i64);
        let manual_stale_before = Utc::now()
            - chrono::Duration::seconds(self.config.stale_after_secs as i64);

        let stale_sessions = match self
            .ideation_session_repo
            .get_stale_in_progress_sessions(auto_stale_before)
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
            // Never reset ImportedVerified sessions — their pre-verified status must be preserved.
            if session.verification_status == VerificationStatus::ImportedVerified {
                tracing::debug!(
                    session_id = %session.id.as_str(),
                    "Skipping imported_verified session — not eligible for reconciliation reset"
                );
                continue;
            }

            // Dual threshold: manual sessions need the longer stale period before reset.
            // Auto-verify sessions (generation > 0) are already filtered by auto_stale_before.
            if session.verification_generation == 0 && session.updated_at > manual_stale_before {
                tracing::debug!(
                    session_id = %session.id.as_str(),
                    generation = session.verification_generation,
                    "Skipping manual-verify session not yet stale for longer threshold"
                );
                continue;
            }

            let effective_stale_secs = if session.verification_generation > 0 {
                self.config.auto_verify_stale_secs
            } else {
                self.config.stale_after_secs
            };

            // Force-reset via update_verification_state (unconditional).
            // Preserve existing metadata so the frontend can show what happened.
            // reset_verification() guards on in_progress=false and is only for
            // conditional resets on plan artifact updates — not for crash recovery.
            match self
                .ideation_session_repo
                .update_verification_state(
                    &session.id,
                    VerificationStatus::Unverified,
                    false,
                    session.verification_metadata.clone(),
                )
                .await
            {
                Ok(()) => {
                    tracing::info!(
                        session_id = %session.id.as_str(),
                        generation = session.verification_generation,
                        stale_after_secs = effective_stale_secs,
                        "Reconciliation reset stuck verification"
                    );
                    // Emit UI event so the frontend reflects the reset immediately
                    if let Some(ref handle) = self.app_handle {
                        emit_verification_status_changed(
                            handle,
                            session.id.as_str(),
                            VerificationStatus::Unverified,
                            false,
                            None,
                            None,
                        );
                    }
                    // Archive any orphaned verification children for this parent session
                    match self
                        .ideation_session_repo
                        .get_verification_children(&session.id)
                        .await
                    {
                        Ok(children) => {
                            for child in &children {
                                match self
                                    .ideation_session_repo
                                    .update_status(&child.id, IdeationSessionStatus::Archived)
                                    .await
                                {
                                    Ok(()) => {
                                        tracing::info!(
                                            child_session_id = %child.id.as_str(),
                                            parent_session_id = %session.id.as_str(),
                                            "Archived orphaned verification child session"
                                        );
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            child_session_id = %child.id.as_str(),
                                            error = %e,
                                            "Failed to archive orphaned verification child session"
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                session_id = %session.id.as_str(),
                                error = %e,
                                "Failed to query verification children during reconciliation"
                            );
                        }
                    }
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
