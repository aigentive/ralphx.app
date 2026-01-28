// HTTP server for MCP proxy - exposes Tauri commands via HTTP
// This allows the MCP server to call RalphX functionality via REST API

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tauri::Emitter;
use uuid::Uuid;

use crate::application::{AppState, CreateProposalOptions, PermissionDecision, UpdateProposalOptions};
use crate::commands::ExecutionState;
use crate::domain::entities::{
    Artifact, ArtifactContent, ArtifactId, ArtifactType, ArtifactSummary, IdeationSessionId, InternalStatus,
    Priority, ProjectId, Task, TaskContext, TaskId, TaskProposalId,
    TaskStep, TaskStepId, TaskStepStatus, StepProgressSummary,
};
use crate::error::{AppError, AppResult};


// ============================================================================
// Submodules
// ============================================================================

mod types;
mod helpers;

pub use types::*;
use helpers::*;

pub async fn start_http_server(app_state: Arc<AppState>, execution_state: Arc<ExecutionState>) -> AppResult<()> {
    let state = HttpServerState {
        app_state,
        execution_state,
    };

    let app = Router::new()
        // Ideation tools (orchestrator-ideation agent)
        .route("/api/create_task_proposal", post(create_task_proposal))
        .route("/api/update_task_proposal", post(update_task_proposal))
        .route("/api/delete_task_proposal", post(delete_task_proposal))
        .route("/api/add_proposal_dependency", post(add_proposal_dependency))
        // Plan artifact tools (orchestrator-ideation agent)
        .route("/api/create_plan_artifact", post(create_plan_artifact))
        .route("/api/update_plan_artifact", post(update_plan_artifact))
        .route("/api/get_plan_artifact/:artifact_id", get(get_plan_artifact))
        .route("/api/link_proposals_to_plan", post(link_proposals_to_plan))
        .route("/api/get_session_plan/:session_id", get(get_session_plan))
        // Task tools (chat-task agent)
        .route("/api/update_task", post(update_task))
        .route("/api/add_task_note", post(add_task_note))
        .route("/api/get_task_details", post(get_task_details))
        // Project tools (chat-project agent)
        .route("/api/list_tasks", post(list_tasks))
        .route("/api/suggest_task", post(suggest_task))
        // Review tools (reviewer agent)
        .route("/api/complete_review", post(complete_review))
        .route("/api/review_notes/:task_id", get(get_review_notes))
        // Worker context tools (worker agent)
        .route("/api/task_context/:task_id", get(get_task_context))
        .route("/api/artifact/:artifact_id", get(get_artifact_full))
        .route("/api/artifact/:artifact_id/version/:version", get(get_artifact_version))
        .route("/api/artifact/:artifact_id/related", get(get_related_artifacts))
        .route("/api/artifacts/search", post(search_artifacts))
        // Task step endpoints (worker agent)
        .route("/api/task_steps/:task_id", get(get_task_steps_http))
        .route("/api/start_step", post(start_step_http))
        .route("/api/complete_step", post(complete_step_http))
        .route("/api/skip_step", post(skip_step_http))
        .route("/api/fail_step", post(fail_step_http))
        .route("/api/add_step", post(add_step_http))
        .route("/api/step_progress/:task_id", get(get_step_progress_http))
        // Permission bridge endpoints
        .route("/api/permission/request", post(request_permission))
        .route("/api/permission/await/:request_id", get(await_permission))
        .route("/api/permission/resolve", post(resolve_permission))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3847")
        .await
        .map_err(|e| AppError::Infrastructure(format!("Failed to bind HTTP server to port 3847: {}", e)))?;

    tracing::info!("MCP HTTP server listening on http://127.0.0.1:3847");

    axum::serve(listener, app)
        .await
        .map_err(|e| AppError::Infrastructure(format!("HTTP server crashed: {}", e)))?;

    Ok(())
}

// ============================================================================
// Handlers - Ideation Tools
// ============================================================================

async fn create_task_proposal(
    State(state): State<HttpServerState>,
    Json(req): Json<CreateProposalRequest>,
) -> Result<Json<ProposalResponse>, StatusCode> {
    let session_id = IdeationSessionId::from_string(req.session_id);

    // Parse category
    let category = parse_category(&req.category).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Parse priority (default to Medium if not provided)
    let priority = req
        .priority
        .as_ref()
        .map(|s| parse_priority(s.as_str()))
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .unwrap_or(Priority::Medium);

    // Convert steps and acceptance criteria to JSON strings
    let steps = req
        .steps
        .map(|s| serde_json::to_string(&s).map_err(|_| StatusCode::BAD_REQUEST))
        .transpose()?;
    let acceptance_criteria = req
        .acceptance_criteria
        .map(|ac| serde_json::to_string(&ac).map_err(|_| StatusCode::BAD_REQUEST))
        .transpose()?;

    let options = CreateProposalOptions {
        title: req.title,
        description: req.description,
        category,
        suggested_priority: priority,
        steps,
        acceptance_criteria,
    };

    // Create proposal using IdeationService logic
    let proposal = create_proposal_impl(&state.app_state, session_id, options)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ProposalResponse::from(proposal)))
}

