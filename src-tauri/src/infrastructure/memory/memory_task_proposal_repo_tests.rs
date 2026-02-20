use super::*;
use crate::domain::entities::{Priority, ProposalCategory};

fn create_test_proposal(session_id: &IdeationSessionId) -> TaskProposal {
    TaskProposal::new(
        session_id.clone(),
        "Test Proposal",
        ProposalCategory::Feature,
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
