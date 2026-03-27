use super::*;

async fn initialize_verification_state(
    state: &HttpServerState,
    parent_id: &IdeationSessionId,
    parent: &IdeationSession,
) -> Result<Option<i32>, JsonError> {
    if parent.plan_artifact_id.is_none() {
        return Err(json_error(
            StatusCode::BAD_REQUEST,
            "Cannot start verification: parent session has no plan artifact",
        ));
    }

    let verify_cfg = verification_config();
    let parent_id_str = parent_id.as_str().to_string();
    let verify_result = state
        .app_state
        .db
        .run(move |conn| SessionRepo::trigger_auto_verify_sync(conn, &parent_id_str))
        .await
        .map_err(|e| {
            error!("Failed to trigger verification sync: {}", e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to initialize verification state: {}", e),
            )
        })?;

    match verify_result {
        Some(new_generation) => {
            if let Some(app_handle) = &state.app_state.app_handle {
                emit_verification_started(
                    app_handle,
                    parent_id.as_str(),
                    new_generation,
                    verify_cfg.max_rounds,
                );
            }
            Ok(Some(new_generation))
        }
        None => {
            let parent_id_str = parent_id.as_str().to_string();
            let fresh_parent = state
                .app_state
                .db
                .run(move |conn| SessionRepo::get_by_id_sync(conn, &parent_id_str))
                .await
                .map_err(|e| {
                    error!("Failed to re-query parent session: {}", e);
                    json_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to check verification state: {}", e),
                    )
                })?;
            let fresh_parent = fresh_parent
                .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "Parent session not found"))?;
            if fresh_parent.verification_in_progress {
                Err(json_error(
                    StatusCode::CONFLICT,
                    "Verification already in progress",
                ))
            } else {
                Err(json_error(
                    StatusCode::BAD_REQUEST,
                    "Cannot verify imported-verified sessions",
                ))
            }
        }
    }
}

fn build_effective_prompts(
    req: &CreateChildSessionRequest,
    verification_generation: Option<i32>,
    max_rounds: u32,
    parent_session_id: &str,
) -> (Option<String>, Option<String>) {
    let effective_initial_prompt = req.initial_prompt.as_ref().map(|prompt| {
        if let Some(generation) = verification_generation {
            format!(
                "{}\n\nparent_session_id: {}, generation: {}, max_rounds: {}",
                prompt, parent_session_id, generation, max_rounds
            )
        } else {
            prompt.clone()
        }
    });
    let effective_description = req.description.as_ref().map(|description| {
        if let Some(generation) = verification_generation {
            format!(
                "{}\n\nparent_session_id: {}, generation: {}, max_rounds: {}",
                description, parent_session_id, generation, max_rounds
            )
        } else {
            description.clone()
        }
    });

    let effective_initial_prompt = effective_initial_prompt.or_else(|| {
        synthesize_verification_prompt(
            &req.purpose,
            verification_generation,
            max_rounds,
            &effective_description,
            parent_session_id,
        )
    });

    (effective_initial_prompt, effective_description)
}

async fn spawn_child_orchestration(
    state: &HttpServerState,
    created_session: &IdeationSession,
    child_session_str: &str,
    prompt: &str,
    parent_id: &IdeationSessionId,
    verification_generation: &mut Option<i32>,
    error_context: &'static str,
) -> bool {
    let chat_service = build_ideation_chat_service(state, created_session);
    match chat_service
        .send_message(
            ChatContextType::Ideation,
            child_session_str,
            prompt,
            Default::default(),
        )
        .await
    {
        Ok(_) => true,
        Err(e) => {
            error!("{error_context} {}: {}", child_session_str, e);
            if let Some(current_generation) = verification_generation.take() {
                rollback_verification_state(state, parent_id, current_generation, "spawn failure");
            }
            // Only archive verification children on spawn failure — general follow-up
            // children remain Active so users can retry orchestration later.
            if created_session.session_purpose == SessionPurpose::Verification {
                let child_id_obj = IdeationSessionId::from_string(child_session_str.to_string());
                if let Err(archive_err) = state
                    .app_state
                    .ideation_session_repo
                    .update_status(&child_id_obj, IdeationSessionStatus::Archived)
                    .await
                {
                    error!(
                        "Failed to archive verification child session {} after spawn failure: {}",
                        child_session_str, archive_err
                    );
                }
            }
            false
        }
    }
}

