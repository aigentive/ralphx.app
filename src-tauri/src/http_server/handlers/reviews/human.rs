use super::*;

/// Approve a task after AI review has passed or escalated
/// Only available when task is in ReviewPassed or Escalated status
pub async fn approve_task(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Json(req): Json<super::ApproveTaskRequest>,
) -> Result<Json<CompleteReviewResponse>, (StatusCode, String)> {
    let task_id = TaskId::from_string(req.task_id);

    // 1. Get task and validate state is ReviewPassed
    let task = state
        .app_state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Task not found".to_string()))?;

    // Enforce project scope (no-op for internal requests without the header)
    task.assert_project_scope(&scope)
        .map_err(|e| (e.status, e.message.unwrap_or_default()))?;

    if task.internal_status != InternalStatus::ReviewPassed
        && task.internal_status != InternalStatus::Escalated
    {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Task must be in 'review_passed' or 'escalated' status to approve. Current status: {}. \
                This tool is only available after the AI reviewer has approved or escalated the task.",
                task.internal_status.as_str()
            ),
        ));
    }

    // 2. Create a human approval review note
    let review_note = ReviewNote::with_notes(
        task_id.clone(),
        ReviewerType::Human,
        ReviewOutcome::Approved,
        req.comment
            .unwrap_or_else(|| "Approved by user".to_string()),
    );
    state
        .app_state
        .review_repo
        .add_note(&review_note)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 3. Transition to Approved
    let approve_scheduler_concrete = Arc::new(
        TaskSchedulerService::new(
            Arc::clone(&state.execution_state),
            Arc::clone(&state.app_state.project_repo),
            Arc::clone(&state.app_state.task_repo),
            Arc::clone(&state.app_state.task_dependency_repo),
            Arc::clone(&state.app_state.chat_message_repo),
            Arc::clone(&state.app_state.chat_attachment_repo),
            Arc::clone(&state.app_state.chat_conversation_repo),
            Arc::clone(&state.app_state.agent_run_repo),
            Arc::clone(&state.app_state.ideation_session_repo),
            Arc::clone(&state.app_state.activity_event_repo),
            Arc::clone(&state.app_state.message_queue),
            Arc::clone(&state.app_state.running_agent_registry),
            Arc::clone(&state.app_state.memory_event_repo),
            state.app_state.app_handle.as_ref().cloned(),
        )
        .with_execution_settings_repo(Arc::clone(&state.app_state.execution_settings_repo))
        .with_plan_branch_repo(Arc::clone(&state.app_state.plan_branch_repo))
        .with_interactive_process_registry(Arc::clone(
            &state.app_state.interactive_process_registry,
        )),
    );
    approve_scheduler_concrete
        .set_self_ref(Arc::clone(&approve_scheduler_concrete) as Arc<dyn TaskScheduler>);
    let approve_task_scheduler: Arc<dyn TaskScheduler> = approve_scheduler_concrete;

    let transition_service = TaskTransitionService::new(
        Arc::clone(&state.app_state.task_repo),
        Arc::clone(&state.app_state.task_dependency_repo),
        Arc::clone(&state.app_state.project_repo),
        Arc::clone(&state.app_state.chat_message_repo),
        Arc::clone(&state.app_state.chat_attachment_repo),
        Arc::clone(&state.app_state.chat_conversation_repo),
        Arc::clone(&state.app_state.agent_run_repo),
        Arc::clone(&state.app_state.ideation_session_repo),
        Arc::clone(&state.app_state.activity_event_repo),
        Arc::clone(&state.app_state.message_queue),
        Arc::clone(&state.app_state.running_agent_registry),
        Arc::clone(&state.execution_state),
        state.app_state.app_handle.as_ref().cloned(),
        Arc::clone(&state.app_state.memory_event_repo),
    )
    .with_execution_settings_repo(Arc::clone(&state.app_state.execution_settings_repo))
    .with_task_scheduler(approve_task_scheduler)
    .with_plan_branch_repo(Arc::clone(&state.app_state.plan_branch_repo))
    .with_interactive_process_registry(Arc::clone(&state.app_state.interactive_process_registry));

    transition_service
        .transition_task(&task_id, InternalStatus::Approved)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 4. Emit events
    if let Some(app_handle) = &state.app_state.app_handle {
        let _ = app_handle.emit(
            "review:human_approved",
            serde_json::json!({
                "task_id": task_id.as_str(),
            }),
        );
        let _ = app_handle.emit(
            "task:status_changed",
            serde_json::json!({
                "task_id": task_id.as_str(),
                "old_status": task.internal_status.as_str(),
                "new_status": "approved",
            }),
        );
    }

    Ok(Json(CompleteReviewResponse {
        success: true,
        message: "Task approved and complete".to_string(),
        new_status: "approved".to_string(),
        fix_task_id: None,
        followup_session_id: None,
    }))
}

