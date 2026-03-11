use super::*;

#[tokio::test]
async fn test_new_cache_is_empty() {
    let cache = StreamingStateCache::new();
    let state = cache.get("conv-123").await;
    assert!(state.is_none());
}

#[tokio::test]
async fn test_upsert_tool_call_creates_state() {
    let cache = StreamingStateCache::new();
    let tool_call = CachedToolCall {
        id: "toolu_001".to_string(),
        name: "bash".to_string(),
        arguments: serde_json::json!({"command": "ls"}),
        result: None,
        diff_context: None,
        parent_tool_use_id: None,
    };

    cache.upsert_tool_call("conv-123", tool_call).await;

    let state = cache.get("conv-123").await;
    assert!(state.is_some());
    let state = state.unwrap();
    assert_eq!(state.tool_calls.len(), 1);
    assert_eq!(state.tool_calls[0].name, "bash");
}

#[tokio::test]
async fn test_upsert_tool_call_updates_existing() {
    let cache = StreamingStateCache::new();

    // Add initial tool call
    let tool_call = CachedToolCall {
        id: "toolu_001".to_string(),
        name: "bash".to_string(),
        arguments: serde_json::json!({"command": "ls"}),
        result: None,
        diff_context: None,
        parent_tool_use_id: None,
    };
    cache.upsert_tool_call("conv-123", tool_call).await;

    // Update with result
    let updated = CachedToolCall {
        id: "toolu_001".to_string(),
        name: "bash".to_string(),
        arguments: serde_json::json!({"command": "ls"}),
        result: Some(serde_json::json!({"output": "file1.txt\nfile2.txt"})),
        diff_context: None,
        parent_tool_use_id: None,
    };
    cache.upsert_tool_call("conv-123", updated).await;

    let state = cache.get("conv-123").await.unwrap();
    assert_eq!(state.tool_calls.len(), 1); // Still just one
    assert!(state.tool_calls[0].result.is_some());
}

#[tokio::test]
async fn test_add_task() {
    let cache = StreamingStateCache::new();
    let task = CachedStreamingTask {
        tool_use_id: "toolu_002".to_string(),
        description: Some("Running tests".to_string()),
        subagent_type: Some("ralphx:coder".to_string()),
        model: Some("sonnet".to_string()),
        status: "running".to_string(),
        teammate_name: None,
        total_tokens: None,
        total_tool_uses: None,
        duration_ms: None,
    };

    cache.add_task("conv-123", task).await;

    let state = cache.get("conv-123").await.unwrap();
    assert_eq!(state.streaming_tasks.len(), 1);
    assert_eq!(state.streaming_tasks[0].status, "running");
}

#[tokio::test]
async fn test_complete_task() {
    let cache = StreamingStateCache::new();
    let task = CachedStreamingTask {
        tool_use_id: "toolu_002".to_string(),
        description: Some("Running tests".to_string()),
        subagent_type: Some("ralphx:coder".to_string()),
        model: Some("sonnet".to_string()),
        status: "running".to_string(),
        teammate_name: None,
        total_tokens: None,
        total_tool_uses: None,
        duration_ms: None,
    };
    cache.add_task("conv-123", task).await;

    cache.complete_task("conv-123", "toolu_002", None).await;

    let state = cache.get("conv-123").await.unwrap();
    assert_eq!(state.streaming_tasks[0].status, "completed");
}

#[tokio::test]
async fn test_append_text() {
    let cache = StreamingStateCache::new();

    cache.append_text("conv-123", "Hello ").await;
    cache.append_text("conv-123", "world!").await;

    let state = cache.get("conv-123").await.unwrap();
    assert_eq!(state.partial_text, "Hello world!");
}

#[tokio::test]
async fn test_clear() {
    let cache = StreamingStateCache::new();
    let tool_call = CachedToolCall {
        id: "toolu_001".to_string(),
        name: "bash".to_string(),
        arguments: serde_json::json!({}),
        result: None,
        diff_context: None,
        parent_tool_use_id: None,
    };
    cache.upsert_tool_call("conv-123", tool_call).await;

    cache.clear("conv-123").await;

    let state = cache.get("conv-123").await;
    assert!(state.is_none());
}

#[tokio::test]
async fn test_clear_nonexistent_is_noop() {
    let cache = StreamingStateCache::new();
    // Should not panic
    cache.clear("nonexistent").await;
}

