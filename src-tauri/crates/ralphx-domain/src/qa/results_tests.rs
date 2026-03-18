use super::*;

// ----------------
// QAStepStatus Tests
// ----------------

#[test]
fn test_step_status_default() {
    let s: QAStepStatus = Default::default();
    assert_eq!(s, QAStepStatus::Pending);
}

#[test]
fn test_step_status_all() {
    let all = QAStepStatus::all();
    assert_eq!(all.len(), 5);
}

#[test]
fn test_step_status_as_str() {
    assert_eq!(QAStepStatus::Pending.as_str(), "pending");
    assert_eq!(QAStepStatus::Running.as_str(), "running");
    assert_eq!(QAStepStatus::Passed.as_str(), "passed");
    assert_eq!(QAStepStatus::Failed.as_str(), "failed");
    assert_eq!(QAStepStatus::Skipped.as_str(), "skipped");
}

#[test]
fn test_step_status_is_terminal() {
    assert!(!QAStepStatus::Pending.is_terminal());
    assert!(!QAStepStatus::Running.is_terminal());
    assert!(QAStepStatus::Passed.is_terminal());
    assert!(QAStepStatus::Failed.is_terminal());
    assert!(QAStepStatus::Skipped.is_terminal());
}

#[test]
fn test_step_status_serialize() {
    let s = QAStepStatus::Passed;
    let json = serde_json::to_string(&s).unwrap();
    assert_eq!(json, "\"passed\"");
}

#[test]
fn test_step_status_deserialize() {
    let s: QAStepStatus = serde_json::from_str("\"failed\"").unwrap();
    assert_eq!(s, QAStepStatus::Failed);
}

// ----------------
// QAOverallStatus Tests
// ----------------

#[test]
fn test_overall_status_default() {
    let s: QAOverallStatus = Default::default();
    assert_eq!(s, QAOverallStatus::Pending);
}

#[test]
fn test_overall_status_is_complete() {
    assert!(!QAOverallStatus::Pending.is_complete());
    assert!(!QAOverallStatus::Running.is_complete());
    assert!(QAOverallStatus::Passed.is_complete());
    assert!(QAOverallStatus::Failed.is_complete());
}

// ----------------
// QAStepResult Tests
// ----------------

#[test]
fn test_step_result_pending() {
    let r = QAStepResult::pending("QA1");
    assert_eq!(r.step_id, "QA1");
    assert_eq!(r.status, QAStepStatus::Pending);
    assert!(r.screenshot.is_none());
    assert!(r.error.is_none());
}

#[test]
fn test_step_result_passed() {
    let r = QAStepResult::passed("QA1", Some("screenshot.png".into()));
    assert_eq!(r.status, QAStepStatus::Passed);
    assert_eq!(r.screenshot, Some("screenshot.png".into()));
}

#[test]
fn test_step_result_failed() {
    let r = QAStepResult::failed("QA1", "Element not found", None);
    assert_eq!(r.status, QAStepStatus::Failed);
    assert_eq!(r.error, Some("Element not found".into()));
}

#[test]
fn test_step_result_failed_comparison() {
    let r = QAStepResult::failed_comparison("QA1", "7 columns", "5 columns", None);
    assert_eq!(r.status, QAStepStatus::Failed);
    assert_eq!(r.expected, Some("7 columns".into()));
    assert_eq!(r.actual, Some("5 columns".into()));
}

#[test]
fn test_step_result_skipped() {
    let r = QAStepResult::skipped("QA1", Some("Previous step failed".into()));
    assert_eq!(r.status, QAStepStatus::Skipped);
    assert_eq!(r.error, Some("Previous step failed".into()));
}

#[test]
fn test_step_result_mark_running() {
    let mut r = QAStepResult::pending("QA1");
    r.mark_running();
    assert_eq!(r.status, QAStepStatus::Running);
}

#[test]
fn test_step_result_mark_passed() {
    let mut r = QAStepResult::pending("QA1");
    r.mark_passed(Some("ss.png".into()));
    assert_eq!(r.status, QAStepStatus::Passed);
    assert_eq!(r.screenshot, Some("ss.png".into()));
}

#[test]
fn test_step_result_mark_failed() {
    let mut r = QAStepResult::pending("QA1");
    r.mark_failed("Error".into(), None);
    assert_eq!(r.status, QAStepStatus::Failed);
    assert_eq!(r.error, Some("Error".into()));
}

