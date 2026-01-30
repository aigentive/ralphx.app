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

use super::super::helpers::{
    create_proposal_impl, maybe_trigger_dependency_analysis, parse_category, parse_priority,
    update_proposal_impl,
};
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

    // Auto-trigger dependency analysis when we have 2+ proposals
    maybe_trigger_dependency_analysis(&proposal.session_id, &state.app_state).await;

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

    // Auto-trigger dependency analysis when proposals change
    maybe_trigger_dependency_analysis(&updated.session_id, &state.app_state).await;

    Ok(Json(ProposalResponse::from(updated)))
}

pub async fn delete_task_proposal(
    State(state): State<HttpServerState>,
    Json(req): Json<DeleteProposalRequest>,
) -> Result<Json<SuccessResponse>, StatusCode> {
    let proposal_id = TaskProposalId::from_string(req.proposal_id.clone());

    // Fetch proposal first to get session_id (needed for auto-trigger)
    let proposal = state
        .app_state
        .task_proposal_repo
        .get_by_id(&proposal_id)
        .await
        .map_err(|e| {
            error!("Failed to get proposal {}: {}", proposal_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let session_id = proposal.session_id.clone();

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

    // Auto-trigger dependency analysis after deletion (if we still have 2+ proposals)
    maybe_trigger_dependency_analysis(&session_id, &state.app_state).await;

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

/// Analyze dependencies for a session - returns full graph with critical path, cycles, etc.
/// Used by chat agent to provide intelligent dependency recommendations
pub async fn analyze_session_dependencies(
    State(state): State<HttpServerState>,
    Path(session_id): Path<String>,
) -> Result<Json<super::super::types::AnalyzeDependenciesResponse>, StatusCode> {
    use crate::domain::entities::{DependencyGraph, DependencyGraphEdge, DependencyGraphNode};
    use super::super::types::{
        AnalyzeDependenciesResponse, DependencyAnalysisSummary, DependencyEdgeResponse,
        DependencyNodeResponse,
    };

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
            error!("Failed to get proposals for session {}: {}", session_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get all dependencies for the session
    let dependencies = state
        .app_state
        .proposal_dependency_repo
        .get_all_for_session(&session_id)
        .await
        .map_err(|e| {
            error!("Failed to get dependencies for session {}: {}", session_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Build adjacency lists
    let mut from_map: HashMap<TaskProposalId, Vec<TaskProposalId>> = HashMap::new();
    let mut to_map: HashMap<TaskProposalId, Vec<TaskProposalId>> = HashMap::new();

    for (from, to) in &dependencies {
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

    // Build edges
    let edges: Vec<DependencyGraphEdge> = dependencies
        .iter()
        .map(|(from, to)| DependencyGraphEdge::new(from.clone(), to.clone()))
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

    let response_edges: Vec<DependencyEdgeResponse> = graph
        .edges
        .iter()
        .map(|e| DependencyEdgeResponse {
            from: e.from.to_string(),
            to: e.to.to_string(),
        })
        .collect();

    let response_critical_path: Vec<String> = critical_path.iter().map(|id| id.to_string()).collect();

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
