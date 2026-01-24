// Mock service implementations for testing
// These implementations record calls for verification in tests

use super::services::{AgentSpawner, DependencyManager, EventEmitter, Notifier};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

/// Records a call to a service method
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceCall {
    pub method: String,
    pub args: Vec<String>,
}

impl ServiceCall {
    pub fn new(method: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            method: method.into(),
            args,
        }
    }
}

/// Mock implementation of AgentSpawner that records all calls.
#[derive(Debug, Default)]
pub struct MockAgentSpawner {
    calls: Arc<Mutex<Vec<ServiceCall>>>,
    /// If set, spawn will "fail" by not recording the call
    pub should_fail: Arc<Mutex<bool>>,
}

impl MockAgentSpawner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns all recorded calls
    pub fn get_calls(&self) -> Vec<ServiceCall> {
        self.calls.lock().unwrap().clone()
    }

    /// Clears all recorded calls
    pub fn clear(&self) {
        self.calls.lock().unwrap().clear();
    }

    /// Returns the number of spawn calls
    pub fn spawn_count(&self) -> usize {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|c| c.method == "spawn" || c.method == "spawn_background")
            .count()
    }

    /// Sets whether spawn should "fail"
    pub fn set_should_fail(&self, should_fail: bool) {
        *self.should_fail.lock().unwrap() = should_fail;
    }
}

#[async_trait]
impl AgentSpawner for MockAgentSpawner {
    async fn spawn(&self, agent_type: &str, task_id: &str) {
        if !*self.should_fail.lock().unwrap() {
            self.calls.lock().unwrap().push(ServiceCall::new(
                "spawn",
                vec![agent_type.to_string(), task_id.to_string()],
            ));
        }
    }

    async fn spawn_background(&self, agent_type: &str, task_id: &str) {
        if !*self.should_fail.lock().unwrap() {
            self.calls.lock().unwrap().push(ServiceCall::new(
                "spawn_background",
                vec![agent_type.to_string(), task_id.to_string()],
            ));
        }
    }

    async fn wait_for(&self, agent_type: &str, task_id: &str) {
        self.calls.lock().unwrap().push(ServiceCall::new(
            "wait_for",
            vec![agent_type.to_string(), task_id.to_string()],
        ));
    }

    async fn stop(&self, agent_type: &str, task_id: &str) {
        self.calls.lock().unwrap().push(ServiceCall::new(
            "stop",
            vec![agent_type.to_string(), task_id.to_string()],
        ));
    }
}

/// Mock implementation of EventEmitter that records all emitted events.
#[derive(Debug, Default)]
pub struct MockEventEmitter {
    events: Arc<Mutex<Vec<ServiceCall>>>,
}

impl MockEventEmitter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns all recorded events
    pub fn get_events(&self) -> Vec<ServiceCall> {
        self.events.lock().unwrap().clone()
    }

    /// Clears all recorded events
    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }

    /// Returns the number of events emitted
    pub fn event_count(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    /// Checks if a specific event type was emitted
    pub fn has_event(&self, event_type: &str) -> bool {
        self.events
            .lock()
            .unwrap()
            .iter()
            .any(|e| e.args.first().map(|s| s.as_str()) == Some(event_type))
    }
}

#[async_trait]
impl EventEmitter for MockEventEmitter {
    async fn emit(&self, event_type: &str, task_id: &str) {
        self.events.lock().unwrap().push(ServiceCall::new(
            "emit",
            vec![event_type.to_string(), task_id.to_string()],
        ));
    }

    async fn emit_with_payload(&self, event_type: &str, task_id: &str, payload: &str) {
        self.events.lock().unwrap().push(ServiceCall::new(
            "emit_with_payload",
            vec![
                event_type.to_string(),
                task_id.to_string(),
                payload.to_string(),
            ],
        ));
    }
}

/// Mock implementation of Notifier that records all notifications.
#[derive(Debug, Default)]
pub struct MockNotifier {
    notifications: Arc<Mutex<Vec<ServiceCall>>>,
}

