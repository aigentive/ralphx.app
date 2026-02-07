// Service for reopening accepted/archived ideation sessions
// Handles cleanup: stop agents, delete tasks, clean git resources, clear proposals, reset session

use std::path::PathBuf;
use std::sync::Arc;

use crate::commands::execution_commands::AGENT_ACTIVE_STATUSES;
use crate::domain::entities::plan_branch::PlanBranchStatus;
use crate::domain::entities::{IdeationSessionId, IdeationSessionStatus, InternalStatus};
use crate::domain::repositories::{
    IdeationSessionRepository, PlanBranchRepository, ProjectRepository, TaskProposalRepository,
    TaskRepository,
};
use crate::domain::services::{RunningAgentKey, RunningAgentRegistry};
use crate::application::git_service::GitService;
use crate::error::AppResult;

pub struct SessionReopenService {
    task_repo: Arc<dyn TaskRepository>,
    task_proposal_repo: Arc<dyn TaskProposalRepository>,
    ideation_session_repo: Arc<dyn IdeationSessionRepository>,
    plan_branch_repo: Arc<dyn PlanBranchRepository>,
    project_repo: Arc<dyn ProjectRepository>,
    running_agent_registry: Arc<RunningAgentRegistry>,
}

impl SessionReopenService {
    pub fn new(
        task_repo: Arc<dyn TaskRepository>,
        task_proposal_repo: Arc<dyn TaskProposalRepository>,
        ideation_session_repo: Arc<dyn IdeationSessionRepository>,
        plan_branch_repo: Arc<dyn PlanBranchRepository>,
        project_repo: Arc<dyn ProjectRepository>,
        running_agent_registry: Arc<RunningAgentRegistry>,
    ) -> Self {
        Self {
            task_repo,
            task_proposal_repo,
            ideation_session_repo,
            plan_branch_repo,
            project_repo,
            running_agent_registry,
        }
    }

