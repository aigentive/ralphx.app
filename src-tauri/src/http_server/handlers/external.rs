// External API handlers — Phase 4 + Phase 5
//
// These endpoints are exposed to external consumers (via the external MCP server)
// and require API key authentication + project scope enforcement.
//
// All endpoints extract `ProjectScope` from the X-RalphX-Project-Scope header
// (injected by the external MCP server) and enforce scope boundaries via
// `ProjectScopeGuard::assert_project_scope`.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::error;
use tauri::Emitter;

use crate::application::chat_service::{ChatService, ClaudeChatService, SendMessageOptions};
use crate::application::task_cleanup_service::TaskCleanupService;
use crate::commands::ideation_commands::{apply_proposals_core, ApplyProposalsInput};
use crate::domain::entities::{
    ideation::IdeationSession, task::Task, types::ProjectId, ChatContextType, IdeationSessionId,
    InternalStatus, SessionOrigin, TaskId,
};
use crate::domain::services::{
    check_verification_gate, emit_verification_started, emit_verification_status_changed,
};
use crate::domain::services::text_similarity::{jaccard_similarity, tokenize_for_similarity};
use crate::http_server::project_scope::{ProjectScope, ProjectScopeGuard};
use crate::infrastructure::agents::claude::verification_config;

use super::{HttpError, HttpServerState};

/// SQLite error message fragment for UNIQUE constraint violations.
/// Used to detect idempotency key race conditions on concurrent inserts.
pub(crate) const SQLITE_UNIQUE_VIOLATION: &str = "unique";

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
    pub title: Option<String>,
    pub prompt: Option<String>,
    pub initial_prompt: Option<String>,
    pub idempotency_key: Option<String>,
}

