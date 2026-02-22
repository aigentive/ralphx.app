// Team IPC commands — thin Tauri wrappers for TeamService operations
//
// These commands bridge the frontend to the TeamService for managing
// agent team lifecycle, status, and messaging. Mutation commands use
// TeamService (which emits events), read-only commands use raw tracker.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

use crate::application::app_state::AppState;
use crate::application::team_service::TeamService;
use crate::application::team_state_tracker::{
    TeamMessageResponse, TeamStateTracker, TeamStatusResponse, TeammateCost, TeammateCostResponse,
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

/// Input for send_teammate_message command (stdin routing)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendTeammateMessageInput {
    pub team_name: String,
    pub teammate_name: String,
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
    service: State<'_, Arc<TeamService>>,
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
    service: State<'_, Arc<TeamService>>,
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

/// Send a message directly to a specific teammate's stdin pipe (interactive mode).
///
/// This writes the content to the teammate's Claude process stdin,
/// allowing the user to interact with a specific teammate.
#[tauri::command]
pub async fn send_teammate_message(
    input: SendTeammateMessageInput,
    service: State<'_, Arc<TeamService>>,
) -> Result<(), String> {
    service
        .send_stdin_message(&input.team_name, &input.teammate_name, &input.content)
        .await
        .map_err(|e| e.to_string())
}

/// Stop a specific teammate in a team
///
/// Kills the teammate's child process, marks them as Shutdown,
/// and emits team:teammate_shutdown.
#[tauri::command]
pub async fn stop_teammate(
    team_name: String,
    teammate_name: String,
    service: State<'_, Arc<TeamService>>,
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
    service: State<'_, Arc<TeamService>>,
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
    service: State<'_, Arc<TeamService>>,
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

// ============================================================================
// History types & command (reads from persisted DB)
// ============================================================================

/// Response for a single teammate snapshot
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeammateSnapshotResponse {
    pub name: String,
    pub color: String,
    pub model: String,
    pub role: String,
    pub status: String,
    pub cost: TeammateCost,
    pub spawned_at: String,
    pub last_activity_at: String,
}

/// Response for a single team message record
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamMessageRecordResponse {
    pub id: String,
    pub sender: String,
    pub recipient: Option<String>,
    pub content: String,
    pub message_type: String,
    pub created_at: String,
}

/// Response for a single team session (history)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamSessionResponse {
    pub id: String,
    pub team_name: String,
    pub context_id: String,
    pub context_type: String,
    pub lead_name: Option<String>,
    pub phase: String,
    pub teammates: Vec<TeammateSnapshotResponse>,
    pub created_at: String,
    pub disbanded_at: Option<String>,
}

/// Input for get_team_history command
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTeamHistoryInput {
    pub context_id: String,
    pub context_type: String,
}

/// Response for get_team_history command.
/// Returns the most recent session (or null) with its messages.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamHistoryResponse {
    pub session: Option<TeamSessionResponse>,
    pub messages: Vec<TeamMessageRecordResponse>,
}

/// Get persisted team history for a context (task/ideation session).
///
/// Returns the most recent team session with its messages, or null session + empty messages.
#[tauri::command]
pub async fn get_team_history(
    input: GetTeamHistoryInput,
    state: State<'_, AppState>,
) -> Result<TeamHistoryResponse, String> {
    let sessions = state
        .team_session_repo
        .get_by_context(&input.context_type, &input.context_id)
        .await
        .map_err(|e| e.to_string())?;

    // Take the most recent session (last in the list, ordered by created_at)
    let session = sessions.into_iter().last();

    match session {
        Some(session) => {
            let messages = state
                .team_message_repo
                .get_by_session(&session.id)
                .await
                .map_err(|e| e.to_string())?;

            let teammates = session
                .teammates
                .iter()
                .map(|t| TeammateSnapshotResponse {
                    name: t.name.clone(),
                    color: t.color.clone(),
                    model: t.model.clone(),
                    role: t.role.clone(),
                    status: t.status.clone(),
                    cost: t.cost.clone(),
                    spawned_at: t.spawned_at.clone(),
                    last_activity_at: t.last_activity_at.clone(),
                })
                .collect();

            let msg_responses = messages
                .iter()
                .map(|m| TeamMessageRecordResponse {
                    id: m.id.0.clone(),
                    sender: m.sender.clone(),
                    recipient: m.recipient.clone(),
                    content: m.content.clone(),
                    message_type: m.message_type.clone(),
                    created_at: m.created_at.to_rfc3339(),
                })
                .collect();

            Ok(TeamHistoryResponse {
                session: Some(TeamSessionResponse {
                    id: session.id.0,
                    team_name: session.team_name,
                    context_id: session.context_id,
                    context_type: session.context_type,
                    lead_name: session.lead_name,
                    phase: session.phase,
                    teammates,
                    created_at: session.created_at.to_rfc3339(),
                    disbanded_at: session.disbanded_at.map(|d| d.to_rfc3339()),
                }),
                messages: msg_responses,
            })
        }
        None => Ok(TeamHistoryResponse {
            session: None,
            messages: vec![],
        }),
    }
}

#[cfg(test)]
#[path = "team_commands_tests.rs"]
mod tests;
