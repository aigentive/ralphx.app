// Ideation tool handlers for MCP orchestrator-ideation agent

use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use crate::http_server::project_scope::{ProjectScope, ProjectScopeGuard};
use tauri::Emitter;
use tracing::error;

type JsonError = (StatusCode, Json<serde_json::Value>);

fn json_error(status: StatusCode, error: impl Into<String>) -> JsonError {
    (status, Json(serde_json::json!({ "error": error.into() })))
}

use std::sync::Arc;

use crate::application::app_state::AppState;
use crate::application::chat_service::{AgentRunCompletedPayload, ChatService, ClaudeChatService, SendMessageOptions};
use crate::application::{CreateProposalOptions, InteractiveProcessKey, UpdateProposalOptions, UpdateSource};
use crate::error::AppError;
use crate::domain::entities::ideation::IdeationSessionStatus;
use crate::domain::entities::{ChatContextType, IdeationSessionId, Priority, TaskProposalId};
use crate::domain::repositories::ExternalEventsRepository;
use crate::domain::services::emit_verification_status_changed;
use crate::domain::services::running_agent_registry::RunningAgentKey;

use super::super::helpers::{
    archive_proposal_impl, create_proposal_impl, finalize_proposals_impl, parse_category,
    parse_priority, update_proposal_impl,
};
use super::super::types::{
    CreateProposalRequest, DeleteProposalRequest, FinalizeProposalsRequest,
    FinalizeProposalsResponse, GetSessionMessagesRequest, GetSessionMessagesResponse,
    HttpServerState, ListProposalsResponse, ProposalDetailResponse, ProposalResponse,
    ProposalSummary, RevertAndSkipRequest, SendSessionMessageRequest, SendSessionMessageResponse,
    SessionMessageResponse, SuccessResponse, UpdateProposalRequest, UpdateSessionTitleRequest,
    UpdateVerificationRequest, VerificationResponse,
};
use super::session_linking::session_is_team_mode;

pub async fn create_task_proposal(
    State(state): State<HttpServerState>,
    Json(req): Json<CreateProposalRequest>,
) -> Result<Json<ProposalResponse>, JsonError> {
    let session_id = IdeationSessionId::from_string(req.session_id.clone());

    // Parse category
    let category = parse_category(&req.category).map_err(|e| {
        error!("Invalid category '{}': {}", req.category, e);
        json_error(StatusCode::BAD_REQUEST, e)
    })?;

    // Parse priority (default to Medium if not provided)
    let priority = req
        .priority
        .as_ref()
        .map(|s| parse_priority(s.as_str()))
        .transpose()
        .map_err(|e| {
            error!("Invalid priority: {}", e);
            json_error(StatusCode::BAD_REQUEST, e)
        })?
        .unwrap_or(Priority::Medium);

    // Convert steps and acceptance criteria to JSON strings
    let steps = req
        .steps
        .map(|s| {
            serde_json::to_string(&s).map_err(|e| {
                error!("Failed to serialize steps: {}", e);
                json_error(
                    StatusCode::BAD_REQUEST,
                    format!("Failed to serialize steps: {}", e),
                )
            })
        })
        .transpose()?;
    let acceptance_criteria = req
        .acceptance_criteria
        .map(|ac| {
            serde_json::to_string(&ac).map_err(|e| {
                error!("Failed to serialize acceptance_criteria: {}", e);
                json_error(
                    StatusCode::BAD_REQUEST,
                    format!("Failed to serialize acceptance_criteria: {}", e),
                )
            })
        })
        .transpose()?;

    let options = CreateProposalOptions {
        title: req.title,
        description: req.description,
        category,
        suggested_priority: priority,
        steps,
        acceptance_criteria,
        estimated_complexity: None,
        depends_on: req.depends_on,
        target_project: req.target_project,
        expected_proposal_count: req.expected_proposal_count,
    };

    // Create proposal — events and dep analysis emitted inside create_proposal_impl()
    let session_id_str = session_id.as_str().to_string();
    let (proposal, dep_errors, ready_to_finalize) =
        create_proposal_impl(&state.app_state, session_id, options)
            .await
            .map_err(|e| {
                error!(
                    "Failed to create proposal for session {}: {}",
                    session_id_str, e
                );
                let status = match &e {
                    crate::error::AppError::Validation(_) => StatusCode::BAD_REQUEST,
                    crate::error::AppError::NotFound(_) => StatusCode::NOT_FOUND,
                    _ => StatusCode::INTERNAL_SERVER_ERROR,
                };
                json_error(status, e.to_string())
            })?;

    let mut response = ProposalResponse::from(proposal);
    response.dependency_errors = dep_errors;
    response.ready_to_finalize = ready_to_finalize;
    Ok(Json(response))
}

pub async fn finalize_proposals(
    State(state): State<HttpServerState>,
    Json(req): Json<FinalizeProposalsRequest>,
) -> Result<Json<FinalizeProposalsResponse>, JsonError> {
    // TODO(webhook): emit EventType::IdeationProposalsReady via AppState::webhook_publisher
    // when AppState gains that field. Payload: { session_id, project_id, proposal_count }.
    let response = finalize_proposals_impl(&state.app_state, &req.session_id)
        .await
        .map_err(|e| {
            error!("Failed to finalize proposals for session {}: {}", req.session_id, e);
            let status = match &e {
                crate::error::AppError::Validation(_) => StatusCode::BAD_REQUEST,
                crate::error::AppError::NotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            json_error(status, e.to_string())
        })?;

    if response.session_status == "accepted" {
        if let Some(app_handle) = &state.app_state.app_handle {
            // Notify frontend: session moved to Accepted → PlanBrowser refreshes
            if let Err(e) = app_handle.emit(
                "ideation:session_accepted",
                serde_json::json!({
                    "sessionId": req.session_id,
                    "projectId": response.project_id,
                }),
            ) {
                tracing::warn!("Failed to emit ideation:session_accepted event: {}", e);
            }

            // Notify task scheduler: newly created tasks may be Ready
            let project_id = crate::domain::entities::ProjectId::from_string(response.project_id.clone());
            let queued_count = match state
                .app_state
                .task_repo
                .get_by_status(&project_id, crate::domain::entities::InternalStatus::Ready)
                .await
            {
                Ok(tasks) => tasks.len(),
                Err(e) => {
                    tracing::warn!("Failed to count Ready tasks for queue_changed event: {}", e);
                    0
                }
            };
            if let Err(e) = app_handle.emit(
                "execution:queue_changed",
                serde_json::json!({
                    "queuedCount": queued_count,
                    "projectId": response.project_id,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                }),
            ) {
                tracing::warn!("Failed to emit execution:queue_changed event: {}", e);
            }
        }
    }

    Ok(Json(response))
}

pub async fn update_task_proposal(
    State(state): State<HttpServerState>,
    Json(req): Json<UpdateProposalRequest>,
) -> Result<Json<ProposalResponse>, JsonError> {
    let proposal_id = TaskProposalId::from_string(req.proposal_id);

    // Parse category if provided
    let category = req
        .category
        .as_ref()
        .map(|s| parse_category(s.as_str()))
        .transpose()
        .map_err(|e| {
            error!("Invalid category: {}", e);
            json_error(StatusCode::BAD_REQUEST, e)
        })?;

    // Parse priority if provided
    let user_priority = req
        .user_priority
        .as_ref()
        .map(|s| parse_priority(s.as_str()))
        .transpose()
        .map_err(|e| {
            error!("Invalid priority: {}", e);
            json_error(StatusCode::BAD_REQUEST, e)
        })?;

    // Convert steps and acceptance criteria to JSON strings
    let steps = req
        .steps
        .map(|s| {
            serde_json::to_string(&s).map_err(|e| {
                error!("Failed to serialize steps: {}", e);
                json_error(
                    StatusCode::BAD_REQUEST,
                    format!("Failed to serialize steps: {}", e),
                )
            })
        })
        .transpose()?;
    let acceptance_criteria = req
        .acceptance_criteria
        .map(|ac| {
            serde_json::to_string(&ac).map_err(|e| {
                error!("Failed to serialize acceptance_criteria: {}", e);
                json_error(
                    StatusCode::BAD_REQUEST,
                    format!("Failed to serialize acceptance_criteria: {}", e),
                )
            })
        })
        .transpose()?;

    let options = UpdateProposalOptions {
        title: req.title,
        description: req.description.map(Some),
        category,
        steps: steps.map(Some),
        acceptance_criteria: acceptance_criteria.map(Some),
        user_priority,
        estimated_complexity: None,
        source: UpdateSource::Api,
        add_depends_on: req.add_depends_on,
        add_blocks: req.add_blocks,
        target_project: req.target_project.map(Some),
    };

    // Update proposal — events and dep analysis emitted inside update_proposal_impl()
    let (updated, dep_errors) = update_proposal_impl(&state.app_state, &proposal_id, options)
        .await
        .map_err(|e| {
            error!("Failed to update proposal {}: {}", proposal_id.as_str(), e);
            let status = match &e {
                crate::error::AppError::Validation(_) => StatusCode::BAD_REQUEST,
                crate::error::AppError::NotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            json_error(status, e.to_string())
        })?;

    let mut response = ProposalResponse::from(updated);
    response.dependency_errors = dep_errors;
    Ok(Json(response))
}

pub async fn archive_task_proposal(
    State(state): State<HttpServerState>,
    Json(req): Json<DeleteProposalRequest>,
) -> Result<Json<SuccessResponse>, StatusCode> {
    let proposal_id = TaskProposalId::from_string(req.proposal_id.clone());

    // Archive proposal — assert_session_mutable(), events, and dep analysis inside impl
    archive_proposal_impl(&state.app_state, proposal_id)
        .await
        .map_err(|e| {
            error!("Failed to archive proposal {}: {}", req.proposal_id, e);
            match e {
                crate::error::AppError::NotFound(_) => StatusCode::NOT_FOUND,
                crate::error::AppError::Validation(_) => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        message: "Proposal archived successfully".to_string(),
    }))
}

pub async fn update_session_title(
    State(state): State<HttpServerState>,
    Json(req): Json<UpdateSessionTitleRequest>,
) -> Result<Json<SuccessResponse>, StatusCode> {
    let session_id = IdeationSessionId::from_string(req.session_id.clone());

    // Update title in database (MCP agent calls = auto-generated title)
    state
        .app_state
        .ideation_session_repo
        .update_title(&session_id, Some(req.title.clone()), "auto")
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
    for (from, to, _reason) in all_deps {
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
                target_project: p.target_project.clone(),
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
        target_project: proposal.target_project.clone(),
    }))
}

