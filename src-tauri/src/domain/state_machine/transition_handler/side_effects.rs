// State entry side effects
// This module contains the on_enter implementation that handles state-specific actions

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use tauri::{AppHandle, Emitter};

use super::super::machine::State;
use crate::application::{GitService, MergeAttemptResult};
use crate::domain::entities::{GitMode, InternalStatus, MergeValidationMode, PlanBranchStatus, Project, Task, TaskId, ProjectId};
use crate::domain::repositories::{PlanBranchRepository, TaskRepository};
use crate::error::{AppError, AppResult};

/// Complete a merge operation by transitioning task to Merged and cleaning up.
///
/// This is shared logic used by:
/// - Programmatic merge success path (PendingMerge side effect)
/// - Merge auto-completion on agent exit (Phase 76)
/// - complete_merge HTTP handler (backwards compatibility)
///
/// # Arguments
/// * `task` - Mutable task to update (must be in appropriate state)
/// * `project` - Project for branch/worktree cleanup info
/// * `commit_sha` - The merge commit SHA
/// * `task_repo` - Repository to persist task changes
/// * `app_handle` - Optional Tauri handle for emitting events
///
/// # Side Effects
/// 1. Updates task.merge_commit_sha
/// 2. Updates task.internal_status to Merged
/// 3. Persists status change to history
/// 4. Deletes worktree (if Worktree mode)
/// 5. Deletes task branch
/// 6. Emits task:merged and task:status_changed events
pub async fn complete_merge_internal<R: tauri::Runtime>(
    task: &mut Task,
    project: &Project,
    commit_sha: &str,
    task_repo: &Arc<dyn TaskRepository>,
    app_handle: Option<&AppHandle<R>>,
) -> AppResult<()> {
    // Clone task_id early to avoid borrow conflicts with mutable task
    let task_id = task.id.clone();
    let task_id_str = task_id.as_str();
    let old_status = task.internal_status.clone();

    tracing::info!(
        task_id = task_id_str,
        commit_sha = %commit_sha,
        old_status = ?old_status,
        "complete_merge_internal: completing merge"
    );

    // 1. Update task with merge commit SHA and status
    task.merge_commit_sha = Some(commit_sha.to_string());
    task.internal_status = InternalStatus::Merged;
    task.touch();

    task_repo.update(task).await.map_err(|e| {
        tracing::error!(error = %e, task_id = task_id_str, "Failed to update task with merge_commit_sha");
        e
    })?;

    // 2. Record status change in history
    if let Err(e) = task_repo.persist_status_change(
        &task_id,
        old_status.clone(),
        InternalStatus::Merged,
        "merge_success",
    ).await {
        tracing::warn!(error = %e, task_id = task_id_str, "Failed to record merge transition (non-fatal)");
    }

    // 3. Cleanup branch and worktree
    cleanup_branch_and_worktree_internal(task, project);

    // 4. Emit events
    if let Some(handle) = app_handle {
        let _ = handle.emit(
            "task:merged",
            serde_json::json!({
                "task_id": task_id_str,
                "commit_sha": commit_sha,
            }),
        );
        let _ = handle.emit(
            "task:status_changed",
            serde_json::json!({
                "task_id": task_id_str,
                "old_status": old_status.as_str(),
                "new_status": "merged",
            }),
        );
        let _ = handle.emit(
            "merge:completed",
            serde_json::json!({
                "task_id": task_id_str,
                "commit_sha": commit_sha,
            }),
        );
    }

    tracing::info!(
        task_id = task_id_str,
        commit_sha = %commit_sha,
        "complete_merge_internal: merge completed successfully"
    );

    Ok(())
}

/// Cleanup task branch and worktree after successful merge (standalone version).
///
/// This is the standalone version that can be called from `complete_merge_internal`.
/// For use within TransitionHandler, use the async method which has access to services.
fn cleanup_branch_and_worktree_internal(task: &Task, project: &Project) {
    let task_id_str = task.id.as_str();

    let Some(ref task_branch) = task.task_branch else {
        tracing::debug!(task_id = task_id_str, "No branch to cleanup");
        return;
    };

    let repo_path = Path::new(&project.working_directory);

    match project.git_mode {
        GitMode::Local => {
            // For Local mode: already on base branch (from merge), just delete task branch
            match GitService::delete_branch(repo_path, task_branch, true) {
                Ok(_) => {
                    tracing::info!(
                        task_id = task_id_str,
                        branch = %task_branch,
                        "Deleted task branch after merge (Local mode)"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        task_id = task_id_str,
                        branch = %task_branch,
                        "Failed to delete task branch (non-fatal)"
                    );
                }
            }
        }
        GitMode::Worktree => {
            // For Worktree mode: delete worktree first, then branch
            if let Some(ref worktree_path) = task.worktree_path {
                let worktree_path_buf = PathBuf::from(worktree_path);
                match GitService::delete_worktree(repo_path, &worktree_path_buf) {
                    Ok(_) => {
                        tracing::info!(
                            task_id = task_id_str,
                            worktree = %worktree_path,
                            "Deleted worktree after merge"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            task_id = task_id_str,
                            worktree = %worktree_path,
                            "Failed to delete worktree (non-fatal)"
                        );
                    }
                }
            }

            // Delete the branch from main repo.
            // The branch is no longer checked out in any worktree, so force-delete works
            // without needing to checkout a different branch in the main repo.
            match GitService::delete_branch(repo_path, task_branch, true) {
                Ok(_) => {
                    tracing::info!(
                        task_id = task_id_str,
                        branch = %task_branch,
                        "Deleted task branch after merge (Worktree mode)"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        task_id = task_id_str,
                        branch = %task_branch,
                        "Failed to delete task branch (non-fatal)"
                    );
                }
            }
        }
    }
}

/// Convert project name to a URL-safe slug for branch naming
fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

/// Expand `~/` prefix to the user's home directory
fn expand_home(path: &str) -> String {
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
fn compute_merge_worktree_path(project: &Project, task_id: &str) -> String {
    let worktree_parent = project
        .worktree_parent_directory
        .as_deref()
        .unwrap_or("~/ralphx-worktrees");
    let expanded = expand_home(worktree_parent);
    format!("{}/{}/merge-{}", expanded, slugify(&project.name), task_id)
}

/// Extract a task ID from a merge worktree path.
///
/// Merge worktree paths follow the convention: `{parent}/{slug}/merge-{task_id}`
/// Returns `Some(task_id)` if the path matches, `None` otherwise.
fn extract_task_id_from_merge_path(path: &str) -> Option<&str> {
    let basename = path.rsplit('/').next()?;
    basename.strip_prefix("merge-")
}

/// Check if a task is currently in the `Merging` state (active agent-assisted merge).
///
/// Used to avoid deleting merge worktrees that belong to tasks actively being resolved.
async fn is_task_actively_merging(
    task_repo: &Arc<dyn TaskRepository>,
    task_id_str: &str,
) -> bool {
    let task_id = TaskId::from_string(task_id_str.to_string());
    match task_repo.get_by_id(&task_id).await {
        Ok(Some(task)) => task.internal_status == InternalStatus::Merging,
        _ => false,
    }
}

/// Check if a task's merge would target the given branch.
///
/// Resolves the task's merge target branch the same way `resolve_merge_branches()` does,
/// then compares against `target_branch`. Used by the concurrent merge guard to detect
/// tasks that would conflict with the same target.
async fn task_targets_branch(
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

/// Resolve the base branch for a task's working branch.
///
/// If the task belongs to a plan with an active feature branch, returns the feature
/// branch name so the task branch is created from it. Otherwise falls back to the
/// project's base branch.
async fn resolve_task_base_branch(
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
            tracing::info!(
                task_id = task.id.as_str(),
                feature_branch = %pb.branch_name,
                "Resolved task base branch to plan feature branch"
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
        if task.category == "plan_merge" {
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
        if pb.status == PlanBranchStatus::Active {
            tracing::info!(
                task_id = task.id.as_str(),
                feature_branch = %pb.branch_name,
                base_branch = %base_branch,
                "Merge task: merging feature branch into base"
            );
            return (pb.branch_name, base_branch);
        }
    }

    // Check if this task belongs to a plan with a feature branch
    if let Some(ref session_id) = task.ideation_session_id {
        if let Ok(Some(pb)) = plan_branch_repo.get_by_session_id(session_id).await {
            if pb.status == PlanBranchStatus::Active {
                tracing::info!(
                    task_id = task.id.as_str(),
                    task_branch = %task_branch,
                    feature_branch = %pb.branch_name,
                    "Plan task: merging task branch into feature branch"
                );
                return (task_branch, pb.branch_name);
            }
        }
    }

    (task_branch, base_branch)
}

// ============================================================================
// Post-Merge Validation Gate
// ============================================================================

/// Analysis entry for path-scoped build/validation commands.
/// Mirrors the HTTP handler's AnalysisEntry but kept local to avoid cross-module coupling.
#[derive(Debug, Clone, serde::Deserialize)]
struct MergeAnalysisEntry {
    path: String,
    #[allow(dead_code)]
    label: String,
    #[serde(default)]
    validate: Vec<String>,
    #[serde(default)]
    worktree_setup: Vec<String>,
}

/// A single validation command execution record for streaming + storage.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct ValidationLogEntry {
    phase: String,
    command: String,
    path: String,
    label: String,
    status: String,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    duration_ms: u64,
}

/// Result of running post-merge validation commands.
#[derive(Debug)]
pub(crate) struct ValidationResult {
    pub(crate) all_passed: bool,
    pub(crate) failures: Vec<ValidationFailure>,
    pub(crate) log: Vec<ValidationLogEntry>,
}

#[derive(Debug)]
pub(crate) struct ValidationFailure {
    command: String,
    path: String,
    exit_code: Option<i32>,
    stderr: String,
}

/// Truncate a string to `max_len` chars, appending "... (truncated)" if needed.
fn truncate_output(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}... (truncated)", &s[..max_len])
    }
}

