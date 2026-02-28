// Tests for SqliteTeamSessionRepository

use super::sqlite_team_session_repo::SqliteTeamSessionRepository;
use crate::application::team_state_tracker::TeammateCost;
use crate::domain::entities::team::{TeamSession, TeamSessionId, TeammateSnapshot};
use crate::domain::repositories::TeamSessionRepository;
use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

fn setup_test_db() -> rusqlite::Connection {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    conn
}

fn make_teammate(name: &str) -> TeammateSnapshot {
    TeammateSnapshot {
        name: name.to_string(),
        color: "#ff6b35".to_string(),
        model: "sonnet".to_string(),
        role: "worker".to_string(),
        status: "active".to_string(),
        cost: TeammateCost::default(),
        spawned_at: "2024-01-01T00:00:00+00:00".to_string(),
        last_activity_at: "2024-01-01T00:00:00+00:00".to_string(),
        conversation_id: None,
    }
}

// ==================== CREATE TESTS ====================

#[tokio::test]
async fn test_create_returns_session() {
    let conn = setup_test_db();
    let repo = SqliteTeamSessionRepository::new(conn);

    let session = TeamSession::new("my-team", "ctx-1", "project");
    let id = session.id.clone();

    let result = repo.create(session).await;

    assert!(result.is_ok());
    let created = result.unwrap();
    assert_eq!(created.id, id);
    assert_eq!(created.team_name, "my-team");
    assert_eq!(created.context_id, "ctx-1");
    assert_eq!(created.context_type, "project");
}

#[tokio::test]
async fn test_create_duplicate_id_fails() {
    let conn = setup_test_db();
    let repo = SqliteTeamSessionRepository::new(conn);

    let session = TeamSession::new("team-a", "ctx-1", "project");
    let session2 = TeamSession {
        id: session.id.clone(),
        ..TeamSession::new("team-b", "ctx-2", "project")
    };

    repo.create(session).await.unwrap();
    let result = repo.create(session2).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_with_teammates_persists_json() {
    let conn = setup_test_db();
    let repo = SqliteTeamSessionRepository::new(conn);

    let mut session = TeamSession::new("team-x", "ctx-1", "project");
    session.teammates = vec![make_teammate("alice"), make_teammate("bob")];
    let id = session.id.clone();

    repo.create(session).await.unwrap();

    let fetched = repo.get_by_id(&id).await.unwrap().unwrap();
    assert_eq!(fetched.teammates.len(), 2);
    assert_eq!(fetched.teammates[0].name, "alice");
    assert_eq!(fetched.teammates[1].name, "bob");
}

#[tokio::test]
async fn test_create_with_empty_teammates_persists_empty() {
    let conn = setup_test_db();
    let repo = SqliteTeamSessionRepository::new(conn);

    let session = TeamSession::new("team-empty", "ctx-1", "project");
    let id = session.id.clone();

    repo.create(session).await.unwrap();

    let fetched = repo.get_by_id(&id).await.unwrap().unwrap();
    assert!(fetched.teammates.is_empty());
}

// ==================== GET BY ID TESTS ====================

#[tokio::test]
async fn test_get_by_id_returns_session() {
    let conn = setup_test_db();
    let repo = SqliteTeamSessionRepository::new(conn);

    let session = TeamSession::new("team-1", "ctx-1", "project");
    let id = session.id.clone();
    repo.create(session).await.unwrap();

    let result = repo.get_by_id(&id).await;

    assert!(result.is_ok());
    let found = result.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, id);
}

#[tokio::test]
async fn test_get_by_id_returns_none_for_nonexistent() {
    let conn = setup_test_db();
    let repo = SqliteTeamSessionRepository::new(conn);

    let id = TeamSessionId::new();
    let result = repo.get_by_id(&id).await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_get_by_id_preserves_all_fields() {
    let conn = setup_test_db();
    let repo = SqliteTeamSessionRepository::new(conn);

    let mut session = TeamSession::new("full-team", "ctx-full", "task");
    session.lead_name = Some("lead-agent".to_string());
    session.phase = "executing".to_string();
    session.teammates = vec![make_teammate("worker1")];
    let id = session.id.clone();

    repo.create(session).await.unwrap();

    let found = repo.get_by_id(&id).await.unwrap().unwrap();
    assert_eq!(found.team_name, "full-team");
    assert_eq!(found.context_type, "task");
    assert_eq!(found.lead_name, Some("lead-agent".to_string()));
    assert_eq!(found.phase, "executing");
    assert_eq!(found.teammates.len(), 1);
    assert_eq!(found.teammates[0].name, "worker1");
    assert!(found.disbanded_at.is_none());
}

// ==================== GET BY CONTEXT TESTS ====================

#[tokio::test]
async fn test_get_by_context_returns_matching_sessions() {
    let conn = setup_test_db();
    let repo = SqliteTeamSessionRepository::new(conn);

    let s1 = TeamSession::new("team-a", "ctx-1", "project");
    let s2 = TeamSession::new("team-b", "ctx-1", "project");
    let s3 = TeamSession::new("team-c", "ctx-2", "project");

    repo.create(s1).await.unwrap();
    repo.create(s2).await.unwrap();
    repo.create(s3).await.unwrap();

    let result = repo.get_by_context("project", "ctx-1").await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 2);
}

