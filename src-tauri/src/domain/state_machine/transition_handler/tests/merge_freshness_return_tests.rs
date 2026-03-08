// Unit tests for freshness return routing logic.
//
// Tests the metadata decision logic for handle_freshness_return_routing:
//   1. valid_executing_origin: branch_freshness_conflict=true, freshness_origin_state="executing" → Ready
//   2. valid_re_executing_origin: freshness_origin_state="re_executing" → Ready
//   3. valid_reviewing_origin: freshness_origin_state="reviewing" → PendingReview
//   4. unknown_origin: freshness_origin_state="unknown_state" → Ready (fallback)
//   5. corrupted_no_origin: branch_freshness_conflict=true but freshness_origin_state=None → Ready
//   6. no_freshness_flag: no branch_freshness_conflict → normal merge flow (no early return)
//
// Also tests the metadata clearing logic (FreshnessMetadata::clear_from) as used
// during the return routing.

use crate::domain::entities::InternalStatus;
use crate::domain::state_machine::transition_handler::freshness::FreshnessMetadata;
use serde_json::json;

/// Helper: derive target InternalStatus from freshness_origin_state, matching
/// the logic in handle_freshness_return_routing.
fn routing_target_for(origin: Option<&str>) -> InternalStatus {
    match origin {
        Some("executing") | Some("re_executing") => InternalStatus::Ready,
        Some("reviewing") => InternalStatus::PendingReview,
        _ => InternalStatus::Ready,
    }
}

// ==================
// Routing decision tests (match logic parity with handle_freshness_return_routing)
// ==================

#[test]
fn test_routing_executing_origin_routes_to_ready() {
    let target = routing_target_for(Some("executing"));
    assert_eq!(
        target,
        InternalStatus::Ready,
        "executing origin must route to Ready"
    );
}

#[test]
fn test_routing_re_executing_origin_routes_to_ready() {
    let target = routing_target_for(Some("re_executing"));
    assert_eq!(
        target,
        InternalStatus::Ready,
        "re_executing origin must route to Ready"
    );
}

#[test]
fn test_routing_reviewing_origin_routes_to_pending_review() {
    let target = routing_target_for(Some("reviewing"));
    assert_eq!(
        target,
        InternalStatus::PendingReview,
        "reviewing origin must route to PendingReview"
    );
}

#[test]
fn test_routing_unknown_origin_routes_to_ready() {
    // Unknown state falls through to Ready as a safe default
    let target = routing_target_for(Some("unknown_state"));
    assert_eq!(
        target,
        InternalStatus::Ready,
        "unknown origin must fall back to Ready"
    );
}

#[test]
fn test_routing_none_origin_routes_to_ready() {
    // Absent freshness_origin_state (corrupted metadata) defaults to Ready
    let target = routing_target_for(None);
    assert_eq!(
        target,
        InternalStatus::Ready,
        "absent freshness_origin_state must fall back to Ready"
    );
}

// ==================
// Metadata flag detection tests
// ==================

#[test]
fn test_freshness_flag_true_triggers_early_return() {
    // Simulates the check: if freshness.branch_freshness_conflict { ... }
    let meta = json!({
        "branch_freshness_conflict": true,
        "freshness_origin_state": "executing",
        "freshness_conflict_count": 1,
        "plan_update_conflict": true,
        "source_update_conflict": false,
    });
    let freshness = FreshnessMetadata::from_task_metadata(&meta);
    assert!(
        freshness.branch_freshness_conflict,
        "branch_freshness_conflict=true must trigger freshness routing"
    );
}

#[test]
fn test_freshness_flag_false_skips_routing() {
    // Normal merge metadata — no freshness routing
    let meta = json!({
        "branch_freshness_conflict": false,
        "plan_update_conflict": false,
    });
    let freshness = FreshnessMetadata::from_task_metadata(&meta);
    assert!(
        !freshness.branch_freshness_conflict,
        "branch_freshness_conflict=false must not trigger freshness routing"
    );
}

#[test]
fn test_no_freshness_keys_in_metadata_skips_routing() {
    // Empty metadata (normal merge path) — no freshness routing
    let meta = json!({});
    let freshness = FreshnessMetadata::from_task_metadata(&meta);
    assert!(
        !freshness.branch_freshness_conflict,
        "absent branch_freshness_conflict must not trigger freshness routing"
    );
}

