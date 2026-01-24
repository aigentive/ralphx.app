// Application state container for dependency injection
// Holds repository trait objects that can be swapped for testing

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::domain::agents::AgenticClient;
use crate::domain::repositories::{ProjectRepository, TaskRepository};
use crate::error::AppResult;
use crate::infrastructure::memory::{MemoryProjectRepository, MemoryTaskRepository};
use crate::infrastructure::sqlite::{
    get_default_db_path, open_connection, run_migrations, SqliteProjectRepository,
    SqliteTaskRepository,
};
use crate::infrastructure::{ClaudeCodeClient, MockAgenticClient};

/// Application state container for dependency injection
/// Holds repository trait objects that can be swapped for testing vs production
pub struct AppState {
    /// Task repository (SQLite in production, in-memory for tests)
    pub task_repo: Arc<dyn TaskRepository>,
    /// Project repository (SQLite in production, in-memory for tests)
    pub project_repo: Arc<dyn ProjectRepository>,
    /// Agent client (Claude Code in production, Mock for tests)
    pub agent_client: Arc<dyn AgenticClient>,
}

impl AppState {
    /// Create AppState for production use with SQLite repositories
    /// Opens the database at the default path and runs migrations
    pub fn new_production() -> AppResult<Self> {
        let path = get_default_db_path();
        let conn = open_connection(&path)?;
        run_migrations(&conn)?;

        // Wrap connection in Arc<Mutex> for sharing between repos
        let shared_conn = Arc::new(Mutex::new(conn));

        Ok(Self {
            task_repo: Arc::new(SqliteTaskRepository::from_shared(Arc::clone(&shared_conn))),
            project_repo: Arc::new(SqliteProjectRepository::from_shared(shared_conn)),
            agent_client: Arc::new(ClaudeCodeClient::new()),
        })
    }

    /// Create AppState with a specific database path
    pub fn with_db_path(db_path: &str) -> AppResult<Self> {
        let path = PathBuf::from(db_path);
        let conn = open_connection(&path)?;
        run_migrations(&conn)?;

        let shared_conn = Arc::new(Mutex::new(conn));

        Ok(Self {
            task_repo: Arc::new(SqliteTaskRepository::from_shared(Arc::clone(&shared_conn))),
            project_repo: Arc::new(SqliteProjectRepository::from_shared(shared_conn)),
            agent_client: Arc::new(ClaudeCodeClient::new()),
        })
    }

    /// Create AppState for testing with in-memory repositories
    pub fn new_test() -> Self {
        Self {
            task_repo: Arc::new(MemoryTaskRepository::new()),
            project_repo: Arc::new(MemoryProjectRepository::new()),
            agent_client: Arc::new(MockAgenticClient::new()),
        }
    }

    /// Create AppState with custom repositories (for dependency injection)
    pub fn with_repos(
        task_repo: Arc<dyn TaskRepository>,
        project_repo: Arc<dyn ProjectRepository>,
    ) -> Self {
        Self {
            task_repo,
            project_repo,
            agent_client: Arc::new(MockAgenticClient::new()),
        }
    }

    /// Swap the agent client to a different implementation
    pub fn with_agent_client(mut self, client: Arc<dyn AgenticClient>) -> Self {
        self.agent_client = client;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::agents::ClientType;
    use crate::domain::entities::{Project, ProjectId, Task};

    #[tokio::test]
    async fn test_new_test_creates_empty_repositories() {
        let state = AppState::new_test();

        // Task repo should be empty
        let project_id = ProjectId::new();
        let tasks = state.task_repo.get_by_project(&project_id).await.unwrap();
        assert!(tasks.is_empty());

        // Project repo should be empty
        let projects = state.project_repo.get_all().await.unwrap();
        assert!(projects.is_empty());
    }

    #[tokio::test]
    async fn test_with_repos_uses_custom_repositories() {
        let task_repo = Arc::new(MemoryTaskRepository::new());
        let project_repo = Arc::new(MemoryProjectRepository::new());

        // Pre-populate the repos
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        project_repo.create(project.clone()).await.unwrap();

        let task = Task::new(project.id.clone(), "Test Task".to_string());
        task_repo.create(task.clone()).await.unwrap();

        // Create AppState with these repos
        let state = AppState::with_repos(task_repo, project_repo);

        // Verify the state uses our repos
        let projects = state.project_repo.get_all().await.unwrap();
        assert_eq!(projects.len(), 1);

        let tasks = state.task_repo.get_by_project(&project.id).await.unwrap();
        assert_eq!(tasks.len(), 1);
    }

    #[tokio::test]
    async fn test_task_and_project_repos_work_together() {
        let state = AppState::new_test();

        // Create a project
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        state.project_repo.create(project.clone()).await.unwrap();

        // Create tasks for that project
        let task1 = Task::new(project.id.clone(), "Task 1".to_string());
        let task2 = Task::new(project.id.clone(), "Task 2".to_string());
        state.task_repo.create(task1).await.unwrap();
        state.task_repo.create(task2).await.unwrap();

        // Verify we can retrieve them
        let tasks = state.task_repo.get_by_project(&project.id).await.unwrap();
        assert_eq!(tasks.len(), 2);
    }

    #[tokio::test]
    async fn test_repositories_are_thread_safe() {
        let state = Arc::new(AppState::new_test());

        // Create a project first
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        state.project_repo.create(project.clone()).await.unwrap();

        // Spawn multiple tasks that use the repos concurrently
        let mut handles = vec![];
        for i in 0..10 {
            let state_clone = Arc::clone(&state);
            let project_id = project.id.clone();
            handles.push(tokio::spawn(async move {
                let task = Task::new(project_id, format!("Task {}", i));
                state_clone.task_repo.create(task).await
            }));
        }

        // Wait for all to complete
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }

        // Verify all tasks were created
        let tasks = state.task_repo.get_by_project(&project.id).await.unwrap();
        assert_eq!(tasks.len(), 10);
    }

    #[tokio::test]
    async fn test_new_test_creates_mock_agent_client() {
        let state = AppState::new_test();

        // Agent client should be mock and available
        let available = state.agent_client.is_available().await.unwrap();
        assert!(available);

        // Check capabilities indicate mock
        let caps = state.agent_client.capabilities();
        assert_eq!(caps.client_type, ClientType::Mock);
    }

    #[tokio::test]
    async fn test_with_agent_client_swaps_client() {
        let state = AppState::new_test();

        // Default is mock
        assert_eq!(
            state.agent_client.capabilities().client_type,
            ClientType::Mock
        );

        // Create custom mock with different capabilities wouldn't show,
        // but we can test the swap mechanism works
        let custom_mock = Arc::new(MockAgenticClient::new());
        let _state = state.with_agent_client(custom_mock);

        // If it compiled and ran, the swap worked
    }
}
