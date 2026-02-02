// Mock Chat Service
//
// Extracted from chat_service.rs to improve modularity and reduce file size.
// Provides a mock implementation for testing purposes.

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::domain::entities::{
    AgentRun, ChatConversation, ChatConversationId, ChatContextType, IdeationSessionId,
    ProjectId, TaskId,
};
use crate::domain::services::{MessageQueue, QueuedMessage};
use crate::infrastructure::agents::claude::ToolCall;

use super::{ChatConversationWithMessages, ChatService, ChatServiceError, SendResult};

// ============================================================================
// MockChatService - For testing
// ============================================================================

/// Mock chat service for testing
pub struct MockChatService {
    responses: Mutex<Vec<MockChatResponse>>,
    is_available: Mutex<bool>,
    conversations: Mutex<Vec<ChatConversation>>,
    active_run: Mutex<Option<AgentRun>>,
    message_queue: Arc<MessageQueue>,
    call_count: std::sync::atomic::AtomicU32,
}

pub struct MockChatResponse {
    pub text: String,
    pub tool_calls: Vec<ToolCall>,
    pub claude_session_id: Option<String>,
}

impl MockChatService {
    pub fn new() -> Self {
        Self {
            responses: Mutex::new(Vec::new()),
            is_available: Mutex::new(true),
            conversations: Mutex::new(Vec::new()),
            active_run: Mutex::new(None),
            message_queue: Arc::new(MessageQueue::new()),
            call_count: std::sync::atomic::AtomicU32::new(0),
        }
    }

    pub fn with_queue(message_queue: Arc<MessageQueue>) -> Self {
        Self {
            responses: Mutex::new(Vec::new()),
            is_available: Mutex::new(true),
            conversations: Mutex::new(Vec::new()),
            active_run: Mutex::new(None),
            message_queue,
            call_count: std::sync::atomic::AtomicU32::new(0),
        }
    }

    pub fn call_count(&self) -> u32 {
        self.call_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub async fn set_available(&self, available: bool) {
        *self.is_available.lock().await = available;
    }

    pub async fn queue_response(&self, response: MockChatResponse) {
        self.responses.lock().await.push(response);
    }

    pub async fn queue_text_response(&self, text: impl Into<String>) {
        self.queue_response(MockChatResponse {
            text: text.into(),
            tool_calls: Vec::new(),
            claude_session_id: Some(uuid::Uuid::new_v4().to_string()),
        })
        .await;
    }

    pub async fn set_active_run(&self, run: Option<AgentRun>) {
        *self.active_run.lock().await = run;
    }

    pub async fn add_conversation(&self, conv: ChatConversation) {
        self.conversations.lock().await.push(conv);
    }
}

impl Default for MockChatService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ChatService for MockChatService {
    async fn send_message(
        &self,
        context_type: ChatContextType,
        context_id: &str,
        _message: &str,
    ) -> Result<SendResult, ChatServiceError> {
        self.call_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        if !*self.is_available.lock().await {
            return Err(ChatServiceError::AgentNotAvailable(
                "Mock agent not available".to_string(),
            ));
        }

        let conversation = self
            .get_or_create_conversation(context_type, context_id)
            .await?;
        let agent_run = AgentRun::new(conversation.id);

        Ok(SendResult {
            conversation_id: conversation.id.as_str().to_string(),
            agent_run_id: agent_run.id.as_str().to_string(),
            is_new_conversation: conversation.claude_session_id.is_none(),
        })
    }

    async fn queue_message(
        &self,
        context_type: ChatContextType,
        context_id: &str,
        content: &str,
        client_id: Option<&str>,
    ) -> Result<QueuedMessage, ChatServiceError> {
        Ok(match client_id {
            Some(id) => self.message_queue.queue_with_client_id(
                context_type,
                context_id,
                content.to_string(),
                id.to_string(),
            ),
            None => self
                .message_queue
                .queue(context_type, context_id, content.to_string()),
        })
    }

    async fn get_queued_messages(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> Result<Vec<QueuedMessage>, ChatServiceError> {
        Ok(self.message_queue.get_queued(context_type, context_id))
    }

    async fn delete_queued_message(
        &self,
        context_type: ChatContextType,
        context_id: &str,
        message_id: &str,
    ) -> Result<bool, ChatServiceError> {
        Ok(self
            .message_queue
            .delete(context_type, context_id, message_id))
    }

    async fn get_or_create_conversation(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> Result<ChatConversation, ChatServiceError> {
        let conversations = self.conversations.lock().await;

        if let Some(conv) = conversations
            .iter()
            .find(|c| c.context_type == context_type && c.context_id == context_id)
        {
            return Ok(conv.clone());
        }
        drop(conversations);

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
            ChatContextType::Merge => {
                ChatConversation::new_merge(TaskId::from_string(context_id.to_string()))
            }
        };

        self.conversations.lock().await.push(conv.clone());
        Ok(conv)
    }

    async fn get_conversation_with_messages(
        &self,
        conversation_id: &ChatConversationId,
    ) -> Result<Option<ChatConversationWithMessages>, ChatServiceError> {
        let conversations = self.conversations.lock().await;
        let conv = conversations
            .iter()
            .find(|c| c.id == *conversation_id)
            .cloned();

        Ok(conv.map(|c| ChatConversationWithMessages {
            conversation: c,
            messages: Vec::new(),
        }))
    }

    async fn list_conversations(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> Result<Vec<ChatConversation>, ChatServiceError> {
        let conversations = self.conversations.lock().await;
        Ok(conversations
            .iter()
            .filter(|c| c.context_type == context_type && c.context_id == context_id)
            .cloned()
            .collect())
    }

    async fn get_active_run(
        &self,
        _conversation_id: &ChatConversationId,
    ) -> Result<Option<AgentRun>, ChatServiceError> {
        Ok(self.active_run.lock().await.clone())
    }

    async fn is_available(&self) -> bool {
        *self.is_available.lock().await
    }

    async fn stop_agent(
        &self,
        _context_type: ChatContextType,
        _context_id: &str,
    ) -> Result<bool, ChatServiceError> {
        // Mock implementation - always returns false (no agent to stop)
        Ok(false)
    }

    async fn is_agent_running(
        &self,
        _context_type: ChatContextType,
        _context_id: &str,
    ) -> bool {
        // Mock implementation - always returns false
        false
    }
}
