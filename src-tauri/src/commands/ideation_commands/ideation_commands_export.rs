// Export/import Tauri commands for ideation sessions

use ralphx_domain::entities::EventType;
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

    let payload_json = serde_json::json!({
        "sessionId": result.session_id,
        "projectId": input.project_id,
    });
    app_handle
        .emit("ideation:session_created", &payload_json)
        .ok();
    app_handle
        .emit("ideation:session_imported", &result.session_id)
        .ok();

    // Layer 2: persist to external_events table (non-fatal)
    if let Err(e) = app_state
        .external_events_repo
        .insert_event(
            "ideation:session_created",
            &input.project_id,
            &payload_json.to_string(),
        )
        .await
    {
        tracing::warn!(error = %e, "Failed to persist IdeationSessionCreated event");
    }

    // Layer 3: webhook push (fire-and-forget, non-fatal)
    if let Some(ref publisher) = app_state.webhook_publisher {
        let _ = publisher
            .publish(
                EventType::IdeationSessionCreated,
                &input.project_id,
                payload_json,
            )
            .await;
    }

    Ok(result)
}