#[tokio::test]
async fn test_multiple_conversations_independent() {
    let cache = StreamingStateCache::new();

    let tool1 = CachedToolCall {
        id: "toolu_001".to_string(),
        name: "bash".to_string(),
        arguments: serde_json::json!({}),
        result: None,
        diff_context: None,
        parent_tool_use_id: None,
    };
    let tool2 = CachedToolCall {
        id: "toolu_002".to_string(),
        name: "read".to_string(),
        arguments: serde_json::json!({"file_path": "/tmp/test.txt"}),
        result: None,
        diff_context: None,
        parent_tool_use_id: None,
    };

    cache.upsert_tool_call("conv-1", tool1).await;
    cache.upsert_tool_call("conv-2", tool2).await;

    let state1 = cache.get("conv-1").await.unwrap();
    let state2 = cache.get("conv-2").await.unwrap();

    assert_eq!(state1.tool_calls.len(), 1);
    assert_eq!(state1.tool_calls[0].name, "bash");
    assert_eq!(state2.tool_calls.len(), 1);
    assert_eq!(state2.tool_calls[0].name, "read");

    // Clear one doesn't affect the other
    cache.clear("conv-1").await;
    assert!(cache.get("conv-1").await.is_none());
    assert!(cache.get("conv-2").await.is_some());
}

#[tokio::test]
async fn test_updated_at_changes_on_modification() {
    let cache = StreamingStateCache::new();

    cache.append_text("conv-123", "test").await;
    let first_update = cache.get("conv-123").await.unwrap().updated_at;

    // Small delay to ensure timestamp difference
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    cache.append_text("conv-123", " more").await;
    let second_update = cache.get("conv-123").await.unwrap().updated_at;

    assert!(second_update > first_update);
}

#[tokio::test]
async fn test_serialize_produces_expected_json() {
    let state = ConversationStreamingState {
        tool_calls: vec![CachedToolCall {
            id: "toolu_001".to_string(),
            name: "bash".to_string(),
            arguments: serde_json::json!({"command": "ls"}),
            result: None,
            diff_context: None,
            parent_tool_use_id: None,
        }],
        streaming_tasks: vec![CachedStreamingTask {
            tool_use_id: "toolu_002".to_string(),
            description: Some("Test task".to_string()),
            subagent_type: None,
            model: None,
            status: "running".to_string(),
            teammate_name: None,
            total_tokens: None,
            total_tool_uses: None,
            duration_ms: None,
        }],
        partial_text: "Hello".to_string(),
        updated_at: Utc::now(),
    };

    let json = serde_json::to_string(&state).unwrap();
    assert!(json.contains("\"tool_calls\""));
    assert!(json.contains("\"streaming_tasks\""));
    assert!(json.contains("\"partial_text\""));
    assert!(json.contains("\"toolu_001\""));
    assert!(json.contains("\"running\""));
    assert!(json.contains("\"Hello\""));
}

#[tokio::test]
async fn test_serialize_skips_none_fields() {
    let tool_call = CachedToolCall {
        id: "toolu_001".to_string(),
        name: "bash".to_string(),
        arguments: serde_json::json!({}),
        result: None,
        diff_context: None,
        parent_tool_use_id: None,
    };

    let json = serde_json::to_string(&tool_call).unwrap();
    assert!(!json.contains("\"result\""));
    assert!(!json.contains("\"diff_context\""));
    assert!(!json.contains("\"parent_tool_use_id\""));
}

#[tokio::test]
async fn test_complete_task_with_stats() {
    let cache = StreamingStateCache::new();
    let task = CachedStreamingTask {
        tool_use_id: "toolu_002".to_string(),
        description: Some("Running tests".to_string()),
        subagent_type: None,
        model: None,
        status: "running".to_string(),
        teammate_name: None,
        total_tokens: None,
        total_tool_uses: None,
        duration_ms: None,
    };
    cache.add_task("conv-123", task).await;

    use crate::infrastructure::agents::claude::ToolCallStats;
    let stats = ToolCallStats {
        model: Some("sonnet".to_string()),
        total_tokens: Some(1234),
        total_tool_uses: Some(5),
        duration_ms: Some(30000),
    };
    cache.complete_task("conv-123", "toolu_002", Some(stats)).await;

    let state = cache.get("conv-123").await.unwrap();
    assert_eq!(state.streaming_tasks[0].status, "completed");
    assert_eq!(state.streaming_tasks[0].total_tokens, Some(1234));
    assert_eq!(state.streaming_tasks[0].total_tool_uses, Some(5));
    assert_eq!(state.streaming_tasks[0].duration_ms, Some(30000));
}

#[tokio::test]
async fn test_complete_task_with_none_stats_clears_nothing() {
    let cache = StreamingStateCache::new();
    let task = CachedStreamingTask {
        tool_use_id: "toolu_003".to_string(),
        description: None,
        subagent_type: None,
        model: None,
        status: "running".to_string(),
        teammate_name: None,
        total_tokens: None,
        total_tool_uses: None,
        duration_ms: None,
    };
    cache.add_task("conv-abc", task).await;

    cache.complete_task("conv-abc", "toolu_003", None).await;

    let state = cache.get("conv-abc").await.unwrap();
    assert_eq!(state.streaming_tasks[0].status, "completed");
    assert_eq!(state.streaming_tasks[0].total_tokens, None);
    assert_eq!(state.streaming_tasks[0].total_tool_uses, None);
    assert_eq!(state.streaming_tasks[0].duration_ms, None);
}
