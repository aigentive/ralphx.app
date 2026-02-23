use super::*;
use crate::application::AppState;
use crate::domain::entities::{
    IdeationSession, IdeationSessionStatus, Priority, ProjectId, ProposalCategory, Task,
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
    let created = state.ideation_session_repo.create(session).await.unwrap();
    state
        .ideation_session_repo
        .update_status(&created.id, IdeationSessionStatus::Accepted)
        .await
        .unwrap();

    // Create a proposal with created_task_id set
    let mut proposal = TaskProposal::new(
        created.id.clone(),
        "Test Proposal",
        ProposalCategory::Feature,
        Priority::Medium,
    );
    proposal.created_task_id = Some(crate::domain::entities::TaskId::new());
    state
        .task_proposal_repo
        .create(proposal.clone())
        .await
        .unwrap();

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
    let created = state.ideation_session_repo.create(session).await.unwrap();
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
    let created = state.ideation_session_repo.create(session).await.unwrap();

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
    let created = state.ideation_session_repo.create(session).await.unwrap();
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

#[tokio::test]
async fn test_reopen_deletes_plan_branch_record() {
    use crate::domain::entities::{ArtifactId, PlanBranch};

    let state = AppState::new_test();
    let project_id = ProjectId::new();

    // Create session and accept it
    let session = IdeationSession::new(project_id.clone());
    let created = state.ideation_session_repo.create(session).await.unwrap();
    state
        .ideation_session_repo
        .update_status(&created.id, IdeationSessionStatus::Accepted)
        .await
        .unwrap();

    // Create plan branch for this session
    let plan_branch = PlanBranch::new(
        ArtifactId::new(),
        created.id.clone(),
        project_id,
        "ralphx/test-project/plan-test".to_string(),
        "main".to_string(),
    );
    state
        .plan_branch_repo
        .create(plan_branch.clone())
        .await
        .unwrap();

    // Verify plan branch exists
    let found = state
        .plan_branch_repo
        .get_by_session_id(&created.id)
        .await
        .unwrap();
    assert!(found.is_some());

    // Reopen session
    let service = build_service(&state);
    service.reopen(&created.id).await.unwrap();

    // Verify plan branch record is deleted (None)
    let after_reopen = state
        .plan_branch_repo
        .get_by_session_id(&created.id)
        .await
        .unwrap();
    assert!(after_reopen.is_none());
}

#[tokio::test]
async fn test_reopen_session_with_merged_plan_branch_deletes_db_record() {
    use crate::domain::entities::{ArtifactId, PlanBranch};

    let state = AppState::new_test();
    let project_id = ProjectId::new();

    // Create session and accept it
    let session = IdeationSession::new(project_id.clone());
    let created = state.ideation_session_repo.create(session).await.unwrap();
    state
        .ideation_session_repo
        .update_status(&created.id, IdeationSessionStatus::Accepted)
        .await
        .unwrap();

    // Create plan branch and set it to Merged (simulates completed first accept)
    let plan_branch = PlanBranch::new(
        ArtifactId::new(),
        created.id.clone(),
        project_id,
        "ralphx/test-project/plan-merged".to_string(),
        "main".to_string(),
    );
    let created_branch = state
        .plan_branch_repo
        .create(plan_branch)
        .await
        .unwrap();
    state
        .plan_branch_repo
        .set_merged(&created_branch.id)
        .await
        .unwrap();

    // Verify plan branch is Merged
    let found = state
        .plan_branch_repo
        .get_by_session_id(&created.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.status, PlanBranchStatus::Merged);

    // Reopen session
    let service = build_service(&state);
    service.reopen(&created.id).await.unwrap();

    // Verify plan branch DB record is deleted even though it was Merged
    let after_reopen = state
        .plan_branch_repo
        .get_by_session_id(&created.id)
        .await
        .unwrap();
    assert!(
        after_reopen.is_none(),
        "Merged plan branch should be deleted on reopen so re-accept creates fresh merge task"
    );

    // Verify session is back to Active
    let reopened = state
        .ideation_session_repo
        .get_by_id(&created.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(reopened.status, IdeationSessionStatus::Active);
}
