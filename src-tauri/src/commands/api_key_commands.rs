// Tauri IPC commands for API key management.
//
// These replace the HTTP fetch calls in useApiKeys.ts. Tauri IPC is inherently
// trusted (only the webview can call invoke()), so no auth check is needed here.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::application::app_state::AppState;
use crate::domain::entities::AuditLogEntry;
use crate::domain::services::api_key_service::{ApiKeyService, KeySource};
use crate::domain::entities::ApiKeyId;

// ── Input structs ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiKeyInput {
    pub name: String,
    pub permissions: Option<i32>,
    pub project_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevokeApiKeyInput {
    pub id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RotateApiKeyInput {
    pub id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApiKeyProjectsInput {
    pub id: String,
    pub project_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApiKeyPermissionsInput {
    pub id: String,
    pub permissions: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAuditLogInput {
    pub id: String,
}

// ── Output structs ─────────────────────────────────────────────────────────────

/// Response for list_api_keys — one entry per key.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyInfoResponse {
    pub id: String,
    pub name: String,
    pub key_prefix: String,
    pub permissions: i32,
    pub created_at: String,
    pub revoked_at: Option<String>,
    pub last_used_at: Option<String>,
    pub project_ids: Vec<String>,
}

/// Response for create_api_key and rotate_api_key — includes one-time raw key.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyCreatedResponse {
    pub id: String,
    pub name: String,
    pub raw_key: String,
    pub key_prefix: String,
    pub permissions: i32,
}

// ── Commands ───────────────────────────────────────────────────────────────────

/// List all active API keys with their project associations.
#[tauri::command]
pub async fn list_api_keys(
    app_state: State<'_, AppState>,
) -> Result<Vec<ApiKeyInfoResponse>, String> {
    let repo = app_state.api_key_repo.as_ref();
    let keys = repo.list().await.map_err(|e| e.to_string())?;

    let mut result = Vec::with_capacity(keys.len());
    for key in keys {
        let project_ids = repo
            .get_projects(&key.id)
            .await
            .unwrap_or_default();
        result.push(ApiKeyInfoResponse {
            id: key.id.as_str().to_string(),
            name: key.name,
            key_prefix: key.key_prefix,
            permissions: key.permissions,
            created_at: key.created_at,
            revoked_at: key.revoked_at,
            last_used_at: key.last_used_at,
            project_ids,
        });
    }
    Ok(result)
}

/// Create a new API key via the settings UI.
///
/// Default permissions: 7 (read + write + admin) for settings-created keys.
#[tauri::command]
pub async fn create_api_key(
    app_state: State<'_, AppState>,
    input: CreateApiKeyInput,
) -> Result<ApiKeyCreatedResponse, String> {
    let repo = app_state.api_key_repo.as_ref();
    let project_ids = input.project_ids.unwrap_or_default();
    let created = ApiKeyService::create_key(
        repo,
        &input.name,
        input.permissions,
        &project_ids,
        KeySource::SettingsUi,
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(ApiKeyCreatedResponse {
        id: created.key.id.as_str().to_string(),
        name: created.key.name,
        raw_key: created.raw_key,
        key_prefix: created.key.key_prefix,
        permissions: created.key.permissions,
    })
}

/// Revoke an API key immediately.
#[tauri::command]
pub async fn revoke_api_key(
    app_state: State<'_, AppState>,
    input: RevokeApiKeyInput,
) -> Result<(), String> {
    let repo = app_state.api_key_repo.as_ref();
    ApiKeyService::revoke_key(repo, &input.id, KeySource::SettingsUi)
        .await
        .map_err(|e| e.to_string())
}

/// Rotate an API key — returns the new raw key; old key gets a 60-second grace period.
#[tauri::command]
pub async fn rotate_api_key(
    app_state: State<'_, AppState>,
    input: RotateApiKeyInput,
) -> Result<ApiKeyCreatedResponse, String> {
    let repo = app_state.api_key_repo.as_ref();
    let created = ApiKeyService::rotate_key(repo, &input.id, KeySource::SettingsUi)
        .await
        .map_err(|e| e.to_string())?;

    Ok(ApiKeyCreatedResponse {
        id: created.key.id.as_str().to_string(),
        name: created.key.name,
        raw_key: created.raw_key,
        key_prefix: created.key.key_prefix,
        permissions: created.key.permissions,
    })
}

/// Replace the project associations for a key.
#[tauri::command]
pub async fn update_api_key_projects(
    app_state: State<'_, AppState>,
    input: UpdateApiKeyProjectsInput,
) -> Result<(), String> {
    let repo = app_state.api_key_repo.as_ref();
    let key_id = ApiKeyId::from_string(&input.id);
    repo.set_projects(&key_id, &input.project_ids)
        .await
        .map_err(|e| e.to_string())?;
    let _ = repo
        .log_audit(&input.id, "settings_ui", None, true, None)
        .await;
    Ok(())
}

/// Update the permissions bitmask for a key.
#[tauri::command]
pub async fn update_api_key_permissions(
    app_state: State<'_, AppState>,
    input: UpdateApiKeyPermissionsInput,
) -> Result<(), String> {
    let repo = app_state.api_key_repo.as_ref();
    repo.update_api_key_permissions(&input.id, input.permissions as i64)
        .await
        .map_err(|e| e.to_string())?;
    let _ = repo
        .log_audit(&input.id, "settings_ui", None, true, None)
        .await;
    Ok(())
}

/// Retrieve the audit log for an API key (most recent first, up to 100 entries).
#[tauri::command]
pub async fn get_api_key_audit_log(
    app_state: State<'_, AppState>,
    input: GetAuditLogInput,
) -> Result<Vec<AuditLogEntry>, String> {
    let repo = app_state.api_key_repo.as_ref();
    repo.get_audit_log(&input.id, Some(100))
        .await
        .map_err(|e| e.to_string())
}
