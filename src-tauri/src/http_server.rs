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
    Artifact, ArtifactContent, ArtifactId, ArtifactType, IdeationSessionId, Priority, TaskCategory,
    TaskId, TaskProposal, TaskProposalId,
};
use crate::error::AppResult;

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
        // Review tools (reviewer agent)
        .route("/api/complete_review", post(complete_review))
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
        .map(|p| parse_priority(p))
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
        .map(|c| parse_category(c))
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Parse priority if provided
    let user_priority = req
        .user_priority
        .as_ref()
        .map(|p| parse_priority(p))
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
