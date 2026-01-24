// AgenticClientSpawner
// Bridges the state machine's AgentSpawner trait to the AgenticClient

use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

use crate::domain::agents::{AgentConfig, AgenticClient, AgentRole};
use crate::domain::state_machine::AgentSpawner;
use crate::domain::supervisor::{ErrorInfo, SupervisorEvent, ToolCallInfo};
use crate::infrastructure::supervisor::EventBus;

/// Bridge between the state machine's AgentSpawner and the AgenticClient
///
/// This allows the state machine to spawn agents without knowing
/// the implementation details of the agentic client.
pub struct AgenticClientSpawner {
    /// The underlying agentic client
    client: Arc<dyn AgenticClient>,
    /// Working directory for spawned agents
    working_directory: PathBuf,
    /// Event bus for supervisor events (optional)
    event_bus: Option<Arc<EventBus>>,
}

impl AgenticClientSpawner {
    /// Create a new spawner with the given client
    pub fn new(client: Arc<dyn AgenticClient>) -> Self {
        Self {
            client,
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            event_bus: None,
        }
    }

    /// Create with a specific working directory
    pub fn with_working_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.working_directory = path.into();
        self
    }

    /// Attach an event bus for supervisor events
    pub fn with_event_bus(mut self, event_bus: Arc<EventBus>) -> Self {
        self.event_bus = Some(event_bus);
        self
    }

    /// Map agent type string to AgentRole
    fn role_from_string(agent_type: &str) -> AgentRole {
        match agent_type {
            "worker" => AgentRole::Worker,
            "qa-prep" => AgentRole::QaPrep,
            "qa-refiner" => AgentRole::QaRefiner,
            "qa-tester" => AgentRole::QaTester,
            "reviewer" => AgentRole::Reviewer,
            "supervisor" => AgentRole::Supervisor,
            other => AgentRole::Custom(other.to_string()),
        }
    }

    /// Emit a TaskStart event if event bus is configured
    fn emit_task_start(&self, task_id: &str, agent_type: &str) {
        if let Some(ref event_bus) = self.event_bus {
            let event = SupervisorEvent::task_start(task_id, agent_type);
            // Ignore if no subscribers - that's okay
            let _ = event_bus.publish(event);
        }
    }

    /// Emit a ToolCall event if event bus is configured
    pub fn emit_tool_call(&self, task_id: &str, info: ToolCallInfo) {
        if let Some(ref event_bus) = self.event_bus {
            let event = SupervisorEvent::tool_call(task_id, info);
            let _ = event_bus.publish(event);
        }
    }

    /// Emit an Error event if event bus is configured
    pub fn emit_error(&self, task_id: &str, info: ErrorInfo) {
        if let Some(ref event_bus) = self.event_bus {
            let event = SupervisorEvent::error(task_id, info);
            let _ = event_bus.publish(event);
        }
    }

    /// Get a reference to the event bus if configured
    pub fn event_bus(&self) -> Option<&Arc<EventBus>> {
        self.event_bus.as_ref()
    }
}

#[async_trait]
impl AgentSpawner for AgenticClientSpawner {
    async fn spawn(&self, agent_type: &str, task_id: &str) {
        // Emit TaskStart event before spawning
        self.emit_task_start(task_id, agent_type);

        let role = Self::role_from_string(agent_type);

        let config = AgentConfig {
            role,
            prompt: format!("Execute task {}", task_id),
            working_directory: self.working_directory.clone(),
            plugin_dir: Some(std::path::PathBuf::from("./ralphx-plugin")),
            agent: Some(agent_type.to_string()),
            model: None,
            max_tokens: None,
            timeout_secs: None,
            env: std::collections::HashMap::new(),
        };

        // Spawn and handle errors
        match self.client.spawn_agent(config).await {
            Ok(_) => {}
            Err(e) => {
                // Emit error event
                self.emit_error(task_id, ErrorInfo::new(e.to_string(), "spawn_agent"));
            }
        }
    }

    async fn spawn_background(&self, agent_type: &str, task_id: &str) {
        // For background spawning, we just spawn without waiting
        self.spawn(agent_type, task_id).await;
    }

