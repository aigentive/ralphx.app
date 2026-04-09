use super::*;
use crate::application::harness_runtime_registry::default_verification_config;

#[derive(Debug, Deserialize)]
pub struct TriggerVerificationRequest {
    pub session_id: String,
}

#[derive(Debug, Serialize)]
pub struct TriggerVerificationResponse {
    pub status: String,
    pub session_id: String,
}

/// A single verification gap in the external API response
#[derive(Debug, Serialize)]
pub struct ExternalGapDetail {
    pub severity: String,
    pub category: String,
    pub description: String,
}

/// Continuity context for the most recent verification child session (external API).
/// Defined independently from the internal VerificationChildInfo — no type sharing.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalVerificationChildInfo {
    /// Non-null only when in_progress=true and the child session is not archived
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_child_session_id: Option<String>,
    /// Always present when this block exists — the most recent child session ID
    pub latest_child_session_id: String,
    /// True when the latest child session is archived
    pub latest_child_archived: bool,
    /// updated_at timestamp of the latest child session (RFC3339)
    pub latest_child_updated_at: String,
    /// Inferred agent state: "likely_generating" | "likely_waiting" | "idle"
    pub agent_state: String,
    /// Deferred launch prompt waiting for capacity, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_initial_prompt: Option<String>,
    /// Last orchestrator message content truncated to 500 chars, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_assistant_message: Option<String>,
    /// Timestamp of the last orchestrator message (RFC3339), if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_assistant_message_at: Option<String>,
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
    /// Continuity context for the most recent verification child session, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_child: Option<ExternalVerificationChildInfo>,
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

    session.assert_project_scope(&scope).map_err(|e| e.status)?;

    if session.plan_artifact_id.is_none() && session.inherited_plan_artifact_id.is_none() {
        return Ok(Json(TriggerVerificationResponse {
            status: "no_plan".to_string(),
            session_id,
        }));
    }

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

    let cfg = default_verification_config();
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
        &[],
    )
    .await
    {
        Ok(true) => {}
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

    {
        let trigger_session_id = IdeationSessionId::from_string(session_id.clone());
        if let Err(e) = state
            .app_state
            .ideation_session_repo
            .update_external_activity_phase(&trigger_session_id, Some("verifying"))
            .await
        {
            error!(
                "Failed to set activity phase 'verifying' for session {}: {}",
                trigger_session_id.as_str(),
                e
            );
        }
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

    session.assert_project_scope(&scope).map_err(|e| e.status)?;

    let status_str = session.verification_status.to_string();
    let in_progress = session.verification_in_progress;

    let metadata: Option<VerificationMetadata> = session
        .verification_metadata
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    let round = metadata.as_ref().and_then(|m| {
        if m.current_round > 0 {
            Some(m.current_round)
        } else {
            None
        }
    });
    let max_rounds = metadata.as_ref().and_then(|m| {
        if m.max_rounds > 0 {
            Some(m.max_rounds)
        } else {
            None
        }
    });
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

    // Fetch verification child continuity data
    let verification_child = {
        use crate::domain::entities::IdeationSessionId as SessionId;
        use crate::domain::entities::ideation::IdeationSessionStatus;
        use crate::domain::services::running_agent_registry::RunningAgentKey;
        use crate::infrastructure::agents::claude::ideation_activity_threshold_secs;

        match state
            .app_state
            .ideation_session_repo
            .get_latest_verification_child(&session_id_obj)
            .await
        {
            Ok(Some(child)) => {
                let child_id_str = child.id.as_str().to_string();
                let child_session_id = SessionId::from_string(child_id_str.clone());

                // Check running_agent_registry under both keys
                let session_key = RunningAgentKey::new("session", &child_id_str);
                let ideation_key = RunningAgentKey::new("ideation", &child_id_str);
                let registry = &state.app_state.running_agent_registry;
                let agent_info = if let Some(info) = registry.get(&session_key).await {
                    Some(info)
                } else {
                    registry.get(&ideation_key).await
                };

                let threshold_secs = ideation_activity_threshold_secs();
                let agent_state = match &agent_info {
                    None => "idle".to_string(),
                    Some(info) => {
                        if let Some(last_active) = info.last_active_at {
                            let elapsed = chrono::Utc::now()
                                .signed_duration_since(last_active)
                                .num_seconds();
                            if elapsed >= 0 && (elapsed as u64) < threshold_secs {
                                "likely_generating".to_string()
                            } else {
                                "likely_waiting".to_string()
                            }
                        } else {
                            "likely_generating".to_string()
                        }
                    }
                };

                // Get last orchestrator message, truncated to 500 chars
                let last_msg = state
                    .app_state
                    .chat_message_repo
                    .get_latest_message_by_role(&child_session_id, "orchestrator")
                    .await
                    .ok()
                    .flatten();
                let (last_assistant_message, last_assistant_message_at) = match last_msg {
                    Some(msg) => {
                        let content = msg.content.chars().take(500).collect::<String>();
                        let at = msg.created_at.to_rfc3339();
                        (Some(content), Some(at))
                    }
                    None => (None, None),
                };

                // active_child_session_id: Some only when in_progress=true and child not archived
                let latest_child_archived = child.status == IdeationSessionStatus::Archived;
                let active_child_session_id = if in_progress && !latest_child_archived {
                    Some(child_id_str.clone())
                } else {
                    None
                };

                Some(ExternalVerificationChildInfo {
                    active_child_session_id,
                    latest_child_session_id: child_id_str,
                    latest_child_archived,
                    latest_child_updated_at: child.updated_at.to_rfc3339(),
                    agent_state,
                    pending_initial_prompt: child.pending_initial_prompt.clone(),
                    last_assistant_message,
                    last_assistant_message_at,
                })
            }
            Ok(None) => None,
            Err(e) => {
                error!(
                    "Failed to fetch verification child for {}: {}",
                    session_id, e
                );
                None
            }
        }
    };

    Ok(Json(ExternalVerificationResponse {
        status: status_str,
        in_progress,
        round,
        max_rounds,
        gap_count,
        gap_score: gap_count,
        gaps,
        convergence_reason,
        verification_child,
    }))
}
