// In-memory TeamMessageRepository implementation for testing

use std::cmp::Reverse;
use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;

use crate::domain::entities::team::{TeamMessageId, TeamMessageRecord, TeamSessionId};
use crate::domain::repositories::TeamMessageRepository;
use crate::error::AppResult;

pub struct MemoryTeamMessageRepository {
    messages: RwLock<HashMap<String, TeamMessageRecord>>,
}

impl MemoryTeamMessageRepository {
    pub fn new() -> Self {
        Self {
            messages: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryTeamMessageRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TeamMessageRepository for MemoryTeamMessageRepository {
    async fn create(&self, message: TeamMessageRecord) -> AppResult<TeamMessageRecord> {
        self.messages
            .write()
            .unwrap()
            .insert(message.id.as_str().to_string(), message.clone());
        Ok(message)
    }

    async fn get_by_session(
        &self,
        session_id: &TeamSessionId,
    ) -> AppResult<Vec<TeamMessageRecord>> {
        let mut messages: Vec<_> = self
            .messages
            .read()
            .unwrap()
            .values()
            .filter(|m| m.team_session_id == *session_id)
            .cloned()
            .collect();
        messages.sort_by_key(|m| m.created_at);
        Ok(messages)
    }

    async fn get_recent_by_session(
        &self,
        session_id: &TeamSessionId,
        limit: u32,
    ) -> AppResult<Vec<TeamMessageRecord>> {
        let mut messages: Vec<_> = self
            .messages
            .read()
            .unwrap()
            .values()
            .filter(|m| m.team_session_id == *session_id)
            .cloned()
            .collect();
        messages.sort_by_key(|m| Reverse(m.created_at));
        messages.truncate(limit as usize);
        messages.reverse();
        Ok(messages)
    }

    async fn count_by_session(&self, session_id: &TeamSessionId) -> AppResult<u32> {
        Ok(self
            .messages
            .read()
            .unwrap()
            .values()
            .filter(|m| m.team_session_id == *session_id)
            .count() as u32)
    }

    async fn delete_by_session(&self, session_id: &TeamSessionId) -> AppResult<()> {
        self.messages
            .write()
            .unwrap()
            .retain(|_, m| m.team_session_id != *session_id);
        Ok(())
    }

    async fn delete(&self, id: &TeamMessageId) -> AppResult<()> {
        self.messages
            .write()
            .unwrap()
            .remove(id.as_str());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::team::TeamMessageRecord;

    #[tokio::test]
    async fn test_create_and_get_by_session() {
        let repo = MemoryTeamMessageRepository::new();
        let session_id = TeamSessionId::new();
        let msg = TeamMessageRecord::new(session_id.clone(), "worker", "hello");

        repo.create(msg).await.unwrap();

        let messages = repo.get_by_session(&session_id).await.unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "hello");
    }

    #[tokio::test]
    async fn test_count_by_session() {
        let repo = MemoryTeamMessageRepository::new();
        let session_id = TeamSessionId::new();

        repo.create(TeamMessageRecord::new(session_id.clone(), "a", "msg1"))
            .await
            .unwrap();
        repo.create(TeamMessageRecord::new(session_id.clone(), "b", "msg2"))
            .await
            .unwrap();

        assert_eq!(repo.count_by_session(&session_id).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_delete_by_session() {
        let repo = MemoryTeamMessageRepository::new();
        let session_id = TeamSessionId::new();

        repo.create(TeamMessageRecord::new(session_id.clone(), "a", "msg1"))
            .await
            .unwrap();
        repo.delete_by_session(&session_id).await.unwrap();

        assert_eq!(repo.count_by_session(&session_id).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_delete_single() {
        let repo = MemoryTeamMessageRepository::new();
        let session_id = TeamSessionId::new();
        let msg = TeamMessageRecord::new(session_id.clone(), "a", "msg1");
        let msg_id = msg.id.clone();

        repo.create(msg).await.unwrap();
        repo.delete(&msg_id).await.unwrap();

        assert_eq!(repo.count_by_session(&session_id).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_get_recent_by_session() {
        let repo = MemoryTeamMessageRepository::new();
        let session_id = TeamSessionId::new();

        for i in 1..=5 {
            repo.create(TeamMessageRecord::new(
                session_id.clone(),
                "sender",
                format!("msg {}", i),
            ))
            .await
            .unwrap();
        }

        let recent = repo.get_recent_by_session(&session_id, 3).await.unwrap();
        assert_eq!(recent.len(), 3);
    }
}
