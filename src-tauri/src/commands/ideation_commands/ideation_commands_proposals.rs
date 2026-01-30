// Proposal CRUD and management commands

use std::collections::{HashMap, HashSet, VecDeque};
use tauri::{Emitter, State};

use crate::application::AppState;
use crate::domain::entities::{
    BusinessValueFactor, Complexity, ComplexityFactor, CriticalPathFactor,
    DependencyFactor, DependencyGraph, DependencyGraphEdge, DependencyGraphNode,
    IdeationSessionId, IdeationSessionStatus, Priority, PriorityAssessment,
    PriorityAssessmentFactors, TaskCategory, TaskProposal, TaskProposalId,
    UserHintFactor,
};
use crate::http_server::helpers::maybe_trigger_dependency_analysis;

use super::ideation_commands_types::{
    CreateProposalInput, PriorityAssessmentResponse, TaskProposalResponse,
    UpdateProposalInput,
};

// ============================================================================
// Proposal Management Commands
// ============================================================================

/// Create a new task proposal
#[tauri::command]
pub async fn create_task_proposal(
    input: CreateProposalInput,
    state: State<'_, AppState>,
) -> Result<TaskProposalResponse, String> {
    let session_id = IdeationSessionId::from_string(input.session_id);

    // Validate session exists and is active
    let session = state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Session not found".to_string())?;

    if session.status != IdeationSessionStatus::Active {
        return Err("Cannot create proposals in archived or converted sessions".to_string());
    }

    // Parse category
    let category: TaskCategory = input
        .category
        .parse()
        .map_err(|_| format!("Invalid category: {}", input.category))?;

    // Parse priority if provided, default to Medium
    let priority: Priority = input
        .priority
        .map(|p| p.parse().unwrap_or(Priority::Medium))
        .unwrap_or(Priority::Medium);

    // Create proposal
    let mut proposal = TaskProposal::new(session_id, &input.title, category, priority);

    // Set optional fields
    if let Some(desc) = input.description {
        proposal.description = Some(desc);
    }
    if let Some(steps) = input.steps {
        proposal.steps = Some(serde_json::to_string(&steps).unwrap_or_default());
    }
    if let Some(criteria) = input.acceptance_criteria {
        proposal.acceptance_criteria = Some(serde_json::to_string(&criteria).unwrap_or_default());
    }
    if let Some(complexity_str) = input.complexity {
        if let Ok(complexity) = complexity_str.parse::<Complexity>() {
            proposal.estimated_complexity = complexity;
        }
    }

    let created_proposal = state
        .task_proposal_repo
        .create(proposal)
        .await
        .map_err(|e| e.to_string())?;

    // Emit event to frontend
    if let Some(app_handle) = &state.app_handle {
        let response = TaskProposalResponse::from(created_proposal.clone());
        let _ = app_handle.emit(
            "proposal:created",
            serde_json::json!({
                "proposal": response
            }),
        );
    }

    // Auto-trigger dependency analysis when we have 2+ proposals
    maybe_trigger_dependency_analysis(&created_proposal.session_id, state.inner()).await;

    Ok(TaskProposalResponse::from(created_proposal))
}

