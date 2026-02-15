// SQLite-based SessionLinkRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use rusqlite::Connection;

use crate::domain::entities::{IdeationSessionId, SessionLink, SessionLinkId};
use crate::domain::repositories::SessionLinkRepository;
use crate::error::{AppError, AppResult};

/// SQLite implementation of SessionLinkRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteSessionLinkRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteSessionLinkRepository {
    /// Create a new SQLite session link repository with the given connection
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
impl SessionLinkRepository for SqliteSessionLinkRepository {
    async fn create(&self, link: SessionLink) -> AppResult<SessionLink> {
        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT INTO session_links (id, parent_session_id, child_session_id, relationship, notes, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                link.id.as_str(),
                link.parent_session_id.as_str(),
                link.child_session_id.as_str(),
                link.relationship.to_string(),
                link.notes,
                link.created_at.to_rfc3339(),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(link)
    }

    async fn get_by_parent(&self, parent_id: &IdeationSessionId) -> AppResult<Vec<SessionLink>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, parent_session_id, child_session_id, relationship, notes, created_at
                 FROM session_links WHERE parent_session_id = ?1 ORDER BY created_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let links = stmt
            .query_map([parent_id.as_str()], SessionLink::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(links)
    }

    async fn get_by_child(&self, child_id: &IdeationSessionId) -> AppResult<Vec<SessionLink>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, parent_session_id, child_session_id, relationship, notes, created_at
                 FROM session_links WHERE child_session_id = ?1 ORDER BY created_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let links = stmt
            .query_map([child_id.as_str()], SessionLink::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(links)
    }

    async fn delete(&self, id: &SessionLinkId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute("DELETE FROM session_links WHERE id = ?1", [id.as_str()])
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete_by_child(&self, child_id: &IdeationSessionId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "DELETE FROM session_links WHERE child_session_id = ?1",
            [child_id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::SessionRelationship;

    fn create_test_link(
        parent_id: &IdeationSessionId,
        child_id: &IdeationSessionId,
    ) -> SessionLink {
        SessionLink::new(
            parent_id.clone(),
            child_id.clone(),
            SessionRelationship::FollowOn,
        )
    }

    #[tokio::test]
    async fn test_create_link() {
        let conn =
            rusqlite::Connection::open_in_memory().expect("Failed to open in-memory database");

        // Set up the schema
        conn.execute_batch(
            "CREATE TABLE session_links (
                id TEXT PRIMARY KEY,
                parent_session_id TEXT NOT NULL,
                child_session_id TEXT NOT NULL,
                relationship TEXT NOT NULL,
                notes TEXT,
                created_at TEXT NOT NULL,
                CHECK (parent_session_id != child_session_id),
                UNIQUE(parent_session_id, child_session_id)
            );",
        )
        .expect("Failed to create table");

        let conn = Arc::new(Mutex::new(conn));
        let repo = SqliteSessionLinkRepository::from_shared(conn);

        let parent_id = IdeationSessionId::new();
        let child_id = IdeationSessionId::new();
        let link = create_test_link(&parent_id, &child_id);

        let result = repo.create(link.clone()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, link.id);
    }

    #[tokio::test]
    async fn test_get_by_parent() {
        let conn =
            rusqlite::Connection::open_in_memory().expect("Failed to open in-memory database");

        conn.execute_batch(
            "CREATE TABLE session_links (
                id TEXT PRIMARY KEY,
                parent_session_id TEXT NOT NULL,
                child_session_id TEXT NOT NULL,
                relationship TEXT NOT NULL,
                notes TEXT,
                created_at TEXT NOT NULL,
                CHECK (parent_session_id != child_session_id),
                UNIQUE(parent_session_id, child_session_id)
            );",
        )
        .expect("Failed to create table");

        let conn = Arc::new(Mutex::new(conn));
        let repo = SqliteSessionLinkRepository::from_shared(conn);

        let parent_id = IdeationSessionId::new();
        let child_id1 = IdeationSessionId::new();
        let child_id2 = IdeationSessionId::new();

        let link1 = create_test_link(&parent_id, &child_id1);
        let link2 = create_test_link(&parent_id, &child_id2);

        repo.create(link1.clone())
            .await
            .expect("Failed to create link1");
        repo.create(link2.clone())
            .await
            .expect("Failed to create link2");

        let result = repo.get_by_parent(&parent_id).await;
        assert!(result.is_ok());
        let links = result.unwrap();
        assert_eq!(links.len(), 2);
    }

    #[tokio::test]
    async fn test_get_by_child() {
        let conn =
            rusqlite::Connection::open_in_memory().expect("Failed to open in-memory database");

        conn.execute_batch(
            "CREATE TABLE session_links (
                id TEXT PRIMARY KEY,
                parent_session_id TEXT NOT NULL,
                child_session_id TEXT NOT NULL,
                relationship TEXT NOT NULL,
                notes TEXT,
                created_at TEXT NOT NULL,
                CHECK (parent_session_id != child_session_id),
                UNIQUE(parent_session_id, child_session_id)
            );",
        )
        .expect("Failed to create table");

        let conn = Arc::new(Mutex::new(conn));
        let repo = SqliteSessionLinkRepository::from_shared(conn);

        let parent_id1 = IdeationSessionId::new();
        let parent_id2 = IdeationSessionId::new();
        let child_id = IdeationSessionId::new();

        let link1 = create_test_link(&parent_id1, &child_id);
        let link2 = create_test_link(&parent_id2, &child_id);

        repo.create(link1.clone())
            .await
            .expect("Failed to create link1");
        repo.create(link2.clone())
            .await
            .expect("Failed to create link2");

        let result = repo.get_by_child(&child_id).await;
        assert!(result.is_ok());
        let links = result.unwrap();
        assert_eq!(links.len(), 2);
    }

    #[tokio::test]
    async fn test_delete() {
        let conn =
            rusqlite::Connection::open_in_memory().expect("Failed to open in-memory database");

        conn.execute_batch(
            "CREATE TABLE session_links (
                id TEXT PRIMARY KEY,
                parent_session_id TEXT NOT NULL,
                child_session_id TEXT NOT NULL,
                relationship TEXT NOT NULL,
                notes TEXT,
                created_at TEXT NOT NULL,
                CHECK (parent_session_id != child_session_id),
                UNIQUE(parent_session_id, child_session_id)
            );",
        )
        .expect("Failed to create table");

        let conn = Arc::new(Mutex::new(conn));
        let repo = SqliteSessionLinkRepository::from_shared(conn);

        let parent_id = IdeationSessionId::new();
        let child_id = IdeationSessionId::new();
        let link = create_test_link(&parent_id, &child_id);

        repo.create(link.clone())
            .await
            .expect("Failed to create link");

        // Verify it exists
        let links = repo
            .get_by_parent(&parent_id)
            .await
            .expect("Failed to query");
        assert_eq!(links.len(), 1);

        // Delete it
        let result = repo.delete(&link.id).await;
        assert!(result.is_ok());

        // Verify it's gone
        let links = repo
            .get_by_parent(&parent_id)
            .await
            .expect("Failed to query");
        assert_eq!(links.len(), 0);
    }

