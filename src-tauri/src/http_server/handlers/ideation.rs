// Ideation tool handlers for MCP orchestrator-ideation agent

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};

use crate::application::{CreateProposalOptions, UpdateProposalOptions};
use crate::domain::entities::{IdeationSessionId, Priority, TaskProposalId};

use super::super::helpers::{create_proposal_impl, parse_category, parse_priority, update_proposal_impl};
use super::super::types::{
    AddDependencyRequest, CreateProposalRequest, DeleteProposalRequest,
    HttpServerState, ProposalResponse, SuccessResponse, UpdateProposalRequest,
};

pub async fn create_task_proposal(
    State(state): State<HttpServerState>,
    Json(req): Json<CreateProposalRequest>,
) -> Result<Json<ProposalResponse>, StatusCode> {
    let session_id = IdeationSessionId::from_string(req.session_id);

    // Parse category
    let category = parse_category(&req.category).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Parse priority (default to Medium if not provided)
    let priority = req
        .priority
        .as_ref()
        .map(|s| parse_priority(s.as_str()))
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .unwrap_or(Priority::Medium);

    // Convert steps and acceptance criteria to JSON strings
    let steps = req
        .steps
        .map(|s| serde_json::to_string(&s).map_err(|_| StatusCode::BAD_REQUEST))
        .transpose()?;
    let acceptance_criteria = req
        .acceptance_criteria
        .map(|ac| serde_json::to_string(&ac).map_err(|_| StatusCode::BAD_REQUEST))
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
    let proposal = create_proposal_impl(&state.app_state, session_id, options)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Parse priority if provided
    let user_priority = req
        .user_priority
        .as_ref()
        .map(|s| parse_priority(s.as_str()))
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Convert steps and acceptance criteria to JSON strings
    let steps = req
        .steps
        .map(|s| serde_json::to_string(&s).map_err(|_| StatusCode::BAD_REQUEST))
        .transpose()?;
    let acceptance_criteria = req
        .acceptance_criteria
        .map(|ac| serde_json::to_string(&ac).map_err(|_| StatusCode::BAD_REQUEST))
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
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ProposalResponse::from(updated)))
}

pub async fn delete_task_proposal(
    State(state): State<HttpServerState>,
    Json(req): Json<DeleteProposalRequest>,
) -> Result<Json<SuccessResponse>, StatusCode> {
    let proposal_id = TaskProposalId::from_string(req.proposal_id);

    state
        .app_state
        .task_proposal_repo
        .delete(&proposal_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(SuccessResponse {
        success: true,
        message: "Dependency added successfully".to_string(),
    }))
}
