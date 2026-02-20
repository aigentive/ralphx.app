use super::*;

// ----------------
// AcceptanceCriteriaType Tests
// ----------------

#[test]
fn test_criteria_type_default() {
    let t: AcceptanceCriteriaType = Default::default();
    assert_eq!(t, AcceptanceCriteriaType::Visual);
}

#[test]
fn test_criteria_type_all() {
    let all = AcceptanceCriteriaType::all();
    assert_eq!(all.len(), 4);
    assert!(all.contains(&AcceptanceCriteriaType::Visual));
    assert!(all.contains(&AcceptanceCriteriaType::Behavior));
    assert!(all.contains(&AcceptanceCriteriaType::Data));
    assert!(all.contains(&AcceptanceCriteriaType::Accessibility));
}

#[test]
fn test_criteria_type_as_str() {
    assert_eq!(AcceptanceCriteriaType::Visual.as_str(), "visual");
    assert_eq!(AcceptanceCriteriaType::Behavior.as_str(), "behavior");
    assert_eq!(AcceptanceCriteriaType::Data.as_str(), "data");
    assert_eq!(
        AcceptanceCriteriaType::Accessibility.as_str(),
        "accessibility"
    );
}

#[test]
fn test_criteria_type_display() {
    assert_eq!(format!("{}", AcceptanceCriteriaType::Visual), "visual");
    assert_eq!(format!("{}", AcceptanceCriteriaType::Behavior), "behavior");
}

#[test]
fn test_criteria_type_serialize() {
    let visual = AcceptanceCriteriaType::Visual;
    let json = serde_json::to_string(&visual).unwrap();
    assert_eq!(json, "\"visual\"");
}

#[test]
fn test_criteria_type_deserialize() {
    let t: AcceptanceCriteriaType = serde_json::from_str("\"behavior\"").unwrap();
    assert_eq!(t, AcceptanceCriteriaType::Behavior);
}

// ----------------
// AcceptanceCriterion Tests
// ----------------

#[test]
fn test_criterion_new() {
    let c = AcceptanceCriterion::new(
        "AC1",
        "User can see the dashboard",
        true,
        AcceptanceCriteriaType::Visual,
    );
    assert_eq!(c.id, "AC1");
    assert_eq!(c.description, "User can see the dashboard");
    assert!(c.testable);
    assert_eq!(c.criteria_type, AcceptanceCriteriaType::Visual);
}

#[test]
fn test_criterion_visual_helper() {
    let c = AcceptanceCriterion::visual("AC1", "Dashboard renders");
    assert_eq!(c.id, "AC1");
    assert!(c.testable);
    assert_eq!(c.criteria_type, AcceptanceCriteriaType::Visual);
}

#[test]
fn test_criterion_behavior_helper() {
    let c = AcceptanceCriterion::behavior("AC2", "Click triggers action");
    assert_eq!(c.id, "AC2");
    assert!(c.testable);
    assert_eq!(c.criteria_type, AcceptanceCriteriaType::Behavior);
}

#[test]
fn test_criterion_serialize() {
    let c = AcceptanceCriterion::visual("AC1", "Test");
    let json = serde_json::to_string(&c).unwrap();
    assert!(json.contains("\"id\":\"AC1\""));
    assert!(json.contains("\"type\":\"visual\""));
    assert!(json.contains("\"testable\":true"));
}

#[test]
fn test_criterion_deserialize() {
    let json = r#"{"id":"AC1","description":"Test","testable":true,"type":"visual"}"#;
    let c: AcceptanceCriterion = serde_json::from_str(json).unwrap();
    assert_eq!(c.id, "AC1");
    assert_eq!(c.description, "Test");
    assert!(c.testable);
    assert_eq!(c.criteria_type, AcceptanceCriteriaType::Visual);
}

// ----------------
// AcceptanceCriteria Tests
// ----------------

#[test]
fn test_criteria_new_empty() {
    let c = AcceptanceCriteria::new();
    assert!(c.is_empty());
    assert_eq!(c.len(), 0);
}

