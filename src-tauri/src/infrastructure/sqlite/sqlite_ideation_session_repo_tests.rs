use super::*;
use crate::domain::entities::VerificationStatus;
use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

fn setup_test_db() -> Connection {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    conn
}

fn create_test_project(conn: &Connection, id: &ProjectId, name: &str, path: &str) {
    conn.execute(
        "INSERT INTO projects (id, name, working_directory, git_mode, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'single_branch', strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'), strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
        rusqlite::params![id.as_str(), name, path],
    )
    .unwrap();
}

fn create_test_session(project_id: &ProjectId, title: Option<&str>) -> IdeationSession {
    let mut builder = IdeationSession::builder().project_id(project_id.clone());

    if let Some(t) = title {
        builder = builder.title(t);
    }

    builder.build()
}

// ==================== CREATE TESTS ====================

#[tokio::test]
async fn test_create_inserts_session_and_returns_it() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);
    let session = create_test_session(&project_id, Some("My Ideation"));

    let result = repo.create(session.clone()).await;

    assert!(result.is_ok());
    let created = result.unwrap();
    assert_eq!(created.id, session.id);
    assert_eq!(created.title, Some("My Ideation".to_string()));
    assert_eq!(created.status, IdeationSessionStatus::Active);
}

#[tokio::test]
async fn test_create_session_without_title() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);
    let session = create_test_session(&project_id, None);

    let result = repo.create(session.clone()).await;

    assert!(result.is_ok());
    let created = result.unwrap();
    assert_eq!(created.title, None);
}

#[tokio::test]
async fn test_create_duplicate_id_fails() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);
    let session = create_test_session(&project_id, Some("Original"));

    repo.create(session.clone()).await.unwrap();
    let result = repo.create(session).await;

    assert!(result.is_err());
}

// ==================== GET BY ID TESTS ====================

#[tokio::test]
async fn test_get_by_id_retrieves_session_correctly() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);
    let session = create_test_session(&project_id, Some("Test Session"));

    repo.create(session.clone()).await.unwrap();
    let result = repo.get_by_id(&session.id).await;

    assert!(result.is_ok());
    let found = result.unwrap();
    assert!(found.is_some());
    let found_session = found.unwrap();
    assert_eq!(found_session.id, session.id);
    assert_eq!(found_session.title, Some("Test Session".to_string()));
    assert_eq!(found_session.project_id, project_id);
}

#[tokio::test]
async fn test_get_by_id_returns_none_for_nonexistent() {
    let conn = setup_test_db();
    let repo = SqliteIdeationSessionRepository::new(conn);
    let id = IdeationSessionId::new();

    let result = repo.get_by_id(&id).await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_get_by_id_preserves_all_fields() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);

    // Create a session with all fields set
    let mut session = create_test_session(&project_id, Some("Full Session"));
    session.archive(); // This sets archived_at

    repo.create(session.clone()).await.unwrap();
    let found = repo.get_by_id(&session.id).await.unwrap().unwrap();

    assert_eq!(found.id, session.id);
    assert_eq!(found.project_id, session.project_id);
    assert_eq!(found.title, session.title);
    assert_eq!(found.status, IdeationSessionStatus::Archived);
    assert!(found.archived_at.is_some());
}

// ==================== GET BY PROJECT TESTS ====================

#[tokio::test]
async fn test_get_by_project_returns_all_sessions() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);

    let session1 = create_test_session(&project_id, Some("Session 1"));
    let session2 = create_test_session(&project_id, Some("Session 2"));
    let session3 = create_test_session(&project_id, Some("Session 3"));

    repo.create(session1).await.unwrap();
    repo.create(session2).await.unwrap();
    repo.create(session3).await.unwrap();

    let result = repo.get_by_project(&project_id).await;

    assert!(result.is_ok());
    let sessions = result.unwrap();
    assert_eq!(sessions.len(), 3);
}

