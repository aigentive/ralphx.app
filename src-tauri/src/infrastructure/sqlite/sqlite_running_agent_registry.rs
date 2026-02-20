// SQLite implementation of RunningAgentRegistry
//
// Persists running agent PIDs to the running_agents table so they survive app restarts.
// On restart, stop_all() kills orphaned processes before new agents are spawned.

use async_trait::async_trait;
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::domain::services::{
    is_process_alive, kill_process, kill_worktree_processes, RunningAgentInfo, RunningAgentKey,
    RunningAgentRegistry,
};

/// SQLite-backed implementation of RunningAgentRegistry.
/// Persists agent PIDs across app restarts for orphan cleanup.
pub struct SqliteRunningAgentRegistry {
    conn: Arc<Mutex<Connection>>,
    /// In-memory map for cancellation tokens (not persisted to SQLite)
    tokens: Arc<Mutex<HashMap<RunningAgentKey, CancellationToken>>>,
}

impl SqliteRunningAgentRegistry {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            conn,
            tokens: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl RunningAgentRegistry for SqliteRunningAgentRegistry {
    async fn register(
        &self,
        key: RunningAgentKey,
        pid: u32,
        conversation_id: String,
        agent_run_id: String,
        worktree_path: Option<String>,
        cancellation_token: Option<CancellationToken>,
    ) {
        // Check for existing agent and stop it if still alive
        {
            let conn = self.conn.lock().await;
            let existing = conn
                .query_row(
                    "SELECT pid, worktree_path FROM running_agents WHERE context_type = ?1 AND context_id = ?2",
                    rusqlite::params![key.context_type, key.context_id],
                    |row| {
                        let old_pid: u32 = row.get(0)?;
                        let old_worktree: Option<String> = row.get(1)?;
                        Ok((old_pid, old_worktree))
                    },
                )
                .ok();
            drop(conn);

            if let Some((old_pid, old_worktree)) = existing {
                if old_pid != pid && is_process_alive(old_pid) {
                    tracing::warn!(
                        old_pid,
                        new_pid = pid,
                        context_type = %key.context_type,
                        context_id = %key.context_id,
                        "Detected orphaned agent process — stopping before re-registration"
                    );
                    // Cancel old cancellation token
                    {
                        let mut tokens = self.tokens.lock().await;
                        if let Some(old_token) = tokens.remove(&key) {
                            old_token.cancel();
                        }
                    }
                    // Kill worktree processes if applicable
                    if let Some(ref path) = old_worktree {
                        let worktree = PathBuf::from(path);
                        if worktree.exists() {
                            kill_worktree_processes(&worktree);
                        }
                    }
                    kill_process(old_pid);
                }
            }
        }

        let conn = self.conn.lock().await;
        let started_at = chrono::Utc::now().to_rfc3339();
        let _ = conn.execute(
            "INSERT OR REPLACE INTO running_agents (context_type, context_id, pid, conversation_id, agent_run_id, started_at, worktree_path, last_active_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                key.context_type,
                key.context_id,
                pid,
                conversation_id,
                agent_run_id,
                started_at,
                worktree_path,
                Option::<String>::None
            ],
        );
        drop(conn);

        // Store cancellation token in memory (not persisted to SQLite)
        if let Some(token) = cancellation_token {
            let mut tokens = self.tokens.lock().await;
            tokens.insert(key, token);
        }
    }

