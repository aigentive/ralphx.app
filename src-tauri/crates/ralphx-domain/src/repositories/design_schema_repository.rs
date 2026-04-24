use async_trait::async_trait;

use crate::domain::entities::{DesignSchemaVersion, DesignSchemaVersionId, DesignSystemId};
use crate::error::AppResult;

#[async_trait]
pub trait DesignSchemaRepository: Send + Sync {
    async fn create_version(&self, version: DesignSchemaVersion) -> AppResult<DesignSchemaVersion>;

    async fn get_version(
        &self,
        id: &DesignSchemaVersionId,
    ) -> AppResult<Option<DesignSchemaVersion>>;

    async fn get_current_for_design_system(
        &self,
        design_system_id: &DesignSystemId,
    ) -> AppResult<Option<DesignSchemaVersion>>;

    async fn list_versions(
        &self,
        design_system_id: &DesignSystemId,
    ) -> AppResult<Vec<DesignSchemaVersion>>;
}
