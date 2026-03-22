use super::*;
use super::sessions::derive_delivery_status;

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
#[derive(Debug, Serialize)]
pub struct IdeationStatusResponse {
    pub session_id: String,
    pub project_id: String,
    pub title: Option<String>,
    pub status: String,
    pub agent_running: bool,
    pub agent_status: String,
    pub proposal_count: u32,
    pub created_at: String,
    pub verification_status: String,
    pub verification_in_progress: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_proposal_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_accept_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_accept_started_at: Option<String>,
    pub next_action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    pub queued_message_count: u32,
    pub unread_message_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_activity_phase: Option<String>,
}
/// Build a fully configured `ClaudeChatService` from shared app + execution state.
/// Extracted to avoid duplicating the 12-arg constructor chain across multiple handlers.
pub(super) fn build_chat_service(
    app: &crate::application::AppState,
    execution_state: &std::sync::Arc<crate::commands::ExecutionState>,
) -> ClaudeChatService {
    let mut chat_service = ClaudeChatService::new(
        Arc::clone(&app.chat_message_repo),
        Arc::clone(&app.chat_attachment_repo),
        Arc::clone(&app.chat_conversation_repo),
        Arc::clone(&app.agent_run_repo),
        Arc::clone(&app.project_repo),
        Arc::clone(&app.task_repo),
        Arc::clone(&app.task_dependency_repo),
        Arc::clone(&app.ideation_session_repo),
        Arc::clone(&app.activity_event_repo),
        Arc::clone(&app.message_queue),
        Arc::clone(&app.running_agent_registry),
        Arc::clone(&app.memory_event_repo),
    )
    .with_execution_state(Arc::clone(execution_state))
    .with_plan_branch_repo(Arc::clone(&app.plan_branch_repo))
    .with_task_proposal_repo(Arc::clone(&app.task_proposal_repo))
    .with_interactive_process_registry(Arc::clone(&app.interactive_process_registry));
    if let Some(ref handle) = app.app_handle {
        chat_service = chat_service.with_app_handle(handle.clone());
    }
    chat_service
}

