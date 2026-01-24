// Memory-based ProjectRepository implementation for testing
// Full implementation will be added in a later task

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::domain::entities::{Project, ProjectId};

/// In-memory implementation of ProjectRepository for testing
/// Uses RwLock<HashMap> for thread-safe storage
pub struct MemoryProjectRepository {
    pub(crate) projects: Arc<RwLock<HashMap<ProjectId, Project>>>,
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
