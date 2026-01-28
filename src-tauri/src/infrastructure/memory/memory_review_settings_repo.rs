// Memory-based ReviewSettingsRepository implementation for testing
// Uses RwLock for thread-safe storage without a real database

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::domain::review::ReviewSettings;
use crate::domain::repositories::ReviewSettingsRepository;

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
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_default_settings() {
        let repo = MemoryReviewSettingsRepository::new();

        let settings = repo.get_settings().await.unwrap();
        assert!(settings.ai_review_enabled);
        assert!(settings.ai_review_auto_fix);
        assert!(!settings.require_fix_approval);
        assert!(!settings.require_human_review);
        assert_eq!(settings.max_fix_attempts, 3);
        assert_eq!(settings.max_revision_cycles, 5);
    }

    #[tokio::test]
    async fn test_update_settings() {
        let repo = MemoryReviewSettingsRepository::new();

        let new_settings = ReviewSettings {
            ai_review_enabled: false,
            ai_review_auto_fix: false,
            require_fix_approval: true,
            require_human_review: true,
            max_fix_attempts: 7,
            max_revision_cycles: 10,
        };

        let updated = repo.update_settings(&new_settings).await.unwrap();
        assert!(!updated.ai_review_enabled);
        assert_eq!(updated.max_revision_cycles, 10);

        // Verify persistence
        let retrieved = repo.get_settings().await.unwrap();
        assert!(!retrieved.ai_review_enabled);
        assert!(retrieved.require_fix_approval);
        assert_eq!(retrieved.max_revision_cycles, 10);
    }

    #[tokio::test]
    async fn test_with_settings() {
        let initial_settings = ReviewSettings {
            ai_review_enabled: false,
            ai_review_auto_fix: false,
            require_fix_approval: true,
            require_human_review: true,
            max_fix_attempts: 2,
            max_revision_cycles: 3,
        };

        let repo = MemoryReviewSettingsRepository::with_settings(initial_settings);

        let settings = repo.get_settings().await.unwrap();
        assert!(!settings.ai_review_enabled);
        assert!(settings.require_fix_approval);
        assert_eq!(settings.max_revision_cycles, 3);
    }
}
