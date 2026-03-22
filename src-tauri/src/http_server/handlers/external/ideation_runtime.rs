use super::*;
use super::ideation_start::{build_chat_service, determine_agent_status};

#[derive(Debug, Deserialize)]
pub struct IdeationMessageRequest {
    pub session_id: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct IdeationMessageResponse {
    /// Delivery outcome: "sent" | "queued" | "spawned"
    pub status: String,
    pub session_id: String,
    pub next_action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}
// ============================================================================
// External apply_proposals endpoint (D5 — closes external MCP bypass gap)
// ============================================================================

/// Request body for `POST /api/external/apply_proposals`.
///
/// Maps to [`ApplyProposalsInput`] used by the Tauri IPC path. The `target_column`
/// defaults to `"auto"` so task status is determined from dependency graph automatically.
#[derive(Debug, Deserialize)]
pub struct ExternalApplyProposalsRequest {
    pub session_id: String,
    pub proposal_ids: Vec<String>,
    /// Controls initial task placement. Use `"auto"` (default) to derive status from
    /// the dependency graph: tasks with no blockers → Ready, with blockers → Blocked.
    #[serde(default = "external_apply_default_column")]
    pub target_column: String,
    /// Per-plan override for feature branch usage. `None` uses the project default.
    #[serde(default)]
    pub use_feature_branch: Option<bool>,
    /// Per-plan override for the base branch. External callers can specify a custom branch;
    /// the backend validates it exists locally (see apply_proposals_core).
    #[serde(default)]
    pub base_branch_override: Option<String>,
}

fn external_apply_default_column() -> String {
    "auto".to_string()
}

impl From<ExternalApplyProposalsRequest> for ApplyProposalsInput {
    fn from(req: ExternalApplyProposalsRequest) -> Self {
        Self {
            session_id: req.session_id,
            proposal_ids: req.proposal_ids,
            target_column: req.target_column,
            use_feature_branch: req.use_feature_branch,
            base_branch_override: req.base_branch_override,
        }
    }
}

/// Response body for `POST /api/external/apply_proposals`.
#[derive(Debug, Serialize)]
pub struct ExternalApplyProposalsResponse {
    pub created_task_ids: Vec<String>,
    /// Number of proposal-to-proposal dependency edges created (excludes merge task edges).
    pub dependencies_created: usize,
    /// Number of plan tasks created (excludes the auto-generated merge task).
    pub tasks_created: usize,
    /// Human-readable summary of the finalization result.
    pub message: Option<String>,
    pub warnings: Vec<String>,
    pub session_converted: bool,
    pub execution_plan_id: Option<String>,
}

/// POST /api/external/apply_proposals
///
/// Apply accepted proposals to the Kanban board from the external MCP path.
///
/// Enforces:
/// 1. **Project scope** — the caller's API key must have access to the session's project.
/// 2. **Verification gate** — the plan must pass `check_verification_gate` before
///    proposals are accepted. Full enforcement requires Wave 1 schema migration.
///
/// Unlike the Tauri IPC path (`apply_proposals_to_kanban`), this endpoint does **not**
/// trigger the task scheduler. External agents poll
/// `GET /api/external/pipeline/:project_id` to monitor when tasks become Ready.
pub async fn external_apply_proposals(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Json(req): Json<ExternalApplyProposalsRequest>,
) -> Result<Json<ExternalApplyProposalsResponse>, HttpError> {
    let session_id = IdeationSessionId::from_string(req.session_id.clone());

    // Fetch session to verify project scope and verification gate
    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|e| {
            error!("Failed to get session {}: {}", req.session_id, e);
            HttpError::from(StatusCode::INTERNAL_SERVER_ERROR)
        })?
        .ok_or_else(|| HttpError::from(StatusCode::NOT_FOUND))?;

    // Enforce project scope: API key must have access to session's project
    session.assert_project_scope(&scope)?;

    // Enforce verification gate: plan must be verified before external acceptance
    let ideation_settings = state
        .app_state
        .ideation_settings_repo
        .get_settings()
        .await
        .map_err(|e| {
            error!("Failed to get ideation settings: {}", e);
            HttpError::from(StatusCode::INTERNAL_SERVER_ERROR)
        })?;
    check_verification_gate(&session, &ideation_settings)
        .map_err(|e| HttpError::validation(e.to_string()))?;

    // Apply proposals — no scheduler trigger (external agents poll get_pipeline_overview)
    let result = apply_proposals_core(&state.app_state, req.into())
        .await
        .map_err(|e| {
            error!("apply_proposals_core failed: {}", e);
            HttpError::validation(e.to_string())
        })?;

