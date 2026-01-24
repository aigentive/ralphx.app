// Testing utilities
// Cost-optimized test prompts and helpers

pub mod test_prompts;

// Re-export commonly used items
pub use test_prompts::{
    assert_marker, contains_marker, iteration_expected, iteration_test_prompt, ECHO_MARKER,
    QA_PREP_TEST, QA_REFINER_TEST, QA_TESTER_TEST, REVIEWER_TEST, SUPERVISOR_TEST,
    WORKER_SPAWN_TEST,
};
