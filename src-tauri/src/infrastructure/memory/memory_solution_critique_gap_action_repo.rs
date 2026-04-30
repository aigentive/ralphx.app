use std::sync::RwLock;

use async_trait::async_trait;

use crate::domain::entities::{
    ArtifactId, ContextTargetRef, IdeationSessionId, SolutionCritiqueGapAction,
};
use crate::domain::repositories::SolutionCritiqueGapActionRepository;
use crate::error::AppResult;

pub struct MemorySolutionCritiqueGapActionRepository {
    actions: RwLock<Vec<SolutionCritiqueGapAction>>,
}

impl MemorySolutionCritiqueGapActionRepository {
    pub fn new() -> Self {
        Self {
            actions: RwLock::new(Vec::new()),
        }
    }
}

impl Default for MemorySolutionCritiqueGapActionRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SolutionCritiqueGapActionRepository for MemorySolutionCritiqueGapActionRepository {
    async fn append(&self, action: SolutionCritiqueGapAction) -> AppResult<()> {
        let mut actions = self.actions.write().unwrap();
        actions.push(action);
        Ok(())
    }

    async fn list_for_critique(
        &self,
        critique_artifact_id: &ArtifactId,
    ) -> AppResult<Vec<SolutionCritiqueGapAction>> {
        let mut actions = self
            .actions
            .read()
            .unwrap()
            .iter()
            .filter(|action| action.critique_artifact_id == critique_artifact_id.as_str())
            .cloned()
            .collect::<Vec<_>>();
        actions.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.id.cmp(&left.id))
        });
        Ok(actions)
    }

    async fn list_for_target(
        &self,
        session_id: &IdeationSessionId,
        target: &ContextTargetRef,
    ) -> AppResult<Vec<SolutionCritiqueGapAction>> {
        let mut actions = self
            .actions
            .read()
            .unwrap()
            .iter()
            .filter(|action| {
                action.session_id == session_id.as_str()
                    && action.target_type == target.target_type
                    && action.target_id == target.id
            })
            .cloned()
            .collect::<Vec<_>>();
        actions.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.id.cmp(&left.id))
        });
        Ok(actions)
    }
}
