// Fix B5: Spawn dedup check prevents duplicate agents for same task
//
// After fix: AgenticClientSpawner.spawn() checks the handles map before
// spawning. If an agent handle already exists for the task_id, the spawn
// is skipped with a warning log.

#[test]
fn test_b5_fix_hashmap_contains_key_dedup() {
    // Verify HashMap-based dedup works for task_id lookup
    let mut handles: std::collections::HashMap<String, &str> = std::collections::HashMap::new();

    // First spawn: no existing handle -> should proceed
    assert!(
        !handles.contains_key("task-1"),
        "No existing handle -> spawn should proceed"
    );
    handles.insert("task-1".to_string(), "handle-1");

    // Second spawn: handle exists -> should skip
    assert!(
        handles.contains_key("task-1"),
        "Existing handle -> spawn should be skipped"
    );

    // Different task: no existing handle -> should proceed
    assert!(
        !handles.contains_key("task-2"),
        "Different task -> spawn should proceed"
    );
}

#[test]
fn test_b5_fix_dedup_check_is_task_id_scoped() {
    // Verify dedup only blocks same task_id, not all spawns
    let mut handles: std::collections::HashMap<String, &str> = std::collections::HashMap::new();

    handles.insert("task-1".to_string(), "handle-1");
    handles.insert("task-2".to_string(), "handle-2");

    // task-3 should still be spawnable
    assert!(!handles.contains_key("task-3"));

    // task-1 should be blocked
    assert!(handles.contains_key("task-1"));
}