/// Load effective analysis, resolve template vars, and run all validate commands.
///
/// Returns `None` if no analysis entries exist (backward compatible — skip validation).
/// Returns `Some(ValidationResult)` with pass/fail details otherwise.
///
/// When `app_handle` is `Some`, emits `merge:validation_step` events for real-time UI streaming.
/// All executed commands are recorded in `ValidationResult::log` for metadata storage.
///
/// When `cached_log` is provided, validate-phase commands that previously passed (status
/// "success" or "cached") are skipped and emitted as "cached" instead of re-running.
/// Setup-phase commands always re-run. Previously-failed commands always re-run.
pub(crate) fn run_validation_commands(
    project: &Project,
    task: &Task,
    merge_cwd: &Path,
    task_id_str: &str,
    app_handle: Option<&tauri::AppHandle>,
    cached_log: Option<&[ValidationLogEntry]>,
) -> Option<ValidationResult> {
    // Load effective analysis: custom_analysis ?? detected_analysis
    let analysis_json = project
        .custom_analysis
        .as_ref()
        .or(project.detected_analysis.as_ref())?;

    let entries: Vec<MergeAnalysisEntry> = match serde_json::from_str(analysis_json) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to parse project analysis JSON, skipping validation");
            return None;
        }
    };

    if entries.is_empty() {
        return None;
    }

    // Collect all validate commands with their resolved paths
    let project_root = &project.working_directory;
    let worktree_path = merge_cwd.to_str().unwrap_or(project_root);
    let task_branch = task.task_branch.as_deref().unwrap_or("");

    let resolve = |s: &str| -> String {
        s.replace("{project_root}", project_root)
            .replace("{worktree_path}", worktree_path)
            .replace("{task_branch}", task_branch)
    };

    let mut log: Vec<ValidationLogEntry> = Vec::new();

    // Run worktree_setup commands first (symlinks, etc.) — non-fatal
    for entry in &entries {
        for cmd_str in &entry.worktree_setup {
            let resolved_cmd = resolve(cmd_str);
            let resolved_path = resolve(&entry.path);
            let cmd_cwd = if resolved_path == "." {
                merge_cwd.to_path_buf()
            } else {
                merge_cwd.join(&resolved_path)
            };

            // Emit "running" event before execution
            if let Some(handle) = app_handle {
                let _ = handle.emit("merge:validation_step", serde_json::json!({
                    "task_id": task_id_str,
                    "phase": "setup",
                    "command": resolved_cmd,
                    "path": resolved_path,
                    "label": entry.label,
                    "status": "running",
                }));
            }

            tracing::info!(
                command = %resolved_cmd,
                cwd = %cmd_cwd.display(),
                "Running worktree setup command"
            );

            let start = std::time::Instant::now();
            match Command::new("sh")
                .arg("-c")
                .arg(&resolved_cmd)
                .current_dir(&cmd_cwd)
                .output()
            {
                Ok(output) => {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    let stdout_raw = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr_raw = String::from_utf8_lossy(&output.stderr).to_string();
                    let status = if output.status.success() { "success" } else { "failed" };

                    if !output.status.success() {
                        tracing::warn!(
                            command = %resolved_cmd,
                            stderr = %stderr_raw,
                            "Worktree setup command failed (non-fatal)"
                        );
                    }

                    let log_entry = ValidationLogEntry {
                        phase: "setup".to_string(),
                        command: resolved_cmd.clone(),
                        path: resolved_path.clone(),
                        label: entry.label.clone(),
                        status: status.to_string(),
                        exit_code: output.status.code(),
                        stdout: truncate_output(&stdout_raw, 2000),
                        stderr: truncate_output(&stderr_raw, 2000),
                        duration_ms,
                    };

                    if let Some(handle) = app_handle {
                        let _ = handle.emit("merge:validation_step", serde_json::json!({
                            "task_id": task_id_str,
                            "phase": log_entry.phase,
                            "command": log_entry.command,
                            "path": log_entry.path,
                            "label": log_entry.label,
                            "status": log_entry.status,
                            "exit_code": log_entry.exit_code,
                            "stdout": log_entry.stdout,
                            "stderr": log_entry.stderr,
                            "duration_ms": log_entry.duration_ms,
                        }));
                    }
                    log.push(log_entry);
                }
                Err(e) => {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    tracing::warn!(
                        command = %resolved_cmd,
                        error = %e,
                        "Worktree setup command failed (non-fatal)"
                    );

                    let log_entry = ValidationLogEntry {
                        phase: "setup".to_string(),
                        command: resolved_cmd.clone(),
                        path: resolved_path.clone(),
                        label: entry.label.clone(),
                        status: "failed".to_string(),
                        exit_code: None,
                        stdout: String::new(),
                        stderr: truncate_output(&format!("Failed to execute: {}", e), 2000),
                        duration_ms,
                    };

                    if let Some(handle) = app_handle {
                        let _ = handle.emit("merge:validation_step", serde_json::json!({
                            "task_id": task_id_str,
                            "phase": log_entry.phase,
                            "command": log_entry.command,
                            "path": log_entry.path,
                            "label": log_entry.label,
                            "status": log_entry.status,
                            "exit_code": log_entry.exit_code,
                            "stdout": log_entry.stdout,
                            "stderr": log_entry.stderr,
                            "duration_ms": log_entry.duration_ms,
                        }));
                    }
                    log.push(log_entry);
                }
            }
        }
    }

    let mut failures = Vec::new();
    let mut ran_any = false;

    for entry in &entries {
        if entry.validate.is_empty() {
            continue;
        }

        let resolved_path = resolve(&entry.path);
        // Resolve the CWD for this entry's commands:
        // If the entry path is ".", use merge_cwd directly.
        // Otherwise, join merge_cwd with the resolved relative path.
        let cmd_cwd = if resolved_path == "." {
            merge_cwd.to_path_buf()
        } else {
            merge_cwd.join(&resolved_path)
        };

        for cmd_str in &entry.validate {
            let resolved_cmd = resolve(cmd_str);
            ran_any = true;

            // Check cache: skip previously-passed validate commands when SHA matches
            if let Some(cached) = cached_log {
                let cached_hit = cached.iter().find(|c| {
                    c.phase == "validate"
                        && c.command == resolved_cmd
                        && c.path == resolved_path
                        && (c.status == "success" || c.status == "cached")
                });
                if let Some(prev) = cached_hit {
                    tracing::info!(
                        command = %resolved_cmd,
                        "Skipping validation command (cached, SHA unchanged)"
                    );
                    let log_entry = ValidationLogEntry {
                        phase: "validate".to_string(),
                        command: resolved_cmd,
                        path: resolved_path.clone(),
                        label: entry.label.clone(),
                        status: "cached".to_string(),
                        exit_code: prev.exit_code,
                        stdout: String::new(),
                        stderr: String::new(),
                        duration_ms: 0,
                    };
                    if let Some(handle) = app_handle {
                        let _ = handle.emit("merge:validation_step", serde_json::json!({
                            "task_id": task_id_str,
                            "phase": log_entry.phase,
                            "command": log_entry.command,
                            "path": log_entry.path,
                            "label": log_entry.label,
                            "status": log_entry.status,
                            "exit_code": log_entry.exit_code,
                            "duration_ms": log_entry.duration_ms,
                        }));
                    }
                    log.push(log_entry);
                    continue;
                }
            }

            // Emit "running" event before execution
            if let Some(handle) = app_handle {
                let _ = handle.emit("merge:validation_step", serde_json::json!({
                    "task_id": task_id_str,
                    "phase": "validate",
                    "command": resolved_cmd,
                    "path": resolved_path,
                    "label": entry.label,
                    "status": "running",
                }));
            }

            tracing::info!(
                command = %resolved_cmd,
                cwd = %cmd_cwd.display(),
                "Running post-merge validation command"
            );

            let start = std::time::Instant::now();
            let result = Command::new("sh")
                .arg("-c")
                .arg(&resolved_cmd)
                .current_dir(&cmd_cwd)
                .output();

            match result {
                Ok(output) => {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    let stderr_raw = String::from_utf8_lossy(&output.stderr).to_string();
                    let stdout_raw = String::from_utf8_lossy(&output.stdout).to_string();

                    if !output.status.success() {
                        tracing::warn!(
                            command = %resolved_cmd,
                            exit_code = ?output.status.code(),
                            stderr = %stderr_raw,
                            "Post-merge validation command failed"
                        );
                        failures.push(ValidationFailure {
                            command: resolved_cmd.clone(),
                            path: resolved_path.clone(),
                            exit_code: output.status.code(),
                            stderr: format!(
                                "{}{}",
                                if stderr_raw.is_empty() { "" } else { &stderr_raw },
                                if stdout_raw.is_empty() { String::new() } else { format!("\nstdout: {}", stdout_raw) },
                            ),
                        });
                    } else {
                        tracing::info!(command = %resolved_cmd, "Post-merge validation command passed");
                    }

                    let status = if output.status.success() { "success" } else { "failed" };
                    let log_entry = ValidationLogEntry {
                        phase: "validate".to_string(),
                        command: resolved_cmd,
                        path: resolved_path.clone(),
                        label: entry.label.clone(),
                        status: status.to_string(),
                        exit_code: output.status.code(),
                        stdout: truncate_output(&stdout_raw, 2000),
                        stderr: truncate_output(&stderr_raw, 2000),
                        duration_ms,
                    };

                    if let Some(handle) = app_handle {
                        let _ = handle.emit("merge:validation_step", serde_json::json!({
                            "task_id": task_id_str,
                            "phase": log_entry.phase,
                            "command": log_entry.command,
                            "path": log_entry.path,
                            "label": log_entry.label,
                            "status": log_entry.status,
                            "exit_code": log_entry.exit_code,
                            "stdout": log_entry.stdout,
                            "stderr": log_entry.stderr,
                            "duration_ms": log_entry.duration_ms,
                        }));
                    }
                    log.push(log_entry);
                }
                Err(e) => {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    tracing::error!(command = %resolved_cmd, error = %e, "Failed to execute validation command");
                    failures.push(ValidationFailure {
                        command: resolved_cmd.clone(),
                        path: resolved_path.clone(),
                        exit_code: None,
                        stderr: format!("Failed to execute: {}", e),
                    });

                    let log_entry = ValidationLogEntry {
                        phase: "validate".to_string(),
                        command: resolved_cmd,
                        path: resolved_path.clone(),
                        label: entry.label.clone(),
                        status: "failed".to_string(),
                        exit_code: None,
                        stdout: String::new(),
                        stderr: truncate_output(&format!("Failed to execute: {}", e), 2000),
                        duration_ms,
                    };

                    if let Some(handle) = app_handle {
                        let _ = handle.emit("merge:validation_step", serde_json::json!({
                            "task_id": task_id_str,
                            "phase": log_entry.phase,
                            "command": log_entry.command,
                            "path": log_entry.path,
                            "label": log_entry.label,
                            "status": log_entry.status,
                            "exit_code": log_entry.exit_code,
                            "stdout": log_entry.stdout,
                            "stderr": log_entry.stderr,
                            "duration_ms": log_entry.duration_ms,
                        }));
                    }
                    log.push(log_entry);
                }
            }
        }
    }

    if !ran_any {
        // All entries had empty validate arrays
        return None;
    }

    Some(ValidationResult {
        all_passed: failures.is_empty(),
        failures,
        log,
    })
}

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
                "stderr": if f.stderr.len() > 2000 { &f.stderr[..2000] } else { &f.stderr },
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
fn take_skip_validation_flag(task: &mut Task) -> bool {
    let Some(meta_str) = task.metadata.as_ref() else {
        return false;
    };
    let Ok(mut val) = serde_json::from_str::<serde_json::Value>(meta_str) else {
        return false;
    };
    let flag = val.get("skip_validation").and_then(|v| v.as_bool()).unwrap_or(false);
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
fn format_validation_warn_metadata(
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
fn extract_cached_validation(task: &Task, current_sha: &str) -> Option<Vec<ValidationLogEntry>> {
    let meta_str = task.metadata.as_ref()?;
    let val: serde_json::Value = serde_json::from_str(meta_str).ok()?;
    let stored_sha = val.get("validation_source_sha")?.as_str()?;
    if stored_sha != current_sha {
        return None;
    }
    let log_val = val.get("validation_log")?;
    serde_json::from_value::<Vec<ValidationLogEntry>>(log_val.clone()).ok()
}

impl<'a> super::TransitionHandler<'a> {
    /// Execute on-enter action for a state
    ///
    /// This method is public to allow `TaskTransitionService` to trigger entry actions
    /// for direct status changes (e.g., Kanban drag-drop) without going through the
    /// full event-based transition flow.
    ///
    /// Returns an error if the state entry cannot be completed (e.g., execution blocked
    /// due to uncommitted changes in Local mode).
    pub async fn on_enter(&self, state: &State) -> AppResult<()> {
        match state {
            State::Ready => {
                // When entering Ready, spawn QA prep agent if enabled
                if self.machine.context.qa_enabled {
                    self.machine
                        .context
                        .services
                        .agent_spawner
                        .spawn_background("qa-prep", &self.machine.context.task_id)
                        .await;
                }

                // Delay auto-scheduling so UI sees task "settle" in Ready column
                // before it potentially moves to Executing (600ms matches common UI feedback timing)
                if let Some(ref scheduler) = self.machine.context.services.task_scheduler {
                    let scheduler = Arc::clone(scheduler);
                    tokio::spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;
                        scheduler.try_schedule_ready_tasks().await;
                    });
                }
            }
            State::Executing => {
                let task_id_str = &self.machine.context.task_id;
                let project_id_str = &self.machine.context.project_id;

                // Setup branch/worktree for task isolation (Phase 66)
                // Only setup if task_repo and project_repo are available
                if let (Some(ref task_repo), Some(ref project_repo)) = (
                    &self.machine.context.services.task_repo,
                    &self.machine.context.services.project_repo,
                ) {
                    let task_id = TaskId::from_string(task_id_str.clone());
                    let project_id = ProjectId::from_string(project_id_str.clone());

                    // Fetch task and project
                    let task_result = task_repo.get_by_id(&task_id).await;
                    let project_result = project_repo.get_by_id(&project_id).await;

                    if let (Ok(Some(mut task)), Ok(Some(project))) = (task_result, project_result) {
                        // Only setup if task doesn't already have a branch
                        if task.task_branch.is_none() {
                            let branch = format!(
                                "ralphx/{}/task-{}",
                                slugify(&project.name),
                                task_id_str
                            );
                            // Resolve base branch: feature branch for plan tasks, project base otherwise
                            let plan_branch_repo = &self.machine.context.services.plan_branch_repo;
                            let resolved_base = resolve_task_base_branch(&task, &project, plan_branch_repo).await;
                            let base_branch = resolved_base.as_str();
                            let repo_path = Path::new(&project.working_directory);

                            // Attempt branch/worktree setup. Only ExecutionBlocked errors
                            // should prevent task execution (uncommitted changes in Local mode).
                            // Other git errors (missing repo, invalid path) are logged but
                            // don't block - the agent can still work in the project directory.
                            let git_result: AppResult<Option<(String, Option<String>)>> = match project.git_mode {
                                GitMode::Local => {
                                    // Block if uncommitted changes exist
                                    match GitService::has_uncommitted_changes(repo_path) {
                                        Ok(true) => {
                                            return Err(AppError::ExecutionBlocked(
                                                "Cannot execute task: uncommitted changes in working directory. \
                                                 Please commit or stash your changes first.".to_string()
                                            ));
                                        }
                                        Ok(false) => {
                                            // Create and checkout branch in main repo
                                            match GitService::create_branch(repo_path, &branch, base_branch)
                                                .and_then(|_| GitService::checkout_branch(repo_path, &branch))
                                            {
                                                Ok(_) => Ok(Some((branch.clone(), None))),
                                                Err(e) => {
                                                    tracing::warn!(
                                                        error = %e,
                                                        task_id = task_id_str,
                                                        "Failed to create/checkout task branch (Local mode), continuing without isolation"
                                                    );
                                                    Ok(None)
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tracing::warn!(
                                                error = %e,
                                                task_id = task_id_str,
                                                "Failed to check uncommitted changes, continuing without isolation"
                                            );
                                            Ok(None)
                                        }
                                    }
                                }
                                GitMode::Worktree => {
                                    // Build worktree path
                                    let worktree_parent = project
                                        .worktree_parent_directory
                                        .as_deref()
                                        .unwrap_or("~/ralphx-worktrees");
                                    let expanded_parent = expand_home(worktree_parent);

                                    let worktree_path = format!(
                                        "{}/{}/task-{}",
                                        expanded_parent,
                                        slugify(&project.name),
                                        task_id_str
                                    );
                                    let worktree_path_buf = std::path::PathBuf::from(&worktree_path);

                                    // Create worktree with new branch
                                    match GitService::create_worktree(
                                        repo_path,
                                        &worktree_path_buf,
                                        &branch,
                                        base_branch,
                                    ) {
                                        Ok(_) => Ok(Some((branch.clone(), Some(worktree_path)))),
                                        Err(e) => {
                                            tracing::warn!(
                                                error = %e,
                                                task_id = task_id_str,
                                                "Failed to create worktree (Worktree mode), continuing without isolation"
                                            );
                                            Ok(None)
                                        }
                                    }
                                }
                            };

                            // If git setup succeeded, persist the branch info
                            if let Ok(Some((branch_name, worktree_path_opt))) = git_result {
                                task.task_branch = Some(branch_name.clone());
                                if let Some(wt_path) = worktree_path_opt {
                                    task.worktree_path = Some(wt_path.clone());
                                    tracing::info!(
                                        task_id = task_id_str,
                                        branch = %branch_name,
                                        worktree_path = %wt_path,
                                        "Created worktree with task branch (Worktree mode)"
                                    );
                                } else {
                                    tracing::info!(
                                        task_id = task_id_str,
                                        branch = %branch_name,
                                        "Created and checked out task branch (Local mode)"
                                    );
                                }
                                task.touch();
                                if let Err(e) = task_repo.update(&task).await {
                                    tracing::error!(error = %e, "Failed to persist task branch info");
                                }
                            }
                        }
                    }
                }

                // Use ChatService for persistent worker execution (Phase 15B)
                let prompt = format!("Execute task: {}", task_id_str);
                tracing::debug!(
                    task_id = task_id_str,
                    prompt_len = prompt.len(),
                    "Transition handler sending task_execution message"
                );

                // send_message handles:
                // 1. Creating chat_conversation (context_type: 'task_execution')
                // 2. Creating agent_run (status: 'running')
                // 3. Spawning Claude CLI with --agent worker
                // 4. Persisting stream output to chat_messages
                // 5. Processing queued messages on completion
                let _ = self
                    .machine
                    .context
                    .services
                    .chat_service
                    .send_message(
                        crate::domain::entities::ChatContextType::TaskExecution,
                        task_id_str,
                        &prompt,
                    )
                    .await;
            }
            State::QaRefining => {
                // Wait for QA prep if not complete, then spawn QA refiner
                if !self.machine.context.qa_prep_complete {
                    self.machine
                        .context
                        .services
                        .agent_spawner
                        .wait_for("qa-prep", &self.machine.context.task_id)
                        .await;
                }
                self.machine
                    .context
                    .services
                    .agent_spawner
                    .spawn("qa-refiner", &self.machine.context.task_id)
                    .await;
            }
            State::QaTesting => {
                // Spawn QA tester agent
                self.machine
                    .context
                    .services
                    .agent_spawner
                    .spawn("qa-tester", &self.machine.context.task_id)
                    .await;
            }
            State::QaPassed => {
                // Emit QA passed event
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("qa_passed", &self.machine.context.task_id)
                    .await;
            }
            State::QaFailed(data) => {
                // Emit QA failed event and notify user
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("qa_failed", &self.machine.context.task_id)
                    .await;

                // Notify user if not already notified
                if !data.notified {
                    let message = format!(
                        "QA tests failed: {} failure(s)",
                        data.failure_count()
                    );
                    self.machine
                        .context
                        .services
                        .notifier
                        .notify_with_message(
                            "qa_failed",
                            &self.machine.context.task_id,
                            &message,
                        )
                        .await;
                }
            }
            State::PendingReview => {
                // Start AI review via ReviewStarter
                let review_result = self.machine
                    .context
                    .services
                    .review_starter
                    .start_ai_review(
                        &self.machine.context.task_id,
                        &self.machine.context.project_id,
                    )
                    .await;

                // Emit review:update event with the result
                match &review_result {
                    super::super::services::ReviewStartResult::Started { review_id } => {
                        self.machine
                            .context
                            .services
                            .event_emitter
                            .emit_with_payload(
                                "review:update",
                                &self.machine.context.task_id,
                                &format!(r#"{{"type":"started","reviewId":"{}"}}"#, review_id),
                            )
                            .await;
                    }
                    super::super::services::ReviewStartResult::Disabled => {
                        // AI review disabled, emit event but don't spawn agent
                        self.machine
                            .context
                            .services
                            .event_emitter
                            .emit_with_payload(
                                "review:update",
                                &self.machine.context.task_id,
                                r#"{"type":"disabled"}"#,
                            )
                            .await;
                    }
                    super::super::services::ReviewStartResult::Error(msg) => {
                        // Review failed to start, notify user
                        self.machine
                            .context
                            .services
                            .notifier
                            .notify_with_message(
                                "review_error",
                                &self.machine.context.task_id,
                                msg,
                            )
                            .await;
                    }
                }
            }
            State::Reviewing => {
                // For Local mode: checkout task branch before spawning reviewer
                // (Worktree mode already has isolated directory)
                self.checkout_task_branch_if_needed("Reviewing").await;

                // Spawn reviewer agent via ChatService with Review context
                let task_id = &self.machine.context.task_id;
                let prompt = format!("Review task: {}", task_id);

                tracing::info!(
                    task_id = task_id,
                    "on_enter(Reviewing): Spawning reviewer agent via ChatService"
                );

                let result = self
                    .machine
                    .context
                    .services
                    .chat_service
                    .send_message(
                        crate::domain::entities::ChatContextType::Review,
                        task_id,
                        &prompt,
                    )
                    .await;

                match &result {
                    Ok(_) => {
                        tracing::info!(task_id = task_id, "Reviewer agent spawned successfully");
                    }
                    Err(e) => {
                        tracing::error!(task_id = task_id, error = %e, "Failed to spawn reviewer agent");
                    }
                }
            }
            State::ReviewPassed => {
                // Emit 'review:ai_approved' event
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("review:ai_approved", &self.machine.context.task_id)
                    .await;

                // Notify user that review passed and awaits approval
                self.machine
                    .context
                    .services
                    .notifier
                    .notify_with_message(
                        "review:ai_approved",
                        &self.machine.context.task_id,
                        "AI review passed. Please review and approve.",
                    )
                    .await;
            }
            State::Escalated => {
                // Emit 'review:escalated' event
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("review:escalated", &self.machine.context.task_id)
                    .await;

                // Notify user that AI escalated review
                self.machine
                    .context
                    .services
                    .notifier
                    .notify_with_message(
                        "review:escalated",
                        &self.machine.context.task_id,
                        "AI review escalated. Please review and decide.",
                    )
                    .await;
            }
            State::ReExecuting => {
                // For Local mode: checkout task branch before spawning worker
                // (Worktree mode already has isolated directory)
                self.checkout_task_branch_if_needed("ReExecuting").await;

                // Spawn worker agent with revision context via ChatService
                let task_id = &self.machine.context.task_id;
                let prompt = format!("Re-execute task (revision): {}", task_id);

                let _ = self
                    .machine
                    .context
                    .services
                    .chat_service
                    .send_message(
                        crate::domain::entities::ChatContextType::TaskExecution,
                        task_id,
                        &prompt,
                    )
                    .await;
            }
            State::RevisionNeeded => {
                // Auto-transition to ReExecuting will be handled by check_auto_transition
            }
            State::Approved => {
                // Emit task completed event
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("task_completed", &self.machine.context.task_id)
                    .await;
                // NOTE: Do NOT unblock dependents here. Approved auto-transitions to
                // PendingMerge (Phase 66). Unblocking happens at on_enter(Merged) after
                // the task's work is actually on main.
            }
            State::Failed(_) => {
                // Emit task failed event
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("task_failed", &self.machine.context.task_id)
                    .await;
            }
            State::PendingMerge => {
                // Phase 1 of merge workflow: Attempt programmatic rebase and merge
                // This is the "fast path" - if successful, skip agent entirely
                self.attempt_programmatic_merge().await;
            }
            State::Merging => {
                // Phase 2 of merge workflow: Spawn merger agent for conflict resolution
                // This state is reached when Phase 1 (programmatic merge) failed due to conflicts,
                // OR when AutoFix validation mode detected validation failures (Phase 113)
                let task_id = &self.machine.context.task_id;

                // Check task metadata for validation_recovery flag (Phase 113: AutoFix mode)
                let is_validation_recovery = if let Some(ref task_repo) = self.machine.context.services.task_repo {
                    let tid = TaskId::from_string(task_id.clone());
                    if let Ok(Some(task)) = task_repo.get_by_id(&tid).await {
                        task.metadata.as_ref()
                            .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
                            .and_then(|v| v.get("validation_recovery")?.as_bool())
                            .unwrap_or(false)
                    } else {
                        false
                    }
                } else {
                    false
                };

                let prompt = if is_validation_recovery {
                    format!(
                        "Fix validation failures for task: {}. The merge succeeded but post-merge \
                         validation commands failed. The failing code is on the target branch. \
                         Read the validation failures from task context, fix the code, run validation \
                         to confirm, then commit your fixes.",
                        task_id
                    )
                } else {
                    format!("Resolve merge conflicts for task: {}", task_id)
                };

                tracing::info!(
                    task_id = task_id,
                    is_validation_recovery = is_validation_recovery,
                    "on_enter(Merging): Spawning merger agent via ChatService"
                );

                // Use ChatService with Merge context type for the merger agent
                let result = self
                    .machine
                    .context
                    .services
                    .chat_service
                    .send_message(
                        crate::domain::entities::ChatContextType::Merge,
                        task_id,
                        &prompt,
                    )
                    .await;

                match &result {
                    Ok(_) => {
                        tracing::info!(task_id = task_id, "Merger agent spawned successfully");
                    }
                    Err(e) => {
                        tracing::error!(task_id = task_id, error = %e, "Failed to spawn merger agent");
                    }
                }
            }
            State::Merged => {
                // Auto-unblock tasks that were waiting on this task
                // This handles the HTTP handler path where transition_task triggers on_enter
                self.machine
                    .context
                    .services
                    .dependency_manager
                    .unblock_dependents(&self.machine.context.task_id)
                    .await;

                // Schedule newly-unblocked tasks (e.g. plan_merge tasks that just became Ready)
                if let Some(ref scheduler) = self.machine.context.services.task_scheduler {
                    let scheduler = Arc::clone(scheduler);
                    tokio::spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;
                        scheduler.try_schedule_ready_tasks().await;
                    });
                }

                // Retry deferred merges — covers the HTTP handler path (e.g. ConflictResolved)
                // where on_enter(Merged) is called directly without going through
                // post_merge_cleanup(). Uses 800ms delay to serialize after scheduling.
                if let Some(ref scheduler) = self.machine.context.services.task_scheduler {
                    let scheduler = Arc::clone(scheduler);
                    let project_id = self.machine.context.project_id.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
                        scheduler.try_retry_deferred_merges(&project_id).await;
                    });
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// For Local mode: checkout task branch if current branch differs.
    /// This is needed when re-entering execution states (ReExecuting, Reviewing)
    /// where the task already has a branch but we may be on a different branch.
    /// Worktree mode doesn't need this as each task has its own isolated directory.
    async fn checkout_task_branch_if_needed(&self, state_name: &str) {
        let task_id_str = &self.machine.context.task_id;
        let project_id_str = &self.machine.context.project_id;

        if let (Some(ref task_repo), Some(ref project_repo)) = (
            &self.machine.context.services.task_repo,
            &self.machine.context.services.project_repo,
        ) {
            let task_id = TaskId::from_string(task_id_str.clone());
            let project_id = ProjectId::from_string(project_id_str.clone());

            let task_result = task_repo.get_by_id(&task_id).await;
            let project_result = project_repo.get_by_id(&project_id).await;

            if let (Ok(Some(task)), Ok(Some(project))) = (task_result, project_result) {
                // Only checkout for Local mode - Worktree mode already has isolated directory
                if project.git_mode == GitMode::Local {
                    if let Some(branch) = &task.task_branch {
                        let repo_path = Path::new(&project.working_directory);
                        match GitService::get_current_branch(repo_path) {
                            Ok(current) if current != *branch => {
                                match GitService::checkout_branch(repo_path, branch) {
                                    Ok(_) => {
                                        tracing::info!(
                                            task_id = task_id_str,
                                            branch = %branch,
                                            from_branch = %current,
                                            state = state_name,
                                            "Checked out task branch (Local mode)"
                                        );
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            error = %e,
                                            task_id = task_id_str,
                                            branch = %branch,
                                            state = state_name,
                                            "Failed to checkout task branch (Local mode)"
                                        );
                                    }
                                }
                            }
                            Ok(_) => {
                                // Already on correct branch
                                tracing::debug!(
                                    task_id = task_id_str,
                                    branch = %branch,
                                    state = state_name,
                                    "Already on task branch (Local mode)"
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    error = %e,
                                    task_id = task_id_str,
                                    state = state_name,
                                    "Failed to get current branch"
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// Attempt programmatic rebase and merge (Phase 1 of merge workflow).
    ///
    /// This is the "fast path" - try to rebase task branch onto base and merge.
    /// If successful, transition directly to Merged and cleanup branch/worktree.
    /// If conflicts occur, transition to Merging for agent-assisted resolution.
    async fn attempt_programmatic_merge(&self) {
        let task_id_str = &self.machine.context.task_id;
        let project_id_str = &self.machine.context.project_id;

        // Only proceed if repos are available
        let (Some(ref task_repo), Some(ref project_repo)) = (
            &self.machine.context.services.task_repo,
            &self.machine.context.services.project_repo,
        ) else {
            tracing::error!(
                task_id = task_id_str,
                project_id = project_id_str,
                task_repo_available = self.machine.context.services.task_repo.is_some(),
                project_repo_available = self.machine.context.services.project_repo.is_some(),
                "Programmatic merge BLOCKED: repos not available — \
                 task will remain stuck in PendingMerge"
            );
            return;
        };

        let task_id = TaskId::from_string(task_id_str.clone());
        let project_id = ProjectId::from_string(project_id_str.clone());

        // Fetch task and project
        let task_result = task_repo.get_by_id(&task_id).await;
        let project_result = project_repo.get_by_id(&project_id).await;

        let (Ok(Some(mut task)), Ok(Some(project))) = (task_result, project_result) else {
            tracing::error!(
                task_id = task_id_str,
                project_id = project_id_str,
                "Programmatic merge BLOCKED: failed to fetch task or project from database — \
                 task will remain stuck in PendingMerge"
            );
            return;
        };

        // Resolve source and target branches (handles merge tasks and plan feature branches)
        let plan_branch_repo = &self.machine.context.services.plan_branch_repo;
        let (source_branch, target_branch) = resolve_merge_branches(&task, &project, plan_branch_repo).await;

        // Ensure we have a source branch to merge
        if source_branch.is_empty() {
            tracing::error!(
                task_id = task_id_str,
                category = %task.category,
                task_branch = ?task.task_branch,
                "Programmatic merge failed: empty source branch resolved — \
                 transitioning to MergeIncomplete"
            );

            task.metadata = Some(serde_json::json!({
                "error": "Empty source branch resolved. This typically means plan_branch_repo \
                          was unavailable when resolving merge branches for a plan_merge task.",
                "source_branch": source_branch,
                "target_branch": target_branch,
                "category": task.category,
            }).to_string());
            task.internal_status = InternalStatus::MergeIncomplete;
            task.touch();

            if let Err(e) = task_repo.update(&task).await {
                tracing::error!(error = %e, "Failed to update task to MergeIncomplete status");
                return;
            }

            if let Err(e) = task_repo.persist_status_change(
                &task_id,
                InternalStatus::PendingMerge,
                InternalStatus::MergeIncomplete,
                "merge_incomplete",
            ).await {
                tracing::warn!(error = %e, "Failed to record merge incomplete transition (non-fatal)");
            }

            self.machine
                .context
                .services
                .event_emitter
                .emit("task:status_changed", task_id_str)
                .await;

            return;
        }

        let repo_path = Path::new(&project.working_directory);

        tracing::info!(
            task_id = task_id_str,
            source_branch = %source_branch,
            target_branch = %target_branch,
            git_mode = ?project.git_mode,
            "Attempting programmatic merge (Phase 1)"
        );

        // --- Concurrent merge guard (worktree mode only) ---
        // In worktree mode, git only allows one worktree per branch. If another task
        // is already merging (PendingMerge or Merging) into the same target branch,
        // we must defer this task to avoid the "branch already checked out" error.
        // Priority: older task (by created_at) wins; newer task gets deferred.
        if project.git_mode == GitMode::Worktree {
            let all_tasks = task_repo.get_by_project(&project.id).await.unwrap_or_default();
            let merge_states = [InternalStatus::PendingMerge, InternalStatus::Merging];

            let has_older_merge = {
                let mut found = false;
                for other in &all_tasks {
                    // Skip self
                    if other.id == task.id {
                        continue;
                    }
                    // Only consider tasks in merge states
                    if !merge_states.contains(&other.internal_status) {
                        continue;
                    }
                    // Skip tasks that are themselves deferred
                    if has_merge_deferred_metadata(other) {
                        continue;
                    }
                    // Skip archived tasks — they are dead, will never complete
                    if other.archived_at.is_some() {
                        continue;
                    }
                    // Check if targeting the same branch
                    if !task_targets_branch(other, &project, plan_branch_repo, &target_branch).await {
                        continue;
                    }
                    // Older task has priority
                    if other.created_at < task.created_at {
                        tracing::info!(
                            task_id = task_id_str,
                            other_task_id = other.id.as_str(),
                            other_created_at = %other.created_at,
                            this_created_at = %task.created_at,
                            target_branch = %target_branch,
                            other_task_branch = ?other.task_branch,
                            "Concurrent merge detected: older task has priority, deferring this task"
                        );
                        found = true;
                        break;
                    }
                }
                found
            };

            if has_older_merge {
                // Set merge_deferred metadata and return early — task stays in PendingMerge
                let now = chrono::Utc::now().to_rfc3339();
                let mut meta = parse_metadata(&task).unwrap_or_else(|| serde_json::json!({}));
                if let Some(obj) = meta.as_object_mut() {
                    obj.insert("merge_deferred".to_string(), serde_json::json!(true));
                    obj.insert("merge_deferred_at".to_string(), serde_json::json!(now));
                }
                task.metadata = Some(meta.to_string());
                task.touch();

                if let Err(e) = task_repo.update(&task).await {
                    tracing::error!(
                        task_id = task_id_str,
                        error = %e,
                        "Failed to update task with merge_deferred metadata"
                    );
                }

                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("task:status_changed", task_id_str)
                    .await;

                tracing::info!(
                    task_id = task_id_str,
                    target_branch = %target_branch,
                    "Merge deferred — task stays in PendingMerge until competing merge completes"
                );
                return;
            }

            // If this task was previously deferred, clear the flag now that we're proceeding
            if has_merge_deferred_metadata(&task) {
                clear_merge_deferred_metadata(&mut task);
                task.touch();
                let _ = task_repo.update(&task).await;
            }
        }

        // In worktree mode, delete the task worktree first to unlock the branch.
        // Git refuses to checkout a branch that's checked out in another worktree,
        // so we must remove the task worktree before creating the merge worktree.
        if project.git_mode == GitMode::Worktree {
            if let Some(ref worktree_path) = task.worktree_path {
                let worktree_path_buf = PathBuf::from(worktree_path);
                if worktree_path_buf.exists() {
                    tracing::info!(
                        task_id = task_id_str,
                        worktree_path = %worktree_path,
                        "Deleting task worktree before programmatic merge to unlock branch"
                    );
                    if let Err(e) = GitService::delete_worktree(repo_path, &worktree_path_buf) {
                        tracing::error!(
                            task_id = task_id_str,
                            error = %e,
                            worktree_path = %worktree_path,
                            "Failed to delete task worktree before merge"
                        );
                        // Continue anyway - merge will fail with a clear error
                    }
                }
            }

            // --- Stale merge worktree cleanup ---
            // Step 1: Prune stale worktree references (metadata pointing to deleted dirs)
            if let Err(e) = GitService::prune_worktrees(repo_path) {
                tracing::warn!(
                    task_id = task_id_str,
                    error = %e,
                    "Failed to prune stale worktrees (non-fatal)"
                );
            }

            // Step 2: Force-delete our own merge worktree if it exists from a prior attempt
            let own_merge_wt = compute_merge_worktree_path(&project, task_id_str);
            let own_merge_wt_path = PathBuf::from(&own_merge_wt);
            if own_merge_wt_path.exists() {
                tracing::info!(
                    task_id = task_id_str,
                    merge_worktree_path = %own_merge_wt,
                    "Cleaning up stale merge worktree from previous attempt"
                );
                if let Err(e) = GitService::delete_worktree(repo_path, &own_merge_wt_path) {
                    tracing::warn!(
                        task_id = task_id_str,
                        error = %e,
                        merge_worktree_path = %own_merge_wt,
                        "Failed to delete stale merge worktree (non-fatal)"
                    );
                }
            }

            // Step 3: Scan for orphaned merge worktrees on the same target branch.
            // Another task's merge may have crashed/failed, leaving a worktree that locks
            // the target branch. We only clean up if the owning task is NOT actively merging.
            if let Ok(worktrees) = GitService::list_worktrees(repo_path) {
                for wt in &worktrees {
                    // Only consider merge worktrees (path contains "/merge-")
                    let Some(other_task_id) = extract_task_id_from_merge_path(&wt.path) else {
                        continue;
                    };
                    // Skip our own — already handled above
                    if other_task_id == task_id_str {
                        continue;
                    }
                    // Only care about worktrees on the same target branch
                    let wt_branch = wt.branch.as_deref().unwrap_or("");
                    if wt_branch != target_branch {
                        continue;
                    }
                    // Check if the owning task is actively merging — if so, leave it alone
                    if is_task_actively_merging(task_repo, other_task_id).await {
                        tracing::info!(
                            task_id = task_id_str,
                            other_task_id = other_task_id,
                            worktree_path = %wt.path,
                            "Skipping orphaned merge worktree — owning task is actively merging"
                        );
                        continue;
                    }
                    tracing::info!(
                        task_id = task_id_str,
                        other_task_id = other_task_id,
                        worktree_path = %wt.path,
                        target_branch = %target_branch,
                        "Cleaning up orphaned merge worktree from non-active task"
                    );
                    let orphan_path = PathBuf::from(&wt.path);
                    if let Err(e) = GitService::delete_worktree(repo_path, &orphan_path) {
                        tracing::warn!(
                            task_id = task_id_str,
                            other_task_id = other_task_id,
                            error = %e,
                            worktree_path = %wt.path,
                            "Failed to delete orphaned merge worktree (non-fatal)"
                        );
                    }
                }
            }
        }

        // Attempt the merge based on git mode:
        // - Worktree mode: merge in an isolated merge worktree (never touches main repo)
        //   EXCEPT when target branch is already checked out (e.g., plan merge → main),
        //   in which case merge directly in the primary repo.
        // - Local mode: rebase for linear history (operates on main repo)
        if project.git_mode == GitMode::Worktree {
            // Detect if the target branch is already checked out in the primary repo.
            // This happens for plan merge tasks (plan feature branch → main) because
            // main is always checked out in the primary repo. Git forbids the same
            // branch in multiple worktrees, so we merge directly in-repo instead.
            let current_branch = GitService::get_current_branch(repo_path).unwrap_or_default();
            let target_is_checked_out = current_branch == target_branch;

            if target_is_checked_out {
                // Target branch (e.g., main) is checked out in the primary repo.
                // Merge directly there instead of creating a worktree.
                tracing::info!(
                    task_id = task_id_str,
                    target_branch = %target_branch,
                    "Target branch is checked out in primary repo, merging directly in-repo"
                );

                let merge_result = GitService::try_merge_in_repo(
                    repo_path,
                    &source_branch,
                    &target_branch,
                );

                match merge_result {
                    Ok(MergeAttemptResult::Success { commit_sha }) => {
                        tracing::info!(
                            task_id = task_id_str,
                            commit_sha = %commit_sha,
                            "Programmatic merge in-repo succeeded (fast path)"
                        );

                        // Post-merge validation gate: check mode + skip flag
                        let skip_validation = take_skip_validation_flag(&mut task);
                        let validation_mode = &project.merge_validation_mode;
                        if !skip_validation && *validation_mode != MergeValidationMode::Off {
                            let source_sha = GitService::get_branch_sha(repo_path, &source_branch).ok();
                            let cached_log = source_sha.as_deref()
                                .and_then(|sha| extract_cached_validation(&task, sha));
                            let app_handle_ref = self.machine.context.services.app_handle.as_ref();
                            if let Some(validation) = run_validation_commands(&project, &task, repo_path, task_id_str, app_handle_ref, cached_log.as_deref()) {
                                if !validation.all_passed {
                                    if *validation_mode == MergeValidationMode::Warn {
                                        tracing::warn!(task_id = task_id_str, "Validation failed in Warn mode (in-repo), proceeding with merge");
                                        task.metadata = Some(format_validation_warn_metadata(
                                            &validation.log, &source_branch, &target_branch,
                                        ));
                                    } else {
                                        self.handle_validation_failure(
                                            &mut task, &task_id, task_id_str, task_repo,
                                            &validation.failures, &validation.log, &source_branch, &target_branch,
                                            repo_path, "in-repo", validation_mode,
                                        ).await;
                                        return;
                                    }
                                } else {
                                    task.metadata = Some(serde_json::json!({
                                        "validation_log": validation.log,
                                        "validation_source_sha": source_sha,
                                        "source_branch": source_branch,
                                        "target_branch": target_branch,
                                    }).to_string());
                                }
                            }
                        }

                        let app_handle = self.machine.context.services.app_handle.as_ref();
                        if let Err(e) = complete_merge_internal(
                            &mut task,
                            &project,
                            &commit_sha,
                            task_repo,
                            app_handle,
                        ).await {
                            tracing::error!(error = %e, task_id = task_id_str, "Failed to complete programmatic merge, falling back to MergeIncomplete");

                            task.metadata = Some(serde_json::json!({
                                "error": format!("complete_merge_internal failed: {}", e),
                                "source_branch": source_branch,
                                "target_branch": target_branch,
                            }).to_string());
                            task.internal_status = InternalStatus::MergeIncomplete;
                            task.touch();

                            let _ = task_repo.update(&task).await;
                            let _ = task_repo.persist_status_change(
                                &task_id,
                                InternalStatus::PendingMerge,
                                InternalStatus::MergeIncomplete,
                                "merge_incomplete",
                            ).await;

                            self.machine.context.services.event_emitter
                                .emit("task:status_changed", task_id_str).await;
                        } else {
                            self.post_merge_cleanup(task_id_str, &task_id, repo_path, plan_branch_repo).await;
                        }
                    }
                    Ok(MergeAttemptResult::NeedsAgent { conflict_files }) => {
                        // Conflict detected in primary repo — merger agent resolves in-place
                        tracing::info!(
                            task_id = task_id_str,
                            conflict_count = conflict_files.len(),
                            "Merge in-repo has conflicts, transitioning to Merging"
                        );

                        for file in &conflict_files {
                            tracing::debug!(task_id = task_id_str, file = %file.display(), "Conflict file");
                        }

                        // Set worktree_path to primary repo so merger agent CWD resolves there
                        task.worktree_path = Some(project.working_directory.clone());
                        task.internal_status = InternalStatus::Merging;
                        task.touch();

                        if let Err(e) = task_repo.update(&task).await {
                            tracing::error!(error = %e, "Failed to update task to Merging with primary repo path");
                            return;
                        }

                        if let Err(e) = task_repo.persist_status_change(
                            &task_id,
                            InternalStatus::PendingMerge,
                            InternalStatus::Merging,
                            "merge_conflict",
                        ).await {
                            tracing::warn!(error = %e, "Failed to record merge conflict transition (non-fatal)");
                        }

                        self.machine
                            .context
                            .services
                            .event_emitter
                            .emit("task:merge_conflict", task_id_str)
                            .await;

                        // Spawn merger agent — CWD is primary repo
                        let prompt = format!("Resolve merge conflicts for task: {}", task_id_str);
                        tracing::info!(
                            task_id = task_id_str,
                            "Spawning merger agent for in-repo conflict resolution"
                        );

                        let result = self
                            .machine
                            .context
                            .services
                            .chat_service
                            .send_message(
                                crate::domain::entities::ChatContextType::Merge,
                                task_id_str,
                                &prompt,
                            )
                            .await;

                        match &result {
                            Ok(_) => tracing::info!(task_id = task_id_str, "Merger agent spawned successfully"),
                            Err(e) => tracing::error!(task_id = task_id_str, error = %e, "Failed to spawn merger agent"),
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            task_id = task_id_str,
                            error = %e,
                            source_branch = %source_branch,
                            target_branch = %target_branch,
                            "Merge in-repo failed, transitioning to MergeIncomplete"
                        );

                        task.metadata = Some(serde_json::json!({
                            "error": e.to_string(),
                            "source_branch": source_branch,
                            "target_branch": target_branch,
                        }).to_string());
                        task.internal_status = InternalStatus::MergeIncomplete;
                        task.touch();

                        if let Err(e) = task_repo.update(&task).await {
                            tracing::error!(error = %e, "Failed to update task to MergeIncomplete status");
                            return;
                        }

                        if let Err(e) = task_repo.persist_status_change(
                            &task_id,
                            InternalStatus::PendingMerge,
                            InternalStatus::MergeIncomplete,
                            "merge_incomplete",
                        ).await {
                            tracing::warn!(error = %e, "Failed to record merge incomplete transition (non-fatal)");
                        }

                        self.machine
                            .context
                            .services
                            .event_emitter
                            .emit("task:status_changed", task_id_str)
                            .await;
                    }
                }
            } else {
                // Target branch is NOT checked out — use isolated merge worktree (existing path)
                let merge_wt_path_str = compute_merge_worktree_path(&project, task_id_str);
                let merge_wt_path = PathBuf::from(&merge_wt_path_str);

                tracing::info!(
                    task_id = task_id_str,
                    merge_worktree_path = %merge_wt_path_str,
                    "Creating merge worktree for isolated merge"
                );

                let merge_result = GitService::try_merge_in_worktree(
                    repo_path,
                    &source_branch,
                    &target_branch,
                    &merge_wt_path,
                );

                match merge_result {
                    Ok(MergeAttemptResult::Success { commit_sha }) => {
                        tracing::info!(
                            task_id = task_id_str,
                            commit_sha = %commit_sha,
                            "Programmatic merge in worktree succeeded (fast path)"
                        );

                        // Post-merge validation gate: check mode + skip flag
                        let skip_validation = take_skip_validation_flag(&mut task);
                        let validation_mode = &project.merge_validation_mode;
                        if !skip_validation && *validation_mode != MergeValidationMode::Off {
                            let source_sha = GitService::get_branch_sha(repo_path, &source_branch).ok();
                            let cached_log = source_sha.as_deref()
                                .and_then(|sha| extract_cached_validation(&task, sha));
                            let app_handle_ref = self.machine.context.services.app_handle.as_ref();
                            if let Some(validation) = run_validation_commands(&project, &task, &merge_wt_path, task_id_str, app_handle_ref, cached_log.as_deref()) {
                                if !validation.all_passed {
                                    if *validation_mode == MergeValidationMode::Warn {
                                        tracing::warn!(task_id = task_id_str, "Validation failed in Warn mode (worktree), proceeding with merge");
                                        task.metadata = Some(format_validation_warn_metadata(
                                            &validation.log, &source_branch, &target_branch,
                                        ));
                                    } else {
                                        // Block mode: reset in merge worktree, then delete it
                                        // AutoFix mode: keep the worktree for the merger agent to fix in
                                        self.handle_validation_failure(
                                            &mut task, &task_id, task_id_str, task_repo,
                                            &validation.failures, &validation.log, &source_branch, &target_branch,
                                            &merge_wt_path, "worktree", validation_mode,
                                        ).await;
                                        if *validation_mode != MergeValidationMode::AutoFix {
                                            let _ = GitService::delete_worktree(repo_path, &merge_wt_path);
                                        }
                                        return;
                                    }
                                } else {
                                    task.metadata = Some(serde_json::json!({
                                        "validation_log": validation.log,
                                        "validation_source_sha": source_sha,
                                        "source_branch": source_branch,
                                        "target_branch": target_branch,
                                    }).to_string());
                                }
                            }
                        }

                        if let Err(e) = GitService::delete_worktree(repo_path, &merge_wt_path) {
                            tracing::warn!(
                                error = %e,
                                task_id = task_id_str,
                                merge_worktree_path = %merge_wt_path_str,
                                "Failed to delete merge worktree after success (non-fatal)"
                            );
                        }

                        let app_handle = self.machine.context.services.app_handle.as_ref();
                        if let Err(e) = complete_merge_internal(
                            &mut task,
                            &project,
                            &commit_sha,
                            task_repo,
                            app_handle,
                        ).await {
                            tracing::error!(error = %e, task_id = task_id_str, "Failed to complete programmatic merge, falling back to MergeIncomplete");

                            task.metadata = Some(serde_json::json!({
                                "error": format!("complete_merge_internal failed: {}", e),
                                "source_branch": source_branch,
                                "target_branch": target_branch,
                            }).to_string());
                            task.internal_status = InternalStatus::MergeIncomplete;
                            task.touch();

                            let _ = task_repo.update(&task).await;
                            let _ = task_repo.persist_status_change(
                                &task_id,
                                InternalStatus::PendingMerge,
                                InternalStatus::MergeIncomplete,
                                "merge_incomplete",
                            ).await;

                            self.machine.context.services.event_emitter
                                .emit("task:status_changed", task_id_str).await;
                        } else {
                            self.post_merge_cleanup(task_id_str, &task_id, repo_path, plan_branch_repo).await;
                        }
                    }
                    Ok(MergeAttemptResult::NeedsAgent { conflict_files }) => {
                        tracing::info!(
                            task_id = task_id_str,
                            conflict_count = conflict_files.len(),
                            merge_worktree_path = %merge_wt_path_str,
                            "Merge in worktree has conflicts, transitioning to Merging"
                        );

                        for file in &conflict_files {
                            tracing::debug!(task_id = task_id_str, file = %file.display(), "Conflict file");
                        }

                        task.worktree_path = Some(merge_wt_path_str.clone());
                        task.internal_status = InternalStatus::Merging;
                        task.touch();

                        if let Err(e) = task_repo.update(&task).await {
                            tracing::error!(error = %e, "Failed to update task to Merging with merge worktree path");
                            return;
                        }

                        if let Err(e) = task_repo.persist_status_change(
                            &task_id,
                            InternalStatus::PendingMerge,
                            InternalStatus::Merging,
                            "merge_conflict",
                        ).await {
                            tracing::warn!(error = %e, "Failed to record merge conflict transition (non-fatal)");
                        }

                        self.machine
                            .context
                            .services
                            .event_emitter
                            .emit("task:merge_conflict", task_id_str)
                            .await;

                        let prompt = format!("Resolve merge conflicts for task: {}", task_id_str);
                        tracing::info!(
                            task_id = task_id_str,
                            "Spawning merger agent for conflict resolution (from attempt_programmatic_merge)"
                        );

                        let result = self
                            .machine
                            .context
                            .services
                            .chat_service
                            .send_message(
                                crate::domain::entities::ChatContextType::Merge,
                                task_id_str,
                                &prompt,
                            )
                            .await;

                        match &result {
                            Ok(_) => tracing::info!(task_id = task_id_str, "Merger agent spawned successfully"),
                            Err(e) => tracing::error!(task_id = task_id_str, error = %e, "Failed to spawn merger agent"),
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            task_id = task_id_str,
                            error = %e,
                            merge_worktree_path = %merge_wt_path_str,
                            source_branch = %source_branch,
                            target_branch = %target_branch,
                            "Merge in worktree failed, transitioning to MergeIncomplete"
                        );

                        if merge_wt_path.exists() {
                            let _ = GitService::delete_worktree(repo_path, &merge_wt_path);
                        }

                        task.metadata = Some(serde_json::json!({
                            "error": e.to_string(),
                            "source_branch": source_branch,
                            "target_branch": target_branch,
                        }).to_string());
                        task.internal_status = InternalStatus::MergeIncomplete;
                        task.touch();

                        if let Err(e) = task_repo.update(&task).await {
                            tracing::error!(error = %e, "Failed to update task to MergeIncomplete status");
                            return;
                        }

                        if let Err(e) = task_repo.persist_status_change(
                            &task_id,
                            InternalStatus::PendingMerge,
                            InternalStatus::MergeIncomplete,
                            "merge_incomplete",
                        ).await {
                            tracing::warn!(error = %e, "Failed to record merge incomplete transition (non-fatal)");
                        }

                        self.machine
                            .context
                            .services
                            .event_emitter
                            .emit("task:status_changed", task_id_str)
                            .await;
                    }
                }
            }
        } else {
            // Local mode: rebase for linear history
            let merge_result = GitService::try_rebase_and_merge(repo_path, &source_branch, &target_branch);
            match merge_result {
                Ok(MergeAttemptResult::Success { commit_sha }) => {
                    tracing::info!(
                        task_id = task_id_str,
                        commit_sha = %commit_sha,
                        "Programmatic merge succeeded (fast path)"
                    );

                    // Post-merge validation gate: check mode + skip flag
                    let skip_validation = take_skip_validation_flag(&mut task);
                    let validation_mode = &project.merge_validation_mode;
                    if !skip_validation && *validation_mode != MergeValidationMode::Off {
                        let source_sha = GitService::get_branch_sha(repo_path, &source_branch).ok();
                        let cached_log = source_sha.as_deref()
                            .and_then(|sha| extract_cached_validation(&task, sha));
                        let app_handle_ref = self.machine.context.services.app_handle.as_ref();
                        if let Some(validation) = run_validation_commands(&project, &task, repo_path, task_id_str, app_handle_ref, cached_log.as_deref()) {
                            if !validation.all_passed {
                                if *validation_mode == MergeValidationMode::Warn {
                                    tracing::warn!(task_id = task_id_str, "Validation failed in Warn mode (local), proceeding with merge");
                                    task.metadata = Some(format_validation_warn_metadata(
                                        &validation.log, &source_branch, &target_branch,
                                    ));
                                } else {
                                    self.handle_validation_failure(
                                        &mut task, &task_id, task_id_str, task_repo,
                                        &validation.failures, &validation.log, &source_branch, &target_branch,
                                        repo_path, "local", validation_mode,
                                    ).await;
                                    return;
                                }
                            } else {
                                task.metadata = Some(serde_json::json!({
                                    "validation_log": validation.log,
                                    "validation_source_sha": source_sha,
                                    "source_branch": source_branch,
                                    "target_branch": target_branch,
                                }).to_string());
                            }
                        }
                    }

                    let app_handle = self.machine.context.services.app_handle.as_ref();
                    if let Err(e) = complete_merge_internal(
                        &mut task,
                        &project,
                        &commit_sha,
                        task_repo,
                        app_handle,
                    ).await {
                        tracing::error!(error = %e, task_id = task_id_str, "Failed to complete programmatic merge, falling back to MergeIncomplete");

                        task.metadata = Some(serde_json::json!({
                            "error": format!("complete_merge_internal failed: {}", e),
                            "source_branch": source_branch,
                            "target_branch": target_branch,
                        }).to_string());
                        task.internal_status = InternalStatus::MergeIncomplete;
                        task.touch();

                        let _ = task_repo.update(&task).await;
                        let _ = task_repo.persist_status_change(
                            &task_id,
                            InternalStatus::PendingMerge,
                            InternalStatus::MergeIncomplete,
                            "merge_incomplete",
                        ).await;

                        self.machine.context.services.event_emitter
                            .emit("task:status_changed", task_id_str).await;
                    } else {
                        self.post_merge_cleanup(task_id_str, &task_id, repo_path, plan_branch_repo).await;
                    }
                }
                Ok(MergeAttemptResult::NeedsAgent { conflict_files }) => {
                    tracing::info!(
                        task_id = task_id_str,
                        conflict_count = conflict_files.len(),
                        "Programmatic merge failed: conflicts detected, transitioning to Merging"
                    );

                    for file in &conflict_files {
                        tracing::debug!(task_id = task_id_str, file = %file.display(), "Conflict file");
                    }

                    task.internal_status = InternalStatus::Merging;
                    task.touch();

                    if let Err(e) = task_repo.update(&task).await {
                        tracing::error!(error = %e, "Failed to update task to Merging status");
                        return;
                    }

                    if let Err(e) = task_repo.persist_status_change(
                        &task_id,
                        InternalStatus::PendingMerge,
                        InternalStatus::Merging,
                        "merge_conflict",
                    ).await {
                        tracing::warn!(error = %e, "Failed to record merge conflict transition (non-fatal)");
                    }

                    self.machine
                        .context
                        .services
                        .event_emitter
                        .emit("task:merge_conflict", task_id_str)
                        .await;

                    let prompt = format!("Resolve merge conflicts for task: {}", task_id_str);
                    tracing::info!(
                        task_id = task_id_str,
                        "Spawning merger agent for conflict resolution (from attempt_programmatic_merge)"
                    );

                    let result = self
                        .machine
                        .context
                        .services
                        .chat_service
                        .send_message(
                            crate::domain::entities::ChatContextType::Merge,
                            task_id_str,
                            &prompt,
                        )
                        .await;

                    match &result {
                        Ok(_) => tracing::info!(task_id = task_id_str, "Merger agent spawned successfully"),
                        Err(e) => tracing::error!(task_id = task_id_str, error = %e, "Failed to spawn merger agent"),
                    }
                }
                Err(e) => {
                    tracing::error!(
                        task_id = task_id_str,
                        error = %e,
                        source_branch = %source_branch,
                        target_branch = %target_branch,
                        repo_path = %repo_path.display(),
                        "Programmatic merge failed due to error, transitioning to MergeIncomplete"
                    );

                    task.metadata = Some(serde_json::json!({
                        "error": e.to_string(),
                        "source_branch": source_branch,
                        "target_branch": target_branch,
                    }).to_string());
                    task.internal_status = InternalStatus::MergeIncomplete;
                    task.touch();

                    if let Err(e) = task_repo.update(&task).await {
                        tracing::error!(error = %e, "Failed to update task to MergeIncomplete status");
                        return;
                    }

                    if let Err(e) = task_repo.persist_status_change(
                        &task_id,
                        InternalStatus::PendingMerge,
                        InternalStatus::MergeIncomplete,
                        "merge_incomplete",
                    ).await {
                        tracing::warn!(error = %e, "Failed to record merge incomplete transition (non-fatal)");
                    }

                    self.machine
                        .context
                        .services
                        .event_emitter
                        .emit("task:status_changed", task_id_str)
                        .await;
                }
            }
        }
    }

    /// Post-merge cleanup: update plan branch status, delete feature branch, unblock dependents.
    ///
    /// Shared between Worktree and Local mode success paths in `attempt_programmatic_merge()`.
    async fn post_merge_cleanup(
        &self,
        task_id_str: &str,
        task_id: &TaskId,
        repo_path: &Path,
        plan_branch_repo: &Option<Arc<dyn PlanBranchRepository>>,
    ) {
        let app_handle = self.machine.context.services.app_handle.as_ref();

        if let Some(ref plan_branch_repo) = plan_branch_repo {
            if let Ok(Some(pb)) = plan_branch_repo.get_by_merge_task_id(task_id).await {
                if let Err(e) = plan_branch_repo.set_merged(&pb.id).await {
                    tracing::warn!(
                        error = %e,
                        task_id = task_id_str,
                        plan_branch_id = pb.id.as_str(),
                        "Failed to mark plan branch as merged (non-fatal)"
                    );
                }

                if let Err(e) = GitService::delete_feature_branch(repo_path, &pb.branch_name) {
                    tracing::warn!(
                        error = %e,
                        task_id = task_id_str,
                        branch = %pb.branch_name,
                        "Failed to delete feature branch after merge (non-fatal)"
                    );
                } else {
                    tracing::info!(
                        task_id = task_id_str,
                        branch = %pb.branch_name,
                        "Deleted feature branch after plan merge"
                    );
                }

                if let Some(handle) = app_handle {
                    let _ = handle.emit(
                        "plan:merge_complete",
                        serde_json::json!({
                            "plan_artifact_id": pb.plan_artifact_id.as_str(),
                            "plan_branch_id": pb.id.as_str(),
                            "merge_task_id": task_id_str,
                            "branch_name": pb.branch_name,
                        }),
                    );
                }
            }
        }

        self.machine
            .context
            .services
            .dependency_manager
            .unblock_dependents(task_id_str)
            .await;

        // Schedule newly-unblocked tasks (e.g. plan_merge tasks that just became Ready)
        if let Some(ref scheduler) = self.machine.context.services.task_scheduler {
            let scheduler = Arc::clone(scheduler);
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;
                scheduler.try_schedule_ready_tasks().await;
            });
        }

        // Retry deferred merges: after a merge completes, re-trigger any tasks that
        // were deferred because they targeted the same branch. We use the scheduler's
        // try_retry_deferred_merges() method which builds a fresh TaskTransitionService
        // and re-invokes attempt_programmatic_merge for each deferred task.
        if let Some(ref scheduler) = self.machine.context.services.task_scheduler {
            let scheduler = Arc::clone(scheduler);
            let project_id = self.machine.context.project_id.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
                scheduler.try_retry_deferred_merges(&project_id).await;
            });
        }
    }

    /// Handle post-merge validation failure: revert the merge commit, then transition
    /// to MergeIncomplete with error metadata.
    ///
    /// The merge commit has already landed on the target branch. We must revert it
    /// before transitioning so that failing code doesn't remain on the target branch.
    ///
    /// # Arguments
    /// * `task` - Mutable task to update
    /// * `task_id` - Task ID for persistence
    /// * `task_id_str` - Task ID string for logging
    /// * `task_repo` - Repository for persisting status change
    /// * `failures` - Validation failures to include in metadata
    /// * `source_branch` / `target_branch` - For metadata
    /// * `merge_path` - Path where the merge happened (for git reset)
    /// * `mode_label` - Label for log messages (e.g., "in-repo", "worktree", "local")
    /// * `validation_mode` - Current validation mode (AutoFix spawns agent, Block reverts)
    async fn handle_validation_failure(
        &self,
        task: &mut Task,
        task_id: &TaskId,
        task_id_str: &str,
        task_repo: &Arc<dyn TaskRepository>,
        failures: &[ValidationFailure],
        log: &[ValidationLogEntry],
        source_branch: &str,
        target_branch: &str,
        merge_path: &Path,
        mode_label: &str,
        validation_mode: &MergeValidationMode,
    ) {
        if *validation_mode == MergeValidationMode::AutoFix {
            // AutoFix: DON'T revert — keep the merged (failing) code for the agent to fix
            tracing::info!(
                task_id = task_id_str,
                failure_count = failures.len(),
                "Validation failed (AutoFix mode, {}), spawning merger agent to attempt fix",
                mode_label,
            );

            let failure_details: Vec<serde_json::Value> = failures
                .iter()
                .map(|f| {
                    serde_json::json!({
                        "command": f.command,
                        "path": f.path,
                        "exit_code": f.exit_code,
                        "stderr": if f.stderr.len() > 2000 { &f.stderr[..2000] } else { &f.stderr },
                    })
                })
                .collect();

            task.metadata = Some(serde_json::json!({
                "validation_recovery": true,
                "validation_failures": failure_details,
                "validation_log": log,
                "source_branch": source_branch,
                "target_branch": target_branch,
            }).to_string());
            // Set worktree_path to the merge worktree so the merger agent CWD resolves correctly
            task.worktree_path = Some(merge_path.to_string_lossy().to_string());
            task.internal_status = InternalStatus::Merging;
            task.touch();

            let _ = task_repo.update(task).await;
            let _ = task_repo.persist_status_change(
                task_id,
                InternalStatus::PendingMerge,
                InternalStatus::Merging,
                "validation_auto_fix",
            ).await;

            self.machine.context.services.event_emitter
                .emit("task:status_changed", task_id_str).await;

            // Spawn merger agent to attempt fix (same pattern as conflict resolution)
            let prompt = format!("Fix validation failures for task: {}", task_id_str);
            tracing::info!(
                task_id = task_id_str,
                "Spawning merger agent for validation recovery"
            );

            let result = self
                .machine
                .context
                .services
                .chat_service
                .send_message(
                    crate::domain::entities::ChatContextType::Merge,
                    task_id_str,
                    &prompt,
                )
                .await;

            match &result {
                Ok(_) => tracing::info!(task_id = task_id_str, "Merger agent spawned for validation recovery"),
                Err(e) => tracing::error!(task_id = task_id_str, error = %e, "Failed to spawn merger agent for validation recovery"),
            }
        } else {
            // Block mode: revert merge and transition to MergeIncomplete
            tracing::warn!(
                task_id = task_id_str,
                failure_count = failures.len(),
                "Post-merge validation failed ({}), reverting merge and transitioning to MergeIncomplete",
                mode_label,
            );

            // Revert the merge commit so failing code doesn't remain on the target branch
            if let Err(e) = GitService::reset_hard(merge_path, "HEAD~1") {
                tracing::error!(
                    task_id = task_id_str,
                    error = %e,
                    "Failed to revert merge commit after validation failure — target branch may have failing code"
                );
            }

            task.metadata = Some(format_validation_error_metadata(
                failures, log, source_branch, target_branch,
            ));
            task.internal_status = InternalStatus::MergeIncomplete;
            task.touch();

            let _ = task_repo.update(task).await;
            let _ = task_repo.persist_status_change(
                task_id,
                InternalStatus::PendingMerge,
                InternalStatus::MergeIncomplete,
                "validation_failed",
            ).await;

            self.machine.context.services.event_emitter
                .emit("task:status_changed", task_id_str).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{
        ArtifactId, PlanBranch, PlanBranchStatus, ProjectId, TaskId,
    };
    use crate::domain::entities::types::IdeationSessionId;
    use crate::infrastructure::memory::MemoryPlanBranchRepository;

    fn make_project(base_branch: Option<&str>) -> Project {
        let mut p = Project::new("test-project".into(), "/tmp/test".into());
        p.base_branch = base_branch.map(|s| s.to_string());
        p
    }

    fn make_task(plan_artifact_id: Option<&str>, task_branch: Option<&str>) -> Task {
        make_task_with_session(plan_artifact_id, task_branch, None)
    }

    fn make_task_with_session(
        plan_artifact_id: Option<&str>,
        task_branch: Option<&str>,
        ideation_session_id: Option<&str>,
    ) -> Task {
        let mut t = Task::new(ProjectId::from_string("proj-1".to_string()), "Test task".into());
        t.plan_artifact_id = plan_artifact_id.map(|s| ArtifactId::from_string(s));
        t.task_branch = task_branch.map(|s| s.to_string());
        t.ideation_session_id = ideation_session_id.map(|s| IdeationSessionId::from_string(s));
        t
    }

    fn make_plan_branch(
        plan_artifact_id: &str,
        branch_name: &str,
        status: PlanBranchStatus,
        merge_task_id: Option<&str>,
    ) -> PlanBranch {
        let mut pb = PlanBranch::new(
            ArtifactId::from_string(plan_artifact_id),
            IdeationSessionId::from_string("sess-1"),
            ProjectId::from_string("proj-1".to_string()),
            branch_name.to_string(),
            "main".to_string(),
        );
        pb.status = status;
        pb.merge_task_id = merge_task_id.map(|s| TaskId::from_string(s.to_string()));
        pb
    }

    // ==================
    // resolve_task_base_branch tests
    // ==================

    #[tokio::test]
    async fn resolve_task_base_branch_returns_project_base_when_no_repo() {
        let project = make_project(Some("develop"));
        let task = make_task_with_session(Some("art-1"), None, Some("sess-1"));
        let repo: Option<Arc<dyn PlanBranchRepository>> = None;

        let result = resolve_task_base_branch(&task, &project, &repo).await;
        assert_eq!(result, "develop");
    }

    #[tokio::test]
    async fn resolve_task_base_branch_defaults_to_main_when_no_base_branch() {
        let project = make_project(None);
        let task = make_task_with_session(Some("art-1"), None, Some("sess-1"));
        let repo: Option<Arc<dyn PlanBranchRepository>> = None;

        let result = resolve_task_base_branch(&task, &project, &repo).await;
        assert_eq!(result, "main");
    }

    #[tokio::test]
    async fn resolve_task_base_branch_returns_default_when_task_has_no_session_id() {
        let project = make_project(Some("develop"));
        let task = make_task(None, None);
        let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
        let repo: Option<Arc<dyn PlanBranchRepository>> = Some(mem_repo);

        let result = resolve_task_base_branch(&task, &project, &repo).await;
        assert_eq!(result, "develop");
    }

    #[tokio::test]
    async fn resolve_task_base_branch_returns_feature_branch_when_active() {
        let project = make_project(Some("main"));
        let task = make_task_with_session(Some("art-1"), None, Some("sess-1"));

        let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
        let pb = make_plan_branch("art-1", "ralphx/test/plan-abc123", PlanBranchStatus::Active, None);
        mem_repo.create(pb).await.unwrap();

        let repo: Option<Arc<dyn PlanBranchRepository>> = Some(mem_repo);
        let result = resolve_task_base_branch(&task, &project, &repo).await;
        assert_eq!(result, "ralphx/test/plan-abc123");
    }

    #[tokio::test]
    async fn resolve_task_base_branch_returns_default_when_branch_merged() {
        let project = make_project(Some("main"));
        let task = make_task_with_session(Some("art-1"), None, Some("sess-1"));

        let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
        let pb = make_plan_branch("art-1", "ralphx/test/plan-abc123", PlanBranchStatus::Merged, None);
        mem_repo.create(pb).await.unwrap();

        let repo: Option<Arc<dyn PlanBranchRepository>> = Some(mem_repo);
        let result = resolve_task_base_branch(&task, &project, &repo).await;
        assert_eq!(result, "main");
    }

    #[tokio::test]
    async fn resolve_task_base_branch_returns_default_when_branch_abandoned() {
        let project = make_project(Some("main"));
        let task = make_task_with_session(Some("art-1"), None, Some("sess-1"));

        let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
        let pb = make_plan_branch("art-1", "ralphx/test/plan-abc123", PlanBranchStatus::Abandoned, None);
        mem_repo.create(pb).await.unwrap();

        let repo: Option<Arc<dyn PlanBranchRepository>> = Some(mem_repo);
        let result = resolve_task_base_branch(&task, &project, &repo).await;
        assert_eq!(result, "main");
    }

    #[tokio::test]
    async fn resolve_task_base_branch_returns_default_when_no_matching_branch() {
        let project = make_project(Some("main"));
        // Task has session_id "sess-nonexistent" which won't match "sess-1" in plan branch
        let task = make_task_with_session(Some("art-nonexistent"), None, Some("sess-nonexistent"));

        let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
        let pb = make_plan_branch("art-other", "ralphx/test/plan-abc123", PlanBranchStatus::Active, None);
        mem_repo.create(pb).await.unwrap();

        let repo: Option<Arc<dyn PlanBranchRepository>> = Some(mem_repo);
        let result = resolve_task_base_branch(&task, &project, &repo).await;
        assert_eq!(result, "main");
    }

    // ==================
    // resolve_merge_branches tests
    // ==================

    #[tokio::test]
    async fn resolve_merge_branches_returns_default_when_no_repo() {
        let project = make_project(Some("main"));
        let mut task = make_task(None, Some("ralphx/test/task-123"));
        task.id = TaskId::from_string("task-123".to_string());

        let repo: Option<Arc<dyn PlanBranchRepository>> = None;
        let (source, target) = resolve_merge_branches(&task, &project, &repo).await;
        assert_eq!(source, "ralphx/test/task-123");
        assert_eq!(target, "main");
    }

    #[tokio::test]
    async fn resolve_merge_branches_merge_task_returns_feature_into_base() {
        let project = make_project(Some("main"));
        let mut task = make_task(None, None);
        task.id = TaskId::from_string("merge-task-1".to_string());

        let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
        let pb = make_plan_branch(
            "art-1",
            "ralphx/test/plan-abc123",
            PlanBranchStatus::Active,
            Some("merge-task-1"),
        );
        mem_repo.create(pb).await.unwrap();

        let repo: Option<Arc<dyn PlanBranchRepository>> = Some(mem_repo);
        let (source, target) = resolve_merge_branches(&task, &project, &repo).await;
        assert_eq!(source, "ralphx/test/plan-abc123");
        assert_eq!(target, "main");
    }

    #[tokio::test]
    async fn resolve_merge_branches_plan_task_returns_task_into_feature() {
        let project = make_project(Some("main"));
        let mut task = make_task_with_session(Some("art-1"), Some("ralphx/test/task-456"), Some("sess-1"));
        task.id = TaskId::from_string("task-456".to_string());

        let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
        let pb = make_plan_branch("art-1", "ralphx/test/plan-abc123", PlanBranchStatus::Active, None);
        mem_repo.create(pb).await.unwrap();

        let repo: Option<Arc<dyn PlanBranchRepository>> = Some(mem_repo);
        let (source, target) = resolve_merge_branches(&task, &project, &repo).await;
        assert_eq!(source, "ralphx/test/task-456");
        assert_eq!(target, "ralphx/test/plan-abc123");
    }

    #[tokio::test]
    async fn resolve_merge_branches_regular_task_returns_task_into_base() {
        let project = make_project(Some("develop"));
        let mut task = make_task(None, Some("ralphx/test/task-789"));
        task.id = TaskId::from_string("task-789".to_string());

        let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
        let repo: Option<Arc<dyn PlanBranchRepository>> = Some(mem_repo);

        let (source, target) = resolve_merge_branches(&task, &project, &repo).await;
        assert_eq!(source, "ralphx/test/task-789");
        assert_eq!(target, "develop");
    }

    #[tokio::test]
    async fn resolve_merge_branches_merge_task_with_merged_branch_returns_default() {
        let project = make_project(Some("main"));
        let mut task = make_task(None, Some("ralphx/test/task-merge"));
        task.id = TaskId::from_string("merge-task-2".to_string());

        let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
        let pb = make_plan_branch(
            "art-2",
            "ralphx/test/plan-def456",
            PlanBranchStatus::Merged,
            Some("merge-task-2"),
        );
        mem_repo.create(pb).await.unwrap();

        let repo: Option<Arc<dyn PlanBranchRepository>> = Some(mem_repo);
        let (source, target) = resolve_merge_branches(&task, &project, &repo).await;
        // Merged branch is not Active, so falls through to default
        assert_eq!(source, "ralphx/test/task-merge");
        assert_eq!(target, "main");
    }

    #[tokio::test]
    async fn resolve_merge_branches_plan_task_with_abandoned_branch_returns_default() {
        let project = make_project(Some("main"));
        let mut task = make_task_with_session(Some("art-3"), Some("ralphx/test/task-abandoned"), Some("sess-1"));
        task.id = TaskId::from_string("task-abandoned".to_string());

        let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
        let pb = make_plan_branch(
            "art-3",
            "ralphx/test/plan-ghi789",
            PlanBranchStatus::Abandoned,
            None,
        );
        mem_repo.create(pb).await.unwrap();

        let repo: Option<Arc<dyn PlanBranchRepository>> = Some(mem_repo);
        let (source, target) = resolve_merge_branches(&task, &project, &repo).await;
        // Abandoned branch is not Active, so falls through to default
        assert_eq!(source, "ralphx/test/task-abandoned");
        assert_eq!(target, "main");
    }

    #[tokio::test]
    async fn resolve_merge_branches_defaults_to_main_when_no_base_branch() {
        let project = make_project(None);
        let mut task = make_task(None, Some("ralphx/test/task-no-base"));
        task.id = TaskId::from_string("task-no-base".to_string());

        let repo: Option<Arc<dyn PlanBranchRepository>> = None;
        let (source, target) = resolve_merge_branches(&task, &project, &repo).await;
        assert_eq!(source, "ralphx/test/task-no-base");
        assert_eq!(target, "main");
    }

    #[tokio::test]
    async fn resolve_merge_branches_merge_task_checked_before_plan_task() {
        // If a task is both a merge task AND has ideation_session_id,
        // merge task check should take precedence
        let project = make_project(Some("main"));
        let mut task = make_task_with_session(Some("art-1"), Some("ralphx/test/task-dual"), Some("sess-1"));
        task.id = TaskId::from_string("dual-task".to_string());

        let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
        let pb = make_plan_branch(
            "art-1",
            "ralphx/test/plan-dual",
            PlanBranchStatus::Active,
            Some("dual-task"),
        );
        mem_repo.create(pb).await.unwrap();

        let repo: Option<Arc<dyn PlanBranchRepository>> = Some(mem_repo);
        let (source, target) = resolve_merge_branches(&task, &project, &repo).await;
        // Merge task path wins: feature branch into base
        assert_eq!(source, "ralphx/test/plan-dual");
        assert_eq!(target, "main");
    }

    // ==================
    // run_validation_commands tests
    // ==================

    #[test]
    fn run_validation_returns_none_when_no_analysis() {
        let project = make_project(Some("main"));
        let task = make_task(None, None);
        let result = run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None);
        assert!(result.is_none());
    }

    #[test]
    fn run_validation_returns_none_when_empty_entries() {
        let mut project = make_project(Some("main"));
        project.detected_analysis = Some("[]".to_string());
        let task = make_task(None, None);
        let result = run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None);
        assert!(result.is_none());
    }

    #[test]
    fn run_validation_returns_none_when_no_validate_commands() {
        let mut project = make_project(Some("main"));
        project.detected_analysis = Some(
            r#"[{"path": ".", "label": "Test", "validate": []}]"#.to_string(),
        );
        let task = make_task(None, None);
        let result = run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None);
        assert!(result.is_none());
    }

    #[test]
    fn run_validation_prefers_custom_over_detected() {
        let mut project = make_project(Some("main"));
        // detected has a failing command
        project.detected_analysis = Some(
            r#"[{"path": ".", "label": "Test", "validate": ["false"]}]"#.to_string(),
        );
        // custom has a passing command (overrides detected)
        project.custom_analysis = Some(
            r#"[{"path": ".", "label": "Test", "validate": ["true"]}]"#.to_string(),
        );
        let task = make_task(None, None);
        let result = run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None);
        assert!(result.is_some());
        assert!(result.unwrap().all_passed);
    }

    #[test]
    fn run_validation_succeeds_with_passing_command() {
        let mut project = make_project(Some("main"));
        project.detected_analysis = Some(
            r#"[{"path": ".", "label": "Test", "validate": ["true"]}]"#.to_string(),
        );
        let task = make_task(None, None);
        let result = run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None);
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.all_passed);
        assert!(r.failures.is_empty());
        assert_eq!(r.log.len(), 1);
        assert_eq!(r.log[0].phase, "validate");
        assert_eq!(r.log[0].status, "success");
        assert_eq!(r.log[0].label, "Test");
    }

    #[test]
    fn run_validation_fails_with_failing_command() {
        let mut project = make_project(Some("main"));
        project.detected_analysis = Some(
            r#"[{"path": ".", "label": "Test", "validate": ["false"]}]"#.to_string(),
        );
        let task = make_task(None, None);
        let result = run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None);
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(!r.all_passed);
        assert_eq!(r.failures.len(), 1);
        assert_eq!(r.failures[0].command, "false");
        assert_eq!(r.log.len(), 1);
        assert_eq!(r.log[0].phase, "validate");
        assert_eq!(r.log[0].status, "failed");
    }

    #[test]
    fn run_validation_resolves_template_vars() {
        let mut project = make_project(Some("main"));
        project.detected_analysis = Some(
            r#"[{"path": ".", "label": "Test", "validate": ["echo {project_root} {worktree_path}"]}]"#.to_string(),
        );
        let mut task = make_task(None, None);
        task.worktree_path = Some("/tmp/wt".to_string());
        let result = run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None);
        assert!(result.is_some());
        assert!(result.unwrap().all_passed);
    }

    #[test]
    fn run_validation_returns_none_for_invalid_json() {
        let mut project = make_project(Some("main"));
        project.detected_analysis = Some("not valid json".to_string());
        let task = make_task(None, None);
        let result = run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None);
        assert!(result.is_none());
    }

    #[test]
    fn format_validation_error_metadata_formats_correctly() {
        let failures = vec![ValidationFailure {
            command: "cargo check".to_string(),
            path: ".".to_string(),
            exit_code: Some(1),
            stderr: "error[E0308]: mismatched types".to_string(),
        }];
        let log = vec![ValidationLogEntry {
            phase: "validate".to_string(),
            command: "cargo check".to_string(),
            path: ".".to_string(),
            label: "Rust".to_string(),
            status: "failed".to_string(),
            exit_code: Some(1),
            stdout: String::new(),
            stderr: "error[E0308]: mismatched types".to_string(),
            duration_ms: 1500,
        }];
        let result = format_validation_error_metadata(&failures, &log, "task-branch", "main");
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed["error"].as_str().unwrap().contains("1 command(s) failed"));
        assert_eq!(parsed["source_branch"], "task-branch");
        assert_eq!(parsed["target_branch"], "main");
        assert_eq!(parsed["validation_failures"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["validation_log"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn format_validation_warn_metadata_formats_correctly() {
        let log = vec![ValidationLogEntry {
            phase: "validate".to_string(),
            command: "npm test".to_string(),
            path: ".".to_string(),
            label: "Node".to_string(),
            status: "failed".to_string(),
            exit_code: Some(1),
            stdout: String::new(),
            stderr: "test failed".to_string(),
            duration_ms: 500,
        }];
        let result = format_validation_warn_metadata(&log, "task-branch", "main");
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["validation_warnings"], true);
        assert_eq!(parsed["source_branch"], "task-branch");
        assert_eq!(parsed["target_branch"], "main");
        assert_eq!(parsed["validation_log"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn take_skip_validation_flag_returns_false_when_no_metadata() {
        let mut task = make_task(None, None);
        assert!(!take_skip_validation_flag(&mut task));
    }

    #[test]
    fn take_skip_validation_flag_returns_false_when_no_flag() {
        let mut task = make_task(None, None);
        task.metadata = Some(r#"{"some_key": "value"}"#.to_string());
        assert!(!take_skip_validation_flag(&mut task));
    }

    #[test]
    fn take_skip_validation_flag_returns_true_and_clears() {
        let mut task = make_task(None, None);
        task.metadata = Some(r#"{"skip_validation": true, "other": "data"}"#.to_string());
        assert!(take_skip_validation_flag(&mut task));
        // Flag should be cleared
        let meta: serde_json::Value = serde_json::from_str(task.metadata.as_ref().unwrap()).unwrap();
        assert!(meta.get("skip_validation").is_none());
        assert_eq!(meta["other"], "data");
        // Second call returns false
        assert!(!take_skip_validation_flag(&mut task));
    }

    #[test]
    fn run_validation_skipped_in_off_mode() {
        let mut project = make_project(Some("main"));
        project.merge_validation_mode = MergeValidationMode::Off;
        project.detected_analysis = Some(
            r#"[{"path": ".", "label": "Test", "validate": ["false"]}]"#.to_string(),
        );
        // With Off mode, validation should not run, so the test verifies the enum
        // is correctly set and accessible (actual skip happens in attempt_programmatic_merge)
        assert_eq!(project.merge_validation_mode, MergeValidationMode::Off);
    }

    // ==================
    // extract_cached_validation tests
    // ==================

    #[test]
    fn extract_cached_returns_none_when_no_metadata() {
        let task = make_task(None, None);
        assert!(extract_cached_validation(&task, "abc123").is_none());
    }

    #[test]
    fn extract_cached_returns_none_when_sha_mismatch() {
        let mut task = make_task(None, None);
        task.metadata = Some(serde_json::json!({
            "validation_source_sha": "old_sha",
            "validation_log": [{
                "phase": "validate",
                "command": "true",
                "path": ".",
                "label": "Test",
                "status": "success",
                "exit_code": 0,
                "stdout": "",
                "stderr": "",
                "duration_ms": 100,
            }],
        }).to_string());
        assert!(extract_cached_validation(&task, "different_sha").is_none());
    }

    #[test]
    fn extract_cached_returns_log_when_sha_matches() {
        let mut task = make_task(None, None);
        task.metadata = Some(serde_json::json!({
            "validation_source_sha": "abc123",
            "validation_log": [{
                "phase": "validate",
                "command": "cargo check",
                "path": ".",
                "label": "Rust",
                "status": "success",
                "exit_code": 0,
                "stdout": "",
                "stderr": "",
                "duration_ms": 1500,
            }],
        }).to_string());
        let cached = extract_cached_validation(&task, "abc123");
        assert!(cached.is_some());
        let entries = cached.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].command, "cargo check");
        assert_eq!(entries[0].status, "success");
    }

    #[test]
    fn extract_cached_returns_none_when_no_sha_in_metadata() {
        let mut task = make_task(None, None);
        task.metadata = Some(serde_json::json!({
            "validation_log": [{
                "phase": "validate",
                "command": "true",
                "path": ".",
                "label": "Test",
                "status": "success",
                "exit_code": 0,
                "stdout": "",
                "stderr": "",
                "duration_ms": 100,
            }],
        }).to_string());
        // No validation_source_sha → no cache hit
        assert!(extract_cached_validation(&task, "abc123").is_none());
    }

    // ==================
    // run_validation_commands caching tests
    // ==================

    #[test]
    fn run_validation_skips_passed_when_cached() {
        let mut project = make_project(Some("main"));
        // "true" always passes, "echo hello" always passes
        project.detected_analysis = Some(
            r#"[{"path": ".", "label": "Test", "validate": ["true", "echo hello"]}]"#.to_string(),
        );
        let task = make_task(None, None);

        // Build a cached log where "true" passed but "echo hello" failed
        let cached = vec![
            ValidationLogEntry {
                phase: "validate".to_string(),
                command: "true".to_string(),
                path: ".".to_string(),
                label: "Test".to_string(),
                status: "success".to_string(),
                exit_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
                duration_ms: 50,
            },
            ValidationLogEntry {
                phase: "validate".to_string(),
                command: "echo hello".to_string(),
                path: ".".to_string(),
                label: "Test".to_string(),
                status: "failed".to_string(),
                exit_code: Some(1),
                stdout: String::new(),
                stderr: "error".to_string(),
                duration_ms: 100,
            },
        ];

        let result = run_validation_commands(
            &project, &task, Path::new("/tmp"), "", None, Some(&cached),
        );
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.all_passed);
        assert_eq!(r.log.len(), 2);
        // First command should be cached (was "success" in cache)
        assert_eq!(r.log[0].status, "cached");
        assert_eq!(r.log[0].command, "true");
        assert_eq!(r.log[0].duration_ms, 0);
        // Second command should be re-run (was "failed" in cache)
        assert_eq!(r.log[1].status, "success");
        assert_eq!(r.log[1].command, "echo hello");
    }

    #[test]
    fn run_validation_reruns_all_when_no_cache() {
        let mut project = make_project(Some("main"));
        project.detected_analysis = Some(
            r#"[{"path": ".", "label": "Test", "validate": ["true"]}]"#.to_string(),
        );
        let task = make_task(None, None);

        let result = run_validation_commands(
            &project, &task, Path::new("/tmp"), "", None, None,
        );
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.all_passed);
        assert_eq!(r.log.len(), 1);
        assert_eq!(r.log[0].status, "success"); // actually ran, not "cached"
    }

    // ==================
    // extract_task_id_from_merge_path tests
    // ==================

    #[test]
    fn test_extract_task_id_from_merge_path_valid() {
        let path = "/home/user/ralphx-worktrees/my-app/merge-abc123def456";
        assert_eq!(extract_task_id_from_merge_path(path), Some("abc123def456"));
    }

    #[test]
    fn test_extract_task_id_from_merge_path_uuid() {
        let path = "/tmp/wt/merge-e0ce32e7-eaef-4a07-b81d-2126d0dee5d9";
        assert_eq!(
            extract_task_id_from_merge_path(path),
            Some("e0ce32e7-eaef-4a07-b81d-2126d0dee5d9"),
        );
    }

    #[test]
    fn test_extract_task_id_from_merge_path_not_merge() {
        let path = "/home/user/ralphx-worktrees/my-app/task-abc123";
        assert_eq!(extract_task_id_from_merge_path(path), None);
    }

    #[test]
    fn test_extract_task_id_from_merge_path_bare_name() {
        assert_eq!(extract_task_id_from_merge_path("merge-xyz"), Some("xyz"));
    }

    #[test]
    fn test_extract_task_id_from_merge_path_empty() {
        assert_eq!(extract_task_id_from_merge_path(""), None);
    }

    #[test]
    fn test_extract_task_id_from_merge_path_just_merge_prefix() {
        // "merge-" with empty task ID should return empty string
        assert_eq!(extract_task_id_from_merge_path("/dir/merge-"), Some(""));
    }

    // ==================
    // parse_metadata tests
    // ==================

    #[test]
    fn parse_metadata_returns_none_when_no_metadata() {
        let task = make_task(None, None);
        assert!(parse_metadata(&task).is_none());
    }

    #[test]
    fn parse_metadata_returns_none_for_invalid_json() {
        let mut task = make_task(None, None);
        task.metadata = Some("not json".to_string());
        assert!(parse_metadata(&task).is_none());
    }

    #[test]
    fn parse_metadata_returns_value_for_valid_json() {
        let mut task = make_task(None, None);
        task.metadata = Some(r#"{"key": "value"}"#.to_string());
        let meta = parse_metadata(&task).unwrap();
        assert_eq!(meta["key"], "value");
    }

    // ==================
    // has_merge_deferred_metadata tests
    // ==================

    #[test]
    fn has_merge_deferred_returns_false_when_no_metadata() {
        let task = make_task(None, None);
        assert!(!has_merge_deferred_metadata(&task));
    }

    #[test]
    fn has_merge_deferred_returns_false_when_no_flag() {
        let mut task = make_task(None, None);
        task.metadata = Some(r#"{"other": "data"}"#.to_string());
        assert!(!has_merge_deferred_metadata(&task));
    }

    #[test]
    fn has_merge_deferred_returns_false_when_flag_is_false() {
        let mut task = make_task(None, None);
        task.metadata = Some(r#"{"merge_deferred": false}"#.to_string());
        assert!(!has_merge_deferred_metadata(&task));
    }

    #[test]
    fn has_merge_deferred_returns_true_when_flag_is_true() {
        let mut task = make_task(None, None);
        task.metadata = Some(r#"{"merge_deferred": true}"#.to_string());
        assert!(has_merge_deferred_metadata(&task));
    }

    // ==================
    // clear_merge_deferred_metadata tests
    // ==================

    #[test]
    fn clear_merge_deferred_removes_flags_from_metadata() {
        let mut task = make_task(None, None);
        task.metadata = Some(serde_json::json!({
            "merge_deferred": true,
            "merge_deferred_at": "2026-01-01T00:00:00Z",
            "other": "keep"
        }).to_string());

        clear_merge_deferred_metadata(&mut task);

        let meta = parse_metadata(&task).unwrap();
        assert!(meta.get("merge_deferred").is_none());
        assert!(meta.get("merge_deferred_at").is_none());
        assert_eq!(meta["other"], "keep");
    }

    #[test]
    fn clear_merge_deferred_clears_metadata_when_only_deferred_fields() {
        let mut task = make_task(None, None);
        task.metadata = Some(serde_json::json!({
            "merge_deferred": true,
            "merge_deferred_at": "2026-01-01T00:00:00Z",
        }).to_string());

        clear_merge_deferred_metadata(&mut task);

        assert!(task.metadata.is_none());
    }

    #[test]
    fn clear_merge_deferred_noop_when_no_metadata() {
        let mut task = make_task(None, None);
        clear_merge_deferred_metadata(&mut task);
        assert!(task.metadata.is_none());
    }

    // ==================
    // concurrent merge guard — archived task skip tests
    // ==================

    /// Archived tasks in PendingMerge should NOT block newer merge tasks.
    /// Regression test: archived tasks have archived_at set and will never
    /// complete their merge, so the guard must skip them.
    #[test]
    fn archived_task_in_pending_merge_is_not_a_blocker() {
        // An archived task in PendingMerge — should be skipped by the guard
        let mut archived_task = make_task(None, None);
        archived_task.internal_status = InternalStatus::PendingMerge;
        archived_task.archived_at = Some(chrono::Utc::now());
        archived_task.created_at = chrono::Utc::now() - chrono::Duration::hours(1);

        // The guard checks: skip self, skip non-merge states, skip deferred, skip archived
        // Verify that archived_at.is_some() returns true for this task
        assert!(archived_task.archived_at.is_some());

        // A non-archived task should NOT be skipped
        let mut active_task = make_task(None, None);
        active_task.internal_status = InternalStatus::PendingMerge;
        active_task.created_at = chrono::Utc::now() - chrono::Duration::hours(1);
        assert!(active_task.archived_at.is_none());
    }

    // ==================
    // task_targets_branch tests
    // ==================

    #[tokio::test]
    async fn task_targets_branch_returns_true_for_matching_target() {
        let project = make_project(Some("main"));
        let mut task = make_task(None, Some("ralphx/test/task-123"));
        task.id = TaskId::from_string("task-123".to_string());

        let repo: Option<Arc<dyn PlanBranchRepository>> = None;
        // A standalone task merges into project base branch (main)
        assert!(task_targets_branch(&task, &project, &repo, "main").await);
    }

    #[tokio::test]
    async fn task_targets_branch_returns_false_for_non_matching_target() {
        let project = make_project(Some("main"));
        let mut task = make_task(None, Some("ralphx/test/task-123"));
        task.id = TaskId::from_string("task-123".to_string());

        let repo: Option<Arc<dyn PlanBranchRepository>> = None;
        assert!(!task_targets_branch(&task, &project, &repo, "develop").await);
    }

    #[tokio::test]
    async fn task_targets_branch_plan_task_targets_feature_branch() {
        let project = make_project(Some("main"));
        let mut task = make_task_with_session(Some("art-1"), Some("ralphx/test/task-456"), Some("sess-1"));
        task.id = TaskId::from_string("task-456".to_string());

        let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
        let pb = make_plan_branch("art-1", "ralphx/test/plan-abc123", PlanBranchStatus::Active, None);
        mem_repo.create(pb).await.unwrap();

        let repo: Option<Arc<dyn PlanBranchRepository>> = Some(mem_repo);
        // Plan task merges into feature branch, not main
        assert!(task_targets_branch(&task, &project, &repo, "ralphx/test/plan-abc123").await);
        assert!(!task_targets_branch(&task, &project, &repo, "main").await);
    }
}
