use async_trait::async_trait;

use crate::domain::entities::{DesignSystemId, DesignSystemSource};
use crate::error::AppResult;

#[async_trait]
pub trait DesignSystemSourceRepository: Send + Sync {
    async fn replace_for_design_system(
        &self,
        design_system_id: &DesignSystemId,
        sources: Vec<DesignSystemSource>,
    ) -> AppResult<()>;

    async fn list_by_design_system(
        &self,
        design_system_id: &DesignSystemId,
    ) -> AppResult<Vec<DesignSystemSource>>;
}