/// Analyze dependencies for a session - returns full graph with critical path, cycles, etc.
/// Used by chat agent to provide intelligent dependency recommendations
pub async fn analyze_session_dependencies(
    State(state): State<HttpServerState>,
    Path(session_id): Path<String>,
) -> Result<Json<super::super::types::AnalyzeDependenciesResponse>, StatusCode> {
    use super::super::types::{
        AnalyzeDependenciesResponse, DependencyAnalysisSummary, DependencyEdgeResponse,
        DependencyNodeResponse,
    };
    use crate::domain::entities::{DependencyGraph, DependencyGraphEdge, DependencyGraphNode};

    let session_id = IdeationSessionId::from_string(session_id.clone());

    // Get all proposals for the session
    let proposals = state
        .app_state
        .task_proposal_repo
        .get_by_session(&session_id)
        .await
        .map_err(|e| {
            error!(
                "Failed to get proposals for session {}: {}",
                session_id.as_str(),
                e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get all dependencies for the session
    let dependencies = state
        .app_state
        .proposal_dependency_repo
        .get_all_for_session(&session_id)
        .await
        .map_err(|e| {
            error!(
                "Failed to get dependencies for session {}: {}",
                session_id.as_str(),
                e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Build adjacency lists
    let mut from_map: HashMap<TaskProposalId, Vec<TaskProposalId>> = HashMap::new();
    let mut to_map: HashMap<TaskProposalId, Vec<TaskProposalId>> = HashMap::new();

    for (from, to, _reason) in &dependencies {
        from_map.entry(from.clone()).or_default().push(to.clone());
        to_map.entry(to.clone()).or_default().push(from.clone());
    }

    // Build nodes with degree counts
    let mut nodes: Vec<DependencyGraphNode> = Vec::new();
    for proposal in &proposals {
        let in_degree = from_map.get(&proposal.id).map(|v| v.len()).unwrap_or(0);
        let out_degree = to_map.get(&proposal.id).map(|v| v.len()).unwrap_or(0);
        let node = DependencyGraphNode::new(proposal.id.clone(), &proposal.title)
            .with_in_degree(in_degree)
            .with_out_degree(out_degree);
        nodes.push(node);
    }

    // Build edges for internal graph operations (reason stored separately in response)
    let edges: Vec<DependencyGraphEdge> = dependencies
        .iter()
        .map(|(from, to, _reason)| DependencyGraphEdge::new(from.clone(), to.clone()))
        .collect();

    // Simple cycle detection using DFS
    let cycles = detect_cycles_simple(&proposals, &from_map);

    // Critical path calculation (longest path)
    let critical_path = if cycles.is_empty() {
        find_critical_path_simple(&proposals, &from_map, &to_map)
    } else {
        Vec::new()
    };

    let mut graph = DependencyGraph::with_nodes_and_edges(nodes.clone(), edges);
    graph.set_critical_path(critical_path.clone());
    graph.set_cycles(cycles.clone());

    // Calculate summary
    let root_count = nodes.iter().filter(|n| n.is_root()).count();
    let leaf_count = nodes.iter().filter(|n| n.is_leaf()).count();

    // Convert to response types
    let response_nodes: Vec<DependencyNodeResponse> = nodes
        .iter()
        .map(|n| DependencyNodeResponse {
            id: n.proposal_id.to_string(),
            title: n.title.clone(),
            in_degree: n.in_degree,
            out_degree: n.out_degree,
            is_root: n.is_root(),
            is_blocker: n.is_blocker(),
        })
        .collect();

    // Build response edges directly from dependencies to include reason
    let response_edges: Vec<DependencyEdgeResponse> = dependencies
        .iter()
        .map(|(from, to, reason)| DependencyEdgeResponse {
            from: from.to_string(),
            to: to.to_string(),
            reason: reason.clone(),
        })
        .collect();

    let response_critical_path: Vec<String> =
        critical_path.iter().map(|id| id.to_string()).collect();

    let response_cycles = if cycles.is_empty() {
        None
    } else {
        Some(
            cycles
                .iter()
                .map(|cycle| cycle.iter().map(|id| id.to_string()).collect())
                .collect(),
        )
    };

    let summary = DependencyAnalysisSummary {
        total_proposals: proposals.len(),
        root_count,
        leaf_count,
        max_depth: critical_path.len(),
    };

    // Mark dependencies as acknowledged: agent reviewed the graph, satisfying the finalize gate.
    // Best-effort — don't fail the response if the flag update fails.
    if let Err(e) = state
        .app_state
        .ideation_session_repo
        .set_dependencies_acknowledged(session_id.as_str())
        .await
    {
        error!(
            "Failed to set dependencies_acknowledged for session {}: {}",
            session_id.as_str(),
            e
        );
    }

    Ok(Json(AnalyzeDependenciesResponse {
        nodes: response_nodes,
        edges: response_edges,
        critical_path: response_critical_path,
        critical_path_length: critical_path.len(),
        has_cycles: !cycles.is_empty(),
        cycles: response_cycles,
        message: None,
        summary,
    }))
}

/// Simple cycle detection using DFS
fn detect_cycles_simple(
    proposals: &[crate::domain::entities::TaskProposal],
    from_map: &HashMap<TaskProposalId, Vec<TaskProposalId>>,
) -> Vec<Vec<TaskProposalId>> {
    use std::collections::HashSet;

    let mut cycles = Vec::new();
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    let mut path = Vec::new();

    for proposal in proposals {
        if !visited.contains(&proposal.id) {
            dfs_detect_cycle_simple(
                &proposal.id,
                from_map,
                &mut visited,
                &mut rec_stack,
                &mut path,
                &mut cycles,
            );
        }
    }

    cycles
}

fn dfs_detect_cycle_simple(
    node: &TaskProposalId,
    from_map: &HashMap<TaskProposalId, Vec<TaskProposalId>>,
    visited: &mut std::collections::HashSet<TaskProposalId>,
    rec_stack: &mut std::collections::HashSet<TaskProposalId>,
    path: &mut Vec<TaskProposalId>,
    cycles: &mut Vec<Vec<TaskProposalId>>,
) {
    visited.insert(node.clone());
    rec_stack.insert(node.clone());
    path.push(node.clone());

    if let Some(neighbors) = from_map.get(node) {
        for neighbor in neighbors {
            if !visited.contains(neighbor) {
                dfs_detect_cycle_simple(neighbor, from_map, visited, rec_stack, path, cycles);
            } else if rec_stack.contains(neighbor) {
                // Found a cycle
                if let Some(start_idx) = path.iter().position(|n| n == neighbor) {
                    let cycle: Vec<TaskProposalId> = path[start_idx..].to_vec();
                    cycles.push(cycle);
                }
            }
        }
    }

    path.pop();
    rec_stack.remove(node);
}

/// Find critical path (longest path) using topological sort + DP
fn find_critical_path_simple(
    proposals: &[crate::domain::entities::TaskProposal],
    from_map: &HashMap<TaskProposalId, Vec<TaskProposalId>>,
    to_map: &HashMap<TaskProposalId, Vec<TaskProposalId>>,
) -> Vec<TaskProposalId> {
    use std::collections::{HashMap as StdHashMap, VecDeque};

    if proposals.is_empty() {
        return Vec::new();
    }

    // Calculate in-degrees
    // For topological sort, in_degree[X] = number of prerequisites for X
    // So in_degree[X] = from_map[X].len() (number of things X depends on)
    let mut in_degree: StdHashMap<TaskProposalId, usize> = StdHashMap::new();
    for proposal in proposals {
        let deps_count = from_map.get(&proposal.id).map(|v| v.len()).unwrap_or(0);
        in_degree.insert(proposal.id.clone(), deps_count);
    }

    // Kahn's algorithm for topological sort
    let mut queue: VecDeque<TaskProposalId> = VecDeque::new();
    for (id, &degree) in &in_degree {
        if degree == 0 {
            queue.push_back(id.clone());
        }
    }

    let mut topo_order = Vec::new();
    let mut remaining_degree = in_degree.clone();

    while let Some(node) = queue.pop_front() {
        topo_order.push(node.clone());

        // For each proposal that depends on this node, decrease its in_degree
        if let Some(dependents) = to_map.get(&node) {
            for dependent in dependents {
                if let Some(degree) = remaining_degree.get_mut(dependent) {
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(dependent.clone());
                    }
                }
            }
        }
    }

    // If not all nodes processed, there's a cycle - return empty
    if topo_order.len() != proposals.len() {
        return Vec::new();
    }

    // DP for longest path
    let mut dist: StdHashMap<TaskProposalId, i32> = StdHashMap::new();
    let mut prev: StdHashMap<TaskProposalId, Option<TaskProposalId>> = StdHashMap::new();

    for id in &topo_order {
        dist.insert(id.clone(), 0);
        prev.insert(id.clone(), None);
    }

    // Process in topological order
    for node in &topo_order {
        if let Some(dependents) = to_map.get(node) {
            for dependent in dependents {
                let new_dist = dist.get(node).unwrap_or(&0) + 1;
                if new_dist > *dist.get(dependent).unwrap_or(&0) {
                    dist.insert(dependent.clone(), new_dist);
                    prev.insert(dependent.clone(), Some(node.clone()));
                }
            }
        }
    }

    // Find end node (max distance)
    let mut max_dist = 0;
    let mut end_node: Option<TaskProposalId> = topo_order.first().cloned();

    for (id, &d) in &dist {
        if d > max_dist {
            max_dist = d;
            end_node = Some(id.clone());
        }
    }

    // Reconstruct path
    let mut path = Vec::new();
    let mut current = end_node;

    while let Some(node) = current {
        path.push(node.clone());
        current = prev.get(&node).and_then(|p| p.clone());
    }

    path.reverse();
    path
}

/// Get messages for an ideation session (context recovery for agents)
/// Returns messages newest-first with optional truncation
pub async fn get_session_messages(
    State(state): State<HttpServerState>,
    Json(req): Json<GetSessionMessagesRequest>,
) -> Result<Json<GetSessionMessagesResponse>, JsonError> {
    let session_id = IdeationSessionId::from_string(req.session_id.clone());

    // Cap limit at 200
    let limit = req.limit.clamp(1, 200);
    let offset = req.offset;

    // Get total count first
    let total_available = state
        .app_state
        .chat_message_repo
        .count_by_session(&session_id)
        .await
        .map_err(|e| {
            error!(
                "Failed to count messages for session {}: {}",
                session_id.as_str(),
                e
            );
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to count messages: {}", e),
            )
        })? as usize;

    // Get messages with offset for pagination support
    let messages = state
        .app_state
        .chat_message_repo
        .get_recent_by_session_paginated(&session_id, limit as u32, offset as u32)
        .await
        .map_err(|e| {
            error!(
                "Failed to get messages for session {}: {}",
                session_id.as_str(),
                e
            );
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get messages: {}", e),
            )
        })?;

    // Convert to response format
    let response_messages: Vec<SessionMessageResponse> = messages
        .into_iter()
        .map(|msg| {
            // If include_tool_calls is false and message has tool_calls,
            // we may want to append a note. For now, just return content.
            // The tool_calls field is excluded from SessionMessageResponse by design.
            SessionMessageResponse {
                role: msg.role.to_string(),
                content: msg.content,
                created_at: msg.created_at.to_rfc3339(),
            }
        })
        .collect();

    let count = response_messages.len();
    let truncated = total_available > limit + offset;

    Ok(Json(GetSessionMessagesResponse {
        messages: response_messages,
        count,
        truncated,
        total_available,
    }))
}

/// Stop any running verification child agents for a session.
///
/// Called when verification is skipped or reverted to immediately release the write lock
/// so the parent session can resume plan editing. Best-effort: errors are swallowed so the
/// caller's skip/revert succeeds even if the agent is already dead.
pub(crate) async fn stop_verification_children(
    session_id: &str,
    app_state: &AppState,
) -> Result<(), AppError> {
    use tauri::Emitter;
    let session_id_typed = IdeationSessionId::from_string(session_id.to_string());
    let children = app_state
        .ideation_session_repo
        .get_verification_children(&session_id_typed)
        .await?;

    for child in &children {
        let key = RunningAgentKey::new("ideation", child.id.as_str());
        if app_state.running_agent_registry.is_running(&key).await {
            if let Ok(Some(info)) = app_state.running_agent_registry.stop(&key).await {
                // Remove from interactive process registry (closes stdin pipe)
                let ipr_key = InteractiveProcessKey::new("ideation", child.id.as_str());
                app_state.interactive_process_registry.remove(&ipr_key).await;

                // Mark agent run as failed
                let run_id =
                    crate::domain::entities::AgentRunId::from_string(&info.agent_run_id);
                app_state
                    .agent_run_repo
                    .fail(&run_id, "Verification cancelled")
                    .await
                    .ok();

                // Emit frontend events
                if let Some(ref app_handle) = app_state.app_handle {
                    app_handle
                        .emit(
                            "agent:stopped",
                            serde_json::json!({
                                "conversation_id": info.conversation_id,
                                "agent_run_id": info.agent_run_id,
                                "context_type": "ideation",
                                "context_id": child.id.as_str(),
                            }),
                        )
                        .ok();
                    app_handle
                        .emit(
                            "agent:run_completed",
                            AgentRunCompletedPayload {
                                conversation_id: info.conversation_id.clone(),
                                context_type: "ideation".to_string(),
                                context_id: child.id.as_str().to_string(),
                                claude_session_id: None,
                                run_chain_id: None,
                            },
                        )
                        .ok();
                }
            }
        }
    }
    Ok(())
}

/// Stop an in-progress verification loop for a session.
///
/// Kills any running verification child agents, sets verification status to `skipped`
/// with `convergence_reason: "user_stopped"`, clears the `verification_in_progress` flag,
/// and increments the verification generation to prevent zombie agents from writing stale state.
///
/// Idempotent: if no verification is in progress, returns 200 with a message.
///
/// Route: `POST /api/ideation/sessions/:id/stop-verification`
pub async fn stop_verification(
    State(state): State<HttpServerState>,
    Path(session_id): Path<String>,
) -> Result<Json<SuccessResponse>, JsonError> {
    use crate::domain::entities::ideation::{VerificationMetadata, VerificationStatus};

    let session_id_obj =
        crate::domain::entities::IdeationSessionId::from_string(session_id.clone());

    // Read session
    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id_obj)
        .await
        .map_err(|e| {
            error!("Failed to get session {}: {}", session_id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get session")
        })?
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "Session not found"))?;

    // Guard: reject calls targeting verification child sessions — orchestrators must use parent session_id
    if session.session_purpose == crate::domain::entities::SessionPurpose::Verification {
        return Err(json_error(
            StatusCode::BAD_REQUEST,
            "Cannot stop verification on a verification child session. Use the parent session_id.",
        ));
    }

    // Session must be active
    if !session.is_active() {
        return Err(json_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Session is not active",
        ));
    }

    // Guard: external sessions cannot stop plan verification
    if session.origin == crate::domain::entities::ideation::SessionOrigin::External {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "External sessions cannot stop plan verification.",
        ));
    }

    // Idempotent: if no verification is running, return 200 without doing anything
    if !session.verification_in_progress {
        return Ok(Json(SuccessResponse {
            success: true,
            message: "Verification is not in progress".to_string(),
        }));
    }

    // Kill any running verification child agents (best-effort)
    stop_verification_children(&session_id, &state.app_state).await.ok();

    // Update metadata: preserve existing metadata and set convergence_reason = "user_stopped"
    let mut metadata: VerificationMetadata = session
        .verification_metadata
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    metadata.convergence_reason = Some("user_stopped".to_string());
    let metadata_json = serde_json::to_string(&metadata).ok();

    // Persist: verification_status = skipped, verification_in_progress = false
    state
        .app_state
        .ideation_session_repo
        .update_verification_state(&session_id_obj, VerificationStatus::Skipped, false, metadata_json)
        .await
        .map_err(|e| {
            error!(
                "Failed to update verification state for {}: {}",
                session_id, e
            );
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to stop verification",
            )
        })?;

    tracing::info!(
        session_id = %session_id,
        "Verification stopped by user"
    );

    // Increment generation to prevent zombie verifier from writing stale terminal status
    state
        .app_state
        .ideation_session_repo
        .increment_verification_generation(&session_id_obj)
        .await
        .ok();

    // Emit plan_verification:status_changed event so frontend VerificationBadge updates
    if let Some(app_handle) = &state.app_state.app_handle {
        emit_verification_status_changed(
            app_handle,
            &session_id,
            VerificationStatus::Skipped,
            false,
            Some(&metadata),
            Some("user_stopped"),
            Some(session.verification_generation),
        );
    }

    Ok(Json(SuccessResponse {
        success: true,
        message: "Verification stopped".to_string(),
    }))
}

