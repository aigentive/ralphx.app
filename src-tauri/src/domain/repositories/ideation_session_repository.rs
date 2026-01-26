// Ideation session repository trait - domain layer abstraction
//
// This trait defines the contract for ideation session persistence.
// Implementations can use SQLite, PostgreSQL, in-memory, etc.

use async_trait::async_trait;

use crate::domain::entities::{IdeationSession, IdeationSessionId, IdeationSessionStatus, ProjectId};
use crate::error::AppResult;

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

    /// Update session title
    async fn update_title(&self, id: &IdeationSessionId, title: Option<String>) -> AppResult<()>;

    /// Update session plan artifact ID
    async fn update_plan_artifact_id(&self, id: &IdeationSessionId, plan_artifact_id: Option<String>) -> AppResult<()>;

    /// Delete session (cascades to proposals and messages)
    async fn delete(&self, id: &IdeationSessionId) -> AppResult<()>;

    /// Get active sessions for a project
    async fn get_active_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<IdeationSession>>;

    /// Count sessions by status for a project
    async fn count_by_status(
        &self,
        project_id: &ProjectId,
        status: IdeationSessionStatus,
    ) -> AppResult<u32>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::sync::Arc;

    // Mock implementation for testing trait object usage
    struct MockIdeationSessionRepository {
        return_session: Option<IdeationSession>,
        sessions: Vec<IdeationSession>,
    }

    impl MockIdeationSessionRepository {
        fn new() -> Self {
            Self {
                return_session: None,
                sessions: vec![],
            }
        }

        fn with_session(session: IdeationSession) -> Self {
            Self {
                return_session: Some(session.clone()),
                sessions: vec![session],
            }
        }

        fn with_sessions(sessions: Vec<IdeationSession>) -> Self {
            Self {
                return_session: sessions.first().cloned(),
                sessions,
            }
        }
    }

    #[async_trait]
    impl IdeationSessionRepository for MockIdeationSessionRepository {
        async fn create(&self, session: IdeationSession) -> AppResult<IdeationSession> {
            Ok(session)
        }

        async fn get_by_id(&self, _id: &IdeationSessionId) -> AppResult<Option<IdeationSession>> {
            Ok(self.return_session.clone())
        }

        async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<IdeationSession>> {
            Ok(self
                .sessions
                .iter()
                .filter(|s| &s.project_id == project_id)
                .cloned()
                .collect())
        }

        async fn update_status(
            &self,
            _id: &IdeationSessionId,
            _status: IdeationSessionStatus,
        ) -> AppResult<()> {
            Ok(())
        }

        async fn update_title(
            &self,
            _id: &IdeationSessionId,
            _title: Option<String>,
        ) -> AppResult<()> {
            Ok(())
        }

        async fn update_plan_artifact_id(
            &self,
            _id: &IdeationSessionId,
            _plan_artifact_id: Option<String>,
        ) -> AppResult<()> {
            Ok(())
        }

        async fn delete(&self, _id: &IdeationSessionId) -> AppResult<()> {
            Ok(())
        }

        async fn get_active_by_project(
            &self,
            project_id: &ProjectId,
        ) -> AppResult<Vec<IdeationSession>> {
            Ok(self
                .sessions
                .iter()
                .filter(|s| &s.project_id == project_id && s.status == IdeationSessionStatus::Active)
                .cloned()
                .collect())
        }

        async fn count_by_status(
            &self,
            project_id: &ProjectId,
            status: IdeationSessionStatus,
        ) -> AppResult<u32> {
            Ok(self
                .sessions
                .iter()
                .filter(|s| &s.project_id == project_id && s.status == status)
                .count() as u32)
        }
    }

    fn create_test_session(project_id: &ProjectId) -> IdeationSession {
        IdeationSession {
            id: IdeationSessionId::new(),
            project_id: project_id.clone(),
            title: Some("Test Session".to_string()),
            status: IdeationSessionStatus::Active,
            plan_artifact_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            archived_at: None,
            converted_at: None,
        }
    }

    #[test]
    fn test_ideation_session_repository_trait_can_be_object_safe() {
        // Verify that IdeationSessionRepository can be used as a trait object
        let repo: Arc<dyn IdeationSessionRepository> =
            Arc::new(MockIdeationSessionRepository::new());
        assert!(Arc::strong_count(&repo) == 1);
    }

    #[tokio::test]
    async fn test_mock_repository_create() {
        let repo = MockIdeationSessionRepository::new();
        let project_id = ProjectId::new();
        let session = create_test_session(&project_id);

        let result = repo.create(session.clone()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, session.id);
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_id_returns_none() {
        let repo = MockIdeationSessionRepository::new();
        let session_id = IdeationSessionId::new();

        let result = repo.get_by_id(&session_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_id_returns_session() {
        let project_id = ProjectId::new();
        let session = create_test_session(&project_id);
        let repo = MockIdeationSessionRepository::with_session(session.clone());

        let result = repo.get_by_id(&session.id).await;
        assert!(result.is_ok());
        let returned = result.unwrap();
        assert!(returned.is_some());
        assert_eq!(returned.unwrap().id, session.id);
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_project_empty() {
        let repo = MockIdeationSessionRepository::new();
        let project_id = ProjectId::new();

        let result = repo.get_by_project(&project_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_project_with_sessions() {
        let project_id = ProjectId::new();
        let session1 = create_test_session(&project_id);
        let session2 = create_test_session(&project_id);

        let repo = MockIdeationSessionRepository::with_sessions(vec![session1.clone(), session2.clone()]);

        let result = repo.get_by_project(&project_id).await;
        assert!(result.is_ok());
        let sessions = result.unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_project_filters_by_project() {
        let project_id1 = ProjectId::new();
        let project_id2 = ProjectId::new();
        let session1 = create_test_session(&project_id1);
        let session2 = create_test_session(&project_id2);

        let repo = MockIdeationSessionRepository::with_sessions(vec![session1.clone(), session2.clone()]);

        let result = repo.get_by_project(&project_id1).await;
        assert!(result.is_ok());
        let sessions = result.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].project_id, project_id1);
    }

    #[tokio::test]
    async fn test_mock_repository_update_status() {
        let repo = MockIdeationSessionRepository::new();
        let session_id = IdeationSessionId::new();

        let result = repo
            .update_status(&session_id, IdeationSessionStatus::Archived)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_repository_update_title() {
        let repo = MockIdeationSessionRepository::new();
        let session_id = IdeationSessionId::new();

        let result = repo
            .update_title(&session_id, Some("New Title".to_string()))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_repository_update_title_to_none() {
        let repo = MockIdeationSessionRepository::new();
        let session_id = IdeationSessionId::new();

        let result = repo.update_title(&session_id, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_repository_delete() {
        let repo = MockIdeationSessionRepository::new();
        let session_id = IdeationSessionId::new();

        let result = repo.delete(&session_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_repository_get_active_by_project_empty() {
        let repo = MockIdeationSessionRepository::new();
        let project_id = ProjectId::new();

        let result = repo.get_active_by_project(&project_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_repository_get_active_by_project_filters_status() {
        let project_id = ProjectId::new();
        let mut active_session = create_test_session(&project_id);
        active_session.status = IdeationSessionStatus::Active;

        let mut archived_session = create_test_session(&project_id);
        archived_session.status = IdeationSessionStatus::Archived;

        let repo = MockIdeationSessionRepository::with_sessions(vec![
            active_session.clone(),
            archived_session.clone(),
        ]);

        let result = repo.get_active_by_project(&project_id).await;
        assert!(result.is_ok());
        let sessions = result.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].status, IdeationSessionStatus::Active);
    }

    #[tokio::test]
    async fn test_mock_repository_count_by_status_zero() {
        let repo = MockIdeationSessionRepository::new();
        let project_id = ProjectId::new();

        let result = repo
            .count_by_status(&project_id, IdeationSessionStatus::Active)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_mock_repository_count_by_status_counts_correctly() {
        let project_id = ProjectId::new();
        let mut active1 = create_test_session(&project_id);
        active1.status = IdeationSessionStatus::Active;

        let mut active2 = create_test_session(&project_id);
        active2.status = IdeationSessionStatus::Active;

        let mut archived = create_test_session(&project_id);
        archived.status = IdeationSessionStatus::Archived;

        let repo = MockIdeationSessionRepository::with_sessions(vec![
            active1.clone(),
            active2.clone(),
            archived.clone(),
        ]);

        let active_count = repo
            .count_by_status(&project_id, IdeationSessionStatus::Active)
            .await;
        assert!(active_count.is_ok());
        assert_eq!(active_count.unwrap(), 2);

        let archived_count = repo
            .count_by_status(&project_id, IdeationSessionStatus::Archived)
            .await;
        assert!(archived_count.is_ok());
        assert_eq!(archived_count.unwrap(), 1);

        let converted_count = repo
            .count_by_status(&project_id, IdeationSessionStatus::Converted)
            .await;
        assert!(converted_count.is_ok());
        assert_eq!(converted_count.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_repository_trait_object_in_arc() {
        let project_id = ProjectId::new();
        let session = create_test_session(&project_id);
        let repo: Arc<dyn IdeationSessionRepository> =
            Arc::new(MockIdeationSessionRepository::with_session(session.clone()));

        // Use through trait object
        let result = repo.get_by_id(&session.id).await;
        assert!(result.is_ok());

        let all = repo.get_by_project(&project_id).await;
        assert!(all.is_ok());
        assert_eq!(all.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_repository_trait_object_status_operations() {
        let project_id = ProjectId::new();
        let session = create_test_session(&project_id);
        let repo: Arc<dyn IdeationSessionRepository> =
            Arc::new(MockIdeationSessionRepository::with_session(session.clone()));

        // Test update_status through trait object
        let result = repo
            .update_status(&session.id, IdeationSessionStatus::Converted)
            .await;
        assert!(result.is_ok());

        // Test count_by_status through trait object
        let count = repo
            .count_by_status(&project_id, IdeationSessionStatus::Active)
            .await;
        assert!(count.is_ok());
    }

    #[tokio::test]
    async fn test_repository_trait_object_title_operations() {
        let project_id = ProjectId::new();
        let session = create_test_session(&project_id);
        let repo: Arc<dyn IdeationSessionRepository> =
            Arc::new(MockIdeationSessionRepository::with_session(session.clone()));

        // Test update_title through trait object
        let result = repo
            .update_title(&session.id, Some("Updated Title".to_string()))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_repository_trait_object_delete_operation() {
        let project_id = ProjectId::new();
        let session = create_test_session(&project_id);
        let repo: Arc<dyn IdeationSessionRepository> =
            Arc::new(MockIdeationSessionRepository::with_session(session.clone()));

        // Test delete through trait object
        let result = repo.delete(&session.id).await;
        assert!(result.is_ok());
    }
}
