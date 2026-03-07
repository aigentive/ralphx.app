use super::*;

#[test]
fn test_all_defaults_are_sensible() {
    let cfg = AllRuntimeConfig {
        stream: StreamTimeoutsConfig::default(),
        reconciliation: ReconciliationConfig::default(),
        git: GitRuntimeConfig::default(),
        scheduler: SchedulerConfig::default(),
        supervisor: SupervisorRuntimeConfig::default(),
        limits: LimitsConfig::default(),
    };
    assert_eq!(cfg.stream.merge_line_read_secs, 600);
    assert_eq!(cfg.reconciliation.merger_timeout_secs, 1200);
    assert_eq!(cfg.reconciliation.validation_deadline_secs, 1200);
    assert_eq!(cfg.reconciliation.branch_freshness_timeout_secs, 60);
    assert_eq!(cfg.git.cmd_timeout_secs, 60);
    assert_eq!(cfg.git.retry_backoff_secs, vec![1, 2, 4]);
    assert_eq!(cfg.scheduler.watchdog_interval_secs, 60);
    assert_eq!(cfg.supervisor.time_threshold_secs, 600);
    assert_eq!(cfg.limits.max_resume_attempts, 5);
}

/// Merge speed overhaul: verify reduced timeout defaults for faster merge pipeline.
#[test]
fn test_merge_speed_defaults() {
    let recon = ReconciliationConfig::default();
    let git = GitRuntimeConfig::default();

    // Reconciliation — merge-speed targets
    assert_eq!(recon.attempt_merge_deadline_secs, 60, "merge deadline: 600→60");
    assert_eq!(recon.merge_incomplete_retry_base_secs, 5, "retry base: 30→5");

    // Git — agent cleanup speed targets
    assert_eq!(git.agent_stop_timeout_secs, 3, "agent stop: 10→3");
    assert_eq!(git.agent_kill_settle_secs, 0, "kill settle: 1→0");
    assert_eq!(git.cleanup_worktree_timeout_secs, 5, "worktree cleanup: 10→5");
    assert_eq!(git.step_0b_kill_timeout_secs, 5, "step 0b kill: 20→5");
}

/// Verify env overrides still work for the changed merge-speed fields.
#[test]
fn test_merge_speed_env_overrides() {
    let mut cfg = AllRuntimeConfig {
        stream: StreamTimeoutsConfig::default(),
        reconciliation: ReconciliationConfig::default(),
        git: GitRuntimeConfig::default(),
        scheduler: SchedulerConfig::default(),
        supervisor: SupervisorRuntimeConfig::default(),
        limits: LimitsConfig::default(),
    };

    apply_env_overrides_with(&mut cfg, &|name| match name {
        "RALPHX_RECONCILIATION_ATTEMPT_MERGE_DEADLINE_SECS" => Some("90".to_string()),
        "RALPHX_RECONCILIATION_MERGE_INCOMPLETE_RETRY_BASE_SECS" => Some("10".to_string()),
        "RALPHX_GIT_AGENT_STOP_TIMEOUT_SECS" => Some("7".to_string()),
        "RALPHX_GIT_AGENT_KILL_SETTLE_SECS" => Some("2".to_string()),
        "RALPHX_GIT_CLEANUP_WORKTREE_TIMEOUT_SECS" => Some("8".to_string()),
        "RALPHX_GIT_STEP_0B_KILL_TIMEOUT_SECS" => Some("12".to_string()),
        _ => None,
    });

    assert_eq!(cfg.reconciliation.attempt_merge_deadline_secs, 90);
    assert_eq!(cfg.reconciliation.merge_incomplete_retry_base_secs, 10);
    assert_eq!(cfg.git.agent_stop_timeout_secs, 7);
    assert_eq!(cfg.git.agent_kill_settle_secs, 2);
    assert_eq!(cfg.git.cleanup_worktree_timeout_secs, 8);
    assert_eq!(cfg.git.step_0b_kill_timeout_secs, 12);
}

