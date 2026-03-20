// Session linking handlers for MCP tools: create_child_session, get_parent_session_context

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use tauri::Emitter;
use tracing::error;

use crate::application::chat_service::{ChatService, ClaudeChatService};
use crate::domain::entities::{
    ChatContextType, IdeationSession, IdeationSessionId, IdeationSessionStatus, SessionLink,
    SessionPurpose, SessionRelationship, VerificationStatus,
};
use crate::domain::services::{
    emit_verification_started, emit_verification_status_changed,
};
use crate::infrastructure::agents::claude::{
    get_team_constraints, team_constraints_config, validate_child_team_config,
    verification_config, TeamConstraints,
};
use crate::infrastructure::sqlite::SqliteIdeationSessionRepository as SessionRepo;

use super::super::types::{
    CreateChildSessionRequest, CreateChildSessionResponse, HttpServerState, ParentContextResponse,
    ParentProposalSummary, ParentSessionSummary, TeamConfigInput,
};

type JsonError = (StatusCode, Json<serde_json::Value>);

fn json_error(status: StatusCode, error: impl Into<String>) -> JsonError {
    (status, Json(serde_json::json!({ "error": error.into() })))
}

/// Convert TeamConfigInput (HTTP layer) to TeamConstraints (domain layer).
/// Uses defaults for unspecified fields.
fn team_config_input_to_constraints(input: &TeamConfigInput) -> TeamConstraints {
    TeamConstraints {
        max_teammates: input.max_teammates.map(|v| v as u8).unwrap_or(5),
        model_cap: input
            .model_ceiling
            .clone()
            .unwrap_or_else(|| "sonnet".to_string()),
        budget_limit: input.budget_limit,
        ..TeamConstraints::default()
    }
}

/// Convert TeamConstraints (domain layer) back to TeamConfigInput (HTTP layer).
fn constraints_to_team_config_input(constraints: &TeamConstraints) -> TeamConfigInput {
    TeamConfigInput {
        max_teammates: Some(constraints.max_teammates as i32),
        model_ceiling: Some(constraints.model_cap.clone()),
        budget_limit: constraints.budget_limit,
        composition_mode: None, // Not stored in TeamConstraints
    }
}

/// Parse team_config_json string into TeamConstraints, using defaults if invalid or missing.
fn parse_team_config_json(json_str: Option<&String>) -> TeamConstraints {
    json_str
        .and_then(|s| serde_json::from_str::<TeamConfigInput>(s).ok())
        .map(|input| team_config_input_to_constraints(&input))
        .unwrap_or_default()
}

/// Validate resolved team config against project constraints from ralphx.yaml.
/// Returns capped TeamConstraints and serialized JSON for storage.
/// Uses "ideation" as the process context (child sessions don't know their process at creation time).
fn validate_resolved_team_config(
    resolved_team_mode: Option<&String>,
    resolved_team_config_json: Option<&String>,
) -> (Option<String>, Option<String>) {
    // If no team mode, child runs in solo mode (no validation needed)
    let team_mode = match resolved_team_mode {
        Some(mode) => mode.clone(),
        None => return (None, None),
    };

    // Parse resolved config (or use defaults if invalid)
    let resolved_constraints = parse_team_config_json(resolved_team_config_json);

    // Get project constraints from ralphx.yaml for "ideation" process
    let config = team_constraints_config();
    let yaml_constraints = get_team_constraints(config, "ideation");

    // Validate: cap resolved config at yaml constraints to prevent escalation
    let validated = validate_child_team_config(&resolved_constraints, &yaml_constraints);

    // Convert back to storage format
    let validated_input = constraints_to_team_config_input(&validated);
    let validated_json = serde_json::to_string(&validated_input).ok();

    (Some(team_mode), validated_json)
}

