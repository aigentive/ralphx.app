// Unified Message Queue Service
//
// Generic message queue that handles all chat context types.
// Keyed by (ChatContextType, context_id) instead of just TaskId.
//
// This is a consolidation of ExecutionMessageQueue to support
// queueing messages for all context types, not just TaskExecution.

use crate::domain::entities::{ChatContextType, TaskId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Key for the message queue - combines context type and ID
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct QueueKey {
    pub context_type: ChatContextType,
    pub context_id: String,
}

impl QueueKey {
    pub fn new(context_type: ChatContextType, context_id: impl Into<String>) -> Self {
        Self {
            context_type,
            context_id: context_id.into(),
        }
    }

    /// Create a key for task execution context (convenience method)
    pub fn task_execution(task_id: &TaskId) -> Self {
        Self::new(ChatContextType::TaskExecution, task_id.as_str())
    }

    /// Create a key for ideation context
    pub fn ideation(session_id: &str) -> Self {
        Self::new(ChatContextType::Ideation, session_id)
    }

    /// Create a key for task context
    pub fn task(task_id: &str) -> Self {
        Self::new(ChatContextType::Task, task_id)
    }

    /// Create a key for project context
    pub fn project(project_id: &str) -> Self {
        Self::new(ChatContextType::Project, project_id)
    }
}

/// A queued message waiting to be sent to an agent
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueuedMessage {
    pub id: String,
    pub content: String,
    pub created_at: String,
    pub is_editing: bool,
}

impl QueuedMessage {
    /// Create a new queued message with generated ID and timestamp
    pub fn new(content: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            created_at: chrono::Utc::now().to_rfc3339(),
            is_editing: false,
        }
    }

    /// Create a new queued message with a client-provided ID
    /// This allows the frontend to track the message with its own ID
    pub fn with_id(id: String, content: String) -> Self {
        Self {
            id,
            content,
            created_at: chrono::Utc::now().to_rfc3339(),
            is_editing: false,
        }
    }
}

/// Unified in-memory queue for chat messages
///
/// Stores queued messages per (context_type, context_id) pair.
/// Messages are ephemeral and lost on app restart.
/// This is intentional - queued messages are short-lived and should be sent
/// when the agent responds. If the app restarts, the user can re-type their message.
#[derive(Debug, Clone)]
pub struct MessageQueue {
    queues: Arc<Mutex<HashMap<QueueKey, Vec<QueuedMessage>>>>,
}

