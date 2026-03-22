use super::*;

#[derive(Debug, Deserialize)]
pub struct StartIdeationRequest {
    pub project_id: String,
    pub title: Option<String>,
    pub prompt: Option<String>,
    pub initial_prompt: Option<String>,
    pub idempotency_key: Option<String>,
}

/// Lightweight summary of an active external session for dedup awareness.
#[derive(Debug, Serialize, Clone)]
pub struct ExternalSessionSummary {
    pub session_id: String,
    pub title: Option<String>,
    pub status: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_activity_phase: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StartIdeationResponse {
    pub session_id: String,
    pub status: String,
    pub agent_spawned: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_spawn_blocked_reason: Option<String>,
    /// All active external sessions for the project (for agent visibility)
    pub existing_active_sessions: Vec<ExternalSessionSummary>,
    /// True if this response reuses an existing session due to idempotency key match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exists: Option<bool>,
    /// True if this response reuses an existing session due to Jaccard similarity match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplicate_detected: Option<bool>,
    /// Jaccard similarity score when duplicate_detected is true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similarity_score: Option<f64>,
    /// Behavioral hint for the caller
    pub next_action: String,
    /// Human-readable hint message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

/// POST /api/external/start_ideation
/// Create a new ideation session for a project.
pub async fn start_ideation_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    headers: HeaderMap,
    Json(req): Json<StartIdeationRequest>,
) -> Result<Json<StartIdeationResponse>, HttpError> {
    let project_id = ProjectId::from_string(req.project_id.clone());

    let project = state
        .app_state
        .project_repo
        .get_by_id(&project_id)
        .await
        .map_err(|e| {
            error!("Failed to get project {}: {}", project_id.as_str(), e);
            HttpError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: Some("Failed to get project".to_string()),
            }
        })?
        .ok_or(HttpError {
            status: StatusCode::NOT_FOUND,
            message: Some("Project not found".to_string()),
        })?;

    project.assert_project_scope(&scope).map_err(|e| HttpError {
        status: e.status,
        message: e.message,
    })?;

    let api_key_id = headers
        .get(crate::http_server::handlers::external_auth::EXTERNAL_KEY_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    if let (Some(ref key_id), Some(ref idem_key)) = (&api_key_id, &req.idempotency_key) {
        if let Ok(Some(existing)) = state
            .app_state
            .ideation_session_repo
            .get_by_idempotency_key(key_id, idem_key)
            .await
        {
            let active_sessions = state
                .app_state
                .ideation_session_repo
                .list_active_external_by_project(&project_id)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|s| ExternalSessionSummary {
                    session_id: s.id.to_string(),
                    title: s.title.clone(),
                    status: s.status.to_string(),
                    created_at: s.created_at.to_rfc3339(),
                    external_activity_phase: s.external_activity_phase.clone(),
                })
                .collect::<Vec<_>>();
            return Ok(Json(StartIdeationResponse {
                session_id: existing.id.to_string(),
                status: existing.status.to_string(),
                agent_spawned: false,
                agent_spawn_blocked_reason: None,
                existing_active_sessions: active_sessions,
                exists: Some(true),
                duplicate_detected: None,
                similarity_score: None,
                next_action: "poll_status".to_string(),
                hint: Some("Idempotent retry: returning existing session.".to_string()),
            }));
        }
    }

    let active_sessions = state
        .app_state
        .ideation_session_repo
        .list_active_external_by_project(&project_id)
        .await
        .unwrap_or_default();

    let effective_prompt = req.prompt.clone().or_else(|| req.initial_prompt.clone());
    let has_candidate_text = req.prompt.is_some() || req.title.is_some();