/// Synthesize a default initial prompt for verification child sessions that were created
/// without an explicit `initial_prompt` or `description`. Returns `None` when:
/// - `purpose` is not `"verification"`, OR
/// - `effective_description` is `Some` (description branch handles auto-spawn in that case)
///
/// When returning `Some`, the prompt includes the metadata suffix that `plan-verifier`
/// expects to parse: `parent_session_id: X, generation: Y, max_rounds: N`.
#[doc(hidden)]
pub fn synthesize_verification_prompt(
    purpose: &Option<String>,
    verification_generation: Option<i32>,
    max_rounds: u32,
    effective_description: &Option<String>,
    parent_session_id: &str,
) -> Option<String> {
    if purpose.as_deref() != Some("verification") || effective_description.is_some() {
        return None;
    }
    let gen = verification_generation.unwrap_or(1);
    Some(format!(
        "Begin plan verification.\n\nparent_session_id: {}, generation: {}, max_rounds: {}",
        parent_session_id, gen, max_rounds
    ))
}

/// Create a child session linked to a parent session
///
/// Validates parent exists, checks for cycles, creates session with parent_session_id,
/// creates SessionLink row, optionally assembles ParentSessionContext, emits event
pub async fn create_child_session(
    State(state): State<HttpServerState>,
    Json(req): Json<CreateChildSessionRequest>,
) -> Result<Json<CreateChildSessionResponse>, JsonError> {
    let parent_id = IdeationSessionId::from_string(req.parent_session_id.clone());
    let verify_cfg = verification_config();

    // Verification generation; set below when purpose == "verification" and init succeeds
    let mut verification_generation: Option<i32> = None;

    // Validate parent session exists
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

    // Check for cycles: get ancestor chain and validate parent is not a descendant of a future child
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

    // Check for self-reference (already impossible at DB level but good to check)
    if ancestor_chain.iter().any(|s| s.id == parent_id) {
        return Err(json_error(
            StatusCode::BAD_REQUEST,
            "Circular reference detected: session cannot be its own parent",
        ));
    }

    // Verification branch: auto-initialize verification state when purpose == "verification"
    if req.purpose.as_deref() == Some("verification") {
        // Pre-flight: parent must have a plan artifact
        if parent.plan_artifact_id.is_none() {
            return Err(json_error(
                StatusCode::BAD_REQUEST,
                "Cannot start verification: parent session has no plan artifact",
            ));
        }

        // Concurrency guard: atomically set in_progress=1 and increment generation
        let pid = parent_id.as_str().to_string();
        let verify_result = state
            .app_state
            .db
            .run(move |conn| SessionRepo::trigger_auto_verify_sync(conn, &pid))
            .await
            .map_err(|e| {
                error!("Failed to trigger verification sync: {}", e);
                json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to initialize verification state: {}", e),
                )
            })?;

        match verify_result {
            Some(new_gen) => {
                // Verification initialized; record generation for prompt augmentation and response
                verification_generation = Some(new_gen);
                if let Some(app_handle) = &state.app_state.app_handle {
                    emit_verification_started(
                        app_handle,
                        parent_id.as_str(),
                        new_gen,
                        verify_cfg.max_rounds,
                    );
                }
            }
            None => {
                // Either already in_progress or imported_verified — re-query for fresh state
                let pid2 = parent_id.as_str().to_string();
                let fresh_parent = state
                    .app_state
                    .db
                    .run(move |conn| SessionRepo::get_by_id_sync(conn, &pid2))
                    .await
                    .map_err(|e| {
                        error!("Failed to re-query parent session: {}", e);
                        json_error(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Failed to check verification state: {}", e),
                        )
                    })?;
                let fresh = fresh_parent.ok_or_else(|| {
                    json_error(StatusCode::NOT_FOUND, "Parent session not found")
                })?;
                if fresh.verification_in_progress {
                    return Err(json_error(
                        StatusCode::CONFLICT,
                        "Verification already in progress",
                    ));
                } else {
                    return Err(json_error(
                        StatusCode::BAD_REQUEST,
                        "Cannot verify imported-verified sessions",
                    ));
                }
            }
        }
    }

    // Create new child session with parent_session_id set
    // Resolve team config: explicit > inherited > None
    let (resolved_team_mode, resolved_team_config_json) = if let Some(mode) = &req.team_mode {
        // Explicit team_mode provided - use it with optional config
        let config_json = req
            .team_config
            .as_ref()
            .and_then(|c| serde_json::to_value(c).ok());
        (Some(mode.clone()), config_json.map(|v| v.to_string()))
    } else if req.inherit_context {
        // No explicit team_mode, but inherit_context=true: inherit from parent
        (parent.team_mode.clone(), parent.team_config_json.clone())
    } else {
        // No explicit team_mode and inherit_context=false: solo mode
        (None, None)
    };

    // Validate resolved config against project constraints from ralphx.yaml
    // Caps values at min(resolved, yaml) to prevent privilege escalation
    let (team_mode, team_config_json) = validate_resolved_team_config(
        resolved_team_mode.as_ref(),
        resolved_team_config_json.as_ref(),
    );

    let child_session = IdeationSession {
        id: IdeationSessionId::new(),
        project_id: parent.project_id.clone(),
        title: req.title.clone(),
        status: IdeationSessionStatus::Active,
        // Child starts with no own plan — the inherited plan is read-only via inherited_plan_artifact_id
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
            .and_then(|p| p.parse::<SessionPurpose>().ok())
            .unwrap_or_default(),
        cross_project_checked: true,
        plan_version_last_read: None,
        origin: parent.origin,
        expected_proposal_count: None,
        auto_accept_status: None,
        auto_accept_started_at: None,
    };

    let child_id = child_session.id.clone();
    let child_session_str = child_id.as_str().to_string();
    let parent_session_str = parent_id.as_str().to_string();

    // Create the session in the database
    let created_session = state
        .app_state
        .ideation_session_repo
        .create(child_session)
        .await
        .map_err(|e| {
            error!("Failed to create child session: {}", e);
            // Rollback verification state if we initialized it
            if let Some(current_generation) = verification_generation {
                let pid = parent_id.as_str().to_string();
                let pid_for_reset = pid.clone();
                let db = state.app_state.db.clone();
                let app_handle = state.app_state.app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(re) = db
                        .run(move |conn| SessionRepo::reset_auto_verify_sync(conn, &pid_for_reset))
                        .await
                    {
                        error!("Failed to rollback verification state after child DB insert failure: {}", re);
                    } else if let Some(handle) = app_handle {
                        emit_verification_status_changed(
                            &handle,
                            &pid,
                            VerificationStatus::Unverified,
                            false,
                            None,
                            Some("spawn_failed"),
                            Some(current_generation),
                        );
                    }
                });
            }
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create session: {}", e),
            )
        })?;

    // Create the SessionLink row
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
                let pid = parent_id.as_str().to_string();
                let pid_for_reset = pid.clone();
                let db = state.app_state.db.clone();
                let app_handle = state.app_state.app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(re) = db
                        .run(move |conn| SessionRepo::reset_auto_verify_sync(conn, &pid_for_reset))
                        .await
                    {
                        error!("Failed to rollback verification state after link creation failure: {}", re);
                    } else if let Some(handle) = app_handle {
                        emit_verification_status_changed(
                            &handle,
                            &pid,
                            VerificationStatus::Unverified,
                            false,
                            None,
                            Some("spawn_failed"),
                            Some(current_generation),
                        );
                    }
                });
            }
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create session link: {}", e),
            )
        })?;

    // Optionally assemble ParentSessionContext if requested
    let parent_context = if req.inherit_context {
        let plan_content = if let Some(plan_id) = &parent.plan_artifact_id {
            state
                .app_state
                .artifact_repo
                .get_by_id(plan_id)
                .await
                .ok()
                .flatten()
                .and_then(|artifact| {
                    if let crate::domain::entities::ArtifactContent::Inline { text } =
                        artifact.content
                    {
                        Some(text)
                    } else {
                        None
                    }
                })
        } else {
            None
        };

        // Fetch parent's proposals
        let proposals = state
            .app_state
            .task_proposal_repo
            .get_by_session(&parent_id)
            .await
            .unwrap_or_default();

        let proposal_summaries = proposals
            .iter()
            .map(|p| ParentProposalSummary {
                id: p.id.to_string(),
                title: p.title.clone(),
                category: p.category.to_string(),
                priority: p.suggested_priority.to_string(),
                status: p.status.to_string(),
                acceptance_criteria: p.acceptance_criteria.clone(),
            })
            .collect();

        Some(ParentContextResponse {
            parent_session: ParentSessionSummary {
                id: parent.id.to_string(),
                title: parent.title.unwrap_or_else(|| "Untitled".to_string()),
                status: parent.status.to_string(),
            },
            plan_content,
            proposals: proposal_summaries,
        })
    } else {
        None
    };

    let title = created_session
        .title
        .clone()
        .unwrap_or_else(|| "Child Session".to_string());

    // Build effective prompts: when verification_generation is set, append verification metadata
    // in the format the plan-verifier agent already expects to parse.
    let effective_initial_prompt: Option<String> = req.initial_prompt.as_ref().map(|p| {
        if let Some(gen) = verification_generation {
            format!(
                "{}\n\nparent_session_id: {}, generation: {}, max_rounds: {}",
                p,
                parent_session_str,
                gen,
                verify_cfg.max_rounds
            )
        } else {
            p.clone()
        }
    });
    let effective_description: Option<String> = req.description.as_ref().map(|d| {
        if let Some(gen) = verification_generation {
            format!(
                "{}\n\nparent_session_id: {}, generation: {}, max_rounds: {}",
                d,
                parent_session_str,
                gen,
                verify_cfg.max_rounds
            )
        } else {
            d.clone()
        }
    });

    // Verification sessions MUST auto-spawn — the parent's in_progress flag is already set.
    // If no explicit prompt was provided, synthesize a default with verification metadata.
    // Only fires when BOTH initial_prompt AND description are absent for verification sessions.
    let effective_initial_prompt = effective_initial_prompt.or_else(|| {
        synthesize_verification_prompt(
            &req.purpose,
            verification_generation,
            verify_cfg.max_rounds,
            &effective_description,
            &parent_session_str,
        )
    });

    // Auto-spawn orchestrator agent on child session if initial_prompt is set.
    // send_message stores the user message and spawns a background agent — non-blocking.
    let orchestration_triggered = if let Some(ref prompt) = effective_initial_prompt {
        let app = &state.app_state;
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
        .with_execution_state(Arc::clone(&state.execution_state))
        .with_plan_branch_repo(Arc::clone(&app.plan_branch_repo))
        .with_task_proposal_repo(Arc::clone(&app.task_proposal_repo))
        .with_interactive_process_registry(Arc::clone(&app.interactive_process_registry));
        if let Some(ref handle) = app.app_handle {
            chat_service = chat_service.with_app_handle(handle.clone());
        }
        // Apply team_mode from child session (mirrors logic in unified_chat_commands.rs:216-228)
        if session_is_team_mode(&created_session) {
            chat_service = chat_service.with_team_mode(true);
        }

        match chat_service
            .send_message(ChatContextType::Ideation, &child_session_str, prompt, Default::default())
            .await
        {
            Ok(_) => true,
            Err(e) => {
                // Log but don't fail the create_child_session call — agent spawn is best-effort
                error!(
                    "Failed to auto-spawn agent on child session {}: {}",
                    child_session_str, e
                );
                // Rollback verification state — parent's in_progress must not stay locked
                if let Some(current_generation) = verification_generation {
                    let pid = parent_id.as_str().to_string();
                    let pid_for_reset = pid.clone();
                    let db = state.app_state.db.clone();
                    let app_handle = state.app_state.app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(re) = db
                            .run(move |conn| SessionRepo::reset_auto_verify_sync(conn, &pid_for_reset))
                            .await
                        {
                            error!("Failed to rollback verification state after spawn failure: {}", re);
                        } else if let Some(handle) = app_handle {
                            emit_verification_status_changed(
                                &handle,
                                &pid,
                                VerificationStatus::Unverified,
                                false,
                                None,
                                Some("spawn_failed"),
                                Some(current_generation),
                            );
                        }
                    });
                    // Reset local generation so response reflects failure
                    verification_generation = None;
                }
                false
            }
        }
    } else if let Some(ref desc) = effective_description {
        // Use description as the initial prompt via ChatService (same as initial_prompt path)
        // This ensures a conversation is created and streaming events are emitted to the frontend
        if !desc.trim().is_empty() {
            let app = &state.app_state;
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
            .with_execution_state(Arc::clone(&state.execution_state))
            .with_plan_branch_repo(Arc::clone(&app.plan_branch_repo))
            .with_task_proposal_repo(Arc::clone(&app.task_proposal_repo))
            .with_interactive_process_registry(Arc::clone(&app.interactive_process_registry));
            if let Some(ref handle) = app.app_handle {
                chat_service = chat_service.with_app_handle(handle.clone());
            }
            // Apply team_mode from child session (mirrors logic in unified_chat_commands.rs:216-228)
            if session_is_team_mode(&created_session) {
                chat_service = chat_service.with_team_mode(true);
            }

            match chat_service
                .send_message(ChatContextType::Ideation, &child_session_str, desc, Default::default())
                .await
            {
                Ok(_) => true,
                Err(e) => {
                    error!(
                        "Failed to auto-spawn agent on child session {} (from description): {}",
                        child_session_str, e
                    );
                    // Rollback verification state — parent's in_progress must not stay locked
                    if let Some(current_generation) = verification_generation {
                        let pid = parent_id.as_str().to_string();
                        let pid_for_reset = pid.clone();
                        let db = state.app_state.db.clone();
                        let app_handle = state.app_state.app_handle.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(re) = db
                                .run(move |conn| SessionRepo::reset_auto_verify_sync(conn, &pid_for_reset))
                                .await
                            {
                                error!("Failed to rollback verification state after spawn failure: {}", re);
                            } else if let Some(handle) = app_handle {
                                emit_verification_status_changed(
                                    &handle,
                                    &pid,
                                    VerificationStatus::Unverified,
                                    false,
                                    None,
                                    Some("spawn_failed"),
                                    Some(current_generation),
                                );
                            }
                        });
                        verification_generation = None;
                    }
                    false
                }
            }
        } else {
            false
        }
    } else {
        false
    };

    // Emit Tauri event AFTER conversation is created (avoids race where frontend
    // navigates to child session before conversation exists in DB)
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

    // Parse team_config_json back to TeamConfigInput for response
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

