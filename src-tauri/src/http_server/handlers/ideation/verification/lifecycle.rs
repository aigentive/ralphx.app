use super::*;

/// Stop any running verification child agents for a session.
///
/// Called when verification is skipped or reverted to immediately release the write lock
/// so the parent session can resume plan editing. Best-effort: errors are swallowed so the
/// caller's skip/revert succeeds even if the agent is already dead.
pub(crate) async fn stop_verification_children(
    session_id: &str,
    app_state: &AppState,
) -> Result<(), AppError> {
    use tauri::Emitter;
    let session_id_typed = IdeationSessionId::from_string(session_id.to_string());
    let children = app_state
        .ideation_session_repo
        .get_verification_children(&session_id_typed)
        .await?;

    for child in &children {
        let key = RunningAgentKey::new("ideation", child.id.as_str());
        if app_state.running_agent_registry.is_running(&key).await {
            if let Ok(Some(info)) = app_state.running_agent_registry.stop(&key).await {
                // Remove from interactive process registry (closes stdin pipe)
                let ipr_key = InteractiveProcessKey::new("ideation", child.id.as_str());
                app_state.interactive_process_registry.remove(&ipr_key).await;

                // Mark agent run as failed
                let run_id =
                    crate::domain::entities::AgentRunId::from_string(&info.agent_run_id);
                app_state
                    .agent_run_repo
                    .fail(&run_id, "Verification cancelled")
                    .await
                    .ok();

                // Emit frontend events
                if let Some(ref app_handle) = app_state.app_handle {
                    app_handle
                        .emit(
                            "agent:stopped",
                            serde_json::json!({
                                "conversation_id": info.conversation_id,
                                "agent_run_id": info.agent_run_id,
                                "context_type": "ideation",
                                "context_id": child.id.as_str(),
                            }),
                        )
                        .ok();
                    app_handle
                        .emit(
                            "agent:run_completed",
                            AgentRunCompletedPayload {
                                conversation_id: info.conversation_id.clone(),
                                context_type: "ideation".to_string(),
                                context_id: child.id.as_str().to_string(),
                                claude_session_id: None,
                                run_chain_id: None,
                            },
                        )
                        .ok();
                }
            }
        }
    }
    Ok(())
}

