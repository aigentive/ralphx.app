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

// Tests for update_plan_from_main: bringing plan branches up-to-date before task merge
mod plan_update_from_main;

// Tests extracted from inline #[cfg(test)] blocks in production modules
mod merge_helpers_inline;
mod merge_helpers_inline_2;
mod merge_validation_inline;
mod metadata_builder_tests;
mod metadata_builder_tests_2;
mod on_enter_states_tests;
mod symlink_idempotent;

// Tests for has_prior_validation_failure guard (validation metadata checks)
mod merge_helpers_validation_guard;

// Tests for update_source_from_target: bringing feature branches up-to-date before merge
mod source_update_from_target;

// Tests for merge progress event emission and phase ordering
mod progress_events;

// Regression tests: on_enter_dispatch coverage for all Merging state entry paths
mod merge_dispatch_agent_spawn;

// Orchestration chain integration tests: real git + real DB + MockChatService
// Verifies merger agent is spawned: B2 (merge conflict) and C1 (AutoFix validation failure)
mod orchestration_chain_tests;

// Tests for spawn failure recovery: on_enter(Merging) + failing chat service
// Covers Fix 2 (commit 189e8eaf): AttemptFailed events + retry budget exhaustion
mod spawn_failure_recovery_tests;

// Tests for AgentAlreadyRunning guard in on_enter(Merging) and on_enter(Reviewing)
// RC#2: double on_enter returns no-op instead of recording spawn failure
mod on_enter_already_running_tests;

// RC#6: plan_update_conflict must create merge-* worktree before spawning merger agent
mod plan_update_conflict_worktree;