/// Send an `<auto-propose>` message to the orchestrator agent for external sessions
/// that reached verification convergence via `zero_blocking`.
///
/// Retries up to 3 times with exponential backoff (1s/2s/4s between retries).
/// On final failure: emits `ideation:auto_propose_failed` to the external_events table.
async fn auto_propose_for_external(
    session_id: &str,
    session: &crate::domain::entities::ideation::IdeationSession,
    state: &HttpServerState,
) {
    use crate::domain::entities::ideation::SessionOrigin;
    if session.origin != SessionOrigin::External {
        return;
    }

    // Transition external activity phase to "proposing" (fire-and-forget)
    {
        let repo = std::sync::Arc::clone(&state.app_state.ideation_session_repo);
        let sid = crate::domain::entities::IdeationSessionId::from_string(session_id.to_string());
        tokio::spawn(async move {
            if let Err(e) = repo.update_external_activity_phase(&sid, "proposing").await {
                tracing::error!("Failed to set activity phase 'proposing' for session {}: {}", sid.as_str(), e);
            }
        });
    }

    let is_team_mode = session_is_team_mode(session);
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
    chat_service = chat_service.with_team_mode(is_team_mode);

    let project_id = session.project_id.as_str().to_string();
    auto_propose_with_retry(
        session_id,
        &project_id,
        &chat_service,
        Arc::clone(&app.external_events_repo),
        &[1_000, 2_000, 4_000],
    )
    .await;

    // Set "ready" phase after proposing completes
    {
        let repo_ready = std::sync::Arc::clone(&state.app_state.ideation_session_repo);
        let sid_ready =
            crate::domain::entities::IdeationSessionId::from_string(session_id.to_string());
        if let Err(e) = repo_ready
            .update_external_activity_phase(&sid_ready, "ready")
            .await
        {
            tracing::error!(
                "Failed to set activity phase 'ready' for session {}: {}",
                session_id,
                e
            );
        }
    }
}

