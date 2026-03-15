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
    IdeationSession, IdeationSessionId, VerificationStatus,
};
use rusqlite::Connection;
use crate::domain::services::emit_verification_status_changed;
use crate::error::AppError;
use crate::infrastructure::agents::claude::verification_config;
use crate::infrastructure::sqlite::{
    SqliteArtifactRepository as ArtifactRepo, SqliteIdeationSessionRepository as SessionRepo,
    SqliteTaskProposalRepository as ProposalRepo,
};

// ============================================================================
// EditError Types
// ============================================================================

/// Error type for apply_edits pure function.
#[derive(Debug)]
pub enum EditError {
    /// The old_text anchor was not found in the content.
    AnchorNotFound {
        edit_index: usize,
        old_text_preview: String,
    },
    /// The old_text anchor matches multiple locations (ambiguous).
    AmbiguousAnchor {
        edit_index: usize,
        old_text_preview: String,
    },
}

/// Apply sequential old_text→new_text edits to content.
///
/// Edits are applied SEQUENTIALLY — each edit sees the result of all previous edits,
/// not the original content. If any edit fails (anchor not found or ambiguous),
/// the entire operation returns an error and no changes are applied.
///
/// **Ambiguity check**: Verifies that each old_text appears exactly once in the
/// CURRENT content (after prior edits). The check starts searching AFTER the first
/// match ends (`pos + old_text.len()`) to avoid false positives from the match itself.
///
/// **Phantom match note**: If edit N's `new_text` introduces text matching edit N+1's
/// `old_text`, edit N+1 will operate on the introduced text (by design). Agents should
/// use unique 20+ char anchors to avoid ambiguity from sequential interactions.
#[allow(dead_code)]
pub fn apply_edits(content: &str, edits: &[PlanEdit]) -> Result<String, EditError> {
    let mut result = content.to_string();
    for (i, edit) in edits.iter().enumerate() {
        // Find exact match
        let pos = result.find(&edit.old_text).ok_or_else(|| EditError::AnchorNotFound {
            edit_index: i,
            old_text_preview: edit.old_text.chars().take(80).collect(),
        })?;

        // Verify unique match — check for second occurrence AFTER the first match ends.
        // Use pos + old_text.len() to skip the matched text itself.
        if result[pos + edit.old_text.len()..].contains(&edit.old_text) {
            return Err(EditError::AmbiguousAnchor {
                edit_index: i,
                old_text_preview: edit.old_text.chars().take(80).collect(),
            });
        }

        // Apply replacement
        result = format!(
            "{}{}{}",
            &result[..pos],
            &edit.new_text,
            &result[pos + edit.old_text.len()..],
        );
    }
    Ok(result)
}

/// Map an AppError to an HttpError for handler responses.
fn map_app_err(e: AppError) -> HttpError {
    match e {
        AppError::Validation(msg) => HttpError::validation(msg),
        AppError::NotFound(_) => StatusCode::NOT_FOUND.into(),
        _ => StatusCode::INTERNAL_SERVER_ERROR.into(),
    }
}

