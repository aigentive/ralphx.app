use crate::domain::entities::Task;
use crate::utils::truncate_str;

use super::{ValidationFailure, ValidationLogEntry};

/// Format validation failures as a JSON metadata string for MergeIncomplete.
pub(crate) fn format_validation_error_metadata(
    failures: &[ValidationFailure],
    log: &[ValidationLogEntry],
    source_branch: &str,
    target_branch: &str,
) -> String {
    let failure_details: Vec<serde_json::Value> = failures
        .iter()
        .map(|f| {
            serde_json::json!({
                "command": f.command,
                "path": f.path,
                "exit_code": f.exit_code,
                "stderr": truncate_str(&f.stderr, 2000),
            })
        })
        .collect();

    serde_json::json!({
        "error": format!("Merge validation failed: {} command(s) failed", failures.len()),
        "validation_failures": failure_details,
        "validation_log": log,
        "source_branch": source_branch,
        "target_branch": target_branch,
    })
    .to_string()
}

/// Check if task metadata has the skip_validation flag set, and clear it (one-shot).
pub(crate) fn take_skip_validation_flag(task: &mut Task) -> bool {
    let Some(meta_str) = task.metadata.as_ref() else {
        return false;
    };
    let Ok(mut val) = serde_json::from_str::<serde_json::Value>(meta_str) else {
        return false;
    };
    let flag = val
        .get("skip_validation")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if flag {
        if let Some(obj) = val.as_object_mut() {
            obj.remove("skip_validation");
            task.metadata = Some(val.to_string());
        }
    }
    flag
}

/// Format validation warnings as a JSON metadata string for Warn mode.
/// Stores the log but allows merge to proceed.
pub(crate) fn format_validation_warn_metadata(
    log: &[ValidationLogEntry],
    source_branch: &str,
    target_branch: &str,
) -> String {
    serde_json::json!({
        "validation_log": log,
        "validation_warnings": true,
        "source_branch": source_branch,
        "target_branch": target_branch,
    })
    .to_string()
}

/// Extract cached validation log from task metadata if the source branch SHA matches.
///
/// Returns `Some(entries)` when the previous validation ran against the same source SHA,
/// meaning the branch code has not changed and previously-passed checks can be skipped.
///
/// Note: Caching is effective in worktree mode. In local mode, rebase rewrites the source
/// branch SHA on each retry, so cache hits are rare.
pub(crate) fn extract_cached_validation(
    task: &Task,
    current_sha: &str,
) -> Option<Vec<ValidationLogEntry>> {
    let meta_str = task.metadata.as_ref()?;
    let val: serde_json::Value = serde_json::from_str(meta_str).ok()?;
    let stored_sha = val.get("validation_source_sha")?.as_str()?;
    if stored_sha != current_sha {
        return None;
    }
    let log_val = val.get("validation_log")?;
    serde_json::from_value::<Vec<ValidationLogEntry>>(log_val.clone()).ok()
}
