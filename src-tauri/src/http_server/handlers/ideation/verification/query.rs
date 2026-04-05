use super::*;

/// GET /api/ideation/sessions/:id/verification
///
/// Get current verification status for a session's plan (lightweight read).
///
/// D9: If `X-RalphX-Project-Scope` header is present, enforces project scope.
/// Internal agents (no header) bypass scope enforcement for backward compatibility.
pub async fn get_plan_verification(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Path(session_id): Path<String>,
) -> Result<Json<VerificationResponse>, JsonError> {
    use crate::domain::entities::ideation::VerificationMetadata;
    use crate::domain::services::gap_score;
    use crate::http_server::types::{VerificationGapResponse, VerificationRoundSummary};

    let session_id_obj = crate::domain::entities::IdeationSessionId::from_string(session_id.clone());

    if !scope.is_unrestricted() {
        let session = state
            .app_state
            .ideation_session_repo
            .get_by_id(&session_id_obj)
            .await
            .ok()
            .flatten()
            .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "Session not found"))?;
        session
            .assert_project_scope(&scope)
            .map_err(|_| json_error(StatusCode::FORBIDDEN, "Forbidden"))?;
    }

    let (status, in_progress, metadata_json) = state
        .app_state
        .ideation_session_repo
        .get_verification_status(&session_id_obj)
        .await
        .map_err(|e| {
            error!(
                "Failed to get verification status for {}: {}",
                session_id, e
            );
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get verification status",
            )
        })?
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "Session not found"))?;

    let metadata: Option<VerificationMetadata> = metadata_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    let current_round = metadata.as_ref().and_then(|m| {
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
    let gap_sc = metadata.as_ref().map(|m| gap_score(&m.current_gaps));
    let convergence_reason = metadata.as_ref().and_then(|m| m.convergence_reason.clone());
    let best_round_index = metadata.as_ref().and_then(|m| m.best_round_index);

    let current_gaps = metadata
        .as_ref()
        .map(|m| {
            m.current_gaps
                .iter()
                .map(|g| VerificationGapResponse {
                    severity: g.severity.clone(),
                    category: g.category.clone(),
                    description: g.description.clone(),
                    why_it_matters: g.why_it_matters.clone(),
                    source: g.source.clone(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let rounds = metadata
        .as_ref()
        .map(|m| {
            m.rounds
                .iter()
                .enumerate()
                .rev()
                .take(10)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .map(|(i, r)| VerificationRoundSummary {
                    round: (i + 1) as u32,
                    gap_score: r.gap_score,
                    gap_count: r.fingerprints.len() as u32,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let (plan_version, verification_generation) = {
        let session = state
            .app_state
            .ideation_session_repo
            .get_by_id(&session_id_obj)
            .await
            .ok()
            .flatten();
        let generation = session
            .as_ref()
            .map(|s| s.verification_generation)
            .unwrap_or(0);
        let plan_version = if let Some(ref session) = session {
            if let Some(ref artifact_id) = session.plan_artifact_id {
                state
                    .app_state
                    .artifact_repo
                    .get_by_id(artifact_id)
                    .await
                    .ok()
                    .flatten()
                    .map(|artifact| artifact.metadata.version)
            } else {
                None
            }
        } else {
            None
        };
        (plan_version, generation)
    };

    // Step 5: Fetch verification child continuity data
    let verification_child = {
        use crate::http_server::types::VerificationChildInfo;
        use crate::infrastructure::agents::claude::ideation_activity_threshold_secs;

        match state
            .app_state
            .ideation_session_repo
            .get_latest_verification_child(&session_id_obj)
            .await
        {
            Ok(Some(child)) => {
                let child_id_str = child.id.as_str().to_string();
                let child_session_id = IdeationSessionId::from_string(child_id_str.clone());

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
                let active_child_session_id =
                    if in_progress && !latest_child_archived {
                        Some(child_id_str.clone())
                    } else {
                        None
                    };

                Some(VerificationChildInfo {
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

    Ok(Json(VerificationResponse {
        session_id,
        status: status.to_string(),
        in_progress,
        current_round,
        max_rounds,
        gap_score: gap_sc,
        convergence_reason,
        best_round_index,
        current_gaps,
        rounds,
        plan_version,
        verification_generation,
        verification_child,
    }))
}
