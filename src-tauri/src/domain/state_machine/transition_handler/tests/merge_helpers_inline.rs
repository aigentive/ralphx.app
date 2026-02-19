// Tests extracted from merge_helpers.rs #[cfg(test)] mod tests — part 1 of 2
//
// Covers: discover_and_attach_task_branch, main_merge_deferred metadata, conflict metadata

use super::super::merge_helpers::*;
use crate::domain::entities::{Project, ProjectId, Task};
use crate::domain::repositories::TaskRepository;
use crate::infrastructure::memory::MemoryTaskRepository;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
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
