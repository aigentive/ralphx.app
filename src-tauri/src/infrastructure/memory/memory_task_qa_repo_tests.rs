use super::*;
use crate::domain::qa::{AcceptanceCriterion, QAStepResult, QATestStep};

fn create_test_task_qa(task_id: &str) -> TaskQA {
    TaskQA::new(TaskId::from_string(task_id.to_string()))
}

#[tokio::test]
async fn test_create_and_get_by_id() {
    let repo = MemoryTaskQARepository::new();
    let task_qa = create_test_task_qa("task-123");
    let qa_id = task_qa.id.clone();

    repo.create(&task_qa).await.unwrap();

    let retrieved = repo.get_by_id(&qa_id).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().task_id.as_str(), "task-123");
}

#[tokio::test]
async fn test_get_by_task_id() {
    let repo = MemoryTaskQARepository::new();
    let task_qa = create_test_task_qa("task-123");
    let task_id = task_qa.task_id.clone();

    repo.create(&task_qa).await.unwrap();

    let retrieved = repo.get_by_task_id(&task_id).await.unwrap();
    assert!(retrieved.is_some());
}

#[tokio::test]
async fn test_get_by_id_returns_none_for_missing() {
    let repo = MemoryTaskQARepository::new();
    let qa_id = TaskQAId::new();

    let retrieved = repo.get_by_id(&qa_id).await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_update_prep() {
    let repo = MemoryTaskQARepository::new();
    let task_qa = create_test_task_qa("task-123");
    let qa_id = task_qa.id.clone();
    repo.create(&task_qa).await.unwrap();

    let criteria =
        AcceptanceCriteria::from_criteria(vec![AcceptanceCriterion::visual("AC1", "Test visual")]);
    let steps = QATestSteps::from_steps(vec![QATestStep::new(
        "QA1",
        "AC1",
        "Test step",
        vec![],
        "Expected",
    )]);

    repo.update_prep(&qa_id, "agent-1", &criteria, &steps)
        .await
        .unwrap();

    let retrieved = repo.get_by_id(&qa_id).await.unwrap().unwrap();
    assert!(retrieved.acceptance_criteria.is_some());
    assert!(retrieved.qa_test_steps.is_some());
    assert!(retrieved.prep_completed_at.is_some());
    assert_eq!(retrieved.prep_agent_id, Some("agent-1".to_string()));
}

#[tokio::test]
async fn test_update_refinement() {
    let repo = MemoryTaskQARepository::new();
    let task_qa = create_test_task_qa("task-123");
    let qa_id = task_qa.id.clone();
    repo.create(&task_qa).await.unwrap();

    let refined_steps = QATestSteps::from_steps(vec![QATestStep::new(
        "QA1",
        "AC1",
        "Refined step",
        vec![],
        "Expected",
    )]);

    repo.update_refinement(&qa_id, "agent-2", "Added button", &refined_steps)
        .await
        .unwrap();

    let retrieved = repo.get_by_id(&qa_id).await.unwrap().unwrap();
    assert!(retrieved.actual_implementation.is_some());
    assert!(retrieved.refined_test_steps.is_some());
    assert!(retrieved.refinement_completed_at.is_some());
}

#[tokio::test]
async fn test_update_results() {
    let repo = MemoryTaskQARepository::new();
    let task_qa = create_test_task_qa("task-123");
    let qa_id = task_qa.id.clone();
    repo.create(&task_qa).await.unwrap();

    let results = QAResults::from_results(
        "task-123",
        vec![QAStepResult::passed("QA1", Some("ss.png".into()))],
    );
    let screenshots = vec!["ss.png".to_string()];

    repo.update_results(&qa_id, "agent-3", &results, &screenshots)
        .await
        .unwrap();

    let retrieved = repo.get_by_id(&qa_id).await.unwrap().unwrap();
    assert!(retrieved.test_results.is_some());
    assert!(!retrieved.screenshots.is_empty());
    assert!(retrieved.test_completed_at.is_some());
}

#[tokio::test]
async fn test_get_pending_prep() {
    let repo = MemoryTaskQARepository::new();

    // Create two task QA records - one will have prep completed
    let task_qa1 = create_test_task_qa("task-1");
    let qa_id1 = task_qa1.id.clone();
    let task_qa2 = create_test_task_qa("task-2");

    repo.create(&task_qa1).await.unwrap();
    repo.create(&task_qa2).await.unwrap();

    // Complete prep for first one
    let criteria =
        AcceptanceCriteria::from_criteria(vec![AcceptanceCriterion::visual("AC1", "Test")]);
    let steps = QATestSteps::from_steps(vec![QATestStep::new(
        "QA1",
        "AC1",
        "Step",
        vec![],
        "Expected",
    )]);
    repo.update_prep(&qa_id1, "agent-1", &criteria, &steps)
        .await
        .unwrap();

    // Get pending prep - should only return task-2
    let pending = repo.get_pending_prep().await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].task_id.as_str(), "task-2");
}

#[tokio::test]
async fn test_delete() {
    let repo = MemoryTaskQARepository::new();
    let task_qa = create_test_task_qa("task-123");
    let qa_id = task_qa.id.clone();
    repo.create(&task_qa).await.unwrap();

    repo.delete(&qa_id).await.unwrap();

    let retrieved = repo.get_by_id(&qa_id).await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_delete_by_task_id() {
    let repo = MemoryTaskQARepository::new();
    let task_qa = create_test_task_qa("task-123");
    let task_id = task_qa.task_id.clone();
    repo.create(&task_qa).await.unwrap();

    repo.delete_by_task_id(&task_id).await.unwrap();

    let retrieved = repo.get_by_task_id(&task_id).await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_exists_for_task() {
    let repo = MemoryTaskQARepository::new();
    let task_id = TaskId::from_string("task-123".to_string());

    assert!(!repo.exists_for_task(&task_id).await.unwrap());

    let task_qa = TaskQA::new(task_id.clone());
    repo.create(&task_qa).await.unwrap();

    assert!(repo.exists_for_task(&task_id).await.unwrap());
}

#[tokio::test]
async fn test_with_records_prepopulates() {
    let task_qa1 = create_test_task_qa("task-1");
    let task_qa2 = create_test_task_qa("task-2");
    let qa_id1 = task_qa1.id.clone();
    let qa_id2 = task_qa2.id.clone();

    let repo = MemoryTaskQARepository::with_records(vec![task_qa1, task_qa2]);

    assert!(repo.get_by_id(&qa_id1).await.unwrap().is_some());
    assert!(repo.get_by_id(&qa_id2).await.unwrap().is_some());
}