#[tokio::test]
async fn test_get_by_project_ordered_by_updated_at_desc() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);

    // Create sessions with different timestamps
    let session1 = IdeationSession::builder()
        .project_id(project_id.clone())
        .title("Oldest")
        .created_at(chrono::Utc::now() - chrono::Duration::hours(3))
        .updated_at(chrono::Utc::now() - chrono::Duration::hours(3))
        .build();
    let session2 = IdeationSession::builder()
        .project_id(project_id.clone())
        .title("Middle")
        .created_at(chrono::Utc::now() - chrono::Duration::hours(2))
        .updated_at(chrono::Utc::now() - chrono::Duration::hours(2))
        .build();
    let session3 = IdeationSession::builder()
        .project_id(project_id.clone())
        .title("Newest")
        .created_at(chrono::Utc::now() - chrono::Duration::hours(1))
        .updated_at(chrono::Utc::now() - chrono::Duration::hours(1))
        .build();

    // Insert in non-order (oldest first, then newest, then middle)
    repo.create(session1).await.unwrap();
    repo.create(session3).await.unwrap();
    repo.create(session2).await.unwrap();

    let sessions = repo.get_by_project(&project_id).await.unwrap();

    // Should be ordered newest first
    assert_eq!(sessions.len(), 3);
    assert_eq!(sessions[0].title, Some("Newest".to_string()));
    assert_eq!(sessions[1].title, Some("Middle".to_string()));
    assert_eq!(sessions[2].title, Some("Oldest".to_string()));
}

#[tokio::test]
async fn test_get_by_project_returns_empty_for_no_sessions() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);

    let result = repo.get_by_project(&project_id).await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn test_get_by_project_filters_by_project() {
    let conn = setup_test_db();
    let project_id1 = ProjectId::new();
    let project_id2 = ProjectId::new();
    create_test_project(&conn, &project_id1, "Project 1", "/path1");
    create_test_project(&conn, &project_id2, "Project 2", "/path2");

    let repo = SqliteIdeationSessionRepository::new(conn);

    let session1 = create_test_session(&project_id1, Some("Session for P1"));
    let session2 = create_test_session(&project_id2, Some("Session for P2"));

    repo.create(session1).await.unwrap();
    repo.create(session2).await.unwrap();

    let sessions = repo.get_by_project(&project_id1).await.unwrap();

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].project_id, project_id1);
}

// ==================== UPDATE STATUS TESTS ====================

#[tokio::test]
async fn test_update_status_to_archived() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);
    let session = create_test_session(&project_id, Some("To Archive"));

    repo.create(session.clone()).await.unwrap();

    let result = repo
        .update_status(&session.id, IdeationSessionStatus::Archived)
        .await;
    assert!(result.is_ok());

    let found = repo.get_by_id(&session.id).await.unwrap().unwrap();
    assert_eq!(found.status, IdeationSessionStatus::Archived);
    assert!(found.archived_at.is_some());
}

#[tokio::test]
async fn test_update_status_to_converted() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);
    let session = create_test_session(&project_id, Some("To Convert"));

    repo.create(session.clone()).await.unwrap();

    let result = repo
        .update_status(&session.id, IdeationSessionStatus::Accepted)
        .await;
    assert!(result.is_ok());

    let found = repo.get_by_id(&session.id).await.unwrap().unwrap();
    assert_eq!(found.status, IdeationSessionStatus::Accepted);
    assert!(found.converted_at.is_some());
}

#[tokio::test]
async fn test_update_status_back_to_active() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);
    let mut session = create_test_session(&project_id, Some("Reactivate"));
    session.archive();

    repo.create(session.clone()).await.unwrap();

    let result = repo
        .update_status(&session.id, IdeationSessionStatus::Active)
        .await;
    assert!(result.is_ok());

    let found = repo.get_by_id(&session.id).await.unwrap().unwrap();
    assert_eq!(found.status, IdeationSessionStatus::Active);
}

#[tokio::test]
async fn test_update_status_updates_updated_at() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);
    let session = create_test_session(&project_id, Some("Check Timestamp"));
    let original_updated = session.updated_at;

    repo.create(session.clone()).await.unwrap();

    // Small delay to ensure timestamp difference
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    repo.update_status(&session.id, IdeationSessionStatus::Archived)
        .await
        .unwrap();

    let found = repo.get_by_id(&session.id).await.unwrap().unwrap();
    assert!(found.updated_at >= original_updated);
}

// ==================== UPDATE TITLE TESTS ====================

#[tokio::test]
async fn test_update_title_sets_new_title() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);
    let session = create_test_session(&project_id, Some("Original Title"));

    repo.create(session.clone()).await.unwrap();

    let result = repo
        .update_title(&session.id, Some("New Title".to_string()), "auto")
        .await;
    assert!(result.is_ok());

    let found = repo.get_by_id(&session.id).await.unwrap().unwrap();
    assert_eq!(found.title, Some("New Title".to_string()));
    assert_eq!(found.title_source, Some("auto".to_string()));
}

#[tokio::test]
async fn test_update_title_clears_title() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);
    let session = create_test_session(&project_id, Some("Has Title"));

    repo.create(session.clone()).await.unwrap();

    let result = repo.update_title(&session.id, None, "auto").await;
    assert!(result.is_ok());

    let found = repo.get_by_id(&session.id).await.unwrap().unwrap();
    assert_eq!(found.title, None);
}

