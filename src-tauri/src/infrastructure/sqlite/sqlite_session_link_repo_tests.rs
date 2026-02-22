use super::*;
use crate::domain::entities::SessionRelationship;

fn create_test_link(parent_id: &IdeationSessionId, child_id: &IdeationSessionId) -> SessionLink {
    SessionLink::new(
        parent_id.clone(),
        child_id.clone(),
        SessionRelationship::FollowOn,
    )
}

#[tokio::test]
async fn test_create_link() {
    let conn = rusqlite::Connection::open_in_memory().expect("Failed to open in-memory database");

    // Set up the schema
    conn.execute_batch(
        "CREATE TABLE session_links (
            id TEXT PRIMARY KEY,
            parent_session_id TEXT NOT NULL,
            child_session_id TEXT NOT NULL,
            relationship TEXT NOT NULL,
            notes TEXT,
            created_at TEXT NOT NULL,
            CHECK (parent_session_id != child_session_id),
            UNIQUE(parent_session_id, child_session_id)
        );",
    )
    .expect("Failed to create table");

    let conn = Arc::new(Mutex::new(conn));
    let repo = SqliteSessionLinkRepository::from_shared(conn);

    let parent_id = IdeationSessionId::new();
    let child_id = IdeationSessionId::new();
    let link = create_test_link(&parent_id, &child_id);

    let result = repo.create(link.clone()).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().id, link.id);
}

#[tokio::test]
async fn test_get_by_parent() {
    let conn = rusqlite::Connection::open_in_memory().expect("Failed to open in-memory database");

    conn.execute_batch(
        "CREATE TABLE session_links (
            id TEXT PRIMARY KEY,
            parent_session_id TEXT NOT NULL,
            child_session_id TEXT NOT NULL,
            relationship TEXT NOT NULL,
            notes TEXT,
            created_at TEXT NOT NULL,
            CHECK (parent_session_id != child_session_id),
            UNIQUE(parent_session_id, child_session_id)
        );",
    )
    .expect("Failed to create table");

    let conn = Arc::new(Mutex::new(conn));
    let repo = SqliteSessionLinkRepository::from_shared(conn);

    let parent_id = IdeationSessionId::new();
    let child_id1 = IdeationSessionId::new();
    let child_id2 = IdeationSessionId::new();

    let link1 = create_test_link(&parent_id, &child_id1);
    let link2 = create_test_link(&parent_id, &child_id2);

    repo.create(link1.clone())
        .await
        .expect("Failed to create link1");
    repo.create(link2.clone())
        .await
        .expect("Failed to create link2");

    let result = repo.get_by_parent(&parent_id).await;
    assert!(result.is_ok());
    let links = result.unwrap();
    assert_eq!(links.len(), 2);
}

#[tokio::test]
async fn test_get_by_child() {
    let conn = rusqlite::Connection::open_in_memory().expect("Failed to open in-memory database");

    conn.execute_batch(
        "CREATE TABLE session_links (
            id TEXT PRIMARY KEY,
            parent_session_id TEXT NOT NULL,
            child_session_id TEXT NOT NULL,
            relationship TEXT NOT NULL,
            notes TEXT,
            created_at TEXT NOT NULL,
            CHECK (parent_session_id != child_session_id),
            UNIQUE(parent_session_id, child_session_id)
        );",
    )
    .expect("Failed to create table");

    let conn = Arc::new(Mutex::new(conn));
    let repo = SqliteSessionLinkRepository::from_shared(conn);

    let parent_id1 = IdeationSessionId::new();
    let parent_id2 = IdeationSessionId::new();
    let child_id = IdeationSessionId::new();

    let link1 = create_test_link(&parent_id1, &child_id);
    let link2 = create_test_link(&parent_id2, &child_id);

    repo.create(link1.clone())
        .await
        .expect("Failed to create link1");
    repo.create(link2.clone())
        .await
        .expect("Failed to create link2");

    let result = repo.get_by_child(&child_id).await;
    assert!(result.is_ok());
    let links = result.unwrap();
    assert_eq!(links.len(), 2);
}

#[tokio::test]
async fn test_delete() {
    let conn = rusqlite::Connection::open_in_memory().expect("Failed to open in-memory database");

    conn.execute_batch(
        "CREATE TABLE session_links (
            id TEXT PRIMARY KEY,
            parent_session_id TEXT NOT NULL,
            child_session_id TEXT NOT NULL,
            relationship TEXT NOT NULL,
            notes TEXT,
            created_at TEXT NOT NULL,
            CHECK (parent_session_id != child_session_id),
            UNIQUE(parent_session_id, child_session_id)
        );",
    )
    .expect("Failed to create table");

    let conn = Arc::new(Mutex::new(conn));
    let repo = SqliteSessionLinkRepository::from_shared(conn);

    let parent_id = IdeationSessionId::new();
    let child_id = IdeationSessionId::new();
    let link = create_test_link(&parent_id, &child_id);

    repo.create(link.clone())
        .await
        .expect("Failed to create link");

    // Verify it exists
    let links = repo
        .get_by_parent(&parent_id)
        .await
        .expect("Failed to query");
    assert_eq!(links.len(), 1);

    // Delete it
    let result = repo.delete(&link.id).await;
    assert!(result.is_ok());

    // Verify it's gone
    let links = repo
        .get_by_parent(&parent_id)
        .await
        .expect("Failed to query");
    assert_eq!(links.len(), 0);
}

#[tokio::test]
async fn test_delete_by_child() {
    let conn = rusqlite::Connection::open_in_memory().expect("Failed to open in-memory database");

    conn.execute_batch(
        "CREATE TABLE session_links (
            id TEXT PRIMARY KEY,
            parent_session_id TEXT NOT NULL,
            child_session_id TEXT NOT NULL,
            relationship TEXT NOT NULL,
            notes TEXT,
            created_at TEXT NOT NULL,
            CHECK (parent_session_id != child_session_id),
            UNIQUE(parent_session_id, child_session_id)
        );",
    )
    .expect("Failed to create table");

    let conn = Arc::new(Mutex::new(conn));
    let repo = SqliteSessionLinkRepository::from_shared(conn);

    let parent_id1 = IdeationSessionId::new();
    let parent_id2 = IdeationSessionId::new();
    let child_id = IdeationSessionId::new();

    let link1 = create_test_link(&parent_id1, &child_id);
    let link2 = create_test_link(&parent_id2, &child_id);

    repo.create(link1.clone())
        .await
        .expect("Failed to create link1");
    repo.create(link2.clone())
        .await
        .expect("Failed to create link2");

    // Verify both exist
    let links = repo.get_by_child(&child_id).await.expect("Failed to query");
    assert_eq!(links.len(), 2);

    // Delete all links where child is child_id
    let result = repo.delete_by_child(&child_id).await;
    assert!(result.is_ok());

    // Verify all are gone
    let links = repo.get_by_child(&child_id).await.expect("Failed to query");
    assert_eq!(links.len(), 0);
}
