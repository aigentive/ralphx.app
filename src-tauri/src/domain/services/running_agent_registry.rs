// Running Agent Registry
//
// Tracks running agent processes so they can be stopped on demand.
// Uses process IDs (PIDs) to allow cross-thread process termination.
//
// Trait-based design allows SQLite persistence (production) or in-memory (tests).

use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

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
    /// Optional worktree path used as the agent's working directory
    pub worktree_path: Option<String>,
    /// Token for cooperative cancellation of the background async task
    pub cancellation_token: Option<CancellationToken>,
    /// Last time a stream event was received (throttled heartbeat, ~5s interval).
    /// Used by the reconciler to distinguish active agents from stale ones.
    pub last_active_at: Option<chrono::DateTime<chrono::Utc>>,
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
        worktree_path: Option<String>,
        cancellation_token: Option<CancellationToken>,
    );

    /// Unregister a running agent (called when agent completes or is stopped).
    ///
    /// Only removes the entry if the stored `agent_run_id` matches the caller's, so a
    /// finishing agent cannot accidentally delete a newer agent's slot for the same context.
    async fn unregister(
        &self,
        key: &RunningAgentKey,
        agent_run_id: &str,
    ) -> Option<RunningAgentInfo>;

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

    /// Update the last_active_at timestamp for a running agent (throttled heartbeat).
    /// Called from the streaming loop every ~5 seconds on any parsed event.
    async fn update_heartbeat(&self, key: &RunningAgentKey, at: chrono::DateTime<chrono::Utc>);

    /// Atomically check-and-register an agent slot.
    ///
    /// If no agent is registered for this key, inserts a placeholder (pid=0) and
    /// returns `Ok(())`. If an agent is already registered, returns `Err` with the
    /// existing agent's info. This prevents the TOCTOU race between separate
    /// `is_running()` + `register()` calls.
    ///
    /// After a successful `try_register`, call `update_agent_process()` once the
    /// CLI process has been spawned. On spawn failure, call `unregister()` to
    /// release the slot.
    async fn try_register(
        &self,
        key: RunningAgentKey,
        conversation_id: String,
        agent_run_id: String,
    ) -> Result<(), RunningAgentInfo>;

    /// Update process details for an already-registered agent.
    ///
    /// Called after the CLI process has been spawned to fill in the real PID,
    /// agent_run_id, worktree path, and cancellation token.
    ///
    /// If the placeholder row was pruned between `try_register` and this call
    /// (TOCTOU race with GC), re-inserts the full registration via INSERT OR REPLACE.
    ///
    /// Returns `Err` only if the DB operation itself fails.
    async fn update_agent_process(
        &self,
        key: &RunningAgentKey,
        pid: u32,
        conversation_id: &str,
        agent_run_id: &str,
        worktree_path: Option<String>,
        cancellation_token: Option<CancellationToken>,
    ) -> Result<(), String>;
}

/// Check if a process with the given PID is still alive.
///
/// Uses `kill -0` on Unix to probe without sending a signal.
/// Returns false if the process does not exist or we lack permissions.
/// Returns false for PID 0, which refers to the process group on Unix
/// and would incorrectly report as alive via `kill -0`.
pub fn is_process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    {
        let output = std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output();
        matches!(output, Ok(result) if result.status.success())
    }

    #[cfg(windows)]
    {
        let output = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/NH"])
            .output();
        matches!(output, Ok(result) if result.status.success()
            && !String::from_utf8_lossy(&result.stdout).contains("No tasks"))
    }
}

/// Send SIGTERM to a process and all its children (process tree kill).
///
/// On Unix: first kills children via `pkill -TERM -P <pid>`, then kills the parent.
/// This prevents orphaned child processes (e.g. MCP server nodes) from lingering.
pub fn kill_process(pid: u32) {
    #[cfg(unix)]
    {
        // Kill children first (MCP server nodes, etc.)
        let _ = std::process::Command::new("pkill")
            .args(["-TERM", "-P", &pid.to_string()])
            .output();

        // Then kill the parent process
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
        // /T flag kills the process tree on Windows
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .output();
    }
}

