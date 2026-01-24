// Service traits for state machine dependency injection
// These traits define the interfaces for services used by the state machine
// during state transitions. Actual implementations are provided in Phase 4+.

use async_trait::async_trait;

/// Trait for spawning and managing AI agents.
///
/// The state machine uses this trait to spawn worker, reviewer, and QA agents
/// during state transitions. The actual implementation connects to Claude Code
/// or other agent providers.
///
/// Implementations are provided in Phase 4 (Agentic Client).
#[async_trait]
pub trait AgentSpawner: Send + Sync {
    /// Spawns an agent synchronously and waits for it to complete.
    ///
    /// # Arguments
    /// * `agent_type` - The type of agent to spawn (e.g., "worker", "reviewer", "qa-prep")
    /// * `task_id` - The ID of the task the agent should work on
    async fn spawn(&self, agent_type: &str, task_id: &str);

    /// Spawns an agent in the background without waiting.
    ///
    /// Used for QA prep which runs in parallel with execution.
    ///
    /// # Arguments
    /// * `agent_type` - The type of agent to spawn
    /// * `task_id` - The ID of the task the agent should work on
    async fn spawn_background(&self, agent_type: &str, task_id: &str);

    /// Waits for a background agent to complete.
    ///
    /// # Arguments
    /// * `agent_type` - The type of agent to wait for
    /// * `task_id` - The task ID the agent is working on
    async fn wait_for(&self, agent_type: &str, task_id: &str);

    /// Stops a running agent.
    ///
    /// # Arguments
    /// * `agent_type` - The type of agent to stop
    /// * `task_id` - The task ID the agent is working on
    async fn stop(&self, agent_type: &str, task_id: &str);
}

/// Trait for emitting events to the frontend.
///
/// The state machine emits events on state transitions that the frontend
/// uses to update the UI in real-time. Events are sent via Tauri's event system.
///
/// Implementations are provided in Phase 5 (Frontend Core).
#[async_trait]
pub trait EventEmitter: Send + Sync {
    /// Emits an event for a task state change.
    ///
    /// # Arguments
    /// * `event_type` - The type of event (e.g., "task_started", "task_blocked", "qa_passed")
    /// * `task_id` - The ID of the task that triggered the event
    async fn emit(&self, event_type: &str, task_id: &str);

    /// Emits an event with additional payload data.
    ///
    /// # Arguments
    /// * `event_type` - The type of event
    /// * `task_id` - The ID of the task
    /// * `payload` - JSON string with additional event data
    async fn emit_with_payload(&self, event_type: &str, task_id: &str, payload: &str);
}

/// Trait for sending notifications to users.
///
/// Used for important events that require user attention, such as:
/// - Task failures that need manual intervention
/// - QA failures that might need review
/// - Tasks blocked waiting for human input
///
/// Implementations are provided in Phase 9 (Review & Supervision).
#[async_trait]
pub trait Notifier: Send + Sync {
    /// Sends a notification to the user.
    ///
    /// # Arguments
    /// * `notification_type` - The type of notification (e.g., "task_failed", "qa_failed", "needs_input")
    /// * `task_id` - The ID of the task that triggered the notification
    async fn notify(&self, notification_type: &str, task_id: &str);

    /// Sends a notification with a custom message.
    ///
    /// # Arguments
    /// * `notification_type` - The type of notification
    /// * `task_id` - The ID of the task
    /// * `message` - The notification message
    async fn notify_with_message(&self, notification_type: &str, task_id: &str, message: &str);
}

/// Trait for managing dependent tasks.
///
/// When a task completes, we may need to unblock dependent tasks.
/// This trait provides methods for managing task dependencies.
#[async_trait]
pub trait DependencyManager: Send + Sync {
    /// Resolves blockers for tasks that were waiting on the completed task.
    ///
    /// # Arguments
    /// * `completed_task_id` - The ID of the task that just completed
    async fn unblock_dependents(&self, completed_task_id: &str);

    /// Checks if a task has any unresolved blockers.
    ///
    /// # Arguments
    /// * `task_id` - The ID of the task to check
    async fn has_unresolved_blockers(&self, task_id: &str) -> bool;

    /// Gets the IDs of tasks blocking the given task.
    ///
    /// # Arguments
    /// * `task_id` - The ID of the task to check
    async fn get_blocking_tasks(&self, task_id: &str) -> Vec<String>;
}

