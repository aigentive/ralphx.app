// Chat message commands

use tauri::State;

use crate::application::AppState;
use crate::domain::entities::{
    ChatMessage, ChatMessageId, IdeationSessionId, IdeationSessionStatus,
    MessageRole, ProjectId, TaskId,
};

use super::ideation_commands_types::{ChatMessageResponse, SendChatMessageInput};

// ============================================================================
// Chat Message Commands
// ============================================================================

/// Send a chat message
#[tauri::command]
pub async fn send_chat_message(
    input: SendChatMessageInput,
    state: State<'_, AppState>,
) -> Result<ChatMessageResponse, String> {
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
            MessageRole::Reviewer => {
                // Reviewer messages are typically not created through this endpoint
                // but we handle them for completeness
                let mut msg = ChatMessage::user_in_session(session_id, &input.content);
                msg.role = MessageRole::Reviewer;
                msg
            }
            MessageRole::Merger => {
                // Merger messages are typically not created through this endpoint
                // but we handle them for completeness
                let mut msg = ChatMessage::user_in_session(session_id, &input.content);
                msg.role = MessageRole::Merger;
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
