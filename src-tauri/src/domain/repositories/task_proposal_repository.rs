// Task proposal repository trait - domain layer abstraction
//
// This trait defines the contract for task proposal persistence.
// Implementations can use SQLite, PostgreSQL, in-memory, etc.

use async_trait::async_trait;

use crate::domain::entities::{
    IdeationSessionId, PriorityAssessment, TaskId, TaskProposal, TaskProposalId,
};
use crate::error::AppResult;

/// Repository trait for TaskProposal persistence.
/// Implementations can use SQLite, PostgreSQL, in-memory, etc.
#[async_trait]
pub trait TaskProposalRepository: Send + Sync {
    /// Create a new task proposal
    async fn create(&self, proposal: TaskProposal) -> AppResult<TaskProposal>;

    /// Get proposal by ID
    async fn get_by_id(&self, id: &TaskProposalId) -> AppResult<Option<TaskProposal>>;

    /// Get all proposals for a session, ordered by sort_order
    async fn get_by_session(&self, session_id: &IdeationSessionId) -> AppResult<Vec<TaskProposal>>;

    /// Update an existing proposal
    async fn update(&self, proposal: &TaskProposal) -> AppResult<()>;

    /// Update priority assessment for a proposal
    async fn update_priority(
        &self,
        id: &TaskProposalId,
        assessment: &PriorityAssessment,
    ) -> AppResult<()>;

    /// Update selection state for a proposal
    async fn update_selection(&self, id: &TaskProposalId, selected: bool) -> AppResult<()>;

    /// Set the created task ID after converting proposal to task
    async fn set_created_task_id(
        &self,
        id: &TaskProposalId,
        task_id: &TaskId,
    ) -> AppResult<()>;

    /// Delete a proposal
    async fn delete(&self, id: &TaskProposalId) -> AppResult<()>;

    /// Reorder proposals within a session
    /// Updates sort_order for each proposal based on position in the provided list
    async fn reorder(
        &self,
        session_id: &IdeationSessionId,
        proposal_ids: Vec<TaskProposalId>,
    ) -> AppResult<()>;

