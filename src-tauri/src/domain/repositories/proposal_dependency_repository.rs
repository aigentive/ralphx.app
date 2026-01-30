// Proposal dependency repository trait - domain layer abstraction
//
// This trait defines the contract for proposal dependency persistence.
// Dependencies track which proposals depend on other proposals within a session.

use async_trait::async_trait;

use crate::domain::entities::{IdeationSessionId, TaskProposalId};
use crate::error::AppResult;

/// Repository trait for proposal dependency persistence.
/// Implementations can use SQLite, PostgreSQL, in-memory, etc.
#[async_trait]
pub trait ProposalDependencyRepository: Send + Sync {
    /// Add a dependency (proposal_id depends on depends_on_id)
    async fn add_dependency(
        &self,
        proposal_id: &TaskProposalId,
        depends_on_id: &TaskProposalId,
    ) -> AppResult<()>;

    /// Remove a dependency
    async fn remove_dependency(
        &self,
        proposal_id: &TaskProposalId,
        depends_on_id: &TaskProposalId,
    ) -> AppResult<()>;

    /// Get all proposals that this proposal depends on
    async fn get_dependencies(
        &self,
        proposal_id: &TaskProposalId,
    ) -> AppResult<Vec<TaskProposalId>>;

    /// Get all proposals that depend on this proposal
    async fn get_dependents(
        &self,
        proposal_id: &TaskProposalId,
    ) -> AppResult<Vec<TaskProposalId>>;

    /// Get all dependency relationships for a session
    /// Returns tuples of (proposal_id, depends_on_proposal_id)
    async fn get_all_for_session(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<Vec<(TaskProposalId, TaskProposalId)>>;

    /// Check if adding a dependency would create a cycle
    async fn would_create_cycle(
        &self,
        proposal_id: &TaskProposalId,
        depends_on_id: &TaskProposalId,
    ) -> AppResult<bool>;

    /// Clear all dependencies for a proposal (both directions)
    async fn clear_dependencies(&self, proposal_id: &TaskProposalId) -> AppResult<()>;

    /// Clear all dependencies for all proposals in a session
    async fn clear_session_dependencies(&self, session_id: &IdeationSessionId) -> AppResult<()>;

    /// Count dependencies for a proposal (how many it depends on)
    async fn count_dependencies(&self, proposal_id: &TaskProposalId) -> AppResult<u32>;

    /// Count dependents for a proposal (how many depend on it)
    async fn count_dependents(&self, proposal_id: &TaskProposalId) -> AppResult<u32>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};
    use std::sync::Arc;

    // Mock implementation for testing trait object usage
    struct MockProposalDependencyRepository {
        // Map from proposal_id to set of depends_on_ids
        dependencies: HashMap<TaskProposalId, HashSet<TaskProposalId>>,
    }

    impl MockProposalDependencyRepository {
        fn new() -> Self {
            Self {
                dependencies: HashMap::new(),
            }
        }

        fn with_dependency(from: TaskProposalId, to: TaskProposalId) -> Self {
            let mut deps = HashMap::new();
            let mut set = HashSet::new();
            set.insert(to);
            deps.insert(from, set);
            Self { dependencies: deps }
        }

        fn with_dependencies(deps: Vec<(TaskProposalId, TaskProposalId)>) -> Self {
            let mut map: HashMap<TaskProposalId, HashSet<TaskProposalId>> = HashMap::new();
            for (from, to) in deps {
                map.entry(from).or_default().insert(to);
            }
            Self { dependencies: map }
        }
    }

    #[async_trait]
    impl ProposalDependencyRepository for MockProposalDependencyRepository {
        async fn add_dependency(
            &self,
            _proposal_id: &TaskProposalId,
            _depends_on_id: &TaskProposalId,
        ) -> AppResult<()> {
            Ok(())
        }

        async fn remove_dependency(
            &self,
            _proposal_id: &TaskProposalId,
            _depends_on_id: &TaskProposalId,
        ) -> AppResult<()> {
            Ok(())
        }

        async fn get_dependencies(
            &self,
            proposal_id: &TaskProposalId,
        ) -> AppResult<Vec<TaskProposalId>> {
            Ok(self
                .dependencies
                .get(proposal_id)
                .map(|set| set.iter().cloned().collect())
                .unwrap_or_default())
        }

        async fn get_dependents(
            &self,
            proposal_id: &TaskProposalId,
        ) -> AppResult<Vec<TaskProposalId>> {
            // Find all proposals that have this proposal in their dependency set
            Ok(self
                .dependencies
                .iter()
                .filter_map(|(id, deps)| {
                    if deps.contains(proposal_id) {
                        Some(id.clone())
                    } else {
                        None
                    }
                })
                .collect())
        }

        async fn get_all_for_session(
            &self,
            _session_id: &IdeationSessionId,
        ) -> AppResult<Vec<(TaskProposalId, TaskProposalId)>> {
            Ok(self
                .dependencies
                .iter()
                .flat_map(|(from, tos)| tos.iter().map(|to| (from.clone(), to.clone())))
                .collect())
        }

