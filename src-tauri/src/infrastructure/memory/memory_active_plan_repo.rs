// Memory-based ActivePlanRepository implementation for testing
// Uses RwLock for thread-safe storage without a real database

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::domain::entities::{IdeationSessionId, ProjectId};
use crate::domain::repositories::ActivePlanRepository;

/// In-memory implementation of ActivePlanRepository for testing
pub struct MemoryActivePlanRepository {
    active_plans: Arc<RwLock<HashMap<String, String>>>, // project_id -> session_id
}

impl Default for MemoryActivePlanRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryActivePlanRepository {
    /// Create a new empty in-memory active plan repository
    pub fn new() -> Self {
        Self {
            active_plans: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl ActivePlanRepository for MemoryActivePlanRepository {
    async fn get(
        &self,
        project_id: &ProjectId,
    ) -> Result<Option<IdeationSessionId>, Box<dyn std::error::Error>> {
        let plans = self.active_plans.read().await;
        Ok(plans
            .get(project_id.as_str())
            .map(|s| IdeationSessionId::from_string(s.clone())))
    }

    async fn set(
        &self,
        project_id: &ProjectId,
        ideation_session_id: &IdeationSessionId,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut plans = self.active_plans.write().await;
        plans.insert(
            project_id.as_str().to_string(),
            ideation_session_id.as_str().to_string(),
        );
        Ok(())
    }

    async fn clear(&self, project_id: &ProjectId) -> Result<(), Box<dyn std::error::Error>> {
        let mut plans = self.active_plans.write().await;
        plans.remove(project_id.as_str());
        Ok(())
    }

    async fn exists(&self, project_id: &ProjectId) -> Result<bool, Box<dyn std::error::Error>> {
        let plans = self.active_plans.read().await;
        Ok(plans.contains_key(project_id.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_returns_none_when_no_active_plan() {
        let repo = MemoryActivePlanRepository::new();
        let project_id = ProjectId::from_string("proj-123".to_string());

        let result = repo.get(&project_id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_set_and_get_active_plan() {
        let repo = MemoryActivePlanRepository::new();
        let project_id = ProjectId::from_string("proj-123".to_string());
        let session_id = IdeationSessionId::from_string("session-456");

        repo.set(&project_id, &session_id).await.unwrap();

        let result = repo.get(&project_id).await.unwrap();
        assert_eq!(result, Some(session_id));
    }

    #[tokio::test]
    async fn test_set_updates_existing_active_plan() {
        let repo = MemoryActivePlanRepository::new();
        let project_id = ProjectId::from_string("proj-123".to_string());
        let session_id1 = IdeationSessionId::from_string("session-456");
        let session_id2 = IdeationSessionId::from_string("session-789");

        repo.set(&project_id, &session_id1).await.unwrap();
        repo.set(&project_id, &session_id2).await.unwrap();

        let result = repo.get(&project_id).await.unwrap();
        assert_eq!(result, Some(session_id2));
    }

    #[tokio::test]
    async fn test_clear_removes_active_plan() {
        let repo = MemoryActivePlanRepository::new();
        let project_id = ProjectId::from_string("proj-123".to_string());
        let session_id = IdeationSessionId::from_string("session-456");

        repo.set(&project_id, &session_id).await.unwrap();
        repo.clear(&project_id).await.unwrap();

        let result = repo.get(&project_id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_exists_returns_false_when_no_active_plan() {
        let repo = MemoryActivePlanRepository::new();
        let project_id = ProjectId::from_string("proj-123".to_string());

        let exists = repo.exists(&project_id).await.unwrap();
        assert!(!exists);
    }

    #[tokio::test]
    async fn test_exists_returns_true_when_active_plan_set() {
        let repo = MemoryActivePlanRepository::new();
        let project_id = ProjectId::from_string("proj-123".to_string());
        let session_id = IdeationSessionId::from_string("session-456");

        repo.set(&project_id, &session_id).await.unwrap();

        let exists = repo.exists(&project_id).await.unwrap();
        assert!(exists);
    }

    #[tokio::test]
    async fn test_multiple_projects() {
        let repo = MemoryActivePlanRepository::new();
        let project_id1 = ProjectId::from_string("proj-123".to_string());
        let project_id2 = ProjectId::from_string("proj-456".to_string());
        let session_id1 = IdeationSessionId::from_string("session-789");
        let session_id2 = IdeationSessionId::from_string("session-101");

        repo.set(&project_id1, &session_id1).await.unwrap();
        repo.set(&project_id2, &session_id2).await.unwrap();

        let result1 = repo.get(&project_id1).await.unwrap();
        let result2 = repo.get(&project_id2).await.unwrap();

        assert_eq!(result1, Some(session_id1));
        assert_eq!(result2, Some(session_id2));
    }
}
