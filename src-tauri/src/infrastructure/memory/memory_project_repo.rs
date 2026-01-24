// Memory-based ProjectRepository implementation for testing
// Uses RwLock<HashMap> for thread-safe storage

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;

use crate::domain::entities::{Project, ProjectId};
use crate::domain::repositories::ProjectRepository;
use crate::error::AppResult;

/// In-memory implementation of ProjectRepository for testing
/// Uses RwLock<HashMap> for thread-safe storage
pub struct MemoryProjectRepository {
    projects: Arc<RwLock<HashMap<ProjectId, Project>>>,
}

impl Default for MemoryProjectRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryProjectRepository {
    /// Create a new empty in-memory project repository
    pub fn new() -> Self {
        Self {
            projects: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create with pre-populated projects (for tests)
    pub fn with_projects(projects: Vec<Project>) -> Self {
        let map: HashMap<ProjectId, Project> =
            projects.into_iter().map(|p| (p.id.clone(), p)).collect();
        Self {
            projects: Arc::new(RwLock::new(map)),
        }
    }
}

#[async_trait]
impl ProjectRepository for MemoryProjectRepository {
    async fn create(&self, project: Project) -> AppResult<Project> {
        let mut projects = self.projects.write().await;
        projects.insert(project.id.clone(), project.clone());
        Ok(project)
    }

    async fn get_by_id(&self, id: &ProjectId) -> AppResult<Option<Project>> {
        let projects = self.projects.read().await;
        Ok(projects.get(id).cloned())
    }

    async fn get_all(&self) -> AppResult<Vec<Project>> {
        let projects = self.projects.read().await;
        Ok(projects.values().cloned().collect())
    }

    async fn update(&self, project: &Project) -> AppResult<()> {
        let mut projects = self.projects.write().await;
        projects.insert(project.id.clone(), project.clone());
        Ok(())
    }

    async fn delete(&self, id: &ProjectId) -> AppResult<()> {
        let mut projects = self.projects.write().await;
        projects.remove(id);
        Ok(())
    }

    async fn get_by_working_directory(&self, path: &str) -> AppResult<Option<Project>> {
        let projects = self.projects.read().await;
        Ok(projects
            .values()
            .find(|p| p.working_directory == path)
            .cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::GitMode;

    fn create_test_project(name: &str, path: &str) -> Project {
        Project::new(name.to_string(), path.to_string())
    }

    // ==================== CREATE TESTS ====================

    #[tokio::test]
    async fn test_create_project_succeeds() {
        let repo = MemoryProjectRepository::new();
        let project = create_test_project("Test Project", "/path/to/project");

        let result = repo.create(project.clone()).await;

        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.id, project.id);
        assert_eq!(created.name, "Test Project");
        assert_eq!(created.working_directory, "/path/to/project");
    }

    #[tokio::test]
    async fn test_create_project_can_be_retrieved() {
        let repo = MemoryProjectRepository::new();
        let project = create_test_project("Test Project", "/path/to/project");

        repo.create(project.clone()).await.unwrap();

        let found = repo.get_by_id(&project.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test Project");
    }

    #[tokio::test]
    async fn test_create_overwrites_duplicate_id() {
        let project = create_test_project("Test Project", "/path/to/project");
        let repo = MemoryProjectRepository::with_projects(vec![project.clone()]);

        // Create another project with the same ID but different name
        let mut updated = project.clone();
        updated.name = "Updated Name".to_string();

        let result = repo.create(updated).await;

        assert!(result.is_ok());
        let found = repo.get_by_id(&project.id).await.unwrap().unwrap();
        assert_eq!(found.name, "Updated Name");
    }

    // ==================== GET BY ID TESTS ====================

    #[tokio::test]
    async fn test_get_by_id_returns_project_when_exists() {
        let project = create_test_project("Test Project", "/path/to/project");
        let repo = MemoryProjectRepository::with_projects(vec![project.clone()]);

        let result = repo.get_by_id(&project.id).await;

        assert!(result.is_ok());
        let found = result.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, project.id);
    }

    #[tokio::test]
    async fn test_get_by_id_returns_none_when_not_exists() {
        let repo = MemoryProjectRepository::new();
        let id = ProjectId::new();

        let result = repo.get_by_id(&id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    // ==================== GET ALL TESTS ====================

    #[tokio::test]
    async fn test_get_all_returns_empty_when_no_projects() {
        let repo = MemoryProjectRepository::new();

        let result = repo.get_all().await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_all_returns_all_projects() {
        let project1 = create_test_project("Project 1", "/path/one");
        let project2 = create_test_project("Project 2", "/path/two");
        let project3 = create_test_project("Project 3", "/path/three");
        let repo =
            MemoryProjectRepository::with_projects(vec![project1, project2, project3]);

        let result = repo.get_all().await;

        assert!(result.is_ok());
        let projects = result.unwrap();
        assert_eq!(projects.len(), 3);
    }

    // ==================== UPDATE TESTS ====================

    #[tokio::test]
    async fn test_update_project_succeeds() {
        let mut project = create_test_project("Original Name", "/path/to/project");
        let repo = MemoryProjectRepository::with_projects(vec![project.clone()]);

        project.name = "Updated Name".to_string();
        project.git_mode = GitMode::Worktree;

        let result = repo.update(&project).await;

        assert!(result.is_ok());

        // Verify the update persisted
        let found = repo.get_by_id(&project.id).await.unwrap().unwrap();
        assert_eq!(found.name, "Updated Name");
        assert_eq!(found.git_mode, GitMode::Worktree);
    }

    #[tokio::test]
    async fn test_update_nonexistent_project_creates_it() {
        let repo = MemoryProjectRepository::new();
        let project = create_test_project("Test Project", "/path/to/project");

        // Update on nonexistent inserts it (HashMap::insert behavior)
        let result = repo.update(&project).await;

        assert!(result.is_ok());
        let found = repo.get_by_id(&project.id).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_update_working_directory() {
        let mut project = create_test_project("Test Project", "/path/to/project");
        let repo = MemoryProjectRepository::with_projects(vec![project.clone()]);

        project.working_directory = "/new/path".to_string();

        let result = repo.update(&project).await;

        assert!(result.is_ok());
        let found = repo.get_by_id(&project.id).await.unwrap().unwrap();
        assert_eq!(found.working_directory, "/new/path");
    }

    // ==================== DELETE TESTS ====================

    #[tokio::test]
    async fn test_delete_project_succeeds() {
        let project = create_test_project("Test Project", "/path/to/project");
        let repo = MemoryProjectRepository::with_projects(vec![project.clone()]);

        let result = repo.delete(&project.id).await;

        assert!(result.is_ok());

        // Verify deletion
        let found = repo.get_by_id(&project.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_project_succeeds() {
        let repo = MemoryProjectRepository::new();
        let id = ProjectId::new();

        // Delete on nonexistent is a no-op (HashMap::remove behavior)
        let result = repo.delete(&id).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_only_removes_specified_project() {
        let project1 = create_test_project("Project 1", "/path/one");
        let project2 = create_test_project("Project 2", "/path/two");
        let repo =
            MemoryProjectRepository::with_projects(vec![project1.clone(), project2.clone()]);

        repo.delete(&project1.id).await.unwrap();

        assert!(repo.get_by_id(&project1.id).await.unwrap().is_none());
        assert!(repo.get_by_id(&project2.id).await.unwrap().is_some());
    }

    // ==================== GET BY WORKING DIRECTORY TESTS ====================

    #[tokio::test]
    async fn test_get_by_working_directory_returns_project_when_found() {
        let project = create_test_project("Test Project", "/path/to/project");
        let repo = MemoryProjectRepository::with_projects(vec![project.clone()]);

        let result = repo.get_by_working_directory("/path/to/project").await;

        assert!(result.is_ok());
        let found = result.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, project.id);
    }

    #[tokio::test]
    async fn test_get_by_working_directory_returns_none_when_not_found() {
        let project = create_test_project("Test Project", "/path/to/project");
        let repo = MemoryProjectRepository::with_projects(vec![project]);

        let result = repo.get_by_working_directory("/different/path").await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_by_working_directory_empty_repo() {
        let repo = MemoryProjectRepository::new();

        let result = repo.get_by_working_directory("/any/path").await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_by_working_directory_finds_correct_project() {
        let project1 = create_test_project("Project 1", "/path/one");
        let project2 = create_test_project("Project 2", "/path/two");
        let repo =
            MemoryProjectRepository::with_projects(vec![project1.clone(), project2.clone()]);

        let found = repo.get_by_working_directory("/path/two").await.unwrap();

        assert!(found.is_some());
        assert_eq!(found.unwrap().id, project2.id);
    }

    // ==================== THREAD SAFETY TESTS ====================

    #[tokio::test]
    async fn test_concurrent_reads() {
        let project = create_test_project("Test Project", "/path/to/project");
        let repo = Arc::new(MemoryProjectRepository::with_projects(vec![project.clone()]));

        let mut handles = vec![];
        for _ in 0..10 {
            let repo_clone = Arc::clone(&repo);
            let id_clone = project.id.clone();
            handles.push(tokio::spawn(async move {
                repo_clone.get_by_id(&id_clone).await
            }));
        }

        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
            assert!(result.unwrap().is_some());
        }
    }

    #[tokio::test]
    async fn test_concurrent_creates_with_different_paths() {
        let repo = Arc::new(MemoryProjectRepository::new());

        let mut handles = vec![];
        for i in 0..10 {
            let repo_clone = Arc::clone(&repo);
            handles.push(tokio::spawn(async move {
                let project =
                    create_test_project(&format!("Project {}", i), &format!("/path/{}", i));
                repo_clone.create(project).await
            }));
        }

        let mut successes = 0;
        for handle in handles {
            if handle.await.unwrap().is_ok() {
                successes += 1;
            }
        }

        assert_eq!(successes, 10);
        assert_eq!(repo.get_all().await.unwrap().len(), 10);
    }

    // ==================== DEFAULT TRAIT TEST ====================

    #[tokio::test]
    async fn test_default_creates_empty_repository() {
        let repo = MemoryProjectRepository::default();

        let result = repo.get_all().await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
