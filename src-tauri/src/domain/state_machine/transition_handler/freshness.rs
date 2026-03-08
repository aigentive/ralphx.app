//! Branch freshness checks for execution and review entry points.
//!
//! Ensures both plan←main and task←feature branches are fresh before
//! an agent is spawned. On conflict, routes to Merging state for resolution.

// Callers in on_enter_states.rs and side_effects.rs are added in subsequent steps.

use std::path::Path;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{info, warn};

use crate::application::git_service::git_cmd;
use crate::application::GitService;
use crate::domain::entities::{
    ActivityEvent, ActivityEventRole, ActivityEventType, Project, Task, TaskId,
};
use crate::domain::repositories::ActivityEventRepository;
use crate::infrastructure::agents::claude::ReconciliationConfig;

use super::merge_coordination::{
    update_plan_from_main, update_source_from_target, PlanUpdateResult, SourceUpdateResult,
};

/// Typed metadata for branch freshness conflict tracking.
///
/// Stored in/extracted from task metadata JSON. Using a struct provides
/// compile-time validation of field names — prevents typos and stale keys.
///
/// Lifecycle:
/// - Initialized: defaults (absent from metadata)
/// - Incremented: once per `ensure_branches_fresh()` call that routes to Merging
/// - Reset: when freshness check passes without conflicts
/// - Cap: 3 (fourth attempt → ExecutionBlocked → task goes to Failed)
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct FreshnessMetadata {
    /// True when task was routed to Merging due to stale branches.
    #[serde(default)]
    pub branch_freshness_conflict: bool,

    /// The state from which the freshness conflict was detected.
    /// Values: "executing" | "re_executing" | "reviewing"
    #[serde(default)]
    pub freshness_origin_state: Option<String>,

    /// Number of times freshness routing has occurred for this task.
    /// Incremented once per `ensure_branches_fresh()` call (not per conflict within a call).
    /// Reset to 0 when freshness check passes without conflicts.
    #[serde(default)]
    pub freshness_conflict_count: u32,

    /// True when the plan←main update had conflicts requiring merger agent resolution.
    #[serde(default)]
    pub plan_update_conflict: bool,

    /// True when the task←feature update had conflicts requiring merger agent resolution.
    #[serde(default)]
    pub source_update_conflict: bool,

    /// RFC3339 timestamp of the last successful freshness check.
    /// Used for skip-if-recently-checked optimization (default window: 30s).
    #[serde(default)]
    pub last_freshness_check_at: Option<String>,

    /// Files involved in the freshness conflict (from git conflict output).
    #[serde(default)]
    pub conflict_files: Vec<String>,

    /// The task/source branch that was being updated (task←feature direction).
    #[serde(default)]
    pub source_branch: Option<String>,

    /// The plan/target branch that was the merge target (task←feature direction).
    #[serde(default)]
    pub target_branch: Option<String>,
}

impl FreshnessMetadata {
    /// All JSON keys managed by FreshnessMetadata.
    const KEYS: &'static [&'static str] = &[
        "branch_freshness_conflict",
        "freshness_origin_state",
        "freshness_conflict_count",
        "plan_update_conflict",
        "source_update_conflict",
        "last_freshness_check_at",
        "conflict_files",
        "source_branch",
        "target_branch",
    ];

    /// Extract FreshnessMetadata from task metadata JSON.
    /// Returns struct with defaults for any missing fields.
    pub fn from_task_metadata(metadata: &Value) -> Self {
        serde_json::from_value(metadata.clone()).unwrap_or_default()
    }

    /// Merge freshness fields back into task metadata JSON.
    /// Preserves existing non-freshness keys. Explicitly handles Option
    /// fields by removing keys when None.
    pub fn merge_into(&self, metadata: &mut Value) {
        let Some(obj) = metadata.as_object_mut() else {
            return;
        };

        obj.insert(
            "branch_freshness_conflict".to_owned(),
            Value::Bool(self.branch_freshness_conflict),
        );
        match &self.freshness_origin_state {
            Some(s) => obj.insert(
                "freshness_origin_state".to_owned(),
                Value::String(s.clone()),
            ),
            None => obj.remove("freshness_origin_state"),
        };
        obj.insert(
            "freshness_conflict_count".to_owned(),
            Value::Number(self.freshness_conflict_count.into()),
        );
        obj.insert(
            "plan_update_conflict".to_owned(),
            Value::Bool(self.plan_update_conflict),
        );
        obj.insert(
            "source_update_conflict".to_owned(),
            Value::Bool(self.source_update_conflict),
        );
        match &self.last_freshness_check_at {
            Some(s) => obj.insert(
                "last_freshness_check_at".to_owned(),
                Value::String(s.clone()),
            ),
            None => obj.remove("last_freshness_check_at"),
        };
        obj.insert(
            "conflict_files".to_owned(),
            Value::Array(
                self.conflict_files
                    .iter()
                    .map(|f| Value::String(f.clone()))
                    .collect(),
            ),
        );
        match &self.source_branch {
            Some(s) => obj.insert("source_branch".to_owned(), Value::String(s.clone())),
            None => obj.remove("source_branch"),
        };
        match &self.target_branch {
            Some(s) => obj.insert("target_branch".to_owned(), Value::String(s.clone())),
            None => obj.remove("target_branch"),
        };
    }

    /// Remove all freshness keys from task metadata JSON.
    pub fn clear_from(metadata: &mut Value) {
        if let Some(obj) = metadata.as_object_mut() {
            for key in Self::KEYS {
                obj.remove(*key);
            }
        }
    }
}

