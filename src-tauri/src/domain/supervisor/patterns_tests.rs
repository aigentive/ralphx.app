use super::*;

#[test]
fn test_pattern_display() {
    assert_eq!(Pattern::InfiniteLoop.to_string(), "Infinite loop detected");
    assert_eq!(Pattern::Stuck.to_string(), "Agent appears stuck");
}

#[test]
fn test_detection_result_new() {
    let result = DetectionResult::new(Pattern::InfiniteLoop, 85, "Test", 3);
    assert_eq!(result.pattern, Pattern::InfiniteLoop);
    assert_eq!(result.confidence, 85);
    assert!(result.is_high_confidence());
}

#[test]
fn test_detection_result_confidence_capped() {
    let result = DetectionResult::new(Pattern::Stuck, 120, "Test", 1);
    assert_eq!(result.confidence, 100);
}

#[test]
fn test_tool_call_window_new() {
    let window = ToolCallWindow::new(10, 3);
    assert!(window.is_empty());
    assert_eq!(window.len(), 0);
}

#[test]
fn test_tool_call_window_push() {
    let mut window = ToolCallWindow::new(3, 2);
    window.push(ToolCallInfo::new("Write", "{}"));
    window.push(ToolCallInfo::new("Read", "{}"));
    window.push(ToolCallInfo::new("Edit", "{}"));
    assert_eq!(window.len(), 3);

    // Should evict oldest
    window.push(ToolCallInfo::new("Bash", "{}"));
    assert_eq!(window.len(), 3);
}

#[test]
fn test_tool_call_window_clear() {
    let mut window = ToolCallWindow::new(10, 3);
    window.push(ToolCallInfo::new("Write", "{}"));
    window.push(ToolCallInfo::new("Read", "{}"));
    window.clear();
    assert!(window.is_empty());
}

#[test]
fn test_detect_loop_not_enough_calls() {
    let window = ToolCallWindow::new(10, 3);
    assert!(window.detect_loop().is_none());
}

#[test]
fn test_detect_loop_no_repetition() {
    let mut window = ToolCallWindow::new(10, 3);
    window.push(ToolCallInfo::new("Write", r#"{"path": "a.txt"}"#));
    window.push(ToolCallInfo::new("Read", r#"{"path": "b.txt"}"#));
    window.push(ToolCallInfo::new("Edit", r#"{"path": "c.txt"}"#));
    assert!(window.detect_loop().is_none());
}

#[test]
fn test_detect_loop_found() {
    let mut window = ToolCallWindow::new(10, 3);
    window.push(ToolCallInfo::new("Write", r#"{"path": "test.txt"}"#));
    window.push(ToolCallInfo::new("Write", r#"{"path": "test.txt"}"#));
    window.push(ToolCallInfo::new("Write", r#"{"path": "test.txt"}"#));

    let result = window.detect_loop();
    assert!(result.is_some());
    let result = result.expect("Expected Some(DetectionResult) for infinite loop");
    assert_eq!(result.pattern, Pattern::InfiniteLoop);
    assert_eq!(result.occurrences, 3);
}

#[test]
fn test_detect_stuck_not_stuck() {
    let window = ToolCallWindow::new(10, 3);
    assert!(window.detect_stuck(5).is_none());
}

#[test]
fn test_detect_stuck_found() {
    let mut window = ToolCallWindow::new(10, 3);
    for _ in 0..5 {
        window.record_no_progress();
    }

    let result = window.detect_stuck(5);
    assert!(result.is_some());
    let result = result.expect("Expected Some(DetectionResult) for stuck detection");
    assert_eq!(result.pattern, Pattern::Stuck);
    assert_eq!(result.occurrences, 5);
}

#[test]
fn test_detect_stuck_reset_on_progress() {
    let mut window = ToolCallWindow::new(10, 3);
    for _ in 0..3 {
        window.record_no_progress();
    }
    window.record_progress();

    assert!(window.detect_stuck(5).is_none());
}

#[test]
fn test_detect_repeating_error() {
    let mut window = ToolCallWindow::new(10, 3);
    window.push(ToolCallInfo::failed("Write", "{}", "Permission denied"));
    window.push(ToolCallInfo::new("Read", "{}"));
    window.push(ToolCallInfo::failed("Write", "{}", "Permission denied"));

    let result = window.detect_repeating_error();
    assert!(result.is_some());
    let result = result.expect("Expected Some(DetectionResult) for repeating error");
    assert_eq!(result.pattern, Pattern::RepeatingError);
    assert_eq!(result.occurrences, 2);
}

#[test]
fn test_detect_repeating_error_no_errors() {
    let mut window = ToolCallWindow::new(10, 3);
    window.push(ToolCallInfo::new("Write", "{}"));
    window.push(ToolCallInfo::new("Read", "{}"));

    assert!(window.detect_repeating_error().is_none());
}

#[test]
fn test_detect_all() {
    let mut window = ToolCallWindow::new(10, 3);
    window.push(ToolCallInfo::new("Write", r#"{"path": "test.txt"}"#));
    window.push(ToolCallInfo::new("Write", r#"{"path": "test.txt"}"#));
    window.push(ToolCallInfo::new("Write", r#"{"path": "test.txt"}"#));
    for _ in 0..5 {
        window.record_no_progress();
    }

    let results = window.detect_all(5);
    assert_eq!(results.len(), 2); // Loop and Stuck
}

#[test]
fn test_detect_poor_task_definition_none() {
    assert!(detect_poor_task_definition(2).is_none());
}

#[test]
fn test_detect_poor_task_definition_found() {
    let result = detect_poor_task_definition(3);
    assert!(result.is_some());
    if let Some(result) = result {
        assert_eq!(result.pattern, Pattern::PoorTaskDefinition);
    }
}

#[test]
fn test_pattern_serialize() {
    let json = serde_json::to_string(&Pattern::InfiniteLoop)
        .expect("Pattern serialization should not fail");
    assert_eq!(json, "\"infinite_loop\"");
}

#[test]
fn test_detection_result_serialize() {
    let result = DetectionResult::new(Pattern::Stuck, 80, "Test", 5);
    let json =
        serde_json::to_string(&result).expect("DetectionResult serialization should not fail");
    assert!(json.contains("\"pattern\":\"stuck\""));
    assert!(json.contains("\"confidence\":80"));
}