pub async fn create_child_session(
    State(state): State<HttpServerState>,
    Json(req): Json<CreateChildSessionRequest>,
) -> Result<Json<CreateChildSessionResponse>, JsonError> {
    let parent_id = IdeationSessionId::from_string(req.parent_session_id.clone());
    let verify_cfg = verification_config();
    let mut verification_generation = None;

    let parent = state
        .app_state
        .ideation_session_repo
        .get_by_id(&parent_id)
        .await
        .map_err(|e| {
            error!(
                "Failed to fetch parent session {}: {}",
                parent_id.as_str(),
                e
            );
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch parent session: {}", e),
            )
        })?
        .ok_or_else(|| {
            error!("Parent session {} not found", parent_id.as_str());
            json_error(StatusCode::NOT_FOUND, "Parent session not found")
        })?;

    let ancestor_chain = state
        .app_state
        .ideation_session_repo
        .get_ancestor_chain(&parent_id)
        .await
        .map_err(|e| {
            error!(
                "Failed to get ancestor chain for {}: {}",
                parent_id.as_str(),
                e
            );
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to check for cycles: {}", e),
            )
        })?;

    if ancestor_chain.iter().any(|session| session.id == parent_id) {
        return Err(json_error(
            StatusCode::BAD_REQUEST,
            "Circular reference detected: session cannot be its own parent",
        ));
    }

    if req.purpose.as_deref() == Some("verification") {
        verification_generation =
            initialize_verification_state(&state, &parent_id, &parent).await?;
    }

    let (resolved_team_mode, resolved_team_config_json) = if let Some(mode) = &req.team_mode {
        let config_json = req
            .team_config
            .as_ref()
            .and_then(|config| serde_json::to_value(config).ok());
        (Some(mode.clone()), config_json.map(|value| value.to_string()))
    } else if req.inherit_context {
        (parent.team_mode.clone(), parent.team_config_json.clone())
    } else {
        (None, None)
    };

    let (team_mode, team_config_json) = validate_resolved_team_config(
        resolved_team_mode.as_ref(),
        resolved_team_config_json.as_ref(),
    );

    let child_session = IdeationSession {
        id: IdeationSessionId::new(),
        project_id: parent.project_id.clone(),
        title: req.title.clone(),
        status: IdeationSessionStatus::Active,
        plan_artifact_id: None,
        inherited_plan_artifact_id: if req.inherit_context {
            parent.plan_artifact_id.clone()
        } else {
            None
        },
        seed_task_id: None,
        parent_session_id: Some(parent_id.clone()),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        archived_at: None,
        converted_at: None,
        team_mode,
        team_config_json,
        title_source: None,
        verification_status: VerificationStatus::default(),
        verification_in_progress: false,
        verification_metadata: None,
        verification_generation: 0,
        source_project_id: None,
        source_session_id: None,
        session_purpose: req
            .purpose
            .as_deref()
            .and_then(|purpose| purpose.parse::<SessionPurpose>().ok())
            .unwrap_or_default(),
        cross_project_checked: true,
        plan_version_last_read: None,
        origin: {
            let purpose = req
                .purpose
                .as_deref()
                .and_then(|p| p.parse::<SessionPurpose>().ok())
                .unwrap_or_default();
            if purpose == SessionPurpose::Verification {
                // Verification children are system artifacts; inherit parent origin.
                parent.origin
            } else if req.is_external_trigger {
                SessionOrigin::External
            } else {
                SessionOrigin::Internal
            }
        },
        expected_proposal_count: None,
        auto_accept_status: None,
        auto_accept_started_at: None,
        api_key_id: None,
        idempotency_key: None,
        external_activity_phase: None,
        external_last_read_message_id: None,
        dependencies_acknowledged: false,
    };

    let child_id = child_session.id.clone();
    let child_session_str = child_id.as_str().to_string();
    let parent_session_str = parent_id.as_str().to_string();

    let created_session = state
        .app_state
        .ideation_session_repo
        .create(child_session)
        .await
        .map_err(|e| {
            error!("Failed to create child session: {}", e);
            if let Some(current_generation) = verification_generation {
                rollback_verification_state(
                    &state,
                    &parent_id,
                    current_generation,
                    "child DB insert failure",
                );
            }
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create session: {}", e),
            )
        })?;

    let link = SessionLink::new(
        parent_id.clone(),
        child_id.clone(),
        SessionRelationship::FollowOn,
    );
    state
        .app_state
        .session_link_repo
        .create(link)
        .await
        .map_err(|e| {
            error!("Failed to create session link: {}", e);
            if let Some(current_generation) = verification_generation {
                rollback_verification_state(
                    &state,
                    &parent_id,
                    current_generation,
                    "link creation failure",
                );
            }
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create session link: {}", e),
            )
        })?;

    let parent_context = if req.inherit_context {
        Some(load_parent_context(&state, &parent).await)
    } else {
        None
    };

    let title = created_session
        .title
        .clone()
        .unwrap_or_else(|| "Child Session".to_string());
    let (effective_initial_prompt, effective_description) = build_effective_prompts(
        &req,
        verification_generation,
        verify_cfg.max_rounds,
        &parent_session_str,
    );

    let orchestration_triggered = if let Some(ref prompt) = effective_initial_prompt {
        spawn_child_orchestration(
            &state,
            &created_session,
            &child_session_str,
            prompt,
            &parent_id,
            &mut verification_generation,
            "Failed to auto-spawn agent on child session",
        )
        .await
    } else if let Some(ref description) = effective_description {
        if description.trim().is_empty() {
            false
        } else {
            spawn_child_orchestration(
                &state,
                &created_session,
                &child_session_str,
                description,
                &parent_id,
                &mut verification_generation,
                "Failed to auto-spawn agent on child session (from description)",
            )
            .await
        }
    } else {
        false
    };

    if let Some(app_handle) = &state.app_state.app_handle {
        let mut event_payload = serde_json::json!({
            "sessionId": child_session_str,
            "parentSessionId": parent_session_str,
            "title": title,
            "purpose": created_session.session_purpose.to_string()
        });
        if let Some(ref prompt) = req.initial_prompt {
            event_payload["initialPrompt"] = serde_json::json!(prompt);
        }
        let _ = app_handle.emit("ideation:child_session_created", event_payload);
    }

    let team_config = created_session
        .team_config_json
        .as_ref()
        .and_then(|json_str| serde_json::from_str(json_str).ok());

    Ok(Json(CreateChildSessionResponse {
        session_id: child_session_str,
        parent_session_id: parent_session_str,
        title,
        status: created_session.status.to_string(),
        created_at: created_session.created_at.to_rfc3339(),
        inherited_plan_id: created_session.inherited_plan_artifact_id.map(|id| id.to_string()),
        initial_prompt: req.initial_prompt.clone(),
        parent_context,
        orchestration_triggered,
        team_mode: created_session.team_mode.clone(),
        team_config,
        generation: verification_generation,
    }))
}
