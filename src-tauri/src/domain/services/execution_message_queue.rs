// Execution Message Queue Service
//
// Manages queued messages for worker executions. Messages are held in memory
// and sent when the worker finishes its current response.

use crate::domain::entities::types::TaskId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// A queued message waiting to be sent to a worker
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
}

/// In-memory queue for execution messages
///
/// Stores queued messages per task_id. Messages are ephemeral and lost on app restart.
/// This is intentional - queued messages are short-lived and should be sent when the
/// worker responds. If the app restarts, the user can re-type their message.
#[derive(Debug, Clone)]
pub struct ExecutionMessageQueue {
    queues: Arc<Mutex<HashMap<TaskId, Vec<QueuedMessage>>>>,
}

impl ExecutionMessageQueue {
    /// Create a new empty queue
    pub fn new() -> Self {
        Self {
            queues: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Queue a message for a task
    pub fn queue(&self, task_id: TaskId, content: String) -> QueuedMessage {
        let message = QueuedMessage::new(content);
        let mut queues = self.queues.lock().unwrap();
        queues
            .entry(task_id)
            .or_default()
            .push(message.clone());
        message
    }

    /// Pop the next message from the queue (FIFO)
    pub fn pop(&self, task_id: &TaskId) -> Option<QueuedMessage> {
        let mut queues = self.queues.lock().unwrap();
        queues.get_mut(task_id).and_then(|queue| {
            if queue.is_empty() {
                None
            } else {
                Some(queue.remove(0))
            }
        })
    }

    /// Get all queued messages for a task (without removing them)
    pub fn get_queued(&self, task_id: &TaskId) -> Vec<QueuedMessage> {
        let queues = self.queues.lock().unwrap();
        queues.get(task_id).cloned().unwrap_or_default()
    }

    /// Clear all queued messages for a task
    pub fn clear(&self, task_id: &TaskId) {
        let mut queues = self.queues.lock().unwrap();
        queues.remove(task_id);
    }

    /// Delete a specific queued message by ID
    pub fn delete(&self, task_id: &TaskId, message_id: &str) -> bool {
        let mut queues = self.queues.lock().unwrap();
        if let Some(queue) = queues.get_mut(task_id) {
            if let Some(pos) = queue.iter().position(|m| m.id == message_id) {
                queue.remove(pos);
                return true;
            }
        }
        false
    }
}

impl Default for ExecutionMessageQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_task_id(id: &str) -> TaskId {
        TaskId(id.to_string())
    }

    #[test]
    fn test_queue_and_pop() {
        let queue = ExecutionMessageQueue::new();
        let task_id = create_task_id("task-1");

        // Queue two messages
        let _msg1 = queue.queue(task_id.clone(), "First message".to_string());
        let _msg2 = queue.queue(task_id.clone(), "Second message".to_string());

        // Pop should return in FIFO order
        let popped1 = queue.pop(&task_id);
        assert!(popped1.is_some());
        assert_eq!(popped1.unwrap().content, "First message");

        let popped2 = queue.pop(&task_id);
        assert!(popped2.is_some());
        assert_eq!(popped2.unwrap().content, "Second message");

        // Queue should be empty now
        let popped3 = queue.pop(&task_id);
        assert!(popped3.is_none());
    }

    #[test]
    fn test_get_queued() {
        let queue = ExecutionMessageQueue::new();
        let task_id = create_task_id("task-1");

        // Initially empty
        assert_eq!(queue.get_queued(&task_id).len(), 0);

        // Queue two messages
        queue.queue(task_id.clone(), "First".to_string());
        queue.queue(task_id.clone(), "Second".to_string());

        // get_queued should return all messages without removing
        let queued = queue.get_queued(&task_id);
        assert_eq!(queued.len(), 2);
        assert_eq!(queued[0].content, "First");
        assert_eq!(queued[1].content, "Second");

        // Messages should still be in queue
        assert_eq!(queue.get_queued(&task_id).len(), 2);
    }

    #[test]
    fn test_clear() {
        let queue = ExecutionMessageQueue::new();
        let task_id = create_task_id("task-1");

        queue.queue(task_id.clone(), "Message 1".to_string());
        queue.queue(task_id.clone(), "Message 2".to_string());

        assert_eq!(queue.get_queued(&task_id).len(), 2);

        queue.clear(&task_id);

        assert_eq!(queue.get_queued(&task_id).len(), 0);
        assert!(queue.pop(&task_id).is_none());
    }

    #[test]
    fn test_delete() {
        let queue = ExecutionMessageQueue::new();
        let task_id = create_task_id("task-1");

        let _msg1 = queue.queue(task_id.clone(), "First".to_string());
        let msg2 = queue.queue(task_id.clone(), "Second".to_string());
        let _msg3 = queue.queue(task_id.clone(), "Third".to_string());

        assert_eq!(queue.get_queued(&task_id).len(), 3);

        // Delete middle message
        let deleted = queue.delete(&task_id, &msg2.id);
        assert!(deleted);

        let remaining = queue.get_queued(&task_id);
        assert_eq!(remaining.len(), 2);
        assert_eq!(remaining[0].content, "First");
        assert_eq!(remaining[1].content, "Third");

        // Try deleting non-existent message
        let deleted = queue.delete(&task_id, "non-existent-id");
        assert!(!deleted);
    }

    #[test]
    fn test_multiple_tasks() {
        let queue = ExecutionMessageQueue::new();
        let task1 = create_task_id("task-1");
        let task2 = create_task_id("task-2");

        queue.queue(task1.clone(), "Task 1 Message 1".to_string());
        queue.queue(task2.clone(), "Task 2 Message 1".to_string());
        queue.queue(task1.clone(), "Task 1 Message 2".to_string());

        assert_eq!(queue.get_queued(&task1).len(), 2);
        assert_eq!(queue.get_queued(&task2).len(), 1);

        // Pop from task1 should not affect task2
        let popped = queue.pop(&task1);
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().content, "Task 1 Message 1");

        assert_eq!(queue.get_queued(&task1).len(), 1);
        assert_eq!(queue.get_queued(&task2).len(), 1);
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
        let queue1 = ExecutionMessageQueue::new();
        let queue2 = queue1.clone();
        let task_id = create_task_id("task-1");

        // Queue via queue1
        queue1.queue(task_id.clone(), "Message".to_string());

        // Should be visible via queue2 (shared Arc)
        assert_eq!(queue2.get_queued(&task_id).len(), 1);

        // Pop via queue2
        let popped = queue2.pop(&task_id);
        assert!(popped.is_some());

        // Should be empty in both
        assert_eq!(queue1.get_queued(&task_id).len(), 0);
        assert_eq!(queue2.get_queued(&task_id).len(), 0);
    }
}