impl MockNotifier {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns all recorded notifications
    pub fn get_notifications(&self) -> Vec<ServiceCall> {
        self.notifications.lock().unwrap().clone()
    }

    /// Clears all recorded notifications
    pub fn clear(&self) {
        self.notifications.lock().unwrap().clear();
    }

    /// Returns the number of notifications sent
    pub fn notification_count(&self) -> usize {
        self.notifications.lock().unwrap().len()
    }

    /// Checks if a specific notification type was sent
    pub fn has_notification(&self, notification_type: &str) -> bool {
        self.notifications
            .lock()
            .unwrap()
            .iter()
            .any(|n| n.args.first().map(|s| s.as_str()) == Some(notification_type))
    }
}

#[async_trait]
impl Notifier for MockNotifier {
    async fn notify(&self, notification_type: &str, task_id: &str) {
        self.notifications.lock().unwrap().push(ServiceCall::new(
            "notify",
            vec![notification_type.to_string(), task_id.to_string()],
        ));
    }

    async fn notify_with_message(&self, notification_type: &str, task_id: &str, message: &str) {
        self.notifications.lock().unwrap().push(ServiceCall::new(
            "notify_with_message",
            vec![
                notification_type.to_string(),
                task_id.to_string(),
                message.to_string(),
            ],
        ));
    }
}

/// Mock implementation of DependencyManager that tracks blocker state.
#[derive(Debug, Default)]
pub struct MockDependencyManager {
    calls: Arc<Mutex<Vec<ServiceCall>>>,
    /// Map of task_id -> list of blocking task IDs
    blockers: Arc<Mutex<std::collections::HashMap<String, Vec<String>>>>,
}

impl MockDependencyManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the blockers for a task (for test setup)
    pub fn set_blockers(&self, task_id: impl Into<String>, blocker_ids: Vec<String>) {
        self.blockers
            .lock()
            .unwrap()
            .insert(task_id.into(), blocker_ids);
    }

    /// Removes a blocker from a task
    pub fn remove_blocker(&self, task_id: &str, blocker_id: &str) {
        if let Some(blockers) = self.blockers.lock().unwrap().get_mut(task_id) {
            blockers.retain(|id| id != blocker_id);
        }
    }

    /// Returns all recorded calls
    pub fn get_calls(&self) -> Vec<ServiceCall> {
        self.calls.lock().unwrap().clone()
    }

    /// Clears all recorded calls
    pub fn clear(&self) {
        self.calls.lock().unwrap().clear();
    }
}

#[async_trait]
impl DependencyManager for MockDependencyManager {
    async fn unblock_dependents(&self, completed_task_id: &str) {
        self.calls.lock().unwrap().push(ServiceCall::new(
            "unblock_dependents",
            vec![completed_task_id.to_string()],
        ));

        // Remove this task as a blocker from all tasks
        let mut blockers = self.blockers.lock().unwrap();
        for (_, task_blockers) in blockers.iter_mut() {
            task_blockers.retain(|id| id != completed_task_id);
        }
    }

    async fn has_unresolved_blockers(&self, task_id: &str) -> bool {
        self.calls.lock().unwrap().push(ServiceCall::new(
            "has_unresolved_blockers",
            vec![task_id.to_string()],
        ));

        self.blockers
            .lock()
            .unwrap()
            .get(task_id)
            .map(|v| !v.is_empty())
            .unwrap_or(false)
    }