#[tokio::test]
async fn test_update_title_user_source() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);
    let session = create_test_session(&project_id, Some("Original"));

    repo.create(session.clone()).await.unwrap();

    repo.update_title(&session.id, Some("User Renamed".to_string()), "user")
        .await
        .unwrap();

    let found = repo.get_by_id(&session.id).await.unwrap().unwrap();
    assert_eq!(found.title, Some("User Renamed".to_string()));
    assert_eq!(found.title_source, Some("user".to_string()));
}

#[tokio::test]
async fn test_update_title_updates_updated_at() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);
    let session = create_test_session(&project_id, Some("Check Timestamp"));
    let original_updated = session.updated_at;

    repo.create(session.clone()).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    repo.update_title(&session.id, Some("Updated".to_string()), "auto")
        .await
        .unwrap();

    let found = repo.get_by_id(&session.id).await.unwrap().unwrap();
    assert!(found.updated_at >= original_updated);
}

// ==================== DELETE TESTS ====================

#[tokio::test]
async fn test_delete_removes_session_from_database() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);
    let session = create_test_session(&project_id, Some("To Delete"));

    repo.create(session.clone()).await.unwrap();

    let delete_result = repo.delete(&session.id).await;
    assert!(delete_result.is_ok());

    let found = repo.get_by_id(&session.id).await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn test_delete_nonexistent_succeeds() {
    let conn = setup_test_db();
    let repo = SqliteIdeationSessionRepository::new(conn);
    let id = IdeationSessionId::new();

    // Deleting a non-existent session should not error
    let result = repo.delete(&id).await;
    assert!(result.is_ok());
}

// ==================== GET ACTIVE BY PROJECT TESTS ====================

#[tokio::test]
async fn test_get_active_by_project_returns_only_active() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);

    let active = create_test_session(&project_id, Some("Active"));
    let mut archived = create_test_session(&project_id, Some("Archived"));
    archived.archive();
    let mut accepted = create_test_session(&project_id, Some("Accepted"));
    accepted.mark_accepted();

    repo.create(active.clone()).await.unwrap();
    repo.create(archived).await.unwrap();
    repo.create(accepted).await.unwrap();

    let result = repo.get_active_by_project(&project_id).await;

    assert!(result.is_ok());
    let sessions = result.unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, active.id);
    assert_eq!(sessions[0].status, IdeationSessionStatus::Active);
}

#[tokio::test]
async fn test_get_active_by_project_returns_empty_when_none_active() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);

    let mut archived = create_test_session(&project_id, Some("Archived"));
    archived.archive();

    repo.create(archived).await.unwrap();

    let result = repo.get_active_by_project(&project_id).await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn test_get_active_by_project_ordered_by_updated_at_desc() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);

    let session1 = IdeationSession::builder()
        .project_id(project_id.clone())
        .title("Oldest Active")
        .updated_at(chrono::Utc::now() - chrono::Duration::hours(2))
        .build();
    let session2 = IdeationSession::builder()
        .project_id(project_id.clone())
        .title("Newest Active")
        .updated_at(chrono::Utc::now() - chrono::Duration::hours(1))
        .build();

    repo.create(session1).await.unwrap();
    repo.create(session2).await.unwrap();

    let sessions = repo.get_active_by_project(&project_id).await.unwrap();

    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].title, Some("Newest Active".to_string()));
    assert_eq!(sessions[1].title, Some("Oldest Active".to_string()));
}

// ==================== COUNT BY STATUS TESTS ====================

#[tokio::test]
async fn test_count_by_status_returns_zero_for_no_sessions() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);

    let result = repo
        .count_by_status(&project_id, IdeationSessionStatus::Active)
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[tokio::test]
async fn test_count_by_status_counts_correctly() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);

    let active1 = create_test_session(&project_id, Some("Active 1"));
    let active2 = create_test_session(&project_id, Some("Active 2"));
    let mut archived = create_test_session(&project_id, Some("Archived"));
    archived.archive();
    let mut accepted = create_test_session(&project_id, Some("Accepted"));
    accepted.mark_accepted();

    repo.create(active1).await.unwrap();
    repo.create(active2).await.unwrap();
    repo.create(archived).await.unwrap();
    repo.create(accepted).await.unwrap();

    let active_count = repo
        .count_by_status(&project_id, IdeationSessionStatus::Active)
        .await
        .unwrap();
    let archived_count = repo
        .count_by_status(&project_id, IdeationSessionStatus::Archived)
        .await
        .unwrap();
    let accepted_count = repo
        .count_by_status(&project_id, IdeationSessionStatus::Accepted)
        .await
        .unwrap();

    assert_eq!(active_count, 2);
    assert_eq!(archived_count, 1);
    assert_eq!(accepted_count, 1);
}

