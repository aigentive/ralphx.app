// SQLite-based TaskDependencyRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use rusqlite::Connection;
use uuid::Uuid;

use crate::domain::entities::TaskId;
use crate::domain::repositories::TaskDependencyRepository;
use crate::error::{AppError, AppResult};

/// SQLite implementation of TaskDependencyRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteTaskDependencyRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteTaskDependencyRepository {
    /// Create a new SQLite task dependency repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    /// Create from an Arc-wrapped mutex connection (for sharing)
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Helper to convert String to TaskId
    fn string_to_task_id(s: String) -> TaskId {
        TaskId(s)
    }
}

#[async_trait]
impl TaskDependencyRepository for SqliteTaskDependencyRepository {
    async fn add_dependency(&self, task_id: &TaskId, depends_on_task_id: &TaskId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        let id = Uuid::new_v4().to_string();

        // INSERT OR IGNORE to handle UNIQUE constraint gracefully
        conn.execute(
            "INSERT OR IGNORE INTO task_dependencies (id, task_id, depends_on_task_id)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![id, task_id.as_str(), depends_on_task_id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn remove_dependency(
        &self,
        task_id: &TaskId,
        depends_on_task_id: &TaskId,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "DELETE FROM task_dependencies
             WHERE task_id = ?1 AND depends_on_task_id = ?2",
            rusqlite::params![task_id.as_str(), depends_on_task_id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_blockers(&self, task_id: &TaskId) -> AppResult<Vec<TaskId>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT depends_on_task_id FROM task_dependencies
                 WHERE task_id = ?1",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let blockers = stmt
            .query_map([task_id.as_str()], |row| {
                let id: String = row.get(0)?;
                Ok(Self::string_to_task_id(id))
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(blockers)
    }

    async fn get_blocked_by(&self, task_id: &TaskId) -> AppResult<Vec<TaskId>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT task_id FROM task_dependencies
                 WHERE depends_on_task_id = ?1",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let blocked_by = stmt
            .query_map([task_id.as_str()], |row| {
                let id: String = row.get(0)?;
                Ok(Self::string_to_task_id(id))
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(blocked_by)
    }

    async fn has_circular_dependency(
        &self,
        task_id: &TaskId,
        potential_dep: &TaskId,
    ) -> AppResult<bool> {
        // Self-dependency is always a cycle
        if task_id == potential_dep {
            return Ok(true);
        }

        let conn = self.conn.lock().await;

        // Use DFS to detect if potential_dep can reach task_id
        // If so, adding task_id -> potential_dep would create a cycle
        let mut visited = HashSet::new();
        let mut stack = vec![potential_dep.clone()];

        while let Some(current) = stack.pop() {
            if current == *task_id {
                // We found a path from potential_dep to task_id
                // Adding task_id -> potential_dep would create a cycle
                return Ok(true);
            }

            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());

            // Get all tasks that current depends on (blockers of current)
            let mut stmt = conn
                .prepare(
                    "SELECT depends_on_task_id FROM task_dependencies
                     WHERE task_id = ?1",
                )
                .map_err(|e| AppError::Database(e.to_string()))?;

            let deps: Vec<TaskId> = stmt
                .query_map([current.as_str()], |row| {
                    let id: String = row.get(0)?;
                    Ok(Self::string_to_task_id(id))
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

    async fn clear_dependencies(&self, task_id: &TaskId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        // Clear both directions: where this task depends on others,
        // and where others depend on this task
        conn.execute(
            "DELETE FROM task_dependencies
             WHERE task_id = ?1 OR depends_on_task_id = ?1",
            [task_id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn count_blockers(&self, task_id: &TaskId) -> AppResult<u32> {
        let conn = self.conn.lock().await;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_dependencies WHERE task_id = ?1",
                [task_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count as u32)
    }

    async fn count_blocked_by(&self, task_id: &TaskId) -> AppResult<u32> {
        let conn = self.conn.lock().await;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_dependencies WHERE depends_on_task_id = ?1",
                [task_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count as u32)
    }

    async fn has_dependency(
        &self,
        task_id: &TaskId,
        depends_on_task_id: &TaskId,
    ) -> AppResult<bool> {
        let conn = self.conn.lock().await;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_dependencies
                 WHERE task_id = ?1 AND depends_on_task_id = ?2",
                rusqlite::params![task_id.as_str(), depends_on_task_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count > 0)
    }
}

#[cfg(test)]
#[path = "sqlite_task_dependency_repo_tests.rs"]
mod tests;
