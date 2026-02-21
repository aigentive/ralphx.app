// Merge helper utilities: path computation, metadata parsing, branch resolution
//
// Extracted from side_effects.rs — pure helpers with no side effects beyond metadata mutation.

use std::path::Path;
use std::sync::Arc;

use crate::application::GitService;
use crate::domain::entities::InternalStatus;
use crate::domain::entities::{PlanBranchStatus, Project, Task, TaskCategory, TaskId};
use crate::domain::repositories::{PlanBranchRepository, TaskRepository};
use crate::error::AppResult;

// ===== Pre-merge validation =====

/// Errors surfaced when pre-merge preconditions fail for a `plan_merge` task.
///
/// Each variant carries a human-readable message suitable for storing in task metadata
/// and displaying in the UI as an actionable error.
#[derive(Debug, PartialEq)]
pub(crate) enum PreMergeValidationError {
    /// `plan_branch_repo` is not wired in the service context.
    PlanBranchRepoNotWired,
    /// The `PlanBranch` record for this task's session has `status != Active`.
    PlanBranchNotActive { status: String },
    /// The feature branch does not exist in the git repository.
    FeatureBranchMissing { branch_name: String },
}

impl PreMergeValidationError {
    /// A short, human-readable error message for UI display.
    pub(crate) fn message(&self) -> String {
        match self {
            PreMergeValidationError::PlanBranchRepoNotWired => {
                "Plan branch repository is not configured. \
                 This is a server configuration error — please restart the application."
                    .to_string()
            }
            PreMergeValidationError::PlanBranchNotActive { status } => {
                format!(
                    "The plan branch is not active (current status: {status}). \
                     It may have already been merged or was abandoned. \
                     Check the plan branch status before retrying."
                )
            }
            PreMergeValidationError::FeatureBranchMissing { branch_name } => {
                format!(
                    "Feature branch '{branch_name}' does not exist in git. \
                     It may have been deleted. Re-create the branch or reset the plan to retry."
                )
            }
        }
    }

    /// A short machine-readable error code for metadata storage.
    pub(crate) fn error_code(&self) -> &'static str {
        match self {
            PreMergeValidationError::PlanBranchRepoNotWired => "plan_branch_repo_not_wired",
            PreMergeValidationError::PlanBranchNotActive { .. } => "plan_branch_not_active",
            PreMergeValidationError::FeatureBranchMissing { .. } => "feature_branch_missing",
        }
    }
}

/// Validate preconditions required before attempting a `plan_merge` task merge.
///
/// Checks:
/// 1. `plan_branch_repo` is wired (Some) in the context
/// 2. The `PlanBranch` record for this task has `status == Active`
/// 3. The feature branch referenced by the `PlanBranch` exists in git
///
/// Returns `Ok(())` when all checks pass, `Err(PreMergeValidationError)` on first failure.
/// Callers should transition the task to `MergeIncomplete` with the error's `message()` on failure.
///
/// Non-`plan_merge` tasks always pass (returns `Ok(())`).
pub(crate) async fn validate_plan_merge_preconditions(
    task: &Task,
    project: &Project,
    plan_branch_repo: &Option<Arc<dyn PlanBranchRepository>>,
) -> Result<(), PreMergeValidationError> {
    // Only validate plan_merge category tasks
    if task.category != TaskCategory::PlanMerge {
        return Ok(());
    }

    // Check 1: plan_branch_repo must be wired
    let Some(ref pb_repo) = plan_branch_repo else {
        return Err(PreMergeValidationError::PlanBranchRepoNotWired);
    };

    // Check 2: PlanBranch must exist and have Active status
    let plan_branch = pb_repo.get_by_merge_task_id(&task.id).await
        .ok()
        .flatten();

    let Some(pb) = plan_branch else {
        // No PlanBranch record for this merge task — treat as not active
        return Err(PreMergeValidationError::PlanBranchNotActive {
            status: "not_found".to_string(),
        });
    };

    if pb.status != PlanBranchStatus::Active {
        return Err(PreMergeValidationError::PlanBranchNotActive {
            status: pb.status.to_string(),
        });
    }

    // Check 3: Feature branch must exist in git
    let repo_path = Path::new(&project.working_directory);
    if !GitService::branch_exists(repo_path, &pb.branch_name).await {
        return Err(PreMergeValidationError::FeatureBranchMissing {
            branch_name: pb.branch_name.clone(),
        });
    }

    Ok(())
}

