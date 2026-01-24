// SQLite-based TaskRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use rusqlite::Connection;

use crate::domain::entities::{InternalStatus, ProjectId, Task, TaskId};
use crate::domain::repositories::{StatusTransition, TaskRepository};
use crate::error::{AppError, AppResult};

/// SQLite implementation of TaskRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteTaskRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteTaskRepository {
    /// Create a new SQLite task repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    /// Create from an Arc-wrapped mutex connection (for sharing)
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl TaskRepository for SqliteTaskRepository {
    async fn create(&self, task: Task) -> AppResult<Task> {
        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title, description, priority, internal_status, needs_review_point, created_at, updated_at, started_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            rusqlite::params![
                task.id.as_str(),
                task.project_id.as_str(),
                task.category,
                task.title,
                task.description,
                task.priority,
                task.internal_status.as_str(),
                task.needs_review_point,
                task.created_at.to_rfc3339(),
                task.updated_at.to_rfc3339(),
                task.started_at.map(|dt| dt.to_rfc3339()),
                task.completed_at.map(|dt| dt.to_rfc3339()),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(task)
    }

    async fn get_by_id(&self, id: &TaskId) -> AppResult<Option<Task>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, project_id, category, title, description, priority, internal_status, needs_review_point, created_at, updated_at, started_at, completed_at
             FROM tasks WHERE id = ?1",
            [id.as_str()],
            |row| Task::from_row(row),
        );

        match result {
            Ok(task) => Ok(Some(task)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<Task>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, category, title, description, priority, internal_status, needs_review_point, created_at, updated_at, started_at, completed_at
                 FROM tasks WHERE project_id = ?1
                 ORDER BY priority DESC, created_at ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let tasks = stmt
            .query_map([project_id.as_str()], |row| Task::from_row(row))
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(tasks)
    }

    async fn update(&self, task: &Task) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "UPDATE tasks SET project_id = ?2, category = ?3, title = ?4, description = ?5, priority = ?6, internal_status = ?7, updated_at = ?8, started_at = ?9, completed_at = ?10
             WHERE id = ?1",
            rusqlite::params![
                task.id.as_str(),
                task.project_id.as_str(),
                task.category,
                task.title,
                task.description,
                task.priority,
                task.internal_status.as_str(),
                task.updated_at.to_rfc3339(),
                task.started_at.map(|dt| dt.to_rfc3339()),
                task.completed_at.map(|dt| dt.to_rfc3339()),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: &TaskId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute("DELETE FROM tasks WHERE id = ?1", [id.as_str()])
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_by_status(
        &self,
        project_id: &ProjectId,
        status: InternalStatus,
    ) -> AppResult<Vec<Task>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, category, title, description, priority, internal_status, needs_review_point, created_at, updated_at, started_at, completed_at
                 FROM tasks WHERE project_id = ?1 AND internal_status = ?2
                 ORDER BY priority DESC, created_at ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let tasks = stmt
            .query_map(
                rusqlite::params![project_id.as_str(), status.as_str()],
                |row| Task::from_row(row),
            )
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(tasks)
    }

    async fn persist_status_change(
        &self,
        id: &TaskId,
        from: InternalStatus,
        to: InternalStatus,
        trigger: &str,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;
        let now = Utc::now();

        // Use a transaction for atomicity
        conn.execute("BEGIN TRANSACTION", [])
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Update task status
        let update_result = conn.execute(
            "UPDATE tasks SET internal_status = ?2, updated_at = ?3 WHERE id = ?1",
            rusqlite::params![id.as_str(), to.as_str(), now.to_rfc3339()],
        );

        if let Err(e) = update_result {
            let _ = conn.execute("ROLLBACK", []);
            return Err(AppError::Database(e.to_string()));
        }

        // Insert history record
        let history_id = uuid::Uuid::new_v4().to_string();
        let insert_result = conn.execute(
            "INSERT INTO task_state_history (id, task_id, from_status, to_status, changed_by, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                history_id,
                id.as_str(),
                from.as_str(),
                to.as_str(),
                trigger,
                now.to_rfc3339()
            ],
        );

        if let Err(e) = insert_result {
            let _ = conn.execute("ROLLBACK", []);
            return Err(AppError::Database(e.to_string()));
        }

        conn.execute("COMMIT", [])
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_status_history(&self, id: &TaskId) -> AppResult<Vec<StatusTransition>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT from_status, to_status, changed_by, created_at
                 FROM task_state_history WHERE task_id = ?1
                 ORDER BY created_at ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let transitions = stmt
            .query_map([id.as_str()], |row| {
                let from_str: String = row.get(0)?;
                let to_str: String = row.get(1)?;
                let trigger: String = row.get(2)?;
                let created_at_str: String = row.get(3)?;

                let from = from_str.parse().unwrap_or(InternalStatus::Backlog);
                let to = to_str.parse().unwrap_or(InternalStatus::Backlog);
                let timestamp = Task::parse_datetime(created_at_str);

                Ok(StatusTransition::with_timestamp(from, to, trigger, timestamp))
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(transitions)
    }

    async fn get_next_executable(&self, project_id: &ProjectId) -> AppResult<Option<Task>> {
        let conn = self.conn.lock().await;

        // Find READY tasks that have no blockers
        let result = conn.query_row(
            "SELECT t.id, t.project_id, t.category, t.title, t.description, t.priority, t.internal_status, t.needs_review_point, t.created_at, t.updated_at, t.started_at, t.completed_at
             FROM tasks t
             WHERE t.project_id = ?1
               AND t.internal_status = 'ready'
               AND NOT EXISTS (
                   SELECT 1 FROM task_blockers tb WHERE tb.task_id = t.id
               )
             ORDER BY t.priority DESC, t.created_at ASC
             LIMIT 1",
            [project_id.as_str()],
            |row| Task::from_row(row),
        );

        match result {
            Ok(task) => Ok(Some(task)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_blockers(&self, id: &TaskId) -> AppResult<Vec<Task>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT t.id, t.project_id, t.category, t.title, t.description, t.priority, t.internal_status, t.needs_review_point, t.created_at, t.updated_at, t.started_at, t.completed_at
                 FROM tasks t
                 INNER JOIN task_blockers tb ON t.id = tb.blocker_id
                 WHERE tb.task_id = ?1",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let tasks = stmt
            .query_map([id.as_str()], |row| Task::from_row(row))
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(tasks)
    }

    async fn get_dependents(&self, id: &TaskId) -> AppResult<Vec<Task>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT t.id, t.project_id, t.category, t.title, t.description, t.priority, t.internal_status, t.needs_review_point, t.created_at, t.updated_at, t.started_at, t.completed_at
                 FROM tasks t
                 INNER JOIN task_blockers tb ON t.id = tb.task_id
                 WHERE tb.blocker_id = ?1",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let tasks = stmt
            .query_map([id.as_str()], |row| Task::from_row(row))
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(tasks)
    }

    async fn add_blocker(&self, task_id: &TaskId, blocker_id: &TaskId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT INTO task_blockers (task_id, blocker_id) VALUES (?1, ?2)",
            rusqlite::params![task_id.as_str(), blocker_id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn resolve_blocker(&self, task_id: &TaskId, blocker_id: &TaskId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "DELETE FROM task_blockers WHERE task_id = ?1 AND blocker_id = ?2",
            rusqlite::params![task_id.as_str(), blocker_id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

    fn setup_test_db() -> Connection {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        // Insert a test project (required for foreign key)
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('test-project', 'Test Project', '/test/path')",
            [],
        )
        .unwrap();
        conn
    }

    fn create_test_task(title: &str) -> Task {
        Task::new_with_category(
            ProjectId::from_string("test-project".to_string()),
            title.to_string(),
            "feature".to_string(),
        )
    }

    // ==================== CRUD TESTS ====================

    #[tokio::test]
    async fn test_create_inserts_task_and_returns_it() {
        let conn = setup_test_db();
        let repo = SqliteTaskRepository::new(conn);
        let task = create_test_task("Test Task");

        let result = repo.create(task.clone()).await;

        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.id, task.id);
        assert_eq!(created.title, "Test Task");
    }

    #[tokio::test]
    async fn test_get_by_id_retrieves_task_correctly() {
        let conn = setup_test_db();
        let repo = SqliteTaskRepository::new(conn);
        let task = create_test_task("Test Task");

        repo.create(task.clone()).await.unwrap();
        let result = repo.get_by_id(&task.id).await;

        assert!(result.is_ok());
        let found = result.unwrap();
        assert!(found.is_some());
        let found_task = found.unwrap();
        assert_eq!(found_task.id, task.id);
        assert_eq!(found_task.title, "Test Task");
        assert_eq!(found_task.category, "feature");
    }

    #[tokio::test]
    async fn test_get_by_id_returns_none_for_nonexistent() {
        let conn = setup_test_db();
        let repo = SqliteTaskRepository::new(conn);
        let id = TaskId::new();

        let result = repo.get_by_id(&id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_by_project_returns_sorted_tasks() {
        let conn = setup_test_db();
        let repo = SqliteTaskRepository::new(conn);
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create tasks with different priorities
        let mut task1 = create_test_task("Low Priority");
        task1.priority = 1;

        let mut task2 = create_test_task("High Priority");
        task2.priority = 10;

        let mut task3 = create_test_task("Medium Priority");
        task3.priority = 5;

        repo.create(task1.clone()).await.unwrap();
        repo.create(task2.clone()).await.unwrap();
        repo.create(task3.clone()).await.unwrap();

        let result = repo.get_by_project(&project_id).await;

        assert!(result.is_ok());
        let tasks = result.unwrap();
        assert_eq!(tasks.len(), 3);
        // Should be sorted by priority DESC
        assert_eq!(tasks[0].title, "High Priority");
        assert_eq!(tasks[1].title, "Medium Priority");
        assert_eq!(tasks[2].title, "Low Priority");
    }

    #[tokio::test]
    async fn test_update_modifies_task_fields() {
        let conn = setup_test_db();
        let repo = SqliteTaskRepository::new(conn);
        let mut task = create_test_task("Original Title");

        repo.create(task.clone()).await.unwrap();

        task.title = "Updated Title".to_string();
        task.priority = 99;
        task.description = Some("New description".to_string());

        let update_result = repo.update(&task).await;
        assert!(update_result.is_ok());

        let found = repo.get_by_id(&task.id).await.unwrap().unwrap();
        assert_eq!(found.title, "Updated Title");
        assert_eq!(found.priority, 99);
        assert_eq!(found.description, Some("New description".to_string()));
    }

    #[tokio::test]
    async fn test_delete_removes_task_from_database() {
        let conn = setup_test_db();
        let repo = SqliteTaskRepository::new(conn);
        let task = create_test_task("To Delete");

        repo.create(task.clone()).await.unwrap();

        let delete_result = repo.delete(&task.id).await;
        assert!(delete_result.is_ok());

        let found = repo.get_by_id(&task.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_create_and_retrieve_preserves_all_fields() {
        let conn = setup_test_db();
        let repo = SqliteTaskRepository::new(conn);

        let mut task = create_test_task("Full Task");
        task.description = Some("A description".to_string());
        task.priority = 42;
        task.internal_status = InternalStatus::Ready;

        repo.create(task.clone()).await.unwrap();
        let found = repo.get_by_id(&task.id).await.unwrap().unwrap();

        assert_eq!(found.id, task.id);
        assert_eq!(found.project_id, task.project_id);
        assert_eq!(found.category, task.category);
        assert_eq!(found.title, task.title);
        assert_eq!(found.description, task.description);
        assert_eq!(found.priority, task.priority);
        assert_eq!(found.internal_status, task.internal_status);
    }

    #[tokio::test]
    async fn test_get_by_project_returns_empty_for_no_tasks() {
        let conn = setup_test_db();
        let repo = SqliteTaskRepository::new(conn);
        let project_id = ProjectId::from_string("test-project".to_string());

        let result = repo.get_by_project(&project_id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_by_project_only_returns_matching_project() {
        let conn = setup_test_db();

        // Add another project
        {
            let lock = conn;
            lock.execute(
                "INSERT INTO projects (id, name, working_directory) VALUES ('other-project', 'Other', '/other')",
                [],
            )
            .unwrap();

            let repo = SqliteTaskRepository::new(lock);

            let task1 = create_test_task("Task 1");
            let task2 = Task::new_with_category(
                ProjectId::from_string("other-project".to_string()),
                "Task 2".to_string(),
                "feature".to_string(),
            );

            repo.create(task1).await.unwrap();
            repo.create(task2).await.unwrap();

            let project_id = ProjectId::from_string("test-project".to_string());
            let result = repo.get_by_project(&project_id).await;

            assert!(result.is_ok());
            let tasks = result.unwrap();
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].title, "Task 1");
        }
    }

    // ==================== STATUS OPERATION TESTS ====================

    #[tokio::test]
    async fn test_persist_status_change_updates_task_status() {
        let conn = setup_test_db();
        let repo = SqliteTaskRepository::new(conn);
        let task = create_test_task("Test Task");

        repo.create(task.clone()).await.unwrap();

        let result = repo
            .persist_status_change(
                &task.id,
                InternalStatus::Backlog,
                InternalStatus::Ready,
                "user",
            )
            .await;

        assert!(result.is_ok());

        // Verify task status was updated
        let found = repo.get_by_id(&task.id).await.unwrap().unwrap();
        assert_eq!(found.internal_status, InternalStatus::Ready);
    }

    #[tokio::test]
    async fn test_persist_status_change_creates_history_record() {
        let conn = setup_test_db();
        let repo = SqliteTaskRepository::new(conn);
        let task = create_test_task("Test Task");

        repo.create(task.clone()).await.unwrap();

        repo.persist_status_change(
            &task.id,
            InternalStatus::Backlog,
            InternalStatus::Ready,
            "system",
        )
        .await
        .unwrap();

        let history = repo.get_status_history(&task.id).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].from, InternalStatus::Backlog);
        assert_eq!(history[0].to, InternalStatus::Ready);
        assert_eq!(history[0].trigger, "system");
    }

    #[tokio::test]
    async fn test_status_change_and_history_are_atomic() {
        let conn = setup_test_db();
        let repo = SqliteTaskRepository::new(conn);
        let task = create_test_task("Test Task");

        repo.create(task.clone()).await.unwrap();

        // Make multiple status changes
        repo.persist_status_change(
            &task.id,
            InternalStatus::Backlog,
            InternalStatus::Ready,
            "user",
        )
        .await
        .unwrap();

        repo.persist_status_change(
            &task.id,
            InternalStatus::Ready,
            InternalStatus::Executing,
            "agent",
        )
        .await
        .unwrap();

        // Verify both status and history are consistent
        let found = repo.get_by_id(&task.id).await.unwrap().unwrap();
        assert_eq!(found.internal_status, InternalStatus::Executing);

        let history = repo.get_status_history(&task.id).await.unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[1].from, InternalStatus::Ready);
        assert_eq!(history[1].to, InternalStatus::Executing);
    }

    #[tokio::test]
    async fn test_get_status_history_returns_transitions_in_order() {
        let conn = setup_test_db();
        let repo = SqliteTaskRepository::new(conn);
        let task = create_test_task("Test Task");

        repo.create(task.clone()).await.unwrap();

        // Create a sequence of transitions
        repo.persist_status_change(
            &task.id,
            InternalStatus::Backlog,
            InternalStatus::Ready,
            "step1",
        )
        .await
        .unwrap();

        repo.persist_status_change(
            &task.id,
            InternalStatus::Ready,
            InternalStatus::Executing,
            "step2",
        )
        .await
        .unwrap();

        repo.persist_status_change(
            &task.id,
            InternalStatus::Executing,
            InternalStatus::ExecutionDone,
            "step3",
        )
        .await
        .unwrap();

        let history = repo.get_status_history(&task.id).await.unwrap();

        assert_eq!(history.len(), 3);
        // Should be in chronological order (oldest first)
        assert_eq!(history[0].trigger, "step1");
        assert_eq!(history[1].trigger, "step2");
        assert_eq!(history[2].trigger, "step3");
    }

    #[tokio::test]
    async fn test_get_status_history_returns_empty_for_no_transitions() {
        let conn = setup_test_db();
        let repo = SqliteTaskRepository::new(conn);
        let task = create_test_task("Test Task");

        repo.create(task.clone()).await.unwrap();

        let history = repo.get_status_history(&task.id).await.unwrap();
        assert!(history.is_empty());
    }

    #[tokio::test]
    async fn test_get_by_status_filters_correctly() {
        let conn = setup_test_db();
        let repo = SqliteTaskRepository::new(conn);
        let project_id = ProjectId::from_string("test-project".to_string());

        let mut task1 = create_test_task("Backlog Task");
        task1.internal_status = InternalStatus::Backlog;

        let mut task2 = create_test_task("Ready Task 1");
        task2.internal_status = InternalStatus::Ready;

        let mut task3 = create_test_task("Ready Task 2");
        task3.internal_status = InternalStatus::Ready;

        let mut task4 = create_test_task("Executing Task");
        task4.internal_status = InternalStatus::Executing;

        repo.create(task1).await.unwrap();
        repo.create(task2).await.unwrap();
        repo.create(task3).await.unwrap();
        repo.create(task4).await.unwrap();

        let ready_tasks = repo
            .get_by_status(&project_id, InternalStatus::Ready)
            .await
            .unwrap();

        assert_eq!(ready_tasks.len(), 2);
        assert!(ready_tasks.iter().all(|t| t.internal_status == InternalStatus::Ready));
    }

    #[tokio::test]
    async fn test_get_by_status_returns_empty_for_no_matches() {
        let conn = setup_test_db();
        let repo = SqliteTaskRepository::new(conn);
        let project_id = ProjectId::from_string("test-project".to_string());

        let task = create_test_task("Backlog Task");
        repo.create(task).await.unwrap();

        let ready_tasks = repo
            .get_by_status(&project_id, InternalStatus::Ready)
            .await
            .unwrap();

        assert!(ready_tasks.is_empty());
    }

    // ==================== BLOCKER OPERATION TESTS ====================

    #[tokio::test]
    async fn test_add_blocker_creates_relationship() {
        let conn = setup_test_db();
        let repo = SqliteTaskRepository::new(conn);

        let task1 = create_test_task("Blocked Task");
        let task2 = create_test_task("Blocker Task");

        repo.create(task1.clone()).await.unwrap();
        repo.create(task2.clone()).await.unwrap();

        let result = repo.add_blocker(&task1.id, &task2.id).await;
        assert!(result.is_ok());

        let blockers = repo.get_blockers(&task1.id).await.unwrap();
        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].id, task2.id);
    }

    #[tokio::test]
    async fn test_resolve_blocker_removes_relationship() {
        let conn = setup_test_db();
        let repo = SqliteTaskRepository::new(conn);

        let task1 = create_test_task("Blocked Task");
        let task2 = create_test_task("Blocker Task");

        repo.create(task1.clone()).await.unwrap();
        repo.create(task2.clone()).await.unwrap();
        repo.add_blocker(&task1.id, &task2.id).await.unwrap();

        repo.resolve_blocker(&task1.id, &task2.id).await.unwrap();

        let blockers = repo.get_blockers(&task1.id).await.unwrap();
        assert!(blockers.is_empty());
    }

    #[tokio::test]
    async fn test_get_blockers_returns_blocking_tasks() {
        let conn = setup_test_db();
        let repo = SqliteTaskRepository::new(conn);

        let task1 = create_test_task("Blocked Task");
        let task2 = create_test_task("Blocker 1");
        let task3 = create_test_task("Blocker 2");

        repo.create(task1.clone()).await.unwrap();
        repo.create(task2.clone()).await.unwrap();
        repo.create(task3.clone()).await.unwrap();

        repo.add_blocker(&task1.id, &task2.id).await.unwrap();
        repo.add_blocker(&task1.id, &task3.id).await.unwrap();

        let blockers = repo.get_blockers(&task1.id).await.unwrap();
        assert_eq!(blockers.len(), 2);
    }

    #[tokio::test]
    async fn test_get_dependents_returns_dependent_tasks() {
        let conn = setup_test_db();
        let repo = SqliteTaskRepository::new(conn);

        let blocker = create_test_task("Blocker");
        let dep1 = create_test_task("Dependent 1");
        let dep2 = create_test_task("Dependent 2");

        repo.create(blocker.clone()).await.unwrap();
        repo.create(dep1.clone()).await.unwrap();
        repo.create(dep2.clone()).await.unwrap();

        // dep1 and dep2 are blocked by blocker
        repo.add_blocker(&dep1.id, &blocker.id).await.unwrap();
        repo.add_blocker(&dep2.id, &blocker.id).await.unwrap();

        let dependents = repo.get_dependents(&blocker.id).await.unwrap();
        assert_eq!(dependents.len(), 2);
    }

    #[tokio::test]
    async fn test_get_next_executable_excludes_blocked_tasks() {
        let conn = setup_test_db();
        let repo = SqliteTaskRepository::new(conn);
        let project_id = ProjectId::from_string("test-project".to_string());

        let mut blocked_task = create_test_task("Blocked Ready");
        blocked_task.internal_status = InternalStatus::Ready;
        blocked_task.priority = 10; // Higher priority

        let mut unblocked_task = create_test_task("Unblocked Ready");
        unblocked_task.internal_status = InternalStatus::Ready;
        unblocked_task.priority = 1; // Lower priority

        let blocker = create_test_task("Blocker");

        repo.create(blocked_task.clone()).await.unwrap();
        repo.create(unblocked_task.clone()).await.unwrap();
        repo.create(blocker.clone()).await.unwrap();

        repo.add_blocker(&blocked_task.id, &blocker.id).await.unwrap();

        // Should return unblocked task even though blocked has higher priority
        let next = repo.get_next_executable(&project_id).await.unwrap();
        assert!(next.is_some());
        assert_eq!(next.unwrap().id, unblocked_task.id);
    }

    #[tokio::test]
    async fn test_get_next_executable_returns_highest_priority_ready() {
        let conn = setup_test_db();
        let repo = SqliteTaskRepository::new(conn);
        let project_id = ProjectId::from_string("test-project".to_string());

        let mut low = create_test_task("Low");
        low.internal_status = InternalStatus::Ready;
        low.priority = 1;

        let mut high = create_test_task("High");
        high.internal_status = InternalStatus::Ready;
        high.priority = 10;

        repo.create(low).await.unwrap();
        repo.create(high.clone()).await.unwrap();

        let next = repo.get_next_executable(&project_id).await.unwrap();
        assert!(next.is_some());
        assert_eq!(next.unwrap().id, high.id);
    }

    #[tokio::test]
    async fn test_get_next_executable_returns_none_when_no_ready_tasks() {
        let conn = setup_test_db();
        let repo = SqliteTaskRepository::new(conn);
        let project_id = ProjectId::from_string("test-project".to_string());

        let task = create_test_task("Backlog Task"); // Default status is Backlog
        repo.create(task).await.unwrap();

        let next = repo.get_next_executable(&project_id).await.unwrap();
        assert!(next.is_none());
    }
}