#[tokio::test]
async fn test_get_by_context_returns_empty_for_no_match() {
    let conn = setup_test_db();
    let repo = SqliteTeamSessionRepository::new(conn);

    let result = repo.get_by_context("project", "nonexistent").await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn test_get_by_context_filters_by_context_type() {
    let conn = setup_test_db();
    let repo = SqliteTeamSessionRepository::new(conn);

    let s1 = TeamSession::new("team-a", "ctx-1", "project");
    let s2 = TeamSession::new("team-b", "ctx-1", "task");

    repo.create(s1).await.unwrap();
    repo.create(s2).await.unwrap();

    let project_sessions = repo.get_by_context("project", "ctx-1").await.unwrap();
    let task_sessions = repo.get_by_context("task", "ctx-1").await.unwrap();

    assert_eq!(project_sessions.len(), 1);
    assert_eq!(task_sessions.len(), 1);
    assert_eq!(project_sessions[0].context_type, "project");
    assert_eq!(task_sessions[0].context_type, "task");
}

#[tokio::test]
async fn test_get_by_context_includes_disbanded_sessions() {
    let conn = setup_test_db();
    let repo = SqliteTeamSessionRepository::new(conn);

    let active = TeamSession::new("active-team", "ctx-1", "project");
    let disbanded = TeamSession::new("old-team", "ctx-1", "project");
    let disbanded_id = disbanded.id.clone();

    repo.create(active).await.unwrap();
    repo.create(disbanded).await.unwrap();
    repo.set_disbanded(&disbanded_id).await.unwrap();

    // get_by_context returns ALL sessions (active + disbanded)
    let sessions = repo.get_by_context("project", "ctx-1").await.unwrap();
    assert_eq!(sessions.len(), 2);
}

// ==================== GET ACTIVE FOR CONTEXT TESTS ====================

#[tokio::test]
async fn test_get_active_for_context_returns_non_disbanded_session() {
    let conn = setup_test_db();
    let repo = SqliteTeamSessionRepository::new(conn);

    let active = TeamSession::new("active-team", "ctx-1", "project");
    let active_id = active.id.clone();
    let disbanded = TeamSession::new("old-team", "ctx-1", "project");
    let disbanded_id = disbanded.id.clone();

    repo.create(active).await.unwrap();
    repo.create(disbanded).await.unwrap();
    repo.set_disbanded(&disbanded_id).await.unwrap();

    let result = repo.get_active_for_context("project", "ctx-1").await;

    assert!(result.is_ok());
    let found = result.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, active_id);
}

