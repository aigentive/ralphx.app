use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use tracing::error;

use super::*;
use crate::domain::entities::{
    Artifact, ArtifactBucketId, ArtifactContent, ArtifactId, ArtifactMetadata, ArtifactType,
    IdeationSessionId, TaskProposalId,
};

pub async fn create_plan_artifact(
    State(state): State<HttpServerState>,
    Json(req): Json<CreatePlanArtifactRequest>,
) -> Result<Json<ArtifactResponse>, StatusCode> {
    let session_id = IdeationSessionId::from_string(req.session_id);

    // Verify session exists
    state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|e| {
            error!("Failed to get session {} for plan artifact creation: {}", session_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Create the specification artifact
    let bucket_id = ArtifactBucketId::from_string("prd-library");
    let artifact = Artifact {
        id: ArtifactId::new(),
        artifact_type: ArtifactType::Specification,
        name: req.title.clone(),
        content: ArtifactContent::inline(&req.content),
        metadata: ArtifactMetadata::new("orchestrator").with_version(1),
        derived_from: vec![],
        bucket_id: Some(bucket_id),
    };

    let created = state
        .app_state
        .artifact_repo
        .create(artifact)
        .await
        .map_err(|e| {
            error!("Failed to create plan artifact for session {}: {}", session_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Link artifact to session
    state
        .app_state
        .ideation_session_repo
        .update_plan_artifact_id(&session_id, Some(created.id.to_string()))
        .await
        .map_err(|e| {
            error!("Failed to link artifact {} to session {}: {}", created.id.as_str(), session_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(ArtifactResponse::from(created)))
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
        .map_err(|e| {
            error!("Failed to get artifact {} for update: {}", artifact_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Update content and increment version
    artifact.content = ArtifactContent::Inline { text: req.content };
    artifact.metadata.version += 1;

    // Save updated artifact
    state
        .app_state
        .artifact_repo
        .update(&artifact)
        .await
        .map_err(|e| {
            error!("Failed to update artifact {}: {}", artifact_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

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
        .map_err(|e| {
            error!("Failed to get artifact {}: {}", artifact_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(ArtifactResponse::from(artifact)))
}

pub async fn link_proposals_to_plan(
    State(state): State<HttpServerState>,
    Json(req): Json<LinkProposalsToPlanRequest>,
) -> Result<Json<SuccessResponse>, StatusCode> {
    let artifact_id = ArtifactId::from_string(req.artifact_id);

    // Verify artifact exists
    let artifact = state
        .app_state
        .artifact_repo
        .get_by_id(&artifact_id)
        .await
        .map_err(|e| {
            error!("Failed to get artifact {} for linking proposals: {}", artifact_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Update each proposal
    for proposal_id_str in req.proposal_ids {
        let proposal_id = TaskProposalId::from_string(proposal_id_str);

        let mut proposal = state
            .app_state
            .task_proposal_repo
            .get_by_id(&proposal_id)
            .await
            .map_err(|e| {
                error!("Failed to get proposal {} for linking: {}", proposal_id.as_str(), e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .ok_or(StatusCode::NOT_FOUND)?;

        proposal.plan_artifact_id = Some(artifact_id.clone());
        proposal.plan_version_at_creation = Some(artifact.metadata.version);

        state
            .app_state
            .task_proposal_repo
            .update(&proposal)
            .await
            .map_err(|e| {
                error!("Failed to update proposal {} with plan link: {}", proposal_id.as_str(), e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    }

    Ok(Json(SuccessResponse {
        success: true,
        message: "Proposals linked to plan successfully".to_string(),
    }))
}

pub async fn get_session_plan(
    State(state): State<HttpServerState>,
    Path(session_id): Path<String>,
) -> Result<Json<Option<ArtifactResponse>>, StatusCode> {
    let session_id = IdeationSessionId::from_string(session_id);

    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|e| {
            error!("Failed to get session {} for plan retrieval: {}", session_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    if let Some(artifact_id) = session.plan_artifact_id {
        let artifact = state
            .app_state
            .artifact_repo
            .get_by_id(&artifact_id)
            .await
            .map_err(|e| {
                error!("Failed to get plan artifact {} for session {}: {}", artifact_id.as_str(), session_id.as_str(), e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .ok_or(StatusCode::NOT_FOUND)?;

        Ok(Json(Some(ArtifactResponse::from(artifact))))
    } else {
        Ok(Json(None))
    }
}
