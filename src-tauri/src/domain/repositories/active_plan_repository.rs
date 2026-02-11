use crate::domain::entities::{IdeationSessionId, ProjectId};
use async_trait::async_trait;

#[async_trait]
pub trait ActivePlanRepository: Send + Sync {
    /// Get the active plan (ideation session ID) for a project
    async fn get(
        &self,
        project_id: &ProjectId,
    ) -> Result<Option<IdeationSessionId>, Box<dyn std::error::Error>>;

    /// Set the active plan for a project
    /// Validates that the session exists, belongs to the project, and is accepted
    async fn set(
        &self,
        project_id: &ProjectId,
        ideation_session_id: &IdeationSessionId,
    ) -> Result<(), Box<dyn std::error::Error>>;

    /// Clear the active plan for a project
    async fn clear(&self, project_id: &ProjectId) -> Result<(), Box<dyn std::error::Error>>;

    /// Check if a project has an active plan set
    async fn exists(&self, project_id: &ProjectId) -> Result<bool, Box<dyn std::error::Error>>;

    /// Record selection stats (increment count, update timestamp and source)
    async fn record_selection(
        &self,
        project_id: &ProjectId,
        ideation_session_id: &IdeationSessionId,
        source: &str,
    ) -> Result<(), Box<dyn std::error::Error>>;
}