#[tokio::test]
async fn test_count_by_status_filters_by_project() {
    let conn = setup_test_db();
    let project_id1 = ProjectId::new();
    let project_id2 = ProjectId::new();
    create_test_project(&conn, &project_id1, "Project 1", "/path1");
    create_test_project(&conn, &project_id2, "Project 2", "/path2");

    let repo = SqliteIdeationSessionRepository::new(conn);

    let session1 = create_test_session(&project_id1, Some("P1 Session"));
    let session2 = create_test_session(&project_id2, Some("P2 Session 1"));
    let session3 = create_test_session(&project_id2, Some("P2 Session 2"));

    repo.create(session1).await.unwrap();
    repo.create(session2).await.unwrap();
    repo.create(session3).await.unwrap();

    let count_p1 = repo
        .count_by_status(&project_id1, IdeationSessionStatus::Active)
        .await
        .unwrap();
    let count_p2 = repo
        .count_by_status(&project_id2, IdeationSessionStatus::Active)
        .await
        .unwrap();

    assert_eq!(count_p1, 1);
    assert_eq!(count_p2, 2);
}

// ==================== SHARED CONNECTION TESTS ====================

#[tokio::test]
async fn test_from_shared_works_correctly() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let shared_conn = Arc::new(Mutex::new(conn));
    let repo = SqliteIdeationSessionRepository::from_shared(shared_conn);

    let session = create_test_session(&project_id, Some("Shared Connection"));

    let result = repo.create(session.clone()).await;
    assert!(result.is_ok());

    let found = repo.get_by_id(&session.id).await.unwrap();
    assert!(found.is_some());
}

// ==================== GET CHILDREN TESTS ====================

#[tokio::test]
async fn test_get_children_returns_all_direct_children() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);

    let parent = create_test_session(&project_id, Some("Parent"));
    let mut child1 = create_test_session(&project_id, Some("Child 1"));
    child1.parent_session_id = Some(parent.id.clone());
    let mut child2 = create_test_session(&project_id, Some("Child 2"));
    child2.parent_session_id = Some(parent.id.clone());

    repo.create(parent.clone()).await.unwrap();
    repo.create(child1.clone()).await.unwrap();
    repo.create(child2.clone()).await.unwrap();

    let children = repo.get_children(&parent.id).await.unwrap();
    assert_eq!(children.len(), 2);
}

#[tokio::test]
async fn test_get_children_returns_empty_for_sessions_without_children() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);

    let session = create_test_session(&project_id, Some("No Children"));
    repo.create(session.clone()).await.unwrap();

    let children = repo.get_children(&session.id).await.unwrap();
    assert!(children.is_empty());
}

// ==================== GET ANCESTOR CHAIN TESTS ====================

#[tokio::test]
async fn test_get_ancestor_chain_three_levels_deep() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);

    let level1 = create_test_session(&project_id, Some("Level 1"));
    let mut level2 = create_test_session(&project_id, Some("Level 2"));
    level2.parent_session_id = Some(level1.id.clone());
    let mut level3 = create_test_session(&project_id, Some("Level 3"));
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
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);

    let parent = create_test_session(&project_id, Some("Parent"));
    let mut child = create_test_session(&project_id, Some("Child"));
    child.parent_session_id = Some(parent.id.clone());

    repo.create(parent.clone()).await.unwrap();
    repo.create(child.clone()).await.unwrap();

    let chain = repo.get_ancestor_chain(&child.id).await.unwrap();
    assert_eq!(chain.len(), 1);
    assert_eq!(chain[0].id, parent.id);
}

#[tokio::test]
async fn test_get_ancestor_chain_no_parent() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);

    let session = create_test_session(&project_id, Some("Root Session"));
    repo.create(session.clone()).await.unwrap();

    let chain = repo.get_ancestor_chain(&session.id).await.unwrap();
    assert!(chain.is_empty());
}

// ==================== SET PARENT TESTS ====================