/// Request changes on a task after AI review has passed or escalated
/// Only available when task is in ReviewPassed or Escalated status
pub async fn request_task_changes(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Json(req): Json<super::RequestTaskChangesRequest>,
) -> Result<Json<CompleteReviewResponse>, (StatusCode, String)> {
    let task_id = TaskId::from_string(req.task_id);

    // 1. Get task and validate state is ReviewPassed
    let task = state
        .app_state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Task not found".to_string()))?;

    // Enforce project scope (no-op for internal requests without the header)
    task.assert_project_scope(&scope)
        .map_err(|e| (e.status, e.message.unwrap_or_default()))?;

    if task.internal_status != InternalStatus::ReviewPassed
        && task.internal_status != InternalStatus::Escalated
    {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Task must be in 'review_passed' or 'escalated' status to request changes. Current status: {}. \
                This tool is only available after the AI reviewer has approved or escalated the task.",
                task.internal_status.as_str()
            ),
        ));
    }

    // 2. Create a human changes-requested review note
    let review_note = ReviewNote::with_notes(
        task_id.clone(),
        ReviewerType::Human,
        ReviewOutcome::ChangesRequested,
        req.feedback.clone(),
    );
    state
        .app_state
        .review_repo
        .add_note(&review_note)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 3. Transition to RevisionNeeded (will auto-trigger re-execution)
    let transition_service = TaskTransitionService::new(
        Arc::clone(&state.app_state.task_repo),
        Arc::clone(&state.app_state.task_dependency_repo),
        Arc::clone(&state.app_state.project_repo),
        Arc::clone(&state.app_state.chat_message_repo),
        Arc::clone(&state.app_state.chat_attachment_repo),
        Arc::clone(&state.app_state.chat_conversation_repo),
        Arc::clone(&state.app_state.agent_run_repo),
        Arc::clone(&state.app_state.ideation_session_repo),
        Arc::clone(&state.app_state.activity_event_repo),
        Arc::clone(&state.app_state.message_queue),
        Arc::clone(&state.app_state.running_agent_registry),
        Arc::clone(&state.execution_state),
        state.app_state.app_handle.as_ref().cloned(),
        Arc::clone(&state.app_state.memory_event_repo),
    )
    .with_execution_settings_repo(Arc::clone(&state.app_state.execution_settings_repo))
    .with_plan_branch_repo(Arc::clone(&state.app_state.plan_branch_repo))
    .with_interactive_process_registry(Arc::clone(&state.app_state.interactive_process_registry));

    transition_service
        .transition_task(&task_id, InternalStatus::RevisionNeeded)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 4. Emit events
    if let Some(app_handle) = &state.app_state.app_handle {
        let _ = app_handle.emit(
            "review:human_changes_requested",
            serde_json::json!({
                "task_id": task_id.as_str(),
                "feedback": req.feedback,
            }),
        );
        let _ = app_handle.emit(
            "task:status_changed",
            serde_json::json!({
                "task_id": task_id.as_str(),
                "old_status": task.internal_status.as_str(),
                "new_status": "revision_needed",
            }),
        );
    }

    Ok(Json(CompleteReviewResponse {
        success: true,
        message: "Changes requested. Task will be re-executed with your feedback.".to_string(),
        new_status: "revision_needed".to_string(),
        fix_task_id: None,
        followup_session_id: None,
    }))
}
