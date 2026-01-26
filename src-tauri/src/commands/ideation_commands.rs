// Tauri commands for Ideation Session and Proposal CRUD operations
// Thin layer that delegates to repositories

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use tauri::{Emitter, State};

use crate::application::AppState;
use crate::domain::entities::{
    ChatMessage, Complexity, DependencyGraph, DependencyGraphEdge, DependencyGraphNode,
    IdeationSession, IdeationSessionId, IdeationSessionStatus, InternalStatus, Priority,
    ProjectId, Task, TaskCategory, TaskId, TaskProposal, TaskProposalId,
};
use crate::domain::ideation::IdeationSettings;

/// Input for creating a new ideation session
#[derive(Debug, Deserialize)]
pub struct CreateSessionInput {
    pub project_id: String,
    pub title: Option<String>,
}

/// Response wrapper for ideation session operations
#[derive(Debug, Serialize)]
pub struct IdeationSessionResponse {
    pub id: String,
    pub project_id: String,
    pub title: Option<String>,
    pub status: String,
    pub plan_artifact_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
    pub converted_at: Option<String>,
}

impl From<IdeationSession> for IdeationSessionResponse {
    fn from(session: IdeationSession) -> Self {
        Self {
            id: session.id.as_str().to_string(),
            project_id: session.project_id.as_str().to_string(),
            title: session.title,
            status: session.status.to_string(),
            plan_artifact_id: session.plan_artifact_id.map(|id| id.as_str().to_string()),
            created_at: session.created_at.to_rfc3339(),
            updated_at: session.updated_at.to_rfc3339(),
            archived_at: session.archived_at.map(|dt| dt.to_rfc3339()),
            converted_at: session.converted_at.map(|dt| dt.to_rfc3339()),
        }
    }
}

/// Response for TaskProposal
#[derive(Debug, Serialize)]
pub struct TaskProposalResponse {
    pub id: String,
    pub session_id: String,
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub steps: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub suggested_priority: String,
    pub priority_score: i32,
    pub priority_reason: Option<String>,
    pub estimated_complexity: String,
    pub user_priority: Option<String>,
    pub user_modified: bool,
    pub status: String,
    pub selected: bool,
    pub created_task_id: Option<String>,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl From<TaskProposal> for TaskProposalResponse {
    fn from(proposal: TaskProposal) -> Self {
        // Parse JSON strings to Vec<String>, defaulting to empty vec
        let steps: Vec<String> = proposal
            .steps
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let acceptance_criteria: Vec<String> = proposal
            .acceptance_criteria
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        Self {
            id: proposal.id.as_str().to_string(),
            session_id: proposal.session_id.as_str().to_string(),
            title: proposal.title,
            description: proposal.description,
            category: proposal.category.to_string(),
            steps,
            acceptance_criteria,
            suggested_priority: proposal.suggested_priority.to_string(),
            priority_score: proposal.priority_score,
            priority_reason: proposal.priority_reason,
            estimated_complexity: proposal.estimated_complexity.to_string(),
            user_priority: proposal.user_priority.map(|p| p.to_string()),
            user_modified: proposal.user_modified,
            status: proposal.status.to_string(),
            selected: proposal.selected,
            created_task_id: proposal.created_task_id.map(|id| id.as_str().to_string()),
            sort_order: proposal.sort_order,
            created_at: proposal.created_at.to_rfc3339(),
            updated_at: proposal.updated_at.to_rfc3339(),
        }
    }
}

/// Response for ChatMessage
#[derive(Debug, Serialize)]
pub struct ChatMessageResponse {
    pub id: String,
    pub session_id: Option<String>,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    pub role: String,
    pub content: String,
    pub metadata: Option<String>,
    pub parent_message_id: Option<String>,
    pub tool_calls: Option<String>,
    pub created_at: String,
}

impl From<ChatMessage> for ChatMessageResponse {
    fn from(message: ChatMessage) -> Self {
        Self {
            id: message.id.as_str().to_string(),
            session_id: message.session_id.map(|id| id.as_str().to_string()),
            project_id: message.project_id.map(|id| id.as_str().to_string()),
            task_id: message.task_id.map(|id| id.as_str().to_string()),
            role: message.role.to_string(),
            content: message.content,
            metadata: message.metadata,
            parent_message_id: message.parent_message_id.map(|id| id.as_str().to_string()),
            tool_calls: message.tool_calls,
            created_at: message.created_at.to_rfc3339(),
        }
    }
}

/// Response for session with proposals and messages
#[derive(Debug, Serialize)]
pub struct SessionWithDataResponse {
    pub session: IdeationSessionResponse,
    pub proposals: Vec<TaskProposalResponse>,
    pub messages: Vec<ChatMessageResponse>,
}

/// Create a new ideation session
#[tauri::command]
pub async fn create_ideation_session(
    input: CreateSessionInput,
    state: State<'_, AppState>,
) -> Result<IdeationSessionResponse, String> {
    let project_id = ProjectId::from_string(input.project_id);
    let session = if let Some(title) = input.title {
        IdeationSession::new_with_title(project_id, &title)
    } else {
        IdeationSession::new(project_id)
    };

    state
        .ideation_session_repo
        .create(session)
        .await
        .map(IdeationSessionResponse::from)
        .map_err(|e| e.to_string())
}

/// Get a single ideation session by ID
#[tauri::command]
pub async fn get_ideation_session(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<IdeationSessionResponse>, String> {
    let session_id = IdeationSessionId::from_string(id);
    state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map(|opt| opt.map(IdeationSessionResponse::from))
        .map_err(|e| e.to_string())
}

/// Get session with proposals and messages
#[tauri::command]
pub async fn get_ideation_session_with_data(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<SessionWithDataResponse>, String> {
    let session_id = IdeationSessionId::from_string(id);

    // Get session
    let session = match state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|e| e.to_string())?
    {
        Some(s) => s,
        None => return Ok(None),
    };

    // Get proposals
    let proposals = state
        .task_proposal_repo
        .get_by_session(&session_id)
        .await
        .map_err(|e| e.to_string())?;

    // Get messages
    let messages = state
        .chat_message_repo
        .get_by_session(&session_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Some(SessionWithDataResponse {
        session: IdeationSessionResponse::from(session),
        proposals: proposals.into_iter().map(TaskProposalResponse::from).collect(),
        messages: messages.into_iter().map(ChatMessageResponse::from).collect(),
    }))
}

/// List all ideation sessions for a project
#[tauri::command]
pub async fn list_ideation_sessions(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<IdeationSessionResponse>, String> {
    let project_id = ProjectId::from_string(project_id);
    state
        .ideation_session_repo
        .get_by_project(&project_id)
        .await
        .map(|sessions| {
            sessions
                .into_iter()
                .map(IdeationSessionResponse::from)
                .collect()
        })
        .map_err(|e| e.to_string())
}

/// Archive an ideation session
#[tauri::command]
pub async fn archive_ideation_session(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let session_id = IdeationSessionId::from_string(id);
    state
        .ideation_session_repo
        .update_status(&session_id, IdeationSessionStatus::Archived)
        .await
        .map_err(|e| e.to_string())
}

/// Delete an ideation session
#[tauri::command]
pub async fn delete_ideation_session(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let session_id = IdeationSessionId::from_string(id);
    state
        .ideation_session_repo
        .delete(&session_id)
        .await
        .map_err(|e| e.to_string())
}

// ============================================================================
// Task Proposal Commands
// ============================================================================

/// Input for creating a new task proposal
#[derive(Debug, Deserialize)]
pub struct CreateProposalInput {
    pub session_id: String,
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub steps: Option<Vec<String>>,
    pub acceptance_criteria: Option<Vec<String>>,
    pub priority: Option<String>,
    pub complexity: Option<String>,
}

/// Input for updating a task proposal
#[derive(Debug, Deserialize)]
pub struct UpdateProposalInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub steps: Option<Vec<String>>,
    pub acceptance_criteria: Option<Vec<String>>,
    pub user_priority: Option<String>,
    pub complexity: Option<String>,
}

/// Response for priority assessment
#[derive(Debug, Serialize)]
pub struct PriorityAssessmentResponse {
    pub proposal_id: String,
    pub priority: String,
    pub score: i32,
    pub reason: String,
}

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

    Ok(TaskProposalResponse::from(proposal))
}

/// Delete a task proposal
#[tauri::command]
pub async fn delete_task_proposal(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let proposal_id = TaskProposalId::from_string(id.clone());

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
        .map_err(|e| e.to_string())
}

/// Reorder proposals within a session
#[tauri::command]
pub async fn reorder_proposals(
    session_id: String,
    proposal_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let session_id = IdeationSessionId::from_string(session_id);
    let proposal_ids: Vec<TaskProposalId> = proposal_ids
        .into_iter()
        .map(TaskProposalId::from_string)
        .collect();

    state
        .task_proposal_repo
        .reorder(&session_id, proposal_ids)
        .await
        .map_err(|e| e.to_string())
}

/// Assess priority for a single proposal (stub - real implementation in PriorityService)
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

