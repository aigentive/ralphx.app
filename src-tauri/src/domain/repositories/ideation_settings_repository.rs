use crate::domain::ideation::IdeationSettings;
use async_trait::async_trait;

#[async_trait]
pub trait IdeationSettingsRepository: Send + Sync {
    /// Get ideation settings (returns default if no settings exist)
    async fn get_settings(&self) -> Result<IdeationSettings, Box<dyn std::error::Error>>;

    /// Update ideation settings
    async fn update_settings(
        &self,
        settings: &IdeationSettings,
    ) -> Result<IdeationSettings, Box<dyn std::error::Error>>;
}
