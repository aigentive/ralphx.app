use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use super::*;
use crate::domain::entities::{InternalStatus, ProjectId, TaskId};

pub async fn list_tasks(
    State(state): State<HttpServerState>,
    Json(req): Json<ListTasksRequest>,
) -> Result<Json<ListTasksResponse>, StatusCode> {
    let project_id = ProjectId::from_string(req.project_id);

    // Parse optional status filter
    let status_filter = req
        .status
        .as_ref()
        .map(|s| parse_internal_status(s.as_str()))
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Get all tasks for project
    let mut tasks = state
        .app_state
        .task_repo
        .get_by_project(&project_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Apply status filter if provided
    if let Some(status) = status_filter {
        tasks.retain(|t| t.internal_status == status);
    }

    // Convert to response
    let task_responses: Vec<_> = tasks.iter().map(task_to_response).collect();

    Ok(Json(ListTasksResponse {
        tasks: task_responses,
    }))
}

pub async fn suggest_task(
    State(state): State<HttpServerState>,
    Json(req): Json<SuggestTaskRequest>,
) -> Result<Json<SuggestTaskResponse>, StatusCode> {
    let project_id = ProjectId::from_string(req.project_id);

    // Get all backlog tasks for the project
    let tasks = state
        .app_state
        .task_repo
        .get_by_project(&project_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let backlog_tasks: Vec<_> = tasks
        .into_iter()
        .filter(|t| t.internal_status == InternalStatus::Backlog)
        .collect();

    if backlog_tasks.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Find highest priority task (higher i32 = higher priority)
    let suggested = backlog_tasks
        .iter()
        .max_by_key(|t| t.priority)
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(SuggestTaskResponse {
        task: task_to_response(suggested),
    }))
}

// ============================================================================
// Project Analysis Endpoints
// ============================================================================

/// Single analysis entry (path-scoped build/validation commands)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisEntry {
    pub path: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub validate: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub worktree_setup: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct AnalysisResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_secs: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entries: Option<Vec<AnalysisEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analyzed_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AnalysisQueryParams {
    pub task_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SaveAnalysisRequest {
    pub entries: Vec<AnalysisEntry>,
}

/// GET /api/projects/:id/analysis?task_id=
///
/// Returns project analysis data with resolved template variables.
/// If no analysis exists, returns { status: "analyzing", retry_after_secs: 30 }.
pub async fn get_project_analysis(
    State(state): State<HttpServerState>,
    Path(project_id): Path<String>,
    Query(params): Query<AnalysisQueryParams>,
) -> Result<Json<AnalysisResponse>, StatusCode> {
    let project_id = ProjectId::from_string(project_id);

    let project = state
        .app_state
        .project_repo
        .get_by_id(&project_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check if analysis has been run
    if project.analyzed_at.is_none() && project.custom_analysis.is_none() {
        return Ok(Json(AnalysisResponse {
            status: "analyzing".to_string(),
            retry_after_secs: Some(30),
            entries: None,
            analyzed_at: None,
        }));
    }

    // Merge strategy: custom_analysis wins if set, else detected_analysis
    let analysis_json = project
        .custom_analysis
        .as_ref()
        .or(project.detected_analysis.as_ref());

    let entries: Vec<AnalysisEntry> = match analysis_json {
        Some(json) => serde_json::from_str(json).unwrap_or_default(),
        None => Vec::new(),
    };

    // Resolve template variables if task_id provided
    let resolved_entries = if let Some(task_id_str) = &params.task_id {
        let task_id = TaskId::from_string(task_id_str.to_string());
        let task = state
            .app_state
            .task_repo
            .get_by_id(&task_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let worktree_path = task
            .as_ref()
            .and_then(|t| t.worktree_path.as_deref())
            .unwrap_or("");
        let task_branch = task
            .as_ref()
            .and_then(|t| t.task_branch.as_deref())
            .unwrap_or("");

        resolve_template_vars(entries, &project.working_directory, worktree_path, task_branch)
    } else {
        resolve_template_vars(entries, &project.working_directory, "", "")
    };

    Ok(Json(AnalysisResponse {
        status: "ready".to_string(),
        retry_after_secs: None,
        entries: Some(resolved_entries),
        analyzed_at: project.analyzed_at,
    }))
}

/// POST /api/projects/:id/analysis
///
/// Saves detected analysis data. Updates detected_analysis + analyzed_at.
/// Never touches custom_analysis.
pub async fn save_project_analysis(
    State(state): State<HttpServerState>,
    Path(project_id): Path<String>,
    Json(req): Json<SaveAnalysisRequest>,
) -> Result<Json<SuccessResponse>, StatusCode> {
    let project_id = ProjectId::from_string(project_id);

    let mut project = state
        .app_state
        .project_repo
        .get_by_id(&project_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Update detected_analysis (never touch custom_analysis)
    let entries_json = serde_json::to_string(&req.entries)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    project.detected_analysis = Some(entries_json);
    project.analyzed_at = Some(chrono::Utc::now().to_rfc3339());
    project.touch();

    state
        .app_state
        .project_repo
        .update(&project)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(SuccessResponse {
        success: true,
        message: format!("Saved {} analysis entries", req.entries.len()),
    }))
}

/// Resolve template variables in analysis entries
fn resolve_template_vars(
    entries: Vec<AnalysisEntry>,
    project_root: &str,
    worktree_path: &str,
    task_branch: &str,
) -> Vec<AnalysisEntry> {
    entries
        .into_iter()
        .map(|entry| {
            let resolve = |s: String| -> String {
                s.replace("{project_root}", project_root)
                    .replace("{worktree_path}", worktree_path)
                    .replace("{task_branch}", task_branch)
            };
            AnalysisEntry {
                path: resolve(entry.path),
                label: entry.label,
                install: entry.install.map(resolve),
                validate: entry.validate.into_iter().map(resolve).collect(),
                worktree_setup: entry.worktree_setup.into_iter().map(resolve).collect(),
            }
        })
        .collect()
}
