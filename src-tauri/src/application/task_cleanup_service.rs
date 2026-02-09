// Service for task cleanup: stop agent → git cleanup → DB delete → event emission
// Consolidates the inline cleanup logic from delete_ideation_session,
// SessionReopenService::reopen, and permanently_delete_task.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::application::git_service::GitService;
use crate::commands::execution_commands::AGENT_ACTIVE_STATUSES;
use crate::domain::entities::project::GitMode;
use crate::domain::entities::{
    IdeationSessionId, InternalStatus, ProjectId, Task, TaskId,
};
use crate::domain::repositories::{ProjectRepository, TaskRepository};
use crate::domain::services::{RunningAgentKey, RunningAgentRegistry};
use crate::error::AppResult;

/// Abstraction for transitioning a task to Stopped status via the state machine.
/// Implemented by TaskTransitionService in production; allows test doubles.
#[async_trait]
pub trait TaskStopper: Send + Sync {
    /// Transition a task to Stopped, triggering on_exit side effects
    /// (decrement running_count, emit events, etc.).
    async fn transition_to_stopped(&self, task_id: &TaskId) -> AppResult<()>;
}

/// Controls how running agents are stopped during cleanup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopMode {
    /// Use TransitionHandler to transition task → Stopped.
    /// Triggers on_exit side effects (decrement running_count, etc.).
    /// Use when cleanup is a deliberate user action (e.g., session deletion).
    Graceful,
    /// Directly stop the agent process via registry.stop() without
    /// transitioning through the state machine.
    /// Use when the task will be deleted immediately after stop (e.g., session reopen),
    /// or when the task may be in a transient state with no valid → Stopped transition.
    DirectStop,
}

/// Identifies a group of tasks for bulk operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum TaskGroup {
    /// All tasks belonging to an ideation session.
    #[serde(rename = "session")]
    Session {
        session_id: String,
        project_id: String,
    },
    /// All tasks with a given status in a project.
    #[serde(rename = "status")]
    Status {
        status: String,
        project_id: String,
    },
    /// All tasks in a project with no ideation_session_id (standalone tasks).
    #[serde(rename = "uncategorized")]
    Uncategorized {
        project_id: String,
    },
}

/// Report of cleanup results for batch operations.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CleanupReport {
    pub tasks_stopped: usize,
    pub tasks_deleted: usize,
    pub git_cleanups: usize,
    pub errors: Vec<String>,
}

impl CleanupReport {
    /// Convenience accessors matching the Tauri command response field names.
    pub fn deleted_count(&self) -> usize {
        self.tasks_deleted
    }
    pub fn failed_count(&self) -> usize {
        self.errors.len()
    }
    pub fn stopped_agents(&self) -> usize {
        self.tasks_stopped
    }
}

pub struct TaskCleanupService {
    task_repo: Arc<dyn TaskRepository>,
    project_repo: Arc<dyn ProjectRepository>,
    running_agent_registry: Arc<dyn RunningAgentRegistry>,
    app_handle: Option<AppHandle>,
    /// Optional task stopper for Graceful mode. When set, Graceful stop will
    /// transition tasks to Stopped via the state machine (triggering on_exit
    /// side effects). When None, Graceful falls back to DirectStop behavior.
    task_stopper: Option<Arc<dyn TaskStopper>>,
}

impl TaskCleanupService {
    pub fn new(
        task_repo: Arc<dyn TaskRepository>,
        project_repo: Arc<dyn ProjectRepository>,
        running_agent_registry: Arc<dyn RunningAgentRegistry>,
        app_handle: Option<AppHandle>,
    ) -> Self {
        Self {
            task_repo,
            project_repo,
            running_agent_registry,
            app_handle,
            task_stopper: None,
        }
    }

    /// Set the task stopper for Graceful mode (builder pattern).
    /// Required when using `StopMode::Graceful` to properly transition tasks
    /// through the state machine.
    pub fn with_task_stopper(mut self, stopper: Arc<dyn TaskStopper>) -> Self {
        self.task_stopper = Some(stopper);
        self
    }

