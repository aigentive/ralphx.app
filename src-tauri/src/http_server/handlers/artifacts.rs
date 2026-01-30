use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use tauri::Emitter;
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

    // Get session and check for existing plan
    let session = state
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

    // If session has an existing plan, chain to it via previous_version_id
    let created = if let Some(existing_plan_id) = &session.plan_artifact_id {
        let existing_artifact_id = existing_plan_id.clone();
        state
            .app_state
            .artifact_repo
            .create_with_previous_version(artifact, existing_artifact_id)
            .await
            .map_err(|e| {
                error!("Failed to create plan artifact with version chain for session {}: {}", session_id.as_str(), e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
    } else {
        state
            .app_state
            .artifact_repo
            .create(artifact)
            .await
            .map_err(|e| {
                error!("Failed to create plan artifact for session {}: {}", session_id.as_str(), e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
    };

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

    // Emit event for real-time UI update
    if let Some(app_handle) = &state.app_state.app_handle {
        let content_text = match &created.content {
            ArtifactContent::Inline { text } => text.clone(),
            ArtifactContent::File { path } => format!("[File: {}]", path),
        };
        let _ = app_handle.emit(
            "plan_artifact:created",
            serde_json::json!({
                "sessionId": session_id.as_str(),
                "artifact": {
                    "id": created.id.as_str(),
                    "name": created.name,
                    "content": content_text,
                    "version": created.metadata.version,
                }
            }),
        );
    }

    Ok(Json(ArtifactResponse::from(created)))
}

pub async fn update_plan_artifact(
    State(state): State<HttpServerState>,
    Json(req): Json<UpdatePlanArtifactRequest>,
) -> Result<Json<ArtifactResponse>, StatusCode> {
    let old_artifact_id = ArtifactId::from_string(req.artifact_id);

    // Get existing artifact
    let old_artifact = state
        .app_state
        .artifact_repo
        .get_by_id(&old_artifact_id)
        .await
        .map_err(|e| {
            error!("Failed to get artifact {} for update: {}", old_artifact_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Create NEW artifact with incremented version (version chain, not in-place update)
    let new_artifact = Artifact {
        id: ArtifactId::new(),
        artifact_type: old_artifact.artifact_type.clone(),
        name: old_artifact.name.clone(),
        content: ArtifactContent::Inline { text: req.content },
        metadata: ArtifactMetadata::new(&old_artifact.metadata.created_by)
            .with_version(old_artifact.metadata.version + 1),
        derived_from: vec![],
        bucket_id: old_artifact.bucket_id.clone(),
    };

    // Create the new artifact with previous_version_id link
    let created = state
        .app_state
        .artifact_repo
        .create_with_previous_version(new_artifact, old_artifact_id.clone())
        .await
        .map_err(|e| {
            error!("Failed to create new version of artifact {}: {}", old_artifact_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Find all sessions that link to the old artifact and update them to point to the new one
    let sessions = state
        .app_state
        .ideation_session_repo
        .get_by_plan_artifact_id(old_artifact_id.as_str())
        .await
        .map_err(|e| {
            error!("Failed to find sessions for artifact {}: {}", old_artifact_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    for session in &sessions {
        state
            .app_state
            .ideation_session_repo
            .update_plan_artifact_id(&session.id, Some(created.id.to_string()))
            .await
            .map_err(|e| {
                error!("Failed to update session {} plan artifact link: {}", session.id.as_str(), e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    }

    // Get proposals linked to the old artifact for the sync notification
    let linked_proposals = state
        .app_state
        .task_proposal_repo
        .get_by_plan_artifact_id(&old_artifact_id)
        .await
        .map_err(|e| {
            error!("Failed to get proposals linked to artifact {}: {}", old_artifact_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Emit event for real-time UI update
    if let Some(app_handle) = &state.app_state.app_handle {
        let content_text = match &created.content {
            ArtifactContent::Inline { text } => text.clone(),
            ArtifactContent::File { path } => format!("[File: {}]", path),
        };

        // Emit plan_artifact:updated event with the NEW artifact info
        let _ = app_handle.emit(
            "plan_artifact:updated",
            serde_json::json!({
                "artifactId": created.id.as_str(),
                "previousArtifactId": old_artifact_id.as_str(),
                "artifact": {
                    "id": created.id.as_str(),
                    "name": created.name,
                    "content": content_text,
                    "version": created.metadata.version,
                }
            }),
        );

        // If there are linked proposals, emit sync notification
        if !linked_proposals.is_empty() {
            let payload = PlanProposalsSyncPayload {
                artifact_id: created.id.to_string(),
                previous_artifact_id: old_artifact_id.to_string(),
                proposal_ids: linked_proposals.iter().map(|p| p.id.to_string()).collect(),
                new_version: created.metadata.version,
            };
            let _ = app_handle.emit("plan:proposals_may_need_update", payload);
        }
    }

    Ok(Json(ArtifactResponse::from(created)))
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

/// Get version history for a plan artifact
/// Returns list of version summaries from newest to oldest
pub async fn get_plan_artifact_history(
    State(state): State<HttpServerState>,
    Path(artifact_id): Path<String>,
) -> Result<Json<Vec<ArtifactVersionSummaryResponse>>, StatusCode> {
    let artifact_id = ArtifactId::from_string(artifact_id);

    // Verify artifact exists
    state
        .app_state
        .artifact_repo
        .get_by_id(&artifact_id)
        .await
        .map_err(|e| {
            error!("Failed to get artifact {} for history: {}", artifact_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Get the version history
    let history = state
        .app_state
        .artifact_repo
        .get_version_history(&artifact_id)
        .await
        .map_err(|e| {
            error!("Failed to get history for artifact {}: {}", artifact_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(history.into_iter().map(ArtifactVersionSummaryResponse::from).collect()))
}
