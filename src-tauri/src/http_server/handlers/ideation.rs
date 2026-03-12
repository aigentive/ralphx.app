// Ideation tool handlers for MCP orchestrator-ideation agent

use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use tauri::Emitter;
use tracing::error;

type JsonError = (StatusCode, Json<serde_json::Value>);

fn json_error(status: StatusCode, error: impl Into<String>) -> JsonError {
    (status, Json(serde_json::json!({ "error": error.into() })))
}

use crate::application::{CreateProposalOptions, UpdateProposalOptions, UpdateSource};
use crate::domain::entities::{IdeationSessionId, Priority, TaskProposalId};
use crate::domain::services::emit_verification_status_changed;

use super::super::helpers::{
    create_proposal_impl, delete_proposal_impl, parse_category, parse_priority,
    update_proposal_impl,
};
use super::super::types::{
    AddDependencyRequest, ApplyDependenciesResponse, ApplyDependencySuggestionsRequest,
    CreateProposalRequest, DeleteProposalRequest, GetSessionMessagesRequest,
    GetSessionMessagesResponse, HttpServerState, ListProposalsResponse, ProposalDetailResponse,
    ProposalResponse, ProposalSummary, RevertAndSkipRequest, SessionMessageResponse,
    SuccessResponse, UpdateProposalRequest, UpdateSessionTitleRequest,
    UpdateVerificationRequest, VerificationResponse,
};

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
    };

    // Create proposal — events and dep analysis emitted inside create_proposal_impl()
    let session_id_str = session_id.as_str().to_string();
    let proposal = create_proposal_impl(&state.app_state, session_id, options)
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

    Ok(Json(ProposalResponse::from(proposal)))
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
    };

    // Update proposal — events and dep analysis emitted inside update_proposal_impl()
    let updated = update_proposal_impl(&state.app_state, &proposal_id, options)
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

    Ok(Json(ProposalResponse::from(updated)))
}

