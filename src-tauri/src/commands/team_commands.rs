// Team IPC commands — thin Tauri wrappers for TeamService operations
//
// These commands bridge the frontend to the TeamService for managing
// agent team lifecycle, status, and messaging. Mutation commands use
// TeamService (which emits events), read-only commands use raw tracker.

use serde::Deserialize;
use tauri::State;

use crate::application::team_service::TeamService;
use crate::application::team_state_tracker::{
    TeamMessageResponse, TeamStateTracker, TeamStatusResponse, TeammateCostResponse,
};

// ============================================================================
// Request types
// ============================================================================

/// Input for create_team command
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTeamInput {
    pub team_name: String,
    pub context_type: String,
    pub context_id: String,
}

/// Input for send_team_message command
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendTeamMessageInput {
    pub team_name: String,
    pub content: String,
}

// ============================================================================
// Mutation commands (via TeamService — emits events)
// ============================================================================

/// Create a new team
///
/// Registers the team in the tracker and emits team:created.
#[tauri::command]
pub async fn create_team(
    input: CreateTeamInput,
    service: State<'_, TeamService>,
) -> Result<(), String> {
    service
        .create_team(&input.team_name, &input.context_id, &input.context_type)
        .await
        .map_err(|e| e.to_string())
}

/// Send a user message to a team
///
/// The message is stored in the tracker and a team:message event is emitted.
#[tauri::command]
pub async fn send_team_message(
    input: SendTeamMessageInput,
    service: State<'_, TeamService>,
) -> Result<TeamMessageResponse, String> {
    let msg = service
        .send_user_message(&input.team_name, &input.content)
        .await
        .map_err(|e| e.to_string())?;

    Ok(TeamMessageResponse {
        id: msg.id,
        sender: msg.sender,
        recipient: msg.recipient,
        content: msg.content,
        message_type: msg.message_type,
        timestamp: msg.timestamp.to_rfc3339(),
    })
}

/// Stop a specific teammate in a team
///
/// Kills the teammate's child process, marks them as Shutdown,
/// and emits team:teammate_shutdown.
#[tauri::command]
pub async fn stop_teammate(
    team_name: String,
    teammate_name: String,
    service: State<'_, TeamService>,
) -> Result<(), String> {
    service
        .stop_teammate(&team_name, &teammate_name)
        .await
        .map_err(|e| e.to_string())
}

/// Stop all teammates in a team
///
/// Kills all child processes, transitions to Winding phase,
/// and emits per-teammate shutdown events.
#[tauri::command]
pub async fn stop_team(
    team_name: String,
    service: State<'_, TeamService>,
) -> Result<(), String> {
    service
        .stop_team(&team_name)
        .await
        .map_err(|e| e.to_string())
}

/// Disband a team
///
/// Stops all teammates, marks the team as Disbanded,
/// and emits team:disbanded.
#[tauri::command]
pub async fn disband_team(
    team_name: String,
    service: State<'_, TeamService>,
) -> Result<(), String> {
    service
        .disband_team(&team_name)
        .await
        .map_err(|e| e.to_string())
}

// ============================================================================
// Read-only commands (via raw TeamStateTracker — no events needed)
// ============================================================================

/// Get the status of an active team
///
/// Returns team name, teammates with their statuses, phase, and message count.
#[tauri::command]
pub async fn get_team_status(
    team_name: String,
    tracker: State<'_, TeamStateTracker>,
) -> Result<TeamStatusResponse, String> {
    tracker
        .get_team_status(&team_name)
        .await
        .map_err(|e| e.to_string())
}

/// Get team messages, optionally limited
///
/// Returns messages in reverse chronological order.
#[tauri::command]
pub async fn get_team_messages(
    team_name: String,
    limit: Option<usize>,
    tracker: State<'_, TeamStateTracker>,
) -> Result<Vec<TeamMessageResponse>, String> {
    tracker
        .get_team_messages(&team_name, limit)
        .await
        .map_err(|e| e.to_string())
}

/// Get cost tracking for a specific teammate
#[tauri::command]
pub async fn get_teammate_cost(
    team_name: String,
    teammate_name: String,
    tracker: State<'_, TeamStateTracker>,
) -> Result<TeammateCostResponse, String> {
    tracker
        .get_teammate_cost(&team_name, &teammate_name)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_team_message_input_deserialize() {
        let json = r#"{"teamName":"my-team","content":"Hello"}"#;
        let input: SendTeamMessageInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.team_name, "my-team");
        assert_eq!(input.content, "Hello");
    }

    #[test]
    fn test_create_team_input_deserialize() {
        let json = r#"{"teamName":"alpha","contextType":"ideation","contextId":"session-123"}"#;
        let input: CreateTeamInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.team_name, "alpha");
        assert_eq!(input.context_type, "ideation");
        assert_eq!(input.context_id, "session-123");
    }
}
