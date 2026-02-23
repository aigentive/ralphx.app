// Execution Plan Cascade Hardening Tests
//
// Tests covering three fixes from the execution-plan-cascade session:
//
// Fix 1 (skipped): resolve_task_base_branch with plan_branch_repo returns plan branch.
//   Already covered by `resolve_task_base_branch_returns_feature_branch_when_branch_exists`
//   in transition_handler/tests/merge_helpers_branch_resolution.rs.
//
// Fix 2: Per-task scheduling guard in ExecutionState::try_start_scheduling prevents
//   double-scheduling the same task under concurrent scheduler invocations.
//
// Fix 3: Registry cleanup (stop) before reconciliation auto-recovery re-spawn prevents
//   infinite thrash loop when try_register fails on an occupied slot.

use crate::commands::ExecutionState;
use crate::domain::services::{MemoryRunningAgentRegistry, RunningAgentKey, RunningAgentRegistry};

// ============================================================================
// Fix 2: Per-task scheduling guard (ExecutionState::try_start_scheduling)
// ============================================================================

#[test]
fn test_per_task_scheduling_guard_prevents_double_scheduling() {
    // Guard prevents two concurrent scheduler invocations from both scheduling
    // the same task (TOCTOU race on async on_enter_Executing).
    let state = ExecutionState::new();

    // First caller acquires the slot
    assert!(
        state.try_start_scheduling("task-1"),
        "First call should succeed"
    );

    // Second concurrent caller for same task is rejected
    assert!(
        !state.try_start_scheduling("task-1"),
        "Second call for same task should be rejected (already in flight)"
    );

    // A different task is not affected
    assert!(
        state.try_start_scheduling("task-2"),
        "A different task should not be blocked by task-1's guard"
    );

    // After finish, the slot is released
    state.finish_scheduling("task-1");
    assert!(
        state.try_start_scheduling("task-1"),
        "After finish_scheduling, the slot should be available again"
    );
}

#[test]
fn test_scheduling_guard_multiple_tasks_independent() {
    // Guards for distinct tasks are independent — acquiring one does not block others.
    let state = ExecutionState::new();

    assert!(state.try_start_scheduling("task-a"));
    assert!(state.try_start_scheduling("task-b"));
    assert!(state.try_start_scheduling("task-c"));

    // Each is individually blocked while in flight
    assert!(!state.try_start_scheduling("task-a"));
    assert!(!state.try_start_scheduling("task-b"));
    assert!(!state.try_start_scheduling("task-c"));

    // Release one; only that slot becomes free
    state.finish_scheduling("task-a");
    assert!(
        state.try_start_scheduling("task-a"),
        "task-a slot should be free after finish"
    );
    assert!(
        !state.try_start_scheduling("task-b"),
        "task-b slot should still be held"
    );
    assert!(
        !state.try_start_scheduling("task-c"),
        "task-c slot should still be held"
    );
}

#[test]
fn test_scheduling_guard_finish_on_nonexistent_is_noop() {
    // finish_scheduling for an ID that was never started must not panic.
    let state = ExecutionState::new();
    state.finish_scheduling("never-started"); // should not panic

    // And a subsequent try_start should succeed (not poisoned)
    assert!(state.try_start_scheduling("never-started"));
}

// ============================================================================
// Fix 3: Registry cleanup before reconciliation re-spawn prevents thrashing
// ============================================================================

#[tokio::test]
async fn test_registry_cleanup_before_respawn_prevents_thrash() {
    // Without the fix: apply_recovery_decision called execute_entry_actions while a
    // stale registry entry for the same task still existed. try_register then returned
    // Err (slot occupied), the spawn was skipped, and the reconciler retried forever.
    //
    // With the fix: apply_recovery_decision calls registry.stop() before spawning,
    // so try_register finds an empty slot and succeeds.

    let registry = MemoryRunningAgentRegistry::new();
    let key = RunningAgentKey::new("task_execution", "task-stale-123");

    // Simulate a stale entry left over from a crashed agent
    registry
        .register(
            key.clone(),
            99999, // stale PID — process no longer exists
            "conv-old".to_string(),
            "run-old".to_string(),
            None,
            None,
        )
        .await;

    // Stale entry is present
    assert!(
        registry.is_running(&key).await,
        "Stale entry should be present before cleanup"
    );

    // Without cleanup: try_register returns Err because the slot is occupied
    let result_without_cleanup = registry
        .try_register(
            key.clone(),
            "conv-new".to_string(),
            "run-new".to_string(),
        )
        .await;
    assert!(
        result_without_cleanup.is_err(),
        "try_register must fail when stale entry is present (no cleanup)"
    );

    // The fix: clear the stale entry first (stop removes it from the registry)
    let stop_result = registry.stop(&key).await;
    assert!(
        stop_result.is_ok(),
        "stop() should succeed even for a stale entry"
    );

    // Entry is now cleared
    assert!(
        !registry.is_running(&key).await,
        "Registry entry should be absent after stop()"
    );

    // After cleanup: try_register succeeds and the new agent can be spawned
    let result_after_cleanup = registry
        .try_register(
            key.clone(),
            "conv-new".to_string(),
            "run-new".to_string(),
        )
        .await;
    assert!(
        result_after_cleanup.is_ok(),
        "try_register should succeed after the stale entry is cleaned up"
    );
}

#[tokio::test]
async fn test_registry_cleanup_idempotent_when_no_entry() {
    // stop() on a key with no entry should return Ok(None) without panicking.
    // Ensures the cleanup call in apply_recovery_decision is safe even if the
    // entry was already removed by a concurrent prune.
    let registry = MemoryRunningAgentRegistry::new();
    let key = RunningAgentKey::new("task_execution", "task-no-entry");

    assert!(
        !registry.is_running(&key).await,
        "No entry should exist initially"
    );

    // stop on absent key should not panic and should return Ok
    let result = registry.stop(&key).await;
    assert!(
        result.is_ok(),
        "stop() on absent entry should return Ok (idempotent)"
    );
    let stopped_info = result.unwrap();
    assert!(
        stopped_info.is_none(),
        "stop() on absent entry should return None (no entry was removed)"
    );
}
