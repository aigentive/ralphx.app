use std::sync::Arc;

use async_trait::async_trait;
use serde::Serialize;

use crate::application::AppState;
use crate::domain::entities::{ContextTargetType, TaskId};
use crate::domain::repositories::TaskRepository;
use crate::domain::state_machine::services::{
    ReviewCritiquePreparation, ReviewCritiquePreparationResult, ReviewCritiquePreparer,
};
use crate::error::AppResult;

use super::service::SolutionCritiqueService;
use super::types::{CompileContextRequest, CritiqueArtifactRequest};

pub struct SolutionCritiqueReviewPreparer {
    task_repo: Arc<dyn TaskRepository>,
    service: SolutionCritiqueService,
}

impl SolutionCritiqueReviewPreparer {
    pub fn from_app_state(app_state: &AppState) -> Self {
        Self {
            task_repo: Arc::clone(&app_state.task_repo),
            service: SolutionCritiqueService::from_app_state(app_state),
        }
    }
}

#[async_trait]
impl ReviewCritiquePreparer for SolutionCritiqueReviewPreparer {
    async fn prepare_task_execution_critique(
        &self,
        task_id: &str,
        project_id: &str,
    ) -> ReviewCritiquePreparationResult {
        match self.prepare(task_id, project_id).await {
            Ok(result) => result,
            Err(error) => ReviewCritiquePreparationResult::Error(error.to_string()),
        }
    }
}

impl SolutionCritiqueReviewPreparer {
    async fn prepare(
        &self,
        task_id: &str,
        project_id: &str,
    ) -> AppResult<ReviewCritiquePreparationResult> {
        let task = self
            .task_repo
            .get_by_id(&TaskId::from_string(task_id.to_string()))
            .await?
            .ok_or_else(|| crate::error::AppError::TaskNotFound(task_id.to_string()))?;

        if task.project_id.as_str() != project_id {
            return Ok(ReviewCritiquePreparationResult::Skipped {
                reason: "task belongs to a different project".to_string(),
            });
        }

        let Some(session_id) = task.ideation_session_id.as_ref() else {
            return Ok(ReviewCritiquePreparationResult::Skipped {
                reason: "task has no ideation session for solution critique scope".to_string(),
            });
        };

        let context = self
            .service
            .compile_context(
                session_id.as_str(),
                CompileContextRequest::for_target(ContextTargetType::TaskExecution, task_id),
            )
            .await?;
        let critique = self
            .service
            .critique_artifact(
                session_id.as_str(),
                CritiqueArtifactRequest::for_target(
                    ContextTargetType::TaskExecution,
                    task_id,
                    context.artifact_id.as_str(),
                ),
            )
            .await?;

        Ok(ReviewCritiquePreparationResult::Prepared(
            ReviewCritiquePreparation {
                compiled_context_artifact_id: context.artifact_id,
                critique_artifact_id: critique.artifact_id,
                projected_gap_count: critique.projected_gaps.len(),
                verdict: Some(serialized_enum(&critique.solution_critique.verdict)),
                safe_next_action: critique.solution_critique.safe_next_action,
            },
        ))
    }
}

fn serialized_enum<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}