impl MessageQueue {
    /// Create a new empty queue
    pub fn new() -> Self {
        Self {
            queues: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Queue a message for a context
    pub fn queue(
        &self,
        context_type: ChatContextType,
        context_id: impl Into<String>,
        content: String,
    ) -> QueuedMessage {
        let key = QueueKey::new(context_type, context_id);
        let message = QueuedMessage::new(content);
        let mut queues = self.queues.lock().unwrap();
        queues.entry(key).or_default().push(message.clone());
        message
    }

    /// Queue a message using a QueueKey
    pub fn queue_with_key(&self, key: QueueKey, content: String) -> QueuedMessage {
        let message = QueuedMessage::new(content);
        let mut queues = self.queues.lock().unwrap();
        queues.entry(key).or_default().push(message.clone());
        message
    }

    /// Queue a message with a client-provided ID
    /// This allows frontend and backend to use the same ID for tracking
    pub fn queue_with_client_id(
        &self,
        context_type: ChatContextType,
        context_id: impl Into<String>,
        content: String,
        client_id: String,
    ) -> QueuedMessage {
        let key = QueueKey::new(context_type, context_id);
        let message = QueuedMessage::with_id(client_id, content);
        let mut queues = self.queues.lock().unwrap();
        queues.entry(key).or_default().push(message.clone());
        message
    }

    /// Pop the next message from the queue (FIFO)
    pub fn pop(&self, context_type: ChatContextType, context_id: &str) -> Option<QueuedMessage> {
        let key = QueueKey::new(context_type, context_id.to_string());
        let mut queues = self.queues.lock().unwrap();
        queues.get_mut(&key).and_then(|queue| {
            if queue.is_empty() {
                None
            } else {
                Some(queue.remove(0))
            }
        })
    }

    /// Pop using a QueueKey
    pub fn pop_with_key(&self, key: &QueueKey) -> Option<QueuedMessage> {
        let mut queues = self.queues.lock().unwrap();
        queues.get_mut(key).and_then(|queue| {
            if queue.is_empty() {
                None
            } else {
                Some(queue.remove(0))
            }
        })
    }

    /// Get all queued messages for a context (without removing them)
    pub fn get_queued(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> Vec<QueuedMessage> {
        let key = QueueKey::new(context_type, context_id.to_string());
        let queues = self.queues.lock().unwrap();
        queues.get(&key).cloned().unwrap_or_default()
    }

    /// Get queued messages using a QueueKey
    pub fn get_queued_with_key(&self, key: &QueueKey) -> Vec<QueuedMessage> {
        let queues = self.queues.lock().unwrap();
        queues.get(key).cloned().unwrap_or_default()
    }

    /// Clear all queued messages for a context
    pub fn clear(&self, context_type: ChatContextType, context_id: &str) {
        let key = QueueKey::new(context_type, context_id.to_string());
        let mut queues = self.queues.lock().unwrap();
        queues.remove(&key);
    }

    /// Clear using a QueueKey
    pub fn clear_with_key(&self, key: &QueueKey) {
        let mut queues = self.queues.lock().unwrap();
        queues.remove(key);
    }

    /// Delete a specific queued message by ID
    pub fn delete(
        &self,
        context_type: ChatContextType,
        context_id: &str,
        message_id: &str,
    ) -> bool {
        let key = QueueKey::new(context_type, context_id.to_string());
        let mut queues = self.queues.lock().unwrap();
        if let Some(queue) = queues.get_mut(&key) {
            if let Some(pos) = queue.iter().position(|m| m.id == message_id) {
                queue.remove(pos);
                return true;
            }
        }
        false
    }

    /// Delete using a QueueKey
    pub fn delete_with_key(&self, key: &QueueKey, message_id: &str) -> bool {
        let mut queues = self.queues.lock().unwrap();
        if let Some(queue) = queues.get_mut(key) {
            if let Some(pos) = queue.iter().position(|m| m.id == message_id) {
                queue.remove(pos);
                return true;
            }
        }
        false
    }

    // =========================================================================
    // Backwards-compatible methods for TaskId (used by existing code)
    // =========================================================================

    /// Queue a message for a task execution (backwards compatibility)
    pub fn queue_for_task(&self, task_id: TaskId, content: String) -> QueuedMessage {
        self.queue(ChatContextType::TaskExecution, task_id.as_str(), content)
    }

    /// Pop the next message for a task execution (backwards compatibility)
    pub fn pop_for_task(&self, task_id: &TaskId) -> Option<QueuedMessage> {
        self.pop(ChatContextType::TaskExecution, task_id.as_str())
    }

    /// Get all queued messages for a task execution (backwards compatibility)
    pub fn get_queued_for_task(&self, task_id: &TaskId) -> Vec<QueuedMessage> {
        self.get_queued(ChatContextType::TaskExecution, task_id.as_str())
    }

    /// Clear all queued messages for a task execution (backwards compatibility)
    pub fn clear_for_task(&self, task_id: &TaskId) {
        self.clear(ChatContextType::TaskExecution, task_id.as_str())
    }

    /// Delete a queued message for a task execution (backwards compatibility)
    pub fn delete_for_task(&self, task_id: &TaskId, message_id: &str) -> bool {
        self.delete(ChatContextType::TaskExecution, task_id.as_str(), message_id)
    }
}

impl Default for MessageQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_and_pop() {
        let queue = MessageQueue::new();

        // Queue two messages
        let _msg1 = queue.queue(ChatContextType::Ideation, "session-1", "First message".to_string());
        let _msg2 = queue.queue(ChatContextType::Ideation, "session-1", "Second message".to_string());

        // Pop should return in FIFO order
        let popped1 = queue.pop(ChatContextType::Ideation, "session-1");
        assert!(popped1.is_some());
        assert_eq!(popped1.unwrap().content, "First message");

        let popped2 = queue.pop(ChatContextType::Ideation, "session-1");
        assert!(popped2.is_some());
        assert_eq!(popped2.unwrap().content, "Second message");

        // Queue should be empty now
        let popped3 = queue.pop(ChatContextType::Ideation, "session-1");
        assert!(popped3.is_none());
    }

    #[test]
    fn test_get_queued() {
        let queue = MessageQueue::new();

        // Initially empty
        assert_eq!(queue.get_queued(ChatContextType::Task, "task-1").len(), 0);

        // Queue two messages
        queue.queue(ChatContextType::Task, "task-1", "First".to_string());
        queue.queue(ChatContextType::Task, "task-1", "Second".to_string());

        // get_queued should return all messages without removing
        let queued = queue.get_queued(ChatContextType::Task, "task-1");
        assert_eq!(queued.len(), 2);
        assert_eq!(queued[0].content, "First");
        assert_eq!(queued[1].content, "Second");

        // Messages should still be in queue
        assert_eq!(queue.get_queued(ChatContextType::Task, "task-1").len(), 2);
    }

    #[test]
    fn test_clear() {
        let queue = MessageQueue::new();

        queue.queue(ChatContextType::Project, "proj-1", "Message 1".to_string());
        queue.queue(ChatContextType::Project, "proj-1", "Message 2".to_string());

        assert_eq!(queue.get_queued(ChatContextType::Project, "proj-1").len(), 2);

        queue.clear(ChatContextType::Project, "proj-1");

        assert_eq!(queue.get_queued(ChatContextType::Project, "proj-1").len(), 0);
        assert!(queue.pop(ChatContextType::Project, "proj-1").is_none());
    }

    #[test]
    fn test_delete() {
        let queue = MessageQueue::new();

        let _msg1 = queue.queue(ChatContextType::Ideation, "sess-1", "First".to_string());
        let msg2 = queue.queue(ChatContextType::Ideation, "sess-1", "Second".to_string());
        let _msg3 = queue.queue(ChatContextType::Ideation, "sess-1", "Third".to_string());

        assert_eq!(queue.get_queued(ChatContextType::Ideation, "sess-1").len(), 3);

        // Delete middle message
        let deleted = queue.delete(ChatContextType::Ideation, "sess-1", &msg2.id);
        assert!(deleted);

        let remaining = queue.get_queued(ChatContextType::Ideation, "sess-1");
        assert_eq!(remaining.len(), 2);
        assert_eq!(remaining[0].content, "First");
        assert_eq!(remaining[1].content, "Third");

        // Try deleting non-existent message
        let deleted = queue.delete(ChatContextType::Ideation, "sess-1", "non-existent-id");
        assert!(!deleted);
    }

    #[test]
    fn test_different_contexts_isolated() {
        let queue = MessageQueue::new();

        // Queue messages for different context types
        queue.queue(ChatContextType::Ideation, "id-1", "Ideation message".to_string());
        queue.queue(ChatContextType::Task, "id-1", "Task message".to_string());
        queue.queue(ChatContextType::Project, "id-1", "Project message".to_string());
        queue.queue(ChatContextType::TaskExecution, "id-1", "Execution message".to_string());

        // Each context type has its own queue
        assert_eq!(queue.get_queued(ChatContextType::Ideation, "id-1").len(), 1);
        assert_eq!(queue.get_queued(ChatContextType::Task, "id-1").len(), 1);
        assert_eq!(queue.get_queued(ChatContextType::Project, "id-1").len(), 1);
        assert_eq!(queue.get_queued(ChatContextType::TaskExecution, "id-1").len(), 1);

        // Popping from one doesn't affect others
        let popped = queue.pop(ChatContextType::Ideation, "id-1");
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().content, "Ideation message");

        assert_eq!(queue.get_queued(ChatContextType::Ideation, "id-1").len(), 0);
        assert_eq!(queue.get_queued(ChatContextType::Task, "id-1").len(), 1);
    }

    #[test]
    fn test_different_context_ids_isolated() {
        let queue = MessageQueue::new();

        queue.queue(ChatContextType::Ideation, "session-1", "Session 1 message".to_string());
        queue.queue(ChatContextType::Ideation, "session-2", "Session 2 message".to_string());

        assert_eq!(queue.get_queued(ChatContextType::Ideation, "session-1").len(), 1);
        assert_eq!(queue.get_queued(ChatContextType::Ideation, "session-2").len(), 1);

        let popped = queue.pop(ChatContextType::Ideation, "session-1");
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().content, "Session 1 message");

        assert_eq!(queue.get_queued(ChatContextType::Ideation, "session-1").len(), 0);
        assert_eq!(queue.get_queued(ChatContextType::Ideation, "session-2").len(), 1);
    }

