// Orchestrator agent integration and ideation settings commands

use tauri::State;

use crate::application::AppState;
use crate::domain::entities::{IdeationSessionId, IdeationSessionStatus};
use crate::domain::ideation::IdeationSettings;

use super::ideation_commands_types::{OrchestratorMessageResponse, SendOrchestratorMessageInput};

// ============================================================================
// Orchestrator Integration Commands
// ============================================================================

/// Send a message to the orchestrator agent and get a response
/// This invokes the Claude CLI with the orchestrator-ideation agent
///
/// The service now:
/// - Automatically manages conversations (creates/resumes based on claude_session_id)
/// - Uses --resume flag for follow-up messages (Claude manages conversation context)
/// - Delegates tool execution to MCP server
/// - Emits Tauri events for real-time UI updates
///
/// DEPRECATED: Use send_context_message with context_type="ideation" instead.
/// This command now delegates to the unified ChatService.
#[tauri::command]
pub async fn send_orchestrator_message(
    input: SendOrchestratorMessageInput,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<OrchestratorMessageResponse, String> {
    use crate::application::{ChatService, ClaudeChatService};
    use crate::domain::entities::ChatContextType;

    // First verify the session exists and is active
    let session_id = IdeationSessionId::from_string(input.session_id.clone());
    let session = state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Session not found".to_string())?;

    if session.status != IdeationSessionStatus::Active {
        return Err("Session is not active".to_string());
    }

    // Create unified chat service
    let chat_service: ClaudeChatService<tauri::Wry> = ClaudeChatService::new(
        state.chat_message_repo.clone(),
        state.chat_conversation_repo.clone(),
        state.agent_run_repo.clone(),
        state.project_repo.clone(),
        state.task_repo.clone(),
        state.ideation_session_repo.clone(),
        state.message_queue.clone(),
        state.running_agent_registry.clone(),
    )
    .with_app_handle(app);

    // Check if service is available
    if !chat_service.is_available().await {
        return Err("Claude CLI is not available".to_string());
    }

    // Send message via unified service (returns immediately, response via events)
    let _result = chat_service
        .send_message(ChatContextType::Ideation, &input.session_id, &input.content)
        .await
        .map_err(|e| e.to_string())?;

    // Note: The unified service uses background spawn pattern.
    // Response comes via agent:* events, not in the return value.
    // Return empty response for backward compatibility.
    Ok(OrchestratorMessageResponse {
        response_text: String::new(),
        proposals_created: Vec::new(),
        tool_calls: Vec::new(),
    })
}

/// Check if the orchestrator agent is available
///
/// DEPRECATED: Use the unified ChatService availability check instead.
#[tauri::command]
pub async fn is_orchestrator_available(state: State<'_, AppState>) -> Result<bool, String> {
    use crate::application::{ChatService, ClaudeChatService};

    let chat_service: ClaudeChatService<tauri::Wry> = ClaudeChatService::new(
        state.chat_message_repo.clone(),
        state.chat_conversation_repo.clone(),
        state.agent_run_repo.clone(),
        state.project_repo.clone(),
        state.task_repo.clone(),
        state.ideation_session_repo.clone(),
        state.message_queue.clone(),
        state.running_agent_registry.clone(),
    );

    Ok(chat_service.is_available().await)
}

// ============================================================================
// Ideation Settings Commands
// ============================================================================

/// Get ideation settings
#[tauri::command]
pub async fn get_ideation_settings(
    state: State<'_, AppState>,
) -> Result<IdeationSettings, String> {
    state
        .ideation_settings_repo
        .get_settings()
        .await
        .map_err(|e| e.to_string())
}

/// Update ideation settings
#[tauri::command]
pub async fn update_ideation_settings(
    settings: IdeationSettings,
    state: State<'_, AppState>,
) -> Result<IdeationSettings, String> {
    state
        .ideation_settings_repo
        .update_settings(&settings)
        .await
        .map_err(|e| e.to_string())
}
