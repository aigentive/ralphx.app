use super::*;

#[tokio::test]
async fn test_get_returns_none_when_no_active_plan() {
    let repo = MemoryActivePlanRepository::new();
    let project_id = ProjectId::from_string("proj-123".to_string());

    let result = repo.get(&project_id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_set_and_get_active_plan() {
    let repo = MemoryActivePlanRepository::new();
    let project_id = ProjectId::from_string("proj-123".to_string());
    let session_id = IdeationSessionId::from_string("session-456");

    repo.set(&project_id, &session_id).await.unwrap();

    let result = repo.get(&project_id).await.unwrap();
    assert_eq!(result, Some(session_id));
}

#[tokio::test]
async fn test_set_updates_existing_active_plan() {
    let repo = MemoryActivePlanRepository::new();
    let project_id = ProjectId::from_string("proj-123".to_string());
    let session_id1 = IdeationSessionId::from_string("session-456");
    let session_id2 = IdeationSessionId::from_string("session-789");

    repo.set(&project_id, &session_id1).await.unwrap();
    repo.set(&project_id, &session_id2).await.unwrap();

    let result = repo.get(&project_id).await.unwrap();
    assert_eq!(result, Some(session_id2));
}

#[tokio::test]
async fn test_clear_removes_active_plan() {
    let repo = MemoryActivePlanRepository::new();
    let project_id = ProjectId::from_string("proj-123".to_string());
    let session_id = IdeationSessionId::from_string("session-456");

    repo.set(&project_id, &session_id).await.unwrap();
    repo.clear(&project_id).await.unwrap();

    let result = repo.get(&project_id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_exists_returns_false_when_no_active_plan() {
    let repo = MemoryActivePlanRepository::new();
    let project_id = ProjectId::from_string("proj-123".to_string());

    let exists = repo.exists(&project_id).await.unwrap();
    assert!(!exists);
}

#[tokio::test]
async fn test_exists_returns_true_when_active_plan_set() {
    let repo = MemoryActivePlanRepository::new();
    let project_id = ProjectId::from_string("proj-123".to_string());
    let session_id = IdeationSessionId::from_string("session-456");

    repo.set(&project_id, &session_id).await.unwrap();

    let exists = repo.exists(&project_id).await.unwrap();
    assert!(exists);
}

#[tokio::test]
async fn test_multiple_projects() {
    let repo = MemoryActivePlanRepository::new();
    let project_id1 = ProjectId::from_string("proj-123".to_string());
    let project_id2 = ProjectId::from_string("proj-456".to_string());
    let session_id1 = IdeationSessionId::from_string("session-789");
    let session_id2 = IdeationSessionId::from_string("session-101");

    repo.set(&project_id1, &session_id1).await.unwrap();
    repo.set(&project_id2, &session_id2).await.unwrap();

    let result1 = repo.get(&project_id1).await.unwrap();
    let result2 = repo.get(&project_id2).await.unwrap();

    assert_eq!(result1, Some(session_id1));
    assert_eq!(result2, Some(session_id2));
}

#[tokio::test]
async fn test_record_selection_creates_stats() {
    let repo = MemoryActivePlanRepository::new();
    let project_id = ProjectId::from_string("proj-123".to_string());
    let session_id = IdeationSessionId::from_string("session-456");

    repo.record_selection(&project_id, &session_id, "kanban_inline")
        .await
        .unwrap();

    let stats = repo.selection_stats.read().await;
    let key = (
        project_id.as_str().to_string(),
        session_id.as_str().to_string(),
    );
    let stat = stats.get(&key).unwrap();

    assert_eq!(stat.selected_count, 1);
    assert_eq!(stat.last_selected_source, "kanban_inline");
}

#[tokio::test]
async fn test_record_selection_increments_count() {
    let repo = MemoryActivePlanRepository::new();
    let project_id = ProjectId::from_string("proj-123".to_string());
    let session_id = IdeationSessionId::from_string("session-456");

    repo.record_selection(&project_id, &session_id, "kanban_inline")
        .await
        .unwrap();
    repo.record_selection(&project_id, &session_id, "graph_inline")
        .await
        .unwrap();
    repo.record_selection(&project_id, &session_id, "quick_switcher")
        .await
        .unwrap();

    let stats = repo.selection_stats.read().await;
    let key = (
        project_id.as_str().to_string(),
        session_id.as_str().to_string(),
    );
    let stat = stats.get(&key).unwrap();

    assert_eq!(stat.selected_count, 3);
    assert_eq!(stat.last_selected_source, "quick_switcher");
}
