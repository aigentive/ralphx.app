use super::*;
use crate::domain::entities::IdeationSessionId;
use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

fn create_test_branch() -> PlanBranch {
    PlanBranch::new(
        ArtifactId::from_string("art-test-1"),
        IdeationSessionId::from_string("sess-test-1"),
        ProjectId::from_string("proj-test-1".to_string()),
        "ralphx/test-project/plan-abc123".to_string(),
        "main".to_string(),
    )
}

async fn setup_repo() -> SqlitePlanBranchRepository {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    SqlitePlanBranchRepository::new(conn)
}

#[tokio::test]
async fn test_create_and_get_by_plan_artifact_id() {
    let repo = setup_repo().await;
    let branch = create_test_branch();
    let artifact_id = branch.plan_artifact_id.clone();

    let created = repo.create(branch).await.unwrap();
    assert_eq!(created.plan_artifact_id, artifact_id);

    let retrieved = repo
        .get_by_plan_artifact_id(&artifact_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.plan_artifact_id, artifact_id);
    assert_eq!(retrieved.branch_name, "ralphx/test-project/plan-abc123");
    assert_eq!(retrieved.source_branch, "main");
    assert_eq!(retrieved.status, PlanBranchStatus::Active);
    assert!(retrieved.merge_task_id.is_none());
    assert!(retrieved.merged_at.is_none());
}

