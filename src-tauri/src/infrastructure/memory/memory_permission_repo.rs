use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::application::permission_state::{PendingPermissionInfo, PermissionDecision};
use crate::domain::repositories::PermissionRepository;
use crate::error::AppResult;

pub struct MemoryPermissionRepository {
    permissions: RwLock<HashMap<String, (PendingPermissionInfo, Option<PermissionDecision>)>>,
}

impl MemoryPermissionRepository {
    pub fn new() -> Self {
        Self {
            permissions: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryPermissionRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PermissionRepository for MemoryPermissionRepository {
    async fn create_pending(&self, info: &PendingPermissionInfo) -> AppResult<()> {
        let mut permissions = self.permissions.write().unwrap();
        permissions.insert(info.request_id.clone(), (info.clone(), None));
        Ok(())
    }

    async fn resolve(
        &self,
        request_id: &str,
        decision: &PermissionDecision,
    ) -> AppResult<bool> {
        let mut permissions = self.permissions.write().unwrap();
        if let Some(entry) = permissions.get_mut(request_id) {
            entry.1 = Some(decision.clone());
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn get_pending(&self) -> AppResult<Vec<PendingPermissionInfo>> {
        let permissions = self.permissions.read().unwrap();
        Ok(permissions
            .values()
            .filter(|(_, decision)| decision.is_none())
            .map(|(info, _)| info.clone())
            .collect())
    }

    async fn get_by_request_id(
        &self,
        request_id: &str,
    ) -> AppResult<Option<PendingPermissionInfo>> {
        let permissions = self.permissions.read().unwrap();
        Ok(permissions.get(request_id).map(|(info, _)| info.clone()))
    }

    async fn expire_all_pending(&self) -> AppResult<u64> {
        let mut permissions = self.permissions.write().unwrap();
        let pending_ids: Vec<String> = permissions
            .iter()
            .filter(|(_, (_, decision))| decision.is_none())
            .map(|(id, _)| id.clone())
            .collect();
        let count = pending_ids.len() as u64;
        for id in pending_ids {
            permissions.remove(&id);
        }
        Ok(count)
    }

    async fn remove(&self, request_id: &str) -> AppResult<bool> {
        let mut permissions = self.permissions.write().unwrap();
        Ok(permissions.remove(request_id).is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_permission(request_id: &str) -> PendingPermissionInfo {
        PendingPermissionInfo {
            request_id: request_id.to_string(),
            tool_name: "Bash".to_string(),
            tool_input: serde_json::json!({"command": "ls"}),
            context: Some("List files".to_string()),
        }
    }

    #[tokio::test]
    async fn test_create_and_get_pending() {
        let repo = MemoryPermissionRepository::new();
        repo.create_pending(&sample_permission("perm-1")).await.unwrap();

        let pending = repo.get_pending().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].request_id, "perm-1");
        assert_eq!(pending[0].tool_name, "Bash");
    }

    #[tokio::test]
    async fn test_get_by_request_id() {
        let repo = MemoryPermissionRepository::new();
        repo.create_pending(&sample_permission("perm-42")).await.unwrap();

        let found = repo.get_by_request_id("perm-42").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().tool_name, "Bash");

        let not_found = repo.get_by_request_id("nonexistent").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_resolve() {
        let repo = MemoryPermissionRepository::new();
        repo.create_pending(&sample_permission("perm-1")).await.unwrap();

        let decision = PermissionDecision {
            decision: "allow".to_string(),
            message: Some("Approved".to_string()),
        };
        assert!(repo.resolve("perm-1", &decision).await.unwrap());

        let pending = repo.get_pending().await.unwrap();
        assert!(pending.is_empty());

        // Record still exists
        assert!(repo.get_by_request_id("perm-1").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_resolve_nonexistent() {
        let repo = MemoryPermissionRepository::new();
        let decision = PermissionDecision {
            decision: "deny".to_string(),
            message: None,
        };
        assert!(!repo.resolve("nope", &decision).await.unwrap());
    }

    #[tokio::test]
    async fn test_expire_all_pending() {
        let repo = MemoryPermissionRepository::new();
        for i in 0..3 {
            repo.create_pending(&sample_permission(&format!("perm-{i}")))
                .await
                .unwrap();
        }

        // Resolve one
        let decision = PermissionDecision {
            decision: "allow".to_string(),
            message: None,
        };
        repo.resolve("perm-0", &decision).await.unwrap();

        let expired = repo.expire_all_pending().await.unwrap();
        assert_eq!(expired, 2);
        assert!(repo.get_pending().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_remove() {
        let repo = MemoryPermissionRepository::new();
        repo.create_pending(&sample_permission("perm-rm")).await.unwrap();

        assert!(repo.remove("perm-rm").await.unwrap());
        assert!(repo.get_by_request_id("perm-rm").await.unwrap().is_none());
        assert!(!repo.remove("perm-rm").await.unwrap());
    }

    #[test]
    fn test_default_impl() {
        let repo = MemoryPermissionRepository::default();
        let permissions = repo.permissions.read().unwrap();
        assert!(permissions.is_empty());
    }
}