    // IPR cleanup — stop the ideation session's interactive CLI process (if any)
    if result.session_converted {
        let task_cleanup = TaskCleanupService::new(
            Arc::clone(&state.app_state.task_repo),
            Arc::clone(&state.app_state.project_repo),
            Arc::clone(&state.app_state.running_agent_registry),
            None, // No AppHandle in HTTP context
        )
        .with_interactive_process_registry(Arc::clone(
            &state.app_state.interactive_process_registry,
        ));

        let stopped = task_cleanup
            .stop_ideation_session_agent(&result.session_id)
            .await;
        if !stopped {
            tracing::warn!(
                session_id = %result.session_id,
                "IPR cleanup: no running process found for accepted session (HTTP path)"
            );
        }
    }

    tracing::info!(
        session_id = %session_id.as_str(),
        created = result.created_task_ids.len(),
        "External apply_proposals completed"
    );

    Ok(Json(ExternalApplyProposalsResponse {
        created_task_ids: result.created_task_ids,
        dependencies_created: result.dependencies_created,
        tasks_created: result.tasks_created,
        message: result.message,
        warnings: result.warnings,
        session_converted: result.session_converted,
        execution_plan_id: result.execution_plan_id,
    }))
}

/// POST /api/external/ideation_message
/// Send a message to an active ideation session.
///
/// Tri-state delivery:
/// 1. "sent"    — interactive process is open; message written directly to stdin
/// 2. "queued"  — agent is running but has no open stdin; message queued for resume
/// 3. "spawned" — no agent running; new agent process is spawned with the message
pub async fn ideation_message_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Json(req): Json<IdeationMessageRequest>,
) -> Result<Json<IdeationMessageResponse>, (StatusCode, Json<serde_json::Value>)> {
    let session_id = IdeationSessionId::from_string(req.session_id.clone());

    // Validate session exists
    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|e| {
            error!("Failed to get ideation session {}: {}", session_id.as_str(), e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Failed to get ideation session"})))
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Session not found"}))))?;

    // Enforce project scope
    session.assert_project_scope(&scope).map_err(|e| {
        (e.status, Json(serde_json::json!({"error": e.message.unwrap_or_default()})))
    })?;

    // Enforce Active status
    if session.status != crate::domain::entities::ideation::IdeationSessionStatus::Active {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Session is not active"}))));
    }

    let session_id_str = session_id.as_str().to_string();

    // Capture current phase for fire-and-forget transition logic later
    let current_phase = session.external_activity_phase.clone();

    // Helper: fire-and-forget 'created' → 'planning' phase transition
    let maybe_transition_to_planning = |repo: Arc<dyn crate::domain::repositories::IdeationSessionRepository>, sid: IdeationSessionId, phase: Option<String>| {
        if phase.as_deref() == Some("created") {
            tokio::spawn(async move {
                if let Err(e) = repo.update_external_activity_phase(&sid, "planning").await {
                    error!("Failed to set activity phase 'planning' for session {}: {}", sid.as_str(), e);
                }
            });
        }
    };

    // Read-before-write guard: external sessions must read agent responses before sending
    if session.origin == SessionOrigin::External {
        let last_read = session.external_last_read_message_id.as_deref();
        match state
            .app_state
            .chat_message_repo
            .count_unread_assistant_messages(&session_id_str, last_read)
            .await
        {
            Ok(unread_count) if unread_count > 0 => {
                return Err((
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({
                        "error": "unread_messages",
                        "unread_count": unread_count,
                        "hint": format!(
                            "You have {} unread agent response(s). Call v1_get_ideation_messages to read them before sending another message.",
                            unread_count
                        ),
                        "next_action": "fetch_messages"
                    })),
                ));
            }
            Ok(_) => {} // No unread messages, allow through
            Err(e) => {
                error!(
                    "Failed to count unread messages for session {}: {}",
                    session_id_str, e
                );
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "Failed to check message read status"})),
                ));
            }
        }
    }

    // Try 1: write directly to open interactive process (agent in multi-turn mode)
    let ipr_key = crate::application::InteractiveProcessKey {
        context_type: "ideation".to_string(),
        context_id: session_id_str.clone(),
    };
    if state
        .app_state
        .interactive_process_registry
        .has_process(&ipr_key)
        .await
    {
        let stream_json_message = crate::http_server::handlers::format_interactive_stdin_message(
            ChatContextType::Ideation,
            &session_id_str,
            &req.message,
        );
        match state
            .app_state
            .interactive_process_registry
            .write_message(&ipr_key, &stream_json_message)
            .await
        {
            Ok(()) => {
                maybe_transition_to_planning(
                    Arc::clone(&state.app_state.ideation_session_repo),
                    IdeationSessionId::from_string(session_id_str.clone()),
                    current_phase,
                );
                return Ok(Json(IdeationMessageResponse {
                    status: "sent".to_string(),
                    session_id: session_id_str,
                    next_action: "poll_status".to_string(),
                    hint: Some("Wait for agent to respond. Poll v1_get_ideation_status (5-10s interval)".to_string()),
                }));
            }
            Err(e) => {
                // Process may have closed between has_process and write_message; fall through
                error!(
                    "Failed to write to interactive process for session {}: {}",
                    session_id_str, e
                );
            }
        }
    }

    // Try 2: queue message if agent is running (will be delivered on next resume)
    let agent_key =
        crate::domain::services::running_agent_registry::RunningAgentKey::new("ideation", &session_id_str);
    if state
        .app_state
        .running_agent_registry
        .is_running(&agent_key)
        .await
    {
        // Queue depth cap: prevent flooding when agent is busy (generating).
        // Bypass for "sent" (interactive process) and "spawned" (no agent) since those deliver immediately.
        let cap = crate::infrastructure::agents::claude::external_mcp_config()
            .external_message_queue_cap as usize;
        let queued_count = state
            .app_state
            .message_queue
            .count_for_context("ideation", &session_id_str);
        if queued_count >= cap {
            return Err((
                axum::http::StatusCode::TOO_MANY_REQUESTS,
                Json(serde_json::json!({
                    "error": "queue_full",
                    "queued_count": queued_count,
                    "hint": format!(
                        "Message queue is full ({queued_count} pending). Wait for the agent to process messages. Poll v1_get_ideation_status."
                    ),
                    "next_action": "poll_status"
                })),
            ));
        }

        state
            .app_state
            .message_queue
            .queue(ChatContextType::Ideation, &session_id_str, req.message.clone());
        maybe_transition_to_planning(
            Arc::clone(&state.app_state.ideation_session_repo),
            IdeationSessionId::from_string(session_id_str.clone()),
            current_phase,
        );
        return Ok(Json(IdeationMessageResponse {
            status: "queued".to_string(),
            session_id: session_id_str,
            next_action: "poll_status".to_string(),
            hint: Some("Wait for agent to respond. Poll v1_get_ideation_status (5-10s interval)".to_string()),
        }));
    }

    // Try 3: spawn a new agent
    let chat_service = build_chat_service(&state.app_state, &state.execution_state);

    let send_result = chat_service
        .send_message(
            ChatContextType::Ideation,
            &session_id_str,
            &req.message,
            SendMessageOptions {
                is_external_mcp: true,
                ..Default::default()
            },
        )
        .await;

    match send_result {
        Ok(result) if result.was_queued => {
            maybe_transition_to_planning(
                Arc::clone(&state.app_state.ideation_session_repo),
                IdeationSessionId::from_string(session_id_str.clone()),
                current_phase,
            );
            return Ok(Json(IdeationMessageResponse {
                status: "queued".to_string(),
                session_id: session_id_str,
                next_action: "poll_status".to_string(),
                hint: Some("Wait for agent to respond. Poll v1_get_ideation_status (5-10s interval)".to_string()),
            }));
        }
        Ok(_) => {}
        Err(e) => {
            error!("Failed to send message to ideation session {}: {}", session_id_str, e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Failed to send message"}))));
        }
    }

    maybe_transition_to_planning(
        Arc::clone(&state.app_state.ideation_session_repo),
        IdeationSessionId::from_string(session_id_str.clone()),
        current_phase,
    );
    Ok(Json(IdeationMessageResponse {
        status: "spawned".to_string(),
        session_id: session_id_str,
        next_action: "poll_status".to_string(),
        hint: Some("Wait for agent to respond. Poll v1_get_ideation_status (5-10s interval)".to_string()),
    }))
}