#[test]
fn test_step_result_serialize() {
    let r = QAStepResult::passed("QA1", Some("ss.png".into()));
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains("\"step_id\":\"QA1\""));
    assert!(json.contains("\"status\":\"passed\""));
    assert!(json.contains("\"screenshot\":\"ss.png\""));
    // Nulls should be skipped
    assert!(!json.contains("\"error\""));
}

#[test]
fn test_step_result_deserialize() {
    let json = r#"{"step_id":"QA1","status":"passed","screenshot":"ss.png"}"#;
    let r: QAStepResult = serde_json::from_str(json).unwrap();
    assert_eq!(r.step_id, "QA1");
    assert_eq!(r.status, QAStepStatus::Passed);
    assert_eq!(r.screenshot, Some("ss.png".into()));
    assert!(r.error.is_none());
}

// ----------------
// QAResultsTotals Tests
// ----------------

#[test]
fn test_totals_from_results() {
    let results = vec![
        QAStepResult::passed("QA1", None),
        QAStepResult::passed("QA2", None),
        QAStepResult::failed("QA3", "Error", None),
        QAStepResult::skipped("QA4", None),
    ];
    let totals = QAResultsTotals::from_results(&results);
    assert_eq!(totals.total_steps, 4);
    assert_eq!(totals.passed_steps, 2);
    assert_eq!(totals.failed_steps, 1);
    assert_eq!(totals.skipped_steps, 1);
}

#[test]
fn test_totals_pass_rate() {
    let results = vec![
        QAStepResult::passed("QA1", None),
        QAStepResult::passed("QA2", None),
        QAStepResult::failed("QA3", "Error", None),
        QAStepResult::passed("QA4", None),
    ];
    let totals = QAResultsTotals::from_results(&results);
    assert_eq!(totals.pass_rate(), 75.0);
}

#[test]
fn test_totals_pass_rate_empty() {
    let totals = QAResultsTotals::default();
    assert_eq!(totals.pass_rate(), 0.0);
}

#[test]
fn test_totals_all_passed() {
    let results = vec![
        QAStepResult::passed("QA1", None),
        QAStepResult::passed("QA2", None),
    ];
    let totals = QAResultsTotals::from_results(&results);
    assert!(totals.all_passed());
}

#[test]
fn test_totals_has_failures() {
    let results = vec![
        QAStepResult::passed("QA1", None),
        QAStepResult::failed("QA2", "Error", None),
    ];
    let totals = QAResultsTotals::from_results(&results);
    assert!(totals.has_failures());
}

// ----------------
// QAResults Tests
// ----------------

#[test]
fn test_results_new() {
    let r = QAResults::new("task-123", vec!["QA1".into(), "QA2".into()]);
    assert_eq!(r.task_id, "task-123");
    assert_eq!(r.overall_status, QAOverallStatus::Pending);
    assert_eq!(r.total_steps, 2);
    assert_eq!(r.passed_steps, 0);
    assert_eq!(r.failed_steps, 0);
    assert_eq!(r.steps.len(), 2);
}

#[test]
fn test_results_from_results_all_passed() {
    let steps = vec![
        QAStepResult::passed("QA1", None),
        QAStepResult::passed("QA2", None),
    ];
    let r = QAResults::from_results("task-123", steps);
    assert_eq!(r.overall_status, QAOverallStatus::Passed);
    assert_eq!(r.passed_steps, 2);
}

#[test]
fn test_results_from_results_with_failures() {
    let steps = vec![
        QAStepResult::passed("QA1", None),
        QAStepResult::failed("QA2", "Error", None),
    ];
    let r = QAResults::from_results("task-123", steps);
    assert_eq!(r.overall_status, QAOverallStatus::Failed);
    assert_eq!(r.passed_steps, 1);
    assert_eq!(r.failed_steps, 1);
}

#[test]
fn test_results_update_step() {
    let mut r = QAResults::new("task-123", vec!["QA1".into(), "QA2".into()]);
    r.update_step("QA1", QAStepStatus::Passed, None, Some("ss.png".into()));

    let step = r.get_step("QA1").unwrap();
    assert_eq!(step.status, QAStepStatus::Passed);
    assert_eq!(step.screenshot, Some("ss.png".into()));
    assert_eq!(r.passed_steps, 1);
}

