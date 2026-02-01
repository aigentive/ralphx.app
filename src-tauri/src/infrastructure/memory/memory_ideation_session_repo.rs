// In-memory IdeationSessionRepository implementation for testing
// Uses RwLock<HashMap> for thread-safe in-memory storage

use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;
use chrono::Utc;

use crate::domain::entities::{IdeationSession, IdeationSessionId, IdeationSessionStatus, ProjectId};
use crate::domain::repositories::IdeationSessionRepository;
use crate::error::AppResult;

/// In-memory implementation of IdeationSessionRepository for testing
pub struct MemoryIdeationSessionRepository {
    sessions: RwLock<HashMap<String, IdeationSession>>,
}

impl MemoryIdeationSessionRepository {
    /// Create a new empty repository
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryIdeationSessionRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl IdeationSessionRepository for MemoryIdeationSessionRepository {
    async fn create(&self, session: IdeationSession) -> AppResult<IdeationSession> {
        self.sessions
            .write()
            .unwrap()
            .insert(session.id.to_string(), session.clone());
        Ok(session)
    }

    async fn get_by_id(&self, id: &IdeationSessionId) -> AppResult<Option<IdeationSession>> {
        Ok(self.sessions.read().unwrap().get(&id.to_string()).cloned())
    }

    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<IdeationSession>> {
        let mut sessions: Vec<_> = self
            .sessions
            .read()
            .unwrap()
            .values()
            .filter(|s| &s.project_id == project_id)
            .cloned()
            .collect();
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(sessions)
    }

    async fn update_status(
        &self,
        id: &IdeationSessionId,
        status: IdeationSessionStatus,
    ) -> AppResult<()> {
        if let Some(session) = self.sessions.write().unwrap().get_mut(&id.to_string()) {
            session.status = status;
            session.updated_at = Utc::now();
            if status == IdeationSessionStatus::Archived {
                session.archived_at = Some(Utc::now());
            }
            if status == IdeationSessionStatus::Accepted {
                session.converted_at = Some(Utc::now());
            }
        }
        Ok(())
    }

    async fn update_title(&self, id: &IdeationSessionId, title: Option<String>) -> AppResult<()> {
        if let Some(session) = self.sessions.write().unwrap().get_mut(&id.to_string()) {
            session.title = title;
            session.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn update_plan_artifact_id(&self, id: &IdeationSessionId, plan_artifact_id: Option<String>) -> AppResult<()> {
        if let Some(session) = self.sessions.write().unwrap().get_mut(&id.to_string()) {
            session.plan_artifact_id = plan_artifact_id.map(crate::domain::entities::ArtifactId::from_string);
            session.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn delete(&self, id: &IdeationSessionId) -> AppResult<()> {
        self.sessions.write().unwrap().remove(&id.to_string());
        Ok(())
    }

    async fn get_active_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<IdeationSession>> {
        let mut sessions: Vec<_> = self
            .sessions
            .read()
            .unwrap()
            .values()
            .filter(|s| &s.project_id == project_id && s.status == IdeationSessionStatus::Active)
            .cloned()
            .collect();
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(sessions)
    }

    async fn count_by_status(
        &self,
        project_id: &ProjectId,
        status: IdeationSessionStatus,
    ) -> AppResult<u32> {
        Ok(self
            .sessions
            .read()
            .unwrap()
            .values()
            .filter(|s| &s.project_id == project_id && s.status == status)
            .count() as u32)
    }

    async fn get_by_plan_artifact_id(
        &self,
        plan_artifact_id: &str,
    ) -> AppResult<Vec<IdeationSession>> {
        Ok(self
            .sessions
            .read()
            .unwrap()
            .values()
            .filter(|s| s.plan_artifact_id.as_ref().map(|id| id.as_str()) == Some(plan_artifact_id))
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_and_get() {
        let repo = MemoryIdeationSessionRepository::new();
        let project_id = ProjectId::new();
        let session = IdeationSession::new_with_title(project_id.clone(), "Test Session");

        repo.create(session.clone()).await.unwrap();

        let retrieved = repo.get_by_id(&session.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, session.id);
    }

    #[tokio::test]
    async fn test_get_by_project() {
        let repo = MemoryIdeationSessionRepository::new();
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());

        repo.create(session).await.unwrap();

        let sessions = repo.get_by_project(&project_id).await.unwrap();
        assert_eq!(sessions.len(), 1);
    }

    #[tokio::test]
    async fn test_update_status() {
        let repo = MemoryIdeationSessionRepository::new();
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();

        repo.create(session).await.unwrap();
        repo.update_status(&session_id, IdeationSessionStatus::Archived)
            .await
            .unwrap();

        let updated = repo.get_by_id(&session_id).await.unwrap().unwrap();
        assert_eq!(updated.status, IdeationSessionStatus::Archived);
        assert!(updated.archived_at.is_some());
    }

    #[tokio::test]
    async fn test_delete() {
        let repo = MemoryIdeationSessionRepository::new();
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();

        repo.create(session).await.unwrap();
        repo.delete(&session_id).await.unwrap();

        let result = repo.get_by_id(&session_id).await.unwrap();
        assert!(result.is_none());
    }
}
