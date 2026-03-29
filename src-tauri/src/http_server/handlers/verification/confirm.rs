use super::*;

pub async fn confirm_verification(
    State(state): State<HttpServerState>,
    Json(req): Json<ConfirmVerificationRequest>,
) -> Result<Json<VerificationActionResponse>, HttpError> {
    let session_id_str = req.session_id.clone();
    let disabled_specialists = req.disabled_specialists.clone();

    // Remove pending confirmation entry (no-op if user-initiated path has no pending entry)
    {
        let mut pending = state.app_state.pending_verifications.lock().await;
        pending.remove(&session_id_str);
    }

    let cfg = verification_config();

    // Run transaction: verify session exists + trigger auto-verify
    let sid_clone = session_id_str.clone();
    let (session_id, generation) = state
        .app_state
        .db
        .run_transaction(move |conn| {
            let sid = IdeationSessionId::from_string(sid_clone);
            // Ensure session exists
            let _session = SessionRepo::get_by_id_sync(conn, sid.as_str())?
                .ok_or_else(|| AppError::NotFound(format!("Session {} not found", sid)))?;

            let generation = SessionRepo::trigger_auto_verify_sync(conn, sid.as_str())?
                .ok_or_else(|| {
                    AppError::Infrastructure(
                        "trigger_auto_verify_sync returned None — session may already be verifying"
                            .to_string(),
                    )
                })?;

            Ok((sid, generation))
        })
        .await
        .map_err(|e| {
            error!("confirm_verification transaction failed: {}", e);
            map_app_err_local(e)
        })?;

    // Emit verification started event
    if let Some(app_handle) = &state.app_state.app_handle {
        emit_verification_started(app_handle, session_id.as_str(), generation, cfg.max_rounds);
    }

    // Build description (includes DISABLED_SPECIALISTS if any — injected by create_verification_child_session)
    let description = format!(
        "Run verification round loop. parent_session_id: {}, generation: {generation}, max_rounds: {}",
        session_id.as_str(),
        cfg.max_rounds
    );
    let title = format!("Auto-verification (gen {generation})");

    match crate::http_server::handlers::session_linking::create_verification_child_session(
        &state,
        session_id.as_str(),
        &description,
        &title,
        &disabled_specialists,
    )
    .await
    {
        Ok(true) => {}
        Ok(false) => {
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
            } else if let Some(app_handle) = &state.app_state.app_handle {
                emit_verification_status_changed(
                    app_handle,
                    session_id.as_str(),
                    VerificationStatus::Unverified,
                    false,
                    None,
                    Some("spawn_failed"),
                    Some(generation),
                );
            }
        }
        Err(e) => {
            error!(
                "Verifier spawn failed for session {}: {}",
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
            } else if let Some(app_handle) = &state.app_state.app_handle {
                emit_verification_status_changed(
                    app_handle,
                    session_id.as_str(),
                    VerificationStatus::Unverified,
                    false,
                    None,
                    Some("spawn_failed"),
                    Some(generation),
                );
            }
        }
    }

    Ok(Json(VerificationActionResponse {
        status: "ok".to_string(),
    }))
}
