use super::*;
use crate::infrastructure::memory::MemoryProjectRepository;
use crate::infrastructure::memory::MemoryTaskRepository;
use std::sync::Arc;

fn setup_test_state() -> AppState {
    let task_repo = Arc::new(MemoryTaskRepository::new());
    let project_repo = Arc::new(MemoryProjectRepository::new());
    AppState::with_repos(task_repo, project_repo)
}

#[tokio::test]
async fn test_create_project_with_defaults() {
    let state = setup_test_state();

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    let created = state.project_repo.create(project).await.unwrap();

    assert_eq!(created.name, "Test Project");
    assert_eq!(created.working_directory, "/test/path");
    assert_eq!(created.git_mode, GitMode::Worktree);
}

#[tokio::test]
async fn test_create_project_with_worktree_mode() {
    let state = setup_test_state();

    let mut project = Project::new("Worktree Project".to_string(), "/main/repo".to_string());
    project.git_mode = GitMode::Worktree;
    project.base_branch = Some("main".to_string());
    let created = state.project_repo.create(project).await.unwrap();

    assert_eq!(created.name, "Worktree Project");
    assert_eq!(created.git_mode, GitMode::Worktree);
    assert_eq!(created.base_branch, Some("main".to_string()));
}

#[tokio::test]
async fn test_get_project_returns_none_for_nonexistent() {
    let state = setup_test_state();
    let id = ProjectId::new();

    let result = state.project_repo.get_by_id(&id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_update_project_modifies_fields() {
    let state = setup_test_state();

    let project = Project::new("Original Name".to_string(), "/original/path".to_string());
    let created = state.project_repo.create(project).await.unwrap();

    let mut updated = created.clone();
    updated.name = "Updated Name".to_string();
    updated.working_directory = "/updated/path".to_string();

    state.project_repo.update(&updated).await.unwrap();

    let found = state
        .project_repo
        .get_by_id(&created.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.name, "Updated Name");
    assert_eq!(found.working_directory, "/updated/path");
}

#[tokio::test]
async fn test_delete_project_removes_it() {
    let state = setup_test_state();

    let project = Project::new("To Delete".to_string(), "/delete/me".to_string());
    let created = state.project_repo.create(project).await.unwrap();

    state.project_repo.delete(&created.id).await.unwrap();

    let found = state.project_repo.get_by_id(&created.id).await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn test_list_projects_returns_all() {
    let state = setup_test_state();

    state
        .project_repo
        .create(Project::new("Project 1".to_string(), "/path/1".to_string()))
        .await
        .unwrap();
    state
        .project_repo
        .create(Project::new("Project 2".to_string(), "/path/2".to_string()))
        .await
        .unwrap();
    state
        .project_repo
        .create(Project::new("Project 3".to_string(), "/path/3".to_string()))
        .await
        .unwrap();

    let projects = state.project_repo.get_all().await.unwrap();
    assert_eq!(projects.len(), 3);
}

#[tokio::test]
async fn test_project_response_serialization() {
    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    let response = ProjectResponse::from(project);

    assert!(!response.id.is_empty());
    assert_eq!(response.name, "Test Project");
    assert_eq!(response.working_directory, "/test/path");
    assert_eq!(response.git_mode, "worktree");

    // Verify it serializes to JSON with snake_case (Rust default)
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"name\":\"Test Project\""));
    assert!(json.contains("\"working_directory\":\"/test/path\""));
    assert!(json.contains("\"git_mode\":\"worktree\""));
}

// ===== get_git_default_branch tests =====

/// Helper to create a temp dir with git initialized
fn create_git_repo() -> tempfile::TempDir {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .expect("Failed to init git repo");

    // Configure git user for commits
    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(path)
        .output()
        .expect("Failed to set git email");

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(path)
        .output()
        .expect("Failed to set git name");

    temp_dir
}

/// Helper to create an initial commit on a branch
fn create_commit_on_branch(path: &std::path::Path, branch_name: &str) {
    // Create and checkout branch
    std::process::Command::new("git")
        .args(["checkout", "-b", branch_name])
        .current_dir(path)
        .output()
        .expect("Failed to create branch");

    // Create a file and commit
    std::fs::write(path.join("README.md"), "# Test").unwrap();
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output()
        .expect("Failed to stage files");

    std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(path)
        .output()
        .expect("Failed to commit");
}

#[tokio::test]
async fn test_get_git_default_branch_nonexistent_directory() {
    let result = get_git_default_branch("/nonexistent/path/that/does/not/exist".to_string()).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("does not exist"));
}

#[tokio::test]
async fn test_get_git_default_branch_not_a_git_repo() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().to_str().unwrap().to_string();

    let result = get_git_default_branch(path).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Not a git repository"));
}

#[tokio::test]
async fn test_get_git_default_branch_empty_repo_no_branches() {
    let temp_dir = create_git_repo();
    let path = temp_dir.path().to_str().unwrap().to_string();

    // Empty repo with no commits = no branches
    let result = get_git_default_branch(path).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No branches found"));
}

#[tokio::test]
async fn test_get_git_default_branch_returns_main() {
    let temp_dir = create_git_repo();
    let path = temp_dir.path();

    // Create main branch with a commit
    create_commit_on_branch(path, "main");

    let result = get_git_default_branch(path.to_str().unwrap().to_string()).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "main");
}

