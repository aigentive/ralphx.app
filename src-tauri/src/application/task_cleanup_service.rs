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
use crate::domain::entities::{
    IdeationSessionId, InternalStatus, ProjectId, Task, TaskCategory, TaskId,
};
use crate::domain::repositories::{ProjectRepository, TaskRepository};
use crate::application::interactive_process_registry::{
    InteractiveProcessKey, InteractiveProcessRegistry,
};
use crate::domain::services::{RunningAgentKey, RunningAgentRegistry};
use crate::error::AppResult;

/// Abstraction for transitioning a task to Stopped status via the state machine.
/// Implemented by TaskTransitionService in production; allows test doubles.
#[async_trait]
pub trait TaskStopper: Send + Sync {
    /// Transition a task to Stopped, triggering on_exit side effects
    /// (decrement running_count, emit events, etc.).
    async fn transition_to_stopped(&self, task_id: &TaskId) -> AppResult<()>;

    /// Transition a task to Stopped with context capture for smart resume.
    ///
    /// This method captures the from_status and optional reason in metadata,
    /// enabling the "smart resume" feature to restore context when restarted.
    ///
    /// # Arguments
    /// * `task_id` - The task to stop
    /// * `from_status` - The status the task was in when stopped
    /// * `reason` - Optional reason for stopping
    async fn transition_to_stopped_with_context(
        &self,
        task_id: &TaskId,
        from_status: InternalStatus,
        reason: Option<String>,
    ) -> AppResult<()>;
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
    Status { status: String, project_id: String },
    /// All tasks in a project with no ideation_session_id (standalone tasks).
    #[serde(rename = "uncategorized")]
    Uncategorized { project_id: String },
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
    interactive_process_registry: Option<Arc<InteractiveProcessRegistry>>,
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
            interactive_process_registry: None,
            app_handle,
            task_stopper: None,
        }
    }

    /// Set the interactive process registry for IPR cleanup on stop (builder pattern).
    pub fn with_interactive_process_registry(mut self, ipr: Arc<InteractiveProcessRegistry>) -> Self {
        self.interactive_process_registry = Some(ipr);
        self
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
        if let Some(ref branch) = task.task_branch {
            tracing::info!(
                task_id = task.id.as_str(),
                branch = branch.as_str(),
                "Cleaning up git resources for task"
            );
        }
        self.cleanup_git_resources(task).await;

        // 3. Clear FK references to task before deletion (defense-in-depth)
        if let Err(e) = self.task_repo.clear_task_references(&task.id).await {
            tracing::warn!(
                task_id = task.id.as_str(),
                error = %e,
                "Failed to clear task references during cleanup"
            );
            return Err(e);
        }

        // 4. Delete task from DB
        if let Err(e) = self.task_repo.delete(&task.id).await {
            tracing::warn!(
                task_id = task.id.as_str(),
                error = %e,
                "Failed to delete task during cleanup"
            );
            return Err(e);
        }

        // 5. Emit event for real-time UI updates
        if emit_events {
            self.emit_task_deleted(task.id.as_str(), &project_id_str);
        }

        Ok(())
    }

    /// Clean delete a single task by reference (convenience wrapper).
    /// Uses Graceful stop mode, no event emission. Returns whether an agent was stopped.
    pub async fn cleanup_task_ref(&self, task: &Task) -> AppResult<bool> {
        let was_active = AGENT_ACTIVE_STATUSES.contains(&task.internal_status);
        self.cleanup_single_task(task, StopMode::Graceful, false)
            .await?;
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
            // Clear FK references before deletion (defense-in-depth)
            if let Err(e) = self.task_repo.clear_task_references(&task.id).await {
                tracing::warn!(
                    task_id = task.id.as_str(),
                    error = %e,
                    "Failed to clear task references during batch cleanup"
                );
                report
                    .errors
                    .push(format!("Clear references {}: {}", task.id.as_str(), e));
                continue;
            }
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

        // Post-delete verification: wait 200ms then check if tasks reappeared
        // (due to concurrent merge side effects writing them back).
        // This is a safety net for the race condition.
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        for task in tasks {
            if let Ok(Some(_)) = self.task_repo.get_by_id(&task.id).await {
                // Task reappeared, retry the delete
                tracing::warn!(
                    task_id = task.id.as_str(),
                    "Task reappeared after delete, retrying cleanup"
                );
                let project_id_str = task.project_id.as_str().to_string();
                // Clear FK references before retry delete (defense-in-depth)
                if let Err(e) = self.task_repo.clear_task_references(&task.id).await {
                    tracing::warn!(
                        task_id = task.id.as_str(),
                        error = %e,
                        "Failed to clear task references during post-delete verification"
                    );
                    report.errors.push(format!(
                        "Clear references (retry) {}: {}",
                        task.id.as_str(),
                        e
                    ));
                    continue;
                }
                if let Err(e) = self.task_repo.delete(&task.id).await {
                    tracing::warn!(
                        task_id = task.id.as_str(),
                        error = %e,
                        "Failed to delete task during post-delete verification"
                    );
                    report
                        .errors
                        .push(format!("Delete (retry) {}: {}", task.id.as_str(), e));
                } else {
                    report.tasks_deleted += 1;
                    if emit_events {
                        self.emit_task_deleted(task.id.as_str(), &project_id_str);
                    }
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
            .filter(|t| t.category != TaskCategory::PlanMerge)
            .collect();
        Ok(self
            .cleanup_tasks(&filtered, StopMode::Graceful, true)
            .await)
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

        // Remove from interactive process registry first — closes stdin pipe
        // so the process doesn't linger waiting for input after SIGTERM.
        if let Some(ref ipr) = self.interactive_process_registry {
            let ipr_key = InteractiveProcessKey::new(context_type, task.id.as_str());
            ipr.remove(&ipr_key).await;
        }

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

        // Delete worktree first if it exists
        if let Some(ref worktree_path) = task.worktree_path {
            let worktree_path_buf = PathBuf::from(worktree_path);
            if let Err(e) = GitService::delete_worktree(&repo_path, &worktree_path_buf).await {
                tracing::warn!(
                    worktree = worktree_path.as_str(),
                    error = %e,
                    "Failed to delete worktree during cleanup (non-fatal)"
                );
            }
        }

        // Only checkout base branch if the task branch is currently checked out in main repo.
        // In Worktree mode the task branch lives in a worktree, not the main checkout,
        // so this is normally a no-op. Guards against edge cases from old Local mode.
        let current_branch = GitService::get_current_branch(&repo_path)
            .await
            .unwrap_or_default();
        if current_branch == task_branch {
            if let Err(e) = GitService::checkout_branch(&repo_path, base_branch).await {
                tracing::warn!(
                    base_branch = base_branch,
                    error = %e,
                    "Failed to checkout base branch during cleanup (non-fatal)"
                );
            }
        }

        // Delete task branch
        if let Err(e) = GitService::delete_branch(&repo_path, &task_branch, true).await {
            tracing::warn!(
                branch = task_branch.as_str(),
                error = %e,
                "Failed to delete branch during cleanup (non-fatal)"
            );
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
                let internal_status: InternalStatus = status.parse().map_err(|_| {
                    crate::error::AppError::Validation(format!("Invalid status: {}", status))
                })?;
                self.task_repo
                    .get_by_status(&project_id, internal_status)
                    .await
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
#[path = "task_cleanup_service_tests.rs"]
mod tests;