/// Core retry logic for auto-propose delivery.
///
/// Attempts delivery up to `retry_delays_ms.len() + 1` times. Between each retry,
/// sleeps for the corresponding duration from `retry_delays_ms`. On final failure,
/// writes an `ideation:auto_propose_failed` row to the `external_events` table.
///
/// # Test usage
/// Pass `retry_delays_ms = &[0, 0, 0]` to eliminate sleep delays in tests.
#[doc(hidden)]
pub async fn auto_propose_with_retry(
    session_id: &str,
    project_id: &str,
    chat_service: &dyn ChatService,
    external_events_repo: Arc<dyn ExternalEventsRepository>,
    retry_delays_ms: &[u64],
) {
    let message = "<auto-propose>\nThe plan has been verified with zero blocking gaps (convergence: zero_blocking).\nThis is an external MCP session. Auto-propose triggered.\n</auto-propose>";
    let max_attempts = retry_delays_ms.len() + 1;
    let mut last_error: Option<String> = None;

    for attempt in 0..max_attempts {
        match chat_service
            .send_message(
                ChatContextType::Ideation,
                session_id,
                message,
                SendMessageOptions::default(),
            )
            .await
        {
            Ok(result) => {
                tracing::info!(
                    session_id = %session_id,
                    attempt = attempt + 1,
                    delivery_status = if result.was_queued { "queued" } else { "spawned" },
                    "auto_propose_for_external: message delivered"
                );
                // TODO(webhook): emit EventType::IdeationAutoProposeSent via AppState::webhook_publisher
                // when AppState gains that field. Payload: { session_id, project_id }.
                return;
            }
            Err(e) => {
                let err_str = e.to_string();
                tracing::warn!(
                    session_id = %session_id,
                    attempt = attempt + 1,
                    max_attempts = max_attempts,
                    error = %err_str,
                    "auto_propose_for_external: send attempt failed"
                );
                last_error = Some(err_str);
                if attempt < retry_delays_ms.len() {
                    tokio::time::sleep(std::time::Duration::from_millis(retry_delays_ms[attempt]))
                        .await;
                }
            }
        }
    }

    // All attempts exhausted — emit failure event to external_events table.
    let error_msg = last_error.unwrap_or_else(|| "unknown error".to_string());
    tracing::error!(
        session_id = %session_id,
        max_attempts = max_attempts,
        error = %error_msg,
        "auto_propose_for_external: all retry attempts exhausted, emitting failure event"
    );
    let payload = serde_json::json!({
        "session_id": session_id,
        "project_id": project_id,
        "error": error_msg,
    });
    if let Err(insert_err) = external_events_repo
        .insert_event(
            "ideation:auto_propose_failed",
            project_id,
            &payload.to_string(),
        )
        .await
    {
        tracing::warn!(
            session_id = %session_id,
            error = %insert_err,
            "auto_propose_for_external: failed to persist failure event (non-fatal)"
        );
    }
    // TODO(webhook): emit EventType::IdeationAutoProposeFailed via AppState::webhook_publisher
    // when AppState gains that field. Payload: { session_id, project_id, error }.
}