/// Get parent session context (plan + proposals)
///
/// Fetches session, validates parent_session_id exists, retrieves parent session metadata
/// + plan artifact content + proposals, assembles response
pub async fn get_parent_session_context(
    State(state): State<HttpServerState>,
    Path(session_id_str): Path<String>,
) -> Result<Json<ParentContextResponse>, JsonError> {
    let session_id = IdeationSessionId::from_string(session_id_str.clone());

    // Get the session
    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|e| {
            error!("Failed to fetch session {}: {}", session_id.as_str(), e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch session: {}", e),
            )
        })?
        .ok_or_else(|| {
            error!("Session {} not found", session_id.as_str());
            json_error(StatusCode::NOT_FOUND, "Session not found")
        })?;

    // Verify the session has a parent
    let parent_id = session.parent_session_id.ok_or_else(|| {
        // Top-level (root) sessions have no parent — this is a normal caller error,
        // not an application fault. Downgraded from ERROR to debug to avoid noise.
        tracing::debug!(
            session_id = session_id.as_str(),
            "Session does not have a parent"
        );
        json_error(
            StatusCode::NOT_FOUND,
            "Session does not have a parent session",
        )
    })?;

    // Get parent session
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

    // Get plan artifact if present
    let plan_content = if let Some(plan_id) = &parent.plan_artifact_id {
        state
            .app_state
            .artifact_repo
            .get_by_id(plan_id)
            .await
            .ok()
            .flatten()
            .and_then(|artifact| {
                if let crate::domain::entities::ArtifactContent::Inline { text } = artifact.content
                {
                    Some(text)
                } else {
                    None
                }
            })
    } else {
        None
    };

    // Get parent's proposals
    let proposals = state
        .app_state
        .task_proposal_repo
        .get_by_session(&parent_id)
        .await
        .unwrap_or_default();

    let proposal_summaries = proposals
        .iter()
        .map(|p| ParentProposalSummary {
            id: p.id.to_string(),
            title: p.title.clone(),
            category: p.category.to_string(),
            priority: p.suggested_priority.to_string(),
            status: p.status.to_string(),
            acceptance_criteria: p.acceptance_criteria.clone(),
        })
        .collect();

    Ok(Json(ParentContextResponse {
        parent_session: ParentSessionSummary {
            id: parent.id.to_string(),
            title: parent.title.unwrap_or_else(|| "Untitled".to_string()),
            status: parent.status.to_string(),
        },
        plan_content,
        proposals: proposal_summaries,
    }))
}

