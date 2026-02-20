// SQLite-based IdeationSessionRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use rusqlite::Connection;

use crate::domain::entities::{
    IdeationSession, IdeationSessionId, IdeationSessionStatus, ProjectId,
};
use crate::domain::repositories::IdeationSessionRepository;
use crate::error::{AppError, AppResult};

/// SQLite implementation of IdeationSessionRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteIdeationSessionRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteIdeationSessionRepository {
    /// Create a new SQLite ideation session repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    /// Create from an Arc-wrapped mutex connection (for sharing)
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl IdeationSessionRepository for SqliteIdeationSessionRepository {
    async fn create(&self, session: IdeationSession) -> AppResult<IdeationSession> {
        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, title, title_source, status, plan_artifact_id, seed_task_id, parent_session_id, created_at, updated_at, archived_at, converted_at, team_mode, team_config_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            rusqlite::params![
                session.id.as_str(),
                session.project_id.as_str(),
                session.title,
                session.title_source,
                session.status.to_string(),
                session.plan_artifact_id.as_ref().map(|id| id.as_str()),
                session.seed_task_id.as_ref().map(|id| id.as_str()),
                session.parent_session_id.as_ref().map(|id| id.as_str()),
                session.created_at.to_rfc3339(),
                session.updated_at.to_rfc3339(),
                session.archived_at.map(|dt| dt.to_rfc3339()),
                session.converted_at.map(|dt| dt.to_rfc3339()),
                session.team_mode,
                session.team_config_json,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(session)
    }

    async fn get_by_id(&self, id: &IdeationSessionId) -> AppResult<Option<IdeationSession>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, project_id, title, title_source, status, plan_artifact_id, seed_task_id, parent_session_id, created_at, updated_at, archived_at, converted_at, team_mode, team_config_json
             FROM ideation_sessions WHERE id = ?1",
            [id.as_str()],
            |row| IdeationSession::from_row(row),
        );

        match result {
            Ok(session) => Ok(Some(session)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<IdeationSession>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, title, title_source, status, plan_artifact_id, seed_task_id, parent_session_id, created_at, updated_at, archived_at, converted_at, team_mode, team_config_json
                 FROM ideation_sessions WHERE project_id = ?1 ORDER BY updated_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let sessions = stmt
            .query_map([project_id.as_str()], IdeationSession::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(sessions)
    }

    async fn update_status(
        &self,
        id: &IdeationSessionId,
        status: IdeationSessionStatus,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;
        let now = Utc::now();

        // Build the update query based on the new status
        // If archiving, set archived_at; if converting, set converted_at
        let query = match status {
            IdeationSessionStatus::Archived => {
                "UPDATE ideation_sessions SET status = ?2, updated_at = ?3, archived_at = ?4 WHERE id = ?1"
            }
            IdeationSessionStatus::Accepted => {
                "UPDATE ideation_sessions SET status = ?2, updated_at = ?3, converted_at = ?4 WHERE id = ?1"
            }
            IdeationSessionStatus::Active => {
                // When reactivating, clear archived_at and converted_at
                "UPDATE ideation_sessions SET status = ?2, updated_at = ?3, archived_at = NULL, converted_at = NULL WHERE id = ?1"
            }
        };

        match status {
            IdeationSessionStatus::Archived | IdeationSessionStatus::Accepted => {
                conn.execute(
                    query,
                    rusqlite::params![
                        id.as_str(),
                        status.to_string(),
                        now.to_rfc3339(),
                        now.to_rfc3339(),
                    ],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            }
            IdeationSessionStatus::Active => {
                conn.execute(
                    query,
                    rusqlite::params![id.as_str(), status.to_string(), now.to_rfc3339(),],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            }
        }

        Ok(())
    }

    async fn update_title(
        &self,
        id: &IdeationSessionId,
        title: Option<String>,
        title_source: &str,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;
        let now = Utc::now();

        conn.execute(
            "UPDATE ideation_sessions SET title = ?2, title_source = ?3, updated_at = ?4 WHERE id = ?1",
            rusqlite::params![id.as_str(), title, title_source, now.to_rfc3339(),],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_plan_artifact_id(
        &self,
        id: &IdeationSessionId,
        plan_artifact_id: Option<String>,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;
        let now = Utc::now();

        conn.execute(
            "UPDATE ideation_sessions SET plan_artifact_id = ?2, updated_at = ?3 WHERE id = ?1",
            rusqlite::params![id.as_str(), plan_artifact_id, now.to_rfc3339(),],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: &IdeationSessionId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        // CASCADE is defined in the schema, so deleting the session
        // will automatically delete related proposals and messages
        conn.execute("DELETE FROM ideation_sessions WHERE id = ?1", [id.as_str()])
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_active_by_project(
        &self,
        project_id: &ProjectId,
    ) -> AppResult<Vec<IdeationSession>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, title, title_source, status, plan_artifact_id, seed_task_id, parent_session_id, created_at, updated_at, archived_at, converted_at, team_mode, team_config_json
                 FROM ideation_sessions
                 WHERE project_id = ?1 AND status = 'active'
                 ORDER BY updated_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let sessions = stmt
            .query_map([project_id.as_str()], IdeationSession::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(sessions)
    }

    async fn count_by_status(
        &self,
        project_id: &ProjectId,
        status: IdeationSessionStatus,
    ) -> AppResult<u32> {
        let conn = self.conn.lock().await;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ideation_sessions WHERE project_id = ?1 AND status = ?2",
                rusqlite::params![project_id.as_str(), status.to_string()],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count as u32)
    }

    async fn get_by_plan_artifact_id(
        &self,
        plan_artifact_id: &str,
    ) -> AppResult<Vec<IdeationSession>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, title, title_source, status, plan_artifact_id, seed_task_id, parent_session_id, created_at, updated_at, archived_at, converted_at, team_mode, team_config_json
                 FROM ideation_sessions WHERE plan_artifact_id = ?1",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let sessions = stmt
            .query_map([plan_artifact_id], IdeationSession::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(sessions)
    }

    async fn get_children(&self, parent_id: &IdeationSessionId) -> AppResult<Vec<IdeationSession>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, title, title_source, status, plan_artifact_id, seed_task_id, parent_session_id, created_at, updated_at, archived_at, converted_at, team_mode, team_config_json
                 FROM ideation_sessions WHERE parent_session_id = ?1 ORDER BY created_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let sessions = stmt
            .query_map([parent_id.as_str()], IdeationSession::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(sessions)
    }

    async fn get_ancestor_chain(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<Vec<IdeationSession>> {
        let conn = self.conn.lock().await;

        let mut chain = Vec::new();
        let mut current_id = session_id.clone();

        // Walk up the parent chain iteratively
        loop {
            let result = conn.query_row(
                "SELECT id, project_id, title, title_source, status, plan_artifact_id, seed_task_id, parent_session_id, created_at, updated_at, archived_at, converted_at, team_mode, team_config_json
                 FROM ideation_sessions WHERE id = ?1",
                [current_id.as_str()],
                |row| IdeationSession::from_row(row),
            );

            match result {
                Ok(session) => {
                    if let Some(parent_id) = &session.parent_session_id {
                        current_id = parent_id.clone();
                        // Fetch parent and add to chain
                        match conn.query_row(
                            "SELECT id, project_id, title, title_source, status, plan_artifact_id, seed_task_id, parent_session_id, created_at, updated_at, archived_at, converted_at, team_mode, team_config_json
                             FROM ideation_sessions WHERE id = ?1",
                            [parent_id.as_str()],
                            |row| IdeationSession::from_row(row),
                        ) {
                            Ok(parent) => {
                                chain.push(parent);
                            }
                            Err(rusqlite::Error::QueryReturnedNoRows) => {
                                // Parent doesn't exist, stop here
                                break;
                            }
                            Err(e) => {
                                return Err(AppError::Database(e.to_string()));
                            }
                        }
                    } else {
                        // No parent, end of chain
                        break;
                    }
                }
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    // Session doesn't exist, empty chain
                    break;
                }
                Err(e) => {
                    return Err(AppError::Database(e.to_string()));
                }
            }
        }

        Ok(chain)
    }

    async fn set_parent(
        &self,
        id: &IdeationSessionId,
        parent_id: Option<&IdeationSessionId>,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;
        let now = Utc::now();

        conn.execute(
            "UPDATE ideation_sessions SET parent_session_id = ?2, updated_at = ?3 WHERE id = ?1",
            rusqlite::params![id.as_str(), parent_id.map(|p| p.as_str()), now.to_rfc3339(),],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
#[path = "sqlite_ideation_session_repo_tests.rs"]
mod tests;