/// Result of starting a review.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewStartResult {
    /// Review started successfully with the review ID
    Started { review_id: String },
    /// Review was not started because AI review is disabled
    Disabled,
    /// Review failed to start due to an error
    Error(String),
}

/// Trait for starting reviews on tasks.
///
/// The state machine uses this trait to initiate AI reviews when
/// a task enters the PendingReview state. The actual implementation
/// connects to the ReviewService.
///
/// Implementations are provided in Phase 9 (Review & Supervision).
#[async_trait]
pub trait ReviewStarter: Send + Sync {
    /// Starts an AI review for a task.
    ///
    /// Creates a Review record and prepares for AI review.
    /// The actual reviewer agent is spawned separately via AgentSpawner.
    ///
    /// # Arguments
    /// * `task_id` - The ID of the task to review
    /// * `project_id` - The ID of the project the task belongs to
    ///
    /// # Returns
    /// * `ReviewStartResult::Started` - Review started with review ID
    /// * `ReviewStartResult::Disabled` - AI review is disabled in settings
    /// * `ReviewStartResult::Error` - Failed to start review
    async fn start_ai_review(&self, task_id: &str, project_id: &str) -> ReviewStartResult;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // Test that traits are object-safe
    #[test]
    fn test_agent_spawner_is_object_safe() {
        fn _assert_object_safe(_: &dyn AgentSpawner) {}
    }

    #[test]
    fn test_event_emitter_is_object_safe() {
        fn _assert_object_safe(_: &dyn EventEmitter) {}
    }

    #[test]
    fn test_notifier_is_object_safe() {
        fn _assert_object_safe(_: &dyn Notifier) {}
    }

    #[test]
    fn test_dependency_manager_is_object_safe() {
        fn _assert_object_safe(_: &dyn DependencyManager) {}
    }

    #[test]
    fn test_review_starter_is_object_safe() {
        fn _assert_object_safe(_: &dyn ReviewStarter) {}
    }

    #[test]
    fn test_traits_can_be_wrapped_in_arc() {
        // This is important for sharing services across threads
        fn _takes_arc_spawner(_: Arc<dyn AgentSpawner>) {}
        fn _takes_arc_emitter(_: Arc<dyn EventEmitter>) {}
        fn _takes_arc_notifier(_: Arc<dyn Notifier>) {}
        fn _takes_arc_manager(_: Arc<dyn DependencyManager>) {}
        fn _takes_arc_review_starter(_: Arc<dyn ReviewStarter>) {}
    }

    #[test]
    fn test_traits_can_be_boxed() {
        fn _takes_box_spawner(_: Box<dyn AgentSpawner>) {}
        fn _takes_box_emitter(_: Box<dyn EventEmitter>) {}
        fn _takes_box_notifier(_: Box<dyn Notifier>) {}
        fn _takes_box_manager(_: Box<dyn DependencyManager>) {}
        fn _takes_box_review_starter(_: Box<dyn ReviewStarter>) {}
    }

    // ReviewStartResult tests
    #[test]
    fn test_review_start_result_started() {
        let result = ReviewStartResult::Started {
            review_id: "rev-123".to_string(),
        };
        if let ReviewStartResult::Started { review_id } = result {
            assert_eq!(review_id, "rev-123");
        } else {
            panic!("Expected Started variant");
        }
    }

    #[test]
    fn test_review_start_result_disabled() {
        let result = ReviewStartResult::Disabled;
        assert_eq!(result, ReviewStartResult::Disabled);
    }

    #[test]
    fn test_review_start_result_error() {
        let result = ReviewStartResult::Error("Something failed".to_string());
        if let ReviewStartResult::Error(msg) = result {
            assert_eq!(msg, "Something failed");
        } else {
            panic!("Expected Error variant");
        }
    }

    #[test]
    fn test_review_start_result_clone() {
        let result = ReviewStartResult::Started {
            review_id: "rev-1".to_string(),
        };
        let cloned = result.clone();
        assert_eq!(result, cloned);
    }

    #[test]
    fn test_review_start_result_debug() {
        let result = ReviewStartResult::Started {
            review_id: "rev-1".to_string(),
        };
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("Started"));
        assert!(debug_str.contains("rev-1"));
    }
}
