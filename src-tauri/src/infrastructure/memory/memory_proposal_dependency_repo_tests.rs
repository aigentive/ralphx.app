use super::*;

#[tokio::test]
async fn test_add_and_get_dependencies() {
    let repo = MemoryProposalDependencyRepository::new();
    let p1 = TaskProposalId::new();
    let p2 = TaskProposalId::new();

    repo.add_dependency(&p1, &p2, None, None).await.unwrap();

    let deps = repo.get_dependencies(&p1).await.unwrap();
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0].to_string(), p2.to_string());
}

#[tokio::test]
async fn test_get_dependents() {
    let repo = MemoryProposalDependencyRepository::new();
    let p1 = TaskProposalId::new();
    let p2 = TaskProposalId::new();

    repo.add_dependency(&p1, &p2, None, None).await.unwrap();

    let dependents = repo.get_dependents(&p2).await.unwrap();
    assert_eq!(dependents.len(), 1);
    assert_eq!(dependents[0].to_string(), p1.to_string());
}

#[tokio::test]
async fn test_remove_dependency() {
    let repo = MemoryProposalDependencyRepository::new();
    let p1 = TaskProposalId::new();
    let p2 = TaskProposalId::new();

    repo.add_dependency(&p1, &p2, None, None).await.unwrap();
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

    repo.add_dependency(&p1, &p2, None, None).await.unwrap();
    repo.add_dependency(&p3, &p1, None, None).await.unwrap();

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

    repo.add_with_session(&p1, &p2, &session_id, "auto");

    let all = repo.get_all_for_session(&session_id).await.unwrap();
    assert_eq!(all.len(), 1);
}

// ==================== SOURCE-AWARE METHODS TESTS ====================

#[tokio::test]
async fn test_get_all_for_session_with_source_includes_source_field() {
    let repo = MemoryProposalDependencyRepository::new();
    let session_id = IdeationSessionId::new();
    let p1 = TaskProposalId::new();
    let p2 = TaskProposalId::new();

    repo.add_with_session(&p1, &p2, &session_id, "auto");

    let all = repo.get_all_for_session_with_source(&session_id).await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].0, p1);
    assert_eq!(all[0].1, p2);
    assert_eq!(all[0].3, "auto");
}

#[tokio::test]
async fn test_add_dependency_with_manual_source() {
    let repo = MemoryProposalDependencyRepository::new();
    let p1 = TaskProposalId::new();
    let p2 = TaskProposalId::new();

    repo.add_dependency(&p1, &p2, None, Some("manual"))
        .await
        .unwrap();

    let deps = repo.get_dependencies(&p1).await.unwrap();
    assert_eq!(deps.len(), 1);
}

#[tokio::test]
async fn test_add_dependency_defaults_to_auto() {
    let repo = MemoryProposalDependencyRepository::new();
    let session_id = IdeationSessionId::new();
    let p1 = TaskProposalId::new();
    let p2 = TaskProposalId::new();

    // Add dependency with source="auto" explicitly
    repo.add_with_session(&p1, &p2, &session_id, "auto");

    let all = repo.get_all_for_session_with_source(&session_id).await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].3, "auto");
}

#[tokio::test]
async fn test_clear_auto_dependencies_preserves_manual_deps() {
    let repo = MemoryProposalDependencyRepository::new();
    let session_id = IdeationSessionId::new();
    let p1 = TaskProposalId::new();
    let p2 = TaskProposalId::new();
    let p3 = TaskProposalId::new();

    // Add auto dependency: p1 -> p2
    repo.add_with_session(&p1, &p2, &session_id, "auto");
    // Add manual dependency: p2 -> p3
    repo.add_with_session(&p2, &p3, &session_id, "manual");

    // Clear only auto dependencies
    repo.clear_auto_dependencies(&session_id).await.unwrap();

    let all = repo.get_all_for_session_with_source(&session_id).await.unwrap();
    assert_eq!(all.len(), 1);
    // Only the manual dependency should remain
    assert_eq!(all[0].0, p2);
    assert_eq!(all[0].1, p3);
    assert_eq!(all[0].3, "manual");
}

#[tokio::test]
async fn test_clear_auto_dependencies_clears_only_in_session() {
    let repo = MemoryProposalDependencyRepository::new();
    let session1 = IdeationSessionId::new();
    let session2 = IdeationSessionId::new();
    let s1_p1 = TaskProposalId::new();
    let s1_p2 = TaskProposalId::new();
    let s2_p1 = TaskProposalId::new();
    let s2_p2 = TaskProposalId::new();

    // Create auto deps in both sessions
    repo.add_with_session(&s1_p1, &s1_p2, &session1, "auto");
    repo.add_with_session(&s2_p1, &s2_p2, &session2, "auto");

    // Clear auto deps only for session 1
    repo.clear_auto_dependencies(&session1).await.unwrap();

    // Session 1 should have no deps
    let s1_all = repo.get_all_for_session_with_source(&session1).await.unwrap();
    assert_eq!(s1_all.len(), 0);

    // Session 2 should still have its auto dep
    let s2_all = repo.get_all_for_session_with_source(&session2).await.unwrap();
    assert_eq!(s2_all.len(), 1);
    assert_eq!(s2_all[0].3, "auto");
}
