use super::*;

#[tokio::test]
async fn test_register_and_get() {
    let registry = MemoryRunningAgentRegistry::new();
    let key = RunningAgentKey::new("ideation", "session-123");

    registry
        .register(
            key.clone(),
            12345,
            "conv-abc".to_string(),
            "run-xyz".to_string(),
            None,
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
    let registry = MemoryRunningAgentRegistry::new();
    let key = RunningAgentKey::new("task", "task-cancel");
    let token = CancellationToken::new();

    registry
        .register(
            key.clone(),
            99999,
            "conv-ct".to_string(),
            "run-ct".to_string(),
            None,
            Some(token.clone()),
        )
        .await;

    let info = registry.get(&key).await.unwrap();
    assert!(info.cancellation_token.is_some());
    assert!(!token.is_cancelled());

    // Stop should cancel token
    let _ = registry.stop(&key).await;
    assert!(token.is_cancelled());
}

#[tokio::test]
async fn test_is_running() {
    let registry = MemoryRunningAgentRegistry::new();
    let key = RunningAgentKey::new("task", "task-123");

    assert!(!registry.is_running(&key).await);

    registry
        .register(
            key.clone(),
            12345,
            "conv-abc".to_string(),
            "run-xyz".to_string(),
            None,
            None,
        )
        .await;

    assert!(registry.is_running(&key).await);
}

#[tokio::test]
async fn test_unregister() {
    let registry = MemoryRunningAgentRegistry::new();
    let key = RunningAgentKey::new("project", "proj-123");

    registry
        .register(
            key.clone(),
            12345,
            "conv-abc".to_string(),
            "run-xyz".to_string(),
            None,
            None,
        )
        .await;

    let info = registry.unregister(&key, "run-xyz").await;
    assert!(info.is_some());

    assert!(!registry.is_running(&key).await);

    // Double unregister should return None
    let info = registry.unregister(&key, "run-xyz").await;
    assert!(info.is_none());
}

#[tokio::test]
async fn test_register_stops_orphaned_process() {
    let registry = MemoryRunningAgentRegistry::new();
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
}

#[tokio::test]
async fn test_list_all() {
    let registry = MemoryRunningAgentRegistry::new();

    registry
        .register(
            RunningAgentKey::new("ideation", "session-1"),
            111,
            "conv-1".to_string(),
            "run-1".to_string(),
            None,
            None,
        )
        .await;

    registry
        .register(
            RunningAgentKey::new("task", "task-2"),
            222,
            "conv-2".to_string(),
            "run-2".to_string(),
            None,
            None,
        )
        .await;

    let all = registry.list_all().await;
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn test_try_register_succeeds_when_empty() {
    let registry = MemoryRunningAgentRegistry::new();
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
    let registry = MemoryRunningAgentRegistry::new();
    let key = RunningAgentKey::new("task_execution", "task-occupied");

    // First registration succeeds
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
    let registry = MemoryRunningAgentRegistry::new();
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
    assert!(info.cancellation_token.is_none());

    // Update with real process details
    registry
        .update_agent_process(
            &key,
            54321,
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

#[tokio::test]
async fn test_try_register_cleanup_on_spawn_failure() {
    let registry = MemoryRunningAgentRegistry::new();
    let key = RunningAgentKey::new("task_execution", "task-fail");

    // Claim the slot
    let result = registry
        .try_register(key.clone(), "conv-1".to_string(), "run-1".to_string())
        .await;
    assert!(result.is_ok());
    assert!(registry.is_running(&key).await);

    // Simulate spawn failure: unregister to release the slot
    registry.unregister(&key, "run-1").await;

    // Slot should be free again
    assert!(!registry.is_running(&key).await);

    // Another try_register should succeed now
    let result = registry
        .try_register(key.clone(), "conv-2".to_string(), "run-2".to_string())
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_try_register_blocks_concurrent_claim() {
    let registry = MemoryRunningAgentRegistry::new();
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
