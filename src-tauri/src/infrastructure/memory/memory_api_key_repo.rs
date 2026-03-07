// Memory-based ApiKeyRepository implementation for testing
// Uses RwLock<HashMap> for thread-safe storage

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;

use crate::domain::entities::{ApiKey, ApiKeyId};
use crate::domain::repositories::ApiKeyRepository;
use crate::error::AppResult;

/// In-memory implementation of ApiKeyRepository for testing
/// Uses RwLock<HashMap> for thread-safe storage
pub struct MemoryApiKeyRepository {
    keys: Arc<RwLock<HashMap<String, ApiKey>>>,
    /// project associations: api_key_id -> Vec<project_id>
    projects: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// audit log entries (api_key_id, tool_name, project_id, success, latency_ms)
    audit_log: Arc<RwLock<Vec<(String, String, Option<String>, bool, Option<i64>)>>>,
}

impl Default for MemoryApiKeyRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryApiKeyRepository {
    /// Create a new empty in-memory API key repository
    pub fn new() -> Self {
        Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
            projects: Arc::new(RwLock::new(HashMap::new())),
            audit_log: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

#[async_trait]
impl ApiKeyRepository for MemoryApiKeyRepository {
    async fn create(&self, api_key: ApiKey) -> AppResult<ApiKey> {
        let mut keys = self.keys.write().await;
        keys.insert(api_key.id.as_str().to_string(), api_key.clone());
        Ok(api_key)
    }

    async fn list(&self) -> AppResult<Vec<ApiKey>> {
        let keys = self.keys.read().await;
        let mut result: Vec<ApiKey> = keys
            .values()
            .filter(|k| k.revoked_at.is_none())
            .cloned()
            .collect();
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(result)
    }

    async fn get_by_id(&self, id: &ApiKeyId) -> AppResult<Option<ApiKey>> {
        let keys = self.keys.read().await;
        Ok(keys.get(id.as_str()).cloned())
    }

    async fn get_by_hash(&self, key_hash: &str) -> AppResult<Option<ApiKey>> {
        let keys = self.keys.read().await;
        Ok(keys.values().find(|k| k.key_hash == key_hash).cloned())
    }

    async fn revoke(&self, id: &ApiKeyId) -> AppResult<()> {
        let mut keys = self.keys.write().await;
        if let Some(key) = keys.get_mut(id.as_str()) {
            let now = chrono::Utc::now()
                .format("%Y-%m-%dT%H:%M:%SZ")
                .to_string();
            key.revoked_at = Some(now);
        }
        Ok(())
    }

    async fn set_grace_period(&self, id: &ApiKeyId, grace_expires_at: &str) -> AppResult<()> {
        let mut keys = self.keys.write().await;
        if let Some(key) = keys.get_mut(id.as_str()) {
            key.grace_expires_at = Some(grace_expires_at.to_string());
        }
        Ok(())
    }

    async fn update_last_used(&self, id: &ApiKeyId, timestamp: &str) -> AppResult<()> {
        let mut keys = self.keys.write().await;
        if let Some(key) = keys.get_mut(id.as_str()) {
            key.last_used_at = Some(timestamp.to_string());
        }
        Ok(())
    }

    async fn get_projects(&self, id: &ApiKeyId) -> AppResult<Vec<String>> {
        let projects = self.projects.read().await;
        Ok(projects.get(id.as_str()).cloned().unwrap_or_default())
    }

    async fn set_projects(&self, id: &ApiKeyId, project_ids: &[String]) -> AppResult<()> {
        let mut projects = self.projects.write().await;
        projects.insert(id.as_str().to_string(), project_ids.to_vec());
        Ok(())
    }

    async fn log_audit(
        &self,
        api_key_id: &str,
        tool_name: &str,
        project_id: Option<&str>,
        success: bool,
        latency_ms: Option<i64>,
    ) -> AppResult<()> {
        let mut log = self.audit_log.write().await;
        log.push((
            api_key_id.to_string(),
            tool_name.to_string(),
            project_id.map(|s| s.to_string()),
            success,
            latency_ms,
        ));
        Ok(())
    }
}
