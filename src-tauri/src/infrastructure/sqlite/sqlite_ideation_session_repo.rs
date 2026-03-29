// SQLite-based IdeationSessionRepository implementation for production use
// Uses DbConnection (spawn_blocking) for non-blocking rusqlite access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::Connection;

use crate::domain::entities::{
    AcceptanceStatus, IdeationSession, IdeationSessionId, IdeationSessionStatus, ProjectId,
    VerificationMetadata, VerificationStatus,
};
use crate::domain::repositories::ideation_session_repository::{
    IdeationSessionWithProgress, SessionGroupCounts, SessionProgress,
};
use crate::domain::repositories::IdeationSessionRepository;
use crate::error::{AppError, AppResult};

use super::DbConnection;

// Status classification constants — keep in sync with src/types/status.ts:47-82 (categorizeStatus)
// IDLE: tasks that haven't started yet
const _IDLE_STATUSES: &[&str] = &["backlog", "ready", "blocked"];

/// All 39 SELECT columns for IdeationSession — single source of truth (DRY).
/// Must be kept in sync with IdeationSession::from_row column names.
/// Column order: id(0)..origin(24), expected_proposal_count(25), auto_accept_status(26),
/// auto_accept_started_at(27), api_key_id(28), idempotency_key(29),
/// external_activity_phase(30), external_last_read_message_id(31), dependencies_acknowledged(32),
/// pending_initial_prompt(33), source_task_id(34), source_context_type(35),
/// source_context_id(36), spawn_reason(37), blocker_fingerprint(38)
const SESSION_COLUMNS: &str = "id, project_id, title, title_source, status, plan_artifact_id, \
    inherited_plan_artifact_id, seed_task_id, parent_session_id, created_at, \
    updated_at, archived_at, converted_at, team_mode, team_config_json, \
    verification_status, verification_in_progress, verification_metadata, \
    verification_generation, source_project_id, source_session_id, session_purpose, \
    cross_project_checked, plan_version_last_read, origin, \
    expected_proposal_count, auto_accept_status, auto_accept_started_at, \
    api_key_id, idempotency_key, external_activity_phase, external_last_read_message_id, \
    dependencies_acknowledged, pending_initial_prompt, source_task_id, source_context_type, \
    source_context_id, spawn_reason, blocker_fingerprint, acceptance_status";
// TERMINAL: tasks that have reached a final state
const _TERMINAL_STATUSES: &[&str] = &["approved", "merged", "failed", "cancelled", "stopped"];
// ACTIVE: any status NOT in IDLE or TERMINAL (catch-all, matches categorizeStatus() logic)
// The SQL queries use NOT IN ('backlog','ready','blocked','approved','merged','failed','cancelled','stopped')
// to identify active statuses implicitly.

/// SQLite implementation of IdeationSessionRepository for production use
pub struct SqliteIdeationSessionRepository {
    db: DbConnection,
}

