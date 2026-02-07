// Memory-based TaskRepository implementation for testing
// Uses RwLock<HashMap> for thread-safe storage without a real database

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;
use chrono::Utc;

use crate::domain::entities::{IdeationSessionId, InternalStatus, ProjectId, Task, TaskId};
use crate::domain::repositories::{StateHistoryMetadata, StatusTransition, TaskRepository};
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

    async fn get_by_ideation_session(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<Vec<Task>> {
        let tasks = self.tasks.read().await;
        let mut result: Vec<Task> = tasks
            .values()
            .filter(|t| {
                t.ideation_session_id
                    .as_ref()
                    .map(|id| id == session_id)
                    .unwrap_or(false)
            })
            .cloned()
            .collect();
        result.sort_by(|a, b| a.created_at.cmp(&b.created_at));
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

    async fn get_by_project_filtered(
        &self,
        project_id: &ProjectId,
        include_archived: bool,
    ) -> AppResult<Vec<Task>> {
        let tasks = self.tasks.read().await;
        let mut result: Vec<Task> = tasks
            .values()
            .filter(|t| {
                t.project_id == *project_id
                    && (include_archived || t.archived_at.is_none())
            })
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

    async fn archive(&self, task_id: &TaskId) -> AppResult<Task> {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(task_id) {
            task.archived_at = Some(Utc::now());
            task.updated_at = Utc::now();
            Ok(task.clone())
        } else {
            Err(crate::error::AppError::NotFound(format!(
                "Task with id {} not found",
                task_id.as_str()
            )))
        }
    }

    async fn restore(&self, task_id: &TaskId) -> AppResult<Task> {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(task_id) {
            task.archived_at = None;
            task.updated_at = Utc::now();
            Ok(task.clone())
        } else {
            Err(crate::error::AppError::NotFound(format!(
                "Task with id {} not found",
                task_id.as_str()
            )))
        }
    }

    async fn get_archived_count(&self, project_id: &ProjectId) -> AppResult<u32> {
        let tasks = self.tasks.read().await;
        let count = tasks
            .values()
            .filter(|t| t.project_id == *project_id && t.archived_at.is_some())
            .count();
        Ok(count as u32)
    }

    async fn list_paginated(
        &self,
        project_id: &ProjectId,
        statuses: Option<Vec<InternalStatus>>,
        offset: u32,
        limit: u32,
        include_archived: bool,
    ) -> AppResult<Vec<Task>> {
        let tasks = self.tasks.read().await;

        // Filter tasks based on criteria
        let mut result: Vec<Task> = tasks
            .values()
            .filter(|t| {
                // Match project
                if t.project_id != *project_id {
                    return false;
                }

                // Match archived status
                if !include_archived && t.archived_at.is_some() {
                    return false;
                }

                // Match status if provided (any of the statuses)
                if let Some(ref status_vec) = statuses {
                    if !status_vec.contains(&t.internal_status) {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        // Sort by created_at DESC (newest first)
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        // Apply pagination
        let start = offset as usize;
        let paginated = result.into_iter().skip(start).take(limit as usize).collect();

        Ok(paginated)
    }

    async fn count_tasks(
        &self,
        project_id: &ProjectId,
        include_archived: bool,
    ) -> AppResult<u32> {
        let tasks = self.tasks.read().await;
        let count = tasks
            .values()
            .filter(|t| {
                t.project_id == *project_id
                    && (include_archived || t.archived_at.is_none())
            })
            .count();
        Ok(count as u32)
    }

    async fn search(
        &self,
        project_id: &ProjectId,
        query: &str,
        include_archived: bool,
    ) -> AppResult<Vec<Task>> {
        let tasks = self.tasks.read().await;

        // Convert query to lowercase for case-insensitive search
        let query_lower = query.to_lowercase();

        let mut result: Vec<Task> = tasks
            .values()
            .filter(|t| {
                // Match project
                if t.project_id != *project_id {
                    return false;
                }

                // Match archived status
                if !include_archived && t.archived_at.is_some() {
                    return false;
                }

                // Search in title OR description (case-insensitive)
                let title_matches = t.title.to_lowercase().contains(&query_lower);
                let description_matches = t
                    .description
                    .as_ref()
                    .map(|d| d.to_lowercase().contains(&query_lower))
                    .unwrap_or(false);

                title_matches || description_matches
            })
            .cloned()
            .collect();

        // Sort by created_at DESC (newest first)
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(result)
    }

    async fn get_oldest_ready_task(&self) -> AppResult<Option<Task>> {
        let tasks = self.tasks.read().await;

        // Find all Ready tasks that are not archived
        let mut ready_tasks: Vec<&Task> = tasks
            .values()
            .filter(|t| t.internal_status == InternalStatus::Ready && t.archived_at.is_none())
            .collect();

        // Sort by created_at ASC (oldest first) for FIFO
        ready_tasks.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        Ok(ready_tasks.first().cloned().cloned())
    }

    async fn get_oldest_ready_tasks(&self, limit: u32) -> AppResult<Vec<Task>> {
        let tasks = self.tasks.read().await;

        // Find all Ready tasks that are not archived
        let mut ready_tasks: Vec<Task> = tasks
            .values()
            .filter(|t| t.internal_status == InternalStatus::Ready && t.archived_at.is_none())
            .cloned()
            .collect();

        // Sort by created_at ASC (oldest first) for FIFO
        ready_tasks.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        // Apply limit
        ready_tasks.truncate(limit as usize);

        Ok(ready_tasks)
    }

    async fn update_latest_state_history_metadata(
        &self,
        _task_id: &TaskId,
        _metadata: &StateHistoryMetadata,
    ) -> AppResult<()> {
        // In-memory implementation doesn't persist metadata
        Ok(())
    }

    async fn has_task_in_states(
        &self,
        project_id: &ProjectId,
        statuses: &[InternalStatus],
    ) -> AppResult<bool> {
        if statuses.is_empty() {
            return Ok(false);
        }

        let tasks = self.tasks.read().await;
        let has_match = tasks.values().any(|t| {
            t.project_id == *project_id
                && t.archived_at.is_none()
                && statuses.contains(&t.internal_status)
        });

        Ok(has_match)
    }
}

#[cfg(test)]
mod tests;
