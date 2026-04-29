use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use tracing::error;

use crate::application::solution_critic::{
    CompileContextRequest, CompileContextResult, CompiledContextReadResult,
    CritiqueArtifactRequest, CritiqueArtifactResult, SolutionCritiqueReadResult,
};
use crate::application::SolutionCritiqueService;
use crate::domain::entities::IdeationSessionId;
use crate::error::AppError;
use crate::http_server::project_scope::{ProjectScope, ProjectScopeGuard};
use crate::http_server::types::HttpServerState;

use super::{json_error, JsonError};

/// POST /api/ideation/sessions/:id/compiled-context
pub async fn post_compiled_context(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Path(session_id): Path<String>,
    Json(request): Json<CompileContextRequest>,
) -> Result<Json<CompileContextResult>, JsonError> {
    assert_session_scope(&state, &scope, &session_id).await?;
    let service = SolutionCritiqueService::from_app_state(&state.app_state);
    service
        .compile_context(&session_id, request)
        .await
        .map(Json)
        .map_err(map_solution_critic_error)
}

/// GET /api/ideation/sessions/:id/compiled-context/:artifact_id
pub async fn get_compiled_context_artifact(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Path((session_id, artifact_id)): Path<(String, String)>,
) -> Result<Json<CompiledContextReadResult>, JsonError> {
    assert_session_scope(&state, &scope, &session_id).await?;
    let service = SolutionCritiqueService::from_app_state(&state.app_state);
    service
        .get_compiled_context(&session_id, &artifact_id)
        .await
        .map(Json)
        .map_err(map_solution_critic_error)
}

/// GET /api/ideation/sessions/:id/compiled-context
pub async fn get_latest_compiled_context(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Path(session_id): Path<String>,
) -> Result<Json<Option<CompiledContextReadResult>>, JsonError> {
    assert_session_scope(&state, &scope, &session_id).await?;
    let service = SolutionCritiqueService::from_app_state(&state.app_state);
    service
        .get_latest_compiled_context(&session_id)
        .await
        .map(Json)
        .map_err(map_solution_critic_error)
}

/// POST /api/ideation/sessions/:id/solution-critique
pub async fn post_solution_critique(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Path(session_id): Path<String>,
    Json(request): Json<CritiqueArtifactRequest>,
) -> Result<Json<CritiqueArtifactResult>, JsonError> {
    assert_session_scope(&state, &scope, &session_id).await?;
    let service = SolutionCritiqueService::from_app_state(&state.app_state);
    service
        .critique_artifact(&session_id, request)
        .await
        .map(Json)
        .map_err(map_solution_critic_error)
}

/// GET /api/ideation/sessions/:id/solution-critique
pub async fn get_latest_solution_critique(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Path(session_id): Path<String>,
) -> Result<Json<Option<SolutionCritiqueReadResult>>, JsonError> {
    assert_session_scope(&state, &scope, &session_id).await?;
    let service = SolutionCritiqueService::from_app_state(&state.app_state);
    service
        .get_latest_solution_critique(&session_id)
        .await
        .map(Json)
        .map_err(map_solution_critic_error)
}

/// GET /api/ideation/sessions/:id/solution-critique/:artifact_id
pub async fn get_solution_critique_artifact(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Path((session_id, artifact_id)): Path<(String, String)>,
) -> Result<Json<SolutionCritiqueReadResult>, JsonError> {
    assert_session_scope(&state, &scope, &session_id).await?;
    let service = SolutionCritiqueService::from_app_state(&state.app_state);
    service
        .get_solution_critique(&session_id, &artifact_id)
        .await
        .map(Json)
        .map_err(map_solution_critic_error)
}

async fn assert_session_scope(
    state: &HttpServerState,
    scope: &ProjectScope,
    session_id: &str,
) -> Result<(), JsonError> {
    if scope.is_unrestricted() {
        return Ok(());
    }

    let session_id = IdeationSessionId::from_string(session_id);
    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|error| {
            error!(
                "Failed to load session {} for solution critic scope check: {}",
                session_id.as_str(),
                error
            );
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get session")
        })?
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "Session not found"))?;

    session
        .assert_project_scope(scope)
        .map_err(|_| json_error(StatusCode::FORBIDDEN, "Forbidden"))
}

fn map_solution_critic_error(error: AppError) -> JsonError {
    let status = match &error {
        AppError::Validation(_) => StatusCode::BAD_REQUEST,
        AppError::NotFound(_) | AppError::ProjectNotFound(_) | AppError::TaskNotFound(_) => {
            StatusCode::NOT_FOUND
        }
        AppError::Conflict(_)
        | AppError::ExecutionBlocked(_)
        | AppError::BranchFreshnessConflict
        | AppError::ReviewWorktreeMissing
        | AppError::DuplicatePr => StatusCode::CONFLICT,
        _ => {
            error!("Solution critic endpoint failed: {}", error);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    };
    let message = if status == StatusCode::INTERNAL_SERVER_ERROR {
        "Solution critic request failed".to_string()
    } else {
        error.to_string()
    };
    json_error(status, message)
}