fn collect_pids_in_worktree(path: &Path) -> Result<Vec<u32>, String> {
    #[cfg(unix)]
    {
        let output = std::process::Command::new("lsof")
            .args(["-t", "+D", path.to_str().unwrap_or("")])
            .output()
            .map_err(|e| format!("lsof failure: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "lsof exited with {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let text = String::from_utf8_lossy(&output.stdout);
        let mut pids = Vec::new();
        for line in text.lines() {
            if let Ok(pid) = line.trim().parse::<u32>() {
                pids.push(pid);
            }
        }
        Ok(pids)
    }

    #[cfg(not(unix))]
    {
        Ok(Vec::new())
    }
}

pub fn kill_worktree_processes(path: &Path) {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    match collect_pids_in_worktree(&canonical) {
        Ok(pids) => {
            for pid in pids.into_iter().collect::<HashSet<_>>() {
                tracing::info!(
                    pid,
                    worktree = %canonical.display(),
                    "Killing lingering process from worktree"
                );
                kill_process(pid);
            }
        }
        Err(err) => {
            tracing::debug!(
                worktree = %canonical.display(),
                error = %err,
                "Could not enumerate processes under worktree"
            );
        }
    }
}

/// Async version of `kill_worktree_processes` that wraps the blocking `lsof +D`
/// call in `spawn_blocking` with a configurable timeout.
///
/// On large worktrees (especially those with `target/` directories), `lsof +D`
/// can block for minutes. This function prevents that from eating into the merge
/// deadline by bounding the scan.
///
/// On timeout, logs a warning and returns — this is non-fatal because agents
/// have already been killed by PID before this point.
pub async fn kill_worktree_processes_async(path: &Path, timeout_secs: u64) {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let display_path = canonical.display().to_string();
    let start = std::time::Instant::now();

    tracing::info!(
        worktree = %display_path,
        timeout_secs,
        "kill_worktree_processes_async: starting lsof scan"
    );

    let canonical_clone = canonical.clone();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        tokio::task::spawn_blocking(move || collect_pids_in_worktree(&canonical_clone)),
    )
    .await;

    match result {
        Ok(Ok(Ok(pids))) => {
            let unique_pids: HashSet<u32> = pids.into_iter().collect();
            let elapsed_ms = start.elapsed().as_millis();
            tracing::info!(
                worktree = %display_path,
                elapsed_ms,
                pid_count = unique_pids.len(),
                "kill_worktree_processes_async: lsof scan complete"
            );
            for pid in unique_pids {
                tracing::info!(
                    pid,
                    worktree = %display_path,
                    "Killing lingering process from worktree (async)"
                );
                kill_process(pid);
            }
        }
        Ok(Ok(Err(err))) => {
            let elapsed_ms = start.elapsed().as_millis();
            tracing::debug!(
                worktree = %display_path,
                elapsed_ms,
                error = %err,
                "kill_worktree_processes_async: could not enumerate processes"
            );
        }
        Ok(Err(join_err)) => {
            tracing::warn!(
                worktree = %display_path,
                error = %join_err,
                "kill_worktree_processes_async: spawn_blocking task panicked"
            );
        }
        Err(_) => {
            let elapsed_ms = start.elapsed().as_millis();
            tracing::warn!(
                worktree = %display_path,
                elapsed_ms,
                timeout_secs,
                "kill_worktree_processes_async: lsof scan timed out (non-fatal)"
            );
        }
    }
}

/// Kill orphaned MCP server processes from previous app sessions.
///
/// Pattern-matches on `node ... ralphx-mcp-server/build/index.js` to catch any
/// leaked processes that escaped PID-based tracking (e.g. app crash before registration).
/// Safe to call on startup — only kills ralphx MCP servers, not user processes.
pub fn kill_orphaned_mcp_servers() -> u32 {
    #[cfg(unix)]
    {
        // Find node processes running our MCP server
        let output = std::process::Command::new("pgrep")
            .args(["-f", "ralphx-mcp-server/build/index.js"])
            .output();

        match output {
            Ok(result) if result.status.success() => {
                let stdout = String::from_utf8_lossy(&result.stdout);
                let pids: Vec<&str> = stdout.trim().lines().collect();
                let count = pids.len() as u32;

                for pid_str in &pids {
                    if let Ok(pid) = pid_str.trim().parse::<u32>() {
                        tracing::info!(pid, "Killing orphaned MCP server process");
                        let _ = std::process::Command::new("kill")
                            .args(["-TERM", pid_str.trim()])
                            .output();
                    }
                }

                if count > 0 {
                    tracing::info!(count, "Killed orphaned ralphx MCP server processes");
                }
                count
            }
            _ => 0, // No matches or pgrep failed
        }
    }

    #[cfg(windows)]
    {
        // Windows: use wmic or tasklist to find matching processes
        // For now, rely on process tree kill (/T flag in taskkill)
        0
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
        worktree_path: Option<String>,
        cancellation_token: Option<CancellationToken>,
    ) {
        let info = RunningAgentInfo {
            pid,
            conversation_id,
            agent_run_id,
            started_at: chrono::Utc::now(),
            worktree_path,
            cancellation_token,
            last_active_at: None,
        };
        let mut agents = self.agents.lock().await;

        // Stop orphaned agent if one already exists for this key
        if let Some(existing) = agents.get(&key) {
            let old_pid = existing.pid;
            if old_pid != pid && is_process_alive(old_pid) {
                tracing::warn!(
                    old_pid,
                    new_pid = pid,
                    context_type = %key.context_type,
                    context_id = %key.context_id,
                    "Detected orphaned agent process — stopping before re-registration"
                );
                if let Some(ref token) = existing.cancellation_token {
                    token.cancel();
                }
                kill_process(old_pid);
            }
        }

        agents.insert(key, info);
    }

    async fn unregister(
        &self,
        key: &RunningAgentKey,
        agent_run_id: &str,
    ) -> Option<RunningAgentInfo> {
        let mut agents = self.agents.lock().await;
        if agents.get(key).map(|i| i.agent_run_id.as_str()) == Some(agent_run_id) {
            agents.remove(key)
        } else {
            None
        }
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
        let agent_run_id = {
            let agents = self.agents.lock().await;
            agents
                .get(key)
                .map(|i| i.agent_run_id.clone())
                .unwrap_or_default()
        };
        let info = self.unregister(key, &agent_run_id).await;

        if let Some(ref agent_info) = info {
            // Cancel the async task before killing the process
            if let Some(ref token) = agent_info.cancellation_token {
                token.cancel();
            }
            kill_process(agent_info.pid);
        }

        Ok(info)
    }

    async fn list_all(&self) -> Vec<(RunningAgentKey, RunningAgentInfo)> {
        let agents = self.agents.lock().await;
        agents.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
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

    async fn update_heartbeat(&self, key: &RunningAgentKey, at: chrono::DateTime<chrono::Utc>) {
        let mut agents = self.agents.lock().await;
        if let Some(info) = agents.get_mut(key) {
            info.last_active_at = Some(at);
        }
    }

    async fn try_register(
        &self,
        key: RunningAgentKey,
        conversation_id: String,
        agent_run_id: String,
    ) -> Result<(), RunningAgentInfo> {
        let mut agents = self.agents.lock().await;
        if let Some(existing) = agents.get(&key) {
            return Err(existing.clone());
        }
        agents.insert(
            key,
            RunningAgentInfo {
                pid: 0,
                conversation_id,
                agent_run_id,
                started_at: chrono::Utc::now(),
                worktree_path: None,
                cancellation_token: None,
                last_active_at: None,
            },
        );
        Ok(())
    }

    async fn update_agent_process(
        &self,
        key: &RunningAgentKey,
        pid: u32,
        conversation_id: &str,
        agent_run_id: &str,
        worktree_path: Option<String>,
        cancellation_token: Option<CancellationToken>,
    ) -> Result<(), String> {
        let mut agents = self.agents.lock().await;
        if let Some(info) = agents.get_mut(key) {
            info.pid = pid;
            info.agent_run_id = agent_run_id.to_string();
            info.worktree_path = worktree_path;
            info.cancellation_token = cancellation_token;
        } else {
            tracing::warn!(
                context_type = %key.context_type,
                context_id = %key.context_id,
                pid,
                "update_agent_process: entry pruned — re-inserting full registration"
            );
            agents.insert(
                key.clone(),
                RunningAgentInfo {
                    pid,
                    conversation_id: conversation_id.to_string(),
                    agent_run_id: agent_run_id.to_string(),
                    started_at: chrono::Utc::now(),
                    worktree_path,
                    cancellation_token,
                    last_active_at: None,
                },
            );
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "running_agent_registry_tests.rs"]
mod tests;
