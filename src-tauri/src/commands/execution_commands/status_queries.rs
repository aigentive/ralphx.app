use super::*;

/// Get current execution status
/// Phase 82: Optional project_id for per-project scoping.
/// If project_id is None, falls back to active project or aggregates across all projects.
#[tauri::command]
pub async fn get_execution_status(
    project_id: Option<String>,
    active_project_state: State<'_, Arc<ActiveProjectState>>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app_state: State<'_, AppState>,
) -> Result<ExecutionStatusResponse, String> {
    // Sync runtime quota with persisted project settings before returning status
    let project_id = project_id.map(|id| ProjectId::from_string(id));
    let (effective_project_id, _max_concurrent) = sync_quota_from_project(
        project_id,
        &active_project_state,
        &execution_state,
        &app_state,
    )
    .await?;

    // Count queued tasks (tasks in Ready status)
    let mut queued_count = 0u32;

    if let Some(pid) = &effective_project_id {
        // Scoped to single project
        let tasks = app_state
            .task_repo
            .get_by_project(pid)
            .await
            .map_err(|e| e.to_string())?;

        queued_count = tasks
            .iter()
            .filter(|t| t.internal_status == InternalStatus::Ready)
            .count() as u32;
    } else {
        // Aggregate across all projects
        let all_projects = app_state
            .project_repo
            .get_all()
            .await
            .map_err(|e| e.to_string())?;

        for project in &all_projects {
            let tasks = app_state
                .task_repo
                .get_by_project(&project.id)
                .await
                .map_err(|e| e.to_string())?;

            queued_count += tasks
                .iter()
                .filter(|t| t.internal_status == InternalStatus::Ready)
                .count() as u32;
        }
    }

    let queued_message_count =
        count_slot_consuming_queued_messages(effective_project_id.as_ref(), &app_state).await?;

    // Runtime GC pass to prune stale rows on every status poll.
    prune_stale_execution_registry_entries(&app_state, &execution_state).await;

    let registry_entries = app_state.running_agent_registry.list_all().await;

    // Keep execution state synchronized to global execution contexts.
    // Subtract idle interactive slots (processes alive between turns that already
    // freed their execution slot via TurnComplete) to avoid re-inflating the counter.
    let mut total_with_slot = 0usize;
    for (key, _) in &registry_entries {
        let context_type = match ChatContextType::from_str(&key.context_type) {
            Ok(value) => value,
            Err(_) => continue,
        };

        if !uses_execution_slot(context_type) {
            continue;
        }

        if matches!(context_type, ChatContextType::Ideation) {
            total_with_slot += 1;
            continue;
        }

        let task_id = TaskId::from_string(key.context_id.clone());
        let task = match app_state.task_repo.get_by_id(&task_id).await {
            Ok(Some(task)) => task,
            _ => continue,
        };

        if task.archived_at.is_some()
            || !context_matches_running_status_for_gc(context_type, task.internal_status)
        {
            continue;
        }

        total_with_slot += 1;
    }
    let active_count =
        (total_with_slot.saturating_sub(execution_state.interactive_idle_count())) as u32;
    execution_state.set_running_count(active_count);
    let global_running_count = active_count;

    let mut scoped_subjects = Vec::new();
    for (key, _) in registry_entries {
        let context_type = match ChatContextType::from_str(&key.context_type) {
            Ok(value) => value,
            Err(_) => continue,
        };

        if !uses_execution_slot(context_type) {
            continue;
        }

        // Ideation uses session IDs (not task IDs) — look up session for project filtering.
        // Track active (generating) and idle (waiting_for_input) separately.
        if matches!(context_type, ChatContextType::Ideation) {
            let session_id = IdeationSessionId::from_string(key.context_id.clone());
            let session = match app_state
                .ideation_session_repo
                .get_by_id(&session_id)
                .await
            {
                Ok(Some(s)) => s,
                _ => continue, // orphaned registry entry — skip
            };
            if let Some(pid) = &effective_project_id {
                if session.project_id != *pid {
                    continue;
                }
            }
            let slot_key = format!("{}/{}", key.context_type, key.context_id);
            scoped_subjects.push(ScopedExecutionSubject::Ideation {
                project_id: session.project_id,
                is_idle: execution_state.is_interactive_idle(&slot_key),
            });
            continue;
        }

        let task_id = TaskId::from_string(key.context_id);
        let task = match app_state.task_repo.get_by_id(&task_id).await {
            Ok(Some(task)) => task,
            _ => continue,
        };

        if task.archived_at.is_some() {
            continue;
        }

        scoped_subjects.push(ScopedExecutionSubject::Task {
            context_type,
            project_id: task.project_id,
            status: task.internal_status,
        });
    }
    let counts = count_execution_status(scoped_subjects, effective_project_id.as_ref());

    // Count sessions waiting for ideation capacity (have pending_initial_prompt set).
    let ideation_waiting = match &effective_project_id {
        Some(pid) => app_state
            .ideation_session_repo
            .count_pending_sessions_for_project(pid)
            .await
            .unwrap_or(0),
        None => 0,
    };

    let max_concurrent = execution_state.max_concurrent();
    let global_max = execution_state.global_max_concurrent();
    let halt_mode = load_execution_halt_mode(&app_state).await?;

    Ok(build_execution_status_response(ExecutionStatusInput {
        is_paused: execution_state.is_paused(),
        halt_mode: execution_halt_mode_str(halt_mode).to_string(),
        running_count: counts.running_count,
        max_concurrent,
        global_max_concurrent: global_max,
        queued_count,
        queued_message_count,
        provider_blocked: execution_state.is_provider_blocked(),
        provider_blocked_until_epoch: execution_state.provider_blocked_until_epoch(),
        total_project_active: counts.total_project_active,
        global_running_count,
        ideation_active: counts.ideation_active,
        ideation_idle: counts.ideation_idle,
        ideation_waiting,
        ideation_max_project: execution_state.project_ideation_max(),
        ideation_max_global: execution_state.global_ideation_max(),
    }))
}
// ========================================
// Running Processes Query
// ========================================

