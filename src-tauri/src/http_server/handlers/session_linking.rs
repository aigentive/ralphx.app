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
    SessionRelationship, VerificationStatus,
};
use crate::infrastructure::agents::claude::{
    get_team_constraints, team_constraints_config, validate_child_team_config, TeamConstraints,
};

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

/// Create a child session linked to a parent session
///
/// Validates parent exists, checks for cycles, creates session with parent_session_id,
/// creates SessionLink row, optionally assembles ParentSessionContext, emits event
pub async fn create_child_session(
    State(state): State<HttpServerState>,
    Json(req): Json<CreateChildSessionRequest>,
) -> Result<Json<CreateChildSessionResponse>, JsonError> {
    let parent_id = IdeationSessionId::from_string(req.parent_session_id.clone());

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

    // Auto-spawn orchestrator agent on child session if initial_prompt is set.
    // send_message stores the user message and spawns a background agent — non-blocking.
    let orchestration_triggered = if let Some(ref prompt) = req.initial_prompt {
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
            .send_message(ChatContextType::Ideation, &child_session_str, prompt)
            .await
        {
            Ok(_) => true,
            Err(e) => {
                // Log but don't fail the create_child_session call — agent spawn is best-effort
                error!(
                    "Failed to auto-spawn agent on child session {}: {}",
                    child_session_str, e
                );
                false
            }
        }
    } else if let Some(ref desc) = req.description {
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
                .send_message(ChatContextType::Ideation, &child_session_str, desc)
                .await
            {
                Ok(_) => true,
                Err(e) => {
                    error!(
                        "Failed to auto-spawn agent on child session {} (from description): {}",
                        child_session_str, e
                    );
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
            "title": title
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
fn session_is_team_mode(session: &IdeationSession) -> bool {
    session.team_mode.as_deref().is_some_and(|m| m != "solo")
}

#[cfg(test)]
#[path = "session_linking_tests.rs"]
mod tests;
