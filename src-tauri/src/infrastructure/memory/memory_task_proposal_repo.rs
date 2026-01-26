// In-memory TaskProposalRepository implementation for testing
// Uses RwLock<HashMap> for thread-safe in-memory storage

use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;

use crate::domain::entities::{
    ArtifactId, IdeationSessionId, PriorityAssessment, TaskId, TaskProposal, TaskProposalId,
};
use crate::domain::repositories::TaskProposalRepository;
use crate::error::AppResult;

/// In-memory implementation of TaskProposalRepository for testing
pub struct MemoryTaskProposalRepository {
    proposals: RwLock<HashMap<String, TaskProposal>>,
}

impl MemoryTaskProposalRepository {
    /// Create a new empty repository
    pub fn new() -> Self {
        Self {
            proposals: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryTaskProposalRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TaskProposalRepository for MemoryTaskProposalRepository {
    async fn create(&self, proposal: TaskProposal) -> AppResult<TaskProposal> {
        self.proposals
            .write()
            .unwrap()
            .insert(proposal.id.to_string(), proposal.clone());
        Ok(proposal)
    }

    async fn get_by_id(&self, id: &TaskProposalId) -> AppResult<Option<TaskProposal>> {
        Ok(self.proposals.read().unwrap().get(&id.to_string()).cloned())
    }

    async fn get_by_session(&self, session_id: &IdeationSessionId) -> AppResult<Vec<TaskProposal>> {
        let mut proposals: Vec<_> = self
            .proposals
            .read()
            .unwrap()
            .values()
            .filter(|p| &p.session_id == session_id)
            .cloned()
            .collect();
        proposals.sort_by_key(|p| p.sort_order);
        Ok(proposals)
    }

    async fn update(&self, proposal: &TaskProposal) -> AppResult<()> {
        self.proposals
            .write()
            .unwrap()
            .insert(proposal.id.to_string(), proposal.clone());
        Ok(())
    }

    async fn update_priority(
        &self,
        id: &TaskProposalId,
        assessment: &PriorityAssessment,
    ) -> AppResult<()> {
        if let Some(p) = self.proposals.write().unwrap().get_mut(&id.to_string()) {
            p.suggested_priority = assessment.suggested_priority;
            p.priority_score = assessment.priority_score;
        }
        Ok(())
    }

    async fn update_selection(&self, id: &TaskProposalId, selected: bool) -> AppResult<()> {
        if let Some(p) = self.proposals.write().unwrap().get_mut(&id.to_string()) {
            p.selected = selected;
        }
        Ok(())
    }

    async fn set_created_task_id(
        &self,
        id: &TaskProposalId,
        task_id: &TaskId,
    ) -> AppResult<()> {
        if let Some(p) = self.proposals.write().unwrap().get_mut(&id.to_string()) {
            p.created_task_id = Some(task_id.clone());
        }
        Ok(())
    }

    async fn delete(&self, id: &TaskProposalId) -> AppResult<()> {
        self.proposals.write().unwrap().remove(&id.to_string());
        Ok(())
    }

    async fn reorder(
        &self,
        _session_id: &IdeationSessionId,
        proposal_ids: Vec<TaskProposalId>,
    ) -> AppResult<()> {
        let mut proposals = self.proposals.write().unwrap();
        for (i, id) in proposal_ids.iter().enumerate() {
            if let Some(p) = proposals.get_mut(&id.to_string()) {
                p.sort_order = i as i32;
            }
        }
        Ok(())
    }

    async fn get_selected_by_session(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<Vec<TaskProposal>> {
        let mut proposals: Vec<_> = self
            .proposals
            .read()
            .unwrap()
            .values()
            .filter(|p| &p.session_id == session_id && p.selected)
            .cloned()
            .collect();
        proposals.sort_by_key(|p| p.sort_order);
        Ok(proposals)
    }

    async fn count_by_session(&self, session_id: &IdeationSessionId) -> AppResult<u32> {
        Ok(self
            .proposals
            .read()
            .unwrap()
            .values()
            .filter(|p| &p.session_id == session_id)
            .count() as u32)
    }

    async fn count_selected_by_session(&self, session_id: &IdeationSessionId) -> AppResult<u32> {
        Ok(self
            .proposals
            .read()
            .unwrap()
            .values()
            .filter(|p| &p.session_id == session_id && p.selected)
            .count() as u32)
    }

    async fn get_by_plan_artifact_id(&self, artifact_id: &ArtifactId) -> AppResult<Vec<TaskProposal>> {
        let mut proposals: Vec<_> = self
            .proposals
            .read()
            .unwrap()
            .values()
            .filter(|p| p.plan_artifact_id.as_ref() == Some(artifact_id))
            .cloned()
            .collect();
        proposals.sort_by_key(|p| p.sort_order);
        Ok(proposals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{Priority, TaskCategory};

    fn create_test_proposal(session_id: &IdeationSessionId) -> TaskProposal {
        TaskProposal::new(
            session_id.clone(),
            "Test Proposal",
            TaskCategory::Feature,
            Priority::Medium,
        )
    }

    #[tokio::test]
    async fn test_create_and_get() {
        let repo = MemoryTaskProposalRepository::new();
        let session_id = IdeationSessionId::new();
        let proposal = create_test_proposal(&session_id);

        repo.create(proposal.clone()).await.unwrap();

        let retrieved = repo.get_by_id(&proposal.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, proposal.id);
    }

    #[tokio::test]
    async fn test_get_by_session() {
        let repo = MemoryTaskProposalRepository::new();
        let session_id = IdeationSessionId::new();
        let proposal = create_test_proposal(&session_id);

        repo.create(proposal).await.unwrap();

        let proposals = repo.get_by_session(&session_id).await.unwrap();
        assert_eq!(proposals.len(), 1);
    }

    #[tokio::test]
    async fn test_update_selection() {
        let repo = MemoryTaskProposalRepository::new();
        let session_id = IdeationSessionId::new();
        let proposal = create_test_proposal(&session_id);
        let proposal_id = proposal.id.clone();

        repo.create(proposal).await.unwrap();
        repo.update_selection(&proposal_id, false).await.unwrap();

        let updated = repo.get_by_id(&proposal_id).await.unwrap().unwrap();
        assert!(!updated.selected);
    }

    #[tokio::test]
    async fn test_get_selected_by_session() {
        let repo = MemoryTaskProposalRepository::new();
        let session_id = IdeationSessionId::new();

        let mut p1 = create_test_proposal(&session_id);
        p1.selected = true;
        let mut p2 = create_test_proposal(&session_id);
        p2.selected = false;

        repo.create(p1).await.unwrap();
        repo.create(p2).await.unwrap();

        let selected = repo.get_selected_by_session(&session_id).await.unwrap();
        assert_eq!(selected.len(), 1);
    }

    #[tokio::test]
    async fn test_delete() {
        let repo = MemoryTaskProposalRepository::new();
        let session_id = IdeationSessionId::new();
        let proposal = create_test_proposal(&session_id);
        let proposal_id = proposal.id.clone();

        repo.create(proposal).await.unwrap();
        repo.delete(&proposal_id).await.unwrap();

        let result = repo.get_by_id(&proposal_id).await.unwrap();
        assert!(result.is_none());
    }
}
