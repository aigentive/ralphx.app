//! Git HTTP handlers for merge operations
//!
//! Provides HTTP endpoints for the merger agent to signal merge completion or conflicts.
//! Also provides query endpoints for commits and diff statistics.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Emitter;

use super::*;
use crate::application::{GitService, TaskSchedulerService, TaskTransitionService};
use crate::domain::entities::{InternalStatus, TaskId};
use crate::domain::state_machine::resolve_merge_branches;
use crate::domain::state_machine::services::TaskScheduler;

// ============================================================================
// Request/Response Types
// ============================================================================

/// Request to complete a merge after agent resolution
#[derive(Debug, serde::Deserialize)]
pub struct CompleteMergeRequest {
    /// SHA of the merge commit
    pub commit_sha: String,
}

/// Request to report unresolvable conflict
#[derive(Debug, serde::Deserialize)]
pub struct ReportConflictRequest {
    /// List of files with unresolved conflicts
    pub conflict_files: Vec<String>,
}

/// Request to report a non-conflict merge failure
#[derive(Debug, serde::Deserialize)]
pub struct ReportIncompleteRequest {
    /// Detailed explanation of why merge failed
    pub reason: String,
    /// Optional diagnostic info (git status, logs, etc.)
    pub diagnostic_info: Option<String>,
}

/// Response for merge operations
#[derive(Debug, serde::Serialize)]
pub struct MergeOperationResponse {
    pub success: bool,
    pub message: String,
    pub new_status: String,
}

/// Commit information for get_task_commits
#[derive(Debug, serde::Serialize)]
pub struct CommitInfoResponse {
    pub sha: String,
    pub short_sha: String,
    pub message: String,
    pub author: String,
    pub timestamp: String,
}

/// Diff statistics for get_task_diff_stats
#[derive(Debug, serde::Serialize)]
pub struct DiffStatsResponse {
    pub files_changed: u32,
    pub insertions: u32,
    pub deletions: u32,
    pub changed_files: Vec<String>,
}

