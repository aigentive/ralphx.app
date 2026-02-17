// Merge helper utilities: path computation, metadata parsing, branch resolution
//
// Extracted from side_effects.rs — pure helpers with no side effects beyond metadata mutation.

use std::path::Path;
use std::sync::Arc;

use crate::application::GitService;
use crate::domain::entities::InternalStatus;
use crate::domain::entities::{PlanBranchStatus, Project, Task, TaskId};
use crate::domain::repositories::{PlanBranchRepository, TaskRepository};
use crate::error::AppResult;

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
            if !GitService::branch_exists(repo_path, &pb.branch_name) {
                match GitService::create_feature_branch(
                    repo_path,
                    &pb.branch_name,
                    &pb.source_branch,
                ) {
                    Ok(_) => {
                        tracing::info!(
                            branch = %pb.branch_name,
                            source = %pb.source_branch,
                            "Created deferred plan branch"
                        );
                    }
                    Err(e) => {
                        // Race condition: another task may have created it concurrently
                        if GitService::branch_exists(repo_path, &pb.branch_name) {
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
    if !GitService::branch_exists(repo_path, &branch_name) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{Project, ProjectId, Task};
    use crate::infrastructure::memory::MemoryTaskRepository;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
    use tempfile::TempDir;

    /// Create a temporary git repository for testing
    fn create_temp_git_repo() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_path_buf();

        // Initialize git repo
        Command::new("git")
            .args(["init"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        // Configure git
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        // Create initial commit
        fs::write(repo_path.join("README.md"), "test").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(&repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        (temp_dir, repo_path)
    }

    /// Create a test project with the given name and working directory
    fn create_test_project(name: &str, working_directory: String) -> Project {
        Project::new(name.to_string(), working_directory)
    }

    /// Create a test task for a project
    fn create_test_task(project_id: ProjectId) -> Task {
        Task::new(project_id, "Test task".to_string())
    }

    #[tokio::test]
    async fn test_discover_and_attach_branch_when_branch_exists() {
        let (_temp_dir, repo_path) = create_temp_git_repo();
        let project = create_test_project("test-project", repo_path.to_string_lossy().to_string());
        let mut task = create_test_task(project.id.clone());
        let task_repo: Arc<dyn TaskRepository> = Arc::new(MemoryTaskRepository::new());

        // Create the task in the repository
        task_repo.create(task.clone()).await.unwrap();

        // Create the expected branch
        let expected_branch = format!(
            "ralphx/{}/task-{}",
            slugify(&project.name),
            task.id.as_str()
        );
        Command::new("git")
            .args(["branch", &expected_branch])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        // Call discover_and_attach_task_branch
        let result = discover_and_attach_task_branch(&mut task, &project, &task_repo).await;

        // Should succeed and return true (branch was discovered and attached)
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Task should now have task_branch set
        assert_eq!(task.task_branch, Some(expected_branch.clone()));

        // Verify task was persisted with updated branch
        let saved_task = task_repo.get_by_id(&task.id).await.unwrap().unwrap();
        assert_eq!(saved_task.task_branch, Some(expected_branch));
    }

    #[tokio::test]
    async fn test_discover_and_attach_branch_when_branch_missing() {
        let (_temp_dir, repo_path) = create_temp_git_repo();
        let project = create_test_project("test-project", repo_path.to_string_lossy().to_string());
        let mut task = create_test_task(project.id.clone());
        let task_repo: Arc<dyn TaskRepository> = Arc::new(MemoryTaskRepository::new());

        // Create the task in the repository
        task_repo.create(task.clone()).await.unwrap();

        // Do NOT create the branch - it should not exist

        // Call discover_and_attach_task_branch
        let result = discover_and_attach_task_branch(&mut task, &project, &task_repo).await;

        // Should succeed but return false (branch was not found)
        assert!(result.is_ok());
        assert!(!result.unwrap());

        // Task should still have task_branch as None
        assert_eq!(task.task_branch, None);

        // Verify task was NOT updated in repository
        let saved_task = task_repo.get_by_id(&task.id).await.unwrap().unwrap();
        assert_eq!(saved_task.task_branch, None);
    }

    #[tokio::test]
    async fn test_discover_and_attach_branch_when_task_branch_already_set() {
        let (_temp_dir, repo_path) = create_temp_git_repo();
        let project = create_test_project("test-project", repo_path.to_string_lossy().to_string());
        let mut task = create_test_task(project.id.clone());
        let task_repo: Arc<dyn TaskRepository> = Arc::new(MemoryTaskRepository::new());

        // Set task_branch to an existing value
        task.task_branch = Some("existing-branch".to_string());
        task_repo.create(task.clone()).await.unwrap();

        // Create a different branch (should be ignored)
        let expected_branch = format!(
            "ralphx/{}/task-{}",
            slugify(&project.name),
            task.id.as_str()
        );
        Command::new("git")
            .args(["branch", &expected_branch])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        // Call discover_and_attach_task_branch
        let result = discover_and_attach_task_branch(&mut task, &project, &task_repo).await;

        // Should succeed but return false (early return, no-op)
        assert!(result.is_ok());
        assert!(!result.unwrap());

        // Task should still have the original branch
        assert_eq!(task.task_branch, Some("existing-branch".to_string()));

        // Verify task was NOT updated in repository
        let saved_task = task_repo.get_by_id(&task.id).await.unwrap().unwrap();
        assert_eq!(saved_task.task_branch, Some("existing-branch".to_string()));
    }

    #[tokio::test]
    async fn test_discover_and_attach_branch_slugifies_project_name() {
        let (_temp_dir, repo_path) = create_temp_git_repo();
        let project = create_test_project(
            "Test Project With Spaces!",
            repo_path.to_string_lossy().to_string(),
        );
        let mut task = create_test_task(project.id.clone());
        let task_repo: Arc<dyn TaskRepository> = Arc::new(MemoryTaskRepository::new());

        // Create the task in the repository
        task_repo.create(task.clone()).await.unwrap();

        // Create the branch with slugified name
        let expected_branch = format!(
            "ralphx/{}/task-{}",
            slugify(&project.name),
            task.id.as_str()
        );
        assert_eq!(
            expected_branch,
            format!("ralphx/test-project-with-spaces/task-{}", task.id.as_str())
        );

        Command::new("git")
            .args(["branch", &expected_branch])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        // Call discover_and_attach_task_branch
        let result = discover_and_attach_task_branch(&mut task, &project, &task_repo).await;

        // Should succeed and return true
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Task should have the correctly slugified branch name
        assert_eq!(task.task_branch, Some(expected_branch));
    }

    // ===== Main Merge Deferred Metadata Tests =====

    #[test]
    fn test_has_main_merge_deferred_metadata_returns_false_when_no_metadata() {
        let project = Project::new("test".to_string(), "/tmp".to_string());
        let task = Task::new(project.id, "Test task".to_string());

        assert!(!has_main_merge_deferred_metadata(&task));
    }

    #[test]
    fn test_has_main_merge_deferred_metadata_returns_false_when_flag_missing() {
        let project = Project::new("test".to_string(), "/tmp".to_string());
        let mut task = Task::new(project.id, "Test task".to_string());
        task.metadata = Some(r#"{"other_field": true}"#.to_string());

        assert!(!has_main_merge_deferred_metadata(&task));
    }

    #[test]
    fn test_has_main_merge_deferred_metadata_returns_true_when_flag_set() {
        let project = Project::new("test".to_string(), "/tmp".to_string());
        let mut task = Task::new(project.id, "Test task".to_string());
        task.metadata = Some(r#"{"main_merge_deferred": true}"#.to_string());

        assert!(has_main_merge_deferred_metadata(&task));
    }

    #[test]
    fn test_has_main_merge_deferred_metadata_returns_false_when_flag_false() {
        let project = Project::new("test".to_string(), "/tmp".to_string());
        let mut task = Task::new(project.id, "Test task".to_string());
        task.metadata = Some(r#"{"main_merge_deferred": false}"#.to_string());

        assert!(!has_main_merge_deferred_metadata(&task));
    }

    #[test]
    fn test_set_main_merge_deferred_metadata_creates_metadata_if_missing() {
        let project = Project::new("test".to_string(), "/tmp".to_string());
        let mut task = Task::new(project.id, "Test task".to_string());
        assert!(task.metadata.is_none());

        set_main_merge_deferred_metadata(&mut task);

        assert!(task.metadata.is_some());
        let meta: serde_json::Value = serde_json::from_str(task.metadata.as_ref().unwrap()).unwrap();
        assert_eq!(meta["main_merge_deferred"], true);
        assert!(meta["main_merge_deferred_at"].is_string());
    }

    #[test]
    fn test_set_main_merge_deferred_metadata_preserves_existing_fields() {
        let project = Project::new("test".to_string(), "/tmp".to_string());
        let mut task = Task::new(project.id, "Test task".to_string());
        task.metadata = Some(r#"{"existing_field": "value"}"#.to_string());

        set_main_merge_deferred_metadata(&mut task);

        let meta: serde_json::Value = serde_json::from_str(task.metadata.as_ref().unwrap()).unwrap();
        assert_eq!(meta["existing_field"], "value");
        assert_eq!(meta["main_merge_deferred"], true);
        assert!(meta["main_merge_deferred_at"].is_string());
    }

    #[test]
    fn test_set_main_merge_deferred_metadata_overwrites_existing_flag() {
        let project = Project::new("test".to_string(), "/tmp".to_string());
        let mut task = Task::new(project.id, "Test task".to_string());
        task.metadata = Some(r#"{"main_merge_deferred": false, "main_merge_deferred_at": "old-time"}"#.to_string());

        set_main_merge_deferred_metadata(&mut task);

        let meta: serde_json::Value = serde_json::from_str(task.metadata.as_ref().unwrap()).unwrap();
        assert_eq!(meta["main_merge_deferred"], true);
        // Timestamp should be updated
        assert_ne!(meta["main_merge_deferred_at"], "old-time");
    }

    #[test]
    fn test_clear_main_merge_deferred_metadata_preserves_other_fields() {
        let project = Project::new("test".to_string(), "/tmp".to_string());
        let mut task = Task::new(project.id, "Test task".to_string());
        task.metadata = Some(
            r#"{"main_merge_deferred": true, "main_merge_deferred_at": "2026-02-15T00:00:00Z", "other_field": "value"}"#.to_string(),
        );

        clear_main_merge_deferred_metadata(&mut task);

        let meta: serde_json::Value = serde_json::from_str(task.metadata.as_ref().unwrap()).unwrap();
        assert_eq!(meta["other_field"], "value");
        assert!(meta.get("main_merge_deferred").is_none());
        assert!(meta.get("main_merge_deferred_at").is_none());
    }

    #[test]
    fn test_clear_main_merge_deferred_metadata_clears_metadata_if_empty() {
        let project = Project::new("test".to_string(), "/tmp".to_string());
        let mut task = Task::new(project.id, "Test task".to_string());
        task.metadata = Some(r#"{"main_merge_deferred": true, "main_merge_deferred_at": "2026-02-15T00:00:00Z"}"#.to_string());

        clear_main_merge_deferred_metadata(&mut task);

        // Metadata should be cleared entirely when only main_merge_deferred fields were present
        assert!(task.metadata.is_none());
    }

    #[test]
    fn test_clear_main_merge_deferred_metadata_noop_when_no_metadata() {
        let project = Project::new("test".to_string(), "/tmp".to_string());
        let mut task = Task::new(project.id, "Test task".to_string());
        assert!(task.metadata.is_none());

        clear_main_merge_deferred_metadata(&mut task);

        // Should remain None without error
        assert!(task.metadata.is_none());
    }

    #[test]
    fn test_set_and_clear_main_merge_deferred_roundtrip() {
        let project = Project::new("test".to_string(), "/tmp".to_string());
        let mut task = Task::new(project.id, "Test task".to_string());

        // Set the flag
        set_main_merge_deferred_metadata(&mut task);
        assert!(has_main_merge_deferred_metadata(&task));

        // Clear the flag
        clear_main_merge_deferred_metadata(&mut task);
        assert!(!has_main_merge_deferred_metadata(&task));

        // Metadata should be None after clearing (only had main_merge_deferred fields)
        assert!(task.metadata.is_none());
    }

    // ===== Conflict Metadata Tests =====

    #[test]
    fn test_set_conflict_metadata_creates_metadata_if_missing() {
        let project = Project::new("test".to_string(), "/tmp".to_string());
        let mut task = Task::new(project.id, "Test task".to_string());
        assert!(task.metadata.is_none());

        let conflict_files = vec![
            "src/main.rs".to_string(),
            "src/lib.rs".to_string(),
        ];
        set_conflict_metadata(&mut task, &conflict_files, "programmatic");

        assert!(task.metadata.is_some());
        let meta: serde_json::Value = serde_json::from_str(task.metadata.as_ref().unwrap()).unwrap();
        assert_eq!(
            meta["conflict_files"],
            serde_json::json!(["src/main.rs", "src/lib.rs"])
        );
        assert!(meta["conflict_snapshot_at"].is_string());
        assert_eq!(meta["conflict_detected_by"], "programmatic");
    }

    #[test]
    fn test_set_conflict_metadata_preserves_existing_fields() {
        let project = Project::new("test".to_string(), "/tmp".to_string());
        let mut task = Task::new(project.id, "Test task".to_string());
        task.metadata = Some(r#"{"existing_field": "value"}"#.to_string());

        let conflict_files = vec!["src/conflict.rs".to_string()];
        set_conflict_metadata(&mut task, &conflict_files, "agent");

        let meta: serde_json::Value = serde_json::from_str(task.metadata.as_ref().unwrap()).unwrap();
        assert_eq!(meta["existing_field"], "value");
        assert_eq!(meta["conflict_files"], serde_json::json!(["src/conflict.rs"]));
        assert_eq!(meta["conflict_detected_by"], "agent");
    }

    #[test]
    fn test_set_conflict_metadata_overwrites_existing_conflict_files() {
        let project = Project::new("test".to_string(), "/tmp".to_string());
        let mut task = Task::new(project.id, "Test task".to_string());
        task.metadata = Some(
            r#"{"conflict_files": ["old_file.rs"], "conflict_snapshot_at": "2026-02-15T00:00:00Z"}"#
                .to_string(),
        );

        let conflict_files = vec!["new_file.rs".to_string()];
        set_conflict_metadata(&mut task, &conflict_files, "programmatic");

        let meta: serde_json::Value = serde_json::from_str(task.metadata.as_ref().unwrap()).unwrap();
        assert_eq!(meta["conflict_files"], serde_json::json!(["new_file.rs"]));
        // Timestamp should be updated
        assert_ne!(meta["conflict_snapshot_at"], "2026-02-15T00:00:00Z");
    }

    #[test]
    fn test_set_conflict_metadata_with_agent_source() {
        let project = Project::new("test".to_string(), "/tmp".to_string());
        let mut task = Task::new(project.id, "Test task".to_string());

        let conflict_files = vec!["path/to/file.ts".to_string()];
        set_conflict_metadata(&mut task, &conflict_files, "agent");

        let meta: serde_json::Value = serde_json::from_str(task.metadata.as_ref().unwrap()).unwrap();
        assert_eq!(meta["conflict_detected_by"], "agent");
    }

    #[test]
    fn test_set_conflict_metadata_with_programmatic_source() {
        let project = Project::new("test".to_string(), "/tmp".to_string());
        let mut task = Task::new(project.id, "Test task".to_string());

        let conflict_files: Vec<String> = vec![];
        set_conflict_metadata(&mut task, &conflict_files, "programmatic");

        let meta: serde_json::Value = serde_json::from_str(task.metadata.as_ref().unwrap()).unwrap();
        assert_eq!(meta["conflict_detected_by"], "programmatic");
    }
}
