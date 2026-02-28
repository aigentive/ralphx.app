// Tauri commands for active plan operations
// Thin layer that delegates to ActivePlanRepository

use chrono::Utc;
use serde::Serialize;
use tauri::State;

use crate::application::compute_final_score;
use crate::application::AppState;
use crate::domain::entities::{
    IdeationSessionId, IdeationSessionStatus, InternalStatus, ProjectId,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanTaskStatsResponse {
    pub total: u32,
    pub incomplete: u32,
    pub active_now: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanInteractionStatsResponse {
    pub selected_count: u32,
    pub last_selected_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanSelectorCandidateResponse {
    pub session_id: String,
    pub title: Option<String>,
    pub accepted_at: String,
    pub task_stats: PlanTaskStatsResponse,
    pub interaction_stats: PlanInteractionStatsResponse,
    pub score: f64,
}

/// Get the active plan (ideation session ID) for a project
#[tauri::command]
pub async fn get_active_plan(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let project_id = ProjectId::from_string(project_id);

    state
        .active_plan_repo
        .get(&project_id)
        .await
        .map(|opt| opt.map(|id| id.as_str().to_string()))
        .map_err(|e| e.to_string())
}

/// Set the active plan for a project
/// Validates that the session exists, belongs to the project, and is accepted
#[tauri::command]
pub async fn set_active_plan(
    project_id: String,
    ideation_session_id: String,
    source: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let project_id = ProjectId::from_string(project_id.clone());
    let ideation_session_id = IdeationSessionId::from_string(ideation_session_id.clone());

    // Validate and set the active plan
    state
        .active_plan_repo
        .set(&project_id, &ideation_session_id)
        .await
        .map_err(|e| e.to_string())?;

    // Auto-derive and store execution_plan_id from the active session
    if let Ok(Some(ep)) = state
        .execution_plan_repo
        .get_active_for_session(&ideation_session_id)
        .await
    {
        let _ = state
            .active_plan_repo
            .set_execution_plan_id(&project_id, &ep.id)
            .await;
    }

    // Record selection in plan_selection_stats
    state
        .active_plan_repo
        .record_selection(&project_id, &ideation_session_id, &source)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Get the active execution plan ID for a project
#[tauri::command]
pub async fn get_active_execution_plan(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let project_id = ProjectId::from_string(project_id);
    state
        .active_plan_repo
        .get_execution_plan_id(&project_id)
        .await
        .map(|opt| opt.map(|id| id.as_str().to_string()))
        .map_err(|e| e.to_string())
}

/// Clear the active plan for a project
#[tauri::command]
pub async fn clear_active_plan(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let project_id = ProjectId::from_string(project_id);

    state
        .active_plan_repo
        .clear(&project_id)
        .await
        .map_err(|e| e.to_string())
}

/// List accepted plan candidates for selectors (Kanban/Graph/Quick Switcher)
#[tauri::command]
pub async fn list_plan_selector_candidates(
    project_id: String,
    query: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<PlanSelectorCandidateResponse>, String> {
    let project_id = ProjectId::from_string(project_id);
    let search = query
        .as_deref()
        .map(str::trim)
        .filter(|q| !q.is_empty())
        .map(|q| q.to_lowercase());

    let sessions = state
        .ideation_session_repo
        .get_by_project(&project_id)
        .await
        .map_err(|e| e.to_string())?;

    let accepted_sessions: Vec<_> = sessions
        .into_iter()
        .filter(|s| s.status == IdeationSessionStatus::Accepted)
        .filter(|s| {
            if let Some(ref term) = search {
                s.title
                    .as_deref()
                    .unwrap_or("Untitled Plan")
                    .to_lowercase()
                    .contains(term)
            } else {
                true
            }
        })
        .collect();

    let mut candidates = Vec::with_capacity(accepted_sessions.len());
    let now = Utc::now();

    for session in accepted_sessions {
        let tasks = state
            .task_repo
            .get_by_ideation_session(&session.id)
            .await
            .map_err(|e| e.to_string())?;

        let total = tasks.len() as u32;
        let incomplete = tasks
            .iter()
            .filter(|t| {
                !matches!(
                    t.internal_status,
                    InternalStatus::Approved
                        | InternalStatus::Merged
                        | InternalStatus::Failed
                        | InternalStatus::Cancelled
                )
            })
            .count() as u32;
        let active_now = tasks
            .iter()
            .filter(|t| {
                matches!(
                    t.internal_status,
                    InternalStatus::Executing
                        | InternalStatus::ReExecuting
                        | InternalStatus::QaRefining
                        | InternalStatus::QaTesting
                        | InternalStatus::Reviewing
                        | InternalStatus::PendingMerge
                        | InternalStatus::Merging
                )
            })
            .count() as u32;

        let stats = state
            .plan_selection_stats_repo
            .get_stats(&project_id, &session.id)
            .await
            .map_err(|e| e.to_string())?;

        let selected_count = stats.as_ref().map_or(0, |s| s.selected_count);
        let last_selected_at_dt = stats.as_ref().and_then(|s| s.last_selected_at);
        let last_selected_at = last_selected_at_dt.map(|dt| dt.to_rfc3339());
        let accepted_at = session.converted_at.unwrap_or(session.updated_at);

        let score = compute_final_score(
            selected_count,
            last_selected_at_dt,
            active_now,
            incomplete,
            total,
            accepted_at,
            now,
        );

        candidates.push(PlanSelectorCandidateResponse {
            session_id: session.id.as_str().to_string(),
            title: session.title,
            accepted_at: accepted_at.to_rfc3339(),
            task_stats: PlanTaskStatsResponse {
                total,
                incomplete,
                active_now,
            },
            interaction_stats: PlanInteractionStatsResponse {
                selected_count,
                last_selected_at,
            },
            score,
        });
    }

    candidates.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(candidates)
}