/// Action returned by `ensure_branches_fresh()` when branches are not clean.
#[derive(Debug)]
pub enum FreshnessAction {
    /// Branch conflict detected — route to Merging with freshness metadata.
    RouteToMerging {
        conflict_files: Vec<String>,
        conflict_type: &'static str, // "plan_update" | "source_update"
        freshness_metadata: FreshnessMetadata,
    },
    /// Fatal error or retry cap exceeded — task should fail.
    ExecutionBlocked { reason: String },
}

/// Ensures both plan←main and task←feature branches are fresh.
///
/// Must be called BEFORE any agent process is spawned (before `send_message()`).
///
/// # Returns
/// - `Ok(updated_meta)` — both checks passed; caller should merge updated_meta into task metadata
/// - `Err(FreshnessAction::RouteToMerging)` — conflict; caller sets metadata + transitions to Merging
/// - `Err(FreshnessAction::ExecutionBlocked)` — timeout or retry cap exceeded
///
/// # Errors
/// Returns `Err(FreshnessAction)` when a conflict or execution-blocking condition is detected.
pub async fn ensure_branches_fresh(
    repo_path: &Path,
    task: &Task,
    project: &Project,
    task_id_str: &str,
    plan_branch: Option<&str>,
    app_handle: Option<&tauri::AppHandle>,
    activity_event_repo: Option<&Arc<dyn ActivityEventRepository>>,
    origin_state: &str,
    config: &ReconciliationConfig,
) -> Result<FreshnessMetadata, FreshnessAction> {
    // 1. Config toggle
    if !config.execution_freshness_enabled {
        info!(
            task_id = task_id_str,
            "Freshness check disabled via config (execution_freshness_enabled=false)"
        );
        return Ok(FreshnessMetadata::default());
    }

    // 2. Parse current freshness metadata
    let task_metadata_val: serde_json::Value = task
        .metadata
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    let mut freshness = FreshnessMetadata::from_task_metadata(&task_metadata_val);

    // 3. Skip-if-recently-checked
    if let Some(ref last_check_str) = freshness.last_freshness_check_at.clone() {
        if let Ok(last_check) = last_check_str.parse::<DateTime<Utc>>() {
            let elapsed = Utc::now() - last_check;
            let skip_window = config.freshness_skip_window_secs as i64;
            if elapsed.num_seconds() < skip_window {
                info!(
                    task_id = task_id_str,
                    elapsed_secs = elapsed.num_seconds(),
                    skip_window_secs = skip_window,
                    "Skipping freshness check — last checked recently"
                );
                emit_freshness_activity(
                    activity_event_repo,
                    task_id_str,
                    "branch_freshness_skipped",
                    serde_json::json!({
                        "reason": "recently_checked",
                        "last_check_secs_ago": elapsed.num_seconds(),
                    }),
                )
                .await;
                return Ok(freshness);
            }
        }
    }

    // 4. Dirty worktree guard
    if is_worktree_dirty(repo_path).await {
        warn!(
            task_id = task_id_str,
            "Dirty worktree detected before freshness check — attempting emergency auto-commit"
        );
        match GitService::commit_all_including_deletions(
            repo_path,
            "chore: auto-commit before freshness check",
        )
        .await
        {
            Ok(Some(sha)) => {
                info!(
                    task_id = task_id_str,
                    sha = %sha,
                    "Emergency auto-commit succeeded"
                );
            }
            Ok(None) => {
                info!(
                    task_id = task_id_str,
                    "Emergency auto-commit: nothing to commit (race condition)"
                );
            }
            Err(e) => {
                warn!(
                    task_id = task_id_str,
                    error = %e,
                    "Emergency auto-commit failed — skipping freshness check to unblock execution"
                );
                return Ok(freshness);
            }
        }
    }

    let freshness_timeout =
        std::time::Duration::from_secs(config.branch_freshness_timeout_secs);

    let base_branch = project.base_branch.as_deref().unwrap_or("main");

    // 5. Plan freshness check (plan←main)
    if let Some(plan_branch_name) = plan_branch {
        let plan_result = tokio::time::timeout(
            freshness_timeout,
            update_plan_from_main(
                repo_path,
                plan_branch_name,
                base_branch,
                project,
                task_id_str,
                app_handle,
            ),
        )
        .await;

        match plan_result {
            Err(_timeout) => {
                emit_freshness_activity(
                    activity_event_repo,
                    task_id_str,
                    "branch_freshness_blocked",
                    serde_json::json!({
                        "reason": "timeout",
                        "check": "plan_update",
                        "conflict_count": freshness.freshness_conflict_count,
                    }),
                )
                .await;
                return Err(FreshnessAction::ExecutionBlocked {
                    reason: format!(
                        "update_plan_from_main timed out after {}s",
                        config.branch_freshness_timeout_secs
                    ),
                });
            }
            Ok(PlanUpdateResult::Conflicts { conflict_files }) => {
                // Single-increment per ensure_branches_fresh() call
                freshness.freshness_conflict_count += 1;
                if freshness.freshness_conflict_count > config.freshness_max_conflict_retries {
                    emit_freshness_activity(
                        activity_event_repo,
                        task_id_str,
                        "branch_freshness_blocked",
                        serde_json::json!({
                            "reason": "retry_cap_exceeded",
                            "conflict_count": freshness.freshness_conflict_count,
                            "max_retries": config.freshness_max_conflict_retries,
                        }),
                    )
                    .await;
                    return Err(FreshnessAction::ExecutionBlocked {
                        reason: format!(
                            "Freshness conflict retry cap exceeded ({}/{})",
                            freshness.freshness_conflict_count,
                            config.freshness_max_conflict_retries
                        ),
                    });
                }

                let conflict_files_str: Vec<String> = conflict_files
                    .iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();

                freshness.branch_freshness_conflict = true;
                freshness.freshness_origin_state = Some(origin_state.to_string());
                freshness.plan_update_conflict = true;
                freshness.source_update_conflict = false;
                freshness.conflict_files = conflict_files_str.clone();
                freshness.source_branch = task.task_branch.clone();
                freshness.target_branch = Some(plan_branch_name.to_string());

                emit_freshness_activity(
                    activity_event_repo,
                    task_id_str,
                    "branch_freshness_conflict",
                    serde_json::json!({
                        "conflict_type": "plan_update",
                        "conflict_files": conflict_files_str,
                        "retry_count": freshness.freshness_conflict_count,
                        "origin_state": origin_state,
                    }),
                )
                .await;

                return Err(FreshnessAction::RouteToMerging {
                    conflict_files: conflict_files_str,
                    conflict_type: "plan_update",
                    freshness_metadata: freshness,
                });
            }
            Ok(PlanUpdateResult::Error(e)) => {
                warn!(
                    task_id = task_id_str,
                    error = %e,
                    "update_plan_from_main failed (non-fatal) — continuing to source check"
                );
            }
            Ok(
                PlanUpdateResult::AlreadyUpToDate
                | PlanUpdateResult::Updated
                | PlanUpdateResult::NotPlanBranch,
            ) => {
                // Plan is fresh (or not applicable) — continue to source check
            }
        }
    }

    // 6. Source freshness check (task←plan)
    let source_branch = task.task_branch.as_deref().unwrap_or("");
    let target_branch = plan_branch.unwrap_or(base_branch);

    if source_branch.is_empty() {
        // No task branch assigned yet — skip source check
        info!(
            task_id = task_id_str,
            "No task branch set — skipping source freshness check"
        );
    } else {
        let source_result = tokio::time::timeout(
            freshness_timeout,
            update_source_from_target(
                repo_path,
                source_branch,
                target_branch,
                project,
                task_id_str,
                app_handle,
            ),
        )
        .await;

        match source_result {
            Err(_timeout) => {
                emit_freshness_activity(
                    activity_event_repo,
                    task_id_str,
                    "branch_freshness_blocked",
                    serde_json::json!({
                        "reason": "timeout",
                        "check": "source_update",
                        "conflict_count": freshness.freshness_conflict_count,
                    }),
                )
                .await;
                return Err(FreshnessAction::ExecutionBlocked {
                    reason: format!(
                        "update_source_from_target timed out after {}s",
                        config.branch_freshness_timeout_secs
                    ),
                });
            }
            Ok(SourceUpdateResult::Conflicts { conflict_files }) => {
                // Single-increment per ensure_branches_fresh() call
                freshness.freshness_conflict_count += 1;
                if freshness.freshness_conflict_count > config.freshness_max_conflict_retries {
                    emit_freshness_activity(
                        activity_event_repo,
                        task_id_str,
                        "branch_freshness_blocked",
                        serde_json::json!({
                            "reason": "retry_cap_exceeded",
                            "conflict_count": freshness.freshness_conflict_count,
                            "max_retries": config.freshness_max_conflict_retries,
                        }),
                    )
                    .await;
                    return Err(FreshnessAction::ExecutionBlocked {
                        reason: format!(
                            "Freshness conflict retry cap exceeded ({}/{})",
                            freshness.freshness_conflict_count,
                            config.freshness_max_conflict_retries
                        ),
                    });
                }

                let conflict_files_str: Vec<String> = conflict_files
                    .iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();

                freshness.branch_freshness_conflict = true;
                freshness.freshness_origin_state = Some(origin_state.to_string());
                freshness.plan_update_conflict = false;
                freshness.source_update_conflict = true;
                freshness.conflict_files = conflict_files_str.clone();
                freshness.source_branch = Some(source_branch.to_string());
                freshness.target_branch = Some(target_branch.to_string());

                emit_freshness_activity(
                    activity_event_repo,
                    task_id_str,
                    "branch_freshness_conflict",
                    serde_json::json!({
                        "conflict_type": "source_update",
                        "conflict_files": conflict_files_str,
                        "retry_count": freshness.freshness_conflict_count,
                        "origin_state": origin_state,
                    }),
                )
                .await;

                return Err(FreshnessAction::RouteToMerging {
                    conflict_files: conflict_files_str,
                    conflict_type: "source_update",
                    freshness_metadata: freshness,
                });
            }
            Ok(SourceUpdateResult::Error(e)) => {
                warn!(
                    task_id = task_id_str,
                    error = %e,
                    "update_source_from_target failed (non-fatal) — continuing"
                );
            }
            Ok(SourceUpdateResult::AlreadyUpToDate | SourceUpdateResult::Updated) => {
                // Source is fresh — continue
            }
        }
    }

    // 7. Both checks passed — update timestamp and reset conflict count
    freshness.last_freshness_check_at = Some(Utc::now().to_rfc3339());
    freshness.branch_freshness_conflict = false;
    freshness.freshness_conflict_count = 0;

    emit_freshness_activity(
        activity_event_repo,
        task_id_str,
        "branch_freshness_checked",
        serde_json::json!({ "status": "passed" }),
    )
    .await;

    Ok(freshness)
}

