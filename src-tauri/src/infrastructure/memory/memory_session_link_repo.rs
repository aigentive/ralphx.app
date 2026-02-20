// In-memory SessionLinkRepository implementation for testing
// Uses RwLock<HashMap> for thread-safe in-memory storage

use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;

use crate::domain::entities::{IdeationSessionId, SessionLink, SessionLinkId};
use crate::domain::repositories::SessionLinkRepository;
use crate::error::AppResult;

/// In-memory implementation of SessionLinkRepository for testing
pub struct MemorySessionLinkRepository {
    links: RwLock<HashMap<String, SessionLink>>,
}

impl MemorySessionLinkRepository {
    /// Create a new empty repository
    pub fn new() -> Self {
        Self {
            links: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemorySessionLinkRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SessionLinkRepository for MemorySessionLinkRepository {
    async fn create(&self, link: SessionLink) -> AppResult<SessionLink> {
        self.links
            .write()
            .unwrap()
            .insert(link.id.to_string(), link.clone());
        Ok(link)
    }

    async fn get_by_parent(&self, parent_id: &IdeationSessionId) -> AppResult<Vec<SessionLink>> {
        let mut links: Vec<_> = self
            .links
            .read()
            .unwrap()
            .values()
            .filter(|link| &link.parent_session_id == parent_id)
            .cloned()
            .collect();
        links.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(links)
    }

    async fn get_by_child(&self, child_id: &IdeationSessionId) -> AppResult<Vec<SessionLink>> {
        let mut links: Vec<_> = self
            .links
            .read()
            .unwrap()
            .values()
            .filter(|link| &link.child_session_id == child_id)
            .cloned()
            .collect();
        links.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(links)
    }

    async fn delete(&self, id: &SessionLinkId) -> AppResult<()> {
        self.links.write().unwrap().remove(&id.to_string());
        Ok(())
    }

    async fn delete_by_child(&self, child_id: &IdeationSessionId) -> AppResult<()> {
        let mut links = self.links.write().unwrap();
        links.retain(|_, link| &link.child_session_id != child_id);
        Ok(())
    }
}

#[cfg(test)]
#[path = "memory_session_link_repo_tests.rs"]
mod tests;
