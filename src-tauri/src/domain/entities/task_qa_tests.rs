use super::*;
use crate::domain::qa::{AcceptanceCriterion, QAStepResult, QATestStep};

#[test]
fn test_new_task_qa() {
    let task_id = TaskId::from_string("task-123".to_string());
    let qa = TaskQA::new(task_id.clone());

    assert_eq!(qa.task_id, task_id);
    assert!(qa.acceptance_criteria.is_none());
    assert!(qa.qa_test_steps.is_none());
    assert!(qa.prep_started_at.is_none());
    assert!(qa.prep_completed_at.is_none());
    assert!(qa.test_results.is_none());
    assert!(qa.screenshots.is_empty());
}

#[test]
fn test_with_id() {
    let qa_id = TaskQAId::from_string("qa-123");
    let task_id = TaskId::from_string("task-123".to_string());
    let qa = TaskQA::with_id(qa_id.clone(), task_id.clone());

    assert_eq!(qa.id, qa_id);
    assert_eq!(qa.task_id, task_id);
}

#[test]
fn test_start_prep() {
    let task_id = TaskId::from_string("task-123".to_string());
    let mut qa = TaskQA::new(task_id);

    qa.start_prep("agent-1".to_string());

    assert_eq!(qa.prep_agent_id, Some("agent-1".to_string()));
    assert!(qa.prep_started_at.is_some());
    assert!(qa.is_prep_in_progress());
    assert!(!qa.is_prep_complete());
}

#[test]
fn test_complete_prep() {
    let task_id = TaskId::from_string("task-123".to_string());
    let mut qa = TaskQA::new(task_id);

    qa.start_prep("agent-1".to_string());

    let criteria =
        AcceptanceCriteria::from_criteria(vec![AcceptanceCriterion::visual("AC1", "Test visual")]);
    let steps = QATestSteps::from_steps(vec![QATestStep::new(
        "QA1",
        "AC1",
        "Test step",
        vec![],
        "Expected",
    )]);

    qa.complete_prep(criteria.clone(), steps.clone());

    assert!(qa.is_prep_complete());
    assert!(!qa.is_prep_in_progress());
    assert!(qa.acceptance_criteria.is_some());
    assert!(qa.qa_test_steps.is_some());
}

#[test]
fn test_complete_refinement() {
    let task_id = TaskId::from_string("task-123".to_string());
    let mut qa = TaskQA::new(task_id);

    let refined_steps = QATestSteps::from_steps(vec![QATestStep::new(
        "QA1",
        "AC1",
        "Refined step",
        vec![],
        "Expected",
    )]);

    qa.complete_refinement(
        "agent-2".to_string(),
        "Added button to header".to_string(),
        refined_steps,
    );

    assert!(qa.is_refinement_complete());
    assert_eq!(qa.refinement_agent_id, Some("agent-2".to_string()));
    assert!(qa.actual_implementation.is_some());
    assert!(qa.refined_test_steps.is_some());
}

#[test]
fn test_effective_test_steps() {
    let task_id = TaskId::from_string("task-123".to_string());
    let mut qa = TaskQA::new(task_id);

    // No steps initially
    assert!(qa.effective_test_steps().is_none());

    // Add initial steps
    let initial_steps = QATestSteps::from_steps(vec![QATestStep::new(
        "QA1",
        "AC1",
        "Initial step",
        vec![],
        "Expected",
    )]);
    qa.qa_test_steps = Some(initial_steps);

    // Should return initial steps
    assert!(qa.effective_test_steps().is_some());
    assert_eq!(
        qa.effective_test_steps().unwrap().qa_steps[0].description,
        "Initial step"
    );

    // Add refined steps
    let refined_steps = QATestSteps::from_steps(vec![QATestStep::new(
        "QA1",
        "AC1",
        "Refined step",
        vec![],
        "Expected",
    )]);
    qa.refined_test_steps = Some(refined_steps);

    // Should return refined steps
    assert_eq!(
        qa.effective_test_steps().unwrap().qa_steps[0].description,
        "Refined step"
    );
}

#[test]
fn test_start_testing() {
    let task_id = TaskId::from_string("task-123".to_string());
    let mut qa = TaskQA::new(task_id);

    qa.start_testing("agent-3".to_string());

    assert_eq!(qa.test_agent_id, Some("agent-3".to_string()));
}

#[test]
fn test_complete_testing_passed() {
    let task_id = TaskId::from_string("task-123".to_string());
    let mut qa = TaskQA::new(task_id.clone());

    let results =
        QAResults::from_results(task_id.as_str(), vec![QAStepResult::passed("QA1", None)]);

    qa.complete_testing(results);

    assert!(qa.is_testing_complete());
    assert!(qa.is_passed());
    assert!(!qa.is_failed());
}

#[test]
fn test_complete_testing_failed() {
    let task_id = TaskId::from_string("task-123".to_string());
    let mut qa = TaskQA::new(task_id.clone());

    let results = QAResults::from_results(
        task_id.as_str(),
        vec![QAStepResult::failed("QA1", "Element not found", None)],
    );

    qa.complete_testing(results);

    assert!(qa.is_testing_complete());
    assert!(!qa.is_passed());
    assert!(qa.is_failed());
}

#[test]
fn test_add_screenshot() {
    let task_id = TaskId::from_string("task-123".to_string());
    let mut qa = TaskQA::new(task_id);

    qa.add_screenshot("screenshots/test1.png".to_string());
    qa.add_screenshot("screenshots/test2.png".to_string());

    assert_eq!(qa.screenshots.len(), 2);
    assert!(qa
        .screenshots
        .contains(&"screenshots/test1.png".to_string()));
}

#[test]
fn test_task_qa_serialization() {
    let task_id = TaskId::from_string("task-123".to_string());
    let qa = TaskQA::new(task_id);

    let json = serde_json::to_string(&qa).unwrap();
    let parsed: TaskQA = serde_json::from_str(&json).unwrap();

    assert_eq!(qa.id, parsed.id);
    assert_eq!(qa.task_id, parsed.task_id);
}
