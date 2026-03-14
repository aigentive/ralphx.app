// Cross-project session creation command
//
// Creates a new ideation session in a target project by inheriting a verified plan
// from a source session in another project on the same RalphX instance.

use tauri::{Emitter, State};

use crate::application::AppState;
use crate::domain::entities::{
    IdeationSession, IdeationSessionId, IdeationSessionStatus, ProjectId, VerificationStatus,
};
use crate::domain::services::validate_project_path;
use crate::infrastructure::sqlite::SqliteIdeationSessionRepository;

use super::ideation_commands_types::{CreateCrossProjectSessionInput, IdeationSessionResponse};

// ============================================================================
// Core Implementation
// ============================================================================

/// Core implementation for creating a cross-project ideation session.
/// Generic over Runtime to enable unit testing with MockRuntime.
///
/// Logic:
/// 1. Validate the target project path (security check).
/// 2. Resolve target project: query by path, auto-create if not found.
/// 3. Fetch source session, validate its plan is verified.
/// 4. Resolve artifact: prefer plan_artifact_id, fall back to inherited_plan_artifact_id.
/// 5. Validate no circular import (TOCTOU-safe: done inside the INSERT db.run closure).
/// 6. Insert the new session with ImportedVerified status and source tracking fields.
/// 7. Emit events and return response.
pub(crate) async fn create_cross_project_session_impl<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    state: &AppState,
    input: CreateCrossProjectSessionInput,
) -> Result<IdeationSessionResponse, String> {
    tracing::info!(
        target_path = %input.target_project_path,
        source_session_id = %input.source_session_id,
        "Creating cross-project session"
    );

    // 1. Validate the target path (canonicalize, blocklist, home dir check)
    let canonical = validate_project_path(&input.target_project_path)
        .map_err(|e| e.to_string())?;
    let canonical_str = canonical.to_string_lossy().to_string();

    // 2. Resolve target project — query by working_directory, auto-create if not found
    let target_project = match state
        .project_repo
        .get_by_working_directory(&canonical_str)
        .await
        .map_err(|e| e.to_string())?
    {
        Some(project) => project,
        None => {
            // Validate the directory exists on disk before auto-creating
            if !canonical.exists() {
                return Err(format!(
                    "Target project path does not exist on disk: {canonical_str}"
                ));
            }

            // Auto-create: name = directory basename
            let name = canonical
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unnamed Project".to_string());

            tracing::info!(path = %canonical_str, name = %name, "Auto-creating target project for cross-project session");

            let new_project = crate::domain::entities::Project::new(name, canonical_str.clone());
            let project_id_str = new_project.id.as_str().to_string();

            let created = state
                .project_repo
                .create(new_project)
                .await
                .map_err(|e| e.to_string())?;

            // Emit project:created event for real-time UI updates
            let _ = app.emit(
                "project:created",
                serde_json::json!({
                    "projectId": project_id_str,
                    "workingDirectory": canonical_str,
                }),
            );

            created
        }
    };

    let target_project_id = target_project.id.as_str().to_string();

    // 3. Fetch source session and validate its plan is verified
    let source_session_id_entity = IdeationSessionId::from_string(input.source_session_id.clone());
    let source_session = state
        .ideation_session_repo
        .get_by_id(&source_session_id_entity)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Source session not found: {}", input.source_session_id))?;

    let is_verified = matches!(
        source_session.verification_status,
        VerificationStatus::Verified
            | VerificationStatus::Skipped
            | VerificationStatus::ImportedVerified
    );
    if !is_verified {
        return Err(format!(
            "Source session plan is not verified (status: {}). \
             Only Verified, Skipped, or ImportedVerified plans can be exported.",
            source_session.verification_status
        ));
    }

    // 4. Resolve artifact: prefer own plan_artifact_id, fall back to inherited_plan_artifact_id
    let artifact_id = source_session
        .plan_artifact_id
        .clone()
        .or_else(|| source_session.inherited_plan_artifact_id.clone())
        .ok_or_else(|| {
            format!(
                "Source session has no plan artifact to inherit (session: {})",
                input.source_session_id
            )
        })?;

    // Build the new session entity
    let new_session_id = IdeationSessionId::new();
    let mut builder = IdeationSession::builder()
        .id(new_session_id)
        .project_id(ProjectId::from_string(target_project_id.clone()))
        .inherited_plan_artifact_id(artifact_id)
        .status(IdeationSessionStatus::Active)
        .verification_status(VerificationStatus::ImportedVerified)
        .source_project_id(source_session.project_id.as_str().to_string())
        .source_session_id(input.source_session_id.clone());

    if let Some(title) = input.title {
        builder = builder.title(title);
    }

    let new_session = builder.build();

    // 5+6. Circular import check + INSERT in a single db.run() closure (TOCTOU safety)
    let source_id_for_check = input.source_session_id.clone();
    let target_project_id_for_check = target_project_id.clone();

    let created = state
        .db
        .run(move |conn| {
            // Circular import guard runs in same closure as INSERT (TOCTOU safety)
            SqliteIdeationSessionRepository::validate_no_circular_import_sync(
                conn,
                &source_id_for_check,
                &target_project_id_for_check,
                10, // max chain depth
            )?;

            // Atomically insert the new session
            SqliteIdeationSessionRepository::insert_sync(conn, &new_session)
        })
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("CIRCULAR_IMPORT")
                || msg.contains("SELF_REFERENCE")
                || msg.contains("CHAIN_TOO_DEEP")
            {
                tracing::warn!(error = %msg, "Cross-project import rejected: circular import detected");
            }
            msg
        })?;

    tracing::debug!(
        session_id = %created.id.as_str(),
        project_id = %created.project_id.as_str(),
        "Cross-project session created successfully"
    );

    // 7. Emit ideation:session_created event (same payload shape as regular sessions)
    let _ = app.emit(
        "ideation:session_created",
        serde_json::json!({
            "sessionId": created.id.to_string(),
            "projectId": created.project_id.to_string(),
        }),
    );

    Ok(IdeationSessionResponse::from(created))
}

// ============================================================================
// Tauri Command Wrapper
// ============================================================================

/// Create a new ideation session in a target project by inheriting a verified plan
/// from a source session in another project on the same RalphX instance.
///
/// The target project is resolved by filesystem path; it is auto-created if it doesn't exist
/// in RalphX yet (the directory must exist on disk). The source session's plan must be
/// in a verified state (Verified, Skipped, or ImportedVerified).
#[tauri::command]
pub async fn create_cross_project_session(
    input: CreateCrossProjectSessionInput,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<IdeationSessionResponse, String> {
    create_cross_project_session_impl(&app, &state, input).await
}
