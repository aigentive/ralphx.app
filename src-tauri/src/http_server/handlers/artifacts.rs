use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use super::*;
use crate::domain::entities::{
    Artifact, ArtifactContent, ArtifactId, ArtifactType, IdeationSessionId, TaskProposalId,
};

pub async fn create_plan_artifact(
    State(state): State<HttpServerState>,
    Json(req): Json<CreatePlanArtifactRequest>,
) -> Result<Json<ArtifactResponse>, StatusCode> {
    let session_id = IdeationSessionId::from_string(req.session_id);

    // Create a plan artifact
    let artifact = Artifact::new_inline(
        req.title.clone(),
        ArtifactType::Plan,
        req.content.clone(),
    );

    // Store artifact
    state
        .app_state
        .artifact_repo
        .create(artifact.clone())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Link artifact to session
    let _session_link = state
        .app_state
        .artifact_session_link_repo
        .link_artifact_to_session(session_id.clone(), artifact.id.clone())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ArtifactResponse::from(artifact)))
}

pub async fn update_plan_artifact(
    State(state): State<HttpServerState>,
    Json(req): Json<UpdatePlanArtifactRequest>,
) -> Result<Json<ArtifactResponse>, StatusCode> {
    let artifact_id = ArtifactId::from_string(req.artifact_id);

    // Get existing artifact
    let mut artifact = state
        .app_state
        .artifact_repo
        .get_by_id(&artifact_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Update content
    artifact.content = ArtifactContent::Inline { text: req.content };
    artifact.metadata.bump_version();

    // Save updated artifact
    state
        .app_state
        .artifact_repo
        .update(artifact.clone())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ArtifactResponse::from(artifact)))
}

pub async fn get_plan_artifact(
    State(state): State<HttpServerState>,
    Path(artifact_id): Path<String>,
) -> Result<Json<ArtifactResponse>, StatusCode> {
    let artifact_id = ArtifactId::from_string(artifact_id);

    let artifact = state
        .app_state
        .artifact_repo
        .get_by_id(&artifact_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(ArtifactResponse::from(artifact)))
}

pub async fn link_proposals_to_plan(
    State(state): State<HttpServerState>,
    Json(req): Json<LinkProposalsToPlanRequest>,
) -> Result<StatusCode, StatusCode> {
    let artifact_id = ArtifactId::from_string(req.artifact_id);
    let proposal_ids: Vec<TaskProposalId> = req
        .proposal_ids
        .into_iter()
        .map(TaskProposalId::from_string)
        .collect();

    // Link each proposal to the artifact
    for proposal_id in proposal_ids {
        state
            .app_state
            .artifact_proposal_link_repo
            .link_artifact_to_proposal(artifact_id.clone(), proposal_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_session_plan(
    State(state): State<HttpServerState>,
    Path(session_id): Path<String>,
) -> Result<Json<ArtifactResponse>, StatusCode> {
    let session_id = IdeationSessionId::from_string(session_id);

    // Get session artifacts
    let artifact_ids = state
        .app_state
        .artifact_session_link_repo
        .get_artifacts_for_session(&session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Find the plan artifact
    for artifact_id in artifact_ids {
        let artifact = state
            .app_state
            .artifact_repo
            .get_by_id(&artifact_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if let Some(artifact) = artifact {
            if artifact.artifact_type == ArtifactType::Plan {
                return Ok(Json(ArtifactResponse::from(artifact)));
            }
        }
    }

    Err(StatusCode::NOT_FOUND)
}
