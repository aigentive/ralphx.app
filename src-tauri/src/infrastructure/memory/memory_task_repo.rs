// Memory-based TaskRepository implementation for testing
// Uses RwLock<HashMap> for thread-safe storage without a real database

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;
use chrono::Utc;

use crate::domain::entities::{InternalStatus, ProjectId, Task, TaskId};
use crate::domain::repositories::{StatusTransition, TaskRepository};
use crate::error::AppResult;

/// In-memory implementation of TaskRepository for testing
/// Uses RwLock<HashMap> for thread-safe storage
pub struct MemoryTaskRepository {
    tasks: Arc<RwLock<HashMap<TaskId, Task>>>,
    history: Arc<RwLock<Vec<(TaskId, StatusTransition)>>>,
    blockers: Arc<RwLock<HashMap<TaskId, Vec<TaskId>>>>,
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

#[async_trait]
impl TaskRepository for MemoryTaskRepository {
    async fn create(&self, task: Task) -> AppResult<Task> {
        let mut tasks = self.tasks.write().await;
        tasks.insert(task.id.clone(), task.clone());
        Ok(task)
    }

    async fn get_by_id(&self, id: &TaskId) -> AppResult<Option<Task>> {
        let tasks = self.tasks.read().await;
        Ok(tasks.get(id).cloned())
    }

    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<Task>> {
        let tasks = self.tasks.read().await;
        let mut result: Vec<Task> = tasks
            .values()
            .filter(|t| t.project_id == *project_id)
            .cloned()
            .collect();
        // Sort by priority (desc) then created_at (asc)
        result.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| a.created_at.cmp(&b.created_at))
        });
        Ok(result)
    }

    async fn update(&self, task: &Task) -> AppResult<()> {
        let mut tasks = self.tasks.write().await;
        tasks.insert(task.id.clone(), task.clone());
        Ok(())
    }

    async fn delete(&self, id: &TaskId) -> AppResult<()> {
        let mut tasks = self.tasks.write().await;
        tasks.remove(id);
        // Also remove any blockers referencing this task
        let mut blockers = self.blockers.write().await;
        blockers.remove(id);
        for blocked_by in blockers.values_mut() {
            blocked_by.retain(|blocker_id| blocker_id != id);
        }
        Ok(())
    }

    async fn get_by_status(
        &self,
        project_id: &ProjectId,
        status: InternalStatus,
    ) -> AppResult<Vec<Task>> {
        let tasks = self.tasks.read().await;
        let mut result: Vec<Task> = tasks
            .values()
            .filter(|t| t.project_id == *project_id && t.internal_status == status)
            .cloned()
            .collect();
        result.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| a.created_at.cmp(&b.created_at))
        });
        Ok(result)
    }

    async fn persist_status_change(
        &self,
        id: &TaskId,
        from: InternalStatus,
        to: InternalStatus,
        trigger: &str,
    ) -> AppResult<()> {
        // Update task status
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(id) {
            task.internal_status = to;
            task.updated_at = Utc::now();
        }
        drop(tasks);

        // Record history
        let mut history = self.history.write().await;
        history.push((
            id.clone(),
            StatusTransition::new(from, to, trigger),
        ));

        Ok(())
    }

    async fn get_status_history(&self, id: &TaskId) -> AppResult<Vec<StatusTransition>> {
        let history = self.history.read().await;
        let transitions: Vec<StatusTransition> = history
            .iter()
            .filter(|(task_id, _)| task_id == id)
            .map(|(_, transition)| transition.clone())
            .collect();
        Ok(transitions)
    }

    async fn get_next_executable(&self, project_id: &ProjectId) -> AppResult<Option<Task>> {
        let tasks = self.tasks.read().await;
        let blockers = self.blockers.read().await;

        let mut ready_tasks: Vec<&Task> = tasks
            .values()
            .filter(|t| {
                t.project_id == *project_id
                    && t.internal_status == InternalStatus::Ready
                    && !blockers.get(&t.id).map(|b| !b.is_empty()).unwrap_or(false)
            })
            .collect();

        ready_tasks.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| a.created_at.cmp(&b.created_at))
        });

        Ok(ready_tasks.first().cloned().cloned())
    }

    async fn get_blockers(&self, id: &TaskId) -> AppResult<Vec<Task>> {
        let tasks = self.tasks.read().await;
        let blockers = self.blockers.read().await;

        let blocker_ids = blockers.get(id).cloned().unwrap_or_default();
        let blocker_tasks: Vec<Task> = blocker_ids
            .iter()
            .filter_map(|blocker_id| tasks.get(blocker_id).cloned())
            .collect();

        Ok(blocker_tasks)
    }

    async fn get_dependents(&self, id: &TaskId) -> AppResult<Vec<Task>> {
        let tasks = self.tasks.read().await;
        let blockers = self.blockers.read().await;

        // Find all tasks that have this task as a blocker
        let dependent_ids: Vec<TaskId> = blockers
            .iter()
            .filter(|(_, blocked_by)| blocked_by.contains(id))
            .map(|(task_id, _)| task_id.clone())
            .collect();

        let dependent_tasks: Vec<Task> = dependent_ids
            .iter()
            .filter_map(|task_id| tasks.get(task_id).cloned())
            .collect();

        Ok(dependent_tasks)
    }

    async fn add_blocker(&self, task_id: &TaskId, blocker_id: &TaskId) -> AppResult<()> {
        let mut blockers = self.blockers.write().await;
        blockers
            .entry(task_id.clone())
            .or_default()
            .push(blocker_id.clone());
        Ok(())
    }

    async fn resolve_blocker(&self, task_id: &TaskId, blocker_id: &TaskId) -> AppResult<()> {
        let mut blockers = self.blockers.write().await;
        if let Some(blocked_by) = blockers.get_mut(task_id) {
            blocked_by.retain(|id| id != blocker_id);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_task(project_id: ProjectId, title: &str, priority: i32) -> Task {
        let mut task = Task::new(project_id, title.to_string());
        task.priority = priority;
        task
    }

    // ===== CRUD Tests =====

    #[tokio::test]
    async fn test_create_returns_task_with_id() {
        let repo = MemoryTaskRepository::new();
        let project_id = ProjectId::new();
        let task = Task::new(project_id, "Test task".to_string());

        let result = repo.create(task.clone()).await.unwrap();

        assert_eq!(result.id, task.id);
        assert_eq!(result.title, "Test task");
    }

    #[tokio::test]
    async fn test_get_by_id_returns_task() {
        let repo = MemoryTaskRepository::new();
        let project_id = ProjectId::new();
        let task = Task::new(project_id, "Find me".to_string());
        repo.create(task.clone()).await.unwrap();

        let result = repo.get_by_id(&task.id).await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().title, "Find me");
    }

    #[tokio::test]
    async fn test_get_by_id_returns_none_for_missing() {
        let repo = MemoryTaskRepository::new();
        let task_id = TaskId::new();

        let result = repo.get_by_id(&task_id).await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_by_project_filters_correctly() {
        let repo = MemoryTaskRepository::new();
        let project1 = ProjectId::new();
        let project2 = ProjectId::new();

        repo.create(create_test_task(project1.clone(), "P1 Task 1", 1))
            .await
            .unwrap();
        repo.create(create_test_task(project1.clone(), "P1 Task 2", 2))
            .await
            .unwrap();
        repo.create(create_test_task(project2.clone(), "P2 Task 1", 3))
            .await
            .unwrap();

        let p1_tasks = repo.get_by_project(&project1).await.unwrap();
        let p2_tasks = repo.get_by_project(&project2).await.unwrap();

        assert_eq!(p1_tasks.len(), 2);
        assert_eq!(p2_tasks.len(), 1);
        assert!(p1_tasks.iter().all(|t| t.project_id == project1));
    }

    #[tokio::test]
    async fn test_get_by_project_sorts_by_priority_and_created_at() {
        let repo = MemoryTaskRepository::new();
        let project = ProjectId::new();

        // Create tasks with different priorities
        repo.create(create_test_task(project.clone(), "Low", 1))
            .await
            .unwrap();
        repo.create(create_test_task(project.clone(), "High", 10))
            .await
            .unwrap();
        repo.create(create_test_task(project.clone(), "Medium", 5))
            .await
            .unwrap();

        let tasks = repo.get_by_project(&project).await.unwrap();

        assert_eq!(tasks[0].title, "High");
        assert_eq!(tasks[1].title, "Medium");
        assert_eq!(tasks[2].title, "Low");
    }

    #[tokio::test]
    async fn test_update_modifies_existing_task() {
        let repo = MemoryTaskRepository::new();
        let project_id = ProjectId::new();
        let mut task = Task::new(project_id, "Original".to_string());
        repo.create(task.clone()).await.unwrap();

        task.title = "Updated".to_string();
        task.priority = 100;
        repo.update(&task).await.unwrap();

        let result = repo.get_by_id(&task.id).await.unwrap().unwrap();
        assert_eq!(result.title, "Updated");
        assert_eq!(result.priority, 100);
    }

    #[tokio::test]
    async fn test_delete_removes_task() {
        let repo = MemoryTaskRepository::new();
        let project_id = ProjectId::new();
        let task = Task::new(project_id, "Delete me".to_string());
        repo.create(task.clone()).await.unwrap();

        repo.delete(&task.id).await.unwrap();

        let result = repo.get_by_id(&task.id).await.unwrap();
        assert!(result.is_none());
    }

    // ===== Status Operations Tests =====

    #[tokio::test]
    async fn test_get_by_status_filters_correctly() {
        let repo = MemoryTaskRepository::new();
        let project = ProjectId::new();

        let mut task1 = create_test_task(project.clone(), "Backlog", 1);
        task1.internal_status = InternalStatus::Backlog;

        let mut task2 = create_test_task(project.clone(), "Ready", 2);
        task2.internal_status = InternalStatus::Ready;

        let mut task3 = create_test_task(project.clone(), "Also Ready", 3);
        task3.internal_status = InternalStatus::Ready;

        repo.create(task1).await.unwrap();
        repo.create(task2).await.unwrap();
        repo.create(task3).await.unwrap();

        let ready = repo
            .get_by_status(&project, InternalStatus::Ready)
            .await
            .unwrap();
        let backlog = repo
            .get_by_status(&project, InternalStatus::Backlog)
            .await
            .unwrap();

        assert_eq!(ready.len(), 2);
        assert_eq!(backlog.len(), 1);
    }

    #[tokio::test]
    async fn test_persist_status_change_updates_task() {
        let repo = MemoryTaskRepository::new();
        let project_id = ProjectId::new();
        let task = Task::new(project_id, "Status test".to_string());
        repo.create(task.clone()).await.unwrap();

        repo.persist_status_change(
            &task.id,
            InternalStatus::Backlog,
            InternalStatus::Ready,
            "user",
        )
        .await
        .unwrap();

        let updated = repo.get_by_id(&task.id).await.unwrap().unwrap();
        assert_eq!(updated.internal_status, InternalStatus::Ready);
    }

    #[tokio::test]
    async fn test_persist_status_change_records_history() {
        let repo = MemoryTaskRepository::new();
        let project_id = ProjectId::new();
        let task = Task::new(project_id, "History test".to_string());
        repo.create(task.clone()).await.unwrap();

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

        let history = repo.get_status_history(&task.id).await.unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].from, InternalStatus::Backlog);
        assert_eq!(history[0].to, InternalStatus::Ready);
        assert_eq!(history[0].trigger, "user");
        assert_eq!(history[1].from, InternalStatus::Ready);
        assert_eq!(history[1].to, InternalStatus::Executing);
        assert_eq!(history[1].trigger, "agent");
    }

    #[tokio::test]
    async fn test_get_status_history_empty_for_new_task() {
        let repo = MemoryTaskRepository::new();
        let task_id = TaskId::new();

        let history = repo.get_status_history(&task_id).await.unwrap();
        assert!(history.is_empty());
    }

    // ===== Query Operations Tests =====

    #[tokio::test]
    async fn test_get_next_executable_returns_ready_task() {
        let repo = MemoryTaskRepository::new();
        let project = ProjectId::new();

        let mut task = create_test_task(project.clone(), "Ready", 10);
        task.internal_status = InternalStatus::Ready;
        repo.create(task.clone()).await.unwrap();

        let mut backlog = create_test_task(project.clone(), "Backlog", 100);
        backlog.internal_status = InternalStatus::Backlog;
        repo.create(backlog).await.unwrap();

        let next = repo.get_next_executable(&project).await.unwrap();
        assert!(next.is_some());
        assert_eq!(next.unwrap().title, "Ready");
    }

    #[tokio::test]
    async fn test_get_next_executable_respects_priority() {
        let repo = MemoryTaskRepository::new();
        let project = ProjectId::new();

        let mut low = create_test_task(project.clone(), "Low", 1);
        low.internal_status = InternalStatus::Ready;
        repo.create(low).await.unwrap();

        let mut high = create_test_task(project.clone(), "High", 100);
        high.internal_status = InternalStatus::Ready;
        repo.create(high).await.unwrap();

        let next = repo.get_next_executable(&project).await.unwrap();
        assert_eq!(next.unwrap().title, "High");
    }

    #[tokio::test]
    async fn test_get_next_executable_excludes_blocked() {
        let repo = MemoryTaskRepository::new();
        let project = ProjectId::new();

        let mut task = create_test_task(project.clone(), "Blocked", 100);
        task.internal_status = InternalStatus::Ready;
        repo.create(task.clone()).await.unwrap();

        let blocker = create_test_task(project.clone(), "Blocker", 1);
        repo.create(blocker.clone()).await.unwrap();

        // Block the high-priority task
        repo.add_blocker(&task.id, &blocker.id).await.unwrap();

        let next = repo.get_next_executable(&project).await.unwrap();
        assert!(next.is_none()); // Only blocked Ready task
    }

    #[tokio::test]
    async fn test_get_next_executable_returns_none_when_empty() {
        let repo = MemoryTaskRepository::new();
        let project = ProjectId::new();

        let next = repo.get_next_executable(&project).await.unwrap();
        assert!(next.is_none());
    }

    // ===== Blocker Operations Tests =====

    #[tokio::test]
    async fn test_add_blocker_creates_relationship() {
        let repo = MemoryTaskRepository::new();
        let project = ProjectId::new();

        let task = create_test_task(project.clone(), "Task", 1);
        let blocker = create_test_task(project.clone(), "Blocker", 2);
        repo.create(task.clone()).await.unwrap();
        repo.create(blocker.clone()).await.unwrap();

        repo.add_blocker(&task.id, &blocker.id).await.unwrap();

        let blockers = repo.get_blockers(&task.id).await.unwrap();
        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].title, "Blocker");
    }

    #[tokio::test]
    async fn test_get_blockers_returns_empty_for_unblocked() {
        let repo = MemoryTaskRepository::new();
        let project = ProjectId::new();

        let task = create_test_task(project, "Task", 1);
        repo.create(task.clone()).await.unwrap();

        let blockers = repo.get_blockers(&task.id).await.unwrap();
        assert!(blockers.is_empty());
    }

    #[tokio::test]
    async fn test_get_dependents_returns_blocked_tasks() {
        let repo = MemoryTaskRepository::new();
        let project = ProjectId::new();

        let blocker = create_test_task(project.clone(), "Blocker", 1);
        let dependent1 = create_test_task(project.clone(), "Dependent 1", 2);
        let dependent2 = create_test_task(project.clone(), "Dependent 2", 3);

        repo.create(blocker.clone()).await.unwrap();
        repo.create(dependent1.clone()).await.unwrap();
        repo.create(dependent2.clone()).await.unwrap();

        repo.add_blocker(&dependent1.id, &blocker.id).await.unwrap();
        repo.add_blocker(&dependent2.id, &blocker.id).await.unwrap();

        let dependents = repo.get_dependents(&blocker.id).await.unwrap();
        assert_eq!(dependents.len(), 2);
    }

    #[tokio::test]
    async fn test_resolve_blocker_removes_relationship() {
        let repo = MemoryTaskRepository::new();
        let project = ProjectId::new();

        let task = create_test_task(project.clone(), "Task", 1);
        let blocker = create_test_task(project.clone(), "Blocker", 2);
        repo.create(task.clone()).await.unwrap();
        repo.create(blocker.clone()).await.unwrap();

        repo.add_blocker(&task.id, &blocker.id).await.unwrap();
        repo.resolve_blocker(&task.id, &blocker.id).await.unwrap();

        let blockers = repo.get_blockers(&task.id).await.unwrap();
        assert!(blockers.is_empty());
    }

    #[tokio::test]
    async fn test_delete_removes_blocker_references() {
        let repo = MemoryTaskRepository::new();
        let project = ProjectId::new();

        let task = create_test_task(project.clone(), "Task", 1);
        let blocker = create_test_task(project.clone(), "Blocker", 2);
        repo.create(task.clone()).await.unwrap();
        repo.create(blocker.clone()).await.unwrap();

        repo.add_blocker(&task.id, &blocker.id).await.unwrap();
        repo.delete(&blocker.id).await.unwrap();

        let blockers = repo.get_blockers(&task.id).await.unwrap();
        assert!(blockers.is_empty());
    }

    // ===== with_tasks Constructor Test =====

    #[tokio::test]
    async fn test_with_tasks_prepopulates() {
        let project = ProjectId::new();
        let task1 = create_test_task(project.clone(), "Prepop 1", 1);
        let task2 = create_test_task(project.clone(), "Prepop 2", 2);

        let repo = MemoryTaskRepository::with_tasks(vec![task1.clone(), task2.clone()]);

        let result = repo.get_by_project(&project).await.unwrap();
        assert_eq!(result.len(), 2);
    }
}