/// POST /api/ideation/sessions/:id/verification
///
/// Update verification state for a session's plan (from MCP orchestrator).
/// Validates the state machine transition and persists gap metadata.
pub async fn update_plan_verification(
    State(state): State<HttpServerState>,
    Path(session_id): Path<String>,
    Json(req): Json<UpdateVerificationRequest>,
) -> Result<Json<VerificationResponse>, JsonError> {
    use std::collections::HashSet;
    use crate::domain::entities::ideation::{
        VerificationGap, VerificationMetadata, VerificationRound, VerificationStatus,
    };
    use crate::domain::services::{gap_fingerprint, gap_score, jaccard_similarity};

    let session_id_obj = crate::domain::entities::IdeationSessionId::from_string(session_id.clone());

    // Fetch session
    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id_obj)
        .await
        .map_err(|e| {
            error!("Failed to get session {}: {}", session_id, e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get session: {}", e),
            )
        })?
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "Session not found"))?;

    // Guard: reject calls targeting verification child sessions — plan-verifier must use parent session_id
    if session.session_purpose == crate::domain::entities::SessionPurpose::Verification {
        return Err(json_error(
            StatusCode::BAD_REQUEST,
            "Cannot update verification state on a verification child session. Use the parent session_id.",
        ));
    }

    // Server-side generation guard: when generation is provided, verify it matches.
    // Applies to ALL calls (including terminal in_progress=false) to prevent zombie agents
    // from writing stale terminal status (e.g., verified/needs_revision after a reset).
    if let Some(req_gen) = req.generation {
        if req_gen != session.verification_generation {
            return Err(json_error(
                StatusCode::CONFLICT,
                format!(
                    "Generation mismatch: request generation {} != current generation {}. \
                     Verification was reset — zombie agent detected.",
                    req_gen, session.verification_generation
                ),
            ));
        }
    }

    // Parse new status (mut — server-side convergence conditions may override)
    let mut new_status: VerificationStatus = req.status.parse().map_err(|_| {
        json_error(
            StatusCode::BAD_REQUEST,
            format!("Invalid status: {}", req.status),
        )
    })?;
    // in_progress may be overridden by condition 6 (reviewing+gaps → needs_revision)
    let mut effective_in_progress = req.in_progress;

    // Guard: external sessions cannot skip plan verification
    if new_status == VerificationStatus::Skipped
        && session.origin == crate::domain::entities::ideation::SessionOrigin::External
    {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "External sessions cannot skip plan verification. Run verification to completion (update_plan_verification with status 'reviewing').",
        ));
    }

    // Transition validation matrix
    let current = session.verification_status;
    let has_convergence_reason = req.convergence_reason.is_some();
    let is_valid = match (current, new_status) {
        (_, VerificationStatus::Skipped) => true,
        // Skipped can transition to Reviewing to allow users to verify after skipping
        (VerificationStatus::Skipped, VerificationStatus::Reviewing) => true,
        (VerificationStatus::Skipped, _) => false,
        (VerificationStatus::Unverified, VerificationStatus::Reviewing) => true,
        (VerificationStatus::Reviewing, VerificationStatus::NeedsRevision) => true,
        (VerificationStatus::Reviewing, VerificationStatus::Verified) => true,
        (VerificationStatus::NeedsRevision, VerificationStatus::Reviewing) => true,
        // Allow needs_revision → verified ONLY when convergence_reason is provided
        (VerificationStatus::NeedsRevision, VerificationStatus::Verified) => has_convergence_reason,
        // ImportedVerified can transition to Reviewing to re-run verification if desired
        (VerificationStatus::ImportedVerified, VerificationStatus::Reviewing) => true,
        // Verified can transition to Reviewing to re-run verification
        (VerificationStatus::Verified, VerificationStatus::Reviewing) => true,
        _ => false,
    };

    if !is_valid {
        if matches!(current, VerificationStatus::Skipped) {
            return Err(json_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Verification was skipped — cannot update from critic",
            ));
        }
        if matches!(
            (current, new_status),
            (VerificationStatus::NeedsRevision, VerificationStatus::Verified)
        ) {
            return Err(json_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Cannot transition needs_revision → verified without convergence_reason",
            ));
        }
        return Err(json_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!(
                "Invalid verification transition: {} → {}",
                current, new_status
            ),
        ));
    }

    // Re-verify fast path: terminal → Reviewing (Verified, Skipped, ImportedVerified)
    // Atomically clears stale metadata + increments generation + sets Reviewing+in_progress.
    // Skips update_verification_state entirely — reset_and_begin_reverify does everything.
    let is_reverify = matches!(
        current,
        VerificationStatus::Verified
            | VerificationStatus::Skipped
            | VerificationStatus::ImportedVerified
    ) && new_status == VerificationStatus::Reviewing;

    if is_reverify {
        let (new_gen, cleared_metadata) = state
            .app_state
            .ideation_session_repo
            .reset_and_begin_reverify(&session_id)
            .await
            .map_err(|e| {
                error!("Failed to reset verification for {}: {}", session_id, e);
                json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to reset verification state",
                )
            })?;

        tracing::info!(
            session_id = %session_id,
            from_status = %current,
            new_gen = new_gen,
            "Re-verify: stale metadata cleared, generation incremented"
        );

        if let Some(app_handle) = &state.app_state.app_handle {
            emit_verification_status_changed(
                app_handle,
                &session_id,
                VerificationStatus::Reviewing,
                true,
                Some(&cleared_metadata),
                None,
                Some(new_gen),
            );
        }

        return Ok(Json(VerificationResponse {
            session_id,
            status: VerificationStatus::Reviewing.to_string(),
            in_progress: true,
            current_round: None,
            max_rounds: None,
            gap_score: Some(0),
            convergence_reason: None,
            best_round_index: None,
            current_gaps: vec![],
            rounds: vec![],
            plan_version: None,
            verification_generation: new_gen,
        }));
    }

    // Build/update metadata
    let mut metadata: VerificationMetadata = session
        .verification_metadata
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    if let Some(max_r) = req.max_rounds {
        metadata.max_rounds = max_r;
    }

    // Process gaps if provided
    if let Some(ref gaps_req) = req.gaps {
        let gaps: Vec<VerificationGap> = gaps_req
            .iter()
            .map(|g| VerificationGap {
                severity: g.severity.clone(),
                category: g.category.clone(),
                description: g.description.clone(),
                why_it_matters: g.why_it_matters.clone(),
                source: g.source.clone(),
            })
            .collect();

        let fingerprints: Vec<String> = gaps
            .iter()
            .map(|g| gap_fingerprint(&g.description))
            .collect();
        let score = gap_score(&gaps);

        if let Some(round) = req.round {
            metadata.current_round = round;
        }

        // ── Server-side convergence evaluation (D3) ──
        // Evaluate before pushing new round — metadata.current_gaps = previous round's gaps.

        // Condition 1: 0 critical AND 0 high AND 0 medium (zero_blocking, AD3)
        let critical_count = gaps_req.iter().filter(|g| g.severity == "critical").count() as u32;
        let high_count = gaps_req.iter().filter(|g| g.severity == "high").count() as u32;
        let medium_count = gaps_req.iter().filter(|g| g.severity == "medium").count() as u32;
        let zero_blocking_converged = critical_count == 0 && high_count == 0 && medium_count == 0;

        // Condition 2: Jaccard ≥ 0.8 for 2 consecutive rounds (R4-C2)
        let jaccard_converged = if metadata.rounds.len() >= 2 {
            let prev_round = metadata.rounds.last().unwrap();
            let prev_prev_round = &metadata.rounds[metadata.rounds.len() - 2];
            let new_fp_set: HashSet<String> = fingerprints.iter().cloned().collect();
            let prev_fp_set: HashSet<String> = prev_round.fingerprints.iter().cloned().collect();
            let prev_prev_fp_set: HashSet<String> =
                prev_prev_round.fingerprints.iter().cloned().collect();
            let jaccard_curr = jaccard_similarity(&new_fp_set, &prev_fp_set);
            let jaccard_prev = jaccard_similarity(&prev_fp_set, &prev_prev_fp_set);
            tracing::info!(
                session_id = %session_id,
                round = metadata.current_round,
                jaccard_curr = jaccard_curr,
                jaccard_prev = jaccard_prev,
                "Verification Jaccard similarity (2-round check)"
            );
            jaccard_curr >= 0.8 && jaccard_prev >= 0.8
        } else if metadata.rounds.len() == 1 {
            let prev_round = metadata.rounds.last().unwrap();
            let new_fp_set: HashSet<String> = fingerprints.iter().cloned().collect();
            let prev_fp_set: HashSet<String> = prev_round.fingerprints.iter().cloned().collect();
            let jaccard = jaccard_similarity(&new_fp_set, &prev_fp_set);
            tracing::info!(
                session_id = %session_id,
                round = metadata.current_round,
                jaccard = jaccard,
                "Verification Jaccard similarity (need 2 consecutive rounds for convergence)"
            );
            false // need at least 2 consecutive rounds
        } else {
            false
        };

        // Track best version (lowest gap_score)
        let round_idx = metadata.rounds.len() as u32;
        let is_better = metadata.best_round_index.is_none() || {
            let best_idx = metadata.best_round_index.unwrap() as usize;
            metadata
                .rounds
                .get(best_idx)
                .map(|r| r.gap_score)
                .unwrap_or(u32::MAX)
                > score
        };
        if is_better {
            metadata.best_round_index = Some(round_idx);
        }

        metadata
            .rounds
            .push(VerificationRound { fingerprints, gap_score: score });
        metadata.current_gaps = gaps;

        // Auto-converge: override NeedsRevision → Verified when conditions are met
        if new_status == VerificationStatus::NeedsRevision {
            // R1 empty round guard: require at least round 2 before zero_blocking convergence.
            // Round 1 with 0 gaps may be a broken critic — need round 2 to confirm.
            let current_round_for_convergence = req.round.unwrap_or(metadata.current_round);
            if zero_blocking_converged && current_round_for_convergence >= 2 {
                new_status = VerificationStatus::Verified;
                if metadata.convergence_reason.is_none() {
                    metadata.convergence_reason = Some("zero_blocking".to_string());
                }
                tracing::info!(
                    session_id = %session_id,
                    round = current_round_for_convergence,
                    "Server-side convergence: 0 critical + 0 high + 0 medium → Verified"
                );
            } else if jaccard_converged {
                new_status = VerificationStatus::Verified;
                if metadata.convergence_reason.is_none() {
                    metadata.convergence_reason = Some("jaccard_converged".to_string());
                }
                tracing::info!(
                    session_id = %session_id,
                    "Server-side convergence: Jaccard ≥ 0.8 × 2 rounds → Verified"
                );
            }
        }
    }

    // Condition 3: max_rounds hard cap (R4-H3)
    if !matches!(new_status, VerificationStatus::Verified | VerificationStatus::Skipped) {
        let current_round = req.round.unwrap_or(metadata.current_round);
        if metadata.max_rounds > 0 && current_round >= metadata.max_rounds {
            new_status = VerificationStatus::Verified;
            if metadata.convergence_reason.is_none() {
                metadata.convergence_reason = Some("max_rounds".to_string());
            }
            tracing::info!(
                session_id = %session_id,
                round = current_round,
                max_rounds = metadata.max_rounds,
                "Server-side convergence: max_rounds reached → Verified"
            );
        }
    }

    // Condition 4: parse failure tracking — sliding window ≥ 3 of last 5 rounds (R4-M3)
    if req.parse_failed == Some(true) {
        if let Some(round) = req.round {
            metadata.parse_failures.push(round);
        }
        let last_5_failures = metadata.parse_failures.iter().rev().take(5).count();
        if last_5_failures >= 3
            && !matches!(new_status, VerificationStatus::Verified | VerificationStatus::Skipped)
        {
            new_status = VerificationStatus::Verified;
            if metadata.convergence_reason.is_none() {
                metadata.convergence_reason = Some("critic_parse_failure".to_string());
            }
            tracing::warn!(
                session_id = %session_id,
                failures = last_5_failures,
                "Server-side convergence: critic parse failures ≥ 3/5 → Verified"
            );
        }
    }

    if let Some(ref reason) = req.convergence_reason {
        // Orchestrator-provided reason takes precedence only if not already set server-side
        if metadata.convergence_reason.is_none() {
            metadata.convergence_reason = Some(reason.clone());
        }
    }

    // Condition 6: reviewing with gaps → needs_revision (auto-override, placed after convergence
    // checks so convergence always takes priority). Triggers on ANY gap severity.
    // TODO: Extract auto-transition logic to domain service state machine
    if new_status == VerificationStatus::Reviewing && !metadata.current_gaps.is_empty() {
        new_status = VerificationStatus::NeedsRevision;
        effective_in_progress = false;
        tracing::info!(
            session_id = %session_id,
            gap_count = metadata.current_gaps.len(),
            "Server-side auto-transition: reviewing with gaps → NeedsRevision"
        );
    }

    let current_gap_score = gap_score(&metadata.current_gaps);
    let metadata_json = serde_json::to_string(&metadata).ok();

    // Persist state
    state
        .app_state
        .ideation_session_repo
        .update_verification_state(&session_id_obj, new_status, effective_in_progress, metadata_json)
        .await
        .map_err(|e| {
            error!(
                "Failed to update verification state for {}: {}",
                session_id, e
            );
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update verification state",
            )
        })?;

    tracing::info!(
        session_id = %session_id,
        status = %new_status,
        round = ?req.round,
        "Verification state updated"
    );

    // For terminal statuses, kill any running verification child agents before emitting events.
    // This releases the write lock so the parent can immediately resume plan editing.
    if matches!(new_status, VerificationStatus::Verified | VerificationStatus::Skipped) {
        stop_verification_children(&session_id, &state.app_state).await.ok();
    }

    // Defense-in-depth: increment generation on skip so any in-flight zombie agent
    // calls get rejected with 409 Conflict.
    if matches!(new_status, VerificationStatus::Skipped) {
        state
            .app_state
            .ideation_session_repo
            .increment_verification_generation(&session_id_obj)
            .await
            .ok();
    }

    // Emit plan_verification:status_changed event (B1: includes current_gaps + last 5 rounds)
    if let Some(app_handle) = &state.app_state.app_handle {
        emit_verification_status_changed(
            app_handle,
            &session_id,
            new_status,
            effective_in_progress,
            Some(&metadata),
            None,
            Some(session.verification_generation),
        );
    }

    // TODO(webhook): emit EventType::IdeationVerified via AppState::webhook_publisher when
    // new_status == Verified and AppState gains that field.
    // Payload: { session_id, project_id, convergence_reason }.

    // Auto-propose for external sessions that converged via zero_blocking
    if new_status == VerificationStatus::Verified
        && metadata.convergence_reason.as_deref() == Some("zero_blocking")
        && session.origin == crate::domain::entities::ideation::SessionOrigin::External
    {
        auto_propose_for_external(&session_id, &session, &state).await;
    }

    // For external sessions that reach Verified WITHOUT auto-propose (non-zero_blocking):
    // transition to "ready" immediately since there's no proposing phase
    if new_status == VerificationStatus::Verified
        && session.origin == crate::domain::entities::ideation::SessionOrigin::External
        && metadata.convergence_reason.as_deref() != Some("zero_blocking")
    {
        let repo = std::sync::Arc::clone(&state.app_state.ideation_session_repo);
        let sid = crate::domain::entities::IdeationSessionId::from_string(session_id.clone());
        tokio::spawn(async move {
            if let Err(e) = repo.update_external_activity_phase(&sid, "ready").await {
                error!(
                    "Failed to set activity phase 'ready' for session {}: {}",
                    sid.as_str(),
                    e
                );
            }
        });
    }

    use crate::http_server::types::{VerificationGapResponse, VerificationRoundSummary};

    let post_current_gaps = metadata
        .current_gaps
        .iter()
        .map(|g| VerificationGapResponse {
            severity: g.severity.clone(),
            category: g.category.clone(),
            description: g.description.clone(),
            why_it_matters: g.why_it_matters.clone(),
            source: g.source.clone(),
        })
        .collect::<Vec<_>>();

    let post_rounds = metadata
        .rounds
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
        .collect::<Vec<_>>();

    Ok(Json(VerificationResponse {
        session_id,
        status: new_status.to_string(),
        in_progress: effective_in_progress,
        current_round: if metadata.current_round > 0 {
            Some(metadata.current_round)
        } else {
            None
        },
        max_rounds: if metadata.max_rounds > 0 {
            Some(metadata.max_rounds)
        } else {
            None
        },
        gap_score: Some(current_gap_score),
        convergence_reason: metadata.convergence_reason,
        best_round_index: metadata.best_round_index,
        current_gaps: post_current_gaps,
        rounds: post_rounds,
        plan_version: None,
        verification_generation: session.verification_generation,
    }))
}

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

    // D9: optional scope enforcement — if header present, check project access
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

    let current_round = metadata
        .as_ref()
        .and_then(|m| if m.current_round > 0 { Some(m.current_round) } else { None });
    let max_rounds = metadata
        .as_ref()
        .and_then(|m| if m.max_rounds > 0 { Some(m.max_rounds) } else { None });
    let gap_sc = metadata.as_ref().map(|m| gap_score(&m.current_gaps));
    let convergence_reason = metadata.as_ref().and_then(|m| m.convergence_reason.clone());
    let best_round_index = metadata.as_ref().and_then(|m| m.best_round_index);

    // Map current gaps to response structs
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

    // Map round history — take last 10 in chronological order
    // Rust vec is append-only (newest last); rev().take(10).rev() → last 10 oldest-first
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

    // Resolve plan_version and verification_generation: get the session's fields
    let (plan_version, verification_generation) = {
        let session = state
            .app_state
            .ideation_session_repo
            .get_by_id(&session_id_obj)
            .await
            .ok()
            .flatten();
        let gen = session
            .as_ref()
            .map(|s| s.verification_generation)
            .unwrap_or(0);
        let pv = if let Some(ref s) = session {
            if let Some(ref artifact_id) = s.plan_artifact_id {
                state
                    .app_state
                    .artifact_repo
                    .get_by_id(artifact_id)
                    .await
                    .ok()
                    .flatten()
                    .map(|a| a.metadata.version)
            } else {
                None
            }
        } else {
            None
        };
        (pv, gen)
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
    }))
}

