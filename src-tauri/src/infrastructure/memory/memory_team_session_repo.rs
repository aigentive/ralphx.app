// In-memory TeamSessionRepository implementation for testing

use async_trait::async_trait;
use chrono::Utc;
use std::sync::RwLock;

use crate::domain::entities::team::{TeamSession, TeamSessionId, TeammateSnapshot};
use crate::domain::repositories::TeamSessionRepository;
use crate::error::AppResult;

pub struct MemoryTeamSessionRepository {
    sessions: RwLock<Vec<TeamSession>>,
}

impl MemoryTeamSessionRepository {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(Vec::new()),
        }
    }
}

impl Default for MemoryTeamSessionRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TeamSessionRepository for MemoryTeamSessionRepository {
    async fn create(&self, session: TeamSession) -> AppResult<TeamSession> {
        self.sessions.write().unwrap().push(session.clone());
        Ok(session)
    }

    async fn get_by_id(&self, id: &TeamSessionId) -> AppResult<Option<TeamSession>> {
        Ok(self
            .sessions
            .read()
            .unwrap()
            .iter()
            .find(|s| s.id == *id)
            .cloned())
    }

    async fn get_by_context(
        &self,
        context_type: &str,
        context_id: &str,
    ) -> AppResult<Vec<TeamSession>> {
        let mut results: Vec<_> = self
            .sessions
            .read()
            .unwrap()
            .iter()
            .filter(|s| s.context_type == context_type && s.context_id == context_id)
            .cloned()
            .collect();
        results.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(results)
    }

    async fn get_active_for_context(
        &self,
        context_type: &str,
        context_id: &str,
    ) -> AppResult<Option<TeamSession>> {
        Ok(self
            .sessions
            .read()
            .unwrap()
            .iter()
            .filter(|s| {
                s.context_type == context_type
                    && s.context_id == context_id
                    && s.disbanded_at.is_none()
            })
            .max_by_key(|s| s.created_at)
            .cloned())
    }

    async fn update_phase(&self, id: &TeamSessionId, phase: &str) -> AppResult<()> {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(s) = sessions.iter_mut().find(|s| s.id == *id) {
            s.phase = phase.to_string();
            s.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn update_teammates(
        &self,
        id: &TeamSessionId,
        teammates: &[TeammateSnapshot],
    ) -> AppResult<()> {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(s) = sessions.iter_mut().find(|s| s.id == *id) {
            s.teammates = teammates.to_vec();
            s.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn set_disbanded(&self, id: &TeamSessionId) -> AppResult<()> {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(s) = sessions.iter_mut().find(|s| s.id == *id) {
            let now = Utc::now();
            s.disbanded_at = Some(now);
            s.updated_at = now;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_and_get() {
        let repo = MemoryTeamSessionRepository::new();
        let session = TeamSession::new("team-1", "ctx-1", "task");
        let id = session.id.clone();

        repo.create(session).await.unwrap();
        let found = repo.get_by_id(&id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().team_name, "team-1");
    }

    #[tokio::test]
    async fn test_get_by_context() {
        let repo = MemoryTeamSessionRepository::new();
        let s1 = TeamSession::new("team-a", "ctx-1", "task");
        let s2 = TeamSession::new("team-b", "ctx-2", "project");

        repo.create(s1).await.unwrap();
        repo.create(s2).await.unwrap();

        let results = repo.get_by_context("task", "ctx-1").await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_get_active_for_context() {
        let repo = MemoryTeamSessionRepository::new();
        let s1 = TeamSession::new("team-a", "ctx-1", "task");
        let id1 = s1.id.clone();

        repo.create(s1).await.unwrap();
        let active = repo.get_active_for_context("task", "ctx-1").await.unwrap();
        assert!(active.is_some());

        repo.set_disbanded(&id1).await.unwrap();
        let active = repo.get_active_for_context("task", "ctx-1").await.unwrap();
        assert!(active.is_none());
    }

    #[tokio::test]
    async fn test_update_phase() {
        let repo = MemoryTeamSessionRepository::new();
        let session = TeamSession::new("team-1", "ctx-1", "task");
        let id = session.id.clone();

        repo.create(session).await.unwrap();
        repo.update_phase(&id, "working").await.unwrap();

        let found = repo.get_by_id(&id).await.unwrap().unwrap();
        assert_eq!(found.phase, "working");
    }

    #[tokio::test]
    async fn test_update_teammates() {
        let repo = MemoryTeamSessionRepository::new();
        let session = TeamSession::new("team-1", "ctx-1", "task");
        let id = session.id.clone();

        repo.create(session).await.unwrap();

        let teammates = vec![TeammateSnapshot {
            name: "worker-1".to_string(),
            color: "#ff6b35".to_string(),
            model: "sonnet".to_string(),
            role: "coder".to_string(),
            status: "active".to_string(),
            cost: crate::application::team_state_tracker::TeammateCost {
                input_tokens: 1000,
                output_tokens: 500,
                cache_creation_tokens: 200,
                cache_read_tokens: 100,
                estimated_usd: 0.05,
            },
            spawned_at: "2024-01-01T00:00:00Z".to_string(),
            last_activity_at: "2024-01-01T00:01:00Z".to_string(),
        }];
        repo.update_teammates(&id, &teammates).await.unwrap();

        let found = repo.get_by_id(&id).await.unwrap().unwrap();
        assert_eq!(found.teammates.len(), 1);
        assert_eq!(found.teammates[0].name, "worker-1");
    }
}