    /// Clean up a single task: stop agent → git cleanup → DB delete → optional event.
    ///
    /// This is the core per-task cleanup unit. Callers control:
    /// - `stop_mode`: How to stop running agents (Graceful vs DirectStop)
    /// - `emit_events`: Whether to emit `task:deleted` events for real-time UI updates
    pub async fn cleanup_single_task(
        &self,
        task: &Task,
        stop_mode: StopMode,
        emit_events: bool,
    ) -> AppResult<()> {
        let project_id_str = task.project_id.as_str().to_string();

        // 1. Stop running agent if task is in an active state
        if AGENT_ACTIVE_STATUSES.contains(&task.internal_status) {
            self.stop_task_agent(task, stop_mode).await;
        }

        // 2. Clean up git resources (worktree + branch)
        self.cleanup_git_resources(task).await;

        // 3. Delete task from DB
        if let Err(e) = self.task_repo.delete(&task.id).await {
            tracing::warn!(
                task_id = task.id.as_str(),
                error = %e,
                "Failed to delete task during cleanup"
            );
            return Err(e);
        }

        // 4. Emit event for real-time UI updates
        if emit_events {
            self.emit_task_deleted(task.id.as_str(), &project_id_str);
        }

        Ok(())
    }

    /// Clean delete a single task by reference (convenience wrapper).
    /// Uses Graceful stop mode, no event emission. Returns whether an agent was stopped.
    pub async fn cleanup_task_ref(&self, task: &Task) -> AppResult<bool> {
        let was_active = AGENT_ACTIVE_STATUSES.contains(&task.internal_status);
        self.cleanup_single_task(task, StopMode::Graceful, false).await?;
        Ok(was_active)
    }

    /// Clean up multiple tasks in batch.
    pub async fn cleanup_tasks(
        &self,
        tasks: &[Task],
        stop_mode: StopMode,
        emit_events: bool,
    ) -> CleanupReport {
        let mut report = CleanupReport::default();

        // If any tasks are active, stop them first (batch all stops before deletes)
        for task in tasks {
            if AGENT_ACTIVE_STATUSES.contains(&task.internal_status) {
                self.stop_task_agent(task, stop_mode).await;
                report.tasks_stopped += 1;
            }
        }

        // Git cleanup for all tasks
        for task in tasks {
            if task.task_branch.is_some() || task.worktree_path.is_some() {
                self.cleanup_git_resources(task).await;
                report.git_cleanups += 1;
            }
        }

        // Delete tasks from DB and emit events
        for task in tasks {
            let project_id_str = task.project_id.as_str().to_string();
            if let Err(e) = self.task_repo.delete(&task.id).await {
                tracing::warn!(
                    task_id = task.id.as_str(),
                    error = %e,
                    "Failed to delete task during batch cleanup"
                );
                report
                    .errors
                    .push(format!("Delete {}: {}", task.id.as_str(), e));
            } else {
                report.tasks_deleted += 1;
                if emit_events {
                    self.emit_task_deleted(task.id.as_str(), &project_id_str);
                }
            }
        }

        report
    }

    /// Clean up all tasks in a group. Uses Graceful stop mode and emits events.
    /// Skips plan_merge tasks (system-managed).
    pub async fn cleanup_tasks_in_group(&self, group: TaskGroup) -> AppResult<CleanupReport> {
        let tasks = self.resolve_group_tasks(&group).await?;
        // Filter out plan_merge tasks (system-managed)
        let filtered: Vec<Task> = tasks
            .into_iter()
            .filter(|t| t.category != "plan_merge")
            .collect();
        Ok(self.cleanup_tasks(&filtered, StopMode::Graceful, true).await)
    }

    // ── Private helpers ──────────────────────────────────────────────────

    /// Stop a running agent for a task.
    ///
    /// - `Graceful`: stop agent process, then transition to Stopped via state machine
    ///   (triggers on_exit side effects like decrement running_count).
    /// - `DirectStop`: stop agent process only, bypass state machine.
    async fn stop_task_agent(&self, task: &Task, stop_mode: StopMode) {
        // Step 1: Always stop the agent process
        let context_type = match task.internal_status {
            InternalStatus::Reviewing => "review",
            InternalStatus::Merging => "merge",
            _ => "task_execution",
        };
        let key = RunningAgentKey::new(context_type, task.id.as_str());
        let _ = self.running_agent_registry.stop(&key).await;

        // Step 2: For Graceful mode, also transition to Stopped via state machine
        if stop_mode == StopMode::Graceful {
            if let Some(ref stopper) = self.task_stopper {
                if let Err(e) = stopper.transition_to_stopped(&task.id).await {
                    tracing::warn!(
                        task_id = task.id.as_str(),
                        error = %e,
                        "Failed to transition task to Stopped during cleanup (non-fatal)"
                    );
                }
            }
        }
    }

