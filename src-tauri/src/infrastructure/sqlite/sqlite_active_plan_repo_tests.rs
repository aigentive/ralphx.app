use super::*;
use crate::domain::entities::{IdeationSession, Project};
use crate::testing::SqliteTestDb;

fn setup_repo() -> (SqliteTestDb, SqliteActivePlanRepository) {
    let db = SqliteTestDb::new("sqlite-active-plan-repo");
    let repo = SqliteActivePlanRepository::from_shared(db.shared_conn());
    (db, repo)
}

fn seed_accepted_session(db: &SqliteTestDb, project_id: ProjectId) -> IdeationSession {
    let mut session = IdeationSession::new(project_id);
    session.mark_accepted();
    db.insert_ideation_session(session)
}

fn seed_project_with_accepted_session(db: &SqliteTestDb) -> (Project, IdeationSession) {
    let project = db.seed_project("Test Project");
    let session = seed_accepted_session(db, project.id.clone());
    (project, session)
}

#[tokio::test]
async fn test_get_returns_none_when_no_active_plan() {
    let (db, repo) = setup_repo();
    let (project, _session) = seed_project_with_accepted_session(&db);

    let result = repo.get(&project.id).await.unwrap();

    assert!(result.is_none());
}

#[tokio::test]
async fn test_set_and_get_active_plan() {
    let (db, repo) = setup_repo();
    let (project, session) = seed_project_with_accepted_session(&db);

    repo.set(&project.id, &session.id).await.unwrap();

    let result = repo.get(&project.id).await.unwrap();
    assert_eq!(result, Some(session.id));
}

#[tokio::test]
async fn test_set_updates_existing_active_plan() {
    let (db, repo) = setup_repo();
    let project = db.seed_project("Test Project");
    let session1 = seed_accepted_session(&db, project.id.clone());
    let session2 = seed_accepted_session(&db, project.id.clone());

    repo.set(&project.id, &session1.id).await.unwrap();
    repo.set(&project.id, &session2.id).await.unwrap();

    let result = repo.get(&project.id).await.unwrap();
    assert_eq!(result, Some(session2.id));
}

#[tokio::test]
async fn test_set_rejects_non_accepted_session() {
    let (db, repo) = setup_repo();
    let project = db.seed_project("Test Project");
    let session = db.seed_ideation_session(project.id.clone());

    let result = repo.set(&project.id, &session.id).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("must be accepted"));
}

#[tokio::test]
async fn test_set_rejects_session_from_different_project() {
    let (db, repo) = setup_repo();
    let project1 = db.seed_project("Test Project 1");
    let project2 = db.seed_project("Test Project 2");
    let session = seed_accepted_session(&db, project1.id.clone());

    let result = repo.set(&project2.id, &session.id).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_clear_removes_active_plan() {
    let (db, repo) = setup_repo();
    let (project, session) = seed_project_with_accepted_session(&db);

    repo.set(&project.id, &session.id).await.unwrap();
    repo.clear(&project.id).await.unwrap();

    let result = repo.get(&project.id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_exists_returns_false_when_no_active_plan() {
    let (db, repo) = setup_repo();
    let (project, _session) = seed_project_with_accepted_session(&db);

    let exists = repo.exists(&project.id).await.unwrap();

    assert!(!exists);
}

#[tokio::test]
async fn test_exists_returns_true_when_active_plan_set() {
    let (db, repo) = setup_repo();
    let (project, session) = seed_project_with_accepted_session(&db);

    repo.set(&project.id, &session.id).await.unwrap();

    let exists = repo.exists(&project.id).await.unwrap();
    assert!(exists);
}

#[tokio::test]
async fn test_cascade_delete_when_session_deleted() {
    let (db, repo) = setup_repo();
    let (project, session) = seed_project_with_accepted_session(&db);

    repo.set(&project.id, &session.id).await.unwrap();
    db.with_connection(|conn| {
        conn.execute(
            "DELETE FROM ideation_sessions WHERE id = ?1",
            [session.id.as_str()],
        )
        .unwrap();
    });

    let result = repo.get(&project.id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_cascade_delete_when_project_deleted() {
    let (db, repo) = setup_repo();
    let (project, session) = seed_project_with_accepted_session(&db);

    repo.set(&project.id, &session.id).await.unwrap();
    db.with_connection(|conn| {
        conn.execute("DELETE FROM projects WHERE id = ?1", [project.id.as_str()])
            .unwrap();
    });

    let result = repo.get(&project.id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_record_selection_creates_new_stats() {
    let (db, repo) = setup_repo();
    let (project, session) = seed_project_with_accepted_session(&db);

    repo.record_selection(&project.id, &session.id, "kanban_inline")
        .await
        .unwrap();

    let (count, source): (u32, String) = db
        .with_connection(|conn| {
            conn.query_row(
                "SELECT selected_count, last_selected_source FROM plan_selection_stats WHERE project_id = ?1 AND ideation_session_id = ?2",
                [project.id.as_str(), session.id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
        })
        .unwrap();

    assert_eq!(count, 1);
    assert_eq!(source, "kanban_inline");
}

#[tokio::test]
async fn test_record_selection_increments_count() {
    let (db, repo) = setup_repo();
    let (project, session) = seed_project_with_accepted_session(&db);

    repo.record_selection(&project.id, &session.id, "kanban_inline")
        .await
        .unwrap();
    repo.record_selection(&project.id, &session.id, "graph_inline")
        .await
        .unwrap();
    repo.record_selection(&project.id, &session.id, "quick_switcher")
        .await
        .unwrap();

    let (count, source): (u32, String) = db
        .with_connection(|conn| {
            conn.query_row(
                "SELECT selected_count, last_selected_source FROM plan_selection_stats WHERE project_id = ?1 AND ideation_session_id = ?2",
                [project.id.as_str(), session.id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
        })
        .unwrap();

    assert_eq!(count, 3);
    assert_eq!(source, "quick_switcher");
}

#[tokio::test]
async fn test_record_selection_updates_timestamp() {
    let (db, repo) = setup_repo();
    let (project, session) = seed_project_with_accepted_session(&db);

    repo.record_selection(&project.id, &session.id, "kanban_inline")
        .await
        .unwrap();

    let first_timestamp: String = db
        .with_connection(|conn| {
            conn.query_row(
                "SELECT last_selected_at FROM plan_selection_stats WHERE project_id = ?1 AND ideation_session_id = ?2",
                [project.id.as_str(), session.id.as_str()],
                |row| row.get(0),
            )
        })
        .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    repo.record_selection(&project.id, &session.id, "graph_inline")
        .await
        .unwrap();

    let second_timestamp: String = db
        .with_connection(|conn| {
            conn.query_row(
                "SELECT last_selected_at FROM plan_selection_stats WHERE project_id = ?1 AND ideation_session_id = ?2",
                [project.id.as_str(), session.id.as_str()],
                |row| row.get(0),
            )
        })
        .unwrap();

    assert_ne!(first_timestamp, second_timestamp);
    assert!(second_timestamp > first_timestamp);
}