/// Shared core for both update_plan_artifact and (future) edit_plan_artifact.
///
/// Takes the resolved artifact + new content, creates a new version,
/// batch-updates sessions/proposals, resets verification, and returns
/// data needed for event emission.
///
/// IMPORTANT: This helper does NOT trigger auto-verification.
/// Auto-verify is triggered ONLY by create_plan_artifact (which calls
/// trigger_auto_verify_sync separately). Both update and edit handlers
/// use finalize_plan_update, which handles:
///   - Create new version (version + 1, previous_version_id = old.id)
///   - Batch-update sessions pointing to old → new
///   - Batch-update proposals (preserve plan_version_at_creation)
///   - Conditional verification reset (CAS: only if in_progress=0)
///
/// The caller is responsible for emitting events:
///   - plan_artifact:updated { previous_artifact_id: old.id, new_artifact_id: new.id, session_id }
///   - plan:proposals_may_need_update (only if linked proposals exist)
///
/// Returns a tuple containing:
///   - (created_artifact, old_artifact_id, owning_sessions, linked_proposal_ids, verification_reset)
fn finalize_plan_update(
    conn: &Connection,
    old_artifact: &Artifact,
    new_content: String,
) -> Result<(Artifact, String, Vec<IdeationSession>, Vec<String>, bool), AppError> {
    let old_id = old_artifact.id.as_str().to_string();

    // 1. Create NEW artifact with incremented version (version chain, not in-place update)
    let new_artifact = Artifact {
        id: ArtifactId::new(),
        artifact_type: old_artifact.artifact_type.clone(),
        name: old_artifact.name.clone(),
        content: ArtifactContent::Inline { text: new_content },
        metadata: ArtifactMetadata::new(&old_artifact.metadata.created_by)
            .with_version(old_artifact.metadata.version + 1),
        derived_from: vec![],
        bucket_id: old_artifact.bucket_id.clone(),
    };
    let created =
        ArtifactRepo::create_with_previous_version_sync(conn, new_artifact, &old_id)?;

    // 2. Batch-update all sessions pointing to old artifact to point to new one
    let owning_sessions = SessionRepo::get_by_plan_artifact_id_sync(conn, &old_id)?;
    let session_ids: Vec<String> = owning_sessions
        .iter()
        .map(|s| s.id.as_str().to_string())
        .collect();
    SessionRepo::batch_update_artifact_id_sync(conn, &session_ids, created.id.as_str())?;

    // 3. Fetch proposals linked to old artifact (before batch-updating them)
    let linked_proposals = ProposalRepo::get_by_plan_artifact_id_sync(conn, &old_id)?;
    let linked_proposal_ids: Vec<String> =
        linked_proposals.iter().map(|p| p.id.to_string()).collect();

    // 4. Batch-update all linked proposals to point to the new artifact version.
    //    plan_version_at_creation is intentionally NOT changed (preserves original birth version).
    ProposalRepo::batch_update_artifact_id_sync(conn, &old_id, created.id.as_str())?;

    // 5. Conditionally reset verification — only when verification_in_progress = 0.
    //    Prevents the loop-reset paradox where auto-corrections reset verification mid-loop.
    let verification_reset = if let Some(session) = owning_sessions.first() {
        SessionRepo::reset_verification_sync(conn, session.id.as_str())?
    } else {
        false
    };

    Ok((created, old_id, owning_sessions, linked_proposal_ids, verification_reset))
}