#[tokio::test]
async fn test_get_git_default_branch_returns_master() {
    let temp_dir = create_git_repo();
    let path = temp_dir.path();

    // Create master branch with a commit (not main)
    create_commit_on_branch(path, "master");

    let result = get_git_default_branch(path.to_str().unwrap().to_string()).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "master");
}

#[tokio::test]
async fn test_get_git_default_branch_prefers_main_over_master() {
    let temp_dir = create_git_repo();
    let path = temp_dir.path();

    // Create main branch first
    create_commit_on_branch(path, "main");

    // Create master branch
    std::process::Command::new("git")
        .args(["checkout", "-b", "master"])
        .current_dir(path)
        .output()
        .expect("Failed to create master branch");

    let result = get_git_default_branch(path.to_str().unwrap().to_string()).await;
    assert!(result.is_ok());
    // Should prefer main (checked before master in fallback chain)
    assert_eq!(result.unwrap(), "main");
}

#[tokio::test]
async fn test_get_git_default_branch_falls_back_to_first_branch() {
    let temp_dir = create_git_repo();
    let path = temp_dir.path();

    // Create a branch that's NOT main or master
    create_commit_on_branch(path, "develop");

    let result = get_git_default_branch(path.to_str().unwrap().to_string()).await;
    assert!(result.is_ok());
    // Should fall back to the only branch available
    assert_eq!(result.unwrap(), "develop");
}

#[tokio::test]
async fn test_get_git_default_branch_first_branch_alphabetically() {
    let temp_dir = create_git_repo();
    let path = temp_dir.path();

    // Create feature-z branch first
    create_commit_on_branch(path, "feature-z");

    // Create feature-a branch
    std::process::Command::new("git")
        .args(["checkout", "-b", "feature-a"])
        .current_dir(path)
        .output()
        .expect("Failed to create feature-a branch");

    let result = get_git_default_branch(path.to_str().unwrap().to_string()).await;
    assert!(result.is_ok());
    // The function gets first line from `git branch --format=%(refname:short)`
    // which lists branches alphabetically, so feature-a comes first
    assert_eq!(result.unwrap(), "feature-a");
}

// ── IPC contract tests ─────────────────────────────────────────────────────────
// Verify camelCase deserialization for project command input structs.

#[cfg(test)]
mod ipc_contract {
    use super::super::{CreateProjectInput, UpdateProjectInput};

    // ── CreateProjectInput ──────────────────────────────────────────────────

    #[test]
    fn create_project_input_deserializes_camel_case() {
        let json = r#"{"name":"My Project","workingDirectory":"/code/my-project","gitMode":"worktree","baseBranch":"main"}"#;
        let input: CreateProjectInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, "My Project");
        assert_eq!(input.working_directory, "/code/my-project");
        assert_eq!(input.git_mode, Some("worktree".to_string()));
        assert_eq!(input.base_branch, Some("main".to_string()));
    }

    #[test]
    fn create_project_input_optional_fields_absent() {
        let json = r#"{"name":"Minimal","workingDirectory":"/tmp/proj"}"#;
        let input: CreateProjectInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, "Minimal");
        assert_eq!(input.working_directory, "/tmp/proj");
        assert!(input.git_mode.is_none());
        assert!(input.base_branch.is_none());
    }

    #[test]
    fn create_project_input_rejects_snake_case() {
        // snake_case keys must NOT be accepted when camelCase is required
        let json = r#"{"name":"Bad","working_directory":"/bad"}"#;
        let result: Result<CreateProjectInput, _> = serde_json::from_str(json);
        // working_directory won't map to working_directory field due to rename_all camelCase
        // The struct will deserialize but working_directory field will be missing → error
        assert!(
            result.is_err(),
            "snake_case working_directory must not deserialize (missing required field)"
        );
    }

    // ── UpdateProjectInput ──────────────────────────────────────────────────

    #[test]
    fn update_project_input_deserializes_all_camel_case_fields() {
        let json = r#"{"name":"Updated","workingDirectory":"/new/path","gitMode":"local","baseBranch":"develop","mergeValidationMode":"strict","mergeStrategy":"rebase_squash"}"#;
        let input: UpdateProjectInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, Some("Updated".to_string()));
        assert_eq!(input.working_directory, Some("/new/path".to_string()));
        assert_eq!(input.git_mode, Some("local".to_string()));
        assert_eq!(input.base_branch, Some("develop".to_string()));
        assert_eq!(
            input.merge_validation_mode,
            Some("strict".to_string())
        );
        assert_eq!(input.merge_strategy, Some("rebase_squash".to_string()));
    }

    #[test]
    fn update_project_input_partial_fields() {
        let json = r#"{"name":"Just Name"}"#;
        let input: UpdateProjectInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, Some("Just Name".to_string()));
        assert!(input.working_directory.is_none());
        assert!(input.git_mode.is_none());
        assert!(input.merge_strategy.is_none());
    }

    #[test]
    fn update_project_input_empty_object() {
        let json = r#"{}"#;
        let input: UpdateProjectInput = serde_json::from_str(json).unwrap();
        assert!(input.name.is_none());
        assert!(input.working_directory.is_none());
        assert!(input.git_mode.is_none());
        assert!(input.base_branch.is_none());
        assert!(input.merge_validation_mode.is_none());
        assert!(input.merge_strategy.is_none());
    }
}
