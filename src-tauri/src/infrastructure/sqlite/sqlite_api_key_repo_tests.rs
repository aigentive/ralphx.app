// Tests for SqliteApiKeyRepository
// Tests run against in-memory SQLite with full migrations

use std::sync::Arc;

use rusqlite::Connection;
use tokio::sync::Mutex;

use crate::domain::entities::{ApiKey, ApiKeyId, PERMISSION_ADMIN, PERMISSION_READ, PERMISSION_WRITE};
use crate::domain::repositories::ApiKeyRepository;
use crate::infrastructure::sqlite::{
    migrations::run_migrations,
    sqlite_api_key_repo::{generate_raw_key, hash_key, key_prefix, SqliteApiKeyRepository},
};

fn setup_repo() -> SqliteApiKeyRepository {
    let conn = Connection::open_in_memory().expect("in-memory DB");
    run_migrations(&conn).expect("migrations failed");
    SqliteApiKeyRepository::new(conn)
}

fn setup_repo_with_conn() -> (SqliteApiKeyRepository, Arc<Mutex<Connection>>) {
    let conn = Connection::open_in_memory().expect("in-memory DB");
    run_migrations(&conn).expect("migrations failed");
    let shared = Arc::new(Mutex::new(conn));
    let repo = SqliteApiKeyRepository::from_shared(Arc::clone(&shared));
    (repo, shared)
}

async fn insert_test_project(conn: &Arc<Mutex<Connection>>, project_id: &str) {
    conn.lock()
        .await
        .execute(
            "INSERT OR IGNORE INTO projects (id, name, working_directory, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                project_id,
                "test-project",
                "/tmp/test",
                "2026-01-01T00:00:00Z",
                "2026-01-01T00:00:00Z"
            ],
        )
        .expect("insert test project failed");
}

fn make_key(name: &str) -> ApiKey {
    let raw_key = generate_raw_key();
    ApiKey {
        id: ApiKeyId::new(),
        name: name.to_string(),
        key_hash: hash_key(&raw_key),
        key_prefix: key_prefix(&raw_key),
        permissions: 3,
        created_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        revoked_at: None,
        last_used_at: None,
        grace_expires_at: None,
        metadata: None,
    }
}

#[test]
fn test_generate_raw_key_format() {
    let key = generate_raw_key();
    assert!(key.starts_with("rxk_live_"), "key should start with rxk_live_");
    assert_eq!(key.len(), 41, "rxk_live_ (9) + 32 chars = 41");
}

#[test]
fn test_hash_key_deterministic() {
    let hash1 = hash_key("test_key_value");
    let hash2 = hash_key("test_key_value");
    assert_eq!(hash1, hash2, "same input must produce same hash");
    assert_ne!(hash_key("a"), hash_key("b"), "different inputs must produce different hashes");
}

#[test]
fn test_key_prefix_length() {
    let raw = "rxk_live_abcdefgh12345";
    let prefix = key_prefix(raw);
    assert_eq!(prefix, "rxk_live_abc", "prefix should be first 12 chars");
    assert_eq!(prefix.len(), 12);
}

#[test]
fn test_permissions_bitmask() {
    let key = ApiKey {
        id: ApiKeyId::new(),
        name: "test".to_string(),
        key_hash: "x".to_string(),
        key_prefix: "x".to_string(),
        permissions: 5, // read(1) + admin(4)
        created_at: "2026-01-01T00:00:00Z".to_string(),
        revoked_at: None,
        last_used_at: None,
        grace_expires_at: None,
        metadata: None,
    };
    assert!(key.has_permission(PERMISSION_READ), "should have read");
    assert!(!key.has_permission(PERMISSION_WRITE), "should NOT have write");
    assert!(key.has_permission(PERMISSION_ADMIN), "should have admin");
}

#[test]
fn test_is_in_grace_period_true() {
    let future = (chrono::Utc::now() + chrono::Duration::seconds(60))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let key = ApiKey {
        id: ApiKeyId::new(),
        name: "test".to_string(),
        key_hash: "x".to_string(),
        key_prefix: "x".to_string(),
        permissions: 3,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        revoked_at: Some("2026-01-01T00:00:01Z".to_string()),
        last_used_at: None,
        grace_expires_at: Some(future),
        metadata: None,
    };
    assert!(key.is_in_grace_period(), "key within grace period should return true");
}

#[test]
fn test_is_in_grace_period_false_expired() {
    let past = "2020-01-01T00:00:00Z".to_string();
    let key = ApiKey {
        id: ApiKeyId::new(),
        name: "test".to_string(),
        key_hash: "x".to_string(),
        key_prefix: "x".to_string(),
        permissions: 3,
        created_at: "2020-01-01T00:00:00Z".to_string(),
        revoked_at: Some("2020-01-01T00:00:01Z".to_string()),
        last_used_at: None,
        grace_expires_at: Some(past),
        metadata: None,
    };
    assert!(!key.is_in_grace_period(), "expired grace period should return false");
}

#[tokio::test]
async fn test_create_and_get_by_id() {
    let repo = setup_repo();
    let key = make_key("Test Key");
    let id = key.id.clone();

    let created = repo.create(key).await.expect("create failed");
    let found = repo.get_by_id(&id).await.expect("get_by_id failed").expect("not found");

    assert_eq!(found.id, id);
    assert_eq!(found.name, "Test Key");
    assert_eq!(found.permissions, 3);
    assert!(found.revoked_at.is_none());
    assert!(found.last_used_at.is_none());
    assert_eq!(found.key_prefix, created.key_prefix);
}

#[tokio::test]
async fn test_get_by_id_missing_returns_none() {
    let repo = setup_repo();
    let missing_id = ApiKeyId::from_string("nonexistent-id");
    let result = repo.get_by_id(&missing_id).await.expect("get should succeed");
    assert!(result.is_none());
}

