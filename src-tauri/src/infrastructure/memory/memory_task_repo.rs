// Memory-based TaskRepository implementation for testing
// Full implementation will be added in the next task

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::domain::entities::{Task, TaskId};
use crate::domain::repositories::StatusTransition;

/// In-memory implementation of TaskRepository for testing
/// Uses RwLock<HashMap> for thread-safe storage
pub struct MemoryTaskRepository {
    pub(crate) tasks: Arc<RwLock<HashMap<TaskId, Task>>>,
    pub(crate) history: Arc<RwLock<Vec<(TaskId, StatusTransition)>>>,
    pub(crate) blockers: Arc<RwLock<HashMap<TaskId, Vec<TaskId>>>>,
}

impl Default for MemoryTaskRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryTaskRepository {
    /// Create a new empty in-memory task repository
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            blockers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create with pre-populated tasks (for tests)
    pub fn with_tasks(tasks: Vec<Task>) -> Self {
        let map: HashMap<TaskId, Task> = tasks.into_iter().map(|t| (t.id.clone(), t)).collect();
        Self {
            tasks: Arc::new(RwLock::new(map)),
            history: Arc::new(RwLock::new(Vec::new())),
            blockers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}
