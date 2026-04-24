use async_trait::async_trait;

use crate::domain::entities::{
    DesignSchemaVersionId, DesignStyleguideItem, DesignStyleguideItemId, DesignSystemId,
};
use crate::error::AppResult;

#[async_trait]
pub trait DesignStyleguideRepository: Send + Sync {
    async fn replace_items_for_schema_version(
        &self,
        schema_version_id: &DesignSchemaVersionId,
        items: Vec<DesignStyleguideItem>,
    ) -> AppResult<()>;

    async fn list_items(
        &self,
        design_system_id: &DesignSystemId,
        schema_version_id: Option<&DesignSchemaVersionId>,
    ) -> AppResult<Vec<DesignStyleguideItem>>;

    async fn get_item(
        &self,
        design_system_id: &DesignSystemId,
        item_id: &str,
    ) -> AppResult<Option<DesignStyleguideItem>>;

    async fn update_item(&self, item: &DesignStyleguideItem) -> AppResult<()>;

    async fn approve_item(&self, id: &DesignStyleguideItemId) -> AppResult<()>;
}