#[test]
fn test_env_overrides_apply() {
    let mut cfg = AllRuntimeConfig {
        stream: StreamTimeoutsConfig::default(),
        reconciliation: ReconciliationConfig::default(),
        git: GitRuntimeConfig::default(),
        scheduler: SchedulerConfig::default(),
        supervisor: SupervisorRuntimeConfig::default(),
        limits: LimitsConfig::default(),
    };

    apply_env_overrides_with(&mut cfg, &|name| match name {
        "RALPHX_STREAM_MERGE_LINE_READ_SECS" => Some("999".to_string()),
        "RALPHX_RECONCILIATION_MERGER_TIMEOUT_SECS" => Some("2400".to_string()),
        "RALPHX_GIT_CMD_TIMEOUT_SECS" => Some("120".to_string()),
        "RALPHX_GIT_RETRY_BACKOFF_SECS" => Some("2,4,8,16".to_string()),
        "RALPHX_SCHEDULER_READY_SETTLE_MS" => Some("500".to_string()),
        "RALPHX_SUPERVISOR_MAX_TOKENS" => Some("200000".to_string()),
        "RALPHX_LIMITS_MAX_RESUME_ATTEMPTS" => Some("10".to_string()),
        _ => None,
    });

    assert_eq!(cfg.stream.merge_line_read_secs, 999);
    assert_eq!(cfg.reconciliation.merger_timeout_secs, 2400);
    // validation_deadline_secs not overridden — should keep default
    assert_eq!(cfg.reconciliation.validation_deadline_secs, 1200);
    assert_eq!(cfg.git.cmd_timeout_secs, 120);
    assert_eq!(cfg.git.retry_backoff_secs, vec![2, 4, 8, 16]);
    assert_eq!(cfg.scheduler.ready_settle_ms, 500);
    assert_eq!(cfg.supervisor.max_tokens, 200000);
    assert_eq!(cfg.limits.max_resume_attempts, 10);
}

#[test]
fn test_backward_compat_merger_timeout_env() {
    let mut cfg = AllRuntimeConfig {
        stream: StreamTimeoutsConfig::default(),
        reconciliation: ReconciliationConfig::default(),
        git: GitRuntimeConfig::default(),
        scheduler: SchedulerConfig::default(),
        supervisor: SupervisorRuntimeConfig::default(),
        limits: LimitsConfig::default(),
    };

    // Old key only
    apply_env_overrides_with(&mut cfg, &|name| match name {
        "RALPHX_MERGER_TIMEOUT_SECS" => Some("1800".to_string()),
        _ => None,
    });
    assert_eq!(cfg.reconciliation.merger_timeout_secs, 1800);
}

#[test]
fn test_new_key_takes_precedence_over_old() {
    let mut cfg = AllRuntimeConfig {
        stream: StreamTimeoutsConfig::default(),
        reconciliation: ReconciliationConfig::default(),
        git: GitRuntimeConfig::default(),
        scheduler: SchedulerConfig::default(),
        supervisor: SupervisorRuntimeConfig::default(),
        limits: LimitsConfig::default(),
    };

    // Both keys set — new one should win (applied second)
    apply_env_overrides_with(&mut cfg, &|name| match name {
        "RALPHX_MERGER_TIMEOUT_SECS" => Some("1800".to_string()),
        "RALPHX_RECONCILIATION_MERGER_TIMEOUT_SECS" => Some("2400".to_string()),
        _ => None,
    });
    assert_eq!(cfg.reconciliation.merger_timeout_secs, 2400);
}

#[test]
fn test_invalid_env_values_ignored() {
    let mut cfg = AllRuntimeConfig {
        stream: StreamTimeoutsConfig::default(),
        reconciliation: ReconciliationConfig::default(),
        git: GitRuntimeConfig::default(),
        scheduler: SchedulerConfig::default(),
        supervisor: SupervisorRuntimeConfig::default(),
        limits: LimitsConfig::default(),
    };

    apply_env_overrides_with(&mut cfg, &|name| match name {
        "RALPHX_STREAM_MERGE_LINE_READ_SECS" => Some("not_a_number".to_string()),
        "RALPHX_GIT_RETRY_BACKOFF_SECS" => Some("".to_string()),
        _ => None,
    });

    // Should keep defaults
    assert_eq!(cfg.stream.merge_line_read_secs, 600);
    assert_eq!(cfg.git.retry_backoff_secs, vec![1, 2, 4]);
}

#[test]
fn test_validation_deadline_env_override() {
    let mut cfg = AllRuntimeConfig {
        stream: StreamTimeoutsConfig::default(),
        reconciliation: ReconciliationConfig::default(),
        git: GitRuntimeConfig::default(),
        scheduler: SchedulerConfig::default(),
        supervisor: SupervisorRuntimeConfig::default(),
        limits: LimitsConfig::default(),
    };

    apply_env_overrides_with(&mut cfg, &|name| match name {
        "RALPHX_RECONCILIATION_VALIDATION_DEADLINE_SECS" => Some("900".to_string()),
        _ => None,
    });

    assert_eq!(cfg.reconciliation.validation_deadline_secs, 900);
    // merge deadline should remain unchanged
    assert_eq!(cfg.reconciliation.attempt_merge_deadline_secs, 60);
}