/// Lightweight summary of an active external session for dedup awareness.
#[derive(Debug, Serialize, Clone)]
pub struct ExternalSessionSummary {
    pub session_id: String,
    pub title: Option<String>,
    pub status: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_activity_phase: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StartIdeationResponse {
    pub session_id: String,
    pub status: String,
    pub agent_spawned: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_spawn_blocked_reason: Option<String>,
    /// All active external sessions for the project (for agent visibility)
    pub existing_active_sessions: Vec<ExternalSessionSummary>,
    /// True if this response reuses an existing session due to idempotency key match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exists: Option<bool>,
    /// True if this response reuses an existing session due to Jaccard similarity match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplicate_detected: Option<bool>,
    /// Jaccard similarity score when duplicate_detected is true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similarity_score: Option<f64>,
    /// Behavioral hint for the caller
    pub next_action: String,
    /// Human-readable hint message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct IdeationMessageRequest {
    pub session_id: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct IdeationMessageResponse {
    /// Delivery outcome: "sent" | "queued" | "spawned"
    pub status: String,
    pub session_id: String,
    pub next_action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct IdeationStatusResponse {
    pub session_id: String,
    pub project_id: String,
    pub title: Option<String>,
    pub status: String,
    pub agent_running: bool,
    pub agent_status: String,
    pub proposal_count: u32,
    pub created_at: String,
    pub verification_status: String,
    pub verification_in_progress: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_proposal_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_accept_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_accept_started_at: Option<String>,
    pub next_action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    pub queued_message_count: u32,
    pub unread_message_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_activity_phase: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SessionTask {
    pub id: String,
    pub title: String,
    pub status: String,
    pub proposal_id: Option<String>,
    pub category: String,
    pub priority: i32,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct SessionTasksResponse {
    pub session_id: String,
    pub tasks: Vec<SessionTask>,
    pub delivery_status: String,
    pub task_count: usize,
}

/// Derive an aggregate delivery status from a slice of tasks linked to a session.
///
/// Rules (in priority order):
/// 1. `not_scheduled` — 0 tasks
/// 2. `delivered`     — all tasks are Merged
/// 3. `in_progress`   — any task is still actively executing / queued / in merge pipeline
/// 4. `pending_review`— no active tasks, some are in review states
/// 5. `partial`       — some Merged + rest are terminal (Cancelled / Failed / Stopped / Paused)
/// 6. `in_progress`   — fallback
fn derive_delivery_status(tasks: &[Task]) -> String {
    if tasks.is_empty() {
        return "not_scheduled".to_string();
    }

    let mut all_merged = true;
    let mut has_merged = false;
    let mut has_active = false;
    let mut has_terminal = false;
    let mut has_review = false;

    for task in tasks {
        match task.internal_status {
            InternalStatus::Merged => {
                has_merged = true;
            }
            InternalStatus::Cancelled
            | InternalStatus::Failed
            | InternalStatus::Stopped
            | InternalStatus::Paused => {
                has_terminal = true;
                all_merged = false;
            }
            InternalStatus::PendingReview
            | InternalStatus::Reviewing
            | InternalStatus::ReviewPassed
            | InternalStatus::Escalated
            | InternalStatus::RevisionNeeded
            | InternalStatus::Approved => {
                has_review = true;
                all_merged = false;
            }
            _ => {
                // Backlog, Ready, Executing, ReExecuting, QaTesting, QaRefining, QaPassed,
                // QaFailed, Blocked, PendingMerge, Merging, MergeIncomplete, MergeConflict
                has_active = true;
                all_merged = false;
            }
        }
    }

    if all_merged {
        return "delivered".to_string();
    }
    if has_active {
        return "in_progress".to_string();
    }
    if has_review {
        return "pending_review".to_string();
    }
    if has_merged && has_terminal {
        return "partial".to_string();
    }
    "in_progress".to_string()
}

#[derive(Debug, Serialize)]
pub struct SessionSummary {
    pub id: String,
    pub title: Option<String>,
    pub status: String,
    pub proposal_count: u32,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ListSessionsResponse {
    pub sessions: Vec<SessionSummary>,
}

#[derive(Debug, Deserialize)]
pub struct ListSessionsParams {
    pub status: Option<String>,
    pub limit: Option<u32>,
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

/// Build a fully configured `ClaudeChatService` from shared app + execution state.
/// Extracted to avoid duplicating the 12-arg constructor chain across multiple handlers.
fn build_chat_service(
    app: &crate::application::AppState,
    execution_state: &std::sync::Arc<crate::commands::ExecutionState>,
) -> ClaudeChatService {
    let mut chat_service = ClaudeChatService::new(
        Arc::clone(&app.chat_message_repo),
        Arc::clone(&app.chat_attachment_repo),
        Arc::clone(&app.chat_conversation_repo),
        Arc::clone(&app.agent_run_repo),
        Arc::clone(&app.project_repo),
        Arc::clone(&app.task_repo),
        Arc::clone(&app.task_dependency_repo),
        Arc::clone(&app.ideation_session_repo),
        Arc::clone(&app.activity_event_repo),
        Arc::clone(&app.message_queue),
        Arc::clone(&app.running_agent_registry),
        Arc::clone(&app.memory_event_repo),
    )
    .with_execution_state(Arc::clone(execution_state))
    .with_plan_branch_repo(Arc::clone(&app.plan_branch_repo))
    .with_task_proposal_repo(Arc::clone(&app.task_proposal_repo))
    .with_interactive_process_registry(Arc::clone(&app.interactive_process_registry));
    if let Some(ref handle) = app.app_handle {
        chat_service = chat_service.with_app_handle(handle.clone());
    }
    chat_service
}

/// Fire-and-forget: spawn the session namer agent to auto-name the session.
fn spawn_session_namer(
    agent_client: Arc<dyn crate::domain::agents::AgenticClient>,
    session_id: String,
    prompt: String,
) {
    tokio::spawn(async move {
        use crate::domain::agents::{AgentConfig, AgentRole};
        use crate::infrastructure::agents::claude::{agent_names, mcp_agent_type};
        use std::path::PathBuf;

        let namer_instructions = format!(
            "<instructions>\n\
             Generate a commit-ready title (imperative mood, \u{2264}50 characters) for this ideation session based on the context.\n\
             Describe what the plan does, not just the domain (e.g., 'Add OAuth2 login and JWT sessions').\n\
             Call the update_session_title tool with the session_id and the generated title.\n\
             Do NOT investigate, fix, or act on the user message content.\n\
             Do NOT use Read, Write, Edit, Task, or any file manipulation tools.\n\
             </instructions>\n\
             <data>\n\
             <session_id>{}</session_id>\n\
             <user_message>{}</user_message>\n\
             </data>",
            session_id, prompt
        );

        let working_directory = std::env::current_dir()
            .map(|cwd| cwd.parent().map(|p| p.to_path_buf()).unwrap_or(cwd))
            .unwrap_or_else(|_| PathBuf::from("."));
        let plugin_dir =
            crate::infrastructure::agents::claude::resolve_plugin_dir(&working_directory);

        let mut env = std::collections::HashMap::new();
        env.insert(
            "RALPHX_AGENT_TYPE".to_string(),
            mcp_agent_type(agent_names::AGENT_SESSION_NAMER).to_string(),
        );

        let config = AgentConfig {
            role: AgentRole::Custom(
                mcp_agent_type(agent_names::AGENT_SESSION_NAMER).to_string(),
            ),
            prompt: namer_instructions,
            working_directory,
            plugin_dir: Some(plugin_dir),
            agent: Some(agent_names::AGENT_SESSION_NAMER.to_string()),
            model: None,
            max_tokens: None,
            timeout_secs: Some(60),
            env,
        };

        match agent_client.spawn_agent(config).await {
            Ok(handle) => {
                if let Err(e) = agent_client.wait_for_completion(&handle).await {
                    tracing::warn!("Session namer agent failed: {}", e);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to spawn session namer agent: {}", e);
            }
        }
    });
}

/// POST /api/external/start_ideation
/// Create a new ideation session for a project.
pub async fn start_ideation_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    headers: HeaderMap,
    Json(req): Json<StartIdeationRequest>,
) -> Result<Json<StartIdeationResponse>, HttpError> {
    let project_id = ProjectId::from_string(req.project_id.clone());

    // Load project to validate it exists and enforce scope
    let project = state
        .app_state
        .project_repo
        .get_by_id(&project_id)
        .await
        .map_err(|e| {
            error!("Failed to get project {}: {}", project_id.as_str(), e);
            HttpError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: Some("Failed to get project".to_string()),
            }
        })?
        .ok_or(HttpError {
            status: StatusCode::NOT_FOUND,
            message: Some("Project not found".to_string()),
        })?;

    project
        .assert_project_scope(&scope)
        .map_err(|e| HttpError {
            status: e.status,
            message: e.message,
        })?;

    // Extract api_key_id from X-RalphX-Key-Id header
    let api_key_id = headers
        .get(crate::http_server::handlers::external_auth::EXTERNAL_KEY_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // ── Idempotency key check ──────────────────────────────────────────────
    if let (Some(ref key_id), Some(ref idem_key)) = (&api_key_id, &req.idempotency_key) {
        if let Ok(Some(existing)) = state
            .app_state
            .ideation_session_repo
            .get_by_idempotency_key(key_id, idem_key)
            .await
        {
            let active_sessions = state
                .app_state
                .ideation_session_repo
                .list_active_external_by_project(&project_id)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|s| ExternalSessionSummary {
                    session_id: s.id.to_string(),
                    title: s.title.clone(),
                    status: s.status.to_string(),
                    created_at: s.created_at.to_rfc3339(),
                    external_activity_phase: s.external_activity_phase.clone(),
                })
                .collect::<Vec<_>>();
            return Ok(Json(StartIdeationResponse {
                session_id: existing.id.to_string(),
                status: existing.status.to_string(),
                agent_spawned: false,
                agent_spawn_blocked_reason: None,
                existing_active_sessions: active_sessions,
                exists: Some(true),
                duplicate_detected: None,
                similarity_score: None,
                next_action: "poll_status".to_string(),
                hint: Some("Idempotent retry: returning existing session.".to_string()),
            }));
        }
    }

    // ── Query active external sessions for this project ───────────────────
    let active_sessions = state
        .app_state
        .ideation_session_repo
        .list_active_external_by_project(&project_id)
        .await
        .unwrap_or_default();

    // ── Jaccard similarity dedup ───────────────────────────────────────────
    let effective_prompt = req.prompt.clone().or_else(|| req.initial_prompt.clone());
    let has_candidate_text = req.prompt.is_some() || req.title.is_some();

    if has_candidate_text && !active_sessions.is_empty() {
        let candidate_text = format!(
            "{} {}",
            req.prompt.as_deref().unwrap_or(""),
            req.title.as_deref().unwrap_or("")
        );
        let candidate_tokens = tokenize_for_similarity(&candidate_text);
        let similarity_threshold =
            crate::infrastructure::agents::claude::external_mcp_config()
                .external_session_similarity_threshold;

        let mut best_match: Option<(f64, &crate::domain::entities::ideation::IdeationSession)> =
            None;
        for session in &active_sessions {
            let session_title = session.title.as_deref().unwrap_or("");
            let first_msg = state
                .app_state
                .chat_message_repo
                .get_first_user_message_by_context("ideation", session.id.as_str())
                .await
                .unwrap_or_default()
                .unwrap_or_default();
            let comparison_text = format!("{} {}", session_title, first_msg);
            let comparison_tokens = tokenize_for_similarity(&comparison_text);
            let score = jaccard_similarity(&candidate_tokens, &comparison_tokens);
            if score >= similarity_threshold && best_match.map(|(s, _)| score > s).unwrap_or(true) {
                best_match = Some((score, session));
            }
        }

        if let Some((score, matched_session)) = best_match {
            let active_summaries = active_sessions
                .iter()
                .map(|s| ExternalSessionSummary {
                    session_id: s.id.to_string(),
                    title: s.title.clone(),
                    status: s.status.to_string(),
                    created_at: s.created_at.to_rfc3339(),
                    external_activity_phase: s.external_activity_phase.clone(),
                })
                .collect::<Vec<_>>();
            let hint_msg = format!(
                "A similar session already exists ('{}', {:.0}% match). Reusing it instead of creating a duplicate.",
                matched_session.title.as_deref().unwrap_or("untitled"),
                score * 100.0
            );
            return Ok(Json(StartIdeationResponse {
                session_id: matched_session.id.to_string(),
                status: matched_session.status.to_string(),
                agent_spawned: false,
                agent_spawn_blocked_reason: None,
                existing_active_sessions: active_summaries,
                exists: None,
                duplicate_detected: Some(true),
                similarity_score: Some(score),
                next_action: "use_existing_session".to_string(),
                hint: Some(hint_msg),
            }));
        }
    }

    // ── Create new session ────────────────────────────────────────────────
    let mut session_builder = match req.title.clone() {
        None => IdeationSession::new(project_id.clone()),
        Some(t) => IdeationSession::new_with_title(project_id.clone(), t),
    };
    session_builder.origin = SessionOrigin::External;
    session_builder.external_activity_phase = Some("created".to_string());
    if let Some(ref key_id) = api_key_id {
        session_builder.api_key_id = Some(key_id.clone());
    }
    if let Some(ref idem_key) = req.idempotency_key {
        session_builder.idempotency_key = Some(idem_key.clone());
    }
    let created = match state
        .app_state
        .ideation_session_repo
        .create(session_builder)
        .await
    {
        Ok(session) => session,
        Err(e)
            if e.to_string().to_lowercase().contains(SQLITE_UNIQUE_VIOLATION)
                && api_key_id.is_some()
                && req.idempotency_key.is_some() =>
        {
            // Race condition: concurrent create with same idempotency key
            if let (Some(ref key_id), Some(ref idem_key)) = (&api_key_id, &req.idempotency_key) {
                if let Ok(Some(existing)) = state
                    .app_state
                    .ideation_session_repo
                    .get_by_idempotency_key(key_id, idem_key)
                    .await
                {
                    let active_summaries = active_sessions
                        .iter()
                        .map(|s| ExternalSessionSummary {
                            session_id: s.id.to_string(),
                            title: s.title.clone(),
                            status: s.status.to_string(),
                            created_at: s.created_at.to_rfc3339(),
                            external_activity_phase: s.external_activity_phase.clone(),
                        })
                        .collect::<Vec<_>>();
                    return Ok(Json(StartIdeationResponse {
                        session_id: existing.id.to_string(),
                        status: existing.status.to_string(),
                        agent_spawned: false,
                        agent_spawn_blocked_reason: None,
                        existing_active_sessions: active_summaries,
                        exists: Some(true),
                        duplicate_detected: None,
                        similarity_score: None,
                        next_action: "poll_status".to_string(),
                        hint: Some(
                            "Idempotent retry (concurrent): returning existing session."
                                .to_string(),
                        ),
                    }));
                }
            }
            error!("Failed to create ideation session (unique conflict, re-query failed): {}", e);
            return Err(HttpError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: Some("Failed to create ideation session".to_string()),
            });
        }
        Err(e) => {
            error!("Failed to create ideation session: {}", e);
            return Err(HttpError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: Some("Failed to create ideation session".to_string()),
            });
        }
    };

    let session_id_str = created.id.to_string();

    // Set external activity phase to "created"
    {
        let repo = Arc::clone(&state.app_state.ideation_session_repo);
        let sid = IdeationSessionId::from_string(session_id_str.clone());
        tokio::spawn(async move {
            if let Err(e) = repo.update_external_activity_phase(&sid, "created").await {
                error!("Failed to set activity phase 'created' for session {}: {}", sid.as_str(), e);
            }
        });
    }

    // Emit ideation:session_created event for frontend
    if let Some(ref handle) = state.app_state.app_handle {
        let _ = handle.emit(
            "ideation:session_created",
            serde_json::json!({
                "sessionId": session_id_str,
                "projectId": project_id.to_string(),
            }),
        );
    }

    // Build existing_active_sessions for response (include the freshly created session too)
    let existing_summaries = {
        let mut summaries: Vec<ExternalSessionSummary> = active_sessions
            .iter()
            .map(|s| ExternalSessionSummary {
                session_id: s.id.to_string(),
                title: s.title.clone(),
                status: s.status.to_string(),
                created_at: s.created_at.to_rfc3339(),
                external_activity_phase: s.external_activity_phase.clone(),
            })
            .collect();
        // Prepend the new session
        summaries.insert(
            0,
            ExternalSessionSummary {
                session_id: session_id_str.clone(),
                title: created.title.clone(),
                status: created.status.to_string(),
                created_at: created.created_at.to_rfc3339(),
                external_activity_phase: created.external_activity_phase.clone(),
            },
        );
        summaries
    };

    // If a prompt was provided, spawn the orchestrator agent (external sessions are always solo mode)
    let mut agent_spawned = false;
    let mut agent_spawn_blocked_reason: Option<String> = None;
    if let Some(ref prompt_str) = effective_prompt {
        let chat_service = build_chat_service(&state.app_state, &state.execution_state);
        // External sessions are always solo mode — no team_mode check needed

        match chat_service
            .send_message(
                ChatContextType::Ideation,
                &session_id_str,
                prompt_str,
                SendMessageOptions {
                    is_external_mcp: true,
                    ..Default::default()
                },
            )
            .await
        {
            Ok(result) if result.was_queued => {
                // Agent is running, message was queued — treat as success
                agent_spawned = true;
            }
            Ok(_) => {
                agent_spawned = true;
                spawn_session_namer(
                    Arc::clone(&state.app_state.agent_client),
                    session_id_str.clone(),
                    prompt_str.clone(),
                );
            }
            Err(e) => {
                error!(
                    "Failed to auto-spawn agent on external ideation session {}: {}",
                    session_id_str, e
                );
                agent_spawn_blocked_reason = Some(e.to_string());
            }
        }
    }

    Ok(Json(StartIdeationResponse {
        session_id: session_id_str,
        status: "ideating".to_string(),
        agent_spawned,
        agent_spawn_blocked_reason,
        existing_active_sessions: existing_summaries,
        exists: None,
        duplicate_detected: None,
        similarity_score: None,
        next_action: "poll_status".to_string(),
        hint: Some("Poll v1_get_ideation_status to track agent progress.".to_string()),
    }))
}

/// Determine agent tri-state status for a session:
/// "idle" | "generating" | "waiting_for_input"
async fn determine_agent_status(
    running_agent_registry: &dyn crate::domain::services::running_agent_registry::RunningAgentRegistry,
    interactive_process_registry: &crate::application::InteractiveProcessRegistry,
    context_id: &str,
) -> String {
    let agent_key =
        crate::domain::services::running_agent_registry::RunningAgentKey::new("ideation", context_id);
    if running_agent_registry.is_running(&agent_key).await {
        let ipr_key = crate::application::InteractiveProcessKey {
            context_type: "ideation".to_string(),
            context_id: context_id.to_string(),
        };
        if interactive_process_registry.has_process(&ipr_key).await {
            "waiting_for_input".to_string()
        } else {
            "generating".to_string()
        }
    } else {
        "idle".to_string()
    }
}

/// GET /api/external/ideation_status/:id
/// Get ideation session status.
pub async fn get_ideation_status_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Path(id): Path<String>,
) -> Result<Json<IdeationStatusResponse>, HttpError> {
    let session_id = IdeationSessionId::from_string(id);

    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|e| {
            error!("Failed to get ideation session {}: {}", session_id.as_str(), e);
            HttpError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: Some("Failed to get ideation session".to_string()),
            }
        })?
        .ok_or(HttpError {
            status: StatusCode::NOT_FOUND,
            message: Some("Session not found".to_string()),
        })?;

    // Enforce scope
    session
        .assert_project_scope(&scope)
        .map_err(|e| HttpError {
            status: e.status,
            message: e.message,
        })?;

    // Count proposals for this session
    let proposal_count = state
        .app_state
        .task_proposal_repo
        .count_by_session(&session_id)
        .await
        .map_err(|e| {
            error!("Failed to count proposals: {}", e);
            HttpError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: Some("Failed to count proposals".to_string()),
            }
        })?;

