use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use tauri::Emitter;

use super::*;
use crate::application::{GitService, TaskSchedulerService, TaskTransitionService};
use crate::domain::entities::{
    IssueCategory, IssueSeverity, InternalStatus, Review, ReviewIssue as ReviewNoteIssue,
    ReviewIssueEntity, ReviewNote, ReviewOutcome, ReviewScopeMetadata, ReviewerType,
    ScopeDriftStatus, TaskId, TaskStepId,
};
use crate::domain::services::running_agent_registry::RunningAgentKey;
use crate::domain::tools::complete_review::ScopeDriftClassification;
use crate::domain::state_machine::services::TaskScheduler;
use crate::domain::state_machine::transition_handler::{
    deferred_merge_cleanup, set_no_code_changes_metadata, set_pending_cleanup_metadata,
};
use crate::domain::tools::complete_review::ReviewToolOutcome;
use crate::http_server::handlers::session_linking::create_child_session_impl;
use crate::http_server::helpers::get_task_context_impl;
use crate::http_server::project_scope::{ProjectScope, ProjectScopeGuard};
use crate::http_server::types::{CreateChildSessionRequest, ReviewIssueRequest};
use std::sync::Arc;

pub async fn complete_review(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Json(req): Json<CompleteReviewRequest>,
) -> Result<Json<CompleteReviewResponse>, (StatusCode, String)> {
    let task_id = TaskId::from_string(req.task_id);

    // 1. Get task and validate state is Reviewing
    let mut task = state
        .app_state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Task not found".to_string()))?;

    // Enforce project scope (no-op for internal requests without the header)
    task.assert_project_scope(&scope)
        .map_err(|e| (e.status, e.message.unwrap_or_default()))?;

    if task.internal_status != InternalStatus::Reviewing {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Task not in reviewing state. Current state: {}",
                task.internal_status.as_str()
            ),
        ));
    }

    let task_context = get_task_context_impl(&state.app_state, &task_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let scope_drift_classification = req
        .scope_drift_classification
        .as_deref()
        .map(parse_scope_drift_classification)
        .transpose()
        .map_err(|msg| (StatusCode::BAD_REQUEST, msg))?;
    let prior_review_notes = state
        .app_state
        .review_repo
        .get_notes_by_task_id(&task_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let revision_count = prior_review_notes
        .iter()
        .filter(|note| note.outcome == ReviewOutcome::ChangesRequested)
        .count() as u32;
    let review_settings = state
        .app_state
        .review_settings_repo
        .get_settings()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if matches!(
        task_context.scope_drift_status,
        ScopeDriftStatus::ScopeExpansion
    ) && scope_drift_classification.is_none()
    {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Scope drift classification required when changed files exceed planned scope: {}",
                task_context.out_of_scope_files.join(", ")
            ),
        ));
    }

    // 2. Parse and map decision to ReviewToolOutcome
    let outcome = match req.decision.as_str() {
        "approved" => ReviewToolOutcome::Approved,
        "approved_no_changes" => ReviewToolOutcome::ApprovedNoChanges,
        "needs_changes" => ReviewToolOutcome::NeedsChanges,
        "escalate" => ReviewToolOutcome::Escalate,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!(
                    "Invalid decision: '{}'. Expected 'approved', 'approved_no_changes', \
                     'needs_changes', or 'escalate'",
                    req.decision
                ),
            ))
        }
    };

    if matches!(
        outcome,
        ReviewToolOutcome::Approved | ReviewToolOutcome::ApprovedNoChanges
    ) && matches!(
        scope_drift_classification,
        Some(ScopeDriftClassification::UnrelatedDrift)
    ) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Cannot approve task with unrelated scope drift; request changes or escalate instead"
                .to_string(),
        ));
    }

    if matches!(
        scope_drift_classification,
        Some(ScopeDriftClassification::UnrelatedDrift)
    ) {
        if matches!(outcome, ReviewToolOutcome::Escalate)
            && !review_settings.exceeded_max_revisions(revision_count)
        {
            return Err((
                StatusCode::BAD_REQUEST,
                format!(
                    "Unrelated scope drift must go back through revise while revision budget remains ({revision_count}/{max_revisions} used). Use needs_changes with structured issues first, then escalate only if repeated revise cycles fail.",
                    max_revisions = review_settings.max_revision_cycles
                ),
            ));
        }

        if matches!(outcome, ReviewToolOutcome::NeedsChanges)
            && req.issues.as_ref().map_or(true, |issues| issues.is_empty())
        {
            return Err((
                StatusCode::BAD_REQUEST,
                "Needs-changes for unrelated scope drift requires at least one structured issue so the worker can revise the branch cleanly.".to_string(),
            ));
        }
    }

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
        // Phase 3 will implement the full approved_no_changes path (skip merge pipeline)
        ReviewToolOutcome::ApprovedNoChanges => ReviewOutcome::ApprovedNoChanges,
        ReviewToolOutcome::NeedsChanges => ReviewOutcome::ChangesRequested,
        ReviewToolOutcome::Escalate => ReviewOutcome::Rejected,
    };

    // Update review status
    match outcome {
        ReviewToolOutcome::Approved => {
            review.approve(feedback.clone());
        }
        // Phase 3 will implement the full approved_no_changes path (skip merge pipeline)
        ReviewToolOutcome::ApprovedNoChanges => {
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

    let parsed_issues = req
        .issues
        .as_ref()
        .map(|issues| parse_review_issues(issues))
        .transpose()
        .map_err(|msg| (StatusCode::BAD_REQUEST, msg))?;

    let domain_issues = parsed_issues.as_ref().map(|issues| {
        issues
            .iter()
            .map(|issue| ReviewNoteIssue {
                severity: issue.severity.to_db_string().to_string(),
                file: issue.file_path.clone(),
                line: issue.line_number,
                description: issue
                    .description
                    .clone()
                    .unwrap_or_else(|| issue.title.clone()),
            })
            .collect()
    });

    // For now, we don't create fix tasks automatically - that can be added later
    let fix_task_id: Option<TaskId> = None;
    let followup_session_id = maybe_spawn_unrelated_drift_followup(
        &state,
        &task,
        &review,
        &task_context,
        outcome,
        scope_drift_classification,
        revision_count,
        &review_settings,
        req.summary.as_deref(),
        req.feedback.as_deref(),
        req.escalation_reason.as_deref(),
    )
    .await;

    // Create review note for history.
    // For escalations, prefer escalation_reason over generic feedback so the
    // frontend EscalatedTaskDetail can display a precise reason.
    let note_content = if matches!(outcome, ReviewToolOutcome::Escalate) {
        req.escalation_reason.clone().or_else(|| req.feedback.clone())
    } else {
        req.feedback.clone()
    };
    // Legitimate AI decision via MCP tool — agent deliberately called complete_review. Do NOT change to System.
    let mut review_note = ReviewNote::with_content(
        task_id.clone(),
        ReviewerType::Ai,
        review_outcome,
        req.summary.clone(),
        note_content,
        domain_issues,
    );
    review_note.followup_session_id = followup_session_id.clone();
    state
        .app_state
        .review_repo
        .add_note(&review_note)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    persist_review_scope_snapshot(
        &state,
        &mut task,
        &task_context,
        scope_drift_classification,
        req.scope_drift_notes.clone(),
    )
    .await?;

    if matches!(outcome, ReviewToolOutcome::NeedsChanges) {
        if let Some(issues) = parsed_issues {
            if !issues.is_empty() {
                state
                    .app_state
                    .review_issue_repo
                    .bulk_create(
                        issues
                            .into_iter()
                            .map(|issue| {
                                let mut entity = ReviewIssueEntity::new(
                                    review_note.id.clone(),
                                    task_id.clone(),
                                    issue.title,
                                    issue.severity,
                                );
                                entity.description = issue.description;
                                entity.category = issue.category;
                                entity.step_id = issue.step_id;
                                entity.no_step_reason = issue.no_step_reason;
                                entity.file_path = issue.file_path;
                                entity.line_number = issue.line_number;
                                entity.code_snippet = issue.code_snippet;
                                entity
                            })
                            .collect(),
                    )
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            }
        }
    }

    // 6. Trigger state transition via TaskTransitionService
    // Create scheduler for auto-scheduling next Ready task when this one exits Reviewing
    let scheduler_concrete = Arc::new(
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
        .with_interactive_process_registry(Arc::clone(&state.app_state.interactive_process_registry)),
    );
    scheduler_concrete.set_self_ref(Arc::clone(&scheduler_concrete) as Arc<dyn TaskScheduler>);
    let task_scheduler: Arc<dyn TaskScheduler> = scheduler_concrete;

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
    .with_task_scheduler(task_scheduler)
    .with_plan_branch_repo(Arc::clone(&state.app_state.plan_branch_repo))
    .with_interactive_process_registry(Arc::clone(&state.app_state.interactive_process_registry));

    // Early unregister: remove the review agent from running_agent_registry BEFORE triggering
    // the state transition. This prevents pre_merge_cleanup from seeing the review agent as
    // "still running" and stopping it — which would kill this very HTTP connection and cancel
    // the entire inline merge pipeline chain. The registry's unregister is idempotent:
    // process_stream_background's own unregister later becomes a no-op.
    {
        let review_key = RunningAgentKey::new("review", task_id.as_str());
        if let Some(agent_info) = state
            .app_state
            .running_agent_registry
            .get(&review_key)
            .await
        {
            let _ = state
                .app_state
                .running_agent_registry
                .unregister(&review_key, &agent_info.agent_run_id)
                .await;
            tracing::info!(
                task_id = task_id.as_str(),
                agent_run_id = %agent_info.agent_run_id,
                "Early-unregistered review agent before state transition to prevent merge self-sabotage"
            );
        }
    }

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
        ReviewToolOutcome::ApprovedNoChanges => {
            // Extract fields BEFORE transition (transition may clear these from task)
            let task_branch = task.task_branch.clone();
            let worktree_path = task.worktree_path.clone();

            let require_human = state
                .app_state
                .review_settings_repo
                .get_settings()
                .await
                .map(|s| s.require_human_review)
                .unwrap_or(false);

            // Fetch project for repo_path and working_directory (needed for git diff + cleanup)
            let project_opt = state
                .app_state
                .project_repo
                .get_by_id(&task.project_id)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            // Git diff validation safety gate (BEFORE metadata persistence).
            // If the branch has code changes, fall back to standard Approved flow.
            let has_code_changes =
                if let (Some(ref project), Some(ref branch)) = (&project_opt, &task_branch) {
                    let repo_path = std::path::Path::new(&project.working_directory);
                    let base = project.base_branch_or_default();
                    match GitService::branches_have_same_content(repo_path, branch, base).await {
                        Ok(false) => {
                            // Not same content → branch has code changes
                            tracing::warn!(
                                task_id = %task_id.as_str(),
                                branch = %branch,
                                base_branch = %base,
                                "Reviewer marked approved_no_changes but branch has code changes \
                                 — falling back to standard Approved flow"
                            );
                            true
                        }
                        Ok(true) => false, // Same content — no changes, proceed with no-changes path
                        Err(e) => {
                            // Git error — defensive: proceed with no-changes path
                            tracing::warn!(
                                task_id = %task_id.as_str(),
                                error = %e,
                                "Git diff validation failed — proceeding with \
                                 no-changes path (defensive)"
                            );
                            false
                        }
                    }
                } else {
                    // No project or no branch — proceed with no-changes path (defensive)
                    false
                };

            if has_code_changes {
                // Fall back to standard Approved flow (reviewer decision treated as regular Approved)
                let target_status = if require_human {
                    InternalStatus::ReviewPassed
                } else {
                    InternalStatus::Approved
                };
                transition_service
                    .transition_task(&task_id, target_status.clone())
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
                target_status
            } else {
                // No code changes confirmed — set metadata and skip merge pipeline.
                // Re-fetch task for a fresh mutable copy to avoid borrow conflicts.
                let mut fresh_task = state
                    .app_state
                    .task_repo
                    .get_by_id(&task_id)
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                    .ok_or_else(|| {
                        (StatusCode::NOT_FOUND, "Task not found after review bookkeeping".to_string())
                    })?;

                set_no_code_changes_metadata(&mut fresh_task);
                set_pending_cleanup_metadata(&mut fresh_task);
                fresh_task.touch();

                state
                    .app_state
                    .task_repo
                    .update(&fresh_task)
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

                let target_status = if require_human {
                    InternalStatus::ReviewPassed
                } else {
                    InternalStatus::Merged
                };

                transition_service
                    .transition_task(&task_id, target_status.clone())
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

                // Direct-to-Merged path: clear merge progress + spawn deferred cleanup
                if !require_human {
                    crate::domain::entities::merge_progress_event::clear_merge_progress(
                        task_id.as_str(),
                    );

                    let project_working_dir = project_opt
                        .as_ref()
                        .map(|p| p.working_directory.clone())
                        .unwrap_or_default();

                    tokio::spawn(deferred_merge_cleanup(
                        task_id.clone(),
                        Arc::clone(&state.app_state.task_repo),
                        project_working_dir,
                        task_branch,
                        worktree_path,
                        None,
                    ));
                }

                target_status
            }
        }
    };

    // 7. Emit events
    if let Some(app_handle) = &state.app_state.app_handle {
        let _ = app_handle.emit(
            "review:completed",
            serde_json::json!({
                "task_id": task_id.as_str(),
                "decision": req.decision,
                "new_status": new_status.as_str(),
            }),
        );
        let _ = app_handle.emit(
            "task:status_changed",
            serde_json::json!({
                "task_id": task_id.as_str(),
                "old_status": task.internal_status.as_str(),
                "new_status": new_status.as_str(),
            }),
        );
        // For direct-to-Merged (approved_no_changes, no human review gate), emit task:merged
        if new_status == InternalStatus::Merged {
            let _ = app_handle.emit(
                "task:merged",
                serde_json::json!({
                    "task_id": task_id.as_str(),
                }),
            );
        }
    }

    // 8. Notify completion signal then close stdin via IPR
    {
        use crate::application::interactive_process_registry::InteractiveProcessKey;
        let key = InteractiveProcessKey::new("review", task_id.as_str());
        if let Some(signal) = state.app_state.interactive_process_registry.get_completion_signal(&key).await {
            signal.notify_one();
        }
        if state.app_state.interactive_process_registry.remove(&key).await.is_some() {
            tracing::info!("IPR removed for reviewer on task {}", task_id.as_str());
        }
    }

    // 9. Return response
    Ok(Json(CompleteReviewResponse {
        success: true,
        message: match &followup_session_id {
            Some(session_id) => format!(
                "Review submitted successfully. Follow-up ideation session created: {session_id}"
            ),
            None => "Review submitted successfully".to_string(),
        },
        new_status: new_status.as_str().to_string(),
        fix_task_id: fix_task_id.map(|id| id.as_str().to_string()),
        followup_session_id,
    }))
}

