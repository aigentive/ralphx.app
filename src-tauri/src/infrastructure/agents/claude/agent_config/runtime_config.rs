use serde::Deserialize;

// ── Top-level wrapper ────────────────────────────────────────────────────

/// All runtime configuration collected from ralphx.yaml + env overrides.
#[derive(Debug, Clone)]
pub struct AllRuntimeConfig {
    pub stream: StreamTimeoutsConfig,
    pub reconciliation: ReconciliationConfig,
    pub git: GitRuntimeConfig,
    pub scheduler: SchedulerConfig,
    pub supervisor: SupervisorRuntimeConfig,
    pub limits: LimitsConfig,
}

// ── YAML wrapper for nested `timeouts:` key ──────────────────────────────

#[derive(Debug, Clone, Deserialize, Default)]
pub(crate) struct TimeoutsWrapper {
    #[serde(default)]
    pub stream: StreamTimeoutsConfig,
}

// ── Individual config structs ────────────────────────────────────────────

/// All fields required in ralphx.yaml — no serde defaults.
/// `Default` impl retained only for fallback/test use.
#[derive(Debug, Clone, Deserialize)]
pub struct StreamTimeoutsConfig {
    pub merge_line_read_secs: u64,
    pub merge_parse_stall_secs: u64,
    pub review_line_read_secs: u64,
    pub review_parse_stall_secs: u64,
    pub default_line_read_secs: u64,
    pub default_parse_stall_secs: u64,
    pub team_line_read_secs: u64,
    pub team_parse_stall_secs: u64,
}

impl Default for StreamTimeoutsConfig {
    fn default() -> Self {
        Self {
            merge_line_read_secs: 600,
            merge_parse_stall_secs: 180,
            review_line_read_secs: 300,
            review_parse_stall_secs: 120,
            default_line_read_secs: 600,
            default_parse_stall_secs: 180,
            team_line_read_secs: 3600,
            team_parse_stall_secs: 3600,
        }
    }
}

/// All fields required in ralphx.yaml — no serde defaults.
/// `Default` impl retained only for fallback/test use.
#[derive(Debug, Clone, Deserialize)]
pub struct ReconciliationConfig {
    pub merger_timeout_secs: u64,
    pub merging_max_retries: u64,
    pub pending_merge_stale_minutes: u64,
    pub qa_stale_minutes: u64,
    pub merge_incomplete_retry_base_secs: u64,
    pub merge_incomplete_retry_max_secs: u64,
    pub merge_incomplete_max_retries: u64,
    pub validation_revert_max_count: u64,
    pub merge_conflict_retry_base_secs: u64,
    pub merge_conflict_retry_max_secs: u64,
    pub merge_conflict_max_retries: u64,
    pub executing_max_retries: u64,
    pub reviewing_max_retries: u64,
    pub qa_max_retries: u64,
    pub executing_max_wall_clock_minutes: u64,
    pub reviewing_max_wall_clock_minutes: u64,
    pub qa_max_wall_clock_minutes: u64,
    /// Maximum wall-clock seconds for the entire programmatic merge attempt
    /// (cleanup + strategy dispatch). If exceeded, task transitions to MergeIncomplete.
    pub attempt_merge_deadline_secs: u64,
}

impl Default for ReconciliationConfig {
    fn default() -> Self {
        Self {
            merger_timeout_secs: 1200,
            merging_max_retries: 3,
            pending_merge_stale_minutes: 2,
            qa_stale_minutes: 5,
            merge_incomplete_retry_base_secs: 30,
            merge_incomplete_retry_max_secs: 1800,
            merge_incomplete_max_retries: 50,
            validation_revert_max_count: 2,
            merge_conflict_retry_base_secs: 60,
            merge_conflict_retry_max_secs: 600,
            merge_conflict_max_retries: 3,
            executing_max_retries: 5,
            reviewing_max_retries: 3,
            qa_max_retries: 3,
            executing_max_wall_clock_minutes: 60,
            reviewing_max_wall_clock_minutes: 30,
            qa_max_wall_clock_minutes: 15,
            attempt_merge_deadline_secs: 120,
        }
    }
}

/// All fields required in ralphx.yaml — no serde defaults.
/// `Default` impl retained only for fallback/test use.
#[derive(Debug, Clone, Deserialize)]
pub struct GitRuntimeConfig {
    pub cmd_timeout_secs: u64,
    pub max_retries: u64,
    pub retry_backoff_secs: Vec<u64>,
    pub index_lock_stale_secs: u64,
}

impl Default for GitRuntimeConfig {
    fn default() -> Self {
        Self {
            cmd_timeout_secs: 60,
            max_retries: 3,
            retry_backoff_secs: vec![1, 2, 4],
            index_lock_stale_secs: 5,
        }
    }
}

