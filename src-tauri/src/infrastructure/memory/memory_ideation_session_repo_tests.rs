use super::*;

#[tokio::test]
async fn test_create_and_get() {
    let repo = MemoryIdeationSessionRepository::new();
    let project_id = ProjectId::new();
    let session = IdeationSession::new_with_title(project_id.clone(), "Test Session");

    repo.create(session.clone()).await.unwrap();

    let retrieved = repo.get_by_id(&session.id).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id, session.id);
}

#[tokio::test]
async fn test_get_by_project() {
    let repo = MemoryIdeationSessionRepository::new();
    let project_id = ProjectId::new();
    let session = IdeationSession::new(project_id.clone());

    repo.create(session).await.unwrap();

    let sessions = repo.get_by_project(&project_id).await.unwrap();
    assert_eq!(sessions.len(), 1);
}

#[tokio::test]
async fn test_update_status() {
    let repo = MemoryIdeationSessionRepository::new();
    let project_id = ProjectId::new();
    let session = IdeationSession::new(project_id.clone());
    let session_id = session.id.clone();

    repo.create(session).await.unwrap();
    repo.update_status(&session_id, IdeationSessionStatus::Archived)
        .await
        .unwrap();

    let updated = repo.get_by_id(&session_id).await.unwrap().unwrap();
    assert_eq!(updated.status, IdeationSessionStatus::Archived);
    assert!(updated.archived_at.is_some());
}

#[tokio::test]
async fn test_delete() {
    let repo = MemoryIdeationSessionRepository::new();
    let project_id = ProjectId::new();
    let session = IdeationSession::new(project_id.clone());
    let session_id = session.id.clone();

    repo.create(session).await.unwrap();
    repo.delete(&session_id).await.unwrap();

    let result = repo.get_by_id(&session_id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_get_children() {
    let repo = MemoryIdeationSessionRepository::new();
    let project_id = ProjectId::new();

    let parent = IdeationSession::new(project_id.clone());
    let mut child1 = IdeationSession::new(project_id.clone());
    child1.parent_session_id = Some(parent.id.clone());
    let mut child2 = IdeationSession::new(project_id.clone());
    child2.parent_session_id = Some(parent.id.clone());

    repo.create(parent.clone()).await.unwrap();
    repo.create(child1.clone()).await.unwrap();
    repo.create(child2.clone()).await.unwrap();

    let children = repo.get_children(&parent.id).await.unwrap();
    assert_eq!(children.len(), 2);
}

#[tokio::test]
async fn test_get_children_returns_empty_for_sessions_without_children() {
    let repo = MemoryIdeationSessionRepository::new();
    let project_id = ProjectId::new();

    let session = IdeationSession::new(project_id.clone());
    repo.create(session.clone()).await.unwrap();

    let children = repo.get_children(&session.id).await.unwrap();
    assert!(children.is_empty());
}

#[tokio::test]
async fn test_get_ancestor_chain_three_levels_deep() {
    let repo = MemoryIdeationSessionRepository::new();
    let project_id = ProjectId::new();

    let level1 = IdeationSession::new(project_id.clone());
    let mut level2 = IdeationSession::new(project_id.clone());
    level2.parent_session_id = Some(level1.id.clone());
    let mut level3 = IdeationSession::new(project_id.clone());
    level3.parent_session_id = Some(level2.id.clone());

    repo.create(level1.clone()).await.unwrap();
    repo.create(level2.clone()).await.unwrap();
    repo.create(level3.clone()).await.unwrap();

    let chain = repo.get_ancestor_chain(&level3.id).await.unwrap();
    // Should return: [level2, level1] (direct parent to root)
    assert_eq!(chain.len(), 2);
    assert_eq!(chain[0].id, level2.id);
    assert_eq!(chain[1].id, level1.id);
}

#[tokio::test]
async fn test_get_ancestor_chain_single_parent() {
    let repo = MemoryIdeationSessionRepository::new();
    let project_id = ProjectId::new();

    let parent = IdeationSession::new(project_id.clone());
    let mut child = IdeationSession::new(project_id.clone());
    child.parent_session_id = Some(parent.id.clone());

    repo.create(parent.clone()).await.unwrap();
    repo.create(child.clone()).await.unwrap();

    let chain = repo.get_ancestor_chain(&child.id).await.unwrap();
    assert_eq!(chain.len(), 1);
    assert_eq!(chain[0].id, parent.id);
}

#[tokio::test]
async fn test_get_ancestor_chain_no_parent() {
    let repo = MemoryIdeationSessionRepository::new();
    let project_id = ProjectId::new();

    let session = IdeationSession::new(project_id.clone());
    repo.create(session.clone()).await.unwrap();

    let chain = repo.get_ancestor_chain(&session.id).await.unwrap();
    assert!(chain.is_empty());
}

#[tokio::test]
async fn test_set_parent() {
    let repo = MemoryIdeationSessionRepository::new();
    let project_id = ProjectId::new();

    let parent = IdeationSession::new(project_id.clone());
    let child = IdeationSession::new(project_id.clone());

    repo.create(parent.clone()).await.unwrap();
    repo.create(child.clone()).await.unwrap();

    repo.set_parent(&child.id, Some(&parent.id)).await.unwrap();

    let updated_child = repo.get_by_id(&child.id).await.unwrap().unwrap();
    assert_eq!(updated_child.parent_session_id, Some(parent.id.clone()));
}

#[tokio::test]
async fn test_set_parent_with_null() {
    let repo = MemoryIdeationSessionRepository::new();
    let project_id = ProjectId::new();

    let parent = IdeationSession::new(project_id.clone());
    let mut child = IdeationSession::new(project_id.clone());
    child.parent_session_id = Some(parent.id.clone());

    repo.create(parent.clone()).await.unwrap();
    repo.create(child.clone()).await.unwrap();

    repo.set_parent(&child.id, None).await.unwrap();

    let updated_child = repo.get_by_id(&child.id).await.unwrap().unwrap();
    assert!(updated_child.parent_session_id.is_none());
}