pub async fn create_plan_artifact(
    State(state): State<HttpServerState>,
    Json(req): Json<CreatePlanArtifactRequest>,
) -> Result<Json<ArtifactResponse>, HttpError> {
    let session_id_str = req.session_id.clone();
    let title = req.title.clone();
    let content = req.content.clone();
    let cfg = verification_config();
    let auto_verify_enabled = cfg.auto_verify;

    // Single lock acquisition: all DB work in one transaction.
    // Events emitted after db.run_transaction() returns (acceptable crash-consistency gap).
    // Returns auto_verify_generation=Some(gen) if auto-verify trigger was atomically applied.
    let (session_id, created, auto_verify_generation) = state
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

            // Atomically trigger auto-verify within the same transaction.
            // Condition: auto_verify enabled AND verification_in_progress == 0.
            // Sets: status=reviewing, in_progress=1, generation++ in a single UPDATE.
            let auto_verify_generation = if auto_verify_enabled {
                SessionRepo::trigger_auto_verify_sync(conn, sid.as_str())?
            } else {
                None
            };

            Ok((sid, created, auto_verify_generation))
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

    // Spawn auto-verifier after commit, if trigger was applied.
    // Fire-and-forget: spawn failure resets in_progress so reconciler/user can retry.
    if let Some(generation) = auto_verify_generation {
        let cfg = verification_config();
        let title = format!("Auto-verification (gen {generation})");
        let description = format!(
            "Run verification round loop. parent_session_id: {}, generation: {generation}, max_rounds: {}",
            session_id.as_str(),
            cfg.max_rounds
        );
        match crate::http_server::handlers::session_linking::create_verification_child_session(
            &state,
            session_id.as_str(),
            &description,
            &title,
        )
        .await
        {
            Ok(true) => {} // orchestration triggered — success
            Ok(false) => {
                // Plan-verifier failed to spawn — reset in_progress to avoid permanently locking
                tracing::warn!(
                    "Verification agent failed to spawn for session {}",
                    session_id.as_str()
                );
                let sid_str = session_id.as_str().to_string();
                if let Err(reset_err) = state
                    .app_state
                    .db
                    .run(move |conn| SessionRepo::reset_auto_verify_sync(conn, &sid_str))
                    .await
                {
                    error!(
                        "Failed to reset auto-verify state for session {} after spawn failure: {}",
                        session_id.as_str(),
                        reset_err
                    );
                }
            }
            Err(e) => {
                error!(
                    "Auto-verifier spawn failed for session {}: {}",
                    session_id.as_str(),
                    e
                );
                let sid_str = session_id.as_str().to_string();
                if let Err(reset_err) = state
                    .app_state
                    .db
                    .run(move |conn| SessionRepo::reset_auto_verify_sync(conn, &sid_str))
                    .await
                {
                    error!(
                        "Failed to reset auto-verify state for session {} after spawn failure: {}",
                        session_id.as_str(),
                        reset_err
                    );
                }
            }
        }
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

            // 5. Shared finalization: create version, batch-update, verification reset
            //    Does NOT trigger auto-verify — that's only in create_plan_artifact
            finalize_plan_update(conn, &old_artifact, content)
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
                // B4: use shared helper for canonical payload (was missing round/gaps/rounds fields)
                emit_verification_status_changed(
                    app_handle,
                    session.id.as_str(),
                    VerificationStatus::Unverified,
                    false,
                    None,
                    None,
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

pub async fn edit_plan_artifact(
    State(state): State<HttpServerState>,
    Json(req): Json<EditPlanArtifactRequest>,
) -> Result<Json<ArtifactResponse>, HttpError> {
    let input_artifact_id = req.artifact_id.clone();
    let edits = req.edits;

    // Pre-transaction input validation (defense-in-depth — MCP schema validates too)
    if edits.is_empty() {
        return Err(HttpError::validation("edits array must not be empty".to_string()));
    }
    for (i, edit) in edits.iter().enumerate() {
        if edit.old_text.is_empty() {
            return Err(HttpError::validation(format!(
                "Edit #{i}: old_text must not be empty"
            )));
        }
        if edit.old_text.len() > 100_000 || edit.new_text.len() > 100_000 {
            return Err(HttpError::validation(format!(
                "Edit #{i}: old_text/new_text exceeds 100KB limit"
            )));
        }
    }

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
                .ok_or_else(|| AppError::NotFound(format!("Artifact {old_id} not found")))?;

            // 3. Guard: reject mutations on Archived/Accepted sessions
            let owning_sessions = SessionRepo::get_by_plan_artifact_id_sync(conn, &old_id)?;
            if let Some(session) = owning_sessions.first() {
                crate::http_server::helpers::assert_session_mutable(session)?;
            }

            // 4. Guard: reject edit if this artifact is only referenced as an inherited plan
            if owning_sessions.is_empty() {
                let inherited =
                    SessionRepo::get_by_inherited_plan_artifact_id_sync(conn, &old_id)?;
                if !inherited.is_empty() {
                    return Err(AppError::Validation(
                        "Cannot edit inherited plan. Use create_plan_artifact to create a session-specific plan.".to_string(),
                    ));
                }
            }

            // 5. Guard: only inline content is supported (file-backed artifacts cannot be edited)
            let current_content = match &old_artifact.content {
                ArtifactContent::Inline { text } => text.clone(),
                ArtifactContent::File { .. } => {
                    return Err(AppError::Validation(
                        "Cannot edit file-backed artifacts. Use update_plan_artifact with full content.".to_string(),
                    ));
                }
            };

            // 6. Apply edits (pure function — returns error if any anchor not found/ambiguous)
            let new_content = apply_edits(&current_content, &edits).map_err(|e| {
                let http_err: HttpError = e.into();
                AppError::Validation(
                    http_err
                        .message
                        .unwrap_or_else(|| "Edit failed".to_string()),
                )
            })?;

            // 7. Guard: post-apply content size (prevent unbounded growth)
            if new_content.len() > 500_000 {
                return Err(AppError::Validation(format!(
                    "Resulting plan content exceeds 500KB limit ({} bytes). Use fewer/smaller edits.",
                    new_content.len()
                )));
            }

            // 8. Shared finalization: create version, batch-update, verification reset
            //    Does NOT trigger auto-verify — that's only in create_plan_artifact
            finalize_plan_update(conn, &old_artifact, new_content)
        })
        .await
        .map_err(|e| {
            error!("edit_plan_artifact transaction failed: {}", e);
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
                emit_verification_status_changed(
                    app_handle,
                    session.id.as_str(),
                    VerificationStatus::Unverified,
                    false,
                    None,
                    None,
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

pub async fn get_artifact_history(
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