#[tokio::test]
async fn test_set_parent_establishes_parent_child_relationship() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);

    let parent = create_test_session(&project_id, Some("Parent"));
    let child = create_test_session(&project_id, Some("Child"));

    repo.create(parent.clone()).await.unwrap();
    repo.create(child.clone()).await.unwrap();

    repo.set_parent(&child.id, Some(&parent.id)).await.unwrap();

    let updated_child = repo.get_by_id(&child.id).await.unwrap().unwrap();
    assert_eq!(updated_child.parent_session_id, Some(parent.id.clone()));
}

#[tokio::test]
async fn test_set_parent_with_null_clears_parent() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);

    let parent = create_test_session(&project_id, Some("Parent"));
    let mut child = create_test_session(&project_id, Some("Child"));
    child.parent_session_id = Some(parent.id.clone());

    repo.create(parent.clone()).await.unwrap();
    repo.create(child.clone()).await.unwrap();

    // Clear the parent
    repo.set_parent(&child.id, None).await.unwrap();

    let updated_child = repo.get_by_id(&child.id).await.unwrap().unwrap();
    assert!(updated_child.parent_session_id.is_none());
}

#[tokio::test]
async fn test_set_parent_updates_updated_at() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);

    let parent = create_test_session(&project_id, Some("Parent"));
    let child = create_test_session(&project_id, Some("Child"));
    let original_updated_at = child.updated_at;

    repo.create(parent.clone()).await.unwrap();
    repo.create(child.clone()).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    repo.set_parent(&child.id, Some(&parent.id)).await.unwrap();

    let updated_child = repo.get_by_id(&child.id).await.unwrap().unwrap();
    assert!(updated_child.updated_at >= original_updated_at);
}

// ==================== UPDATE_PLAN_ARTIFACT_ID TESTS ====================

use crate::domain::entities::ArtifactId;

#[tokio::test]
async fn test_update_plan_artifact_id_sets_value() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);
    let session = create_test_session(&project_id, Some("Plan Session"));
    repo.create(session.clone()).await.unwrap();

    repo.update_plan_artifact_id(&session.id, Some("artifact-abc".to_string()))
        .await
        .unwrap();

    let found = repo.get_by_id(&session.id).await.unwrap().unwrap();
    assert_eq!(
        found.plan_artifact_id,
        Some(ArtifactId::from_string("artifact-abc"))
    );
}

#[tokio::test]
async fn test_update_plan_artifact_id_clears_value() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);
    let session = create_test_session(&project_id, Some("Plan Session"));
    repo.create(session.clone()).await.unwrap();

    // Set then clear
    repo.update_plan_artifact_id(&session.id, Some("artifact-abc".to_string()))
        .await
        .unwrap();
    repo.update_plan_artifact_id(&session.id, None)
        .await
        .unwrap();

    let found = repo.get_by_id(&session.id).await.unwrap().unwrap();
    assert!(found.plan_artifact_id.is_none());
}

// ==================== GET_BY_PLAN_ARTIFACT_ID TESTS ====================

#[tokio::test]
async fn test_get_by_plan_artifact_id_returns_matching_sessions() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);
    let session1 = create_test_session(&project_id, Some("Session 1"));
    let session2 = create_test_session(&project_id, Some("Session 2"));
    let session3 = create_test_session(&project_id, Some("Session 3 Different Artifact"));
    repo.create(session1.clone()).await.unwrap();
    repo.create(session2.clone()).await.unwrap();
    repo.create(session3.clone()).await.unwrap();

    repo.update_plan_artifact_id(&session1.id, Some("plan-artifact-xyz".to_string()))
        .await
        .unwrap();
    repo.update_plan_artifact_id(&session3.id, Some("plan-artifact-other".to_string()))
        .await
        .unwrap();

    let results = repo
        .get_by_plan_artifact_id("plan-artifact-xyz")
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, session1.id);
}

#[tokio::test]
async fn test_get_by_plan_artifact_id_returns_empty_when_no_match() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);
    let session = create_test_session(&project_id, Some("Session"));
    repo.create(session).await.unwrap();

    let results = repo
        .get_by_plan_artifact_id("nonexistent-artifact")
        .await
        .unwrap();
    assert!(results.is_empty());
}

// ==================== VERIFICATION STATE TESTS ====================