    /// Reopen an accepted/archived session back to Active.
    ///
    /// Cleanup order:
    /// 1. Validate session is Accepted or Archived
    /// 2. Get all tasks for this session
    /// 3. Stop running agents (bypass TransitionHandler — transient states have no valid → Stopped)
    /// 4. Abort any in-progress rebase (Local mode safety)
    /// 5. Checkout base branch (Local mode — avoid deleting current branch)
    /// 6. Delete worktrees, task branches, and tasks from DB
    /// 7. Clean plan branch (delete git branch, mark Abandoned)
    /// 8. Clear created_task_id on all proposals
    /// 9. Set session status to Active
    pub async fn reopen(&self, session_id: &IdeationSessionId) -> AppResult<()> {
        // 1. Validate session is Accepted or Archived
        let session = self
            .ideation_session_repo
            .get_by_id(session_id)
            .await?
            .ok_or_else(|| crate::error::AppError::NotFound(format!(
                "Session not found: {}",
                session_id.as_str()
            )))?;

        match session.status {
            IdeationSessionStatus::Accepted | IdeationSessionStatus::Archived => {}
            _ => {
                return Err(crate::error::AppError::Validation(format!(
                    "Cannot reopen session in '{}' status. Only Accepted or Archived sessions can be reopened.",
                    session.status
                )));
            }
        }

        // 2. Get all tasks for this session
        let tasks = self.task_repo.get_by_ideation_session(session_id).await?;

        // 3. Stop running agents (direct stop, bypass TransitionHandler)
        for task in &tasks {
            if AGENT_ACTIVE_STATUSES.contains(&task.internal_status) {
                let context_type = match task.internal_status {
                    InternalStatus::Reviewing => "review",
                    InternalStatus::Merging => "merge",
                    _ => "task_execution",
                };
                let key = RunningAgentKey::new(context_type, task.id.as_str());
                let _ = self.running_agent_registry.stop(&key).await;
            }
        }

        // Get project for git operations (best-effort — git cleanup is secondary)
        let project = self
            .project_repo
            .get_by_id(&session.project_id)
            .await
            .ok()
            .flatten();

        if let Some(ref project) = project {
            let repo_path = PathBuf::from(&project.working_directory);
            let base_branch = project.base_branch.as_deref().unwrap_or("main");

            // 4. Abort any in-progress rebase (Local mode safety)
            if GitService::is_rebase_in_progress(&repo_path) {
                let _ = GitService::abort_rebase(&repo_path);
            }

            // 5. Checkout base branch (Local mode — avoid deleting current branch)
            let _ = GitService::checkout_branch(&repo_path, base_branch);

            // 6a. Delete worktrees and task branches
            for task in &tasks {
                if let Some(ref worktree_path) = task.worktree_path {
                    let _ =
                        GitService::delete_worktree(&repo_path, &PathBuf::from(worktree_path));
                }
                if let Some(ref branch) = task.task_branch {
                    let _ = GitService::delete_branch(&repo_path, branch, true);
                }
            }
        }

        // 6b. Delete tasks from DB
        for task in &tasks {
            if let Err(e) = self.task_repo.delete(&task.id).await {
                tracing::warn!(
                    task_id = task.id.as_str(),
                    session_id = session_id.as_str(),
                    error = %e,
                    "Failed to delete task during session reopen"
                );
            }
        }

        // 7. Clean plan branch (delete git branch, mark Abandoned)
        if let Ok(Some(plan_branch)) = self.plan_branch_repo.get_by_session_id(session_id).await {
            if plan_branch.status == PlanBranchStatus::Active {
                if let Some(ref project) = project {
                    let repo_path = PathBuf::from(&project.working_directory);
                    let _ =
                        GitService::delete_feature_branch(&repo_path, &plan_branch.branch_name);
                }
                let _ = self
                    .plan_branch_repo
                    .update_status(&plan_branch.id, PlanBranchStatus::Abandoned)
                    .await;
            }
        }

        // 8. Clear created_task_id on all proposals
        self.task_proposal_repo
            .clear_created_task_ids_by_session(session_id)
            .await?;

        // 9. Set session status to Active (clears archived_at/converted_at)
        self.ideation_session_repo
            .update_status(session_id, IdeationSessionStatus::Active)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::AppState;
    use crate::domain::entities::{
        IdeationSession, IdeationSessionStatus, Priority, ProjectId, Task, TaskCategory,
        TaskProposal,
    };

    #[tokio::test]
    async fn test_reopen_accepted_session() {
        let state = AppState::new_test();
        let project_id = ProjectId::new();

        // Create session and accept it
        let session = IdeationSession::new(project_id.clone());
        let created = state
            .ideation_session_repo
            .create(session)
            .await
            .unwrap();
        state
            .ideation_session_repo
            .update_status(&created.id, IdeationSessionStatus::Accepted)
            .await
            .unwrap();

        // Create a proposal with created_task_id set
        let mut proposal = TaskProposal::new(
            created.id.clone(),
            "Test Proposal",
            TaskCategory::Feature,
            Priority::Medium,
        );
        proposal.created_task_id = Some(crate::domain::entities::TaskId::new());
        state.task_proposal_repo.create(proposal.clone()).await.unwrap();

        // Create tasks linked to this session
        let mut task = Task::new(project_id.clone(), "Test Task".to_string());
        task.ideation_session_id = Some(created.id.clone());
        let created_task = state.task_repo.create(task).await.unwrap();

        // Reopen
        let service = SessionReopenService::new(
            Arc::clone(&state.task_repo),
            Arc::clone(&state.task_proposal_repo),
            Arc::clone(&state.ideation_session_repo),
            Arc::clone(&state.plan_branch_repo),
            Arc::clone(&state.project_repo),
            Arc::clone(&state.running_agent_registry),
        );
        service.reopen(&created.id).await.unwrap();

        // Verify session is Active
        let reopened = state
            .ideation_session_repo
            .get_by_id(&created.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(reopened.status, IdeationSessionStatus::Active);

        // Verify task is deleted
        assert!(state
            .task_repo
            .get_by_id(&created_task.id)
            .await
            .unwrap()
            .is_none());

        // Verify proposal created_task_id is cleared
        let updated_proposal = state
            .task_proposal_repo
            .get_by_id(&proposal.id)
            .await
            .unwrap()
            .unwrap();
        assert!(updated_proposal.created_task_id.is_none());
    }

    #[tokio::test]
    async fn test_reopen_archived_session() {
        let state = AppState::new_test();
        let project_id = ProjectId::new();

        let session = IdeationSession::new(project_id);
        let created = state
            .ideation_session_repo
            .create(session)
            .await
            .unwrap();
        state
            .ideation_session_repo
            .update_status(&created.id, IdeationSessionStatus::Archived)
            .await
            .unwrap();

        let service = SessionReopenService::new(
            Arc::clone(&state.task_repo),
            Arc::clone(&state.task_proposal_repo),
            Arc::clone(&state.ideation_session_repo),
            Arc::clone(&state.plan_branch_repo),
            Arc::clone(&state.project_repo),
            Arc::clone(&state.running_agent_registry),
        );
        service.reopen(&created.id).await.unwrap();

        let reopened = state
            .ideation_session_repo
            .get_by_id(&created.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(reopened.status, IdeationSessionStatus::Active);
    }

    #[tokio::test]
    async fn test_reopen_active_session_fails() {
        let state = AppState::new_test();
        let project_id = ProjectId::new();

        let session = IdeationSession::new(project_id);
        let created = state
            .ideation_session_repo
            .create(session)
            .await
            .unwrap();

        let service = SessionReopenService::new(
            Arc::clone(&state.task_repo),
            Arc::clone(&state.task_proposal_repo),
            Arc::clone(&state.ideation_session_repo),
            Arc::clone(&state.plan_branch_repo),
            Arc::clone(&state.project_repo),
            Arc::clone(&state.running_agent_registry),
        );

        let result = service.reopen(&created.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_reopen_nonexistent_session_fails() {
        let state = AppState::new_test();

        let service = SessionReopenService::new(
            Arc::clone(&state.task_repo),
            Arc::clone(&state.task_proposal_repo),
            Arc::clone(&state.ideation_session_repo),
            Arc::clone(&state.plan_branch_repo),
            Arc::clone(&state.project_repo),
            Arc::clone(&state.running_agent_registry),
        );

        let result = service.reopen(&IdeationSessionId::new()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_reopen_with_no_tasks() {
        let state = AppState::new_test();
        let project_id = ProjectId::new();

        let session = IdeationSession::new(project_id);
        let created = state
            .ideation_session_repo
            .create(session)
            .await
            .unwrap();
        state
            .ideation_session_repo
            .update_status(&created.id, IdeationSessionStatus::Accepted)
            .await
            .unwrap();

        let service = SessionReopenService::new(
            Arc::clone(&state.task_repo),
            Arc::clone(&state.task_proposal_repo),
            Arc::clone(&state.ideation_session_repo),
            Arc::clone(&state.plan_branch_repo),
            Arc::clone(&state.project_repo),
            Arc::clone(&state.running_agent_registry),
        );
        service.reopen(&created.id).await.unwrap();

        let reopened = state
            .ideation_session_repo
            .get_by_id(&created.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(reopened.status, IdeationSessionStatus::Active);
    }
}
