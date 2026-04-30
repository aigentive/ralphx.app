use async_trait::async_trait;

use crate::domain::entities::{
    ArtifactId, ContextTargetRef, IdeationSessionId, SolutionCritiqueGapAction,
};
use crate::error::AppResult;

#[async_trait]
pub trait SolutionCritiqueGapActionRepository: Send + Sync {
    async fn append(&self, action: SolutionCritiqueGapAction) -> AppResult<()>;

    async fn list_for_critique(
        &self,
        critique_artifact_id: &ArtifactId,
    ) -> AppResult<Vec<SolutionCritiqueGapAction>>;

    async fn list_for_target(
        &self,
        session_id: &IdeationSessionId,
        target: &ContextTargetRef,
    ) -> AppResult<Vec<SolutionCritiqueGapAction>>;
}
