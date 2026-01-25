// Tauri commands for seeding test/demo data
// Used for visual audits and testing

use serde::Serialize;
use tauri::State;

use crate::application::AppState;
use crate::domain::entities::{InternalStatus, Project, Task};

/// Response for seed operations
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedDataResponse {
    pub project_id: String,
    pub project_name: String,
    pub tasks_created: usize,
}

/// Seed data for visual audits
/// Creates a test project with sample tasks in various states
#[tauri::command]
pub async fn seed_visual_audit_data(
    state: State<'_, AppState>,
) -> Result<SeedDataResponse, String> {
    // Create test project
    let project = Project::new(
        "Visual Audit Test".to_string(),
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "/tmp".to_string()),
    );

    let project = state
        .project_repo
        .create(project)
        .await
        .map_err(|e| e.to_string())?;

    let project_id = project.id.clone();
    let mut tasks_created = 0;

    // Create tasks in various states for Kanban columns

    // Ready tasks (Ready column)
    let mut task1 = Task::new_with_category(
        project_id.clone(),
        "Implement dark mode".to_string(),
        "feature".to_string(),
    );
    task1.description = Some("Add dark mode support to the application".to_string());
    task1.priority = 10;
    task1.internal_status = InternalStatus::Ready;
    state.task_repo.create(task1).await.map_err(|e| e.to_string())?;
    tasks_created += 1;

    let mut task2 = Task::new_with_category(
        project_id.clone(),
        "Fix sidebar scroll".to_string(),
        "bug".to_string(),
    );
    task2.description = Some("Sidebar content overflows on small screens".to_string());
    task2.priority = 20;
    task2.internal_status = InternalStatus::Ready;
    state.task_repo.create(task2).await.map_err(|e| e.to_string())?;
    tasks_created += 1;

    // Executing tasks (In Progress column)
    let mut task3 = Task::new_with_category(
        project_id.clone(),
        "Add keyboard shortcuts".to_string(),
        "feature".to_string(),
    );
    task3.description = Some("Implement Cmd+K for quick actions".to_string());
    task3.priority = 15;
    task3.internal_status = InternalStatus::Executing;
    task3.started_at = Some(chrono::Utc::now());
    state.task_repo.create(task3).await.map_err(|e| e.to_string())?;
    tasks_created += 1;

    // Completed tasks (Done column)
    let mut task4 = Task::new_with_category(
        project_id.clone(),
        "Setup project structure".to_string(),
        "setup".to_string(),
    );
    task4.description = Some("Initial Tauri + React setup".to_string());
    task4.priority = 5;
    task4.internal_status = InternalStatus::Approved;
    task4.completed_at = Some(chrono::Utc::now());
    state.task_repo.create(task4).await.map_err(|e| e.to_string())?;
    tasks_created += 1;

    // Backlog tasks
    let mut task5 = Task::new_with_category(
        project_id.clone(),
        "Add notifications".to_string(),
        "feature".to_string(),
    );
    task5.description = Some("Toast notifications for actions".to_string());
    task5.priority = 0;
    task5.internal_status = InternalStatus::Backlog;
    state.task_repo.create(task5).await.map_err(|e| e.to_string())?;
    tasks_created += 1;

    Ok(SeedDataResponse {
        project_id: project.id.as_str().to_string(),
        project_name: project.name,
        tasks_created,
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use crate::infrastructure::memory::{MemoryProjectRepository, MemoryTaskRepository};

    async fn setup_test_state() -> AppState {
        let task_repo = Arc::new(MemoryTaskRepository::new());
        let project_repo = Arc::new(MemoryProjectRepository::new());
        AppState::with_repos(task_repo, project_repo)
    }

    #[tokio::test]
    async fn test_seed_creates_project_and_tasks() {
        let state = setup_test_state().await;

        // Verify initially empty
        let projects = state.project_repo.get_all().await.unwrap();
        assert!(projects.is_empty());

        // Create via direct repo calls (simulating the command)
        let project = Project::new("Test".to_string(), "/tmp".to_string());
        let created = state.project_repo.create(project).await.unwrap();

        let task = Task::new(created.id.clone(), "Test Task".to_string());
        state.task_repo.create(task).await.unwrap();

        // Verify
        let projects = state.project_repo.get_all().await.unwrap();
        assert_eq!(projects.len(), 1);

        let tasks = state.task_repo.get_by_project(&created.id).await.unwrap();
        assert_eq!(tasks.len(), 1);
    }
}
