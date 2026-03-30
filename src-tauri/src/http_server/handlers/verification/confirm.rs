use super::*;
use super::helpers::handle_verification_spawn_failure;

pub async fn confirm_verification(
    State(state): State<HttpServerState>,
    Json(req): Json<ConfirmVerificationRequest>,
) -> Result<Json<VerificationActionResponse>, HttpError> {
    let session_id_str = req.session_id.clone();
    let disabled_specialists = req.disabled_specialists.clone();

    // Set DB status to 'accepted' — marks that user has confirmed and verification is starting.
    let session_id_for_status = IdeationSessionId::from_string(session_id_str.clone());
    state
        .app_state
        .ideation_session_repo
        .set_verification_confirmation_status(
            &session_id_for_status,
            Some(VerificationConfirmationStatus::Accepted),
        )
        .await
        .map_err(map_app_err_local)?;

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
            handle_verification_spawn_failure(&state, &session_id, generation, None).await;
        }
        Err(e) => {
            handle_verification_spawn_failure(&state, &session_id, generation, Some(&e)).await;
        }
    }

    Ok(Json(VerificationActionResponse {
        status: "ok".to_string(),
    }))
}