#[tokio::test]
async fn test_update_verification_state_roundtrip() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);
    let session = create_test_session(&project_id, Some("Verify Session"));
    repo.create(session.clone()).await.unwrap();

    // Default state
    let (status, in_progress, _) = repo
        .get_verification_status(&session.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(status, VerificationStatus::Unverified);
    assert!(!in_progress);

    // Update to reviewing + in_progress
    let metadata = Some(r#"{"v":1,"current_round":1,"max_rounds":5}"#.to_string());
    repo.update_verification_state(
        &session.id,
        VerificationStatus::Reviewing,
        true,
        metadata.clone(),
    )
    .await
    .unwrap();

    let found = repo.get_by_id(&session.id).await.unwrap().unwrap();
    assert_eq!(found.verification_status, VerificationStatus::Reviewing);
    assert!(found.verification_in_progress);
    assert_eq!(found.verification_metadata, metadata);

    let (status2, in_progress2, _) = repo
        .get_verification_status(&session.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(status2, VerificationStatus::Reviewing);
    assert!(in_progress2);
}

#[tokio::test]
async fn test_update_verification_state_all_status_variants() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);
    let session = create_test_session(&project_id, Some("All Statuses"));
    repo.create(session.clone()).await.unwrap();

    for status in [
        VerificationStatus::Reviewing,
        VerificationStatus::NeedsRevision,
        VerificationStatus::Verified,
        VerificationStatus::Skipped,
        VerificationStatus::Unverified,
    ] {
        repo.update_verification_state(&session.id, status, false, None)
            .await
            .unwrap();
        let (s, _, _) = repo
            .get_verification_status(&session.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(s, status);
    }
}

#[tokio::test]
async fn test_reset_verification_clears_all_3_columns_when_not_in_progress() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);
    let session = create_test_session(&project_id, Some("Reset Session"));
    repo.create(session.clone()).await.unwrap();

    // Set to needs_revision, not in progress
    repo.update_verification_state(
        &session.id,
        VerificationStatus::NeedsRevision,
        false,
        Some(r#"{"v":1}"#.to_string()),
    )
    .await
    .unwrap();

    // Reset should clear all 3 columns and return true
    let reset = repo.reset_verification(&session.id).await.unwrap();
    assert!(reset, "reset_verification must return true when in_progress=0");

    let found = repo.get_by_id(&session.id).await.unwrap().unwrap();
    assert_eq!(found.verification_status, VerificationStatus::Unverified);
    assert!(!found.verification_in_progress);
    assert!(found.verification_metadata.is_none());
}

#[tokio::test]
async fn test_reset_verification_is_noop_when_in_progress() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test Project", "/test/path");

    let repo = SqliteIdeationSessionRepository::new(conn);
    let session = create_test_session(&project_id, Some("In Progress Session"));
    repo.create(session.clone()).await.unwrap();

    let metadata = Some(r#"{"v":1,"current_round":3}"#.to_string());

    // Set to reviewing with in_progress = true
    repo.update_verification_state(
        &session.id,
        VerificationStatus::Reviewing,
        true,
        metadata.clone(),
    )
    .await
    .unwrap();

    // Reset should be a no-op because in_progress = 1 and return false
    let reset = repo.reset_verification(&session.id).await.unwrap();
    assert!(!reset, "reset_verification must return false when in_progress=1");

    // All 3 columns should remain unchanged
    let found = repo.get_by_id(&session.id).await.unwrap().unwrap();
    assert_eq!(found.verification_status, VerificationStatus::Reviewing);
    assert!(found.verification_in_progress);
    assert_eq!(found.verification_metadata, metadata);
}

#[tokio::test]
async fn test_reset_verification_returns_false_for_nonexistent_session() {
    let conn = setup_test_db();
    let repo = SqliteIdeationSessionRepository::new(conn);
    let fake_id = IdeationSessionId::new();

    let reset = repo.reset_verification(&fake_id).await.unwrap();
    assert!(!reset, "reset_verification must return false for nonexistent session");
}

#[tokio::test]
async fn test_get_verification_status_returns_none_for_nonexistent_session() {
    let conn = setup_test_db();
    let repo = SqliteIdeationSessionRepository::new(conn);
    let id = IdeationSessionId::new();

    let result = repo.get_verification_status(&id).await.unwrap();
    assert!(result.is_none());
}

// ==================== CIRCULAR IMPORT VALIDATION TESTS ====================

fn create_test_session_with_source(
    conn: &Connection,
    project_id: &ProjectId,
    source_session_id: Option<&str>,
    source_project_id: Option<&str>,
) -> IdeationSession {
    let mut builder = IdeationSession::builder()
        .project_id(project_id.clone())
        .verification_status(VerificationStatus::Verified);

    if let Some(sid) = source_session_id {
        builder = builder.source_session_id(sid.to_string());
    }
    if let Some(pid) = source_project_id {
        builder = builder.source_project_id(pid.to_string());
    }

    let session = builder.build();
    SqliteIdeationSessionRepository::insert_sync(conn, &session).unwrap();
    session
}

