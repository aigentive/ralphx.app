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
            // Emit status_changed event to frontend for real-time UI update
            if let Some(ref handle) = self.app_handle {
                exec.emit_status_changed(handle, "task_started");
            }
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
#[path = "spawner_tests.rs"]
mod tests;
