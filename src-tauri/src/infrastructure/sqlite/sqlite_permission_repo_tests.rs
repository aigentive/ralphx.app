use super::*;
use crate::testing::SqliteTestDb;

fn setup() -> (SqliteTestDb, SqlitePermissionRepository) {
    let db = SqliteTestDb::new("sqlite_permission_repo_tests");
    let repo = SqlitePermissionRepository::from_shared(db.shared_conn());
    (db, repo)
}

fn sample_info() -> PendingPermissionInfo {
    PendingPermissionInfo {
        request_id: "perm-1".to_string(),
        tool_name: "Bash".to_string(),
        tool_input: serde_json::json!({"command": "ls -la"}),
        context: Some("List files".to_string()),
    }
}

#[tokio::test]
async fn test_create_and_get_pending() {
    let (_db, repo) = setup();
    repo.create_pending(&sample_info()).await.unwrap();

    let pending = repo.get_pending().await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].request_id, "perm-1");
    assert_eq!(pending[0].tool_name, "Bash");
    assert_eq!(pending[0].tool_input["command"], "ls -la");
    assert_eq!(pending[0].context, Some("List files".to_string()));
}

#[tokio::test]
async fn test_get_by_request_id() {
    let (_db, repo) = setup();
    repo.create_pending(&sample_info()).await.unwrap();

    let found = repo.get_by_request_id("perm-1").await.unwrap();
    assert!(found.is_some());
    let p = found.unwrap();
    assert_eq!(p.tool_name, "Bash");
    assert_eq!(p.tool_input["command"], "ls -la");

    let not_found = repo.get_by_request_id("nonexistent").await.unwrap();
    assert!(not_found.is_none());
}

#[tokio::test]
async fn test_resolve() {
    let (_db, repo) = setup();
    repo.create_pending(&sample_info()).await.unwrap();

    let decision = PermissionDecision {
        decision: "allow".to_string(),
        message: Some("Approved".to_string()),
    };
    let resolved = repo.resolve("perm-1", &decision).await.unwrap();
    assert!(resolved);

    // After resolving, no longer in pending
    let pending = repo.get_pending().await.unwrap();
    assert!(pending.is_empty());

    // But still retrievable by id
    let found = repo.get_by_request_id("perm-1").await.unwrap();
    assert!(found.is_some());
}

#[tokio::test]
async fn test_resolve_nonexistent() {
    let (_db, repo) = setup();
    let decision = PermissionDecision {
        decision: "deny".to_string(),
        message: None,
    };
    let resolved = repo.resolve("nope", &decision).await.unwrap();
    assert!(!resolved);
}

#[tokio::test]
async fn test_expire_all_pending() {
    let (_db, repo) = setup();

    for i in 0..3 {
        let info = PendingPermissionInfo {
            request_id: format!("perm-{}", i),
            tool_name: "Bash".to_string(),
            tool_input: serde_json::json!({}),
            context: None,
        };
        repo.create_pending(&info).await.unwrap();
    }

    // Resolve one so it's not pending
    let decision = PermissionDecision {
        decision: "allow".to_string(),
        message: None,
    };
    repo.resolve("perm-0", &decision).await.unwrap();

    let expired = repo.expire_all_pending().await.unwrap();
    assert_eq!(expired, 2);

    let pending = repo.get_pending().await.unwrap();
    assert!(pending.is_empty());
}

#[tokio::test]
async fn test_remove() {
    let (_db, repo) = setup();
    repo.create_pending(&sample_info()).await.unwrap();

    let removed = repo.remove("perm-1").await.unwrap();
    assert!(removed);

    let found = repo.get_by_request_id("perm-1").await.unwrap();
    assert!(found.is_none());

    let removed_again = repo.remove("perm-1").await.unwrap();
    assert!(!removed_again);
}

#[tokio::test]
async fn test_expire_all_pending_via_permission_state() {
    use crate::application::permission_state::PermissionState;
    let db = SqliteTestDb::new("sqlite_permission_repo_tests-permission_state");
    let repo = Arc::new(SqlitePermissionRepository::from_shared(db.shared_conn()));

    // Seed pending permissions (simulating leftover from a previous app run)
    for i in 0..3 {
        let info = PendingPermissionInfo {
            request_id: format!("stale-{}", i),
            tool_name: "Bash".to_string(),
            tool_input: serde_json::json!({}),
            context: None,
        };
        repo.create_pending(&info).await.unwrap();
    }

    // Resolve one so only 2 remain pending
    let decision = PermissionDecision {
        decision: "allow".to_string(),
        message: None,
    };
    repo.resolve("stale-0", &decision).await.unwrap();

    assert_eq!(repo.get_pending().await.unwrap().len(), 2);

    // Simulate startup: create PermissionState with the repo, call expire
    let state = PermissionState::with_repo(repo.clone()
        as Arc<dyn crate::domain::repositories::permission_repository::PermissionRepository>);
    state.expire_stale_on_startup().await;

    // All pending should be expired
    assert!(repo.get_pending().await.unwrap().is_empty());
}

#[tokio::test]
async fn test_empty_tool_input_round_trip() {
    let (_db, repo) = setup();
    let info = PendingPermissionInfo {
        request_id: "perm-empty".to_string(),
        tool_name: "Read".to_string(),
        tool_input: serde_json::json!({}),
        context: None,
    };
    repo.create_pending(&info).await.unwrap();

    let found = repo.get_by_request_id("perm-empty").await.unwrap().unwrap();
    assert_eq!(found.tool_input, serde_json::json!({}));
    assert!(found.context.is_none());
}