impl SqliteIdeationSessionRepository {
    /// Create a new SQLite ideation session repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            db: DbConnection::new(conn),
        }
    }

    /// Create from an Arc-wrapped mutex connection (for sharing)
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            db: DbConnection::from_shared(conn),
        }
    }

    // ============================================================================
    // Sync helpers — pub(crate) methods containing SQL logic.
    // Part of the sync-helper pattern: batch callers (e.g., artifact HTTP handlers)
    // call these directly with &Connection inside a db.run_transaction() closure.
    // Async trait methods wrap these in db.run() for single-operation use.
    // ============================================================================

    /// Insert a session into the database synchronously (for use inside db.run() closures).
    /// Returns the inserted session unchanged (no re-fetch needed — all fields are set by caller).
    #[doc(hidden)]
    pub fn insert_sync(
        conn: &Connection,
        session: &IdeationSession,
    ) -> AppResult<IdeationSession> {
        conn.execute(
            "INSERT INTO ideation_sessions \
             (id, project_id, title, title_source, status, plan_artifact_id, \
              inherited_plan_artifact_id, seed_task_id, parent_session_id, created_at, \
             updated_at, archived_at, converted_at, team_mode, team_config_json, \
              verification_status, source_project_id, source_session_id, session_purpose, \
              cross_project_checked, origin, api_key_id, idempotency_key, \
              external_activity_phase, external_last_read_message_id, dependencies_acknowledged, \
              pending_initial_prompt, source_task_id, source_context_type, source_context_id, spawn_reason, blocker_fingerprint) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32)",
            rusqlite::params![
                session.id.as_str(),
                session.project_id.as_str(),
                session.title,
                session.title_source,
                session.status.to_string(),
                session.plan_artifact_id.as_ref().map(|id| id.as_str()),
                session.inherited_plan_artifact_id.as_ref().map(|id| id.as_str()),
                session.seed_task_id.as_ref().map(|id| id.as_str()),
                session.parent_session_id.as_ref().map(|id| id.as_str()),
                session.created_at.to_rfc3339(),
                session.updated_at.to_rfc3339(),
                session.archived_at.map(|dt| dt.to_rfc3339()),
                session.converted_at.map(|dt| dt.to_rfc3339()),
                session.team_mode,
                session.team_config_json,
                session.verification_status.to_string(),
                session.source_project_id,
                session.source_session_id,
                session.session_purpose.to_string(),
                session.cross_project_checked as i32,
                session.origin.to_string(),
                session.api_key_id.as_deref(),
                session.idempotency_key.as_deref(),
                session.external_activity_phase.as_deref(),
                session.external_last_read_message_id.as_deref(),
                session.dependencies_acknowledged as i32,
                session.pending_initial_prompt.as_deref(),
                session.source_task_id.as_ref().map(|id| id.as_str()),
                session.source_context_type.as_deref(),
                session.source_context_id.as_deref(),
                session.spawn_reason.as_deref(),
                session.blocker_fingerprint.as_deref(),
            ],
        )?;
        Ok(session.clone())
    }

    /// Fetch a single session by ID; returns None if not found.
    #[doc(hidden)]
    pub fn get_by_id_sync(
        conn: &Connection,
        id: &str,
    ) -> AppResult<Option<IdeationSession>> {
        let sql = format!("SELECT {} FROM ideation_sessions WHERE id = ?1", SESSION_COLUMNS);
        match conn.query_row(&sql, [id], |row| IdeationSession::from_row(row)) {
            Ok(session) => Ok(Some(session)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    /// Fetch sessions by their own plan_artifact_id (not inherited).
    pub(crate) fn get_by_plan_artifact_id_sync(
        conn: &Connection,
        plan_artifact_id: &str,
    ) -> AppResult<Vec<IdeationSession>> {
        let sql = format!("SELECT {} FROM ideation_sessions WHERE plan_artifact_id = ?1", SESSION_COLUMNS);
        let mut stmt = conn.prepare(&sql)?;
        let sessions = stmt
            .query_map([plan_artifact_id], IdeationSession::from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(sessions)
    }

    /// Fetch sessions by their inherited_plan_artifact_id.
    pub(crate) fn get_by_inherited_plan_artifact_id_sync(
        conn: &Connection,
        artifact_id: &str,
    ) -> AppResult<Vec<IdeationSession>> {
        let sql = format!("SELECT {} FROM ideation_sessions WHERE inherited_plan_artifact_id = ?1", SESSION_COLUMNS);
        let mut stmt = conn.prepare(&sql)?;
        let sessions = stmt
            .query_map([artifact_id], IdeationSession::from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(sessions)
    }

    /// Update the plan_artifact_id for a single session.
    pub(crate) fn update_plan_artifact_id_sync(
        conn: &Connection,
        id: &str,
        plan_artifact_id: Option<&str>,
    ) -> AppResult<()> {
        let now = Utc::now();
        conn.execute(
            "UPDATE ideation_sessions SET plan_artifact_id = ?2, updated_at = ?3 WHERE id = ?1",
            rusqlite::params![id, plan_artifact_id, now.to_rfc3339()],
        )?;
        Ok(())
    }

    /// Update plan_artifact_id for a batch of sessions in one pass (no per-row lock).
    /// Uses a single UPDATE with WHERE id IN (...) instead of per-row statements.
    pub(crate) fn batch_update_artifact_id_sync(
        conn: &Connection,
        session_ids: &[String],
        new_artifact_id: &str,
    ) -> AppResult<()> {
        if session_ids.is_empty() {
            return Ok(());
        }
        let now = Utc::now().to_rfc3339();
        let placeholders: Vec<String> = (0..session_ids.len())
            .map(|i| format!("?{}", i + 3))
            .collect();
        let sql = format!(
            "UPDATE ideation_sessions SET plan_artifact_id = ?1, updated_at = ?2 \
             WHERE id IN ({})",
            placeholders.join(", ")
        );
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::with_capacity(2 + session_ids.len());
        params.push(Box::new(new_artifact_id.to_string()));
        params.push(Box::new(now));
        for id in session_ids {
            params.push(Box::new(id.clone()));
        }
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice())?;
        Ok(())
    }

    /// Reset verification status to 'unverified' if verification is not in progress.
    /// Returns true if the row was updated (verification was reset).
    /// ImportedVerified sessions are never reset — their pre-verified status must be preserved.
    pub(crate) fn reset_verification_sync(conn: &Connection, id: &str) -> AppResult<bool> {
        let now = Utc::now();
        let rows = conn.execute(
            "UPDATE ideation_sessions SET verification_status = 'unverified', \
             verification_in_progress = 0, verification_metadata = NULL, \
             updated_at = ?2 WHERE id = ?1 AND verification_in_progress = 0 \
             AND verification_status != 'imported_verified'",
            rusqlite::params![id, now.to_rfc3339()],
        )?;
        Ok(rows > 0)
    }

    /// Atomically trigger auto-verification: sets verification_status=reviewing,
    /// in_progress=1, and increments verification_generation — only when in_progress=0.
    ///
    /// ImportedVerified sessions are never auto-verified — their pre-verified status is preserved.
    ///
    /// Returns `Some(new_generation)` if the trigger was applied, `None` if already in_progress
    /// or if the session is `imported_verified`.
    #[doc(hidden)]
    pub fn trigger_auto_verify_sync(
        conn: &Connection,
        id: &str,
    ) -> AppResult<Option<i32>> {
        let now = Utc::now();
        let rows = conn.execute(
            "UPDATE ideation_sessions SET \
             verification_status = 'reviewing', \
             verification_in_progress = 1, \
             verification_generation = verification_generation + 1, \
             updated_at = ?2 \
             WHERE id = ?1 AND verification_in_progress = 0 \
             AND verification_status != 'imported_verified'",
            rusqlite::params![id, now.to_rfc3339()],
        )?;
        if rows == 0 {
            return Ok(None);
        }
        let generation: i32 = conn.query_row(
            "SELECT verification_generation FROM ideation_sessions WHERE id = ?1",
            [id],
            |row| row.get(0),
        )?;
        Ok(Some(generation))
    }

    /// Reset auto-verify state after a spawn failure.
    /// Sets in_progress=0 and verification_status=unverified.
    /// ImportedVerified sessions are never reset — their pre-verified status must be preserved.
    #[doc(hidden)]
    pub fn reset_auto_verify_sync(conn: &Connection, id: &str) -> AppResult<()> {
        let now = Utc::now();
        conn.execute(
            "UPDATE ideation_sessions SET \
             verification_in_progress = 0, \
             verification_status = 'unverified', \
             updated_at = ?2 \
             WHERE id = ?1 AND verification_status != 'imported_verified'",
            rusqlite::params![id, now.to_rfc3339()],
        )?;
        Ok(())
    }

    /// Update the plan_version_last_read field for a session (sync, for use inside db.run() closures).
    #[allow(dead_code)]
    pub(crate) fn update_plan_version_last_read_sync(
        conn: &Connection,
        session_id: &str,
        version: i32,
    ) -> AppResult<()> {
        conn.execute(
            "UPDATE ideation_sessions SET plan_version_last_read = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![version, Utc::now().to_rfc3339(), session_id],
        )?;
        Ok(())
    }

    /// Validate that creating a cross-project session from `source_session_id` into
    /// `target_project_id` does not introduce a circular import chain.
    ///
    /// Walks the chain: source_session → source_session.source_session_id → … up to `max_depth`.
    ///
    /// **MUST use `get_by_id_sync` (not async `get_by_id`)** — this runs inside a `db.run()`
    /// closure alongside the INSERT; using the async variant would deadlock.
    ///
    /// Error codes embedded in the message (checked by callers / tests):
    /// - `SELF_REFERENCE` — source is in the same project as the target
    /// - `CIRCULAR_IMPORT` — chain leads back to the target project
    /// - `CHAIN_TOO_DEEP` — chain exceeds `max_depth` (default: 10)
    #[doc(hidden)]
    pub fn validate_no_circular_import_sync(
        conn: &Connection,
        source_session_id: &str,
        target_project_id: &str,
        max_depth: usize,
    ) -> AppResult<()> {
        // Fetch the source session to check its project membership
        let source_session = Self::get_by_id_sync(conn, source_session_id)?
            .ok_or_else(|| AppError::Validation(format!("Source session not found: {source_session_id}")))?;

        // SELF_REFERENCE: source session is in the target project (can't import from yourself)
        if source_session.project_id.as_str() == target_project_id {
            return Err(AppError::Validation(
                "SELF_REFERENCE: source session belongs to the same project as the target".to_string(),
            ));
        }

        // Walk the import chain: source.source_session_id → grandparent → …
        let mut current_source_id = source_session.source_session_id;
        let mut depth = 0usize;

        while let Some(ref chain_id) = current_source_id {
            depth += 1;
            if depth >= max_depth {
                return Err(AppError::Validation(format!(
                    "CHAIN_TOO_DEEP: import chain exceeds maximum depth of {max_depth}"
                )));
            }

            match Self::get_by_id_sync(conn, chain_id)? {
                None => {
                    // Dangling reference — source was deleted; chain ends gracefully
                    break;
                }
                Some(ancestor) => {
                    if ancestor.project_id.as_str() == target_project_id {
                        return Err(AppError::Validation(
                            "CIRCULAR_IMPORT: import chain creates a cycle through the target project".to_string(),
                        ));
                    }
                    current_source_id = ancestor.source_session_id;
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl IdeationSessionRepository for SqliteIdeationSessionRepository {
    async fn create(&self, session: IdeationSession) -> AppResult<IdeationSession> {
        self.db
            .run(move |conn| Self::insert_sync(conn, &session))
            .await
    }

    async fn get_by_id(&self, id: &IdeationSessionId) -> AppResult<Option<IdeationSession>> {
        let id = id.as_str().to_string();
        self.db
            .query_optional(move |conn| {
                let sql = format!("SELECT {} FROM ideation_sessions WHERE id = ?1", SESSION_COLUMNS);
                conn.query_row(&sql, [&id], |row| IdeationSession::from_row(row))
            })
            .await
    }

    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<IdeationSession>> {
        let project_id = project_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let sql = format!("SELECT {} FROM ideation_sessions WHERE project_id = ?1 AND archived_at IS NULL ORDER BY updated_at DESC", SESSION_COLUMNS);
                let mut stmt = conn.prepare(&sql)?;
                let sessions = stmt
                    .query_map([&project_id], IdeationSession::from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(sessions)
            })
            .await
    }

    async fn update_status(
        &self,
        id: &IdeationSessionId,
        status: IdeationSessionStatus,
    ) -> AppResult<()> {
        let id = id.as_str().to_string();
        let now = Utc::now();

        self.db
            .run(move |conn| {
                let query = match status {
                    IdeationSessionStatus::Archived => {
                        "UPDATE ideation_sessions SET status = ?2, updated_at = ?3, archived_at = ?4, verification_in_progress = 0, pending_initial_prompt = NULL WHERE id = ?1"
                    }
                    IdeationSessionStatus::Accepted => {
                        "UPDATE ideation_sessions SET status = ?2, updated_at = ?3, converted_at = ?4, pending_initial_prompt = NULL WHERE id = ?1"
                    }
                    IdeationSessionStatus::Active => {
                        "UPDATE ideation_sessions SET status = ?2, updated_at = ?3, archived_at = NULL, converted_at = NULL WHERE id = ?1"
                    }
                };

                match status {
                    IdeationSessionStatus::Archived | IdeationSessionStatus::Accepted => {
                        conn.execute(
                            query,
                            rusqlite::params![
                                id,
                                status.to_string(),
                                now.to_rfc3339(),
                                now.to_rfc3339(),
                            ],
                        )?;
                    }
                    IdeationSessionStatus::Active => {
                        conn.execute(
                            query,
                            rusqlite::params![id, status.to_string(), now.to_rfc3339()],
                        )?;
                    }
                }

                Ok(())
            })
            .await
    }

    async fn update_title(
        &self,
        id: &IdeationSessionId,
        title: Option<String>,
        title_source: &str,
    ) -> AppResult<()> {
        let id = id.as_str().to_string();
        let title_source = title_source.to_string();
        let now = Utc::now();

        self.db
            .run(move |conn| {
                conn.execute(
                    "UPDATE ideation_sessions SET title = ?2, title_source = ?3, updated_at = ?4 WHERE id = ?1",
                    rusqlite::params![id, title, title_source, now.to_rfc3339()],
                )?;
                Ok(())
            })
            .await
    }

    async fn update_plan_artifact_id(
        &self,
        id: &IdeationSessionId,
        plan_artifact_id: Option<String>,
    ) -> AppResult<()> {
        let id = id.as_str().to_string();
        let now = Utc::now();

        self.db
            .run(move |conn| {
                conn.execute(
                    "UPDATE ideation_sessions SET plan_artifact_id = ?2, updated_at = ?3 WHERE id = ?1",
                    rusqlite::params![id, plan_artifact_id, now.to_rfc3339()],
                )?;
                Ok(())
            })
            .await
    }

    async fn delete(&self, id: &IdeationSessionId) -> AppResult<()> {
        let id = id.as_str().to_string();
        self.db
            .run(move |conn| {
                // CASCADE is defined in the schema, so deleting the session
                // will automatically delete related proposals and messages
                conn.execute("DELETE FROM ideation_sessions WHERE id = ?1", [id])?;
                Ok(())
            })
            .await
    }

    async fn get_active_by_project(
        &self,
        project_id: &ProjectId,
    ) -> AppResult<Vec<IdeationSession>> {
        let project_id = project_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let sql = format!("SELECT {} FROM ideation_sessions WHERE project_id = ?1 AND status = 'active' ORDER BY updated_at DESC", SESSION_COLUMNS);
                let mut stmt = conn.prepare(&sql)?;
                let sessions = stmt
                    .query_map([&project_id], IdeationSession::from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(sessions)
            })
            .await
    }

    async fn count_by_status(
        &self,
        project_id: &ProjectId,
        status: IdeationSessionStatus,
    ) -> AppResult<u32> {
        let project_id = project_id.as_str().to_string();
        let status_str = status.to_string();
        self.db
            .run(move |conn| {
                let count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM ideation_sessions WHERE project_id = ?1 AND status = ?2",
                    rusqlite::params![project_id, status_str],
                    |row| row.get(0),
                )?;
                Ok(count as u32)
            })
            .await
    }

    async fn get_by_plan_artifact_id(
        &self,
        plan_artifact_id: &str,
    ) -> AppResult<Vec<IdeationSession>> {
        let plan_artifact_id = plan_artifact_id.to_string();
        self.db
            .run(move |conn| {
                let sql = format!("SELECT {} FROM ideation_sessions WHERE plan_artifact_id = ?1", SESSION_COLUMNS);
                let mut stmt = conn.prepare(&sql)?;
                let sessions = stmt
                    .query_map([&plan_artifact_id], IdeationSession::from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(sessions)
            })
            .await
    }

    async fn get_by_inherited_plan_artifact_id(
        &self,
        artifact_id: &str,
    ) -> AppResult<Vec<IdeationSession>> {
        let artifact_id = artifact_id.to_string();
        self.db
            .run(move |conn| {
                let sql = format!("SELECT {} FROM ideation_sessions WHERE inherited_plan_artifact_id = ?1", SESSION_COLUMNS);
                let mut stmt = conn.prepare(&sql)?;
                let sessions = stmt
                    .query_map([&artifact_id], IdeationSession::from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(sessions)
            })
            .await
    }

    async fn get_children(&self, parent_id: &IdeationSessionId) -> AppResult<Vec<IdeationSession>> {
        let parent_id = parent_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let sql = format!("SELECT {} FROM ideation_sessions WHERE parent_session_id = ?1 ORDER BY created_at DESC", SESSION_COLUMNS);
                let mut stmt = conn.prepare(&sql)?;
                let sessions = stmt
                    .query_map([&parent_id], IdeationSession::from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(sessions)
            })
            .await
    }

    async fn get_ancestor_chain(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<Vec<IdeationSession>> {
        let session_id = session_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let mut chain = Vec::new();
                let mut current_id = session_id;

                // Walk up the parent chain iteratively
                loop {
                    let ancestor_sql = format!("SELECT {} FROM ideation_sessions WHERE id = ?1", SESSION_COLUMNS);
                    let result = conn.query_row(&ancestor_sql, [&current_id], |row| IdeationSession::from_row(row));

                    match result {
                        Ok(session) => {
                            if let Some(parent_id) = &session.parent_session_id {
                                let parent_id_str = parent_id.as_str().to_string();
                                current_id = parent_id_str.clone();
                                match conn.query_row(&ancestor_sql, [&parent_id_str], |row| IdeationSession::from_row(row)) {
                                    Ok(parent) => {
                                        chain.push(parent);
                                    }
                                    Err(rusqlite::Error::QueryReturnedNoRows) => {
                                        break;
                                    }
                                    Err(e) => {
                                        return Err(AppError::Database(e.to_string()));
                                    }
                                }
                            } else {
                                break;
                            }
                        }
                        Err(rusqlite::Error::QueryReturnedNoRows) => {
                            break;
                        }
                        Err(e) => {
                            return Err(AppError::Database(e.to_string()));
                        }
                    }
                }

                Ok(chain)
            })
            .await
    }

    async fn set_parent(
        &self,
        id: &IdeationSessionId,
        parent_id: Option<&IdeationSessionId>,
    ) -> AppResult<()> {
        let id = id.as_str().to_string();
        let parent_id = parent_id.map(|p| p.as_str().to_string());
        let now = Utc::now();

        self.db
            .run(move |conn| {
                conn.execute(
                    "UPDATE ideation_sessions SET parent_session_id = ?2, updated_at = ?3 WHERE id = ?1",
                    rusqlite::params![id, parent_id, now.to_rfc3339()],
                )?;
                Ok(())
            })
            .await
    }

    async fn update_verification_state(
        &self,
        id: &IdeationSessionId,
        status: VerificationStatus,
        in_progress: bool,
        metadata_json: Option<String>,
    ) -> AppResult<()> {
        let id = id.as_str().to_string();
        let status_str = status.to_string();
        let in_progress_int: i64 = if in_progress { 1 } else { 0 };
        let now = Utc::now();
        self.db
            .run(move |conn| {
                conn.execute(
                    "UPDATE ideation_sessions SET verification_status = ?2, verification_in_progress = ?3, verification_metadata = ?4, updated_at = ?5 WHERE id = ?1",
                    rusqlite::params![id, status_str, in_progress_int, metadata_json, now.to_rfc3339()],
                )?;
                Ok(())
            })
            .await
    }

    async fn reset_verification(&self, id: &IdeationSessionId) -> AppResult<bool> {
        let id = id.as_str().to_string();
        let now = Utc::now();

        self.db
            .run(move |conn| {
                let rows = conn.execute(
                    "UPDATE ideation_sessions SET verification_status = 'unverified', verification_in_progress = 0, verification_metadata = NULL, updated_at = ?2 WHERE id = ?1 AND verification_in_progress = 0",
                    rusqlite::params![id, now.to_rfc3339()],
                )?;
                Ok(rows > 0)
            })
            .await
    }

    async fn reset_and_begin_reverify(
        &self,
        session_id: &str,
    ) -> AppResult<(i32, VerificationMetadata)> {
        let session_id = session_id.to_string();
        self.db
            .run_transaction(move |conn| {
                // Read current metadata + generation
                let (metadata_json, current_gen): (Option<String>, i32) = conn.query_row(
                    "SELECT verification_metadata, verification_generation FROM ideation_sessions WHERE id = ?1",
                    rusqlite::params![session_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                ).map_err(|e| crate::error::AppError::Database(format!("Session not found: {e}")))?;

                // Parse existing metadata (or use default), then clear all stale fields
                let mut metadata: VerificationMetadata = metadata_json
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or_default();
                metadata.current_gaps = vec![];
                metadata.rounds = vec![];
                metadata.convergence_reason = None;
                metadata.best_round_index = None;
                metadata.current_round = 0;
                metadata.parse_failures = vec![];

                let new_metadata_json = serde_json::to_string(&metadata)
                    .map_err(|e| crate::error::AppError::Database(format!("Metadata serialize failed: {e}")))?;
                let new_gen = current_gen + 1;

                // Atomic: clear metadata + increment generation + set status + set in_progress
                let rows_affected = conn.execute(
                    "UPDATE ideation_sessions SET \
                       verification_status = 'reviewing', \
                       verification_in_progress = 1, \
                       verification_generation = ?2, \
                       verification_metadata = ?3, \
                       updated_at = CURRENT_TIMESTAMP \
                     WHERE id = ?1",
                    rusqlite::params![session_id, new_gen, new_metadata_json],
                )?;

                if rows_affected == 0 {
                    return Err(crate::error::AppError::Database(
                        format!("Session not found: {}", session_id),
                    ));
                }

                Ok((new_gen, metadata))
            })
            .await
    }

    async fn get_verification_status(
        &self,
        id: &IdeationSessionId,
    ) -> AppResult<Option<(VerificationStatus, bool, Option<String>)>> {
        let id = id.as_str().to_string();
        self.db
            .query_optional(move |conn| {
                conn.query_row(
                    "SELECT verification_status, verification_in_progress, verification_metadata FROM ideation_sessions WHERE id = ?1",
                    [&id],
                    |row| {
                        let status_str: Option<String> = row.get(0).unwrap_or(None);
                        let in_progress: Option<i64> = row.get(1).unwrap_or(None);
                        let metadata: Option<String> = row.get(2).unwrap_or(None);
                        let status = status_str
                            .as_deref()
                            .and_then(|s| s.parse().ok())
                            .unwrap_or_default();
                        let in_prog = in_progress.map(|v| v != 0).unwrap_or(false);
                        Ok((status, in_prog, metadata))
                    },
                )
            })
            .await
    }

    async fn revert_plan_and_skip_verification(
        &self,
        id: &IdeationSessionId,
        new_plan_artifact_id: String,
        convergence_reason: String,
    ) -> AppResult<()> {
        let id = id.as_str().to_string();
        let now = Utc::now();
        let metadata_json = serde_json::json!({
            "v": 1,
            "current_round": 0,
            "max_rounds": 0,
            "rounds": [],
            "current_gaps": [],
            "convergence_reason": convergence_reason,
            "best_round_index": null,
            "parse_failures": []
        })
        .to_string();
        self.db
            .run(move |conn| {
                conn.execute(
                    "UPDATE ideation_sessions SET plan_artifact_id = ?2, verification_status = 'skipped', verification_in_progress = 0, verification_metadata = ?3, updated_at = ?4, verification_generation = verification_generation + 1 WHERE id = ?1",
                    rusqlite::params![id, new_plan_artifact_id, metadata_json, now.to_rfc3339()],
                )?;
                Ok(())
            })
            .await
    }

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
    ) -> AppResult<()> {
        let session_id = session_id.as_str().to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        let artifact_metadata_json = serde_json::json!({
            "created_at": now_str,
            "created_by": "system",
            "version": version,
        })
        .to_string();

        let session_metadata_json = serde_json::json!({
            "v": 1,
            "current_round": 0,
            "max_rounds": 0,
            "rounds": [],
            "current_gaps": [],
            "convergence_reason": convergence_reason,
            "best_round_index": null,
            "parse_failures": []
        })
        .to_string();

        let artifact_id_clone = new_artifact_id.clone();

        self.db
            .run(move |conn| {
                conn.execute(
                    "INSERT INTO artifacts \
                     (id, type, name, content_type, content_text, content_path, \
                      bucket_id, task_id, process_id, created_by, version, \
                      previous_version_id, created_at, metadata_json) \
                     VALUES (?1, ?2, ?3, 'inline', ?4, NULL, \
                             NULL, NULL, NULL, 'system', ?5, \
                             ?6, ?7, ?8)",
                    rusqlite::params![
                        new_artifact_id,
                        artifact_type_str,
                        artifact_name,
                        content_text,
                        version,
                        previous_version_id,
                        now_str,
                        artifact_metadata_json,
                    ],
                )?;

                conn.execute(
                    "UPDATE ideation_sessions \
                     SET plan_artifact_id = ?2, \
                         verification_status = 'skipped', \
                         verification_in_progress = 0, \
                         verification_metadata = ?3, \
                         updated_at = ?4, \
                         verification_generation = verification_generation + 1 \
                     WHERE id = ?1",
                    rusqlite::params![
                        session_id,
                        artifact_id_clone,
                        session_metadata_json,
                        now.to_rfc3339(),
                    ],
                )?;
                Ok(())
            })
            .await
    }

    async fn increment_verification_generation(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<()> {
        let id = session_id.as_str().to_string();
        self.db
            .run(move |conn| {
                conn.execute(
                    "UPDATE ideation_sessions SET verification_generation = verification_generation + 1 WHERE id = ?1",
                    rusqlite::params![id],
                )?;
                Ok(())
            })
            .await
    }

    async fn get_stale_in_progress_sessions(
        &self,
        stale_before: DateTime<Utc>,
    ) -> AppResult<Vec<IdeationSession>> {
        let stale_before_str = stale_before.to_rfc3339();
        self.db
            .run(move |conn| {
                let sql = format!("SELECT {} FROM ideation_sessions WHERE verification_in_progress = 1 AND updated_at < ?1 AND status != 'archived'", SESSION_COLUMNS);
                let mut stmt = conn.prepare(&sql)?;
                let sessions = stmt
                    .query_map([&stale_before_str], IdeationSession::from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(sessions)
            })
            .await
    }

    async fn get_all_in_progress_sessions(&self) -> AppResult<Vec<IdeationSession>> {
        self.db
            .run(move |conn| {
                let sql = format!("SELECT {} FROM ideation_sessions WHERE verification_in_progress = 1 AND status != 'archived'", SESSION_COLUMNS);
                let mut stmt = conn.prepare(&sql)?;
                let sessions = stmt
                    .query_map([], IdeationSession::from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(sessions)
            })
            .await
    }

    async fn get_verification_children(
        &self,
        parent_session_id: &IdeationSessionId,
    ) -> AppResult<Vec<IdeationSession>> {
        let param_str = parent_session_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let sql = format!("SELECT {} FROM ideation_sessions WHERE parent_session_id = ?1 AND session_purpose = 'verification' AND status != 'archived' ORDER BY created_at DESC LIMIT 1", SESSION_COLUMNS);
                let mut stmt = conn.prepare(&sql)?;
                let sessions = stmt
                    .query_map([&param_str], IdeationSession::from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(sessions)
            })
            .await
    }

    async fn get_by_project_and_status(
        &self,
        project_id: &str,
        status: &str,
        limit: u32,
    ) -> AppResult<Vec<IdeationSession>> {
        let project_id = project_id.to_string();
        let status = status.to_string();
        self.db
            .run(move |conn| {
                let sql = format!("SELECT {} FROM ideation_sessions WHERE project_id = ?1 AND status = ?2 ORDER BY created_at DESC LIMIT ?3", SESSION_COLUMNS);
                let mut stmt = conn.prepare(&sql)?;
                let sessions = stmt
                    .query_map(rusqlite::params![project_id, status, limit], IdeationSession::from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(sessions)
            })
            .await
    }

    async fn get_group_counts(
        &self,
        project_id: &ProjectId,
        search: Option<&str>,
    ) -> AppResult<SessionGroupCounts> {
        let project_id = project_id.as_str().to_string();
        // Normalize empty string to None
        let search = search.filter(|s| !s.is_empty()).map(|s| {
            format!("%{}%", s.replace('%', "\\%").replace('_', "\\_"))
        });
        self.db
            .run(move |conn| {
                let search_clause = if search.is_some() {
                    " AND s.title LIKE ?2 ESCAPE '\\' COLLATE NOCASE"
                } else {
                    ""
                };
                let sql = format!(
                    "SELECT \
                      COALESCE(SUM(CASE WHEN s.status = 'active' THEN 1 ELSE 0 END), 0) as drafts, \
                      COALESCE(SUM(CASE WHEN s.status = 'archived' THEN 1 ELSE 0 END), 0) as archived, \
                      COALESCE(SUM(CASE WHEN s.status = 'accepted' THEN 1 ELSE 0 END), 0) as total_accepted, \
                      COALESCE(SUM(CASE WHEN s.status = 'accepted' AND EXISTS ( \
                        SELECT 1 FROM tasks t WHERE t.ideation_session_id = s.id \
                          AND t.internal_status NOT IN ('backlog','ready','blocked','approved','merged','failed','cancelled','stopped') \
                      ) THEN 1 ELSE 0 END), 0) as in_progress, \
                      COALESCE(SUM(CASE WHEN s.status = 'accepted' \
                        AND EXISTS (SELECT 1 FROM tasks t2 WHERE t2.ideation_session_id = s.id) \
                        AND NOT EXISTS ( \
                          SELECT 1 FROM tasks t3 WHERE t3.ideation_session_id = s.id \
                            AND t3.internal_status NOT IN ('approved','merged','failed','cancelled','stopped') \
                        ) \
                      THEN 1 ELSE 0 END), 0) as done \
                    FROM ideation_sessions s \
                    WHERE s.project_id = ?1 \
                      AND (s.session_purpose IS NULL OR s.session_purpose = 'general'){}",
                    search_clause
                );
                let row = if let Some(ref pattern) = search {
                    conn.query_row(&sql, rusqlite::params![project_id, pattern], |row| {
                        let drafts: u32 = row.get::<_, i64>(0)? as u32;
                        let archived: u32 = row.get::<_, i64>(1)? as u32;
                        let total_accepted: u32 = row.get::<_, i64>(2)? as u32;
                        let in_progress: u32 = row.get::<_, i64>(3)? as u32;
                        let done: u32 = row.get::<_, i64>(4)? as u32;
                        let accepted = total_accepted.saturating_sub(in_progress).saturating_sub(done);
                        Ok(SessionGroupCounts {
                            drafts,
                            in_progress,
                            accepted,
                            done,
                            archived,
                        })
                    })?
                } else {
                    conn.query_row(&sql, [&project_id], |row| {
                        let drafts: u32 = row.get::<_, i64>(0)? as u32;
                        let archived: u32 = row.get::<_, i64>(1)? as u32;
                        let total_accepted: u32 = row.get::<_, i64>(2)? as u32;
                        let in_progress: u32 = row.get::<_, i64>(3)? as u32;
                        let done: u32 = row.get::<_, i64>(4)? as u32;
                        let accepted = total_accepted.saturating_sub(in_progress).saturating_sub(done);
                        Ok(SessionGroupCounts {
                            drafts,
                            in_progress,
                            accepted,
                            done,
                            archived,
                        })
                    })?
                };
                Ok(row)
            })
            .await
    }

    async fn list_by_group(
        &self,
        project_id: &ProjectId,
        group: &str,
        offset: u32,
        limit: u32,
        search: Option<&str>,
    ) -> AppResult<(Vec<IdeationSessionWithProgress>, u32)> {
        let project_id = project_id.as_str().to_string();
        let group = group.to_string();
        // Normalize empty string to None
        let search = search.filter(|s| !s.is_empty()).map(|s| {
            format!("%{}%", s.replace('%', "\\%").replace('_', "\\_"))
        });

        self.db
            .run(move |conn| {
                // Validate group and build WHERE clause
                // Note: all variants include session_purpose filter to exclude verification child sessions
                let where_clause = match group.as_str() {
                    "drafts" => "s.status = 'active' AND s.project_id = ?1 \
                         AND (s.session_purpose IS NULL OR s.session_purpose = 'general')",
                    "archived" => "s.status = 'archived' AND s.project_id = ?1 \
                         AND (s.session_purpose IS NULL OR s.session_purpose = 'general')",
                    "in_progress" => {
                        "s.status = 'accepted' AND s.project_id = ?1 \
                         AND (s.session_purpose IS NULL OR s.session_purpose = 'general') \
                         AND EXISTS (SELECT 1 FROM tasks t WHERE t.ideation_session_id = s.id \
                           AND t.internal_status NOT IN ('backlog','ready','blocked','approved','merged','failed','cancelled','stopped'))"
                    }
                    "done" => {
                        "s.status = 'accepted' AND s.project_id = ?1 \
                         AND (s.session_purpose IS NULL OR s.session_purpose = 'general') \
                         AND EXISTS (SELECT 1 FROM tasks t WHERE t.ideation_session_id = s.id) \
                         AND NOT EXISTS (SELECT 1 FROM tasks t WHERE t.ideation_session_id = s.id \
                           AND t.internal_status NOT IN ('approved','merged','failed','cancelled','stopped'))"
                    }
                    "accepted" => {
                        "s.status = 'accepted' AND s.project_id = ?1 \
                         AND (s.session_purpose IS NULL OR s.session_purpose = 'general') \
                         AND NOT EXISTS (SELECT 1 FROM tasks t WHERE t.ideation_session_id = s.id \
                           AND t.internal_status NOT IN ('backlog','ready','blocked','approved','merged','failed','cancelled','stopped')) \
                         AND NOT (EXISTS (SELECT 1 FROM tasks t WHERE t.ideation_session_id = s.id) \
                           AND NOT EXISTS (SELECT 1 FROM tasks t WHERE t.ideation_session_id = s.id \
                             AND t.internal_status NOT IN ('approved','merged','failed','cancelled','stopped')))"
                    }
                    _ => {
                        return Err(AppError::Validation(format!(
                            "Unknown session group: '{}'. Valid groups: drafts, in_progress, accepted, done, archived",
                            group
                        )));
                    }
                };

                // Optional search filter appended after base WHERE clause
                let search_clause = if search.is_some() {
                    " AND s.title LIKE ?4 ESCAPE '\\' COLLATE NOCASE"
                } else {
                    ""
                };

                let include_progress = matches!(group.as_str(), "in_progress" | "accepted" | "done");

                // Count query (?1 = project_id, ?2 = search pattern when present)
                let count_sql = if search.is_some() {
                    format!(
                        "SELECT COUNT(*) FROM ideation_sessions s WHERE {} AND s.title LIKE ?2 ESCAPE '\\' COLLATE NOCASE",
                        where_clause
                    )
                } else {
                    format!(
                        "SELECT COUNT(*) FROM ideation_sessions s WHERE {}",
                        where_clause
                    )
                };
                let total: u32 = if let Some(ref pattern) = search {
                    conn.query_row(&count_sql, rusqlite::params![project_id, pattern], |row| {
                        row.get::<_, i64>(0)
                    })? as u32
                } else {
                    conn.query_row(&count_sql, [&project_id], |row| {
                        row.get::<_, i64>(0)
                    })? as u32
                };

                // Data query with LEFT JOIN for parent title and correlated subqueries for progress.
                // SESSION_COLUMNS are selected first, followed by:
                // parent_session_title, active_count, done_count, total_count,
                // verification_child_count, has_pending_prompt.
                // Params: ?1=project_id, ?2=offset, ?3=limit, ?4=search_pattern (when present)
                let data_sql = if include_progress {
                    format!(
                        "SELECT s.id, s.project_id, s.title, s.title_source, s.status, s.plan_artifact_id, \
                         s.inherited_plan_artifact_id, s.seed_task_id, s.parent_session_id, s.created_at, \
                         s.updated_at, s.archived_at, s.converted_at, s.team_mode, s.team_config_json, \
                         s.verification_status, s.verification_in_progress, s.verification_metadata, \
                         s.verification_generation, s.source_project_id, s.source_session_id, \
                         s.session_purpose, s.cross_project_checked, s.plan_version_last_read, s.origin, \
                         s.expected_proposal_count, s.auto_accept_status, s.auto_accept_started_at, \
                         s.api_key_id, s.idempotency_key, s.external_activity_phase, s.external_last_read_message_id, \
                         s.dependencies_acknowledged, s.pending_initial_prompt, s.source_task_id, s.source_context_type, \
                         s.source_context_id, s.spawn_reason, s.blocker_fingerprint, \
                         parent.title as parent_session_title, \
                         (SELECT COUNT(*) FROM tasks t WHERE t.ideation_session_id = s.id \
                           AND t.internal_status NOT IN ('backlog','ready','blocked','approved','merged','failed','cancelled','stopped')) as active_count, \
                         (SELECT COUNT(*) FROM tasks t WHERE t.ideation_session_id = s.id \
                           AND t.internal_status IN ('approved','merged','failed','cancelled','stopped')) as done_count, \
                         (SELECT COUNT(*) FROM tasks t WHERE t.ideation_session_id = s.id) as total_count, \
                         (SELECT COUNT(*) FROM ideation_sessions vc WHERE vc.parent_session_id = s.id AND vc.session_purpose = 'verification') as verification_child_count, \
                         (s.pending_initial_prompt IS NOT NULL AND s.status = 'active') as has_pending_prompt \
                         FROM ideation_sessions s \
                         LEFT JOIN ideation_sessions parent ON s.parent_session_id = parent.id \
                         WHERE {}{} \
                         ORDER BY s.updated_at DESC \
                         LIMIT ?3 OFFSET ?2",
                        where_clause, search_clause
                    )
                } else {
                    format!(
                        "SELECT s.id, s.project_id, s.title, s.title_source, s.status, s.plan_artifact_id, \
                         s.inherited_plan_artifact_id, s.seed_task_id, s.parent_session_id, s.created_at, \
                         s.updated_at, s.archived_at, s.converted_at, s.team_mode, s.team_config_json, \
                         s.verification_status, s.verification_in_progress, s.verification_metadata, \
                         s.verification_generation, s.source_project_id, s.source_session_id, \
                         s.session_purpose, s.cross_project_checked, s.plan_version_last_read, s.origin, \
                         s.expected_proposal_count, s.auto_accept_status, s.auto_accept_started_at, \
                         s.api_key_id, s.idempotency_key, s.external_activity_phase, s.external_last_read_message_id, \
                         s.dependencies_acknowledged, s.pending_initial_prompt, s.source_task_id, s.source_context_type, \
                         s.source_context_id, s.spawn_reason, s.blocker_fingerprint, \
                         parent.title as parent_session_title, \
                         NULL as active_count, NULL as done_count, NULL as total_count, \
                         (SELECT COUNT(*) FROM ideation_sessions vc WHERE vc.parent_session_id = s.id AND vc.session_purpose = 'verification') as verification_child_count, \
                         (s.pending_initial_prompt IS NOT NULL AND s.status = 'active') as has_pending_prompt \
                         FROM ideation_sessions s \
                         LEFT JOIN ideation_sessions parent ON s.parent_session_id = parent.id \
                         WHERE {}{} \
                         ORDER BY s.updated_at DESC \
                         LIMIT ?3 OFFSET ?2",
                        where_clause, search_clause
                    )
                };

                let row_mapper = |row: &rusqlite::Row<'_>| {
                    let session = IdeationSession::from_row(row)?;
                    let parent_session_title: Option<String> = row.get(39)?;
                    let active_count: Option<i64> = row.get(40)?;
                    let done_count: Option<i64> = row.get(41)?;
                    let total_count: Option<i64> = row.get(42)?;
                    let verification_child_count: i64 = row.get(43)?;
                    let has_pending_prompt: bool = row.get::<_, bool>(44)?;

                    let progress = if let (Some(active), Some(done_ct), Some(total)) =
                        (active_count, done_count, total_count)
                    {
                        let active = active as u32;
                        let done_ct = done_ct as u32;
                        let total = total as u32;
                        let idle = total.saturating_sub(active).saturating_sub(done_ct);
                        Some(SessionProgress {
                            idle,
                            active,
                            done: done_ct,
                            total,
                        })
                    } else {
                        None
                    };

                    Ok(IdeationSessionWithProgress {
                        session,
                        progress,
                        parent_session_title,
                        verification_child_count: verification_child_count as u32,
                        has_pending_prompt,
                    })
                };

                let mut stmt = conn.prepare(&data_sql)?;
                let sessions: Vec<IdeationSessionWithProgress> = if let Some(ref pattern) = search {
                    stmt
                        .query_map(rusqlite::params![project_id, offset, limit, pattern], row_mapper)?
                        .collect::<Result<Vec<_>, _>>()?
                } else {
                    stmt
                        .query_map(rusqlite::params![project_id, offset, limit], row_mapper)?
                        .collect::<Result<Vec<_>, _>>()?
                };

                Ok((sessions, total))
            })
            .await
    }

    fn set_expected_proposal_count_sync(
        conn: &rusqlite::Connection,
        session_id: &str,
        count: u32,
    ) -> AppResult<()> {
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE ideation_sessions SET expected_proposal_count = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![count as i64, now, session_id],
        )?;
        Ok(())
    }

    async fn set_auto_accept_status(
        &self,
        session_id: &str,
        status: &str,
        auto_accept_started_at: Option<String>,
    ) -> AppResult<()> {
        let session_id = session_id.to_string();
        let status = status.to_string();
        self.db
            .run(move |conn| {
                let now = Utc::now().to_rfc3339();
                if let Some(ref started_at) = auto_accept_started_at {
                    conn.execute(
                        "UPDATE ideation_sessions SET auto_accept_status = ?1, auto_accept_started_at = ?2, updated_at = ?3 WHERE id = ?4",
                        rusqlite::params![status, started_at, now, session_id],
                    )?;
                } else {
                    conn.execute(
                        "UPDATE ideation_sessions SET auto_accept_status = ?1, updated_at = ?2 WHERE id = ?3",
                        rusqlite::params![status, now, session_id],
                    )?;
                }
                Ok(())
            })
            .await
    }

    fn count_active_by_session_sync(
        conn: &rusqlite::Connection,
        session_id: &str,
    ) -> AppResult<i64> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM task_proposals WHERE session_id = ?1 AND archived_at IS NULL",
            rusqlite::params![session_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    async fn get_by_idempotency_key(
        &self,
        api_key_id: &str,
        idempotency_key: &str,
    ) -> AppResult<Option<IdeationSession>> {
        let api_key_id = api_key_id.to_string();
        let idempotency_key = idempotency_key.to_string();
        self.db
            .query_optional(move |conn| {
                let sql = format!(
                    "SELECT {} FROM ideation_sessions \
                     WHERE api_key_id = ?1 AND idempotency_key = ?2",
                    SESSION_COLUMNS
                );
                conn.query_row(&sql, rusqlite::params![api_key_id, idempotency_key], |row| {
                    IdeationSession::from_row(row)
                })
            })
            .await
    }

    async fn update_external_activity_phase(
        &self,
        id: &IdeationSessionId,
        phase: Option<&str>,
    ) -> AppResult<()> {
        let id = id.as_str().to_string();
        let phase = phase.map(|p| p.to_string());
        self.db
            .run(move |conn| {
                conn.execute(
                    "UPDATE ideation_sessions SET external_activity_phase = ?1, updated_at = ?2 \
                     WHERE id = ?3",
                    rusqlite::params![phase, Utc::now().to_rfc3339(), id],
                )?;
                Ok(())
            })
            .await
    }

    async fn update_external_last_read_message_id(
        &self,
        id: &IdeationSessionId,
        message_id: &str,
    ) -> AppResult<()> {
        let id = id.as_str().to_string();
        let message_id = message_id.to_string();
        self.db
            .run(move |conn| {
                conn.execute(
                    "UPDATE ideation_sessions SET external_last_read_message_id = ?1, updated_at = ?2 \
                     WHERE id = ?3",
                    rusqlite::params![message_id, Utc::now().to_rfc3339(), id],
                )?;
                Ok(())
            })
            .await
    }

    async fn list_active_external_by_project(
        &self,
        project_id: &ProjectId,
    ) -> AppResult<Vec<IdeationSession>> {
        let project_id = project_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let sql = format!(
                    "SELECT {} FROM ideation_sessions \
                     WHERE project_id = ?1 AND status = 'active' AND origin = 'external' \
                     ORDER BY created_at DESC",
                    SESSION_COLUMNS
                );
                let mut stmt = conn.prepare(&sql)?;
                let sessions = stmt
                    .query_map([&project_id], IdeationSession::from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(sessions)
            })
            .await
    }

    async fn list_active_external_sessions_for_archival(
        &self,
        stale_before: Option<DateTime<Utc>>,
    ) -> AppResult<Vec<IdeationSession>> {
        let stale_before_str = stale_before.map(|dt| dt.to_rfc3339());
        self.db
            .run(move |conn| {
                let sessions = if let Some(ref cutoff) = stale_before_str {
                    let sql = format!(
                        "SELECT {} FROM ideation_sessions \
                         WHERE origin = 'external' AND status = 'active' \
                         AND external_activity_phase IN ('created', 'error') \
                         AND updated_at < ?1 \
                         ORDER BY updated_at ASC",
                        SESSION_COLUMNS
                    );
                    let mut stmt = conn.prepare(&sql)?;
                    let result = stmt
                        .query_map([cutoff], IdeationSession::from_row)?
                        .collect::<Result<Vec<_>, _>>()?;
                    result
                } else {
                    let sql = format!(
                        "SELECT {} FROM ideation_sessions \
                         WHERE origin = 'external' AND status = 'active' \
                         AND external_activity_phase IN ('created', 'error') \
                         ORDER BY updated_at ASC",
                        SESSION_COLUMNS
                    );
                    let mut stmt = conn.prepare(&sql)?;
                    let result = stmt
                        .query_map([], IdeationSession::from_row)?
                        .collect::<Result<Vec<_>, _>>()?;
                    result
                };
                Ok(sessions)
            })
            .await
    }

    async fn list_stalled_external_sessions(
        &self,
        stalled_before: DateTime<Utc>,
    ) -> AppResult<Vec<IdeationSession>> {
        let stalled_before_str = stalled_before.to_rfc3339();
        self.db
            .run(move |conn| {
                let sql = format!(
                    "SELECT {} FROM ideation_sessions \
                     WHERE origin = 'external' AND status = 'active' \
                     AND external_activity_phase IS NOT NULL \
                     AND external_activity_phase NOT IN ('error', 'stalled') \
                     AND updated_at < ?1 \
                     ORDER BY updated_at ASC",
                    SESSION_COLUMNS
                );
                let mut stmt = conn.prepare(&sql)?;
                let sessions = stmt
                    .query_map([&stalled_before_str], IdeationSession::from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(sessions)
            })
            .await
    }

    async fn set_dependencies_acknowledged(&self, session_id: &str) -> AppResult<()> {
        let session_id = session_id.to_string();
        self.db
            .run(move |conn| {
                let now = Utc::now().to_rfc3339();
                conn.execute(
                    "UPDATE ideation_sessions SET dependencies_acknowledged = 1, updated_at = ?1 WHERE id = ?2",
                    rusqlite::params![now, session_id],
                )?;
                Ok(())
            })
            .await
    }

    async fn reset_acceptance_cycle_fields(&self, session_id: &str) -> AppResult<()> {
        let session_id = session_id.to_string();
        self.db
            .run(move |conn| {
                let now = Utc::now().to_rfc3339();
                conn.execute(
                    "UPDATE ideation_sessions \
                     SET expected_proposal_count = NULL, \
                         dependencies_acknowledged = 0, \
                         auto_accept_status = NULL, \
                         auto_accept_started_at = NULL, \
                         cross_project_checked = 0, \
                         updated_at = ?1 \
                     WHERE id = ?2",
                    rusqlite::params![now, session_id],
                )?;
                Ok(())
            })
            .await
    }

    async fn touch_updated_at(&self, session_id: &str) -> AppResult<()> {
        let session_id = session_id.to_string();
        self.db
            .run(move |conn| {
                let now = Utc::now().to_rfc3339();
                conn.execute(
                    "UPDATE ideation_sessions SET updated_at = ?1 WHERE id = ?2",
                    rusqlite::params![now, session_id],
                )?;
                Ok(())
            })
            .await
    }

    async fn list_active_verification_children(&self) -> AppResult<Vec<IdeationSession>> {
        self.db
            .run(move |conn| {
                let sql = format!(
                    "SELECT {} FROM ideation_sessions \
                     WHERE session_purpose = 'verification' AND status != 'archived' \
                     ORDER BY created_at ASC",
                    SESSION_COLUMNS
                );
                let mut stmt = conn.prepare(&sql)?;
                let sessions = stmt
                    .query_map([], IdeationSession::from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(sessions)
            })
            .await
    }

    async fn set_pending_initial_prompt(
        &self,
        session_id: &str,
        prompt: Option<String>,
    ) -> AppResult<()> {
        let session_id = session_id.to_string();
        self.db
            .run(move |conn| {
                conn.execute(
                    "UPDATE ideation_sessions SET pending_initial_prompt = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                    rusqlite::params![prompt, session_id],
                )?;
                Ok(())
            })
            .await
    }

    async fn set_pending_initial_prompt_if_unset(
        &self,
        session_id: &str,
        prompt: String,
    ) -> AppResult<bool> {
        let session_id = session_id.to_string();
        self.db
            .run(move |conn| {
                let rows_changed = conn.execute(
                    "UPDATE ideation_sessions \
                     SET pending_initial_prompt = ?1, updated_at = CURRENT_TIMESTAMP \
                     WHERE id = ?2 AND pending_initial_prompt IS NULL",
                    rusqlite::params![prompt, session_id],
                )?;
                Ok(rows_changed == 1)
            })
            .await
    }

    async fn claim_pending_session_for_project(
        &self,
        project_id: &str,
    ) -> AppResult<Option<(String, String)>> {
        let project_id = project_id.to_string();
        self.db
            .run_transaction(move |conn| {
                // SELECT oldest active session with a pending prompt for this project
                let result: Option<(String, String)> = match conn.query_row(
                    "SELECT id, pending_initial_prompt FROM ideation_sessions \
                     WHERE project_id = ?1 \
                       AND status = 'active' \
                       AND pending_initial_prompt IS NOT NULL \
                     ORDER BY created_at ASC \
                     LIMIT 1",
                    rusqlite::params![project_id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                ) {
                    Ok(row) => Some(row),
                    Err(rusqlite::Error::QueryReturnedNoRows) => None,
                    Err(e) => return Err(AppError::Database(e.to_string())),
                };

                match result {
                    None => Ok(None),
                    Some((session_id, prompt)) => {
                        // Atomically clear the prompt so no other drain can claim this session
                        conn.execute(
                            "UPDATE ideation_sessions SET pending_initial_prompt = NULL, updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
                            rusqlite::params![session_id],
                        )?;
                        Ok(Some((session_id, prompt)))
                    }
                }
            })
            .await
    }

    async fn list_projects_with_pending_sessions(&self) -> AppResult<Vec<String>> {
        self.db
            .run(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT DISTINCT project_id FROM ideation_sessions \
                     WHERE pending_initial_prompt IS NOT NULL \
                       AND status = 'active'",
                )?;
                let ids = stmt
                    .query_map([], |row| row.get::<_, String>(0))?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(ids)
            })
            .await
    }

    async fn count_pending_sessions_for_project(
        &self,
        project_id: &ProjectId,
    ) -> AppResult<u32> {
        let project_id = project_id.to_string();
        self.db
            .run(move |conn| {
                let count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM ideation_sessions \
                     WHERE project_id = ?1 \
                       AND pending_initial_prompt IS NOT NULL \
                       AND status = 'active'",
                    rusqlite::params![project_id],
                    |row| row.get(0),
                )?;
                Ok(count as u32)
            })
            .await
    }

    async fn update_acceptance_status(
        &self,
        session_id: &IdeationSessionId,
        expected_current: Option<AcceptanceStatus>,
        new_status: Option<AcceptanceStatus>,
    ) -> AppResult<bool> {
        let session_id_str = session_id.as_str().to_string();
        let expected_str: Option<String> = expected_current.map(|s| s.to_string());
        let new_str: Option<String> = new_status.map(|s| s.to_string());
        self.db
            .run(move |conn| {
                let rows_affected = if let Some(ref expected) = expected_str {
                    conn.execute(
                        "UPDATE ideation_sessions \
                         SET acceptance_status = ?1, \
                             updated_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now') \
                         WHERE id = ?2 AND acceptance_status = ?3",
                        rusqlite::params![new_str, session_id_str, expected],
                    )
                    .map_err(|e| AppError::Database(e.to_string()))?
                } else {
                    conn.execute(
                        "UPDATE ideation_sessions \
                         SET acceptance_status = ?1, \
                             updated_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now') \
                         WHERE id = ?2 AND acceptance_status IS NULL",
                        rusqlite::params![new_str, session_id_str],
                    )
                    .map_err(|e| AppError::Database(e.to_string()))?
                };
                Ok(rows_affected > 0)
            })
            .await
    }

    async fn get_sessions_with_pending_acceptance(
        &self,
        project_id: &ProjectId,
    ) -> AppResult<Vec<IdeationSession>> {
        let project_id_str = project_id.to_string();
        self.db
            .run(move |conn| {
                let mut stmt = conn
                    .prepare(&format!(
                        "SELECT {SESSION_COLUMNS} FROM ideation_sessions \
                         WHERE project_id = ?1 \
                           AND status = 'active' \
                           AND acceptance_status = 'pending' \
                         ORDER BY updated_at DESC",
                    ))
                    .map_err(|e| AppError::Database(e.to_string()))?;
                let sessions = stmt
                    .query_map(rusqlite::params![project_id_str], |row| {
                        IdeationSession::from_row(row)
                    })
                    .map_err(|e| AppError::Database(e.to_string()))?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| AppError::Database(e.to_string()))?;
                Ok(sessions)
            })
            .await
    }
}
