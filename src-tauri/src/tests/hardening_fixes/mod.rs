// Agent Execution Hardening — Phase 2: Fix Specification Tests
//
// These tests assert the CORRECT behavior after fixes are applied.
// They complement the Phase 1 hardening tests (which document the broken behavior).
//
// Layer 1 (hardening/): Gap discovery — documents what IS (including broken behavior)
// Layer 2 (hardening_fixes/): Fix specifications — asserts what SHOULD BE
//
// Each test file corresponds to a specific gap fix (B1, H2, B2, E7, etc.)

mod a7_git_mode_validation_fix_test;
mod b1_send_message_fix_test;
mod b2_spawn_blocked_fix_test;
mod b5_spawn_dedup_fix_test;
mod c5_wall_clock_timeout_fix_test;
mod d4_transition_retry_fix_test;
mod e7_retry_limit_fix_test;
mod f4_worktree_error_fix_test;
mod h2_on_enter_visibility_fix_test;
mod helpers;
