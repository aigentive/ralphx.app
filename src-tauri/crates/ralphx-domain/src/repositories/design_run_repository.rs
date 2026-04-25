use async_trait::async_trait;

use crate::domain::entities::{DesignRun, DesignRunId, DesignSystemId};
use crate::error::AppResult;

#[async_trait]
pub trait DesignRunRepository: Send + Sync {
    async fn create(&self, run: DesignRun) -> AppResult<DesignRun>;

    async fn get_by_id(&self, id: &DesignRunId) -> AppResult<Option<DesignRun>>;

    async fn list_by_design_system(
        &self,
        design_system_id: &DesignSystemId,
    ) -> AppResult<Vec<DesignRun>>;

    async fn update(&self, run: &DesignRun) -> AppResult<()>;
}