#[tokio::test]
async fn test_get_active_for_context_returns_none_when_all_disbanded() {
    let conn = setup_test_db();
    let repo = SqliteTeamSessionRepository::new(conn);

    let session = TeamSession::new("old-team", "ctx-1", "project");
    let id = session.id.clone();
    repo.create(session).await.unwrap();
    repo.set_disbanded(&id).await.unwrap();

    let result = repo.get_active_for_context("project", "ctx-1").await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_get_active_for_context_returns_none_when_empty() {
    let conn = setup_test_db();
    let repo = SqliteTeamSessionRepository::new(conn);

    let result = repo.get_active_for_context("project", "ctx-1").await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

// ==================== UPDATE PHASE TESTS ====================

#[tokio::test]
async fn test_update_phase_changes_phase() {
    let conn = setup_test_db();
    let repo = SqliteTeamSessionRepository::new(conn);

    let session = TeamSession::new("team-1", "ctx-1", "project");
    let id = session.id.clone();
    repo.create(session).await.unwrap();

    repo.update_phase(&id, "executing").await.unwrap();

    let found = repo.get_by_id(&id).await.unwrap().unwrap();
    assert_eq!(found.phase, "executing");
}

#[tokio::test]
async fn test_update_phase_does_not_affect_other_fields() {
    let conn = setup_test_db();
    let repo = SqliteTeamSessionRepository::new(conn);

    let mut session = TeamSession::new("team-1", "ctx-1", "project");
    session.lead_name = Some("leader".to_string());
    session.teammates = vec![make_teammate("alice")];
    let id = session.id.clone();
    repo.create(session).await.unwrap();

    repo.update_phase(&id, "done").await.unwrap();

    let found = repo.get_by_id(&id).await.unwrap().unwrap();
    assert_eq!(found.phase, "done");
    assert_eq!(found.lead_name, Some("leader".to_string()));
    assert_eq!(found.teammates.len(), 1);
}

// ==================== UPDATE TEAMMATES TESTS ====================

#[tokio::test]
async fn test_update_teammates_replaces_all() {
    let conn = setup_test_db();
    let repo = SqliteTeamSessionRepository::new(conn);

    let mut session = TeamSession::new("team-1", "ctx-1", "project");
    session.teammates = vec![make_teammate("alice")];
    let id = session.id.clone();
    repo.create(session).await.unwrap();

    let new_teammates = vec![make_teammate("bob"), make_teammate("charlie")];
    repo.update_teammates(&id, &new_teammates).await.unwrap();

    let found = repo.get_by_id(&id).await.unwrap().unwrap();
    assert_eq!(found.teammates.len(), 2);
    assert_eq!(found.teammates[0].name, "bob");
    assert_eq!(found.teammates[1].name, "charlie");
}

#[tokio::test]
async fn test_update_teammates_with_empty_clears_list() {
    let conn = setup_test_db();
    let repo = SqliteTeamSessionRepository::new(conn);

    let mut session = TeamSession::new("team-1", "ctx-1", "project");
    session.teammates = vec![make_teammate("alice")];
    let id = session.id.clone();
    repo.create(session).await.unwrap();

    repo.update_teammates(&id, &[]).await.unwrap();

    let found = repo.get_by_id(&id).await.unwrap().unwrap();
    assert!(found.teammates.is_empty());
}

#[tokio::test]
async fn test_update_teammates_preserves_teammate_cost() {
    let conn = setup_test_db();
    let repo = SqliteTeamSessionRepository::new(conn);

    let session = TeamSession::new("team-1", "ctx-1", "project");
    let id = session.id.clone();
    repo.create(session).await.unwrap();

    let mut teammate = make_teammate("worker");
    teammate.cost = TeammateCost {
        input_tokens: 1000,
        output_tokens: 500,
        cache_creation_tokens: 200,
        cache_read_tokens: 100,
        estimated_usd: 0.05,
    };
    repo.update_teammates(&id, &[teammate]).await.unwrap();

    let found = repo.get_by_id(&id).await.unwrap().unwrap();
    assert_eq!(found.teammates.len(), 1);
    assert_eq!(found.teammates[0].cost.input_tokens, 1000);
    assert_eq!(found.teammates[0].cost.output_tokens, 500);
    assert!((found.teammates[0].cost.estimated_usd - 0.05).abs() < 0.001);
}

// ==================== SET DISBANDED TESTS ====================

#[tokio::test]
async fn test_set_disbanded_sets_disbanded_at() {
    let conn = setup_test_db();
    let repo = SqliteTeamSessionRepository::new(conn);

    let session = TeamSession::new("team-1", "ctx-1", "project");
    let id = session.id.clone();
    repo.create(session).await.unwrap();

    let before = repo.get_by_id(&id).await.unwrap().unwrap();
    assert!(before.disbanded_at.is_none());

    repo.set_disbanded(&id).await.unwrap();

    let after = repo.get_by_id(&id).await.unwrap().unwrap();
    assert!(after.disbanded_at.is_some());
}

#[tokio::test]
async fn test_set_disbanded_does_not_affect_other_sessions() {
    let conn = setup_test_db();
    let repo = SqliteTeamSessionRepository::new(conn);

    let s1 = TeamSession::new("team-1", "ctx-1", "project");
    let s2 = TeamSession::new("team-2", "ctx-2", "project");
    let id1 = s1.id.clone();
    let id2 = s2.id.clone();

    repo.create(s1).await.unwrap();
    repo.create(s2).await.unwrap();
    repo.set_disbanded(&id1).await.unwrap();

    let found2 = repo.get_by_id(&id2).await.unwrap().unwrap();
    assert!(found2.disbanded_at.is_none());
}

// ==================== FROM SHARED TESTS ====================

#[tokio::test]
async fn test_from_shared_creates_and_retrieves() {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let conn = setup_test_db();
    let shared_conn = Arc::new(Mutex::new(conn));
    let repo = SqliteTeamSessionRepository::from_shared(shared_conn);

    let session = TeamSession::new("shared-team", "ctx-1", "project");
    let id = session.id.clone();

    repo.create(session).await.unwrap();
    let found = repo.get_by_id(&id).await.unwrap();
    assert!(found.is_some());
}