    #[tokio::test]
    async fn test_delete_by_child() {
        let conn =
            rusqlite::Connection::open_in_memory().expect("Failed to open in-memory database");

        conn.execute_batch(
            "CREATE TABLE session_links (
                id TEXT PRIMARY KEY,
                parent_session_id TEXT NOT NULL,
                child_session_id TEXT NOT NULL,
                relationship TEXT NOT NULL,
                notes TEXT,
                created_at TEXT NOT NULL,
                CHECK (parent_session_id != child_session_id),
                UNIQUE(parent_session_id, child_session_id)
            );",
        )
        .expect("Failed to create table");

        let conn = Arc::new(Mutex::new(conn));
        let repo = SqliteSessionLinkRepository::from_shared(conn);

        let parent_id1 = IdeationSessionId::new();
        let parent_id2 = IdeationSessionId::new();
        let child_id = IdeationSessionId::new();

        let link1 = create_test_link(&parent_id1, &child_id);
        let link2 = create_test_link(&parent_id2, &child_id);

        repo.create(link1.clone())
            .await
            .expect("Failed to create link1");
        repo.create(link2.clone())
            .await
            .expect("Failed to create link2");

        // Verify both exist
        let links = repo.get_by_child(&child_id).await.expect("Failed to query");
        assert_eq!(links.len(), 2);

        // Delete all links where child is child_id
        let result = repo.delete_by_child(&child_id).await;
        assert!(result.is_ok());

        // Verify all are gone
        let links = repo.get_by_child(&child_id).await.expect("Failed to query");
        assert_eq!(links.len(), 0);
    }
}
