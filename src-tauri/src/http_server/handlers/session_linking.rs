// Session linking handlers for MCP tools: create_child_session, get_parent_session_context

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use tauri::Emitter;
use tracing::{error, info};

use crate::application::chat_service::{ChatService, ClaudeChatService};
use crate::domain::entities::{
    ChatContextType, IdeationSession, IdeationSessionId, IdeationSessionStatus, SessionLink,
    SessionRelationship, SpawnOrchestratorJob,
};

use super::super::types::{
    CreateChildSessionRequest, CreateChildSessionResponse, HttpServerState, ParentContextResponse,
    ParentProposalSummary, ParentSessionSummary,
};

type JsonError = (StatusCode, Json<serde_json::Value>);

fn json_error(status: StatusCode, error: impl Into<String>) -> JsonError {
    (status, Json(serde_json::json!({ "error": error.into() })))
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
    let child_session = IdeationSession {
        id: IdeationSessionId::new(),
        project_id: parent.project_id.clone(),
        title: req.title.clone(),
        status: IdeationSessionStatus::Active,
        plan_artifact_id: if req.inherit_context {
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
        .unwrap_or_else(|| "Child Session".to_string());

    // Emit Tauri event (include initial_prompt so downstream consumers can spawn agents)
    if let Some(app_handle) = &state.app_state.app_handle {
        let mut event_payload = serde_json::json!({
            "sessionId": child_session_str,
            "parentSessionId": parent_session_str,
            "title": title
        });
        if let Some(ref prompt) = req.initial_prompt {
            event_payload["initialPrompt"] = serde_json::json!(prompt);
        }
        let _ = app_handle.emit(
            "ideation:child_session_created",
            event_payload,
        );
    }

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
        .with_plan_branch_repo(Arc::clone(&app.plan_branch_repo));
        if let Some(ref handle) = app.app_handle {
            chat_service = chat_service.with_app_handle(handle.clone());
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
        // Fallback: If description is provided but no initial_prompt, enqueue a SpawnOrchestratorJob for async processing
        if !desc.trim().is_empty() {
            let job = SpawnOrchestratorJob::new(
                child_id.clone(),
                parent.project_id.clone(),
                desc.clone(),
            );

            match state.app_state.spawn_orchestrator_job_repo.create(job).await {
                Ok(_) => {
                    info!(
                        session_id = %child_id,
                        "Enqueued spawn orchestrator job for child session"
                    );
                    true
                }
                Err(e) => {
                    error!(
                        session_id = %child_id,
                        error = %e,
                        "Failed to enqueue spawn orchestrator job"
                    );
                    // Don't fail the request - the session was created successfully
                    false
                }
            }
        } else {
            false
        }
    } else {
        false
    };

    Ok(Json(CreateChildSessionResponse {
        session_id: child_session_str,
        parent_session_id: parent_session_str,
        title,
        status: created_session.status.to_string(),
        created_at: created_session.created_at.to_rfc3339(),
        inherited_plan_id: created_session.plan_artifact_id.map(|id| id.to_string()),
        initial_prompt: req.initial_prompt.clone(),
        parent_context,
        orchestration_triggered,
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
        error!("Session {} does not have a parent", session_id.as_str());
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