    /// Get selected proposals for a session
    async fn get_selected_by_session(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<Vec<TaskProposal>>;

    /// Count proposals by session
    async fn count_by_session(&self, session_id: &IdeationSessionId) -> AppResult<u32>;

    /// Count selected proposals by session
    async fn count_selected_by_session(&self, session_id: &IdeationSessionId) -> AppResult<u32>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{
        BusinessValueFactor, Complexity, ComplexityFactor, CriticalPathFactor, DependencyFactor,
        Priority, PriorityAssessmentFactors, TaskCategory, UserHintFactor,
    };
    use std::sync::Arc;

    // Mock implementation for testing trait object usage
    struct MockTaskProposalRepository {
        return_proposal: Option<TaskProposal>,
        proposals: Vec<TaskProposal>,
    }

    impl MockTaskProposalRepository {
        fn new() -> Self {
            Self {
                return_proposal: None,
                proposals: vec![],
            }
        }

        fn with_proposal(proposal: TaskProposal) -> Self {
            Self {
                return_proposal: Some(proposal.clone()),
                proposals: vec![proposal],
            }
        }

        fn with_proposals(proposals: Vec<TaskProposal>) -> Self {
            Self {
                return_proposal: proposals.first().cloned(),
                proposals,
            }
        }
    }

    #[async_trait]
    impl TaskProposalRepository for MockTaskProposalRepository {
        async fn create(&self, proposal: TaskProposal) -> AppResult<TaskProposal> {
            Ok(proposal)
        }

        async fn get_by_id(&self, _id: &TaskProposalId) -> AppResult<Option<TaskProposal>> {
            Ok(self.return_proposal.clone())
        }

        async fn get_by_session(
            &self,
            session_id: &IdeationSessionId,
        ) -> AppResult<Vec<TaskProposal>> {
            let mut filtered: Vec<_> = self
                .proposals
                .iter()
                .filter(|p| &p.session_id == session_id)
                .cloned()
                .collect();
            filtered.sort_by_key(|p| p.sort_order);
            Ok(filtered)
        }

        async fn update(&self, _proposal: &TaskProposal) -> AppResult<()> {
            Ok(())
        }

        async fn update_priority(
            &self,
            _id: &TaskProposalId,
            _assessment: &PriorityAssessment,
        ) -> AppResult<()> {
            Ok(())
        }

        async fn update_selection(&self, _id: &TaskProposalId, _selected: bool) -> AppResult<()> {
            Ok(())
        }

        async fn set_created_task_id(
            &self,
            _id: &TaskProposalId,
            _task_id: &TaskId,
        ) -> AppResult<()> {
            Ok(())
        }

        async fn delete(&self, _id: &TaskProposalId) -> AppResult<()> {
            Ok(())
        }

        async fn reorder(
            &self,
            _session_id: &IdeationSessionId,
            _proposal_ids: Vec<TaskProposalId>,
        ) -> AppResult<()> {
            Ok(())
        }

        async fn get_selected_by_session(
            &self,
            session_id: &IdeationSessionId,
        ) -> AppResult<Vec<TaskProposal>> {
            let mut filtered: Vec<_> = self
                .proposals
                .iter()
                .filter(|p| &p.session_id == session_id && p.selected)
                .cloned()
                .collect();
            filtered.sort_by_key(|p| p.sort_order);
            Ok(filtered)
        }

        async fn count_by_session(&self, session_id: &IdeationSessionId) -> AppResult<u32> {
            Ok(self
                .proposals
                .iter()
                .filter(|p| &p.session_id == session_id)
                .count() as u32)
        }

        async fn count_selected_by_session(
            &self,
            session_id: &IdeationSessionId,
        ) -> AppResult<u32> {
            Ok(self
                .proposals
                .iter()
                .filter(|p| &p.session_id == session_id && p.selected)
                .count() as u32)
        }
    }

    fn create_test_proposal(session_id: &IdeationSessionId) -> TaskProposal {
        TaskProposal::new(
            session_id.clone(),
            "Test Proposal",
            TaskCategory::Feature,
            Priority::Medium,
        )
    }

    fn create_test_assessment(proposal_id: &TaskProposalId) -> PriorityAssessment {
        let factors = PriorityAssessmentFactors {
            dependency_factor: DependencyFactor {
                score: 15,
                blocks_count: 2,
                reason: "Blocks 2 tasks".to_string(),
            },
            critical_path_factor: CriticalPathFactor {
                score: 20,
                is_on_critical_path: true,
                path_length: 3,
                reason: "On critical path".to_string(),
            },
            business_value_factor: BusinessValueFactor {
                score: 15,
                keywords: vec!["core".to_string()],
                reason: "Core functionality".to_string(),
            },
            complexity_factor: ComplexityFactor {
                score: 10,
                complexity: Complexity::Simple,
                reason: "Simple task".to_string(),
            },
            user_hint_factor: UserHintFactor {
                score: 5,
                hints: vec!["important".to_string()],
                reason: "User marked important".to_string(),
            },
        };
        PriorityAssessment::new(proposal_id.clone(), factors)
    }

    #[test]
    fn test_task_proposal_repository_trait_can_be_object_safe() {
        // Verify that TaskProposalRepository can be used as a trait object
        let repo: Arc<dyn TaskProposalRepository> =
            Arc::new(MockTaskProposalRepository::new());
        assert!(Arc::strong_count(&repo) == 1);
    }

    #[tokio::test]
    async fn test_mock_repository_create() {
        let repo = MockTaskProposalRepository::new();
        let session_id = IdeationSessionId::new();
        let proposal = create_test_proposal(&session_id);

        let result = repo.create(proposal.clone()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, proposal.id);
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_id_returns_none() {
        let repo = MockTaskProposalRepository::new();
        let proposal_id = TaskProposalId::new();

        let result = repo.get_by_id(&proposal_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_id_returns_proposal() {
        let session_id = IdeationSessionId::new();
        let proposal = create_test_proposal(&session_id);
        let repo = MockTaskProposalRepository::with_proposal(proposal.clone());

        let result = repo.get_by_id(&proposal.id).await;
        assert!(result.is_ok());
        let returned = result.unwrap();
        assert!(returned.is_some());
        assert_eq!(returned.unwrap().id, proposal.id);
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_session_empty() {
        let repo = MockTaskProposalRepository::new();
        let session_id = IdeationSessionId::new();

        let result = repo.get_by_session(&session_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_session_with_proposals() {
        let session_id = IdeationSessionId::new();
        let proposal1 = create_test_proposal(&session_id);
        let proposal2 = create_test_proposal(&session_id);

        let repo = MockTaskProposalRepository::with_proposals(vec![
            proposal1.clone(),
            proposal2.clone(),
        ]);

        let result = repo.get_by_session(&session_id).await;
        assert!(result.is_ok());
        let proposals = result.unwrap();
        assert_eq!(proposals.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_session_filters_by_session() {
        let session_id1 = IdeationSessionId::new();
        let session_id2 = IdeationSessionId::new();
        let proposal1 = create_test_proposal(&session_id1);
        let proposal2 = create_test_proposal(&session_id2);

        let repo = MockTaskProposalRepository::with_proposals(vec![
            proposal1.clone(),
            proposal2.clone(),
        ]);

        let result = repo.get_by_session(&session_id1).await;
        assert!(result.is_ok());
        let proposals = result.unwrap();
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].session_id, session_id1);
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_session_orders_by_sort_order() {
        let session_id = IdeationSessionId::new();
        let mut proposal1 = create_test_proposal(&session_id);
        proposal1.sort_order = 2;
        let mut proposal2 = create_test_proposal(&session_id);
        proposal2.sort_order = 1;
        let mut proposal3 = create_test_proposal(&session_id);
        proposal3.sort_order = 3;

        let repo = MockTaskProposalRepository::with_proposals(vec![
            proposal1.clone(),
            proposal2.clone(),
            proposal3.clone(),
        ]);

        let result = repo.get_by_session(&session_id).await;
        assert!(result.is_ok());
        let proposals = result.unwrap();
        assert_eq!(proposals.len(), 3);
        assert_eq!(proposals[0].sort_order, 1);
        assert_eq!(proposals[1].sort_order, 2);
        assert_eq!(proposals[2].sort_order, 3);
    }

    #[tokio::test]
    async fn test_mock_repository_update() {
        let repo = MockTaskProposalRepository::new();
        let session_id = IdeationSessionId::new();
        let proposal = create_test_proposal(&session_id);

        let result = repo.update(&proposal).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_repository_update_priority() {
        let repo = MockTaskProposalRepository::new();
        let proposal_id = TaskProposalId::new();
        let assessment = create_test_assessment(&proposal_id);

        let result = repo.update_priority(&proposal_id, &assessment).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_repository_update_selection() {
        let repo = MockTaskProposalRepository::new();
        let proposal_id = TaskProposalId::new();

        let result = repo.update_selection(&proposal_id, true).await;
        assert!(result.is_ok());

        let result = repo.update_selection(&proposal_id, false).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_repository_set_created_task_id() {
        let repo = MockTaskProposalRepository::new();
        let proposal_id = TaskProposalId::new();
        let task_id = TaskId::new();

        let result = repo.set_created_task_id(&proposal_id, &task_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_repository_delete() {
        let repo = MockTaskProposalRepository::new();
        let proposal_id = TaskProposalId::new();

        let result = repo.delete(&proposal_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_repository_reorder() {
        let repo = MockTaskProposalRepository::new();
        let session_id = IdeationSessionId::new();
        let ids = vec![
            TaskProposalId::new(),
            TaskProposalId::new(),
            TaskProposalId::new(),
        ];

        let result = repo.reorder(&session_id, ids).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_repository_get_selected_by_session_empty() {
        let repo = MockTaskProposalRepository::new();
        let session_id = IdeationSessionId::new();

        let result = repo.get_selected_by_session(&session_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_repository_get_selected_by_session_filters_selected() {
        let session_id = IdeationSessionId::new();
        let mut selected_proposal = create_test_proposal(&session_id);
        selected_proposal.selected = true;

        let mut unselected_proposal = create_test_proposal(&session_id);
        unselected_proposal.selected = false;

        let repo = MockTaskProposalRepository::with_proposals(vec![
            selected_proposal.clone(),
            unselected_proposal.clone(),
        ]);

        let result = repo.get_selected_by_session(&session_id).await;
        assert!(result.is_ok());
        let proposals = result.unwrap();
        assert_eq!(proposals.len(), 1);
        assert!(proposals[0].selected);
    }

    #[tokio::test]
    async fn test_mock_repository_count_by_session_zero() {
        let repo = MockTaskProposalRepository::new();
        let session_id = IdeationSessionId::new();

        let result = repo.count_by_session(&session_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_mock_repository_count_by_session_counts_correctly() {
        let session_id = IdeationSessionId::new();
        let other_session_id = IdeationSessionId::new();
        let proposal1 = create_test_proposal(&session_id);
        let proposal2 = create_test_proposal(&session_id);
        let proposal3 = create_test_proposal(&other_session_id);

        let repo = MockTaskProposalRepository::with_proposals(vec![
            proposal1.clone(),
            proposal2.clone(),
            proposal3.clone(),
        ]);

        let result = repo.count_by_session(&session_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_mock_repository_count_selected_by_session() {
        let session_id = IdeationSessionId::new();
        let mut selected1 = create_test_proposal(&session_id);
        selected1.selected = true;
        let mut selected2 = create_test_proposal(&session_id);
        selected2.selected = true;
        let mut unselected = create_test_proposal(&session_id);
        unselected.selected = false;

        let repo = MockTaskProposalRepository::with_proposals(vec![
            selected1.clone(),
            selected2.clone(),
            unselected.clone(),
        ]);

        let selected_count = repo.count_selected_by_session(&session_id).await;
        assert!(selected_count.is_ok());
        assert_eq!(selected_count.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_repository_trait_object_in_arc() {
        let session_id = IdeationSessionId::new();
        let proposal = create_test_proposal(&session_id);
        let repo: Arc<dyn TaskProposalRepository> =
            Arc::new(MockTaskProposalRepository::with_proposal(proposal.clone()));

        // Use through trait object
        let result = repo.get_by_id(&proposal.id).await;
        assert!(result.is_ok());

        let all = repo.get_by_session(&session_id).await;
        assert!(all.is_ok());
        assert_eq!(all.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_repository_trait_object_priority_operations() {
        let session_id = IdeationSessionId::new();
        let proposal = create_test_proposal(&session_id);
        let repo: Arc<dyn TaskProposalRepository> =
            Arc::new(MockTaskProposalRepository::with_proposal(proposal.clone()));

        let assessment = create_test_assessment(&proposal.id);
        let result = repo.update_priority(&proposal.id, &assessment).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_repository_trait_object_selection_operations() {
        let session_id = IdeationSessionId::new();
        let proposal = create_test_proposal(&session_id);
        let repo: Arc<dyn TaskProposalRepository> =
            Arc::new(MockTaskProposalRepository::with_proposal(proposal.clone()));

        let result = repo.update_selection(&proposal.id, false).await;
        assert!(result.is_ok());

        let selected = repo.get_selected_by_session(&session_id).await;
        assert!(selected.is_ok());
    }

    #[tokio::test]
    async fn test_repository_trait_object_task_linking() {
        let session_id = IdeationSessionId::new();
        let proposal = create_test_proposal(&session_id);
        let repo: Arc<dyn TaskProposalRepository> =
            Arc::new(MockTaskProposalRepository::with_proposal(proposal.clone()));

        let task_id = TaskId::new();
        let result = repo.set_created_task_id(&proposal.id, &task_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_repository_trait_object_reorder_operations() {
        let session_id = IdeationSessionId::new();
        let proposal1 = create_test_proposal(&session_id);
        let proposal2 = create_test_proposal(&session_id);
        let repo: Arc<dyn TaskProposalRepository> = Arc::new(
            MockTaskProposalRepository::with_proposals(vec![proposal1.clone(), proposal2.clone()]),
        );

        let result = repo
            .reorder(&session_id, vec![proposal2.id.clone(), proposal1.id.clone()])
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_repository_trait_object_count_operations() {
        let session_id = IdeationSessionId::new();
        let proposal = create_test_proposal(&session_id);
        let repo: Arc<dyn TaskProposalRepository> =
            Arc::new(MockTaskProposalRepository::with_proposal(proposal.clone()));

        let total_count = repo.count_by_session(&session_id).await;
        assert!(total_count.is_ok());
        assert_eq!(total_count.unwrap(), 1);

        let selected_count = repo.count_selected_by_session(&session_id).await;
        assert!(selected_count.is_ok());
    }
}