async fn persist_review_scope_snapshot(
    state: &HttpServerState,
    task: &mut crate::domain::entities::Task,
    task_context: &crate::domain::entities::TaskContext,
    scope_drift_classification: Option<ScopeDriftClassification>,
    scope_drift_notes: Option<String>,
) -> Result<(), (StatusCode, String)> {
    let planned_paths = task_context
        .source_proposal
        .as_ref()
        .map(|proposal| proposal.affected_paths.clone())
        .unwrap_or_default();

    let updated_metadata = if planned_paths.is_empty() {
        ReviewScopeMetadata::clear_from_task_metadata(task.metadata.as_deref())
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else {
        let review_scope = ReviewScopeMetadata::new(
            planned_paths,
            task_context.out_of_scope_files.clone(),
            scope_drift_classification.map(scope_drift_classification_to_str),
            scope_drift_notes,
        );
        Some(
            review_scope
                .update_task_metadata(task.metadata.as_deref())
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
        )
    };

    task.metadata = updated_metadata;
    state
        .app_state
        .task_repo
        .update(task)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(())
}

fn parse_scope_drift_classification(
    value: &str,
) -> Result<ScopeDriftClassification, String> {
    match value {
        "adjacent_scope_expansion" => Ok(ScopeDriftClassification::AdjacentScopeExpansion),
        "plan_correction" => Ok(ScopeDriftClassification::PlanCorrection),
        "unrelated_drift" => Ok(ScopeDriftClassification::UnrelatedDrift),
        other => Err(format!(
            "Invalid scope_drift_classification: '{}'. Expected 'adjacent_scope_expansion', 'plan_correction', or 'unrelated_drift'",
            other
        )),
    }
}

fn scope_drift_classification_to_str(
    classification: ScopeDriftClassification,
) -> String {
    match classification {
        ScopeDriftClassification::AdjacentScopeExpansion => {
            "adjacent_scope_expansion".to_string()
        }
        ScopeDriftClassification::PlanCorrection => "plan_correction".to_string(),
        ScopeDriftClassification::UnrelatedDrift => "unrelated_drift".to_string(),
    }
}

async fn maybe_spawn_unrelated_drift_followup(
    state: &HttpServerState,
    task: &crate::domain::entities::Task,
    review: &Review,
    task_context: &crate::domain::entities::TaskContext,
    outcome: ReviewToolOutcome,
    scope_drift_classification: Option<ScopeDriftClassification>,
    revision_count: u32,
    review_settings: &crate::domain::review::ReviewSettings,
    summary: Option<&str>,
    feedback: Option<&str>,
    escalation_reason: Option<&str>,
) -> Option<String> {
    if !matches!(outcome, ReviewToolOutcome::Escalate) {
        return None;
    }
    if !matches!(
        scope_drift_classification,
        Some(ScopeDriftClassification::UnrelatedDrift)
    ) {
        return None;
    }
    if !review_settings.exceeded_max_revisions(revision_count) {
        return None;
    }

    let parent_session_id = match task.ideation_session_id.clone() {
        Some(session_id) => session_id,
        None => {
            tracing::warn!(
                task_id = %task.id.as_str(),
                "Cannot auto-create follow-up session for unrelated drift: task has no ideation session"
            );
            return None;
        }
    };

    match find_existing_unrelated_drift_followup(state, &parent_session_id, &task.id).await {
        Ok(Some(existing_id)) => return Some(existing_id),
        Ok(None) => {}
        Err(e) => {
            tracing::warn!(
                task_id = %task.id.as_str(),
                error = %e,
                "Failed to check for existing unrelated-drift follow-up session"
            );
        }
    }

    let title = format!("Follow-up: {}", task.title);
    let prompt = build_unrelated_drift_followup_prompt(
        task,
        task_context,
        summary,
        feedback,
        escalation_reason,
        revision_count,
        review_settings.max_revision_cycles,
    );

    let request = CreateChildSessionRequest {
        parent_session_id: parent_session_id.as_str().to_string(),
        title: Some(title),
        description: Some(
            "Separate follow-up spawned automatically because repeated revise cycles could not resolve unrelated scope drift in the original task."
                .to_string(),
        ),
        inherit_context: true,
        initial_prompt: Some(prompt),
        team_mode: None,
        team_config: None,
        purpose: Some("general".to_string()),
        is_external_trigger: false,
        source_task_id: Some(task.id.as_str().to_string()),
        source_context_type: Some("review".to_string()),
        source_context_id: Some(review.id.as_str().to_string()),
        spawn_reason: Some("out_of_scope_failure".to_string()),
    };

    match create_child_session_impl(state, request).await {
        Ok(response) => Some(response.session_id),
        Err((status, body)) => {
            tracing::warn!(
                task_id = %task.id.as_str(),
                status = %status,
                body = %body.0,
                "Failed to auto-create follow-up session for unrelated scope drift"
            );
            None
        }
    }
}

async fn find_existing_unrelated_drift_followup(
    state: &HttpServerState,
    parent_session_id: &crate::domain::entities::IdeationSessionId,
    task_id: &TaskId,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let children = state
        .app_state
        .ideation_session_repo
        .get_children(parent_session_id)
        .await?;

    Ok(children
        .into_iter()
        .find(|session| {
            session.archived_at.is_none()
                && session.source_task_id.as_ref() == Some(task_id)
                && session.source_context_type.as_deref() == Some("review")
                && session.spawn_reason.as_deref() == Some("out_of_scope_failure")
        })
        .map(|session| session.id.as_str().to_string()))
}

fn build_unrelated_drift_followup_prompt(
    task: &crate::domain::entities::Task,
    task_context: &crate::domain::entities::TaskContext,
    summary: Option<&str>,
    feedback: Option<&str>,
    escalation_reason: Option<&str>,
    revision_count: u32,
    max_revision_cycles: u32,
) -> String {
    let planned_paths = task_context
        .source_proposal
        .as_ref()
        .map(|proposal| proposal.affected_paths.clone())
        .unwrap_or_default();

    format!(
        "This ideation follow-up was spawned automatically from AI review because task '{title}' \
could not be kept within scope after {revision_count}/{max_revision_cycles} revise cycles.\n\n\
Source task id: {task_id}\n\
Reason: unrelated out-of-scope drift blocked clean approval and merge.\n\
Review summary: {summary}\n\
Review feedback: {feedback}\n\
Escalation reason: {escalation_reason}\n\
Planned scope: {planned_scope}\n\
Out-of-scope files: {out_of_scope}\n\
Actual changed files: {actual_changed}\n\n\
Your job is to create isolated follow-up work that addresses the blocker separately from the \
original accepted session. Do not mutate the accepted parent session; instead propose standalone \
follow-up tasks for the unrelated work needed to resolve this blocker cleanly.",
        title = task.title,
        revision_count = revision_count,
        max_revision_cycles = max_revision_cycles,
        task_id = task.id.as_str(),
        summary = summary.unwrap_or("(none)"),
        feedback = feedback.unwrap_or("(none)"),
        escalation_reason = escalation_reason.unwrap_or("(none)"),
        planned_scope = if planned_paths.is_empty() {
            "(none recorded)".to_string()
        } else {
            planned_paths.join(", ")
        },
        out_of_scope = if task_context.out_of_scope_files.is_empty() {
            "(none)".to_string()
        } else {
            task_context.out_of_scope_files.join(", ")
        },
        actual_changed = if task_context.actual_changed_files.is_empty() {
            "(none)".to_string()
        } else {
            task_context.actual_changed_files.join(", ")
        },
    )
}

const DEFAULT_REVIEW_ISSUE_TITLE: &str = "Review issue";
const DEFAULT_NO_STEP_REASON: &str =
    "Reviewer did not associate this issue with a specific task step";

#[derive(Debug, Clone)]
struct ParsedReviewIssue {
    title: String,
    description: Option<String>,
    severity: IssueSeverity,
    category: Option<IssueCategory>,
    step_id: Option<TaskStepId>,
    no_step_reason: Option<String>,
    file_path: Option<String>,
    line_number: Option<i32>,
    code_snippet: Option<String>,
}

fn parse_review_issues(issues: &[ReviewIssueRequest]) -> Result<Vec<ParsedReviewIssue>, String> {
    issues.iter().map(parse_review_issue).collect()
}

fn parse_review_issue(issue: &ReviewIssueRequest) -> Result<ParsedReviewIssue, String> {
    let severity = IssueSeverity::from_db_string(&issue.severity).map_err(|_| {
        format!(
            "Invalid issue severity: '{}'. Expected 'critical', 'major', 'minor', or 'suggestion'",
            issue.severity
        )
    })?;
    let category = issue
        .category
        .as_deref()
        .map(IssueCategory::from_db_string)
        .transpose()
        .map_err(|_| {
            format!(
                "Invalid issue category: '{}'. Expected 'bug', 'missing', 'quality', or 'design'",
                issue.category.as_deref().unwrap_or_default()
            )
        })?;
    let step_id = issue.step_id.as_deref().map(TaskStepId::from_string);
    let title = issue
        .title
        .clone()
        .or_else(|| issue.description.clone())
        .unwrap_or_else(|| DEFAULT_REVIEW_ISSUE_TITLE.to_string());
    let no_step_reason = match (&step_id, &issue.no_step_reason) {
        (Some(_), _) => None,
        (None, Some(reason)) if !reason.trim().is_empty() => Some(reason.clone()),
        (None, _) => Some(DEFAULT_NO_STEP_REASON.to_string()),
    };

    Ok(ParsedReviewIssue {
        title,
        description: issue.description.clone(),
        severity,
        category,
        step_id,
        no_step_reason,
        file_path: issue.file_path.clone(),
        line_number: issue.line_number.map(|line| line as i32),
        code_snippet: issue.code_snippet.clone(),
    })
}

pub async fn get_review_notes(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Path(task_id): Path<String>,
) -> Result<Json<ReviewNotesResponse>, (StatusCode, String)> {
    let task_id = TaskId::from_string(task_id);

    // Load task to enforce project scope before returning review notes
    let task = state
        .app_state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Task not found".to_string()))?;
    task.assert_project_scope(&scope)
        .map_err(|e| (e.status, e.message.unwrap_or_default()))?;

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
                followup_session_id: note.followup_session_id,
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