    async fn wait_for(&self, _agent_type: &str, _task_id: &str) {
        // TODO: Implement handle tracking to wait for specific agent
        // For now, this is a no-op since we don't track handles by task_id yet
    }

    async fn stop(&self, _agent_type: &str, _task_id: &str) {
        // TODO: Implement handle tracking to stop specific agent
        // For now, this is a no-op since we don't track handles by task_id yet
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::MockAgenticClient;

    #[test]
    fn test_role_from_string_worker() {
        let role = AgenticClientSpawner::role_from_string("worker");
        assert_eq!(role, AgentRole::Worker);
    }

    #[test]
    fn test_role_from_string_qa_prep() {
        let role = AgenticClientSpawner::role_from_string("qa-prep");
        assert_eq!(role, AgentRole::QaPrep);
    }

    #[test]
    fn test_role_from_string_qa_refiner() {
        let role = AgenticClientSpawner::role_from_string("qa-refiner");
        assert_eq!(role, AgentRole::QaRefiner);
    }

    #[test]
    fn test_role_from_string_qa_tester() {
        let role = AgenticClientSpawner::role_from_string("qa-tester");
        assert_eq!(role, AgentRole::QaTester);
    }

    #[test]
    fn test_role_from_string_reviewer() {
        let role = AgenticClientSpawner::role_from_string("reviewer");
        assert_eq!(role, AgentRole::Reviewer);
    }

    #[test]
    fn test_role_from_string_supervisor() {
        let role = AgenticClientSpawner::role_from_string("supervisor");
        assert_eq!(role, AgentRole::Supervisor);
    }

    #[test]
    fn test_role_from_string_custom() {
        let role = AgenticClientSpawner::role_from_string("my-custom-agent");
        assert_eq!(role, AgentRole::Custom("my-custom-agent".to_string()));
    }

    #[tokio::test]
    async fn test_spawn_calls_client() {
        let mock = Arc::new(MockAgenticClient::new());
        let spawner = AgenticClientSpawner::new(mock.clone());

        spawner.spawn("worker", "task-123").await;

        let calls = mock.get_spawn_calls().await;
        assert_eq!(calls.len(), 1);
    }

    #[tokio::test]
    async fn test_spawn_uses_correct_role() {
        let mock = Arc::new(MockAgenticClient::new());
        let spawner = AgenticClientSpawner::new(mock.clone());

        spawner.spawn("reviewer", "task-456").await;

        let calls = mock.get_spawn_calls().await;
        assert_eq!(calls.len(), 1);
        if let crate::infrastructure::MockCallType::Spawn { role, prompt } = &calls[0].call_type {
            assert_eq!(*role, AgentRole::Reviewer);
            assert!(prompt.contains("task-456"));
        } else {
            panic!("Expected Spawn call");
        }
    }

    #[tokio::test]
    async fn test_spawn_background_calls_client() {
        let mock = Arc::new(MockAgenticClient::new());
        let spawner = AgenticClientSpawner::new(mock.clone());

        spawner.spawn_background("qa-prep", "task-789").await;

        let calls = mock.get_spawn_calls().await;
        assert_eq!(calls.len(), 1);
    }

    #[tokio::test]
    async fn test_with_working_dir() {
        let mock = Arc::new(MockAgenticClient::new());
        let spawner =
            AgenticClientSpawner::new(mock.clone()).with_working_dir("/custom/work/dir");

        assert_eq!(spawner.working_directory, PathBuf::from("/custom/work/dir"));
    }

    #[tokio::test]
    async fn test_wait_for_is_noop() {
        let mock = Arc::new(MockAgenticClient::new());
        let spawner = AgenticClientSpawner::new(mock.clone());

        // Should not panic or error
        spawner.wait_for("worker", "task-123").await;
    }

    // ==================== Event Bus Tests ====================

    #[tokio::test]
    async fn test_with_event_bus() {
        let mock = Arc::new(MockAgenticClient::new());
        let event_bus = Arc::new(EventBus::new());
        let spawner = AgenticClientSpawner::new(mock.clone()).with_event_bus(event_bus.clone());

        assert!(spawner.event_bus().is_some());
    }

    #[tokio::test]
    async fn test_spawn_emits_task_start_event() {
        let mock = Arc::new(MockAgenticClient::new());
        let event_bus = Arc::new(EventBus::new());
        let mut subscriber = event_bus.subscribe();

        let spawner = AgenticClientSpawner::new(mock.clone()).with_event_bus(event_bus);

        spawner.spawn("worker", "task-123").await;

        // Check that TaskStart event was emitted
        let event = subscriber.try_recv().unwrap();
        if let SupervisorEvent::TaskStart {
            task_id,
            agent_role,
            ..
        } = event
        {
            assert_eq!(task_id, "task-123");
            assert_eq!(agent_role, "worker");
        } else {
            panic!("Expected TaskStart event, got {:?}", event);
        }
    }

    #[tokio::test]
    async fn test_emit_tool_call() {
        let mock = Arc::new(MockAgenticClient::new());
        let event_bus = Arc::new(EventBus::new());
        let mut subscriber = event_bus.subscribe();

        let spawner = AgenticClientSpawner::new(mock.clone()).with_event_bus(event_bus);

        let info = ToolCallInfo::new("Write", r#"{"path": "test.txt"}"#);
        spawner.emit_tool_call("task-123", info);

        let event = subscriber.try_recv().unwrap();
        if let SupervisorEvent::ToolCall { task_id, info } = event {
            assert_eq!(task_id, "task-123");
            assert_eq!(info.tool_name, "Write");
        } else {
            panic!("Expected ToolCall event, got {:?}", event);
        }
    }

    #[tokio::test]
    async fn test_emit_error() {
        let mock = Arc::new(MockAgenticClient::new());
        let event_bus = Arc::new(EventBus::new());
        let mut subscriber = event_bus.subscribe();

        let spawner = AgenticClientSpawner::new(mock.clone()).with_event_bus(event_bus);

        let info = ErrorInfo::new("Something went wrong", "test_source");
        spawner.emit_error("task-123", info);

        let event = subscriber.try_recv().unwrap();
        if let SupervisorEvent::Error { task_id, info } = event {
            assert_eq!(task_id, "task-123");
            assert_eq!(info.message, "Something went wrong");
            assert_eq!(info.source, "test_source");
        } else {
            panic!("Expected Error event, got {:?}", event);
        }
    }

    #[tokio::test]
    async fn test_spawn_without_event_bus_works() {
        let mock = Arc::new(MockAgenticClient::new());
        let spawner = AgenticClientSpawner::new(mock.clone());

        // Should not panic even without event bus
        spawner.spawn("worker", "task-123").await;

        let calls = mock.get_spawn_calls().await;
        assert_eq!(calls.len(), 1);
    }

    #[tokio::test]
    async fn test_emit_without_event_bus_is_noop() {
        let mock = Arc::new(MockAgenticClient::new());
        let spawner = AgenticClientSpawner::new(mock.clone());

        // Should not panic
        let info = ToolCallInfo::new("Read", "{}");
        spawner.emit_tool_call("task-123", info);

        let error_info = ErrorInfo::new("Test error", "test");
        spawner.emit_error("task-123", error_info);
    }

    #[tokio::test]
    async fn test_multiple_spawns_emit_multiple_events() {
        let mock = Arc::new(MockAgenticClient::new());
        let event_bus = Arc::new(EventBus::new());
        let mut subscriber = event_bus.subscribe();

        let spawner = AgenticClientSpawner::new(mock.clone()).with_event_bus(event_bus);

        spawner.spawn("worker", "task-1").await;
        spawner.spawn("reviewer", "task-2").await;
        spawner.spawn("supervisor", "task-3").await;

        // Check all three events
        let event1 = subscriber.try_recv().unwrap();
        let event2 = subscriber.try_recv().unwrap();
        let event3 = subscriber.try_recv().unwrap();

        assert_eq!(event1.task_id(), "task-1");
        assert_eq!(event2.task_id(), "task-2");
        assert_eq!(event3.task_id(), "task-3");
    }
}
