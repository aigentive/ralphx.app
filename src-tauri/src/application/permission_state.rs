// Permission state for handling UI-based permission approvals
// Used by the permission bridge system to coordinate between MCP tools and frontend

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::{watch, Mutex};

/// Permission decision made by the user in the UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionDecision {
    pub decision: String, // "allow" or "deny"
    pub message: Option<String>,
}

/// Metadata for a pending permission request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingPermissionInfo {
    pub request_id: String,
    pub tool_name: String,
    pub tool_input: serde_json::Value,
    pub context: Option<String>,
}

/// A pending permission request with its signaling channel
pub struct PendingPermissionRequest {
    pub info: PendingPermissionInfo,
    pub sender: watch::Sender<Option<PermissionDecision>>,
}

/// Shared state for managing pending permission requests
///
/// Uses tokio::sync::watch channels to allow long-polling:
/// - MCP server registers a request and waits on a receiver
/// - Frontend resolves the request by sending through the channel
pub struct PermissionState {
    /// Map of request_id -> PendingPermissionRequest
    /// The PendingPermissionRequest contains both metadata and the signaling channel
    pub pending: Mutex<HashMap<String, PendingPermissionRequest>>,
}

impl PermissionState {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
        }
    }

    /// Get info about all pending permission requests
    pub async fn get_pending_info(&self) -> Vec<PendingPermissionInfo> {
        let pending = self.pending.lock().await;
        pending.values().map(|p| p.info.clone()).collect()
    }

    /// Register a new pending permission request
    pub async fn register(
        &self,
        request_id: String,
        tool_name: String,
        tool_input: serde_json::Value,
        context: Option<String>,
    ) -> watch::Receiver<Option<PermissionDecision>> {
        let (tx, rx) = watch::channel(None);
        let request = PendingPermissionRequest {
            info: PendingPermissionInfo {
                request_id: request_id.clone(),
                tool_name,
                tool_input,
                context,
            },
            sender: tx,
        };
        self.pending.lock().await.insert(request_id, request);
        rx
    }

    /// Resolve a pending permission request with a decision
    /// Returns true if the request was found and resolved
    pub async fn resolve(&self, request_id: &str, decision: PermissionDecision) -> bool {
        let pending = self.pending.lock().await;
        if let Some(request) = pending.get(request_id) {
            let _ = request.sender.send(Some(decision));
            true
        } else {
            false
        }
    }

    /// Remove a pending permission request
    pub async fn remove(&self, request_id: &str) -> bool {
        self.pending.lock().await.remove(request_id).is_some()
    }
}

impl Default for PermissionState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_permission_state_new() {
        let state = PermissionState::new();
        let pending = state.pending.lock().await;
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn test_permission_state_default() {
        let state = PermissionState::default();
        let pending = state.pending.lock().await;
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn test_permission_decision_clone() {
        let decision = PermissionDecision {
            decision: "allow".to_string(),
            message: Some("User approved".to_string()),
        };
        let cloned = decision.clone();
        assert_eq!(cloned.decision, "allow");
        assert_eq!(cloned.message, Some("User approved".to_string()));
    }

    #[tokio::test]
    async fn test_permission_decision_serialization() {
        let decision = PermissionDecision {
            decision: "deny".to_string(),
            message: None,
        };
        let json = serde_json::to_string(&decision).unwrap();
        assert!(json.contains("\"decision\":\"deny\""));

        let deserialized: PermissionDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.decision, "deny");
        assert!(deserialized.message.is_none());
    }

    #[tokio::test]
    async fn test_pending_permission_info_serialization() {
        let info = PendingPermissionInfo {
            request_id: "req-123".to_string(),
            tool_name: "Bash".to_string(),
            tool_input: serde_json::json!({"command": "ls -la"}),
            context: Some("User wants to list files".to_string()),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"request_id\":\"req-123\""));
        assert!(json.contains("\"tool_name\":\"Bash\""));
        assert!(json.contains("\"command\":\"ls -la\""));
    }

    #[tokio::test]
    async fn test_register_and_resolve_permission() {
        let state = PermissionState::new();

        // Register a pending permission using the helper method
        let request_id = "test-request-123".to_string();
        let rx = state
            .register(
                request_id.clone(),
                "Bash".to_string(),
                serde_json::json!({"command": "rm -rf /tmp/test"}),
                Some("Cleanup temp files".to_string()),
            )
            .await;

        // Verify it's in pending
        {
            let pending = state.pending.lock().await;
            assert!(pending.contains_key(&request_id));
            let request = pending.get(&request_id).unwrap();
            assert_eq!(request.info.tool_name, "Bash");
        }

        // Resolve with a decision
        let resolved = state
            .resolve(
                &request_id,
                PermissionDecision {
                    decision: "allow".to_string(),
                    message: Some("Approved by user".to_string()),
                },
            )
            .await;
        assert!(resolved);

        // Check the decision was received
        let decision = rx.borrow().clone();
        assert!(decision.is_some());
        let decision = decision.unwrap();
        assert_eq!(decision.decision, "allow");
        assert_eq!(decision.message, Some("Approved by user".to_string()));
    }

    #[tokio::test]
    async fn test_get_pending_info() {
        let state = PermissionState::new();

        // Register multiple pending permissions
        for i in 0..3 {
            state
                .register(
                    format!("request-{}", i),
                    format!("Tool{}", i),
                    serde_json::json!({"arg": i}),
                    None,
                )
                .await;
        }

        // Get pending info
        let pending_info = state.get_pending_info().await;
        assert_eq!(pending_info.len(), 3);

        // Verify all are present (order not guaranteed)
        let request_ids: Vec<_> = pending_info.iter().map(|p| p.request_id.as_str()).collect();
        assert!(request_ids.contains(&"request-0"));
        assert!(request_ids.contains(&"request-1"));
        assert!(request_ids.contains(&"request-2"));
    }

    #[tokio::test]
    async fn test_multiple_pending_permissions() {
        let state = PermissionState::new();

        // Register multiple pending permissions
        for i in 0..5 {
            state
                .register(
                    format!("request-{}", i),
                    "TestTool".to_string(),
                    serde_json::json!({}),
                    None,
                )
                .await;
        }

        // Verify all are registered
        let pending = state.pending.lock().await;
        assert_eq!(pending.len(), 5);
        for i in 0..5 {
            assert!(pending.contains_key(&format!("request-{}", i)));
        }
    }

    #[tokio::test]
    async fn test_remove_pending_permission() {
        let state = PermissionState::new();

        // Register a pending permission
        let request_id = "to-remove".to_string();
        state
            .register(request_id.clone(), "Bash".to_string(), serde_json::json!({}), None)
            .await;

        // Verify it exists
        {
            let pending = state.pending.lock().await;
            assert!(pending.contains_key(&request_id));
        }

        // Remove it
        let removed = state.remove(&request_id).await;
        assert!(removed);

        // Verify it's gone
        {
            let pending = state.pending.lock().await;
            assert!(!pending.contains_key(&request_id));
        }

        // Try to remove again - should return false
        let removed_again = state.remove(&request_id).await;
        assert!(!removed_again);
    }

    #[tokio::test]
    async fn test_resolve_nonexistent_request() {
        let state = PermissionState::new();

        // Try to resolve a request that doesn't exist
        let resolved = state
            .resolve(
                "nonexistent",
                PermissionDecision {
                    decision: "deny".to_string(),
                    message: None,
                },
            )
            .await;
        assert!(!resolved);
    }
}