#[test]
fn test_yaml_deserialization_requires_all_fields() {
    // Partial YAML should fail — all fields are required (no serde defaults)
    let partial_yaml = "merge_line_read_secs: 900";
    let result: Result<StreamTimeoutsConfig, _> = serde_yaml::from_str(partial_yaml);
    assert!(
        result.is_err(),
        "partial YAML should fail without serde defaults"
    );
}

#[test]
fn test_yaml_deserialization_with_all_fields() {
    let yaml = r#"
merge_line_read_secs: 900
merge_parse_stall_secs: 180
review_line_read_secs: 600
review_parse_stall_secs: 120
default_line_read_secs: 600
default_parse_stall_secs: 180
team_line_read_secs: 3600
team_parse_stall_secs: 3600
"#;
    let cfg: StreamTimeoutsConfig = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(cfg.merge_line_read_secs, 900);
    assert_eq!(cfg.merge_parse_stall_secs, 180);
}

#[test]
fn test_branch_freshness_timeout_env_override() {
    let mut cfg = AllRuntimeConfig {
        stream: StreamTimeoutsConfig::default(),
        reconciliation: ReconciliationConfig::default(),
        git: GitRuntimeConfig::default(),
        scheduler: SchedulerConfig::default(),
        supervisor: SupervisorRuntimeConfig::default(),
        limits: LimitsConfig::default(),
    };

    apply_env_overrides_with(&mut cfg, &|name| match name {
        "RALPHX_RECONCILIATION_BRANCH_FRESHNESS_TIMEOUT_SECS" => Some("120".to_string()),
        _ => None,
    });

    assert_eq!(cfg.reconciliation.branch_freshness_timeout_secs, 120);
    // Other reconciliation fields should remain unchanged
    assert_eq!(cfg.reconciliation.attempt_merge_deadline_secs, 60);
}

// ── Execution recovery config defaults + validation (GAP M7) ──────────────────

#[test]
fn test_execution_failed_config_defaults_are_sensible() {
    let recon = ReconciliationConfig::default();

    assert_eq!(recon.execution_failed_max_retries, 3, "default max retries: 3");
    assert_eq!(recon.execution_failed_retry_base_secs, 30, "default base: 30s");
    assert_eq!(recon.execution_failed_retry_max_secs, 600, "default max: 600s");
}

/// GAP M7: base_secs must be ≤ max_secs in default config.
#[test]
fn test_execution_failed_retry_base_le_max_in_defaults() {
    let recon = ReconciliationConfig::default();
    assert!(
        recon.execution_failed_retry_base_secs <= recon.execution_failed_retry_max_secs,
        "base ({}) must be ≤ max ({})",
        recon.execution_failed_retry_base_secs,
        recon.execution_failed_retry_max_secs
    );
}

#[test]
fn test_execution_failed_max_retries_is_positive() {
    let recon = ReconciliationConfig::default();
    assert!(
        recon.execution_failed_max_retries > 0,
        "execution_failed_max_retries must be > 0"
    );
}

#[test]
fn test_execution_failed_max_retries_env_override() {
    let mut cfg = AllRuntimeConfig {
        stream: StreamTimeoutsConfig::default(),
        reconciliation: ReconciliationConfig::default(),
        git: GitRuntimeConfig::default(),
        scheduler: SchedulerConfig::default(),
        supervisor: SupervisorRuntimeConfig::default(),
        limits: LimitsConfig::default(),
    };

    apply_env_overrides_with(&mut cfg, &|name| match name {
        "RALPHX_RECONCILIATION_EXECUTION_FAILED_MAX_RETRIES" => Some("5".to_string()),
        _ => None,
    });

    assert_eq!(cfg.reconciliation.execution_failed_max_retries, 5);
    // Other fields remain unchanged
    assert_eq!(cfg.reconciliation.execution_failed_retry_base_secs, 30);
}

#[test]
fn test_execution_failed_retry_base_secs_env_override() {
    let mut cfg = AllRuntimeConfig {
        stream: StreamTimeoutsConfig::default(),
        reconciliation: ReconciliationConfig::default(),
        git: GitRuntimeConfig::default(),
        scheduler: SchedulerConfig::default(),
        supervisor: SupervisorRuntimeConfig::default(),
        limits: LimitsConfig::default(),
    };

    apply_env_overrides_with(&mut cfg, &|name| match name {
        "RALPHX_RECONCILIATION_EXECUTION_FAILED_RETRY_BASE_SECS" => Some("60".to_string()),
        _ => None,
    });

    assert_eq!(cfg.reconciliation.execution_failed_retry_base_secs, 60);
}

