// SQLite-based IdeationSessionRepository implementation for production use
// Uses DbConnection (spawn_blocking) for non-blocking rusqlite access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::Connection;

use crate::domain::entities::{
    IdeationSession, IdeationSessionId, IdeationSessionStatus, ProjectId, VerificationStatus,
};
use crate::domain::repositories::IdeationSessionRepository;
use crate::error::{AppError, AppResult};

use super::DbConnection;

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
    pub(crate) fn insert_sync(
        conn: &Connection,
        session: &IdeationSession,
    ) -> AppResult<IdeationSession> {
        conn.execute(
            "INSERT INTO ideation_sessions \
             (id, project_id, title, title_source, status, plan_artifact_id, \
              inherited_plan_artifact_id, seed_task_id, parent_session_id, created_at, \
              updated_at, archived_at, converted_at, team_mode, team_config_json, \
              verification_status, source_project_id, source_session_id) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
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
            ],
        )?;
        Ok(session.clone())
    }

    /// Fetch a single session by ID; returns None if not found.
    pub(crate) fn get_by_id_sync(
        conn: &Connection,
        id: &str,
    ) -> AppResult<Option<IdeationSession>> {
        match conn.query_row(
            "SELECT id, project_id, title, title_source, status, plan_artifact_id, \
             inherited_plan_artifact_id, seed_task_id, parent_session_id, created_at, \
             updated_at, archived_at, converted_at, team_mode, team_config_json, \
             verification_status, verification_in_progress, verification_metadata, \
             verification_generation, source_project_id, source_session_id \
             FROM ideation_sessions WHERE id = ?1",
            [id],
            |row| IdeationSession::from_row(row),
        ) {
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
        let mut stmt = conn.prepare(
            "SELECT id, project_id, title, title_source, status, plan_artifact_id, \
             inherited_plan_artifact_id, seed_task_id, parent_session_id, created_at, \
             updated_at, archived_at, converted_at, team_mode, team_config_json, \
             verification_status, verification_in_progress, verification_metadata, \
             verification_generation, source_project_id, source_session_id \
             FROM ideation_sessions WHERE plan_artifact_id = ?1",
        )?;
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
        let mut stmt = conn.prepare(
            "SELECT id, project_id, title, title_source, status, plan_artifact_id, \
             inherited_plan_artifact_id, seed_task_id, parent_session_id, created_at, \
             updated_at, archived_at, converted_at, team_mode, team_config_json, \
             verification_status, verification_in_progress, verification_metadata, \
             verification_generation, source_project_id, source_session_id \
             FROM ideation_sessions WHERE inherited_plan_artifact_id = ?1",
        )?;
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
    pub(crate) fn trigger_auto_verify_sync(
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
    pub(crate) fn reset_auto_verify_sync(conn: &Connection, id: &str) -> AppResult<()> {
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
    pub(crate) fn validate_no_circular_import_sync(
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
            .run(move |conn| {
                conn.execute(
                    "INSERT INTO ideation_sessions (id, project_id, title, title_source, status, plan_artifact_id, inherited_plan_artifact_id, seed_task_id, parent_session_id, created_at, updated_at, archived_at, converted_at, team_mode, team_config_json, verification_status, source_project_id, source_session_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
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
                    ],
                )?;
                Ok(session)
            })
            .await
    }

    async fn get_by_id(&self, id: &IdeationSessionId) -> AppResult<Option<IdeationSession>> {
        let id = id.as_str().to_string();
        self.db
            .query_optional(move |conn| {
                conn.query_row(
                    "SELECT id, project_id, title, title_source, status, plan_artifact_id, inherited_plan_artifact_id, seed_task_id, parent_session_id, created_at, updated_at, archived_at, converted_at, team_mode, team_config_json, verification_status, verification_in_progress, verification_metadata, verification_generation, source_project_id, source_session_id
                     FROM ideation_sessions WHERE id = ?1",
                    [&id],
                    |row| IdeationSession::from_row(row),
                )
            })
            .await
    }

    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<IdeationSession>> {
        let project_id = project_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, project_id, title, title_source, status, plan_artifact_id, inherited_plan_artifact_id, seed_task_id, parent_session_id, created_at, updated_at, archived_at, converted_at, team_mode, team_config_json, verification_status, verification_in_progress, verification_metadata, verification_generation, source_project_id, source_session_id
                     FROM ideation_sessions WHERE project_id = ?1 ORDER BY updated_at DESC",
                )?;
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
                        "UPDATE ideation_sessions SET status = ?2, updated_at = ?3, archived_at = ?4 WHERE id = ?1"
                    }
                    IdeationSessionStatus::Accepted => {
                        "UPDATE ideation_sessions SET status = ?2, updated_at = ?3, converted_at = ?4 WHERE id = ?1"
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
                let mut stmt = conn.prepare(
                    "SELECT id, project_id, title, title_source, status, plan_artifact_id, inherited_plan_artifact_id, seed_task_id, parent_session_id, created_at, updated_at, archived_at, converted_at, team_mode, team_config_json, verification_status, verification_in_progress, verification_metadata, verification_generation, source_project_id, source_session_id
                     FROM ideation_sessions
                     WHERE project_id = ?1 AND status = 'active'
                     ORDER BY updated_at DESC",
                )?;
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
                let mut stmt = conn.prepare(
                    "SELECT id, project_id, title, title_source, status, plan_artifact_id, inherited_plan_artifact_id, seed_task_id, parent_session_id, created_at, updated_at, archived_at, converted_at, team_mode, team_config_json, verification_status, verification_in_progress, verification_metadata, verification_generation, source_project_id, source_session_id
                     FROM ideation_sessions WHERE plan_artifact_id = ?1",
                )?;
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
                let mut stmt = conn.prepare(
                    "SELECT id, project_id, title, title_source, status, plan_artifact_id, inherited_plan_artifact_id, seed_task_id, parent_session_id, created_at, updated_at, archived_at, converted_at, team_mode, team_config_json, verification_status, verification_in_progress, verification_metadata, verification_generation, source_project_id, source_session_id
                     FROM ideation_sessions WHERE inherited_plan_artifact_id = ?1",
                )?;
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
                let mut stmt = conn.prepare(
                    "SELECT id, project_id, title, title_source, status, plan_artifact_id, inherited_plan_artifact_id, seed_task_id, parent_session_id, created_at, updated_at, archived_at, converted_at, team_mode, team_config_json, verification_status, verification_in_progress, verification_metadata, verification_generation, source_project_id, source_session_id
                     FROM ideation_sessions WHERE parent_session_id = ?1 ORDER BY created_at DESC",
                )?;
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
                    let result = conn.query_row(
                        "SELECT id, project_id, title, title_source, status, plan_artifact_id, inherited_plan_artifact_id, seed_task_id, parent_session_id, created_at, updated_at, archived_at, converted_at, team_mode, team_config_json, verification_status, verification_in_progress, verification_metadata, verification_generation, source_project_id, source_session_id
                         FROM ideation_sessions WHERE id = ?1",
                        [&current_id],
                        |row| IdeationSession::from_row(row),
                    );

                    match result {
                        Ok(session) => {
                            if let Some(parent_id) = &session.parent_session_id {
                                let parent_id_str = parent_id.as_str().to_string();
                                current_id = parent_id_str.clone();
                                match conn.query_row(
                                    "SELECT id, project_id, title, title_source, status, plan_artifact_id, inherited_plan_artifact_id, seed_task_id, parent_session_id, created_at, updated_at, archived_at, converted_at, team_mode, team_config_json, verification_status, verification_in_progress, verification_metadata, verification_generation
                                     FROM ideation_sessions WHERE id = ?1",
                                    [&parent_id_str],
                                    |row| IdeationSession::from_row(row),
                                ) {
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
                    "UPDATE ideation_sessions SET plan_artifact_id = ?2, verification_status = 'skipped', verification_in_progress = 0, verification_metadata = ?3, updated_at = ?4 WHERE id = ?1",
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
                         updated_at = ?4 \
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

    async fn get_stale_in_progress_sessions(
        &self,
        stale_before: DateTime<Utc>,
    ) -> AppResult<Vec<IdeationSession>> {
        let stale_before_str = stale_before.to_rfc3339();
        self.db
            .run(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, project_id, title, title_source, status, plan_artifact_id, \
                     inherited_plan_artifact_id, seed_task_id, parent_session_id, created_at, \
                     updated_at, archived_at, converted_at, team_mode, team_config_json, \
                     verification_status, verification_in_progress, verification_metadata, \
                     verification_generation, source_project_id, source_session_id \
                     FROM ideation_sessions \
                     WHERE verification_in_progress = 1 AND updated_at < ?1",
                )?;
                let sessions = stmt
                    .query_map([&stale_before_str], IdeationSession::from_row)?
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
                let mut stmt = conn.prepare(
                    "SELECT id, project_id, title, title_source, status, plan_artifact_id, \
                     inherited_plan_artifact_id, seed_task_id, parent_session_id, created_at, \
                     updated_at, archived_at, converted_at, team_mode, team_config_json, \
                     verification_status, verification_in_progress, verification_metadata, \
                     verification_generation, source_project_id, source_session_id \
                     FROM ideation_sessions \
                     WHERE project_id = ?1 AND status = ?2 \
                     ORDER BY created_at DESC LIMIT ?3",
                )?;
                let sessions = stmt
                    .query_map(rusqlite::params![project_id, status, limit], IdeationSession::from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(sessions)
            })
            .await
    }
}

#[cfg(test)]
#[path = "sqlite_ideation_session_repo_tests.rs"]
mod tests;
