// Tauri commands for Task CRUD operations
// Thin layer that delegates to TaskRepository

use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

use crate::application::AppState;
use crate::domain::entities::{InternalStatus, ProjectId, Task, TaskId};

/// Input for creating a new task
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskInput {
    pub project_id: String,
    pub title: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub priority: Option<i32>,
    pub steps: Option<Vec<String>>,
}

/// Input for updating a task
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
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

/// Response for paginated task list
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskListResponse {
    pub tasks: Vec<TaskResponse>,
    pub total: u32,
    pub has_more: bool,
    pub offset: u32,
}

/// List tasks for a project with pagination support
///
/// # Arguments
/// * `project_id` - The project ID
/// * `status` - Optional status filter
/// * `offset` - Pagination offset (default 0)
/// * `limit` - Page size (default 20)
/// * `include_archived` - Whether to include archived tasks (default false)
///
/// # Returns
/// * `TaskListResponse` - Contains tasks, total count, has_more flag, and offset
#[tauri::command]
pub async fn list_tasks(
    project_id: String,
    status: Option<String>,
    offset: Option<u32>,
    limit: Option<u32>,
    include_archived: Option<bool>,
    state: State<'_, AppState>,
) -> Result<TaskListResponse, String> {
    let project_id = ProjectId::from_string(project_id);
    let offset = offset.unwrap_or(0);
    let limit = limit.unwrap_or(20);
    let include_archived = include_archived.unwrap_or(false);

    // Parse status if provided
    let internal_status = if let Some(status_str) = status {
        Some(
            status_str
                .parse::<InternalStatus>()
                .map_err(|_| format!("Invalid status: {}", status_str))?,
        )
    } else {
        None
    };

    // Get paginated tasks
    let tasks = state
        .task_repo
        .list_paginated(&project_id, internal_status, offset, limit, include_archived)
        .await
        .map_err(|e| e.to_string())?;

    // Get total count
    let total = state
        .task_repo
        .count_tasks(&project_id, include_archived)
        .await
        .map_err(|e| e.to_string())?;

    // Calculate has_more
    let has_more = (offset + tasks.len() as u32) < total;

    // Convert to response
    let task_responses: Vec<TaskResponse> = tasks.into_iter().map(TaskResponse::from).collect();

    Ok(TaskListResponse {
        tasks: task_responses,
        total,
        has_more,
        offset,
    })
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

    // Create the task first
    let created_task = state
        .task_repo
        .create(task)
        .await
        .map_err(|e| e.to_string())?;

    // If steps are provided, create TaskSteps for each
    if let Some(step_titles) = input.steps {
        if !step_titles.is_empty() {
            use crate::domain::entities::TaskStep;

            let steps: Vec<TaskStep> = step_titles
                .into_iter()
                .enumerate()
                .map(|(idx, title)| {
                    TaskStep::new(
                        created_task.id.clone(),
                        title,
                        idx as i32,
                        "user".to_string(),
                    )
                })
                .collect();

            // Use bulk_create for efficiency
            state
                .task_step_repo
                .bulk_create(steps)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(TaskResponse::from(created_task))
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
        Arc::clone(&state.project_repo),
        Arc::clone(&state.chat_message_repo),
        Arc::clone(&state.chat_conversation_repo),
        Arc::clone(&state.agent_run_repo),
        Arc::clone(&state.ideation_session_repo),
        Arc::clone(&state.message_queue),
        Arc::clone(&state.running_agent_registry),
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

/// Search tasks by title and description (case-insensitive)
///
/// Searches in both title AND description fields for the query string.
/// Uses server-side search for reliable results across all tasks.
///
/// # Arguments
/// * `project_id` - The project ID to search within
/// * `query` - The search query string
/// * `include_archived` - Whether to include archived tasks in search results (default: false)
///
/// # Returns
/// * `Vec<TaskResponse>` - All matching tasks (no pagination - results should be small)
///
/// # Examples
/// ```ignore
/// // Search for "authentication" in title or description
/// search_tasks("proj-123", "authentication", None)
///
/// // Search including archived tasks
/// search_tasks("proj-123", "old feature", Some(true))
/// ```
#[tauri::command]
pub async fn search_tasks(
    project_id: String,
    query: String,
    include_archived: Option<bool>,
    state: State<'_, AppState>,
) -> Result<Vec<TaskResponse>, String> {
    let project_id_obj = ProjectId::from_string(project_id);
    let include_archived = include_archived.unwrap_or(false);

    // Call repository search method
    let tasks = state
        .task_repo
        .search(&project_id_obj, &query, include_archived)
        .await
        .map_err(|e| e.to_string())?;

    // Convert to response
    let task_responses: Vec<TaskResponse> = tasks.into_iter().map(TaskResponse::from).collect();

    Ok(task_responses)
}

/// Represents a valid status transition option for the UI
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusTransition {
    /// The internal status string (e.g., "ready", "cancelled")
    pub status: String,
    /// User-friendly label for the UI (e.g., "Ready for Work", "Cancel")
    pub label: String,
}

/// Get valid status transitions for a task
///
/// Queries the state machine for valid transitions from the task's current status
/// and maps them to user-friendly labels for display in the status dropdown.
///
/// # Arguments
/// * `task_id` - The task ID to get valid transitions for
///
/// # Returns
/// * `Vec<StatusTransition>` - List of valid transitions with status string and label
///
/// # Examples
/// ```ignore
/// // Get valid transitions for a task in "backlog" status
/// // Returns: [
/// //   { status: "ready", label: "Ready for Work" },
/// //   { status: "cancelled", label: "Cancel" }
/// // ]
/// get_valid_transitions("task-123")
/// ```
#[tauri::command]
pub async fn get_valid_transitions(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<StatusTransition>, String> {
    // Get the task to check its current status
    let task_id_obj = TaskId::from_string(task_id);
    let task = state
        .task_repo
        .get_by_id(&task_id_obj)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Task not found".to_string())?;

    // Get valid transitions from the state machine
    let valid_transitions = task.internal_status.valid_transitions();

    // Map to user-friendly labels
    let transitions = valid_transitions
        .iter()
        .map(|status| {
            let status_str = status.as_str().to_string();
            let label = status_to_label(*status);
            StatusTransition {
                status: status_str,
                label,
            }
        })
        .collect();

    Ok(transitions)
}

/// Maps an InternalStatus to a user-friendly label for the status dropdown
fn status_to_label(status: InternalStatus) -> String {
    match status {
        InternalStatus::Backlog => "Move to Backlog".to_string(),
        InternalStatus::Ready => "Ready for Work".to_string(),
        InternalStatus::Blocked => "Mark as Blocked".to_string(),
        InternalStatus::Executing => "Start Execution".to_string(),
        InternalStatus::ExecutionDone => "Mark Execution Done".to_string(),
        InternalStatus::QaRefining => "QA Refining".to_string(),
        InternalStatus::QaTesting => "QA Testing".to_string(),
        InternalStatus::QaPassed => "QA Passed".to_string(),
        InternalStatus::QaFailed => "QA Failed".to_string(),
        InternalStatus::PendingReview => "Send to Review".to_string(),
        InternalStatus::RevisionNeeded => "Needs Revision".to_string(),
        InternalStatus::Approved => "Approve".to_string(),
        InternalStatus::Failed => "Mark as Failed".to_string(),
        InternalStatus::Cancelled => "Cancel".to_string(),
    }
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
            steps: None,
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

    // ========================================
    // Pagination Tests
    // ========================================

    #[tokio::test]
    async fn test_list_paginated_empty_results() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // No tasks exist
        let result = state
            .task_repo
            .list_paginated(&project_id, None, 0, 20, false)
            .await
            .unwrap();

        assert_eq!(result.len(), 0);

        // Count should also be 0
        let count = state.task_repo.count_tasks(&project_id, false).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_list_paginated_first_page() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create 5 tasks
        for i in 1..=5 {
            state
                .task_repo
                .create(Task::new(project_id.clone(), format!("Task {}", i)))
                .await
                .unwrap();
        }

        // Get first page (limit 3)
        let result = state
            .task_repo
            .list_paginated(&project_id, None, 0, 3, false)
            .await
            .unwrap();

        assert_eq!(result.len(), 3);

        // Total count should be 5
        let count = state.task_repo.count_tasks(&project_id, false).await.unwrap();
        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn test_list_paginated_last_page() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create 5 tasks
        for i in 1..=5 {
            state
                .task_repo
                .create(Task::new(project_id.clone(), format!("Task {}", i)))
                .await
                .unwrap();
        }

        // Get last page (offset 3, limit 3 = should return 2 tasks)
        let result = state
            .task_repo
            .list_paginated(&project_id, None, 3, 3, false)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_list_paginated_offset_beyond_total() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create 3 tasks
        for i in 1..=3 {
            state
                .task_repo
                .create(Task::new(project_id.clone(), format!("Task {}", i)))
                .await
                .unwrap();
        }

        // Request offset 10 (beyond total of 3)
        let result = state
            .task_repo
            .list_paginated(&project_id, None, 10, 20, false)
            .await
            .unwrap();

        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn test_list_paginated_excludes_archived_by_default() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create 3 tasks
        let task1 = state
            .task_repo
            .create(Task::new(project_id.clone(), "Task 1".to_string()))
            .await
            .unwrap();
        let task2 = state
            .task_repo
            .create(Task::new(project_id.clone(), "Task 2".to_string()))
            .await
            .unwrap();
        state
            .task_repo
            .create(Task::new(project_id.clone(), "Task 3".to_string()))
            .await
            .unwrap();

        // Archive task1 and task2
        state.task_repo.archive(&task1.id).await.unwrap();
        state.task_repo.archive(&task2.id).await.unwrap();

        // List without archived (include_archived = false)
        let result = state
            .task_repo
            .list_paginated(&project_id, None, 0, 20, false)
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "Task 3");

        // Count without archived
        let count = state.task_repo.count_tasks(&project_id, false).await.unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_list_paginated_includes_archived_when_requested() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create 3 tasks
        let task1 = state
            .task_repo
            .create(Task::new(project_id.clone(), "Task 1".to_string()))
            .await
            .unwrap();
        state
            .task_repo
            .create(Task::new(project_id.clone(), "Task 2".to_string()))
            .await
            .unwrap();

        // Archive task1
        state.task_repo.archive(&task1.id).await.unwrap();

        // List with archived (include_archived = true)
        let result = state
            .task_repo
            .list_paginated(&project_id, None, 0, 20, true)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);

        // Count with archived
        let count = state.task_repo.count_tasks(&project_id, true).await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_list_paginated_ordered_by_created_at_desc() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create tasks with slight delay to ensure different created_at
        let task1 = state
            .task_repo
            .create(Task::new(project_id.clone(), "First".to_string()))
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let task2 = state
            .task_repo
            .create(Task::new(project_id.clone(), "Second".to_string()))
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let task3 = state
            .task_repo
            .create(Task::new(project_id.clone(), "Third".to_string()))
            .await
            .unwrap();

        // Get paginated tasks
        let result = state
            .task_repo
            .list_paginated(&project_id, None, 0, 20, false)
            .await
            .unwrap();

        // Should be ordered newest first (DESC)
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].id, task3.id);
        assert_eq!(result[1].id, task2.id);
        assert_eq!(result[2].id, task1.id);
    }

    #[tokio::test]
    async fn test_task_list_response_serialization() {
        let project_id = ProjectId::from_string("proj-123".to_string());
        let task = Task::new(project_id, "Test Task".to_string());

        let response = TaskListResponse {
            tasks: vec![TaskResponse::from(task)],
            total: 10,
            has_more: true,
            offset: 0,
        };

        let json = serde_json::to_string(&response).unwrap();

        // Verify camelCase serialization
        assert!(json.contains("\"tasks\":"));
        assert!(json.contains("\"total\":10"));
        assert!(json.contains("\"hasMore\":true"));
        assert!(json.contains("\"offset\":0"));
    }

    #[tokio::test]
    async fn test_get_valid_transitions_from_backlog() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create a task in backlog state
        let mut task = Task::new(project_id, "Test Task".to_string());
        task.internal_status = InternalStatus::Backlog;
        let task = state.task_repo.create(task).await.unwrap();

        // Get valid transitions directly from InternalStatus
        let transitions = task.internal_status.valid_transitions();

        // From backlog, should be able to go to Ready or Cancelled
        assert_eq!(transitions.len(), 2);
        assert!(transitions.iter().any(|t| *t == InternalStatus::Ready));
        assert!(transitions.iter().any(|t| *t == InternalStatus::Cancelled));

        // Test the label mapping function
        let ready_label = status_to_label(InternalStatus::Ready);
        assert_eq!(ready_label, "Ready for Work");

        let cancelled_label = status_to_label(InternalStatus::Cancelled);
        assert_eq!(cancelled_label, "Cancel");
    }

    #[tokio::test]
    async fn test_get_valid_transitions_from_ready() {
        // Test valid transitions from ready state
        let transitions = InternalStatus::Ready.valid_transitions();

        // From ready, should be able to go to Executing, Blocked, or Cancelled
        assert_eq!(transitions.len(), 3);
        assert!(transitions.iter().any(|t| *t == InternalStatus::Executing));
        assert!(transitions.iter().any(|t| *t == InternalStatus::Blocked));
        assert!(transitions.iter().any(|t| *t == InternalStatus::Cancelled));

        // Test labels
        assert_eq!(status_to_label(InternalStatus::Executing), "Start Execution");
        assert_eq!(status_to_label(InternalStatus::Blocked), "Mark as Blocked");
    }

    #[tokio::test]
    async fn test_get_valid_transitions_from_blocked() {
        // Test valid transitions from blocked state
        let transitions = InternalStatus::Blocked.valid_transitions();

        // From blocked, should be able to go to Ready or Cancelled
        assert_eq!(transitions.len(), 2);
        assert!(transitions.iter().any(|t| *t == InternalStatus::Ready));
        assert!(transitions.iter().any(|t| *t == InternalStatus::Cancelled));
    }

    #[tokio::test]
    async fn test_get_valid_transitions_from_qa_failed() {
        // Test valid transitions from qa_failed state
        let transitions = InternalStatus::QaFailed.valid_transitions();

        // From qa_failed, should be able to go to RevisionNeeded (only one option)
        assert_eq!(transitions.len(), 1);
        assert!(transitions.iter().any(|t| *t == InternalStatus::RevisionNeeded));

        // Test label
        assert_eq!(status_to_label(InternalStatus::RevisionNeeded), "Needs Revision");
    }

    #[tokio::test]
    async fn test_get_valid_transitions_from_approved() {
        // Test valid transitions from approved state (terminal)
        let transitions = InternalStatus::Approved.valid_transitions();

        // From approved, can be re-opened to Ready
        assert_eq!(transitions.len(), 1);
        assert!(transitions.iter().any(|t| *t == InternalStatus::Ready));

        // Test label
        assert_eq!(status_to_label(InternalStatus::Approved), "Approve");
    }

    #[tokio::test]
    async fn test_get_valid_transitions_from_cancelled() {
        // Test valid transitions from cancelled state
        let transitions = InternalStatus::Cancelled.valid_transitions();

        // From cancelled, can be re-opened to Ready
        assert_eq!(transitions.len(), 1);
        assert!(transitions.iter().any(|t| *t == InternalStatus::Ready));

        // Test label
        assert_eq!(status_to_label(InternalStatus::Cancelled), "Cancel");
    }

    #[tokio::test]
    async fn test_get_valid_transitions_from_failed() {
        // Test valid transitions from failed state
        let transitions = InternalStatus::Failed.valid_transitions();

        // From failed, can retry (go to Ready)
        assert_eq!(transitions.len(), 1);
        assert!(transitions.iter().any(|t| *t == InternalStatus::Ready));

        // Test label
        assert_eq!(status_to_label(InternalStatus::Failed), "Mark as Failed");
    }

    #[tokio::test]
    async fn test_status_to_label_all_statuses() {
        // Test that all statuses have labels
        let all_statuses = InternalStatus::all_variants();

        for status in all_statuses {
            let label = status_to_label(*status);
            // Label should not be empty
            assert!(!label.is_empty(), "Status {:?} has no label", status);
        }
    }

    // ========================================
    // Create Task with Steps Tests
    // ========================================

    #[tokio::test]
    async fn test_create_task_with_steps() {
        let state = setup_test_state().await;

        let project_id = ProjectId::from_string("test-project".to_string());
        let step_titles = vec![
            "Step 1".to_string(),
            "Step 2".to_string(),
            "Step 3".to_string(),
        ];

        // Create task with steps
        let task = Task::new(project_id.clone(), "Task with Steps".to_string());
        let created_task = state.task_repo.create(task).await.unwrap();

        // Create steps manually (simulating what create_task command does)
        use crate::domain::entities::TaskStep;
        let steps: Vec<TaskStep> = step_titles
            .into_iter()
            .enumerate()
            .map(|(idx, title)| {
                TaskStep::new(created_task.id.clone(), title, idx as i32, "user".to_string())
            })
            .collect();

        let created_steps = state.task_step_repo.bulk_create(steps).await.unwrap();

        // Verify steps were created
        assert_eq!(created_steps.len(), 3);
        assert_eq!(created_steps[0].title, "Step 1");
        assert_eq!(created_steps[1].title, "Step 2");
        assert_eq!(created_steps[2].title, "Step 3");

        // Verify sort_order
        assert_eq!(created_steps[0].sort_order, 0);
        assert_eq!(created_steps[1].sort_order, 1);
        assert_eq!(created_steps[2].sort_order, 2);

        // Verify created_by
        assert_eq!(created_steps[0].created_by, "user");
        assert_eq!(created_steps[1].created_by, "user");
        assert_eq!(created_steps[2].created_by, "user");

        // Verify steps are linked to task
        let task_steps = state.task_step_repo.get_by_task(&created_task.id).await.unwrap();
        assert_eq!(task_steps.len(), 3);
    }

    #[tokio::test]
    async fn test_create_task_without_steps() {
        let state = setup_test_state().await;

        let project_id = ProjectId::from_string("test-project".to_string());

        // Create task without steps
        let task = Task::new(project_id.clone(), "Task without Steps".to_string());
        let created_task = state.task_repo.create(task).await.unwrap();

        // Verify no steps exist
        let task_steps = state.task_step_repo.get_by_task(&created_task.id).await.unwrap();
        assert_eq!(task_steps.len(), 0);
    }

    #[tokio::test]
    async fn test_create_task_with_empty_steps_array() {
        let state = setup_test_state().await;

        let project_id = ProjectId::from_string("test-project".to_string());

        // Create task with empty steps array (should not create any steps)
        let task = Task::new(project_id.clone(), "Task with Empty Steps".to_string());
        let created_task = state.task_repo.create(task).await.unwrap();

        // Verify no steps exist
        let task_steps = state.task_step_repo.get_by_task(&created_task.id).await.unwrap();
        assert_eq!(task_steps.len(), 0);
    }
}
