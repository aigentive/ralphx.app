// Tauri commands for active plan operations
// Thin layer that delegates to ActivePlanRepository

use tauri::State;

use crate::application::AppState;
use crate::domain::entities::{IdeationSessionId, ProjectId};

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

    // Record selection in plan_selection_stats
    state
        .active_plan_repo
        .record_selection(&project_id, &ideation_session_id, &source)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
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
