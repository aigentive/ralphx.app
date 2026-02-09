// Service for cleaning up tasks: force-stop agents, delete from DB, cleanup git branches/worktrees
// Extracted from delete_ideation_session and session_reopen_service patterns

use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::commands::execution_commands::AGENT_ACTIVE_STATUSES;
use crate::domain::entities::{
    IdeationSessionId, InternalStatus, ProjectId, Task, TaskId,
};
use crate::domain::repositories::{ProjectRepository, TaskRepository};
use crate::domain::services::{RunningAgentKey, RunningAgentRegistry};
use crate::application::git_service::GitService;
use crate::error::AppResult;

/// Identifies a group of tasks for bulk cleanup
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum TaskGroup {
    /// All tasks with a given internal status in a project
    #[serde(rename = "status")]
    Status {
        status: String,
        project_id: String,
    },
    /// All tasks belonging to a specific ideation session
    #[serde(rename = "session")]
    Session {
        session_id: String,
        project_id: String,
    },
    /// All tasks in a project that have no ideation_session_id
    #[serde(rename = "uncategorized")]
    Uncategorized {
        project_id: String,
    },
}

/// Report of cleanup results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupReport {
    pub deleted_count: usize,
    pub failed_count: usize,
    pub stopped_agents: usize,
}

pub struct TaskCleanupService {
    task_repo: Arc<dyn TaskRepository>,
    project_repo: Arc<dyn ProjectRepository>,
    running_agent_registry: Arc<dyn RunningAgentRegistry>,
}

impl TaskCleanupService {
    pub fn new(
        task_repo: Arc<dyn TaskRepository>,
        project_repo: Arc<dyn ProjectRepository>,
        running_agent_registry: Arc<dyn RunningAgentRegistry>,
    ) -> Self {
        Self {
            task_repo,
            project_repo,
            running_agent_registry,
        }
    }

    /// Clean delete a single task: force-stop agent if active, cleanup branch/worktree, delete from DB
    pub async fn cleanup_task(&self, task_id: &TaskId) -> AppResult<()> {
        let task = self
            .task_repo
            .get_by_id(task_id)
            .await?
            .ok_or_else(|| crate::error::AppError::NotFound(
                format!("Task not found: {}", task_id.as_str())
            ))?;

        self.cleanup_single_task(&task).await
    }

    /// Clean delete all tasks matching a group
    pub async fn cleanup_tasks_in_group(&self, group: TaskGroup) -> AppResult<CleanupReport> {
        let tasks = self.resolve_group_tasks(&group).await?;
        let mut report = CleanupReport {
            deleted_count: 0,
            failed_count: 0,
            stopped_agents: 0,
        };

        for task in &tasks {
            // Skip plan_merge tasks (system-managed)
            if task.category == "plan_merge" {
                continue;
            }

            match self.cleanup_single_task(task).await {
                Ok(()) => {
                    report.deleted_count += 1;
                    if AGENT_ACTIVE_STATUSES.contains(&task.internal_status) {
                        report.stopped_agents += 1;
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        task_id = task.id.as_str(),
                        error = %e,
                        "Failed to cleanup task in group"
                    );
                    report.failed_count += 1;
                }
            }
        }

        Ok(report)
    }

    /// Internal: clean up a single task (stop agent, cleanup git, delete from DB)
    async fn cleanup_single_task(&self, task: &Task) -> AppResult<()> {
        // 1. Force-stop agent if active
        if AGENT_ACTIVE_STATUSES.contains(&task.internal_status) {
            let context_type = match task.internal_status {
                InternalStatus::Reviewing => "review",
                InternalStatus::Merging => "merge",
                _ => "task_execution",
            };
            let key = RunningAgentKey::new(context_type, task.id.as_str());
            let _ = self.running_agent_registry.stop(&key).await;
        }

        // 2. Cleanup git branch/worktree (best-effort)
        if task.task_branch.is_some() || task.worktree_path.is_some() {
            let project = self
                .project_repo
                .get_by_id(&task.project_id)
                .await
                .ok()
                .flatten();

            if let Some(project) = project {
                let repo_path = PathBuf::from(&project.working_directory);

                // Delete worktree first (unlocks branch)
                if let Some(ref worktree_path) = task.worktree_path {
                    let _ = GitService::delete_worktree(&repo_path, &PathBuf::from(worktree_path));
                }

                // Delete task branch
                if let Some(ref branch) = task.task_branch {
                    let _ = GitService::delete_branch(&repo_path, branch, true);
                }
            }
        }

        // 3. Delete task from DB
        self.task_repo.delete(&task.id).await
    }