#[test]
fn test_validate_no_circular_import_happy_path() {
    let conn = setup_test_db();
    let source_project_id = ProjectId::new();
    let target_project_id = ProjectId::new();
    create_test_project(&conn, &source_project_id, "Source", "/source");
    create_test_project(&conn, &target_project_id, "Target", "/target");

    let source = create_test_session_with_source(&conn, &source_project_id, None, None);

    let result = SqliteIdeationSessionRepository::validate_no_circular_import_sync(
        &conn,
        source.id.as_str(),
        target_project_id.as_str(),
        10,
    );

    assert!(result.is_ok(), "Simple cross-project import should be allowed");
}

#[test]
fn test_validate_no_circular_import_self_reference() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Project", "/project");

    let session = create_test_session_with_source(&conn, &project_id, None, None);

    // Trying to import from the same project (self-reference)
    let result = SqliteIdeationSessionRepository::validate_no_circular_import_sync(
        &conn,
        session.id.as_str(),
        project_id.as_str(), // target == source project
        10,
    );

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("SELF_REFERENCE"),
        "Expected SELF_REFERENCE error, got: {err_msg}"
    );
}

#[test]
fn test_validate_no_circular_import_a_to_b_to_a() {
    let conn = setup_test_db();
    let project_a = ProjectId::new();
    let project_b = ProjectId::new();
    create_test_project(&conn, &project_a, "Project A", "/project-a");
    create_test_project(&conn, &project_b, "Project B", "/project-b");

    // Session in B that was imported from A
    let _session_b =
        create_test_session_with_source(&conn, &project_b, None, Some(project_a.as_str()));

    // Now trying to import from session_b into project_a would create A→B→A
    // session_b is in project_b, which is NOT project_a, so no SELF_REFERENCE.
    // But session_b.source_project_id == project_a → CIRCULAR_IMPORT.
    // Note: validate_no_circular_import walks session_b.source_session_id (which is None here).
    // The cycle is detected via project membership checks.
    // Source session_b.project_id = project_b ≠ project_a → ok on first check.
    // session_b.source_session_id = None → chain ends.
    // No cycle detected at the session-chain level (because source_project_id is not walked).
    //
    // Actually: the CIRCULAR_IMPORT detection works when session_b has a source_session_id
    // pointing to a session in project_a. Let's set up a proper 2-hop cycle.

    // Create a session in project_a (the "original") with no parent
    let session_a_original = create_test_session_with_source(&conn, &project_a, None, None);

    // session_b2 was imported from session_a_original
    let session_b2 = create_test_session_with_source(
        &conn,
        &project_b,
        Some(session_a_original.id.as_str()),
        Some(project_a.as_str()),
    );

    // Now project_a tries to import from session_b2 (which itself came from project_a)
    // Walk: session_b2.source_session_id = session_a_original, which is in project_a = target
    let result = SqliteIdeationSessionRepository::validate_no_circular_import_sync(
        &conn,
        session_b2.id.as_str(),
        project_a.as_str(),
        10,
    );

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("CIRCULAR_IMPORT"),
        "Expected CIRCULAR_IMPORT error for A→B→A cycle, got: {err_msg}"
    );
}

#[test]
fn test_validate_no_circular_import_three_hop_cycle() {
    let conn = setup_test_db();
    let project_a = ProjectId::new();
    let project_b = ProjectId::new();
    let project_c = ProjectId::new();
    create_test_project(&conn, &project_a, "A", "/a");
    create_test_project(&conn, &project_b, "B", "/b");
    create_test_project(&conn, &project_c, "C", "/c");

    // session_a in A
    let session_a = create_test_session_with_source(&conn, &project_a, None, None);
    // session_b in B, imported from session_a
    let session_b = create_test_session_with_source(
        &conn,
        &project_b,
        Some(session_a.id.as_str()),
        Some(project_a.as_str()),
    );
    // session_c in C, imported from session_b
    let session_c = create_test_session_with_source(
        &conn,
        &project_c,
        Some(session_b.id.as_str()),
        Some(project_b.as_str()),
    );

    // Now A tries to import from session_c: A→C→B→A (3-hop)
    // Walk: session_c.source_session_id = session_b (project_b ≠ project_a → ok)
    //       session_b.source_session_id = session_a (project_a == target → CIRCULAR_IMPORT)
    let result = SqliteIdeationSessionRepository::validate_no_circular_import_sync(
        &conn,
        session_c.id.as_str(),
        project_a.as_str(),
        10,
    );

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("CIRCULAR_IMPORT"),
        "Expected CIRCULAR_IMPORT for 3-hop cycle, got: {err_msg}"
    );
}

