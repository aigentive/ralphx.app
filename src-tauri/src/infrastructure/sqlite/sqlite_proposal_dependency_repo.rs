// SQLite-based ProposalDependencyRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use rusqlite::Connection;
use uuid::Uuid;

use crate::domain::entities::{IdeationSessionId, TaskProposalId};
use crate::domain::repositories::ProposalDependencyRepository;
use crate::error::{AppError, AppResult};

/// SQLite implementation of ProposalDependencyRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteProposalDependencyRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteProposalDependencyRepository {
    /// Create a new SQLite proposal dependency repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    /// Create from an Arc-wrapped mutex connection (for sharing)
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Helper to convert String to TaskProposalId
    fn string_to_proposal_id(s: String) -> TaskProposalId {
        TaskProposalId(s)
    }
}

#[async_trait]
impl ProposalDependencyRepository for SqliteProposalDependencyRepository {
    async fn add_dependency(
        &self,
        proposal_id: &TaskProposalId,
        depends_on_id: &TaskProposalId,
        reason: Option<&str>,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;

        let id = Uuid::new_v4().to_string();

        // INSERT OR IGNORE to handle UNIQUE constraint gracefully
        conn.execute(
            "INSERT OR IGNORE INTO proposal_dependencies (id, proposal_id, depends_on_proposal_id, reason)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, proposal_id.as_str(), depends_on_id.as_str(), reason],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn remove_dependency(
        &self,
        proposal_id: &TaskProposalId,
        depends_on_id: &TaskProposalId,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "DELETE FROM proposal_dependencies
             WHERE proposal_id = ?1 AND depends_on_proposal_id = ?2",
            rusqlite::params![proposal_id.as_str(), depends_on_id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_dependencies(
        &self,
        proposal_id: &TaskProposalId,
    ) -> AppResult<Vec<TaskProposalId>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT depends_on_proposal_id FROM proposal_dependencies
                 WHERE proposal_id = ?1",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let deps = stmt
            .query_map([proposal_id.as_str()], |row| {
                let id: String = row.get(0)?;
                Ok(Self::string_to_proposal_id(id))
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(deps)
    }

    async fn get_dependents(
        &self,
        proposal_id: &TaskProposalId,
    ) -> AppResult<Vec<TaskProposalId>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT proposal_id FROM proposal_dependencies
                 WHERE depends_on_proposal_id = ?1",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let dependents = stmt
            .query_map([proposal_id.as_str()], |row| {
                let id: String = row.get(0)?;
                Ok(Self::string_to_proposal_id(id))
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(dependents)
    }

    async fn get_all_for_session(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<Vec<(TaskProposalId, TaskProposalId, Option<String>)>> {
        let conn = self.conn.lock().await;

        // Join with task_proposals to filter by session
        let mut stmt = conn
            .prepare(
                "SELECT pd.proposal_id, pd.depends_on_proposal_id, pd.reason
                 FROM proposal_dependencies pd
                 INNER JOIN task_proposals tp ON pd.proposal_id = tp.id
                 WHERE tp.session_id = ?1",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let deps = stmt
            .query_map([session_id.as_str()], |row| {
                let from_id: String = row.get(0)?;
                let to_id: String = row.get(1)?;
                let reason: Option<String> = row.get(2)?;
                Ok((Self::string_to_proposal_id(from_id), Self::string_to_proposal_id(to_id), reason))
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(deps)
    }

    async fn would_create_cycle(
        &self,
        proposal_id: &TaskProposalId,
        depends_on_id: &TaskProposalId,
    ) -> AppResult<bool> {
        // Self-dependency is always a cycle
        if proposal_id == depends_on_id {
            return Ok(true);
        }

        let conn = self.conn.lock().await;

        // Use DFS to detect if depends_on_id can reach proposal_id
        // If so, adding proposal_id -> depends_on_id would create a cycle
        let mut visited = HashSet::new();
        let mut stack = vec![depends_on_id.clone()];

        while let Some(current) = stack.pop() {
            if current == *proposal_id {
                // We found a path from depends_on_id to proposal_id
                // Adding proposal_id -> depends_on_id would create a cycle
                return Ok(true);
            }

            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());

            // Get all dependencies of current (what current depends on)
            let mut stmt = conn
                .prepare(
                    "SELECT depends_on_proposal_id FROM proposal_dependencies
                     WHERE proposal_id = ?1",
                )
                .map_err(|e| AppError::Database(e.to_string()))?;

            let deps: Vec<TaskProposalId> = stmt
                .query_map([current.as_str()], |row| {
                    let id: String = row.get(0)?;
                    Ok(Self::string_to_proposal_id(id))
                })
                .map_err(|e| AppError::Database(e.to_string()))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| AppError::Database(e.to_string()))?;

            for dep in deps {
                if !visited.contains(&dep) {
                    stack.push(dep);
                }
            }
        }

        Ok(false)
    }

    async fn clear_dependencies(&self, proposal_id: &TaskProposalId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        // Clear both directions: where this proposal depends on others,
        // and where others depend on this proposal
        conn.execute(
            "DELETE FROM proposal_dependencies
             WHERE proposal_id = ?1 OR depends_on_proposal_id = ?1",
            [proposal_id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn clear_session_dependencies(&self, session_id: &IdeationSessionId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        // Delete all dependencies for proposals in this session
        conn.execute(
            "DELETE FROM proposal_dependencies
             WHERE proposal_id IN (
                 SELECT id FROM task_proposals WHERE session_id = ?1
             )",
            [session_id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn count_dependencies(&self, proposal_id: &TaskProposalId) -> AppResult<u32> {
        let conn = self.conn.lock().await;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM proposal_dependencies WHERE proposal_id = ?1",
                [proposal_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count as u32)
    }

    async fn count_dependents(&self, proposal_id: &TaskProposalId) -> AppResult<u32> {
        let conn = self.conn.lock().await;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM proposal_dependencies WHERE depends_on_proposal_id = ?1",
                [proposal_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count as u32)
    }
}

#[cfg(test)]
#[path = "sqlite_proposal_dependency_repo_tests.rs"]
mod tests;
