use super::*;
use crate::domain::entities::TaskQA;
use crate::domain::qa::{
    AcceptanceCriteria, AcceptanceCriterion, QAOverallStatus, QAStepStatus, QATestStep, QATestSteps,
};

async fn setup_test_state() -> AppState {
    AppState::new_test()
}

// ==================== QA Settings Tests ====================

#[tokio::test]
async fn test_get_qa_settings_returns_default() {
    let state = setup_test_state().await;

    let settings = state.qa_settings.read().await;

    assert!(settings.qa_enabled);
    assert!(settings.auto_qa_for_ui_tasks);
    assert!(!settings.auto_qa_for_api_tasks);
    assert_eq!(settings.browser_testing_url, "http://localhost:1420");
}

#[tokio::test]
async fn test_update_qa_settings_partial_update() {
    let state = setup_test_state().await;

    // Update only some fields
    {
        let mut settings = state.qa_settings.write().await;
        settings.qa_enabled = false;
        settings.browser_testing_url = "http://localhost:3000".to_string();
    }

    let settings = state.qa_settings.read().await;
    assert!(!settings.qa_enabled);
    assert!(settings.auto_qa_for_ui_tasks); // Unchanged
    assert_eq!(settings.browser_testing_url, "http://localhost:3000");
}

// ==================== TaskQA Tests ====================