#[tokio::test]
async fn test_get_by_plan_artifact_id_not_found() {
    let repo = setup_repo().await;
    let result = repo
        .get_by_plan_artifact_id(&ArtifactId::from_string("nonexistent"))
        .await
        .unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_get_by_session_id() {
    let repo = setup_repo().await;
    let branch = create_test_branch();
    let session_id = branch.session_id.clone();

    repo.create(branch).await.unwrap();

    let retrieved = repo.get_by_session_id(&session_id).await.unwrap().unwrap();
    assert_eq!(retrieved.session_id, session_id);
    assert_eq!(retrieved.branch_name, "ralphx/test-project/plan-abc123");
    assert_eq!(retrieved.status, PlanBranchStatus::Active);
}

#[tokio::test]
async fn test_get_by_session_id_not_found() {
    let repo = setup_repo().await;
    let result = repo
        .get_by_session_id(&IdeationSessionId::from_string("nonexistent"))
        .await
        .unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_get_by_merge_task_id() {
    let repo = setup_repo().await;
    let mut branch = create_test_branch();
    let merge_task_id = TaskId::from_string("merge-task-1".to_string());
    branch.merge_task_id = Some(merge_task_id.clone());

    repo.create(branch).await.unwrap();

    let retrieved = repo
        .get_by_merge_task_id(&merge_task_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        retrieved.merge_task_id.as_ref().unwrap().as_str(),
        "merge-task-1"
    );
}

#[tokio::test]
async fn test_get_by_merge_task_id_not_found() {
    let repo = setup_repo().await;
    let result = repo
        .get_by_merge_task_id(&TaskId::from_string("nonexistent".to_string()))
        .await
        .unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_get_by_project_id() {
    let repo = setup_repo().await;
    let project_id = ProjectId::from_string("proj-multi".to_string());

    let branch1 = PlanBranch::new(
        ArtifactId::from_string("art-1"),
        IdeationSessionId::from_string("sess-1"),
        project_id.clone(),
        "ralphx/proj/plan-1".to_string(),
        "main".to_string(),
    );
    let branch2 = PlanBranch::new(
        ArtifactId::from_string("art-2"),
        IdeationSessionId::from_string("sess-2"),
        project_id.clone(),
        "ralphx/proj/plan-2".to_string(),
        "main".to_string(),
    );

    repo.create(branch1).await.unwrap();
    repo.create(branch2).await.unwrap();

    let branches = repo.get_by_project_id(&project_id).await.unwrap();
    assert_eq!(branches.len(), 2);
}

#[tokio::test]
async fn test_get_by_project_id_empty() {
    let repo = setup_repo().await;
    let branches = repo
        .get_by_project_id(&ProjectId::from_string("empty-proj".to_string()))
        .await
        .unwrap();
    assert!(branches.is_empty());
}

#[tokio::test]
async fn test_update_status() {
    let repo = setup_repo().await;
    let branch = create_test_branch();
    let branch_id = branch.id.clone();
    let artifact_id = branch.plan_artifact_id.clone();

    repo.create(branch).await.unwrap();
    repo.update_status(&branch_id, PlanBranchStatus::Abandoned)
        .await
        .unwrap();

    let retrieved = repo
        .get_by_plan_artifact_id(&artifact_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.status, PlanBranchStatus::Abandoned);
}

#[tokio::test]
async fn test_update_status_not_found() {
    let repo = setup_repo().await;
    let result = repo
        .update_status(
            &PlanBranchId::from_string("nonexistent"),
            PlanBranchStatus::Merged,
        )
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_set_merge_task_id() {
    let repo = setup_repo().await;
    let branch = create_test_branch();
    let branch_id = branch.id.clone();
    let artifact_id = branch.plan_artifact_id.clone();
    let merge_task_id = TaskId::from_string("mt-1".to_string());

    repo.create(branch).await.unwrap();
    repo.set_merge_task_id(&branch_id, &merge_task_id)
        .await
        .unwrap();

    let retrieved = repo
        .get_by_plan_artifact_id(&artifact_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.merge_task_id.unwrap().as_str(), "mt-1");
}

#[tokio::test]
async fn test_set_merge_task_id_not_found() {
    let repo = setup_repo().await;
    let result = repo
        .set_merge_task_id(
            &PlanBranchId::from_string("nonexistent"),
            &TaskId::from_string("mt-1".to_string()),
        )
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_set_merged() {
    let repo = setup_repo().await;
    let branch = create_test_branch();
    let branch_id = branch.id.clone();
    let artifact_id = branch.plan_artifact_id.clone();

    repo.create(branch).await.unwrap();
    repo.set_merged(&branch_id).await.unwrap();

    let retrieved = repo
        .get_by_plan_artifact_id(&artifact_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.status, PlanBranchStatus::Merged);
    assert!(retrieved.merged_at.is_some());
}

#[tokio::test]
async fn test_set_merged_not_found() {
    let repo = setup_repo().await;
    let result = repo
        .set_merged(&PlanBranchId::from_string("nonexistent"))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_unique_constraint_on_plan_artifact_id() {
    let repo = setup_repo().await;
    let branch1 = create_test_branch();
    let branch2 = PlanBranch::new(
        branch1.plan_artifact_id.clone(), // same artifact id
        IdeationSessionId::from_string("sess-different"),
        ProjectId::from_string("proj-different".to_string()),
        "ralphx/other/plan-xyz".to_string(),
        "main".to_string(),
    );

    repo.create(branch1).await.unwrap();
    let result = repo.create(branch2).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_with_merge_task_id() {
    let repo = setup_repo().await;
    let mut branch = create_test_branch();
    branch.merge_task_id = Some(TaskId::from_string("mt-preset".to_string()));

    let created = repo.create(branch).await.unwrap();
    assert_eq!(created.merge_task_id.unwrap().as_str(), "mt-preset");
}

#[tokio::test]
async fn test_get_by_merge_task_id_after_set() {
    let repo = setup_repo().await;
    let branch = create_test_branch();
    let branch_id = branch.id.clone();
    let merge_task_id = TaskId::from_string("mt-lookup".to_string());

    repo.create(branch).await.unwrap();
    repo.set_merge_task_id(&branch_id, &merge_task_id)
        .await
        .unwrap();

    let retrieved = repo
        .get_by_merge_task_id(&merge_task_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.id, branch_id);
}
