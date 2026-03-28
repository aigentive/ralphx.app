// Ideation session repository trait - domain layer abstraction
//
// This trait defines the contract for ideation session persistence.
// Implementations can use SQLite, PostgreSQL, in-memory, etc.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::Connection;

use crate::domain::entities::{
    IdeationSession, IdeationSessionId, IdeationSessionStatus, ProjectId, VerificationMetadata,
    VerificationStatus,
};
use crate::error::AppResult;

/// Session group counts for the plan browser sidebar
#[derive(Debug, Clone)]
pub struct SessionGroupCounts {
    pub drafts: u32,
    pub in_progress: u32,
    pub accepted: u32,
    pub done: u32,
    pub archived: u32,
}

/// Task progress summary for an ideation session
#[derive(Debug, Clone)]
pub struct SessionProgress {
    pub idle: u32,
    pub active: u32,
    pub done: u32,
    pub total: u32,
}

/// Ideation session with optional task progress (for accepted sub-groups)
#[derive(Debug, Clone)]
pub struct IdeationSessionWithProgress {
    pub session: IdeationSession,
    /// Populated for accepted sub-groups (in_progress, accepted, done); None for drafts/archived
    pub progress: Option<SessionProgress>,
    /// Resolved server-side via LEFT JOIN on parent_session_id
    pub parent_session_title: Option<String>,
    /// Count of verification child sessions (session_purpose = 'verification') for this session
    pub verification_child_count: u32,
}

/// Repository trait for IdeationSession persistence.
/// Implementations can use SQLite, PostgreSQL, in-memory, etc.
#[async_trait]
pub trait IdeationSessionRepository: Send + Sync {
    /// Create a new ideation session
    async fn create(&self, session: IdeationSession) -> AppResult<IdeationSession>;

    /// Get session by ID
    async fn get_by_id(&self, id: &IdeationSessionId) -> AppResult<Option<IdeationSession>>;

    /// Get all sessions for a project, ordered by updated_at DESC
    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<IdeationSession>>;

    /// Update session status with appropriate timestamp updates
    async fn update_status(
        &self,
        id: &IdeationSessionId,
        status: IdeationSessionStatus,
    ) -> AppResult<()>;

    /// Update session title and source ("auto" for session-namer, "user" for manual rename)
    async fn update_title(
        &self,
        id: &IdeationSessionId,
        title: Option<String>,
        title_source: &str,
    ) -> AppResult<()>;

    /// Update session plan artifact ID
    async fn update_plan_artifact_id(
        &self,
        id: &IdeationSessionId,
        plan_artifact_id: Option<String>,
    ) -> AppResult<()>;

    /// Delete session (cascades to proposals and messages)
    async fn delete(&self, id: &IdeationSessionId) -> AppResult<()>;

    /// Get active sessions for a project
    async fn get_active_by_project(
        &self,
        project_id: &ProjectId,
    ) -> AppResult<Vec<IdeationSession>>;

    /// Count sessions by status for a project
    async fn count_by_status(
        &self,
        project_id: &ProjectId,
        status: IdeationSessionStatus,
    ) -> AppResult<u32>;

    /// Get sessions that have a specific plan artifact ID
    /// Used when updating a plan artifact to find sessions to re-link
    async fn get_by_plan_artifact_id(
        &self,
        plan_artifact_id: &str,
    ) -> AppResult<Vec<IdeationSession>>;

    /// Get sessions that have a specific inherited plan artifact ID
    /// Used in update_plan_artifact to detect and reject attempts to modify inherited plans
    async fn get_by_inherited_plan_artifact_id(
        &self,
        artifact_id: &str,
    ) -> AppResult<Vec<IdeationSession>>;

    /// Get all child sessions for a given parent session ID
    async fn get_children(&self, parent_id: &IdeationSessionId) -> AppResult<Vec<IdeationSession>>;

