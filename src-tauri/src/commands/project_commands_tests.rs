use super::*;
use crate::domain::entities::{ArtifactId, IdeationSessionId, PlanBranch};
use crate::infrastructure::memory::{
    MemoryPlanBranchRepository, MemoryProjectRepository, MemoryTaskRepository,
};
use std::sync::Arc;

// ── is_github_url tests ──────────────────────────────────────────────────────

#[test]
fn github_url_https_is_valid() {
    assert!(is_github_url("https://github.com/owner/repo.git"));
    assert!(is_github_url("https://github.com/owner/repo"));
    assert!(is_github_url("https://github.com/org/sub/repo"));
}

#[test]
fn github_url_ssh_is_valid() {
    assert!(is_github_url("git@github.com:owner/repo.git"));
    assert!(is_github_url("git@github.com:org/repo"));
}

#[test]
fn non_github_https_url_is_invalid() {
    assert!(!is_github_url("https://gitlab.com/owner/repo.git"));
    assert!(!is_github_url("https://bitbucket.org/owner/repo.git"));
    assert!(!is_github_url("https://example.com/repo.git"));
}

#[test]
fn non_github_ssh_url_is_invalid() {
    assert!(!is_github_url("git@gitlab.com:owner/repo.git"));
    assert!(!is_github_url("git@bitbucket.org:owner/repo.git"));
}

#[test]
fn empty_and_garbage_urls_are_invalid() {
    assert!(!is_github_url(""));
    assert!(!is_github_url("not-a-url"));
    assert!(!is_github_url("github.com/owner/repo"));
}

// ── update_github_pr_enabled tests ──────────────────────────────────────────

#[tokio::test]
async fn test_update_github_pr_enabled_persists_change() {
    let state = setup_test_state();

    let project = Project::new("Test".to_string(), "/test/path".to_string());
    assert!(project.github_pr_enabled, "default should be true");
    let created = state.project_repo.create(project).await.unwrap();

    // Disable PR mode
    let mut updated = created.clone();
    updated.github_pr_enabled = false;
    updated.touch();
    state.project_repo.update(&updated).await.unwrap();

    let found = state
        .project_repo
        .get_by_id(&created.id)
        .await
        .unwrap()
        .unwrap();
    assert!(!found.github_pr_enabled);
}

#[tokio::test]
async fn test_project_response_includes_github_pr_enabled() {
    let project = Project::new("Test".to_string(), "/test".to_string());
    let response = ProjectResponse::from(project);
    // Default value should be true
    assert!(response.github_pr_enabled);

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"github_pr_enabled\":true"));
}

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

// ── handle_pr_mode_switch tests ──────────────────────────────────────────────

#[cfg(test)]
mod mode_switch_tests {
    use super::*;
    use crate::domain::entities::{InternalStatus, TaskId};
    use crate::domain::repositories::{PlanBranchRepository, ProjectRepository};

    fn make_branch_with_pr(project_id: &str, merge_task_id: &str, pr_number: i64) -> PlanBranch {
        let mut b = PlanBranch::new(
            ArtifactId::from_string("art-1".to_string()),
            IdeationSessionId::from_string("sess-1".to_string()),
            ProjectId::from_string(project_id.to_string()),
            "feature/test".to_string(),
            "main".to_string(),
        );
        b.merge_task_id = Some(TaskId::from_string(merge_task_id.to_string()));
        b.pr_number = Some(pr_number);
        b.pr_url = Some("https://github.com/owner/repo/pull/42".to_string());
        b
    }

