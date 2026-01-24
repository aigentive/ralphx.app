// Project repository trait - domain layer abstraction
//
// This trait defines the contract for project persistence.
// Implementations can use SQLite, PostgreSQL, in-memory, etc.

use async_trait::async_trait;

use crate::domain::entities::{Project, ProjectId};
use crate::error::AppResult;

/// Repository trait for Project persistence.
/// Implementations can use SQLite, PostgreSQL, in-memory, etc.
#[async_trait]
pub trait ProjectRepository: Send + Sync {
    /// Create a new project
    async fn create(&self, project: Project) -> AppResult<Project>;

    /// Get project by ID
    async fn get_by_id(&self, id: &ProjectId) -> AppResult<Option<Project>>;

    /// Get all projects
    async fn get_all(&self) -> AppResult<Vec<Project>>;

    /// Update a project
    async fn update(&self, project: &Project) -> AppResult<()>;

    /// Delete a project
    async fn delete(&self, id: &ProjectId) -> AppResult<()>;

    /// Find project by working directory path
    async fn get_by_working_directory(&self, path: &str) -> AppResult<Option<Project>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // Mock implementation for testing trait object usage
    struct MockProjectRepository {
        return_project: Option<Project>,
    }

    impl MockProjectRepository {
        fn new() -> Self {
            Self { return_project: None }
        }

        fn with_project(project: Project) -> Self {
            Self {
                return_project: Some(project),
            }
        }
    }

    #[async_trait]
    impl ProjectRepository for MockProjectRepository {
        async fn create(&self, project: Project) -> AppResult<Project> {
            Ok(project)
        }

        async fn get_by_id(&self, _id: &ProjectId) -> AppResult<Option<Project>> {
            Ok(self.return_project.clone())
        }

        async fn get_all(&self) -> AppResult<Vec<Project>> {
            match &self.return_project {
                Some(p) => Ok(vec![p.clone()]),
                None => Ok(vec![]),
            }
        }

        async fn update(&self, _project: &Project) -> AppResult<()> {
            Ok(())
        }

        async fn delete(&self, _id: &ProjectId) -> AppResult<()> {
            Ok(())
        }

        async fn get_by_working_directory(&self, _path: &str) -> AppResult<Option<Project>> {
            Ok(self.return_project.clone())
        }
    }

    #[test]
    fn test_project_repository_trait_can_be_object_safe() {
        // Verify that ProjectRepository can be used as a trait object
        let repo: Arc<dyn ProjectRepository> = Arc::new(MockProjectRepository::new());
        assert!(Arc::strong_count(&repo) == 1);
    }

    #[tokio::test]
    async fn test_mock_project_repository_create() {
        let repo = MockProjectRepository::new();
        let project = Project::new("Test Project".to_string(), "/path/to/project".to_string());

        let result = repo.create(project.clone()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, project.id);
    }

    #[tokio::test]
    async fn test_mock_project_repository_get_by_id_returns_none() {
        let repo = MockProjectRepository::new();
        let project_id = ProjectId::new();

        let result = repo.get_by_id(&project_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_mock_project_repository_get_by_id_returns_project() {
        let project = Project::new("Test Project".to_string(), "/path/to/project".to_string());
        let repo = MockProjectRepository::with_project(project.clone());

        let result = repo.get_by_id(&project.id).await;
        assert!(result.is_ok());
        let returned = result.unwrap();
        assert!(returned.is_some());
        assert_eq!(returned.unwrap().id, project.id);
    }

    #[tokio::test]
    async fn test_mock_project_repository_get_all_empty() {
        let repo = MockProjectRepository::new();

        let result = repo.get_all().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_project_repository_get_all_with_project() {
        let project = Project::new("Test Project".to_string(), "/path/to/project".to_string());
        let repo = MockProjectRepository::with_project(project.clone());

        let result = repo.get_all().await;
        assert!(result.is_ok());
        let projects = result.unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].id, project.id);
    }

    #[tokio::test]
    async fn test_mock_project_repository_update() {
        let repo = MockProjectRepository::new();
        let project = Project::new("Test Project".to_string(), "/path/to/project".to_string());

        let result = repo.update(&project).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_project_repository_delete() {
        let repo = MockProjectRepository::new();
        let project_id = ProjectId::new();

        let result = repo.delete(&project_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_project_repository_get_by_working_directory_not_found() {
        let repo = MockProjectRepository::new();

        let result = repo.get_by_working_directory("/nonexistent/path").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_mock_project_repository_get_by_working_directory_found() {
        let project = Project::new("Test Project".to_string(), "/path/to/project".to_string());
        let repo = MockProjectRepository::with_project(project.clone());

        let result = repo.get_by_working_directory("/path/to/project").await;
        assert!(result.is_ok());
        let returned = result.unwrap();
        assert!(returned.is_some());
        assert_eq!(returned.unwrap().working_directory, "/path/to/project");
    }

    #[tokio::test]
    async fn test_project_repository_trait_object_in_arc() {
        let project = Project::new("Test Project".to_string(), "/path/to/project".to_string());
        let repo: Arc<dyn ProjectRepository> =
            Arc::new(MockProjectRepository::with_project(project.clone()));

        // Use through trait object
        let result = repo.get_by_id(&project.id).await;
        assert!(result.is_ok());

        let all = repo.get_all().await;
        assert!(all.is_ok());
        assert_eq!(all.unwrap().len(), 1);
    }
}
