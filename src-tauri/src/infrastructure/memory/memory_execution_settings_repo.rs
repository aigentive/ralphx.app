// Memory-based ExecutionSettingsRepository implementation for testing
// Uses RwLock for thread-safe storage without a real database

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::domain::execution::ExecutionSettings;
use crate::domain::repositories::ExecutionSettingsRepository;

/// In-memory implementation of ExecutionSettingsRepository for testing
/// Uses RwLock for thread-safe storage
pub struct MemoryExecutionSettingsRepository {
    settings: Arc<RwLock<ExecutionSettings>>,
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
            settings: Arc::new(RwLock::new(ExecutionSettings::default())),
        }
    }

    /// Create with specific settings (for tests)
    pub fn with_settings(settings: ExecutionSettings) -> Self {
        Self {
            settings: Arc::new(RwLock::new(settings)),
        }
    }
}

#[async_trait]
impl ExecutionSettingsRepository for MemoryExecutionSettingsRepository {
    async fn get_settings(&self) -> Result<ExecutionSettings, Box<dyn std::error::Error>> {
        let settings = self.settings.read().await;
        Ok(settings.clone())
    }

    async fn update_settings(
        &self,
        new_settings: &ExecutionSettings,
    ) -> Result<ExecutionSettings, Box<dyn std::error::Error>> {
        let mut settings = self.settings.write().await;
        *settings = new_settings.clone();
        Ok(settings.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_default_settings() {
        let repo = MemoryExecutionSettingsRepository::new();

        let settings = repo.get_settings().await.unwrap();
        assert_eq!(settings.max_concurrent_tasks, 2);
        assert!(settings.auto_commit);
        assert!(settings.pause_on_failure);
    }

    #[tokio::test]
    async fn test_update_settings() {
        let repo = MemoryExecutionSettingsRepository::new();

        let new_settings = ExecutionSettings {
            max_concurrent_tasks: 4,
            auto_commit: false,
            pause_on_failure: false,
        };

        let updated = repo.update_settings(&new_settings).await.unwrap();
        assert_eq!(updated.max_concurrent_tasks, 4);

        // Verify persistence
        let retrieved = repo.get_settings().await.unwrap();
        assert_eq!(retrieved.max_concurrent_tasks, 4);
        assert!(!retrieved.auto_commit);
        assert!(!retrieved.pause_on_failure);
    }

    #[tokio::test]
    async fn test_with_settings() {
        let initial_settings = ExecutionSettings {
            max_concurrent_tasks: 8,
            auto_commit: true,
            pause_on_failure: false,
        };

        let repo = MemoryExecutionSettingsRepository::with_settings(initial_settings);

        let settings = repo.get_settings().await.unwrap();
        assert_eq!(settings.max_concurrent_tasks, 8);
        assert!(settings.auto_commit);
        assert!(!settings.pause_on_failure);
    }
}
