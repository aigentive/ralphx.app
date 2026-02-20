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
#[path = "memory_project_repo_tests.rs"]
mod tests;
