// Mutation (write) handlers for task_commands module

use std::sync::Arc;
use tauri::{Emitter, State};
use crate::application::AppState;
use crate::commands::ExecutionState;
use crate::domain::entities::{InternalStatus, ProjectId, Task, TaskId};
use super::types::{
    CreateTaskInput, UpdateTaskInput, InjectTaskInput, AnswerUserQuestionInput,
    TaskResponse, InjectTaskResponse, AnswerUserQuestionResponse
};
use super::helpers::{emit_queue_changed, emit_task_lifecycle_event};

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
    task_id: String,
    input: UpdateTaskInput,
    state: State<'_, AppState>,
) -> Result<TaskResponse, String> {
    let task_id = TaskId::from_string(task_id);

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
pub async fn move_task(
    task_id: String,
    to_status: String,
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app: tauri::AppHandle,
) -> Result<TaskResponse, String> {
    use crate::application::{TaskSchedulerService, TaskTransitionService};
    use crate::domain::state_machine::services::TaskScheduler;

    tracing::info!(task_id = %task_id, to_status = %to_status, "move_task command invoked");

    let task_id = TaskId::from_string(task_id);

    // Parse the target status
    let new_status: InternalStatus = to_status
        .parse()
        .map_err(|_| format!("Invalid status: {}", to_status))?;

    // Get the old task to know its current status before transition
    let old_task = state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Task not found: {}", task_id.as_str()))?;

    let old_status = old_task.internal_status;
    let project_id = old_task.project_id.clone();

    // Create the task scheduler for auto-scheduling Ready tasks
    let scheduler_concrete = Arc::new(TaskSchedulerService::new(
        Arc::clone(&execution_state),
        Arc::clone(&state.project_repo),
        Arc::clone(&state.task_repo),
        Arc::clone(&state.task_dependency_repo),
        Arc::clone(&state.chat_message_repo),
        Arc::clone(&state.chat_conversation_repo),
        Arc::clone(&state.agent_run_repo),
        Arc::clone(&state.ideation_session_repo),
        Arc::clone(&state.activity_event_repo),
        Arc::clone(&state.message_queue),
        Arc::clone(&state.running_agent_registry),
        Some(app.clone()),
    ).with_plan_branch_repo(Arc::clone(&state.plan_branch_repo)));
    scheduler_concrete.set_self_ref(Arc::clone(&scheduler_concrete) as Arc<dyn TaskScheduler>);
    let task_scheduler: Arc<dyn TaskScheduler> = scheduler_concrete;

    // Create the transition service with all required dependencies
    let transition_service = TaskTransitionService::new(
        Arc::clone(&state.task_repo),
        Arc::clone(&state.task_dependency_repo),
        Arc::clone(&state.project_repo),
        Arc::clone(&state.chat_message_repo),
        Arc::clone(&state.chat_conversation_repo),
        Arc::clone(&state.agent_run_repo),
        Arc::clone(&state.ideation_session_repo),
        Arc::clone(&state.activity_event_repo),
        Arc::clone(&state.message_queue),
        Arc::clone(&state.running_agent_registry),
        Arc::clone(&execution_state),
        Some(app.clone()),
    )
    .with_task_scheduler(task_scheduler)
    .with_plan_branch_repo(Arc::clone(&state.plan_branch_repo));

    // Transition the task - this triggers entry actions like spawning workers!
    let task = transition_service
        .transition_task(&task_id, new_status)
        .await
        .map_err(|e| e.to_string())?;

    // Emit queue_changed event if the move affects Ready status
    if old_status == InternalStatus::Ready || new_status == InternalStatus::Ready {
        emit_queue_changed(&state, &project_id, &app).await;
    }

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
        // Emit queue_changed since we're adding a task to Ready status
        emit_queue_changed(&state, &project_id, &app).await;
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
    app: tauri::AppHandle,
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

    let project_id = task.project_id.clone();

    // Transition to Ready status (per state machine: Blocked -> Ready)
    task.internal_status = InternalStatus::Ready;
    task.touch();

    // Persist the update
    state
        .task_repo
        .update(&task)
        .await
        .map_err(|e| e.to_string())?;

    // Emit queue_changed since we're transitioning a task to Ready status
    emit_queue_changed(&state, &project_id, &app).await;

    // Note: The answer data (selected_options, custom_response) is not persisted to the database.
    // The frontend passes answers directly to the agent via the MCP protocol when resuming execution.
    // This keeps the backend stateless and avoids coupling task state to agent communication details.

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
    emit_task_lifecycle_event(
        &app,
        "task:archived",
        archived_task.id.as_str(),
        archived_task.project_id.as_str(),
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
    emit_task_lifecycle_event(
        &app,
        "task:restored",
        restored_task.id.as_str(),
        restored_task.project_id.as_str(),
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
    emit_task_lifecycle_event(&app, "task:deleted", &task_id, &project_id);

    Ok(())
}

/// Block a task with an optional reason
///
/// Transitions the task to Blocked status and optionally records why it's blocked.
/// The blocked reason is displayed on the task card and can help track dependencies
/// or external blockers.
///
/// # Arguments
/// * `task_id` - The task ID to block
/// * `reason` - Optional reason why the task is blocked
/// * `app` - Tauri app handle for event emission
///
/// # Returns
/// * `TaskResponse` - The blocked task with updated status and reason
///
/// # Errors
/// * Task not found
/// * Invalid state transition (task cannot transition to Blocked from current status)
#[tauri::command]
pub async fn block_task(
    task_id: String,
    reason: Option<String>,
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app: tauri::AppHandle,
) -> Result<TaskResponse, String> {
    use crate::application::{TaskSchedulerService, TaskTransitionService};
    use crate::domain::state_machine::services::TaskScheduler;

    tracing::info!(task_id = %task_id, reason = ?reason, "block_task command invoked");

    let task_id_obj = TaskId::from_string(task_id.clone());

    // Get the task first to capture project_id for events
    let task = state
        .task_repo
        .get_by_id(&task_id_obj)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Task not found: {}", task_id))?;

    let project_id = task.project_id.clone();

    // Create the task scheduler for auto-scheduling Ready tasks
    let scheduler_concrete = Arc::new(TaskSchedulerService::new(
        Arc::clone(&execution_state),
        Arc::clone(&state.project_repo),
        Arc::clone(&state.task_repo),
        Arc::clone(&state.task_dependency_repo),
        Arc::clone(&state.chat_message_repo),
        Arc::clone(&state.chat_conversation_repo),
        Arc::clone(&state.agent_run_repo),
        Arc::clone(&state.ideation_session_repo),
        Arc::clone(&state.activity_event_repo),
        Arc::clone(&state.message_queue),
        Arc::clone(&state.running_agent_registry),
        Some(app.clone()),
    ).with_plan_branch_repo(Arc::clone(&state.plan_branch_repo)));
    scheduler_concrete.set_self_ref(Arc::clone(&scheduler_concrete) as Arc<dyn TaskScheduler>);
    let task_scheduler: Arc<dyn TaskScheduler> = scheduler_concrete;

    // Create the transition service
    let transition_service = TaskTransitionService::new(
        Arc::clone(&state.task_repo),
        Arc::clone(&state.task_dependency_repo),
        Arc::clone(&state.project_repo),
        Arc::clone(&state.chat_message_repo),
        Arc::clone(&state.chat_conversation_repo),
        Arc::clone(&state.agent_run_repo),
        Arc::clone(&state.ideation_session_repo),
        Arc::clone(&state.activity_event_repo),
        Arc::clone(&state.message_queue),
        Arc::clone(&state.running_agent_registry),
        Arc::clone(&execution_state),
        Some(app.clone()),
    )
    .with_task_scheduler(task_scheduler)
    .with_plan_branch_repo(Arc::clone(&state.plan_branch_repo));

    // Transition to Blocked status
    let mut blocked_task = transition_service
        .transition_task(&task_id_obj, InternalStatus::Blocked)
        .await
        .map_err(|e| e.to_string())?;

    // Set the blocked reason (must update separately after transition)
    blocked_task.blocked_reason = reason;
    blocked_task.touch();

    state
        .task_repo
        .update(&blocked_task)
        .await
        .map_err(|e| e.to_string())?;

    // Emit queue_changed since the task was likely in Ready status
    emit_queue_changed(&state, &project_id, &app).await;

    Ok(TaskResponse::from(blocked_task))
}

/// Unblock a task
///
/// Transitions the task from Blocked to Ready status and clears the blocked reason.
///
/// # Arguments
/// * `task_id` - The task ID to unblock
/// * `app` - Tauri app handle for event emission
///
/// # Returns
/// * `TaskResponse` - The unblocked task with Ready status
///
/// # Errors
/// * Task not found
/// * Invalid state transition (task must be in Blocked status)
#[tauri::command]
pub async fn unblock_task(
    task_id: String,
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app: tauri::AppHandle,
) -> Result<TaskResponse, String> {
    use crate::application::{TaskSchedulerService, TaskTransitionService};
    use crate::domain::state_machine::services::TaskScheduler;

    tracing::info!(task_id = %task_id, "unblock_task command invoked");

    let task_id_obj = TaskId::from_string(task_id.clone());

    // Get the task first to verify it's blocked and capture project_id
    let task = state
        .task_repo
        .get_by_id(&task_id_obj)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Task not found: {}", task_id))?;

    if task.internal_status != InternalStatus::Blocked {
        return Err(format!(
            "Task {} is not in Blocked status (current: {}). Cannot unblock.",
            task_id,
            task.internal_status
        ));
    }

    let project_id = task.project_id.clone();

    // Create the task scheduler for auto-scheduling Ready tasks
    let scheduler_concrete = Arc::new(TaskSchedulerService::new(
        Arc::clone(&execution_state),
        Arc::clone(&state.project_repo),
        Arc::clone(&state.task_repo),
        Arc::clone(&state.task_dependency_repo),
        Arc::clone(&state.chat_message_repo),
        Arc::clone(&state.chat_conversation_repo),
        Arc::clone(&state.agent_run_repo),
        Arc::clone(&state.ideation_session_repo),
        Arc::clone(&state.activity_event_repo),
        Arc::clone(&state.message_queue),
        Arc::clone(&state.running_agent_registry),
        Some(app.clone()),
    ).with_plan_branch_repo(Arc::clone(&state.plan_branch_repo)));
    scheduler_concrete.set_self_ref(Arc::clone(&scheduler_concrete) as Arc<dyn TaskScheduler>);
    let task_scheduler: Arc<dyn TaskScheduler> = scheduler_concrete;

    // Create the transition service
    let transition_service = TaskTransitionService::new(
        Arc::clone(&state.task_repo),
        Arc::clone(&state.task_dependency_repo),
        Arc::clone(&state.project_repo),
        Arc::clone(&state.chat_message_repo),
        Arc::clone(&state.chat_conversation_repo),
        Arc::clone(&state.agent_run_repo),
        Arc::clone(&state.ideation_session_repo),
        Arc::clone(&state.activity_event_repo),
        Arc::clone(&state.message_queue),
        Arc::clone(&state.running_agent_registry),
        Arc::clone(&execution_state),
        Some(app.clone()),
    )
    .with_task_scheduler(task_scheduler)
    .with_plan_branch_repo(Arc::clone(&state.plan_branch_repo));

    // Transition to Ready status
    let mut unblocked_task = transition_service
        .transition_task(&task_id_obj, InternalStatus::Ready)
        .await
        .map_err(|e| e.to_string())?;

    // Clear the blocked reason
    unblocked_task.blocked_reason = None;
    unblocked_task.touch();

    state
        .task_repo
        .update(&unblocked_task)
        .await
        .map_err(|e| e.to_string())?;

    // Emit queue_changed since we're adding a task to Ready status
    emit_queue_changed(&state, &project_id, &app).await;

    Ok(TaskResponse::from(unblocked_task))
}