    // Check if agent is running for this session
    let agent_key = crate::domain::services::running_agent_registry::RunningAgentKey::new(
        "ideation",
        session_id.as_str(),
    );
    let agent_running = state
        .app_state
        .running_agent_registry
        .is_running(&agent_key)
        .await;

    // Determine agent tri-state status
    let agent_status = determine_agent_status(
        state.app_state.running_agent_registry.as_ref(),
        &state.app_state.interactive_process_registry,
        session_id.as_str(),
    )
    .await;

    // For accepted sessions, derive delivery_status from linked tasks
    let delivery_status = if session.status == crate::domain::entities::ideation::IdeationSessionStatus::Accepted {
        let tasks = state
            .app_state
            .task_repo
            .get_by_ideation_session(&session_id)
            .await
            .unwrap_or_default();
        Some(derive_delivery_status(&tasks))
    } else {
        None
    };

    // Count unread assistant messages (since last read position)
    let unread_message_count = state
        .app_state
        .chat_message_repo
        .count_unread_assistant_messages(
            session_id.as_str(),
            session.external_last_read_message_id.as_deref(),
        )
        .await
        .unwrap_or(0);

    // Count queued messages
    let queued_message_count = state
        .app_state
        .message_queue
        .count_for_context("ideation", session_id.as_str()) as u32;