        async fn would_create_cycle(
            &self,
            proposal_id: &TaskProposalId,
            depends_on_id: &TaskProposalId,
        ) -> AppResult<bool> {
            // Simple check: if depends_on_id already depends on proposal_id, adding this would create a cycle
            if let Some(deps) = self.dependencies.get(depends_on_id) {
                if deps.contains(proposal_id) {
                    return Ok(true);
                }
            }
            // Also check for self-dependency
            if proposal_id == depends_on_id {
                return Ok(true);
            }
            Ok(false)
        }

        async fn clear_dependencies(&self, _proposal_id: &TaskProposalId) -> AppResult<()> {
            Ok(())
        }

        async fn clear_session_dependencies(&self, _session_id: &IdeationSessionId) -> AppResult<()> {
            Ok(())
        }

        async fn count_dependencies(&self, proposal_id: &TaskProposalId) -> AppResult<u32> {
            Ok(self
                .dependencies
                .get(proposal_id)
                .map(|set| set.len() as u32)
                .unwrap_or(0))
        }

        async fn count_dependents(&self, proposal_id: &TaskProposalId) -> AppResult<u32> {
            Ok(self
                .dependencies
                .iter()
                .filter(|(_, deps)| deps.contains(proposal_id))
                .count() as u32)
        }
    }

    #[test]
    fn test_proposal_dependency_repository_trait_can_be_object_safe() {
        // Verify that ProposalDependencyRepository can be used as a trait object
        let repo: Arc<dyn ProposalDependencyRepository> =
            Arc::new(MockProposalDependencyRepository::new());
        assert!(Arc::strong_count(&repo) == 1);
    }

