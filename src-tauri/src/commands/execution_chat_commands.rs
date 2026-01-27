// Tauri commands for task execution chat
// Supports viewing worker execution as chat and queueing messages to worker

use serde::Serialize;
use tauri::State;

use crate::application::AppState;
use crate::domain::entities::{ChatContextType, TaskId};
use crate::domain::services::QueuedMessage;

/// Response for QueuedMessage
#[derive(Debug, Serialize)]
pub struct QueuedMessageResponse {
    pub id: String,
    pub content: String,
    pub created_at: String,
    pub is_editing: bool,
}

impl From<QueuedMessage> for QueuedMessageResponse {
    fn from(message: QueuedMessage) -> Self {
        Self {
            id: message.id,
            content: message.content,
            created_at: message.created_at,
            is_editing: message.is_editing,
        }
    }
}

/// Get the active execution conversation for a task
///
/// Returns the most recent task_execution conversation for the given task_id.
/// This is the conversation created when the worker starts executing the task.
#[tauri::command]
pub async fn get_execution_conversation(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<Option<super::context_chat_commands::ChatConversationResponse>, String> {
    let task_id = TaskId::from_string(task_id);

    state
        .chat_conversation_repo
        .get_active_for_context(ChatContextType::TaskExecution, task_id.as_str())
        .await
        .map(|opt| opt.map(super::context_chat_commands::ChatConversationResponse::from))
        .map_err(|e| e.to_string())
}

/// List all execution attempts for a task
///
/// Returns all task_execution conversations for the given task_id,
/// ordered by created_at DESC (most recent first).
/// Each execution attempt creates a new conversation.
#[tauri::command]
pub async fn list_task_executions(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<super::context_chat_commands::ChatConversationResponse>, String> {
    let task_id = TaskId::from_string(task_id);

    state
        .chat_conversation_repo
        .get_by_context(ChatContextType::TaskExecution, task_id.as_str())
        .await
        .map(|conversations| {
            conversations
                .into_iter()
                .map(super::context_chat_commands::ChatConversationResponse::from)
                .collect()
        })
        .map_err(|e| e.to_string())
}

/// Queue a message to be sent to the worker when it finishes its current response
///
/// The message is held in memory and will be sent via --resume when the worker
/// completes its current response. Returns the queued message with generated ID
/// and timestamp.
#[tauri::command]
pub async fn queue_execution_message(
    task_id: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<QueuedMessageResponse, String> {
    let message = state.message_queue.queue(ChatContextType::TaskExecution, &task_id, content);

    Ok(QueuedMessageResponse::from(message))
}

/// Get all queued messages for a task
///
/// Returns messages in the order they will be sent (FIFO).
#[tauri::command]
pub async fn get_queued_execution_messages(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<QueuedMessageResponse>, String> {
    let messages = state.message_queue.get_queued(ChatContextType::TaskExecution, &task_id);

    Ok(messages
        .into_iter()
        .map(QueuedMessageResponse::from)
        .collect())
}

/// Delete a queued message before it's sent
///
/// Returns true if the message was found and deleted, false otherwise.
#[tauri::command]
pub async fn delete_queued_execution_message(
    task_id: String,
    message_id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    Ok(state.message_queue.delete(ChatContextType::TaskExecution, &task_id, &message_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queued_message_response_serialization() {
        let message = QueuedMessage::new("Test message".to_string());
        let response = QueuedMessageResponse::from(message.clone());

        assert_eq!(response.id, message.id);
        assert_eq!(response.content, "Test message");
        assert_eq!(response.created_at, message.created_at);
        assert!(!response.is_editing);
    }
}
