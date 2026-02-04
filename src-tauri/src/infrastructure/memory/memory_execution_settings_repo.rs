// Memory-based ExecutionSettingsRepository implementation for testing
// Uses RwLock for thread-safe storage without a real database
// Phase 82: Extended to support per-project settings and global execution settings

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::domain::entities::ProjectId;
use crate::domain::execution::{ExecutionSettings, GlobalExecutionSettings};
use crate::domain::repositories::{
    ExecutionSettingsRepository, GlobalExecutionSettingsRepository,
};

/// Maximum allowed value for global_max_concurrent
const GLOBAL_MAX_CONCURRENT_LIMIT: u32 = 50;

/// In-memory implementation of ExecutionSettingsRepository for testing
/// Phase 82: Supports per-project settings with project_id key
pub struct MemoryExecutionSettingsRepository {
    /// Global default settings (when project_id is None)
    global_settings: Arc<RwLock<ExecutionSettings>>,
    /// Per-project settings keyed by ProjectId
    project_settings: Arc<RwLock<HashMap<ProjectId, ExecutionSettings>>>,
}

impl Default for MemoryExecutionSettingsRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryExecutionSettingsRepository {
    /// Create a new empty in-memory execution settings repository
    pub fn new() -> Self {
        Self {
            global_settings: Arc::new(RwLock::new(ExecutionSettings::default())),
            project_settings: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create with specific global settings (for tests)
    pub fn with_settings(settings: ExecutionSettings) -> Self {
        Self {
            global_settings: Arc::new(RwLock::new(settings)),
            project_settings: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl ExecutionSettingsRepository for MemoryExecutionSettingsRepository {
    /// Get execution settings for a project
    /// Phase 82: If project_id is None, returns global defaults
    /// If project_id is Some but no project-specific settings exist, returns global defaults
    async fn get_settings(
        &self,
        project_id: Option<&ProjectId>,
    ) -> Result<ExecutionSettings, Box<dyn std::error::Error>> {
        if let Some(pid) = project_id {
            // Try to get project-specific settings first
            let project_settings = self.project_settings.read().await;
            if let Some(settings) = project_settings.get(pid) {
                return Ok(settings.clone());
            }
        }
        // Fall back to global defaults
        let settings = self.global_settings.read().await;
        Ok(settings.clone())
    }

    /// Update execution settings for a project
    /// Phase 82: If project_id is None, updates global defaults
    async fn update_settings(
        &self,
        project_id: Option<&ProjectId>,
        new_settings: &ExecutionSettings,
    ) -> Result<ExecutionSettings, Box<dyn std::error::Error>> {
        if let Some(pid) = project_id {
            let mut project_settings = self.project_settings.write().await;
            project_settings.insert(pid.clone(), new_settings.clone());
        } else {
            let mut settings = self.global_settings.write().await;
            *settings = new_settings.clone();
        }
        Ok(new_settings.clone())
    }
}

/// In-memory implementation of GlobalExecutionSettingsRepository for testing
/// Phase 82: Manages global_max_concurrent setting
pub struct MemoryGlobalExecutionSettingsRepository {
    settings: Arc<RwLock<GlobalExecutionSettings>>,
}

impl Default for MemoryGlobalExecutionSettingsRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryGlobalExecutionSettingsRepository {
    /// Create a new repository with default settings
    pub fn new() -> Self {
        Self {
            settings: Arc::new(RwLock::new(GlobalExecutionSettings::default())),
        }
    }

    /// Create with specific settings (for tests)
    pub fn with_settings(settings: GlobalExecutionSettings) -> Self {
        Self {
            settings: Arc::new(RwLock::new(settings)),
        }
    }
}

#[async_trait]
impl GlobalExecutionSettingsRepository for MemoryGlobalExecutionSettingsRepository {
    async fn get_settings(&self) -> Result<GlobalExecutionSettings, Box<dyn std::error::Error>> {
        let settings = self.settings.read().await;
        Ok(settings.clone())
    }

    async fn update_settings(
        &self,
        new_settings: &GlobalExecutionSettings,
    ) -> Result<GlobalExecutionSettings, Box<dyn std::error::Error>> {
        // Enforce max limit of 50
        let clamped_max = new_settings.global_max_concurrent.min(GLOBAL_MAX_CONCURRENT_LIMIT);

        let mut settings = self.settings.write().await;
        settings.global_max_concurrent = clamped_max;
        Ok(settings.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_default_global_settings() {
        let repo = MemoryExecutionSettingsRepository::new();

        // Get global defaults (project_id = None)
        let settings = repo.get_settings(None).await.unwrap();
        assert_eq!(settings.max_concurrent_tasks, 2);
        assert!(settings.auto_commit);
        assert!(settings.pause_on_failure);
    }

    #[tokio::test]
    async fn test_update_global_settings() {
        let repo = MemoryExecutionSettingsRepository::new();

        let new_settings = ExecutionSettings {
            max_concurrent_tasks: 4,
            auto_commit: false,
            pause_on_failure: false,
        };

        let updated = repo.update_settings(None, &new_settings).await.unwrap();
        assert_eq!(updated.max_concurrent_tasks, 4);

        // Verify persistence
        let retrieved = repo.get_settings(None).await.unwrap();
        assert_eq!(retrieved.max_concurrent_tasks, 4);
        assert!(!retrieved.auto_commit);
        assert!(!retrieved.pause_on_failure);
    }

    #[tokio::test]
    async fn test_per_project_settings() {
        let repo = MemoryExecutionSettingsRepository::new();
        let project_id = ProjectId::from_string("test-project-123".to_string());

        // Initially, get_settings for a project should return global defaults
        let settings = repo.get_settings(Some(&project_id)).await.unwrap();
        assert_eq!(settings.max_concurrent_tasks, 2); // global default

        // Create project-specific settings
        let project_settings = ExecutionSettings {
            max_concurrent_tasks: 5,
            auto_commit: false,
            pause_on_failure: true,
        };

        repo.update_settings(Some(&project_id), &project_settings)
            .await
            .unwrap();

        // Now get_settings should return project-specific values
        let retrieved = repo.get_settings(Some(&project_id)).await.unwrap();
        assert_eq!(retrieved.max_concurrent_tasks, 5);
        assert!(!retrieved.auto_commit);
        assert!(retrieved.pause_on_failure);

        // Global settings should remain unchanged
        let global = repo.get_settings(None).await.unwrap();
        assert_eq!(global.max_concurrent_tasks, 2);
    }

    #[tokio::test]
    async fn test_with_settings() {
        let initial_settings = ExecutionSettings {
            max_concurrent_tasks: 8,
            auto_commit: true,
            pause_on_failure: false,
        };

        let repo = MemoryExecutionSettingsRepository::with_settings(initial_settings);

        let settings = repo.get_settings(None).await.unwrap();
        assert_eq!(settings.max_concurrent_tasks, 8);
        assert!(settings.auto_commit);
        assert!(!settings.pause_on_failure);
    }

    #[tokio::test]
    async fn test_global_execution_settings() {
        let repo = MemoryGlobalExecutionSettingsRepository::new();

        // Get default global settings
        let settings = repo.get_settings().await.unwrap();
        assert_eq!(settings.global_max_concurrent, 20);

        // Update global settings
        let new_settings = GlobalExecutionSettings {
            global_max_concurrent: 30,
        };
        let updated = repo.update_settings(&new_settings).await.unwrap();
        assert_eq!(updated.global_max_concurrent, 30);

        // Verify persistence
        let retrieved = repo.get_settings().await.unwrap();
        assert_eq!(retrieved.global_max_concurrent, 30);
    }

    #[tokio::test]
    async fn test_global_max_concurrent_capped_at_50() {
        let repo = MemoryGlobalExecutionSettingsRepository::new();

        // Try to set above max
        let new_settings = GlobalExecutionSettings {
            global_max_concurrent: 100,
        };
        let updated = repo.update_settings(&new_settings).await.unwrap();

        // Should be clamped to 50
        assert_eq!(updated.global_max_concurrent, 50);

        // Verify persistence
        let retrieved = repo.get_settings().await.unwrap();
        assert_eq!(retrieved.global_max_concurrent, 50);
    }
}