#[test]
fn test_validate_no_circular_import_depth_limit_9_ok() {
    let conn = setup_test_db();

    // Build a chain of 9 projects: P1 ← P2 ← P3 ← … ← P9 ← P10
    // Then P10 tries to import from the end of the chain (depth = 9 hops): should PASS
    let projects: Vec<ProjectId> = (0..10).map(|_| ProjectId::new()).collect();
    for (i, pid) in projects.iter().enumerate() {
        create_test_project(&conn, pid, &format!("P{i}"), &format!("/p{i}"));
    }

    // Create sessions: each session points to the previous project's session
    let session_0 = create_test_session_with_source(&conn, &projects[0], None, None);
    let mut prev_session = session_0;
    for i in 1..9 {
        let s = create_test_session_with_source(
            &conn,
            &projects[i],
            Some(prev_session.id.as_str()),
            Some(projects[i - 1].as_str()),
        );
        prev_session = s;
    }

    // session at depth 9 (prev_session), target = projects[9]
    // Walk depth 9 (should succeed since max is 10)
    let result = SqliteIdeationSessionRepository::validate_no_circular_import_sync(
        &conn,
        prev_session.id.as_str(),
        projects[9].as_str(),
        10,
    );

    assert!(result.is_ok(), "9-hop chain should be within depth limit of 10");
}

#[test]
fn test_validate_no_circular_import_depth_limit_exceeded() {
    let conn = setup_test_db();

    // Build a chain of 11 projects so the walk exceeds depth 10
    let projects: Vec<ProjectId> = (0..12).map(|_| ProjectId::new()).collect();
    for (i, pid) in projects.iter().enumerate() {
        create_test_project(&conn, pid, &format!("P{i}"), &format!("/p{i}"));
    }

    let session_0 = create_test_session_with_source(&conn, &projects[0], None, None);
    let mut prev_session = session_0;
    for i in 1..11 {
        let s = create_test_session_with_source(
            &conn,
            &projects[i],
            Some(prev_session.id.as_str()),
            Some(projects[i - 1].as_str()),
        );
        prev_session = s;
    }

    // Session at depth 11, target = projects[11] (no cycle, just too deep)
    let result = SqliteIdeationSessionRepository::validate_no_circular_import_sync(
        &conn,
        prev_session.id.as_str(),
        projects[11].as_str(),
        10,
    );

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("CHAIN_TOO_DEEP"),
        "Expected CHAIN_TOO_DEEP error, got: {err_msg}"
    );
}

#[test]
fn test_validate_no_circular_import_dangling_source_is_ok() {
    let conn = setup_test_db();
    let source_project = ProjectId::new();
    let target_project = ProjectId::new();
    create_test_project(&conn, &source_project, "Source", "/source");
    create_test_project(&conn, &target_project, "Target", "/target");

    // Source session points to a non-existent (deleted) session — dangling reference
    let nonexistent_id = IdeationSessionId::new();
    let source = create_test_session_with_source(
        &conn,
        &source_project,
        Some(nonexistent_id.as_str()),
        None,
    );

    let result = SqliteIdeationSessionRepository::validate_no_circular_import_sync(
        &conn,
        source.id.as_str(),
        target_project.as_str(),
        10,
    );

    assert!(
        result.is_ok(),
        "Dangling source reference should be handled gracefully"
    );
}

#[test]
fn test_insert_sync_and_get_by_id_sync() {
    let conn = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&conn, &project_id, "Test", "/test");

    let session = IdeationSession::builder()
        .project_id(project_id.clone())
        .title("Cross-project session")
        .verification_status(VerificationStatus::ImportedVerified)
        .source_project_id("source-proj-123")
        .source_session_id("source-sess-456")
        .build();

    let inserted = SqliteIdeationSessionRepository::insert_sync(&conn, &session).unwrap();
    assert_eq!(inserted.id, session.id);
    assert_eq!(inserted.verification_status, VerificationStatus::ImportedVerified);
    assert_eq!(inserted.source_project_id, Some("source-proj-123".to_string()));
    assert_eq!(inserted.source_session_id, Some("source-sess-456".to_string()));

    let fetched = SqliteIdeationSessionRepository::get_by_id_sync(&conn, session.id.as_str())
        .unwrap()
        .unwrap();
    assert_eq!(fetched.verification_status, VerificationStatus::ImportedVerified);
    assert_eq!(fetched.source_session_id, Some("source-sess-456".to_string()));
}
