use async_trait::async_trait;

use crate::domain::entities::{DesignSystem, DesignSystemId, ProjectId};
use crate::error::AppResult;

#[async_trait]
pub trait DesignSystemRepository: Send + Sync {
    async fn create(&self, system: DesignSystem) -> AppResult<DesignSystem>;

    async fn get_by_id(&self, id: &DesignSystemId) -> AppResult<Option<DesignSystem>>;

    async fn list_by_project(
        &self,
        project_id: &ProjectId,
        include_archived: bool,
    ) -> AppResult<Vec<DesignSystem>>;

    async fn update(&self, system: &DesignSystem) -> AppResult<()>;

    async fn archive(&self, id: &DesignSystemId) -> AppResult<()>;
}
