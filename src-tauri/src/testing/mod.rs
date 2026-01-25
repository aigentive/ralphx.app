// Testing utilities
// Cost-optimized test prompts and helpers

pub mod test_prompts;

// Re-export commonly used items
pub use test_prompts::{
    assert_marker, contains_marker, iteration_expected, iteration_test_prompt, ECHO_MARKER,
    QA_PREP_TEST, QA_REFINER_TEST, QA_TESTER_TEST, REVIEWER_TEST, SUPERVISOR_TEST,
    WORKER_SPAWN_TEST,
};

use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};

/// Create a mock Tauri app for testing purposes
/// Returns the mock AppHandle that can be used to create AppState in tests
pub fn create_mock_app() -> tauri::App<MockRuntime> {
    mock_builder()
        .build(mock_context(noop_assets()))
        .expect("Failed to create mock Tauri app for testing")
}

/// Create a mock AppHandle for testing
/// This is a convenience function that creates a mock app and returns its handle
pub fn create_mock_app_handle() -> tauri::AppHandle<MockRuntime> {
    create_mock_app().handle().clone()
}