    // Compute next_action and hint based on agent state
    let (next_action, hint) = match agent_status.as_str() {
        "waiting_for_input" if unread_message_count > 0 => (
            "fetch_messages".to_string(),
            Some("Agent has responded. Fetch messages before sending.".to_string()),
        ),
        "waiting_for_input" => (
            "send_message".to_string(),
            Some("Agent is ready for input.".to_string()),
        ),
        "generating" => (
            "wait".to_string(),
            Some("Agent is working. Poll again in 5-10s.".to_string()),
        ),
        _ => (
            "send_message".to_string(),
            Some("No agent running. Send a message to start.".to_string()),
        ),
    };

    Ok(Json(IdeationStatusResponse {
        session_id: session.id.to_string(),
        project_id: session.project_id.to_string(),
        title: session.title.clone(),
        status: session.status.to_string(),
        agent_running,
        agent_status,
        proposal_count,
        created_at: session.created_at.to_rfc3339(),
        verification_status: session.verification_status.to_string(),
        verification_in_progress: session.verification_in_progress,
        delivery_status,
        expected_proposal_count: session.expected_proposal_count,
        auto_accept_status: session.auto_accept_status.clone(),
        auto_accept_started_at: session.auto_accept_started_at.clone(),
        next_action,
        hint,
        queued_message_count,
        unread_message_count,
        external_activity_phase: session.external_activity_phase.clone(),
    }))
}

/// GET /api/external/sessions/:session_id/tasks
/// Get all tasks created from an ideation session with aggregate delivery_status.
pub async fn get_session_tasks_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Path(session_id): Path<String>,
) -> Result<Json<SessionTasksResponse>, HttpError> {
    let session_id_obj = IdeationSessionId::from_string(session_id.clone());

    // Load session to verify it exists and enforce scope
    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id_obj)
        .await
        .map_err(|e| {
            error!("Failed to get ideation session {}: {}", session_id, e);
            HttpError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: Some("Failed to get ideation session".to_string()),
            }
        })?
        .ok_or(HttpError {
            status: StatusCode::NOT_FOUND,
            message: Some("Session not found".to_string()),
        })?;

    // Enforce project scope
    session
        .assert_project_scope(&scope)
        .map_err(|e| HttpError {
            status: e.status,
            message: e.message,
        })?;

    // Fetch all tasks linked to this session
    let tasks = state
        .app_state
        .task_repo
        .get_by_ideation_session(&session_id_obj)
        .await
        .map_err(|e| {
            error!("Failed to get tasks for session {}: {}", session_id, e);
            HttpError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: Some("Failed to get session tasks".to_string()),
            }
        })?;

    let delivery_status = derive_delivery_status(&tasks);
    let task_count = tasks.len();

    let session_tasks: Vec<SessionTask> = tasks
        .into_iter()
        .map(|t| SessionTask {
            id: t.id.to_string(),
            title: t.title.clone(),
            status: t.internal_status.to_string(),
            proposal_id: t.source_proposal_id.as_ref().map(|p| p.to_string()),
            category: t.category.to_string(),
            priority: t.priority,
            created_at: t.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(SessionTasksResponse {
        session_id,
        tasks: session_tasks,
        delivery_status,
        task_count,
    }))
}

/// GET /api/external/sessions/:project_id?status=active&limit=20
/// List ideation sessions for a project, optionally filtered by status.
pub async fn list_ideation_sessions_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Path(project_id): Path<String>,
    Query(params): Query<ListSessionsParams>,
) -> Result<Json<ListSessionsResponse>, (StatusCode, Json<serde_json::Value>)> {
    let pid = ProjectId::from_string(project_id.clone());

    // Validate project exists and enforce scope
    let project = state
        .app_state
        .project_repo
        .get_by_id(&pid)
        .await
        .map_err(|e| {
            error!("Failed to get project {}: {}", project_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Internal server error"})),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Project not found"})),
            )
        })?;

    project.assert_project_scope(&scope).map_err(|e| {
        (
            e.status,
            Json(serde_json::json!({"error": "Forbidden"})),
        )
    })?;

    let limit = params.limit.unwrap_or(20).clamp(1, 100);

    // Fetch sessions based on status filter
    let sessions = match params.status.as_deref() {
        None | Some("all") => {
            // Return all sessions for the project (up to limit, ordered by updated_at DESC)
            let all = state
                .app_state
                .ideation_session_repo
                .get_by_project(&pid)
                .await
                .map_err(|e| {
                    error!("Failed to list sessions for project {}: {}", project_id, e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({"error": "Internal server error"})),
                    )
                })?;
            all.into_iter().take(limit as usize).collect::<Vec<_>>()
        }
        Some(s @ ("active" | "accepted" | "archived")) => {
            let status_str = s.to_string();
            state
                .app_state
                .ideation_session_repo
                .get_by_project_and_status(pid.as_str(), &status_str, limit)
                .await
                .map_err(|e| {
                    error!("Failed to list sessions by status for project {}: {}", project_id, e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({"error": "Internal server error"})),
                    )
                })?
        }
        Some(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid status filter. Valid values: active, accepted, archived, all"
                })),
            ));
        }
    };

    // Build summaries with proposal counts
    let mut summaries = Vec::with_capacity(sessions.len());
    for session in &sessions {
        let proposal_count = state
            .app_state
            .task_proposal_repo
            .count_by_session(&session.id)
            .await
            .map_err(|e| {
                error!("Failed to count proposals for session {}: {}", session.id.as_str(), e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "Internal server error"})),
                )
            })?;
        summaries.push(SessionSummary {
            id: session.id.to_string(),
            title: session.title.clone(),
            status: session.status.to_string(),
            proposal_count,
            created_at: session.created_at.to_rfc3339(),
        });
    }

    Ok(Json(ListSessionsResponse { sessions: summaries }))
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
    let mut transition_service_builder = crate::application::TaskTransitionService::new(
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
    ));

    if let Some(ref pub_) = state.app_state.webhook_publisher {
        transition_service_builder = transition_service_builder.with_webhook_publisher_for_emitter(std::sync::Arc::clone(pub_));
    }

    let transition_service = transition_service_builder
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

    let mut transition_service_builder = crate::application::TaskTransitionService::new(
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
    ));

    if let Some(ref pub_) = state.app_state.webhook_publisher {
        transition_service_builder = transition_service_builder.with_webhook_publisher_for_emitter(std::sync::Arc::clone(pub_));
    }

    let transition_service = transition_service_builder
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

// ============================================================================
// External apply_proposals endpoint (D5 — closes external MCP bypass gap)
// ============================================================================

/// Request body for `POST /api/external/apply_proposals`.
///
/// Maps to [`ApplyProposalsInput`] used by the Tauri IPC path. The `target_column`
/// defaults to `"auto"` so task status is determined from dependency graph automatically.
#[derive(Debug, Deserialize)]
pub struct ExternalApplyProposalsRequest {
    pub session_id: String,
    pub proposal_ids: Vec<String>,
    /// Controls initial task placement. Use `"auto"` (default) to derive status from
    /// the dependency graph: tasks with no blockers → Ready, with blockers → Blocked.
    #[serde(default = "external_apply_default_column")]
    pub target_column: String,
    /// Per-plan override for feature branch usage. `None` uses the project default.
    #[serde(default)]
    pub use_feature_branch: Option<bool>,
    /// Per-plan override for the base branch. External callers can specify a custom branch;
    /// the backend validates it exists locally (see apply_proposals_core).
    #[serde(default)]
    pub base_branch_override: Option<String>,
}

