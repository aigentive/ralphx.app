// Session link repository trait - domain layer abstraction
//
// This trait defines the contract for session link persistence.
// Implementations can use SQLite, PostgreSQL, in-memory, etc.

use async_trait::async_trait;

use crate::domain::entities::{IdeationSessionId, SessionLink, SessionLinkId};
use crate::error::AppResult;

/// Repository trait for SessionLink persistence.
/// Implementations can use SQLite, PostgreSQL, in-memory, etc.
#[async_trait]
pub trait SessionLinkRepository: Send + Sync {
    /// Create a new session link
    async fn create(&self, link: SessionLink) -> AppResult<SessionLink>;

    /// Get session links where the given session is the parent
    async fn get_by_parent(&self, parent_id: &IdeationSessionId) -> AppResult<Vec<SessionLink>>;

    /// Get session links where the given session is the child
    async fn get_by_child(&self, child_id: &IdeationSessionId) -> AppResult<Vec<SessionLink>>;

    /// Delete a specific session link by ID
    async fn delete(&self, id: &SessionLinkId) -> AppResult<()>;

    /// Delete all session links where the given session is the child
    async fn delete_by_child(&self, child_id: &IdeationSessionId) -> AppResult<()>;
}

#[cfg(test)]
#[path = "session_link_repository_tests.rs"]
mod tests;
