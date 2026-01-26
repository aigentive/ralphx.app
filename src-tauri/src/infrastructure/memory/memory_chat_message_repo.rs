// In-memory ChatMessageRepository implementation for testing
// Uses RwLock<HashMap> for thread-safe in-memory storage

use std::cmp::Reverse;
use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;

use crate::domain::entities::{ChatMessage, ChatMessageId, ChatConversationId, IdeationSessionId, ProjectId, TaskId};
use crate::domain::repositories::ChatMessageRepository;
use crate::error::AppResult;

/// In-memory implementation of ChatMessageRepository for testing
pub struct MemoryChatMessageRepository {
    messages: RwLock<HashMap<String, ChatMessage>>,
}

impl MemoryChatMessageRepository {
    /// Create a new empty repository
    pub fn new() -> Self {
        Self {
            messages: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryChatMessageRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ChatMessageRepository for MemoryChatMessageRepository {
    async fn create(&self, message: ChatMessage) -> AppResult<ChatMessage> {
        self.messages
            .write()
            .unwrap()
            .insert(message.id.to_string(), message.clone());
        Ok(message)
    }

    async fn get_by_id(&self, id: &ChatMessageId) -> AppResult<Option<ChatMessage>> {
        Ok(self.messages.read().unwrap().get(&id.to_string()).cloned())
    }

    async fn get_by_session(&self, session_id: &IdeationSessionId) -> AppResult<Vec<ChatMessage>> {
        let mut messages: Vec<_> = self
            .messages
            .read()
            .unwrap()
            .values()
            .filter(|m| m.session_id.as_ref() == Some(session_id))
            .cloned()
            .collect();
        messages.sort_by_key(|m| m.created_at);
        Ok(messages)
    }

    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<ChatMessage>> {
        let mut messages: Vec<_> = self
            .messages
            .read()
            .unwrap()
            .values()
            .filter(|m| m.project_id.as_ref() == Some(project_id) && m.session_id.is_none())
            .cloned()
            .collect();
        messages.sort_by_key(|m| m.created_at);
        Ok(messages)
    }

    async fn get_by_task(&self, task_id: &TaskId) -> AppResult<Vec<ChatMessage>> {
        let mut messages: Vec<_> = self
            .messages
            .read()
            .unwrap()
            .values()
            .filter(|m| m.task_id.as_ref() == Some(task_id))
            .cloned()
            .collect();
        messages.sort_by_key(|m| m.created_at);
        Ok(messages)
    }

    async fn get_by_conversation(&self, conversation_id: &ChatConversationId) -> AppResult<Vec<ChatMessage>> {
        let mut messages: Vec<_> = self
            .messages
            .read()
            .unwrap()
            .values()
            .filter(|m| m.conversation_id.as_ref() == Some(conversation_id))
            .cloned()
            .collect();
        messages.sort_by_key(|m| m.created_at);
        Ok(messages)
    }

    async fn delete_by_session(&self, session_id: &IdeationSessionId) -> AppResult<()> {
        self.messages
            .write()
            .unwrap()
            .retain(|_, m| m.session_id.as_ref() != Some(session_id));
        Ok(())
    }

    async fn delete_by_project(&self, project_id: &ProjectId) -> AppResult<()> {
        self.messages
            .write()
            .unwrap()
            .retain(|_, m| m.project_id.as_ref() != Some(project_id));
        Ok(())
    }

    async fn delete_by_task(&self, task_id: &TaskId) -> AppResult<()> {
        self.messages
            .write()
            .unwrap()
            .retain(|_, m| m.task_id.as_ref() != Some(task_id));
        Ok(())
    }

    async fn delete(&self, id: &ChatMessageId) -> AppResult<()> {
        self.messages.write().unwrap().remove(&id.to_string());
        Ok(())
    }

    async fn count_by_session(&self, session_id: &IdeationSessionId) -> AppResult<u32> {
        Ok(self
            .messages
            .read()
            .unwrap()
            .values()
            .filter(|m| m.session_id.as_ref() == Some(session_id))
            .count() as u32)
    }

    async fn get_recent_by_session(
        &self,
        session_id: &IdeationSessionId,
        limit: u32,
    ) -> AppResult<Vec<ChatMessage>> {
        let mut messages: Vec<_> = self
            .messages
            .read()
            .unwrap()
            .values()
            .filter(|m| m.session_id.as_ref() == Some(session_id))
            .cloned()
            .collect();
        messages.sort_by_key(|m| Reverse(m.created_at));
        messages.truncate(limit as usize);
        messages.reverse();
        Ok(messages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_and_get() {
        let repo = MemoryChatMessageRepository::new();
        let session_id = IdeationSessionId::new();
        let message = ChatMessage::user_in_session(session_id.clone(), "Hello");

        repo.create(message.clone()).await.unwrap();

        let retrieved = repo.get_by_id(&message.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, message.id);
    }

    #[tokio::test]
    async fn test_get_by_session() {
        let repo = MemoryChatMessageRepository::new();
        let session_id = IdeationSessionId::new();
        let message = ChatMessage::user_in_session(session_id.clone(), "Hello");

        repo.create(message).await.unwrap();

        let messages = repo.get_by_session(&session_id).await.unwrap();
        assert_eq!(messages.len(), 1);
    }

    #[tokio::test]
    async fn test_delete() {
        let repo = MemoryChatMessageRepository::new();
        let session_id = IdeationSessionId::new();
        let message = ChatMessage::user_in_session(session_id.clone(), "Hello");
        let message_id = message.id.clone();

        repo.create(message).await.unwrap();
        repo.delete(&message_id).await.unwrap();

        let result = repo.get_by_id(&message_id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete_by_session() {
        let repo = MemoryChatMessageRepository::new();
        let session_id = IdeationSessionId::new();

        repo.create(ChatMessage::user_in_session(session_id.clone(), "Hello 1"))
            .await
            .unwrap();
        repo.create(ChatMessage::user_in_session(session_id.clone(), "Hello 2"))
            .await
            .unwrap();

        repo.delete_by_session(&session_id).await.unwrap();

        let messages = repo.get_by_session(&session_id).await.unwrap();
        assert!(messages.is_empty());
    }

    #[tokio::test]
    async fn test_get_recent_by_session() {
        let repo = MemoryChatMessageRepository::new();
        let session_id = IdeationSessionId::new();

        for i in 1..=5 {
            repo.create(ChatMessage::user_in_session(
                session_id.clone(),
                format!("Message {}", i),
            ))
            .await
            .unwrap();
        }

        let recent = repo.get_recent_by_session(&session_id, 3).await.unwrap();
        assert_eq!(recent.len(), 3);
    }
}