    if has_candidate_text && !active_sessions.is_empty() {
        let candidate_text = format!(
            "{} {}",
            req.prompt.as_deref().unwrap_or(""),
            req.title.as_deref().unwrap_or("")
        );
        let candidate_tokens = tokenize_for_similarity(&candidate_text);
        let similarity_threshold =
            crate::infrastructure::agents::claude::external_mcp_config()
                .external_session_similarity_threshold;

        let mut best_match: Option<(f64, &crate::domain::entities::ideation::IdeationSession)> =
            None;
        for session in &active_sessions {
            let session_title = session.title.as_deref().unwrap_or("");
            let first_msg = state
                .app_state
                .chat_message_repo
                .get_first_user_message_by_context("ideation", session.id.as_str())
                .await
                .unwrap_or_default()
                .unwrap_or_default();
            let comparison_text = format!("{} {}", session_title, first_msg);
            let comparison_tokens = tokenize_for_similarity(&comparison_text);
            let score = jaccard_similarity(&candidate_tokens, &comparison_tokens);
            if score >= similarity_threshold && best_match.map(|(s, _)| score > s).unwrap_or(true) {
                best_match = Some((score, session));
            }
        }

        if let Some((score, matched_session)) = best_match {
            let active_summaries = active_sessions
                .iter()
                .map(|s| ExternalSessionSummary {
                    session_id: s.id.to_string(),
                    title: s.title.clone(),
                    status: s.status.to_string(),
                    created_at: s.created_at.to_rfc3339(),
                    external_activity_phase: s.external_activity_phase.clone(),
                })
                .collect::<Vec<_>>();
            let hint_msg = format!(
                "A similar session already exists ('{}', {:.0}% match). Reusing it instead of creating a duplicate.",
                matched_session.title.as_deref().unwrap_or("untitled"),
                score * 100.0
            );
            return Ok(Json(StartIdeationResponse {
                session_id: matched_session.id.to_string(),
                status: matched_session.status.to_string(),
                agent_spawned: false,
                agent_spawn_blocked_reason: None,
                existing_active_sessions: active_summaries,
                exists: None,
                duplicate_detected: Some(true),
                similarity_score: Some(score),
                next_action: "use_existing_session".to_string(),
                hint: Some(hint_msg),
            }));
        }
    }