    async fn get_blocking_tasks(&self, task_id: &str) -> Vec<String> {
        self.calls.lock().unwrap().push(ServiceCall::new(
            "get_blocking_tasks",
            vec![task_id.to_string()],
        ));

        self.blockers
            .lock()
            .unwrap()
            .get(task_id)
            .cloned()
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================
    // MockAgentSpawner tests
    // ==================

    #[tokio::test]
    async fn test_mock_agent_spawner_records_spawn() {
        let spawner = MockAgentSpawner::new();
        spawner.spawn("worker", "task-123").await;

        let calls = spawner.get_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].method, "spawn");
        assert_eq!(calls[0].args, vec!["worker", "task-123"]);
    }

    #[tokio::test]
    async fn test_mock_agent_spawner_records_spawn_background() {
        let spawner = MockAgentSpawner::new();
        spawner.spawn_background("qa-prep", "task-456").await;

        let calls = spawner.get_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].method, "spawn_background");
    }

    #[tokio::test]
    async fn test_mock_agent_spawner_records_wait_for() {
        let spawner = MockAgentSpawner::new();
        spawner.wait_for("qa-prep", "task-789").await;

        let calls = spawner.get_calls();
        assert_eq!(calls[0].method, "wait_for");
    }

    #[tokio::test]
    async fn test_mock_agent_spawner_records_stop() {
        let spawner = MockAgentSpawner::new();
        spawner.stop("worker", "task-abc").await;

        let calls = spawner.get_calls();
        assert_eq!(calls[0].method, "stop");
    }

    #[tokio::test]
    async fn test_mock_agent_spawner_spawn_count() {
        let spawner = MockAgentSpawner::new();
        spawner.spawn("worker", "task-1").await;
        spawner.spawn_background("qa-prep", "task-2").await;
        spawner.wait_for("qa-prep", "task-2").await;

        assert_eq!(spawner.spawn_count(), 2);
    }

    #[tokio::test]
    async fn test_mock_agent_spawner_clear() {
        let spawner = MockAgentSpawner::new();
        spawner.spawn("worker", "task-1").await;
        assert_eq!(spawner.get_calls().len(), 1);

        spawner.clear();
        assert_eq!(spawner.get_calls().len(), 0);
    }

    #[tokio::test]
    async fn test_mock_agent_spawner_should_fail() {
        let spawner = MockAgentSpawner::new();
        spawner.set_should_fail(true);
        spawner.spawn("worker", "task-1").await;

        assert_eq!(spawner.get_calls().len(), 0);
    }

    // ==================
    // MockEventEmitter tests
    // ==================

    #[tokio::test]
    async fn test_mock_event_emitter_records_emit() {
        let emitter = MockEventEmitter::new();
        emitter.emit("task_started", "task-123").await;

        let events = emitter.get_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].method, "emit");
        assert_eq!(events[0].args, vec!["task_started", "task-123"]);
    }

    #[tokio::test]
    async fn test_mock_event_emitter_records_emit_with_payload() {
        let emitter = MockEventEmitter::new();
        emitter
            .emit_with_payload("task_blocked", "task-456", r#"{"blocker":"task-1"}"#)
            .await;

        let events = emitter.get_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].method, "emit_with_payload");
        assert_eq!(events[0].args.len(), 3);
    }

    #[tokio::test]
    async fn test_mock_event_emitter_event_count() {
        let emitter = MockEventEmitter::new();
        emitter.emit("event1", "task-1").await;
        emitter.emit("event2", "task-2").await;

        assert_eq!(emitter.event_count(), 2);
    }

    #[tokio::test]
    async fn test_mock_event_emitter_has_event() {
        let emitter = MockEventEmitter::new();
        emitter.emit("task_started", "task-1").await;

        assert!(emitter.has_event("task_started"));
        assert!(!emitter.has_event("task_completed"));
    }

    #[tokio::test]
    async fn test_mock_event_emitter_clear() {
        let emitter = MockEventEmitter::new();
        emitter.emit("event", "task-1").await;
        emitter.clear();

        assert_eq!(emitter.event_count(), 0);
    }

    // ==================
    // MockNotifier tests
    // ==================

    #[tokio::test]
    async fn test_mock_notifier_records_notify() {
        let notifier = MockNotifier::new();
        notifier.notify("task_failed", "task-123").await;

        let notifications = notifier.get_notifications();
        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0].method, "notify");
    }

    #[tokio::test]
    async fn test_mock_notifier_records_notify_with_message() {
        let notifier = MockNotifier::new();
        notifier
            .notify_with_message("qa_failed", "task-456", "3 tests failed")
            .await;

        let notifications = notifier.get_notifications();
        assert_eq!(notifications[0].args.len(), 3);
    }

    #[tokio::test]
    async fn test_mock_notifier_notification_count() {
        let notifier = MockNotifier::new();
        notifier.notify("n1", "task-1").await;
        notifier.notify("n2", "task-2").await;

        assert_eq!(notifier.notification_count(), 2);
    }

    #[tokio::test]
    async fn test_mock_notifier_has_notification() {
        let notifier = MockNotifier::new();
        notifier.notify("task_failed", "task-1").await;

        assert!(notifier.has_notification("task_failed"));
        assert!(!notifier.has_notification("qa_failed"));
    }

    #[tokio::test]
    async fn test_mock_notifier_clear() {
        let notifier = MockNotifier::new();
        notifier.notify("n", "task-1").await;
        notifier.clear();

        assert_eq!(notifier.notification_count(), 0);
    }

    // ==================
    // MockDependencyManager tests
    // ==================

    #[tokio::test]
    async fn test_mock_dependency_manager_no_blockers() {
        let manager = MockDependencyManager::new();
        assert!(!manager.has_unresolved_blockers("task-1").await);
    }

    #[tokio::test]
    async fn test_mock_dependency_manager_with_blockers() {
        let manager = MockDependencyManager::new();
        manager.set_blockers("task-1", vec!["task-2".to_string(), "task-3".to_string()]);

        assert!(manager.has_unresolved_blockers("task-1").await);
        let blockers = manager.get_blocking_tasks("task-1").await;
        assert_eq!(blockers.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_dependency_manager_unblock_dependents() {
        let manager = MockDependencyManager::new();
        manager.set_blockers("task-1", vec!["task-2".to_string()]);

        assert!(manager.has_unresolved_blockers("task-1").await);
        manager.unblock_dependents("task-2").await;
        assert!(!manager.has_unresolved_blockers("task-1").await);
    }

    #[tokio::test]
    async fn test_mock_dependency_manager_remove_specific_blocker() {
        let manager = MockDependencyManager::new();
        manager.set_blockers("task-1", vec!["task-2".to_string(), "task-3".to_string()]);

        manager.remove_blocker("task-1", "task-2");
        let blockers = manager.get_blocking_tasks("task-1").await;
        assert_eq!(blockers, vec!["task-3"]);
    }

    #[tokio::test]
    async fn test_mock_dependency_manager_records_calls() {
        let manager = MockDependencyManager::new();
        let _ = manager.has_unresolved_blockers("task-1").await;
        let _ = manager.get_blocking_tasks("task-1").await;
        manager.unblock_dependents("task-2").await;

        let calls = manager.get_calls();
        assert_eq!(calls.len(), 3);
    }

    // ==================
    // ServiceCall tests
    // ==================

    #[test]
    fn test_service_call_new() {
        let call = ServiceCall::new("method", vec!["arg1".to_string(), "arg2".to_string()]);
        assert_eq!(call.method, "method");
        assert_eq!(call.args, vec!["arg1", "arg2"]);
    }

    #[test]
    fn test_service_call_equality() {
        let call1 = ServiceCall::new("m", vec!["a".to_string()]);
        let call2 = ServiceCall::new("m", vec!["a".to_string()]);
        let call3 = ServiceCall::new("m", vec!["b".to_string()]);

        assert_eq!(call1, call2);
        assert_ne!(call1, call3);
    }

    #[test]
    fn test_service_call_clone() {
        let call = ServiceCall::new("method", vec!["arg".to_string()]);
        let cloned = call.clone();
        assert_eq!(call, cloned);
    }

    #[test]
    fn test_service_call_debug() {
        let call = ServiceCall::new("test", vec![]);
        let debug_str = format!("{:?}", call);
        assert!(debug_str.contains("ServiceCall"));
    }
}