    // For now, return current priority info
    // Real implementation would use PriorityService
    Ok(PriorityAssessmentResponse {
        proposal_id: proposal.id.as_str().to_string(),
        priority: proposal.suggested_priority.to_string(),
        score: proposal.priority_score,
        reason: proposal
            .priority_reason
            .unwrap_or_else(|| "No assessment yet".to_string()),
    })
}

/// Assess priority for all proposals in a session (stub)
#[tauri::command]
pub async fn assess_all_priorities(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<PriorityAssessmentResponse>, String> {
    let session_id = IdeationSessionId::from_string(session_id);

    let proposals = state
        .task_proposal_repo
        .get_by_session(&session_id)
        .await
        .map_err(|e| e.to_string())?;

    // For now, return current priority info for all proposals
    // Real implementation would use PriorityService
    Ok(proposals
        .into_iter()
        .map(|p| PriorityAssessmentResponse {
            proposal_id: p.id.as_str().to_string(),
            priority: p.suggested_priority.to_string(),
            score: p.priority_score,
            reason: p
                .priority_reason
                .unwrap_or_else(|| "No assessment yet".to_string()),
        })
        .collect())
}

// ============================================================================
// Dependency and Apply Commands
// ============================================================================

/// Response for DependencyGraph
#[derive(Debug, Serialize)]
pub struct DependencyGraphResponse {
    pub nodes: Vec<DependencyGraphNodeResponse>,
    pub edges: Vec<DependencyGraphEdgeResponse>,
    pub critical_path: Vec<String>,
    pub has_cycles: bool,
    pub cycles: Option<Vec<Vec<String>>>,
}

#[derive(Debug, Serialize)]
pub struct DependencyGraphNodeResponse {
    pub proposal_id: String,
    pub title: String,
    pub in_degree: usize,
    pub out_degree: usize,
}

#[derive(Debug, Serialize)]
pub struct DependencyGraphEdgeResponse {
    pub from: String,
    pub to: String,
}

impl From<DependencyGraph> for DependencyGraphResponse {
    fn from(graph: DependencyGraph) -> Self {
        Self {
            nodes: graph
                .nodes
                .into_iter()
                .map(|n| DependencyGraphNodeResponse {
                    proposal_id: n.proposal_id.as_str().to_string(),
                    title: n.title,
                    in_degree: n.in_degree,
                    out_degree: n.out_degree,
                })
                .collect(),
            edges: graph
                .edges
                .into_iter()
                .map(|e| DependencyGraphEdgeResponse {
                    from: e.from.as_str().to_string(),
                    to: e.to.as_str().to_string(),
                })
                .collect(),
            critical_path: graph
                .critical_path
                .into_iter()
                .map(|id| id.as_str().to_string())
                .collect(),
            has_cycles: graph.has_cycles,
            cycles: graph.cycles.map(|cs| {
                cs.into_iter()
                    .map(|c| c.into_iter().map(|id| id.as_str().to_string()).collect())
                    .collect()
            }),
        }
    }
}

/// Input for apply proposals
#[derive(Debug, Deserialize)]
pub struct ApplyProposalsInput {
    pub session_id: String,
    pub proposal_ids: Vec<String>,
    pub target_column: String,
    pub preserve_dependencies: bool,
}

/// Response for apply proposals
#[derive(Debug, Serialize)]
pub struct ApplyProposalsResultResponse {
    pub created_task_ids: Vec<String>,
    pub dependencies_created: usize,
    pub warnings: Vec<String>,
    pub session_converted: bool,
}

/// Add a dependency between proposals
#[tauri::command]
pub async fn add_proposal_dependency(
    proposal_id: String,
    depends_on_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let proposal_id = TaskProposalId::from_string(proposal_id);
    let depends_on_id = TaskProposalId::from_string(depends_on_id);

    // Prevent self-dependency
    if proposal_id == depends_on_id {
        return Err("A proposal cannot depend on itself".to_string());
    }

    state
        .proposal_dependency_repo
        .add_dependency(&proposal_id, &depends_on_id)
        .await
        .map_err(|e| e.to_string())
}

/// Remove a dependency between proposals
#[tauri::command]
pub async fn remove_proposal_dependency(
    proposal_id: String,
    depends_on_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let proposal_id = TaskProposalId::from_string(proposal_id);
    let depends_on_id = TaskProposalId::from_string(depends_on_id);

    state
        .proposal_dependency_repo
        .remove_dependency(&proposal_id, &depends_on_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get dependencies for a proposal
#[tauri::command]
pub async fn get_proposal_dependencies(
    proposal_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let proposal_id = TaskProposalId::from_string(proposal_id);

    state
        .proposal_dependency_repo
        .get_dependencies(&proposal_id)
        .await
        .map(|deps| deps.into_iter().map(|id| id.as_str().to_string()).collect())
        .map_err(|e| e.to_string())
}

/// Get dependents for a proposal (proposals that depend on this one)
#[tauri::command]
pub async fn get_proposal_dependents(
    proposal_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let proposal_id = TaskProposalId::from_string(proposal_id);

    state
        .proposal_dependency_repo
        .get_dependents(&proposal_id)
        .await
        .map(|deps| deps.into_iter().map(|id| id.as_str().to_string()).collect())
        .map_err(|e| e.to_string())
}

/// Analyze dependencies for a session and return the dependency graph
#[tauri::command]
pub async fn analyze_dependencies(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<DependencyGraphResponse, String> {
    let session_id = IdeationSessionId::from_string(session_id);

    // Get proposals for this session
    let proposals = state
        .task_proposal_repo
        .get_by_session(&session_id)
        .await
        .map_err(|e| e.to_string())?;

    // Get all dependencies for this session
    let deps = state
        .proposal_dependency_repo
        .get_all_for_session(&session_id)
        .await
        .map_err(|e| e.to_string())?;

    // Build the graph directly (inline version of DependencyService logic)
    let graph = build_dependency_graph(&proposals, &deps);

    Ok(DependencyGraphResponse::from(graph))
}

/// Build a dependency graph from proposals and their dependencies
fn build_dependency_graph(
    proposals: &[TaskProposal],
    dependencies: &[(TaskProposalId, TaskProposalId)],
) -> DependencyGraph {
    // Build adjacency lists
    let mut from_map: HashMap<TaskProposalId, Vec<TaskProposalId>> = HashMap::new();
    let mut to_map: HashMap<TaskProposalId, Vec<TaskProposalId>> = HashMap::new();

    for (from, to) in dependencies {
        from_map.entry(from.clone()).or_default().push(to.clone());
        to_map.entry(to.clone()).or_default().push(from.clone());
    }

    // Build nodes with degree counts
    let mut nodes = Vec::new();
    for proposal in proposals {
        let in_degree = from_map.get(&proposal.id).map(|v| v.len()).unwrap_or(0);
        let out_degree = to_map.get(&proposal.id).map(|v| v.len()).unwrap_or(0);

        let mut node = DependencyGraphNode::new(proposal.id.clone(), &proposal.title);
        node.in_degree = in_degree;
        node.out_degree = out_degree;
        nodes.push(node);
    }

    // Build edges
    let edges: Vec<DependencyGraphEdge> = dependencies
        .iter()
        .map(|(from, to)| DependencyGraphEdge::new(from.clone(), to.clone()))
        .collect();

    // Detect cycles using DFS
    let (has_cycles, cycles) = detect_cycles(&from_map, proposals);

    // Find critical path if no cycles
    let critical_path = if !has_cycles {
        find_critical_path(&from_map, proposals)
    } else {
        Vec::new()
    };

    DependencyGraph {
        nodes,
        edges,
        critical_path,
        has_cycles,
        cycles: if has_cycles { Some(cycles) } else { None },
    }
}

/// Detect cycles in the dependency graph using DFS
fn detect_cycles(
    from_map: &HashMap<TaskProposalId, Vec<TaskProposalId>>,
    proposals: &[TaskProposal],
) -> (bool, Vec<Vec<TaskProposalId>>) {
    let proposal_ids: HashSet<TaskProposalId> = proposals.iter().map(|p| p.id.clone()).collect();
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    let mut cycles = Vec::new();

    for proposal in proposals {
        if !visited.contains(&proposal.id) {
            let mut path = Vec::new();
            if dfs_detect_cycle(
                &proposal.id,
                from_map,
                &proposal_ids,
                &mut visited,
                &mut rec_stack,
                &mut path,
                &mut cycles,
            ) {
                // Continue to find all cycles
            }
        }
    }

    (!cycles.is_empty(), cycles)
}

fn dfs_detect_cycle(
    node: &TaskProposalId,
    from_map: &HashMap<TaskProposalId, Vec<TaskProposalId>>,
    proposal_ids: &HashSet<TaskProposalId>,
    visited: &mut HashSet<TaskProposalId>,
    rec_stack: &mut HashSet<TaskProposalId>,
    path: &mut Vec<TaskProposalId>,
    cycles: &mut Vec<Vec<TaskProposalId>>,
) -> bool {
    visited.insert(node.clone());
    rec_stack.insert(node.clone());
    path.push(node.clone());

    if let Some(deps) = from_map.get(node) {
        for dep in deps {
            if !proposal_ids.contains(dep) {
                continue;
            }
            if !visited.contains(dep) {
                if dfs_detect_cycle(dep, from_map, proposal_ids, visited, rec_stack, path, cycles) {
                    // Found a cycle
                }
            } else if rec_stack.contains(dep) {
                // Found a cycle - extract it
                let cycle_start = path.iter().position(|p| p == dep).unwrap_or(0);
                let cycle: Vec<TaskProposalId> = path[cycle_start..].to_vec();
                cycles.push(cycle);
            }
        }
    }

    path.pop();
    rec_stack.remove(node);

    !cycles.is_empty()
}

/// Find the critical path (longest path through the graph)
fn find_critical_path(
    from_map: &HashMap<TaskProposalId, Vec<TaskProposalId>>,
    proposals: &[TaskProposal],
) -> Vec<TaskProposalId> {
    if proposals.is_empty() {
        return Vec::new();
    }

    // Build reverse map for topological sort
    let mut in_degree: HashMap<TaskProposalId, usize> = HashMap::new();
    for proposal in proposals {
        in_degree.insert(proposal.id.clone(), 0);
    }

    for deps in from_map.values() {
        for dep in deps {
            if in_degree.contains_key(dep) {
                *in_degree.get_mut(dep).unwrap() += 1;
            }
        }
    }

    // Topological sort using Kahn's algorithm
    let mut queue: VecDeque<TaskProposalId> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(id, _)| id.clone())
        .collect();

    let mut sorted = Vec::new();
    while let Some(node) = queue.pop_front() {
        sorted.push(node.clone());
        if let Some(deps) = from_map.get(&node) {
            for dep in deps {
                if let Some(deg) = in_degree.get_mut(dep) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(dep.clone());
                    }
                }
            }
        }
    }

    // Find longest path using dynamic programming
    let mut dist: HashMap<TaskProposalId, usize> = HashMap::new();
    let mut prev: HashMap<TaskProposalId, Option<TaskProposalId>> = HashMap::new();

    for id in &sorted {
        dist.insert(id.clone(), 0);
        prev.insert(id.clone(), None);
    }

    for node in &sorted {
        if let Some(deps) = from_map.get(node) {
            for dep in deps {
                if let Some(&node_dist) = dist.get(node) {
                    if let Some(dep_dist) = dist.get_mut(dep) {
                        if node_dist + 1 > *dep_dist {
                            *dep_dist = node_dist + 1;
                            prev.insert(dep.clone(), Some(node.clone()));
                        }
                    }
                }
            }
        }
    }

    // Find the end of the longest path
    let (end_node, _) = dist.iter().max_by_key(|(_, &d)| d).unwrap_or((&sorted[0], &0));

    // Reconstruct path
    let mut path = vec![end_node.clone()];
    let mut current = end_node.clone();
    while let Some(Some(prev_node)) = prev.get(&current) {
        path.push(prev_node.clone());
        current = prev_node.clone();
    }

    path.reverse();
    path
}

/// Apply selected proposals to the Kanban board as tasks
#[tauri::command]
pub async fn apply_proposals_to_kanban(
    input: ApplyProposalsInput,
    state: State<'_, AppState>,
) -> Result<ApplyProposalsResultResponse, String> {
    let session_id = IdeationSessionId::from_string(input.session_id);

    // Parse target column and map to internal status
    let target_status = match input.target_column.to_lowercase().as_str() {
        "draft" => InternalStatus::Backlog,
        "backlog" => InternalStatus::Backlog,
        "todo" => InternalStatus::Ready,
        _ => return Err(format!("Invalid target column: {}", input.target_column)),
    };

    // Get the session to know the project_id
    let session = state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    if session.status != IdeationSessionStatus::Active {
        return Err("Cannot apply proposals from an inactive session".to_string());
    }

    let proposal_ids: HashSet<TaskProposalId> = input
        .proposal_ids
        .into_iter()
        .map(TaskProposalId::from_string)
        .collect();

    // Validate that all proposals exist and belong to this session
    let all_proposals = state
        .task_proposal_repo
        .get_by_session(&session_id)
        .await
        .map_err(|e| e.to_string())?;

    let proposals_to_apply: Vec<TaskProposal> = all_proposals
        .into_iter()
        .filter(|p| proposal_ids.contains(&p.id))
        .collect();

    if proposals_to_apply.len() != proposal_ids.len() {
        return Err("Some proposals not found in session".to_string());
    }

    // Create tasks and track dependencies
    let mut created_tasks = Vec::new();
    let mut proposal_to_task: HashMap<TaskProposalId, TaskId> = HashMap::new();
    let mut warnings = Vec::new();

    for proposal in &proposals_to_apply {
        // Create task from proposal
        let mut task = Task::new(session.project_id.clone(), proposal.title.clone());
        task.description = proposal.description.clone();
        task.category = proposal.category.to_string();
        task.internal_status = target_status;

        // Set priority based on user override or suggested (use priority score as i32)
        if proposal.user_priority.is_some() {
            task.priority = proposal.priority_score; // Use calculated score
        } else {
            task.priority = proposal.priority_score;
        }

        let created_task = state
            .task_repo
            .create(task)
            .await
            .map_err(|e| e.to_string())?;

        proposal_to_task.insert(proposal.id.clone(), created_task.id.clone());
        created_tasks.push(created_task);
    }

    // Create task dependencies if requested
    let mut dependencies_created = 0;
    if input.preserve_dependencies {
        for proposal in &proposals_to_apply {
            let deps = state
                .proposal_dependency_repo
                .get_dependencies(&proposal.id)
                .await
                .map_err(|e| e.to_string())?;

            for dep_proposal_id in deps {
                if let (Some(task_id), Some(dep_task_id)) = (
                    proposal_to_task.get(&proposal.id),
                    proposal_to_task.get(&dep_proposal_id),
                ) {
                    state
                        .task_dependency_repo
                        .add_dependency(task_id, dep_task_id)
                        .await
                        .map_err(|e| e.to_string())?;
                    dependencies_created += 1;
                } else {
                    warnings.push(format!(
                        "Dependency from {} to {} not preserved (not in selection)",
                        proposal.id, dep_proposal_id
                    ));
                }
            }
        }
    }

    // Update proposal statuses and link to created tasks
    for proposal in &proposals_to_apply {
        if let Some(task_id) = proposal_to_task.get(&proposal.id) {
            state
                .task_proposal_repo
                .set_created_task_id(&proposal.id, task_id)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    // Check if all proposals in session are now applied
    let remaining = state
        .task_proposal_repo
        .get_by_session(&session_id)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|p| p.created_task_id.is_none())
        .count();

    let session_converted = remaining == 0;
    if session_converted {
        state
            .ideation_session_repo
            .update_status(&session_id, IdeationSessionStatus::Converted)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(ApplyProposalsResultResponse {
        created_task_ids: created_tasks
            .into_iter()
            .map(|t| t.id.as_str().to_string())
            .collect(),
        dependencies_created,
        warnings,
        session_converted,
    })
}

/// Get blockers for a task (tasks it depends on)
#[tauri::command]
pub async fn get_task_blockers(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let task_id = TaskId::from_string(task_id);

    state
        .task_dependency_repo
        .get_blockers(&task_id)
        .await
        .map(|blockers| {
            blockers
                .into_iter()
                .map(|id| id.as_str().to_string())
                .collect()
        })
        .map_err(|e| e.to_string())
}

/// Get tasks blocked by a task (tasks that depend on this one)
#[tauri::command]
pub async fn get_blocked_tasks(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let task_id = TaskId::from_string(task_id);

    state
        .task_dependency_repo
        .get_blocked_by(&task_id)
        .await
        .map(|blocked| {
            blocked
                .into_iter()
                .map(|id| id.as_str().to_string())
                .collect()
        })
        .map_err(|e| e.to_string())
}

// ============================================================================
// Chat Message Commands
// ============================================================================

/// Input for sending a chat message
#[derive(Debug, Deserialize)]
pub struct SendChatMessageInput {
    /// Session ID for session context (optional, mutually exclusive with project_id and task_id)
    pub session_id: Option<String>,
    /// Project ID for project context (optional, only used if session_id is None)
    pub project_id: Option<String>,
    /// Task ID for task context (optional, only used if session_id and project_id are None)
    pub task_id: Option<String>,
    /// Role of the message sender
    pub role: String,
    /// Message content (supports Markdown)
    pub content: String,
    /// Optional metadata (JSON)
    pub metadata: Option<String>,
    /// Optional parent message ID for threading
    pub parent_message_id: Option<String>,
}

/// Send a chat message
#[tauri::command]
pub async fn send_chat_message(
    input: SendChatMessageInput,
    state: State<'_, AppState>,
) -> Result<ChatMessageResponse, String> {
    use crate::domain::entities::{ChatMessageId, MessageRole};

    // Determine the context and create the appropriate message
    let mut message = if let Some(session_id_str) = input.session_id {
        let session_id = IdeationSessionId::from_string(session_id_str);

        // Validate session exists
        let session = state
            .ideation_session_repo
            .get_by_id(&session_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Session not found".to_string())?;

        if session.status != IdeationSessionStatus::Active {
            return Err("Cannot send messages to an inactive session".to_string());
        }

        // Create message based on role
        let role: MessageRole = input.role.parse().map_err(|_| format!("Invalid role: {}", input.role))?;
        match role {
            MessageRole::User => ChatMessage::user_in_session(session_id, &input.content),
            MessageRole::Orchestrator => ChatMessage::orchestrator_in_session(session_id, &input.content),
            MessageRole::System => ChatMessage::system_in_session(session_id, &input.content),
            MessageRole::Worker => {
                // Worker messages are typically not created through this endpoint
                // but we handle them for completeness
                let mut msg = ChatMessage::user_in_session(session_id, &input.content);
                msg.role = MessageRole::Worker;
                msg
            }
        }
    } else if let Some(project_id_str) = input.project_id {
        let project_id = ProjectId::from_string(project_id_str);
        ChatMessage::user_in_project(project_id, &input.content)
    } else if let Some(task_id_str) = input.task_id {
        let task_id = TaskId::from_string(task_id_str);
        ChatMessage::user_about_task(task_id, &input.content)
    } else {
        return Err("Must provide session_id, project_id, or task_id".to_string());
    };

    // Set optional fields
    if let Some(metadata) = input.metadata {
        message.metadata = Some(metadata);
    }
    if let Some(parent_id_str) = input.parent_message_id {
        message.parent_message_id = Some(ChatMessageId::from_string(parent_id_str));
    }

    state
        .chat_message_repo
        .create(message)
        .await
        .map(ChatMessageResponse::from)
        .map_err(|e| e.to_string())
}

/// Get all messages for a session
#[tauri::command]
pub async fn get_session_messages(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ChatMessageResponse>, String> {
    let session_id = IdeationSessionId::from_string(session_id);

    state
        .chat_message_repo
        .get_by_session(&session_id)
        .await
        .map(|messages| messages.into_iter().map(ChatMessageResponse::from).collect())
        .map_err(|e| e.to_string())
}

/// Get recent messages for a session with a limit
#[tauri::command]
pub async fn get_recent_session_messages(
    session_id: String,
    limit: u32,
    state: State<'_, AppState>,
) -> Result<Vec<ChatMessageResponse>, String> {
    let session_id = IdeationSessionId::from_string(session_id);

    state
        .chat_message_repo
        .get_recent_by_session(&session_id, limit)
        .await
        .map(|messages| messages.into_iter().map(ChatMessageResponse::from).collect())
        .map_err(|e| e.to_string())
}

/// Get all messages for a project (not in any session)
#[tauri::command]
pub async fn get_project_messages(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ChatMessageResponse>, String> {
    let project_id = ProjectId::from_string(project_id);

    state
        .chat_message_repo
        .get_by_project(&project_id)
        .await
        .map(|messages| messages.into_iter().map(ChatMessageResponse::from).collect())
        .map_err(|e| e.to_string())
}

/// Get all messages for a task
#[tauri::command]
pub async fn get_task_messages(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ChatMessageResponse>, String> {
    let task_id = TaskId::from_string(task_id);

    state
        .chat_message_repo
        .get_by_task(&task_id)
        .await
        .map(|messages| messages.into_iter().map(ChatMessageResponse::from).collect())
        .map_err(|e| e.to_string())
}

/// Delete a chat message
#[tauri::command]
pub async fn delete_chat_message(id: String, state: State<'_, AppState>) -> Result<(), String> {
    use crate::domain::entities::ChatMessageId;

    let message_id = ChatMessageId::from_string(id);
    state
        .chat_message_repo
        .delete(&message_id)
        .await
        .map_err(|e| e.to_string())
}

/// Delete all messages in a session
#[tauri::command]
pub async fn delete_session_messages(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let session_id = IdeationSessionId::from_string(session_id);
    state
        .chat_message_repo
        .delete_by_session(&session_id)
        .await
        .map_err(|e| e.to_string())
}

/// Count messages in a session
#[tauri::command]
pub async fn count_session_messages(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<u32, String> {
    let session_id = IdeationSessionId::from_string(session_id);
    state
        .chat_message_repo
        .count_by_session(&session_id)
        .await
        .map_err(|e| e.to_string())
}

// ============================================================================
// Orchestrator Integration Commands
// ============================================================================

/// Input for sending a message to the orchestrator
#[derive(Debug, Deserialize)]
pub struct SendOrchestratorMessageInput {
    pub session_id: String,
    pub content: String,
}

/// Response from the orchestrator
#[derive(Debug, Serialize)]
pub struct OrchestratorMessageResponse {
    pub response_text: String,
    pub proposals_created: Vec<TaskProposalResponse>,
    pub tool_calls: Vec<ToolCallResultResponse>,
}

/// Tool call result response
#[derive(Debug, Serialize)]
pub struct ToolCallResultResponse {
    pub tool_name: String,
    pub success: bool,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// Send a message to the orchestrator agent and get a response
/// This invokes the Claude CLI with the orchestrator-ideation agent
///
/// The service now:
/// - Automatically manages conversations (creates/resumes based on claude_session_id)
/// - Uses --resume flag for follow-up messages (Claude manages conversation context)
/// - Delegates tool execution to MCP server
/// - Emits Tauri events for real-time UI updates
#[tauri::command]
pub async fn send_orchestrator_message(
    input: SendOrchestratorMessageInput,
    state: State<'_, AppState>,
) -> Result<OrchestratorMessageResponse, String> {
    use crate::application::{ClaudeOrchestratorService, OrchestratorService};

    // First verify the session exists and is active
    let session_id = IdeationSessionId::from_string(input.session_id.clone());
    let session = state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Session not found".to_string())?;

    if session.status != IdeationSessionStatus::Active {
        return Err("Session is not active".to_string());
    }

    // Create orchestrator service with required repositories
    let orchestrator: ClaudeOrchestratorService<tauri::Wry> = ClaudeOrchestratorService::new(
        state.chat_message_repo.clone(),
        state.chat_conversation_repo.clone(),
        state.agent_run_repo.clone(),
        state.project_repo.clone(),
        state.task_repo.clone(),
        state.ideation_session_repo.clone(),
    );

    // Check if orchestrator is available
    if !orchestrator.is_available().await {
        return Err("Orchestrator agent (claude CLI) is not available".to_string());
    }

    // Send message and get response
    let result = orchestrator
        .send_message(&session_id, &input.content)
        .await
        .map_err(|e| e.to_string())?;

    // Note: Proposals are now created via MCP tools, not returned directly from the service
    // The frontend should listen to Tauri events for real-time updates
    let proposals_created: Vec<TaskProposalResponse> = Vec::new();

    // Convert tool calls to response format (for display purposes)
    let tool_calls: Vec<ToolCallResultResponse> = result
        .tool_calls
        .into_iter()
        .map(|tc| ToolCallResultResponse {
            tool_name: tc.name,
            success: true, // MCP handles execution; we just observe
            result: tc.result,
            error: None,
        })
        .collect();

    Ok(OrchestratorMessageResponse {
        response_text: result.response_text,
        proposals_created,
        tool_calls,
    })
}

/// Check if the orchestrator agent is available
#[tauri::command]
pub async fn is_orchestrator_available(state: State<'_, AppState>) -> Result<bool, String> {
    use crate::application::{ClaudeOrchestratorService, OrchestratorService};

    let orchestrator: ClaudeOrchestratorService<tauri::Wry> = ClaudeOrchestratorService::new(
        state.chat_message_repo.clone(),
        state.chat_conversation_repo.clone(),
        state.agent_run_repo.clone(),
        state.project_repo.clone(),
        state.task_repo.clone(),
        state.ideation_session_repo.clone(),
    );

    Ok(orchestrator.is_available().await)
}

// ============================================================================
// Ideation Settings Commands
// ============================================================================

/// Get ideation settings
#[tauri::command]
pub async fn get_ideation_settings(
    state: State<'_, AppState>,
) -> Result<IdeationSettings, String> {
    state
        .ideation_settings_repo
        .get_settings()
        .await
        .map_err(|e| e.to_string())
}

/// Update ideation settings
#[tauri::command]
pub async fn update_ideation_settings(
    settings: IdeationSettings,
    state: State<'_, AppState>,
) -> Result<IdeationSettings, String> {
    state
        .ideation_settings_repo
        .update_settings(&settings)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_state() -> AppState {
        AppState::new_test()
    }

    #[tokio::test]
    async fn test_create_ideation_session_without_title() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        let session = IdeationSession::new(project_id.clone());
        let created = state.ideation_session_repo.create(session).await.unwrap();

        assert_eq!(created.project_id, project_id);
        assert!(created.title.is_none());
        assert_eq!(created.status, IdeationSessionStatus::Active);
    }

    #[tokio::test]
    async fn test_create_ideation_session_with_title() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        let session = IdeationSession::new_with_title(project_id.clone(), "Test Session");
        let created = state.ideation_session_repo.create(session).await.unwrap();

        assert_eq!(created.project_id, project_id);
        assert_eq!(created.title, Some("Test Session".to_string()));
        assert_eq!(created.status, IdeationSessionStatus::Active);
    }

    #[tokio::test]
    async fn test_get_ideation_session_returns_none_for_nonexistent() {
        let state = setup_test_state();
        let id = IdeationSessionId::new();

        let result = state.ideation_session_repo.get_by_id(&id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_ideation_session_returns_existing() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        let session = IdeationSession::new(project_id);
        let created = state.ideation_session_repo.create(session).await.unwrap();

        let result = state
            .ideation_session_repo
            .get_by_id(&created.id)
            .await
            .unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, created.id);
    }

    #[tokio::test]
    async fn test_list_ideation_sessions_by_project() {
        let state = setup_test_state();
        let project_id = ProjectId::new();
        let other_project_id = ProjectId::new();

        // Create sessions for our project
        state
            .ideation_session_repo
            .create(IdeationSession::new(project_id.clone()))
            .await
            .unwrap();
        state
            .ideation_session_repo
            .create(IdeationSession::new(project_id.clone()))
            .await
            .unwrap();

        // Create session for different project
        state
            .ideation_session_repo
            .create(IdeationSession::new(other_project_id))
            .await
            .unwrap();

        let sessions = state
            .ideation_session_repo
            .get_by_project(&project_id)
            .await
            .unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_archive_ideation_session() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        let session = IdeationSession::new(project_id);
        let created = state.ideation_session_repo.create(session).await.unwrap();

        state
            .ideation_session_repo
            .update_status(&created.id, IdeationSessionStatus::Archived)
            .await
            .unwrap();

        let retrieved = state
            .ideation_session_repo
            .get_by_id(&created.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.status, IdeationSessionStatus::Archived);
    }

    #[tokio::test]
    async fn test_delete_ideation_session() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        let session = IdeationSession::new(project_id);
        let created = state.ideation_session_repo.create(session).await.unwrap();

        state
            .ideation_session_repo
            .delete(&created.id)
            .await
            .unwrap();

        let result = state
            .ideation_session_repo
            .get_by_id(&created.id)
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_session_response_serialization() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new_with_title(project_id, "Test Session");
        let response = IdeationSessionResponse::from(session);

        assert!(!response.id.is_empty());
        assert_eq!(response.title, Some("Test Session".to_string()));
        assert_eq!(response.status, "active");

        // Verify it serializes to JSON
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"title\":\"Test Session\""));
    }

    #[tokio::test]
    async fn test_get_session_with_data() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Create session
        let session = IdeationSession::new(project_id);
        let created_session = state
            .ideation_session_repo
            .create(session)
            .await
            .unwrap();

        // Create proposal for session
        let proposal = TaskProposal::new(
            created_session.id.clone(),
            "Test Proposal",
            crate::domain::entities::TaskCategory::Feature,
            crate::domain::entities::Priority::High,
        );
        state.task_proposal_repo.create(proposal).await.unwrap();

        // Create message for session
        let message = ChatMessage::user_in_session(created_session.id.clone(), "Hello");
        state.chat_message_repo.create(message).await.unwrap();

        // Get session with data
        let proposals = state
            .task_proposal_repo
            .get_by_session(&created_session.id)
            .await
            .unwrap();
        let messages = state
            .chat_message_repo
            .get_by_session(&created_session.id)
            .await
            .unwrap();

        assert_eq!(proposals.len(), 1);
        assert_eq!(messages.len(), 1);
    }

    #[tokio::test]
    async fn test_proposal_response_serialization() {
        let session_id = IdeationSessionId::new();
        let proposal = TaskProposal::new(
            session_id,
            "Test Proposal",
            crate::domain::entities::TaskCategory::Feature,
            crate::domain::entities::Priority::High,
        );
        let response = TaskProposalResponse::from(proposal);

        assert!(!response.id.is_empty());
        assert_eq!(response.title, "Test Proposal");
        assert_eq!(response.category, "feature");
        assert_eq!(response.suggested_priority, "high");

        // Verify it serializes to JSON
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"title\":\"Test Proposal\""));
    }

    #[tokio::test]
    async fn test_message_response_serialization() {
        let session_id = IdeationSessionId::new();
        let message = ChatMessage::user_in_session(session_id, "Hello world");
        let response = ChatMessageResponse::from(message);

        assert!(!response.id.is_empty());
        assert_eq!(response.content, "Hello world");
        assert_eq!(response.role, "user");

        // Verify it serializes to JSON
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"content\":\"Hello world\""));
    }

    // ========================================================================
    // Task Proposal Tests
    // ========================================================================

    #[tokio::test]
    async fn test_create_task_proposal() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Create session first
        let session = IdeationSession::new(project_id);
        let created_session = state.ideation_session_repo.create(session).await.unwrap();

        // Create proposal
        let proposal = TaskProposal::new(
            created_session.id.clone(),
            "Test Proposal",
            TaskCategory::Feature,
            Priority::High,
        );
        let created = state.task_proposal_repo.create(proposal).await.unwrap();

        assert_eq!(created.title, "Test Proposal");
        assert_eq!(created.category, TaskCategory::Feature);
        assert_eq!(created.suggested_priority, Priority::High);
    }

    #[tokio::test]
    async fn test_get_task_proposal_returns_none_for_nonexistent() {
        let state = setup_test_state();
        let id = TaskProposalId::new();

        let result = state.task_proposal_repo.get_by_id(&id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_list_session_proposals() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Create session
        let session = IdeationSession::new(project_id);
        let created_session = state.ideation_session_repo.create(session).await.unwrap();

        // Create proposals
        for i in 0..3 {
            let proposal = TaskProposal::new(
                created_session.id.clone(),
                format!("Proposal {}", i),
                TaskCategory::Feature,
                Priority::Medium,
            );
            state.task_proposal_repo.create(proposal).await.unwrap();
        }

        let proposals = state
            .task_proposal_repo
            .get_by_session(&created_session.id)
            .await
            .unwrap();
        assert_eq!(proposals.len(), 3);
    }

    #[tokio::test]
    async fn test_update_task_proposal() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Create session and proposal
        let session = IdeationSession::new(project_id);
        let created_session = state.ideation_session_repo.create(session).await.unwrap();

        let proposal = TaskProposal::new(
            created_session.id.clone(),
            "Original Title",
            TaskCategory::Feature,
            Priority::Low,
        );
        let created = state.task_proposal_repo.create(proposal).await.unwrap();

        // Update proposal
        let mut updated = created.clone();
        updated.title = "Updated Title".to_string();
        updated.user_modified = true;

        state.task_proposal_repo.update(&updated).await.unwrap();

        let retrieved = state
            .task_proposal_repo
            .get_by_id(&created.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.title, "Updated Title");
        assert!(retrieved.user_modified);
    }

    #[tokio::test]
    async fn test_delete_task_proposal() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Create session and proposal
        let session = IdeationSession::new(project_id);
        let created_session = state.ideation_session_repo.create(session).await.unwrap();

        let proposal = TaskProposal::new(
            created_session.id.clone(),
            "To Delete",
            TaskCategory::Feature,
            Priority::Medium,
        );
        let created = state.task_proposal_repo.create(proposal).await.unwrap();

        // Delete proposal
        state.task_proposal_repo.delete(&created.id).await.unwrap();

        let result = state
            .task_proposal_repo
            .get_by_id(&created.id)
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_toggle_proposal_selection() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Create session and proposal
        let session = IdeationSession::new(project_id);
        let created_session = state.ideation_session_repo.create(session).await.unwrap();

        let proposal = TaskProposal::new(
            created_session.id.clone(),
            "Test Proposal",
            TaskCategory::Feature,
            Priority::Medium,
        );
        let created = state.task_proposal_repo.create(proposal).await.unwrap();

        // Initial state should be selected (true)
        assert!(created.selected);

        // Toggle to false
        state
            .task_proposal_repo
            .update_selection(&created.id, false)
            .await
            .unwrap();

        let retrieved = state
            .task_proposal_repo
            .get_by_id(&created.id)
            .await
            .unwrap()
            .unwrap();
        assert!(!retrieved.selected);
    }

    #[tokio::test]
    async fn test_reorder_proposals() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Create session
        let session = IdeationSession::new(project_id);
        let created_session = state.ideation_session_repo.create(session).await.unwrap();

        // Create 3 proposals
        let mut ids = Vec::new();
        for i in 0..3 {
            let proposal = TaskProposal::new(
                created_session.id.clone(),
                format!("Proposal {}", i),
                TaskCategory::Feature,
                Priority::Medium,
            );
            let created = state.task_proposal_repo.create(proposal).await.unwrap();
            ids.push(created.id);
        }

        // Reverse the order
        let reversed_ids: Vec<TaskProposalId> = ids.into_iter().rev().collect();
        state
            .task_proposal_repo
            .reorder(&created_session.id, reversed_ids)
            .await
            .unwrap();

        // Verify order changed
        let proposals = state
            .task_proposal_repo
            .get_by_session(&created_session.id)
            .await
            .unwrap();
        assert_eq!(proposals.len(), 3);
        // The first proposal should now be "Proposal 2"
        assert_eq!(proposals[0].title, "Proposal 2");
    }

    #[tokio::test]
    async fn test_priority_assessment_response() {
        let session_id = IdeationSessionId::new();
        let proposal = TaskProposal::new(
            session_id,
            "Test Proposal",
            TaskCategory::Feature,
            Priority::Critical,
        );

        let response = PriorityAssessmentResponse {
            proposal_id: proposal.id.as_str().to_string(),
            priority: proposal.suggested_priority.to_string(),
            score: proposal.priority_score,
            reason: "Test reason".to_string(),
        };

        assert_eq!(response.priority, "critical");
        assert_eq!(response.reason, "Test reason");

        // Verify it serializes to JSON
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"priority\":\"critical\""));
    }

    // ========================================================================
    // Dependency and Apply Tests
    // ========================================================================

    #[tokio::test]
    async fn test_add_proposal_dependency() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Create session and proposals
        let session = IdeationSession::new(project_id);
        let created_session = state.ideation_session_repo.create(session).await.unwrap();

        let proposal1 = TaskProposal::new(
            created_session.id.clone(),
            "Proposal 1",
            TaskCategory::Feature,
            Priority::Medium,
        );
        let proposal2 = TaskProposal::new(
            created_session.id.clone(),
            "Proposal 2",
            TaskCategory::Feature,
            Priority::Medium,
        );

        let p1 = state.task_proposal_repo.create(proposal1).await.unwrap();
        let p2 = state.task_proposal_repo.create(proposal2).await.unwrap();

        // Add dependency: p1 depends on p2
        state
            .proposal_dependency_repo
            .add_dependency(&p1.id, &p2.id)
            .await
            .unwrap();

        // Verify dependency exists
        let deps = state
            .proposal_dependency_repo
            .get_dependencies(&p1.id)
            .await
            .unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], p2.id);
    }

    #[tokio::test]
    async fn test_remove_proposal_dependency() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Create session and proposals
        let session = IdeationSession::new(project_id);
        let created_session = state.ideation_session_repo.create(session).await.unwrap();

        let proposal1 = TaskProposal::new(
            created_session.id.clone(),
            "Proposal 1",
            TaskCategory::Feature,
            Priority::Medium,
        );
        let proposal2 = TaskProposal::new(
            created_session.id.clone(),
            "Proposal 2",
            TaskCategory::Feature,
            Priority::Medium,
        );

        let p1 = state.task_proposal_repo.create(proposal1).await.unwrap();
        let p2 = state.task_proposal_repo.create(proposal2).await.unwrap();

        // Add then remove dependency
        state
            .proposal_dependency_repo
            .add_dependency(&p1.id, &p2.id)
            .await
            .unwrap();
        state
            .proposal_dependency_repo
            .remove_dependency(&p1.id, &p2.id)
            .await
            .unwrap();

        // Verify dependency was removed
        let deps = state
            .proposal_dependency_repo
            .get_dependencies(&p1.id)
            .await
            .unwrap();
        assert!(deps.is_empty());
    }

    #[tokio::test]
    async fn test_get_proposal_dependents() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Create session and proposals
        let session = IdeationSession::new(project_id);
        let created_session = state.ideation_session_repo.create(session).await.unwrap();

        let proposal1 = TaskProposal::new(
            created_session.id.clone(),
            "Proposal 1",
            TaskCategory::Feature,
            Priority::Medium,
        );
        let proposal2 = TaskProposal::new(
            created_session.id.clone(),
            "Proposal 2",
            TaskCategory::Feature,
            Priority::Medium,
        );

        let p1 = state.task_proposal_repo.create(proposal1).await.unwrap();
        let p2 = state.task_proposal_repo.create(proposal2).await.unwrap();

        // p1 depends on p2, so p2 should have p1 as a dependent
        state
            .proposal_dependency_repo
            .add_dependency(&p1.id, &p2.id)
            .await
            .unwrap();

        let dependents = state
            .proposal_dependency_repo
            .get_dependents(&p2.id)
            .await
            .unwrap();
        assert_eq!(dependents.len(), 1);
        assert_eq!(dependents[0], p1.id);
    }

    #[tokio::test]
    async fn test_analyze_dependencies_empty_session() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Create session with no proposals
        let session = IdeationSession::new(project_id);
        let created_session = state.ideation_session_repo.create(session).await.unwrap();

        // Get dependencies (should be empty graph)
        let proposals = state
            .task_proposal_repo
            .get_by_session(&created_session.id)
            .await
            .unwrap();
        let deps = state
            .proposal_dependency_repo
            .get_all_for_session(&created_session.id)
            .await
            .unwrap();

        let graph = build_dependency_graph(&proposals, &deps);

        assert!(graph.nodes.is_empty());
        assert!(graph.edges.is_empty());
        assert!(!graph.has_cycles);
    }

    #[tokio::test]
    async fn test_dependency_graph_response_serialization() {
        let graph = DependencyGraph::new();
        let response = DependencyGraphResponse::from(graph);

        assert!(response.nodes.is_empty());
        assert!(response.edges.is_empty());
        assert!(!response.has_cycles);

        // Verify it serializes to JSON
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"has_cycles\":false"));
    }

    #[tokio::test]
    async fn test_task_blockers() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Create tasks
        let task1 = crate::domain::entities::Task::new(project_id.clone(), "Task 1".to_string());
        let task2 = crate::domain::entities::Task::new(project_id.clone(), "Task 2".to_string());

        let t1 = state.task_repo.create(task1).await.unwrap();
        let t2 = state.task_repo.create(task2).await.unwrap();

        // Add dependency: t1 depends on t2
        state
            .task_dependency_repo
            .add_dependency(&t1.id, &t2.id)
            .await
            .unwrap();

        // t2 should be a blocker for t1
        let blockers = state
            .task_dependency_repo
            .get_blockers(&t1.id)
            .await
            .unwrap();
        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0], t2.id);

        // t1 should be blocked by t2
        let blocked = state
            .task_dependency_repo
            .get_blocked_by(&t2.id)
            .await
            .unwrap();
        assert_eq!(blocked.len(), 1);
        assert_eq!(blocked[0], t1.id);
    }

    // ========================================================================
    // Chat Message Tests
    // ========================================================================

    #[tokio::test]
    async fn test_send_chat_message_to_session() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Create session first
        let session = IdeationSession::new(project_id);
        let created_session = state.ideation_session_repo.create(session).await.unwrap();

        // Send a message
        let message = ChatMessage::user_in_session(created_session.id.clone(), "Hello world");
        let created = state.chat_message_repo.create(message).await.unwrap();

        assert_eq!(created.content, "Hello world");
        assert_eq!(created.session_id, Some(created_session.id));
    }

    #[tokio::test]
    async fn test_send_chat_message_to_project() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Send a message to project
        let message = ChatMessage::user_in_project(project_id.clone(), "Project message");
        let created = state.chat_message_repo.create(message).await.unwrap();

        assert_eq!(created.content, "Project message");
        assert_eq!(created.project_id, Some(project_id));
        assert!(created.session_id.is_none());
    }

    #[tokio::test]
    async fn test_send_chat_message_about_task() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Create a task
        let task = crate::domain::entities::Task::new(project_id, "Test Task".to_string());
        let created_task = state.task_repo.create(task).await.unwrap();

        // Send a message about the task
        let message = ChatMessage::user_about_task(created_task.id.clone(), "Task message");
        let created = state.chat_message_repo.create(message).await.unwrap();

        assert_eq!(created.content, "Task message");
        assert_eq!(created.task_id, Some(created_task.id));
    }

    #[tokio::test]
    async fn test_get_session_messages() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Create session
        let session = IdeationSession::new(project_id);
        let created_session = state.ideation_session_repo.create(session).await.unwrap();

        // Send multiple messages
        for i in 1..=3 {
            let message =
                ChatMessage::user_in_session(created_session.id.clone(), format!("Message {}", i));
            state.chat_message_repo.create(message).await.unwrap();
        }

        // Get all messages
        let messages = state
            .chat_message_repo
            .get_by_session(&created_session.id)
            .await
            .unwrap();
        assert_eq!(messages.len(), 3);
    }

    #[tokio::test]
    async fn test_get_project_messages() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Send messages to project
        for i in 1..=2 {
            let message =
                ChatMessage::user_in_project(project_id.clone(), format!("Project message {}", i));
            state.chat_message_repo.create(message).await.unwrap();
        }

        // Get all project messages
        let messages = state
            .chat_message_repo
            .get_by_project(&project_id)
            .await
            .unwrap();
        assert_eq!(messages.len(), 2);
    }

    #[tokio::test]
    async fn test_get_task_messages() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Create a task
        let task = crate::domain::entities::Task::new(project_id, "Test Task".to_string());
        let created_task = state.task_repo.create(task).await.unwrap();

        // Send messages about the task
        for i in 1..=2 {
            let message =
                ChatMessage::user_about_task(created_task.id.clone(), format!("Task message {}", i));
            state.chat_message_repo.create(message).await.unwrap();
        }

        // Get all task messages
        let messages = state
            .chat_message_repo
            .get_by_task(&created_task.id)
            .await
            .unwrap();
        assert_eq!(messages.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_chat_message() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Create session and message
        let session = IdeationSession::new(project_id);
        let created_session = state.ideation_session_repo.create(session).await.unwrap();

        let message = ChatMessage::user_in_session(created_session.id.clone(), "To delete");
        let created = state.chat_message_repo.create(message).await.unwrap();

        // Delete the message
        state.chat_message_repo.delete(&created.id).await.unwrap();

        // Verify it's gone
        let result = state.chat_message_repo.get_by_id(&created.id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete_session_messages() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Create session
        let session = IdeationSession::new(project_id);
        let created_session = state.ideation_session_repo.create(session).await.unwrap();

        // Create multiple messages
        for i in 1..=3 {
            let message =
                ChatMessage::user_in_session(created_session.id.clone(), format!("Message {}", i));
            state.chat_message_repo.create(message).await.unwrap();
        }

        // Delete all session messages
        state
            .chat_message_repo
            .delete_by_session(&created_session.id)
            .await
            .unwrap();

        // Verify they're gone
        let messages = state
            .chat_message_repo
            .get_by_session(&created_session.id)
            .await
            .unwrap();
        assert!(messages.is_empty());
    }

    #[tokio::test]
    async fn test_count_session_messages() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Create session
        let session = IdeationSession::new(project_id);
        let created_session = state.ideation_session_repo.create(session).await.unwrap();

        // Create messages
        for i in 1..=5 {
            let message =
                ChatMessage::user_in_session(created_session.id.clone(), format!("Message {}", i));
            state.chat_message_repo.create(message).await.unwrap();
        }

        // Count messages
        let count = state
            .chat_message_repo
            .count_by_session(&created_session.id)
            .await
            .unwrap();
        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn test_get_recent_session_messages() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Create session
        let session = IdeationSession::new(project_id);
        let created_session = state.ideation_session_repo.create(session).await.unwrap();

        // Create 5 messages
        for i in 1..=5 {
            let message =
                ChatMessage::user_in_session(created_session.id.clone(), format!("Message {}", i));
            state.chat_message_repo.create(message).await.unwrap();
        }

        // Get only 3 recent messages
        let messages = state
            .chat_message_repo
            .get_recent_by_session(&created_session.id, 3)
            .await
            .unwrap();
        assert_eq!(messages.len(), 3);
    }

    #[tokio::test]
    async fn test_chat_message_response_includes_all_fields() {
        let session_id = IdeationSessionId::new();
        let mut message = ChatMessage::user_in_session(session_id.clone(), "Test message");
        message.metadata = Some(r#"{"key": "value"}"#.to_string());

        let response = ChatMessageResponse::from(message.clone());

        assert_eq!(response.content, "Test message");
        assert_eq!(response.role, "user");
        assert_eq!(response.session_id, Some(session_id.as_str().to_string()));
        assert!(response.project_id.is_none());
        assert!(response.task_id.is_none());
        assert_eq!(response.metadata, Some(r#"{"key": "value"}"#.to_string()));
    }

    #[tokio::test]
    async fn test_orchestrator_message_in_session() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Create session
        let session = IdeationSession::new(project_id);
        let created_session = state.ideation_session_repo.create(session).await.unwrap();

        // Create orchestrator message
        let message =
            ChatMessage::orchestrator_in_session(created_session.id.clone(), "AI response");
        let created = state.chat_message_repo.create(message).await.unwrap();

        let response = ChatMessageResponse::from(created);
        assert_eq!(response.role, "orchestrator");
        assert_eq!(response.content, "AI response");
    }

    #[tokio::test]
    async fn test_system_message_in_session() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Create session
        let session = IdeationSession::new(project_id);
        let created_session = state.ideation_session_repo.create(session).await.unwrap();

        // Create system message
        let message = ChatMessage::system_in_session(created_session.id.clone(), "Session started");
        let created = state.chat_message_repo.create(message).await.unwrap();

        let response = ChatMessageResponse::from(created);
        assert_eq!(response.role, "system");
        assert_eq!(response.content, "Session started");
    }

    // ========================================================================
    // Ideation Settings Tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_ideation_settings_returns_default() {
        let state = setup_test_state();

        // Get settings (should return default)
        let settings = state
            .ideation_settings_repo
            .get_settings()
            .await
            .unwrap();

        assert_eq!(settings.plan_mode, crate::domain::ideation::IdeationPlanMode::Optional);
        assert!(!settings.require_plan_approval);
        assert!(settings.suggest_plans_for_complex);
        assert!(settings.auto_link_proposals);
    }

    #[tokio::test]
    async fn test_update_ideation_settings() {
        let state = setup_test_state();

        // Create custom settings
        let custom_settings = IdeationSettings {
            plan_mode: crate::domain::ideation::IdeationPlanMode::Required,
            require_plan_approval: true,
            suggest_plans_for_complex: false,
            auto_link_proposals: false,
        };

        // Update settings
        let updated = state
            .ideation_settings_repo
            .update_settings(&custom_settings)
            .await
            .unwrap();

        assert_eq!(updated.plan_mode, crate::domain::ideation::IdeationPlanMode::Required);
        assert!(updated.require_plan_approval);
        assert!(!updated.suggest_plans_for_complex);
        assert!(!updated.auto_link_proposals);
    }

    #[tokio::test]
    async fn test_ideation_settings_persist_across_reads() {
        let state = setup_test_state();

        // Update settings
        let custom_settings = IdeationSettings {
            plan_mode: crate::domain::ideation::IdeationPlanMode::Parallel,
            require_plan_approval: false,
            suggest_plans_for_complex: true,
            auto_link_proposals: false,
        };

        state
            .ideation_settings_repo
            .update_settings(&custom_settings)
            .await
            .unwrap();

        // Read settings again
        let retrieved = state
            .ideation_settings_repo
            .get_settings()
            .await
            .unwrap();

        assert_eq!(retrieved.plan_mode, crate::domain::ideation::IdeationPlanMode::Parallel);
        assert!(!retrieved.require_plan_approval);
        assert!(retrieved.suggest_plans_for_complex);
        assert!(!retrieved.auto_link_proposals);
    }
}
