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
    assert_eq!(cfg.git.cmd_timeout_secs, 60);
    assert_eq!(cfg.git.retry_backoff_secs, vec![1, 2, 4]);
    assert_eq!(cfg.scheduler.watchdog_interval_secs, 60);
    assert_eq!(cfg.supervisor.time_threshold_secs, 600);
    assert_eq!(cfg.limits.max_resume_attempts, 5);
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
    assert_eq!(cfg.reconciliation.attempt_merge_deadline_secs, 120);
}

#[test]
fn test_yaml_deserialization_requires_all_fields() {
    // Partial YAML should fail — all fields are required (no serde defaults)
    let partial_yaml = "merge_line_read_secs: 900";
    let result: Result<StreamTimeoutsConfig, _> = serde_yaml::from_str(partial_yaml);
    assert!(result.is_err(), "partial YAML should fail without serde defaults");
}

#[test]
fn test_yaml_deserialization_with_all_fields() {
    let yaml = r#"
merge_line_read_secs: 900
merge_parse_stall_secs: 180
review_line_read_secs: 300
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
