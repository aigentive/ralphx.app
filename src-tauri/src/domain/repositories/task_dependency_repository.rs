// Task dependency repository trait - domain layer abstraction
//
// This trait defines the contract for task dependency persistence.
// Used for tasks that have been applied from proposals to track blockers.

use async_trait::async_trait;

use crate::domain::entities::TaskId;
use crate::error::AppResult;

/// Repository trait for task dependency persistence.
/// Implementations can use SQLite, PostgreSQL, in-memory, etc.
#[async_trait]
pub trait TaskDependencyRepository: Send + Sync {
    /// Add a dependency (task_id depends on depends_on_task_id)
    async fn add_dependency(
        &self,
        task_id: &TaskId,
        depends_on_task_id: &TaskId,
    ) -> AppResult<()>;

    /// Remove a dependency
    async fn remove_dependency(
        &self,
        task_id: &TaskId,
        depends_on_task_id: &TaskId,
    ) -> AppResult<()>;

    /// Get all tasks that this task depends on (blockers)
    async fn get_blockers(&self, task_id: &TaskId) -> AppResult<Vec<TaskId>>;

    /// Get all tasks that depend on this task (blocked by this)
    async fn get_blocked_by(&self, task_id: &TaskId) -> AppResult<Vec<TaskId>>;

    /// Check if adding a dependency would create a circular dependency
    async fn has_circular_dependency(
        &self,
        task_id: &TaskId,
        potential_dep: &TaskId,
    ) -> AppResult<bool>;

    /// Clear all dependencies for a task (both directions)
    async fn clear_dependencies(&self, task_id: &TaskId) -> AppResult<()>;

    /// Count blockers for a task
    async fn count_blockers(&self, task_id: &TaskId) -> AppResult<u32>;

    /// Count tasks blocked by this task
    async fn count_blocked_by(&self, task_id: &TaskId) -> AppResult<u32>;

