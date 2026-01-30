// Tauri commands for seeding test/demo data
// Used for visual audits and testing
// Extensible profile-based system for different screen needs

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::application::AppState;
use crate::domain::entities::{InternalStatus, Project, Task};

/// Available test data profiles
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum TestDataProfile {
    /// Minimal data - just a project (for testing empty states)
    Minimal,
    /// Kanban view - project + tasks in various states
    Kanban,
    /// Ideation view - project + ideation sessions + proposals (TODO)
    Ideation,
    /// Full demo - all data types for comprehensive testing
    Full,
}

impl Default for TestDataProfile {
    fn default() -> Self {
        Self::Kanban
    }
}

/// Response for seed operations
#[derive(Debug, Serialize)]
pub struct SeedDataResponse {
    pub profile: String,
    pub project_id: String,
    pub project_name: String,
    pub tasks_created: usize,
    pub sessions_created: usize,
    pub proposals_created: usize,
}

/// Seed data for visual audits with specified profile
#[tauri::command]
pub async fn seed_test_data(
    profile: Option<String>,
    state: State<'_, AppState>,
) -> Result<SeedDataResponse, String> {
    let profile = profile
        .map(|p| match p.as_str() {
            "minimal" => TestDataProfile::Minimal,
            "kanban" => TestDataProfile::Kanban,
            "ideation" => TestDataProfile::Ideation,
            "full" => TestDataProfile::Full,
            _ => TestDataProfile::Kanban,
        })
        .unwrap_or_default();

    match profile {
        TestDataProfile::Minimal => seed_minimal(state).await,
        TestDataProfile::Kanban => seed_kanban(state).await,
        TestDataProfile::Ideation => seed_ideation(state).await,
        TestDataProfile::Full => seed_full(state).await,
    }
}

/// Backward compatible alias for seed_test_data with kanban profile
#[tauri::command]
pub async fn seed_visual_audit_data(
    state: State<'_, AppState>,
) -> Result<SeedDataResponse, String> {
    seed_kanban(state).await
}

/// Clear all test data
#[tauri::command]
pub async fn clear_test_data(
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Get all projects
    let projects = state
        .project_repo
        .get_all()
        .await
        .map_err(|e| e.to_string())?;

    let mut deleted = 0;
    for project in projects {
        // Delete all tasks for this project
        let tasks = state
            .task_repo
            .get_by_project(&project.id)
            .await
            .map_err(|e| e.to_string())?;

        for task in tasks {
            state.task_repo.delete(&task.id).await.map_err(|e| e.to_string())?;
        }

        // Delete the project
        state.project_repo.delete(&project.id).await.map_err(|e| e.to_string())?;
        deleted += 1;
    }

    Ok(format!("Cleared {} projects and their tasks", deleted))
}

// ============================================================================
// Profile Implementations
// ============================================================================

/// Minimal profile - just creates a project
async fn seed_minimal(state: State<'_, AppState>) -> Result<SeedDataResponse, String> {
    let project = create_test_project(&state, "Minimal Test").await?;

    Ok(SeedDataResponse {
        profile: "minimal".to_string(),
        project_id: project.id.as_str().to_string(),
        project_name: project.name,
        tasks_created: 0,
        sessions_created: 0,
        proposals_created: 0,
    })
}

/// Kanban profile - project + tasks in various states
async fn seed_kanban(state: State<'_, AppState>) -> Result<SeedDataResponse, String> {
    let project = create_test_project(&state, "Visual Audit Test").await?;
    let project_id = project.id.clone();
    let mut tasks_created = 0;

    // Backlog tasks
    tasks_created += create_task(
        &state,
        &project_id,
        "Add notifications",
        "Toast notifications for actions",
        "feature",
        0,
        InternalStatus::Backlog,
    ).await?;

    // Ready tasks (To Do column)
    tasks_created += create_task(
        &state,
        &project_id,
        "Implement dark mode",
        "Add dark mode support to the application",
        "feature",
        10,
        InternalStatus::Ready,
    ).await?;

    tasks_created += create_task(
        &state,
        &project_id,
        "Fix sidebar scroll",
        "Sidebar content overflows on small screens",
        "bug",
        20,
        InternalStatus::Ready,
    ).await?;

    // Executing task (In Progress)
    let mut executing_task = Task::new_with_category(
        project_id.clone(),
        "Add keyboard shortcuts".to_string(),
        "feature".to_string(),
    );
    executing_task.description = Some("Implement Cmd+K for quick actions".to_string());
    executing_task.priority = 15;
    executing_task.internal_status = InternalStatus::Executing;
    executing_task.started_at = Some(chrono::Utc::now());
    state.task_repo.create(executing_task).await.map_err(|e| e.to_string())?;
    tasks_created += 1;

    // Completed task (Done)
    let mut completed_task = Task::new_with_category(
        project_id.clone(),
        "Setup project structure".to_string(),
        "setup".to_string(),
    );
    completed_task.description = Some("Initial Tauri + React setup".to_string());
    completed_task.priority = 5;
    completed_task.internal_status = InternalStatus::Approved;
    completed_task.completed_at = Some(chrono::Utc::now());
    state.task_repo.create(completed_task).await.map_err(|e| e.to_string())?;
    tasks_created += 1;

    Ok(SeedDataResponse {
        profile: "kanban".to_string(),
        project_id: project.id.as_str().to_string(),
        project_name: project.name,
        tasks_created,
        sessions_created: 0,
        proposals_created: 0,
    })
}

/// Ideation profile - project + ideation sessions + proposals
async fn seed_ideation(state: State<'_, AppState>) -> Result<SeedDataResponse, String> {
    // Start with kanban data
    let mut response = seed_kanban(state.clone()).await?;
    response.profile = "ideation".to_string();

    // Note: Ideation session and proposal seeding not yet implemented
    // Repositories are available in AppState but seeding logic requires design
    // (initial session state, sample proposals, dependencies between proposals, etc.)

    Ok(response)
}

/// Full profile - all data types
async fn seed_full(state: State<'_, AppState>) -> Result<SeedDataResponse, String> {
    // For now, same as ideation - expand as needed
    let mut response = seed_ideation(state).await?;
    response.profile = "full".to_string();
    Ok(response)
}

// ============================================================================
// Helper Functions
// ============================================================================

async fn create_test_project(
    state: &State<'_, AppState>,
    name: &str,
) -> Result<Project, String> {
    let project = Project::new(
        name.to_string(),
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "/tmp".to_string()),
    );

    state
        .project_repo
        .create(project)
        .await
        .map_err(|e| e.to_string())
}

async fn create_task(
    state: &State<'_, AppState>,
    project_id: &crate::domain::entities::ProjectId,
    title: &str,
    description: &str,
    category: &str,
    priority: i32,
    status: InternalStatus,
) -> Result<usize, String> {
    let mut task = Task::new_with_category(
        project_id.clone(),
        title.to_string(),
        category.to_string(),
    );
    task.description = Some(description.to_string());
    task.priority = priority;
    task.internal_status = status;
    state.task_repo.create(task).await.map_err(|e| e.to_string())?;
    Ok(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_parsing() {
        assert_eq!(
            serde_json::from_str::<TestDataProfile>(r#""minimal""#).unwrap(),
            TestDataProfile::Minimal
        );
        assert_eq!(
            serde_json::from_str::<TestDataProfile>(r#""kanban""#).unwrap(),
            TestDataProfile::Kanban
        );
    }
}
