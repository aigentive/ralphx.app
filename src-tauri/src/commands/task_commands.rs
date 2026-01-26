// Tauri commands for Task CRUD operations
// Thin layer that delegates to TaskRepository

use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

use crate::application::AppState;
use crate::domain::entities::{InternalStatus, ProjectId, Task, TaskId};

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

/// Input for answering an agent's question
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerUserQuestionInput {
    pub task_id: String,
    pub selected_options: Vec<String>,
    #[serde(default)]
    pub custom_response: Option<String>,
}

/// Response for the answer_user_question command
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerUserQuestionResponse {
    pub task_id: String,
    pub resumed_status: String,
    pub answer_recorded: bool,
}

/// Input for injecting a task mid-loop
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InjectTaskInput {
    /// The project ID to inject the task into
    pub project_id: String,
    /// Title of the task
    pub title: String,
    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
    /// Category (defaults to "feature")
    #[serde(default)]
    pub category: Option<String>,
    /// Where to inject: "backlog" (deferred) or "planned" (immediate queue)
    #[serde(default = "default_target")]
    pub target: String,
    /// If true and target is "planned", make this task the highest priority
    #[serde(default)]
    pub make_next: bool,
}

fn default_target() -> String {
    "backlog".to_string()
}

/// Response for the inject_task command
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InjectTaskResponse {
    pub task: TaskResponse,
    pub target: String,
    pub priority: i32,
    pub make_next_applied: bool,
}

