use super::*;

#[tokio::test]
async fn test_record_selection_creates_new_entry() {
    let repo = MemoryPlanSelectionStatsRepository::new();
    let project_id = ProjectId::new();
    let session_id = IdeationSessionId::new();
    let timestamp = Utc::now();

    repo.record_selection(
        &project_id,
        &session_id,
        SelectionSource::KanbanInline,
        timestamp,
    )
    .await
    .unwrap();

    let stats = repo.get_stats(&project_id, &session_id).await.unwrap();
    assert!(stats.is_some());
    let stats = stats.unwrap();
    assert_eq!(stats.selected_count, 1);
    assert_eq!(
        stats.last_selected_source,
        Some("kanban_inline".to_string())
    );
}

#[tokio::test]
async fn test_record_selection_increments_count() {
    let repo = MemoryPlanSelectionStatsRepository::new();
    let project_id = ProjectId::new();
    let session_id = IdeationSessionId::new();
    let timestamp1 = Utc::now();

    // First selection
    repo.record_selection(
        &project_id,
        &session_id,
        SelectionSource::KanbanInline,
        timestamp1,
    )
    .await
    .unwrap();

    // Second selection
    let timestamp2 = Utc::now();
    repo.record_selection(
        &project_id,
        &session_id,
        SelectionSource::QuickSwitcher,
        timestamp2,
    )
    .await
    .unwrap();

    let stats = repo
        .get_stats(&project_id, &session_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stats.selected_count, 2);
    assert_eq!(
        stats.last_selected_source,
        Some("quick_switcher".to_string())
    );
}

#[tokio::test]
async fn test_get_stats_batch() {
    let repo = MemoryPlanSelectionStatsRepository::new();
    let project_id = ProjectId::new();
    let session1 = IdeationSessionId::new();
    let session2 = IdeationSessionId::new();
    let session3 = IdeationSessionId::new(); // Not recorded
    let timestamp = Utc::now();

    // Record stats for session1 and session2
    repo.record_selection(
        &project_id,
        &session1,
        SelectionSource::KanbanInline,
        timestamp,
    )
    .await
    .unwrap();
    repo.record_selection(
        &project_id,
        &session2,
        SelectionSource::GraphInline,
        timestamp,
    )
    .await
    .unwrap();

    // Query batch
    let results = repo
        .get_stats_batch(
            &project_id,
            &[session1.clone(), session2.clone(), session3.clone()],
        )
        .await
        .unwrap();

    assert_eq!(results.len(), 3);
    assert!(results[0].is_some());
    assert_eq!(results[0].as_ref().unwrap().ideation_session_id, session1);
    assert!(results[1].is_some());
    assert_eq!(results[1].as_ref().unwrap().ideation_session_id, session2);
    assert!(results[2].is_none()); // session3 not recorded
}

#[tokio::test]
async fn test_get_stats_nonexistent() {
    let repo = MemoryPlanSelectionStatsRepository::new();
    let project_id = ProjectId::new();
    let session_id = IdeationSessionId::new();

    let stats = repo.get_stats(&project_id, &session_id).await.unwrap();
    assert!(stats.is_none());
}
