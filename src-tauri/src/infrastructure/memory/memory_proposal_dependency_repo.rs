// In-memory ProposalDependencyRepository implementation for testing
// Uses RwLock<Vec> for thread-safe in-memory storage

use std::sync::RwLock;

use async_trait::async_trait;

use crate::domain::entities::{IdeationSessionId, TaskProposalId};
use crate::domain::repositories::ProposalDependencyRepository;
use crate::error::AppResult;

/// In-memory implementation of ProposalDependencyRepository for testing
pub struct MemoryProposalDependencyRepository {
    // (proposal_id, depends_on_id, session_id, source)
    dependencies: RwLock<Vec<(String, String, String, String)>>,
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
        source: &str,
    ) {
        self.dependencies.write().unwrap().push((
            proposal_id.to_string(),
            depends_on_id.to_string(),
            session_id.to_string(),
            source.to_string(),
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
        _reason: Option<&str>,
        source: Option<&str>,
    ) -> AppResult<()> {
        // Without session context, we use empty session_id
        // TODO: Store reason when needed for tests
        let source = source.unwrap_or("auto");
        self.dependencies.write().unwrap().push((
            proposal_id.to_string(),
            depends_on_id.to_string(),
            String::new(),
            source.to_string(),
        ));
        Ok(())
    }

    async fn remove_dependency(
        &self,
        proposal_id: &TaskProposalId,
        depends_on_id: &TaskProposalId,
    ) -> AppResult<()> {
        self.dependencies.write().unwrap().retain(|(p, d, _, _)| {
            p != &proposal_id.to_string() || d != &depends_on_id.to_string()
        });
        Ok(())
    }

    async fn get_dependencies(
        &self,
        proposal_id: &TaskProposalId,
    ) -> AppResult<Vec<TaskProposalId>> {
        Ok(self
            .dependencies
            .read()
            .unwrap()
            .iter()
            .filter(|(p, _, _, _)| p == &proposal_id.to_string())
            .map(|(_, d, _, _)| TaskProposalId::from_string(d.clone()))
            .collect())
    }

    async fn get_dependents(&self, proposal_id: &TaskProposalId) -> AppResult<Vec<TaskProposalId>> {
        Ok(self
            .dependencies
            .read()
            .unwrap()
            .iter()
            .filter(|(_, d, _, _)| d == &proposal_id.to_string())
            .map(|(p, _, _, _)| TaskProposalId::from_string(p.clone()))
            .collect())
    }

    async fn get_all_for_session(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<Vec<(TaskProposalId, TaskProposalId, Option<String>)>> {
        Ok(self
            .dependencies
            .read()
            .unwrap()
            .iter()
            .filter(|(_, _, s, _)| s == &session_id.to_string() || s.is_empty())
            .map(|(p, d, _, _)| {
                (
                    TaskProposalId::from_string(p.clone()),
                    TaskProposalId::from_string(d.clone()),
                    None, // TODO: Store and return reason when needed for tests
                )
            })
            .collect())
    }

    async fn get_all_for_session_with_source(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<Vec<(TaskProposalId, TaskProposalId, Option<String>, String)>> {
        Ok(self
            .dependencies
            .read()
            .unwrap()
            .iter()
            .filter(|(_, _, s, _)| s == &session_id.to_string() || s.is_empty())
            .map(|(p, d, _, source)| {
                (
                    TaskProposalId::from_string(p.clone()),
                    TaskProposalId::from_string(d.clone()),
                    None, // TODO: Store and return reason when needed for tests
                    source.clone(),
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
        self.dependencies
            .write()
            .unwrap()
            .retain(|(p, d, _, _)| p != &proposal_id.to_string() && d != &proposal_id.to_string());
        Ok(())
    }

    async fn clear_session_dependencies(&self, session_id: &IdeationSessionId) -> AppResult<()> {
        self.dependencies
            .write()
            .unwrap()
            .retain(|(_, _, s, _)| s != &session_id.to_string());
        Ok(())
    }

    async fn clear_auto_dependencies(&self, session_id: &IdeationSessionId) -> AppResult<()> {
        self.dependencies
            .write()
            .unwrap()
            .retain(|(_, _, s, source)| s != &session_id.to_string() || source != "auto");
        Ok(())
    }

    async fn count_dependencies(&self, proposal_id: &TaskProposalId) -> AppResult<u32> {
        Ok(self
            .dependencies
            .read()
            .unwrap()
            .iter()
            .filter(|(p, _, _, _)| p == &proposal_id.to_string())
            .count() as u32)
    }

    async fn count_dependents(&self, proposal_id: &TaskProposalId) -> AppResult<u32> {
        Ok(self
            .dependencies
            .read()
            .unwrap()
            .iter()
            .filter(|(_, d, _, _)| d == &proposal_id.to_string())
            .count() as u32)
    }
}

#[cfg(test)]
#[path = "memory_proposal_dependency_repo_tests.rs"]
mod tests;