/// Convert project name to a URL-safe slug for branch naming
pub(super) fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

/// Truncate a string to at most `max_bytes` bytes at a valid char boundary.
pub(super) fn truncate_str(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Expand `~/` prefix to the user's home directory
pub(super) fn expand_home(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{}/{}", home, stripped);
        }
    }
    path.to_string()
}

/// Compute the worktree path for a merge operation.
///
/// Convention: `{worktree_parent}/{slug}/merge-{task_id}`
/// This is separate from the task worktree (`task-{task_id}`) to allow
/// the merge to happen in isolation while the task worktree is deleted.
pub(super) fn compute_merge_worktree_path(project: &Project, task_id: &str) -> String {
    let worktree_parent = project
        .worktree_parent_directory
        .as_deref()
        .unwrap_or("~/ralphx-worktrees");
    let expanded = expand_home(worktree_parent);
    format!("{}/{}/merge-{}", expanded, slugify(&project.name), task_id)
}

/// Compute the worktree path for a rebase operation.
///
/// Convention: `{worktree_parent}/{slug}/rebase-{task_id}`
/// This is separate from the merge worktree (`merge-{task_id}`) to allow
/// the rebase and merge steps to use different worktrees.
pub(super) fn compute_rebase_worktree_path(project: &Project, task_id: &str) -> String {
    let worktree_parent = project
        .worktree_parent_directory
        .as_deref()
        .unwrap_or("~/ralphx-worktrees");
    let expanded = expand_home(worktree_parent);
    format!("{}/{}/rebase-{}", expanded, slugify(&project.name), task_id)
}

/// Compute the worktree path for a source-update operation (merging target into source branch).
///
/// Convention: `{worktree_parent}/{slug}/source-update-{task_id}`
/// This is a short-lived worktree used only to bring the feature/task branch up-to-date
/// with its target branch before the actual merge runs.
pub(super) fn compute_source_update_worktree_path(project: &Project, task_id: &str) -> String {
    let worktree_parent = project
        .worktree_parent_directory
        .as_deref()
        .unwrap_or("~/ralphx-worktrees");
    let expanded = expand_home(worktree_parent);
    format!("{}/{}/source-update-{}", expanded, slugify(&project.name), task_id)
}

/// Compute the worktree path for a plan-update operation (merging main into plan branch).
///
/// Convention: `{worktree_parent}/{slug}/plan-update-{task_id}`
/// This is a short-lived worktree used only to bring the plan branch up-to-date with main
/// before the actual task→plan merge runs.
pub(super) fn compute_plan_update_worktree_path(project: &Project, task_id: &str) -> String {
    let worktree_parent = project
        .worktree_parent_directory
        .as_deref()
        .unwrap_or("~/ralphx-worktrees");
    let expanded = expand_home(worktree_parent);
    format!("{}/{}/plan-update-{}", expanded, slugify(&project.name), task_id)
}

/// Extract a task ID from a merge worktree path.
///
/// Merge worktree paths follow the convention: `{parent}/{slug}/merge-{task_id}`
/// Returns `Some(task_id)` if the path matches, `None` otherwise.
pub(super) fn extract_task_id_from_merge_path(path: &str) -> Option<&str> {
    let basename = path.rsplit('/').next()?;
    basename.strip_prefix("merge-")
}

/// Check if a task is currently in an active merge state.
///
/// Only covers `PendingMerge` and `Merging` where a merge worktree is actively in use.
/// Excludes `MergeIncomplete` and `MergeConflict` (human-waiting states) to allow
/// other tasks to clean up orphaned worktrees when merging to the same branch.
pub(super) async fn is_task_in_merge_workflow(
    task_repo: &Arc<dyn TaskRepository>,
    task_id_str: &str,
) -> bool {
    let task_id = TaskId::from_string(task_id_str.to_string());
    match task_repo.get_by_id(&task_id).await {
        Ok(Some(task)) => matches!(
            task.internal_status,
            InternalStatus::PendingMerge | InternalStatus::Merging
        ),
        _ => false,
    }
}