#[tokio::test]
async fn test_get_by_hash() {
    let repo = setup_repo();
    let raw = generate_raw_key();
    let hash = hash_key(&raw);
    let prefix = key_prefix(&raw);

    let key = ApiKey {
        id: ApiKeyId::new(),
        name: "Hash Test".to_string(),
        key_hash: hash.clone(),
        key_prefix: prefix,
        permissions: 3,
        created_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        revoked_at: None,
        last_used_at: None,
        grace_expires_at: None,
        metadata: None,
    };

    repo.create(key).await.expect("create failed");
    let found = repo.get_by_hash(&hash).await.expect("get_by_hash failed").expect("not found");
    assert_eq!(found.key_hash, hash);
    assert_eq!(found.name, "Hash Test");
}

#[tokio::test]
async fn test_get_by_hash_missing_returns_none() {
    let repo = setup_repo();
    let result = repo.get_by_hash("nonexistent_hash").await.expect("should succeed");
    assert!(result.is_none());
}

#[tokio::test]
async fn test_list_returns_all_keys() {
    let repo = setup_repo();
    repo.create(make_key("Key A")).await.expect("create A");
    repo.create(make_key("Key B")).await.expect("create B");
    repo.create(make_key("Key C")).await.expect("create C");

    let keys = repo.list().await.expect("list failed");
    assert_eq!(keys.len(), 3);
    let names: Vec<&str> = keys.iter().map(|k| k.name.as_str()).collect();
    assert!(names.contains(&"Key A"));
    assert!(names.contains(&"Key B"));
    assert!(names.contains(&"Key C"));
}

#[tokio::test]
async fn test_revoke_sets_revoked_at() {
    let repo = setup_repo();
    let key = make_key("Revoke Me");
    let id = key.id.clone();

    repo.create(key).await.expect("create");
    repo.revoke(&id).await.expect("revoke");

    let found = repo.get_by_id(&id).await.expect("get").expect("found");
    assert!(found.revoked_at.is_some(), "revoked_at must be set after revoke");
    assert!(!found.is_active(), "is_active should be false after revoke");
}

#[tokio::test]
async fn test_set_and_get_projects() {
    let (repo, conn) = setup_repo_with_conn();
    let key = make_key("Project Key");
    let id = key.id.clone();
    repo.create(key).await.expect("create");

    insert_test_project(&conn, "proj-1").await;
    insert_test_project(&conn, "proj-2").await;

    let project_ids = vec!["proj-1".to_string(), "proj-2".to_string()];
    repo.set_projects(&id, &project_ids).await.expect("set_projects");

    let mut found = repo.get_projects(&id).await.expect("get_projects");
    found.sort();
    let mut expected = project_ids.clone();
    expected.sort();
    assert_eq!(found, expected);
}

#[tokio::test]
async fn test_set_projects_replaces_existing() {
    let (repo, conn) = setup_repo_with_conn();
    let key = make_key("Replace Projects");
    let id = key.id.clone();
    repo.create(key).await.expect("create");

    insert_test_project(&conn, "old-proj").await;
    insert_test_project(&conn, "new-proj-1").await;
    insert_test_project(&conn, "new-proj-2").await;

    repo.set_projects(&id, &["old-proj".to_string()]).await.expect("first set");
    repo.set_projects(&id, &["new-proj-1".to_string(), "new-proj-2".to_string()])
        .await
        .expect("second set");

    let mut found = repo.get_projects(&id).await.expect("get_projects");
    found.sort();
    assert_eq!(found, vec!["new-proj-1".to_string(), "new-proj-2".to_string()]);
    assert!(!found.contains(&"old-proj".to_string()), "old project should be replaced");
}

#[tokio::test]
async fn test_get_projects_empty_for_new_key() {
    let repo = setup_repo();
    let key = make_key("No Projects");
    let id = key.id.clone();
    repo.create(key).await.expect("create");

    let projects = repo.get_projects(&id).await.expect("get_projects");
    assert!(projects.is_empty(), "new key should have no projects");
}

#[tokio::test]
async fn test_set_grace_period() {
    let repo = setup_repo();
    let key = make_key("Grace Key");
    let id = key.id.clone();
    repo.create(key).await.expect("create");

    let grace_time = (chrono::Utc::now() + chrono::Duration::seconds(60))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    repo.set_grace_period(&id, &grace_time).await.expect("set_grace_period");

    let found = repo.get_by_id(&id).await.expect("get").expect("found");
    assert!(found.grace_expires_at.is_some(), "grace_expires_at should be set");
    assert_eq!(found.grace_expires_at.unwrap(), grace_time);
}

#[tokio::test]
async fn test_update_last_used() {
    let repo = setup_repo();
    let key = make_key("Used Key");
    let id = key.id.clone();
    repo.create(key).await.expect("create");

    let ts = "2026-03-06T12:00:00Z".to_string();
    repo.update_last_used(&id, &ts).await.expect("update_last_used");

    let found = repo.get_by_id(&id).await.expect("get").expect("found");
    assert_eq!(found.last_used_at.as_deref(), Some("2026-03-06T12:00:00Z"));
}

#[tokio::test]
async fn test_log_audit_success() {
    let repo = setup_repo();
    let key = make_key("Audit Key");
    let id = key.id.clone();
    repo.create(key).await.expect("create");

    // Should not error
    repo.log_audit(id.as_str(), "validate_key", Some("proj-1"), true, Some(42))
        .await
        .expect("log_audit failed");

    // Log another entry
    repo.log_audit(id.as_str(), "list_tasks", None, false, None)
        .await
        .expect("second log_audit failed");
}