pub async fn delete_task_proposal(
    State(state): State<HttpServerState>,
    Json(req): Json<DeleteProposalRequest>,
) -> Result<Json<SuccessResponse>, StatusCode> {
    let proposal_id = TaskProposalId::from_string(req.proposal_id.clone());

    // Delete proposal — assert_session_mutable(), events, and dep analysis inside impl
    delete_proposal_impl(&state.app_state, proposal_id)
        .await
        .map_err(|e| {
            error!("Failed to delete proposal {}: {}", req.proposal_id, e);
            match e {
                crate::error::AppError::NotFound(_) => StatusCode::NOT_FOUND,
                crate::error::AppError::Validation(_) => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

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
        .add_dependency(&proposal_id, &depends_on_id, None, Some("manual"))
        .await
        .map_err(|e| {
            error!(
                "Failed to add dependency from {} to {}: {}",
                proposal_id.as_str(),
                depends_on_id.as_str(),
                e
            );
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
            error!(
                "Failed to get proposals for session {}: {}",
                session_id.as_str(),
                e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let valid_ids: std::collections::HashSet<String> =
        proposals.iter().map(|p| p.id.to_string()).collect();

    // Step 1: Clear all existing dependencies for this session
    state
        .app_state
        .proposal_dependency_repo
        .clear_session_dependencies(&session_id)
        .await
        .map_err(|e| {
            error!(
                "Failed to clear dependencies for session {}: {}",
                session_id.as_str(),
                e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Step 2: Add each suggested dependency (skip if would create cycle or invalid)
    let mut applied_count = 0;
    let mut skipped_count = 0;

    for suggestion in &req.dependencies {
        // Validate both IDs belong to this session
        if !valid_ids.contains(&suggestion.proposal_id)
            || !valid_ids.contains(&suggestion.depends_on_id)
        {
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
        let would_cycle =
            would_create_cycle(&state.app_state, &session_id, &proposal_id, &depends_on_id).await;

        if would_cycle {
            skipped_count += 1;
            continue;
        }

        // Add the dependency with reason from AI suggestion
        match state
            .app_state
            .proposal_dependency_repo
            .add_dependency(
                &proposal_id,
                &depends_on_id,
                suggestion.reason.as_deref(),
                Some("auto"),
            )
            .await
        {
            Ok(_) => applied_count += 1,
            Err(e) => {
                error!(
                    "Failed to add dependency {} -> {}: {}",
                    proposal_id.as_str(),
                    depends_on_id.as_str(),
                    e
                );
                skipped_count += 1;
            }
        }
    }

    // Clear analyzing state: this is the success completion path for the dependency-suggester agent
    {
        let mut analyzing = state.app_state.analyzing_dependencies.write().await;
        analyzing.remove(&session_id);
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
        message: format!(
            "Applied {} dependencies, skipped {} (cycles/invalid)",
            applied_count, skipped_count
        ),
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

    // Check if analysis is in progress for this session
    let analysis_in_progress = {
        let analyzing = state.app_state.analyzing_dependencies.read().await;
        analyzing.contains(&session_id)
    };

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

    let message = if analysis_in_progress {
        Some("Background analysis in progress. Results may update shortly.".to_string())
    } else {
        None
    };

    Ok(Json(AnalyzeDependenciesResponse {
        nodes: response_nodes,
        edges: response_edges,
        critical_path: response_critical_path,
        critical_path_length: critical_path.len(),
        has_cycles: !cycles.is_empty(),
        cycles: response_cycles,
        analysis_in_progress,
        message,
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
    for (from, to, _reason) in &deps {
        adj.entry(from.to_string())
            .or_default()
            .push(to.to_string());
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

    // Server-side generation guard: when setting in_progress=true, verify generation matches
    if req.in_progress {
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

    // Transition validation matrix
    let current = session.verification_status;
    let has_convergence_reason = req.convergence_reason.is_some();
    let is_valid = match (current, new_status) {
        (_, VerificationStatus::Skipped) => true,
        (VerificationStatus::Skipped, _) => false,
        (VerificationStatus::Unverified, VerificationStatus::Reviewing) => true,
        (VerificationStatus::Reviewing, VerificationStatus::NeedsRevision) => true,
        (VerificationStatus::Reviewing, VerificationStatus::Verified) => true,
        (VerificationStatus::NeedsRevision, VerificationStatus::Reviewing) => true,
        // Allow needs_revision → verified ONLY when convergence_reason is provided
        (VerificationStatus::NeedsRevision, VerificationStatus::Verified) => has_convergence_reason,
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

    // Emit plan_verification:status_changed event (B1: includes current_gaps + last 5 rounds)
    if let Some(app_handle) = &state.app_state.app_handle {
        emit_verification_status_changed(
            app_handle,
            &session_id,
            new_status,
            effective_in_progress,
            Some(&metadata),
            None,
        );
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
pub async fn get_plan_verification(
    State(state): State<HttpServerState>,
    Path(session_id): Path<String>,
) -> Result<Json<VerificationResponse>, JsonError> {
    use crate::domain::entities::ideation::VerificationMetadata;
    use crate::domain::services::gap_score;
    use crate::http_server::types::{VerificationGapResponse, VerificationRoundSummary};

    let session_id_obj = crate::domain::entities::IdeationSessionId::from_string(session_id.clone());

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

    // Emit event with canonical payload (B3: was missing round/gaps/rounds fields)
    if let Some(app_handle) = &state.app_state.app_handle {
        emit_verification_status_changed(
            app_handle,
            &session_id,
            VerificationStatus::Skipped,
            false,
            None,
            Some("user_reverted"),
        );
    }

    Ok(Json(SuccessResponse {
        success: true,
        message: "Plan reverted and verification skipped".to_string(),
    }))
}

#[cfg(test)]
#[path = "ideation_tests.rs"]
mod tests;
