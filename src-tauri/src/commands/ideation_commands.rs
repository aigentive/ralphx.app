// Tauri commands for Ideation Session and Proposal CRUD operations
// Thin layer that delegates to repositories

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::application::AppState;
use crate::domain::entities::{
    ChatMessage, Complexity, IdeationSession, IdeationSessionId, IdeationSessionStatus, Priority,
    ProjectId, TaskCategory, TaskProposal, TaskProposalId,
};

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

    state
        .task_proposal_repo
        .create(proposal)
        .await
        .map(TaskProposalResponse::from)
        .map_err(|e| e.to_string())
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

    Ok(TaskProposalResponse::from(proposal))
}

/// Delete a task proposal
#[tauri::command]
pub async fn delete_task_proposal(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let proposal_id = TaskProposalId::from_string(id);
    state
        .task_proposal_repo
        .delete(&proposal_id)
        .await
        .map_err(|e| e.to_string())
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
                &format!("Proposal {}", i),
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
                &format!("Proposal {}", i),
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
}
