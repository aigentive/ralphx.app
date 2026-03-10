use crate::application::app_state::AppState;
use crate::domain::services::api_key_service::{ApiKeyService, KeySource};
use crate::domain::entities::ApiKeyId;

fn setup_test_state() -> AppState {
    AppState::new_test()
}

// ── create_key ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_api_key_returns_raw_key() {
    let state = setup_test_state();
    let repo = state.api_key_repo.as_ref();

    let created = ApiKeyService::create_key(repo, "test-key", None, &[], KeySource::SettingsUi)
        .await
        .unwrap();

    assert!(!created.raw_key.is_empty(), "raw_key must be returned");
    assert_eq!(created.key.name, "test-key");
}

#[tokio::test]
async fn test_create_api_key_default_permissions_settings_ui() {
    let state = setup_test_state();
    let repo = state.api_key_repo.as_ref();

    let created = ApiKeyService::create_key(repo, "admin-key", None, &[], KeySource::SettingsUi)
        .await
        .unwrap();

    // SettingsUi default = 7 (read + write + admin)
    assert_eq!(created.key.permissions, 7);
}

#[tokio::test]
async fn test_create_api_key_persists_to_repo() {
    let state = setup_test_state();
    let repo = state.api_key_repo.as_ref();

    let created = ApiKeyService::create_key(repo, "persist-key", None, &[], KeySource::SettingsUi)
        .await
        .unwrap();

    let keys = repo.list().await.unwrap();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].id, created.key.id);
}

// ── list_keys ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_list_api_keys_returns_all_active() {
    let state = setup_test_state();
    let repo = state.api_key_repo.as_ref();

    ApiKeyService::create_key(repo, "key-a", None, &[], KeySource::SettingsUi)
        .await
        .unwrap();
    ApiKeyService::create_key(repo, "key-b", None, &[], KeySource::SettingsUi)
        .await
        .unwrap();

    let keys = repo.list().await.unwrap();
    assert_eq!(keys.len(), 2);
}

// ── revoke_key ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_revoke_api_key_removes_from_active_list() {
    let state = setup_test_state();
    let repo = state.api_key_repo.as_ref();

    let created = ApiKeyService::create_key(repo, "revoke-me", None, &[], KeySource::SettingsUi)
        .await
        .unwrap();

    ApiKeyService::revoke_key(repo, created.key.id.as_str(), KeySource::SettingsUi)
        .await
        .unwrap();

    let keys = repo.list().await.unwrap();
    assert!(
        keys.is_empty(),
        "revoked key must not appear in active list"
    );
}

// ── rotate_key ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_rotate_api_key_returns_new_raw_key() {
    let state = setup_test_state();
    let repo = state.api_key_repo.as_ref();

    let original = ApiKeyService::create_key(repo, "rotate-me", None, &[], KeySource::SettingsUi)
        .await
        .unwrap();

    let rotated = ApiKeyService::rotate_key(repo, original.key.id.as_str(), KeySource::SettingsUi)
        .await
        .unwrap();

    assert!(!rotated.raw_key.is_empty());
    assert_ne!(
        rotated.raw_key, original.raw_key,
        "rotated key must differ from original"
    );
}

// ── update_permissions ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_update_api_key_permissions() {
    let state = setup_test_state();
    let repo = state.api_key_repo.as_ref();

    let created = ApiKeyService::create_key(repo, "perm-key", None, &[], KeySource::SettingsUi)
        .await
        .unwrap();
    let key_id_str = created.key.id.as_str().to_string();

    // Update to read-only (permissions = 1)
    repo.update_api_key_permissions(&key_id_str, 1)
        .await
        .unwrap();

    let key_id = ApiKeyId::from_string(&key_id_str);
    let updated = repo.get_by_id(&key_id).await.unwrap().unwrap();
    assert_eq!(updated.permissions, 1);
}

// ── update_projects ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_update_api_key_projects() {
    let state = setup_test_state();
    let repo = state.api_key_repo.as_ref();

    let created = ApiKeyService::create_key(repo, "proj-key", None, &[], KeySource::SettingsUi)
        .await
        .unwrap();
    let key_id = created.key.id.clone();

    let project_ids = vec!["proj-1".to_string(), "proj-2".to_string()];
    repo.set_projects(&key_id, &project_ids).await.unwrap();

    let stored = repo.get_projects(&key_id).await.unwrap();
    assert_eq!(stored.len(), 2);
    assert!(stored.contains(&"proj-1".to_string()));
    assert!(stored.contains(&"proj-2".to_string()));
}

// ── audit log ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_audit_log_created_on_operations() {
    let state = setup_test_state();
    let repo = state.api_key_repo.as_ref();

    let created = ApiKeyService::create_key(repo, "audit-key", None, &[], KeySource::SettingsUi)
        .await
        .unwrap();
    let key_id_str = created.key.id.as_str().to_string();

    // create_key logs an audit entry for the creator
    let entries = repo.get_audit_log(&key_id_str, Some(10)).await.unwrap();
    assert!(
        !entries.is_empty(),
        "create_key must log at least one audit entry"
    );
}