fn external_apply_default_column() -> String {
    "auto".to_string()
}

impl From<ExternalApplyProposalsRequest> for ApplyProposalsInput {
    fn from(req: ExternalApplyProposalsRequest) -> Self {
        Self {
            session_id: req.session_id,
            proposal_ids: req.proposal_ids,
            target_column: req.target_column,
            use_feature_branch: req.use_feature_branch,
            base_branch_override: req.base_branch_override,
        }
    }
}

/// Response body for `POST /api/external/apply_proposals`.
#[derive(Debug, Serialize)]
pub struct ExternalApplyProposalsResponse {
    pub created_task_ids: Vec<String>,
    /// Number of proposal-to-proposal dependency edges created (excludes merge task edges).
    pub dependencies_created: usize,
    /// Number of plan tasks created (excludes the auto-generated merge task).
    pub tasks_created: usize,
    /// Human-readable summary of the finalization result.
    pub message: Option<String>,
    pub warnings: Vec<String>,
    pub session_converted: bool,
    pub execution_plan_id: Option<String>,
}

/// POST /api/external/apply_proposals
///
/// Apply accepted proposals to the Kanban board from the external MCP path.
///
/// Enforces:
/// 1. **Project scope** — the caller's API key must have access to the session's project.
/// 2. **Verification gate** — the plan must pass `check_verification_gate` before
///    proposals are accepted. Full enforcement requires Wave 1 schema migration.
///
/// Unlike the Tauri IPC path (`apply_proposals_to_kanban`), this endpoint does **not**
/// trigger the task scheduler. External agents poll
/// `GET /api/external/pipeline/:project_id` to monitor when tasks become Ready.
pub async fn external_apply_proposals(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Json(req): Json<ExternalApplyProposalsRequest>,
) -> Result<Json<ExternalApplyProposalsResponse>, HttpError> {
    let session_id = IdeationSessionId::from_string(req.session_id.clone());

    // Fetch session to verify project scope and verification gate
    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|e| {
            error!("Failed to get session {}: {}", req.session_id, e);
            HttpError::from(StatusCode::INTERNAL_SERVER_ERROR)
        })?
        .ok_or_else(|| HttpError::from(StatusCode::NOT_FOUND))?;

    // Enforce project scope: API key must have access to session's project
    session.assert_project_scope(&scope)?;

    // Enforce verification gate: plan must be verified before external acceptance
    let ideation_settings = state
        .app_state
        .ideation_settings_repo
        .get_settings()
        .await
        .map_err(|e| {
            error!("Failed to get ideation settings: {}", e);
            HttpError::from(StatusCode::INTERNAL_SERVER_ERROR)
        })?;
    check_verification_gate(&session, &ideation_settings)
        .map_err(|e| HttpError::validation(e.to_string()))?;

    // Apply proposals — no scheduler trigger (external agents poll get_pipeline_overview)
    let result = apply_proposals_core(&state.app_state, req.into())
        .await
        .map_err(|e| {
            error!("apply_proposals_core failed: {}", e);
            HttpError::validation(e.to_string())
        })?;

    // IPR cleanup — stop the ideation session's interactive CLI process (if any)
    if result.session_converted {
        let task_cleanup = TaskCleanupService::new(
            Arc::clone(&state.app_state.task_repo),
            Arc::clone(&state.app_state.project_repo),
            Arc::clone(&state.app_state.running_agent_registry),
            None, // No AppHandle in HTTP context
        )
        .with_interactive_process_registry(Arc::clone(
            &state.app_state.interactive_process_registry,
        ));

        let stopped = task_cleanup
            .stop_ideation_session_agent(&result.session_id)
            .await;
        if !stopped {
            tracing::warn!(
                session_id = %result.session_id,
                "IPR cleanup: no running process found for accepted session (HTTP path)"
            );
        }
    }

    tracing::info!(
        session_id = %session_id.as_str(),
        created = result.created_task_ids.len(),
        "External apply_proposals completed"
    );

    Ok(Json(ExternalApplyProposalsResponse {
        created_task_ids: result.created_task_ids,
        dependencies_created: result.dependencies_created,
        tasks_created: result.tasks_created,
        message: result.message,
        warnings: result.warnings,
        session_converted: result.session_converted,
        execution_plan_id: result.execution_plan_id,
    }))
}