/// POST /api/ideation/sessions/:id/revert-and-skip
///
/// Atomically revert plan content to a previous version and skip verification.
/// Both the artifact INSERT and session UPDATE happen in a single `db.run(|conn| { ... })`
/// transaction — no partial failure where artifact is created but session update fails.
pub async fn revert_and_skip(
    State(state): State<HttpServerState>,
    Path(session_id): Path<String>,
    Json(req): Json<RevertAndSkipRequest>,
) -> Result<Json<SuccessResponse>, JsonError> {
    use crate::domain::entities::ideation::VerificationStatus;
    use crate::domain::entities::{ArtifactContent, ArtifactId};

    let session_id_obj =
        crate::domain::entities::IdeationSessionId::from_string(session_id.clone());

    // Read session
    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id_obj)
        .await
        .map_err(|e| {
            error!("Failed to get session {}: {}", session_id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get session")
        })?
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "Session not found"))?;

    // Guard: external sessions cannot skip plan verification
    if session.origin == crate::domain::entities::ideation::SessionOrigin::External {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "External sessions cannot skip plan verification. Run verification to completion (update_plan_verification with status 'reviewing').",
        ));
    }

    // Session must be active
    if !session.is_active() {
        return Err(json_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Session is not active",
        ));
    }

    // Read the plan artifact version to restore
    let restore_artifact_id = ArtifactId::from_string(req.plan_version_to_restore.clone());
    let artifact = state
        .app_state
        .artifact_repo
        .get_by_id(&restore_artifact_id)
        .await
        .map_err(|e| {
            error!(
                "Failed to get artifact {}: {}",
                req.plan_version_to_restore, e
            );
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get plan artifact",
            )
        })?
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "Plan artifact not found"))?;

    // Extract inline text content (plan artifacts must be inline)
    let content_text = match &artifact.content {
        ArtifactContent::Inline { text } => text.clone(),
        ArtifactContent::File { .. } => {
            return Err(json_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Plan artifact must be inline text content",
            ));
        }
    };

    // Pre-generate artifact ID for logging before the atomic operation
    let new_artifact_id = ArtifactId::new();
    let new_artifact_id_str = new_artifact_id.as_str().to_string();
    let new_version = artifact.metadata.version + 1;

    // Single atomic operation: INSERT artifact + UPDATE session in one db.run() transaction.
    // Prevents the race where artifact is created but session update fails.
    state
        .app_state
        .ideation_session_repo
        .revert_plan_and_skip_with_artifact(
            &session_id_obj,
            new_artifact_id_str.clone(),
            artifact.artifact_type.to_string(),
            artifact.name.clone(),
            content_text,
            new_version,
            restore_artifact_id.as_str().to_string(),
            "user_reverted".to_string(),
        )
        .await
        .map_err(|e| {
            error!("Failed revert-and-skip for session {}: {}", session_id, e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to atomically revert plan and skip verification",
            )
        })?;

    tracing::info!(
        session_id = %session_id,
        plan_version = %req.plan_version_to_restore,
        new_artifact_id = %new_artifact_id_str,
        "Revert-and-skip completed atomically"
    );

    // Kill any running verification child agents before emitting events.
    // Generation increment is handled inside the atomic SQL transaction above.
    stop_verification_children(&session_id, &state.app_state).await.ok();

    // Emit event with canonical payload (B3: was missing round/gaps/rounds fields)
    if let Some(app_handle) = &state.app_state.app_handle {
        emit_verification_status_changed(
            app_handle,
            &session_id,
            VerificationStatus::Skipped,
            false,
            None,
            Some("user_reverted"),
            Some(session.verification_generation),
        );
    }

    Ok(Json(SuccessResponse {
        success: true,
        message: "Plan reverted and verification skipped".to_string(),
    }))
}