    #[tokio::test]
    async fn test_mock_repository_add_dependency() {
        let repo = MockProposalDependencyRepository::new();
        let proposal_id = TaskProposalId::new();
        let depends_on_id = TaskProposalId::new();

        let result = repo.add_dependency(&proposal_id, &depends_on_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_repository_remove_dependency() {
        let repo = MockProposalDependencyRepository::new();
        let proposal_id = TaskProposalId::new();
        let depends_on_id = TaskProposalId::new();

        let result = repo.remove_dependency(&proposal_id, &depends_on_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_repository_get_dependencies_empty() {
        let repo = MockProposalDependencyRepository::new();
        let proposal_id = TaskProposalId::new();

        let result = repo.get_dependencies(&proposal_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_repository_get_dependencies_with_deps() {
        let proposal_id = TaskProposalId::new();
        let depends_on_id = TaskProposalId::new();
        let repo = MockProposalDependencyRepository::with_dependency(
            proposal_id.clone(),
            depends_on_id.clone(),
        );

        let result = repo.get_dependencies(&proposal_id).await;
        assert!(result.is_ok());
        let deps = result.unwrap();
        assert_eq!(deps.len(), 1);
        assert!(deps.contains(&depends_on_id));
    }

    #[tokio::test]
    async fn test_mock_repository_get_dependents_empty() {
        let repo = MockProposalDependencyRepository::new();
        let proposal_id = TaskProposalId::new();

        let result = repo.get_dependents(&proposal_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_repository_get_dependents_with_dependents() {
        let proposal_a = TaskProposalId::new();
        let proposal_b = TaskProposalId::new();
        let proposal_c = TaskProposalId::new();
        // A depends on C, B depends on C -> C has 2 dependents
        let repo = MockProposalDependencyRepository::with_dependencies(vec![
            (proposal_a.clone(), proposal_c.clone()),
            (proposal_b.clone(), proposal_c.clone()),
        ]);

        let result = repo.get_dependents(&proposal_c).await;
        assert!(result.is_ok());
        let dependents = result.unwrap();
        assert_eq!(dependents.len(), 2);
        assert!(dependents.contains(&proposal_a));
        assert!(dependents.contains(&proposal_b));
    }

    #[tokio::test]
    async fn test_mock_repository_get_all_for_session_empty() {
        let repo = MockProposalDependencyRepository::new();
        let session_id = IdeationSessionId::new();

        let result = repo.get_all_for_session(&session_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_repository_get_all_for_session_with_deps() {
        let proposal_a = TaskProposalId::new();
        let proposal_b = TaskProposalId::new();
        let proposal_c = TaskProposalId::new();
        let repo = MockProposalDependencyRepository::with_dependencies(vec![
            (proposal_a.clone(), proposal_b.clone()),
            (proposal_b.clone(), proposal_c.clone()),
        ]);

        let session_id = IdeationSessionId::new();
        let result = repo.get_all_for_session(&session_id).await;
        assert!(result.is_ok());
        let all = result.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_repository_would_create_cycle_false() {
        let repo = MockProposalDependencyRepository::new();
        let proposal_a = TaskProposalId::new();
        let proposal_b = TaskProposalId::new();

        // No existing deps, so no cycle
        let result = repo.would_create_cycle(&proposal_a, &proposal_b).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_mock_repository_would_create_cycle_self_dependency() {
        let repo = MockProposalDependencyRepository::new();
        let proposal_a = TaskProposalId::new();

        // Self-dependency always creates a cycle
        let result = repo.would_create_cycle(&proposal_a, &proposal_a).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_mock_repository_would_create_cycle_direct() {
        let proposal_a = TaskProposalId::new();
        let proposal_b = TaskProposalId::new();
        // B depends on A
        let repo = MockProposalDependencyRepository::with_dependency(
            proposal_b.clone(),
            proposal_a.clone(),
        );

        // Adding A depends on B would create A -> B -> A cycle
        let result = repo.would_create_cycle(&proposal_a, &proposal_b).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_mock_repository_clear_dependencies() {
        let repo = MockProposalDependencyRepository::new();
        let proposal_id = TaskProposalId::new();

        let result = repo.clear_dependencies(&proposal_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_repository_count_dependencies_zero() {
        let repo = MockProposalDependencyRepository::new();
        let proposal_id = TaskProposalId::new();

        let result = repo.count_dependencies(&proposal_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_mock_repository_count_dependencies_multiple() {
        let proposal_a = TaskProposalId::new();
        let proposal_b = TaskProposalId::new();
        let proposal_c = TaskProposalId::new();
        // A depends on B and C
        let repo = MockProposalDependencyRepository::with_dependencies(vec![
            (proposal_a.clone(), proposal_b.clone()),
            (proposal_a.clone(), proposal_c.clone()),
        ]);

        let result = repo.count_dependencies(&proposal_a).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_mock_repository_count_dependents_zero() {
        let repo = MockProposalDependencyRepository::new();
        let proposal_id = TaskProposalId::new();

        let result = repo.count_dependents(&proposal_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_mock_repository_count_dependents_multiple() {
        let proposal_a = TaskProposalId::new();
        let proposal_b = TaskProposalId::new();
        let proposal_c = TaskProposalId::new();
        // B and C both depend on A
        let repo = MockProposalDependencyRepository::with_dependencies(vec![
            (proposal_b.clone(), proposal_a.clone()),
            (proposal_c.clone(), proposal_a.clone()),
        ]);

        let result = repo.count_dependents(&proposal_a).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_repository_trait_object_in_arc() {
        let proposal_a = TaskProposalId::new();
        let proposal_b = TaskProposalId::new();
        let repo: Arc<dyn ProposalDependencyRepository> = Arc::new(
            MockProposalDependencyRepository::with_dependency(proposal_a.clone(), proposal_b.clone()),
        );

        // Use through trait object
        let deps = repo.get_dependencies(&proposal_a).await;
        assert!(deps.is_ok());
        assert_eq!(deps.unwrap().len(), 1);

        let dependents = repo.get_dependents(&proposal_b).await;
        assert!(dependents.is_ok());
        assert_eq!(dependents.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_repository_trait_object_cycle_detection() {
        let proposal_a = TaskProposalId::new();
        let proposal_b = TaskProposalId::new();
        let repo: Arc<dyn ProposalDependencyRepository> = Arc::new(
            MockProposalDependencyRepository::with_dependency(proposal_b.clone(), proposal_a.clone()),
        );

        let would_cycle = repo.would_create_cycle(&proposal_a, &proposal_b).await;
        assert!(would_cycle.is_ok());
        assert!(would_cycle.unwrap());
    }

    #[tokio::test]
    async fn test_repository_trait_object_count_operations() {
        let proposal_a = TaskProposalId::new();
        let proposal_b = TaskProposalId::new();
        let repo: Arc<dyn ProposalDependencyRepository> = Arc::new(
            MockProposalDependencyRepository::with_dependency(proposal_a.clone(), proposal_b.clone()),
        );

        let dep_count = repo.count_dependencies(&proposal_a).await;
        assert!(dep_count.is_ok());
        assert_eq!(dep_count.unwrap(), 1);

        let dependent_count = repo.count_dependents(&proposal_b).await;
        assert!(dependent_count.is_ok());
        assert_eq!(dependent_count.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_repository_trait_object_add_remove_clear() {
        let repo: Arc<dyn ProposalDependencyRepository> =
            Arc::new(MockProposalDependencyRepository::new());
        let proposal_a = TaskProposalId::new();
        let proposal_b = TaskProposalId::new();

        let add_result = repo.add_dependency(&proposal_a, &proposal_b).await;
        assert!(add_result.is_ok());

        let remove_result = repo.remove_dependency(&proposal_a, &proposal_b).await;
        assert!(remove_result.is_ok());

        let clear_result = repo.clear_dependencies(&proposal_a).await;
        assert!(clear_result.is_ok());
    }
}
