// Service for reopening accepted/archived ideation sessions
// Delegates task cleanup to TaskCleanupService. Handles session-level ops:
// plan branch cleanup, proposal clearing, session status reset.

use std::path::PathBuf;
use std::sync::Arc;

use crate::application::task_cleanup_service::{StopMode, TaskCleanupService};
use crate::domain::entities::plan_branch::PlanBranchStatus;
use crate::domain::entities::{IdeationSessionId, IdeationSessionStatus};
use crate::domain::repositories::{
    IdeationSessionRepository, PlanBranchRepository, ProjectRepository, TaskProposalRepository,
    TaskRepository,
};
use crate::application::git_service::GitService;
use crate::error::AppResult;

pub struct SessionReopenService {
    task_repo: Arc<dyn TaskRepository>,
    task_proposal_repo: Arc<dyn TaskProposalRepository>,
    ideation_session_repo: Arc<dyn IdeationSessionRepository>,
    plan_branch_repo: Arc<dyn PlanBranchRepository>,
    project_repo: Arc<dyn ProjectRepository>,
    task_cleanup_service: TaskCleanupService,
}

impl SessionReopenService {
    pub fn new(
        task_repo: Arc<dyn TaskRepository>,
        task_proposal_repo: Arc<dyn TaskProposalRepository>,
        ideation_session_repo: Arc<dyn IdeationSessionRepository>,
        plan_branch_repo: Arc<dyn PlanBranchRepository>,
        project_repo: Arc<dyn ProjectRepository>,
        task_cleanup_service: TaskCleanupService,
    ) -> Self {
        Self {
            task_repo,
            task_proposal_repo,
            ideation_session_repo,
            plan_branch_repo,
            project_repo,
            task_cleanup_service,
        }
    }

    /// Reopen an accepted/archived session back to Active.
    ///
    /// Cleanup order:
    /// 1. Validate session is Accepted or Archived
    /// 2. Get all tasks for this session
    /// 3. Delegate task cleanup to TaskCleanupService (stop agents, git cleanup, DB delete)
    /// 4. Clean plan branch (delete git branch, mark Abandoned)
    /// 5. Clear created_task_id on all proposals
    /// 6. Set session status to Active
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

        // 3. Cleanup tasks: stop agents (DirectStop), delete git resources, delete from DB
        //    No events emitted — the session-level event handles UI updates.
        let _report = self
            .task_cleanup_service
            .cleanup_tasks(&tasks, StopMode::DirectStop, false)
            .await;

        // 4. Clean plan branch (delete git branch, mark Abandoned)
        if let Ok(Some(plan_branch)) = self.plan_branch_repo.get_by_session_id(session_id).await {
            if plan_branch.status == PlanBranchStatus::Active {
                if let Ok(Some(project)) = self.project_repo.get_by_id(&session.project_id).await {
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

        // 5. Clear created_task_id on all proposals
        self.task_proposal_repo
            .clear_created_task_ids_by_session(session_id)
            .await?;

        // 6. Set session status to Active (clears archived_at/converted_at)
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

    fn build_service(state: &AppState) -> SessionReopenService {
        let cleanup = TaskCleanupService::new(
            Arc::clone(&state.task_repo),
            Arc::clone(&state.project_repo),
            Arc::clone(&state.running_agent_registry),
            None,
        );
        SessionReopenService::new(
            Arc::clone(&state.task_repo),
            Arc::clone(&state.task_proposal_repo),
            Arc::clone(&state.ideation_session_repo),
            Arc::clone(&state.plan_branch_repo),
            Arc::clone(&state.project_repo),
            cleanup,
        )
    }

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
        let service = build_service(&state);
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

        let service = build_service(&state);
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

        let service = build_service(&state);

        let result = service.reopen(&created.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_reopen_nonexistent_session_fails() {
        let state = AppState::new_test();

        let service = build_service(&state);

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

        let service = build_service(&state);
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
