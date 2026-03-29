use super::*;

pub async fn create_plan_artifact(
    State(state): State<HttpServerState>,
    Json(req): Json<CreatePlanArtifactRequest>,
) -> Result<Json<ArtifactResponse>, HttpError> {
    let session_id_str = req.session_id.clone();
    let title = req.title.clone();
    let content = req.content.clone();
    let cfg = verification_config();
    let auto_verify_enabled = cfg.auto_verify;

    // Check in-memory auto-accept state BEFORE the transaction (async lock)
    let is_auto_accept = {
        let auto_accept = state.app_state.auto_accept_sessions.lock().await;
        auto_accept.contains(&session_id_str)
    };

    let (session_id, created, auto_verify_generation, project_id, session_origin, session_title) =
        state
            .app_state
            .db
            .run_transaction(move |conn| {
                let sid = IdeationSessionId::from_string(session_id_str);

                let session = SessionRepo::get_by_id_sync(conn, sid.as_str())?
                    .ok_or_else(|| AppError::NotFound(format!("Session {} not found", sid)))?;

                let is_external = session.origin == SessionOrigin::External;
                let should_auto_verify =
                    auto_verify_enabled || is_external || is_auto_accept;

                crate::http_server::helpers::assert_session_mutable(&session)?;

                let bucket_id = ArtifactBucketId::from_string("prd-library");
                let artifact = Artifact {
                    id: ArtifactId::new(),
                    artifact_type: ArtifactType::Specification,
                    name: title,
                    content: ArtifactContent::inline(&content),
                    metadata: ArtifactMetadata::new("orchestrator").with_version(1),
                    derived_from: vec![],
                    bucket_id: Some(bucket_id),
                    archived_at: None,
                };

                let created = if let Some(existing_plan_id) = &session.plan_artifact_id {
                    let prev_id = existing_plan_id.as_str().to_string();
                    ArtifactRepo::create_with_previous_version_sync(conn, artifact, &prev_id)?
                } else {
                    ArtifactRepo::create_sync(conn, artifact)?
                };

                SessionRepo::update_plan_artifact_id_sync(
                    conn,
                    sid.as_str(),
                    Some(created.id.as_str()),
                )?;
                SessionRepo::update_plan_version_last_read_sync(conn, sid.as_str(), 1)?;

                let auto_verify_generation = if should_auto_verify {
                    SessionRepo::trigger_auto_verify_sync(conn, sid.as_str())?
                } else {
                    None
                };

                let session_title = session.title.clone();
                let session_origin = session.origin.clone();
                Ok((sid, created, auto_verify_generation, session.project_id.clone(), session_origin, session_title))
            })
            .await
            .map_err(|e| {
                error!("create_plan_artifact transaction failed: {}", e);
                map_app_err(e)
            })?;

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

    let ideation_plan_payload = serde_json::json!({
        "session_id": session_id.as_str(),
        "project_id": project_id.as_str(),
        "artifact_id": created.id.as_str(),
        "plan_title": created.name,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    if let Some(app_handle) = &state.app_state.app_handle {
        let _ = app_handle.emit("ideation:plan_created", &ideation_plan_payload);
    }

    if let Err(e) = state
        .app_state
        .external_events_repo
        .insert_event(
            "ideation:plan_created",
            project_id.as_str(),
            &ideation_plan_payload.to_string(),
        )
        .await
    {
        tracing::warn!(error = %e, "Failed to persist IdeationPlanCreated event (non-fatal)");
    }

    if let Some(ref publisher) = state.app_state.webhook_publisher {
        let _ = publisher
            .publish(
                EventType::IdeationPlanCreated,
                project_id.as_str(),
                ideation_plan_payload,
            )
            .await;
    }

    if let Some(generation) = auto_verify_generation {
        let cfg = verification_config();
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
            &state,
            session_id.as_str(),
            &description,
            &title,
            &[],
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
    } else if session_origin != SessionOrigin::External {
        // UI session without auto-verify: insert PendingVerification and emit confirmation event
        let cfg = verification_config();
        let pending = crate::application::app_state::PendingVerification {
            session_id: session_id.as_str().to_string(),
            session_title: session_title.clone().unwrap_or_default(),
            plan_artifact_id: created.id.as_str().to_string(),
            available_specialists: cfg.specialists.clone(),
            created_at: chrono::Utc::now(),
        };
        {
            let mut pending_verifications =
                state.app_state.pending_verifications.lock().await;
            pending_verifications.insert(session_id.as_str().to_string(), pending);
        }
        if let Some(app_handle) = &state.app_state.app_handle {
            crate::domain::services::emit_verification_pending_confirmation(
                app_handle,
                session_id.as_str(),
                &session_title.unwrap_or_default(),
                created.id.as_str(),
            );
        }
    }

    Ok(Json(ArtifactResponse::from(created)))
}
