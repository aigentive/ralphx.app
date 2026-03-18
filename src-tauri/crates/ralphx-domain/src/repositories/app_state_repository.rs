use crate::domain::entities::app_state::AppSettings;
use crate::domain::entities::ProjectId;
use async_trait::async_trait;

#[async_trait]
pub trait AppStateRepository: Send + Sync {
    async fn get(&self) -> Result<AppSettings, Box<dyn std::error::Error>>;
    async fn set_active_project(
        &self,
        project_id: Option<&ProjectId>,
    ) -> Result<(), Box<dyn std::error::Error>>;
}
