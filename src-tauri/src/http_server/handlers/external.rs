// External API handlers — Phase 4 + Phase 5
//
// These endpoints are exposed to external consumers (via the external MCP server)
// and require API key authentication + project scope enforcement.
//
// All endpoints extract `ProjectScope` from the X-RalphX-Project-Scope header
// (injected by the external MCP server) and enforce scope boundaries via
// `ProjectScopeGuard::assert_project_scope`.

#[cfg(test)]
#[path = "external_tests.rs"]
mod tests;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::domain::entities::{
    ideation::IdeationSession, types::ProjectId, IdeationSessionId, InternalStatus, TaskId,
};
use crate::http_server::project_scope::{ProjectScope, ProjectScopeGuard};

use super::HttpServerState;

// ============================================================================
// Response types
// ============================================================================

#[derive(Debug, Serialize)]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub task_count: u32,
}

#[derive(Debug, Serialize)]
pub struct ListProjectsResponse {
    pub projects: Vec<ProjectSummary>,
}

#[derive(Debug, Serialize)]
pub struct ProjectStatusTaskCounts {
    pub total: usize,
    pub backlog: usize,
    pub ready: usize,
    pub executing: usize,
    pub reviewing: usize,
    pub merging: usize,
    pub merged: usize,
    pub cancelled: usize,
    pub stopped: usize,
    pub blocked: usize,
    pub pending_review: usize,
    pub pending_merge: usize,
    pub other: usize,
}

#[derive(Debug, Serialize)]
pub struct ProjectStatusProjectInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct ProjectStatusResponse {
    pub project: ProjectStatusProjectInfo,
    pub task_counts: ProjectStatusTaskCounts,
    pub running_agents: usize,
}

