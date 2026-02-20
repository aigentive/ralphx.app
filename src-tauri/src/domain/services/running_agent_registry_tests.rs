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

    let info = registry.unregister(&key).await;
    assert!(info.is_some());

    assert!(!registry.is_running(&key).await);

    // Double unregister should return None
    let info = registry.unregister(&key).await;
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
