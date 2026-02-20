// transition_handler test modules
//
// Organized by source module and concern area.
// Each file contains tests for a specific subsystem.
// Shared helpers: use super::helpers::*;

mod helpers;

// Tests extracted from side_effects.rs embedded #[cfg(test)] block
mod commit_message;
mod merge_arbitration;
mod merge_helpers_branch_resolution;
mod merge_helpers_metadata;
mod merge_helpers_workflow;
mod merge_validation_tests;

// Tests extracted from tests.rs (integration tests with mock services)
mod execution_state;
mod integration_branch_discovery;
mod merge_retry;
mod merge_workflow;
mod metadata_skip_guard;
mod transitions_agents;
mod transitions_basic;

// Tests for merge-hang fixes (step 0, deadline, timeouts, config)
mod merge_cleanup;

// Test quality overhaul: adversarial mocks, ordering assertions, outcome coverage
mod test_quality_overhaul;

// Real git repo integration tests: merge strategy dispatch with actual git operations
mod real_git_integration;

// Tests extracted from inline #[cfg(test)] blocks in production modules
mod merge_helpers_inline;
mod merge_helpers_inline_2;
mod merge_validation_inline;
mod metadata_builder_tests;
mod metadata_builder_tests_2;
mod on_enter_states_tests;
