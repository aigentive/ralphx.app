use super::*;
use crate::domain::qa::{AcceptanceCriterion, QAStepResult, QATestStep};
use crate::infrastructure::sqlite::connection::open_memory_connection;
use crate::infrastructure::sqlite::migrations::run_migrations;

async fn setup_test_db() -> SqliteTaskQARepository {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Insert a project and task for foreign key constraint
    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Test Task')",
        [],
    ).unwrap();

    SqliteTaskQARepository::new(conn)
}

#[tokio::test]
async fn test_create_and_get_by_id() {
    let repo = setup_test_db().await;
    let task_id = TaskId::from_string("task-1".to_string());
    let task_qa = TaskQA::new(task_id);
    let qa_id = task_qa.id.clone();

    repo.create(&task_qa).await.unwrap();

    let retrieved = repo.get_by_id(&qa_id).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id, qa_id);
}

#[tokio::test]
async fn test_get_by_task_id() {
    let repo = setup_test_db().await;
    let task_id = TaskId::from_string("task-1".to_string());
    let task_qa = TaskQA::new(task_id.clone());

    repo.create(&task_qa).await.unwrap();

    let retrieved = repo.get_by_task_id(&task_id).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().task_id, task_id);
}

#[tokio::test]
async fn test_update_prep() {
    let repo = setup_test_db().await;
    let task_id = TaskId::from_string("task-1".to_string());
    let task_qa = TaskQA::new(task_id);
    let qa_id = task_qa.id.clone();

    repo.create(&task_qa).await.unwrap();

    let criteria = AcceptanceCriteria::from_criteria(vec![AcceptanceCriterion::visual(
        "AC1",
        "Test visual check",
    )]);
    let steps = QATestSteps::from_steps(vec![QATestStep::new(
        "QA1",
        "AC1",
        "Test step",
        vec!["cmd".into()],
        "Expected",
    )]);

    repo.update_prep(&qa_id, "agent-1", &criteria, &steps)
        .await
        .unwrap();

    let retrieved = repo.get_by_id(&qa_id).await.unwrap().unwrap();
    assert!(retrieved.acceptance_criteria.is_some());
    assert!(retrieved.qa_test_steps.is_some());
    assert!(retrieved.prep_completed_at.is_some());
    assert_eq!(retrieved.prep_agent_id, Some("agent-1".into()));
}

#[tokio::test]
async fn test_update_refinement() {
    let repo = setup_test_db().await;
    let task_id = TaskId::from_string("task-1".to_string());
    let task_qa = TaskQA::new(task_id);
    let qa_id = task_qa.id.clone();

    repo.create(&task_qa).await.unwrap();

    let refined_steps = QATestSteps::from_steps(vec![QATestStep::new(
        "QA1",
        "AC1",
        "Refined step",
        vec![],
        "Expected",
    )]);

    repo.update_refinement(&qa_id, "agent-2", "Added button to header", &refined_steps)
        .await
        .unwrap();

    let retrieved = repo.get_by_id(&qa_id).await.unwrap().unwrap();
    assert!(retrieved.refined_test_steps.is_some());
    assert!(retrieved.actual_implementation.is_some());
    assert!(retrieved.refinement_completed_at.is_some());
    assert_eq!(retrieved.refinement_agent_id, Some("agent-2".into()));
}

#[tokio::test]
async fn test_update_results() {
    let repo = setup_test_db().await;
    let task_id = TaskId::from_string("task-1".to_string());
    let task_qa = TaskQA::new(task_id.clone());
    let qa_id = task_qa.id.clone();

    repo.create(&task_qa).await.unwrap();

    let results = QAResults::from_results(
        task_id.as_str(),
        vec![
            QAStepResult::passed("QA1", Some("ss1.png".into())),
            QAStepResult::passed("QA2", Some("ss2.png".into())),
        ],
    );
    let screenshots = vec!["ss1.png".to_string(), "ss2.png".to_string()];

    repo.update_results(&qa_id, "agent-3", &results, &screenshots)
        .await
        .unwrap();

    let retrieved = repo.get_by_id(&qa_id).await.unwrap().unwrap();
    assert!(retrieved.test_results.is_some());
    assert!(retrieved.test_completed_at.is_some());
    assert_eq!(retrieved.test_agent_id, Some("agent-3".into()));
    assert_eq!(retrieved.screenshots.len(), 2);
}