    let mut session_builder = match req.title.clone() {
        None => IdeationSession::new(project_id.clone()),
        Some(t) => IdeationSession::new_with_title(project_id.clone(), t),
    };
    session_builder.origin = SessionOrigin::External;
    session_builder.external_activity_phase = Some("created".to_string());
    if let Some(ref key_id) = api_key_id {
        session_builder.api_key_id = Some(key_id.clone());
    }
    if let Some(ref idem_key) = req.idempotency_key {
        session_builder.idempotency_key = Some(idem_key.clone());
    }
    let created = match state.app_state.ideation_session_repo.create(session_builder).await {
        Ok(session) => session,
        Err(e)
            if e.to_string().to_lowercase().contains(SQLITE_UNIQUE_VIOLATION)
                && api_key_id.is_some()
                && req.idempotency_key.is_some() =>
        {
            if let (Some(ref key_id), Some(ref idem_key)) = (&api_key_id, &req.idempotency_key) {
                if let Ok(Some(existing)) = state
                    .app_state
                    .ideation_session_repo
                    .get_by_idempotency_key(key_id, idem_key)
                    .await
                {
                    let active_summaries = active_sessions
                        .iter()
                        .map(|s| ExternalSessionSummary {
                            session_id: s.id.to_string(),
                            title: s.title.clone(),
                            status: s.status.to_string(),
                            created_at: s.created_at.to_rfc3339(),
                            external_activity_phase: s.external_activity_phase.clone(),
                        })
                        .collect::<Vec<_>>();
                    return Ok(Json(StartIdeationResponse {
                        session_id: existing.id.to_string(),
                        status: existing.status.to_string(),
                        agent_spawned: false,
                        agent_spawn_blocked_reason: None,
                        existing_active_sessions: active_summaries,
                        exists: Some(true),
                        duplicate_detected: None,
                        similarity_score: None,
                        next_action: "poll_status".to_string(),
                        hint: Some(
                            "Idempotent retry (concurrent): returning existing session."
                                .to_string(),
                        ),
                    }));
                }
            }
            error!(
                "Failed to create ideation session (unique conflict, re-query failed): {}",
                e
            );
            return Err(HttpError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: Some("Failed to create ideation session".to_string()),
            });
        }
        Err(e) => {
            error!("Failed to create ideation session: {}", e);
            return Err(HttpError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: Some("Failed to create ideation session".to_string()),
            });
        }
    };

    let session_id_str = created.id.to_string();

    {
        let repo = Arc::clone(&state.app_state.ideation_session_repo);
        let sid = IdeationSessionId::from_string(session_id_str.clone());
        tokio::spawn(async move {
            if let Err(e) = repo.update_external_activity_phase(&sid, "created").await {
                error!(
                    "Failed to set activity phase 'created' for session {}: {}",
                    sid.as_str(),
                    e
                );
            }
        });
    }

    let session_created_payload = serde_json::json!({
        "sessionId": session_id_str,
        "projectId": project_id.to_string(),
    });
    if let Some(ref handle) = state.app_state.app_handle {
        let _ = handle.emit("ideation:session_created", &session_created_payload);
    }

    if let Err(e) = state
        .app_state
        .external_events_repo
        .insert_event(
            "ideation:session_created",
            &project_id.to_string(),
            &session_created_payload.to_string(),
        )
        .await
    {
        tracing::warn!(error = %e, "Failed to persist IdeationSessionCreated event");
    }

    if let Some(ref publisher) = state.app_state.webhook_publisher {
        let _ = publisher
            .publish(
                EventType::IdeationSessionCreated,
                &project_id.to_string(),
                session_created_payload,
            )
            .await;
    }

    let existing_summaries = {
        let mut summaries: Vec<ExternalSessionSummary> = active_sessions
            .iter()
            .map(|s| ExternalSessionSummary {
                session_id: s.id.to_string(),
                title: s.title.clone(),
                status: s.status.to_string(),
                created_at: s.created_at.to_rfc3339(),
                external_activity_phase: s.external_activity_phase.clone(),
            })
            .collect();
        summaries.insert(
            0,
            ExternalSessionSummary {
                session_id: session_id_str.clone(),
                title: created.title.clone(),
                status: created.status.to_string(),
                created_at: created.created_at.to_rfc3339(),
                external_activity_phase: created.external_activity_phase.clone(),
            },
        );
        summaries
    };

    let mut agent_spawned = false;
    let mut agent_spawn_blocked_reason: Option<String> = None;
    if let Some(ref prompt_str) = effective_prompt {
        let chat_service = build_chat_service(&state.app_state, &state.execution_state);

        match chat_service
            .send_message(
                ChatContextType::Ideation,
                &session_id_str,
                prompt_str,
                SendMessageOptions {
                    is_external_mcp: true,
                    ..Default::default()
                },
            )
            .await
        {
            Ok(result) if result.was_queued => {
                agent_spawned = true;
            }
            Ok(_) => {
                agent_spawned = true;
                spawn_session_namer(
                    Arc::clone(&state.app_state.agent_client),
                    session_id_str.clone(),
                    prompt_str.clone(),
                );
            }
            Err(e) => {
                error!(
                    "Failed to auto-spawn agent on external ideation session {}: {}",
                    session_id_str, e
                );
                agent_spawn_blocked_reason = Some(e.to_string());
            }
        }
    }

    Ok(Json(StartIdeationResponse {
        session_id: session_id_str,
        status: "ideating".to_string(),
        agent_spawned,
        agent_spawn_blocked_reason,
        existing_active_sessions: existing_summaries,
        exists: None,
        duplicate_detected: None,
        similarity_score: None,
        next_action: "poll_status".to_string(),
        hint: Some("Poll v1_get_ideation_status to track agent progress.".to_string()),
    }))
}