/// Returns true if the session should use team mode for agent spawning.
/// "solo" and None are treated as non-team; any other value ("research", "debate", etc.) is team.
#[doc(hidden)]
pub fn session_is_team_mode(session: &IdeationSession) -> bool {
    session.team_mode.as_deref().is_some_and(|m| m != "solo")
}

/// Create a verification child session for auto-verification.
///
/// Simplified internal variant of `create_child_session` for spawning plan-verifier agents.
/// Skips cycle detection and team config inheritance — verification sessions always run in
/// solo mode and are routed to the `plan-verifier` agent via `session_purpose = Verification`.
///
/// Returns `Ok(orchestration_triggered)` where `orchestration_triggered` is `true` when the
/// plan-verifier agent was successfully enqueued, `false` on agent spawn failure.
pub(crate) async fn create_verification_child_session(
    state: &HttpServerState,
    parent_session_id: &str,
    description: &str,
    title: &str,
) -> Result<bool, String> {
    let parent_id = IdeationSessionId::from_string(parent_session_id.to_string());

    // Fetch parent to inherit plan artifact
    let parent = state
        .app_state
        .ideation_session_repo
        .get_by_id(&parent_id)
        .await
        .map_err(|e| format!("Failed to fetch parent session: {}", e))?
        .ok_or_else(|| format!("Parent session {} not found", parent_session_id))?;

    let child_session = IdeationSession {
        id: IdeationSessionId::new(),
        project_id: parent.project_id.clone(),
        title: Some(title.to_string()),
        status: IdeationSessionStatus::Active,
        plan_artifact_id: None,
        inherited_plan_artifact_id: parent.plan_artifact_id.clone(),
        seed_task_id: None,
        parent_session_id: Some(parent_id.clone()),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        archived_at: None,
        converted_at: None,
        team_mode: None,
        team_config_json: None,
        title_source: None,
        verification_status: VerificationStatus::default(),
        verification_in_progress: false,
        verification_metadata: None,
        verification_generation: 0,
        source_project_id: None,
        source_session_id: None,
        session_purpose: SessionPurpose::Verification,
        cross_project_checked: true,
        plan_version_last_read: None,
        origin: parent.origin,
        expected_proposal_count: None,
        auto_accept_status: None,
        auto_accept_started_at: None,
    };

    let child_id = child_session.id.clone();
    let child_session_str = child_id.as_str().to_string();
    let parent_session_str = parent_session_id.to_string();

    // Insert child session
    let created_session = state
        .app_state
        .ideation_session_repo
        .create(child_session)
        .await
        .map_err(|e| format!("Failed to create verification child session: {}", e))?;

    // Create SessionLink
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
        .map_err(|e| format!("Failed to create session link: {}", e))?;

    // Spawn plan-verifier agent via send_message (non-blocking)
    let app = &state.app_state;
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
    .with_execution_state(Arc::clone(&state.execution_state))
    .with_plan_branch_repo(Arc::clone(&app.plan_branch_repo))
    .with_task_proposal_repo(Arc::clone(&app.task_proposal_repo))
    .with_interactive_process_registry(Arc::clone(&app.interactive_process_registry));
    if let Some(ref handle) = app.app_handle {
        chat_service = chat_service.with_app_handle(handle.clone());
    }

    let orchestration_triggered = match chat_service
        .send_message(
            ChatContextType::Ideation,
            &child_session_str,
            description,
            Default::default(),
        )
        .await
    {
        Ok(_) => true,
        Err(e) => {
            error!(
                "Failed to spawn plan-verifier on verification child session {}: {}",
                child_session_str, e
            );
            false
        }
    };

    // Emit event so frontend can suppress notification for verification children
    if let Some(app_handle) = &state.app_state.app_handle {
        let session_title = created_session
            .title
            .clone()
            .unwrap_or_else(|| title.to_string());
        let _ = app_handle.emit(
            "ideation:child_session_created",
            serde_json::json!({
                "sessionId": child_session_str,
                "parentSessionId": parent_session_str,
                "title": session_title,
                "purpose": "verification"
            }),
        );
    }

    Ok(orchestration_triggered)
}