/// Returns true if the git worktree has uncommitted changes.
async fn is_worktree_dirty(path: &Path) -> bool {
    match git_cmd::run(&["status", "--porcelain", "-z"], path).await {
        Ok(output) => !output.stdout.is_empty(),
        Err(e) => {
            warn!(path = %path.display(), error = %e, "Failed to check worktree status — assuming clean");
            false
        }
    }
}

/// Emit a freshness-related activity event. Non-fatal: logs warning on failure.
async fn emit_freshness_activity(
    activity_event_repo: Option<&Arc<dyn ActivityEventRepository>>,
    task_id_str: &str,
    event_kind: &str,
    metadata: serde_json::Value,
) {
    let Some(repo) = activity_event_repo else {
        return;
    };
    let tid = TaskId::from_string(task_id_str.to_string());
    let content = match event_kind {
        "branch_freshness_checked" => "Branch freshness check passed".to_string(),
        "branch_freshness_conflict" => format!(
            "Branch freshness conflict detected ({})",
            metadata
                .get("conflict_type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
        ),
        "branch_freshness_skipped" => {
            "Branch freshness check skipped (recently checked)".to_string()
        }
        "branch_freshness_blocked" => format!(
            "Branch freshness blocked: {}",
            metadata
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
        ),
        _ => event_kind.to_string(),
    };
    let metadata_str = serde_json::json!({
        "event_kind": event_kind,
        "details": metadata,
    })
    .to_string();
    let event = ActivityEvent::new_task_event(tid, ActivityEventType::System, content)
        .with_role(ActivityEventRole::System)
        .with_metadata(metadata_str);
    if let Err(e) = repo.save(event).await {
        tracing::warn!(
            task_id = task_id_str,
            event_kind = event_kind,
            error = %e,
            "Failed to save freshness activity event (non-fatal)"
        );
    }
}