/// All fields required in ralphx.yaml — no serde defaults.
/// `Default` impl retained only for fallback/test use.
#[derive(Debug, Clone, Deserialize)]
pub struct SchedulerConfig {
    pub watchdog_interval_secs: u64,
    pub watchdog_stale_threshold_secs: u64,
    pub max_contention_retries: u64,
    pub contention_retry_delay_ms: u64,
    pub ready_settle_ms: u64,
    pub merge_settle_ms: u64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            watchdog_interval_secs: 60,
            watchdog_stale_threshold_secs: 30,
            max_contention_retries: 3,
            contention_retry_delay_ms: 200,
            ready_settle_ms: 300,
            merge_settle_ms: 100,
        }
    }
}

/// All fields required in ralphx.yaml — no serde defaults.
/// `Default` impl retained only for fallback/test use.
#[derive(Debug, Clone, Deserialize)]
pub struct SupervisorRuntimeConfig {
    pub time_threshold_secs: u64,
    pub token_threshold: u64,
    pub max_tokens: u64,
    pub progress_interval_secs: u64,
    pub loop_threshold: u64,
    pub stuck_threshold: u64,
}

impl Default for SupervisorRuntimeConfig {
    fn default() -> Self {
        Self {
            time_threshold_secs: 600,
            token_threshold: 50000,
            max_tokens: 100000,
            progress_interval_secs: 30,
            loop_threshold: 3,
            stuck_threshold: 5,
        }
    }
}

/// All fields required in ralphx.yaml — no serde defaults.
/// `Default` impl retained only for fallback/test use.
#[derive(Debug, Clone, Deserialize)]
pub struct LimitsConfig {
    pub max_resume_attempts: u64,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_resume_attempts: 5,
        }
    }
}

// ── Env overrides ────────────────────────────────────────────────────────

pub fn apply_env_overrides(cfg: &mut AllRuntimeConfig) {
    apply_env_overrides_with(cfg, &|name| std::env::var(name).ok());
}

