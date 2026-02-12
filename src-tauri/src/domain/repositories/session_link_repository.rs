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
mod tests {
    use super::*;
    use std::sync::Arc;
    use crate::domain::entities::SessionRelationship;

    // Mock implementation for testing trait object usage
    struct MockSessionLinkRepository {
        links: Vec<SessionLink>,
    }

    impl MockSessionLinkRepository {
        fn new() -> Self {
            Self {
                links: vec![],
            }
        }

        fn with_links(links: Vec<SessionLink>) -> Self {
            Self { links }
        }
    }

    #[async_trait]
    impl SessionLinkRepository for MockSessionLinkRepository {
        async fn create(&self, link: SessionLink) -> AppResult<SessionLink> {
            Ok(link)
        }

        async fn get_by_parent(&self, parent_id: &IdeationSessionId) -> AppResult<Vec<SessionLink>> {
            Ok(self
                .links
                .iter()
                .filter(|link| &link.parent_session_id == parent_id)
                .cloned()
                .collect())
        }

        async fn get_by_child(&self, child_id: &IdeationSessionId) -> AppResult<Vec<SessionLink>> {
            Ok(self
                .links
                .iter()
                .filter(|link| &link.child_session_id == child_id)
                .cloned()
                .collect())
        }

        async fn delete(&self, _id: &SessionLinkId) -> AppResult<()> {
            Ok(())
        }

        async fn delete_by_child(&self, _child_id: &IdeationSessionId) -> AppResult<()> {
            Ok(())
        }
    }

    #[test]
    fn test_session_link_repository_trait_can_be_object_safe() {
        // Verify that SessionLinkRepository can be used as a trait object
        let repo: Arc<dyn SessionLinkRepository> = Arc::new(MockSessionLinkRepository::new());
        assert!(Arc::strong_count(&repo) == 1);
    }

    #[tokio::test]
    async fn test_mock_repository_create() {
        let repo = MockSessionLinkRepository::new();
        let parent_id = IdeationSessionId::new();
        let child_id = IdeationSessionId::new();
        let link = SessionLink::new(
            parent_id.clone(),
            child_id.clone(),
            SessionRelationship::FollowOn,
        );

        let result = repo.create(link.clone()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, link.id);
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_parent() {
        let parent_id = IdeationSessionId::new();
        let child_id1 = IdeationSessionId::new();
        let child_id2 = IdeationSessionId::new();
        let link1 = SessionLink::new(
            parent_id.clone(),
            child_id1.clone(),
            SessionRelationship::FollowOn,
        );
        let link2 = SessionLink::new(
            parent_id.clone(),
            child_id2.clone(),
            SessionRelationship::Alternative,
        );

        let repo = MockSessionLinkRepository::with_links(vec![link1.clone(), link2.clone()]);

        let result = repo.get_by_parent(&parent_id).await;
        assert!(result.is_ok());
        let links = result.unwrap();
        assert_eq!(links.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_parent_empty() {
        let repo = MockSessionLinkRepository::new();
        let parent_id = IdeationSessionId::new();

        let result = repo.get_by_parent(&parent_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_child() {
        let parent_id1 = IdeationSessionId::new();
        let parent_id2 = IdeationSessionId::new();
        let child_id = IdeationSessionId::new();
        let link1 = SessionLink::new(
            parent_id1.clone(),
            child_id.clone(),
            SessionRelationship::FollowOn,
        );
        let link2 = SessionLink::new(
            parent_id2.clone(),
            child_id.clone(),
            SessionRelationship::Alternative,
        );

        let repo = MockSessionLinkRepository::with_links(vec![link1.clone(), link2.clone()]);

        let result = repo.get_by_child(&child_id).await;
        assert!(result.is_ok());
        let links = result.unwrap();
        assert_eq!(links.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_child_empty() {
        let repo = MockSessionLinkRepository::new();
        let child_id = IdeationSessionId::new();

        let result = repo.get_by_child(&child_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_repository_delete() {
        let repo = MockSessionLinkRepository::new();
        let link_id = SessionLinkId::new();

        let result = repo.delete(&link_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_repository_delete_by_child() {
        let repo = MockSessionLinkRepository::new();
        let child_id = IdeationSessionId::new();

        let result = repo.delete_by_child(&child_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_repository_trait_object_in_arc() {
        let repo: Arc<dyn SessionLinkRepository> = Arc::new(MockSessionLinkRepository::new());
        let parent_id = IdeationSessionId::new();

        // Use through trait object
        let result = repo.get_by_parent(&parent_id).await;
        assert!(result.is_ok());
    }
}