// ============================================================================
// trigger_verification_http + get_plan_verification_external_http
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct TriggerVerificationRequest {
    pub session_id: String,
}

#[derive(Debug, Serialize)]
pub struct TriggerVerificationResponse {
    pub status: String, // "triggered" | "already_running" | "no_plan"
    pub session_id: String,
}

/// A single verification gap in the external API response
#[derive(Debug, Serialize)]
pub struct ExternalGapDetail {
    pub severity: String,
    pub category: String,
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct ExternalVerificationResponse {
    pub status: String,
    pub in_progress: bool,
    pub round: Option<u32>,
    pub max_rounds: Option<u32>,
    pub gap_count: Option<u32>,
    pub gap_score: Option<u32>,
    #[serde(default)]
    pub gaps: Vec<ExternalGapDetail>,
    pub convergence_reason: Option<String>,
}

/// POST /api/external/trigger_verification
pub async fn trigger_verification_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Json(req): Json<TriggerVerificationRequest>,
) -> Result<Json<TriggerVerificationResponse>, StatusCode> {
    use crate::infrastructure::sqlite::sqlite_ideation_session_repo::SqliteIdeationSessionRepository as SessionRepo;

    let session_id = req.session_id.clone();
    let session_id_obj = IdeationSessionId::from_string(session_id.clone());

    // Load session
    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id_obj)
        .await
        .map_err(|e| {
            error!("Failed to load session {}: {}", session_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Scope check
    session.assert_project_scope(&scope).map_err(|e| e.status)?;

    // No-plan check: neither own plan nor inherited
    if session.plan_artifact_id.is_none() && session.inherited_plan_artifact_id.is_none() {
        return Ok(Json(TriggerVerificationResponse {
            status: "no_plan".to_string(),
            session_id,
        }));
    }

    // CAS: atomically trigger auto_verify_sync
    let sid_for_trigger = session_id.clone();
    let generation_opt = state
        .app_state
        .db
        .run(move |conn| SessionRepo::trigger_auto_verify_sync(conn, &sid_for_trigger))
        .await
        .map_err(|e| {
            error!("trigger_auto_verify_sync failed for session {}: {}", session_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let Some(generation) = generation_opt else {
        return Ok(Json(TriggerVerificationResponse {
            status: "already_running".to_string(),
            session_id,
        }));
    };

    // Spawn verifier; reset on failure
    let cfg = verification_config();
    if let Some(app_handle) = &state.app_state.app_handle {
        emit_verification_started(app_handle, &session_id, generation, cfg.max_rounds);
    }
    let title = format!("Auto-verification (gen {generation})");
    let description = format!(
        "Run verification round loop. parent_session_id: {session_id}, generation: {generation}, max_rounds: {}",
        cfg.max_rounds
    );
    match crate::http_server::handlers::session_linking::create_verification_child_session(
        &state,
        &session_id,
        &description,
        &title,
    )
    .await
    {
        Ok(true) => {} // orchestration triggered — success
        Ok(false) | Err(_) => {
            error!(
                "Verification agent failed to spawn for session {}",
                session_id
            );
            let sid_reset = session_id.clone();
            if let Err(reset_err) = state
                .app_state
                .db
                .run(move |conn| SessionRepo::reset_auto_verify_sync(conn, &sid_reset))
                .await
            {
                error!(
                    "Failed to reset auto-verify state for session {} after spawn failure: {}",
                    session_id, reset_err
                );
            } else if let Some(app_handle) = &state.app_state.app_handle {
                emit_verification_status_changed(
                    app_handle,
                    &session_id,
                    crate::domain::entities::VerificationStatus::Unverified,
                    false,
                    None,
                    Some("spawn_failed"),
                    Some(generation),
                );
            }
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    // Transition external activity phase to "verifying"
    {
        let repo = Arc::clone(&state.app_state.ideation_session_repo);
        let trigger_session_id = IdeationSessionId::from_string(session_id.clone());
        tokio::spawn(async move {
            if let Err(e) = repo.update_external_activity_phase(&trigger_session_id, "verifying").await {
                error!("Failed to set activity phase 'verifying' for session {}: {}", trigger_session_id.as_str(), e);
            }
        });
    }

    Ok(Json(TriggerVerificationResponse {
        status: "triggered".to_string(),
        session_id,
    }))
}

/// GET /api/external/plan_verification/:session_id
pub async fn get_plan_verification_external_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Path(session_id): Path<String>,
) -> Result<Json<ExternalVerificationResponse>, StatusCode> {
    use crate::domain::entities::ideation::VerificationMetadata;
    use crate::domain::services::gap_score;

    let session_id_obj = IdeationSessionId::from_string(session_id.clone());

    // Load session for scope check
    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id_obj)
        .await
        .map_err(|e| {
            error!("Failed to load session {}: {}", session_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Scope check
    session.assert_project_scope(&scope).map_err(|e| e.status)?;

    // Read verification state from session entity
    let status_str = session.verification_status.to_string();
    let in_progress = session.verification_in_progress;

    let metadata: Option<VerificationMetadata> = session
        .verification_metadata
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    let round = metadata
        .as_ref()
        .and_then(|m| if m.current_round > 0 { Some(m.current_round) } else { None });
    let max_rounds = metadata
        .as_ref()
        .and_then(|m| if m.max_rounds > 0 { Some(m.max_rounds) } else { None });
    let gap_count = metadata.as_ref().map(|m| gap_score(&m.current_gaps));
    let convergence_reason = metadata.as_ref().and_then(|m| m.convergence_reason.clone());
    let gaps: Vec<ExternalGapDetail> = metadata
        .as_ref()
        .map(|m| {
            m.current_gaps
                .iter()
                .map(|g| ExternalGapDetail {
                    severity: g.severity.clone(),
                    category: g.category.clone(),
                    description: g.description.clone(),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(Json(ExternalVerificationResponse {
        status: status_str,
        in_progress,
        round,
        max_rounds,
        gap_count,
        gap_score: gap_count,
        gaps,
        convergence_reason,
    }))
}

// ============================================================================
// Get ideation messages
// ============================================================================

/// A single message returned to external consumers.
#[derive(Debug, Serialize)]
pub struct IdeationMessageSummary {
    pub id: String,
    /// "user" or "assistant" (Orchestrator is mapped to "assistant")
    pub role: String,
    pub content: String,
    pub created_at: String,
}

/// Response for GET /api/external/ideation_messages/:session_id
#[derive(Debug, Serialize)]
pub struct GetIdeationMessagesResponse {
    pub messages: Vec<IdeationMessageSummary>,
    pub has_more: bool,
    /// "idle" | "generating" | "waiting_for_input"
    pub agent_status: String,
    pub next_action: String,
}

/// Query params for pagination.
#[derive(Debug, Deserialize)]
pub struct GetIdeationMessagesQuery {
    #[serde(default = "default_messages_limit")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
}

fn default_messages_limit() -> u32 {
    50
}

/// GET /api/external/ideation_messages/:session_id
///
/// Returns orchestrator and user messages for an ideation session.
/// Filter: User + Orchestrator roles only (Orchestrator → "assistant").
pub async fn get_ideation_messages_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Path(session_id): Path<String>,
    Query(params): Query<GetIdeationMessagesQuery>,
) -> Result<Json<GetIdeationMessagesResponse>, StatusCode> {
    use crate::domain::entities::ideation::MessageRole;

    let session_id = IdeationSessionId::from_string(session_id);

    // Load session and enforce scope
    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|e| {
            error!("Failed to get ideation session {}: {}", session_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    session.assert_project_scope(&scope).map_err(|e| e.status)?;

    // Fetch limit+1 to detect has_more (SQL already filters User + Orchestrator roles)
    let fetch_limit = params.limit.saturating_add(1);
    let raw_messages = state
        .app_state
        .chat_message_repo
        .get_recent_by_session_paginated(&session_id, fetch_limit, params.offset)
        .await
        .map_err(|e| {
            error!("Failed to get messages for session {}: {}", session_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Determine has_more before truncating
    let has_more = raw_messages.len() > params.limit as usize;
    let messages_slice = if has_more {
        &raw_messages[..params.limit as usize]
    } else {
        &raw_messages[..]
    };

    // Role filter — User and Orchestrator only (SQL already does this, but be defensive)
    let messages: Vec<IdeationMessageSummary> = messages_slice
        .iter()
        .filter(|msg| matches!(msg.role, MessageRole::User | MessageRole::Orchestrator))
        .map(|msg| {
            let role = match msg.role {
                MessageRole::Orchestrator => "assistant".to_string(),
                _ => "user".to_string(),
            };
            IdeationMessageSummary {
                id: msg.id.to_string(),
                role,
                content: msg.content.clone(),
                created_at: msg.created_at.to_rfc3339(),
            }
        })
        .collect();

    // Fire-and-forget: update read cursor for external sessions after fetching messages
    if session.origin == SessionOrigin::External {
        if let Some(latest_msg) = messages.last() {
            let latest_id = latest_msg.id.clone();
            if let Err(e) = state
                .app_state
                .ideation_session_repo
                .update_external_last_read_message_id(&session_id, &latest_id)
                .await
            {
                error!(
                    "Failed to update external_last_read_message_id for session {}: {}",
                    session_id.as_str(),
                    e
                );
            }
        }
    }

    // Determine agent tri-state status
    let agent_status = determine_agent_status(
        state.app_state.running_agent_registry.as_ref(),
        &state.app_state.interactive_process_registry,
        session_id.as_str(),
    )
    .await;

    let next_action = match agent_status.as_str() {
        "waiting_for_input" => "send_message".to_string(),
        "generating" => "wait".to_string(),
        _ => "send_message".to_string(),
    };

    Ok(Json(GetIdeationMessagesResponse {
        messages,
        has_more,
        agent_status,
        next_action,
    }))
}