fn apply_env_overrides_with(cfg: &mut AllRuntimeConfig, lookup: &dyn Fn(&str) -> Option<String>) {
    macro_rules! env_u64 {
        ($field:expr, $key:expr) => {
            if let Some(v) = lookup($key) {
                if let Ok(n) = v.parse::<u64>() {
                    $field = n;
                }
            }
        };
    }

    // Stream timeouts
    env_u64!(cfg.stream.merge_line_read_secs, "RALPHX_STREAM_MERGE_LINE_READ_SECS");
    env_u64!(cfg.stream.merge_parse_stall_secs, "RALPHX_STREAM_MERGE_PARSE_STALL_SECS");
    env_u64!(cfg.stream.review_line_read_secs, "RALPHX_STREAM_REVIEW_LINE_READ_SECS");
    env_u64!(cfg.stream.review_parse_stall_secs, "RALPHX_STREAM_REVIEW_PARSE_STALL_SECS");
    env_u64!(cfg.stream.default_line_read_secs, "RALPHX_STREAM_DEFAULT_LINE_READ_SECS");
    env_u64!(cfg.stream.default_parse_stall_secs, "RALPHX_STREAM_DEFAULT_PARSE_STALL_SECS");
    env_u64!(cfg.stream.team_line_read_secs, "RALPHX_STREAM_TEAM_LINE_READ_SECS");
    env_u64!(cfg.stream.team_parse_stall_secs, "RALPHX_STREAM_TEAM_PARSE_STALL_SECS");

    // Reconciliation
    // Backward compat: old env key
    env_u64!(cfg.reconciliation.merger_timeout_secs, "RALPHX_MERGER_TIMEOUT_SECS");
    // New canonical key (takes precedence if both set)
    env_u64!(cfg.reconciliation.merger_timeout_secs, "RALPHX_RECONCILIATION_MERGER_TIMEOUT_SECS");
    env_u64!(cfg.reconciliation.merging_max_retries, "RALPHX_RECONCILIATION_MERGING_MAX_RETRIES");
    env_u64!(cfg.reconciliation.pending_merge_stale_minutes, "RALPHX_RECONCILIATION_PENDING_MERGE_STALE_MINUTES");
    env_u64!(cfg.reconciliation.qa_stale_minutes, "RALPHX_RECONCILIATION_QA_STALE_MINUTES");
    env_u64!(cfg.reconciliation.merge_incomplete_retry_base_secs, "RALPHX_RECONCILIATION_MERGE_INCOMPLETE_RETRY_BASE_SECS");
    env_u64!(cfg.reconciliation.merge_incomplete_retry_max_secs, "RALPHX_RECONCILIATION_MERGE_INCOMPLETE_RETRY_MAX_SECS");
    env_u64!(cfg.reconciliation.merge_incomplete_max_retries, "RALPHX_RECONCILIATION_MERGE_INCOMPLETE_MAX_RETRIES");
    env_u64!(cfg.reconciliation.validation_revert_max_count, "RALPHX_RECONCILIATION_VALIDATION_REVERT_MAX_COUNT");
    env_u64!(cfg.reconciliation.merge_conflict_retry_base_secs, "RALPHX_RECONCILIATION_MERGE_CONFLICT_RETRY_BASE_SECS");
    env_u64!(cfg.reconciliation.merge_conflict_retry_max_secs, "RALPHX_RECONCILIATION_MERGE_CONFLICT_RETRY_MAX_SECS");
    env_u64!(cfg.reconciliation.merge_conflict_max_retries, "RALPHX_RECONCILIATION_MERGE_CONFLICT_MAX_RETRIES");
    env_u64!(cfg.reconciliation.executing_max_retries, "RALPHX_RECONCILIATION_EXECUTING_MAX_RETRIES");
    env_u64!(cfg.reconciliation.reviewing_max_retries, "RALPHX_RECONCILIATION_REVIEWING_MAX_RETRIES");
    env_u64!(cfg.reconciliation.qa_max_retries, "RALPHX_RECONCILIATION_QA_MAX_RETRIES");
    env_u64!(cfg.reconciliation.executing_max_wall_clock_minutes, "RALPHX_RECONCILIATION_EXECUTING_MAX_WALL_CLOCK_MINUTES");
    env_u64!(cfg.reconciliation.reviewing_max_wall_clock_minutes, "RALPHX_RECONCILIATION_REVIEWING_MAX_WALL_CLOCK_MINUTES");
    env_u64!(cfg.reconciliation.qa_max_wall_clock_minutes, "RALPHX_RECONCILIATION_QA_MAX_WALL_CLOCK_MINUTES");
    env_u64!(cfg.reconciliation.attempt_merge_deadline_secs, "RALPHX_RECONCILIATION_ATTEMPT_MERGE_DEADLINE_SECS");

    // Git
    env_u64!(cfg.git.cmd_timeout_secs, "RALPHX_GIT_CMD_TIMEOUT_SECS");
    env_u64!(cfg.git.max_retries, "RALPHX_GIT_MAX_RETRIES");
    env_u64!(cfg.git.index_lock_stale_secs, "RALPHX_GIT_INDEX_LOCK_STALE_SECS");
    // retry_backoff_secs: comma-separated list
    if let Some(v) = lookup("RALPHX_GIT_RETRY_BACKOFF_SECS") {
        let parsed: Vec<u64> = v.split(',')
            .filter_map(|s| s.trim().parse::<u64>().ok())
            .collect();
        if !parsed.is_empty() {
            cfg.git.retry_backoff_secs = parsed;
        }
    }

    // Scheduler
    env_u64!(cfg.scheduler.watchdog_interval_secs, "RALPHX_SCHEDULER_WATCHDOG_INTERVAL_SECS");
    env_u64!(cfg.scheduler.watchdog_stale_threshold_secs, "RALPHX_SCHEDULER_WATCHDOG_STALE_THRESHOLD_SECS");
    env_u64!(cfg.scheduler.max_contention_retries, "RALPHX_SCHEDULER_MAX_CONTENTION_RETRIES");
    env_u64!(cfg.scheduler.contention_retry_delay_ms, "RALPHX_SCHEDULER_CONTENTION_RETRY_DELAY_MS");
    env_u64!(cfg.scheduler.ready_settle_ms, "RALPHX_SCHEDULER_READY_SETTLE_MS");
    env_u64!(cfg.scheduler.merge_settle_ms, "RALPHX_SCHEDULER_MERGE_SETTLE_MS");

    // Supervisor
    env_u64!(cfg.supervisor.time_threshold_secs, "RALPHX_SUPERVISOR_TIME_THRESHOLD_SECS");
    env_u64!(cfg.supervisor.token_threshold, "RALPHX_SUPERVISOR_TOKEN_THRESHOLD");
    env_u64!(cfg.supervisor.max_tokens, "RALPHX_SUPERVISOR_MAX_TOKENS");
    env_u64!(cfg.supervisor.progress_interval_secs, "RALPHX_SUPERVISOR_PROGRESS_INTERVAL_SECS");
    env_u64!(cfg.supervisor.loop_threshold, "RALPHX_SUPERVISOR_LOOP_THRESHOLD");
    env_u64!(cfg.supervisor.stuck_threshold, "RALPHX_SUPERVISOR_STUCK_THRESHOLD");

    // Limits
    env_u64!(cfg.limits.max_resume_attempts, "RALPHX_LIMITS_MAX_RESUME_ATTEMPTS");
}

#[cfg(test)]
mod tests {
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
}