/// Merge target branches for get_merge_target
#[derive(Debug, serde::Serialize)]
pub struct MergeTargetResponse {
    pub source_branch: String,
    pub target_branch: String,
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /api/git/tasks/{id}/complete-merge
///
/// Called by merger agent when conflicts have been successfully resolved.
/// Transitions task from Merging → Merged and triggers cleanup.
pub async fn complete_merge(
    State(state): State<HttpServerState>,
    Path(task_id): Path<String>,
    Json(req): Json<CompleteMergeRequest>,
) -> Result<Json<MergeOperationResponse>, JsonError> {
    // 1. Validate SHA format (must be 40 hex characters)
    if !is_valid_git_sha(&req.commit_sha) {
        return Err(json_error(
            StatusCode::BAD_REQUEST,
            "commit_sha must be a full 40-character SHA (use `git rev-parse HEAD`)",
            None,
        ));
    }

    let task_id = TaskId::from_string(task_id);

    // 2. Get task
    let mut task = state
        .app_state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string(), None))?
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "Task not found", None))?;

    // 3. Idempotent: if already merged, return success
    if task.internal_status == InternalStatus::Merged {
        return Ok(Json(MergeOperationResponse {
            success: true,
            message: "Merge already completed".to_string(),
            new_status: "already_merged".to_string(),
        }));
    }

    // 4. Validate state is Merging
    if task.internal_status != InternalStatus::Merging {
        return Err(json_error(
            StatusCode::BAD_REQUEST,
            format!(
                "Task must be in 'merging' status to complete merge. Current status: {}",
                task.internal_status.as_str()
            ),
            None,
        ));
    }

    // 5. Get project for cleanup info and verification
    let project = state
        .app_state
        .project_repo
        .get_by_id(&task.project_id)
        .await
        .map_err(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string(), None))?
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "Project not found", None))?;

    // 6. Verify commit is on target branch (resolved via plan branch or base branch)
    let plan_branch_repo = Some(Arc::clone(&state.app_state.plan_branch_repo));
    let (_, target_branch) =
        resolve_merge_branches(&task, &project, &plan_branch_repo).await;
    let repo_path = PathBuf::from(&project.working_directory);

    if !GitService::is_commit_on_branch(&repo_path, &req.commit_sha, &target_branch)
        .map_err(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string(), None))?
    {
        return Err(json_error(
            StatusCode::BAD_REQUEST,
            format!(
                "Commit {} is not on {} branch. The merge may not have completed successfully.",
                req.commit_sha, target_branch
            ),
            Some(format!(
                "Ensure you merged the task branch INTO {} and obtained the SHA from {} (git rev-parse HEAD on {})",
                target_branch, target_branch, target_branch
            )),
        ));
    }

    // 7. Set merge_commit_sha on task
    task.merge_commit_sha = Some(req.commit_sha.clone());
    state
        .app_state
        .task_repo
        .update(&task)
        .await
        .map_err(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string(), None))?;

    // 8. Persist merge commit SHA before transitioning status
    if task.merge_commit_sha.as_deref() != Some(&req.commit_sha) {
        let mut updated_task = task.clone();
        updated_task.merge_commit_sha = Some(req.commit_sha.clone());
        updated_task.touch();
        state
            .app_state
            .task_repo
            .update(&updated_task)
            .await
            .map_err(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string(), None))?;
    }

    // 9. Create transition service and transition to Merged
    let task_scheduler: Arc<dyn TaskScheduler> = Arc::new(TaskSchedulerService::new(
        Arc::clone(&state.execution_state),
        Arc::clone(&state.app_state.project_repo),
        Arc::clone(&state.app_state.task_repo),
        Arc::clone(&state.app_state.task_dependency_repo),
        Arc::clone(&state.app_state.chat_message_repo),
        Arc::clone(&state.app_state.chat_conversation_repo),
        Arc::clone(&state.app_state.agent_run_repo),
        Arc::clone(&state.app_state.ideation_session_repo),
        Arc::clone(&state.app_state.activity_event_repo),
        Arc::clone(&state.app_state.message_queue),
        Arc::clone(&state.app_state.running_agent_registry),
        state.app_state.app_handle.as_ref().cloned(),
    ));

    let transition_service = TaskTransitionService::new(
        Arc::clone(&state.app_state.task_repo),
        Arc::clone(&state.app_state.task_dependency_repo),
        Arc::clone(&state.app_state.project_repo),
        Arc::clone(&state.app_state.chat_message_repo),
        Arc::clone(&state.app_state.chat_conversation_repo),
        Arc::clone(&state.app_state.agent_run_repo),
        Arc::clone(&state.app_state.ideation_session_repo),
        Arc::clone(&state.app_state.activity_event_repo),
        Arc::clone(&state.app_state.message_queue),
        Arc::clone(&state.app_state.running_agent_registry),
        Arc::clone(&state.execution_state),
        state.app_state.app_handle.as_ref().cloned(),
    )
    .with_task_scheduler(task_scheduler)
    .with_plan_branch_repo(Arc::clone(&state.app_state.plan_branch_repo));

    transition_service
        .transition_task(&task_id, InternalStatus::Merged)
        .await
        .map_err(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string(), None))?;

    // 10. Cleanup branch/worktree
    if let Some(task_branch) = &task.task_branch {
        // repo_path already defined in step 6

        // Delete worktree if exists (Worktree mode)
        if let Some(worktree_path) = &task.worktree_path {
            let _ = GitService::delete_worktree(&repo_path, &PathBuf::from(worktree_path));
        }

        // Delete branch (both modes)
        let _ = GitService::delete_branch(&repo_path, task_branch, true);
    }

    // 11. Emit events
    if let Some(app_handle) = &state.app_state.app_handle {
        let _ = app_handle.emit(
            "merge:completed",
            serde_json::json!({
                "task_id": task_id.as_str(),
                "commit_sha": req.commit_sha,
            }),
        );
        let _ = app_handle.emit(
            "task:status_changed",
            serde_json::json!({
                "task_id": task_id.as_str(),
                "old_status": "merging",
                "new_status": "merged",
            }),
        );
    }

    Ok(Json(MergeOperationResponse {
        success: true,
        message: "Merge completed successfully".to_string(),
        new_status: "merged".to_string(),
    }))
}