async fn update_task_proposal(
    State(state): State<HttpServerState>,
    Json(req): Json<UpdateProposalRequest>,
) -> Result<Json<ProposalResponse>, StatusCode> {
    let proposal_id = TaskProposalId::from_string(req.proposal_id);

    // Parse category if provided
    let category = req
        .category
        .as_ref()
        .map(|s| parse_category(s.as_str()))
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Parse priority if provided
    let user_priority = req
        .user_priority
        .as_ref()
        .map(|s| parse_priority(s.as_str()))
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Convert steps and acceptance criteria to JSON strings
    let steps = req
        .steps
        .map(|s| serde_json::to_string(&s).map_err(|_| StatusCode::BAD_REQUEST))
        .transpose()?;
    let acceptance_criteria = req
        .acceptance_criteria
        .map(|ac| serde_json::to_string(&ac).map_err(|_| StatusCode::BAD_REQUEST))
        .transpose()?;

    let options = UpdateProposalOptions {
        title: req.title,
        description: req.description.map(Some),
        category,
        steps: steps.map(Some),
        acceptance_criteria: acceptance_criteria.map(Some),
        user_priority,
    };

    let updated = update_proposal_impl(&state.app_state, &proposal_id, options)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ProposalResponse::from(updated)))
}

async fn delete_task_proposal(
    State(state): State<HttpServerState>,
    Json(req): Json<DeleteProposalRequest>,
) -> Result<Json<SuccessResponse>, StatusCode> {
    let proposal_id = TaskProposalId::from_string(req.proposal_id);

    state
        .app_state
        .task_proposal_repo
        .delete(&proposal_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(SuccessResponse {
        success: true,
        message: "Proposal deleted successfully".to_string(),
    }))
}

async fn add_proposal_dependency(
    State(state): State<HttpServerState>,
    Json(req): Json<AddDependencyRequest>,
) -> Result<Json<SuccessResponse>, StatusCode> {
    let proposal_id = TaskProposalId::from_string(req.proposal_id);
    let depends_on_id = TaskProposalId::from_string(req.depends_on_id);

    state
        .app_state
        .proposal_dependency_repo
        .add_dependency(&proposal_id, &depends_on_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(SuccessResponse {
        success: true,
        message: "Dependency added successfully".to_string(),
    }))
}

// ============================================================================
// Handlers - Plan Artifact Tools
// ============================================================================

/// POST /api/create_plan_artifact
///
/// Creates a Specification artifact linked to an ideation session.
/// Returns the artifact ID for future operations.
async fn create_plan_artifact(
    State(state): State<HttpServerState>,
    Json(req): Json<CreatePlanArtifactRequest>,
) -> Result<Json<ArtifactResponse>, StatusCode> {
    use crate::domain::entities::{ArtifactBucketId, ArtifactMetadata};

    let session_id = IdeationSessionId::from_string(req.session_id);

    // Verify session exists
    state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Create the specification artifact
    let bucket_id = ArtifactBucketId::from_string("prd-library");
    let artifact = Artifact {
        id: ArtifactId::new(),
        artifact_type: ArtifactType::Specification,
        name: req.title.clone(),
        content: ArtifactContent::inline(&req.content),
        metadata: ArtifactMetadata::new("orchestrator").with_version(1),
        derived_from: vec![],
        bucket_id: Some(bucket_id),
    };

    let created = state
        .app_state
        .artifact_repo
        .create(artifact)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Link artifact to session
    state
        .app_state
        .ideation_session_repo
        .update_plan_artifact_id(&session_id, Some(created.id.to_string()))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ArtifactResponse::from(created)))
}

