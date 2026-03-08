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
    IdeationSessionId,
};
use crate::error::AppError;
use crate::infrastructure::sqlite::{
    SqliteArtifactRepository as ArtifactRepo, SqliteIdeationSessionRepository as SessionRepo,
    SqliteTaskProposalRepository as ProposalRepo,
};

/// Map an AppError to an HttpError for handler responses.
fn map_app_err(e: AppError) -> HttpError {
    match e {
        AppError::Validation(msg) => HttpError::validation(msg),
        AppError::NotFound(_) => StatusCode::NOT_FOUND.into(),
        _ => StatusCode::INTERNAL_SERVER_ERROR.into(),
    }
}

pub async fn create_plan_artifact(
    State(state): State<HttpServerState>,
    Json(req): Json<CreatePlanArtifactRequest>,
) -> Result<Json<ArtifactResponse>, HttpError> {
    let session_id_str = req.session_id.clone();
    let title = req.title.clone();
    let content = req.content.clone();

    // Single lock acquisition: all DB work in one transaction.
    // Events emitted after db.run_transaction() returns (acceptable crash-consistency gap).
    let (session_id, created) = state
        .app_state
        .db
        .run_transaction(move |conn| {
            let sid = IdeationSessionId::from_string(session_id_str);

            // Get session and check for existing plan
            let session = SessionRepo::get_by_id_sync(conn, sid.as_str())?
                .ok_or_else(|| AppError::NotFound(format!("Session {} not found", sid)))?;

            // Guard: reject mutations on Archived/Accepted sessions
            crate::http_server::helpers::assert_session_mutable(&session)?;

            // Create the specification artifact
            let bucket_id = ArtifactBucketId::from_string("prd-library");
            let artifact = Artifact {
                id: ArtifactId::new(),
                artifact_type: ArtifactType::Specification,
                name: title,
                content: ArtifactContent::inline(&content),
                metadata: ArtifactMetadata::new("orchestrator").with_version(1),
                derived_from: vec![],
                bucket_id: Some(bucket_id),
            };

            // Chain only to the session's OWN plan (plan_artifact_id), never to an inherited one.
            // Child sessions with inherit_context=true have plan_artifact_id=None and
            // inherited_plan_artifact_id=Some(parent_id). The else branch creates a fresh,
            // independent artifact for them — not chained to the parent's plan.
            let created = if let Some(existing_plan_id) = &session.plan_artifact_id {
                let prev_id = existing_plan_id.as_str().to_string();
                ArtifactRepo::create_with_previous_version_sync(conn, artifact, &prev_id)?
            } else {
                ArtifactRepo::create_sync(conn, artifact)?
            };

            // Link artifact to session
            SessionRepo::update_plan_artifact_id_sync(
                conn,
                sid.as_str(),
                Some(created.id.as_str()),
            )?;

            Ok((sid, created))
        })
        .await
        .map_err(|e| {
            error!("create_plan_artifact transaction failed: {}", e);
            map_app_err(e)
        })?;

    // Emit event for real-time UI update (outside lock — acceptable crash gap)
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
) -> Result<Json<ArtifactResponse>, HttpError> {
    let input_artifact_id = req.artifact_id.clone();
    let content = req.content;

    // Single lock acquisition: all DB work in one transaction.
    // Events emitted after db.run_transaction() returns (acceptable crash-consistency gap).
    let (created, old_artifact_id_str, sessions, linked_proposal_ids, verification_reset) = state
        .app_state
        .db
        .run_transaction(move |conn| {
            // 1. Resolve stale IDs: walk the version chain forward to find the latest version.
            //    Makes the endpoint idempotent — agents can pass any version ID and it works.
            let old_id = ArtifactRepo::resolve_latest_sync(conn, &input_artifact_id)?;

            // 2. Get existing artifact (using resolved ID)
            let old_artifact = ArtifactRepo::get_by_id_sync(conn, &old_id)?
                .ok_or_else(|| AppError::NotFound(format!("Artifact {} not found", old_id)))?;

            // 3. Guard: reject mutations on Archived/Accepted sessions
            let owning_sessions = SessionRepo::get_by_plan_artifact_id_sync(conn, &old_id)?;
            if let Some(session) = owning_sessions.first() {
                crate::http_server::helpers::assert_session_mutable(session)?;
            }

            // 4. Guard: reject update if this artifact is only referenced as an inherited plan
            if owning_sessions.is_empty() {
                let inherited =
                    SessionRepo::get_by_inherited_plan_artifact_id_sync(conn, &old_id)?;
                if !inherited.is_empty() {
                    return Err(AppError::Validation(
                        "Cannot update inherited plan. Use create_plan_artifact to create a session-specific plan.".to_string(),
                    ));
                }
            }

            // 5. Create NEW artifact with incremented version (version chain, not in-place update)
            let new_artifact = Artifact {
                id: ArtifactId::new(),
                artifact_type: old_artifact.artifact_type.clone(),
                name: old_artifact.name.clone(),
                content: ArtifactContent::Inline { text: content },
                metadata: ArtifactMetadata::new(&old_artifact.metadata.created_by)
                    .with_version(old_artifact.metadata.version + 1),
                derived_from: vec![],
                bucket_id: old_artifact.bucket_id.clone(),
            };
            let created =
                ArtifactRepo::create_with_previous_version_sync(conn, new_artifact, &old_id)?;

            // 6. Batch-update all sessions pointing to old artifact to point to new one
            let session_ids: Vec<String> = owning_sessions
                .iter()
                .map(|s| s.id.as_str().to_string())
                .collect();
            SessionRepo::batch_update_artifact_id_sync(conn, &session_ids, created.id.as_str())?;

            // 7. Fetch proposals linked to old artifact (before batch-updating them)
            let linked_proposals = ProposalRepo::get_by_plan_artifact_id_sync(conn, &old_id)?;
            let linked_proposal_ids: Vec<String> =
                linked_proposals.iter().map(|p| p.id.to_string()).collect();

            // 8. Batch-update all linked proposals to point to the new artifact version.
            //    plan_version_at_creation is intentionally NOT changed (preserves original birth version).
            ProposalRepo::batch_update_artifact_id_sync(conn, &old_id, created.id.as_str())?;

            // 9. Conditionally reset verification — only when verification_in_progress = 0.
            //    Prevents the loop-reset paradox where auto-corrections reset verification mid-loop.
            let reset = if let Some(session) = owning_sessions.first() {
                SessionRepo::reset_verification_sync(conn, session.id.as_str())?
            } else {
                false
            };

            Ok((created, old_id, owning_sessions, linked_proposal_ids, reset))
        })
        .await
        .map_err(|e| {
            error!("update_plan_artifact transaction failed: {}", e);
            map_app_err(e)
        })?;

    // Emit events outside the lock (acceptable crash-consistency gap)
    if let Some(app_handle) = &state.app_state.app_handle {
        let content_text = match &created.content {
            ArtifactContent::Inline { text } => text.clone(),
            ArtifactContent::File { path } => format!("[File: {}]", path),
        };

        if verification_reset {
            if let Some(session) = sessions.first() {
                let _ = app_handle.emit(
                    "plan_verification:status_changed",
                    serde_json::json!({
                        "session_id": session.id.as_str(),
                        "status": "unverified",
                        "in_progress": false,
                    }),
                );
            }
        }

        // Emit plan_artifact:updated event with the NEW artifact info
        let _ = app_handle.emit(
            "plan_artifact:updated",
            serde_json::json!({
                "artifactId": created.id.as_str(),
                "previousArtifactId": old_artifact_id_str,
                "sessionId": sessions.first().map(|s| s.id.as_str()),
                "artifact": {
                    "id": created.id.as_str(),
                    "name": created.name,
                    "content": content_text,
                    "version": created.metadata.version,
                }
            }),
        );

        // If there are linked proposals, emit sync notification
        if !linked_proposal_ids.is_empty() {
            let payload = PlanProposalsSyncPayload {
                artifact_id: created.id.to_string(),
                previous_artifact_id: old_artifact_id_str.clone(),
                proposal_ids: linked_proposal_ids,
                new_version: created.metadata.version,
                session_id: sessions.first().map(|s| s.id.to_string()),
                proposals_relinked: true,
            };
            let _ = app_handle.emit("plan:proposals_may_need_update", payload);
        }
    }

    let mut response = ArtifactResponse::from(created);
    response.previous_artifact_id = Some(old_artifact_id_str);
    response.session_id = sessions.first().map(|s| s.id.to_string());

    Ok(Json(response))
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
) -> Result<Json<SuccessResponse>, HttpError> {
    let input_artifact_id = req.artifact_id.clone();
    let proposal_id_strs = req.proposal_ids;

    // Single lock acquisition: all DB work in one transaction.
    state
        .app_state
        .db
        .run_transaction(move |conn| {
            // 1. Resolve stale artifact ID to latest version in the chain
            let artifact_id_str = ArtifactRepo::resolve_latest_sync(conn, &input_artifact_id)?;

            // 2. Verify resolved artifact exists (and get version for plan_version_at_creation)
            let artifact = ArtifactRepo::get_by_id_sync(conn, &artifact_id_str)?
                .ok_or_else(|| {
                    AppError::NotFound(format!("Artifact {} not found", artifact_id_str))
                })?;

            // 3. Guard: reject mutations on Archived/Accepted sessions
            let owning_sessions =
                SessionRepo::get_by_plan_artifact_id_sync(conn, &artifact_id_str)?;
            if let Some(session) = owning_sessions.first() {
                crate::http_server::helpers::assert_session_mutable(session)?;
            }

            // 4. Batch-link all proposals: set plan_artifact_id + plan_version_at_creation
            ProposalRepo::batch_link_proposals_sync(
                conn,
                &proposal_id_strs,
                &artifact_id_str,
                artifact.metadata.version,
            )?;

            Ok(())
        })
        .await
        .map_err(|e| {
            error!("link_proposals_to_plan transaction failed: {}", e);
            map_app_err(e)
        })?;

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
            error!(
                "Failed to get session {} for plan retrieval: {}",
                session_id.as_str(),
                e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Prefer the session's own plan; fall back to the inherited plan (read-only)
    let (artifact_id, is_inherited) = if let Some(own_plan_id) = session.plan_artifact_id {
        (own_plan_id, false)
    } else if let Some(inherited_id) = session.inherited_plan_artifact_id {
        (inherited_id, true)
    } else {
        return Ok(Json(None));
    };

    let artifact = state
        .app_state
        .artifact_repo
        .get_by_id(&artifact_id)
        .await
        .map_err(|e| {
            error!(
                "Failed to get plan artifact {} for session {}: {}",
                artifact_id.as_str(),
                session_id.as_str(),
                e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut response = ArtifactResponse::from(artifact);
    response.is_inherited = Some(is_inherited);
    Ok(Json(Some(response)))
}

/// Get version history for a plan artifact
/// Returns list of version summaries from newest to oldest
#[cfg(test)]
#[path = "artifacts_tests.rs"]
mod tests;

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
            error!(
                "Failed to get artifact {} for history: {}",
                artifact_id.as_str(),
                e
            );
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
            error!(
                "Failed to get history for artifact {}: {}",
                artifact_id.as_str(),
                e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(
        history
            .into_iter()
            .map(ArtifactVersionSummaryResponse::from)
            .collect(),
    ))
}
