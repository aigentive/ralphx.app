// Tauri commands for Project CRUD operations
// Thin layer that delegates to ProjectRepository

use serde::{Deserialize, Serialize};
use std::process::Command;
use tauri::State;

use crate::application::AppState;
use crate::domain::entities::{GitMode, Project, ProjectId};

/// Input for creating a new project
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectInput {
    pub name: String,
    pub working_directory: String,
    pub git_mode: Option<String>,
    pub worktree_path: Option<String>,
    pub worktree_branch: Option<String>,
    pub base_branch: Option<String>,
}

/// Input for updating a project
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectInput {
    pub name: Option<String>,
    pub working_directory: Option<String>,
    pub git_mode: Option<String>,
    pub worktree_path: Option<String>,
    pub worktree_branch: Option<String>,
    pub base_branch: Option<String>,
}

/// Response wrapper for project operations
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectResponse {
    pub id: String,
    pub name: String,
    pub working_directory: String,
    pub git_mode: String,
    pub worktree_path: Option<String>,
    pub worktree_branch: Option<String>,
    pub base_branch: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Project> for ProjectResponse {
    fn from(project: Project) -> Self {
        Self {
            id: project.id.as_str().to_string(),
            name: project.name,
            working_directory: project.working_directory,
            git_mode: project.git_mode.to_string(),
            worktree_path: project.worktree_path,
            worktree_branch: project.worktree_branch,
            base_branch: project.base_branch,
            created_at: project.created_at.to_rfc3339(),
            updated_at: project.updated_at.to_rfc3339(),
        }
    }
}

/// List all projects
#[tauri::command]
pub async fn list_projects(state: State<'_, AppState>) -> Result<Vec<ProjectResponse>, String> {
    state
        .project_repo
        .get_all()
        .await
        .map(|projects| projects.into_iter().map(ProjectResponse::from).collect())
        .map_err(|e| e.to_string())
}

/// Get a single project by ID
#[tauri::command]
pub async fn get_project(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<ProjectResponse>, String> {
    let project_id = ProjectId::from_string(id);
    state
        .project_repo
        .get_by_id(&project_id)
        .await
        .map(|opt| opt.map(ProjectResponse::from))
        .map_err(|e| e.to_string())
}

/// Create a new project
#[tauri::command]
pub async fn create_project(
    input: CreateProjectInput,
    state: State<'_, AppState>,
) -> Result<ProjectResponse, String> {
    let project = if let (Some(worktree_path), Some(worktree_branch)) =
        (input.worktree_path, input.worktree_branch)
    {
        Project::new_with_worktree(
            input.name,
            input.working_directory,
            worktree_path,
            worktree_branch,
            input.base_branch,
        )
    } else {
        let mut project = Project::new(input.name, input.working_directory);
        if let Some(base_branch) = input.base_branch {
            project.base_branch = Some(base_branch);
        }
        project
    };

    state
        .project_repo
        .create(project)
        .await
        .map(ProjectResponse::from)
        .map_err(|e| e.to_string())
}

/// Update an existing project
#[tauri::command]
pub async fn update_project(
    id: String,
    input: UpdateProjectInput,
    state: State<'_, AppState>,
) -> Result<ProjectResponse, String> {
    let project_id = ProjectId::from_string(id);

    // Get existing project
    let mut project = state
        .project_repo
        .get_by_id(&project_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", project_id.as_str()))?;

    // Apply updates
    if let Some(name) = input.name {
        project.name = name;
    }
    if let Some(working_directory) = input.working_directory {
        project.working_directory = working_directory;
    }
    if let Some(git_mode_str) = input.git_mode {
        project.git_mode = git_mode_str.parse().unwrap_or(GitMode::Local);
    }
    if let Some(worktree_path) = input.worktree_path {
        project.worktree_path = Some(worktree_path);
    }
    if let Some(worktree_branch) = input.worktree_branch {
        project.worktree_branch = Some(worktree_branch);
    }
    if let Some(base_branch) = input.base_branch {
        project.base_branch = Some(base_branch);
    }

    project.touch();

    state
        .project_repo
        .update(&project)
        .await
        .map_err(|e| e.to_string())?;

    Ok(ProjectResponse::from(project))
}

/// Delete a project
#[tauri::command]
pub async fn delete_project(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let project_id = ProjectId::from_string(id);
    state
        .project_repo
        .delete(&project_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get git branches for a working directory
/// Executes `git branch -a` in the specified directory and parses the output
#[tauri::command]
pub async fn get_git_branches(working_directory: String) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args(["branch", "-a"])
        .current_dir(&working_directory)
        .output()
        .map_err(|e| format!("Failed to execute git: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git branch failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let branches: Vec<String> = stdout
        .lines()
        .filter_map(|line| {
            // Remove leading whitespace and asterisk (for current branch)
            let trimmed = line.trim().trim_start_matches("* ");
            // Handle remote branches like "remotes/origin/main" -> just "main"
            if let Some(remote_branch) = trimmed.strip_prefix("remotes/origin/") {
                // Skip HEAD pointer
                if remote_branch.starts_with("HEAD") {
                    return None;
                }
                Some(remote_branch.to_string())
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect::<std::collections::HashSet<_>>() // Deduplicate
        .into_iter()
        .collect();

    // Sort branches with main/master first
    let mut sorted: Vec<String> = branches;
    sorted.sort_by(|a, b| {
        let a_priority = if a == "main" || a == "master" { 0 } else { 1 };
        let b_priority = if b == "main" || b == "master" { 0 } else { 1 };
        a_priority.cmp(&b_priority).then(a.cmp(b))
    });

    Ok(sorted)
}

#[cfg(test)]
mod tests {
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
        assert_eq!(created.git_mode, GitMode::Local);
        assert!(created.worktree_path.is_none());
    }

    #[tokio::test]
    async fn test_create_project_with_worktree() {
        let state = setup_test_state();

        let project = Project::new_with_worktree(
            "Worktree Project".to_string(),
            "/main/repo".to_string(),
            "/worktree/path".to_string(),
            "feature-branch".to_string(),
            Some("main".to_string()),
        );
        let created = state.project_repo.create(project).await.unwrap();

        assert_eq!(created.name, "Worktree Project");
        assert_eq!(created.git_mode, GitMode::Worktree);
        assert_eq!(created.worktree_path, Some("/worktree/path".to_string()));
        assert_eq!(
            created.worktree_branch,
            Some("feature-branch".to_string())
        );
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
        assert_eq!(response.git_mode, "local");

        // Verify it serializes to JSON with camelCase
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"name\":\"Test Project\""));
        assert!(json.contains("\"workingDirectory\":\"/test/path\""));
        assert!(json.contains("\"gitMode\":\"local\""));
    }
}