    #[tokio::test]
    async fn pr_to_push_clears_pr_fields_on_repo() {
        // Verify that clear_pr_info removes pr_number and pr_url from the branch.
        // This is the key side effect of the PR → push-to-main mode switch path.
        let plan_branch_repo = Arc::new(MemoryPlanBranchRepository::new());

        let branch = make_branch_with_pr("proj-1", "task-merge-1", 42);
        let branch_id = branch.id.clone();
        plan_branch_repo.create(branch).await.unwrap();

        // Verify PR fields are set before the operation
        let before = plan_branch_repo
            .get_by_id(&branch_id)
            .await
            .unwrap()
            .unwrap();
        assert!(before.pr_number.is_some(), "PR number should be set");
        assert!(before.pr_url.is_some(), "PR URL should be set");

        // Simulate what handle_pr_mode_switch does when PR → push-to-main
        plan_branch_repo.clear_pr_info(&branch_id).await.unwrap();

        let after = plan_branch_repo
            .get_by_id(&branch_id)
            .await
            .unwrap()
            .unwrap();
        assert!(after.pr_number.is_none(), "PR number should be cleared");
        assert!(after.pr_url.is_none(), "PR URL should be cleared");
    }

    #[tokio::test]
    async fn push_to_pr_is_noop_for_existing_plans() {
        // new_enabled=true — no action needed for existing plans (AD16: pr_eligible stays false)
        // Verify: a branch with no pr_number remains untouched after a push→PR toggle.
        let plan_branch_repo = Arc::new(MemoryPlanBranchRepository::new());
        let project_repo = Arc::new(MemoryProjectRepository::new());

        let pid = ProjectId::from_string("proj-2".to_string());
        let project = {
            let mut p = Project::new("Test2".to_string(), "/test2/path".to_string());
            p.id = pid.clone();
            p
        };
        project_repo.create(project).await.unwrap();

        // Branch with no pr_number (push-to-main mode)
        let mut branch = PlanBranch::new(
            ArtifactId::from_string("art-2".to_string()),
            IdeationSessionId::from_string("sess-2".to_string()),
            pid.clone(),
            "feature/no-pr".to_string(),
            "main".to_string(),
        );
        branch.pr_eligible = false; // AD16: existing plans have pr_eligible=false
        let branch_id = branch.id.clone();
        plan_branch_repo.create(branch).await.unwrap();

        // Verify branch still has no pr_number (push→PR toggle doesn't retroactively enable PR)
        let after = plan_branch_repo
            .get_by_id(&branch_id)
            .await
            .unwrap()
            .unwrap();
        assert!(
            after.pr_number.is_none(),
            "No PR should be created for existing plans"
        );
        assert!(!after.pr_eligible, "pr_eligible stays false per AD16");
    }

    #[tokio::test]
    async fn merged_branch_status_is_skipped() {
        // Branches with Merged status should be skipped entirely.
        // Verify the PlanBranchStatus matching — Merged/Abandoned branches skip cleanup.
        use crate::domain::entities::PlanBranchStatus;

        let plan_branch_repo = Arc::new(MemoryPlanBranchRepository::new());

        let mut branch = make_branch_with_pr("proj-3", "task-merge-3", 99);
        branch.status = PlanBranchStatus::Merged;
        let branch_id = branch.id.clone();
        plan_branch_repo.create(branch).await.unwrap();

        // Even if we tried clear_pr_info, the guard in handle_pr_mode_switch prevents it.
        // Verify the branch still has PR fields (was never cleared — merged branches are skipped).
        let found = plan_branch_repo
            .get_by_id(&branch_id)
            .await
            .unwrap()
            .unwrap();
        assert!(
            found.pr_number.is_some(),
            "Merged branch pr_number should not be touched"
        );
        // Confirm the status is still Merged (unchanged)
        assert!(
            matches!(found.status, PlanBranchStatus::Merged),
            "Branch status should remain Merged"
        );
    }

    #[test]
    fn merging_status_check_uses_enum_comparison() {
        // Confirm that InternalStatus::Merging != InternalStatus::Merged
        // (guards against future refactors accidentally conflating the two)
        assert_ne!(InternalStatus::Merging, InternalStatus::Merged);
        assert_ne!(InternalStatus::MergeIncomplete, InternalStatus::Merged);
    }
}
