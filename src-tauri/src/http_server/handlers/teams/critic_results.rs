use super::*;

use crate::domain::entities::verification_critic_result::CriticKind;
use crate::domain::repositories::verification_critic_result_repo::SubmitCriticResultInput;
use crate::http_server::types::{
    GetRoundResultsQuery, GetRoundResultsResponse, RoundResultEntry, SubmitCriticResultRequest,
    SubmitCriticResultResponse,
};
use axum::extract::Query;
use crate::domain::entities::{ArtifactContent, ArtifactId, IdeationSessionId};
use crate::error::AppError;
use std::collections::HashMap;

// === Verification Critic Results ===

/// POST /internal/verification-critic-results
///
/// Atomically submits a critic result: creates an artifact and inserts a
/// `verification_critic_results` row linked to (parent_session_id, generation, round, critic_kind).
/// Generation is resolved server-side from `IdeationSession.verification_generation`.
pub async fn submit_critic_result(
    State(state): State<HttpServerState>,
    Json(req): Json<SubmitCriticResultRequest>,
) -> Result<Json<SubmitCriticResultResponse>, (StatusCode, String)> {
    // Validate required fields
    if req.parent_session_id.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "parent_session_id is required".to_string()));
    }

    // Parse critic_kind from string
    let critic_kind = CriticKind::from_db_str(&req.critic_kind).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            format!(
                "Unknown critic_kind: '{}'. Valid values: completeness, feasibility, ux, \
                 code_quality, intent, prompt_quality, pipeline_safety, state_machine",
                req.critic_kind
            ),
        )
    })?;

    // Resolve verification_generation from the parent session
    let session_id_obj = IdeationSessionId::from_string(req.parent_session_id.clone());
    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id_obj)
        .await
        .map_err(|e| {
            error!("Failed to look up session {}: {}", req.parent_session_id, e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
        })?
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, "Session not found".to_string())
        })?;

    let verification_generation = session.verification_generation;

    let input = SubmitCriticResultInput {
        parent_session_id: req.parent_session_id.clone(),
        verification_session_id: req.verification_session_id.clone(),
        verification_generation,
        round: req.round,
        critic_kind: critic_kind.clone(),
        title: req.title.clone(),
        content: req.content.clone(),
        artifact_type: req.artifact_type.clone(),
    };

    match state.app_state.verification_critic_result_repo.submit(input).await {
        Ok(output) => {
            info!(
                artifact_id = %output.artifact_id,
                result_id = %output.result_id,
                parent_session_id = %req.parent_session_id,
                generation = verification_generation,
                round = req.round,
                critic_kind = %req.critic_kind,
                "Verification critic result submitted"
            );
            Ok(Json(SubmitCriticResultResponse {
                artifact_id: output.artifact_id,
                result_id: output.result_id,
            }))
        }
        Err(AppError::Conflict(_)) => {
            // UNIQUE constraint: return the original artifact_id by querying existing results
            let original_artifact_id = state
                .app_state
                .verification_critic_result_repo
                .get_round_results(&req.parent_session_id, verification_generation, req.round)
                .await
                .ok()
                .and_then(|results| {
                    results
                        .into_iter()
                        .find(|r| r.critic_kind == critic_kind)
                        .map(|r| r.artifact_id)
                })
                .unwrap_or_default();

            Err((
                StatusCode::CONFLICT,
                serde_json::json!({
                    "error": "Duplicate submission for (parent_session_id, generation, round, critic_kind)",
                    "artifact_id": original_artifact_id,
                })
                .to_string(),
            ))
        }
        Err(e) => {
            error!("Failed to submit critic result: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()))
        }
    }
}

/// GET /internal/verification-critic-results?parent_session_id=&generation=&round=
///
/// Returns all critic results for a (parent_session_id, generation, round) triple,
/// keyed by critic_kind string.
pub async fn get_round_results(
    State(state): State<HttpServerState>,
    Query(params): Query<GetRoundResultsQuery>,
) -> Result<Json<GetRoundResultsResponse>, (StatusCode, String)> {
    let results = state
        .app_state
        .verification_critic_result_repo
        .get_round_results(&params.parent_session_id, params.generation, params.round)
        .await
        .map_err(|e| {
            error!("Failed to get round results: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
        })?;

    let mut results_by_critic_kind: HashMap<String, RoundResultEntry> = HashMap::new();

    for result in results {
        let artifact_id = ArtifactId::from_string(result.artifact_id.clone());
        let (title, content) = match state.app_state.artifact_repo.get_by_id(&artifact_id).await {
            Ok(Some(artifact)) => {
                let content = match artifact.content {
                    ArtifactContent::Inline { text } => text,
                    ArtifactContent::File { path } => format!("[File: {}]", path),
                };
                (artifact.name, content)
            }
            Ok(None) => {
                warn!(artifact_id = %result.artifact_id, "Artifact not found for critic result");
                (String::new(), String::new())
            }
            Err(e) => {
                error!("Failed to fetch artifact {}: {}", result.artifact_id, e);
                (String::new(), String::new())
            }
        };

        results_by_critic_kind.insert(
            result.critic_kind.as_str().to_string(),
            RoundResultEntry {
                artifact_id: result.artifact_id,
                title,
                status: result.status,
                content,
                created_at: result.created_at,
            },
        );
    }

    Ok(Json(GetRoundResultsResponse { results_by_critic_kind }))
}