#[tokio::test]
async fn test_get_task_qa_returns_none_for_missing() {
    let state = setup_test_state().await;
    let task_id = TaskId::from_string("nonexistent".to_string());

    let result = state.task_qa_repo.get_by_task_id(&task_id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_get_task_qa_returns_existing() {
    let state = setup_test_state().await;
    let task_id = TaskId::from_string("task-123".to_string());

    let task_qa = TaskQA::new(task_id.clone());
    state.task_qa_repo.create(&task_qa).await.unwrap();

    let result = state.task_qa_repo.get_by_task_id(&task_id).await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().task_id, task_id);
}

// ==================== QA Results Tests ====================

#[tokio::test]
async fn test_get_qa_results_returns_none_for_missing_task() {
    let state = setup_test_state().await;
    let task_id = TaskId::from_string("nonexistent".to_string());

    let result = state.task_qa_repo.get_by_task_id(&task_id).await.unwrap();
    let results = result.and_then(|qa| qa.test_results);
    assert!(results.is_none());
}

#[tokio::test]
async fn test_get_qa_results_returns_none_when_no_results() {
    let state = setup_test_state().await;
    let task_id = TaskId::from_string("task-123".to_string());

    let task_qa = TaskQA::new(task_id.clone());
    state.task_qa_repo.create(&task_qa).await.unwrap();

    let result = state.task_qa_repo.get_by_task_id(&task_id).await.unwrap();
    let results = result.and_then(|qa| qa.test_results);
    assert!(results.is_none());
}

#[tokio::test]
async fn test_get_qa_results_returns_results() {
    let state = setup_test_state().await;
    let task_id = TaskId::from_string("task-123".to_string());

    let task_qa = TaskQA::new(task_id.clone());
    let qa_id = task_qa.id.clone();
    state.task_qa_repo.create(&task_qa).await.unwrap();

    // Add results
    let results =
        QAResults::from_results(task_id.as_str(), vec![QAStepResult::passed("QA1", None)]);
    state
        .task_qa_repo
        .update_results(&qa_id, "agent-1", &results, &[])
        .await
        .unwrap();

    let result = state.task_qa_repo.get_by_task_id(&task_id).await.unwrap();
    let qa_results = result.and_then(|qa| qa.test_results);
    assert!(qa_results.is_some());
    assert!(qa_results.unwrap().is_passed());
}

// ==================== Retry QA Tests ====================

#[tokio::test]
async fn test_retry_qa_resets_results() {
    let state = setup_test_state().await;
    let task_id = TaskId::from_string("task-123".to_string());

    // Create TaskQA with test steps
    let mut task_qa = TaskQA::new(task_id.clone());
    let qa_id = task_qa.id.clone();

    // Add test steps (needed for retry to generate step IDs)
    let steps = QATestSteps::from_steps(vec![QATestStep::new(
        "QA1",
        "AC1",
        "Test step",
        vec![],
        "Expected",
    )]);
    task_qa.qa_test_steps = Some(steps);
    state.task_qa_repo.create(&task_qa).await.unwrap();

    // Add failed results
    let failed_results = QAResults::from_results(
        task_id.as_str(),
        vec![QAStepResult::failed("QA1", "Something went wrong", None)],
    );
    state
        .task_qa_repo
        .update_results(&qa_id, "agent-1", &failed_results, &[])
        .await
        .unwrap();

    // Verify failed
    let before = state
        .task_qa_repo
        .get_by_task_id(&task_id)
        .await
        .unwrap()
        .unwrap();
    assert!(before.is_failed());

    // Retry
    let step_ids = before
        .effective_test_steps()
        .map(|s| s.qa_steps.iter().map(|step| step.id.clone()).collect())
        .unwrap_or_default();
    let fresh_results = QAResults::new(task_id.as_str(), step_ids);
    state
        .task_qa_repo
        .update_results(&qa_id, "", &fresh_results, &[])
        .await
        .unwrap();

    // Verify reset
    let after = state
        .task_qa_repo
        .get_by_task_id(&task_id)
        .await
        .unwrap()
        .unwrap();
    let results = after.test_results.unwrap();
    assert_eq!(results.overall_status, QAOverallStatus::Pending);
}

// ==================== Skip QA Tests ====================

#[tokio::test]
async fn test_skip_qa_marks_as_skipped() {
    let state = setup_test_state().await;
    let task_id = TaskId::from_string("task-123".to_string());

    // Create TaskQA with test steps
    let mut task_qa = TaskQA::new(task_id.clone());
    let qa_id = task_qa.id.clone();

    let steps = QATestSteps::from_steps(vec![QATestStep::new(
        "QA1",
        "AC1",
        "Test step",
        vec![],
        "Expected",
    )]);
    task_qa.qa_test_steps = Some(steps);
    state.task_qa_repo.create(&task_qa).await.unwrap();

    // Skip QA
    let step_ids: Vec<String> = vec!["QA1".to_string()];
    let skipped_results = QAResults::from_results(
        task_id.as_str(),
        step_ids
            .into_iter()
            .map(|id| QAStepResult::skipped(id, Some("QA skipped by user".to_string())))
            .collect(),
    );
    state
        .task_qa_repo
        .update_results(&qa_id, "user-skip", &skipped_results, &[])
        .await
        .unwrap();

    // Verify skipped (which counts as not passed/failed but complete)
    let after = state
        .task_qa_repo
        .get_by_task_id(&task_id)
        .await
        .unwrap()
        .unwrap();
    let results = after.test_results.unwrap();
    assert_eq!(results.steps[0].status, QAStepStatus::Skipped);
}

// ==================== Response Conversion Tests ====================

#[tokio::test]
async fn test_task_qa_response_conversion() {
    let task_id = TaskId::from_string("task-123".to_string());
    let mut task_qa = TaskQA::new(task_id.clone());

    // Add acceptance criteria
    let criteria =
        AcceptanceCriteria::from_criteria(vec![AcceptanceCriterion::visual("AC1", "Visual test")]);
    let steps = QATestSteps::from_steps(vec![QATestStep::new(
        "QA1",
        "AC1",
        "Test step",
        vec!["cmd1".to_string()],
        "Expected",
    )]);

    task_qa.acceptance_criteria = Some(criteria);
    task_qa.qa_test_steps = Some(steps);

    let response = TaskQAResponse::from(task_qa);

    assert_eq!(response.task_id, "task-123");
    assert!(response.acceptance_criteria.is_some());
    assert_eq!(response.acceptance_criteria.unwrap().len(), 1);
    assert!(response.qa_test_steps.is_some());
    assert_eq!(response.qa_test_steps.unwrap().len(), 1);
}

#[tokio::test]
async fn test_qa_results_response_conversion() {
    let results = QAResults::from_results(
        "task-123",
        vec![
            QAStepResult::passed("QA1", Some("ss1.png".to_string())),
            QAStepResult::failed("QA2", "Error", None),
        ],
    );

    let response = QAResultsResponse::from(results);

    assert_eq!(response.task_id, "task-123");
    assert_eq!(response.overall_status, "failed");
    assert_eq!(response.total_steps, 2);
    assert_eq!(response.passed_steps, 1);
    assert_eq!(response.failed_steps, 1);
    assert_eq!(response.steps.len(), 2);
    assert_eq!(response.steps[0].status, "passed");
    assert_eq!(response.steps[1].status, "failed");
}
