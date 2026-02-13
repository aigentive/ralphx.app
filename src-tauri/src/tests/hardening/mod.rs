// Agent Execution Hardening Test Suite
//
// Comprehensive TDD test suite for all failure scenarios in the agent execution pipeline.
// Tests are organized by subsystem: git isolation, agent spawning, runtime, completion,
// reconciliation, cleanup, concurrency, and error visibility.
//
// Naming convention: test_{scenario_id}_{short_description}
// Covered scenarios (should pass) verify existing safety measures.
// Gap scenarios (expected to fail) serve as executable spec for fixes needed.

mod helpers;
mod git_isolation_tests;
mod agent_spawning_tests;
mod agent_runtime_tests;
mod completion_transition_tests;
mod reconciliation_tests;
mod cleanup_tests;
mod concurrency_tests;
mod error_visibility_tests;
