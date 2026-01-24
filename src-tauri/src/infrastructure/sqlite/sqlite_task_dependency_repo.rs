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
    async fn add_dependency(
        &self,
        task_id: &TaskId,
        depends_on_task_id: &TaskId,
    ) -> AppResult<()> {
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
mod tests {
    use super::*;
    use crate::domain::entities::ProjectId;
    use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

    fn setup_test_db() -> Connection {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    fn create_test_project(conn: &Connection, id: &ProjectId, name: &str, path: &str) {
        conn.execute(
            "INSERT INTO projects (id, name, working_directory, git_mode, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'single_branch', datetime('now'), datetime('now'))",
            rusqlite::params![id.as_str(), name, path],
        )
        .unwrap();
    }

    fn create_test_task(conn: &Connection, project_id: &ProjectId, title: &str) -> TaskId {
        let task_id = TaskId::new();
        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title, created_at, updated_at)
             VALUES (?1, ?2, 'feature', ?3, datetime('now'), datetime('now'))",
            rusqlite::params![task_id.as_str(), project_id.as_str(), title],
        )
        .unwrap();
        task_id
    }

    // ==================== ADD DEPENDENCY TESTS ====================

    #[tokio::test]
    async fn test_add_dependency_creates_record() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task_a = create_test_task(&conn, &project_id, "Task A");
        let task_b = create_test_task(&conn, &project_id, "Task B");

        let repo = SqliteTaskDependencyRepository::new(conn);

        let result = repo.add_dependency(&task_a, &task_b).await;

        assert!(result.is_ok());

        // Verify dependency was created (A depends on B, so B is a blocker of A)
        let blockers = repo.get_blockers(&task_a).await.unwrap();
        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0], task_b);
    }

    #[tokio::test]
    async fn test_add_dependency_duplicate_is_ignored() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task_a = create_test_task(&conn, &project_id, "Task A");
        let task_b = create_test_task(&conn, &project_id, "Task B");

        let repo = SqliteTaskDependencyRepository::new(conn);

        // Add same dependency twice
        repo.add_dependency(&task_a, &task_b).await.unwrap();
        let result = repo.add_dependency(&task_a, &task_b).await;

        assert!(result.is_ok());

        // Should only have one dependency
        let blockers = repo.get_blockers(&task_a).await.unwrap();
        assert_eq!(blockers.len(), 1);
    }

    #[tokio::test]
    async fn test_add_multiple_dependencies() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task_a = create_test_task(&conn, &project_id, "Task A");
        let task_b = create_test_task(&conn, &project_id, "Task B");
        let task_c = create_test_task(&conn, &project_id, "Task C");

        let repo = SqliteTaskDependencyRepository::new(conn);

        // A depends on B and C
        repo.add_dependency(&task_a, &task_b).await.unwrap();
        repo.add_dependency(&task_a, &task_c).await.unwrap();

        let blockers = repo.get_blockers(&task_a).await.unwrap();
        assert_eq!(blockers.len(), 2);
        assert!(blockers.contains(&task_b));
        assert!(blockers.contains(&task_c));
    }

    // ==================== REMOVE DEPENDENCY TESTS ====================

    #[tokio::test]
    async fn test_remove_dependency_deletes_record() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task_a = create_test_task(&conn, &project_id, "Task A");
        let task_b = create_test_task(&conn, &project_id, "Task B");

        let repo = SqliteTaskDependencyRepository::new(conn);

        repo.add_dependency(&task_a, &task_b).await.unwrap();
        let result = repo.remove_dependency(&task_a, &task_b).await;

        assert!(result.is_ok());

        let blockers = repo.get_blockers(&task_a).await.unwrap();
        assert!(blockers.is_empty());
    }

    #[tokio::test]
    async fn test_remove_nonexistent_dependency_succeeds() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task_a = create_test_task(&conn, &project_id, "Task A");
        let task_b = create_test_task(&conn, &project_id, "Task B");

        let repo = SqliteTaskDependencyRepository::new(conn);

        // Should not error
        let result = repo.remove_dependency(&task_a, &task_b).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_remove_only_specified_dependency() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task_a = create_test_task(&conn, &project_id, "Task A");
        let task_b = create_test_task(&conn, &project_id, "Task B");
        let task_c = create_test_task(&conn, &project_id, "Task C");

        let repo = SqliteTaskDependencyRepository::new(conn);

        repo.add_dependency(&task_a, &task_b).await.unwrap();
        repo.add_dependency(&task_a, &task_c).await.unwrap();

        // Remove only B dependency
        repo.remove_dependency(&task_a, &task_b).await.unwrap();

        let blockers = repo.get_blockers(&task_a).await.unwrap();
        assert_eq!(blockers.len(), 1);
        assert!(blockers.contains(&task_c));
    }

    // ==================== GET BLOCKERS TESTS ====================

    #[tokio::test]
    async fn test_get_blockers_empty() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task = create_test_task(&conn, &project_id, "Task");

        let repo = SqliteTaskDependencyRepository::new(conn);

        let blockers = repo.get_blockers(&task).await.unwrap();
        assert!(blockers.is_empty());
    }

    #[tokio::test]
    async fn test_get_blockers_returns_correct_direction() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task_a = create_test_task(&conn, &project_id, "Task A");
        let task_b = create_test_task(&conn, &project_id, "Task B");

        let repo = SqliteTaskDependencyRepository::new(conn);

        // A depends on B
        repo.add_dependency(&task_a, &task_b).await.unwrap();

        // A's blockers should include B
        let a_blockers = repo.get_blockers(&task_a).await.unwrap();
        assert_eq!(a_blockers.len(), 1);
        assert!(a_blockers.contains(&task_b));

        // B should have no blockers
        let b_blockers = repo.get_blockers(&task_b).await.unwrap();
        assert!(b_blockers.is_empty());
    }

    // ==================== GET BLOCKED BY TESTS ====================

    #[tokio::test]
    async fn test_get_blocked_by_empty() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task = create_test_task(&conn, &project_id, "Task");

        let repo = SqliteTaskDependencyRepository::new(conn);

        let blocked_by = repo.get_blocked_by(&task).await.unwrap();
        assert!(blocked_by.is_empty());
    }

    #[tokio::test]
    async fn test_get_blocked_by_returns_correct_direction() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task_a = create_test_task(&conn, &project_id, "Task A");
        let task_b = create_test_task(&conn, &project_id, "Task B");

        let repo = SqliteTaskDependencyRepository::new(conn);

        // A depends on B (B blocks A)
        repo.add_dependency(&task_a, &task_b).await.unwrap();

        // B's blocked_by should include A
        let b_blocked_by = repo.get_blocked_by(&task_b).await.unwrap();
        assert_eq!(b_blocked_by.len(), 1);
        assert!(b_blocked_by.contains(&task_a));

        // A should have no tasks blocked by it
        let a_blocked_by = repo.get_blocked_by(&task_a).await.unwrap();
        assert!(a_blocked_by.is_empty());
    }

    #[tokio::test]
    async fn test_get_blocked_by_multiple() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task_a = create_test_task(&conn, &project_id, "Task A");
        let task_b = create_test_task(&conn, &project_id, "Task B");
        let task_c = create_test_task(&conn, &project_id, "Task C");

        let repo = SqliteTaskDependencyRepository::new(conn);

        // A and B both depend on C
        repo.add_dependency(&task_a, &task_c).await.unwrap();
        repo.add_dependency(&task_b, &task_c).await.unwrap();

        let blocked_by = repo.get_blocked_by(&task_c).await.unwrap();
        assert_eq!(blocked_by.len(), 2);
        assert!(blocked_by.contains(&task_a));
        assert!(blocked_by.contains(&task_b));
    }

    // ==================== HAS CIRCULAR DEPENDENCY TESTS ====================

    #[tokio::test]
    async fn test_has_circular_dependency_self() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task = create_test_task(&conn, &project_id, "Task");

        let repo = SqliteTaskDependencyRepository::new(conn);

        let result = repo.has_circular_dependency(&task, &task).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_has_circular_dependency_direct_cycle() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task_a = create_test_task(&conn, &project_id, "Task A");
        let task_b = create_test_task(&conn, &project_id, "Task B");

        let repo = SqliteTaskDependencyRepository::new(conn);

        // B depends on A
        repo.add_dependency(&task_b, &task_a).await.unwrap();

        // Would adding A -> B create a cycle? Yes (A -> B -> A)
        let result = repo.has_circular_dependency(&task_a, &task_b).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_has_circular_dependency_indirect_cycle() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task_a = create_test_task(&conn, &project_id, "Task A");
        let task_b = create_test_task(&conn, &project_id, "Task B");
        let task_c = create_test_task(&conn, &project_id, "Task C");

        let repo = SqliteTaskDependencyRepository::new(conn);

        // B -> C, C -> A (existing chain: B depends on C, C depends on A)
        repo.add_dependency(&task_b, &task_c).await.unwrap();
        repo.add_dependency(&task_c, &task_a).await.unwrap();

        // Would adding A -> B create a cycle? Yes (A -> B -> C -> A)
        let result = repo.has_circular_dependency(&task_a, &task_b).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_has_circular_dependency_no_cycle() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task_a = create_test_task(&conn, &project_id, "Task A");
        let task_b = create_test_task(&conn, &project_id, "Task B");
        let task_c = create_test_task(&conn, &project_id, "Task C");

        let repo = SqliteTaskDependencyRepository::new(conn);

        // A -> B (existing)
        repo.add_dependency(&task_a, &task_b).await.unwrap();

        // Would adding B -> C create a cycle? No
        let result = repo.has_circular_dependency(&task_b, &task_c).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_has_circular_dependency_empty_graph() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task_a = create_test_task(&conn, &project_id, "Task A");
        let task_b = create_test_task(&conn, &project_id, "Task B");

        let repo = SqliteTaskDependencyRepository::new(conn);

        // No existing dependencies, would A -> B create a cycle? No
        let result = repo.has_circular_dependency(&task_a, &task_b).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_has_circular_dependency_long_chain() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task_a = create_test_task(&conn, &project_id, "Task A");
        let task_b = create_test_task(&conn, &project_id, "Task B");
        let task_c = create_test_task(&conn, &project_id, "Task C");
        let task_d = create_test_task(&conn, &project_id, "Task D");
        let task_e = create_test_task(&conn, &project_id, "Task E");

        let repo = SqliteTaskDependencyRepository::new(conn);

        // Chain: B -> C -> D -> E -> A
        repo.add_dependency(&task_b, &task_c).await.unwrap();
        repo.add_dependency(&task_c, &task_d).await.unwrap();
        repo.add_dependency(&task_d, &task_e).await.unwrap();
        repo.add_dependency(&task_e, &task_a).await.unwrap();

        // Would adding A -> B create a cycle? Yes (A -> B -> C -> D -> E -> A)
        let result = repo.has_circular_dependency(&task_a, &task_b).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    // ==================== CLEAR DEPENDENCIES TESTS ====================

    #[tokio::test]
    async fn test_clear_dependencies_removes_outgoing() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task_a = create_test_task(&conn, &project_id, "Task A");
        let task_b = create_test_task(&conn, &project_id, "Task B");
        let task_c = create_test_task(&conn, &project_id, "Task C");

        let repo = SqliteTaskDependencyRepository::new(conn);

        // A -> B, A -> C
        repo.add_dependency(&task_a, &task_b).await.unwrap();
        repo.add_dependency(&task_a, &task_c).await.unwrap();

        repo.clear_dependencies(&task_a).await.unwrap();

        let blockers = repo.get_blockers(&task_a).await.unwrap();
        assert!(blockers.is_empty());
    }

    #[tokio::test]
    async fn test_clear_dependencies_removes_incoming() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task_a = create_test_task(&conn, &project_id, "Task A");
        let task_b = create_test_task(&conn, &project_id, "Task B");
        let task_c = create_test_task(&conn, &project_id, "Task C");

        let repo = SqliteTaskDependencyRepository::new(conn);

        // B -> A, C -> A
        repo.add_dependency(&task_b, &task_a).await.unwrap();
        repo.add_dependency(&task_c, &task_a).await.unwrap();

        repo.clear_dependencies(&task_a).await.unwrap();

        // A should have no tasks blocked by it anymore
        let blocked_by = repo.get_blocked_by(&task_a).await.unwrap();
        assert!(blocked_by.is_empty());

        // B and C should have no blockers anymore
        let b_blockers = repo.get_blockers(&task_b).await.unwrap();
        assert!(b_blockers.is_empty());
        let c_blockers = repo.get_blockers(&task_c).await.unwrap();
        assert!(c_blockers.is_empty());
    }

    #[tokio::test]
    async fn test_clear_dependencies_removes_both_directions() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task_a = create_test_task(&conn, &project_id, "Task A");
        let task_b = create_test_task(&conn, &project_id, "Task B");
        let task_c = create_test_task(&conn, &project_id, "Task C");

        let repo = SqliteTaskDependencyRepository::new(conn);

        // A -> B (A depends on B), C -> A (C depends on A)
        repo.add_dependency(&task_a, &task_b).await.unwrap();
        repo.add_dependency(&task_c, &task_a).await.unwrap();

        repo.clear_dependencies(&task_a).await.unwrap();

        // A should have no blockers
        let a_blockers = repo.get_blockers(&task_a).await.unwrap();
        assert!(a_blockers.is_empty());

        // A should have no tasks blocked by it
        let a_blocked_by = repo.get_blocked_by(&task_a).await.unwrap();
        assert!(a_blocked_by.is_empty());

        // C should have no blockers (was depending on A)
        let c_blockers = repo.get_blockers(&task_c).await.unwrap();
        assert!(c_blockers.is_empty());
    }

    // ==================== COUNT BLOCKERS TESTS ====================

    #[tokio::test]
    async fn test_count_blockers_zero() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task = create_test_task(&conn, &project_id, "Task");

        let repo = SqliteTaskDependencyRepository::new(conn);

        let count = repo.count_blockers(&task).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_count_blockers_multiple() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task_a = create_test_task(&conn, &project_id, "Task A");
        let task_b = create_test_task(&conn, &project_id, "Task B");
        let task_c = create_test_task(&conn, &project_id, "Task C");

        let repo = SqliteTaskDependencyRepository::new(conn);

        // A depends on B and C
        repo.add_dependency(&task_a, &task_b).await.unwrap();
        repo.add_dependency(&task_a, &task_c).await.unwrap();

        let count = repo.count_blockers(&task_a).await.unwrap();
        assert_eq!(count, 2);
    }

    // ==================== COUNT BLOCKED BY TESTS ====================

    #[tokio::test]
    async fn test_count_blocked_by_zero() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task = create_test_task(&conn, &project_id, "Task");

        let repo = SqliteTaskDependencyRepository::new(conn);

        let count = repo.count_blocked_by(&task).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_count_blocked_by_multiple() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task_a = create_test_task(&conn, &project_id, "Task A");
        let task_b = create_test_task(&conn, &project_id, "Task B");
        let task_c = create_test_task(&conn, &project_id, "Task C");

        let repo = SqliteTaskDependencyRepository::new(conn);

        // B and C depend on A
        repo.add_dependency(&task_b, &task_a).await.unwrap();
        repo.add_dependency(&task_c, &task_a).await.unwrap();

        let count = repo.count_blocked_by(&task_a).await.unwrap();
        assert_eq!(count, 2);
    }

    // ==================== HAS DEPENDENCY TESTS ====================

    #[tokio::test]
    async fn test_has_dependency_true() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task_a = create_test_task(&conn, &project_id, "Task A");
        let task_b = create_test_task(&conn, &project_id, "Task B");

        let repo = SqliteTaskDependencyRepository::new(conn);

        repo.add_dependency(&task_a, &task_b).await.unwrap();

        let has_dep = repo.has_dependency(&task_a, &task_b).await.unwrap();
        assert!(has_dep);
    }

    #[tokio::test]
    async fn test_has_dependency_false() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task_a = create_test_task(&conn, &project_id, "Task A");
        let task_b = create_test_task(&conn, &project_id, "Task B");

        let repo = SqliteTaskDependencyRepository::new(conn);

        let has_dep = repo.has_dependency(&task_a, &task_b).await.unwrap();
        assert!(!has_dep);
    }

    #[tokio::test]
    async fn test_has_dependency_direction_matters() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task_a = create_test_task(&conn, &project_id, "Task A");
        let task_b = create_test_task(&conn, &project_id, "Task B");

        let repo = SqliteTaskDependencyRepository::new(conn);

        // A depends on B
        repo.add_dependency(&task_a, &task_b).await.unwrap();

        // A -> B exists
        assert!(repo.has_dependency(&task_a, &task_b).await.unwrap());

        // B -> A does NOT exist
        assert!(!repo.has_dependency(&task_b, &task_a).await.unwrap());
    }

    // ==================== SHARED CONNECTION TESTS ====================

    #[tokio::test]
    async fn test_from_shared_works_correctly() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task_a = create_test_task(&conn, &project_id, "Task A");
        let task_b = create_test_task(&conn, &project_id, "Task B");

        let shared_conn = Arc::new(Mutex::new(conn));
        let repo = SqliteTaskDependencyRepository::from_shared(shared_conn);

        repo.add_dependency(&task_a, &task_b).await.unwrap();

        let blockers = repo.get_blockers(&task_a).await.unwrap();
        assert_eq!(blockers.len(), 1);
    }

    // ==================== CASCADE DELETE TESTS ====================

    #[tokio::test]
    async fn test_cascade_deletes_when_task_deleted() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task_a = create_test_task(&conn, &project_id, "Task A");
        let task_b = create_test_task(&conn, &project_id, "Task B");

        // Add dependency
        conn.execute(
            "INSERT INTO task_dependencies (id, task_id, depends_on_task_id)
             VALUES ('dep-1', ?1, ?2)",
            rusqlite::params![task_a.as_str(), task_b.as_str()],
        )
        .unwrap();

        // Verify dependency exists
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_dependencies WHERE task_id = ?1",
                [task_a.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        // Delete task A
        conn.execute("DELETE FROM tasks WHERE id = ?1", [task_a.as_str()])
            .unwrap();

        // Dependency should be gone due to CASCADE
        let count_after: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_dependencies WHERE task_id = ?1",
                [task_a.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count_after, 0);
    }

    #[tokio::test]
    async fn test_cascade_deletes_when_depends_on_task_deleted() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task_a = create_test_task(&conn, &project_id, "Task A");
        let task_b = create_test_task(&conn, &project_id, "Task B");

        // A depends on B
        conn.execute(
            "INSERT INTO task_dependencies (id, task_id, depends_on_task_id)
             VALUES ('dep-1', ?1, ?2)",
            rusqlite::params![task_a.as_str(), task_b.as_str()],
        )
        .unwrap();

        // Delete task B
        conn.execute("DELETE FROM tasks WHERE id = ?1", [task_b.as_str()])
            .unwrap();

        // Dependency should be gone due to CASCADE
        let count_after: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_dependencies WHERE depends_on_task_id = ?1",
                [task_b.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count_after, 0);
    }

    // ==================== CHECK CONSTRAINT TESTS ====================

    #[tokio::test]
    async fn test_self_dependency_check_constraint() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let task = create_test_task(&conn, &project_id, "Task");

        // Direct insert should fail due to CHECK constraint
        let result = conn.execute(
            "INSERT INTO task_dependencies (id, task_id, depends_on_task_id)
             VALUES ('dep-1', ?1, ?1)",
            [task.as_str()],
        );

        assert!(result.is_err());
    }

    // ==================== COMPLEX GRAPH TESTS ====================

    #[tokio::test]
    async fn test_diamond_dependency_no_cycle() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");

        // Diamond pattern:
        //     A
        //    / \
        //   B   C
        //    \ /
        //     D
        let task_a = create_test_task(&conn, &project_id, "Task A");
        let task_b = create_test_task(&conn, &project_id, "Task B");
        let task_c = create_test_task(&conn, &project_id, "Task C");
        let task_d = create_test_task(&conn, &project_id, "Task D");

        let repo = SqliteTaskDependencyRepository::new(conn);

        // A depends on nothing
        // B depends on A, C depends on A
        // D depends on B and C
        repo.add_dependency(&task_b, &task_a).await.unwrap();
        repo.add_dependency(&task_c, &task_a).await.unwrap();
        repo.add_dependency(&task_d, &task_b).await.unwrap();
        repo.add_dependency(&task_d, &task_c).await.unwrap();

        // This is valid, no cycle
        // Would adding a new dependency E -> D create a cycle? No
        let task_e = TaskId::new();
        let has_cycle = repo.has_circular_dependency(&task_e, &task_d).await.unwrap();
        assert!(!has_cycle);

        // D has 2 blockers
        assert_eq!(repo.count_blockers(&task_d).await.unwrap(), 2);

        // A has 2 tasks blocked by it (B and C)
        assert_eq!(repo.count_blocked_by(&task_a).await.unwrap(), 2);
    }
}