#[test]
fn test_results_get_step() {
    let r = QAResults::new("task-123", vec!["QA1".into(), "QA2".into()]);
    assert!(r.get_step("QA1").is_some());
    assert!(r.get_step("QA3").is_none());
}

#[test]
fn test_results_failed_steps_iter() {
    let steps = vec![
        QAStepResult::passed("QA1", None),
        QAStepResult::failed("QA2", "Error 1", None),
        QAStepResult::failed("QA3", "Error 2", None),
    ];
    let r = QAResults::from_results("task-123", steps);
    let failed: Vec<_> = r.failed_steps_iter().collect();
    assert_eq!(failed.len(), 2);
}

#[test]
fn test_results_screenshots() {
    let steps = vec![
        QAStepResult::passed("QA1", Some("ss1.png".into())),
        QAStepResult::passed("QA2", None),
        QAStepResult::failed("QA3", "Error", Some("ss3.png".into())),
    ];
    let r = QAResults::from_results("task-123", steps);
    let screenshots = r.screenshots();
    assert_eq!(screenshots, vec!["ss1.png", "ss3.png"]);
}

#[test]
fn test_results_json_roundtrip() {
    let steps = vec![
        QAStepResult::passed("QA1", Some("ss.png".into())),
        QAStepResult::failed("QA2", "Element not found", None),
    ];
    let r = QAResults::from_results("task-123", steps);

    let json = r.to_json().unwrap();
    let parsed = QAResults::from_json(&json).unwrap();

    assert_eq!(r, parsed);
}

#[test]
fn test_results_from_prd_format() {
    // Test parsing the exact format from the PRD
    let json = r#"{
        "task_id": "task-123",
        "overall_status": "passed",
        "total_steps": 5,
        "passed_steps": 5,
        "failed_steps": 0,
        "steps": [
            {
                "step_id": "QA1",
                "status": "passed",
                "screenshot": "screenshots/qa1-result.png"
            }
        ]
    }"#;

    let r = QAResults::from_json(json).unwrap();
    assert_eq!(r.task_id, "task-123");
    assert_eq!(r.overall_status, QAOverallStatus::Passed);
    assert_eq!(r.total_steps, 5);
    assert_eq!(r.steps[0].step_id, "QA1");
    assert_eq!(
        r.steps[0].screenshot,
        Some("screenshots/qa1-result.png".into())
    );
}

#[test]
fn test_wrapper_from_prd_format() {
    // Test the wrapper format with qa_results key
    let json = r#"{
        "qa_results": {
            "task_id": "task-123",
            "overall_status": "passed",
            "total_steps": 5,
            "passed_steps": 5,
            "failed_steps": 0,
            "steps": [
                {
                    "step_id": "QA1",
                    "status": "passed",
                    "screenshot": "screenshots/qa1-result.png",
                    "actual": null,
                    "expected": null,
                    "error": null
                }
            ]
        }
    }"#;

    let w = QAResultsWrapper::from_json(json).unwrap();
    assert_eq!(w.qa_results.task_id, "task-123");
    assert_eq!(w.qa_results.overall_status, QAOverallStatus::Passed);
}

#[test]
fn test_results_is_complete() {
    let r_pending = QAResults::new("task-1", vec!["QA1".into()]);
    assert!(!r_pending.is_complete());

    let r_passed = QAResults::from_results("task-2", vec![QAStepResult::passed("QA1", None)]);
    assert!(r_passed.is_complete());
    assert!(r_passed.is_passed());

    let r_failed =
        QAResults::from_results("task-3", vec![QAStepResult::failed("QA1", "Error", None)]);
    assert!(r_failed.is_complete());
    assert!(r_failed.is_failed());
}

#[test]
fn test_results_recalculate() {
    let mut r = QAResults::new("task-123", vec!["QA1".into(), "QA2".into()]);

    // Manually update steps
    r.steps[0].status = QAStepStatus::Passed;
    r.steps[1].status = QAStepStatus::Passed;

    // Recalculate
    r.recalculate();

    assert_eq!(r.passed_steps, 2);
    assert_eq!(r.overall_status, QAOverallStatus::Passed);
}
