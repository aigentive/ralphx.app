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
use tauri::{AppHandle, Runtime};

use crate::domain::entities::{
    IdeationSessionId, IdeationSessionStatus, SessionPurpose, VerificationMetadata,
    VerificationStatus,
};
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
                            Some(session.verification_generation),
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

    /// Unconditional startup reset — resets ALL sessions with `verification_in_progress=1`.
    ///
    /// On app startup, no verification agents are running (they were killed when the app exited).
    /// Any `verification_in_progress = 1` is stale by definition. This is safe to reset
    /// unconditionally without TTL filters — unlike `startup_scan()` which uses the same
    /// 10/90-minute thresholds as the periodic reconciler.
    ///
    /// Called ONCE at boot, BEFORE the periodic reconciler starts.
    /// Replaces `startup_scan()` in the boot path.
    pub async fn startup_reset_all_in_progress(&self) {
        tracing::info!("Running startup reset for all in-progress verification sessions...");

        let sessions = match self
            .ideation_session_repo
            .get_all_in_progress_sessions()
            .await
        {
            Ok(sessions) => sessions,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "Failed to query in-progress sessions for startup reset"
                );
                return;
            }
        };

        let mut reset_count = 0u32;
        for session in &sessions {
            // Never reset ImportedVerified sessions — their pre-verified status must be preserved.
            if session.verification_status == VerificationStatus::ImportedVerified {
                tracing::debug!(
                    session_id = %session.id.as_str(),
                    "Skipping imported_verified session during startup reset"
                );
                continue;
            }

            let restart_metadata_json = serde_json::to_string(&serde_json::json!({
                "convergence_reason": "app_restart",
            }))
            .ok();

            match self
                .ideation_session_repo
                .update_verification_state(
                    &session.id,
                    VerificationStatus::Unverified,
                    false,
                    restart_metadata_json,
                )
                .await
            {
                Ok(()) => {
                    tracing::info!(
                        session_id = %session.id.as_str(),
                        "Startup reset: cleared stale verification in-progress flag"
                    );
                    if let Some(ref handle) = self.app_handle {
                        emit_verification_status_changed(
                            handle,
                            session.id.as_str(),
                            VerificationStatus::Unverified,
                            false,
                            None,
                            Some("app_restart"),
                            Some(session.verification_generation),
                        );
                    }
                    // Archive any orphaned verification children for this parent
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
                                            "Archived orphaned verification child during startup reset"
                                        );
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            child_session_id = %child.id.as_str(),
                                            error = %e,
                                            "Failed to archive orphaned verification child during startup reset"
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                session_id = %session.id.as_str(),
                                error = %e,
                                "Failed to query verification children during startup reset"
                            );
                        }
                    }
                    reset_count += 1;
                }
                Err(e) => {
                    tracing::warn!(
                        session_id = %session.id.as_str(),
                        error = %e,
                        "Failed to reset in-progress session during startup reset"
                    );
                }
            }
        }

        tracing::info!(count = reset_count, "Verification startup reset complete");
    }
}

// ---------------------------------------------------------------------------
// run_completed reconciliation hooks
// ---------------------------------------------------------------------------

