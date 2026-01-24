// AgenticClientSpawner
// Bridges the state machine's AgentSpawner trait to the AgenticClient

use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

use crate::domain::agents::{AgentConfig, AgenticClient, AgentRole};
use crate::domain::state_machine::AgentSpawner;

/// Bridge between the state machine's AgentSpawner and the AgenticClient
///
/// This allows the state machine to spawn agents without knowing
/// the implementation details of the agentic client.
pub struct AgenticClientSpawner {
    /// The underlying agentic client
    client: Arc<dyn AgenticClient>,
    /// Working directory for spawned agents
    working_directory: PathBuf,
}

impl AgenticClientSpawner {
    /// Create a new spawner with the given client
    pub fn new(client: Arc<dyn AgenticClient>) -> Self {
        Self {
            client,
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    /// Create with a specific working directory
    pub fn with_working_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.working_directory = path.into();
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
}

#[async_trait]
impl AgentSpawner for AgenticClientSpawner {
    async fn spawn(&self, agent_type: &str, task_id: &str) {
        let role = Self::role_from_string(agent_type);

        let config = AgentConfig {
            role,
            prompt: format!("Execute task {}", task_id),
            working_directory: self.working_directory.clone(),
            model: None,
            max_tokens: None,
            timeout_secs: None,
            env: std::collections::HashMap::new(),
        };

        // Spawn and ignore result (logging will happen via state machine hooks)
        let _ = self.client.spawn_agent(config).await;
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
}
