// Memory-based AppStateRepository implementation for testing
// Uses RwLock for thread-safe storage without a real database

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::domain::entities::app_state::AppSettings;
use crate::domain::entities::ProjectId;
use crate::domain::repositories::AppStateRepository;

/// In-memory implementation of AppStateRepository for testing
pub struct MemoryAppStateRepository {
    settings: Arc<RwLock<AppSettings>>,
}

impl Default for MemoryAppStateRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryAppStateRepository {
    /// Create a new empty in-memory app state repository
    pub fn new() -> Self {
        Self {
            settings: Arc::new(RwLock::new(AppSettings::default())),
        }
    }

    /// Create with a specific active project (for tests)
    pub fn with_active_project(project_id: ProjectId) -> Self {
        Self {
            settings: Arc::new(RwLock::new(AppSettings {
                active_project_id: Some(project_id),
            })),
        }
    }
}

#[async_trait]
impl AppStateRepository for MemoryAppStateRepository {
    async fn get(&self) -> Result<AppSettings, Box<dyn std::error::Error>> {
        let settings = self.settings.read().await;
        Ok(settings.clone())
    }

    async fn set_active_project(
        &self,
        project_id: Option<&ProjectId>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut settings = self.settings.write().await;
        settings.active_project_id = project_id.cloned();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_default_app_state() {
        let repo = MemoryAppStateRepository::new();

        let settings = repo.get().await.unwrap();
        assert!(settings.active_project_id.is_none());
    }

    #[tokio::test]
    async fn test_set_and_get_active_project() {
        let repo = MemoryAppStateRepository::new();

        let project_id = ProjectId::from_string("proj-123".to_string());
        repo.set_active_project(Some(&project_id)).await.unwrap();

        let settings = repo.get().await.unwrap();
        assert_eq!(
            settings.active_project_id,
            Some(ProjectId::from_string("proj-123".to_string()))
        );
    }

    #[tokio::test]
    async fn test_clear_active_project() {
        let repo = MemoryAppStateRepository::new();

        let project_id = ProjectId::from_string("proj-123".to_string());
        repo.set_active_project(Some(&project_id)).await.unwrap();

        repo.set_active_project(None).await.unwrap();

        let settings = repo.get().await.unwrap();
        assert!(settings.active_project_id.is_none());
    }

    #[tokio::test]
    async fn test_with_active_project() {
        let project_id = ProjectId::from_string("proj-456".to_string());
        let repo = MemoryAppStateRepository::with_active_project(project_id);

        let settings = repo.get().await.unwrap();
        assert_eq!(
            settings.active_project_id,
            Some(ProjectId::from_string("proj-456".to_string()))
        );
    }
}