    /// Clean up git resources (worktree + branch) for a task.
    /// Best-effort — errors are logged but not propagated.
    ///
    /// Safety: checks out base branch before deleting task branch to avoid
    /// "cannot delete the branch you are currently on" errors in Local mode.
    async fn cleanup_git_resources(&self, task: &Task) {
        let project = match self.project_repo.get_by_id(&task.project_id).await {
            Ok(Some(p)) => p,
            _ => return,
        };

        let repo_path = PathBuf::from(&project.working_directory);
        let base_branch = project.base_branch.as_deref().unwrap_or("main");
        let task_branch = match &task.task_branch {
            Some(branch) => branch.clone(),
            None => return,
        };

        match project.git_mode {
            GitMode::Worktree => {
                // Delete worktree first if it exists
                if let Some(ref worktree_path) = task.worktree_path {
                    let worktree_path_buf = PathBuf::from(worktree_path);
                    if let Err(e) = GitService::delete_worktree(&repo_path, &worktree_path_buf) {
                        tracing::warn!(
                            worktree = worktree_path.as_str(),
                            error = %e,
                            "Failed to delete worktree during cleanup (non-fatal)"
                        );
                    }
                }

                // Checkout base branch before deleting task branch
                if let Err(e) = GitService::checkout_branch(&repo_path, base_branch) {
                    tracing::warn!(
                        base_branch = base_branch,
                        error = %e,
                        "Failed to checkout base branch during cleanup (non-fatal)"
                    );
                }

                // Delete task branch
                if let Err(e) = GitService::delete_branch(&repo_path, &task_branch, true) {
                    tracing::warn!(
                        branch = task_branch.as_str(),
                        error = %e,
                        "Failed to delete branch during cleanup (non-fatal)"
                    );
                }
            }
            GitMode::Local => {
                // Abort any in-progress rebase (safety for Local mode)
                if GitService::is_rebase_in_progress(&repo_path) {
                    let _ = GitService::abort_rebase(&repo_path);
                }

                // Checkout base branch before deleting task branch
                // (avoids "cannot delete the branch you are currently on")
                if let Err(e) = GitService::checkout_branch(&repo_path, base_branch) {
                    tracing::warn!(
                        base_branch = base_branch,
                        error = %e,
                        "Failed to checkout base branch during cleanup (non-fatal)"
                    );
                }

                // Delete task branch
                if let Err(e) = GitService::delete_branch(&repo_path, &task_branch, true) {
                    tracing::warn!(
                        branch = task_branch.as_str(),
                        error = %e,
                        "Failed to delete branch during cleanup (non-fatal)"
                    );
                }
            }
        }
    }

    /// Resolve a TaskGroup to the actual tasks.
    async fn resolve_group_tasks(&self, group: &TaskGroup) -> AppResult<Vec<Task>> {
        match group {
            TaskGroup::Session { session_id, .. } => {
                let session_id = IdeationSessionId::from_string(session_id.clone());
                self.task_repo.get_by_ideation_session(&session_id).await
            }
            TaskGroup::Status { status, project_id } => {
                let project_id = ProjectId::from_string(project_id.clone());
                let internal_status: InternalStatus = status
                    .parse()
                    .map_err(|_| crate::error::AppError::Validation(
                        format!("Invalid status: {}", status)
                    ))?;
                self.task_repo.get_by_status(&project_id, internal_status).await
            }
            TaskGroup::Uncategorized { project_id } => {
                let project_id = ProjectId::from_string(project_id.clone());
                let all_tasks = self.task_repo.get_by_project(&project_id).await?;
                Ok(all_tasks
                    .into_iter()
                    .filter(|t| t.ideation_session_id.is_none())
                    .collect())
            }
        }
    }

