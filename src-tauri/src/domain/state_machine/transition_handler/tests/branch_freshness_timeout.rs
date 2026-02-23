// Tests for branch freshness timeout: update_plan_from_main and update_source_from_target
// are wrapped in tokio::time::timeout to prevent indefinite hangs when git operations stall.
//
// Config/env tests live in runtime_config_tests.rs (same module scope).
// These tests verify the integration behavior from the transition handler perspective.

use crate::infrastructure::agents::claude::ReconciliationConfig;

/// ReconciliationConfig::default() includes branch_freshness_timeout_secs = 60.
#[test]
fn test_branch_freshness_timeout_default() {
    let cfg = ReconciliationConfig::default();
    assert_eq!(
        cfg.branch_freshness_timeout_secs, 60,
        "Default branch freshness timeout should be 60 seconds"
    );
}

/// YAML deserialization requires branch_freshness_timeout_secs (no serde defaults).
#[test]
fn test_yaml_requires_branch_freshness_timeout() {
    // Missing the new field → deserialization fails
    let yaml = r#"
merger_timeout_secs: 1200
merging_max_retries: 3
pending_merge_stale_minutes: 2
qa_stale_minutes: 5
merge_incomplete_retry_base_secs: 30
merge_incomplete_retry_max_secs: 1800
merge_incomplete_max_retries: 5
validation_revert_max_count: 2
merge_conflict_retry_base_secs: 60
merge_conflict_retry_max_secs: 600
merge_conflict_max_retries: 3
executing_max_retries: 5
reviewing_max_retries: 3
qa_max_retries: 3
executing_max_wall_clock_minutes: 60
reviewing_max_wall_clock_minutes: 30
qa_max_wall_clock_minutes: 15
attempt_merge_deadline_secs: 120
validation_deadline_secs: 1200
merge_registry_grace_period_secs: 60
validation_retry_min_cooldown_secs: 120
validation_failure_circuit_breaker_count: 3
merge_starvation_guard_secs: 60
"#;
    let result: Result<ReconciliationConfig, _> = serde_yaml::from_str(yaml);
    assert!(
        result.is_err(),
        "Missing branch_freshness_timeout_secs should fail YAML deserialization"
    );
}

/// YAML deserialization succeeds when branch_freshness_timeout_secs is present.
#[test]
fn test_yaml_with_branch_freshness_timeout() {
    let yaml = r#"
merger_timeout_secs: 1200
merging_max_retries: 3
pending_merge_stale_minutes: 2
qa_stale_minutes: 5
merge_incomplete_retry_base_secs: 30
merge_incomplete_retry_max_secs: 1800
merge_incomplete_max_retries: 5
validation_revert_max_count: 2
merge_conflict_retry_base_secs: 60
merge_conflict_retry_max_secs: 600
merge_conflict_max_retries: 3
executing_max_retries: 5
reviewing_max_retries: 3
qa_max_retries: 3
executing_max_wall_clock_minutes: 60
reviewing_max_wall_clock_minutes: 30
qa_max_wall_clock_minutes: 15
attempt_merge_deadline_secs: 120
validation_deadline_secs: 1200
merge_registry_grace_period_secs: 60
validation_retry_min_cooldown_secs: 120
validation_failure_circuit_breaker_count: 3
merge_starvation_guard_secs: 60
branch_freshness_timeout_secs: 90
"#;
    let cfg: ReconciliationConfig = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(cfg.branch_freshness_timeout_secs, 90);
}