/// POST /api/external/ideation_message
/// Send a message to an active ideation session.
///
/// Tri-state delivery:
/// 1. "sent"    — interactive process is open; message written directly to stdin
/// 2. "queued"  — agent is running but has no open stdin; message queued for resume
/// 3. "spawned" — no agent running; new agent process is spawned with the message
pub async fn ideation_message_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Json(req): Json<IdeationMessageRequest>,
) -> Result<Json<IdeationMessageResponse>, (StatusCode, Json<serde_json::Value>)> {
    let session_id = IdeationSessionId::from_string(req.session_id.clone());

    // Validate session exists
    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|e| {
            error!("Failed to get ideation session {}: {}", session_id.as_str(), e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Failed to get ideation session"})))
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Session not found"}))))?;

    // Enforce project scope
    session.assert_project_scope(&scope).map_err(|e| {
        (e.status, Json(serde_json::json!({"error": e.message.unwrap_or_default()})))
    })?;

    // Enforce Active status
    if session.status != crate::domain::entities::ideation::IdeationSessionStatus::Active {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Session is not active"}))));
    }

    let session_id_str = session_id.as_str().to_string();

    // Capture current phase for fire-and-forget transition logic later
    let current_phase = session.external_activity_phase.clone();

    // Helper: fire-and-forget 'created' → 'planning' phase transition
    let maybe_transition_to_planning = |repo: Arc<dyn crate::domain::repositories::IdeationSessionRepository>, sid: IdeationSessionId, phase: Option<String>| {
        if phase.as_deref() == Some("created") {
            tokio::spawn(async move {
                if let Err(e) = repo.update_external_activity_phase(&sid, "planning").await {
                    error!("Failed to set activity phase 'planning' for session {}: {}", sid.as_str(), e);
                }
            });
        }
    };

    // Read-before-write guard: external sessions must read agent responses before sending
    if session.origin == SessionOrigin::External {
        let last_read = session.external_last_read_message_id.as_deref();
        match state
            .app_state
            .chat_message_repo
            .count_unread_assistant_messages(&session_id_str, last_read)
            .await
        {
            Ok(unread_count) if unread_count > 0 => {
                return Err((
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({
                        "error": "unread_messages",
                        "unread_count": unread_count,
                        "hint": format!(
                            "You have {} unread agent response(s). Call v1_get_ideation_messages to read them before sending another message.",
                            unread_count
                        ),
                        "next_action": "fetch_messages"
                    })),
                ));
            }
            Ok(_) => {} // No unread messages, allow through
            Err(e) => {
                error!(
                    "Failed to count unread messages for session {}: {}",
                    session_id_str, e
                );
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "Failed to check message read status"})),
                ));
            }
        }
    }

    // Try 1: write directly to open interactive process (agent in multi-turn mode)
    let ipr_key = crate::application::InteractiveProcessKey {
        context_type: "ideation".to_string(),
        context_id: session_id_str.clone(),
    };
    if state
        .app_state
        .interactive_process_registry
        .has_process(&ipr_key)
        .await
    {
        match state
            .app_state
            .interactive_process_registry
            .write_message(&ipr_key, &req.message)
            .await
        {
            Ok(()) => {
                maybe_transition_to_planning(
                    Arc::clone(&state.app_state.ideation_session_repo),
                    IdeationSessionId::from_string(session_id_str.clone()),
                    current_phase,
                );
                return Ok(Json(IdeationMessageResponse {
                    status: "sent".to_string(),
                    session_id: session_id_str,
                    next_action: "poll_status".to_string(),
                    hint: Some("Wait for agent to respond. Poll v1_get_ideation_status (5-10s interval)".to_string()),
                }));
            }
            Err(e) => {
                // Process may have closed between has_process and write_message; fall through
                error!(
                    "Failed to write to interactive process for session {}: {}",
                    session_id_str, e
                );
            }
        }
    }

    // Try 2: queue message if agent is running (will be delivered on next resume)
    let agent_key =
        crate::domain::services::running_agent_registry::RunningAgentKey::new("ideation", &session_id_str);
    if state
        .app_state
        .running_agent_registry
        .is_running(&agent_key)
        .await
    {
        // Queue depth cap: prevent flooding when agent is busy (generating).
        // Bypass for "sent" (interactive process) and "spawned" (no agent) since those deliver immediately.
        let cap = crate::infrastructure::agents::claude::external_mcp_config()
            .external_message_queue_cap as usize;
        let queued_count = state
            .app_state
            .message_queue
            .count_for_context("ideation", &session_id_str);
        if queued_count >= cap {
            return Err((
                axum::http::StatusCode::TOO_MANY_REQUESTS,
                Json(serde_json::json!({
                    "error": "queue_full",
                    "queued_count": queued_count,
                    "hint": format!(
                        "Message queue is full ({queued_count} pending). Wait for the agent to process messages. Poll v1_get_ideation_status."
                    ),
                    "next_action": "poll_status"
                })),
            ));
        }

        state
            .app_state
            .message_queue
            .queue(ChatContextType::Ideation, &session_id_str, req.message.clone());
        maybe_transition_to_planning(
            Arc::clone(&state.app_state.ideation_session_repo),
            IdeationSessionId::from_string(session_id_str.clone()),
            current_phase,
        );
        return Ok(Json(IdeationMessageResponse {
            status: "queued".to_string(),
            session_id: session_id_str,
            next_action: "poll_status".to_string(),
            hint: Some("Wait for agent to respond. Poll v1_get_ideation_status (5-10s interval)".to_string()),
        }));
    }

    // Try 3: spawn a new agent
    let chat_service = build_chat_service(&state.app_state, &state.execution_state);

    let send_result = chat_service
        .send_message(
            ChatContextType::Ideation,
            &session_id_str,
            &req.message,
            SendMessageOptions {
                is_external_mcp: true,
                ..Default::default()
            },
        )
        .await;

    match send_result {
        Ok(result) if result.was_queued => {
            maybe_transition_to_planning(
                Arc::clone(&state.app_state.ideation_session_repo),
                IdeationSessionId::from_string(session_id_str.clone()),
                current_phase,
            );
            return Ok(Json(IdeationMessageResponse {
                status: "queued".to_string(),
                session_id: session_id_str,
                next_action: "poll_status".to_string(),
                hint: Some("Wait for agent to respond. Poll v1_get_ideation_status (5-10s interval)".to_string()),
            }));
        }
        Ok(_) => {}
        Err(e) => {
            error!("Failed to send message to ideation session {}: {}", session_id_str, e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Failed to send message"}))));
        }
    }

    maybe_transition_to_planning(
        Arc::clone(&state.app_state.ideation_session_repo),
        IdeationSessionId::from_string(session_id_str.clone()),
        current_phase,
    );
    Ok(Json(IdeationMessageResponse {
        status: "spawned".to_string(),
        session_id: session_id_str,
        next_action: "poll_status".to_string(),
        hint: Some("Wait for agent to respond. Poll v1_get_ideation_status (5-10s interval)".to_string()),
    }))
}

// ============================================================================
// trigger_verification_http + get_plan_verification_external_http
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct TriggerVerificationRequest {
    pub session_id: String,
}

#[derive(Debug, Serialize)]
pub struct TriggerVerificationResponse {
    pub status: String, // "triggered" | "already_running" | "no_plan"
    pub session_id: String,
}