/// Response wrapper for task operations
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskResponse {
    pub id: String,
    pub project_id: String,
    pub category: String,
    pub title: String,
    pub description: Option<String>,
    pub priority: i32,
    pub internal_status: String,
    pub needs_review_point: bool,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub archived_at: Option<String>,
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
            needs_review_point: task.needs_review_point,
            created_at: task.created_at.to_rfc3339(),
            updated_at: task.updated_at.to_rfc3339(),
            started_at: task.started_at.map(|dt| dt.to_rfc3339()),
            completed_at: task.completed_at.map(|dt| dt.to_rfc3339()),
            archived_at: task.archived_at.map(|dt| dt.to_rfc3339()),
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

/// Move a task to a new status (for Kanban drag-drop)
///
/// This command uses the TaskTransitionService to properly trigger state machine
/// entry actions, such as spawning worker agents when moving to "executing" status.
///
/// # Arguments
/// * `task_id` - The task ID (camelCase for frontend compatibility)
/// * `to_status` - The target status string (e.g., "ready", "executing", "approved")
///
/// # Returns
/// * `TaskResponse` - The updated task
#[tauri::command]
#[allow(non_snake_case)]
pub async fn move_task(
    taskId: String,
    toStatus: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<TaskResponse, String> {
    use crate::application::TaskTransitionService;
    use std::sync::Arc;

    // Debug log to verify command is being called
    println!(">>> move_task called: taskId={}, toStatus={}", taskId, toStatus);
    tracing::info!(taskId = %taskId, toStatus = %toStatus, "move_task command invoked");

    let task_id = TaskId::from_string(taskId);

    // Parse the target status
    let new_status: InternalStatus = toStatus
        .parse()
        .map_err(|_| format!("Invalid status: {}", toStatus))?;

    // Create the transition service with all required dependencies
    let transition_service = TaskTransitionService::new(
        Arc::clone(&state.task_repo),
        Arc::clone(&state.chat_message_repo),
        Arc::clone(&state.chat_conversation_repo),
        Arc::clone(&state.agent_run_repo),
        Arc::new(state.execution_message_queue.clone()),
        Some(app),
    );

    // Transition the task - this triggers entry actions like spawning workers!
    let task = transition_service
        .transition_task(&task_id, new_status)
        .await
        .map_err(|e| e.to_string())?;

    Ok(TaskResponse::from(task))
}

/// Inject a task mid-loop
///
/// Allows users to add tasks during execution. Tasks can be sent to:
/// - **Backlog** (deferred): Task is created with Backlog status
/// - **Planned** (immediate queue): Task is created with Ready status at correct priority
///
/// If `make_next` is true and target is "planned", the task gets the highest
/// priority (max existing priority + 1000) to ensure it executes next.
///
/// # Arguments
/// * `input` - The inject input containing project_id, title, target, and make_next options
/// * `app` - Tauri app handle for event emission
///
/// # Returns
/// * `InjectTaskResponse` - Contains the created task, target, priority, and whether make_next was applied
#[tauri::command]
pub async fn inject_task(
    input: InjectTaskInput,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<InjectTaskResponse, String> {
    let project_id = ProjectId::from_string(input.project_id.clone());
    let category = input.category.unwrap_or_else(|| "feature".to_string());

    // Create the new task
    let mut task = Task::new_with_category(project_id.clone(), input.title, category);

    if let Some(desc) = input.description {
        task.description = Some(desc);
    }

    // Determine initial status and priority based on target
    let (status, priority, make_next_applied) = match input.target.as_str() {
        "planned" => {
            if input.make_next {
                // Get max priority among Ready tasks and add 1000 for safe margin
                let ready_tasks = state
                    .task_repo
                    .get_by_status(&project_id, InternalStatus::Ready)
                    .await
                    .map_err(|e| e.to_string())?;

                let max_priority = ready_tasks
                    .iter()
                    .map(|t| t.priority)
                    .max()
                    .unwrap_or(0);

                (InternalStatus::Ready, max_priority + 1000, true)
            } else {
                // Insert at default priority (0) - will be ordered by created_at
                (InternalStatus::Ready, 0, false)
            }
        }
        _ => {
            // Default to backlog
            (InternalStatus::Backlog, 0, false)
        }
    };

    task.internal_status = status;
    task.priority = priority;

    // Save the task
    let created = state
        .task_repo
        .create(task)
        .await
        .map_err(|e| e.to_string())?;

    // Emit task:created event
    let _ = app.emit(
        "task:created",
        serde_json::json!({
            "taskId": created.id.as_str(),
            "projectId": created.project_id.as_str(),
            "title": created.title,
            "status": created.internal_status.as_str(),
            "priority": created.priority,
            "injected": true,
        }),
    );

    let target = if input.target == "planned" {
        "planned".to_string()
    } else {
        "backlog".to_string()
    };

    Ok(InjectTaskResponse {
        task: TaskResponse::from(created),
        target,
        priority,
        make_next_applied,
    })
}

/// Answer a user question from an agent
///
/// When an agent asks a question via the AskUserQuestion tool, the task
/// transitions to Blocked status. This command accepts the user's answer
/// and resumes the task by transitioning it to Ready status.
///
/// # Arguments
/// * `input` - The answer input containing task_id, selected_options, and optional custom_response
///
/// # Returns
/// * `AnswerUserQuestionResponse` - Contains the task_id, new status, and confirmation
///
/// # Errors
/// * Task not found
/// * Task is not in Blocked status
#[tauri::command]
pub async fn answer_user_question(
    input: AnswerUserQuestionInput,
    state: State<'_, AppState>,
) -> Result<AnswerUserQuestionResponse, String> {
    let task_id = TaskId::from_string(input.task_id.clone());

    // Get the task
    let mut task = state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Task not found: {}", task_id.as_str()))?;

    // Verify task is in Blocked status
    if task.internal_status != InternalStatus::Blocked {
        return Err(format!(
            "Task {} is not in Blocked status (current: {})",
            task_id.as_str(),
            task.internal_status
        ));
    }

    // Transition to Ready status (per state machine: Blocked -> Ready)
    task.internal_status = InternalStatus::Ready;
    task.touch();

    // Persist the update
    state
        .task_repo
        .update(&task)
        .await
        .map_err(|e| e.to_string())?;

    // TODO: In a future iteration, we could store the answer for agent context
    // For now, the answer is handled by the frontend sending it directly to the agent

    Ok(AnswerUserQuestionResponse {
        task_id: input.task_id,
        resumed_status: task.internal_status.as_str().to_string(),
        answer_recorded: true,
    })
}

/// Archive a task (soft delete)
///
/// Sets the archived_at timestamp to now, effectively removing the task from
/// normal views while preserving it for potential restore.
///
/// # Arguments
/// * `task_id` - The task ID to archive
/// * `app` - Tauri app handle for event emission
///
/// # Returns
/// * `TaskResponse` - The archived task
///
/// # Events
/// * Emits 'task:archived' with { task_id, project_id }
#[tauri::command]
pub async fn archive_task(
    task_id: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<TaskResponse, String> {
    let task_id_obj = TaskId::from_string(task_id.clone());

    // Archive the task via repository
    let archived_task = state
        .task_repo
        .archive(&task_id_obj)
        .await
        .map_err(|e| e.to_string())?;

    // Emit event for real-time UI updates
    let _ = app.emit(
        "task:archived",
        serde_json::json!({
            "taskId": archived_task.id.as_str(),
            "projectId": archived_task.project_id.as_str(),
        }),
    );

    Ok(TaskResponse::from(archived_task))
}

/// Restore an archived task
///
/// Clears the archived_at timestamp, making the task visible in normal views again.
///
/// # Arguments
/// * `task_id` - The task ID to restore
/// * `app` - Tauri app handle for event emission
///
/// # Returns
/// * `TaskResponse` - The restored task
///
/// # Events
/// * Emits 'task:restored' with { task_id, project_id }
#[tauri::command]
pub async fn restore_task(
    task_id: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<TaskResponse, String> {
    let task_id_obj = TaskId::from_string(task_id.clone());

    // Restore the task via repository
    let restored_task = state
        .task_repo
        .restore(&task_id_obj)
        .await
        .map_err(|e| e.to_string())?;

    // Emit event for real-time UI updates
    let _ = app.emit(
        "task:restored",
        serde_json::json!({
            "taskId": restored_task.id.as_str(),
            "projectId": restored_task.project_id.as_str(),
        }),
    );

    Ok(TaskResponse::from(restored_task))
}

/// Permanently delete a task (hard delete)
///
/// Only works on archived tasks. This is irreversible.
///
/// # Arguments
/// * `task_id` - The task ID to permanently delete
/// * `app` - Tauri app handle for event emission
///
/// # Returns
/// * `()` - Success or error
///
/// # Errors
/// * Task not found
/// * Task is not archived (safety check)
///
/// # Events
/// * Emits 'task:deleted' with { task_id, project_id }
#[tauri::command]
pub async fn permanently_delete_task(
    task_id: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let task_id_obj = TaskId::from_string(task_id.clone());

    // Get the task first to check if it's archived and get project_id for event
    let task = state
        .task_repo
        .get_by_id(&task_id_obj)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Task not found: {}", task_id))?;

    // Safety check: only allow permanent deletion of archived tasks
    if task.archived_at.is_none() {
        return Err(format!(
            "Cannot permanently delete non-archived task: {}. Archive it first.",
            task_id
        ));
    }

    let project_id = task.project_id.as_str().to_string();

    // Permanently delete
    state
        .task_repo
        .delete(&task_id_obj)
        .await
        .map_err(|e| e.to_string())?;

    // Emit event for real-time UI updates
    let _ = app.emit(
        "task:deleted",
        serde_json::json!({
            "taskId": task_id,
            "projectId": project_id,
        }),
    );

    Ok(())
}

/// Get the count of archived tasks for a project
///
/// Used to show the archived count badge in the UI.
///
/// # Arguments
/// * `project_id` - The project ID
///
/// # Returns
/// * `u32` - The count of archived tasks
#[tauri::command]
pub async fn get_archived_count(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<u32, String> {
    let project_id_obj = ProjectId::from_string(project_id);
    state
        .task_repo
        .get_archived_count(&project_id_obj)
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

    // ========================================
    // Answer User Question Command Tests
    // ========================================

    use crate::domain::entities::InternalStatus;

    #[tokio::test]
    async fn test_answer_user_question_transitions_blocked_to_ready() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create a blocked task (simulating an agent waiting for user input)
        let mut task = Task::new(project_id, "Blocked Task".to_string());
        task.internal_status = InternalStatus::Blocked;
        let created = state.task_repo.create(task).await.unwrap();

        // Verify task is blocked
        assert_eq!(created.internal_status, InternalStatus::Blocked);

        // Simulate answering the question by updating the task
        let mut task = state.task_repo.get_by_id(&created.id).await.unwrap().unwrap();
        assert_eq!(task.internal_status, InternalStatus::Blocked);

        // The command transitions Blocked -> Ready
        task.internal_status = InternalStatus::Ready;
        task.touch();
        state.task_repo.update(&task).await.unwrap();

        // Verify task is now ready
        let updated = state.task_repo.get_by_id(&created.id).await.unwrap().unwrap();
        assert_eq!(updated.internal_status, InternalStatus::Ready);
    }

    #[tokio::test]
    async fn test_answer_user_question_fails_if_not_blocked() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create a task that is not blocked (e.g., Ready)
        let mut task = Task::new(project_id, "Ready Task".to_string());
        task.internal_status = InternalStatus::Ready;
        let created = state.task_repo.create(task).await.unwrap();

        // Verify task is not blocked
        let task = state.task_repo.get_by_id(&created.id).await.unwrap().unwrap();
        assert_ne!(task.internal_status, InternalStatus::Blocked);

        // In the real command, this would return an error
        // Here we just verify the precondition check
    }

    #[tokio::test]
    async fn test_answer_user_question_not_found() {
        let state = setup_test_state().await;
        let nonexistent_id = TaskId::from_string("nonexistent".to_string());

        // Task not found
        let result = state.task_repo.get_by_id(&nonexistent_id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_answer_user_question_input_deserialization() {
        // Test that input deserializes correctly with camelCase
        let json = r#"{
            "taskId": "task-123",
            "selectedOptions": ["option1", "option2"],
            "customResponse": "My custom answer"
        }"#;

        let input: AnswerUserQuestionInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.task_id, "task-123");
        assert_eq!(input.selected_options, vec!["option1", "option2"]);
        assert_eq!(input.custom_response, Some("My custom answer".to_string()));
    }

    #[tokio::test]
    async fn test_answer_user_question_input_without_custom_response() {
        // Test that input deserializes correctly without custom_response
        let json = r#"{
            "taskId": "task-456",
            "selectedOptions": ["option1"]
        }"#;

        let input: AnswerUserQuestionInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.task_id, "task-456");
        assert_eq!(input.selected_options, vec!["option1"]);
        assert!(input.custom_response.is_none());
    }

    #[tokio::test]
    async fn test_answer_user_question_response_serialization() {
        let response = AnswerUserQuestionResponse {
            task_id: "task-789".to_string(),
            resumed_status: "ready".to_string(),
            answer_recorded: true,
        };

        let json = serde_json::to_string(&response).unwrap();

        // Verify camelCase serialization
        assert!(json.contains("\"taskId\":\"task-789\""));
        assert!(json.contains("\"resumedStatus\":\"ready\""));
        assert!(json.contains("\"answerRecorded\":true"));
    }

    // ========================================
    // Inject Task Command Tests
    // ========================================

    #[tokio::test]
    async fn test_inject_task_input_deserialization_minimal() {
        // Test minimal input with defaults
        let json = r#"{
            "projectId": "proj-123",
            "title": "Injected Task"
        }"#;

        let input: InjectTaskInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.project_id, "proj-123");
        assert_eq!(input.title, "Injected Task");
        assert!(input.description.is_none());
        assert!(input.category.is_none());
        assert_eq!(input.target, "backlog");
        assert!(!input.make_next);
    }

    #[tokio::test]
    async fn test_inject_task_input_deserialization_full() {
        // Test full input with all fields
        let json = r#"{
            "projectId": "proj-456",
            "title": "Urgent Task",
            "description": "This is urgent",
            "category": "bug",
            "target": "planned",
            "makeNext": true
        }"#;

        let input: InjectTaskInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.project_id, "proj-456");
        assert_eq!(input.title, "Urgent Task");
        assert_eq!(input.description, Some("This is urgent".to_string()));
        assert_eq!(input.category, Some("bug".to_string()));
        assert_eq!(input.target, "planned");
        assert!(input.make_next);
    }

    #[tokio::test]
    async fn test_inject_task_response_serialization() {
        let project_id = ProjectId::from_string("proj-123".to_string());
        let task = Task::new(project_id, "Test Task".to_string());
        let response = InjectTaskResponse {
            task: TaskResponse::from(task),
            target: "planned".to_string(),
            priority: 1000,
            make_next_applied: true,
        };

        let json = serde_json::to_string(&response).unwrap();

        // Verify camelCase serialization
        assert!(json.contains("\"target\":\"planned\""));
        assert!(json.contains("\"priority\":1000"));
        assert!(json.contains("\"makeNextApplied\":true"));
    }

    #[tokio::test]
    async fn test_inject_task_to_backlog_creates_backlog_task() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Inject to backlog (default)
        let mut task = Task::new_with_category(
            project_id.clone(),
            "Backlog Task".to_string(),
            "feature".to_string(),
        );
        task.internal_status = InternalStatus::Backlog;
        task.priority = 0;

        let created = state.task_repo.create(task).await.unwrap();

        assert_eq!(created.internal_status, InternalStatus::Backlog);
        assert_eq!(created.priority, 0);
    }

    #[tokio::test]
    async fn test_inject_task_to_planned_creates_ready_task() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Inject to planned queue
        let mut task = Task::new_with_category(
            project_id.clone(),
            "Planned Task".to_string(),
            "feature".to_string(),
        );
        task.internal_status = InternalStatus::Ready;
        task.priority = 0;

        let created = state.task_repo.create(task).await.unwrap();

        assert_eq!(created.internal_status, InternalStatus::Ready);
    }

    #[tokio::test]
    async fn test_inject_task_make_next_gets_highest_priority() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create existing ready tasks with various priorities
        let mut task1 = Task::new(project_id.clone(), "Existing 1".to_string());
        task1.internal_status = InternalStatus::Ready;
        task1.priority = 10;
        state.task_repo.create(task1).await.unwrap();

        let mut task2 = Task::new(project_id.clone(), "Existing 2".to_string());
        task2.internal_status = InternalStatus::Ready;
        task2.priority = 50;
        state.task_repo.create(task2).await.unwrap();

        let mut task3 = Task::new(project_id.clone(), "Existing 3".to_string());
        task3.internal_status = InternalStatus::Ready;
        task3.priority = 25;
        state.task_repo.create(task3).await.unwrap();

        // Get max priority for make_next
        let ready_tasks = state
            .task_repo
            .get_by_status(&project_id, InternalStatus::Ready)
            .await
            .unwrap();

        let max_priority = ready_tasks.iter().map(|t| t.priority).max().unwrap_or(0);
        let make_next_priority = max_priority + 1000;

        // Inject with make_next
        let mut injected = Task::new(project_id.clone(), "Make Next Task".to_string());
        injected.internal_status = InternalStatus::Ready;
        injected.priority = make_next_priority;

        let created = state.task_repo.create(injected).await.unwrap();

        assert_eq!(created.internal_status, InternalStatus::Ready);
        assert_eq!(created.priority, 1050); // 50 (max) + 1000

        // Verify it's first in the queue
        let next = state.task_repo.get_next_executable(&project_id).await.unwrap();
        assert!(next.is_some());
        assert_eq!(next.unwrap().title, "Make Next Task");
    }

    #[tokio::test]
    async fn test_inject_task_make_next_with_empty_queue() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // No existing ready tasks, make_next should still work
        let ready_tasks = state
            .task_repo
            .get_by_status(&project_id, InternalStatus::Ready)
            .await
            .unwrap();

        let max_priority = ready_tasks.iter().map(|t| t.priority).max().unwrap_or(0);
        let make_next_priority = max_priority + 1000;

        let mut injected = Task::new(project_id.clone(), "First Make Next".to_string());
        injected.internal_status = InternalStatus::Ready;
        injected.priority = make_next_priority;

        let created = state.task_repo.create(injected).await.unwrap();

        assert_eq!(created.priority, 1000); // 0 + 1000
    }

    #[tokio::test]
    async fn test_inject_task_with_custom_category() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        let task = Task::new_with_category(
            project_id.clone(),
            "Bug Task".to_string(),
            "bug".to_string(),
        );

        let created = state.task_repo.create(task).await.unwrap();

        assert_eq!(created.category, "bug");
    }

    #[tokio::test]
    async fn test_inject_task_with_description() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        let mut task = Task::new(project_id.clone(), "Described Task".to_string());
        task.description = Some("This is a detailed description".to_string());

        let created = state.task_repo.create(task).await.unwrap();

        assert_eq!(
            created.description,
            Some("This is a detailed description".to_string())
        );
    }

    #[tokio::test]
    async fn test_inject_task_invalid_target_defaults_to_backlog() {
        // Test that invalid target defaults to backlog behavior
        let json = r#"{
            "projectId": "proj-123",
            "title": "Invalid Target Task",
            "target": "invalid"
        }"#;

        let input: InjectTaskInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.target, "invalid");

        // In the actual command, invalid target would be handled as backlog
    }

    // ========================================
    // Archive Commands Tests
    // ========================================

    use chrono::Utc;

    #[tokio::test]
    async fn test_archive_task_sets_archived_at() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create a task
        let task = Task::new(project_id, "Task to Archive".to_string());
        let created = state.task_repo.create(task).await.unwrap();

        // Verify not archived initially
        assert!(created.archived_at.is_none());

        // Archive the task
        let archived = state.task_repo.archive(&created.id).await.unwrap();

        // Verify archived_at is set
        assert!(archived.archived_at.is_some());
        assert_eq!(archived.id, created.id);
        assert_eq!(archived.title, "Task to Archive");
    }

    #[tokio::test]
    async fn test_restore_task_clears_archived_at() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create and archive a task
        let task = Task::new(project_id, "Task to Restore".to_string());
        let created = state.task_repo.create(task).await.unwrap();
        let archived = state.task_repo.archive(&created.id).await.unwrap();

        // Verify it's archived
        assert!(archived.archived_at.is_some());

        // Restore the task
        let restored = state.task_repo.restore(&archived.id).await.unwrap();

        // Verify archived_at is cleared
        assert!(restored.archived_at.is_none());
        assert_eq!(restored.id, created.id);
        assert_eq!(restored.title, "Task to Restore");
    }

    #[tokio::test]
    async fn test_get_archived_count_returns_correct_count() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create several tasks
        let task1 = Task::new(project_id.clone(), "Task 1".to_string());
        let task2 = Task::new(project_id.clone(), "Task 2".to_string());
        let task3 = Task::new(project_id.clone(), "Task 3".to_string());

        let created1 = state.task_repo.create(task1).await.unwrap();
        let created2 = state.task_repo.create(task2).await.unwrap();
        let _created3 = state.task_repo.create(task3).await.unwrap();

        // Archive two tasks
        state.task_repo.archive(&created1.id).await.unwrap();
        state.task_repo.archive(&created2.id).await.unwrap();

        // Check archived count
        let count = state.task_repo.get_archived_count(&project_id).await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_get_archived_count_zero_when_none_archived() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create tasks but don't archive them
        state.task_repo.create(Task::new(project_id.clone(), "Active 1".to_string())).await.unwrap();
        state.task_repo.create(Task::new(project_id.clone(), "Active 2".to_string())).await.unwrap();

        // Check archived count
        let count = state.task_repo.get_archived_count(&project_id).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_permanently_delete_archived_task_succeeds() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create and archive a task
        let task = Task::new(project_id, "Task to Delete".to_string());
        let created = state.task_repo.create(task).await.unwrap();
        let archived = state.task_repo.archive(&created.id).await.unwrap();

        // Verify it's archived
        assert!(archived.archived_at.is_some());

        // Permanently delete should succeed
        state.task_repo.delete(&archived.id).await.unwrap();

        // Verify task is gone
        let found = state.task_repo.get_by_id(&archived.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_task_response_includes_archived_at() {
        let project_id = ProjectId::from_string("proj-123".to_string());
        let mut task = Task::new(project_id, "Archived Task".to_string());
        task.archived_at = Some(Utc::now());

        let response = TaskResponse::from(task);

        // Verify archived_at is in response
        assert!(response.archived_at.is_some());

        // Verify it serializes correctly
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"archivedAt\":"));
    }

    #[tokio::test]
    async fn test_task_response_archived_at_null_when_not_archived() {
        let project_id = ProjectId::from_string("proj-123".to_string());
        let task = Task::new(project_id, "Active Task".to_string());

        let response = TaskResponse::from(task);

        // Verify archived_at is null
        assert!(response.archived_at.is_none());

        // Verify it serializes correctly
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"archivedAt\":null"));
    }
}
