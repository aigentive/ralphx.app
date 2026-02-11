use crate::domain::entities::{IdeationSessionId, ProjectId};
use crate::domain::repositories::ActivePlanRepository;
use async_trait::async_trait;
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;

/// SQLite implementation of ActivePlanRepository
/// Manages the project_active_plan table with validation
pub struct SqliteActivePlanRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteActivePlanRepository {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl ActivePlanRepository for SqliteActivePlanRepository {
    async fn get(
        &self,
        project_id: &ProjectId,
    ) -> Result<Option<IdeationSessionId>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare(
            "SELECT ideation_session_id FROM project_active_plan WHERE project_id = ?1",
        )?;

        let result = stmt.query_row([project_id.as_str()], |row| {
            let session_id: String = row.get(0)?;
            Ok(IdeationSessionId::from_string(session_id))
        });

        match result {
            Ok(session_id) => Ok(Some(session_id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Box::new(e)),
        }
    }

    async fn set(
        &self,
        project_id: &ProjectId,
        ideation_session_id: &IdeationSessionId,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().await;

        // Validate that the session exists, belongs to the project, and is accepted
        let is_valid: Result<bool, rusqlite::Error> = conn.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM ideation_sessions
                WHERE id = ?1
                  AND project_id = ?2
                  AND status = 'accepted'
                  AND converted_at IS NOT NULL
            )",
            [ideation_session_id.as_str(), project_id.as_str()],
            |row| row.get(0),
        );