#[derive(Debug, Deserialize)]
pub struct StartIdeationRequest {
    pub project_id: String,
    pub title: String,
    // initial_prompt is part of the public API; used in future composite step when agent is spawned.
    #[allow(dead_code)]
    pub initial_prompt: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StartIdeationResponse {
    pub session_id: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct IdeationStatusResponse {
    pub session_id: String,
    pub project_id: String,
    pub title: Option<String>,
    pub status: String,
    pub agent_running: bool,
    pub proposal_count: u32,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct PipelineStages {
    pub pending: usize,
    pub executing: usize,
    pub reviewing: usize,
    pub pending_merge: usize,
    pub merging: usize,
    pub merged: usize,
    pub blocked: usize,
    pub cancelled: usize,
    pub stopped: usize,
}

#[derive(Debug, Serialize)]
pub struct PipelineOverviewResponse {
    pub project_id: String,
    pub stages: PipelineStages,
}

#[derive(Debug, Deserialize)]
pub struct PollEventsQuery {
    pub project_id: String,
    pub cursor: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ExternalEvent {
    pub id: i64,
    pub event_type: String,
    pub project_id: String,
    pub payload: serde_json::Value,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct PollEventsResponse {
    pub events: Vec<ExternalEvent>,
    pub next_cursor: Option<i64>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionAction {
    Pause,
    Cancel,
    Retry,
}

#[derive(Debug, Deserialize)]
pub struct TaskTransitionRequest {
    pub task_id: String,
    pub action: TransitionAction,
}

#[derive(Debug, Serialize)]
pub struct TaskTransitionResponse {
    pub success: bool,
    pub task_id: String,
    pub new_status: String,
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /api/external/projects
/// List all projects, filtered by ProjectScope if present.
pub async fn list_projects_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
) -> Result<Json<ListProjectsResponse>, StatusCode> {
    let projects = state
        .app_state
        .project_repo
        .get_all()
        .await
        .map_err(|e| {
            error!("Failed to list projects: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let mut summaries = Vec::new();
    for project in &projects {
        // Filter by scope if restricted
        if let Some(ref allowed) = scope.0 {
            if !allowed.contains(&project.id) {
                continue;
            }
        }

        let task_count = state
            .app_state
            .task_repo
            .count_tasks(&project.id, false, None, None)
            .await
            .unwrap_or(0);

        summaries.push(ProjectSummary {
            id: project.id.to_string(),
            name: project.name.clone(),
            description: None,
            created_at: project.created_at.to_rfc3339(),
            task_count,
        });
    }

    Ok(Json(ListProjectsResponse { projects: summaries }))
}

/// GET /api/external/project/:id/status
/// Get project status including task counts and running agents.
pub async fn get_project_status_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Path(id): Path<String>,
) -> Result<Json<ProjectStatusResponse>, StatusCode> {
    let project_id = ProjectId::from_string(id);

    let project = state
        .app_state
        .project_repo
        .get_by_id(&project_id)
        .await
        .map_err(|e| {
            error!("Failed to get project {}: {}", project_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Enforce project scope
    project
        .assert_project_scope(&scope)
        .map_err(|e| e.status)?;

    // Load all tasks for project
    let tasks = state
        .app_state
        .task_repo
        .get_by_project(&project_id)
        .await
        .map_err(|e| {
            error!("Failed to get tasks for project {}: {}", project_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let mut counts = ProjectStatusTaskCounts {
        total: tasks.len(),
        backlog: 0,
        ready: 0,
        executing: 0,
        reviewing: 0,
        merging: 0,
        merged: 0,
        cancelled: 0,
        stopped: 0,
        blocked: 0,
        pending_review: 0,
        pending_merge: 0,
        other: 0,
    };

    for task in &tasks {
        match task.internal_status {
            InternalStatus::Backlog => counts.backlog += 1,
            InternalStatus::Ready => counts.ready += 1,
            InternalStatus::Executing
            | InternalStatus::QaRefining
            | InternalStatus::QaTesting
            | InternalStatus::QaPassed
            | InternalStatus::QaFailed
            | InternalStatus::ReExecuting => counts.executing += 1,
            InternalStatus::PendingReview => counts.pending_review += 1,
            InternalStatus::Reviewing
            | InternalStatus::ReviewPassed
            | InternalStatus::Escalated
            | InternalStatus::RevisionNeeded => counts.reviewing += 1,
            InternalStatus::Approved => counts.other += 1,
            InternalStatus::PendingMerge => counts.pending_merge += 1,
            InternalStatus::Merging
            | InternalStatus::MergeIncomplete
            | InternalStatus::MergeConflict => counts.merging += 1,
            InternalStatus::Merged => counts.merged += 1,
            InternalStatus::Failed => counts.other += 1,
            InternalStatus::Cancelled => counts.cancelled += 1,
            InternalStatus::Paused => counts.stopped += 1,
            InternalStatus::Stopped => counts.stopped += 1,
            InternalStatus::Blocked => counts.blocked += 1,
        }
    }

    // Count running agents for this project by iterating the registry
    let all_running = state
        .app_state
        .running_agent_registry
        .list_all()
        .await;
    let running_agents = all_running
        .iter()
        .filter(|(key, _)| {
            // task_execution:{task_id} — check if the task belongs to this project
            // We check by seeing if any task in our list matches
            if key.context_type == "task_execution" {
                tasks.iter().any(|t| t.id.as_str() == key.context_id)
            } else {
                false
            }
        })
        .count();

    Ok(Json(ProjectStatusResponse {
        project: ProjectStatusProjectInfo {
            id: project.id.to_string(),
            name: project.name.clone(),
        },
        task_counts: counts,
        running_agents,
    }))
}

/// POST /api/external/start_ideation
/// Create a new ideation session for a project.
pub async fn start_ideation_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Json(req): Json<StartIdeationRequest>,
) -> Result<Json<StartIdeationResponse>, StatusCode> {
    let project_id = ProjectId::from_string(req.project_id.clone());

    // Load project to validate it exists and enforce scope
    let project = state
        .app_state
        .project_repo
        .get_by_id(&project_id)
        .await
        .map_err(|e| {
            error!("Failed to get project {}: {}", project_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    project
        .assert_project_scope(&scope)
        .map_err(|e| e.status)?;

    // Check max_external_ideation_sessions limit (default: 1)
    let max_sessions: u32 = 1;
    let active_count = state
        .app_state
        .ideation_session_repo
        .count_by_status(
            &project_id,
            crate::domain::entities::ideation::IdeationSessionStatus::Active,
        )
        .await
        .map_err(|e| {
            error!("Failed to count active sessions: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if active_count >= max_sessions {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    // Create the ideation session
    let session = IdeationSession::new_with_title(project_id, req.title.clone());
    let created = state
        .app_state
        .ideation_session_repo
        .create(session)
        .await
        .map_err(|e| {
            error!("Failed to create ideation session: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(StartIdeationResponse {
        session_id: created.id.to_string(),
        status: "ideating".to_string(),
    }))
}

/// GET /api/external/ideation_status/:id
/// Get ideation session status.
pub async fn get_ideation_status_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Path(id): Path<String>,
) -> Result<Json<IdeationStatusResponse>, StatusCode> {
    let session_id = IdeationSessionId::from_string(id);

    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|e| {
            error!("Failed to get ideation session {}: {}", session_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Enforce scope
    session
        .assert_project_scope(&scope)
        .map_err(|e| e.status)?;

    // Count proposals for this session
    let proposal_count = state
        .app_state
        .task_proposal_repo
        .count_by_session(&session_id)
        .await
        .map_err(|e| {
            error!("Failed to count proposals: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Check if agent is running for this session
    let agent_key = crate::domain::services::running_agent_registry::RunningAgentKey::new(
        "session",
        session_id.as_str(),
    );
    let agent_running = state
        .app_state
        .running_agent_registry
        .is_running(&agent_key)
        .await;

    Ok(Json(IdeationStatusResponse {
        session_id: session.id.to_string(),
        project_id: session.project_id.to_string(),
        title: session.title.clone(),
        status: session.status.to_string(),
        agent_running,
        proposal_count,
        created_at: session.created_at.to_rfc3339(),
    }))
}

/// GET /api/external/pipeline/:project_id
/// Get pipeline overview — task counts per stage.
pub async fn get_pipeline_overview_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Path(project_id): Path<String>,
) -> Result<Json<PipelineOverviewResponse>, StatusCode> {
    let project_id = ProjectId::from_string(project_id);

    // Validate project exists and is in scope
    let project = state
        .app_state
        .project_repo
        .get_by_id(&project_id)
        .await
        .map_err(|e| {
            error!("Failed to get project {}: {}", project_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    project
        .assert_project_scope(&scope)
        .map_err(|e| e.status)?;

    // Load all tasks
    let tasks = state
        .app_state
        .task_repo
        .get_by_project(&project_id)
        .await
        .map_err(|e| {
            error!("Failed to get tasks for project {}: {}", project_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let mut stages = PipelineStages {
        pending: 0,
        executing: 0,
        reviewing: 0,
        pending_merge: 0,
        merging: 0,
        merged: 0,
        blocked: 0,
        cancelled: 0,
        stopped: 0,
    };

    for task in &tasks {
        match task.internal_status {
            InternalStatus::Backlog | InternalStatus::Ready => stages.pending += 1,
            InternalStatus::Executing
            | InternalStatus::QaRefining
            | InternalStatus::QaTesting
            | InternalStatus::QaPassed
            | InternalStatus::QaFailed
            | InternalStatus::ReExecuting => stages.executing += 1,
            InternalStatus::PendingReview
            | InternalStatus::Reviewing
            | InternalStatus::ReviewPassed
            | InternalStatus::Escalated
            | InternalStatus::RevisionNeeded => stages.reviewing += 1,
            InternalStatus::Approved | InternalStatus::PendingMerge => stages.pending_merge += 1,
            InternalStatus::Merging
            | InternalStatus::MergeIncomplete
            | InternalStatus::MergeConflict => stages.merging += 1,
            InternalStatus::Merged => stages.merged += 1,
            InternalStatus::Blocked => stages.blocked += 1,
            InternalStatus::Cancelled | InternalStatus::Failed => stages.cancelled += 1,
            InternalStatus::Paused | InternalStatus::Stopped => stages.stopped += 1,
        }
    }

    Ok(Json(PipelineOverviewResponse {
        project_id: project.id.to_string(),
        stages,
    }))
}

/// GET /api/external/events/poll
/// Poll external events for a project with cursor-based pagination.
pub async fn poll_events_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Query(params): Query<PollEventsQuery>,
) -> Result<Json<PollEventsResponse>, StatusCode> {
    let project_id = ProjectId::from_string(params.project_id.clone());

    // Validate project exists and is in scope
    let project = state
        .app_state
        .project_repo
        .get_by_id(&project_id)
        .await
        .map_err(|e| {
            error!("Failed to get project {}: {}", project_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    project
        .assert_project_scope(&scope)
        .map_err(|e| e.status)?;

    let cursor = params.cursor.unwrap_or(0);
    let limit = params.limit.unwrap_or(50).clamp(1, 200);
    let project_id_str = project_id.to_string();

    // Query external_events via the shared db connection
    let events = state
        .app_state
        .db
        .run(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, event_type, project_id, payload, created_at \
                 FROM external_events \
                 WHERE project_id = ?1 AND id > ?2 \
                 ORDER BY id ASC \
                 LIMIT ?3",
            )?;
            let rows = stmt.query_map(
                rusqlite::params![project_id_str, cursor, limit + 1],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )?;
            let mut result = Vec::new();
            for row in rows {
                result.push(row.map_err(|e| crate::error::AppError::Database(e.to_string()))?);
            }
            Ok(result)
        })
        .await
        .map_err(|e| {
            error!("Failed to query external_events: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let has_more = events.len() as i64 > limit;
    let events_page: Vec<_> = events.into_iter().take(limit as usize).collect();
    let next_cursor = if has_more {
        events_page.last().map(|(id, _, _, _, _)| *id)
    } else {
        None
    };

    let event_responses: Vec<ExternalEvent> = events_page
        .into_iter()
        .map(|(id, event_type, proj_id, payload, created_at)| {
            let payload_json: serde_json::Value =
                serde_json::from_str(&payload).unwrap_or(serde_json::json!({}));
            ExternalEvent {
                id,
                event_type,
                project_id: proj_id,
                payload: payload_json,
                created_at,
            }
        })
        .collect();

    Ok(Json(PollEventsResponse {
        events: event_responses,
        next_cursor,
        has_more,
    }))
}

/// POST /api/external/task_transition
/// Transition a task's state (pause, cancel, retry).
pub async fn external_task_transition_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Json(req): Json<TaskTransitionRequest>,
) -> Result<Json<TaskTransitionResponse>, StatusCode> {
    let task_id = TaskId::from_string(req.task_id.clone());

    // Load task and enforce scope
    let task = state
        .app_state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|e| {
            error!("Failed to get task {}: {}", task_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    task.assert_project_scope(&scope).map_err(|e| e.status)?;

    // Map action to target status
    let target_status = match req.action {
        TransitionAction::Pause => InternalStatus::Paused,
        TransitionAction::Cancel => InternalStatus::Cancelled,
        TransitionAction::Retry => {
            // Retry means move from terminal state to Ready
            if task.internal_status.is_terminal() {
                InternalStatus::Ready
            } else {
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    };

    // Build transition service
    let transition_service = crate::application::TaskTransitionService::new(
        std::sync::Arc::clone(&state.app_state.task_repo),
        std::sync::Arc::clone(&state.app_state.task_dependency_repo),
        std::sync::Arc::clone(&state.app_state.project_repo),
        std::sync::Arc::clone(&state.app_state.chat_message_repo),
        std::sync::Arc::clone(&state.app_state.chat_attachment_repo),
        std::sync::Arc::clone(&state.app_state.chat_conversation_repo),
        std::sync::Arc::clone(&state.app_state.agent_run_repo),
        std::sync::Arc::clone(&state.app_state.ideation_session_repo),
        std::sync::Arc::clone(&state.app_state.activity_event_repo),
        std::sync::Arc::clone(&state.app_state.message_queue),
        std::sync::Arc::clone(&state.app_state.running_agent_registry),
        std::sync::Arc::clone(&state.execution_state),
        state.app_state.app_handle.clone(),
        std::sync::Arc::clone(&state.app_state.memory_event_repo),
    )
    .with_plan_branch_repo(std::sync::Arc::clone(&state.app_state.plan_branch_repo))
    .with_interactive_process_registry(std::sync::Arc::clone(
        &state.app_state.interactive_process_registry,
    ))
    .with_external_events_repo(std::sync::Arc::clone(&state.app_state.external_events_repo));

    let updated_task = transition_service
        .transition_task(&task_id, target_status)
        .await
        .map_err(|e| {
            error!("Failed to transition task {}: {}", task_id.as_str(), e);
            StatusCode::UNPROCESSABLE_ENTITY
        })?;

    Ok(Json(TaskTransitionResponse {
        success: true,
        task_id: updated_task.id.to_string(),
        new_status: updated_task.internal_status.to_string(),
    }))
}

// ============================================================================
// Phase 5: Pipeline supervision response types
// ============================================================================

#[derive(Debug, Serialize)]
pub struct TaskStepSummary {
    pub id: String,
    pub title: String,
    pub status: String,
    pub sort_order: i32,
}

#[derive(Debug, Serialize)]
pub struct TaskDetailResponse {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub project_id: String,
    pub task_branch: Option<String>,
    pub worktree_path: Option<String>,
    pub steps: Vec<TaskStepSummary>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct TaskDiffResponse {
    pub task_id: String,
    pub files_changed: u32,
    pub insertions: u32,
    pub deletions: u32,
    pub changed_files: Vec<String>,
    pub task_branch: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReviewNoteSummary {
    pub id: String,
    pub reviewer: String,
    pub outcome: String,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ReviewSummaryResponse {
    pub task_id: String,
    pub task_status: String,
    pub review_notes: Vec<ReviewNoteSummary>,
    pub revision_count: u32,
}

#[derive(Debug, Serialize)]
pub struct MergePipelineTask {
    pub id: String,
    pub title: String,
    pub status: String,
    pub task_branch: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct MergePipelineResponse {
    pub project_id: String,
    pub tasks: Vec<MergePipelineTask>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewActionType {
    ApproveReview,
    RequestChanges,
    ResolveEscalation,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EscalationResolution {
    Approve,
    RequestChanges,
    Cancel,
}

#[derive(Debug, Deserialize)]
pub struct ReviewActionRequest {
    pub task_id: String,
    pub action: ReviewActionType,
    // feedback is part of the public API; reserved for future use (audit log, reviewer note).
    #[allow(dead_code)]
    pub feedback: Option<String>,
    pub resolution: Option<EscalationResolution>,
}

#[derive(Debug, Serialize)]
pub struct ReviewActionResponse {
    pub success: bool,
    pub task_id: String,
    pub new_status: String,
}

// ============================================================================
// Phase 5: Pipeline supervision handlers
// ============================================================================

/// GET /api/external/task/:id
/// Get full task details + steps.
pub async fn get_task_detail_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Path(id): Path<String>,
) -> Result<Json<TaskDetailResponse>, StatusCode> {
    let task_id = crate::domain::entities::TaskId::from_string(id);

    let task = state
        .app_state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|e| {
            error!("Failed to get task {}: {}", task_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    task.assert_project_scope(&scope).map_err(|e| e.status)?;

    let steps = state
        .app_state
        .task_step_repo
        .get_by_task(&task_id)
        .await
        .map_err(|e| {
            error!("Failed to get steps for task {}: {}", task_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let step_summaries: Vec<TaskStepSummary> = steps
        .into_iter()
        .map(|s| TaskStepSummary {
            id: s.id.to_string(),
            title: s.title.clone(),
            status: format!("{:?}", s.status),
            sort_order: s.sort_order,
        })
        .collect();

    Ok(Json(TaskDetailResponse {
        id: task.id.to_string(),
        title: task.title.clone(),
        description: task.description.clone(),
        status: task.internal_status.to_string(),
        project_id: task.project_id.to_string(),
        task_branch: task.task_branch.clone(),
        worktree_path: task.worktree_path.clone(),
        steps: step_summaries,
        created_at: task.created_at.to_rfc3339(),
        updated_at: task.updated_at.to_rfc3339(),
    }))
}

/// GET /api/external/task/:id/diff
/// Get git diff stats for a task branch.
pub async fn get_task_diff_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Path(id): Path<String>,
) -> Result<Json<TaskDiffResponse>, StatusCode> {
    use std::path::PathBuf;
    use crate::application::GitService;

    let task_id = crate::domain::entities::TaskId::from_string(id);

    let task = state
        .app_state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|e| {
            error!("Failed to get task {}: {}", task_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    task.assert_project_scope(&scope).map_err(|e| e.status)?;

    let task_branch = task.task_branch.clone();

    // If no branch, return empty diff
    if task_branch.is_none() {
        return Ok(Json(TaskDiffResponse {
            task_id: task.id.to_string(),
            files_changed: 0,
            insertions: 0,
            deletions: 0,
            changed_files: vec![],
            task_branch: None,
        }));
    }

    let project = state
        .app_state
        .project_repo
        .get_by_id(&task.project_id)
        .await
        .map_err(|e| {
            error!("Failed to get project {}: {}", task.project_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let base_branch = project.base_branch.as_deref().unwrap_or("main");
    let working_path = task
        .worktree_path
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(&project.working_directory));

    let stats = GitService::get_diff_stats(&working_path, base_branch)
        .await
        .map_err(|e| {
            error!("Failed to get diff stats for task {}: {}", task_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(TaskDiffResponse {
        task_id: task.id.to_string(),
        files_changed: stats.files_changed,
        insertions: stats.insertions,
        deletions: stats.deletions,
        changed_files: stats.changed_files,
        task_branch,
    }))
}

/// GET /api/external/task/:id/review_summary
/// Get review notes and findings for a task.
pub async fn get_task_review_summary_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Path(id): Path<String>,
) -> Result<Json<ReviewSummaryResponse>, StatusCode> {
    use crate::domain::entities::review::ReviewOutcome;

    let task_id = crate::domain::entities::TaskId::from_string(id);

    let task = state
        .app_state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|e| {
            error!("Failed to get task {}: {}", task_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    task.assert_project_scope(&scope).map_err(|e| e.status)?;

    let notes = state
        .app_state
        .review_repo
        .get_notes_by_task_id(&task_id)
        .await
        .map_err(|e| {
            error!("Failed to get review notes for task {}: {}", task_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let revision_count = notes
        .iter()
        .filter(|n| n.outcome == ReviewOutcome::ChangesRequested)
        .count() as u32;

    let note_summaries: Vec<ReviewNoteSummary> = notes
        .into_iter()
        .map(|n| ReviewNoteSummary {
            id: n.id.to_string(),
            reviewer: n.reviewer.to_string(),
            outcome: n.outcome.to_string(),
            notes: n.notes,
            created_at: n.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(ReviewSummaryResponse {
        task_id: task.id.to_string(),
        task_status: task.internal_status.to_string(),
        review_notes: note_summaries,
        revision_count,
    }))
}

/// GET /api/external/merge_pipeline/:project_id
/// Get all tasks in merge-related states for a project.
pub async fn get_merge_pipeline_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Path(project_id): Path<String>,
) -> Result<Json<MergePipelineResponse>, StatusCode> {
    let project_id = ProjectId::from_string(project_id);

    let project = state
        .app_state
        .project_repo
        .get_by_id(&project_id)
        .await
        .map_err(|e| {
            error!("Failed to get project {}: {}", project_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    project.assert_project_scope(&scope).map_err(|e| e.status)?;

    let all_tasks = state
        .app_state
        .task_repo
        .get_by_project(&project_id)
        .await
        .map_err(|e| {
            error!("Failed to get tasks for project {}: {}", project_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Filter to tasks in merge-related states
    let merge_tasks: Vec<MergePipelineTask> = all_tasks
        .into_iter()
        .filter(|t| {
            matches!(
                t.internal_status,
                InternalStatus::PendingMerge
                    | InternalStatus::Merging
                    | InternalStatus::MergeIncomplete
                    | InternalStatus::MergeConflict
                    | InternalStatus::Merged
            )
        })
        .map(|t| MergePipelineTask {
            id: t.id.to_string(),
            title: t.title.clone(),
            status: t.internal_status.to_string(),
            task_branch: t.task_branch.clone(),
            updated_at: t.updated_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(MergePipelineResponse {
        project_id: project.id.to_string(),
        tasks: merge_tasks,
    }))
}

/// POST /api/external/review_action
/// Approve a review, request changes, or resolve an escalation.
/// All operations go through TaskTransitionService for state machine enforcement.
pub async fn review_action_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Json(req): Json<ReviewActionRequest>,
) -> Result<Json<ReviewActionResponse>, StatusCode> {
    let task_id = crate::domain::entities::TaskId::from_string(req.task_id.clone());

    let task = state
        .app_state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|e| {
            error!("Failed to get task {}: {}", task_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    task.assert_project_scope(&scope).map_err(|e| e.status)?;

    let target_status = match &req.action {
        ReviewActionType::ApproveReview => InternalStatus::Approved,
        ReviewActionType::RequestChanges => InternalStatus::RevisionNeeded,
        ReviewActionType::ResolveEscalation => {
            match req.resolution.as_ref().ok_or(StatusCode::BAD_REQUEST)? {
                EscalationResolution::Approve => InternalStatus::Approved,
                EscalationResolution::RequestChanges => InternalStatus::RevisionNeeded,
                EscalationResolution::Cancel => InternalStatus::Cancelled,
            }
        }
    };

    let transition_service = crate::application::TaskTransitionService::new(
        std::sync::Arc::clone(&state.app_state.task_repo),
        std::sync::Arc::clone(&state.app_state.task_dependency_repo),
        std::sync::Arc::clone(&state.app_state.project_repo),
        std::sync::Arc::clone(&state.app_state.chat_message_repo),
        std::sync::Arc::clone(&state.app_state.chat_attachment_repo),
        std::sync::Arc::clone(&state.app_state.chat_conversation_repo),
        std::sync::Arc::clone(&state.app_state.agent_run_repo),
        std::sync::Arc::clone(&state.app_state.ideation_session_repo),
        std::sync::Arc::clone(&state.app_state.activity_event_repo),
        std::sync::Arc::clone(&state.app_state.message_queue),
        std::sync::Arc::clone(&state.app_state.running_agent_registry),
        std::sync::Arc::clone(&state.execution_state),
        state.app_state.app_handle.clone(),
        std::sync::Arc::clone(&state.app_state.memory_event_repo),
    )
    .with_plan_branch_repo(std::sync::Arc::clone(&state.app_state.plan_branch_repo))
    .with_interactive_process_registry(std::sync::Arc::clone(
        &state.app_state.interactive_process_registry,
    ))
    .with_external_events_repo(std::sync::Arc::clone(&state.app_state.external_events_repo));

    let updated_task = transition_service
        .transition_task(&task_id, target_status)
        .await
        .map_err(|e| {
            error!("Failed to transition task {} for review action: {}", task_id.as_str(), e);
            StatusCode::UNPROCESSABLE_ENTITY
        })?;

    Ok(Json(ReviewActionResponse {
        success: true,
        task_id: updated_task.id.to_string(),
        new_status: updated_task.internal_status.to_string(),
    }))
}

// ============================================================================
// Phase 6: Attention items + Execution capacity
// ============================================================================

#[derive(Debug, Serialize)]
pub struct AttentionTaskSummary {
    pub task_id: String,
    pub title: String,
    pub status: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct AttentionItemsResponse {
    pub escalated_reviews: Vec<AttentionTaskSummary>,
    pub failed_tasks: Vec<AttentionTaskSummary>,
    pub merge_conflicts: Vec<AttentionTaskSummary>,
}

/// GET /api/external/attention/:project_id
/// Returns tasks that need human attention grouped by category.
pub async fn get_attention_items_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Path(project_id): Path<String>,
) -> Result<Json<AttentionItemsResponse>, StatusCode> {
    let project_id = ProjectId::from_string(project_id);

    let project = state
        .app_state
        .project_repo
        .get_by_id(&project_id)
        .await
        .map_err(|e| {
            error!("Failed to get project {}: {}", project_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    project.assert_project_scope(&scope).map_err(|e| e.status)?;

    let tasks = state
        .app_state
        .task_repo
        .get_by_project(&project_id)
        .await
        .map_err(|e| {
            error!("Failed to get tasks for project {}: {}", project_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let mut escalated_reviews: Vec<AttentionTaskSummary> = Vec::new();
    let mut failed_tasks: Vec<AttentionTaskSummary> = Vec::new();
    let mut merge_conflicts: Vec<AttentionTaskSummary> = Vec::new();

    for task in &tasks {
        let summary = AttentionTaskSummary {
            task_id: task.id.to_string(),
            title: task.title.clone(),
            status: task.internal_status.to_string(),
            updated_at: task.updated_at.to_rfc3339(),
        };
        match task.internal_status {
            InternalStatus::Escalated | InternalStatus::RevisionNeeded => {
                escalated_reviews.push(summary);
            }
            InternalStatus::Failed => {
                failed_tasks.push(summary);
            }
            InternalStatus::MergeConflict | InternalStatus::MergeIncomplete => {
                merge_conflicts.push(summary);
            }
            _ => {}
        }
    }

    Ok(Json(AttentionItemsResponse {
        escalated_reviews,
        failed_tasks,
        merge_conflicts,
    }))
}

#[derive(Debug, Serialize)]
pub struct ExecutionCapacityResponse {
    pub can_start: bool,
    pub project_running: usize,
    pub project_queued: usize,
}

/// GET /api/external/execution_capacity/:project_id
/// Returns project-scoped execution slot counts (no global state exposed).
pub async fn get_execution_capacity_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Path(project_id): Path<String>,
) -> Result<Json<ExecutionCapacityResponse>, StatusCode> {
    let project_id = ProjectId::from_string(project_id);

    let project = state
        .app_state
        .project_repo
        .get_by_id(&project_id)
        .await
        .map_err(|e| {
            error!("Failed to get project {}: {}", project_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    project.assert_project_scope(&scope).map_err(|e| e.status)?;

    let tasks = state
        .app_state
        .task_repo
        .get_by_project(&project_id)
        .await
        .map_err(|e| {
            error!("Failed to get tasks for project {}: {}", project_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let project_running = tasks
        .iter()
        .filter(|t| {
            matches!(
                t.internal_status,
                InternalStatus::Executing
                    | InternalStatus::ReExecuting
                    | InternalStatus::QaRefining
                    | InternalStatus::QaTesting
                    | InternalStatus::Reviewing
                    | InternalStatus::Merging
            )
        })
        .count();

    let project_queued = tasks
        .iter()
        .filter(|t| {
            matches!(
                t.internal_status,
                InternalStatus::Ready
                    | InternalStatus::PendingReview
                    | InternalStatus::PendingMerge
            )
        })
        .count();

    // can_start uses the global ExecutionState (shared across all projects)
    let can_start = state.execution_state.can_start_task();

    Ok(Json(ExecutionCapacityResponse {
        can_start,
        project_running,
        project_queued,
    }))
}

/// GET /api/external/events/stream
/// Server-Sent Events endpoint for real-time task state change notifications.
///
/// Accepts an optional `project_id` query parameter to filter events.
/// Polls the external_events table every 2 seconds, emitting new events as SSE.
pub async fn stream_events_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Query(params): Query<StreamEventsQuery>,
) -> Result<axum::response::sse::Sse<impl futures::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>>, StatusCode> {
    use axum::response::sse::{Event, KeepAlive, Sse};
    use futures::stream;
    use futures::StreamExt as _;

    let project_id_filter = params.project_id.clone();

    // Validate project scope if project_id provided
    if let Some(ref pid) = project_id_filter {
        let project_id = ProjectId::from_string(pid.clone());
        let project = state
            .app_state
            .project_repo
            .get_by_id(&project_id)
            .await
            .map_err(|e| {
                error!("Failed to get project {}: {}", pid, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .ok_or(StatusCode::NOT_FOUND)?;
        project.assert_project_scope(&scope).map_err(|e| e.status)?;
    }

    let db = state.app_state.db.clone();

    // Start from the most-recent existing event (only push NEW events from this point on)
    let initial_cursor: i64 = {
        let pid_clone = project_id_filter.clone();
        db.run(move |conn| {
            let row: i64 = if let Some(ref pid) = pid_clone {
                conn.query_row(
                    "SELECT COALESCE(MAX(id), 0) FROM external_events WHERE project_id = ?1",
                    rusqlite::params![pid],
                    |r| r.get(0),
                )
                .unwrap_or(0)
            } else {
                conn.query_row(
                    "SELECT COALESCE(MAX(id), 0) FROM external_events",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(0)
            };
            Ok(row)
        })
        .await
        .unwrap_or(0)
    };

    // Build SSE stream via unfold — polls every 2 seconds
    let sse_stream = stream::unfold(
        (db, project_id_filter, scope, initial_cursor),
        |(db, project_id_filter, scope, cursor)| async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            let pid_clone = project_id_filter.clone();
            let rows = db
                .run(move |conn| {
                    if let Some(ref pid) = pid_clone {
                        let mut stmt = conn
                            .prepare(
                                "SELECT id, event_type, project_id, payload, created_at \
                                 FROM external_events WHERE id > ?1 AND project_id = ?2 \
                                 ORDER BY id ASC LIMIT 50",
                            )
                            .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
                        let mut result = Vec::new();
                        let rows = stmt
                            .query_map(rusqlite::params![cursor, pid], |row| {
                                Ok((
                                    row.get::<_, i64>(0)?,
                                    row.get::<_, String>(1)?,
                                    row.get::<_, String>(2)?,
                                    row.get::<_, String>(3)?,
                                    row.get::<_, String>(4)?,
                                ))
                            })
                            .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
                        for row in rows {
                            result.push(
                                row.map_err(|e| crate::error::AppError::Database(e.to_string()))?,
                            );
                        }
                        Ok(result)
                    } else {
                        let mut stmt = conn
                            .prepare(
                                "SELECT id, event_type, project_id, payload, created_at \
                                 FROM external_events WHERE id > ?1 \
                                 ORDER BY id ASC LIMIT 50",
                            )
                            .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
                        let mut result = Vec::new();
                        let rows = stmt
                            .query_map(rusqlite::params![cursor], |row| {
                                Ok((
                                    row.get::<_, i64>(0)?,
                                    row.get::<_, String>(1)?,
                                    row.get::<_, String>(2)?,
                                    row.get::<_, String>(3)?,
                                    row.get::<_, String>(4)?,
                                ))
                            })
                            .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
                        for row in rows {
                            result.push(
                                row.map_err(|e| crate::error::AppError::Database(e.to_string()))?,
                            );
                        }
                        Ok(result)
                    }
                })
                .await
                .unwrap_or_default();

            // Enforce scope allowlist
            let rows: Vec<_> = rows
                .into_iter()
                .filter(|(_, _, proj_id, _, _)| {
                    if let Some(ref allowed) = scope.0 {
                        allowed.iter().any(|p| p.to_string() == *proj_id)
                    } else {
                        true
                    }
                })
                .collect();

            let new_cursor = rows.last().map(|(id, _, _, _, _)| *id).unwrap_or(cursor);

            let events: Vec<Result<Event, std::convert::Infallible>> = rows
                .into_iter()
                .map(|(id, event_type, proj_id, payload, created_at)| {
                    let data = serde_json::json!({
                        "id": id,
                        "event_type": event_type,
                        "project_id": proj_id,
                        "payload": serde_json::from_str::<serde_json::Value>(&payload)
                            .unwrap_or(serde_json::json!({})),
                        "created_at": created_at,
                    });
                    Ok(Event::default()
                        .event(event_type)
                        .data(data.to_string()))
                })
                .collect();

            Some((
                stream::iter(events),
                (db, project_id_filter, scope, new_cursor),
            ))
        },
    )
    .flat_map(|s| s);

    Ok(Sse::new(sse_stream).keep_alive(KeepAlive::default()))
}

#[derive(Debug, Deserialize)]
pub struct StreamEventsQuery {
    pub project_id: Option<String>,
}
