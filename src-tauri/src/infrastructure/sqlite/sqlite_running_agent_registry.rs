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
    kill_process, kill_worktree_processes, RunningAgentInfo, RunningAgentKey, RunningAgentRegistry,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

    fn setup_conn() -> Arc<Mutex<Connection>> {
        let conn = open_memory_connection().expect("open memory connection");
        run_migrations(&conn).expect("run migrations");
        Arc::new(Mutex::new(conn))
    }

    #[tokio::test]
    async fn test_register_and_get() {
        let conn = setup_conn();
        let registry = SqliteRunningAgentRegistry::new(conn);
        let key = RunningAgentKey::new("ideation", "session-123");

        registry
            .register(
                key.clone(),
                12345,
                "conv-abc".to_string(),
                "run-xyz".to_string(),
                Some("/tmp/worktree".to_string()),
                None,
            )
            .await;

        let info = registry.get(&key).await;
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.pid, 12345);
        assert_eq!(info.conversation_id, "conv-abc");
        assert_eq!(info.agent_run_id, "run-xyz");
    }

    #[tokio::test]
    async fn test_register_with_cancellation_token() {
        let conn = setup_conn();
        let registry = SqliteRunningAgentRegistry::new(conn);
        let key = RunningAgentKey::new("task", "task-cancel");
        let token = CancellationToken::new();

        registry
            .register(
                key.clone(),
                99999,
                "conv-ct".to_string(),
                "run-ct".to_string(),
                Some("/tmp/ct".to_string()),
                Some(token.clone()),
            )
            .await;

        let info = registry.get(&key).await.unwrap();
        assert!(info.cancellation_token.is_some());
        assert!(!token.is_cancelled());

        // Unregister should return token
        let info = registry.unregister(&key).await.unwrap();
        assert!(info.cancellation_token.is_some());
    }

    #[tokio::test]
    async fn test_unregister() {
        let conn = setup_conn();
        let registry = SqliteRunningAgentRegistry::new(conn);
        let key = RunningAgentKey::new("task", "task-456");

        registry
            .register(
                key.clone(),
                999,
                "conv-1".to_string(),
                "run-1".to_string(),
                Some("/tmp/worktree".to_string()),
                None,
            )
            .await;

        let info = registry.unregister(&key).await;
        assert!(info.is_some());
        assert_eq!(info.unwrap().pid, 999);

        // Should be gone
        assert!(!registry.is_running(&key).await);

        // Double unregister returns None
        let info = registry.unregister(&key).await;
        assert!(info.is_none());
    }

    #[tokio::test]
    async fn test_is_running() {
        let conn = setup_conn();
        let registry = SqliteRunningAgentRegistry::new(conn);
        let key = RunningAgentKey::new("review", "review-789");

        assert!(!registry.is_running(&key).await);

        registry
            .register(
                key.clone(),
                111,
                "conv-x".to_string(),
                "run-x".to_string(),
                Some("/tmp/worktree".to_string()),
                None,
            )
            .await;

        assert!(registry.is_running(&key).await);
    }

    #[tokio::test]
    async fn test_list_all() {
        let conn = setup_conn();
        let registry = SqliteRunningAgentRegistry::new(conn);

        registry
            .register(
                RunningAgentKey::new("ideation", "s1"),
                100,
                "c1".to_string(),
                "r1".to_string(),
                Some("/tmp/k1".to_string()),
                None,
            )
            .await;
        registry
            .register(
                RunningAgentKey::new("task", "t1"),
                200,
                "c2".to_string(),
                "r2".to_string(),
                Some("/tmp/k2".to_string()),
                None,
            )
            .await;

        let all = registry.list_all().await;
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_stop_all_clears_table() {
        let conn = setup_conn();
        let registry = SqliteRunningAgentRegistry::new(conn);

        registry
            .register(
                RunningAgentKey::new("a", "1"),
                10001,
                "c".to_string(),
                "r".to_string(),
                Some("/tmp/a".to_string()),
                None,
            )
            .await;
        registry
            .register(
                RunningAgentKey::new("b", "2"),
                10002,
                "c".to_string(),
                "r".to_string(),
                Some("/tmp/b".to_string()),
                None,
            )
            .await;

        let stopped = registry.stop_all().await;
        assert_eq!(stopped.len(), 2);

        // Table should be empty
        let all = registry.list_all().await;
        assert!(all.is_empty());
    }

    #[tokio::test]
    async fn test_register_replaces_existing() {
        let conn = setup_conn();
        let registry = SqliteRunningAgentRegistry::new(conn);
        let key = RunningAgentKey::new("task", "task-1");

        registry
            .register(
                key.clone(),
                100,
                "conv-old".to_string(),
                "run-old".to_string(),
                Some("/tmp/old".to_string()),
                None,
            )
            .await;
        registry
            .register(
                key.clone(),
                200,
                "conv-new".to_string(),
                "run-new".to_string(),
                Some("/tmp/new".to_string()),
                None,
            )
            .await;

        let info = registry.get(&key).await.unwrap();
        assert_eq!(info.pid, 200);
        assert_eq!(info.conversation_id, "conv-new");

        // Only one entry
        let all = registry.list_all().await;
        assert_eq!(all.len(), 1);
    }
}