#[test]
fn test_criteria_from_criteria() {
    let criteria = vec![
        AcceptanceCriterion::visual("AC1", "Visual test"),
        AcceptanceCriterion::behavior("AC2", "Behavior test"),
    ];
    let c = AcceptanceCriteria::from_criteria(criteria);
    assert_eq!(c.len(), 2);
}

#[test]
fn test_criteria_add() {
    let mut c = AcceptanceCriteria::new();
    c.add(AcceptanceCriterion::visual("AC1", "Test"));
    assert_eq!(c.len(), 1);
}

#[test]
fn test_criteria_testable() {
    let criteria = vec![
        AcceptanceCriterion::new("AC1", "Test 1", true, AcceptanceCriteriaType::Visual),
        AcceptanceCriterion::new("AC2", "Test 2", false, AcceptanceCriteriaType::Behavior),
        AcceptanceCriterion::new("AC3", "Test 3", true, AcceptanceCriteriaType::Data),
    ];
    let c = AcceptanceCriteria::from_criteria(criteria);
    assert_eq!(c.testable_count(), 2);
    assert_eq!(c.testable().count(), 2);
}

#[test]
fn test_criteria_json_roundtrip() {
    let criteria = vec![
        AcceptanceCriterion::visual("AC1", "User can see the task board"),
        AcceptanceCriterion::behavior("AC2", "Dragging triggers execution"),
    ];
    let c = AcceptanceCriteria::from_criteria(criteria);

    let json = c.to_json().unwrap();
    let parsed = AcceptanceCriteria::from_json(&json).unwrap();

    assert_eq!(c, parsed);
}

#[test]
fn test_criteria_from_prd_format() {
    // Test parsing the exact format from the PRD
    let json = r#"{
        "acceptance_criteria": [
            {
                "id": "AC1",
                "description": "User can see the task board with 7 columns",
                "testable": true,
                "type": "visual"
            },
            {
                "id": "AC2",
                "description": "Dragging a task to 'Planned' column triggers execution",
                "testable": true,
                "type": "behavior"
            }
        ]
    }"#;

    let c = AcceptanceCriteria::from_json(json).unwrap();
    assert_eq!(c.len(), 2);
    assert_eq!(c.acceptance_criteria[0].id, "AC1");
    assert_eq!(
        c.acceptance_criteria[0].criteria_type,
        AcceptanceCriteriaType::Visual
    );
    assert_eq!(c.acceptance_criteria[1].id, "AC2");
    assert_eq!(
        c.acceptance_criteria[1].criteria_type,
        AcceptanceCriteriaType::Behavior
    );

    // ----------------
    // QATestStep Tests
    // ----------------

}

#[test]
fn test_step_new() {
    let step = QATestStep::new(
        "QA1",
        "AC1",
        "Verify board renders",
        vec!["agent-browser open http://localhost:1420".to_string()],
        "Board visible",
    );
    assert_eq!(step.id, "QA1");
    assert_eq!(step.criteria_id, "AC1");
    assert_eq!(step.description, "Verify board renders");
    assert_eq!(step.commands.len(), 1);
    assert_eq!(step.expected, "Board visible");
}

#[test]
fn test_step_has_commands() {
    let step = QATestStep::new("QA1", "AC1", "Test", vec![], "Expected");
    assert!(!step.has_commands());

    let step2 = QATestStep::new("QA2", "AC1", "Test", vec!["cmd".to_string()], "Expected");
    assert!(step2.has_commands());
}

#[test]
fn test_step_command_count() {
    let step = QATestStep::new(
        "QA1",
        "AC1",
        "Test",
        vec!["cmd1".to_string(), "cmd2".to_string(), "cmd3".to_string()],
        "Expected",
    );
    assert_eq!(step.command_count(), 3);
}