/// Reconcile verification state when a verification child agent's run completes successfully.
///
/// Called from `handle_stream_success` when the completed session has
/// `session_purpose == Verification`. Analyzes `verification_metadata` on the parent
/// to determine the correct terminal status, updates the parent, archives the child,
/// and emits a frontend event.
///
/// Three decision branches based on metadata state:
/// - `convergence_reason` set → map to status via `convergence_reason_to_status`
/// - `convergence_reason` unset but rounds non-empty → agent crashed mid-round → `NeedsRevision`
/// - no metadata or empty rounds → agent completed without updates → `Unverified`
pub async fn reconcile_verification_on_child_complete<R: Runtime>(
    parent_id: &IdeationSessionId,
    child_id: &IdeationSessionId,
    repo: &Arc<dyn IdeationSessionRepository>,
    app_handle: Option<&AppHandle<R>>,
) {
    // Fetch parent session
    let parent = match repo.get_by_id(parent_id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            tracing::warn!(
                parent_id = %parent_id.as_str(),
                child_id = %child_id.as_str(),
                "reconcile_verification_on_child_complete: parent session not found"
            );
            return;
        }
        Err(e) => {
            tracing::warn!(
                parent_id = %parent_id.as_str(),
                error = %e,
                "reconcile_verification_on_child_complete: failed to fetch parent session"
            );
            return;
        }
    };

    // ImportedVerified guard — never overwrite pre-verified status
    if parent.verification_status == VerificationStatus::ImportedVerified {
        tracing::warn!(
            parent_id = %parent_id.as_str(),
            "Skipping reconciliation for ImportedVerified parent — status must not be overwritten"
        );
        return;
    }

    // No-op if verification is already resolved
    if !parent.verification_in_progress {
        tracing::debug!(
            parent_id = %parent_id.as_str(),
            child_id = %child_id.as_str(),
            "reconcile_verification_on_child_complete: verification not in progress — archiving child only"
        );
        archive_verification_session(repo, child_id).await;
        return;
    }

    // Parse verification_metadata from parent
    let parsed_meta: Option<VerificationMetadata> = parent
        .verification_metadata
        .as_ref()
        .and_then(|s| match serde_json::from_str(s) {
            Ok(m) => Some(m),
            Err(e) => {
                tracing::warn!(
                    parent_id = %parent_id.as_str(),
                    error = %e,
                    "Failed to parse verification_metadata — treating as None"
                );
                None
            }
        });

    let has_convergence_reason = parsed_meta
        .as_ref()
        .and_then(|m| m.convergence_reason.as_deref())
        .is_some();
    let has_rounds = parsed_meta
        .as_ref()
        .map(|m| !m.rounds.is_empty())
        .unwrap_or(false);

    // Determine terminal status and emit metadata based on what the agent produced
    let (terminal_status, updated_metadata_json, emit_metadata, convergence_reason_override) =
        if has_convergence_reason {
            // Branch 1: Agent completed with convergence_reason — map to terminal status
            let reason = parsed_meta
                .as_ref()
                .unwrap()
                .convergence_reason
                .as_deref()
                .unwrap_or("");
            let status = convergence_reason_to_status(reason);
            // Keep existing metadata as-is (convergence_reason already present)
            (status, parent.verification_metadata.clone(), parsed_meta.clone(), None::<String>)
        } else if has_rounds {
            // Branch 2: Agent crashed mid-round with partial progress
            let mut updated_m = parsed_meta.clone().unwrap();
            updated_m.convergence_reason = Some("agent_crashed_mid_round".to_string());
            let updated_json = serde_json::to_string(&updated_m).ok();
            (
                VerificationStatus::NeedsRevision,
                updated_json,
                Some(updated_m),
                None::<String>,
            )
        } else {
            // Branch 3: No metadata or empty rounds — agent completed without any updates
            let minimal_json = serde_json::to_string(&serde_json::json!({
                "convergence_reason": "agent_completed_without_update",
            }))
            .ok();
            (
                VerificationStatus::Unverified,
                minimal_json,
                None::<VerificationMetadata>,
                Some("agent_completed_without_update".to_string()),
            )
        };

    // Update parent verification state
    match repo
        .update_verification_state(parent_id, terminal_status, false, updated_metadata_json)
        .await
    {
        Ok(()) => {
            tracing::info!(
                parent_id = %parent_id.as_str(),
                child_id = %child_id.as_str(),
                status = %terminal_status,
                convergence_reason = ?convergence_reason_override,
                "Reconciled verification state after child agent completion"
            );
        }
        Err(e) => {
            tracing::warn!(
                parent_id = %parent_id.as_str(),
                error = %e,
                "Failed to update parent verification state during reconciliation"
            );
            // Still archive the child and emit even if update failed
        }
    }

    // Emit frontend event so UI updates immediately
    if let Some(handle) = app_handle {
        emit_verification_status_changed(
            handle,
            parent_id.as_str(),
            terminal_status,
            false,
            emit_metadata.as_ref(),
            convergence_reason_override.as_deref(),
            Some(parent.verification_generation),
        );
    }

    // Archive the current child session
    archive_verification_session(repo, child_id).await;

    // Orphan cleanup: archive any OTHER active verification children of this parent
    match repo.get_verification_children(parent_id).await {
        Ok(children) => {
            for child in &children {
                if child.id != *child_id
                    && child.status != IdeationSessionStatus::Archived
                {
                    tracing::info!(
                        orphan_child_id = %child.id.as_str(),
                        parent_id = %parent_id.as_str(),
                        "Archiving orphaned sibling verification child session"
                    );
                    archive_verification_session(repo, &child.id).await;
                }
            }
        }
        Err(e) => {
            tracing::warn!(
                parent_id = %parent_id.as_str(),
                error = %e,
                "Failed to query verification children for orphan cleanup"
            );
        }
    }
}

