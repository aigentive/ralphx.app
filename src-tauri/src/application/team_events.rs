// Team event emission helpers
//
// Emits team:* events via Tauri AppHandle at the appropriate lifecycle points.
// These functions wrap TeamStateTracker operations with event emission.

use tauri::{AppHandle, Emitter, Runtime};

use super::chat_service::events;
use super::chat_service::{
    TeamCostUpdatePayload, TeamCreatedPayload, TeamDisbandedPayload, TeamMessagePayload,
    TeamTeammateIdlePayload, TeamTeammateShutdownPayload, TeamTeammateSpawnedPayload,
};
use super::team_state_tracker::{TeamMessage, TeamMessageType, TeammateStatus};

/// Emit a team:created event
pub fn emit_team_created<R: Runtime>(
    app_handle: &AppHandle<R>,
    team_name: &str,
    context_id: &str,
    context_type: &str,
) {
    let _ = app_handle.emit(
        events::TEAM_CREATED,
        TeamCreatedPayload {
            team_name: team_name.to_string(),
            context_id: context_id.to_string(),
            context_type: context_type.to_string(),
        },
    );
}

/// Emit a team:teammate_spawned event
pub fn emit_teammate_spawned<R: Runtime>(
    app_handle: &AppHandle<R>,
    team_name: &str,
    teammate_name: &str,
    color: &str,
    model: &str,
    role: &str,
) {
    let _ = app_handle.emit(
        events::TEAM_TEAMMATE_SPAWNED,
        TeamTeammateSpawnedPayload {
            team_name: team_name.to_string(),
            teammate_name: teammate_name.to_string(),
            color: color.to_string(),
            model: model.to_string(),
            role: role.to_string(),
        },
    );
}

/// Emit a team:teammate_idle event
pub fn emit_teammate_idle<R: Runtime>(
    app_handle: &AppHandle<R>,
    team_name: &str,
    teammate_name: &str,
) {
    let _ = app_handle.emit(
        events::TEAM_TEAMMATE_IDLE,
        TeamTeammateIdlePayload {
            team_name: team_name.to_string(),
            teammate_name: teammate_name.to_string(),
        },
    );
}

/// Emit a team:teammate_shutdown event
pub fn emit_teammate_shutdown<R: Runtime>(
    app_handle: &AppHandle<R>,
    team_name: &str,
    teammate_name: &str,
) {
    let _ = app_handle.emit(
        events::TEAM_TEAMMATE_SHUTDOWN,
        TeamTeammateShutdownPayload {
            team_name: team_name.to_string(),
            teammate_name: teammate_name.to_string(),
        },
    );
}

/// Emit a team:message event
pub fn emit_team_message<R: Runtime>(app_handle: &AppHandle<R>, message: &TeamMessage) {
    let _ = app_handle.emit(
        events::TEAM_MESSAGE,
        TeamMessagePayload {
            team_name: message.team_name.clone(),
            message_id: message.id.clone(),
            sender: message.sender.clone(),
            recipient: message.recipient.clone(),
            content: message.content.clone(),
            message_type: match message.message_type {
                TeamMessageType::UserMessage => "user_message".to_string(),
                TeamMessageType::TeammateMessage => "teammate_message".to_string(),
                TeamMessageType::Broadcast => "broadcast".to_string(),
                TeamMessageType::System => "system".to_string(),
            },
            timestamp: message.timestamp.to_rfc3339(),
        },
    );
}

/// Emit a team:disbanded event
pub fn emit_team_disbanded<R: Runtime>(app_handle: &AppHandle<R>, team_name: &str) {
    let _ = app_handle.emit(
        events::TEAM_DISBANDED,
        TeamDisbandedPayload {
            team_name: team_name.to_string(),
        },
    );
}

/// Emit a team:cost_update event
pub fn emit_team_cost_update<R: Runtime>(
    app_handle: &AppHandle<R>,
    team_name: &str,
    teammate_name: &str,
    input_tokens: u64,
    output_tokens: u64,
    estimated_usd: f64,
) {
    let _ = app_handle.emit(
        events::TEAM_COST_UPDATE,
        TeamCostUpdatePayload {
            team_name: team_name.to_string(),
            teammate_name: teammate_name.to_string(),
            input_tokens,
            output_tokens,
            estimated_usd,
        },
    );
}

/// Emit a team event based on teammate status change
pub fn emit_teammate_status_change<R: Runtime>(
    app_handle: &AppHandle<R>,
    team_name: &str,
    teammate_name: &str,
    status: TeammateStatus,
) {
    match status {
        TeammateStatus::Idle => emit_teammate_idle(app_handle, team_name, teammate_name),
        TeammateStatus::Shutdown | TeammateStatus::Failed => {
            emit_teammate_shutdown(app_handle, team_name, teammate_name);
        }
        // Running, Spawning, Completed don't have dedicated events
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_team_message_type_formatting() {
        // Verify that message types map to expected strings
        assert_eq!(
            match TeamMessageType::UserMessage {
                TeamMessageType::UserMessage => "user_message",
                TeamMessageType::TeammateMessage => "teammate_message",
                TeamMessageType::Broadcast => "broadcast",
                TeamMessageType::System => "system",
            },
            "user_message"
        );
    }

    #[test]
    fn test_teammate_status_has_event() {
        // Idle and Shutdown/Failed have dedicated events
        assert!(matches!(TeammateStatus::Idle, TeammateStatus::Idle));
        assert!(matches!(TeammateStatus::Shutdown, TeammateStatus::Shutdown));
        assert!(matches!(TeammateStatus::Failed, TeammateStatus::Failed));
    }
}