#[test]
fn test_step_serialize() {
    let step = QATestStep::new("QA1", "AC1", "Test", vec!["cmd".to_string()], "Expected");
    let json = serde_json::to_string(&step).unwrap();
    assert!(json.contains("\"id\":\"QA1\""));
    assert!(json.contains("\"criteria_id\":\"AC1\""));
    assert!(json.contains("\"commands\":[\"cmd\"]"));
}

#[test]
fn test_step_deserialize() {
    let json = r#"{"id":"QA1","criteria_id":"AC1","description":"Test","commands":["cmd"],"expected":"Result"}"#;
    let step: QATestStep = serde_json::from_str(json).unwrap();
    assert_eq!(step.id, "QA1");
    assert_eq!(step.criteria_id, "AC1");
}

// ----------------
// QATestSteps Tests
// ----------------

#[test]
fn test_steps_new_empty() {
    let s = QATestSteps::new();
    assert!(s.is_empty());
    assert_eq!(s.len(), 0);
}

#[test]
fn test_steps_from_steps() {
    let steps = vec![
        QATestStep::new("QA1", "AC1", "Step 1", vec![], ""),
        QATestStep::new("QA2", "AC2", "Step 2", vec![], ""),
    ];
    let s = QATestSteps::from_steps(steps);
    assert_eq!(s.len(), 2);
}

#[test]
fn test_steps_add() {
    let mut s = QATestSteps::new();
    s.add(QATestStep::new("QA1", "AC1", "Test", vec![], ""));
    assert_eq!(s.len(), 1);
}

#[test]
fn test_steps_for_criterion() {
    let steps = vec![
        QATestStep::new("QA1", "AC1", "Step 1", vec![], ""),
        QATestStep::new("QA2", "AC2", "Step 2", vec![], ""),
        QATestStep::new("QA3", "AC1", "Step 3", vec![], ""),
    ];
    let s = QATestSteps::from_steps(steps);
    assert_eq!(s.for_criterion("AC1").count(), 2);
    assert_eq!(s.for_criterion("AC2").count(), 1);
    assert_eq!(s.for_criterion("AC3").count(), 0);
}

#[test]
fn test_steps_total_commands() {
    let steps = vec![
        QATestStep::new("QA1", "AC1", "Step 1", vec!["c1".into(), "c2".into()], ""),
        QATestStep::new("QA2", "AC2", "Step 2", vec!["c3".into()], ""),
    ];
    let s = QATestSteps::from_steps(steps);
    assert_eq!(s.total_commands(), 3);
}

#[test]
fn test_steps_json_roundtrip() {
    let steps = vec![QATestStep::new(
        "QA1",
        "AC1",
        "Verify board",
        vec!["agent-browser open http://localhost:1420".into()],
        "Board visible",
    )];
    let s = QATestSteps::from_steps(steps);

    let json = s.to_json().unwrap();
    let parsed = QATestSteps::from_json(&json).unwrap();

    assert_eq!(s, parsed);
}

#[test]
fn test_steps_from_prd_format() {
    // Test parsing the exact format from the PRD
    let json = r#"{
        "qa_steps": [
            {
                "id": "QA1",
                "criteria_id": "AC1",
                "description": "Verify task board renders with correct columns",
                "commands": [
                    "agent-browser open http://localhost:1420",
                    "agent-browser wait --load",
                    "agent-browser snapshot -i -c",
                    "agent-browser is visible [data-testid='column-draft']",
                    "agent-browser is visible [data-testid='column-planned']",
                    "agent-browser screenshot screenshots/task-board-columns.png"
                ],
                "expected": "All 7 columns visible"
            }
        ]
    }"#;

    let s = QATestSteps::from_json(json).unwrap();
    assert_eq!(s.len(), 1);
    assert_eq!(s.qa_steps[0].id, "QA1");
    assert_eq!(s.qa_steps[0].criteria_id, "AC1");
    assert_eq!(s.qa_steps[0].commands.len(), 6);
    assert_eq!(s.qa_steps[0].expected, "All 7 columns visible");
}