// GET /api/ideation/sessions/:id/child-status?include_messages=true&message_limit=10
pub async fn get_child_session_status_handler(
    State(state): State<HttpServerState>,
    Path(session_id): Path<String>,
    Query(params): Query<super::super::types::ChildSessionStatusParams>,
) -> Result<Json<super::super::types::ChildSessionStatusResponse>, JsonError> {
    use crate::domain::entities::ideation::{VerificationMetadata, VerificationStatus};
    use crate::domain::entities::IdeationSessionId;
    use crate::domain::services::RunningAgentKey;
    use crate::infrastructure::agents::claude::ideation_activity_threshold_secs;
    use super::super::types::{
        AgentStateInfo, ChatMessageSummary, ChildSessionStatusResponse, IdeationSessionSummary,
        VerificationInfo,
    };

    let session_id_obj = IdeationSessionId::from_string(session_id.clone());

    // Step 1: Fetch session — 404 if not found
    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id_obj)
        .await
        .map_err(|e| {
            error!("Failed to get session {}: {}", session_id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get session")
        })?
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "Session not found"))?;

    // Step 2: Check RunningAgentRegistry under both "session" and "ideation" keys.
    // Ideation sessions can be registered under either key depending on how they were spawned.
    let session_key = RunningAgentKey::new("session", &session_id);
    let ideation_key = RunningAgentKey::new("ideation", &session_id);
    let registry = &state.app_state.running_agent_registry;

    let agent_info = if let Some(info) = registry.get(&session_key).await {
        Some(info)
    } else {
        registry.get(&ideation_key).await
    };

    // Step 3: Derive estimated_status from last_active_at using config threshold
    let threshold_secs = ideation_activity_threshold_secs();
    let agent_state = match &agent_info {
        None => AgentStateInfo {
            is_running: false,
            started_at: None,
            last_active_at: None,
            pid: None,
            estimated_status: "idle".to_string(),
        },
        Some(info) => {
            let estimated_status = if let Some(last_active) = info.last_active_at {
                let elapsed = chrono::Utc::now()
                    .signed_duration_since(last_active)
                    .num_seconds();
                if elapsed >= 0 && (elapsed as u64) < threshold_secs {
                    "likely_generating"
                } else {
                    "likely_waiting"
                }
            } else {
                // In registry but no heartbeat yet — assume still generating
                "likely_generating"
            };
            AgentStateInfo {
                is_running: true,
                started_at: Some(info.started_at.to_rfc3339()),
                last_active_at: info.last_active_at.map(|t| t.to_rfc3339()),
                pid: Some(info.pid),
                estimated_status: estimated_status.to_string(),
            }
        }
    };

    // Step 4: Optionally fetch recent messages, clamped to max 50
    let recent_messages = if params.include_messages.unwrap_or(false) {
        let limit = u32::min(params.message_limit.unwrap_or(5), 50);
        let messages = state
            .app_state
            .chat_message_repo
            .get_recent_by_session(&session_id_obj, limit)
            .await
            .map_err(|e| {
                error!("Failed to get messages for session {}: {}", session_id, e);
                json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get messages")
            })?;
        Some(
            messages
                .into_iter()
                .map(|m| {
                    // Truncate content to 500 chars (char-boundary safe)
                    let content = m.content.chars().take(500).collect::<String>();
                    ChatMessageSummary {
                        role: m.role.to_string(),
                        content,
                        created_at: m.created_at.to_rfc3339(),
                    }
                })
                .collect::<Vec<_>>(),
        )
    } else {
        None
    };

    // Step 5: Build VerificationInfo from session entity if verification has been started.
    // gap_score and current_round live in the verification_metadata JSON blob.
    // Malformed JSON → return verification: None (no panic).
    let verification = if session.verification_status != VerificationStatus::Unverified {
        let (current_round, gap_score) = if let Some(meta_json) = &session.verification_metadata {
            match serde_json::from_str::<VerificationMetadata>(meta_json) {
                Ok(meta) => {
                    let round = if meta.current_round > 0 {
                        Some(meta.current_round)
                    } else {
                        None
                    };
                    let score = meta.rounds.last().map(|r| r.gap_score);
                    (round, score)
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to parse verification_metadata for session {}: {}",
                        session_id,
                        e
                    );
                    (None, None)
                }
            }
        } else {
            (None, None)
        };
        Some(VerificationInfo {
            status: session.verification_status.to_string(),
            generation: session.verification_generation,
            current_round,
            gap_score,
        })
    } else {
        None
    };

    // Step 6: Build IdeationSessionSummary from session entity fields
    let session_summary = IdeationSessionSummary {
        id: session.id.as_str().to_string(),
        title: session.title.clone().unwrap_or_default(),
        status: session.status.to_string(),
        session_purpose: Some(session.session_purpose.to_string()),
        parent_session_id: session
            .parent_session_id
            .as_ref()
            .map(|id| id.as_str().to_string()),
        created_at: session.created_at.to_rfc3339(),
        updated_at: session.updated_at.to_rfc3339(),
    };

    Ok(Json(ChildSessionStatusResponse {
        session: session_summary,
        agent_state,
        verification,
        recent_messages,
    }))
}

