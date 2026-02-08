// Running Agent Registry
//
// Tracks running agent processes so they can be stopped on demand.
// Uses process IDs (PIDs) to allow cross-thread process termination.
//
// Trait-based design allows SQLite persistence (production) or in-memory (tests).

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::Mutex;

/// Key for identifying a running agent by context
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct RunningAgentKey {
    pub context_type: String,
    pub context_id: String,
}

impl RunningAgentKey {
    pub fn new(context_type: impl Into<String>, context_id: impl Into<String>) -> Self {
        Self {
            context_type: context_type.into(),
            context_id: context_id.into(),
        }
    }
}

/// Information about a running agent process
#[derive(Debug, Clone)]
pub struct RunningAgentInfo {
    /// Process ID of the running agent
    pub pid: u32,
    /// Conversation ID associated with this run
    pub conversation_id: String,
    /// Agent run ID for tracking
    pub agent_run_id: String,
    /// When the agent was started
    pub started_at: chrono::DateTime<chrono::Utc>,
}

/// Trait for tracking running agent processes.
///
/// Thread-safe registry that maps context keys to process information.
/// Implementations: MemoryRunningAgentRegistry (tests), SqliteRunningAgentRegistry (production).
#[async_trait]
pub trait RunningAgentRegistry: Send + Sync {
    /// Register a running agent process
    async fn register(
        &self,
        key: RunningAgentKey,
        pid: u32,
        conversation_id: String,
        agent_run_id: String,
    );

    /// Unregister a running agent (called when agent completes or is stopped)
    async fn unregister(&self, key: &RunningAgentKey) -> Option<RunningAgentInfo>;

    /// Get information about a running agent
    async fn get(&self, key: &RunningAgentKey) -> Option<RunningAgentInfo>;

    /// Check if an agent is running for a context
    async fn is_running(&self, key: &RunningAgentKey) -> bool;

    /// Stop a running agent by sending SIGTERM to the process
    async fn stop(&self, key: &RunningAgentKey) -> Result<Option<RunningAgentInfo>, String>;

    /// Get all running agents (for debugging/monitoring)
    async fn list_all(&self) -> Vec<(RunningAgentKey, RunningAgentInfo)>;

    /// Stop all running agents (for cleanup on shutdown/restart)
    async fn stop_all(&self) -> Vec<RunningAgentKey>;
}

/// Send SIGTERM to a process by PID. Shared helper for all implementations.
pub fn kill_process(pid: u32) {
    #[cfg(unix)]
    {
        let output = std::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .output();

        match output {
            Ok(result) => {
                if !result.status.success() {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    // "No such process" is fine - process already exited
                    if !stderr.contains("No such process") {
                        tracing::warn!("Failed to kill process {}: {}", pid, stderr);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to run kill command: {}", e);
            }
        }
    }

    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .output();
    }
}

/// In-memory implementation of RunningAgentRegistry (for tests)
#[derive(Debug, Clone)]
pub struct MemoryRunningAgentRegistry {
    agents: Arc<Mutex<HashMap<RunningAgentKey, RunningAgentInfo>>>,
}

impl Default for MemoryRunningAgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryRunningAgentRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            agents: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl RunningAgentRegistry for MemoryRunningAgentRegistry {
    async fn register(
        &self,
        key: RunningAgentKey,
        pid: u32,
        conversation_id: String,
        agent_run_id: String,
    ) {
        let info = RunningAgentInfo {
            pid,
            conversation_id,
            agent_run_id,
            started_at: chrono::Utc::now(),
        };
        let mut agents = self.agents.lock().await;
        agents.insert(key, info);
    }

    async fn unregister(&self, key: &RunningAgentKey) -> Option<RunningAgentInfo> {
        let mut agents = self.agents.lock().await;
        agents.remove(key)
    }

    async fn get(&self, key: &RunningAgentKey) -> Option<RunningAgentInfo> {
        let agents = self.agents.lock().await;
        agents.get(key).cloned()
    }

    async fn is_running(&self, key: &RunningAgentKey) -> bool {
        let agents = self.agents.lock().await;
        agents.contains_key(key)
    }

    async fn stop(&self, key: &RunningAgentKey) -> Result<Option<RunningAgentInfo>, String> {
        let info = self.unregister(key).await;

        if let Some(ref agent_info) = info {
            kill_process(agent_info.pid);
        }

        Ok(info)
    }

    async fn list_all(&self) -> Vec<(RunningAgentKey, RunningAgentInfo)> {
        let agents = self.agents.lock().await;
        agents
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    async fn stop_all(&self) -> Vec<RunningAgentKey> {
        let keys: Vec<RunningAgentKey> = {
            let agents = self.agents.lock().await;
            agents.keys().cloned().collect()
        };

        let mut stopped = Vec::new();
        for key in keys {
            if self.stop(&key).await.is_ok() {
                stopped.push(key);
            }
        }
        stopped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_and_get() {
        let registry = MemoryRunningAgentRegistry::new();
        let key = RunningAgentKey::new("ideation", "session-123");

        registry.register(
            key.clone(),
            12345,
            "conv-abc".to_string(),
            "run-xyz".to_string(),
        ).await;

        let info = registry.get(&key).await;
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.pid, 12345);
        assert_eq!(info.conversation_id, "conv-abc");
        assert_eq!(info.agent_run_id, "run-xyz");
    }

    #[tokio::test]
    async fn test_is_running() {
        let registry = MemoryRunningAgentRegistry::new();
        let key = RunningAgentKey::new("task", "task-123");

        assert!(!registry.is_running(&key).await);

        registry.register(
            key.clone(),
            12345,
            "conv-abc".to_string(),
            "run-xyz".to_string(),
        ).await;

        assert!(registry.is_running(&key).await);
    }

    #[tokio::test]
    async fn test_unregister() {
        let registry = MemoryRunningAgentRegistry::new();
        let key = RunningAgentKey::new("project", "proj-123");

        registry.register(
            key.clone(),
            12345,
            "conv-abc".to_string(),
            "run-xyz".to_string(),
        ).await;

        let info = registry.unregister(&key).await;
        assert!(info.is_some());

        assert!(!registry.is_running(&key).await);

        // Double unregister should return None
        let info = registry.unregister(&key).await;
        assert!(info.is_none());
    }

    #[tokio::test]
    async fn test_list_all() {
        let registry = MemoryRunningAgentRegistry::new();

        registry.register(
            RunningAgentKey::new("ideation", "session-1"),
            111,
            "conv-1".to_string(),
            "run-1".to_string(),
        ).await;

        registry.register(
            RunningAgentKey::new("task", "task-2"),
            222,
            "conv-2".to_string(),
            "run-2".to_string(),
        ).await;

        let all = registry.list_all().await;
        assert_eq!(all.len(), 2);
    }
}