/// Check if a task's merge would target the given branch.
///
/// Resolves the task's merge target branch the same way `resolve_merge_branches()` does,
/// then compares against `target_branch`. Used by the concurrent merge guard to detect
/// tasks that would conflict with the same target.
pub(super) async fn task_targets_branch(
    task: &Task,
    project: &Project,
    plan_branch_repo: &Option<Arc<dyn PlanBranchRepository>>,
    target_branch: &str,
) -> bool {
    let (_, resolved_target) = resolve_merge_branches(task, project, plan_branch_repo).await;
    resolved_target == target_branch
}

/// Parse a task's metadata JSON string into a `serde_json::Value`.
///
/// Returns `None` if the task has no metadata or if parsing fails.
pub(crate) fn parse_metadata(task: &Task) -> Option<serde_json::Value> {
    task.metadata
        .as_ref()
        .and_then(|m| serde_json::from_str(m).ok())
}

/// Check if a task has the `merge_deferred` flag set in its metadata.
pub(crate) fn has_merge_deferred_metadata(task: &Task) -> bool {
    parse_metadata(task)
        .and_then(|v| v.get("merge_deferred")?.as_bool())
        .unwrap_or(false)
}

/// Check if a task has the `branch_missing` flag set in its metadata.
pub(crate) fn has_branch_missing_metadata(task: &Task) -> bool {
    parse_metadata(task)
        .and_then(|v| v.get("branch_missing")?.as_bool())
        .unwrap_or(false)
}