        match is_valid {
            Ok(true) => {
                // Session is valid, proceed with upsert
                conn.execute(
                    "INSERT INTO project_active_plan (project_id, ideation_session_id, updated_at)
                     VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))
                     ON CONFLICT(project_id) DO UPDATE SET
                         ideation_session_id = excluded.ideation_session_id,
                         updated_at = excluded.updated_at",
                    [project_id.as_str(), ideation_session_id.as_str()],
                )?;
                Ok(())
            }
            Ok(false) => Err(Box::from(
                "Session must be accepted and belong to the project",
            )),
            Err(e) => Err(Box::new(e)),
        }
    }

    async fn clear(&self, project_id: &ProjectId) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().await;

        conn.execute(
            "DELETE FROM project_active_plan WHERE project_id = ?1",
            [project_id.as_str()],
        )?;

        Ok(())
    }

    async fn exists(&self, project_id: &ProjectId) -> Result<bool, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().await;

        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM project_active_plan WHERE project_id = ?1)",
            [project_id.as_str()],
            |row| row.get(0),
        )?;

        Ok(exists)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

    async fn setup_test_data(conn: &Connection, project_id: &str, session_id: &str) {
        // Insert test project
        conn.execute(
            "INSERT INTO projects (id, name, working_directory, created_at, updated_at)
             VALUES (?1, 'Test Project', '/test', strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'), strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
            [project_id],
        )
        .unwrap();

        // Insert test ideation session (accepted)
        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, status, created_at, updated_at, converted_at)
             VALUES (?1, ?2, 'accepted', strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'), strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'), strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
            [session_id, project_id],
        )
        .unwrap();
    }

    #[tokio::test]
    async fn test_get_returns_none_when_no_active_plan() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let project_id = ProjectId::from_string("proj-123".to_string());
        setup_test_data(&conn, project_id.as_str(), "session-456").await;

        let repo = SqliteActivePlanRepository::new(conn);
        let result = repo.get(&project_id).await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_set_and_get_active_plan() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let project_id = ProjectId::from_string("proj-123".to_string());
        let session_id = IdeationSessionId::from_string("session-456");
        setup_test_data(&conn, project_id.as_str(), session_id.as_str()).await;

        let repo = SqliteActivePlanRepository::new(conn);

        // Set active plan
        repo.set(&project_id, &session_id).await.unwrap();

        // Get active plan
        let result = repo.get(&project_id).await.unwrap();
        assert_eq!(result, Some(session_id));
    }

    #[tokio::test]
    async fn test_set_updates_existing_active_plan() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let project_id = ProjectId::from_string("proj-123".to_string());
        let session_id1 = IdeationSessionId::from_string("session-456");
        let session_id2 = IdeationSessionId::from_string("session-789");

        setup_test_data(&conn, project_id.as_str(), session_id1.as_str()).await;
        // Add second session
        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, status, created_at, updated_at, converted_at)
             VALUES (?1, ?2, 'accepted', strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'), strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'), strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
            [session_id2.as_str(), project_id.as_str()],
        )
        .unwrap();

        let repo = SqliteActivePlanRepository::new(conn);

        // Set first active plan
        repo.set(&project_id, &session_id1).await.unwrap();

        // Update to second active plan
        repo.set(&project_id, &session_id2).await.unwrap();

        // Verify it's updated
        let result = repo.get(&project_id).await.unwrap();
        assert_eq!(result, Some(session_id2));
    }

    #[tokio::test]
    async fn test_set_rejects_non_accepted_session() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let project_id = ProjectId::from_string("proj-123".to_string());
        let session_id = IdeationSessionId::from_string("session-456");

        // Insert project
        conn.execute(
            "INSERT INTO projects (id, name, working_directory, created_at, updated_at)
             VALUES (?1, 'Test Project', '/test', strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'), strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
            [project_id.as_str()],
        )
        .unwrap();

        // Insert session with 'active' status (not accepted)
        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, status, created_at, updated_at)
             VALUES (?1, ?2, 'active', strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'), strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
            [session_id.as_str(), project_id.as_str()],
        )
        .unwrap();

        let repo = SqliteActivePlanRepository::new(conn);

        // Try to set non-accepted session
        let result = repo.set(&project_id, &session_id).await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must be accepted"));
    }

    #[tokio::test]
    async fn test_set_rejects_session_from_different_project() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let project_id1 = ProjectId::from_string("proj-123".to_string());
        let project_id2 = ProjectId::from_string("proj-456".to_string());
        let session_id = IdeationSessionId::from_string("session-789");

        // Setup project 1
        setup_test_data(&conn, project_id1.as_str(), session_id.as_str()).await;

        // Setup project 2 (no sessions)
        conn.execute(
            "INSERT INTO projects (id, name, working_directory, created_at, updated_at)
             VALUES (?1, 'Test Project 2', '/test2', strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'), strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
            [project_id2.as_str()],
        )
        .unwrap();

        let repo = SqliteActivePlanRepository::new(conn);

        // Try to set session from project1 as active for project2
        let result = repo.set(&project_id2, &session_id).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_clear_removes_active_plan() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let project_id = ProjectId::from_string("proj-123".to_string());
        let session_id = IdeationSessionId::from_string("session-456");
        setup_test_data(&conn, project_id.as_str(), session_id.as_str()).await;

        let repo = SqliteActivePlanRepository::new(conn);

        // Set active plan
        repo.set(&project_id, &session_id).await.unwrap();

        // Clear it
        repo.clear(&project_id).await.unwrap();

        // Verify it's gone
        let result = repo.get(&project_id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_exists_returns_false_when_no_active_plan() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let project_id = ProjectId::from_string("proj-123".to_string());
        setup_test_data(&conn, project_id.as_str(), "session-456").await;

        let repo = SqliteActivePlanRepository::new(conn);
        let exists = repo.exists(&project_id).await.unwrap();

        assert!(!exists);
    }

    #[tokio::test]
    async fn test_exists_returns_true_when_active_plan_set() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let project_id = ProjectId::from_string("proj-123".to_string());
        let session_id = IdeationSessionId::from_string("session-456");
        setup_test_data(&conn, project_id.as_str(), session_id.as_str()).await;

        let repo = SqliteActivePlanRepository::new(conn);

        // Set active plan
        repo.set(&project_id, &session_id).await.unwrap();

        // Check exists
        let exists = repo.exists(&project_id).await.unwrap();
        assert!(exists);
    }

    #[tokio::test]
    async fn test_cascade_delete_when_session_deleted() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let project_id = ProjectId::from_string("proj-123".to_string());
        let session_id = IdeationSessionId::from_string("session-456");
        setup_test_data(&conn, project_id.as_str(), session_id.as_str()).await;

        let shared_conn = Arc::new(Mutex::new(conn));
        let repo = SqliteActivePlanRepository::from_shared(Arc::clone(&shared_conn));

        // Set active plan
        repo.set(&project_id, &session_id).await.unwrap();

        // Delete the session
        {
            let conn = shared_conn.lock().await;
            conn.execute(
                "DELETE FROM ideation_sessions WHERE id = ?1",
                [session_id.as_str()],
            )
            .unwrap();
        }

        // Active plan should be gone due to CASCADE
        let result = repo.get(&project_id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cascade_delete_when_project_deleted() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let project_id = ProjectId::from_string("proj-123".to_string());
        let session_id = IdeationSessionId::from_string("session-456");
        setup_test_data(&conn, project_id.as_str(), session_id.as_str()).await;

        let shared_conn = Arc::new(Mutex::new(conn));
        let repo = SqliteActivePlanRepository::from_shared(Arc::clone(&shared_conn));

        // Set active plan
        repo.set(&project_id, &session_id).await.unwrap();

        // Delete the project
        {
            let conn = shared_conn.lock().await;
            conn.execute(
                "DELETE FROM projects WHERE id = ?1",
                [project_id.as_str()],
            )
            .unwrap();
        }

        // Active plan should be gone due to CASCADE
        let result = repo.get(&project_id).await.unwrap();
        assert!(result.is_none());
    }
}