/// POST /api/update_plan_artifact
///
/// Updates an existing plan artifact by creating a new version.
/// Also emits a proactive sync event if proposals are linked to this plan.
async fn update_plan_artifact(
    State(state): State<HttpServerState>,
    Json(req): Json<UpdatePlanArtifactRequest>,
) -> Result<Json<ArtifactResponse>, StatusCode> {
    use crate::domain::entities::{ArtifactMetadata, ArtifactRelation};

    let artifact_id = ArtifactId::from_string(req.artifact_id);

    // Get the current artifact
    let current = state
        .app_state
        .artifact_repo
        .get_by_id(&artifact_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Create a new version
    let new_version = current.metadata.version + 1;
    let new_artifact = Artifact {
        id: ArtifactId::new(),
        artifact_type: current.artifact_type,
        name: current.name.clone(),
        content: ArtifactContent::inline(&req.content),
        metadata: ArtifactMetadata::new("orchestrator").with_version(new_version),
        derived_from: vec![current.id.clone()],
        bucket_id: current.bucket_id.clone(),
    };

    // Create the new version
    let created = state
        .app_state
        .artifact_repo
        .create(new_artifact)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Add a derived_from relation
    let relation = ArtifactRelation::derived_from(created.id.clone(), current.id.clone());
    state
        .app_state
        .artifact_repo
        .add_relation(relation)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Proactive sync: Find linked proposals and emit event
    // This allows the UI to show a notification like:
    // "Plan updated. N proposals may need revision. [Review]"
    if let Ok(linked_proposals) = state
        .app_state
        .task_proposal_repo
        .get_by_plan_artifact_id(&artifact_id)
        .await
    {
        if !linked_proposals.is_empty() {
            let proposal_ids: Vec<String> = linked_proposals
                .iter()
                .map(|p| p.id.as_str().to_string())
                .collect();

            // Emit event to frontend
            if let Some(app_handle) = &state.app_state.app_handle {
                let _ = app_handle.emit(
                    "plan:proposals_may_need_update",
                    PlanProposalsSyncPayload {
                        artifact_id: created.id.as_str().to_string(),
                        previous_artifact_id: artifact_id.as_str().to_string(),
                        proposal_ids,
                        new_version,
                    },
                );
            }
        }
    }

    Ok(Json(ArtifactResponse::from(created)))
}

/// GET /api/get_plan_artifact/:artifact_id
///
/// Retrieves a plan artifact by ID.
async fn get_plan_artifact(
    State(state): State<HttpServerState>,
    Path(artifact_id): Path<String>,
) -> Result<Json<ArtifactResponse>, StatusCode> {
    let artifact_id = ArtifactId::from_string(artifact_id);

    let artifact = state
        .app_state
        .artifact_repo
        .get_by_id(&artifact_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(ArtifactResponse::from(artifact)))
}

/// POST /api/link_proposals_to_plan
///
/// Links multiple proposals to a plan artifact.
async fn link_proposals_to_plan(
    State(state): State<HttpServerState>,
    Json(req): Json<LinkProposalsToPlanRequest>,
) -> Result<Json<SuccessResponse>, StatusCode> {
    let artifact_id = ArtifactId::from_string(req.artifact_id);

    // Verify artifact exists
    let artifact = state
        .app_state
        .artifact_repo
        .get_by_id(&artifact_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Update each proposal
    for proposal_id_str in req.proposal_ids {
        let proposal_id = TaskProposalId::from_string(proposal_id_str);

        let mut proposal = state
            .app_state
            .task_proposal_repo
            .get_by_id(&proposal_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

        proposal.plan_artifact_id = Some(artifact_id.clone());
        proposal.plan_version_at_creation = Some(artifact.metadata.version);

        state
            .app_state
            .task_proposal_repo
            .update(&proposal)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(Json(SuccessResponse {
        success: true,
        message: "Proposals linked to plan successfully".to_string(),
    }))
}

/// GET /api/get_session_plan/:session_id
///
/// Retrieves the plan artifact for an ideation session.
async fn get_session_plan(
    State(state): State<HttpServerState>,
    Path(session_id): Path<String>,
) -> Result<Json<Option<ArtifactResponse>>, StatusCode> {
    let session_id = IdeationSessionId::from_string(session_id);

    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if let Some(artifact_id) = session.plan_artifact_id {
        let artifact = state
            .app_state
            .artifact_repo
            .get_by_id(&artifact_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

        Ok(Json(Some(ArtifactResponse::from(artifact))))
    } else {
        Ok(Json(None))
    }
}

// ============================================================================
// Handlers - Task Tools
// ============================================================================

async fn update_task(
    State(state): State<HttpServerState>,
    Json(req): Json<UpdateTaskRequest>,
) -> Result<Json<TaskResponse>, StatusCode> {
    let task_id = TaskId::from_string(req.task_id);

    // Get existing task
    let mut task = state
        .app_state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Apply updates
    if let Some(title) = req.title {
        task.title = title;
    }
    if let Some(description) = req.description {
        task.description = Some(description);
    }
    if let Some(priority) = req.priority {
        task.priority = priority;
    }

    // Save updated task
    state
        .app_state
        .task_repo
        .update(&task)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(task_to_response(&task)))
}

async fn add_task_note(
    State(state): State<HttpServerState>,
    Json(req): Json<AddTaskNoteRequest>,
) -> Result<Json<TaskResponse>, StatusCode> {
    let task_id = TaskId::from_string(req.task_id);

    // Get existing task
    let mut task = state
        .app_state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Add note to description (append with newline separator)
    let note_text = format!("\n\n---\n**Note:** {}", req.note);
    task.description = Some(match task.description {
        Some(existing) => format!("{}{}", existing, note_text),
        None => note_text,
    });

    // Save updated task
    state
        .app_state
        .task_repo
        .update(&task)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(task_to_response(&task)))
}

async fn get_task_details(
    State(state): State<HttpServerState>,
    Json(req): Json<GetTaskDetailsRequest>,
) -> Result<Json<TaskResponse>, StatusCode> {
    let task_id = TaskId::from_string(req.task_id);

    let task = state
        .app_state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(task_to_response(&task)))
}

// ============================================================================
// Handlers - Project Tools (chat-project agent)
// ============================================================================

async fn list_tasks(
    State(state): State<HttpServerState>,
    Json(req): Json<ListTasksRequest>,
) -> Result<Json<Vec<TaskResponse>>, StatusCode> {
    let project_id = ProjectId::from_string(req.project_id);

    let tasks = state
        .app_state
        .task_repo
        .get_by_project(&project_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Apply filters
    let filtered: Vec<Task> = tasks
        .into_iter()
        .filter(|task| {
            // Filter by status if provided
            if let Some(ref status) = req.status {
                let task_status = match task.internal_status {
                    InternalStatus::Backlog => "backlog",
                    InternalStatus::Ready => "ready",
                    InternalStatus::Executing | InternalStatus::ReExecuting => "in_progress",
                    InternalStatus::Blocked | InternalStatus::Failed => "blocked",
                    InternalStatus::PendingReview | InternalStatus::Reviewing | InternalStatus::ReviewPassed | InternalStatus::QaRefining | InternalStatus::QaTesting | InternalStatus::QaPassed | InternalStatus::QaFailed => "review",
                    InternalStatus::Approved => "done",
                    InternalStatus::Cancelled => "cancelled",
                    InternalStatus::RevisionNeeded => "in_progress",
                };
                if task_status != status.to_lowercase() {
                    return false;
                }
            }
            // Filter by category if provided
            if let Some(ref category) = req.category {
                if task.category.to_lowercase() != category.to_lowercase() {
                    return false;
                }
            }
            true
        })
        .collect();

    let responses: Vec<TaskResponse> = filtered.iter().map(task_to_response).collect();
    Ok(Json(responses))
}

async fn suggest_task(
    State(state): State<HttpServerState>,
    Json(req): Json<SuggestTaskRequest>,
) -> Result<Json<TaskResponse>, StatusCode> {
    let project_id = ProjectId::from_string(req.project_id);

    // Create a new task from the suggestion
    let mut task = Task::new_with_category(project_id, req.title, req.category.to_lowercase());
    task.description = Some(req.description);

    // Parse priority if provided
    if let Some(priority_str) = req.priority {
        task.priority = match priority_str.to_lowercase().as_str() {
            "critical" => 100,
            "high" => 75,
            "medium" => 50,
            "low" => 25,
            _ => 50, // default to medium
        };
    }

    let created_task = state
        .app_state
        .task_repo
        .create(task)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(task_to_response(&created_task)))
}

// ============================================================================
// Handlers - Review Tools
// ============================================================================

async fn complete_review(
    State(state): State<HttpServerState>,
    Json(req): Json<CompleteReviewRequest>,
) -> Result<Json<CompleteReviewResponse>, (StatusCode, String)> {
    use crate::domain::entities::{Review, ReviewerType, ReviewNote, ReviewOutcome};
    use crate::domain::tools::complete_review::ReviewToolOutcome;
    use crate::application::TaskTransitionService;

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

    // 3. Get feedback
    let feedback = req.comments.unwrap_or_else(|| "No comments provided".to_string());

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
            review.approve(Some(feedback.clone()));
        }
        ReviewToolOutcome::NeedsChanges => {
            review.request_changes(feedback.clone());
        }
        ReviewToolOutcome::Escalate => {
            review.reject(feedback.clone());
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

    // Create review note for history
    let review_note = ReviewNote::with_notes(
        task_id.clone(),
        ReviewerType::Ai,
        review_outcome,
        feedback.clone(),
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
    let transition_service = TaskTransitionService::new(
        Arc::clone(&state.app_state.task_repo),
        Arc::clone(&state.app_state.project_repo),
        Arc::clone(&state.app_state.chat_message_repo),
        Arc::clone(&state.app_state.chat_conversation_repo),
        Arc::clone(&state.app_state.agent_run_repo),
        Arc::clone(&state.app_state.ideation_session_repo),
        Arc::clone(&state.app_state.message_queue),
        Arc::clone(&state.app_state.running_agent_registry),
        Arc::clone(&state.execution_state),
        state.app_state.app_handle.as_ref().cloned(),
    );

    let new_status = match outcome {
        ReviewToolOutcome::Approved => {
            // Approved: transition to ReviewPassed (awaiting human approval)
            transition_service
                .transition_task(&task_id, InternalStatus::ReviewPassed)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            InternalStatus::ReviewPassed
        }
        ReviewToolOutcome::NeedsChanges | ReviewToolOutcome::Escalate => {
            // Needs changes or escalate: transition to RevisionNeeded
            transition_service
                .transition_task(&task_id, InternalStatus::RevisionNeeded)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            InternalStatus::RevisionNeeded
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

/// GET /api/review_notes/:task_id
///
/// Get all review feedback for a task. Returns revision history including
/// AI and human review feedback to help workers understand what needs to be fixed.
///
/// This endpoint is used by:
/// - Worker agents during re-execution to fetch previous review feedback
/// - MCP get_review_notes tool (see ralphx-plugin/ralphx-mcp-server/src/tools.ts)
///
/// Response includes:
/// - task_id: The task being reviewed
/// - revision_count: Number of times changes were requested (for tracking revision cycles)
/// - max_revisions: Maximum allowed revision cycles (default 5, configurable in future)
/// - reviews: Array of all review notes with outcome, feedback, and timestamps
async fn get_review_notes(
    State(state): State<HttpServerState>,
    Path(task_id): Path<String>,
) -> Result<Json<ReviewNotesResponse>, (StatusCode, String)> {
    use crate::domain::entities::ReviewOutcome;

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
        .map(|note| ReviewNoteResponse {
            id: note.id.as_str().to_string(),
            reviewer: note.reviewer.to_string(),
            outcome: note.outcome.to_string(),
            notes: note.notes,
            created_at: note.created_at.to_rfc3339(),
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

// ============================================================================
// Handlers - Worker Context Tools
// ============================================================================

/// GET /api/task_context/:task_id
///
/// Get rich context for a task including linked artifacts and proposals
async fn get_task_context(
    State(state): State<HttpServerState>,
    Path(task_id): Path<String>,
) -> Result<Json<TaskContext>, StatusCode> {
    let task_id = TaskId::from_string(task_id);

    // Get task context using helper function
    let context = get_task_context_impl(&state.app_state, &task_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(context))
}

/// GET /api/artifact/:artifact_id
///
/// Fetch full artifact content by ID
async fn get_artifact_full(
    State(state): State<HttpServerState>,
    Path(artifact_id): Path<String>,
) -> Result<Json<ArtifactResponse>, StatusCode> {
    let artifact_id = ArtifactId::from_string(artifact_id);

    let artifact = state
        .app_state
        .artifact_repo
        .get_by_id(&artifact_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(ArtifactResponse::from(artifact)))
}

/// GET /api/artifact/:artifact_id/version/:version
///
/// Fetch specific version of an artifact
async fn get_artifact_version(
    State(state): State<HttpServerState>,
    Path((artifact_id, version)): Path<(String, u32)>,
) -> Result<Json<ArtifactResponse>, StatusCode> {
    let artifact_id = ArtifactId::from_string(artifact_id);

    let artifact = state
        .app_state
        .artifact_repo
        .get_by_id_at_version(&artifact_id, version)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(ArtifactResponse::from(artifact)))
}

/// GET /api/artifact/:artifact_id/related
///
/// Get artifacts related to a specific artifact
async fn get_related_artifacts(
    State(state): State<HttpServerState>,
    Path(artifact_id): Path<String>,
) -> Result<Json<Vec<ArtifactSummary>>, StatusCode> {
    let artifact_id = ArtifactId::from_string(artifact_id);

    let related = state
        .app_state
        .artifact_repo
        .get_related(&artifact_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Convert to ArtifactSummary with preview
    let summaries: Vec<ArtifactSummary> = related
        .into_iter()
        .map(|artifact| {
            let content_preview = create_artifact_preview(&artifact);
            ArtifactSummary {
                id: artifact.id.clone(),
                title: artifact.name.clone(),
                artifact_type: artifact.artifact_type,
                current_version: artifact.metadata.version,
                content_preview,
            }
        })
        .collect();

    Ok(Json(summaries))
}

/// POST /api/artifacts/search
///
/// Search artifacts by query and optional type filter
async fn search_artifacts(
    State(state): State<HttpServerState>,
    Json(req): Json<SearchArtifactsRequest>,
) -> Result<Json<Vec<ArtifactSummary>>, StatusCode> {
    // For MVP, implement basic search by getting all artifacts and filtering
    // Get all artifacts (we don't have project filtering yet, so get by type)
    let all_artifacts: Vec<Artifact> = if let Some(types) = req.artifact_types {
        let mut results = Vec::new();
        for type_str in types {
            if let Ok(artifact_type) = parse_artifact_type(&type_str) {
                let artifacts = state
                    .app_state
                    .artifact_repo
                    .get_by_type(artifact_type)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                results.extend(artifacts);
            }
        }
        results
    } else {
        // If no type filter, we need to get all types
        let mut results = Vec::new();
        for artifact_type in [
            ArtifactType::Prd,
            ArtifactType::ResearchDocument,
            ArtifactType::DesignDoc,
            ArtifactType::Specification,
            ArtifactType::CodeChange,
            ArtifactType::Diff,
            ArtifactType::TestResult,
            ArtifactType::TaskSpec,
            ArtifactType::ReviewFeedback,
            ArtifactType::Approval,
            ArtifactType::Findings,
            ArtifactType::Recommendations,
            ArtifactType::Context,
            ArtifactType::PreviousWork,
            ArtifactType::ResearchBrief,
        ] {
            let artifacts = state
                .app_state
                .artifact_repo
                .get_by_type(artifact_type)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            results.extend(artifacts);
        }
        results
    };

    // Filter by query (case-insensitive search in name and content)
    let query_lower = req.query.to_lowercase();
    let filtered: Vec<Artifact> = all_artifacts
        .into_iter()
        .filter(|artifact| {
            let name_matches = artifact.name.to_lowercase().contains(&query_lower);
            let content_matches = match &artifact.content {
                ArtifactContent::Inline { text } => text.to_lowercase().contains(&query_lower),
                ArtifactContent::File { path } => path.to_lowercase().contains(&query_lower),
            };
            name_matches || content_matches
        })
        .collect();

    // Convert to ArtifactSummary
    let summaries: Vec<ArtifactSummary> = filtered
        .into_iter()
        .map(|artifact| {
            let content_preview = create_artifact_preview(&artifact);
            ArtifactSummary {
                id: artifact.id.clone(),
                title: artifact.name.clone(),
                artifact_type: artifact.artifact_type,
                current_version: artifact.metadata.version,
                content_preview,
            }
        })
        .collect();

    Ok(Json(summaries))
}

// ============================================================================
// Handlers - Permission Bridge
// ============================================================================

/// POST /api/permission/request
///
/// Called by MCP server when Claude CLI needs permission for a tool.
/// Registers the request, emits Tauri event, returns request_id.
async fn request_permission(
    State(state): State<HttpServerState>,
    Json(input): Json<PermissionRequestInput>,
) -> Json<PermissionRequestResponse> {
    let request_id = Uuid::new_v4().to_string();

    // Store pending request with metadata
    let _rx = state
        .app_state
        .permission_state
        .register(
            request_id.clone(),
            input.tool_name.clone(),
            input.tool_input.clone(),
            input.context.clone(),
        )
        .await;

    // Emit Tauri event to frontend (if app_handle is available)
    if let Some(ref app_handle) = state.app_state.app_handle {
        let _ = app_handle.emit(
            "permission:request",
            serde_json::json!({
                "request_id": request_id,
                "tool_name": input.tool_name,
                "tool_input": input.tool_input,
                "context": input.context,
            }),
        );
    }

    Json(PermissionRequestResponse { request_id })
}

/// GET /api/permission/await/:request_id
///
/// Long-poll endpoint. MCP server calls this and blocks until user decides.
/// Returns 408 on timeout (5 minutes).
async fn await_permission(
    State(state): State<HttpServerState>,
    Path(request_id): Path<String>,
) -> Result<Json<PermissionDecision>, StatusCode> {
    // Get the receiver for this request
    let mut rx = {
        let pending = state.app_state.permission_state.pending.lock().await;
        match pending.get(&request_id).map(|req| req.sender.subscribe()) {
            Some(rx) => rx,
            None => return Err(StatusCode::NOT_FOUND),
        }
    };

    // Wait for decision with 5 minute timeout
    let timeout = tokio::time::Duration::from_secs(300);
    let start = tokio::time::Instant::now();

    // Use loop to poll for changes
    loop {
        // Check if value is Some - extract and drop borrow immediately
        let maybe_decision: Option<PermissionDecision> = {
            let current = rx.borrow();
            current.clone()
        };

        if let Some(decision) = maybe_decision {
            // Clean up
            state.app_state.permission_state.remove(&request_id).await;
            return Ok(Json(decision));
        }

        // Check timeout
        if start.elapsed() >= timeout {
            state.app_state.permission_state.remove(&request_id).await;
            return Err(StatusCode::REQUEST_TIMEOUT);
        }

        // Wait for change with remaining timeout
        let remaining = timeout.saturating_sub(start.elapsed());
        match tokio::time::timeout(remaining, rx.changed()).await {
            Ok(Ok(())) => continue, // Value changed, loop again to check
            Ok(Err(_)) => {
                // Channel closed
                state.app_state.permission_state.remove(&request_id).await;
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
            Err(_) => {
                // Timeout
                state.app_state.permission_state.remove(&request_id).await;
                return Err(StatusCode::REQUEST_TIMEOUT);
            }
        }
    }
}

/// POST /api/permission/resolve
///
/// Called by frontend when user makes a decision.
async fn resolve_permission(
    State(state): State<HttpServerState>,
    Json(input): Json<ResolvePermissionInput>,
) -> StatusCode {
    let resolved = state
        .app_state
        .permission_state
        .resolve(
            &input.request_id,
            PermissionDecision {
                decision: input.decision,
                message: input.message,
            },
        )
        .await;

    if resolved {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

// ============================================================================
// Handlers - Task Step Tools (worker agent)
// ============================================================================

/// GET /api/task_steps/:task_id
///
/// Fetch all steps for a task
async fn get_task_steps_http(
    State(state): State<HttpServerState>,
    Path(task_id): Path<String>,
) -> Result<Json<Vec<StepResponse>>, StatusCode> {
    let task_id = TaskId::from_string(task_id);
    let steps = state
        .app_state
        .task_step_repo
        .get_by_task(&task_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(steps.into_iter().map(StepResponse::from).collect()))
}

/// POST /api/start_step
///
/// Mark a step as in-progress
async fn start_step_http(
    State(state): State<HttpServerState>,
    Json(req): Json<StartStepRequest>,
) -> Result<Json<StepResponse>, StatusCode> {
    let step_id = TaskStepId::from_string(req.step_id);

    // Get existing step
    let mut step = state
        .app_state
        .task_step_repo
        .get_by_id(&step_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Validate step is Pending
    if step.status != TaskStepStatus::Pending {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Update status
    step.status = TaskStepStatus::InProgress;
    step.started_at = Some(chrono::Utc::now());
    step.touch();

    // Save
    state
        .app_state
        .task_step_repo
        .update(&step)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = StepResponse::from(step);

    // Emit event to frontend
    if let Some(app_handle) = &state.app_state.app_handle {
        let _ = app_handle.emit(
            "step:updated",
            serde_json::json!({
                "step": response.clone(),
                "task_id": &response.task_id
            }),
        );
    }

    Ok(Json(response))
}

/// POST /api/complete_step
///
/// Mark a step as completed
async fn complete_step_http(
    State(state): State<HttpServerState>,
    Json(req): Json<CompleteStepRequest>,
) -> Result<Json<StepResponse>, StatusCode> {
    let step_id = TaskStepId::from_string(req.step_id);

    // Get existing step
    let mut step = state
        .app_state
        .task_step_repo
        .get_by_id(&step_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Validate step is InProgress
    if step.status != TaskStepStatus::InProgress {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Update status
    step.status = TaskStepStatus::Completed;
    step.completed_at = Some(chrono::Utc::now());
    step.completion_note = req.note;
    step.touch();

    // Save
    state
        .app_state
        .task_step_repo
        .update(&step)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = StepResponse::from(step);

    // Emit event to frontend
    if let Some(app_handle) = &state.app_state.app_handle {
        let _ = app_handle.emit(
            "step:updated",
            serde_json::json!({
                "step": response.clone(),
                "task_id": &response.task_id
            }),
        );
    }

    Ok(Json(response))
}

/// POST /api/skip_step
///
/// Mark a step as skipped
async fn skip_step_http(
    State(state): State<HttpServerState>,
    Json(req): Json<SkipStepRequest>,
) -> Result<Json<StepResponse>, StatusCode> {
    let step_id = TaskStepId::from_string(req.step_id);

    // Get existing step
    let mut step = state
        .app_state
        .task_step_repo
        .get_by_id(&step_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Validate step is Pending or InProgress
    if step.status != TaskStepStatus::Pending && step.status != TaskStepStatus::InProgress {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Update status
    step.status = TaskStepStatus::Skipped;
    step.completed_at = Some(chrono::Utc::now());
    step.completion_note = Some(req.reason);
    step.touch();

    // Save
    state
        .app_state
        .task_step_repo
        .update(&step)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = StepResponse::from(step);

    // Emit event to frontend
    if let Some(app_handle) = &state.app_state.app_handle {
        let _ = app_handle.emit(
            "step:updated",
            serde_json::json!({
                "step": response.clone(),
                "task_id": &response.task_id
            }),
        );
    }

    Ok(Json(response))
}

/// POST /api/fail_step
///
/// Mark a step as failed
async fn fail_step_http(
    State(state): State<HttpServerState>,
    Json(req): Json<FailStepRequest>,
) -> Result<Json<StepResponse>, StatusCode> {
    let step_id = TaskStepId::from_string(req.step_id);

    // Get existing step
    let mut step = state
        .app_state
        .task_step_repo
        .get_by_id(&step_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Validate step is InProgress
    if step.status != TaskStepStatus::InProgress {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Update status
    step.status = TaskStepStatus::Failed;
    step.completed_at = Some(chrono::Utc::now());
    step.completion_note = Some(req.error);
    step.touch();

    // Save
    state
        .app_state
        .task_step_repo
        .update(&step)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = StepResponse::from(step);

    // Emit event to frontend
    if let Some(app_handle) = &state.app_state.app_handle {
        let _ = app_handle.emit(
            "step:updated",
            serde_json::json!({
                "step": response.clone(),
                "task_id": &response.task_id
            }),
        );
    }

    Ok(Json(response))
}

/// POST /api/add_step
///
/// Add a new step to a task during execution
async fn add_step_http(
    State(state): State<HttpServerState>,
    Json(req): Json<AddStepRequest>,
) -> Result<Json<StepResponse>, StatusCode> {
    let task_id = TaskId::from_string(req.task_id);

    // Determine sort_order
    let sort_order = if let Some(after_step_id_str) = req.after_step_id {
        // Insert after specified step
        let after_step_id = TaskStepId::from_string(after_step_id_str);
        let after_step = state
            .app_state
            .task_step_repo
            .get_by_id(&after_step_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;
        after_step.sort_order + 1
    } else {
        // Append to end - find max sort_order
        let steps = state
            .app_state
            .task_step_repo
            .get_by_task(&task_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        steps.iter().map(|s| s.sort_order).max().unwrap_or(-1) + 1
    };

    // Create new step
    let mut step = TaskStep::new(task_id, req.title, sort_order, "agent".to_string());
    step.description = req.description;

    // Save to repository
    let step = state
        .app_state
        .task_step_repo
        .create(step)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Emit event to frontend
    if let Some(app_handle) = &state.app_state.app_handle {
        let response = StepResponse::from(step.clone());
        let _ = app_handle.emit(
            "step:created",
            serde_json::json!({
                "step": response,
                "task_id": step.task_id.as_str()
            }),
        );
    }

    Ok(Json(StepResponse::from(step)))
}

/// GET /api/step_progress/:task_id
///
/// Get progress summary for a task
async fn get_step_progress_http(
    State(state): State<HttpServerState>,
    Path(task_id): Path<String>,
) -> Result<Json<StepProgressSummary>, StatusCode> {
    let task_id = TaskId::from_string(task_id);
    let steps = state
        .app_state
        .task_step_repo
        .get_by_task(&task_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(StepProgressSummary::from_steps(&task_id, &steps)))
}


// ============================================================================
// Local Wrapper Functions
// ============================================================================

/// Convert a Task to a TaskResponse
/// This wraps the helpers version to provide a local function that matches the call sites
fn task_to_response(task: &Task) -> TaskResponse {
    TaskResponse {
        id: task.id.to_string(),
        title: task.title.clone(),
        description: task.description.clone(),
        status: format!("{:?}", task.internal_status),
        priority: task.priority.to_string(),
    }
}
