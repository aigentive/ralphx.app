// Streaming State Cache
//
// In-memory cache for tracking streaming state per conversation.
// Used to hydrate frontend when navigating to an active agent execution
// where streaming events were missed during component mount.
//
// This is intentionally NOT persisted — streaming state is transient UI feedback.

use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// A cached tool call that's currently in progress or recently completed.
#[derive(Debug, Clone, Serialize)]
pub struct CachedToolCall {
    /// Unique tool call ID (e.g., "toolu_01A...")
    pub id: String,
    /// Tool name (e.g., "bash", "read", "edit")
    pub name: String,
    /// Current arguments (may be partial during streaming)
    pub arguments: serde_json::Value,
    /// Result if completed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Diff context for Edit/Write tools
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_context: Option<serde_json::Value>,
    /// Parent tool use ID for nested tool calls
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_tool_use_id: Option<String>,
}

/// A cached streaming task (subagent execution).
#[derive(Debug, Clone, Serialize)]
pub struct CachedStreamingTask {
    /// Tool use ID that started this task
    pub tool_use_id: String,
    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Subagent type (e.g., "ralphx:coder")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_type: Option<String>,
    /// Model being used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Current status: "running" or "completed"
    pub status: String,
    /// Teammate name if this is a team member task
    #[serde(skip_serializing_if = "Option::is_none")]
    pub teammate_name: Option<String>,
}

/// Complete streaming state for a single conversation.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ConversationStreamingState {
    /// Tool calls currently in progress or recently completed
    pub tool_calls: Vec<CachedToolCall>,
    /// Streaming tasks (subagents) currently running or completed
    pub streaming_tasks: Vec<CachedStreamingTask>,
    /// Partial text content accumulated from agent:chunk events
    pub partial_text: String,
    /// When this state was last updated
    pub updated_at: DateTime<Utc>,
}

impl ConversationStreamingState {
    /// Create a new empty state
    pub fn new() -> Self {
        Self {
            tool_calls: Vec::new(),
            streaming_tasks: Vec::new(),
            partial_text: String::new(),
            updated_at: Utc::now(),
        }
    }
}

/// In-memory cache for streaming state, keyed by conversation_id.
#[derive(Debug, Clone)]
pub struct StreamingStateCache {
    states: Arc<Mutex<HashMap<String, ConversationStreamingState>>>,
}

impl Default for StreamingStateCache {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingStateCache {
    /// Create a new empty cache
    pub fn new() -> Self {
        Self {
            states: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Upsert a tool call into the cache.
    ///
    /// If a tool call with the same ID exists, it's updated.
    /// Otherwise, a new entry is added.
    pub async fn upsert_tool_call(&self, conversation_id: &str, tool_call: CachedToolCall) {
        let mut states = self.states.lock().await;
        let state = states.entry(conversation_id.to_string()).or_insert_with(|| {
            tracing::debug!(
                conversation_id,
                tool_id = %tool_call.id,
                "StreamingStateCache: creating new state for upsert_tool_call"
            );
            ConversationStreamingState::new()
        });

        // Find existing tool call with same ID and update, or add new
        if let Some(existing) = state
            .tool_calls
            .iter_mut()
            .find(|tc| tc.id == tool_call.id)
        {
            *existing = tool_call;
            tracing::trace!(
                conversation_id,
                tool_id = %existing.id,
                "StreamingStateCache: updated existing tool_call"
            );
        } else {
            tracing::debug!(
                conversation_id,
                tool_id = %tool_call.id,
                tool_name = %tool_call.name,
                "StreamingStateCache: added new tool_call"
            );
            state.tool_calls.push(tool_call);
        }
        state.updated_at = Utc::now();
    }

    /// Add a streaming task (subagent started event).
    pub async fn add_task(&self, conversation_id: &str, task: CachedStreamingTask) {
        let mut states = self.states.lock().await;
        let state = states.entry(conversation_id.to_string()).or_insert_with(|| {
            tracing::debug!(
                conversation_id,
                tool_use_id = %task.tool_use_id,
                "StreamingStateCache: creating new state for add_task"
            );
            ConversationStreamingState::new()
        });

        tracing::debug!(
            conversation_id,
            tool_use_id = %task.tool_use_id,
            subagent_type = ?task.subagent_type,
            "StreamingStateCache: added streaming task"
        );
        state.streaming_tasks.push(task);
        state.updated_at = Utc::now();
    }

    /// Mark a streaming task as completed.
    pub async fn complete_task(&self, conversation_id: &str, tool_use_id: &str) {
        let mut states = self.states.lock().await;
        if let Some(state) = states.get_mut(conversation_id) {
            if let Some(task) = state
                .streaming_tasks
                .iter_mut()
                .find(|t| t.tool_use_id == tool_use_id)
            {
                task.status = "completed".to_string();
                state.updated_at = Utc::now();
                tracing::debug!(
                    conversation_id,
                    tool_use_id,
                    "StreamingStateCache: marked task as completed"
                );
            }
        }
    }

    /// Append text to the partial content buffer.
    pub async fn append_text(&self, conversation_id: &str, text: &str) {
        let mut states = self.states.lock().await;
        let state = states.entry(conversation_id.to_string()).or_insert_with(|| {
            tracing::debug!(
                conversation_id,
                text_len = text.len(),
                "StreamingStateCache: creating new state for append_text"
            );
            ConversationStreamingState::new()
        });

        state.partial_text.push_str(text);
        state.updated_at = Utc::now();
        tracing::trace!(
            conversation_id,
            text_len = text.len(),
            total_len = state.partial_text.len(),
            "StreamingStateCache: appended text"
        );
    }

    /// Clear all streaming state for a conversation.
    ///
    /// Called when agent:run_completed or agent:error fires.
    pub async fn clear(&self, conversation_id: &str) {
        let mut states = self.states.lock().await;
        if states.remove(conversation_id).is_some() {
            tracing::debug!(
                conversation_id,
                "StreamingStateCache: cleared state"
            );
        }
    }

    /// Get the current streaming state for a conversation.
    ///
    /// Returns None if no state exists (conversation not currently streaming).
    pub async fn get(&self, conversation_id: &str) -> Option<ConversationStreamingState> {
        let states = self.states.lock().await;
        states.get(conversation_id).cloned()
    }
}

#[cfg(test)]
mod tests {
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
        };
        cache.add_task("conv-123", task).await;

        cache.complete_task("conv-123", "toolu_002").await;

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
}
