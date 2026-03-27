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

use std::collections::HashSet;
use std::sync::Arc;

use chrono::Utc;
use tauri::{AppHandle, Runtime};

use crate::application::reconciliation::recovery_queue::{
    RecoveryItem, RecoveryKind, RecoveryMetadata, RecoveryQueue,
};
use crate::domain::entities::{
    ChatContextType, IdeationSession, IdeationSessionId, IdeationSessionStatus, SessionPurpose,
    VerificationMetadata, VerificationStatus,
};
use crate::domain::repositories::IdeationSessionRepository;
use crate::domain::services::{
    emit_verification_status_changed, is_process_alive, RunningAgentRegistry,
};

/// Configuration for the verification reconciliation service.
#[derive(Debug, Clone, Copy)]
pub struct VerificationReconciliationConfig {
    /// Sessions stuck in `verification_in_progress=1` for longer than this are reset (manual verify).
    pub stale_after_secs: u64,
    /// Shorter stale threshold for auto-verify sessions (generation > 0).
    pub auto_verify_stale_secs: u64,
    /// How often to scan for stuck sessions (seconds).
    pub interval_secs: u64,
    /// TTL for stale external session archival and stall detection (seconds).
    /// External sessions with phase 'created'/'error' older than this are archived.
    /// External sessions with no activity for this long are marked 'stalled'.
    pub external_session_stale_secs: u64,
}

impl Default for VerificationReconciliationConfig {
    fn default() -> Self {
        Self {
            stale_after_secs: 5400,             // 90 minutes for manual verify (D14)
            auto_verify_stale_secs: 600,        // 10 minutes for auto-verify
            interval_secs: 300,                 // 5 minutes
            external_session_stale_secs: 7200,  // 2 hours (matches ExternalMcpConfig default)
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
    /// Shared recovery queue for submitting orphaned verification agent recovery items.
    /// Set via `with_recovery_queue()`. Phase 2 (startup_scan) submits items here.
    /// `None` degrades gracefully to the existing reset-only behavior.
    recovery_queue: Option<Arc<RecoveryQueue>>,
    /// Running agent registry for checking orphaned agent PIDs during startup_scan.
    /// Set via `with_running_agent_registry()`. `None` = no recovery attempted.
    running_agent_registry: Option<Arc<dyn RunningAgentRegistry>>,
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
            recovery_queue: None,
            running_agent_registry: None,
        }
    }

    /// Attach an AppHandle so the service can emit UI events after resetting stuck sessions.
    pub fn with_app_handle(mut self, app_handle: tauri::AppHandle) -> Self {
        self.app_handle = Some(app_handle);
        self
    }

    /// Attach the shared RecoveryQueue for submitting orphaned verification agent recovery items.
    ///
    /// When set, `startup_scan()` (Phase 2) will attempt recovery before falling through
    /// to the existing reset behavior. Without this, `startup_scan()` resets unconditionally.
    pub fn with_recovery_queue(mut self, queue: Arc<RecoveryQueue>) -> Self {
        self.recovery_queue = Some(queue);
        self
    }

    /// Attach the RunningAgentRegistry for checking orphaned agent PIDs during startup_scan.
    pub fn with_running_agent_registry(mut self, registry: Arc<dyn RunningAgentRegistry>) -> Self {
        self.running_agent_registry = Some(registry);
        self
    }

    /// Scan for stuck sessions and reset them. Called on startup and periodically.
    ///
    /// When `cold_boot: true` (app startup): all in-progress sessions are reset unconditionally
    /// using `get_all_in_progress_sessions()` — no TTL filter, since all agent processes are dead.
    /// Injects `app_restart` convergence_reason metadata.
    ///
    /// When `cold_boot: false` (periodic): uses dual thresholds:
    /// - Auto-verify sessions (`verification_generation > 0`): reset after `auto_verify_stale_secs`
    /// - Manual verify sessions (`verification_generation == 0`): reset after `stale_after_secs`
    ///
    /// Returns the number of sessions reset.
    pub async fn scan_and_reset(&self, cold_boot: bool) -> u32 {
        self.scan_and_reset_excluding(cold_boot, &HashSet::new()).await
    }

