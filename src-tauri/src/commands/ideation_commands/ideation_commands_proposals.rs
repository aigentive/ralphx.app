// Proposal CRUD and management commands

use tauri::{Emitter, State};

use crate::application::AppState;
use crate::domain::entities::{
    Complexity, IdeationSessionId, IdeationSessionStatus, Priority,
    TaskCategory, TaskProposal, TaskProposalId,
};

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