/// POST /api/git/tasks/{id}/report-conflict
///
/// Called by merger agent when it cannot resolve conflicts.
/// Transitions task from Merging → MergeConflict for manual resolution.
pub async fn report_conflict(
    State(state): State<HttpServerState>,
    Path(task_id): Path<String>,
    Json(req): Json<ReportConflictRequest>,
) -> Result<Json<MergeOperationResponse>, JsonError> {
    let task_id = TaskId::from_string(task_id);

    // 1. Get task and validate state is Merging
    let task = state
        .app_state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string(), None))?
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "Task not found", None))?;

    if task.internal_status != InternalStatus::Merging {
        return Err(json_error(
            StatusCode::BAD_REQUEST,
            format!(
                "Task must be in 'merging' status to report conflict. Current status: {}",
                task.internal_status.as_str()
            ),
            None,
        ));
    }

    // 2. Transition to MergeConflict
    // Note: Conflict files are passed via the event, not stored on task
    // The UI will display them from the merge:conflict event payload
    let transition_service = TaskTransitionService::new(
        Arc::clone(&state.app_state.task_repo),
        Arc::clone(&state.app_state.task_dependency_repo),
        Arc::clone(&state.app_state.project_repo),
        Arc::clone(&state.app_state.chat_message_repo),
        Arc::clone(&state.app_state.chat_conversation_repo),
        Arc::clone(&state.app_state.agent_run_repo),
        Arc::clone(&state.app_state.ideation_session_repo),
        Arc::clone(&state.app_state.activity_event_repo),
        Arc::clone(&state.app_state.message_queue),
        Arc::clone(&state.app_state.running_agent_registry),
        Arc::clone(&state.execution_state),
        state.app_state.app_handle.as_ref().cloned(),
    )
    .with_plan_branch_repo(Arc::clone(&state.app_state.plan_branch_repo));

    transition_service
        .transition_task(&task_id, InternalStatus::MergeConflict)
        .await
        .map_err(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string(), None))?;

    // 4. Emit events
    if let Some(app_handle) = &state.app_state.app_handle {
        let _ = app_handle.emit(
            "merge:conflict",
            serde_json::json!({
                "task_id": task_id.as_str(),
                "conflict_files": req.conflict_files,
            }),
        );
        let _ = app_handle.emit(
            "task:status_changed",
            serde_json::json!({
                "task_id": task_id.as_str(),
                "old_status": "merging",
                "new_status": "merge_conflict",
            }),
        );
    }

    Ok(Json(MergeOperationResponse {
        success: true,
        message: "Conflict reported. Task requires manual resolution.".to_string(),
        new_status: "merge_conflict".to_string(),
    }))
}

/// POST /api/git/tasks/{id}/report-incomplete
///
/// Called by merger agent when merge cannot be completed due to non-conflict errors
/// (e.g., git operation failures, missing configuration).
/// Transitions task from Merging → MergeIncomplete.
pub async fn report_incomplete(
    State(state): State<HttpServerState>,
    Path(task_id): Path<String>,
    Json(req): Json<ReportIncompleteRequest>,
) -> Result<Json<MergeOperationResponse>, JsonError> {
    let task_id = TaskId::from_string(task_id);

    // 1. Get task and validate state is Merging
    let task = state
        .app_state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string(), None))?
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "Task not found", None))?;

    if task.internal_status != InternalStatus::Merging {
        return Err(json_error(
            StatusCode::BAD_REQUEST,
            format!(
                "Task must be in 'merging' status to report incomplete. Current status: {}",
                task.internal_status.as_str()
            ),
            None,
        ));
    }

    // 2. Transition to MergeIncomplete
    let transition_service = TaskTransitionService::new(
        Arc::clone(&state.app_state.task_repo),
        Arc::clone(&state.app_state.task_dependency_repo),
        Arc::clone(&state.app_state.project_repo),
        Arc::clone(&state.app_state.chat_message_repo),
        Arc::clone(&state.app_state.chat_conversation_repo),
        Arc::clone(&state.app_state.agent_run_repo),
        Arc::clone(&state.app_state.ideation_session_repo),
        Arc::clone(&state.app_state.activity_event_repo),
        Arc::clone(&state.app_state.message_queue),
        Arc::clone(&state.app_state.running_agent_registry),
        Arc::clone(&state.execution_state),
        state.app_state.app_handle.as_ref().cloned(),
    )
    .with_plan_branch_repo(Arc::clone(&state.app_state.plan_branch_repo));

    transition_service
        .transition_task(&task_id, InternalStatus::MergeIncomplete)
        .await
        .map_err(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string(), None))?;

    // 3. Emit events
    if let Some(app_handle) = &state.app_state.app_handle {
        let _ = app_handle.emit(
            "merge:incomplete",
            serde_json::json!({
                "task_id": task_id.as_str(),
                "reason": req.reason,
                "diagnostic_info": req.diagnostic_info,
            }),
        );
        let _ = app_handle.emit(
            "task:status_changed",
            serde_json::json!({
                "task_id": task_id.as_str(),
                "old_status": "merging",
                "new_status": "merge_incomplete",
            }),
        );
    }

    Ok(Json(MergeOperationResponse {
        success: true,
        message: "Merge incomplete reported. Task marked for investigation.".to_string(),
        new_status: "merge_incomplete".to_string(),
    }))
}

