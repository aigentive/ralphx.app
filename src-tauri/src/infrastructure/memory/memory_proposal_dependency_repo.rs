// In-memory ProposalDependencyRepository implementation for testing
// Uses RwLock<Vec> for thread-safe in-memory storage

use std::sync::RwLock;

use async_trait::async_trait;

use crate::domain::entities::{IdeationSessionId, TaskProposalId};
use crate::domain::repositories::ProposalDependencyRepository;
use crate::error::AppResult;

/// In-memory implementation of ProposalDependencyRepository for testing
pub struct MemoryProposalDependencyRepository {
    // (proposal_id, depends_on_id, session_id)
    dependencies: RwLock<Vec<(String, String, String)>>,
}

impl MemoryProposalDependencyRepository {
    /// Create a new empty repository
    pub fn new() -> Self {
        Self {
            dependencies: RwLock::new(Vec::new()),
        }
    }

    /// Add a dependency with session context (for testing)
    pub fn add_with_session(
        &self,
        proposal_id: &TaskProposalId,
        depends_on_id: &TaskProposalId,
        session_id: &IdeationSessionId,
    ) {
        self.dependencies.write().unwrap().push((
            proposal_id.to_string(),
            depends_on_id.to_string(),
            session_id.to_string(),
        ));
    }
}

impl Default for MemoryProposalDependencyRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProposalDependencyRepository for MemoryProposalDependencyRepository {
    async fn add_dependency(
        &self,
        proposal_id: &TaskProposalId,
        depends_on_id: &TaskProposalId,
    ) -> AppResult<()> {
        // Without session context, we use empty session_id
        self.dependencies.write().unwrap().push((
            proposal_id.to_string(),
            depends_on_id.to_string(),
            String::new(),
        ));
        Ok(())
    }

    async fn remove_dependency(
        &self,
        proposal_id: &TaskProposalId,
        depends_on_id: &TaskProposalId,
    ) -> AppResult<()> {
        self.dependencies.write().unwrap().retain(|(p, d, _)| {
            p != &proposal_id.to_string() || d != &depends_on_id.to_string()
        });
        Ok(())
    }

    async fn get_dependencies(&self, proposal_id: &TaskProposalId) -> AppResult<Vec<TaskProposalId>> {
        Ok(self
            .dependencies
            .read()
            .unwrap()
            .iter()
            .filter(|(p, _, _)| p == &proposal_id.to_string())
            .map(|(_, d, _)| TaskProposalId::from_string(d.clone()))
            .collect())
    }

    async fn get_dependents(&self, proposal_id: &TaskProposalId) -> AppResult<Vec<TaskProposalId>> {
        Ok(self
            .dependencies
            .read()
            .unwrap()
            .iter()
            .filter(|(_, d, _)| d == &proposal_id.to_string())
            .map(|(p, _, _)| TaskProposalId::from_string(p.clone()))
            .collect())
    }

    async fn get_all_for_session(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<Vec<(TaskProposalId, TaskProposalId)>> {
        Ok(self
            .dependencies
            .read()
            .unwrap()
            .iter()
            .filter(|(_, _, s)| s == &session_id.to_string() || s.is_empty())
            .map(|(p, d, _)| {
                (
                    TaskProposalId::from_string(p.clone()),
                    TaskProposalId::from_string(d.clone()),
                )
            })
            .collect())
    }

    async fn would_create_cycle(
        &self,
        _proposal_id: &TaskProposalId,
        _depends_on_id: &TaskProposalId,
    ) -> AppResult<bool> {
        // Simple implementation for testing - always returns false
        Ok(false)
    }

    async fn clear_dependencies(&self, proposal_id: &TaskProposalId) -> AppResult<()> {
        self.dependencies.write().unwrap().retain(|(p, d, _)| {
            p != &proposal_id.to_string() && d != &proposal_id.to_string()
        });
        Ok(())
    }

    async fn count_dependencies(&self, proposal_id: &TaskProposalId) -> AppResult<u32> {
        Ok(self
            .dependencies
            .read()
            .unwrap()
            .iter()
            .filter(|(p, _, _)| p == &proposal_id.to_string())
            .count() as u32)
    }

    async fn count_dependents(&self, proposal_id: &TaskProposalId) -> AppResult<u32> {
        Ok(self
            .dependencies
            .read()
            .unwrap()
            .iter()
            .filter(|(_, d, _)| d == &proposal_id.to_string())
            .count() as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_and_get_dependencies() {
        let repo = MemoryProposalDependencyRepository::new();
        let p1 = TaskProposalId::new();
        let p2 = TaskProposalId::new();

        repo.add_dependency(&p1, &p2).await.unwrap();

        let deps = repo.get_dependencies(&p1).await.unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].to_string(), p2.to_string());
    }

    #[tokio::test]
    async fn test_get_dependents() {
        let repo = MemoryProposalDependencyRepository::new();
        let p1 = TaskProposalId::new();
        let p2 = TaskProposalId::new();

        repo.add_dependency(&p1, &p2).await.unwrap();

        let dependents = repo.get_dependents(&p2).await.unwrap();
        assert_eq!(dependents.len(), 1);
        assert_eq!(dependents[0].to_string(), p1.to_string());
    }

    #[tokio::test]
    async fn test_remove_dependency() {
        let repo = MemoryProposalDependencyRepository::new();
        let p1 = TaskProposalId::new();
        let p2 = TaskProposalId::new();

        repo.add_dependency(&p1, &p2).await.unwrap();
        repo.remove_dependency(&p1, &p2).await.unwrap();

        let deps = repo.get_dependencies(&p1).await.unwrap();
        assert!(deps.is_empty());
    }

    #[tokio::test]
    async fn test_clear_dependencies() {
        let repo = MemoryProposalDependencyRepository::new();
        let p1 = TaskProposalId::new();
        let p2 = TaskProposalId::new();
        let p3 = TaskProposalId::new();

        repo.add_dependency(&p1, &p2).await.unwrap();
        repo.add_dependency(&p3, &p1).await.unwrap();

        repo.clear_dependencies(&p1).await.unwrap();

        let deps = repo.get_dependencies(&p1).await.unwrap();
        let dependents = repo.get_dependents(&p1).await.unwrap();

        assert!(deps.is_empty());
        assert!(dependents.is_empty());
    }

    #[tokio::test]
    async fn test_get_all_for_session() {
        let repo = MemoryProposalDependencyRepository::new();
        let session_id = IdeationSessionId::new();
        let p1 = TaskProposalId::new();
        let p2 = TaskProposalId::new();

        repo.add_with_session(&p1, &p2, &session_id);

        let all = repo.get_all_for_session(&session_id).await.unwrap();
        assert_eq!(all.len(), 1);
    }
}
