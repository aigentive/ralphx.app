use super::*;
use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

fn setup_conn() -> Arc<Mutex<Connection>> {
    let conn = open_memory_connection().expect("open memory connection");
    run_migrations(&conn).expect("run migrations");
    Arc::new(Mutex::new(conn))
}

#[tokio::test]
async fn test_register_and_get() {
    let conn = setup_conn();
    let registry = SqliteRunningAgentRegistry::new(conn);
    let key = RunningAgentKey::new("ideation", "session-123");

    registry
        .register(
            key.clone(),
            12345,
            "conv-abc".to_string(),
            "run-xyz".to_string(),
            Some("/tmp/worktree".to_string()),
            None,
        )
        .await;

    let info = registry.get(&key).await;
    assert!(info.is_some());
    let info = info.unwrap();
    assert_eq!(info.pid, 12345);
    assert_eq!(info.conversation_id, "conv-abc");
    assert_eq!(info.agent_run_id, "run-xyz");
}

#[tokio::test]
async fn test_register_with_cancellation_token() {
    let conn = setup_conn();
    let registry = SqliteRunningAgentRegistry::new(conn);
    let key = RunningAgentKey::new("task", "task-cancel");
    let token = CancellationToken::new();

    registry
        .register(
            key.clone(),
            99999,
            "conv-ct".to_string(),
            "run-ct".to_string(),
            Some("/tmp/ct".to_string()),
            Some(token.clone()),
        )
        .await;

    let info = registry.get(&key).await.unwrap();
    assert!(info.cancellation_token.is_some());
    assert!(!token.is_cancelled());

    // Unregister should return token
    let info = registry.unregister(&key, "run-ct").await.unwrap();
    assert!(info.cancellation_token.is_some());
}

#[tokio::test]
async fn test_unregister() {
    let conn = setup_conn();
    let registry = SqliteRunningAgentRegistry::new(conn);
    let key = RunningAgentKey::new("task", "task-456");

    registry
        .register(
            key.clone(),
            999,
            "conv-1".to_string(),
            "run-1".to_string(),
            Some("/tmp/worktree".to_string()),
            None,
        )
        .await;

    let info = registry.unregister(&key, "run-1").await;
    assert!(info.is_some());
    assert_eq!(info.unwrap().pid, 999);

    // Should be gone
    assert!(!registry.is_running(&key).await);

    // Double unregister returns None
    let info = registry.unregister(&key, "run-1").await;
    assert!(info.is_none());
}

#[tokio::test]
async fn test_is_running() {
    let conn = setup_conn();
    let registry = SqliteRunningAgentRegistry::new(conn);
    let key = RunningAgentKey::new("review", "review-789");

    assert!(!registry.is_running(&key).await);

    registry
        .register(
            key.clone(),
            111,
            "conv-x".to_string(),
            "run-x".to_string(),
            Some("/tmp/worktree".to_string()),
            None,
        )
        .await;

    assert!(registry.is_running(&key).await);
}

