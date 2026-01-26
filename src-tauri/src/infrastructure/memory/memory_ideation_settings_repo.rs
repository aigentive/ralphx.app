// Memory-based IdeationSettingsRepository implementation for testing
// Uses RwLock for thread-safe storage without a real database

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::domain::ideation::IdeationSettings;
use crate::domain::repositories::IdeationSettingsRepository;

/// In-memory implementation of IdeationSettingsRepository for testing
/// Uses RwLock for thread-safe storage
pub struct MemoryIdeationSettingsRepository {
    settings: Arc<RwLock<IdeationSettings>>,
}

impl Default for MemoryIdeationSettingsRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryIdeationSettingsRepository {
    /// Create a new empty in-memory ideation settings repository
    pub fn new() -> Self {
        Self {
            settings: Arc::new(RwLock::new(IdeationSettings::default())),
        }
    }

    /// Create with specific settings (for tests)
    pub fn with_settings(settings: IdeationSettings) -> Self {
        Self {
            settings: Arc::new(RwLock::new(settings)),
        }
    }
}

#[async_trait]
impl IdeationSettingsRepository for MemoryIdeationSettingsRepository {
    async fn get_settings(&self) -> Result<IdeationSettings, Box<dyn std::error::Error>> {
        let settings = self.settings.read().await;
        Ok(settings.clone())
    }

    async fn update_settings(
        &self,
        new_settings: &IdeationSettings,
    ) -> Result<IdeationSettings, Box<dyn std::error::Error>> {
        let mut settings = self.settings.write().await;
        *settings = new_settings.clone();
        Ok(settings.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ideation::IdeationPlanMode;

    #[tokio::test]
    async fn test_get_default_settings() {
        let repo = MemoryIdeationSettingsRepository::new();

        let settings = repo.get_settings().await.unwrap();
        assert_eq!(settings.plan_mode, IdeationPlanMode::Optional);
        assert_eq!(settings.require_plan_approval, false);
        assert_eq!(settings.suggest_plans_for_complex, true);
        assert_eq!(settings.auto_link_proposals, true);
    }

    #[tokio::test]
    async fn test_update_settings() {
        let repo = MemoryIdeationSettingsRepository::new();

        let new_settings = IdeationSettings {
            plan_mode: IdeationPlanMode::Required,
            require_plan_approval: true,
            suggest_plans_for_complex: false,
            auto_link_proposals: false,
        };

        let updated = repo.update_settings(&new_settings).await.unwrap();
        assert_eq!(updated.plan_mode, IdeationPlanMode::Required);

        // Verify persistence
        let retrieved = repo.get_settings().await.unwrap();
        assert_eq!(retrieved.plan_mode, IdeationPlanMode::Required);
        assert_eq!(retrieved.require_plan_approval, true);
    }

    #[tokio::test]
    async fn test_with_settings() {
        let initial_settings = IdeationSettings {
            plan_mode: IdeationPlanMode::Parallel,
            require_plan_approval: true,
            suggest_plans_for_complex: false,
            auto_link_proposals: false,
        };

        let repo = MemoryIdeationSettingsRepository::with_settings(initial_settings);

        let settings = repo.get_settings().await.unwrap();
        assert_eq!(settings.plan_mode, IdeationPlanMode::Parallel);
        assert_eq!(settings.require_plan_approval, true);
    }
}