// ==================
// Metadata clearing tests (mirrors handle_freshness_return_routing clear step)
// ==================

#[test]
fn test_metadata_cleared_for_executing_origin() {
    let mut meta = json!({
        "branch_freshness_conflict": true,
        "freshness_origin_state": "executing",
        "freshness_conflict_count": 1,
        "plan_update_conflict": true,
        "source_update_conflict": false,
        "conflict_files": ["src/foo.rs"],
        "source_branch": "task/branch",
        "target_branch": "plan/branch",
        // Non-freshness keys must be preserved
        "trigger_origin": "scheduler",
        "some_other_flag": 42,
    });

    FreshnessMetadata::clear_from(&mut meta);

    // All freshness keys removed
    let obj = meta.as_object().unwrap();
    assert!(!obj.contains_key("branch_freshness_conflict"));
    assert!(!obj.contains_key("freshness_origin_state"));
    assert!(!obj.contains_key("freshness_conflict_count"));
    assert!(!obj.contains_key("plan_update_conflict"));
    assert!(!obj.contains_key("source_update_conflict"));
    assert!(!obj.contains_key("conflict_files"));
    assert!(!obj.contains_key("source_branch"));
    assert!(!obj.contains_key("target_branch"));

    // Non-freshness keys preserved
    assert_eq!(meta["trigger_origin"], "scheduler");
    assert_eq!(meta["some_other_flag"], 42);
}

#[test]
fn test_metadata_cleared_for_reviewing_origin() {
    let mut meta = json!({
        "branch_freshness_conflict": true,
        "freshness_origin_state": "reviewing",
        "freshness_conflict_count": 2,
        "source_update_conflict": true,
        "source_branch": "task/review-branch",
        "target_branch": "plan/main-plan",
    });

    FreshnessMetadata::clear_from(&mut meta);

    let obj = meta.as_object().unwrap();
    assert!(!obj.contains_key("branch_freshness_conflict"));
    assert!(!obj.contains_key("freshness_origin_state"));
    assert!(!obj.contains_key("freshness_conflict_count"));
    assert!(!obj.contains_key("source_update_conflict"));
    assert!(!obj.contains_key("source_branch"));
    assert!(!obj.contains_key("target_branch"));
}

#[test]
fn test_metadata_cleared_for_corrupted_no_origin() {
    // branch_freshness_conflict=true but freshness_origin_state absent
    let mut meta = json!({
        "branch_freshness_conflict": true,
        "freshness_conflict_count": 1,
    });

    FreshnessMetadata::clear_from(&mut meta);

    let obj = meta.as_object().unwrap();
    assert!(!obj.contains_key("branch_freshness_conflict"));
    assert!(!obj.contains_key("freshness_conflict_count"));
    // Object should be empty after clearing
    assert!(obj.is_empty());
}

// ==================
// FreshnessMetadata extraction from metadata JSON
// ==================

#[test]
fn test_from_metadata_extracts_origin_state_for_routing() {
    let meta = json!({
        "branch_freshness_conflict": true,
        "freshness_origin_state": "reviewing",
        "freshness_conflict_count": 3,
    });

    let freshness = FreshnessMetadata::from_task_metadata(&meta);

    assert!(freshness.branch_freshness_conflict);
    assert_eq!(freshness.freshness_origin_state.as_deref(), Some("reviewing"));
    assert_eq!(freshness.freshness_conflict_count, 3);

    // Verify routing decision based on extracted state
    let target = routing_target_for(freshness.freshness_origin_state.as_deref());
    assert_eq!(target, InternalStatus::PendingReview);
}

#[test]
fn test_from_metadata_extracts_re_executing_for_routing() {
    let meta = json!({
        "branch_freshness_conflict": true,
        "freshness_origin_state": "re_executing",
        "freshness_conflict_count": 1,
        "source_update_conflict": true,
    });

    let freshness = FreshnessMetadata::from_task_metadata(&meta);

    assert!(freshness.branch_freshness_conflict);
    assert_eq!(freshness.freshness_origin_state.as_deref(), Some("re_executing"));
    assert!(freshness.source_update_conflict);

    let target = routing_target_for(freshness.freshness_origin_state.as_deref());
    assert_eq!(target, InternalStatus::Ready);
}