#[tokio::test]
async fn test_get_pending_prep() {
    let repo = setup_test_db().await;

    // Add another task
    {
        let conn = repo.db.inner().lock().await;
        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-2', 'proj-1', 'feature', 'Task 2')",
            [],
        ).unwrap();
    }

    // Create two TaskQA records
    let task_id1 = TaskId::from_string("task-1".to_string());
    let task_qa1 = TaskQA::new(task_id1);
    let qa_id1 = task_qa1.id.clone();

    let task_id2 = TaskId::from_string("task-2".to_string());
    let task_qa2 = TaskQA::new(task_id2);

    repo.create(&task_qa1).await.unwrap();
    repo.create(&task_qa2).await.unwrap();

    // Update prep for first one
    let criteria =
        AcceptanceCriteria::from_criteria(vec![AcceptanceCriterion::visual("AC1", "Test")]);
    let steps = QATestSteps::from_steps(vec![]);
    repo.update_prep(&qa_id1, "agent-1", &criteria, &steps)
        .await
        .unwrap();

    // Get pending prep - should only return task-2
    let pending = repo.get_pending_prep().await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(
        pending[0].task_id,
        TaskId::from_string("task-2".to_string())
    );
}

#[tokio::test]
async fn test_delete() {
    let repo = setup_test_db().await;
    let task_id = TaskId::from_string("task-1".to_string());
    let task_qa = TaskQA::new(task_id);
    let qa_id = task_qa.id.clone();

    repo.create(&task_qa).await.unwrap();
    repo.delete(&qa_id).await.unwrap();

    let retrieved = repo.get_by_id(&qa_id).await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_delete_by_task_id() {
    let repo = setup_test_db().await;
    let task_id = TaskId::from_string("task-1".to_string());
    let task_qa = TaskQA::new(task_id.clone());

    repo.create(&task_qa).await.unwrap();
    repo.delete_by_task_id(&task_id).await.unwrap();

    let retrieved = repo.get_by_task_id(&task_id).await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_exists_for_task() {
    let repo = setup_test_db().await;
    let task_id = TaskId::from_string("task-1".to_string());

    assert!(!repo.exists_for_task(&task_id).await.unwrap());

    let task_qa = TaskQA::new(task_id.clone());
    repo.create(&task_qa).await.unwrap();

    assert!(repo.exists_for_task(&task_id).await.unwrap());
}

#[tokio::test]
async fn test_json_storage_roundtrip() {
    let repo = setup_test_db().await;
    let task_id = TaskId::from_string("task-1".to_string());
    let task_qa = TaskQA::new(task_id.clone());
    let qa_id = task_qa.id.clone();

    repo.create(&task_qa).await.unwrap();

    // Add complex criteria with multiple types
    let criteria = AcceptanceCriteria::from_criteria(vec![
        AcceptanceCriterion::visual("AC1", "Visual test"),
        AcceptanceCriterion::behavior("AC2", "Behavior test"),
    ]);
    let steps = QATestSteps::from_steps(vec![
        QATestStep::new(
            "QA1",
            "AC1",
            "Step 1",
            vec!["cmd1".into(), "cmd2".into()],
            "Expected 1",
        ),
        QATestStep::new("QA2", "AC2", "Step 2", vec!["cmd3".into()], "Expected 2"),
    ]);

    repo.update_prep(&qa_id, "agent-1", &criteria, &steps)
        .await
        .unwrap();

    let retrieved = repo.get_by_id(&qa_id).await.unwrap().unwrap();

    // Verify JSON was stored and retrieved correctly
    let retrieved_criteria = retrieved.acceptance_criteria.unwrap();
    assert_eq!(retrieved_criteria.len(), 2);
    assert_eq!(retrieved_criteria.acceptance_criteria[0].id, "AC1");
    assert_eq!(retrieved_criteria.acceptance_criteria[1].id, "AC2");

    let retrieved_steps = retrieved.qa_test_steps.unwrap();
    assert_eq!(retrieved_steps.len(), 2);
    assert_eq!(retrieved_steps.qa_steps[0].commands.len(), 2);
}
