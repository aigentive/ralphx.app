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
#[path = "test_prompts_tests.rs"]
mod tests;
