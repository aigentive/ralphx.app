// Tauri commands for Task CRUD operations
// Thin layer that delegates to TaskRepository

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::application::AppState;
use crate::domain::entities::{ProjectId, Task, TaskId};

/// Input for creating a new task
#[derive(Debug, Deserialize)]
pub struct CreateTaskInput {
    pub project_id: String,
    pub title: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub priority: Option<i32>,
}

/// Input for updating a task
#[derive(Debug, Deserialize)]
pub struct UpdateTaskInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub priority: Option<i32>,
    pub internal_status: Option<String>,
}

/// Response wrapper for task operations
#[derive(Debug, Serialize)]
pub struct TaskResponse {
    pub id: String,
    pub project_id: String,
    pub category: String,
    pub title: String,
    pub description: Option<String>,
    pub priority: i32,
    pub internal_status: String,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

impl From<Task> for TaskResponse {
    fn from(task: Task) -> Self {
        Self {
            id: task.id.as_str().to_string(),
            project_id: task.project_id.as_str().to_string(),
            category: task.category,
            title: task.title,
            description: task.description,
            priority: task.priority,
            internal_status: task.internal_status.as_str().to_string(),
            created_at: task.created_at.to_rfc3339(),
            updated_at: task.updated_at.to_rfc3339(),
            started_at: task.started_at.map(|dt| dt.to_rfc3339()),
            completed_at: task.completed_at.map(|dt| dt.to_rfc3339()),
        }
    }
}

/// List all tasks for a project
#[tauri::command]
pub async fn list_tasks(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<TaskResponse>, String> {
    let project_id = ProjectId::from_string(project_id);
    state
        .task_repo
        .get_by_project(&project_id)
        .await
        .map(|tasks| tasks.into_iter().map(TaskResponse::from).collect())
        .map_err(|e| e.to_string())
}

/// Get a single task by ID
#[tauri::command]
pub async fn get_task(id: String, state: State<'_, AppState>) -> Result<Option<TaskResponse>, String> {
    let task_id = TaskId::from_string(id);
    state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map(|opt| opt.map(TaskResponse::from))
        .map_err(|e| e.to_string())
}

/// Create a new task
#[tauri::command]
pub async fn create_task(
    input: CreateTaskInput,
    state: State<'_, AppState>,
) -> Result<TaskResponse, String> {
    let project_id = ProjectId::from_string(input.project_id);
    let category = input.category.unwrap_or_else(|| "feature".to_string());

    let mut task = Task::new_with_category(project_id, input.title, category);

    if let Some(desc) = input.description {
        task.description = Some(desc);
    }
    if let Some(priority) = input.priority {
        task.priority = priority;
    }

    state
        .task_repo
        .create(task)
        .await
        .map(TaskResponse::from)
        .map_err(|e| e.to_string())
}

/// Update an existing task
#[tauri::command]
pub async fn update_task(
    id: String,
    input: UpdateTaskInput,
    state: State<'_, AppState>,
) -> Result<TaskResponse, String> {
    let task_id = TaskId::from_string(id);

    // Get existing task
    let mut task = state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Task not found: {}", task_id.as_str()))?;

    // Apply updates
    if let Some(title) = input.title {
        task.title = title;
    }
    if let Some(desc) = input.description {
        task.description = Some(desc);
    }
    if let Some(category) = input.category {
        task.category = category;
    }
    if let Some(priority) = input.priority {
        task.priority = priority;
    }
    if let Some(status_str) = input.internal_status {
        task.internal_status = status_str
            .parse()
            .unwrap_or(task.internal_status);
    }

    task.touch();

    state
        .task_repo
        .update(&task)
        .await
        .map_err(|e| e.to_string())?;

    Ok(TaskResponse::from(task))
}

/// Delete a task
#[tauri::command]
pub async fn delete_task(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let task_id = TaskId::from_string(id);
    state
        .task_repo
        .delete(&task_id)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use crate::infrastructure::memory::MemoryTaskRepository;
    use crate::infrastructure::memory::MemoryProjectRepository;
    use crate::domain::entities::Project;
    use crate::domain::repositories::ProjectRepository;

    async fn setup_test_state() -> AppState {
        let task_repo = Arc::new(MemoryTaskRepository::new());
        let project_repo = Arc::new(MemoryProjectRepository::new());

        // Create a test project
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        project_repo.create(project).await.unwrap();

        AppState::with_repos(task_repo, project_repo)
    }

    #[tokio::test]
    async fn test_create_task_with_defaults() {
        let state = setup_test_state().await;

        let input = CreateTaskInput {
            project_id: "test-project".to_string(),
            title: "Test Task".to_string(),
            category: None,
            description: None,
            priority: None,
        };

        // We can't easily call tauri commands without the full runtime,
        // so we test the repository directly
        let project_id = ProjectId::from_string(input.project_id);
        let task = Task::new(project_id.clone(), input.title);

        let created = state.task_repo.create(task).await.unwrap();
        assert_eq!(created.title, "Test Task");
        assert_eq!(created.category, "feature");
        assert_eq!(created.priority, 0);
    }

    #[tokio::test]
    async fn test_create_task_with_all_fields() {
        let state = setup_test_state().await;

        let project_id = ProjectId::from_string("test-project".to_string());
        let mut task = Task::new_with_category(
            project_id.clone(),
            "Full Task".to_string(),
            "bug".to_string(),
        );
        task.description = Some("A description".to_string());
        task.priority = 10;

        let created = state.task_repo.create(task).await.unwrap();

        assert_eq!(created.title, "Full Task");
        assert_eq!(created.category, "bug");
        assert_eq!(created.description, Some("A description".to_string()));
        assert_eq!(created.priority, 10);
    }

    #[tokio::test]
    async fn test_get_task_returns_none_for_nonexistent() {
        let state = setup_test_state().await;
        let id = TaskId::new();

        let result = state.task_repo.get_by_id(&id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_task_modifies_fields() {
        let state = setup_test_state().await;

        let project_id = ProjectId::from_string("test-project".to_string());
        let task = Task::new(project_id, "Original Title".to_string());
        let created = state.task_repo.create(task).await.unwrap();

        let mut updated = created.clone();
        updated.title = "Updated Title".to_string();
        updated.priority = 99;

        state.task_repo.update(&updated).await.unwrap();

        let found = state.task_repo.get_by_id(&created.id).await.unwrap().unwrap();
        assert_eq!(found.title, "Updated Title");
        assert_eq!(found.priority, 99);
    }

    #[tokio::test]
    async fn test_delete_task_removes_it() {
        let state = setup_test_state().await;

        let project_id = ProjectId::from_string("test-project".to_string());
        let task = Task::new(project_id, "To Delete".to_string());
        let created = state.task_repo.create(task).await.unwrap();

        state.task_repo.delete(&created.id).await.unwrap();

        let found = state.task_repo.get_by_id(&created.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_list_tasks_returns_all_for_project() {
        let state = setup_test_state().await;

        let project_id = ProjectId::from_string("test-project".to_string());

        state.task_repo.create(Task::new(project_id.clone(), "Task 1".to_string())).await.unwrap();
        state.task_repo.create(Task::new(project_id.clone(), "Task 2".to_string())).await.unwrap();
        state.task_repo.create(Task::new(project_id.clone(), "Task 3".to_string())).await.unwrap();

        let tasks = state.task_repo.get_by_project(&project_id).await.unwrap();
        assert_eq!(tasks.len(), 3);
    }

    #[tokio::test]
    async fn test_task_response_serialization() {
        let project_id = ProjectId::from_string("proj-123".to_string());
        let task = Task::new(project_id, "Test Task".to_string());
        let response = TaskResponse::from(task);

        // Verify all fields are set
        assert!(!response.id.is_empty());
        assert_eq!(response.project_id, "proj-123");
        assert_eq!(response.title, "Test Task");
        assert_eq!(response.internal_status, "backlog");

        // Verify it serializes to JSON
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"title\":\"Test Task\""));
    }
}