/// Get all currently running processes (tasks with active execution contexts)
///
/// Returns tasks found in the running agent registry (task_execution/review/merge)
/// with enriched data:
/// - Step progress via StepProgressSummary::from_steps()
/// - Elapsed time from task_state_history
/// - Trigger origin from metadata
/// - Branch name
#[tauri::command]
pub async fn get_running_processes(
    project_id: Option<String>,
    active_project_state: State<'_, Arc<ActiveProjectState>>,
    execution_state: State<'_, Arc<ExecutionState>>,
    state: State<'_, AppState>,
) -> Result<RunningProcessesResponse, String> {
    let effective_project_id = match project_id {
        Some(id) => Some(ProjectId::from_string(id)),
        None => active_project_state.get().await,
    };

    // Keep the registry clean so process rows reflect truly running agents.
    prune_stale_execution_registry_entries(&state, &execution_state).await;

    let mut processes = Vec::new();
    let mut ideation_sessions = Vec::new();
    let mut seen_task_ids = std::collections::HashSet::new();
    let mut seen_session_ids = std::collections::HashSet::new();
    let registry_entries = state.running_agent_registry.list_all().await;

    for (key, _) in registry_entries {
        let context_type = match ChatContextType::from_str(&key.context_type) {
            Ok(value) => value,
            Err(_) => continue,
        };

        // Collect ideation sessions separately
        if context_type == ChatContextType::Ideation {
            let session_id_str = key.context_id.clone();
            if !seen_session_ids.insert(session_id_str.clone()) {
                continue;
            }
            let session_id = IdeationSessionId(session_id_str.clone());
            if let Ok(Some(session)) = state.ideation_session_repo.get_by_id(&session_id).await {
                if let Some(pid) = &effective_project_id {
                    if session.project_id != *pid {
                        continue;
                    }
                }
                let now = chrono::Utc::now();
                let slot_key = format!("ideation/{}", session_id_str);
                let is_generating = !execution_state.is_interactive_idle(&slot_key);
                ideation_sessions.push(build_running_ideation_session(
                    session_id_str,
                    &session,
                    is_generating,
                    now,
                ));
            }
            continue;
        }

        // Only include task-based execution contexts in the process list
        if !matches!(
            context_type,
            ChatContextType::TaskExecution | ChatContextType::Review | ChatContextType::Merge
        ) {
            continue;
        }

        let task_id = TaskId::from_string(key.context_id);
        let task = match state.task_repo.get_by_id(&task_id).await {
            Ok(Some(task)) => task,
            _ => continue,
        };

        if let Some(pid) = &effective_project_id {
            if task.project_id != *pid {
                continue;
            }
        }

        // Extra guard against races between status transitions and registry updates.
        if !context_matches_running_status_for_gc(context_type, task.internal_status) {
            continue;
        }

        let task_id_str = task.id.as_str().to_string();
        if !seen_task_ids.insert(task_id_str.clone()) {
            continue;
        }

        // Get step progress
        let steps = state
            .task_step_repo
            .get_by_task(&task_id)
            .await
            .map_err(|e| e.to_string())?;

        let step_progress = if !steps.is_empty() {
            Some(StepProgressSummary::from_steps(&task_id, &steps))
        } else {
            None
        };

        // Get elapsed time from status history
        let history = state
            .task_repo
            .get_status_history(&task_id)
            .await
            .map_err(|e| e.to_string())?;

        let elapsed_seconds =
            elapsed_seconds_for_status(&history, task.internal_status, chrono::Utc::now());

        // Get trigger origin
        let trigger_origin = get_trigger_origin(&task);

        processes.push(build_running_process(
            &task,
            step_progress,
            elapsed_seconds,
            trigger_origin,
        ));
    }

    Ok(RunningProcessesResponse {
        processes,
        ideation_sessions,
    })
}
