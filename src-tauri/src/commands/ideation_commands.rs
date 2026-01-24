// Tauri commands for Ideation Session CRUD operations
// Thin layer that delegates to IdeationSessionRepository

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::application::AppState;
use crate::domain::entities::{
    ChatMessage, IdeationSession, IdeationSessionId, IdeationSessionStatus, ProjectId,
    TaskProposal,
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
}