/// Get a task proposal by ID
#[tauri::command]
pub async fn get_task_proposal(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<TaskProposalResponse>, String> {
    let proposal_id = TaskProposalId::from_string(id);
    state
        .task_proposal_repo
        .get_by_id(&proposal_id)
        .await
        .map(|opt| opt.map(TaskProposalResponse::from))
        .map_err(|e| e.to_string())
}

/// List all proposals for a session
#[tauri::command]
pub async fn list_session_proposals(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<TaskProposalResponse>, String> {
    let session_id = IdeationSessionId::from_string(session_id);
    state
        .task_proposal_repo
        .get_by_session(&session_id)
        .await
        .map(|proposals| {
            proposals
                .into_iter()
                .map(TaskProposalResponse::from)
                .collect()
        })
        .map_err(|e| e.to_string())
}

/// Update a task proposal
#[tauri::command]
pub async fn update_task_proposal(
    id: String,
    input: UpdateProposalInput,
    state: State<'_, AppState>,
) -> Result<TaskProposalResponse, String> {
    let proposal_id = TaskProposalId::from_string(id);

    // Get existing proposal
    let mut proposal = state
        .task_proposal_repo
        .get_by_id(&proposal_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Proposal not found".to_string())?;

    // Apply updates
    if let Some(title) = input.title {
        proposal.title = title;
        proposal.user_modified = true;
    }
    if let Some(desc) = input.description {
        proposal.description = Some(desc);
        proposal.user_modified = true;
    }
    if let Some(category_str) = input.category {
        if let Ok(category) = category_str.parse::<TaskCategory>() {
            proposal.category = category;
            proposal.user_modified = true;
        }
    }
    if let Some(steps) = input.steps {
        proposal.steps = Some(serde_json::to_string(&steps).unwrap_or_default());
        proposal.user_modified = true;
    }
    if let Some(criteria) = input.acceptance_criteria {
        proposal.acceptance_criteria = Some(serde_json::to_string(&criteria).unwrap_or_default());
        proposal.user_modified = true;
    }
    if let Some(priority_str) = input.user_priority {
        if let Ok(priority) = priority_str.parse::<Priority>() {
            proposal.user_priority = Some(priority);
            proposal.user_modified = true;
        }
    }
    if let Some(complexity_str) = input.complexity {
        if let Ok(complexity) = complexity_str.parse::<Complexity>() {
            proposal.estimated_complexity = complexity;
            proposal.user_modified = true;
        }
    }

    proposal.touch();

    state
        .task_proposal_repo
        .update(&proposal)
        .await
        .map_err(|e| e.to_string())?;

    // Emit event to frontend
    if let Some(app_handle) = &state.app_handle {
        let response = TaskProposalResponse::from(proposal.clone());
        let _ = app_handle.emit(
            "proposal:updated",
            serde_json::json!({
                "proposal": response
            }),
        );
    }

    // Auto-trigger dependency analysis when proposals change
    maybe_trigger_dependency_analysis(&proposal.session_id, state.inner()).await;

    Ok(TaskProposalResponse::from(proposal))
}

/// Delete a task proposal
#[tauri::command]
pub async fn delete_task_proposal(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let proposal_id = TaskProposalId::from_string(id.clone());

    // Get the session_id before deleting (needed for auto-trigger)
    let session_id = state
        .task_proposal_repo
        .get_by_id(&proposal_id)
        .await
        .map_err(|e| e.to_string())?
        .map(|p| p.session_id);

    state
        .task_proposal_repo
        .delete(&proposal_id)
        .await
        .map_err(|e| e.to_string())?;

    // Emit event to frontend
    if let Some(app_handle) = &state.app_handle {
        let _ = app_handle.emit(
            "proposal:deleted",
            serde_json::json!({
                "proposalId": id
            }),
        );
    }

    // Auto-trigger dependency analysis after deletion (if we still have 2+ proposals)
    if let Some(sess_id) = session_id {
        maybe_trigger_dependency_analysis(&sess_id, state.inner()).await;
    }

    Ok(())
}

/// Toggle proposal selection state
#[tauri::command]
pub async fn toggle_proposal_selection(
    id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let proposal_id = TaskProposalId::from_string(id);

    // Get current state
    let proposal = state
        .task_proposal_repo
        .get_by_id(&proposal_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Proposal not found".to_string())?;

    let new_selected = !proposal.selected;

    state
        .task_proposal_repo
        .update_selection(&proposal_id, new_selected)
        .await
        .map_err(|e| e.to_string())?;

    // Fetch updated proposal and emit event
    if let Some(app_handle) = &state.app_handle {
        if let Ok(Some(updated_proposal)) =
            state.task_proposal_repo.get_by_id(&proposal_id).await
        {
            let response = TaskProposalResponse::from(updated_proposal);
            let _ = app_handle.emit(
                "proposal:updated",
                serde_json::json!({
                    "proposal": response
                }),
            );
        }
    }

    Ok(new_selected)
}

/// Set proposal selection state
#[tauri::command]
pub async fn set_proposal_selection(
    id: String,
    selected: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let proposal_id = TaskProposalId::from_string(id);
    state
        .task_proposal_repo
        .update_selection(&proposal_id, selected)
        .await
        .map_err(|e| e.to_string())?;

    // Fetch updated proposal and emit event
    if let Some(app_handle) = &state.app_handle {
        if let Ok(Some(updated_proposal)) =
            state.task_proposal_repo.get_by_id(&proposal_id).await
        {
            let response = TaskProposalResponse::from(updated_proposal);
            let _ = app_handle.emit(
                "proposal:updated",
                serde_json::json!({
                    "proposal": response
                }),
            );
        }
    }

    Ok(())
}

/// Reorder proposals within a session
#[tauri::command]
pub async fn reorder_proposals(
    session_id: String,
    proposal_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let session_id = IdeationSessionId::from_string(session_id.clone());
    let proposal_ids: Vec<TaskProposalId> = proposal_ids
        .into_iter()
        .map(TaskProposalId::from_string)
        .collect();

    state
        .task_proposal_repo
        .reorder(&session_id, proposal_ids)
        .await
        .map_err(|e| e.to_string())?;

    // Emit event with updated proposals so frontend can update its state
    if let Some(app_handle) = &state.app_handle {
        // Fetch all proposals for this session with their new order
        if let Ok(proposals) = state.task_proposal_repo.get_by_session(&session_id).await {
            let responses: Vec<TaskProposalResponse> =
                proposals.into_iter().map(TaskProposalResponse::from).collect();
            let _ = app_handle.emit(
                "proposals:reordered",
                serde_json::json!({
                    "sessionId": session_id.as_str(),
                    "proposals": responses
                }),
            );
        }
    }

    Ok(())
}

/// Assess priority for a single proposal
#[tauri::command]
pub async fn assess_proposal_priority(
    id: String,
    state: State<'_, AppState>,
) -> Result<PriorityAssessmentResponse, String> {
    let proposal_id = TaskProposalId::from_string(id);

    let proposal = state
        .task_proposal_repo
        .get_by_id(&proposal_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Proposal not found".to_string())?;

    // Build the dependency graph for the session
    let graph = build_dependency_graph(&proposal.session_id, &state)
        .await
        .map_err(|e| e.to_string())?;

    // Calculate the assessment
    let assessment = calculate_proposal_assessment(&proposal, &graph, &state)
        .await
        .map_err(|e| e.to_string())?;

    // Store the result
    state
        .task_proposal_repo
        .update_priority(&proposal_id, &assessment)
        .await
        .map_err(|e| e.to_string())?;

    // Emit event to frontend
    if let Some(app_handle) = &state.app_handle {
        let _ = app_handle.emit(
            "proposal:priority_assessed",
            serde_json::json!({
                "proposalId": assessment.proposal_id.as_str(),
                "priority": assessment.suggested_priority.to_string(),
                "score": assessment.priority_score,
                "reason": assessment.priority_reason
            }),
        );
    }

    Ok(PriorityAssessmentResponse {
        proposal_id: assessment.proposal_id.as_str().to_string(),
        priority: assessment.suggested_priority.to_string(),
        score: assessment.priority_score,
        reason: assessment.priority_reason,
    })
}

/// Assess priority for all proposals in a session
#[tauri::command]
pub async fn assess_all_priorities(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<PriorityAssessmentResponse>, String> {
    let session_id = IdeationSessionId::from_string(session_id.clone());

    // Build the dependency graph once for the session
    let graph = build_dependency_graph(&session_id, &state)
        .await
        .map_err(|e| e.to_string())?;

    // Get all proposals
    let proposals = state
        .task_proposal_repo
        .get_by_session(&session_id)
        .await
        .map_err(|e| e.to_string())?;

    // Assess each proposal and update the database
    let mut assessments = Vec::with_capacity(proposals.len());
    for proposal in &proposals {
        let assessment = calculate_proposal_assessment(proposal, &graph, &state)
            .await
            .map_err(|e| e.to_string())?;

        // Store the result
        state
            .task_proposal_repo
            .update_priority(&proposal.id, &assessment)
            .await
            .map_err(|e| e.to_string())?;

        assessments.push(PriorityAssessmentResponse {
            proposal_id: assessment.proposal_id.as_str().to_string(),
            priority: assessment.suggested_priority.to_string(),
            score: assessment.priority_score,
            reason: assessment.priority_reason,
        });
    }

    // Emit event to frontend for batch assessment
    if let Some(app_handle) = &state.app_handle {
        let _ = app_handle.emit(
            "session:priorities_assessed",
            serde_json::json!({
                "sessionId": session_id.as_str(),
                "count": assessments.len()
            }),
        );
    }

    Ok(assessments)
}

// ============================================================================
// Helper Functions for Priority Assessment
// ============================================================================

/// Build a dependency graph for a session
async fn build_dependency_graph(
    session_id: &IdeationSessionId,
    state: &State<'_, AppState>,
) -> Result<DependencyGraph, crate::error::AppError> {
    // Get all proposals for the session
    let proposals = state.task_proposal_repo.get_by_session(session_id).await?;

    // Get all dependencies for the session
    let dependencies = state
        .proposal_dependency_repo
        .get_all_for_session(session_id)
        .await?;

    // Build adjacency lists
    // from_map: proposal_id -> list of proposals it depends on
    // to_map: proposal_id -> list of proposals that depend on it (dependents)
    let mut from_map: HashMap<TaskProposalId, Vec<TaskProposalId>> = HashMap::new();
    let mut to_map: HashMap<TaskProposalId, Vec<TaskProposalId>> = HashMap::new();

    for (from, to, _reason) in &dependencies {
        from_map.entry(from.clone()).or_default().push(to.clone());
        to_map.entry(to.clone()).or_default().push(from.clone());
    }

    // Build nodes with degree counts
    let mut nodes = Vec::new();
    for proposal in &proposals {
        let in_degree = from_map.get(&proposal.id).map(|v| v.len()).unwrap_or(0);
        let out_degree = to_map.get(&proposal.id).map(|v| v.len()).unwrap_or(0);
        let node = DependencyGraphNode::new(proposal.id.clone(), &proposal.title)
            .with_in_degree(in_degree)
            .with_out_degree(out_degree);
        nodes.push(node);
    }

    // Build edges
    // TODO: Task 5 will add reason to DependencyGraphEdge
    let edges: Vec<DependencyGraphEdge> = dependencies
        .iter()
        .map(|(from, to, _reason)| DependencyGraphEdge::new(from.clone(), to.clone()))
        .collect();

    // Detect cycles using DFS
    let cycles = detect_cycles(&proposals, &from_map);

    // Find critical path (longest path through the DAG)
    let critical_path = if cycles.is_empty() {
        find_critical_path(&proposals, &from_map)
    } else {
        Vec::new() // Can't compute critical path with cycles
    };

    let mut graph = DependencyGraph::with_nodes_and_edges(nodes, edges);
    graph.set_critical_path(critical_path);
    graph.set_cycles(cycles);

    Ok(graph)
}

/// Calculate priority assessment for a single proposal
async fn calculate_proposal_assessment(
    proposal: &TaskProposal,
    graph: &DependencyGraph,
    state: &State<'_, AppState>,
) -> Result<PriorityAssessment, crate::error::AppError> {
    // Get the number of dependents (tasks that this proposal blocks)
    let blocks_count = state
        .proposal_dependency_repo
        .count_dependents(&proposal.id)
        .await? as i32;

    // Calculate dependency factor
    let dependency_factor = DependencyFactor::calculate(blocks_count);

    // Calculate critical path factor
    let is_on_critical_path = graph.is_on_critical_path(&proposal.id);
    let path_length = if is_on_critical_path {
        graph.critical_path_length() as i32
    } else {
        0
    };
    let critical_path_factor = CriticalPathFactor::calculate(is_on_critical_path, path_length);

    // Calculate business value factor from title and description
    let text = format!(
        "{} {}",
        proposal.title,
        proposal.description.as_deref().unwrap_or("")
    );
    let business_value_factor = BusinessValueFactor::calculate(&text);

    // Calculate complexity factor
    let complexity_factor = ComplexityFactor::calculate(proposal.estimated_complexity);

    // Calculate user hint factor from title and description
    let user_hint_factor = UserHintFactor::calculate(&text);

    // Build factors container
    let factors = PriorityAssessmentFactors {
        dependency_factor,
        critical_path_factor,
        business_value_factor,
        complexity_factor,
        user_hint_factor,
    };

    // Create assessment (this calculates total score and suggested priority)
    let assessment = PriorityAssessment::new(proposal.id.clone(), factors);

    Ok(assessment)
}

/// Detect cycles in the dependency graph using DFS
fn detect_cycles(
    proposals: &[TaskProposal],
    from_map: &HashMap<TaskProposalId, Vec<TaskProposalId>>,
) -> Vec<Vec<TaskProposalId>> {
    let mut cycles = Vec::new();
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    let mut path = Vec::new();

    for proposal in proposals {
        if !visited.contains(&proposal.id) {
            dfs_detect_cycle(
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

/// DFS helper for cycle detection
fn dfs_detect_cycle(
    node: &TaskProposalId,
    from_map: &HashMap<TaskProposalId, Vec<TaskProposalId>>,
    visited: &mut HashSet<TaskProposalId>,
    rec_stack: &mut HashSet<TaskProposalId>,
    path: &mut Vec<TaskProposalId>,
    cycles: &mut Vec<Vec<TaskProposalId>>,
) {
    visited.insert(node.clone());
    rec_stack.insert(node.clone());
    path.push(node.clone());

    if let Some(neighbors) = from_map.get(node) {
        for neighbor in neighbors {
            if !visited.contains(neighbor) {
                dfs_detect_cycle(neighbor, from_map, visited, rec_stack, path, cycles);
            } else if rec_stack.contains(neighbor) {
                // Found a cycle - extract it from the path
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

/// Find the critical path (longest path) in the DAG using topological sort + DP
fn find_critical_path(
    proposals: &[TaskProposal],
    from_map: &HashMap<TaskProposalId, Vec<TaskProposalId>>,
) -> Vec<TaskProposalId> {
    if proposals.is_empty() {
        return Vec::new();
    }

    // Build reverse map (to_map) for topological sort
    // to_map: proposal_id -> list of proposals that depend on this (get unblocked when this completes)
    let mut to_map: HashMap<TaskProposalId, Vec<TaskProposalId>> = HashMap::new();
    let mut in_degree: HashMap<TaskProposalId, usize> = HashMap::new();

    // Initialize all nodes with zero in-degree
    for proposal in proposals {
        in_degree.insert(proposal.id.clone(), 0);
    }

    // Build reverse adjacency and count in-degrees
    for (from, deps) in from_map {
        for to in deps {
            to_map.entry(to.clone()).or_default().push(from.clone());
            *in_degree.entry(from.clone()).or_default() += 1;
        }
    }

    // Topological sort using Kahn's algorithm
    let mut queue: VecDeque<TaskProposalId> = VecDeque::new();
    for (id, &degree) in &in_degree {
        if degree == 0 {
            queue.push_back(id.clone());
        }
    }

    let mut topo_order = Vec::new();
    while let Some(node) = queue.pop_front() {
        topo_order.push(node.clone());

        if let Some(neighbors) = to_map.get(&node) {
            for neighbor in neighbors {
                if let Some(degree) = in_degree.get_mut(neighbor) {
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }
    }

    // If we couldn't process all nodes, there's a cycle
    if topo_order.len() != proposals.len() {
        return Vec::new();
    }

    // DP to find longest path
    let mut dist: HashMap<TaskProposalId, i32> = HashMap::new();
    let mut prev: HashMap<TaskProposalId, Option<TaskProposalId>> = HashMap::new();

    for id in &topo_order {
        dist.insert(id.clone(), 0);
        prev.insert(id.clone(), None);
    }

    // Process nodes in topological order
    for node in &topo_order {
        if let Some(neighbors) = to_map.get(node) {
            for neighbor in neighbors {
                let new_dist = dist.get(node).unwrap_or(&0) + 1;
                if new_dist > *dist.get(neighbor).unwrap_or(&0) {
                    dist.insert(neighbor.clone(), new_dist);
                    prev.insert(neighbor.clone(), Some(node.clone()));
                }
            }
        }
    }

    // Find the node with maximum distance (end of critical path)
    let mut max_dist = 0;
    let mut end_node: Option<TaskProposalId> = topo_order.first().cloned();

    for (id, &d) in &dist {
        if d > max_dist {
            max_dist = d;
            end_node = Some(id.clone());
        }
    }

    // Reconstruct the path from end to start
    let mut path = Vec::new();
    let mut current = end_node;

    while let Some(node) = current {
        path.push(node.clone());
        current = prev.get(&node).and_then(|p| p.clone());
    }

    path.reverse();
    path
}
