// Interactive Process Registry
//
// Maps running interactive Claude CLI processes by (context_type, context_id) to their
// stdin handle. When a message arrives for a context with a running interactive process,
// the message is written directly to stdin instead of spawning a new process.
//
// The Claude CLI handles internal queuing: messages sent to stdin while the agent is
// mid-turn are queued and processed after the current turn completes.

use std::collections::HashMap;
use tokio::io::AsyncWriteExt;
use tokio::process::ChildStdin;
use tokio::sync::Mutex;

/// Key for identifying an interactive process by context.
/// Reuses the same (context_type, context_id) pattern as RunningAgentKey.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct InteractiveProcessKey {
    pub context_type: String,
    pub context_id: String,
}

impl InteractiveProcessKey {
    pub fn new(context_type: impl Into<String>, context_id: impl Into<String>) -> Self {
        Self {
            context_type: context_type.into(),
            context_id: context_id.into(),
        }
    }
}

/// Registry for interactive CLI processes with open stdin handles.
///
/// Thread-safe: uses tokio::sync::Mutex for async-compatible locking.
/// ChildStdin is not Clone, so the registry owns it exclusively.
#[derive(Debug)]
pub struct InteractiveProcessRegistry {
    processes: Mutex<HashMap<InteractiveProcessKey, ChildStdin>>,
}

impl Default for InteractiveProcessRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl InteractiveProcessRegistry {
    pub fn new() -> Self {
        Self {
            processes: Mutex::new(HashMap::new()),
        }
    }

    /// Register a stdin handle for an interactive process.
    ///
    /// If a stdin already exists for this key, the old one is dropped (closes the pipe).
    pub async fn register(&self, key: InteractiveProcessKey, stdin: ChildStdin) {
        let mut processes = self.processes.lock().await;
        if processes.contains_key(&key) {
            tracing::warn!(
                context_type = %key.context_type,
                context_id = %key.context_id,
                "InteractiveProcessRegistry: replacing existing stdin for context"
            );
        }
        processes.insert(key, stdin);
    }

    /// Check if an interactive process exists for this context.
    pub async fn has_process(&self, key: &InteractiveProcessKey) -> bool {
        let processes = self.processes.lock().await;
        processes.contains_key(key)
    }

    /// Write a message to the stdin of a running interactive process.
    ///
    /// Returns Ok(()) if the write succeeded, Err if no process found or write failed.
    /// The Claude CLI reads stdin line-by-line in interactive mode, so messages
    /// should end with a newline (this method appends one if missing).
    pub async fn write_message(&self, key: &InteractiveProcessKey, message: &str) -> Result<(), String> {
        let mut processes = self.processes.lock().await;
        let stdin = processes.get_mut(key).ok_or_else(|| {
            format!(
                "No interactive process for {}/{}",
                key.context_type, key.context_id
            )
        })?;

        // Ensure message ends with newline for CLI's line-based stdin reader
        let msg = if message.ends_with('\n') {
            message.to_string()
        } else {
            format!("{}\n", message)
        };

        stdin.write_all(msg.as_bytes()).await.map_err(|e| {
            format!(
                "Failed to write to interactive process stdin for {}/{}: {}",
                key.context_type, key.context_id, e
            )
        })?;

        stdin.flush().await.map_err(|e| {
            format!(
                "Failed to flush interactive process stdin for {}/{}: {}",
                key.context_type, key.context_id, e
            )
        })
    }

    /// Remove and return the stdin handle for a context (e.g., on process exit).
    ///
    /// Dropping the returned ChildStdin closes the pipe, signaling EOF to the process.
    pub async fn remove(&self, key: &InteractiveProcessKey) -> Option<ChildStdin> {
        let mut processes = self.processes.lock().await;
        processes.remove(key)
    }

    /// Remove all registered processes.
    pub async fn clear(&self) {
        let mut processes = self.processes.lock().await;
        processes.clear();
    }

    /// Get the count of registered interactive processes.
    #[cfg(test)]
    pub async fn count(&self) -> usize {
        let processes = self.processes.lock().await;
        processes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_and_has_process() {
        let registry = InteractiveProcessRegistry::new();
        let key = InteractiveProcessKey::new("ideation", "session-123");
        assert!(!registry.has_process(&key).await);
        assert_eq!(registry.count().await, 0);
    }

    #[tokio::test]
    async fn test_remove_nonexistent_returns_none() {
        let registry = InteractiveProcessRegistry::new();
        let key = InteractiveProcessKey::new("ideation", "session-123");
        assert!(registry.remove(&key).await.is_none());
    }

    #[tokio::test]
    async fn test_write_message_no_process_returns_error() {
        let registry = InteractiveProcessRegistry::new();
        let key = InteractiveProcessKey::new("ideation", "session-123");
        let result = registry.write_message(&key, "hello").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No interactive process"));
    }

    #[tokio::test]
    async fn test_register_and_write_message() {
        // Create a real pipe to test write
        let (stdin, _child) = create_test_stdin().await;
        let registry = InteractiveProcessRegistry::new();
        let key = InteractiveProcessKey::new("task", "task-456");

        registry.register(key.clone(), stdin).await;
        assert!(registry.has_process(&key).await);
        assert_eq!(registry.count().await, 1);

        // Write should succeed
        let result = registry.write_message(&key, "test message").await;
        assert!(result.is_ok());

        // Remove
        let removed = registry.remove(&key).await;
        assert!(removed.is_some());
        assert!(!registry.has_process(&key).await);
    }

    #[tokio::test]
    async fn test_clear_removes_all() {
        let (stdin1, _child1) = create_test_stdin().await;
        let (stdin2, _child2) = create_test_stdin().await;
        let registry = InteractiveProcessRegistry::new();

        registry
            .register(InteractiveProcessKey::new("a", "1"), stdin1)
            .await;
        registry
            .register(InteractiveProcessKey::new("b", "2"), stdin2)
            .await;
        assert_eq!(registry.count().await, 2);

        registry.clear().await;
        assert_eq!(registry.count().await, 0);
    }

    /// Helper: create a real stdin pipe via `cat` subprocess for testing writes.
    async fn create_test_stdin() -> (ChildStdin, tokio::process::Child) {
        let mut child = tokio::process::Command::new("cat")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("failed to spawn cat");
        let stdin = child.stdin.take().expect("no stdin");
        (stdin, child)
    }
}
