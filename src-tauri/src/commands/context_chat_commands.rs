// Tauri commands for context-aware chat
// Supports conversation history, session management, and agent run tracking

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::application::AppState;
use crate::domain::entities::{
    AgentRun, ChatContextType, ChatConversation, ChatConversationId, ChatMessage,
    IdeationSessionId, ProjectId, TaskId,
};

/// Input for sending a message in a context-aware conversation
#[derive(Debug, Deserialize)]
pub struct SendContextMessageInput {
    pub context_type: String, // "ideation", "task", or "project"
    pub context_id: String,   // Session ID, Task ID, or Project ID
    pub content: String,
}

/// Input for creating a new conversation
#[derive(Debug, Deserialize)]
pub struct CreateConversationInput {
    pub context_type: String,
    pub context_id: String,
}

/// Response for ChatConversation
#[derive(Debug, Serialize)]
pub struct ChatConversationResponse {
    pub id: String,
    pub context_type: String,
    pub context_id: String,
    pub claude_session_id: Option<String>,
    pub title: Option<String>,
    pub message_count: i64,
    pub last_message_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ChatConversation> for ChatConversationResponse {
    fn from(conv: ChatConversation) -> Self {
        Self {
            id: conv.id.as_str().to_string(),
            context_type: conv.context_type.to_string(),
            context_id: conv.context_id,
            claude_session_id: conv.claude_session_id,
            title: conv.title,
            message_count: conv.message_count,
            last_message_at: conv.last_message_at.map(|dt| dt.to_rfc3339()),
            created_at: conv.created_at.to_rfc3339(),
            updated_at: conv.updated_at.to_rfc3339(),
        }
    }
}

/// Response for AgentRun
#[derive(Debug, Serialize)]
pub struct AgentRunResponse {
    pub id: String,
    pub conversation_id: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
}

impl From<AgentRun> for AgentRunResponse {
    fn from(run: AgentRun) -> Self {
        Self {
            id: run.id.as_str().to_string(),
            conversation_id: run.conversation_id.as_str().to_string(),
            status: run.status.to_string(),
            started_at: run.started_at.to_rfc3339(),
            completed_at: run.completed_at.map(|dt| dt.to_rfc3339()),
            error_message: run.error_message,
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
    pub conversation_id: Option<String>,
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
            conversation_id: message.conversation_id.map(|id| id.as_str().to_string()),
            role: message.role.to_string(),
            content: message.content,
            metadata: message.metadata,
            parent_message_id: message.parent_message_id.map(|id| id.as_str().to_string()),
            tool_calls: message.tool_calls,
            created_at: message.created_at.to_rfc3339(),
        }
    }
}

/// Response for conversation with messages
#[derive(Debug, Serialize)]
pub struct ConversationWithMessagesResponse {
    pub conversation: ChatConversationResponse,
    pub messages: Vec<ChatMessageResponse>,
}

/// Send a message in a context-aware conversation
///
/// NOTE: This is a stub implementation. The full orchestration logic (agent runs,
/// --resume support, streaming, tool calls) will be implemented when the orchestrator
/// service is refactored in task 16.
#[tauri::command]
pub async fn send_context_message(
    input: SendContextMessageInput,
    state: State<'_, AppState>,
) -> Result<ChatConversationResponse, String> {
    // Parse context type
    let context_type: ChatContextType = input
        .context_type
        .parse()
        .map_err(|e: String| format!("Invalid context type: {}", e))?;

    // Get or create conversation for this context
    let conversation = match state
        .chat_conversation_repo
        .get_active_for_context(context_type, &input.context_id)
        .await
        .map_err(|e| e.to_string())?
    {
        Some(conv) => conv,
        None => {
            // Create a new conversation
            let new_conv = match context_type {
                ChatContextType::Ideation => {
                    ChatConversation::new_ideation(IdeationSessionId::from_string(&input.context_id))
                }
                ChatContextType::Task => {
                    ChatConversation::new_task(TaskId::from_string(input.context_id.clone()))
                }
                ChatContextType::Project => {
                    ChatConversation::new_project(ProjectId::from_string(input.context_id.clone()))
                }
            };
            state
                .chat_conversation_repo
                .create(new_conv)
                .await
                .map_err(|e| e.to_string())?
        }
    };

    // TODO (task 16): Implement orchestrator integration
    // - Create agent run
    // - Spawn Claude CLI with --resume if claude_session_id exists
    // - Stream response
    // - Save messages and tool calls
    // - Emit Tauri events for UI updates

    Ok(ChatConversationResponse::from(conversation))
}

/// List all conversations for a context
#[tauri::command]
pub async fn list_conversations(
    context_type: String,
    context_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ChatConversationResponse>, String> {
    let context_type: ChatContextType = context_type
        .parse()
        .map_err(|e: String| format!("Invalid context type: {}", e))?;

    state
        .chat_conversation_repo
        .get_by_context(context_type, &context_id)
        .await
        .map(|conversations| {
            conversations
                .into_iter()
                .map(ChatConversationResponse::from)
                .collect()
        })
        .map_err(|e| e.to_string())
}

/// Get a conversation with all its messages
#[tauri::command]
pub async fn get_conversation(
    conversation_id: String,
    state: State<'_, AppState>,
) -> Result<Option<ConversationWithMessagesResponse>, String> {
    let conversation_id = ChatConversationId::from_string(&conversation_id);

    // Get conversation
    let conversation = match state
        .chat_conversation_repo
        .get_by_id(&conversation_id)
        .await
        .map_err(|e| e.to_string())?
    {
        Some(c) => c,
        None => return Ok(None),
    };

    // Get messages for this conversation
    // TODO (task 16): Implement get_by_conversation in ChatMessageRepository
    // For now, return messages based on context type
    let messages = match conversation.context_type {
        ChatContextType::Ideation => {
            let session_id = IdeationSessionId::from_string(&conversation.context_id);
            state
                .chat_message_repo
                .get_by_session(&session_id)
                .await
                .map_err(|e| e.to_string())?
        }
        ChatContextType::Task => {
            let task_id = TaskId::from_string(conversation.context_id.clone());
            state
                .chat_message_repo
                .get_by_task(&task_id)
                .await
                .map_err(|e| e.to_string())?
        }
        ChatContextType::Project => {
            let project_id = ProjectId::from_string(conversation.context_id.clone());
            state
                .chat_message_repo
                .get_by_project(&project_id)
                .await
                .map_err(|e| e.to_string())?
        }
    };

    Ok(Some(ConversationWithMessagesResponse {
        conversation: ChatConversationResponse::from(conversation),
        messages: messages.into_iter().map(ChatMessageResponse::from).collect(),
    }))
}

/// Create a new conversation for a context
#[tauri::command]
pub async fn create_conversation(
    input: CreateConversationInput,
    state: State<'_, AppState>,
) -> Result<ChatConversationResponse, String> {
    let context_type: ChatContextType = input
        .context_type
        .parse()
        .map_err(|e: String| format!("Invalid context type: {}", e))?;

    let conversation = match context_type {
        ChatContextType::Ideation => {
            ChatConversation::new_ideation(IdeationSessionId::from_string(&input.context_id))
        }
        ChatContextType::Task => {
            ChatConversation::new_task(TaskId::from_string(input.context_id.clone()))
        }
        ChatContextType::Project => {
            ChatConversation::new_project(ProjectId::from_string(input.context_id.clone()))
        }
    };

    state
        .chat_conversation_repo
        .create(conversation)
        .await
        .map(ChatConversationResponse::from)
        .map_err(|e| e.to_string())
}

/// Get the current agent run status for a conversation (if any)
#[tauri::command]
pub async fn get_agent_run_status(
    conversation_id: String,
    state: State<'_, AppState>,
) -> Result<Option<AgentRunResponse>, String> {
    let conversation_id = ChatConversationId::from_string(&conversation_id);

    state
        .agent_run_repo
        .get_active_for_conversation(&conversation_id)
        .await
        .map(|opt| opt.map(AgentRunResponse::from))
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_conversation_response_serialization() {
        use crate::domain::entities::IdeationSessionId;

        let session_id = IdeationSessionId::new();
        let conversation = ChatConversation::new_ideation(session_id);
        let response = ChatConversationResponse::from(conversation.clone());

        assert_eq!(response.id, conversation.id.as_str().to_string());
        assert_eq!(response.context_type, "ideation");
        assert_eq!(response.context_id, conversation.context_id);
        assert_eq!(response.message_count, 0);
    }

    #[test]
    fn test_agent_run_response_serialization() {
        use crate::domain::entities::{AgentRun, ChatConversationId};

        let conversation_id = ChatConversationId::new();
        let run = AgentRun::new(conversation_id);
        let response = AgentRunResponse::from(run.clone());

        assert_eq!(response.id, run.id.as_str().to_string());
        assert_eq!(response.conversation_id, conversation_id.as_str().to_string());
        assert_eq!(response.status, "running");
        assert!(response.completed_at.is_none());
        assert!(response.error_message.is_none());
    }
}
