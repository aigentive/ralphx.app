// SQLite-based IdeationSessionRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use rusqlite::Connection;

use crate::domain::entities::{IdeationSession, IdeationSessionId, IdeationSessionStatus, ProjectId};
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
            "INSERT INTO ideation_sessions (id, project_id, title, status, plan_artifact_id, created_at, updated_at, archived_at, converted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                session.id.as_str(),
                session.project_id.as_str(),
                session.title,
                session.status.to_string(),
                session.plan_artifact_id.as_ref().map(|id| id.as_str()),
                session.created_at.to_rfc3339(),
                session.updated_at.to_rfc3339(),
                session.archived_at.map(|dt| dt.to_rfc3339()),
                session.converted_at.map(|dt| dt.to_rfc3339()),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(session)
    }

    async fn get_by_id(&self, id: &IdeationSessionId) -> AppResult<Option<IdeationSession>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, project_id, title, status, plan_artifact_id, created_at, updated_at, archived_at, converted_at
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
                "SELECT id, project_id, title, status, plan_artifact_id, created_at, updated_at, archived_at, converted_at
                 FROM ideation_sessions WHERE project_id = ?1 ORDER BY updated_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let sessions = stmt
            .query_map([project_id.as_str()], |row| IdeationSession::from_row(row))
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
            IdeationSessionStatus::Converted => {
                "UPDATE ideation_sessions SET status = ?2, updated_at = ?3, converted_at = ?4 WHERE id = ?1"
            }
            IdeationSessionStatus::Active => {
                // When reactivating, just update status and updated_at
                "UPDATE ideation_sessions SET status = ?2, updated_at = ?3 WHERE id = ?1"
            }
        };

        match status {
            IdeationSessionStatus::Archived | IdeationSessionStatus::Converted => {
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

    async fn update_title(&self, id: &IdeationSessionId, title: Option<String>) -> AppResult<()> {
        let conn = self.conn.lock().await;
        let now = Utc::now();

        conn.execute(
            "UPDATE ideation_sessions SET title = ?2, updated_at = ?3 WHERE id = ?1",
            rusqlite::params![id.as_str(), title, now.to_rfc3339(),],
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

    async fn get_active_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<IdeationSession>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, title, status, plan_artifact_id, created_at, updated_at, archived_at, converted_at
                 FROM ideation_sessions
                 WHERE project_id = ?1 AND status = 'active'
                 ORDER BY updated_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let sessions = stmt
            .query_map([project_id.as_str()], |row| IdeationSession::from_row(row))
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

    fn setup_test_db() -> Connection {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    fn create_test_project(conn: &Connection, id: &ProjectId, name: &str, path: &str) {
        conn.execute(
            "INSERT INTO projects (id, name, working_directory, git_mode, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'single_branch', datetime('now'), datetime('now'))",
            rusqlite::params![id.as_str(), name, path],
        )
        .unwrap();
    }

    fn create_test_session(project_id: &ProjectId, title: Option<&str>) -> IdeationSession {
        let mut builder = IdeationSession::builder()
            .project_id(project_id.clone());

        if let Some(t) = title {
            builder = builder.title(t);
        }

        builder.build()
    }

    // ==================== CREATE TESTS ====================

    #[tokio::test]
    async fn test_create_inserts_session_and_returns_it() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let repo = SqliteIdeationSessionRepository::new(conn);
        let session = create_test_session(&project_id, Some("My Ideation"));

        let result = repo.create(session.clone()).await;

        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.id, session.id);
        assert_eq!(created.title, Some("My Ideation".to_string()));
        assert_eq!(created.status, IdeationSessionStatus::Active);
    }

    #[tokio::test]
    async fn test_create_session_without_title() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let repo = SqliteIdeationSessionRepository::new(conn);
        let session = create_test_session(&project_id, None);

        let result = repo.create(session.clone()).await;

        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.title, None);
    }

    #[tokio::test]
    async fn test_create_duplicate_id_fails() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let repo = SqliteIdeationSessionRepository::new(conn);
        let session = create_test_session(&project_id, Some("Original"));

        repo.create(session.clone()).await.unwrap();
        let result = repo.create(session).await;

        assert!(result.is_err());
    }

    // ==================== GET BY ID TESTS ====================

    #[tokio::test]
    async fn test_get_by_id_retrieves_session_correctly() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let repo = SqliteIdeationSessionRepository::new(conn);
        let session = create_test_session(&project_id, Some("Test Session"));

        repo.create(session.clone()).await.unwrap();
        let result = repo.get_by_id(&session.id).await;

        assert!(result.is_ok());
        let found = result.unwrap();
        assert!(found.is_some());
        let found_session = found.unwrap();
        assert_eq!(found_session.id, session.id);
        assert_eq!(found_session.title, Some("Test Session".to_string()));
        assert_eq!(found_session.project_id, project_id);
    }

    #[tokio::test]
    async fn test_get_by_id_returns_none_for_nonexistent() {
        let conn = setup_test_db();
        let repo = SqliteIdeationSessionRepository::new(conn);
        let id = IdeationSessionId::new();

        let result = repo.get_by_id(&id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_by_id_preserves_all_fields() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let repo = SqliteIdeationSessionRepository::new(conn);

        // Create a session with all fields set
        let mut session = create_test_session(&project_id, Some("Full Session"));
        session.archive();  // This sets archived_at

        repo.create(session.clone()).await.unwrap();
        let found = repo.get_by_id(&session.id).await.unwrap().unwrap();

        assert_eq!(found.id, session.id);
        assert_eq!(found.project_id, session.project_id);
        assert_eq!(found.title, session.title);
        assert_eq!(found.status, IdeationSessionStatus::Archived);
        assert!(found.archived_at.is_some());
    }

    // ==================== GET BY PROJECT TESTS ====================

    #[tokio::test]
    async fn test_get_by_project_returns_all_sessions() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let repo = SqliteIdeationSessionRepository::new(conn);

        let session1 = create_test_session(&project_id, Some("Session 1"));
        let session2 = create_test_session(&project_id, Some("Session 2"));
        let session3 = create_test_session(&project_id, Some("Session 3"));

        repo.create(session1).await.unwrap();
        repo.create(session2).await.unwrap();
        repo.create(session3).await.unwrap();

        let result = repo.get_by_project(&project_id).await;

        assert!(result.is_ok());
        let sessions = result.unwrap();
        assert_eq!(sessions.len(), 3);
    }

    #[tokio::test]
    async fn test_get_by_project_ordered_by_updated_at_desc() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let repo = SqliteIdeationSessionRepository::new(conn);

        // Create sessions with different timestamps
        let session1 = IdeationSession::builder()
            .project_id(project_id.clone())
            .title("Oldest")
            .created_at(chrono::Utc::now() - chrono::Duration::hours(3))
            .updated_at(chrono::Utc::now() - chrono::Duration::hours(3))
            .build();
        let session2 = IdeationSession::builder()
            .project_id(project_id.clone())
            .title("Middle")
            .created_at(chrono::Utc::now() - chrono::Duration::hours(2))
            .updated_at(chrono::Utc::now() - chrono::Duration::hours(2))
            .build();
        let session3 = IdeationSession::builder()
            .project_id(project_id.clone())
            .title("Newest")
            .created_at(chrono::Utc::now() - chrono::Duration::hours(1))
            .updated_at(chrono::Utc::now() - chrono::Duration::hours(1))
            .build();

        // Insert in non-order (oldest first, then newest, then middle)
        repo.create(session1).await.unwrap();
        repo.create(session3).await.unwrap();
        repo.create(session2).await.unwrap();

        let sessions = repo.get_by_project(&project_id).await.unwrap();

        // Should be ordered newest first
        assert_eq!(sessions.len(), 3);
        assert_eq!(sessions[0].title, Some("Newest".to_string()));
        assert_eq!(sessions[1].title, Some("Middle".to_string()));
        assert_eq!(sessions[2].title, Some("Oldest".to_string()));
    }

    #[tokio::test]
    async fn test_get_by_project_returns_empty_for_no_sessions() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let repo = SqliteIdeationSessionRepository::new(conn);

        let result = repo.get_by_project(&project_id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_by_project_filters_by_project() {
        let conn = setup_test_db();
        let project_id1 = ProjectId::new();
        let project_id2 = ProjectId::new();
        create_test_project(&conn, &project_id1, "Project 1", "/path1");
        create_test_project(&conn, &project_id2, "Project 2", "/path2");

        let repo = SqliteIdeationSessionRepository::new(conn);

        let session1 = create_test_session(&project_id1, Some("Session for P1"));
        let session2 = create_test_session(&project_id2, Some("Session for P2"));

        repo.create(session1).await.unwrap();
        repo.create(session2).await.unwrap();

        let sessions = repo.get_by_project(&project_id1).await.unwrap();

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].project_id, project_id1);
    }

    // ==================== UPDATE STATUS TESTS ====================

    #[tokio::test]
    async fn test_update_status_to_archived() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let repo = SqliteIdeationSessionRepository::new(conn);
        let session = create_test_session(&project_id, Some("To Archive"));

        repo.create(session.clone()).await.unwrap();

        let result = repo.update_status(&session.id, IdeationSessionStatus::Archived).await;
        assert!(result.is_ok());

        let found = repo.get_by_id(&session.id).await.unwrap().unwrap();
        assert_eq!(found.status, IdeationSessionStatus::Archived);
        assert!(found.archived_at.is_some());
    }

    #[tokio::test]
    async fn test_update_status_to_converted() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let repo = SqliteIdeationSessionRepository::new(conn);
        let session = create_test_session(&project_id, Some("To Convert"));

        repo.create(session.clone()).await.unwrap();

        let result = repo.update_status(&session.id, IdeationSessionStatus::Converted).await;
        assert!(result.is_ok());

        let found = repo.get_by_id(&session.id).await.unwrap().unwrap();
        assert_eq!(found.status, IdeationSessionStatus::Converted);
        assert!(found.converted_at.is_some());
    }

    #[tokio::test]
    async fn test_update_status_back_to_active() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let repo = SqliteIdeationSessionRepository::new(conn);
        let mut session = create_test_session(&project_id, Some("Reactivate"));
        session.archive();

        repo.create(session.clone()).await.unwrap();

        let result = repo.update_status(&session.id, IdeationSessionStatus::Active).await;
        assert!(result.is_ok());

        let found = repo.get_by_id(&session.id).await.unwrap().unwrap();
        assert_eq!(found.status, IdeationSessionStatus::Active);
    }

    #[tokio::test]
    async fn test_update_status_updates_updated_at() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let repo = SqliteIdeationSessionRepository::new(conn);
        let session = create_test_session(&project_id, Some("Check Timestamp"));
        let original_updated = session.updated_at;

        repo.create(session.clone()).await.unwrap();

        // Small delay to ensure timestamp difference
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        repo.update_status(&session.id, IdeationSessionStatus::Archived).await.unwrap();

        let found = repo.get_by_id(&session.id).await.unwrap().unwrap();
        assert!(found.updated_at >= original_updated);
    }

    // ==================== UPDATE TITLE TESTS ====================

    #[tokio::test]
    async fn test_update_title_sets_new_title() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let repo = SqliteIdeationSessionRepository::new(conn);
        let session = create_test_session(&project_id, Some("Original Title"));

        repo.create(session.clone()).await.unwrap();

        let result = repo.update_title(&session.id, Some("New Title".to_string())).await;
        assert!(result.is_ok());

        let found = repo.get_by_id(&session.id).await.unwrap().unwrap();
        assert_eq!(found.title, Some("New Title".to_string()));
    }

    #[tokio::test]
    async fn test_update_title_clears_title() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let repo = SqliteIdeationSessionRepository::new(conn);
        let session = create_test_session(&project_id, Some("Has Title"));

        repo.create(session.clone()).await.unwrap();

        let result = repo.update_title(&session.id, None).await;
        assert!(result.is_ok());

        let found = repo.get_by_id(&session.id).await.unwrap().unwrap();
        assert_eq!(found.title, None);
    }

    #[tokio::test]
    async fn test_update_title_updates_updated_at() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let repo = SqliteIdeationSessionRepository::new(conn);
        let session = create_test_session(&project_id, Some("Check Timestamp"));
        let original_updated = session.updated_at;

        repo.create(session.clone()).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        repo.update_title(&session.id, Some("Updated".to_string())).await.unwrap();

        let found = repo.get_by_id(&session.id).await.unwrap().unwrap();
        assert!(found.updated_at >= original_updated);
    }

    // ==================== DELETE TESTS ====================

    #[tokio::test]
    async fn test_delete_removes_session_from_database() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let repo = SqliteIdeationSessionRepository::new(conn);
        let session = create_test_session(&project_id, Some("To Delete"));

        repo.create(session.clone()).await.unwrap();

        let delete_result = repo.delete(&session.id).await;
        assert!(delete_result.is_ok());

        let found = repo.get_by_id(&session.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_succeeds() {
        let conn = setup_test_db();
        let repo = SqliteIdeationSessionRepository::new(conn);
        let id = IdeationSessionId::new();

        // Deleting a non-existent session should not error
        let result = repo.delete(&id).await;
        assert!(result.is_ok());
    }

    // ==================== GET ACTIVE BY PROJECT TESTS ====================

    #[tokio::test]
    async fn test_get_active_by_project_returns_only_active() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let repo = SqliteIdeationSessionRepository::new(conn);

        let active = create_test_session(&project_id, Some("Active"));
        let mut archived = create_test_session(&project_id, Some("Archived"));
        archived.archive();
        let mut converted = create_test_session(&project_id, Some("Converted"));
        converted.mark_converted();

        repo.create(active.clone()).await.unwrap();
        repo.create(archived).await.unwrap();
        repo.create(converted).await.unwrap();

        let result = repo.get_active_by_project(&project_id).await;

        assert!(result.is_ok());
        let sessions = result.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, active.id);
        assert_eq!(sessions[0].status, IdeationSessionStatus::Active);
    }

    #[tokio::test]
    async fn test_get_active_by_project_returns_empty_when_none_active() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let repo = SqliteIdeationSessionRepository::new(conn);

        let mut archived = create_test_session(&project_id, Some("Archived"));
        archived.archive();

        repo.create(archived).await.unwrap();

        let result = repo.get_active_by_project(&project_id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_active_by_project_ordered_by_updated_at_desc() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let repo = SqliteIdeationSessionRepository::new(conn);

        let session1 = IdeationSession::builder()
            .project_id(project_id.clone())
            .title("Oldest Active")
            .updated_at(chrono::Utc::now() - chrono::Duration::hours(2))
            .build();
        let session2 = IdeationSession::builder()
            .project_id(project_id.clone())
            .title("Newest Active")
            .updated_at(chrono::Utc::now() - chrono::Duration::hours(1))
            .build();

        repo.create(session1).await.unwrap();
        repo.create(session2).await.unwrap();

        let sessions = repo.get_active_by_project(&project_id).await.unwrap();

        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].title, Some("Newest Active".to_string()));
        assert_eq!(sessions[1].title, Some("Oldest Active".to_string()));
    }

    // ==================== COUNT BY STATUS TESTS ====================

    #[tokio::test]
    async fn test_count_by_status_returns_zero_for_no_sessions() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let repo = SqliteIdeationSessionRepository::new(conn);

        let result = repo.count_by_status(&project_id, IdeationSessionStatus::Active).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_count_by_status_counts_correctly() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let repo = SqliteIdeationSessionRepository::new(conn);

        let active1 = create_test_session(&project_id, Some("Active 1"));
        let active2 = create_test_session(&project_id, Some("Active 2"));
        let mut archived = create_test_session(&project_id, Some("Archived"));
        archived.archive();
        let mut converted = create_test_session(&project_id, Some("Converted"));
        converted.mark_converted();

        repo.create(active1).await.unwrap();
        repo.create(active2).await.unwrap();
        repo.create(archived).await.unwrap();
        repo.create(converted).await.unwrap();

        let active_count = repo.count_by_status(&project_id, IdeationSessionStatus::Active).await.unwrap();
        let archived_count = repo.count_by_status(&project_id, IdeationSessionStatus::Archived).await.unwrap();
        let converted_count = repo.count_by_status(&project_id, IdeationSessionStatus::Converted).await.unwrap();

        assert_eq!(active_count, 2);
        assert_eq!(archived_count, 1);
        assert_eq!(converted_count, 1);
    }

    #[tokio::test]
    async fn test_count_by_status_filters_by_project() {
        let conn = setup_test_db();
        let project_id1 = ProjectId::new();
        let project_id2 = ProjectId::new();
        create_test_project(&conn, &project_id1, "Project 1", "/path1");
        create_test_project(&conn, &project_id2, "Project 2", "/path2");

        let repo = SqliteIdeationSessionRepository::new(conn);

        let session1 = create_test_session(&project_id1, Some("P1 Session"));
        let session2 = create_test_session(&project_id2, Some("P2 Session 1"));
        let session3 = create_test_session(&project_id2, Some("P2 Session 2"));

        repo.create(session1).await.unwrap();
        repo.create(session2).await.unwrap();
        repo.create(session3).await.unwrap();

        let count_p1 = repo.count_by_status(&project_id1, IdeationSessionStatus::Active).await.unwrap();
        let count_p2 = repo.count_by_status(&project_id2, IdeationSessionStatus::Active).await.unwrap();

        assert_eq!(count_p1, 1);
        assert_eq!(count_p2, 2);
    }

    // ==================== SHARED CONNECTION TESTS ====================

    #[tokio::test]
    async fn test_from_shared_works_correctly() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let shared_conn = Arc::new(Mutex::new(conn));
        let repo = SqliteIdeationSessionRepository::from_shared(shared_conn);

        let session = create_test_session(&project_id, Some("Shared Connection"));

        let result = repo.create(session.clone()).await;
        assert!(result.is_ok());

        let found = repo.get_by_id(&session.id).await.unwrap();
        assert!(found.is_some());
    }
}