    #[test]
    fn test_backwards_compatible_task_methods() {
        let queue = MessageQueue::new();
        let task_id = TaskId::from_string("task-123".to_string());

        // Queue using backwards-compatible method
        let msg = queue.queue_for_task(task_id.clone(), "Task message".to_string());
        assert_eq!(msg.content, "Task message");

        // Should be accessible via both APIs
        assert_eq!(queue.get_queued_for_task(&task_id).len(), 1);
        assert_eq!(queue.get_queued(ChatContextType::TaskExecution, "task-123").len(), 1);

        // Pop using backwards-compatible method
        let popped = queue.pop_for_task(&task_id);
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().content, "Task message");
    }

    #[test]
    fn test_queue_key_convenience_methods() {
        let task_id = TaskId::from_string("task-1".to_string());

        let key1 = QueueKey::task_execution(&task_id);
        assert_eq!(key1.context_type, ChatContextType::TaskExecution);
        assert_eq!(key1.context_id, "task-1");

        let key2 = QueueKey::ideation("session-1");
        assert_eq!(key2.context_type, ChatContextType::Ideation);
        assert_eq!(key2.context_id, "session-1");

        let key3 = QueueKey::task("task-2");
        assert_eq!(key3.context_type, ChatContextType::Task);
        assert_eq!(key3.context_id, "task-2");

        let key4 = QueueKey::project("project-1");
        assert_eq!(key4.context_type, ChatContextType::Project);
        assert_eq!(key4.context_id, "project-1");
    }

    #[test]
    fn test_queued_message_creation() {
        let msg = QueuedMessage::new("Test content".to_string());

        assert!(!msg.id.is_empty());
        assert_eq!(msg.content, "Test content");
        assert!(!msg.created_at.is_empty());
        assert!(!msg.is_editing);

        // Verify timestamp is valid RFC3339
        chrono::DateTime::parse_from_rfc3339(&msg.created_at).expect("Valid RFC3339 timestamp");
    }

    #[test]
    fn test_clone_safety() {
        let queue1 = MessageQueue::new();
        let queue2 = queue1.clone();

        // Queue via queue1
        queue1.queue(ChatContextType::Ideation, "session-1", "Message".to_string());

        // Should be visible via queue2 (shared Arc)
        assert_eq!(queue2.get_queued(ChatContextType::Ideation, "session-1").len(), 1);

        // Pop via queue2
        let popped = queue2.pop(ChatContextType::Ideation, "session-1");
        assert!(popped.is_some());

        // Should be empty in both
        assert_eq!(queue1.get_queued(ChatContextType::Ideation, "session-1").len(), 0);
        assert_eq!(queue2.get_queued(ChatContextType::Ideation, "session-1").len(), 0);
    }

    #[test]
    fn test_with_key_methods() {
        let queue = MessageQueue::new();
        let key = QueueKey::ideation("session-1");

        // Queue with key
        let msg = queue.queue_with_key(key.clone(), "Message 1".to_string());
        assert_eq!(msg.content, "Message 1");

        // Get with key
        let queued = queue.get_queued_with_key(&key);
        assert_eq!(queued.len(), 1);

        // Pop with key
        let popped = queue.pop_with_key(&key);
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().content, "Message 1");

        // Should be empty
        assert!(queue.get_queued_with_key(&key).is_empty());
    }
}