    /// Emit a task:deleted event for real-time UI updates.
    fn emit_task_deleted(&self, task_id: &str, project_id: &str) {
        if let Some(ref app) = self.app_handle {
            use tauri::Emitter;
            let _ = app.emit(
                "task:deleted",
                serde_json::json!({
                    "taskId": task_id,
                    "projectId": project_id,
                }),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::AppState;
    use crate::domain::entities::{Project, ProjectId, Task};

    #[tokio::test]
    async fn test_cleanup_single_task_deletes_from_db() {
        let state = AppState::new_test();
        let project_id = ProjectId::new();

        // Create project
        let project = Project::new("Test".to_string(), "/tmp/test".to_string());
        state
            .project_repo
            .create(Project {
                id: project_id.clone(),
                ..project
            })
            .await
            .unwrap();

        // Create task
        let task = Task::new(project_id.clone(), "Test Task".to_string());
        let task_id = task.id.clone();
        let created = state.task_repo.create(task).await.unwrap();

        let service = TaskCleanupService::new(
            Arc::clone(&state.task_repo),
            Arc::clone(&state.project_repo),
            Arc::clone(&state.running_agent_registry),
            None,
        );

        service
            .cleanup_single_task(&created, StopMode::DirectStop, false)
            .await
            .unwrap();

        // Verify task is deleted
        assert!(state
            .task_repo
            .get_by_id(&task_id)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn test_cleanup_task_ref_deletes_from_db() {
        let state = AppState::new_test();
        let project_id = ProjectId::new();

        let task = Task::new(project_id, "Test Task".to_string());
        let created = state.task_repo.create(task).await.unwrap();

        let service = TaskCleanupService::new(
            Arc::clone(&state.task_repo),
            Arc::clone(&state.project_repo),
            Arc::clone(&state.running_agent_registry),
            None,
        );
        let agent_stopped = service.cleanup_task_ref(&created).await.unwrap();
        assert!(!agent_stopped); // backlog task has no active agent

        assert!(state.task_repo.get_by_id(&created.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_cleanup_tasks_batch() {
        let state = AppState::new_test();
        let project_id = ProjectId::new();

        let project = Project::new("Test".to_string(), "/tmp/test".to_string());
        state
            .project_repo
            .create(Project {
                id: project_id.clone(),
                ..project
            })
            .await
            .unwrap();

        // Create multiple tasks
        let task1 = state
            .task_repo
            .create(Task::new(project_id.clone(), "Task 1".to_string()))
            .await
            .unwrap();
        let task2 = state
            .task_repo
            .create(Task::new(project_id.clone(), "Task 2".to_string()))
            .await
            .unwrap();
        let task3 = state
            .task_repo
            .create(Task::new(project_id.clone(), "Task 3".to_string()))
            .await
            .unwrap();

        let service = TaskCleanupService::new(
            Arc::clone(&state.task_repo),
            Arc::clone(&state.project_repo),
            Arc::clone(&state.running_agent_registry),
            None,
        );

        let report = service
            .cleanup_tasks(&[task1, task2, task3], StopMode::DirectStop, false)
            .await;

        assert_eq!(report.tasks_deleted, 3);
        assert!(report.errors.is_empty());

        // Verify all tasks are deleted
        let remaining = state.task_repo.get_by_project(&project_id).await.unwrap();
        assert!(remaining.is_empty());
    }

    #[tokio::test]
    async fn test_cleanup_tasks_in_group_by_session() {
        let state = AppState::new_test();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();

        let project = Project::new("Test".to_string(), "/tmp/test".to_string());
        state
            .project_repo
            .create(Project {
                id: project_id.clone(),
                ..project
            })
            .await
            .unwrap();

        // Create tasks linked to session
        let mut task1 = Task::new(project_id.clone(), "Session Task 1".to_string());
        task1.ideation_session_id = Some(session_id.clone());
        state.task_repo.create(task1).await.unwrap();

        let mut task2 = Task::new(project_id.clone(), "Session Task 2".to_string());
        task2.ideation_session_id = Some(session_id.clone());
        state.task_repo.create(task2).await.unwrap();

        // Create standalone task (should NOT be deleted)
        let standalone = state
            .task_repo
            .create(Task::new(project_id.clone(), "Standalone".to_string()))
            .await
            .unwrap();

        let service = TaskCleanupService::new(
            Arc::clone(&state.task_repo),
            Arc::clone(&state.project_repo),
            Arc::clone(&state.running_agent_registry),
            None,
        );

        let report = service
            .cleanup_tasks_in_group(TaskGroup::Session {
                session_id: session_id.as_str().to_string(),
                project_id: project_id.as_str().to_string(),
            })
            .await
            .unwrap();

        assert_eq!(report.tasks_deleted, 2);

        // Standalone task should still exist
        assert!(state
            .task_repo
            .get_by_id(&standalone.id)
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn test_cleanup_tasks_in_group_by_status() {
        let state = AppState::new_test();
        let project_id = ProjectId::new();

        let project = Project::new("Test".to_string(), "/tmp/test".to_string());
        state
            .project_repo
            .create(Project {
                id: project_id.clone(),
                ..project
            })
            .await
            .unwrap();

        // Create two backlog tasks
        let task1 = Task::new(project_id.clone(), "Task 1".to_string());
        let created1 = state.task_repo.create(task1).await.unwrap();

        let task2 = Task::new(project_id.clone(), "Task 2".to_string());
        let created2 = state.task_repo.create(task2).await.unwrap();

        let service = TaskCleanupService::new(
            Arc::clone(&state.task_repo),
            Arc::clone(&state.project_repo),
            Arc::clone(&state.running_agent_registry),
            None,
        );

        let group = TaskGroup::Status {
            status: "backlog".to_string(),
            project_id: project_id.as_str().to_string(),
        };
        let report = service.cleanup_tasks_in_group(group).await.unwrap();

        assert_eq!(report.tasks_deleted, 2);
        assert!(state.task_repo.get_by_id(&created1.id).await.unwrap().is_none());
        assert!(state.task_repo.get_by_id(&created2.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_cleanup_tasks_in_group_uncategorized() {
        let state = AppState::new_test();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();

        let project = Project::new("Test".to_string(), "/tmp/test".to_string());
        state
            .project_repo
            .create(Project {
                id: project_id.clone(),
                ..project
            })
            .await
            .unwrap();

        // Create session task (should NOT be deleted)
        let mut session_task = Task::new(project_id.clone(), "Session Task".to_string());
        session_task.ideation_session_id = Some(session_id.clone());
        let session_created = state.task_repo.create(session_task).await.unwrap();

        // Create uncategorized tasks (should be deleted)
        state
            .task_repo
            .create(Task::new(project_id.clone(), "Uncat 1".to_string()))
            .await
            .unwrap();
        state
            .task_repo
            .create(Task::new(project_id.clone(), "Uncat 2".to_string()))
            .await
            .unwrap();

        let service = TaskCleanupService::new(
            Arc::clone(&state.task_repo),
            Arc::clone(&state.project_repo),
            Arc::clone(&state.running_agent_registry),
            None,
        );

        let report = service
            .cleanup_tasks_in_group(TaskGroup::Uncategorized {
                project_id: project_id.as_str().to_string(),
            })
            .await
            .unwrap();

        assert_eq!(report.tasks_deleted, 2);

        // Session task should still exist
        assert!(state
            .task_repo
            .get_by_id(&session_created.id)
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn test_cleanup_skips_plan_merge_tasks() {
        let state = AppState::new_test();
        let project_id = ProjectId::new();

        let project = Project::new("Test".to_string(), "/tmp/test".to_string());
        state
            .project_repo
            .create(Project {
                id: project_id.clone(),
                ..project
            })
            .await
            .unwrap();

        let task = Task::new_with_category(
            project_id.clone(),
            "Merge Plan".to_string(),
            "plan_merge".to_string(),
        );
        let created = state.task_repo.create(task).await.unwrap();

        let service = TaskCleanupService::new(
            Arc::clone(&state.task_repo),
            Arc::clone(&state.project_repo),
            Arc::clone(&state.running_agent_registry),
            None,
        );

        let group = TaskGroup::Status {
            status: "backlog".to_string(),
            project_id: project_id.as_str().to_string(),
        };
        let report = service.cleanup_tasks_in_group(group).await.unwrap();

        assert_eq!(report.tasks_deleted, 0);
        // plan_merge task should still exist
        assert!(state.task_repo.get_by_id(&created.id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_stop_mode_enum_equality() {
        assert_eq!(StopMode::Graceful, StopMode::Graceful);
        assert_eq!(StopMode::DirectStop, StopMode::DirectStop);
        assert_ne!(StopMode::Graceful, StopMode::DirectStop);
    }

    #[tokio::test]
    async fn test_cleanup_empty_batch() {
        let state = AppState::new_test();

        let service = TaskCleanupService::new(
            Arc::clone(&state.task_repo),
            Arc::clone(&state.project_repo),
            Arc::clone(&state.running_agent_registry),
            None,
        );

        let report = service
            .cleanup_tasks(&[], StopMode::DirectStop, false)
            .await;

        assert_eq!(report.tasks_deleted, 0);
        assert_eq!(report.tasks_stopped, 0);
        assert!(report.errors.is_empty());
    }

    #[tokio::test]
    async fn test_cleanup_empty_group() {
        let state = AppState::new_test();
        let project_id = ProjectId::new();

        let service = TaskCleanupService::new(
            Arc::clone(&state.task_repo),
            Arc::clone(&state.project_repo),
            Arc::clone(&state.running_agent_registry),
            None,
        );

        let group = TaskGroup::Status {
            status: "backlog".to_string(),
            project_id: project_id.as_str().to_string(),
        };
        let report = service.cleanup_tasks_in_group(group).await.unwrap();

        assert_eq!(report.tasks_deleted, 0);
        assert!(report.errors.is_empty());
    }
}
