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
    pub content_blocks: Option<String>,
    pub created_at: String,
}

/// Response for send_context_message
#[derive(Debug, Serialize)]
pub struct SendContextMessageResponse {
    pub response_text: String,
    pub tool_calls: Vec<ToolCallResponse>,
    pub claude_session_id: Option<String>,
    pub conversation_id: Option<String>,
}

/// Tool call information in response
#[derive(Debug, Serialize)]
pub struct ToolCallResponse {
    pub id: Option<String>,
    pub name: String,
    pub arguments: serde_json::Value,
    pub result: Option<serde_json::Value>,
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
            content_blocks: message.content_blocks,
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
/// This command:
/// 1. Gets or creates a conversation for the context
/// 2. Creates an agent run
/// 3. Saves the user message
/// 4. Spawns Claude CLI with appropriate flags (--agent or --resume)
/// 5. Streams response and emits Tauri events
/// 6. Saves the assistant response with tool calls
/// 7. Returns the orchestrator result
#[tauri::command]
pub async fn send_context_message(
    input: SendContextMessageInput,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<SendContextMessageResponse, String> {
    use crate::application::{ClaudeOrchestratorService, OrchestratorService};

    // Parse context type
    let context_type: ChatContextType = input
        .context_type
        .parse()
        .map_err(|e: String| format!("Invalid context type: {}", e))?;

    // Create orchestrator service with required repositories and app handle for events
    let orchestrator: ClaudeOrchestratorService<tauri::Wry> = ClaudeOrchestratorService::new(
        state.chat_message_repo.clone(),
        state.chat_conversation_repo.clone(),
        state.agent_run_repo.clone(),
        state.project_repo.clone(),
        state.task_repo.clone(),
        state.ideation_session_repo.clone(),
    )
    .with_app_handle(app);

    // Check if orchestrator is available
    if !orchestrator.is_available().await {
        return Err("Claude CLI is not available. Please ensure 'claude' is installed and in your PATH.".to_string());
    }

    // Send message and get response
    let result = orchestrator
        .send_context_message(context_type, &input.context_id, &input.content)
        .await
        .map_err(|e| e.to_string())?;

    // Build response
    Ok(SendContextMessageResponse {
        response_text: result.response_text,
        tool_calls: result.tool_calls.into_iter().map(|tc| ToolCallResponse {
            id: tc.id,
            name: tc.name,
            arguments: tc.arguments,
            result: tc.result,
        }).collect(),
        claude_session_id: result.claude_session_id,
        conversation_id: result.conversation_id.map(|id| id.as_str().to_string()),
    })
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

    // Get messages for this specific conversation
    let messages = state
        .chat_message_repo
        .get_by_conversation(&conversation_id)
        .await
        .map_err(|e| e.to_string())?;

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
        ChatContextType::TaskExecution => {
            ChatConversation::new_task_execution(TaskId::from_string(input.context_id.clone()))
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
