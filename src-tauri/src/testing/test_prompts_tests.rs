use super::*;

#[test]
fn test_echo_marker_defined() {
    assert!(!ECHO_MARKER.is_empty());
    assert!(ECHO_MARKER.contains("TEST_ECHO_OK"));
}

#[test]
fn test_worker_spawn_test_defined() {
    assert!(!WORKER_SPAWN_TEST.is_empty());
    assert!(WORKER_SPAWN_TEST.contains("WORKER_SPAWNED_SUCCESSFULLY"));
}

#[test]
fn test_qa_prep_test_defined() {
    assert!(!QA_PREP_TEST.is_empty());
    assert!(QA_PREP_TEST.contains("QA_PREP_COMPLETE"));
}

#[test]
fn test_reviewer_test_defined() {
    assert!(!REVIEWER_TEST.is_empty());
    assert!(REVIEWER_TEST.contains("REVIEW_COMPLETE_APPROVED"));
}

#[test]
fn test_iteration_test_prompt() {
    let prompt = iteration_test_prompt(1);
    assert!(prompt.contains("ITERATION_1_COMPLETE"));

    let prompt = iteration_test_prompt(42);
    assert!(prompt.contains("ITERATION_42_COMPLETE"));
}

#[test]
fn test_iteration_expected() {
    assert_eq!(iteration_expected(1), "ITERATION_1_COMPLETE");
    assert_eq!(iteration_expected(99), "ITERATION_99_COMPLETE");
}

#[test]
fn test_assert_marker_passes() {
    let output = "Some text TEST_ECHO_OK more text";
    assert_marker(output, "TEST_ECHO_OK");
}

#[test]
#[should_panic(expected = "Expected output to contain")]
fn test_assert_marker_fails() {
    let output = "Some text without the marker";
    assert_marker(output, "TEST_ECHO_OK");
}

#[test]
fn test_contains_marker_true() {
    let output = "Response: WORKER_SPAWNED_SUCCESSFULLY";
    assert!(contains_marker(output, "WORKER_SPAWNED_SUCCESSFULLY"));
}

#[test]
fn test_contains_marker_false() {
    let output = "Some other response";
    assert!(!contains_marker(output, "TEST_ECHO_OK"));
}

#[test]
fn test_expected_markers_match_prompts() {
    assert!(ECHO_MARKER.contains(expected::ECHO_OK));
    assert!(WORKER_SPAWN_TEST.contains(expected::WORKER_OK));
    assert!(QA_PREP_TEST.contains(expected::QA_PREP_OK));
    assert!(QA_REFINER_TEST.contains(expected::QA_REFINER_OK));
    assert!(QA_TESTER_TEST.contains(expected::QA_TESTER_OK));
    assert!(REVIEWER_TEST.contains(expected::REVIEWER_OK));
}
