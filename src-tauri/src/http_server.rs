// HTTP server for MCP proxy - exposes Tauri commands via HTTP
// This allows the MCP server to call RalphX functionality via REST API

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::Emitter;
use uuid::Uuid;

use crate::application::{AppState, CreateProposalOptions, PermissionDecision, UpdateProposalOptions};
use crate::domain::entities::{
    Artifact, ArtifactContent, ArtifactId, ArtifactType, ArtifactSummary, IdeationSessionId, InternalStatus,
    Priority, ProjectId, Task, TaskCategory, TaskContext, TaskId, TaskProposal, TaskProposalId,
    TaskStep, TaskStepId, TaskStepStatus, StepProgressSummary,
};
use crate::error::{AppError, AppResult};

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateProposalRequest {
    pub session_id: String,
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub priority: Option<String>,
    pub steps: Option<Vec<String>>,
    pub acceptance_criteria: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProposalRequest {
    pub proposal_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub steps: Option<Vec<String>>,
    pub acceptance_criteria: Option<Vec<String>>,
    pub user_priority: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteProposalRequest {
    pub proposal_id: String,
}

#[derive(Debug, Deserialize)]
pub struct AddDependencyRequest {
    pub proposal_id: String,
    pub depends_on_id: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskRequest {
    pub task_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct AddTaskNoteRequest {
    pub task_id: String,
    pub note: String,
}

#[derive(Debug, Deserialize)]
pub struct GetTaskDetailsRequest {
    pub task_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CompleteReviewRequest {
    pub task_id: String,
    pub decision: String, // "approved" | "needs_changes" | "escalate"
    pub comments: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListTasksRequest {
    pub project_id: String,
    pub status: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SuggestTaskRequest {
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub priority: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProposalResponse {
    pub id: String,
    pub session_id: String,
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub priority: String,
    pub steps: Option<String>,
    pub acceptance_criteria: Option<String>,
    pub created_at: String,
}

impl From<TaskProposal> for ProposalResponse {
    fn from(proposal: TaskProposal) -> Self {
        Self {
            id: proposal.id.to_string(),
            session_id: proposal.session_id.to_string(),
            title: proposal.title,
            description: proposal.description,
            category: format!("{:?}", proposal.category),
            priority: format!("{:?}", proposal.suggested_priority),
            steps: proposal.steps,
            acceptance_criteria: proposal.acceptance_criteria,
            created_at: proposal.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TaskResponse {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
}

#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

// Permission request/response types
#[derive(Debug, Deserialize)]
pub struct PermissionRequestInput {
    pub tool_name: String,
    pub tool_input: serde_json::Value,
    pub context: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PermissionRequestResponse {
    pub request_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ResolvePermissionInput {
    pub request_id: String,
    pub decision: String, // "allow" or "deny"
    pub message: Option<String>,
}

// Plan artifact request/response types
#[derive(Debug, Deserialize)]
pub struct CreatePlanArtifactRequest {
    pub session_id: String,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePlanArtifactRequest {
    pub artifact_id: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct LinkProposalsToPlanRequest {
    pub proposal_ids: Vec<String>,
    pub artifact_id: String,
}

/// Payload for the plan:proposals_may_need_update event
/// Emitted when a plan artifact is updated and has linked proposals
#[derive(Debug, Clone, Serialize)]
pub struct PlanProposalsSyncPayload {
    /// The new artifact ID (new version)
    pub artifact_id: String,
    /// The previous artifact ID (the one that was updated)
    pub previous_artifact_id: String,
    /// IDs of proposals linked to the original plan
    pub proposal_ids: Vec<String>,
    /// The new version number
    pub new_version: u32,
}

#[derive(Debug, Serialize)]
pub struct ArtifactResponse {
    pub id: String,
    pub artifact_type: String,
    pub name: String,
    pub content: String,
    pub version: u32,
    pub created_at: String,
    pub created_by: String,
}

impl From<Artifact> for ArtifactResponse {
    fn from(artifact: Artifact) -> Self {
        let content = match &artifact.content {
            ArtifactContent::Inline { text } => text.clone(),
            ArtifactContent::File { path } => format!("[File: {}]", path),
        };

        Self {
            id: artifact.id.to_string(),
            artifact_type: format!("{:?}", artifact.artifact_type),
            name: artifact.name,
            content,
            version: artifact.metadata.version,
            created_at: artifact.metadata.created_at.to_rfc3339(),
            created_by: artifact.metadata.created_by.clone(),
        }
    }
}

/// Request for searching artifacts
#[derive(Debug, Deserialize)]
pub struct SearchArtifactsRequest {
    pub project_id: String,
    pub query: String,
    pub artifact_types: Option<Vec<String>>,
}

// ============================================================================
// Request/Response Types - Task Steps
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct StartStepRequest {
    pub step_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CompleteStepRequest {
    pub step_id: String,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SkipStepRequest {
    pub step_id: String,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct FailStepRequest {
    pub step_id: String,
    pub error: String,
}

#[derive(Debug, Deserialize)]
pub struct AddStepRequest {
    pub task_id: String,
    pub title: String,
    pub description: Option<String>,
    pub after_step_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StepResponse {
    pub id: String,
    pub task_id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub sort_order: i32,
    pub completion_note: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

impl From<TaskStep> for StepResponse {
    fn from(step: TaskStep) -> Self {
        Self {
            id: step.id.as_str().to_string(),
            task_id: step.task_id.as_str().to_string(),
            title: step.title,
            description: step.description,
            status: step.status.to_db_string().to_string(),
            sort_order: step.sort_order,
            completion_note: step.completion_note,
            started_at: step.started_at.map(|dt| dt.to_rfc3339()),
            completed_at: step.completed_at.map(|dt| dt.to_rfc3339()),
        }
    }
}

// ============================================================================
// HTTP Server
// ============================================================================

pub async fn start_http_server(state: Arc<AppState>) {
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
        .expect("Failed to bind HTTP server to port 3847");

    tracing::info!("MCP HTTP server listening on http://127.0.0.1:3847");

    axum::serve(listener, app)
        .await
        .expect("HTTP server crashed");
}

// ============================================================================
// Handlers - Ideation Tools
// ============================================================================

async fn create_task_proposal(
    State(state): State<Arc<AppState>>,
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
    let steps = req.steps.map(|s| serde_json::to_string(&s).unwrap());
    let acceptance_criteria = req
        .acceptance_criteria
        .map(|ac| serde_json::to_string(&ac).unwrap());

    let options = CreateProposalOptions {
        title: req.title,
        description: req.description,
        category,
        suggested_priority: priority,
        steps,
        acceptance_criteria,
    };

    // Create proposal using IdeationService logic
    let proposal = create_proposal_impl(&state, session_id, options)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ProposalResponse::from(proposal)))
}

async fn update_task_proposal(
    State(state): State<Arc<AppState>>,
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
    let steps = req.steps.map(|s| serde_json::to_string(&s).unwrap());
    let acceptance_criteria = req
        .acceptance_criteria
        .map(|ac| serde_json::to_string(&ac).unwrap());

    let options = UpdateProposalOptions {
        title: req.title,
        description: req.description.map(Some),
        category,
        steps: steps.map(Some),
        acceptance_criteria: acceptance_criteria.map(Some),
        user_priority,
    };

    let updated = update_proposal_impl(&state, &proposal_id, options)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ProposalResponse::from(updated)))
}

async fn delete_task_proposal(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DeleteProposalRequest>,
) -> Result<Json<SuccessResponse>, StatusCode> {
    let proposal_id = TaskProposalId::from_string(req.proposal_id);

    state
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
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddDependencyRequest>,
) -> Result<Json<SuccessResponse>, StatusCode> {
    let proposal_id = TaskProposalId::from_string(req.proposal_id);
    let depends_on_id = TaskProposalId::from_string(req.depends_on_id);

    state
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
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreatePlanArtifactRequest>,
) -> Result<Json<ArtifactResponse>, StatusCode> {
    use crate::domain::entities::{ArtifactBucketId, ArtifactMetadata};

    let session_id = IdeationSessionId::from_string(req.session_id);

    // Verify session exists
    state
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
        .artifact_repo
        .create(artifact)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Link artifact to session
    state
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
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdatePlanArtifactRequest>,
) -> Result<Json<ArtifactResponse>, StatusCode> {
    use crate::domain::entities::{ArtifactMetadata, ArtifactRelation};

    let artifact_id = ArtifactId::from_string(req.artifact_id);

    // Get the current artifact
    let current = state
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
        .artifact_repo
        .create(new_artifact)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Add a derived_from relation
    let relation = ArtifactRelation::derived_from(created.id.clone(), current.id.clone());
    state
        .artifact_repo
        .add_relation(relation)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Proactive sync: Find linked proposals and emit event
    // This allows the UI to show a notification like:
    // "Plan updated. N proposals may need revision. [Review]"
    if let Ok(linked_proposals) = state
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
            if let Some(app_handle) = &state.app_handle {
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
    State(state): State<Arc<AppState>>,
    Path(artifact_id): Path<String>,
) -> Result<Json<ArtifactResponse>, StatusCode> {
    let artifact_id = ArtifactId::from_string(artifact_id);

    let artifact = state
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
    State(state): State<Arc<AppState>>,
    Json(req): Json<LinkProposalsToPlanRequest>,
) -> Result<Json<SuccessResponse>, StatusCode> {
    let artifact_id = ArtifactId::from_string(req.artifact_id);

    // Verify artifact exists
    let artifact = state
        .artifact_repo
        .get_by_id(&artifact_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Update each proposal
    for proposal_id_str in req.proposal_ids {
        let proposal_id = TaskProposalId::from_string(proposal_id_str);

        let mut proposal = state
            .task_proposal_repo
            .get_by_id(&proposal_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

        proposal.plan_artifact_id = Some(artifact_id.clone());
        proposal.plan_version_at_creation = Some(artifact.metadata.version);

        state
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
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> Result<Json<Option<ArtifactResponse>>, StatusCode> {
    let session_id = IdeationSessionId::from_string(session_id);

    let session = state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if let Some(artifact_id) = session.plan_artifact_id {
        let artifact = state
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
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateTaskRequest>,
) -> Result<Json<TaskResponse>, StatusCode> {
    let task_id = TaskId::from_string(req.task_id);

    // Get existing task
    let mut task = state
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
        .task_repo
        .update(&task)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(task_to_response(&task)))
}

async fn add_task_note(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddTaskNoteRequest>,
) -> Result<Json<TaskResponse>, StatusCode> {
    let task_id = TaskId::from_string(req.task_id);

    // Get existing task
    let mut task = state
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
        .task_repo
        .update(&task)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(task_to_response(&task)))
}

async fn get_task_details(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GetTaskDetailsRequest>,
) -> Result<Json<TaskResponse>, StatusCode> {
    let task_id = TaskId::from_string(req.task_id);

    let task = state
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
    State(state): State<Arc<AppState>>,
    Json(req): Json<ListTasksRequest>,
) -> Result<Json<Vec<TaskResponse>>, StatusCode> {
    let project_id = ProjectId::from_string(req.project_id);

    let tasks = state
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
                    InternalStatus::Executing | InternalStatus::ExecutionDone | InternalStatus::ReExecuting => "in_progress",
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
    State(state): State<Arc<AppState>>,
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
    State(_state): State<Arc<AppState>>,
    Json(req): Json<CompleteReviewRequest>,
) -> Result<Json<SuccessResponse>, StatusCode> {
    let _task_id = TaskId::from_string(req.task_id);

    // Parse decision
    let _decision = match req.decision.as_str() {
        "approved" => "approved",
        "needs_changes" => "needs_changes",
        "escalate" => "escalate",
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    // TODO: Implement review submission logic
    // This will be implemented when ReviewService is updated in future tasks
    // For now, just acknowledge the review

    Ok(Json(SuccessResponse {
        success: true,
        message: "Review submitted successfully".to_string(),
    }))
}

// ============================================================================
// Handlers - Worker Context Tools
// ============================================================================

/// GET /api/task_context/:task_id
///
/// Get rich context for a task including linked artifacts and proposals
async fn get_task_context(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
) -> Result<Json<TaskContext>, StatusCode> {
    let task_id = TaskId::from_string(task_id);

    // Get task context using helper function
    let context = get_task_context_impl(&state, &task_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(context))
}

/// GET /api/artifact/:artifact_id
///
/// Fetch full artifact content by ID
async fn get_artifact_full(
    State(state): State<Arc<AppState>>,
    Path(artifact_id): Path<String>,
) -> Result<Json<ArtifactResponse>, StatusCode> {
    let artifact_id = ArtifactId::from_string(artifact_id);

    let artifact = state
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
    State(state): State<Arc<AppState>>,
    Path((artifact_id, version)): Path<(String, u32)>,
) -> Result<Json<ArtifactResponse>, StatusCode> {
    let artifact_id = ArtifactId::from_string(artifact_id);

    let artifact = state
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
    State(state): State<Arc<AppState>>,
    Path(artifact_id): Path<String>,
) -> Result<Json<Vec<ArtifactSummary>>, StatusCode> {
    let artifact_id = ArtifactId::from_string(artifact_id);

    let related = state
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
    State(state): State<Arc<AppState>>,
    Json(req): Json<SearchArtifactsRequest>,
) -> Result<Json<Vec<ArtifactSummary>>, StatusCode> {
    // For MVP, implement basic search by getting all artifacts and filtering
    // TODO: Optimize with proper database search in future iteration

    // Get all artifacts (we don't have project filtering yet, so get by type)
    let all_artifacts: Vec<Artifact> = if let Some(types) = req.artifact_types {
        let mut results = Vec::new();
        for type_str in types {
            if let Ok(artifact_type) = parse_artifact_type(&type_str) {
                let artifacts = state
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
    State(state): State<Arc<AppState>>,
    Json(input): Json<PermissionRequestInput>,
) -> Json<PermissionRequestResponse> {
    let request_id = Uuid::new_v4().to_string();

    // Store pending request with metadata
    let _rx = state
        .permission_state
        .register(
            request_id.clone(),
            input.tool_name.clone(),
            input.tool_input.clone(),
            input.context.clone(),
        )
        .await;

    // Emit Tauri event to frontend (if app_handle is available)
    if let Some(ref app_handle) = state.app_handle {
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
    State(state): State<Arc<AppState>>,
    Path(request_id): Path<String>,
) -> Result<Json<PermissionDecision>, StatusCode> {
    // Get the receiver for this request
    let mut rx = {
        let pending = state.permission_state.pending.lock().await;
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
            state.permission_state.remove(&request_id).await;
            return Ok(Json(decision));
        }

        // Check timeout
        if start.elapsed() >= timeout {
            state.permission_state.remove(&request_id).await;
            return Err(StatusCode::REQUEST_TIMEOUT);
        }

        // Wait for change with remaining timeout
        let remaining = timeout.saturating_sub(start.elapsed());
        match tokio::time::timeout(remaining, rx.changed()).await {
            Ok(Ok(())) => continue, // Value changed, loop again to check
            Ok(Err(_)) => {
                // Channel closed
                state.permission_state.remove(&request_id).await;
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
            Err(_) => {
                // Timeout
                state.permission_state.remove(&request_id).await;
                return Err(StatusCode::REQUEST_TIMEOUT);
            }
        }
    }
}

/// POST /api/permission/resolve
///
/// Called by frontend when user makes a decision.
async fn resolve_permission(
    State(state): State<Arc<AppState>>,
    Json(input): Json<ResolvePermissionInput>,
) -> StatusCode {
    let resolved = state
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
// Helper Functions
// ============================================================================

/// Create proposal (reuses IdeationService logic)
async fn create_proposal_impl(
    state: &AppState,
    session_id: IdeationSessionId,
    options: CreateProposalOptions,
) -> AppResult<TaskProposal> {
    use crate::domain::entities::{IdeationSessionStatus, TaskProposal};

    // Verify session exists and is active
    let session = state.ideation_session_repo.get_by_id(&session_id).await?;
    match session {
        None => {
            return Err(crate::error::AppError::NotFound(format!(
                "Session {} not found",
                session_id
            )))
        }
        Some(s) if s.status != IdeationSessionStatus::Active => {
            return Err(crate::error::AppError::Validation(format!(
                "Cannot add proposal to {} session",
                s.status
            )));
        }
        _ => {}
    }

    // Get current proposal count for sort_order
    let count = state.task_proposal_repo.count_by_session(&session_id).await?;

    // Create proposal
    let mut proposal = TaskProposal::new(
        session_id,
        options.title,
        options.category,
        options.suggested_priority,
    );
    proposal.description = options.description;
    proposal.steps = options.steps;
    proposal.acceptance_criteria = options.acceptance_criteria;
    proposal.sort_order = count as i32;

    // Save to database
    state.task_proposal_repo.create(proposal.clone()).await?;

    Ok(proposal)
}

/// Update proposal (reuses IdeationService logic)
async fn update_proposal_impl(
    state: &AppState,
    proposal_id: &TaskProposalId,
    options: UpdateProposalOptions,
) -> AppResult<TaskProposal> {
    // Get existing proposal
    let mut proposal = state
        .task_proposal_repo
        .get_by_id(proposal_id)
        .await?
        .ok_or_else(|| {
            crate::error::AppError::NotFound(format!("Proposal {} not found", proposal_id))
        })?;

    // Apply updates
    if let Some(title) = options.title {
        proposal.title = title;
    }
    if let Some(description) = options.description {
        proposal.description = description;
    }
    if let Some(category) = options.category {
        proposal.category = category;
    }
    if let Some(steps) = options.steps {
        proposal.steps = steps;
    }
    if let Some(acceptance_criteria) = options.acceptance_criteria {
        proposal.acceptance_criteria = acceptance_criteria;
    }
    if let Some(priority) = options.user_priority {
        proposal.user_priority = Some(priority);
    }

    // Save updated proposal
    state.task_proposal_repo.update(&proposal).await?;

    Ok(proposal)
}

// ============================================================================
// Handlers - Task Step Tools (worker agent)
// ============================================================================

/// GET /api/task_steps/:task_id
///
/// Fetch all steps for a task
async fn get_task_steps_http(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
) -> Result<Json<Vec<StepResponse>>, StatusCode> {
    let task_id = TaskId::from_string(task_id);
    let steps = state
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
    State(state): State<Arc<AppState>>,
    Json(req): Json<StartStepRequest>,
) -> Result<Json<StepResponse>, StatusCode> {
    let step_id = TaskStepId::from_string(req.step_id);

    // Get existing step
    let mut step = state
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
        .task_step_repo
        .update(&step)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Emit event to frontend
    if let Some(app_handle) = &state.app_handle {
        let response = StepResponse::from(step.clone());
        let _ = app_handle.emit(
            "step:updated",
            serde_json::json!({
                "step": response,
                "task_id": step.task_id.as_str()
            }),
        );
    }

    Ok(Json(StepResponse::from(step)))
}

/// POST /api/complete_step
///
/// Mark a step as completed
async fn complete_step_http(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CompleteStepRequest>,
) -> Result<Json<StepResponse>, StatusCode> {
    let step_id = TaskStepId::from_string(req.step_id);

    // Get existing step
    let mut step = state
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
        .task_step_repo
        .update(&step)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Emit event to frontend
    if let Some(app_handle) = &state.app_handle {
        let response = StepResponse::from(step.clone());
        let _ = app_handle.emit(
            "step:updated",
            serde_json::json!({
                "step": response,
                "task_id": step.task_id.as_str()
            }),
        );
    }

    Ok(Json(StepResponse::from(step)))
}

/// POST /api/skip_step
///
/// Mark a step as skipped
async fn skip_step_http(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SkipStepRequest>,
) -> Result<Json<StepResponse>, StatusCode> {
    let step_id = TaskStepId::from_string(req.step_id);

    // Get existing step
    let mut step = state
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
        .task_step_repo
        .update(&step)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Emit event to frontend
    if let Some(app_handle) = &state.app_handle {
        let response = StepResponse::from(step.clone());
        let _ = app_handle.emit(
            "step:updated",
            serde_json::json!({
                "step": response,
                "task_id": step.task_id.as_str()
            }),
        );
    }

    Ok(Json(StepResponse::from(step)))
}

/// POST /api/fail_step
///
/// Mark a step as failed
async fn fail_step_http(
    State(state): State<Arc<AppState>>,
    Json(req): Json<FailStepRequest>,
) -> Result<Json<StepResponse>, StatusCode> {
    let step_id = TaskStepId::from_string(req.step_id);

    // Get existing step
    let mut step = state
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
        .task_step_repo
        .update(&step)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Emit event to frontend
    if let Some(app_handle) = &state.app_handle {
        let response = StepResponse::from(step.clone());
        let _ = app_handle.emit(
            "step:updated",
            serde_json::json!({
                "step": response,
                "task_id": step.task_id.as_str()
            }),
        );
    }

    Ok(Json(StepResponse::from(step)))
}

/// POST /api/add_step
///
/// Add a new step to a task during execution
async fn add_step_http(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddStepRequest>,
) -> Result<Json<StepResponse>, StatusCode> {
    let task_id = TaskId::from_string(req.task_id);

    // Determine sort_order
    let sort_order = if let Some(after_step_id_str) = req.after_step_id {
        // Insert after specified step
        let after_step_id = TaskStepId::from_string(after_step_id_str);
        let after_step = state
            .task_step_repo
            .get_by_id(&after_step_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;
        after_step.sort_order + 1
    } else {
        // Append to end - find max sort_order
        let steps = state
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
        .task_step_repo
        .create(step)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Emit event to frontend
    if let Some(app_handle) = &state.app_handle {
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
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
) -> Result<Json<StepProgressSummary>, StatusCode> {
    let task_id = TaskId::from_string(task_id);
    let steps = state
        .task_step_repo
        .get_by_task(&task_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(StepProgressSummary::from_steps(&task_id, &steps)))
}

// ============================================================================
// Helper Functions
// ============================================================================

fn task_to_response(task: &crate::domain::entities::Task) -> TaskResponse {
    TaskResponse {
        id: task.id.to_string(),
        title: task.title.clone(),
        description: task.description.clone(),
        status: format!("{:?}", task.internal_status),
        priority: task.priority.to_string(),
    }
}

fn parse_category(s: &str) -> Result<TaskCategory, String> {
    match s.to_lowercase().as_str() {
        "feature" => Ok(TaskCategory::Feature),
        "fix" | "bug" => Ok(TaskCategory::Fix),
        "refactor" => Ok(TaskCategory::Refactor),
        "test" | "testing" => Ok(TaskCategory::Test),
        "docs" | "documentation" => Ok(TaskCategory::Docs),
        "setup" | "infrastructure" | "infra" => Ok(TaskCategory::Setup),
        _ => Err(format!("Invalid category: {}", s)),
    }
}

fn parse_priority(s: &str) -> Result<Priority, String> {
    match s.to_lowercase().as_str() {
        "critical" | "urgent" => Ok(Priority::Critical),
        "high" => Ok(Priority::High),
        "medium" | "med" => Ok(Priority::Medium),
        "low" => Ok(Priority::Low),
        _ => Err(format!("Invalid priority: {}", s)),
    }
}

fn parse_artifact_type(s: &str) -> Result<ArtifactType, String> {
    match s.to_lowercase().as_str() {
        "prd" => Ok(ArtifactType::Prd),
        "specification" => Ok(ArtifactType::Specification),
        "research" | "researchdocument" | "research_document" => Ok(ArtifactType::ResearchDocument),
        "design" | "designdoc" | "design_doc" => Ok(ArtifactType::DesignDoc),
        "code_change" | "codechanges" => Ok(ArtifactType::CodeChange),
        "diff" => Ok(ArtifactType::Diff),
        "test_result" | "testresult" => Ok(ArtifactType::TestResult),
        "task_spec" | "taskspec" => Ok(ArtifactType::TaskSpec),
        "review_feedback" | "reviewfeedback" => Ok(ArtifactType::ReviewFeedback),
        "approval" => Ok(ArtifactType::Approval),
        "findings" => Ok(ArtifactType::Findings),
        "recommendations" => Ok(ArtifactType::Recommendations),
        "context" => Ok(ArtifactType::Context),
        "previous_work" | "previouswork" => Ok(ArtifactType::PreviousWork),
        "research_brief" | "researchbrief" => Ok(ArtifactType::ResearchBrief),
        _ => Err(format!("Invalid artifact type: {}", s)),
    }
}

/// Create a 500-character preview of artifact content
fn create_artifact_preview(artifact: &Artifact) -> String {
    let full_content = match &artifact.content {
        ArtifactContent::Inline { text } => text.clone(),
        ArtifactContent::File { path } => {
            format!("[File artifact at: {}]", path)
        }
    };

    if full_content.len() <= 500 {
        full_content
    } else {
        format!("{}...", &full_content[..500])
    }
}

/// Get task context - implementation that manually aggregates context
/// This replicates the logic from TaskContextService but works with trait objects
async fn get_task_context_impl(
    state: &AppState,
    task_id: &TaskId,
) -> AppResult<TaskContext> {
    // 1. Fetch task by ID
    let task = state
        .task_repo
        .get_by_id(task_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Task not found: {}", task_id)))?;

    // 2. If source_proposal_id present, fetch proposal and create TaskProposalSummary
    let source_proposal = if let Some(proposal_id) = &task.source_proposal_id {
        match state.task_proposal_repo.get_by_id(proposal_id).await? {
            Some(proposal) => {
                // Parse acceptance_criteria from JSON string to Vec<String>
                let acceptance_criteria: Vec<String> = proposal
                    .acceptance_criteria
                    .as_ref()
                    .and_then(|json_str| serde_json::from_str(json_str).ok())
                    .unwrap_or_default();

                Some(crate::domain::entities::TaskProposalSummary {
                    id: proposal.id.clone(),
                    title: proposal.title.clone(),
                    description: proposal.description.clone().unwrap_or_default(),
                    acceptance_criteria,
                    implementation_notes: None,
                    plan_version_at_creation: proposal.plan_version_at_creation,
                })
            }
            None => None,
        }
    } else {
        None
    };

    // 3. If plan_artifact_id present, fetch artifact and create ArtifactSummary
    let plan_artifact = if let Some(artifact_id) = &task.plan_artifact_id {
        match state.artifact_repo.get_by_id(artifact_id).await? {
            Some(artifact) => {
                let content_preview = create_artifact_preview(&artifact);
                Some(ArtifactSummary {
                    id: artifact.id.clone(),
                    title: artifact.name.clone(),
                    artifact_type: artifact.artifact_type,
                    current_version: artifact.metadata.version,
                    content_preview,
                })
            }
            None => None,
        }
    } else {
        None
    };

    // 4. Fetch related artifacts
    let related_artifacts = if let Some(artifact_id) = &task.plan_artifact_id {
        let related = state.artifact_repo.get_related(artifact_id).await?;
        related
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
            .collect()
    } else {
        vec![]
    };

    // 5. Fetch steps for the task
    let steps = state.task_step_repo.get_by_task(task_id).await?;

    // 6. Calculate step progress summary if steps exist
    let step_progress = if !steps.is_empty() {
        Some(StepProgressSummary::from_steps(task_id, &steps))
    } else {
        None
    };

    // 7. Generate context hints
    let mut context_hints = Vec::new();
    if source_proposal.is_some() {
        context_hints.push(
            "Task was created from ideation proposal - check acceptance criteria".to_string(),
        );
    }
    if plan_artifact.is_some() {
        context_hints.push("Implementation plan available - use get_artifact to read full plan before starting".to_string());
    }
    if !related_artifacts.is_empty() {
        context_hints.push(format!(
            "{} related artifact{} found - may contain useful context",
            related_artifacts.len(),
            if related_artifacts.len() == 1 { "" } else { "s" }
        ));
    }
    if !steps.is_empty() {
        context_hints.push(format!(
            "Task has {} step{} defined - use get_task_steps to see them",
            steps.len(),
            if steps.len() == 1 { "" } else { "s" }
        ));
    }
    if task.description.is_some() {
        context_hints.push("Task has description with additional details".to_string());
    }
    if context_hints.is_empty() {
        context_hints.push("No additional context artifacts found - proceed with task description and acceptance criteria".to_string());
    }

    // 8. Return TaskContext
    Ok(TaskContext {
        task,
        source_proposal,
        plan_artifact,
        related_artifacts,
        steps,
        step_progress,
        context_hints,
    })
}