    /// Resolve a TaskGroup into a list of tasks
    async fn resolve_group_tasks(&self, group: &TaskGroup) -> AppResult<Vec<Task>> {
        match group {
            TaskGroup::Status { status, project_id } => {
                let project_id = ProjectId::from_string(project_id.clone());
                let internal_status: InternalStatus = status
                    .parse()
                    .map_err(|_| crate::error::AppError::Validation(
                        format!("Invalid status: {}", status)
                    ))?;
                self.task_repo.get_by_status(&project_id, internal_status).await
            }
            TaskGroup::Session { session_id, .. } => {
                let session_id = IdeationSessionId::from_string(session_id.clone());
                self.task_repo.get_by_ideation_session(&session_id).await
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::AppState;
    use crate::domain::entities::{ProjectId, Task};

    #[tokio::test]
    async fn test_cleanup_task_not_found() {
        let state = AppState::new_test();
        let service = TaskCleanupService::new(
            Arc::clone(&state.task_repo),
            Arc::clone(&state.project_repo),
            Arc::clone(&state.running_agent_registry),
        );

        let result = service.cleanup_task(&TaskId::new()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cleanup_task_deletes_from_db() {
        let state = AppState::new_test();
        let project_id = ProjectId::new();

        let task = Task::new(project_id, "Test Task".to_string());
        let created = state.task_repo.create(task).await.unwrap();

        let service = TaskCleanupService::new(
            Arc::clone(&state.task_repo),
            Arc::clone(&state.project_repo),
            Arc::clone(&state.running_agent_registry),
        );
        service.cleanup_task(&created.id).await.unwrap();

        assert!(state.task_repo.get_by_id(&created.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_cleanup_tasks_in_group_by_status() {
        let state = AppState::new_test();
        let project_id = ProjectId::new();

        // Create two backlog tasks
        let task1 = Task::new(project_id.clone(), "Task 1".to_string());
        let created1 = state.task_repo.create(task1).await.unwrap();

        let task2 = Task::new(project_id.clone(), "Task 2".to_string());
        let created2 = state.task_repo.create(task2).await.unwrap();

        let service = TaskCleanupService::new(
            Arc::clone(&state.task_repo),
            Arc::clone(&state.project_repo),
            Arc::clone(&state.running_agent_registry),
        );

        let group = TaskGroup::Status {
            status: "backlog".to_string(),
            project_id: project_id.as_str().to_string(),
        };
        let report = service.cleanup_tasks_in_group(group).await.unwrap();

        assert_eq!(report.deleted_count, 2);
        assert_eq!(report.failed_count, 0);
        assert!(state.task_repo.get_by_id(&created1.id).await.unwrap().is_none());
        assert!(state.task_repo.get_by_id(&created2.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_cleanup_skips_plan_merge_tasks() {
        let state = AppState::new_test();
        let project_id = ProjectId::new();

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
        );

        let group = TaskGroup::Status {
            status: "backlog".to_string(),
            project_id: project_id.as_str().to_string(),
        };
        let report = service.cleanup_tasks_in_group(group).await.unwrap();

        assert_eq!(report.deleted_count, 0);
        // plan_merge task should still exist
        assert!(state.task_repo.get_by_id(&created.id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_cleanup_empty_group() {
        let state = AppState::new_test();
        let project_id = ProjectId::new();

        let service = TaskCleanupService::new(
            Arc::clone(&state.task_repo),
            Arc::clone(&state.project_repo),
            Arc::clone(&state.running_agent_registry),
        );

        let group = TaskGroup::Status {
            status: "backlog".to_string(),
            project_id: project_id.as_str().to_string(),
        };
        let report = service.cleanup_tasks_in_group(group).await.unwrap();

        assert_eq!(report.deleted_count, 0);
        assert_eq!(report.failed_count, 0);
    }
}
