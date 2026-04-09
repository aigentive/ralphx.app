use super::*;

/// Emit the verification-started event, build the round-loop description, spawn the verification
/// child session, and handle any spawn failures.
///
/// Returns `true` when the agent was spawned successfully, `false` when the spawn failed (in
/// which case [`handle_verification_spawn_failure`] has already been called).
pub async fn spawn_verification_agent(
    state: &HttpServerState,
    session_id: &IdeationSessionId,
    generation: i32,
    disabled_specialists: &[String],
) -> bool {
    let cfg = default_verification_config();
    if let Some(app_handle) = &state.app_state.app_handle {
        emit_verification_started(app_handle, session_id.as_str(), generation, cfg.max_rounds);
    }
    let title = format!("Auto-verification (gen {generation})");
    let description = format!(
        "Run verification round loop. parent_session_id: {}, generation: {generation}, max_rounds: {}",
        session_id.as_str(),
        cfg.max_rounds
    );
    match crate::http_server::handlers::session_linking::create_verification_child_session(
        state,
        session_id.as_str(),
        &description,
        &title,
        disabled_specialists,
    )
    .await
    {
        Ok(true) => true,
        Ok(false) => {
            handle_verification_spawn_failure(state, session_id, generation, None).await;
            false
        }
        Err(e) => {
            handle_verification_spawn_failure(state, session_id, generation, Some(&e)).await;
            false
        }
    }
}

/// Handle a failed verification agent spawn: reset auto-verify state and emit status-changed event.
///
/// Called from both `create_plan_artifact` and `confirm_verification` when
/// `create_verification_child_session` returns `Ok(false)` or `Err(e)`.
pub async fn handle_verification_spawn_failure(
    state: &HttpServerState,
    session_id: &IdeationSessionId,
    generation: i32,
    error: Option<&str>,
) {
    if let Some(msg) = error {
        error!(
            "Verifier spawn failed for session {}: {}",
            session_id.as_str(),
            msg
        );
    } else {
        tracing::warn!(
            "Verification agent failed to spawn for session {}",
            session_id.as_str()
        );
    }
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
