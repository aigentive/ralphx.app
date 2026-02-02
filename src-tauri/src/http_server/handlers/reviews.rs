use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use tauri::Emitter;

use super::*;
use crate::application::{TaskTransitionService, TaskSchedulerService};
use crate::domain::state_machine::services::TaskScheduler;
use crate::domain::entities::{
    InternalStatus, Review, ReviewIssue, ReviewNote, ReviewOutcome, ReviewerType, TaskId,
};
use crate::domain::tools::complete_review::ReviewToolOutcome;
use std::sync::Arc;

pub async fn complete_review(
    State(state): State<HttpServerState>,
    Json(req): Json<CompleteReviewRequest>,
) -> Result<Json<CompleteReviewResponse>, (StatusCode, String)> {
    let task_id = TaskId::from_string(req.task_id);

    // 1. Get task and validate state is Reviewing
    let task = state
        .app_state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Task not found".to_string()))?;

    if task.internal_status != InternalStatus::Reviewing {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Task not in reviewing state. Current state: {}", task.internal_status.as_str()),
        ));
    }

    // 2. Parse and map decision to ReviewToolOutcome
    let outcome = match req.decision.as_str() {
        "approved" => ReviewToolOutcome::Approved,
        "needs_changes" => ReviewToolOutcome::NeedsChanges,
        "escalate" => ReviewToolOutcome::Escalate,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Invalid decision: '{}'. Expected 'approved', 'needs_changes', or 'escalate'", req.decision),
            ))
        }
    };

    // 3. Get feedback - stored separately from issues now
    let feedback = req.feedback.clone();

    // 4. Get or create Review record for this task
    let reviews = state
        .app_state
        .review_repo
        .get_by_task_id(&task_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Find the most recent pending review, or None if none exists
    let existing_review = reviews
        .into_iter()
        .find(|r| r.status == crate::domain::entities::ReviewStatus::Pending);

    let is_new_review = existing_review.is_none();
    let mut review = existing_review
        .unwrap_or_else(|| Review::new(task.project_id.clone(), task_id.clone(), ReviewerType::Ai));

    // 5. Process the review result based on outcome
    let review_outcome = match outcome {
        ReviewToolOutcome::Approved => ReviewOutcome::Approved,
        ReviewToolOutcome::NeedsChanges => ReviewOutcome::ChangesRequested,
        ReviewToolOutcome::Escalate => ReviewOutcome::Rejected,
    };

    // Update review status
    match outcome {
        ReviewToolOutcome::Approved => {
            review.approve(feedback.clone());
        }
        ReviewToolOutcome::NeedsChanges => {
            review.request_changes(feedback.clone().unwrap_or_default());
        }
        ReviewToolOutcome::Escalate => {
            review.reject(feedback.clone().unwrap_or_default());
        }
    }

    // Save review
    if is_new_review {
        // New review, create it
        state
            .app_state
            .review_repo
            .create(&review)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    } else {
        // Existing review, update it
        state
            .app_state
            .review_repo
            .update(&review)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // Convert issues from HTTP type to domain type
    let domain_issues = req.issues.as_ref().map(|issues| {
        issues
            .iter()
            .map(|i| ReviewIssue {
                severity: i.severity.clone(),
                file: i.file.clone(),
                line: i.line.map(|l| l as i32),
                description: i.description.clone(),
            })
            .collect()
    });

    // Create review note for history
    let review_note = ReviewNote::with_content(
        task_id.clone(),
        ReviewerType::Ai,
        review_outcome,
        req.summary.clone(),
        feedback.clone(),
        domain_issues,
    );
    state
        .app_state
        .review_repo
        .add_note(&review_note)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // For now, we don't create fix tasks automatically - that can be added later
    let fix_task_id: Option<TaskId> = None;

    // 6. Trigger state transition via TaskTransitionService
    // Create scheduler for auto-scheduling next Ready task when this one exits Reviewing
    let task_scheduler: Arc<dyn TaskScheduler> = Arc::new(TaskSchedulerService::new(
        Arc::clone(&state.execution_state),
        Arc::clone(&state.app_state.project_repo),
        Arc::clone(&state.app_state.task_repo),
        Arc::clone(&state.app_state.task_dependency_repo),
        Arc::clone(&state.app_state.chat_message_repo),
        Arc::clone(&state.app_state.chat_conversation_repo),
        Arc::clone(&state.app_state.agent_run_repo),
        Arc::clone(&state.app_state.ideation_session_repo),
        Arc::clone(&state.app_state.activity_event_repo),
        Arc::clone(&state.app_state.message_queue),
        Arc::clone(&state.app_state.running_agent_registry),
        state.app_state.app_handle.as_ref().cloned(),
    ));

    let transition_service = TaskTransitionService::new(
        Arc::clone(&state.app_state.task_repo),
        Arc::clone(&state.app_state.task_dependency_repo),
        Arc::clone(&state.app_state.project_repo),
        Arc::clone(&state.app_state.chat_message_repo),
        Arc::clone(&state.app_state.chat_conversation_repo),
        Arc::clone(&state.app_state.agent_run_repo),
        Arc::clone(&state.app_state.ideation_session_repo),
        Arc::clone(&state.app_state.activity_event_repo),
        Arc::clone(&state.app_state.message_queue),
        Arc::clone(&state.app_state.running_agent_registry),
        Arc::clone(&state.execution_state),
        state.app_state.app_handle.as_ref().cloned(),
    )
    .with_task_scheduler(task_scheduler);

    let new_status = match outcome {
        ReviewToolOutcome::Approved => {
            // Check if human review is required
            let require_human = state
                .app_state
                .review_settings_repo
                .get_settings()
                .await
                .map(|s| s.require_human_review)
                .unwrap_or(false);

            let target_status = if require_human {
                InternalStatus::ReviewPassed // Wait for human approval
            } else {
                InternalStatus::Approved // Auto-approve, skip human step
            };

            transition_service
                .transition_task(&task_id, target_status.clone())
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            target_status
        }
        ReviewToolOutcome::NeedsChanges => {
            // Needs changes: transition to RevisionNeeded (auto re-execute)
            transition_service
                .transition_task(&task_id, InternalStatus::RevisionNeeded)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            InternalStatus::RevisionNeeded
        }
        ReviewToolOutcome::Escalate => {
            // Escalate: transition to Escalated (requires human decision)
            transition_service
                .transition_task(&task_id, InternalStatus::Escalated)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            InternalStatus::Escalated
        }
    };

    // 7. Emit events
    if let Some(app_handle) = &state.app_state.app_handle {
        let _ = app_handle.emit("review:completed", serde_json::json!({
            "task_id": task_id.as_str(),
            "decision": req.decision,
            "new_status": new_status.as_str(),
        }));
        let _ = app_handle.emit("task:status_changed", serde_json::json!({
            "task_id": task_id.as_str(),
            "old_status": task.internal_status.as_str(),
            "new_status": new_status.as_str(),
        }));
    }

    // 8. Return response
    Ok(Json(CompleteReviewResponse {
        success: true,
        message: "Review submitted successfully".to_string(),
        new_status: new_status.as_str().to_string(),
        fix_task_id: fix_task_id.map(|id| id.as_str().to_string()),
    }))
}

pub async fn get_review_notes(
    State(state): State<HttpServerState>,
    Path(task_id): Path<String>,
) -> Result<Json<ReviewNotesResponse>, (StatusCode, String)> {
    let task_id = TaskId::from_string(task_id);

    // 1. Fetch all review notes for this task
    let notes = state
        .app_state
        .review_repo
        .get_notes_by_task_id(&task_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 2. Calculate revision count (count of changes_requested outcomes)
    let revision_count = notes
        .iter()
        .filter(|n| n.outcome == ReviewOutcome::ChangesRequested)
        .count() as u32;

    // 3. Get max_revisions from review settings
    let review_settings = state
        .app_state
        .review_settings_repo
        .get_settings()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let max_revisions = review_settings.max_revision_cycles;

    // 4. Convert notes to response format
    let reviews: Vec<ReviewNoteResponse> = notes
        .into_iter()
        .map(|note| {
            // Convert issues from domain type to HTTP type
            let issues = note.issues.map(|issues| {
                issues
                    .into_iter()
                    .map(|i| super::ReviewIssue {
                        severity: i.severity,
                        file: i.file,
                        line: i.line.map(|l| l as u32),
                        description: i.description,
                    })
                    .collect()
            });

            ReviewNoteResponse {
                id: note.id.as_str().to_string(),
                reviewer: note.reviewer.to_string(),
                outcome: note.outcome.to_string(),
                summary: note.summary,
                notes: note.notes,
                issues,
                created_at: note.created_at.to_rfc3339(),
            }
        })
        .collect();

    // 5. Return response
    Ok(Json(ReviewNotesResponse {
        task_id: task_id.as_str().to_string(),
        revision_count,
        max_revisions,
        reviews,
    }))
}

/// Approve a task after AI review has passed or escalated
/// Only available when task is in ReviewPassed or Escalated status
pub async fn approve_task(
    State(state): State<HttpServerState>,
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
        req.comment.unwrap_or_else(|| "Approved by user".to_string()),
    );
    state
        .app_state
        .review_repo
        .add_note(&review_note)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 3. Transition to Approved
    let transition_service = TaskTransitionService::new(
        Arc::clone(&state.app_state.task_repo),
        Arc::clone(&state.app_state.task_dependency_repo),
        Arc::clone(&state.app_state.project_repo),
        Arc::clone(&state.app_state.chat_message_repo),
        Arc::clone(&state.app_state.chat_conversation_repo),
        Arc::clone(&state.app_state.agent_run_repo),
        Arc::clone(&state.app_state.ideation_session_repo),
        Arc::clone(&state.app_state.activity_event_repo),
        Arc::clone(&state.app_state.message_queue),
        Arc::clone(&state.app_state.running_agent_registry),
        Arc::clone(&state.execution_state),
        state.app_state.app_handle.as_ref().cloned(),
    );

    transition_service
        .transition_task(&task_id, InternalStatus::Approved)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 4. Emit events
    if let Some(app_handle) = &state.app_state.app_handle {
        let _ = app_handle.emit("review:human_approved", serde_json::json!({
            "task_id": task_id.as_str(),
        }));
        let _ = app_handle.emit("task:status_changed", serde_json::json!({
            "task_id": task_id.as_str(),
            "old_status": task.internal_status.as_str(),
            "new_status": "approved",
        }));
    }

    Ok(Json(CompleteReviewResponse {
        success: true,
        message: "Task approved and complete".to_string(),
        new_status: "approved".to_string(),
        fix_task_id: None,
    }))
}

/// Request changes on a task after AI review has passed or escalated
/// Only available when task is in ReviewPassed or Escalated status
pub async fn request_task_changes(
    State(state): State<HttpServerState>,
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
        Arc::clone(&state.app_state.chat_conversation_repo),
        Arc::clone(&state.app_state.agent_run_repo),
        Arc::clone(&state.app_state.ideation_session_repo),
        Arc::clone(&state.app_state.activity_event_repo),
        Arc::clone(&state.app_state.message_queue),
        Arc::clone(&state.app_state.running_agent_registry),
        Arc::clone(&state.execution_state),
        state.app_state.app_handle.as_ref().cloned(),
    );

    transition_service
        .transition_task(&task_id, InternalStatus::RevisionNeeded)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 4. Emit events
    if let Some(app_handle) = &state.app_state.app_handle {
        let _ = app_handle.emit("review:human_changes_requested", serde_json::json!({
            "task_id": task_id.as_str(),
            "feedback": req.feedback,
        }));
        let _ = app_handle.emit("task:status_changed", serde_json::json!({
            "task_id": task_id.as_str(),
            "old_status": task.internal_status.as_str(),
            "new_status": "revision_needed",
        }));
    }

    Ok(Json(CompleteReviewResponse {
        success: true,
        message: "Changes requested. Task will be re-executed with your feedback.".to_string(),
        new_status: "revision_needed".to_string(),
        fix_task_id: None,
    }))
}