/// Reset parent verification state when a verification child agent errors or is stopped.
///
/// Used for Path B (agent error — `convergence_reason = "agent_error"`) and
/// Path C (user stop — `convergence_reason = "user_stopped"`). Always resets parent
/// to `Unverified` with the given reason, regardless of metadata state.
///
/// Looks up the child session internally to find the parent. No-ops if the child is
/// not a verification session or has no `parent_session_id`.
pub async fn reset_verification_on_child_error<R: Runtime>(
    child_id: &IdeationSessionId,
    repo: &Arc<dyn IdeationSessionRepository>,
    app_handle: Option<&AppHandle<R>>,
    convergence_reason: &str,
) {
    // Fetch child to determine purpose and parent_id
    let child_session = match repo.get_by_id(child_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            tracing::warn!(
                child_id = %child_id.as_str(),
                "reset_verification_on_child_error: child session not found"
            );
            return;
        }
        Err(e) => {
            tracing::warn!(
                child_id = %child_id.as_str(),
                error = %e,
                "reset_verification_on_child_error: failed to fetch child session"
            );
            return;
        }
    };

    // Only act on verification child sessions
    if child_session.session_purpose != SessionPurpose::Verification {
        return;
    }

    let parent_id = match child_session.parent_session_id {
        Some(id) => id,
        None => {
            tracing::warn!(
                child_id = %child_id.as_str(),
                "Verification child has no parent_session_id — archiving child only"
            );
            archive_verification_session(repo, child_id).await;
            return;
        }
    };

    // Fetch parent session
    let parent = match repo.get_by_id(&parent_id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            tracing::warn!(
                parent_id = %parent_id.as_str(),
                "reset_verification_on_child_error: parent session not found"
            );
            archive_verification_session(repo, child_id).await;
            return;
        }
        Err(e) => {
            tracing::warn!(
                parent_id = %parent_id.as_str(),
                error = %e,
                "reset_verification_on_child_error: failed to fetch parent session"
            );
            return;
        }
    };

    // ImportedVerified guard — never overwrite pre-verified status
    if parent.verification_status == VerificationStatus::ImportedVerified {
        tracing::warn!(
            parent_id = %parent_id.as_str(),
            "Skipping error-reset for ImportedVerified parent — status must not be overwritten"
        );
        return;
    }

    // No-op if verification already resolved; still archive the child
    if !parent.verification_in_progress {
        archive_verification_session(repo, child_id).await;
        return;
    }

    // Build minimal metadata JSON with the error convergence_reason
    let error_metadata_json = serde_json::to_string(&serde_json::json!({
        "convergence_reason": convergence_reason,
    }))
    .ok();

    // Reset parent to Unverified with the given reason
    match repo
        .update_verification_state(
            &parent_id,
            VerificationStatus::Unverified,
            false,
            error_metadata_json,
        )
        .await
    {
        Ok(()) => {
            tracing::info!(
                parent_id = %parent_id.as_str(),
                child_id = %child_id.as_str(),
                convergence_reason,
                "Reset parent verification state after child agent error/stop"
            );
        }
        Err(e) => {
            tracing::warn!(
                parent_id = %parent_id.as_str(),
                error = %e,
                convergence_reason,
                "Failed to reset parent verification state after child error/stop"
            );
        }
    }

    // Emit frontend event
    if let Some(handle) = app_handle {
        emit_verification_status_changed(
            handle,
            parent_id.as_str(),
            VerificationStatus::Unverified,
            false,
            None,
            Some(convergence_reason),
            Some(parent.verification_generation),
        );
    }

    // Archive the child session
    archive_verification_session(repo, child_id).await;

    // Orphan cleanup: archive any other active verification children
    match repo.get_verification_children(&parent_id).await {
        Ok(children) => {
            for child in &children {
                if child.id != *child_id && child.status != IdeationSessionStatus::Archived {
                    tracing::info!(
                        orphan_child_id = %child.id.as_str(),
                        parent_id = %parent_id.as_str(),
                        "Archiving orphaned sibling verification child session after error"
                    );
                    archive_verification_session(repo, &child.id).await;
                }
            }
        }
        Err(e) => {
            tracing::warn!(
                parent_id = %parent_id.as_str(),
                error = %e,
                "Failed to query verification children for orphan cleanup after error"
            );
        }
    }
}

/// Map a `convergence_reason` string to the appropriate `VerificationStatus`.
///
/// | convergence_reason | VerificationStatus |
/// |---|---|
/// | zero_blocking / jaccard_converged / low_remaining_only | Verified |
/// | max_rounds / escalated_to_parent | NeedsRevision |
/// | agent_error / user_skipped / user_reverted / critic_parse_failure / user_stopped | Skipped |
/// | _unknown_ | NeedsRevision (defensive default) |
fn convergence_reason_to_status(reason: &str) -> VerificationStatus {
    match reason {
        "zero_blocking" | "jaccard_converged" | "low_remaining_only" => {
            VerificationStatus::Verified
        }
        "max_rounds" | "escalated_to_parent" => VerificationStatus::NeedsRevision,
        "agent_error" | "user_skipped" | "user_reverted" | "critic_parse_failure"
        | "user_stopped" => VerificationStatus::Skipped,
        _ => VerificationStatus::NeedsRevision, // defensive default for unrecognized reasons
    }
}

/// Archive a verification session by ID, logging warnings on failure.
async fn archive_verification_session(
    repo: &Arc<dyn IdeationSessionRepository>,
    session_id: &IdeationSessionId,
) {
    match repo
        .update_status(session_id, IdeationSessionStatus::Archived)
        .await
    {
        Ok(()) => {
            tracing::info!(
                session_id = %session_id.as_str(),
                "Archived verification session"
            );
        }
        Err(e) => {
            tracing::warn!(
                session_id = %session_id.as_str(),
                error = %e,
                "Failed to archive verification session"
            );
        }
    }
}

#[cfg(test)]
#[path = "verification_reconciliation_tests.rs"]
mod tests;
