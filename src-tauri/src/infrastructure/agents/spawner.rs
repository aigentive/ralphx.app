// AgenticClientSpawner
// Bridges the state machine's AgentSpawner trait to the AgenticClient

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

use tauri::{AppHandle, Wry};

use crate::commands::ExecutionState;
use crate::domain::agents::{AgentConfig, AgentHandle, AgenticClient, AgentRole};
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
    /// Tracks active agent handles by task_id for wait/stop operations
    handles: Arc<Mutex<HashMap<String, AgentHandle>>>,
    /// Execution state for spawn gating (optional)
    execution_state: Option<Arc<ExecutionState>>,
    /// Tauri app handle for emitting events to frontend (optional)
    app_handle: Option<AppHandle<Wry>>,
}

impl AgenticClientSpawner {
    /// Create a new spawner with the given client
    pub fn new(client: Arc<dyn AgenticClient>) -> Self {
        // Working directory should be project root (parent of src-tauri), not src-tauri itself
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let working_directory = cwd
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or(cwd);

        Self {
            client,
            working_directory,
            event_bus: None,
            handles: Arc::new(Mutex::new(HashMap::new())),
            execution_state: None,
            app_handle: None,
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

    /// Attach execution state for spawn gating
    pub fn with_execution_state(mut self, state: Arc<ExecutionState>) -> Self {
        self.execution_state = Some(state);
        self
    }

    /// Attach Tauri app handle for event emission
    pub fn with_app_handle(mut self, handle: AppHandle<Wry>) -> Self {
        self.app_handle = Some(handle);
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
        // Check execution state before spawning
        if let Some(ref exec) = self.execution_state {
            if !exec.can_start_task() {
                info!(
                    task_id = task_id,
                    agent_type = agent_type,
                    is_paused = exec.is_paused(),
                    running_count = exec.running_count(),
                    max_concurrent = exec.max_concurrent(),
                    "Spawn blocked: execution paused or at max concurrent"
                );
                return;
            }
            // Increment running count before spawning
            exec.increment_running();
        }

        // Emit TaskStart event before spawning
        self.emit_task_start(task_id, agent_type);

        let role = Self::role_from_string(agent_type);

        // Plugin dir is relative to working directory (which is now project root)
        let plugin_dir = self.working_directory.join("ralphx-plugin");

        let config = AgentConfig {
            role,
            prompt: format!("Execute task {}", task_id),
            working_directory: self.working_directory.clone(),
            plugin_dir: Some(plugin_dir),
            agent: Some(agent_type.to_string()),
            model: None,
            max_tokens: None,
            timeout_secs: None,
            env: std::collections::HashMap::new(),
        };

        // Spawn and handle errors
        match self.client.spawn_agent(config).await {
            Ok(handle) => {
                // Store handle for wait/stop operations
                let mut handles = self.handles.lock().await;
                handles.insert(task_id.to_string(), handle);
            }
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

    async fn wait_for(&self, _agent_type: &str, task_id: &str) {
        // Remove handle when done waiting (agent has completed)
        let handle = {
            let mut handles = self.handles.lock().await;
            handles.remove(task_id)
        };

        if let Some(handle) = handle {
            // Wait for agent to complete
            if let Err(e) = self.client.wait_for_completion(&handle).await {
                self.emit_error(task_id, ErrorInfo::new(e.to_string(), "wait_for"));
            }
        }
    }

    async fn stop(&self, _agent_type: &str, task_id: &str) {
        let handle = {
            let mut handles = self.handles.lock().await;
            handles.remove(task_id)
        };

        if let Some(handle) = handle {
            if let Err(e) = self.client.stop_agent(&handle).await {
                self.emit_error(task_id, ErrorInfo::new(e.to_string(), "stop_agent"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::ExecutionState;
    use crate::infrastructure::MockAgenticClient;

    #[test]
    fn test_role_from_string() {
        // Standard roles
        assert_eq!(AgenticClientSpawner::role_from_string("worker"), AgentRole::Worker);
        assert_eq!(AgenticClientSpawner::role_from_string("qa-prep"), AgentRole::QaPrep);
        assert_eq!(AgenticClientSpawner::role_from_string("qa-refiner"), AgentRole::QaRefiner);
        assert_eq!(AgenticClientSpawner::role_from_string("qa-tester"), AgentRole::QaTester);
        assert_eq!(AgenticClientSpawner::role_from_string("reviewer"), AgentRole::Reviewer);
        assert_eq!(AgenticClientSpawner::role_from_string("supervisor"), AgentRole::Supervisor);
        // Custom role
        assert_eq!(AgenticClientSpawner::role_from_string("my-custom-agent"), AgentRole::Custom("my-custom-agent".to_string()));
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

    // ==================== Execution State Tests ====================

    #[tokio::test]
    async fn test_with_execution_state() {
        let mock = Arc::new(MockAgenticClient::new());
        let exec_state = Arc::new(ExecutionState::new());
        let spawner =
            AgenticClientSpawner::new(mock.clone()).with_execution_state(exec_state.clone());

        assert!(spawner.execution_state.is_some());
    }

    #[tokio::test]
    async fn test_spawn_blocked_when_paused() {
        let mock = Arc::new(MockAgenticClient::new());
        let exec_state = Arc::new(ExecutionState::new());

        // Pause execution
        exec_state.pause();

        let spawner =
            AgenticClientSpawner::new(mock.clone()).with_execution_state(exec_state.clone());

        // Try to spawn while paused
        spawner.spawn("worker", "task-123").await;

        // Verify no spawn occurred
        let calls = mock.get_spawn_calls().await;
        assert_eq!(calls.len(), 0, "Should not spawn when paused");

        // Running count should not have incremented
        assert_eq!(exec_state.running_count(), 0);
    }

    #[tokio::test]
    async fn test_spawn_blocked_at_max_concurrent() {
        let mock = Arc::new(MockAgenticClient::new());
        let exec_state = Arc::new(ExecutionState::with_max_concurrent(2));

        // Fill up to max concurrent
        exec_state.increment_running();
        exec_state.increment_running();

        let spawner =
            AgenticClientSpawner::new(mock.clone()).with_execution_state(exec_state.clone());

        // Try to spawn at max concurrent
        spawner.spawn("worker", "task-123").await;

        // Verify no spawn occurred
        let calls = mock.get_spawn_calls().await;
        assert_eq!(calls.len(), 0, "Should not spawn when at max concurrent");

        // Running count should still be 2 (not incremented)
        assert_eq!(exec_state.running_count(), 2);
    }

    #[tokio::test]
    async fn test_spawn_increments_running_count() {
        let mock = Arc::new(MockAgenticClient::new());
        let exec_state = Arc::new(ExecutionState::with_max_concurrent(5));

        let spawner =
            AgenticClientSpawner::new(mock.clone()).with_execution_state(exec_state.clone());

        // Verify initial state
        assert_eq!(exec_state.running_count(), 0);

        // Spawn a task
        spawner.spawn("worker", "task-1").await;

        // Verify running count incremented
        assert_eq!(exec_state.running_count(), 1);

        // Spawn another task
        spawner.spawn("reviewer", "task-2").await;

        // Verify running count incremented again
        assert_eq!(exec_state.running_count(), 2);

        // Verify spawns actually occurred
        let calls = mock.get_spawn_calls().await;
        assert_eq!(calls.len(), 2);
    }

    #[tokio::test]
    async fn test_spawn_without_execution_state_still_works() {
        let mock = Arc::new(MockAgenticClient::new());
        // No execution state attached
        let spawner = AgenticClientSpawner::new(mock.clone());

        // Should still spawn normally
        spawner.spawn("worker", "task-123").await;

        let calls = mock.get_spawn_calls().await;
        assert_eq!(calls.len(), 1);
    }

    // ==================== App Handle Tests ====================

    #[test]
    fn test_app_handle_defaults_to_none() {
        let mock = Arc::new(MockAgenticClient::new());
        let spawner = AgenticClientSpawner::new(mock.clone());

        // By default, app_handle should be None
        assert!(spawner.app_handle.is_none());
    }

    #[test]
    fn test_app_handle_field_accessible() {
        let mock = Arc::new(MockAgenticClient::new());
        let spawner = AgenticClientSpawner::new(mock.clone());

        // Verify app_handle can be accessed (compile-time check + runtime assertion)
        // Note: with_app_handle() requires a real AppHandle<Wry> which is not available in tests,
        // but we verify the field exists and defaults correctly.
        let _handle: &Option<AppHandle<Wry>> = &spawner.app_handle;
        assert!(spawner.app_handle.is_none());
    }
}
