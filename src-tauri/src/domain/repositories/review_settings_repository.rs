use crate::domain::review::ReviewSettings;
use async_trait::async_trait;

#[async_trait]
pub trait ReviewSettingsRepository: Send + Sync {
    /// Get review settings (returns default if no settings exist)
    async fn get_settings(&self) -> Result<ReviewSettings, Box<dyn std::error::Error>>;

    /// Update review settings
    async fn update_settings(
        &self,
        settings: &ReviewSettings,
    ) -> Result<ReviewSettings, Box<dyn std::error::Error>>;
}
