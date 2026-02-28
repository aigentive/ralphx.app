// SQLite-based TaskDependencyRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use rusqlite::Connection;
use uuid::Uuid;

use super::DbConnection;
use crate::domain::entities::TaskId;
use crate::domain::repositories::TaskDependencyRepository;
use crate::error::AppResult;

/// SQLite implementation of TaskDependencyRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteTaskDependencyRepository {
    db: DbConnection,
}

impl SqliteTaskDependencyRepository {
    /// Create a new SQLite task dependency repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            db: DbConnection::new(conn),
        }
    }

    /// Create from an Arc-wrapped mutex connection (for sharing)
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            db: DbConnection::from_shared(conn),
        }
    }
}

#[async_trait]
impl TaskDependencyRepository for SqliteTaskDependencyRepository {
    async fn add_dependency(&self, task_id: &TaskId, depends_on_task_id: &TaskId) -> AppResult<()> {
        let id = Uuid::new_v4().to_string();
        let task_id = task_id.as_str().to_string();
        let depends_on = depends_on_task_id.as_str().to_string();
        self.db
            .run(move |conn| {
                conn.execute(
                    "INSERT OR IGNORE INTO task_dependencies (id, task_id, depends_on_task_id)
                     VALUES (?1, ?2, ?3)",
                    rusqlite::params![id, task_id.as_str(), depends_on.as_str()],
                )?;
                Ok(())
            })
            .await
    }

    async fn remove_dependency(
        &self,
        task_id: &TaskId,
        depends_on_task_id: &TaskId,
    ) -> AppResult<()> {
        let task_id = task_id.as_str().to_string();
        let depends_on = depends_on_task_id.as_str().to_string();
        self.db
            .run(move |conn| {
                conn.execute(
                    "DELETE FROM task_dependencies
                     WHERE task_id = ?1 AND depends_on_task_id = ?2",
                    rusqlite::params![task_id.as_str(), depends_on.as_str()],
                )?;
                Ok(())
            })
            .await
    }

    async fn get_blockers(&self, task_id: &TaskId) -> AppResult<Vec<TaskId>> {
        let task_id = task_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT depends_on_task_id FROM task_dependencies
                     WHERE task_id = ?1",
                )?;
                let blockers = stmt
                    .query_map([task_id.as_str()], |row| {
                        let id: String = row.get(0)?;
                        Ok(TaskId(id))
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(blockers)
            })
            .await
    }

    async fn get_blocked_by(&self, task_id: &TaskId) -> AppResult<Vec<TaskId>> {
        let task_id = task_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT task_id FROM task_dependencies
                     WHERE depends_on_task_id = ?1",
                )?;
                let blocked_by = stmt
                    .query_map([task_id.as_str()], |row| {
                        let id: String = row.get(0)?;
                        Ok(TaskId(id))
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(blocked_by)
            })
            .await
    }

    async fn has_circular_dependency(
        &self,
        task_id: &TaskId,
        potential_dep: &TaskId,
    ) -> AppResult<bool> {
        if task_id == potential_dep {
            return Ok(true);
        }

        let task_id = task_id.clone();
        let potential_dep = potential_dep.clone();

        self.db
            .run(move |conn| {
                let mut visited = HashSet::new();
                let mut stack = vec![potential_dep.clone()];

                while let Some(current) = stack.pop() {
                    if current == task_id {
                        return Ok(true);
                    }

                    if visited.contains(&current) {
                        continue;
                    }
                    visited.insert(current.clone());

                    let mut stmt = conn.prepare(
                        "SELECT depends_on_task_id FROM task_dependencies
                         WHERE task_id = ?1",
                    )?;

                    let deps: Vec<TaskId> = stmt
                        .query_map([current.as_str()], |row| {
                            let id: String = row.get(0)?;
                            Ok(TaskId(id))
                        })?
                        .collect::<Result<Vec<_>, _>>()?;

                    for dep in deps {
                        if !visited.contains(&dep) {
                            stack.push(dep);
                        }
                    }
                }

                Ok(false)
            })
            .await
    }

    async fn clear_dependencies(&self, task_id: &TaskId) -> AppResult<()> {
        let task_id = task_id.as_str().to_string();
        self.db
            .run(move |conn| {
                conn.execute(
                    "DELETE FROM task_dependencies
                     WHERE task_id = ?1 OR depends_on_task_id = ?1",
                    [task_id.as_str()],
                )?;
                Ok(())
            })
            .await
    }

    async fn count_blockers(&self, task_id: &TaskId) -> AppResult<u32> {
        let task_id = task_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM task_dependencies WHERE task_id = ?1",
                    [task_id.as_str()],
                    |row| row.get(0),
                )?;
                Ok(count as u32)
            })
            .await
    }

    async fn count_blocked_by(&self, task_id: &TaskId) -> AppResult<u32> {
        let task_id = task_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM task_dependencies WHERE depends_on_task_id = ?1",
                    [task_id.as_str()],
                    |row| row.get(0),
                )?;
                Ok(count as u32)
            })
            .await
    }

    async fn has_dependency(
        &self,
        task_id: &TaskId,
        depends_on_task_id: &TaskId,
    ) -> AppResult<bool> {
        let task_id = task_id.as_str().to_string();
        let depends_on = depends_on_task_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM task_dependencies
                     WHERE task_id = ?1 AND depends_on_task_id = ?2",
                    rusqlite::params![task_id.as_str(), depends_on.as_str()],
                    |row| row.get(0),
                )?;
                Ok(count > 0)
            })
            .await
    }
}

#[cfg(test)]
#[path = "sqlite_task_dependency_repo_tests.rs"]
mod tests;