/// POST /api/ideation/sessions/:id/message
///
/// Tri-state delivery:
/// 1. "sent"    — interactive process open; message written directly to stdin
/// 2. "queued"  — agent running but no open stdin; message queued for resume
/// 3. "spawned" — no agent running; new agent process spawned with the message
pub async fn send_ideation_session_message_handler(
    State(state): State<HttpServerState>,
    Path(session_id): Path<String>,
    Json(req): Json<SendSessionMessageRequest>,
) -> Result<Json<SendSessionMessageResponse>, JsonError> {
    // Step 1: Validate session exists
    let sid = IdeationSessionId::from_string(session_id.clone());
    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&sid)
        .await
        .map_err(|e| {
            error!("Failed to get ideation session {}: {}", session_id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get ideation session")
        })?
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "Session not found"))?;

    // Validate session status (enum comparison, not string per CLAUDE.md rule #5)
    if session.status != IdeationSessionStatus::Active {
        return Err(json_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Session is not active",
        ));
    }

    // Validate message length
    if req.message.is_empty() {
        return Err(json_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Message cannot be empty",
        ));
    }
    if req.message.len() > 10_000 {
        return Err(json_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Message too long (max 10000 chars)",
        ));
    }

    // Step 2: Try interactive process registry under BOTH context type keys.
    // Ideation sessions can be registered under "session" (HTTP-spawned) or "ideation" (Tauri IPC-spawned).
    for context_type in &["session", "ideation"] {
        let ipr_key = InteractiveProcessKey {
            context_type: context_type.to_string(),
            context_id: session_id.clone(),
        };
        if state
            .app_state
            .interactive_process_registry
            .has_process(&ipr_key)
            .await
        {
            match state
                .app_state
                .interactive_process_registry
                .write_message(&ipr_key, &req.message)
                .await
            {
                Ok(()) => {
                    return Ok(Json(SendSessionMessageResponse {
                        delivery_status: "sent".to_string(),
                        conversation_id: None,
                    }));
                }
                Err(e) => {
                    // Process may have closed between has_process and write_message; fall through
                    error!(
                        "Failed to write to interactive process for session {} ({}): {}",
                        session_id, context_type, e
                    );
                }
            }
        }
    }

    // Step 3: Queue if agent running (check both keys)
    for context_type in &["session", "ideation"] {
        let agent_key = RunningAgentKey::new(*context_type, &session_id);
        if state
            .app_state
            .running_agent_registry
            .is_running(&agent_key)
            .await
        {
            state
                .app_state
                .message_queue
                .queue(ChatContextType::Ideation, &session_id, req.message.clone());
            return Ok(Json(SendSessionMessageResponse {
                delivery_status: "queued".to_string(),
                conversation_id: None,
            }));
        }
    }

    // Step 4: Agent not running — construct ClaudeChatService and spawn.
    // Follows session_linking.rs:312-330 positional constructor pattern exactly.
    let is_team_mode = session_is_team_mode(&session);
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
    chat_service = chat_service.with_team_mode(is_team_mode);

    match chat_service
        .send_message(
            ChatContextType::Ideation,
            &session_id,
            &req.message,
            SendMessageOptions::default(),
        )
        .await
    {
        Ok(result) if result.was_queued => Ok(Json(SendSessionMessageResponse {
            delivery_status: "queued".to_string(),
            conversation_id: None,
        })),
        Ok(result) => Ok(Json(SendSessionMessageResponse {
            delivery_status: "spawned".to_string(),
            conversation_id: Some(result.conversation_id),
        })),
        Err(e) => {
            error!(
                "Failed to send message to ideation session {}: {}",
                session_id, e
            );
            Err(json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to send message",
            ))
        }
    }
}