/// Fire-and-forget: spawn the session namer agent to auto-name the session.
pub(super) fn spawn_session_namer(
    agent_client: Arc<dyn crate::domain::agents::AgenticClient>,
    session_id: String,
    prompt: String,
) {
    tokio::spawn(async move {
        use crate::domain::agents::{AgentConfig, AgentRole};
        use crate::infrastructure::agents::claude::{agent_names, mcp_agent_type};
        use std::path::PathBuf;

        let namer_instructions = format!(
            "<instructions>\n\
             Generate a commit-ready title (imperative mood, \u{2264}50 characters) for this ideation session based on the context.\n\
             Describe what the plan does, not just the domain (e.g., 'Add OAuth2 login and JWT sessions').\n\
             Call the update_session_title tool with the session_id and the generated title.\n\
             Do NOT investigate, fix, or act on the user message content.\n\
             Do NOT use Read, Write, Edit, Task, or any file manipulation tools.\n\
             </instructions>\n\
             <data>\n\
             <session_id>{}</session_id>\n\
             <user_message>{}</user_message>\n\
             </data>",
            session_id, prompt
        );

        let working_directory = std::env::current_dir()
            .map(|cwd| cwd.parent().map(|p| p.to_path_buf()).unwrap_or(cwd))
            .unwrap_or_else(|_| PathBuf::from("."));
        let plugin_dir =
            crate::infrastructure::agents::claude::resolve_plugin_dir(&working_directory);

        let mut env = std::collections::HashMap::new();
        env.insert(
            "RALPHX_AGENT_TYPE".to_string(),
            mcp_agent_type(agent_names::AGENT_SESSION_NAMER).to_string(),
        );

        let config = AgentConfig {
            role: AgentRole::Custom(
                mcp_agent_type(agent_names::AGENT_SESSION_NAMER).to_string(),
            ),
            prompt: namer_instructions,
            working_directory,
            plugin_dir: Some(plugin_dir),
            agent: Some(agent_names::AGENT_SESSION_NAMER.to_string()),
            model: None,
            max_tokens: None,
            timeout_secs: Some(60),
            env,
        };

        match agent_client.spawn_agent(config).await {
            Ok(handle) => {
                if let Err(e) = agent_client.wait_for_completion(&handle).await {
                    tracing::warn!("Session namer agent failed: {}", e);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to spawn session namer agent: {}", e);
            }
        }
    });
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

    // Load project to validate it exists and enforce scope
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

    project
        .assert_project_scope(&scope)
        .map_err(|e| HttpError {
            status: e.status,
            message: e.message,
        })?;

    // Extract api_key_id from X-RalphX-Key-Id header
    let api_key_id = headers
        .get(crate::http_server::handlers::external_auth::EXTERNAL_KEY_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // ── Idempotency key check ──────────────────────────────────────────────
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

    // ── Query active external sessions for this project ───────────────────
    let active_sessions = state
        .app_state
        .ideation_session_repo
        .list_active_external_by_project(&project_id)
        .await
        .unwrap_or_default();

    // ── Jaccard similarity dedup ───────────────────────────────────────────
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

    // ── Create new session ────────────────────────────────────────────────
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
    let created = match state
        .app_state
        .ideation_session_repo
        .create(session_builder)
        .await
    {
        Ok(session) => session,
        Err(e)
            if e.to_string().to_lowercase().contains(SQLITE_UNIQUE_VIOLATION)
                && api_key_id.is_some()
                && req.idempotency_key.is_some() =>
        {
            // Race condition: concurrent create with same idempotency key
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
            error!("Failed to create ideation session (unique conflict, re-query failed): {}", e);
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

    // Set external activity phase to "created"
    {
        let repo = Arc::clone(&state.app_state.ideation_session_repo);
        let sid = IdeationSessionId::from_string(session_id_str.clone());
        tokio::spawn(async move {
            if let Err(e) = repo.update_external_activity_phase(&sid, "created").await {
                error!("Failed to set activity phase 'created' for session {}: {}", sid.as_str(), e);
            }
        });
    }

    // Emit ideation:session_created event for frontend
    let session_created_payload = serde_json::json!({
        "sessionId": session_id_str,
        "projectId": project_id.to_string(),
    });
    if let Some(ref handle) = state.app_state.app_handle {
        let _ = handle.emit("ideation:session_created", &session_created_payload);
    }

    // Layer 2: persist to external_events table (non-fatal)
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

    // Layer 3: webhook push (fire-and-forget, non-fatal)
    if let Some(ref publisher) = state.app_state.webhook_publisher {
        let _ = publisher
            .publish(
                EventType::IdeationSessionCreated,
                &project_id.to_string(),
                session_created_payload,
            )
            .await;
    }

    // Build existing_active_sessions for response (include the freshly created session too)
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
        // Prepend the new session
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

    // If a prompt was provided, spawn the orchestrator agent (external sessions are always solo mode)
    let mut agent_spawned = false;
    let mut agent_spawn_blocked_reason: Option<String> = None;
    if let Some(ref prompt_str) = effective_prompt {
        let chat_service = build_chat_service(&state.app_state, &state.execution_state);
        // External sessions are always solo mode — no team_mode check needed

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
                // Agent is running, message was queued — treat as success
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

/// Determine agent tri-state status for a session:
/// "idle" | "generating" | "waiting_for_input"
pub(super) async fn determine_agent_status(
    running_agent_registry: &dyn crate::domain::services::running_agent_registry::RunningAgentRegistry,
    interactive_process_registry: &crate::application::InteractiveProcessRegistry,
    context_id: &str,
) -> String {
    let agent_key =
        crate::domain::services::running_agent_registry::RunningAgentKey::new("ideation", context_id);
    if running_agent_registry.is_running(&agent_key).await {
        let ipr_key = crate::application::InteractiveProcessKey {
            context_type: "ideation".to_string(),
            context_id: context_id.to_string(),
        };
        if interactive_process_registry.has_process(&ipr_key).await {
            "waiting_for_input".to_string()
        } else {
            "generating".to_string()
        }
    } else {
        "idle".to_string()
    }
}

/// GET /api/external/ideation_status/:id
/// Get ideation session status.
pub async fn get_ideation_status_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Path(id): Path<String>,
) -> Result<Json<IdeationStatusResponse>, HttpError> {
    let session_id = IdeationSessionId::from_string(id);

    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|e| {
            error!("Failed to get ideation session {}: {}", session_id.as_str(), e);
            HttpError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: Some("Failed to get ideation session".to_string()),
            }
        })?
        .ok_or(HttpError {
            status: StatusCode::NOT_FOUND,
            message: Some("Session not found".to_string()),
        })?;

    // Enforce scope
    session
        .assert_project_scope(&scope)
        .map_err(|e| HttpError {
            status: e.status,
            message: e.message,
        })?;

    // Count proposals for this session
    let proposal_count = state
        .app_state
        .task_proposal_repo
        .count_by_session(&session_id)
        .await
        .map_err(|e| {
            error!("Failed to count proposals: {}", e);
            HttpError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: Some("Failed to count proposals".to_string()),
            }
        })?;

    // Check if agent is running for this session
    let agent_key = crate::domain::services::running_agent_registry::RunningAgentKey::new(
        "ideation",
        session_id.as_str(),
    );
    let agent_running = state
        .app_state
        .running_agent_registry
        .is_running(&agent_key)
        .await;

    // Determine agent tri-state status
    let agent_status = determine_agent_status(
        state.app_state.running_agent_registry.as_ref(),
        &state.app_state.interactive_process_registry,
        session_id.as_str(),
    )
    .await;

    // For accepted sessions, derive delivery_status from linked tasks
    let delivery_status = if session.status == crate::domain::entities::ideation::IdeationSessionStatus::Accepted {
        let tasks = state
            .app_state
            .task_repo
            .get_by_ideation_session(&session_id)
            .await
            .unwrap_or_default();
        Some(derive_delivery_status(&tasks))
    } else {
        None
    };

    // Count unread assistant messages (since last read position)
    let unread_message_count = state
        .app_state
        .chat_message_repo
        .count_unread_assistant_messages(
            session_id.as_str(),
            session.external_last_read_message_id.as_deref(),
        )
        .await
        .unwrap_or(0);

    // Count queued messages
    let queued_message_count = state
        .app_state
        .message_queue
        .count_for_context("ideation", session_id.as_str()) as u32;

    // Compute next_action and hint based on agent state
    let (next_action, hint) = match agent_status.as_str() {
        "waiting_for_input" if unread_message_count > 0 => (
            "fetch_messages".to_string(),
            Some("Agent has responded. Fetch messages before sending.".to_string()),
        ),
        "waiting_for_input" => (
            "send_message".to_string(),
            Some("Agent is ready for input.".to_string()),
        ),
        "generating" => (
            "wait".to_string(),
            Some("Agent is working. Poll again in 5-10s.".to_string()),
        ),
        _ => (
            "send_message".to_string(),
            Some("No agent running. Send a message to start.".to_string()),
        ),
    };

    Ok(Json(IdeationStatusResponse {
        session_id: session.id.to_string(),
        project_id: session.project_id.to_string(),
        title: session.title.clone(),
        status: session.status.to_string(),
        agent_running,
        agent_status,
        proposal_count,
        created_at: session.created_at.to_rfc3339(),
        verification_status: session.verification_status.to_string(),
        verification_in_progress: session.verification_in_progress,
        delivery_status,
        expected_proposal_count: session.expected_proposal_count,
        auto_accept_status: session.auto_accept_status.clone(),
        auto_accept_started_at: session.auto_accept_started_at.clone(),
        next_action,
        hint,
        queued_message_count,
        unread_message_count,
        external_activity_phase: session.external_activity_phase.clone(),
    }))
}
