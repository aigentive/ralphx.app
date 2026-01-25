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

/// Shared state for managing pending permission requests
///
/// Uses tokio::sync::watch channels to allow long-polling:
/// - MCP server registers a request and waits on a receiver
/// - Frontend resolves the request by sending through the channel
pub struct PermissionState {
    /// Map of request_id -> watch::Sender
    /// The Sender is used to signal the decision to waiting long-poll requests
    pub pending: Mutex<HashMap<String, watch::Sender<Option<PermissionDecision>>>>,
}

impl PermissionState {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
        }
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
    async fn test_register_and_resolve_permission() {
        let state = PermissionState::new();

        // Register a pending permission
        let request_id = "test-request-123".to_string();
        let (tx, rx) = watch::channel(None);
        state.pending.lock().await.insert(request_id.clone(), tx);

        // Verify it's in pending
        {
            let pending = state.pending.lock().await;
            assert!(pending.contains_key(&request_id));
        }

        // Resolve with a decision
        {
            let pending = state.pending.lock().await;
            let tx = pending.get(&request_id).unwrap();
            tx.send(Some(PermissionDecision {
                decision: "allow".to_string(),
                message: Some("Approved by user".to_string()),
            }))
            .unwrap();
        }

        // Check the decision was received
        let decision = rx.borrow().clone();
        assert!(decision.is_some());
        let decision = decision.unwrap();
        assert_eq!(decision.decision, "allow");
        assert_eq!(decision.message, Some("Approved by user".to_string()));
    }

    #[tokio::test]
    async fn test_multiple_pending_permissions() {
        let state = PermissionState::new();

        // Register multiple pending permissions
        for i in 0..5 {
            let request_id = format!("request-{}", i);
            let (tx, _rx) = watch::channel(None);
            state.pending.lock().await.insert(request_id, tx);
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
        let (tx, _rx) = watch::channel(None);
        state.pending.lock().await.insert(request_id.clone(), tx);

        // Verify it exists
        {
            let pending = state.pending.lock().await;
            assert!(pending.contains_key(&request_id));
        }

        // Remove it
        {
            let mut pending = state.pending.lock().await;
            pending.remove(&request_id);
        }

        // Verify it's gone
        {
            let pending = state.pending.lock().await;
            assert!(!pending.contains_key(&request_id));
        }
    }
}