    /// Internal variant of `scan_and_reset` that skips parent session IDs in `skip_parent_ids`.
    ///
    /// Used by `startup_scan` to avoid resetting sessions already claimed for recovery.
    /// `scan_and_reset` delegates here with an empty skip set.
    async fn scan_and_reset_excluding(
        &self,
        cold_boot: bool,
        skip_parent_ids: &HashSet<String>,
    ) -> u32 {
        let manual_stale_before = Utc::now()
            - chrono::Duration::seconds(self.config.stale_after_secs as i64);

        let sessions = if cold_boot {
            // Cold boot: all agent processes are dead. Reset unconditionally — no TTL filter.
            match self.ideation_session_repo.get_all_in_progress_sessions().await {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        "Failed to query in-progress sessions for cold boot reset"
                    );
                    return 0;
                }
            }
        } else {
            // Periodic: query with the shorter auto-verify threshold to get all candidates.
            // Manual sessions that haven't passed the longer threshold will be skipped below.
            let auto_stale_before = Utc::now()
                - chrono::Duration::seconds(self.config.auto_verify_stale_secs as i64);
            match self
                .ideation_session_repo
                .get_stale_in_progress_sessions(auto_stale_before)
                .await
            {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        "Failed to query stale verification sessions"
                    );
                    return 0;
                }
            }
        };

        let mut reset_count = 0u32;
        for session in &sessions {
            // Skip sessions whose verification child was claimed for recovery by startup_scan.
            if skip_parent_ids.contains(session.id.as_str()) {
                tracing::info!(
                    session_id = %session.id.as_str(),
                    "Startup: skipping reset — verification agent recovery in progress"
                );
                continue;
            }

            // Never reset ImportedVerified sessions — their pre-verified status must be preserved.
            if session.verification_status == VerificationStatus::ImportedVerified {
                tracing::debug!(
                    session_id = %session.id.as_str(),
                    "Skipping imported_verified session — not eligible for reconciliation reset"
                );
                continue;
            }

            // Dual threshold: only applies to periodic scans, not cold boot.
            // Cold boot resets all in-progress sessions unconditionally.
            if !cold_boot
                && session.verification_generation == 0
                && session.updated_at > manual_stale_before
            {
                tracing::debug!(
                    session_id = %session.id.as_str(),
                    generation = session.verification_generation,
                    "Skipping manual-verify session not yet stale for longer threshold"
                );
                continue;
            }

            // Cold boot: inject app_restart metadata. Periodic: preserve existing metadata.
            let metadata = if cold_boot {
                serde_json::to_string(&serde_json::json!({
                    "convergence_reason": "app_restart",
                }))
                .ok()
            } else {
                session.verification_metadata.clone()
            };

            // Force-reset via update_verification_state (unconditional).
            // reset_verification() guards on in_progress=false and is only for
            // conditional resets on plan artifact updates — not for crash recovery.
            match self
                .ideation_session_repo
                .update_verification_state(
                    &session.id,
                    VerificationStatus::Unverified,
                    false,
                    metadata,
                )
                .await
            {
                Ok(()) => {
                    if cold_boot {
                        tracing::info!(
                            session_id = %session.id.as_str(),
                            "Startup reset: cleared stale verification in-progress flag"
                        );
                    } else {
                        let effective_stale_secs = if session.verification_generation > 0 {
                            self.config.auto_verify_stale_secs
                        } else {
                            self.config.stale_after_secs
                        };
                        tracing::info!(
                            session_id = %session.id.as_str(),
                            generation = session.verification_generation,
                            stale_after_secs = effective_stale_secs,
                            "Reconciliation reset stuck verification"
                        );
                    }
                    // Emit UI event so the frontend reflects the reset immediately
                    if let Some(ref handle) = self.app_handle {
                        let convergence_reason =
                            if cold_boot { Some("app_restart") } else { None };
                        emit_verification_status_changed(
                            handle,
                            session.id.as_str(),
                            VerificationStatus::Unverified,
                            false,
                            None,
                            convergence_reason,
                            Some(session.verification_generation),
                        );
                    }
                    // Archive any orphaned verification children for this parent session
                    self.archive_orphaned_children(&session.id).await;
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

    /// Scan running_agents for dead verification agent PIDs and submit recoverable orphans
    /// to the RecoveryQueue. Returns the set of parent session IDs that were claimed.
    ///
    /// Only runs when both `running_agent_registry` and `recovery_queue` are set.
    /// Gracefully returns an empty set if either is absent (degraded / test mode).
    ///
    /// Recovery submission criteria:
    /// - context_type == "ideation" AND pid is not alive
    /// - Session exists in DB, purpose == Verification, not Archived
    /// - Parent session exists, is not ImportedVerified, has verification_in_progress = true
    async fn scan_for_recoverable_orphans(&self) -> HashSet<String> {
        let mut claimed_parent_ids = HashSet::new();

        let registry = match self.running_agent_registry.as_ref() {
            Some(r) => r,
            None => return claimed_parent_ids,
        };
        let queue = match self.recovery_queue.as_ref() {
            Some(q) => q,
            None => return claimed_parent_ids,
        };

        let all_agents = registry.list_all().await;
        let ideation_context_type = ChatContextType::Ideation.to_string();

        for (key, info) in &all_agents {
            // Only interested in ideation context entries
            if key.context_type != ideation_context_type {
                continue;
            }
            // Only interested in dead processes
            if is_process_alive(info.pid) {
                continue;
            }

            let session_id = IdeationSessionId::from_string(key.context_id.clone());

            // Fetch the session to check if it is a verification child
            let session = match self.ideation_session_repo.get_by_id(&session_id).await {
                Ok(Some(s)) => s,
                Ok(None) => {
                    tracing::debug!(
                        context_id = %key.context_id,
                        "scan_for_recoverable_orphans: session not found in DB — skipping"
                    );
                    continue;
                }
                Err(e) => {
                    tracing::warn!(
                        context_id = %key.context_id,
                        error = %e,
                        "scan_for_recoverable_orphans: DB error fetching session — skipping"
                    );
                    continue;
                }
            };

            // Only process non-archived verification child sessions
            if session.session_purpose != SessionPurpose::Verification {
                continue;
            }
            if session.status == IdeationSessionStatus::Archived {
                continue;
            }

            let parent_id = match session.parent_session_id.as_ref() {
                Some(id) => id.clone(),
                None => {
                    tracing::warn!(
                        context_id = %key.context_id,
                        "scan_for_recoverable_orphans: verification session has no parent — skipping"
                    );
                    continue;
                }
            };

            // Resolve parent (3-guard check: not found, ImportedVerified, not in progress)
            let parent = match resolve_verification_parent(
                &parent_id,
                &self.ideation_session_repo,
                "scan_for_recoverable_orphans",
            )
            .await
            {
                ResolvedParent::Ready(p) => *p,
                ResolvedParent::NotFound | ResolvedParent::ImportedVerified | ResolvedParent::AlreadyResolved => {
                    continue;
                }
            };

            // Extract recovery metadata from parent's verification_metadata
            let parsed_meta: Option<VerificationMetadata> = parent
                .verification_metadata
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok());

            let current_round = parsed_meta.as_ref().map(|m| m.rounds.len() as u32);
            let verification_generation = if parent.verification_generation >= 0 {
                Some(parent.verification_generation as u32)
            } else {
                None
            };
            let plan_artifact_id = parent
                .plan_artifact_id
                .as_ref()
                .map(|id| id.as_str().to_string());

            let recovery_item = RecoveryItem {
                context_type: ChatContextType::Ideation,
                context_id: key.context_id.clone(),
                recovery_kind: RecoveryKind::VerificationAgent,
                // Verification children get lower priority (5) than parent ideation agents (10)
                priority: 5,
                parent_session_id: Some(parent_id.as_str().to_string()),
                metadata: RecoveryMetadata {
                    current_round,
                    verification_generation,
                    conversation_id: Some(info.conversation_id.clone()),
                    plan_artifact_id,
                },
            };

            match queue.submit(recovery_item) {
                Ok(()) => {
                    tracing::info!(
                        context_id = %key.context_id,
                        parent_id = %parent_id.as_str(),
                        pid = info.pid,
                        "scan_for_recoverable_orphans: submitted verification agent recovery item"
                    );
                    claimed_parent_ids.insert(parent_id.as_str().to_string());
                }
                Err(e) => {
                    tracing::warn!(
                        context_id = %key.context_id,
                        error = %e,
                        "scan_for_recoverable_orphans: failed to submit recovery item — falling through to reset"
                    );
                }
            }
        }

        if !claimed_parent_ids.is_empty() {
            tracing::info!(
                count = claimed_parent_ids.len(),
                "scan_for_recoverable_orphans: claimed verification sessions for recovery"
            );
        }

        claimed_parent_ids
    }

    /// Archive all orphaned verification child sessions linked to `parent_id`.
    async fn archive_orphaned_children(&self, parent_id: &IdeationSessionId) {
        match self
            .ideation_session_repo
            .get_verification_children(parent_id)
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
                                parent_session_id = %parent_id.as_str(),
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
                    session_id = %parent_id.as_str(),
                    error = %e,
                    "Failed to query verification children during reconciliation"
                );
            }
        }
    }

    /// Run periodic reconciliation loop. Never returns (runs until task is cancelled).
    pub async fn run_periodic(self: Arc<Self>) {
        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(self.config.interval_secs));
        interval.tick().await; // skip immediate first tick (startup_scan handles cold boot)

        loop {
            interval.tick().await;
            self.scan_and_reset(false).await;
            self.scan_and_archive_stale_external_sessions(false).await;
        }
    }

    /// Startup scan — run once at boot before the periodic loop begins.
    ///
    /// When `recovery_queue` and `running_agent_registry` are set (production):
    ///   1. Scans running_agents for dead verification agent PIDs.
    ///   2. Submits recoverable orphans to the RecoveryQueue for re-spawn.
    ///   3. Resets remaining stuck sessions (those not claimed by recovery).
    ///
    /// Without registry/queue set (tests or degraded mode): falls through to
    /// `scan_and_reset(cold_boot: true)` which resets ALL in-progress sessions
    /// unconditionally (no TTL filter), since all agent processes are dead on restart.
    ///
    /// Also archives all stale external sessions (cold boot — all agent processes are dead).
    pub async fn startup_scan(&self) {
        tracing::info!("Running verification startup scan (cold boot)...");
        let recovery_claimed = self.scan_for_recoverable_orphans().await;
        let count = self.scan_and_reset_excluding(true, &recovery_claimed).await;
        if count > 0 {
            tracing::info!(count, "Startup: reset orphaned verification in_progress states");
        }
        self.scan_and_archive_stale_external_sessions(true).await;
    }

    /// Scan for stale external sessions and archive them, then detect stalled sessions.
    ///
    /// Stale definition: external + active + phase IN ('created', 'error') + created_at older
    /// than `external_session_stale_secs`. These sessions have abandoned agents.
    ///
    /// When `cold_boot: true` (app startup): no TTL filter — archives ALL matching sessions
    /// since all agent processes are dead after restart.
    /// When `cold_boot: false` (periodic): TTL-based archival (created_at < stale_before).
    ///
    /// After archival, runs stall detection for periodic scans only (cold boot handles all dead
    /// sessions via archival). Stall detection marks sessions with no recent activity as 'stalled'.
    pub async fn scan_and_archive_stale_external_sessions(&self, cold_boot: bool) {
        let stale_before = if cold_boot {
            None // No TTL filter on startup — all agents are dead
        } else {
            Some(
                Utc::now()
                    - chrono::Duration::seconds(self.config.external_session_stale_secs as i64),
            )
        };

        // Archive stale external sessions (phase 'created' or 'error', past TTL or all on boot)
        let sessions = match self
            .ideation_session_repo
            .list_active_external_sessions_for_archival(stale_before)
            .await
        {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    cold_boot,
                    "Failed to query stale external sessions for archival"
                );
                return;
            }
        };

        let archive_count = sessions.len();
        for session in &sessions {
            match self
                .ideation_session_repo
                .update_status(&session.id, IdeationSessionStatus::Archived)
                .await
            {
                Ok(()) => {
                    tracing::info!(
                        session_id = %session.id.as_str(),
                        phase = ?session.external_activity_phase,
                        created_at = %session.created_at,
                        cold_boot,
                        "Archived stale external session"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        session_id = %session.id.as_str(),
                        error = %e,
                        "Failed to archive stale external session"
                    );
                }
            }
        }

        if archive_count > 0 {
            tracing::info!(
                count = archive_count,
                cold_boot,
                "External session reconciliation: archived stale sessions"
            );
        }

        // Detect stalled sessions (periodic only — cold boot archives all dead sessions above)
        if !cold_boot {
            self.detect_and_mark_stalled_external_sessions().await;
        }
    }

    /// Detect stalled external sessions and mark their phase as 'stalled'.
    ///
    /// A stalled session is one that is external + active + has an active phase
    /// (not 'error' or 'stalled') but has had no activity for longer than
    /// `external_session_stale_secs`. Detection is DB-only via `updated_at` timestamp.
    async fn detect_and_mark_stalled_external_sessions(&self) {
        let stalled_before = Utc::now()
            - chrono::Duration::seconds(self.config.external_session_stale_secs as i64);

        let sessions = match self
            .ideation_session_repo
            .list_stalled_external_sessions(stalled_before)
            .await
        {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "Failed to query stalled external sessions"
                );
                return;
            }
        };

        let stall_count = sessions.len();
        for session in &sessions {
            match self
                .ideation_session_repo
                .update_external_activity_phase(&session.id, Some("stalled"))
                .await
            {
                Ok(()) => {
                    tracing::info!(
                        session_id = %session.id.as_str(),
                        phase = ?session.external_activity_phase,
                        updated_at = %session.updated_at,
                        "Marked external session as stalled"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        session_id = %session.id.as_str(),
                        error = %e,
                        "Failed to mark external session as stalled"
                    );
                }
            }
        }

        if stall_count > 0 {
            tracing::info!(
                count = stall_count,
                "External session reconciliation: marked stalled sessions"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// run_completed reconciliation hooks
// ---------------------------------------------------------------------------

/// Outcome of resolving a verification parent session.
///
/// Used by `resolve_verification_parent()` to communicate why a parent was not usable,
/// without losing semantic distinction between the different guard outcomes.
enum ResolvedParent {
    /// Parent found and eligible: not ImportedVerified, verification_in_progress = true.
    Ready(Box<IdeationSession>),
    /// Parent not found in DB (or DB error).
    NotFound,
    /// Parent found but verification_in_progress = false — already resolved, no action needed.
    AlreadyResolved,
    /// Parent found but is ImportedVerified — status must never be overwritten.
    ImportedVerified,
}

/// Fetch a parent session and apply the standard 3-guard checks.
///
/// Guards applied in order:
/// 1. DB lookup failure / not found → `ResolvedParent::NotFound`
/// 2. `verification_status == ImportedVerified` → `ResolvedParent::ImportedVerified`
/// 3. `!verification_in_progress` → `ResolvedParent::AlreadyResolved`
///
/// On success returns `ResolvedParent::Ready(parent)`.
async fn resolve_verification_parent(
    parent_id: &IdeationSessionId,
    repo: &Arc<dyn IdeationSessionRepository>,
    caller: &str,
) -> ResolvedParent {
    match repo.get_by_id(parent_id).await {
        Ok(Some(parent)) => {
            if parent.verification_status == VerificationStatus::ImportedVerified {
                tracing::warn!(
                    parent_id = %parent_id.as_str(),
                    caller,
                    "resolve_verification_parent: parent is ImportedVerified — skip"
                );
                ResolvedParent::ImportedVerified
            } else if !parent.verification_in_progress {
                tracing::debug!(
                    parent_id = %parent_id.as_str(),
                    caller,
                    "resolve_verification_parent: verification not in progress — skip"
                );
                ResolvedParent::AlreadyResolved
            } else {
                ResolvedParent::Ready(Box::new(parent))
            }
        }
        Ok(None) => {
            tracing::warn!(
                parent_id = %parent_id.as_str(),
                caller,
                "resolve_verification_parent: parent session not found"
            );
            ResolvedParent::NotFound
        }
        Err(e) => {
            tracing::warn!(
                parent_id = %parent_id.as_str(),
                error = %e,
                caller,
                "resolve_verification_parent: DB error fetching parent"
            );
            ResolvedParent::NotFound
        }
    }
}

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
    // Resolve parent (fetch + 3-guard check)
    let parent = match resolve_verification_parent(
        parent_id,
        repo,
        "reconcile_verification_on_child_complete",
    )
    .await
    {
        ResolvedParent::Ready(p) => p,
        ResolvedParent::NotFound | ResolvedParent::ImportedVerified => return,
        ResolvedParent::AlreadyResolved => {
            tracing::debug!(
                parent_id = %parent_id.as_str(),
                child_id = %child_id.as_str(),
                "reconcile_verification_on_child_complete: verification not in progress — archiving child only"
            );
            archive_verification_session(repo, child_id).await;
            return;
        }
    };

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
    archive_sibling_verification_children(repo, parent_id, Some(child_id)).await;
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

    // Resolve parent (fetch + 3-guard check)
    let parent = match resolve_verification_parent(
        &parent_id,
        repo,
        "reset_verification_on_child_error",
    )
    .await
    {
        ResolvedParent::Ready(p) => p,
        ResolvedParent::NotFound => {
            archive_verification_session(repo, child_id).await;
            return;
        }
        ResolvedParent::ImportedVerified => return,
        ResolvedParent::AlreadyResolved => {
            archive_verification_session(repo, child_id).await;
            return;
        }
    };

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
    archive_sibling_verification_children(repo, &parent_id, Some(child_id)).await;
}

/// Archive orphaned sibling verification children of a parent session.
///
/// Archives all non-archived verification children of `parent_id`, optionally
/// excluding `exclude_child_id` (the child being reconciled — already archived separately).
///
/// Extracted from duplicated orphan-cleanup blocks in `reconcile_verification_on_child_complete`
/// (formerly L460-482) and `reset_verification_on_child_error` (formerly L622-643).
async fn archive_sibling_verification_children(
    repo: &Arc<dyn IdeationSessionRepository>,
    parent_id: &IdeationSessionId,
    exclude_child_id: Option<&IdeationSessionId>,
) {
    match repo.get_verification_children(parent_id).await {
        Ok(children) => {
            for child in &children {
                let should_skip = child.status == IdeationSessionStatus::Archived
                    || exclude_child_id.is_some_and(|id| child.id == *id);
                if should_skip {
                    continue;
                }
                tracing::info!(
                    orphan_child_id = %child.id.as_str(),
                    parent_id = %parent_id.as_str(),
                    "Archiving orphaned sibling verification child session"
                );
                archive_verification_session(repo, &child.id).await;
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
