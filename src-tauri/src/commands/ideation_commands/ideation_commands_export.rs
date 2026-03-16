// Export/import Tauri commands for ideation sessions

use serde::Deserialize;
use tauri::{Emitter, State};

use crate::application::{AppState, ImportedSession, SessionExportService};

// ============================================================================
// Input types
// ============================================================================

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSessionInput {
    pub json_content: String,
    pub project_id: String,
}

// ============================================================================
// Commands
// ============================================================================

/// Export a complete ideation session as a JSON string.
/// Returns the serialized SessionExport for the caller to save to disk.
#[tauri::command]
pub async fn export_ideation_session(
    id: String,
    project_id: String,
    app_state: State<'_, AppState>,
) -> Result<String, String> {
    let service = SessionExportService::new(app_state.db.clone());
    let export = service
        .export(&id, &project_id)
        .await
        .map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&export).map_err(|e| e.to_string())
}

/// Import an ideation session from JSON content into the given project.
/// Emits ideation:session_created and ideation:session_imported events on success.
#[tauri::command]
pub async fn import_ideation_session(
    input: ImportSessionInput,
    app_handle: tauri::AppHandle,
    app_state: State<'_, AppState>,
) -> Result<ImportedSession, String> {
    let service = SessionExportService::new(app_state.db.clone());
    let result = service
        .import(&input.json_content, &input.project_id)
        .await
        .map_err(|e| e.to_string())?;

    app_handle
        .emit("ideation:session_created", &result.session_id)
        .ok();
    app_handle
        .emit("ideation:session_imported", &result.session_id)
        .ok();

    Ok(result)
}
