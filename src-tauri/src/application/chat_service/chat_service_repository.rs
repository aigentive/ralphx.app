// Repository Operations for ChatService
//
// Helper functions for interacting with repositories in the ChatService implementation.

use std::sync::Arc;

use crate::domain::entities::{
    ChatConversation, ChatConversationId, ChatContextType, IdeationSessionId, ProjectId, TaskId,
};
use crate::domain::repositories::{ChatConversationRepository, ChatMessageRepository};

use super::chat_service_types::{ChatConversationWithMessages, ChatServiceError};

/// Get or create a conversation for a context
pub async fn get_or_create_conversation(
    conversation_repo: Arc<dyn ChatConversationRepository>,
    context_type: ChatContextType,
    context_id: &str,
) -> Result<ChatConversation, ChatServiceError> {
    // Try to get existing active conversation
    if let Some(conv) = conversation_repo
        .get_active_for_context(context_type, context_id)
        .await
        .map_err(|e| ChatServiceError::RepositoryError(e.to_string()))?
    {
        return Ok(conv);
    }

    // Create new conversation based on context type
    let conv = match context_type {
        ChatContextType::Ideation => {
            ChatConversation::new_ideation(IdeationSessionId::from_string(context_id))
        }
        ChatContextType::Task => {
            ChatConversation::new_task(TaskId::from_string(context_id.to_string()))
        }
        ChatContextType::Project => {
            ChatConversation::new_project(ProjectId::from_string(context_id.to_string()))
        }
        ChatContextType::TaskExecution => {
            ChatConversation::new_task_execution(TaskId::from_string(context_id.to_string()))
        }
        ChatContextType::Review => {
            ChatConversation::new_review(TaskId::from_string(context_id.to_string()))
        }
    };

    conversation_repo
        .create(conv.clone())
        .await
        .map_err(|e| ChatServiceError::RepositoryError(e.to_string()))
}

/// Get a conversation by ID with all its messages
pub async fn get_conversation_with_messages(
    conversation_repo: Arc<dyn ChatConversationRepository>,
    chat_message_repo: Arc<dyn ChatMessageRepository>,
    conversation_id: &ChatConversationId,
) -> Result<Option<ChatConversationWithMessages>, ChatServiceError> {
    let conversation = match conversation_repo
        .get_by_id(conversation_id)
        .await
        .map_err(|e| ChatServiceError::RepositoryError(e.to_string()))?
    {
        Some(c) => c,
        None => return Ok(None),
    };

    let messages = chat_message_repo
        .get_by_conversation(conversation_id)
        .await
        .map_err(|e| ChatServiceError::RepositoryError(e.to_string()))?;

    Ok(Some(ChatConversationWithMessages {
        conversation,
        messages,
    }))
}

/// List all conversations for a context
pub async fn list_conversations(
    conversation_repo: Arc<dyn ChatConversationRepository>,
    context_type: ChatContextType,
    context_id: &str,
) -> Result<Vec<ChatConversation>, ChatServiceError> {
    conversation_repo
        .get_by_context(context_type, context_id)
        .await
        .map_err(|e| ChatServiceError::RepositoryError(e.to_string()))
}
