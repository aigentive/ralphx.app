// Tauri commands for permission resolution
// Allows frontend to resolve pending permission requests from agents

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::application::{PendingPermissionInfo, PermissionDecision};
use crate::AppState;

/// Arguments for resolving a permission request
#[derive(Debug, Deserialize)]
pub struct ResolvePermissionArgs {
    pub request_id: String,
    pub decision: String, // "allow" or "deny"
    pub message: Option<String>,
}

/// Response for resolve_permission_request command
#[derive(Debug, Serialize)]
pub struct ResolvePermissionResponse {
    pub success: bool,
    pub message: Option<String>,
}

/// Resolve a pending permission request with a user decision
///
/// Called by the frontend PermissionDialog when the user clicks Allow or Deny.
/// Signals the waiting MCP long-poll request with the decision.
#[tauri::command]
pub async fn resolve_permission_request(
    state: State<'_, AppState>,
    args: ResolvePermissionArgs,
) -> Result<ResolvePermissionResponse, String> {
    // Validate decision value
    if args.decision != "allow" && args.decision != "deny" {
        return Err(format!(
            "Invalid decision '{}'. Must be 'allow' or 'deny'",
            args.decision
        ));
    }

    let decision = PermissionDecision {
        decision: args.decision.clone(),
        message: args.message,
    };

    let resolved = state.permission_state.resolve(&args.request_id, decision).await;

    if resolved {
        Ok(ResolvePermissionResponse {
            success: true,
            message: Some(format!("Permission request {} resolved", args.request_id)),
        })
    } else {
        Err(format!(
            "Permission request '{}' not found",
            args.request_id
        ))
    }
}

/// Get information about all pending permission requests
///
/// Used by the frontend to display any pending requests that might have been
/// missed (e.g., if the app was just opened while an agent was waiting for approval).
#[tauri::command]
pub async fn get_pending_permissions(
    state: State<'_, AppState>,
) -> Result<Vec<PendingPermissionInfo>, String> {
    let pending = state.permission_state.get_pending_info().await;
    Ok(pending)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_permission_args_deserialize() {
        let json = r#"{"request_id": "abc-123", "decision": "allow", "message": "User approved"}"#;
        let args: ResolvePermissionArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.request_id, "abc-123");
        assert_eq!(args.decision, "allow");
        assert_eq!(args.message, Some("User approved".to_string()));
    }

    #[test]
    fn test_resolve_permission_args_without_message() {
        let json = r#"{"request_id": "abc-123", "decision": "deny"}"#;
        let args: ResolvePermissionArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.request_id, "abc-123");
        assert_eq!(args.decision, "deny");
        assert!(args.message.is_none());
    }

    #[test]
    fn test_resolve_permission_response_serialize() {
        let response = ResolvePermissionResponse {
            success: true,
            message: Some("Resolved".to_string()),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"message\":\"Resolved\""));
    }
}