    /// Get the ancestor chain for a session (parents, grandparents, etc.)
    /// Returns sessions in order from direct parent to root ancestor
    async fn get_ancestor_chain(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<Vec<IdeationSession>>;

    /// Set the parent session ID for a session
    async fn set_parent(
        &self,
        id: &IdeationSessionId,
        parent_id: Option<&IdeationSessionId>,
    ) -> AppResult<()>;

    /// Update verification state atomically (status + in_progress flag + metadata)
    async fn update_verification_state(
        &self,
        id: &IdeationSessionId,
        status: VerificationStatus,
        in_progress: bool,
        metadata_json: Option<String>,
    ) -> AppResult<()>;

    /// Conditionally reset verification status to `unverified` — ONLY when `verification_in_progress = 0`.
    /// This prevents the loop-reset paradox where auto-corrections would reset verification mid-loop.
    ///
    /// Returns `true` if the reset occurred (rows_affected > 0), `false` if skipped (in_progress=1).
    async fn reset_verification(&self, id: &IdeationSessionId) -> AppResult<bool>;

    /// Get a session's verification status + metadata (lightweight read)
    async fn get_verification_status(
        &self,
        id: &IdeationSessionId,
    ) -> AppResult<Option<(VerificationStatus, bool, Option<String>)>>;

    /// Atomic revert-and-skip: update plan_artifact_id + set verification status=skipped
    async fn revert_plan_and_skip_verification(
        &self,
        id: &IdeationSessionId,
        new_plan_artifact_id: String,
        convergence_reason: String,
    ) -> AppResult<()>;

    /// Fully atomic revert-and-skip: inserts a new artifact version AND updates the session
    /// in a single `db.run(|conn| { ... })` transaction.
    #[allow(clippy::too_many_arguments)]
    async fn revert_plan_and_skip_with_artifact(
        &self,
        session_id: &IdeationSessionId,
        new_artifact_id: String,
        artifact_type_str: String,
        artifact_name: String,
        content_text: String,
        version: u32,
        previous_version_id: String,
        convergence_reason: String,
    ) -> AppResult<()>;

    /// Increment the verification generation counter by 1 for a session.
    /// Used to invalidate any in-flight verification agents that check the generation
    /// before writing results.
    async fn increment_verification_generation(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<()>;

    /// Atomic reset-and-begin for re-verification.
    ///
    /// Clears stale verification metadata (gaps, rounds, convergence_reason, best_round_index,
    /// current_round, parse_failures), increments verification_generation by 1, and sets
    /// verification_status → reviewing + verification_in_progress → true — all in one transaction.
    ///
    /// Called ONLY for terminal→reviewing transitions (Verified, Skipped, ImportedVerified).
    /// NOT for NeedsRevision → Reviewing (normal inter-round flow).
    ///
    /// Returns `(new_gen, cleared_metadata)` — handler uses new_gen in the response and
    /// cleared_metadata for event emission (the pre-call local metadata is stale after reset).
    ///
    /// # Errors
    ///
    /// Returns an error if the session is not found (0 rows affected) or on DB failure.
    async fn reset_and_begin_reverify(
        &self,
        session_id: &str,
    ) -> AppResult<(i32, VerificationMetadata)>;

    /// Find sessions where `verification_in_progress = 1` and `updated_at < stale_before`.
    async fn get_stale_in_progress_sessions(
        &self,
        stale_before: DateTime<Utc>,
    ) -> AppResult<Vec<IdeationSession>>;

    /// Find ALL sessions where `verification_in_progress = 1` and `status != 'archived'`.
    /// No TTL filter — used on startup to unconditionally reset all stale in-progress sessions.
    /// On startup, no verification agents are running, so any in-progress flag is stale by definition.
    async fn get_all_in_progress_sessions(&self) -> AppResult<Vec<IdeationSession>>;

    /// Get active (non-archived) verification child sessions for a parent session.
    /// Returns at most 1 session (the most recent), ordered by created_at DESC.
    async fn get_verification_children(
        &self,
        parent_session_id: &IdeationSessionId,
    ) -> AppResult<Vec<IdeationSession>>;

    /// List ALL active (non-archived) verification child sessions across the entire project.
    /// Used by the reconciler backstop to detect orphaned children whose parent has already resolved.
    /// Ordered by created_at ASC.
    async fn list_active_verification_children(&self) -> AppResult<Vec<IdeationSession>>;

    /// Get sessions for a project filtered by status, ordered by created_at DESC with a limit.
    async fn get_by_project_and_status(
        &self,
        project_id: &str,
        status: &str,
        limit: u32,
    ) -> AppResult<Vec<IdeationSession>>;

    /// Get counts of sessions in each display group for a project.
    /// Groups: drafts (active), in_progress (accepted+active tasks), accepted (accepted+no active tasks+not all done),
    /// done (accepted+all tasks terminal), archived.
    async fn get_group_counts(&self, project_id: &ProjectId) -> AppResult<SessionGroupCounts>;

    /// List sessions in a specific display group with pagination.
    /// group must be one of: "drafts", "in_progress", "accepted", "done", "archived"
    /// Returns (sessions_with_progress, total_count).
    async fn list_by_group(
        &self,
        project_id: &ProjectId,
        group: &str,
        offset: u32,
        limit: u32,
    ) -> AppResult<(Vec<IdeationSessionWithProgress>, u32)>;

    /// Set expected_proposal_count on a session (set-once: fails if already set to a different value).
    /// MUST be called inside a BEGIN IMMEDIATE transaction via `db.run_transaction()`.
    /// Sync variant for use inside `db.run()` closures.
    fn set_expected_proposal_count_sync(
        conn: &Connection,
        session_id: &str,
        count: u32,
    ) -> AppResult<()>
    where
        Self: Sized;

    /// Set auto_accept_status and optionally auto_accept_started_at on a session.
    /// status: "pending" | "success" | "failed"
    /// If error_reason is Some, it is stored as a JSON field in auto_accept_status (caller formats).
    async fn set_auto_accept_status(
        &self,
        session_id: &str,
        status: &str,
        auto_accept_started_at: Option<String>,
    ) -> AppResult<()>;

    /// Count non-archived proposals for a session (sync for use inside `db.run()` closures).
    /// WHERE session_id = ? AND status != 'archived'
    /// Do NOT use this for sort_order assignment — use the existing count_by_session_sync instead.
    fn count_active_by_session_sync(
        conn: &Connection,
        session_id: &str,
    ) -> AppResult<i64>
    where
        Self: Sized;

    /// Get a session by idempotency key + api_key_id combination.
    /// Returns None if no session found with that key for the given API key.
    async fn get_by_idempotency_key(
        &self,
        api_key_id: &str,
        idempotency_key: &str,
    ) -> AppResult<Option<IdeationSession>>;

    /// Update the external_activity_phase for a session.
    /// Pass `None` to clear the phase (set to NULL), e.g. on session reopen.
    async fn update_external_activity_phase(
        &self,
        id: &IdeationSessionId,
        phase: Option<&str>,
    ) -> AppResult<()>;

    /// Update the external_last_read_message_id cursor for a session.
    /// Called when an external agent fetches messages, to track read position.
    async fn update_external_last_read_message_id(
        &self,
        id: &IdeationSessionId,
        message_id: &str,
    ) -> AppResult<()>;

    /// List active external sessions for a project (origin = 'external', status = 'active').
    /// Used for Jaccard dedup check and existing-session awareness in start_ideation.
    async fn list_active_external_by_project(
        &self,
        project_id: &ProjectId,
    ) -> AppResult<Vec<IdeationSession>>;

    /// List active external sessions eligible for stale archival reconciliation.
    ///
    /// Returns sessions where:
    /// - origin = 'external'
    /// - status = 'active'
    /// - external_activity_phase IN ('created', 'error')
    /// - updated_at < stale_before (if Some; if None, no TTL filter — startup scan)
    ///
    /// Ordered by updated_at ASC.
    async fn list_active_external_sessions_for_archival(
        &self,
        stale_before: Option<DateTime<Utc>>,
    ) -> AppResult<Vec<IdeationSession>>;

    /// List external sessions that appear stalled (no recent activity).
    ///
    /// Returns sessions where:
    /// - origin = 'external'
    /// - status = 'active'
    /// - external_activity_phase IS NOT NULL AND external_activity_phase NOT IN ('error', 'stalled')
    /// - updated_at < stalled_before
    ///
    /// Ordered by updated_at ASC.
    async fn list_stalled_external_sessions(
        &self,
        stalled_before: DateTime<Utc>,
    ) -> AppResult<Vec<IdeationSession>>;

    /// Mark dependencies as acknowledged for a session.
    /// Sets `dependencies_acknowledged = true` and updates `updated_at`.
    async fn set_dependencies_acknowledged(&self, session_id: &str) -> AppResult<()>;

    /// Reset all acceptance-cycle fields so the session can be re-accepted cleanly.
    ///
    /// Sets:
    /// - `expected_proposal_count = NULL`
    /// - `dependencies_acknowledged = 0`
    /// - `auto_accept_status = NULL`
    /// - `auto_accept_started_at = NULL`
    /// - `cross_project_checked = 0`
    ///
    /// Called by `SessionReopenService` before resetting status to Active.
    async fn reset_acceptance_cycle_fields(&self, session_id: &str) -> AppResult<()>;

    /// Bump `updated_at` for a session without changing any other field.
    ///
    /// Called after ideation message creation so that active sessions with
    /// ongoing conversations are not incorrectly archived by the staleness
    /// reconciler (which filters on `updated_at < cutoff`).
    async fn touch_updated_at(&self, session_id: &str) -> AppResult<()>;

    /// Set (or clear) `pending_initial_prompt` on a session.
    ///
    /// Pass `Some(prompt)` to persist the deferred launch prompt;
    /// pass `None` to clear it after the drain service successfully launches the session.
    async fn set_pending_initial_prompt(
        &self,
        session_id: &str,
        prompt: Option<String>,
    ) -> AppResult<()>;

    /// Atomically claim the oldest pending session for a project.
    ///
    /// Uses `BEGIN IMMEDIATE` to prevent two concurrent drain services from
    /// claiming the same session. Selects the oldest active session with a
    /// non-null `pending_initial_prompt` for the given project, clears the field
    /// to NULL, and returns `(session_id, prompt)`. Returns `None` if no
    /// pending session exists.
    ///
    /// WHERE clause includes `status = 'active'` to exclude archived/accepted sessions.
    async fn claim_pending_session_for_project(
        &self,
        project_id: &str,
    ) -> AppResult<Option<(String, String)>>;

    /// List all project IDs that have at least one active session with a
    /// non-null `pending_initial_prompt`.
    ///
    /// Used by `StartupJobRunner` to discover which projects need draining on
    /// app restart. The WHERE clause includes `status = 'active'` to exclude
    /// stale rows on archived sessions.
    async fn list_projects_with_pending_sessions(&self) -> AppResult<Vec<String>>;
}

#[cfg(test)]
#[path = "ideation_session_repository_tests.rs"]
mod tests;