/// Stop an in-progress verification loop for a session.
///
/// Kills any running verification child agents, sets verification status to `skipped`
/// with `convergence_reason: "user_stopped"`, clears the `verification_in_progress` flag,
/// and increments the verification generation to prevent zombie agents from writing stale state.
///
/// Idempotent: if no verification is in progress, returns 200 with a message.
///
/// Route: `POST /api/ideation/sessions/:id/stop-verification`
pub async fn stop_verification(
    State(state): State<HttpServerState>,
    Path(session_id): Path<String>,
) -> Result<Json<SuccessResponse>, JsonError> {
    use crate::domain::entities::ideation::{VerificationMetadata, VerificationStatus};

    let session_id_obj =
        crate::domain::entities::IdeationSessionId::from_string(session_id.clone());

    // Read session
    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id_obj)
        .await
        .map_err(|e| {
            error!("Failed to get session {}: {}", session_id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get session")
        })?
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "Session not found"))?;

    // Guard: reject calls targeting verification child sessions — orchestrators must use parent session_id
    if session.session_purpose == crate::domain::entities::SessionPurpose::Verification {
        return Err(json_error(
            StatusCode::BAD_REQUEST,
            "Cannot stop verification on a verification child session. Use the parent session_id.",
        ));
    }

    // Session must be active
    if !session.is_active() {
        return Err(json_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Session is not active",
        ));
    }

    // Guard: external sessions cannot stop plan verification
    if session.origin == crate::domain::entities::ideation::SessionOrigin::External {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "External sessions cannot stop plan verification.",
        ));
    }

    // Idempotent: if no verification is running, return 200 without doing anything
    if !session.verification_in_progress {
        return Ok(Json(SuccessResponse {
            success: true,
            message: "Verification is not in progress".to_string(),
        }));
    }

    // Kill any running verification child agents (best-effort)
    stop_verification_children(&session_id, &state.app_state).await.ok();

    // Update metadata: preserve existing metadata and set convergence_reason = "user_stopped"
    let mut metadata: VerificationMetadata = session
        .verification_metadata
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    metadata.convergence_reason = Some("user_stopped".to_string());
    let metadata_json = serde_json::to_string(&metadata).ok();

    // Persist: verification_status = skipped, verification_in_progress = false
    state
        .app_state
        .ideation_session_repo
        .update_verification_state(&session_id_obj, VerificationStatus::Skipped, false, metadata_json)
        .await
        .map_err(|e| {
            error!(
                "Failed to update verification state for {}: {}",
                session_id, e
            );
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to stop verification",
            )
        })?;

    tracing::info!(
        session_id = %session_id,
        "Verification stopped by user"
    );

    // Increment generation to prevent zombie verifier from writing stale terminal status
    state
        .app_state
        .ideation_session_repo
        .increment_verification_generation(&session_id_obj)
        .await
        .ok();

    // Emit plan_verification:status_changed event so frontend VerificationBadge updates
    if let Some(app_handle) = &state.app_state.app_handle {
        emit_verification_status_changed(
            app_handle,
            &session_id,
            VerificationStatus::Skipped,
            false,
            Some(&metadata),
            Some("user_stopped"),
            Some(session.verification_generation),
        );
    }

    Ok(Json(SuccessResponse {
        success: true,
        message: "Verification stopped".to_string(),
    }))
}
/// POST /api/ideation/sessions/:id/revert-and-skip
///
/// Atomically revert plan content to a previous version and skip verification.
/// Both the artifact INSERT and session UPDATE happen in a single `db.run(|conn| { ... })`
/// transaction — no partial failure where artifact is created but session update fails.
pub async fn revert_and_skip(
    State(state): State<HttpServerState>,
    Path(session_id): Path<String>,
    Json(req): Json<RevertAndSkipRequest>,
) -> Result<Json<SuccessResponse>, JsonError> {
    use crate::domain::entities::ideation::VerificationStatus;
    use crate::domain::entities::{ArtifactContent, ArtifactId};

    let session_id_obj =
        crate::domain::entities::IdeationSessionId::from_string(session_id.clone());

    // Read session
    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id_obj)
        .await
        .map_err(|e| {
            error!("Failed to get session {}: {}", session_id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get session")
        })?
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "Session not found"))?;

    // Guard: external sessions cannot skip plan verification
    if session.origin == crate::domain::entities::ideation::SessionOrigin::External {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "External sessions cannot skip plan verification. Run verification to completion (update_plan_verification with status 'reviewing').",
        ));
    }

    // Session must be active
    if !session.is_active() {
        return Err(json_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Session is not active",
        ));
    }

    // Read the plan artifact version to restore
    let restore_artifact_id = ArtifactId::from_string(req.plan_version_to_restore.clone());
    let artifact = state
        .app_state
        .artifact_repo
        .get_by_id(&restore_artifact_id)
        .await
        .map_err(|e| {
            error!(
                "Failed to get artifact {}: {}",
                req.plan_version_to_restore, e
            );
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get plan artifact",
            )
        })?
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "Plan artifact not found"))?;

    // Extract inline text content (plan artifacts must be inline)
    let content_text = match &artifact.content {
        ArtifactContent::Inline { text } => text.clone(),
        ArtifactContent::File { .. } => {
            return Err(json_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Plan artifact must be inline text content",
            ));
        }
    };

    // Pre-generate artifact ID for logging before the atomic operation
    let new_artifact_id = ArtifactId::new();
    let new_artifact_id_str = new_artifact_id.as_str().to_string();
    let new_version = artifact.metadata.version + 1;

    // Single atomic operation: INSERT artifact + UPDATE session in one db.run() transaction.
    // Prevents the race where artifact is created but session update fails.
    state
        .app_state
        .ideation_session_repo
        .revert_plan_and_skip_with_artifact(
            &session_id_obj,
            new_artifact_id_str.clone(),
            artifact.artifact_type.to_string(),
            artifact.name.clone(),
            content_text,
            new_version,
            restore_artifact_id.as_str().to_string(),
            "user_reverted".to_string(),
        )
        .await
        .map_err(|e| {
            error!("Failed revert-and-skip for session {}: {}", session_id, e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to atomically revert plan and skip verification",
            )
        })?;

    tracing::info!(
        session_id = %session_id,
        plan_version = %req.plan_version_to_restore,
        new_artifact_id = %new_artifact_id_str,
        "Revert-and-skip completed atomically"
    );

    // Kill any running verification child agents before emitting events.
    // Generation increment is handled inside the atomic SQL transaction above.
    stop_verification_children(&session_id, &state.app_state).await.ok();

    // Emit event with canonical payload (B3: was missing round/gaps/rounds fields)
    if let Some(app_handle) = &state.app_state.app_handle {
        emit_verification_status_changed(
            app_handle,
            &session_id,
            VerificationStatus::Skipped,
            false,
            None,
            Some("user_reverted"),
            Some(session.verification_generation),
        );
    }

    Ok(Json(SuccessResponse {
        success: true,
        message: "Plan reverted and verification skipped".to_string(),
    }))
}