#[test]
fn test_execution_failed_retry_max_secs_env_override() {
    let mut cfg = AllRuntimeConfig {
        stream: StreamTimeoutsConfig::default(),
        reconciliation: ReconciliationConfig::default(),
        git: GitRuntimeConfig::default(),
        scheduler: SchedulerConfig::default(),
        supervisor: SupervisorRuntimeConfig::default(),
        limits: LimitsConfig::default(),
    };

    apply_env_overrides_with(&mut cfg, &|name| match name {
        "RALPHX_RECONCILIATION_EXECUTION_FAILED_RETRY_MAX_SECS" => Some("1200".to_string()),
        _ => None,
    });

    assert_eq!(cfg.reconciliation.execution_failed_retry_max_secs, 1200);
    // Base unchanged
    assert_eq!(cfg.reconciliation.execution_failed_retry_base_secs, 30);
}

#[test]
fn test_execution_failed_all_three_env_overrides_applied_together() {
    let mut cfg = AllRuntimeConfig {
        stream: StreamTimeoutsConfig::default(),
        reconciliation: ReconciliationConfig::default(),
        git: GitRuntimeConfig::default(),
        scheduler: SchedulerConfig::default(),
        supervisor: SupervisorRuntimeConfig::default(),
        limits: LimitsConfig::default(),
    };

    apply_env_overrides_with(&mut cfg, &|name| match name {
        "RALPHX_RECONCILIATION_EXECUTION_FAILED_MAX_RETRIES" => Some("5".to_string()),
        "RALPHX_RECONCILIATION_EXECUTION_FAILED_RETRY_BASE_SECS" => Some("45".to_string()),
        "RALPHX_RECONCILIATION_EXECUTION_FAILED_RETRY_MAX_SECS" => Some("900".to_string()),
        _ => None,
    });

    assert_eq!(cfg.reconciliation.execution_failed_max_retries, 5);
    assert_eq!(cfg.reconciliation.execution_failed_retry_base_secs, 45);
    assert_eq!(cfg.reconciliation.execution_failed_retry_max_secs, 900);

    // GAP M7 validation: base still ≤ max after overrides
    assert!(
        cfg.reconciliation.execution_failed_retry_base_secs
            <= cfg.reconciliation.execution_failed_retry_max_secs
    );
}

#[test]
fn test_circuit_breaker_config_defaults() {
    let config = ReconciliationConfig::default();
    assert_eq!(config.merge_circuit_breaker_threshold, 3);
    assert_eq!(config.merge_circuit_breaker_window, 5);
}

#[test]
fn test_circuit_breaker_env_overrides() {
    let mut cfg = AllRuntimeConfig {
        stream: StreamTimeoutsConfig::default(),
        reconciliation: ReconciliationConfig::default(),
        git: GitRuntimeConfig::default(),
        scheduler: SchedulerConfig::default(),
        supervisor: SupervisorRuntimeConfig::default(),
        limits: LimitsConfig::default(),
    };

    apply_env_overrides_with(&mut cfg, &|name| match name {
        "RALPHX_MERGE_CIRCUIT_BREAKER_THRESHOLD" => Some("5".to_string()),
        "RALPHX_MERGE_CIRCUIT_BREAKER_WINDOW" => Some("10".to_string()),
        _ => None,
    });

    assert_eq!(cfg.reconciliation.merge_circuit_breaker_threshold, 5);
    assert_eq!(cfg.reconciliation.merge_circuit_breaker_window, 10);
}

#[test]
fn test_execution_failed_invalid_env_values_keep_defaults() {
    let mut cfg = AllRuntimeConfig {
        stream: StreamTimeoutsConfig::default(),
        reconciliation: ReconciliationConfig::default(),
        git: GitRuntimeConfig::default(),
        scheduler: SchedulerConfig::default(),
        supervisor: SupervisorRuntimeConfig::default(),
        limits: LimitsConfig::default(),
    };

    apply_env_overrides_with(&mut cfg, &|name| match name {
        "RALPHX_RECONCILIATION_EXECUTION_FAILED_MAX_RETRIES" => Some("not_a_number".to_string()),
        "RALPHX_RECONCILIATION_EXECUTION_FAILED_RETRY_BASE_SECS" => Some("abc".to_string()),
        _ => None,
    });

    // Invalid values ignored — defaults preserved
    assert_eq!(cfg.reconciliation.execution_failed_max_retries, 3);
    assert_eq!(cfg.reconciliation.execution_failed_retry_base_secs, 30);
}
