use crate::domain::execution::ExecutionSettings;
use async_trait::async_trait;

#[async_trait]
pub trait ExecutionSettingsRepository: Send + Sync {
    /// Get execution settings (returns default if no settings exist)
    async fn get_settings(&self) -> Result<ExecutionSettings, Box<dyn std::error::Error>>;

    /// Update execution settings
    async fn update_settings(
        &self,
        settings: &ExecutionSettings,
    ) -> Result<ExecutionSettings, Box<dyn std::error::Error>>;
}
