use async_trait::async_trait;
use crate::domain::entities::{ApiKey, ApiKeyId};
use crate::error::AppResult;

/// Repository trait for ApiKey persistence
#[async_trait]
pub trait ApiKeyRepository: Send + Sync {
    /// Create a new API key record (key_hash must already be computed)
    async fn create(&self, api_key: ApiKey) -> AppResult<ApiKey>;

    /// List all non-revoked API keys
    async fn list(&self) -> AppResult<Vec<ApiKey>>;

    /// Get by ID
    async fn get_by_id(&self, id: &ApiKeyId) -> AppResult<Option<ApiKey>>;

    /// Get by SHA-256 hash of the raw key (for validation)
    async fn get_by_hash(&self, key_hash: &str) -> AppResult<Option<ApiKey>>;

    /// Revoke a key by setting revoked_at
    async fn revoke(&self, id: &ApiKeyId) -> AppResult<()>;

    /// Set grace_expires_at for key rotation (old key valid until this time)
    async fn set_grace_period(&self, id: &ApiKeyId, grace_expires_at: &str) -> AppResult<()>;

    /// Update last_used_at timestamp
    async fn update_last_used(&self, id: &ApiKeyId, timestamp: &str) -> AppResult<()>;

    /// Get all project IDs associated with a key
    async fn get_projects(&self, id: &ApiKeyId) -> AppResult<Vec<String>>;

    /// Replace project associations for a key (delete old, insert new)
    async fn set_projects(&self, id: &ApiKeyId, project_ids: &[String]) -> AppResult<()>;

    /// Insert an audit log entry
    async fn log_audit(
        &self,
        api_key_id: &str,
        tool_name: &str,
        project_id: Option<&str>,
        success: bool,
        latency_ms: Option<i64>,
    ) -> AppResult<()>;
}
