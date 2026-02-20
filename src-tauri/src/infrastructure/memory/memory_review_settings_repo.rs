// Memory-based ReviewSettingsRepository implementation for testing
// Uses RwLock for thread-safe storage without a real database

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::domain::repositories::ReviewSettingsRepository;
use crate::domain::review::ReviewSettings;

/// In-memory implementation of ReviewSettingsRepository for testing
/// Uses RwLock for thread-safe storage
pub struct MemoryReviewSettingsRepository {
    settings: Arc<RwLock<ReviewSettings>>,
}

impl Default for MemoryReviewSettingsRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryReviewSettingsRepository {
    /// Create a new empty in-memory review settings repository
    pub fn new() -> Self {
        Self {
            settings: Arc::new(RwLock::new(ReviewSettings::default())),
        }
    }

    /// Create with specific settings (for tests)
    pub fn with_settings(settings: ReviewSettings) -> Self {
        Self {
            settings: Arc::new(RwLock::new(settings)),
        }
    }
}

#[async_trait]
impl ReviewSettingsRepository for MemoryReviewSettingsRepository {
    async fn get_settings(&self) -> Result<ReviewSettings, Box<dyn std::error::Error>> {
        let settings = self.settings.read().await;
        Ok(settings.clone())
    }

    async fn update_settings(
        &self,
        new_settings: &ReviewSettings,
    ) -> Result<ReviewSettings, Box<dyn std::error::Error>> {
        let mut settings = self.settings.write().await;
        *settings = new_settings.clone();
        Ok(settings.clone())
    }
}

#[cfg(test)]
#[path = "memory_review_settings_repo_tests.rs"]
mod tests;