/// A single verification gap in the external API response
#[derive(Debug, Serialize)]
pub struct ExternalGapDetail {
    pub severity: String,
    pub category: String,
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct ExternalVerificationResponse {
    pub status: String,
    pub in_progress: bool,
    pub round: Option<u32>,
    pub max_rounds: Option<u32>,
    pub gap_count: Option<u32>,
    pub gap_score: Option<u32>,
    #[serde(default)]
    pub gaps: Vec<ExternalGapDetail>,
    pub convergence_reason: Option<String>,
}

/// POST /api/external/trigger_verification
pub async fn trigger_verification_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Json(req): Json<TriggerVerificationRequest>,
) -> Result<Json<TriggerVerificationResponse>, StatusCode> {
    use crate::infrastructure::sqlite::sqlite_ideation_session_repo::SqliteIdeationSessionRepository as SessionRepo;

    let session_id = req.session_id.clone();
    let session_id_obj = IdeationSessionId::from_string(session_id.clone());

    // Load session
    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id_obj)
        .await
        .map_err(|e| {
            error!("Failed to load session {}: {}", session_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Scope check
    session.assert_project_scope(&scope).map_err(|e| e.status)?;

    // No-plan check: neither own plan nor inherited
    if session.plan_artifact_id.is_none() && session.inherited_plan_artifact_id.is_none() {
        return Ok(Json(TriggerVerificationResponse {
            status: "no_plan".to_string(),
            session_id,
        }));
    }

    // CAS: atomically trigger auto_verify_sync
    let sid_for_trigger = session_id.clone();
    let generation_opt = state
        .app_state
        .db
        .run(move |conn| SessionRepo::trigger_auto_verify_sync(conn, &sid_for_trigger))
        .await
        .map_err(|e| {
            error!("trigger_auto_verify_sync failed for session {}: {}", session_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let Some(generation) = generation_opt else {
        return Ok(Json(TriggerVerificationResponse {
            status: "already_running".to_string(),
            session_id,
        }));
    };

    // Spawn verifier; reset on failure
    let cfg = verification_config();
    if let Some(app_handle) = &state.app_state.app_handle {
        emit_verification_started(app_handle, &session_id, generation, cfg.max_rounds);
    }
    let title = format!("Auto-verification (gen {generation})");
    let description = format!(
        "Run verification round loop. parent_session_id: {session_id}, generation: {generation}, max_rounds: {}",
        cfg.max_rounds
    );
    match crate::http_server::handlers::session_linking::create_verification_child_session(
        &state,
        &session_id,
        &description,
        &title,
    )
    .await
    {
        Ok(true) => {} // orchestration triggered — success
        Ok(false) | Err(_) => {
            error!(
                "Verification agent failed to spawn for session {}",
                session_id
            );
            let sid_reset = session_id.clone();
            if let Err(reset_err) = state
                .app_state
                .db
                .run(move |conn| SessionRepo::reset_auto_verify_sync(conn, &sid_reset))
                .await
            {
                error!(
                    "Failed to reset auto-verify state for session {} after spawn failure: {}",
                    session_id, reset_err
                );
            } else if let Some(app_handle) = &state.app_state.app_handle {
                emit_verification_status_changed(
                    app_handle,
                    &session_id,
                    crate::domain::entities::VerificationStatus::Unverified,
                    false,
                    None,
                    Some("spawn_failed"),
                    Some(generation),
                );
            }
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    // Transition external activity phase to "verifying"
    {
        let repo = Arc::clone(&state.app_state.ideation_session_repo);
        let trigger_session_id = IdeationSessionId::from_string(session_id.clone());
        tokio::spawn(async move {
            if let Err(e) = repo.update_external_activity_phase(&trigger_session_id, "verifying").await {
                error!("Failed to set activity phase 'verifying' for session {}: {}", trigger_session_id.as_str(), e);
            }
        });
    }

    Ok(Json(TriggerVerificationResponse {
        status: "triggered".to_string(),
        session_id,
    }))
}

/// GET /api/external/plan_verification/:session_id
pub async fn get_plan_verification_external_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Path(session_id): Path<String>,
) -> Result<Json<ExternalVerificationResponse>, StatusCode> {
    use crate::domain::entities::ideation::VerificationMetadata;
    use crate::domain::services::gap_score;

    let session_id_obj = IdeationSessionId::from_string(session_id.clone());

    // Load session for scope check
    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id_obj)
        .await
        .map_err(|e| {
            error!("Failed to load session {}: {}", session_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Scope check
    session.assert_project_scope(&scope).map_err(|e| e.status)?;

    // Read verification state from session entity
    let status_str = session.verification_status.to_string();
    let in_progress = session.verification_in_progress;

    let metadata: Option<VerificationMetadata> = session
        .verification_metadata
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    let round = metadata
        .as_ref()
        .and_then(|m| if m.current_round > 0 { Some(m.current_round) } else { None });
    let max_rounds = metadata
        .as_ref()
        .and_then(|m| if m.max_rounds > 0 { Some(m.max_rounds) } else { None });
    let gap_count = metadata.as_ref().map(|m| gap_score(&m.current_gaps));
    let convergence_reason = metadata.as_ref().and_then(|m| m.convergence_reason.clone());
    let gaps: Vec<ExternalGapDetail> = metadata
        .as_ref()
        .map(|m| {
            m.current_gaps
                .iter()
                .map(|g| ExternalGapDetail {
                    severity: g.severity.clone(),
                    category: g.category.clone(),
                    description: g.description.clone(),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(Json(ExternalVerificationResponse {
        status: status_str,
        in_progress,
        round,
        max_rounds,
        gap_count,
        gap_score: gap_count,
        gaps,
        convergence_reason,
    }))
}

// ============================================================================
// Get ideation messages
// ============================================================================

/// A single message returned to external consumers.
#[derive(Debug, Serialize)]
pub struct IdeationMessageSummary {
    pub id: String,
    /// "user" or "assistant" (Orchestrator is mapped to "assistant")
    pub role: String,
    pub content: String,
    pub created_at: String,
}

/// Response for GET /api/external/ideation_messages/:session_id
#[derive(Debug, Serialize)]
pub struct GetIdeationMessagesResponse {
    pub messages: Vec<IdeationMessageSummary>,
    pub has_more: bool,
    /// "idle" | "generating" | "waiting_for_input"
    pub agent_status: String,
    pub next_action: String,
}

/// Query params for pagination.
#[derive(Debug, Deserialize)]
pub struct GetIdeationMessagesQuery {
    #[serde(default = "default_messages_limit")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
}

fn default_messages_limit() -> u32 {
    50
}

/// GET /api/external/ideation_messages/:session_id
///
/// Returns orchestrator and user messages for an ideation session.
/// Filter: User + Orchestrator roles only (Orchestrator → "assistant").
pub async fn get_ideation_messages_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Path(session_id): Path<String>,
    Query(params): Query<GetIdeationMessagesQuery>,
) -> Result<Json<GetIdeationMessagesResponse>, StatusCode> {
    use crate::domain::entities::ideation::MessageRole;

    let session_id = IdeationSessionId::from_string(session_id);

    // Load session and enforce scope
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

    session.assert_project_scope(&scope).map_err(|e| e.status)?;

    // Fetch limit+1 to detect has_more (SQL already filters User + Orchestrator roles)
    let fetch_limit = params.limit.saturating_add(1);
    let raw_messages = state
        .app_state
        .chat_message_repo
        .get_recent_by_session_paginated(&session_id, fetch_limit, params.offset)
        .await
        .map_err(|e| {
            error!("Failed to get messages for session {}: {}", session_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Determine has_more before truncating
    let has_more = raw_messages.len() > params.limit as usize;
    let messages_slice = if has_more {
        &raw_messages[..params.limit as usize]
    } else {
        &raw_messages[..]
    };

    // Role filter — User and Orchestrator only (SQL already does this, but be defensive)
    let messages: Vec<IdeationMessageSummary> = messages_slice
        .iter()
        .filter(|msg| matches!(msg.role, MessageRole::User | MessageRole::Orchestrator))
        .map(|msg| {
            let role = match msg.role {
                MessageRole::Orchestrator => "assistant".to_string(),
                _ => "user".to_string(),
            };
            IdeationMessageSummary {
                id: msg.id.to_string(),
                role,
                content: msg.content.clone(),
                created_at: msg.created_at.to_rfc3339(),
            }
        })
        .collect();

    // Fire-and-forget: update read cursor for external sessions after fetching messages
    if session.origin == SessionOrigin::External {
        if let Some(latest_msg) = messages.last() {
            let latest_id = latest_msg.id.clone();
            if let Err(e) = state
                .app_state
                .ideation_session_repo
                .update_external_last_read_message_id(&session_id, &latest_id)
                .await
            {
                error!(
                    "Failed to update external_last_read_message_id for session {}: {}",
                    session_id.as_str(),
                    e
                );
            }
        }
    }

    // Determine agent tri-state status
    let agent_status = determine_agent_status(
        state.app_state.running_agent_registry.as_ref(),
        &state.app_state.interactive_process_registry,
        session_id.as_str(),
    )
    .await;

    let next_action = match agent_status.as_str() {
        "waiting_for_input" => "send_message".to_string(),
        "generating" => "wait".to_string(),
        _ => "send_message".to_string(),
    };

    Ok(Json(GetIdeationMessagesResponse {
        messages,
        has_more,
        agent_status,
        next_action,
    }))
}

// ============================================================================
// Phase 3.2: Batch task status
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct BatchTaskStatusRequest {
    pub task_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct BatchTaskStatusItem {
    pub id: String,
    pub title: String,
    pub status: String,
    pub project_id: String,
}

#[derive(Debug, Serialize)]
pub struct BatchTaskStatusError {
    pub id: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct BatchTaskStatusResponse {
    pub tasks: Vec<BatchTaskStatusItem>,
    pub errors: Vec<BatchTaskStatusError>,
    pub requested_count: usize,
    pub returned_count: usize,
}

/// POST /api/external/tasks/batch_status
/// Batch lookup up to 50 task IDs.
/// Returns tasks array + errors array with reason: "not_found" | "access_denied"
pub async fn batch_task_status_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Json(req): Json<BatchTaskStatusRequest>,
) -> Result<Json<BatchTaskStatusResponse>, (StatusCode, String)> {
    if req.task_ids.len() > 50 {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Too many task IDs: {}. Maximum is 50.",
                req.task_ids.len()
            ),
        ));
    }

    let requested_count = req.task_ids.len();
    let mut tasks = Vec::new();
    let mut errors = Vec::new();

    for raw_id in &req.task_ids {
        let task_id = TaskId::from_string(raw_id.clone());
        match state.app_state.task_repo.get_by_id(&task_id).await {
            Ok(Some(task)) => {
                if task.assert_project_scope(&scope).is_err() {
                    errors.push(BatchTaskStatusError {
                        id: raw_id.clone(),
                        reason: "access_denied".to_string(),
                    });
                } else {
                    tasks.push(BatchTaskStatusItem {
                        id: task.id.to_string(),
                        title: task.title.clone(),
                        status: task.internal_status.to_string(),
                        project_id: task.project_id.to_string(),
                    });
                }
            }
            Ok(None) => {
                errors.push(BatchTaskStatusError {
                    id: raw_id.clone(),
                    reason: "not_found".to_string(),
                });
            }
            Err(e) => {
                error!("Failed to get task {}: {}", raw_id, e);
                errors.push(BatchTaskStatusError {
                    id: raw_id.clone(),
                    reason: "not_found".to_string(),
                });
            }
        }
    }

    let returned_count = tasks.len();
    Ok(Json(BatchTaskStatusResponse {
        tasks,
        errors,
        requested_count,
        returned_count,
    }))
}

// ============================================================================
// Webhook Registration Endpoints
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct RegisterWebhookRequest {
    pub url: String,
    #[serde(default)]
    pub event_types: Option<Vec<String>>,
    #[serde(default)]
    pub project_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct RegisterWebhookResponse {
    pub id: String,
    pub url: String,
    pub secret: String,
    pub event_types: Option<Vec<String>>,
    pub project_ids: Vec<String>,
    pub active: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct WebhookSummary {
    pub id: String,
    pub url: String,
    pub event_types: Option<Vec<String>>,
    pub project_ids: Vec<String>,
    pub active: bool,
    pub failure_count: i64,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ListWebhooksResponse {
    pub webhooks: Vec<WebhookSummary>,
}

#[derive(Debug, Serialize)]
pub struct UnregisterWebhookResponse {
    pub success: bool,
    pub id: String,
}

/// POST /api/external/webhooks/register — register a webhook URL
pub async fn register_webhook_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    headers: axum::http::HeaderMap,
    Json(req): Json<RegisterWebhookRequest>,
) -> Result<Json<RegisterWebhookResponse>, HttpError> {
    // Extract the API key ID from the X-RalphX-Key-Id header (injected by external MCP server)
    let api_key_id = headers
        .get(crate::http_server::handlers::external_auth::EXTERNAL_KEY_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_default();

    // Extract authorized project IDs from scope (empty means unrestricted)
    let authorized_project_ids: Vec<String> = scope
        .0
        .as_deref()
        .map(|ids| ids.iter().map(|id| id.to_string()).collect())
        .unwrap_or_default();

    let svc = crate::application::WebhookService::new(
        Arc::clone(&state.app_state.webhook_registration_repo),
    );

    let registration = svc
        .register(
            &api_key_id,
            &req.url,
            req.event_types,
            req.project_ids,
            &authorized_project_ids,
        )
        .await
        .map_err(|e| {
            error!("Failed to register webhook: {}", e);
            HttpError {
                status: axum::http::StatusCode::UNPROCESSABLE_ENTITY,
                message: Some(e.to_string()),
            }
        })?;

    let event_types: Option<Vec<String>> = registration
        .event_types
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());
    let project_ids: Vec<String> =
        serde_json::from_str(&registration.project_ids).unwrap_or_default();

    Ok(Json(RegisterWebhookResponse {
        id: registration.id,
        url: registration.url,
        secret: registration.secret,
        event_types,
        project_ids,
        active: registration.active,
        created_at: registration.created_at,
    }))
}

/// DELETE /api/external/webhooks/:id — unregister a webhook
pub async fn unregister_webhook_http(
    State(state): State<HttpServerState>,
    headers: axum::http::HeaderMap,
    Path(webhook_id): Path<String>,
) -> Result<Json<UnregisterWebhookResponse>, HttpError> {
    let api_key_id = headers
        .get(crate::http_server::handlers::external_auth::EXTERNAL_KEY_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_default();

    let svc = crate::application::WebhookService::new(
        Arc::clone(&state.app_state.webhook_registration_repo),
    );

    let found = svc
        .unregister(&webhook_id, &api_key_id)
        .await
        .map_err(|e| {
            error!("Failed to unregister webhook: {}", e);
            HttpError {
                status: axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                message: Some(e.to_string()),
            }
        })?;

    if !found {
        return Err(HttpError {
            status: axum::http::StatusCode::NOT_FOUND,
            message: Some("Webhook not found or not owned by this API key".to_string()),
        });
    }

    Ok(Json(UnregisterWebhookResponse {
        success: true,
        id: webhook_id,
    }))
}

/// GET /api/external/webhooks — list webhooks for this API key
pub async fn list_webhooks_http(
    State(state): State<HttpServerState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ListWebhooksResponse>, HttpError> {
    let api_key_id = headers
        .get(crate::http_server::handlers::external_auth::EXTERNAL_KEY_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_default();

    let svc = crate::application::WebhookService::new(
        Arc::clone(&state.app_state.webhook_registration_repo),
    );

    let registrations = svc.list(&api_key_id).await.map_err(|e| {
        error!("Failed to list webhooks: {}", e);
        HttpError {
            status: axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            message: Some(e.to_string()),
        }
    })?;

    let webhooks = registrations
        .into_iter()
        .map(|r| {
            let event_types: Option<Vec<String>> = r
                .event_types
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok());
            let project_ids: Vec<String> =
                serde_json::from_str(&r.project_ids).unwrap_or_default();
            WebhookSummary {
                id: r.id,
                url: r.url,
                event_types,
                project_ids,
                active: r.active,
                failure_count: r.failure_count,
                created_at: r.created_at,
            }
        })
        .collect();

    Ok(Json(ListWebhooksResponse { webhooks }))
}

/// GET /api/external/webhooks/health — delivery health stats per webhook
pub async fn get_webhook_health_http(
    State(state): State<HttpServerState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<WebhookHealthResponse>, HttpError> {
    let api_key_id = headers
        .get(crate::http_server::handlers::external_auth::EXTERNAL_KEY_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_default();

    let svc = crate::application::WebhookService::new(
        Arc::clone(&state.app_state.webhook_registration_repo),
    );

    let registrations = svc.list(&api_key_id).await.map_err(|e| {
        error!("Failed to get webhook health: {}", e);
        HttpError {
            status: axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            message: Some(e.to_string()),
        }
    })?;

    let webhooks = registrations
        .into_iter()
        .map(|r| WebhookHealthItem {
            id: r.id,
            url: r.url,
            active: r.active,
            failure_count: r.failure_count,
            last_failure_at: r.last_failure_at,
        })
        .collect();

    Ok(Json(WebhookHealthResponse { webhooks }))
}

#[derive(Debug, Serialize)]
pub struct WebhookHealthItem {
    pub id: String,
    pub url: String,
    pub active: bool,
    pub failure_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_failure_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WebhookHealthResponse {
    pub webhooks: Vec<WebhookHealthItem>,
}

// ============================================================================
// Task note
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateTaskNoteRequest {
    pub task_id: String,
    pub note: String,
}

#[derive(Debug, Serialize)]
pub struct TaskNoteResponse {
    pub task_id: String,
    pub success: bool,
}

/// POST /api/external/task-note
/// Add a progress note to a task. Proxies to add_task_note logic.
pub async fn create_task_note_http(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Json(req): Json<CreateTaskNoteRequest>,
) -> Result<Json<TaskNoteResponse>, StatusCode> {
    let task_id = crate::domain::entities::TaskId::from_string(req.task_id);

    let mut task = state
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

    let note_text = format!("\n\n---\n**Note:** {}", req.note);
    task.description = Some(match task.description {
        Some(existing) => format!("{}{}", existing, note_text),
        None => note_text,
    });

    state.app_state.task_repo.update(&task).await.map_err(|e| {
        error!("Failed to update task {}: {}", task_id.as_str(), e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(TaskNoteResponse {
        task_id: task.id.to_string(),
        success: true,
    }))
}
