// Repository Operations for ChatService
//
// Helper functions for interacting with repositories in the ChatService implementation.

use std::sync::Arc;

use crate::domain::entities::{
    ChatContextType, ChatConversation, ChatConversationId, IdeationSessionId, ProjectId, TaskId,
};
use crate::domain::repositories::{ChatConversationRepository, ChatMessageRepository};

use super::chat_service_types::{ChatConversationWithMessages, ChatServiceError};

/// Get or create a conversation for a context
pub async fn get_or_create_conversation(
    conversation_repo: Arc<dyn ChatConversationRepository>,
    context_type: ChatContextType,
    context_id: &str,
) -> Result<ChatConversation, ChatServiceError> {
    // TaskExecution: ALWAYS create a fresh conversation — never reuse prior run context.
    // Worker agents reconstruct context via MCP tools (get_task_context, get_review_notes).
    // Mirrors is_fresh_review_cycle logic in chat_service_context.rs.
    if context_type != ChatContextType::TaskExecution {
        // Try to get existing active conversation
        if let Some(conv) = conversation_repo
            .get_active_for_context(context_type, context_id)
            .await
            .map_err(|e| ChatServiceError::RepositoryError(e.to_string()))?
        {
            return Ok(conv);
        }
    }

    // For TaskExecution, look up the most recent prior conversation to set as parent.
    let parent_conversation_id = if context_type == ChatContextType::TaskExecution {
        conversation_repo
            .get_active_for_context(context_type, context_id)
            .await
            .map_err(|e| ChatServiceError::RepositoryError(e.to_string()))?
            .map(|c| c.id.as_str())
    } else {
        None
    };

    // Create new conversation based on context type
    let mut conv = match context_type {
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
        ChatContextType::Merge => {
            ChatConversation::new_merge(TaskId::from_string(context_id.to_string()))
        }
    };

    conv.parent_conversation_id = parent_conversation_id;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::memory::MemoryChatConversationRepository;

    fn make_repo() -> Arc<dyn ChatConversationRepository> {
        Arc::new(MemoryChatConversationRepository::new())
    }

    // ── TaskExecution always creates a new conversation ──────────────────────

    #[tokio::test]
    async fn task_execution_creates_new_conversation_even_when_prior_exists() {
        let repo = make_repo();
        let task_id = "task-abc-123";

        // First call creates a conversation
        let first = get_or_create_conversation(
            repo.clone(),
            ChatContextType::TaskExecution,
            task_id,
        )
        .await
        .unwrap();

        // Second call must create a NEW row, not return the existing one
        let second = get_or_create_conversation(
            repo.clone(),
            ChatContextType::TaskExecution,
            task_id,
        )
        .await
        .unwrap();

        assert_ne!(first.id, second.id, "Expected a fresh conversation each time");
        assert_eq!(second.context_type, ChatContextType::TaskExecution);
        assert_eq!(second.context_id, task_id);
    }

    // ── parent_conversation_id is set correctly on re-execution ──────────────

    #[tokio::test]
    async fn task_execution_second_run_has_parent_conversation_id() {
        let repo = make_repo();
        let task_id = "task-xyz-456";

        // First run — no parent yet
        let first = get_or_create_conversation(
            repo.clone(),
            ChatContextType::TaskExecution,
            task_id,
        )
        .await
        .unwrap();
        assert!(
            first.parent_conversation_id.is_none(),
            "First run must have no parent"
        );

        // Second run — should point to first run
        let second = get_or_create_conversation(
            repo.clone(),
            ChatContextType::TaskExecution,
            task_id,
        )
        .await
        .unwrap();
        assert_eq!(
            second.parent_conversation_id.as_deref(),
            Some(first.id.as_str().as_str()),
            "Second run must reference first run's conversation id"
        );
    }

    // ── Old conversations remain visible via list_conversations ──────────────

    #[tokio::test]
    async fn old_task_execution_conversations_remain_accessible() {
        let repo = make_repo();
        let task_id = "task-old-999";

        // Create two runs
        get_or_create_conversation(repo.clone(), ChatContextType::TaskExecution, task_id)
            .await
            .unwrap();
        get_or_create_conversation(repo.clone(), ChatContextType::TaskExecution, task_id)
            .await
            .unwrap();

        let all = list_conversations(repo, ChatContextType::TaskExecution, task_id)
            .await
            .unwrap();

        assert_eq!(all.len(), 2, "Both execution conversations must be listed");
    }

    // ── Non-TaskExecution contexts reuse existing conversation ───────────────

    #[tokio::test]
    async fn non_task_execution_reuses_existing_conversation() {
        let repo = make_repo();
        let task_id = "task-review-111";

        let first = get_or_create_conversation(
            repo.clone(),
            ChatContextType::Task,
            task_id,
        )
        .await
        .unwrap();

        let second = get_or_create_conversation(
            repo.clone(),
            ChatContextType::Task,
            task_id,
        )
        .await
        .unwrap();

        assert_eq!(first.id, second.id, "Non-TaskExecution must reuse existing conversation");
    }
}
