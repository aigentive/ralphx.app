// SQLite-based SessionLinkRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use rusqlite::Connection;

use super::DbConnection;
use crate::domain::entities::{IdeationSessionId, SessionLink, SessionLinkId};
use crate::domain::repositories::SessionLinkRepository;
use crate::error::AppResult;

/// SQLite implementation of SessionLinkRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteSessionLinkRepository {
    db: DbConnection,
}

impl SqliteSessionLinkRepository {
    /// Create a new SQLite session link repository with the given connection
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
}

#[async_trait]
impl SessionLinkRepository for SqliteSessionLinkRepository {
    async fn create(&self, link: SessionLink) -> AppResult<SessionLink> {
        self.db
            .run(move |conn| {
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
                )?;
                Ok(link)
            })
            .await
    }

    async fn get_by_parent(&self, parent_id: &IdeationSessionId) -> AppResult<Vec<SessionLink>> {
        let parent_id = parent_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, parent_session_id, child_session_id, relationship, notes, created_at
                     FROM session_links WHERE parent_session_id = ?1 ORDER BY created_at DESC",
                )?;
                let links = stmt
                    .query_map([parent_id.as_str()], SessionLink::from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(links)
            })
            .await
    }

    async fn get_by_child(&self, child_id: &IdeationSessionId) -> AppResult<Vec<SessionLink>> {
        let child_id = child_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, parent_session_id, child_session_id, relationship, notes, created_at
                     FROM session_links WHERE child_session_id = ?1 ORDER BY created_at DESC",
                )?;
                let links = stmt
                    .query_map([child_id.as_str()], SessionLink::from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(links)
            })
            .await
    }

    async fn delete(&self, id: &SessionLinkId) -> AppResult<()> {
        let id = id.as_str().to_string();
        self.db
            .run(move |conn| {
                conn.execute("DELETE FROM session_links WHERE id = ?1", [id.as_str()])?;
                Ok(())
            })
            .await
    }

    async fn delete_by_child(&self, child_id: &IdeationSessionId) -> AppResult<()> {
        let child_id = child_id.as_str().to_string();
        self.db
            .run(move |conn| {
                conn.execute(
                    "DELETE FROM session_links WHERE child_session_id = ?1",
                    [child_id.as_str()],
                )?;
                Ok(())
            })
            .await
    }
}

#[cfg(test)]
#[path = "sqlite_session_link_repo_tests.rs"]
mod tests;