/// GET /api/git/tasks/{id}/commits
///
/// Get commits on the task branch since it diverged from base.
pub async fn get_task_commits(
    State(state): State<HttpServerState>,
    Path(task_id): Path<String>,
) -> Result<Json<Vec<CommitInfoResponse>>, (StatusCode, String)> {
    let task_id = TaskId::from_string(task_id);

    // 1. Get task
    let task = state
        .app_state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Task not found".to_string()))?;

    // 2. Check task has a branch (verify exists, don't need the value)
    task.task_branch.as_ref().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "Task does not have a git branch".to_string(),
        )
    })?;

    // 3. Get project for base branch and working directory
    let project = state
        .app_state
        .project_repo
        .get_by_id(&task.project_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Project not found".to_string()))?;

    let base_branch = project.base_branch.as_deref().unwrap_or("main");

    // 4. Determine working path (worktree or main repo)
    let working_path = task
        .worktree_path
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(&project.working_directory));

    // 5. Get commits from GitService
    let commits = GitService::get_commits_since(&working_path, base_branch)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 6. Convert to response format
    let response: Vec<CommitInfoResponse> = commits
        .into_iter()
        .map(|c| CommitInfoResponse {
            sha: c.sha,
            short_sha: c.short_sha,
            message: c.message,
            author: c.author,
            timestamp: c.timestamp,
        })
        .collect();

    Ok(Json(response))
}

/// GET /api/git/tasks/{id}/diff-stats
///
/// Get diff statistics for the task branch compared to base.
pub async fn get_task_diff_stats(
    State(state): State<HttpServerState>,
    Path(task_id): Path<String>,
) -> Result<Json<DiffStatsResponse>, (StatusCode, String)> {
    let task_id = TaskId::from_string(task_id);

    // 1. Get task
    let task = state
        .app_state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Task not found".to_string()))?;

    // 2. Check task has a branch (we need to verify it exists, but don't need to use it)
    task.task_branch.as_ref().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "Task does not have a git branch".to_string(),
        )
    })?;

    // 3. Get project for base branch and working directory
    let project = state
        .app_state
        .project_repo
        .get_by_id(&task.project_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Project not found".to_string()))?;

    let base_branch = project.base_branch.as_deref().unwrap_or("main");

    // 4. Determine working path (worktree or main repo)
    let working_path = task
        .worktree_path
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(&project.working_directory));

    // 5. Get diff stats from GitService
    let stats = GitService::get_diff_stats(&working_path, base_branch)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(DiffStatsResponse {
        files_changed: stats.files_changed,
        insertions: stats.insertions,
        deletions: stats.deletions,
        changed_files: stats.changed_files,
    }))
}