#[tokio::test]
async fn test_list_all() {
    let conn = setup_conn();
    let registry = SqliteRunningAgentRegistry::new(conn);

    registry
        .register(
            RunningAgentKey::new("ideation", "s1"),
            100,
            "c1".to_string(),
            "r1".to_string(),
            Some("/tmp/k1".to_string()),
            None,
        )
        .await;
    registry
        .register(
            RunningAgentKey::new("task", "t1"),
            200,
            "c2".to_string(),
            "r2".to_string(),
            Some("/tmp/k2".to_string()),
            None,
        )
        .await;

    let all = registry.list_all().await;
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn test_stop_all_clears_table() {
    let conn = setup_conn();
    let registry = SqliteRunningAgentRegistry::new(conn);

    registry
        .register(
            RunningAgentKey::new("a", "1"),
            10001,
            "c".to_string(),
            "r".to_string(),
            Some("/tmp/a".to_string()),
            None,
        )
        .await;
    registry
        .register(
            RunningAgentKey::new("b", "2"),
            10002,
            "c".to_string(),
            "r".to_string(),
            Some("/tmp/b".to_string()),
            None,
        )
        .await;

    let stopped = registry.stop_all().await;
    assert_eq!(stopped.len(), 2);

    // Table should be empty
    let all = registry.list_all().await;
    assert!(all.is_empty());
}

#[tokio::test]
async fn test_register_replaces_existing() {
    let conn = setup_conn();
    let registry = SqliteRunningAgentRegistry::new(conn);
    let key = RunningAgentKey::new("task", "task-1");

    registry
        .register(
            key.clone(),
            100,
            "conv-old".to_string(),
            "run-old".to_string(),
            Some("/tmp/old".to_string()),
            None,
        )
        .await;
    registry
        .register(
            key.clone(),
            200,
            "conv-new".to_string(),
            "run-new".to_string(),
            Some("/tmp/new".to_string()),
            None,
        )
        .await;

    let info = registry.get(&key).await.unwrap();
    assert_eq!(info.pid, 200);
    assert_eq!(info.conversation_id, "conv-new");

    // Only one entry
    let all = registry.list_all().await;
    assert_eq!(all.len(), 1);
}

#[tokio::test]
async fn test_register_stops_orphaned_process() {
    let conn = setup_conn();
    let registry = SqliteRunningAgentRegistry::new(conn);
    let key = RunningAgentKey::new("task", "task-orphan");
    let old_token = CancellationToken::new();

    // Spawn a real process so is_process_alive returns true
    let mut child = std::process::Command::new("sleep")
        .arg("60")
        .spawn()
        .expect("spawn sleep");
    let old_pid = child.id();

    registry
        .register(
            key.clone(),
            old_pid,
            "conv-old".to_string(),
            "run-old".to_string(),
            None,
            Some(old_token.clone()),
        )
        .await;

    assert!(!old_token.is_cancelled());
    assert!(is_process_alive(old_pid));

    // Re-register with a new PID — should stop the old process
    registry
        .register(
            key.clone(),
            99999,
            "conv-new".to_string(),
            "run-new".to_string(),
            None,
            None,
        )
        .await;

    // Old token should be cancelled
    assert!(old_token.is_cancelled());

    // Reap the zombie (SIGTERM was sent, wait collects exit status)
    let _ = child.wait();
    assert!(!is_process_alive(old_pid));

    // New registration should be active
    let info = registry.get(&key).await.unwrap();
    assert_eq!(info.pid, 99999);
    assert_eq!(info.conversation_id, "conv-new");

    // Only one entry
    let all = registry.list_all().await;
    assert_eq!(all.len(), 1);
}

#[tokio::test]
async fn test_try_register_succeeds_when_empty() {
    let conn = setup_conn();
    let registry = SqliteRunningAgentRegistry::new(conn);
    let key = RunningAgentKey::new("task_execution", "task-fresh");

    let result = registry
        .try_register(key.clone(), "conv-1".to_string(), "run-1".to_string())
        .await;

    assert!(result.is_ok());
    assert!(registry.is_running(&key).await);

    // Placeholder should have pid=0
    let info = registry.get(&key).await.unwrap();
    assert_eq!(info.pid, 0);
    assert_eq!(info.conversation_id, "conv-1");
    assert_eq!(info.agent_run_id, "run-1");
}

#[tokio::test]
async fn test_try_register_fails_when_occupied() {
    let conn = setup_conn();
    let registry = SqliteRunningAgentRegistry::new(conn);
    let key = RunningAgentKey::new("task_execution", "task-occupied");

    // First registration via register()
    registry
        .register(
            key.clone(),
            12345,
            "conv-existing".to_string(),
            "run-existing".to_string(),
            None,
            None,
        )
        .await;

    // try_register should fail
    let result = registry
        .try_register(key.clone(), "conv-new".to_string(), "run-new".to_string())
        .await;

    assert!(result.is_err());
    let existing = result.unwrap_err();
    assert_eq!(existing.pid, 12345);
    assert_eq!(existing.conversation_id, "conv-existing");

    // Original registration should be unchanged
    let info = registry.get(&key).await.unwrap();
    assert_eq!(info.pid, 12345);
}

#[tokio::test]
async fn test_try_register_then_update_agent_process() {
    let conn = setup_conn();
    let registry = SqliteRunningAgentRegistry::new(conn);
    let key = RunningAgentKey::new("task_execution", "task-update");
    let token = CancellationToken::new();

    // Claim the slot
    let result = registry
        .try_register(key.clone(), "conv-1".to_string(), "run-1".to_string())
        .await;
    assert!(result.is_ok());

    // Placeholder has pid=0
    let info = registry.get(&key).await.unwrap();
    assert_eq!(info.pid, 0);
    assert!(info.worktree_path.is_none());

    // Update with real process details
    registry
        .update_agent_process(
            &key,
            54321,
            "conv-1",
            "run-real",
            Some("/tmp/worktree".to_string()),
            Some(token.clone()),
        )
        .await
        .unwrap();

    // Should now have real PID, agent_run_id, and worktree
    let info = registry.get(&key).await.unwrap();
    assert_eq!(info.pid, 54321);
    assert_eq!(info.agent_run_id, "run-real");
    assert_eq!(info.worktree_path.as_deref(), Some("/tmp/worktree"));
    assert!(info.cancellation_token.is_some());
}

/// TOCTOU race: try_register → pruner deletes row → update_agent_process re-inserts.
/// Ensures the agent is tracked even if the placeholder was pruned mid-spawn.
#[tokio::test]
async fn test_toctou_pruner_deletes_placeholder_then_update_reinserts() {
    let conn = setup_conn();
    let registry = SqliteRunningAgentRegistry::new(Arc::clone(&conn));
    let key = RunningAgentKey::new("task_execution", "task-toctou");

    // Step 1: Claim the slot (placeholder pid=0)
    let result = registry
        .try_register(key.clone(), "conv-toctou".to_string(), "run-toctou".to_string())
        .await;
    assert!(result.is_ok());
    assert!(registry.is_running(&key).await);

    // Step 2: Simulate pruner deleting the placeholder row
    {
        let db = conn.lock().await;
        db.execute(
            "DELETE FROM running_agents WHERE context_type = ?1 AND context_id = ?2",
            rusqlite::params!["task_execution", "task-toctou"],
        )
        .unwrap();
    }
    assert!(!registry.is_running(&key).await);

    // Step 3: update_agent_process should re-insert the full registration
    let token = CancellationToken::new();
    registry
        .update_agent_process(
            &key,
            12345,
            "conv-toctou",
            "run-real",
            Some("/tmp/worktree-toctou".to_string()),
            Some(token.clone()),
        )
        .await
        .unwrap();

    // Step 4: Verify the row was re-inserted with correct data
    assert!(registry.is_running(&key).await);
    let info = registry.get(&key).await.unwrap();
    assert_eq!(info.pid, 12345);
    assert_eq!(info.conversation_id, "conv-toctou");
    assert_eq!(info.agent_run_id, "run-real");
    assert_eq!(
        info.worktree_path.as_deref(),
        Some("/tmp/worktree-toctou")
    );
    assert!(info.cancellation_token.is_some());

    // Only one entry in the table
    let all = registry.list_all().await;
    assert_eq!(all.len(), 1);
}

#[tokio::test]
async fn test_try_register_cleanup_on_spawn_failure() {
    let conn = setup_conn();
    let registry = SqliteRunningAgentRegistry::new(conn);
    let key = RunningAgentKey::new("task_execution", "task-fail");

    // Claim the slot
    let result = registry
        .try_register(key.clone(), "conv-1".to_string(), "run-1".to_string())
        .await;
    assert!(result.is_ok());
    assert!(registry.is_running(&key).await);

    // Simulate spawn failure: unregister to release the slot
    registry.unregister(&key, "run-1").await;
    assert!(!registry.is_running(&key).await);

    // Another try_register should succeed now
    let result = registry
        .try_register(key.clone(), "conv-2".to_string(), "run-2".to_string())
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_try_register_blocks_concurrent_claim() {
    let conn = setup_conn();
    let registry = SqliteRunningAgentRegistry::new(conn);
    let key = RunningAgentKey::new("task_execution", "task-race");

    // First try_register claims the slot
    let r1 = registry
        .try_register(key.clone(), "conv-1".to_string(), "run-1".to_string())
        .await;
    assert!(r1.is_ok());

    // Second try_register should fail (slot is claimed even with pid=0)
    let r2 = registry
        .try_register(key.clone(), "conv-2".to_string(), "run-2".to_string())
        .await;
    assert!(r2.is_err());
    let existing = r2.unwrap_err();
    assert_eq!(existing.pid, 0); // Still placeholder
    assert_eq!(existing.conversation_id, "conv-1");
}
