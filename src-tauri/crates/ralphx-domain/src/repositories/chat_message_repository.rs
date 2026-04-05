// Chat message repository trait - domain layer abstraction
//
// This trait defines the contract for chat message persistence.
// Messages can belong to ideation sessions, projects, or specific tasks.

use async_trait::async_trait;

use crate::domain::entities::{
    ChatConversationId, ChatMessage, ChatMessageId, IdeationSessionId, ProjectId, TaskId,
};
use crate::error::AppResult;

/// Repository trait for ChatMessage persistence.
/// Implementations can use SQLite, PostgreSQL, in-memory, etc.
#[async_trait]
pub trait ChatMessageRepository: Send + Sync {
    /// Create a new chat message
    async fn create(&self, message: ChatMessage) -> AppResult<ChatMessage>;

    /// Get message by ID
    async fn get_by_id(&self, id: &ChatMessageId) -> AppResult<Option<ChatMessage>>;

    /// Get all messages for an ideation session, ordered by created_at ASC
    async fn get_by_session(&self, session_id: &IdeationSessionId) -> AppResult<Vec<ChatMessage>>;

    /// Get all messages for a project (not in any session), ordered by created_at ASC
    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<ChatMessage>>;

    /// Get all messages for a specific task, ordered by created_at ASC
    async fn get_by_task(&self, task_id: &TaskId) -> AppResult<Vec<ChatMessage>>;

    /// Get all messages for a specific conversation, ordered by created_at ASC
    async fn get_by_conversation(
        &self,
        conversation_id: &ChatConversationId,
    ) -> AppResult<Vec<ChatMessage>>;

    /// Delete all messages for a session
    async fn delete_by_session(&self, session_id: &IdeationSessionId) -> AppResult<()>;

    /// Delete all messages for a project
    async fn delete_by_project(&self, project_id: &ProjectId) -> AppResult<()>;

    /// Delete all messages for a task
    async fn delete_by_task(&self, task_id: &TaskId) -> AppResult<()>;

    /// Delete a single message
    async fn delete(&self, id: &ChatMessageId) -> AppResult<()>;

    /// Count messages in a session
    async fn count_by_session(&self, session_id: &IdeationSessionId) -> AppResult<u32>;

    /// Get recent messages for a session (with limit)
    async fn get_recent_by_session(
        &self,
        session_id: &IdeationSessionId,
        limit: u32,
    ) -> AppResult<Vec<ChatMessage>>;

    /// Get recent messages for a session with pagination (limit + offset for older history)
    async fn get_recent_by_session_paginated(
        &self,
        session_id: &IdeationSessionId,
        limit: u32,
        offset: u32,
    ) -> AppResult<Vec<ChatMessage>>;

    /// Update message content, tool_calls, and content_blocks (for incremental persistence)
    async fn update_content(
        &self,
        id: &ChatMessageId,
        content: &str,
        tool_calls: Option<&str>,
        content_blocks: Option<&str>,
    ) -> AppResult<()>;

    /// Count assistant/orchestrator messages in a session newer than the given message ID.
    ///
    /// If `after_message_id` is None, counts ALL assistant/orchestrator messages in the session.
    /// Used for read-before-write enforcement: external agents must read replies before sending.
    ///
    /// Roles counted: "assistant" and "orchestrator"
    async fn count_unread_assistant_messages(
        &self,
        session_id: &str,
        after_message_id: Option<&str>,
    ) -> AppResult<u32>;

    /// Count User + Orchestrator messages in a session newer than the cursor message.
    ///
    /// Matches the role filter used by the GET /messages endpoint (User + Orchestrator only).
    /// System, Worker, Reviewer, and Merger messages are excluded to prevent deadlock.
    ///
    /// Two branches:
    /// - `cursor_message_id` is Some: counts messages created after the cursor's created_at
    /// - `cursor_message_id` is None: counts ALL User + Orchestrator messages in the session
    async fn count_unread_messages(
        &self,
        session_id: &str,
        cursor_message_id: Option<&str>,
    ) -> AppResult<i64>;

    /// Get the content of the first user message for a given context (context_type + context_id).
    ///
    /// Returns the content of the earliest user-role message for this context, or None if no
    /// user messages exist. Used for Jaccard similarity comparison during session dedup.
    async fn get_first_user_message_by_context(
        &self,
        context_type: &str,
        context_id: &str,
    ) -> AppResult<Option<String>>;

    /// Get the most recent message for a session filtered by role.
    ///
    /// Ordered by `created_at DESC, rowid DESC` to guarantee a deterministic result when
    /// multiple messages share the same timestamp. Returns `None` when no messages match.
    async fn get_latest_message_by_role(
        &self,
        session_id: &IdeationSessionId,
        role: &str,
    ) -> AppResult<Option<ChatMessage>>;
}

#[cfg(test)]
#[path = "chat_message_repository_tests.rs"]
mod tests;