/// GET /api/git/tasks/{id}/merge-target
///
/// Get the resolved merge target branches for a task.
/// Returns source_branch (task's branch) and target_branch (where to merge INTO).
/// The target may be a plan feature branch instead of main.
pub async fn get_merge_target(
    State(state): State<HttpServerState>,
    Path(task_id): Path<String>,
) -> Result<Json<MergeTargetResponse>, (StatusCode, String)> {
    let task_id = TaskId::from_string(task_id);

    // 1. Get task
    let task = state
        .app_state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Task not found".to_string()))?;

    // 2. Get project
    let project = state
        .app_state
        .project_repo
        .get_by_id(&task.project_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Project not found".to_string()))?;

    // 3. Resolve merge branches
    let plan_branch_repo = Some(Arc::clone(&state.app_state.plan_branch_repo));
    let (source_branch, target_branch) =
        resolve_merge_branches(&task, &project, &plan_branch_repo).await;

    Ok(Json(MergeTargetResponse {
        source_branch,
        target_branch,
    }))
}

// ============================================================================
// Helpers
// ============================================================================

/// JSON error response type for git handlers
pub type JsonError = (StatusCode, Json<serde_json::Value>);

/// Create a JSON error response with an error message and optional details
fn json_error(status: StatusCode, error: impl Into<String>, details: Option<String>) -> JsonError {
    let mut body = serde_json::json!({ "error": error.into() });
    if let Some(d) = details {
        body["details"] = serde_json::Value::String(d);
    }
    (status, Json(body))
}

/// Validates that a string is a valid full git SHA (40 hexadecimal characters)
fn is_valid_git_sha(sha: &str) -> bool {
    sha.len() == 40 && sha.chars().all(|c| c.is_ascii_hexdigit())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod json_error_format {
        use super::*;

        #[test]
        fn error_without_details() {
            let (status, Json(body)) =
                json_error(StatusCode::BAD_REQUEST, "Invalid input", None);
            assert_eq!(status, StatusCode::BAD_REQUEST);
            assert_eq!(body["error"], "Invalid input");
            assert!(body.get("details").is_none());
        }

        #[test]
        fn error_with_details() {
            let (status, Json(body)) = json_error(
                StatusCode::BAD_REQUEST,
                "Commit not on branch",
                Some("Use git rev-parse HEAD on main".to_string()),
            );
            assert_eq!(status, StatusCode::BAD_REQUEST);
            assert_eq!(body["error"], "Commit not on branch");
            assert_eq!(body["details"], "Use git rev-parse HEAD on main");
        }

        #[test]
        fn internal_server_error_status() {
            let (status, Json(body)) =
                json_error(StatusCode::INTERNAL_SERVER_ERROR, "Database error", None);
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(body["error"], "Database error");
        }

        #[test]
        fn not_found_error_status() {
            let (status, Json(body)) =
                json_error(StatusCode::NOT_FOUND, "Task not found", None);
            assert_eq!(status, StatusCode::NOT_FOUND);
            assert_eq!(body["error"], "Task not found");
        }
    }

    mod sha_validation {
        use super::*;

        #[test]
        fn valid_sha_40_lowercase_hex() {
            let sha = "a1b2c3d4e5f6789012345678901234567890abcd";
            assert!(is_valid_git_sha(sha));
        }

        #[test]
        fn valid_sha_40_uppercase_hex() {
            let sha = "A1B2C3D4E5F6789012345678901234567890ABCD";
            assert!(is_valid_git_sha(sha));
        }

        #[test]
        fn valid_sha_mixed_case() {
            let sha = "a1B2c3D4e5F6789012345678901234567890AbCd";
            assert!(is_valid_git_sha(sha));
        }

        #[test]
        fn valid_sha_all_digits() {
            let sha = "1234567890123456789012345678901234567890";
            assert!(is_valid_git_sha(sha));
        }

        #[test]
        fn invalid_sha_too_short() {
            let sha = "a1b2c3d4";
            assert!(!is_valid_git_sha(sha));
        }

        #[test]
        fn invalid_sha_too_long() {
            let sha = "a1b2c3d4e5f6789012345678901234567890abcd1234";
            assert!(!is_valid_git_sha(sha));
        }

        #[test]
        fn invalid_sha_non_hex_chars() {
            let sha = "g1b2c3d4e5f6789012345678901234567890abcd"; // 'g' is not hex
            assert!(!is_valid_git_sha(sha));
        }

        #[test]
        fn invalid_sha_empty() {
            let sha = "";
            assert!(!is_valid_git_sha(sha));
        }

        #[test]
        fn invalid_sha_spaces() {
            let sha = "a1b2c3d4e5f67890 2345678901234567890abcd";
            assert!(!is_valid_git_sha(sha));
        }

        #[test]
        fn invalid_sha_short_sha_format() {
            // Short SHA (7 chars) should be rejected
            let sha = "a1b2c3d";
            assert!(!is_valid_git_sha(sha));
        }
    }
}