/// Check if task metadata indicates prior validation failures that should block
/// fast-path merge completion (used by `check_already_merged` and
/// `recover_deleted_source_branch` to avoid completing merges with broken code).
///
/// Returns `true` if ANY of these are set:
/// - `merge_commit_unrevertable`: prior merge commit couldn't be reverted
/// - `merge_failure_source` == `"validation_failed"`: prior merge failed validation
/// - `validation_revert_count` > 0: prior validation-triggered reverts
pub(super) fn has_prior_validation_failure(task: &Task) -> bool {
    let Some(meta) = parse_metadata(task) else {
        return false;
    };
    if meta.get("merge_commit_unrevertable").and_then(|v| v.as_bool()).unwrap_or(false) {
        return true;
    }
    if meta.get("merge_failure_source")
        .and_then(|v| v.as_str())
        .map(|s| s == "validation_failed")
        .unwrap_or(false)
    {
        return true;
    }
    if meta.get("validation_revert_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) > 0
    {
        return true;
    }
    false
}

/// Check if a task has the `main_merge_deferred` flag set in its metadata.
/// This flag indicates a merge to main was deferred because agents were running.
pub(crate) fn has_main_merge_deferred_metadata(task: &Task) -> bool {
    parse_metadata(task)
        .and_then(|v| v.get("main_merge_deferred")?.as_bool())
        .unwrap_or(false)
}

/// Set the `main_merge_deferred` flag and `main_merge_deferred_at` timestamp in a task's metadata.
///
/// This is called when a merge to main is deferred because agents are running.
/// Mutates the task in-place, creating metadata if it doesn't exist.
pub(crate) fn set_main_merge_deferred_metadata(task: &mut Task) {
    let mut meta = parse_metadata(task).unwrap_or_else(|| serde_json::json!({}));
    if let Some(obj) = meta.as_object_mut() {
        obj.insert("main_merge_deferred".to_string(), serde_json::json!(true));
        obj.insert(
            "main_merge_deferred_at".to_string(),
            serde_json::json!(chrono::Utc::now().to_rfc3339()),
        );
    }
    task.metadata = Some(meta.to_string());
}

/// Clear the `main_merge_deferred` and `main_merge_deferred_at` fields from a task's metadata.
///
/// Called when retrying a main-merge-deferred task after agents go idle.
/// Mutates the task in-place. If the metadata becomes an empty object after removal,
/// clears metadata entirely.
/// TODO(Phase 3): Used by try_retry_main_merges() when all agents go idle
#[allow(dead_code)]
pub(crate) fn clear_main_merge_deferred_metadata(task: &mut Task) {
    let Some(mut meta) = parse_metadata(task) else {
        return;
    };
    if let Some(obj) = meta.as_object_mut() {
        obj.remove("main_merge_deferred");
        obj.remove("main_merge_deferred_at");
        if obj.is_empty() {
            task.metadata = None;
        } else {
            task.metadata = Some(meta.to_string());
        }
    }
}

/// Clear the `merge_deferred` and `merge_deferred_at` fields from a task's metadata.
///
/// Mutates the task in-place. If the metadata becomes an empty object after removal,
/// clears metadata entirely.
pub(crate) fn clear_merge_deferred_metadata(task: &mut Task) {
    let Some(mut meta) = parse_metadata(task) else {
        return;
    };
    if let Some(obj) = meta.as_object_mut() {
        obj.remove("merge_deferred");
        obj.remove("merge_deferred_at");
        if obj.is_empty() {
            task.metadata = None;
        } else {
            task.metadata = Some(meta.to_string());
        }
    }
}

/// Default timeout in seconds after which a deferred merge is forced to retry.
pub(crate) const DEFERRED_MERGE_TIMEOUT_SECONDS: i64 = 120;

/// Check if a `merge_deferred` task has exceeded the configured timeout.
///
/// Returns true if the `merge_deferred_at` timestamp in metadata is older than
/// `DEFERRED_MERGE_TIMEOUT_SECONDS`. Returns false if the timestamp is missing or unparseable
/// (no timeout enforcement in that case — the reconciliation watchdog handles it instead).
pub(crate) fn is_merge_deferred_timed_out(task: &Task) -> bool {
    let deferred_at = parse_metadata(task)
        .and_then(|v| v.get("merge_deferred_at")?.as_str().map(String::from));

    let Some(deferred_at_str) = deferred_at else {
        return false;
    };

    let Ok(deferred_at) = chrono::DateTime::parse_from_rfc3339(&deferred_at_str) else {
        return false;
    };

    let elapsed = chrono::Utc::now().signed_duration_since(deferred_at.with_timezone(&chrono::Utc));
    elapsed.num_seconds() >= DEFERRED_MERGE_TIMEOUT_SECONDS
}

/// Check if a `main_merge_deferred` task has exceeded the configured timeout.
///
/// Returns true if the `main_merge_deferred_at` timestamp in metadata is older than
/// `DEFERRED_MERGE_TIMEOUT_SECONDS`. Returns false if the timestamp is missing or unparseable.
pub(crate) fn is_main_merge_deferred_timed_out(task: &Task) -> bool {
    let deferred_at = parse_metadata(task)
        .and_then(|v| v.get("main_merge_deferred_at")?.as_str().map(String::from));

    let Some(deferred_at_str) = deferred_at else {
        return false;
    };

    let Ok(deferred_at) = chrono::DateTime::parse_from_rfc3339(&deferred_at_str) else {
        return false;
    };

    let elapsed = chrono::Utc::now().signed_duration_since(deferred_at.with_timezone(&chrono::Utc));
    elapsed.num_seconds() >= DEFERRED_MERGE_TIMEOUT_SECONDS
}

/// Set the `trigger_origin` field in a task's metadata.
///
/// Valid origins: "scheduler", "revision", "recovery", "retry", "qa".
/// Mutates the task in-place, creating metadata if it doesn't exist.
pub(crate) fn set_trigger_origin(task: &mut Task, origin: &str) {
    let mut meta = parse_metadata(task).unwrap_or_else(|| serde_json::json!({}));
    if let Some(obj) = meta.as_object_mut() {
        obj.insert("trigger_origin".to_string(), serde_json::json!(origin));
    }
    task.metadata = Some(meta.to_string());
}

/// Get the `trigger_origin` field from a task's metadata.
///
/// Returns the origin string if present, otherwise `None`.
pub(crate) fn get_trigger_origin(task: &Task) -> Option<String> {
    parse_metadata(task).and_then(|v| v.get("trigger_origin")?.as_str().map(String::from))
}

/// Clear the `trigger_origin` field from a task's metadata.
///
/// Mutates the task in-place. If the metadata becomes an empty object after removal,
/// clears metadata entirely.
pub(crate) fn clear_trigger_origin(task: &mut Task) {
    let Some(mut meta) = parse_metadata(task) else {
        return;
    };
    if let Some(obj) = meta.as_object_mut() {
        obj.remove("trigger_origin");
        if obj.is_empty() {
            task.metadata = None;
        } else {
            task.metadata = Some(meta.to_string());
        }
    }
}

/// Set conflict metadata in a task's metadata.
///
/// Stores:
/// - `conflict_files`: array of file paths with conflicts
/// - `conflict_snapshot_at`: ISO 8601 timestamp when conflicts were detected
/// - `conflict_detected_by`: "programmatic" (system) or "agent" (via report_conflict)
///
/// Mutates the task in-place, creating metadata if it doesn't exist.
pub(crate) fn set_conflict_metadata(
    task: &mut Task,
    conflict_files: &[String],
    detected_by: &str,
) {
    let mut meta = parse_metadata(task).unwrap_or_else(|| serde_json::json!({}));
    if let Some(obj) = meta.as_object_mut() {
        obj.insert(
            "conflict_files".to_string(),
            serde_json::json!(conflict_files),
        );
        obj.insert(
            "conflict_snapshot_at".to_string(),
            serde_json::json!(chrono::Utc::now().to_rfc3339()),
        );
        obj.insert(
            "conflict_detected_by".to_string(),
            serde_json::json!(detected_by),
        );
    }
    task.metadata = Some(meta.to_string());
}

/// Get the `revision_count` from a task's metadata.
///
/// Returns the current revision cycle count, or 0 if not set.
pub(crate) fn get_revision_count(task: &Task) -> u32 {
    parse_metadata(task)
        .and_then(|v| v.get("revision_count")?.as_u64())
        .unwrap_or(0) as u32
}

/// Increment the `revision_count` in a task's metadata.
///
/// Mutates the task in-place, creating metadata if it doesn't exist.
/// Returns the new revision count after incrementing.
pub(crate) fn increment_revision_count(task: &mut Task) -> u32 {
    let mut meta = parse_metadata(task).unwrap_or_else(|| serde_json::json!({}));
    let current = meta
        .get("revision_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let new_count = current + 1;
    if let Some(obj) = meta.as_object_mut() {
        obj.insert("revision_count".to_string(), serde_json::json!(new_count));
    }
    task.metadata = Some(meta.to_string());
    new_count
}

/// Resolve the base branch for a task's working branch.
///
/// If the task belongs to a plan with an active feature branch, returns the feature
/// branch name so the task branch is created from it. Otherwise falls back to the
/// project's base branch.
pub(super) async fn resolve_task_base_branch(
    task: &Task,
    project: &Project,
    plan_branch_repo: &Option<Arc<dyn PlanBranchRepository>>,
) -> String {
    let default = project.base_branch.as_deref().unwrap_or("main").to_string();

    let Some(ref plan_branch_repo) = plan_branch_repo else {
        return default;
    };
    let Some(ref session_id) = task.ideation_session_id else {
        return default;
    };

    match plan_branch_repo.get_by_session_id(session_id).await {
        Ok(Some(pb)) if pb.status == PlanBranchStatus::Active => {
            let repo_path = Path::new(&project.working_directory);
            // Lazily create git branch on first task execution
            if !GitService::branch_exists(repo_path, &pb.branch_name).await {
                match GitService::create_feature_branch(
                    repo_path,
                    &pb.branch_name,
                    &pb.source_branch,
                )
                .await
                {
                    Ok(_) => {
                        tracing::info!(
                            branch = %pb.branch_name,
                            source = %pb.source_branch,
                            "Created deferred plan branch"
                        );
                    }
                    Err(e) => {
                        // Race condition: another task may have created it concurrently
                        if GitService::branch_exists(repo_path, &pb.branch_name).await {
                            tracing::info!(
                                branch = %pb.branch_name,
                                "Deferred plan branch created by concurrent task"
                            );
                        } else {
                            tracing::warn!(
                                error = %e,
                                branch = %pb.branch_name,
                                "Failed to create deferred plan branch, falling back to project base"
                            );
                            return default;
                        }
                    }
                }
            }
            tracing::info!(
                task_id = task.id.as_str(),
                feature_branch = %pb.branch_name,
                "Resolved task base branch to plan feature branch"
            );
            pb.branch_name
        }
        Ok(Some(pb)) if pb.status == PlanBranchStatus::Merged => {
            // Task is being re-executed after the plan was merged and its branch deleted.
            // Recreate the plan branch from source_branch if missing, then reset DB status
            // back to Active so subsequent tasks use this branch.
            let repo_path = Path::new(&project.working_directory);
            if !GitService::branch_exists(repo_path, &pb.branch_name).await {
                match GitService::create_feature_branch(
                    repo_path,
                    &pb.branch_name,
                    &pb.source_branch,
                )
                .await
                {
                    Ok(_) => {
                        tracing::info!(
                            branch = %pb.branch_name,
                            source = %pb.source_branch,
                            "Recreated merged plan branch for re-executed task"
                        );
                        // Reset DB status to Active so subsequent tasks use this branch
                        if let Err(e) = plan_branch_repo
                            .update_status(&pb.id, PlanBranchStatus::Active)
                            .await
                        {
                            tracing::warn!(error = %e, branch = %pb.branch_name, "Failed to reset plan branch status to Active");
                        }
                    }
                    Err(e) => {
                        // Race condition: another task may have created it concurrently
                        if GitService::branch_exists(repo_path, &pb.branch_name).await {
                            tracing::info!(
                                branch = %pb.branch_name,
                                "Merged plan branch recreated by concurrent task"
                            );
                            if let Err(e) = plan_branch_repo
                                .update_status(&pb.id, PlanBranchStatus::Active)
                                .await
                            {
                                tracing::warn!(error = %e, branch = %pb.branch_name, "Failed to reset plan branch status to Active (concurrent)");
                            }
                        } else {
                            tracing::warn!(
                                error = %e,
                                branch = %pb.branch_name,
                                "Failed to recreate merged plan branch, falling back to project base"
                            );
                            return default;
                        }
                    }
                }
            } else {
                // Branch exists in git but DB still says Merged — reset status only
                tracing::info!(
                    branch = %pb.branch_name,
                    "Plan branch exists in git but DB status is Merged — resetting to Active"
                );
                if let Err(e) = plan_branch_repo
                    .update_status(&pb.id, PlanBranchStatus::Active)
                    .await
                {
                    tracing::warn!(error = %e, branch = %pb.branch_name, "Failed to reset plan branch status to Active");
                }
            }
            tracing::info!(
                task_id = task.id.as_str(),
                feature_branch = %pb.branch_name,
                "Resolved task base branch to recreated plan feature branch"
            );
            pb.branch_name
        }
        _ => default,
    }
}

/// Resolve the source and target branches for a merge operation.
///
/// Returns `(source_branch, target_branch)`:
/// - **Merge task** (task is `plan_branches.merge_task_id`): merge feature branch into project base
/// - **Plan task with feature branch**: merge task branch into feature branch
/// - **Regular task**: merge task branch into project base branch
pub async fn resolve_merge_branches(
    task: &Task,
    project: &Project,
    plan_branch_repo: &Option<Arc<dyn PlanBranchRepository>>,
) -> (String, String) {
    let base_branch = project.base_branch.as_deref().unwrap_or("main").to_string();
    let task_branch = task.task_branch.clone().unwrap_or_default();

    tracing::debug!(
        task_id = task.id.as_str(),
        category = %task.category,
        plan_branch_repo_available = plan_branch_repo.is_some(),
        ideation_session_id = ?task.ideation_session_id.as_ref().map(|s| s.as_str()),
        task_branch = %task_branch,
        base_branch = %base_branch,
        "resolve_merge_branches: entry"
    );

    let Some(ref plan_branch_repo) = plan_branch_repo else {
        if task.category == TaskCategory::PlanMerge {
            tracing::warn!(
                task_id = task.id.as_str(),
                "resolve_merge_branches: plan_branch_repo is None for plan_merge task — \
                 merge branch resolution will fall back to task_branch/base_branch"
            );
        }
        return (task_branch, base_branch);
    };

    // Check if this task IS the merge task for a plan branch
    if let Ok(Some(pb)) = plan_branch_repo.get_by_merge_task_id(&task.id).await {
        if pb.status != PlanBranchStatus::Active {
            tracing::warn!(
                task_id = task.id.as_str(),
                feature_branch = %pb.branch_name,
                plan_branch_status = ?pb.status,
                "Merge task: plan branch is not Active, but still using it as source \
                 to avoid incorrect merge direction"
            );
        }
        tracing::debug!(
            task_id = task.id.as_str(),
            feature_branch = %pb.branch_name,
            base_branch = %base_branch,
            "Merge task: merging feature branch into base"
        );
        return (pb.branch_name, base_branch);
    }

    // Check if this task belongs to a plan with a feature branch
    if let Some(ref session_id) = task.ideation_session_id {
        match plan_branch_repo.get_by_session_id(session_id).await {
            Ok(Some(pb)) => {
                if pb.status == PlanBranchStatus::Active {
                    tracing::debug!(
                        task_id = task.id.as_str(),
                        task_branch = %task_branch,
                        feature_branch = %pb.branch_name,
                        "Plan task: merging task branch into feature branch"
                    );
                    return (task_branch, pb.branch_name);
                }
                // Plan branch exists but isn't Active — still use it as the target.
                // Falling through to base_branch would merge task→main instead of task→plan,
                // which is incorrect for tasks that belong to a plan.
                tracing::warn!(
                    task_id = task.id.as_str(),
                    task_branch = %task_branch,
                    feature_branch = %pb.branch_name,
                    plan_branch_status = ?pb.status,
                    "Plan task: plan branch is not Active, but still using it as merge target \
                     to avoid incorrect task→main merge"
                );
                return (task_branch, pb.branch_name);
            }
            Ok(None) => {
                tracing::warn!(
                    task_id = task.id.as_str(),
                    session_id = session_id.as_str(),
                    "Plan task: no plan branch found for session_id — falling back to base branch"
                );
            }
            Err(e) => {
                tracing::error!(
                    task_id = task.id.as_str(),
                    session_id = session_id.as_str(),
                    error = %e,
                    "Plan task: failed to look up plan branch — falling back to base branch"
                );
            }
        }
    }

    (task_branch, base_branch)
}

/// Discover and re-attach an orphaned task branch to a task record.
///
/// When tasks recover from Failed/Critical states and retry merge, the task may have
/// `task_branch` set to `None` even though the git branch exists with committed work.
/// This helper:
/// 1. Early-returns `Ok(false)` if `task.task_branch` is already set
/// 2. Constructs the expected branch name: `ralphx/{project_slug}/task-{task_id}`
/// 3. Checks if the branch exists in the git repository
/// 4. If found: updates `task.task_branch`, calls `task.touch()`, persists via `task_repo.update()`
/// 5. Returns `Ok(true)` if branch was discovered and attached, `Ok(false)` otherwise
///
/// This is called before `resolve_merge_branches()` to ensure merge operations have
/// a valid source branch reference.
pub(super) async fn discover_and_attach_task_branch(
    task: &mut Task,
    project: &Project,
    task_repo: &Arc<dyn TaskRepository>,
) -> AppResult<bool> {
    // Early return if task_branch already set
    if task.task_branch.is_some() {
        return Ok(false);
    }

    // Construct expected branch name using same convention as on_enter_states.rs:92
    let branch_name = format!(
        "ralphx/{}/task-{}",
        slugify(&project.name),
        task.id.as_str()
    );

    // Check if branch exists in the repository
    let repo_path = Path::new(&project.working_directory);
    if !GitService::branch_exists(repo_path, &branch_name).await {
        return Ok(false);
    }

    // Branch exists - re-attach it to the task record
    tracing::info!(
        task_id = task.id.as_str(),
        branch = %branch_name,
        "Discovered orphaned task branch - re-attaching to task record"
    );

    task.task_branch = Some(branch_name.clone());
    task.touch();
    task_repo.update(task).await?;

    tracing::info!(
        task_id = task.id.as_str(),
        branch = %branch_name,
        "Successfully re-attached orphaned task branch"
    );

    Ok(true)
}
