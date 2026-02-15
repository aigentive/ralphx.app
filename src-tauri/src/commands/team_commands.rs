// Team IPC commands — thin Tauri wrappers for TeamStateTracker operations
//
// These commands bridge the frontend to the TeamStateTracker service
// for managing agent team lifecycle, status, and messaging.

use serde::Deserialize;
use tauri::State;

use crate::application::team_state_tracker::{
    TeamMessageResponse, TeamStateTracker, TeamStatusResponse, TeammateCostResponse,
};

// ============================================================================
// Request types
// ============================================================================

/// Input for send_team_message command
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendTeamMessageInput {
    pub team_name: String,
    pub content: String,
}

// ============================================================================
// Commands
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

/// Send a user message to a team
///
/// The message is stored in the TeamStateTracker and a team:message event is emitted.
#[tauri::command]
pub async fn send_team_message(
    input: SendTeamMessageInput,
    tracker: State<'_, TeamStateTracker>,
) -> Result<TeamMessageResponse, String> {
    let msg = tracker
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
/// Kills the teammate's child process and marks them as Shutdown.
#[tauri::command]
pub async fn stop_teammate(
    team_name: String,
    teammate_name: String,
    tracker: State<'_, TeamStateTracker>,
) -> Result<(), String> {
    tracker
        .stop_teammate(&team_name, &teammate_name)
        .await
        .map_err(|e| e.to_string())
}

/// Stop all teammates in a team
///
/// Kills all child processes and transitions the team to Winding phase.
#[tauri::command]
pub async fn stop_team(
    team_name: String,
    tracker: State<'_, TeamStateTracker>,
) -> Result<(), String> {
    tracker
        .stop_team(&team_name)
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
}
