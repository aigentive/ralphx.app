// In-memory IdeationSessionRepository implementation for testing
// Uses RwLock<HashMap> for thread-safe in-memory storage

use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;
use chrono::Utc;

use crate::domain::entities::{
    IdeationSession, IdeationSessionId, IdeationSessionStatus, ProjectId,
};
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
            match status {
                IdeationSessionStatus::Archived => {
                    session.archived_at = Some(Utc::now());
                }
                IdeationSessionStatus::Accepted => {
                    session.converted_at = Some(Utc::now());
                }
                IdeationSessionStatus::Active => {
                    session.archived_at = None;
                    session.converted_at = None;
                }
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
        if let Some(session) = self.sessions.write().unwrap().get_mut(&id.to_string()) {
            session.title = title;
            session.title_source = Some(title_source.to_string());
            session.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn update_plan_artifact_id(
        &self,
        id: &IdeationSessionId,
        plan_artifact_id: Option<String>,
    ) -> AppResult<()> {
        if let Some(session) = self.sessions.write().unwrap().get_mut(&id.to_string()) {
            session.plan_artifact_id =
                plan_artifact_id.map(crate::domain::entities::ArtifactId::from_string);
            session.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn delete(&self, id: &IdeationSessionId) -> AppResult<()> {
        self.sessions.write().unwrap().remove(&id.to_string());
        Ok(())
    }

    async fn get_active_by_project(
        &self,
        project_id: &ProjectId,
    ) -> AppResult<Vec<IdeationSession>> {
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

    async fn get_children(&self, parent_id: &IdeationSessionId) -> AppResult<Vec<IdeationSession>> {
        let mut children: Vec<_> = self
            .sessions
            .read()
            .unwrap()
            .values()
            .filter(|s| s.parent_session_id.as_ref() == Some(parent_id))
            .cloned()
            .collect();
        children.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(children)
    }

    async fn get_ancestor_chain(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<Vec<IdeationSession>> {
        let mut chain = Vec::new();
        let sessions_lock = self.sessions.read().unwrap();
        let mut current_id = session_id.clone();

        // Walk up the parent chain
        loop {
            if let Some(session) = sessions_lock.get(&current_id.to_string()) {
                if let Some(parent_id) = &session.parent_session_id {
                    current_id = parent_id.clone();
                    if let Some(parent) = sessions_lock.get(&current_id.to_string()) {
                        chain.push(parent.clone());
                    } else {
                        // Parent doesn't exist, stop here
                        break;
                    }
                } else {
                    // No parent, end of chain
                    break;
                }
            } else {
                // Session doesn't exist, stop
                break;
            }
        }

        Ok(chain)
    }

    async fn set_parent(
        &self,
        id: &IdeationSessionId,
        parent_id: Option<&IdeationSessionId>,
    ) -> AppResult<()> {
        if let Some(session) = self.sessions.write().unwrap().get_mut(&id.to_string()) {
            session.parent_session_id = parent_id.cloned();
            session.updated_at = Utc::now();
        }
        Ok(())
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

    #[tokio::test]
    async fn test_get_children() {
        let repo = MemoryIdeationSessionRepository::new();
        let project_id = ProjectId::new();

        let parent = IdeationSession::new(project_id.clone());
        let mut child1 = IdeationSession::new(project_id.clone());
        child1.parent_session_id = Some(parent.id.clone());
        let mut child2 = IdeationSession::new(project_id.clone());
        child2.parent_session_id = Some(parent.id.clone());

        repo.create(parent.clone()).await.unwrap();
        repo.create(child1.clone()).await.unwrap();
        repo.create(child2.clone()).await.unwrap();

        let children = repo.get_children(&parent.id).await.unwrap();
        assert_eq!(children.len(), 2);
    }

    #[tokio::test]
    async fn test_get_children_returns_empty_for_sessions_without_children() {
        let repo = MemoryIdeationSessionRepository::new();
        let project_id = ProjectId::new();

        let session = IdeationSession::new(project_id.clone());
        repo.create(session.clone()).await.unwrap();

        let children = repo.get_children(&session.id).await.unwrap();
        assert!(children.is_empty());
    }

    #[tokio::test]
    async fn test_get_ancestor_chain_three_levels_deep() {
        let repo = MemoryIdeationSessionRepository::new();
        let project_id = ProjectId::new();

        let level1 = IdeationSession::new(project_id.clone());
        let mut level2 = IdeationSession::new(project_id.clone());
        level2.parent_session_id = Some(level1.id.clone());
        let mut level3 = IdeationSession::new(project_id.clone());
        level3.parent_session_id = Some(level2.id.clone());

        repo.create(level1.clone()).await.unwrap();
        repo.create(level2.clone()).await.unwrap();
        repo.create(level3.clone()).await.unwrap();

        let chain = repo.get_ancestor_chain(&level3.id).await.unwrap();
        // Should return: [level2, level1] (direct parent to root)
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].id, level2.id);
        assert_eq!(chain[1].id, level1.id);
    }

    #[tokio::test]
    async fn test_get_ancestor_chain_single_parent() {
        let repo = MemoryIdeationSessionRepository::new();
        let project_id = ProjectId::new();

        let parent = IdeationSession::new(project_id.clone());
        let mut child = IdeationSession::new(project_id.clone());
        child.parent_session_id = Some(parent.id.clone());

        repo.create(parent.clone()).await.unwrap();
        repo.create(child.clone()).await.unwrap();

        let chain = repo.get_ancestor_chain(&child.id).await.unwrap();
        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0].id, parent.id);
    }

    #[tokio::test]
    async fn test_get_ancestor_chain_no_parent() {
        let repo = MemoryIdeationSessionRepository::new();
        let project_id = ProjectId::new();

        let session = IdeationSession::new(project_id.clone());
        repo.create(session.clone()).await.unwrap();

        let chain = repo.get_ancestor_chain(&session.id).await.unwrap();
        assert!(chain.is_empty());
    }

    #[tokio::test]
    async fn test_set_parent() {
        let repo = MemoryIdeationSessionRepository::new();
        let project_id = ProjectId::new();

        let parent = IdeationSession::new(project_id.clone());
        let child = IdeationSession::new(project_id.clone());

        repo.create(parent.clone()).await.unwrap();
        repo.create(child.clone()).await.unwrap();

        repo.set_parent(&child.id, Some(&parent.id)).await.unwrap();

        let updated_child = repo.get_by_id(&child.id).await.unwrap().unwrap();
        assert_eq!(updated_child.parent_session_id, Some(parent.id.clone()));
    }

    #[tokio::test]
    async fn test_set_parent_with_null() {
        let repo = MemoryIdeationSessionRepository::new();
        let project_id = ProjectId::new();

        let parent = IdeationSession::new(project_id.clone());
        let mut child = IdeationSession::new(project_id.clone());
        child.parent_session_id = Some(parent.id.clone());

        repo.create(parent.clone()).await.unwrap();
        repo.create(child.clone()).await.unwrap();

        repo.set_parent(&child.id, None).await.unwrap();

        let updated_child = repo.get_by_id(&child.id).await.unwrap().unwrap();
        assert!(updated_child.parent_session_id.is_none());
    }
}
