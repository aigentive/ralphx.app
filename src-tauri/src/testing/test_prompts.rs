// Test prompts module
// Minimal prompts for cost-effective integration testing
//
// These prompts are ~5-10 tokens vs 500-2000 tokens for real prompts,
// achieving ~98% cost savings in integration tests while still verifying
// agent communication and state machine transitions.

/// Minimal prompt that verifies agent received input and can respond
pub const ECHO_MARKER: &str = "Respond with exactly: TEST_ECHO_OK";

/// Minimal prompt for testing worker agent spawning
pub const WORKER_SPAWN_TEST: &str = "Respond with exactly: WORKER_SPAWNED_SUCCESSFULLY";

/// Minimal prompt for testing QA prep agent
pub const QA_PREP_TEST: &str = "Respond with exactly: QA_PREP_COMPLETE";

/// Minimal prompt for testing QA refiner agent
pub const QA_REFINER_TEST: &str = "Respond with exactly: QA_REFINER_COMPLETE";

/// Minimal prompt for testing QA tester agent
pub const QA_TESTER_TEST: &str = "Respond with exactly: QA_TESTER_COMPLETE";

/// Minimal prompt for testing reviewer agent
pub const REVIEWER_TEST: &str = "Respond with exactly: REVIEW_COMPLETE_APPROVED";

/// Minimal prompt for testing supervisor agent
pub const SUPERVISOR_TEST: &str = "Respond with exactly: SUPERVISOR_CHECK_OK";

/// Expected responses for validation
pub mod expected {
    pub const ECHO_OK: &str = "TEST_ECHO_OK";
    pub const WORKER_OK: &str = "WORKER_SPAWNED_SUCCESSFULLY";
    pub const QA_PREP_OK: &str = "QA_PREP_COMPLETE";
    pub const QA_REFINER_OK: &str = "QA_REFINER_COMPLETE";
    pub const QA_TESTER_OK: &str = "QA_TESTER_COMPLETE";
    pub const REVIEWER_OK: &str = "REVIEW_COMPLETE_APPROVED";
    pub const SUPERVISOR_OK: &str = "SUPERVISOR_CHECK_OK";
}

/// Generate a minimal prompt for iteration testing
///
/// Useful for testing loops where each iteration needs unique verification
pub fn iteration_test_prompt(n: u32) -> String {
    format!("Respond with exactly: ITERATION_{}_COMPLETE", n)
}

/// Generate the expected response for an iteration
pub fn iteration_expected(n: u32) -> String {
    format!("ITERATION_{}_COMPLETE", n)
}

/// Verify that output contains the expected marker
///
/// # Panics
///
/// Panics with a helpful message if the marker is not found
pub fn assert_marker(output: &str, marker: &str) {
    assert!(
        output.contains(marker),
        "Expected output to contain '{}', got: {}...",
        marker,
        &output[..output.len().min(200)]
    );
}

/// Check if output contains the expected marker (non-panicking)
pub fn contains_marker(output: &str, marker: &str) -> bool {
    output.contains(marker)
}

#[cfg(test)]
mod tests {
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
        assert!(SUPERVISOR_TEST.contains(expected::SUPERVISOR_OK));
    }
}
