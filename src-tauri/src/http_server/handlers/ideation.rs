// Ideation tool handlers for MCP orchestrator-ideation agent

use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use tauri::Emitter;
use tracing::error;

use crate::application::{CreateProposalOptions, UpdateProposalOptions};
use crate::commands::ideation_commands::TaskProposalResponse;
use crate::domain::entities::{IdeationSessionId, Priority, TaskProposalId};

use super::super::helpers::{create_proposal_impl, parse_category, parse_priority, update_proposal_impl};
use super::super::types::{
    AddDependencyRequest, ApplyDependenciesResponse, ApplyDependencySuggestionsRequest,
    CreateProposalRequest, DeleteProposalRequest,
    HttpServerState, ListProposalsResponse, ProposalDetailResponse, ProposalResponse,
    ProposalSummary, SuccessResponse, UpdateProposalRequest, UpdateSessionTitleRequest,
};

pub async fn create_task_proposal(
    State(state): State<HttpServerState>,
    Json(req): Json<CreateProposalRequest>,
) -> Result<Json<ProposalResponse>, StatusCode> {
    let session_id = IdeationSessionId::from_string(req.session_id.clone());

    // Parse category
    let category = parse_category(&req.category).map_err(|e| {
        error!("Invalid category '{}': {}", req.category, e);
        StatusCode::BAD_REQUEST
    })?;

    // Parse priority (default to Medium if not provided)
    let priority = req
        .priority
        .as_ref()
        .map(|s| parse_priority(s.as_str()))
        .transpose()
        .map_err(|e| {
            error!("Invalid priority: {}", e);
            StatusCode::BAD_REQUEST
        })?
        .unwrap_or(Priority::Medium);

    // Convert steps and acceptance criteria to JSON strings
    let steps = req
        .steps
        .map(|s| serde_json::to_string(&s).map_err(|e| {
            error!("Failed to serialize steps: {}", e);
            StatusCode::BAD_REQUEST
        }))
        .transpose()?;
    let acceptance_criteria = req
        .acceptance_criteria
        .map(|ac| serde_json::to_string(&ac).map_err(|e| {
            error!("Failed to serialize acceptance_criteria: {}", e);
            StatusCode::BAD_REQUEST
        }))
        .transpose()?;

    let options = CreateProposalOptions {
        title: req.title,
        description: req.description,
        category,
        suggested_priority: priority,
        steps,
        acceptance_criteria,
    };

    // Create proposal using IdeationService logic
    let session_id_str = session_id.as_str().to_string();
    let proposal = create_proposal_impl(&state.app_state, session_id, options)
        .await
        .map_err(|e| {
            error!("Failed to create proposal for session {}: {}", session_id_str, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Emit event for real-time UI update
    if let Some(app_handle) = &state.app_state.app_handle {
        let response = TaskProposalResponse::from(proposal.clone());
        let _ = app_handle.emit(
            "proposal:created",
            serde_json::json!({
                "proposal": response
            }),
        );
    }

    Ok(Json(ProposalResponse::from(proposal)))
}

pub async fn update_task_proposal(
    State(state): State<HttpServerState>,
    Json(req): Json<UpdateProposalRequest>,
) -> Result<Json<ProposalResponse>, StatusCode> {
    let proposal_id = TaskProposalId::from_string(req.proposal_id);

    // Parse category if provided
    let category = req
        .category
        .as_ref()
        .map(|s| parse_category(s.as_str()))
        .transpose()
        .map_err(|e| {
            error!("Invalid category: {}", e);
            StatusCode::BAD_REQUEST
        })?;

    // Parse priority if provided
    let user_priority = req
        .user_priority
        .as_ref()
        .map(|s| parse_priority(s.as_str()))
        .transpose()
        .map_err(|e| {
            error!("Invalid priority: {}", e);
            StatusCode::BAD_REQUEST
        })?;

    // Convert steps and acceptance criteria to JSON strings
    let steps = req
        .steps
        .map(|s| serde_json::to_string(&s).map_err(|e| {
            error!("Failed to serialize steps: {}", e);
            StatusCode::BAD_REQUEST
        }))
        .transpose()?;
    let acceptance_criteria = req
        .acceptance_criteria
        .map(|ac| serde_json::to_string(&ac).map_err(|e| {
            error!("Failed to serialize acceptance_criteria: {}", e);
            StatusCode::BAD_REQUEST
        }))
        .transpose()?;

    let options = UpdateProposalOptions {
        title: req.title,
        description: req.description.map(Some),
        category,
        steps: steps.map(Some),
        acceptance_criteria: acceptance_criteria.map(Some),
        user_priority,
    };

    let updated = update_proposal_impl(&state.app_state, &proposal_id, options)
        .await
        .map_err(|e| {
            error!("Failed to update proposal {}: {}", proposal_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Emit event for real-time UI update
    if let Some(app_handle) = &state.app_state.app_handle {
        let response = TaskProposalResponse::from(updated.clone());
        let _ = app_handle.emit(
            "proposal:updated",
            serde_json::json!({
                "proposal": response
            }),
        );
    }

    Ok(Json(ProposalResponse::from(updated)))
}

pub async fn delete_task_proposal(
    State(state): State<HttpServerState>,
    Json(req): Json<DeleteProposalRequest>,
) -> Result<Json<SuccessResponse>, StatusCode> {
    let proposal_id = TaskProposalId::from_string(req.proposal_id.clone());

    state
        .app_state
        .task_proposal_repo
        .delete(&proposal_id)
        .await
        .map_err(|e| {
            error!("Failed to delete proposal {}: {}", proposal_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Emit event for real-time UI update
    if let Some(app_handle) = &state.app_state.app_handle {
        let _ = app_handle.emit(
            "proposal:deleted",
            serde_json::json!({
                "proposalId": req.proposal_id
            }),
        );
    }

    Ok(Json(SuccessResponse {
        success: true,
        message: "Proposal deleted successfully".to_string(),
    }))
}

pub async fn add_proposal_dependency(
    State(state): State<HttpServerState>,
    Json(req): Json<AddDependencyRequest>,
) -> Result<Json<SuccessResponse>, StatusCode> {
    let proposal_id = TaskProposalId::from_string(req.proposal_id);
    let depends_on_id = TaskProposalId::from_string(req.depends_on_id);

    state
        .app_state
        .proposal_dependency_repo
        .add_dependency(&proposal_id, &depends_on_id)
        .await
        .map_err(|e| {
            error!("Failed to add dependency from {} to {}: {}", proposal_id.as_str(), depends_on_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        message: "Dependency added successfully".to_string(),
    }))
}

pub async fn update_session_title(
    State(state): State<HttpServerState>,
    Json(req): Json<UpdateSessionTitleRequest>,
) -> Result<Json<SuccessResponse>, StatusCode> {
    let session_id = IdeationSessionId::from_string(req.session_id.clone());

    // Update title in database
    state
        .app_state
        .ideation_session_repo
        .update_title(&session_id, Some(req.title.clone()))
        .await
        .map_err(|e| {
            error!("Failed to update session title: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Emit event for real-time UI update
    if let Some(app_handle) = &state.app_state.app_handle {
        let _ = app_handle.emit(
            "ideation:session_title_updated",
            serde_json::json!({
                "sessionId": req.session_id,
                "title": req.title
            }),
        );
    }

    Ok(Json(SuccessResponse {
        success: true,
        message: "Session title updated".to_string(),
    }))
}

pub async fn list_session_proposals(
    State(state): State<HttpServerState>,
    Path(session_id): Path<String>,
) -> Result<Json<ListProposalsResponse>, StatusCode> {
    let session_id = IdeationSessionId::from_string(session_id);

    // Get all proposals for session
    let proposals = state
        .app_state
        .task_proposal_repo
        .get_by_session(&session_id)
        .await
        .map_err(|e| {
            error!("Failed to list proposals: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get all dependencies for the session
    let all_deps = state
        .app_state
        .proposal_dependency_repo
        .get_all_for_session(&session_id)
        .await
        .map_err(|e| {
            error!("Failed to get dependencies: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Build dependency map: proposal_id -> [depends_on_ids]
    let mut dep_map: HashMap<String, Vec<String>> = HashMap::new();
    for (from, to) in all_deps {
        dep_map
            .entry(from.to_string())
            .or_default()
            .push(to.to_string());
    }

    let count = proposals.len();
    let summaries: Vec<ProposalSummary> = proposals
        .into_iter()
        .map(|p| {
            let id_str = p.id.to_string();
            let priority = p.effective_priority().to_string();
            let category = p.category.to_string();
            let plan_artifact_id = p.plan_artifact_id.map(|id| id.to_string());
            ProposalSummary {
                id: id_str.clone(),
                title: p.title,
                category,
                priority,
                depends_on: dep_map.remove(&id_str).unwrap_or_default(),
                plan_artifact_id,
            }
        })
        .collect();

    Ok(Json(ListProposalsResponse {
        proposals: summaries,
        count,
    }))
}

pub async fn get_proposal(
    State(state): State<HttpServerState>,
    Path(proposal_id): Path<String>,
) -> Result<Json<ProposalDetailResponse>, StatusCode> {
    let proposal_id = TaskProposalId::from_string(proposal_id);

    let proposal = state
        .app_state
        .task_proposal_repo
        .get_by_id(&proposal_id)
        .await
        .map_err(|e| {
            error!("Failed to get proposal: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Get dependencies for this proposal
    let deps = state
        .app_state
        .proposal_dependency_repo
        .get_dependencies(&proposal_id)
        .await
        .map_err(|e| {
            error!("Failed to get dependencies: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Parse steps and acceptance_criteria from JSON strings
    let steps: Vec<String> = proposal
        .steps
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let acceptance_criteria: Vec<String> = proposal
        .acceptance_criteria
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    // Compute values before moving fields
    let priority = proposal.effective_priority().to_string();
    let category = proposal.category.to_string();
    let created_at = proposal.created_at.to_rfc3339();
    let plan_artifact_id = proposal.plan_artifact_id.map(|id| id.to_string());

    Ok(Json(ProposalDetailResponse {
        id: proposal.id.to_string(),
        session_id: proposal.session_id.to_string(),
        title: proposal.title,
        description: proposal.description,
        category,
        priority,
        steps,
        acceptance_criteria,
        depends_on: deps.iter().map(|d| d.to_string()).collect(),
        plan_artifact_id,
        created_at,
    }))
}

/// Apply AI-suggested dependencies: clear existing, add new (skip cycles)
/// Used by dependency-suggester agent
pub async fn apply_proposal_dependencies(
    State(state): State<HttpServerState>,
    Json(req): Json<ApplyDependencySuggestionsRequest>,
) -> Result<Json<ApplyDependenciesResponse>, StatusCode> {
    let session_id = IdeationSessionId::from_string(req.session_id.clone());

    // Get existing proposals to validate IDs belong to session
    let proposals = state
        .app_state
        .task_proposal_repo
        .get_by_session(&session_id)
        .await
        .map_err(|e| {
            error!("Failed to get proposals for session {}: {}", session_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let valid_ids: std::collections::HashSet<String> = proposals
        .iter()
        .map(|p| p.id.to_string())
        .collect();

    // Step 1: Clear all existing dependencies for this session
    state
        .app_state
        .proposal_dependency_repo
        .clear_session_dependencies(&session_id)
        .await
        .map_err(|e| {
            error!("Failed to clear dependencies for session {}: {}", session_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Step 2: Add each suggested dependency (skip if would create cycle or invalid)
    let mut applied_count = 0;
    let mut skipped_count = 0;

    for suggestion in &req.dependencies {
        // Validate both IDs belong to this session
        if !valid_ids.contains(&suggestion.proposal_id) || !valid_ids.contains(&suggestion.depends_on_id) {
            skipped_count += 1;
            continue;
        }

        // Skip self-dependency
        if suggestion.proposal_id == suggestion.depends_on_id {
            skipped_count += 1;
            continue;
        }

        let proposal_id = TaskProposalId::from_string(suggestion.proposal_id.clone());
        let depends_on_id = TaskProposalId::from_string(suggestion.depends_on_id.clone());

        // Check if adding this would create a cycle
        // Simple check: would depends_on_id eventually reach proposal_id?
        let would_cycle = would_create_cycle(
            &state.app_state,
            &session_id,
            &proposal_id,
            &depends_on_id,
        ).await;

        if would_cycle {
            skipped_count += 1;
            continue;
        }

        // Add the dependency
        match state
            .app_state
            .proposal_dependency_repo
            .add_dependency(&proposal_id, &depends_on_id)
            .await
        {
            Ok(_) => applied_count += 1,
            Err(e) => {
                error!("Failed to add dependency {} -> {}: {}", proposal_id.as_str(), depends_on_id.as_str(), e);
                skipped_count += 1;
            }
        }
    }

    // Emit event for real-time UI update
    if let Some(app_handle) = &state.app_state.app_handle {
        let _ = app_handle.emit(
            "dependencies:suggestions_applied",
            serde_json::json!({
                "sessionId": req.session_id,
                "appliedCount": applied_count,
                "skippedCount": skipped_count
            }),
        );
    }

    Ok(Json(ApplyDependenciesResponse {
        success: true,
        applied_count,
        skipped_count,
        message: format!("Applied {} dependencies, skipped {} (cycles/invalid)", applied_count, skipped_count),
    }))
}

/// Check if adding proposal_id -> depends_on_id would create a cycle
/// (i.e., if depends_on_id can already reach proposal_id through existing deps)
async fn would_create_cycle(
    app_state: &crate::application::AppState,
    session_id: &IdeationSessionId,
    proposal_id: &TaskProposalId,
    depends_on_id: &TaskProposalId,
) -> bool {
    // Get all existing dependencies for the session
    let deps = match app_state
        .proposal_dependency_repo
        .get_all_for_session(session_id)
        .await
    {
        Ok(deps) => deps,
        Err(_) => return false, // If we can't check, allow the dependency
    };

    // Build adjacency list: from_id -> [to_ids]
    let mut adj: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for (from, to) in &deps {
        adj.entry(from.to_string()).or_default().push(to.to_string());
    }

    // DFS from depends_on_id to see if we can reach proposal_id
    let mut visited = std::collections::HashSet::new();
    let mut stack = vec![depends_on_id.to_string()];

    while let Some(current) = stack.pop() {
        if current == proposal_id.to_string() {
            return true; // Found a path, adding would create cycle
        }

        if visited.contains(&current) {
            continue;
        }
        visited.insert(current.clone());

        if let Some(neighbors) = adj.get(&current) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    stack.push(neighbor.clone());
                }
            }
        }
    }

    false // No path found, safe to add
}