    /// Check if a specific dependency exists
    async fn has_dependency(
        &self,
        task_id: &TaskId,
        depends_on_task_id: &TaskId,
    ) -> AppResult<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};
    use std::sync::Arc;

    // Mock implementation for testing trait object usage
    struct MockTaskDependencyRepository {
        // Map from task_id to set of depends_on_task_ids
        dependencies: HashMap<TaskId, HashSet<TaskId>>,
    }

    impl MockTaskDependencyRepository {
        fn new() -> Self {
            Self {
                dependencies: HashMap::new(),
            }
        }

        fn with_dependency(from: TaskId, to: TaskId) -> Self {
            let mut deps = HashMap::new();
            let mut set = HashSet::new();
            set.insert(to);
            deps.insert(from, set);
            Self { dependencies: deps }
        }

        fn with_dependencies(deps: Vec<(TaskId, TaskId)>) -> Self {
            let mut map: HashMap<TaskId, HashSet<TaskId>> = HashMap::new();
            for (from, to) in deps {
                map.entry(from).or_default().insert(to);
            }
            Self { dependencies: map }
        }
    }

    #[async_trait]
    impl TaskDependencyRepository for MockTaskDependencyRepository {
        async fn add_dependency(
            &self,
            _task_id: &TaskId,
            _depends_on_task_id: &TaskId,
        ) -> AppResult<()> {
            Ok(())
        }

        async fn remove_dependency(
            &self,
            _task_id: &TaskId,
            _depends_on_task_id: &TaskId,
        ) -> AppResult<()> {
            Ok(())
        }

        async fn get_blockers(&self, task_id: &TaskId) -> AppResult<Vec<TaskId>> {
            Ok(self
                .dependencies
                .get(task_id)
                .map(|set| set.iter().cloned().collect())
                .unwrap_or_default())
        }

        async fn get_blocked_by(&self, task_id: &TaskId) -> AppResult<Vec<TaskId>> {
            // Find all tasks that have this task in their dependency set
            Ok(self
                .dependencies
                .iter()
                .filter_map(|(id, deps)| {
                    if deps.contains(task_id) {
                        Some(id.clone())
                    } else {
                        None
                    }
                })
                .collect())
        }

        async fn has_circular_dependency(
            &self,
            task_id: &TaskId,
            potential_dep: &TaskId,
        ) -> AppResult<bool> {
            // Simple check: if potential_dep already depends on task_id, adding this would create a cycle
            if let Some(deps) = self.dependencies.get(potential_dep) {
                if deps.contains(task_id) {
                    return Ok(true);
                }
            }
            // Also check for self-dependency
            if task_id == potential_dep {
                return Ok(true);
            }
            Ok(false)
        }

        async fn clear_dependencies(&self, _task_id: &TaskId) -> AppResult<()> {
            Ok(())
        }

        async fn count_blockers(&self, task_id: &TaskId) -> AppResult<u32> {
            Ok(self
                .dependencies
                .get(task_id)
                .map(|set| set.len() as u32)
                .unwrap_or(0))
        }

        async fn count_blocked_by(&self, task_id: &TaskId) -> AppResult<u32> {
            Ok(self
                .dependencies
                .iter()
                .filter(|(_, deps)| deps.contains(task_id))
                .count() as u32)
        }

        async fn has_dependency(
            &self,
            task_id: &TaskId,
            depends_on_task_id: &TaskId,
        ) -> AppResult<bool> {
            Ok(self
                .dependencies
                .get(task_id)
                .map(|set| set.contains(depends_on_task_id))
                .unwrap_or(false))
        }
    }

    #[test]
    fn test_task_dependency_repository_trait_can_be_object_safe() {
        // Verify that TaskDependencyRepository can be used as a trait object
        let repo: Arc<dyn TaskDependencyRepository> =
            Arc::new(MockTaskDependencyRepository::new());
        assert!(Arc::strong_count(&repo) == 1);
    }

    #[tokio::test]
    async fn test_mock_repository_add_dependency() {
        let repo = MockTaskDependencyRepository::new();
        let task_id = TaskId::new();
        let depends_on_id = TaskId::new();

        let result = repo.add_dependency(&task_id, &depends_on_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_repository_remove_dependency() {
        let repo = MockTaskDependencyRepository::new();
        let task_id = TaskId::new();
        let depends_on_id = TaskId::new();

        let result = repo.remove_dependency(&task_id, &depends_on_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_repository_get_blockers_empty() {
        let repo = MockTaskDependencyRepository::new();
        let task_id = TaskId::new();

        let result = repo.get_blockers(&task_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_repository_get_blockers_with_deps() {
        let task_id = TaskId::new();
        let blocker_id = TaskId::new();
        let repo = MockTaskDependencyRepository::with_dependency(
            task_id.clone(),
            blocker_id.clone(),
        );

        let result = repo.get_blockers(&task_id).await;
        assert!(result.is_ok());
        let blockers = result.unwrap();
        assert_eq!(blockers.len(), 1);
        assert!(blockers.contains(&blocker_id));
    }

    #[tokio::test]
    async fn test_mock_repository_get_blocked_by_empty() {
        let repo = MockTaskDependencyRepository::new();
        let task_id = TaskId::new();

        let result = repo.get_blocked_by(&task_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_repository_get_blocked_by_with_dependents() {
        let task_a = TaskId::new();
        let task_b = TaskId::new();
        let task_c = TaskId::new();
        // A depends on C, B depends on C -> C blocks A and B
        let repo = MockTaskDependencyRepository::with_dependencies(vec![
            (task_a.clone(), task_c.clone()),
            (task_b.clone(), task_c.clone()),
        ]);

        let result = repo.get_blocked_by(&task_c).await;
        assert!(result.is_ok());
        let blocked = result.unwrap();
        assert_eq!(blocked.len(), 2);
        assert!(blocked.contains(&task_a));
        assert!(blocked.contains(&task_b));
    }

    #[tokio::test]
    async fn test_mock_repository_has_circular_dependency_false() {
        let repo = MockTaskDependencyRepository::new();
        let task_a = TaskId::new();
        let task_b = TaskId::new();

        // No existing deps, so no cycle
        let result = repo.has_circular_dependency(&task_a, &task_b).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_mock_repository_has_circular_dependency_self() {
        let repo = MockTaskDependencyRepository::new();
        let task_a = TaskId::new();

        // Self-dependency always creates a cycle
        let result = repo.has_circular_dependency(&task_a, &task_a).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_mock_repository_has_circular_dependency_direct() {
        let task_a = TaskId::new();
        let task_b = TaskId::new();
        // B depends on A
        let repo = MockTaskDependencyRepository::with_dependency(
            task_b.clone(),
            task_a.clone(),
        );

        // Adding A depends on B would create A -> B -> A cycle
        let result = repo.has_circular_dependency(&task_a, &task_b).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_mock_repository_clear_dependencies() {
        let repo = MockTaskDependencyRepository::new();
        let task_id = TaskId::new();

        let result = repo.clear_dependencies(&task_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_repository_count_blockers_zero() {
        let repo = MockTaskDependencyRepository::new();
        let task_id = TaskId::new();

        let result = repo.count_blockers(&task_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_mock_repository_count_blockers_multiple() {
        let task_a = TaskId::new();
        let task_b = TaskId::new();
        let task_c = TaskId::new();
        // A depends on B and C
        let repo = MockTaskDependencyRepository::with_dependencies(vec![
            (task_a.clone(), task_b.clone()),
            (task_a.clone(), task_c.clone()),
        ]);

        let result = repo.count_blockers(&task_a).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_mock_repository_count_blocked_by_zero() {
        let repo = MockTaskDependencyRepository::new();
        let task_id = TaskId::new();

        let result = repo.count_blocked_by(&task_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_mock_repository_count_blocked_by_multiple() {
        let task_a = TaskId::new();
        let task_b = TaskId::new();
        let task_c = TaskId::new();
        // B and C both depend on A
        let repo = MockTaskDependencyRepository::with_dependencies(vec![
            (task_b.clone(), task_a.clone()),
            (task_c.clone(), task_a.clone()),
        ]);

        let result = repo.count_blocked_by(&task_a).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_mock_repository_has_dependency_false() {
        let repo = MockTaskDependencyRepository::new();
        let task_a = TaskId::new();
        let task_b = TaskId::new();

        let result = repo.has_dependency(&task_a, &task_b).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_mock_repository_has_dependency_true() {
        let task_a = TaskId::new();
        let task_b = TaskId::new();
        let repo = MockTaskDependencyRepository::with_dependency(task_a.clone(), task_b.clone());

        let result = repo.has_dependency(&task_a, &task_b).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_repository_trait_object_in_arc() {
        let task_a = TaskId::new();
        let task_b = TaskId::new();
        let repo: Arc<dyn TaskDependencyRepository> = Arc::new(
            MockTaskDependencyRepository::with_dependency(task_a.clone(), task_b.clone()),
        );

        // Use through trait object
        let blockers = repo.get_blockers(&task_a).await;
        assert!(blockers.is_ok());
        assert_eq!(blockers.unwrap().len(), 1);

        let blocked = repo.get_blocked_by(&task_b).await;
        assert!(blocked.is_ok());
        assert_eq!(blocked.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_repository_trait_object_cycle_detection() {
        let task_a = TaskId::new();
        let task_b = TaskId::new();
        let repo: Arc<dyn TaskDependencyRepository> = Arc::new(
            MockTaskDependencyRepository::with_dependency(task_b.clone(), task_a.clone()),
        );

        let would_cycle = repo.has_circular_dependency(&task_a, &task_b).await;
        assert!(would_cycle.is_ok());
        assert!(would_cycle.unwrap());
    }

    #[tokio::test]
    async fn test_repository_trait_object_count_operations() {
        let task_a = TaskId::new();
        let task_b = TaskId::new();
        let repo: Arc<dyn TaskDependencyRepository> = Arc::new(
            MockTaskDependencyRepository::with_dependency(task_a.clone(), task_b.clone()),
        );

        let blocker_count = repo.count_blockers(&task_a).await;
        assert!(blocker_count.is_ok());
        assert_eq!(blocker_count.unwrap(), 1);

        let blocked_count = repo.count_blocked_by(&task_b).await;
        assert!(blocked_count.is_ok());
        assert_eq!(blocked_count.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_repository_trait_object_add_remove_clear() {
        let repo: Arc<dyn TaskDependencyRepository> =
            Arc::new(MockTaskDependencyRepository::new());
        let task_a = TaskId::new();
        let task_b = TaskId::new();

        let add_result = repo.add_dependency(&task_a, &task_b).await;
        assert!(add_result.is_ok());

        let remove_result = repo.remove_dependency(&task_a, &task_b).await;
        assert!(remove_result.is_ok());

        let clear_result = repo.clear_dependencies(&task_a).await;
        assert!(clear_result.is_ok());
    }

    #[tokio::test]
    async fn test_repository_trait_object_has_dependency() {
        let task_a = TaskId::new();
        let task_b = TaskId::new();
        let repo: Arc<dyn TaskDependencyRepository> = Arc::new(
            MockTaskDependencyRepository::with_dependency(task_a.clone(), task_b.clone()),
        );

        let has_dep = repo.has_dependency(&task_a, &task_b).await;
        assert!(has_dep.is_ok());
        assert!(has_dep.unwrap());

        let no_dep = repo.has_dependency(&task_b, &task_a).await;
        assert!(no_dep.is_ok());
        assert!(!no_dep.unwrap());
    }
}
