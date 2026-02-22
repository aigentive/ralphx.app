// Service for reopening accepted/archived ideation sessions
// Delegates task cleanup to TaskCleanupService. Handles session-level ops:
// plan branch cleanup, proposal clearing, session status reset.

use std::path::PathBuf;
use std::sync::Arc;

use crate::application::git_service::GitService;
use crate::application::task_cleanup_service::{StopMode, TaskCleanupService};
use crate::domain::entities::plan_branch::PlanBranchStatus;
use crate::domain::entities::{IdeationSessionId, IdeationSessionStatus};
use crate::domain::repositories::{
    IdeationSessionRepository, PlanBranchRepository, ProjectRepository, TaskProposalRepository,
    TaskRepository,
};
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
    /// 4. Clean plan branch (delete git branch, delete DB record)
    /// 5. Clear created_task_id on all proposals
    /// 6. Set session status to Active
    pub async fn reopen(&self, session_id: &IdeationSessionId) -> AppResult<()> {
        // 1. Validate session is Accepted or Archived
        let session = self
            .ideation_session_repo
            .get_by_id(session_id)
            .await?
            .ok_or_else(|| {
                crate::error::AppError::NotFound(format!(
                    "Session not found: {}",
                    session_id.as_str()
                ))
            })?;

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

        // 4. Clean plan branch (delete git branch, delete DB record)
        if let Ok(Some(plan_branch)) = self.plan_branch_repo.get_by_session_id(session_id).await {
            if plan_branch.status == PlanBranchStatus::Active {
                if let Ok(Some(project)) = self.project_repo.get_by_id(&session.project_id).await {
                    let repo_path = PathBuf::from(&project.working_directory);
                    let _ = GitService::delete_feature_branch(&repo_path, &plan_branch.branch_name)
                        .await;
                }
                let _ = self.plan_branch_repo.delete(&plan_branch.id).await;
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
#[path = "session_reopen_service_tests.rs"]
mod tests;