    async fn unregister(&self, key: &RunningAgentKey) -> Option<RunningAgentInfo> {
        let conn = self.conn.lock().await;

        // Read the row first
        let info = conn
            .query_row(
                "SELECT pid, conversation_id, agent_run_id, started_at, worktree_path, last_active_at FROM running_agents WHERE context_type = ?1 AND context_id = ?2",
                rusqlite::params![key.context_type, key.context_id],
                |row| {
                    let pid: u32 = row.get(0)?;
                    let conversation_id: String = row.get(1)?;
                    let agent_run_id: String = row.get(2)?;
                    let started_at_str: String = row.get(3)?;
                    let worktree_path: Option<String> = row.get(4)?;
                    let last_active_at_str: Option<String> = row.get(5)?;
                    let started_at = chrono::DateTime::parse_from_rfc3339(&started_at_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now());
                    let last_active_at = last_active_at_str.and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                            .ok()
                    });
                    Ok(RunningAgentInfo {
                        pid,
                        conversation_id,
                        agent_run_id,
                        started_at,
                        worktree_path,
                        cancellation_token: None, // Populated below from in-memory map
                        last_active_at,
                    })
                },
            )
            .ok();

        // Delete the row
        let _ = conn.execute(
            "DELETE FROM running_agents WHERE context_type = ?1 AND context_id = ?2",
            rusqlite::params![key.context_type, key.context_id],
        );
        drop(conn);

        // Attach cancellation token from in-memory map
        let token = {
            let mut tokens = self.tokens.lock().await;
            tokens.remove(key)
        };

        info.map(|mut i| {
            i.cancellation_token = token;
            i
        })
    }

    async fn get(&self, key: &RunningAgentKey) -> Option<RunningAgentInfo> {
        let conn = self.conn.lock().await;
        let info = conn.query_row(
            "SELECT pid, conversation_id, agent_run_id, started_at, worktree_path, last_active_at FROM running_agents WHERE context_type = ?1 AND context_id = ?2",
            rusqlite::params![key.context_type, key.context_id],
            |row| {
                let pid: u32 = row.get(0)?;
                let conversation_id: String = row.get(1)?;
                let agent_run_id: String = row.get(2)?;
                let started_at_str: String = row.get(3)?;
                let worktree_path: Option<String> = row.get(4)?;
                let last_active_at_str: Option<String> = row.get(5)?;
                let started_at = chrono::DateTime::parse_from_rfc3339(&started_at_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now());
                let last_active_at = last_active_at_str.and_then(|s| {
                    chrono::DateTime::parse_from_rfc3339(&s)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .ok()
                });
                Ok(RunningAgentInfo {
                    pid,
                    conversation_id,
                    agent_run_id,
                    started_at,
                    worktree_path,
                    cancellation_token: None,
                    last_active_at,
                })
            },
        )
        .ok();
        drop(conn);

        // Attach cancellation token from in-memory map
        let tokens = self.tokens.lock().await;
        info.map(|mut i| {
            i.cancellation_token = tokens.get(key).cloned();
            i
        })
    }

    async fn is_running(&self, key: &RunningAgentKey) -> bool {
        let conn = self.conn.lock().await;
        conn.query_row(
            "SELECT 1 FROM running_agents WHERE context_type = ?1 AND context_id = ?2",
            rusqlite::params![key.context_type, key.context_id],
            |_| Ok(()),
        )
        .is_ok()
    }

    async fn stop(&self, key: &RunningAgentKey) -> Result<Option<RunningAgentInfo>, String> {
        let info = self.unregister(key).await;

        if let Some(ref agent_info) = info {
            // Cancel the async task before killing the process
            if let Some(ref token) = agent_info.cancellation_token {
                token.cancel();
            }
            if let Some(ref path) = agent_info.worktree_path {
                let worktree = PathBuf::from(path);
                if worktree.exists() {
                    kill_worktree_processes(&worktree);
                }
            }
            kill_process(agent_info.pid);
        }

        Ok(info)
    }

    async fn list_all(&self) -> Vec<(RunningAgentKey, RunningAgentInfo)> {
        // Scope DB operations so rusqlite types (not Send) are dropped before token lock
        let mut results = {
            let conn = self.conn.lock().await;
            let mut stmt = match conn.prepare(
                "SELECT context_type, context_id, pid, conversation_id, agent_run_id, started_at, worktree_path, last_active_at FROM running_agents",
            ) {
                Ok(stmt) => stmt,
                Err(_) => return Vec::new(),
            };

            let mut results = Vec::new();
            let mut rows = match stmt.query([]) {
                Ok(rows) => rows,
                Err(_) => return Vec::new(),
            };

            while let Ok(Some(row)) = rows.next() {
                let context_type: String = match row.get(0) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let context_id: String = match row.get(1) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let pid: u32 = match row.get(2) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let conversation_id: String = match row.get(3) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let agent_run_id: String = match row.get(4) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let started_at_str: String = match row.get(5) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let worktree_path: Option<String> = match row.get(6) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let last_active_at_str: Option<String> = row.get(7).unwrap_or_default();
                let started_at = chrono::DateTime::parse_from_rfc3339(&started_at_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now());
                let last_active_at = last_active_at_str.and_then(|s| {
                    chrono::DateTime::parse_from_rfc3339(&s)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .ok()
                });

                results.push((
                    RunningAgentKey {
                        context_type,
                        context_id,
                    },
                    RunningAgentInfo {
                        pid,
                        conversation_id,
                        agent_run_id,
                        started_at,
                        worktree_path,
                        cancellation_token: None,
                        last_active_at,
                    },
                ));
            }

            results
        };

        // Attach cancellation tokens from in-memory map
        let tokens = self.tokens.lock().await;
        for (key, info) in &mut results {
            info.cancellation_token = tokens.get(key).cloned();
        }

        results
    }

    async fn stop_all(&self) -> Vec<RunningAgentKey> {
        // Cancel all tokens first
        {
            let mut tokens = self.tokens.lock().await;
            for token in tokens.values() {
                token.cancel();
            }
            tokens.clear();
        }

        // Read all entries, kill processes, then clear table
        let entries = self.list_all().await;

        let mut stopped = Vec::new();
        for (key, info) in &entries {
            if let Some(ref path) = info.worktree_path {
                let worktree = PathBuf::from(path);
                if worktree.exists() {
                    kill_worktree_processes(&worktree);
                }
            }
            kill_process(info.pid);
            stopped.push(key.clone());
        }

        // Clear table
        let conn = self.conn.lock().await;
        let _ = conn.execute("DELETE FROM running_agents", []);

        stopped
    }

    async fn update_heartbeat(&self, key: &RunningAgentKey, at: chrono::DateTime<chrono::Utc>) {
        let conn = self.conn.lock().await;
        let at_str = at.to_rfc3339();
        let _ = conn.execute(
            "UPDATE running_agents SET last_active_at = ?1 WHERE context_type = ?2 AND context_id = ?3",
            rusqlite::params![at_str, key.context_type, key.context_id],
        );
    }

    async fn try_register(
        &self,
        key: RunningAgentKey,
        conversation_id: String,
        agent_run_id: String,
    ) -> Result<(), RunningAgentInfo> {
        // Hold conn mutex across check+insert for atomicity
        let conn = self.conn.lock().await;

        // Check for existing registration
        let existing = conn
            .query_row(
                "SELECT pid, conversation_id, agent_run_id, started_at, worktree_path, last_active_at FROM running_agents WHERE context_type = ?1 AND context_id = ?2",
                rusqlite::params![key.context_type, key.context_id],
                |row| {
                    let pid: u32 = row.get(0)?;
                    let conv_id: String = row.get(1)?;
                    let run_id: String = row.get(2)?;
                    let started_at_str: String = row.get(3)?;
                    let worktree_path: Option<String> = row.get(4)?;
                    let last_active_at_str: Option<String> = row.get(5)?;
                    let started_at = chrono::DateTime::parse_from_rfc3339(&started_at_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now());
                    let last_active_at = last_active_at_str.and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                            .ok()
                    });
                    Ok(RunningAgentInfo {
                        pid,
                        conversation_id: conv_id,
                        agent_run_id: run_id,
                        started_at,
                        worktree_path,
                        cancellation_token: None,
                        last_active_at,
                    })
                },
            )
            .ok();

        if let Some(mut info) = existing {
            drop(conn);
            let tokens = self.tokens.lock().await;
            info.cancellation_token = tokens.get(&key).cloned();
            return Err(info);
        }

        // Insert placeholder registration (pid=0, no worktree)
        let started_at = chrono::Utc::now().to_rfc3339();
        let _ = conn.execute(
            "INSERT INTO running_agents (context_type, context_id, pid, conversation_id, agent_run_id, started_at, worktree_path, last_active_at) VALUES (?1, ?2, 0, ?3, ?4, ?5, NULL, NULL)",
            rusqlite::params![key.context_type, key.context_id, conversation_id, agent_run_id, started_at],
        );

        Ok(())
    }

    async fn update_agent_process(
        &self,
        key: &RunningAgentKey,
        pid: u32,
        agent_run_id: &str,
        worktree_path: Option<String>,
        cancellation_token: Option<CancellationToken>,
    ) {
        let conn = self.conn.lock().await;
        let _ = conn.execute(
            "UPDATE running_agents SET pid = ?1, worktree_path = ?2, agent_run_id = ?3 WHERE context_type = ?4 AND context_id = ?5",
            rusqlite::params![pid, worktree_path, agent_run_id, key.context_type, key.context_id],
        );
        drop(conn);

        if let Some(token) = cancellation_token {
            let mut tokens = self.tokens.lock().await;
            tokens.insert(key.clone(), token);
        }
    }
}

#[cfg(test)]
#[path = "sqlite_running_agent_registry_tests.rs"]
mod tests;
