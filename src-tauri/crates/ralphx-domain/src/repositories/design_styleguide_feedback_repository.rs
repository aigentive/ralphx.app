use async_trait::async_trait;

use crate::domain::entities::{
    DesignStyleguideFeedback, DesignStyleguideFeedbackId, DesignSystemId,
};
use crate::error::AppResult;

#[async_trait]
pub trait DesignStyleguideFeedbackRepository: Send + Sync {
    async fn create(
        &self,
        feedback: DesignStyleguideFeedback,
    ) -> AppResult<DesignStyleguideFeedback>;

    async fn get_by_id(
        &self,
        id: &DesignStyleguideFeedbackId,
    ) -> AppResult<Option<DesignStyleguideFeedback>>;

    async fn list_open_by_design_system(
        &self,
        design_system_id: &DesignSystemId,
    ) -> AppResult<Vec<DesignStyleguideFeedback>>;

    async fn update(&self, feedback: &DesignStyleguideFeedback) -> AppResult<()>;
}
