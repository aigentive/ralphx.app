// Session management commands

use tauri::{Emitter, State};

use crate::application::AppState;
use crate::domain::entities::{
    IdeationSession, IdeationSessionId, IdeationSessionStatus, ProjectId, TaskId,
};

use super::ideation_commands_types::{
    ChatMessageResponse, CreateSessionInput, IdeationSessionResponse,
    SessionWithDataResponse, TaskProposalResponse,
};

// ============================================================================
// Session Management Commands
// ============================================================================

/// Create a new ideation session
#[tauri::command]
pub async fn create_ideation_session(
    input: CreateSessionInput,
    state: State<'_, AppState>,
) -> Result<IdeationSessionResponse, String> {
    let project_id = ProjectId::from_string(input.project_id);
    let seed_task_id = input.seed_task_id.map(TaskId::from_string);

    let mut builder = IdeationSession::builder().project_id(project_id);

    if let Some(title) = input.title {
        builder = builder.title(title);
    }

    if let Some(task_id) = seed_task_id {
        builder = builder.seed_task_id(task_id);
    }

    let session = builder.build();

    state
        .ideation_session_repo
        .create(session)
        .await
        .map(IdeationSessionResponse::from)
        .map_err(|e| e.to_string())
}

/// Get a single ideation session by ID
#[tauri::command]
pub async fn get_ideation_session(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<IdeationSessionResponse>, String> {
    let session_id = IdeationSessionId::from_string(id);
    state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map(|opt| opt.map(IdeationSessionResponse::from))
        .map_err(|e| e.to_string())
}

/// Get session with proposals and messages
#[tauri::command]
pub async fn get_ideation_session_with_data(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<SessionWithDataResponse>, String> {
    let session_id = IdeationSessionId::from_string(id);

    // Get session
    let session = match state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|e| e.to_string())?
    {
        Some(s) => s,
        None => return Ok(None),
    };

    // Get proposals
    let proposals = state
        .task_proposal_repo
        .get_by_session(&session_id)
        .await
        .map_err(|e| e.to_string())?;

    // Get messages
    let messages = state
        .chat_message_repo
        .get_by_session(&session_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Some(SessionWithDataResponse {
        session: IdeationSessionResponse::from(session),
        proposals: proposals.into_iter().map(TaskProposalResponse::from).collect(),
        messages: messages.into_iter().map(ChatMessageResponse::from).collect(),
    }))
}

/// List all ideation sessions for a project
#[tauri::command]
pub async fn list_ideation_sessions(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<IdeationSessionResponse>, String> {
    let project_id = ProjectId::from_string(project_id);
    state
        .ideation_session_repo
        .get_by_project(&project_id)
        .await
        .map(|sessions| {
            sessions
                .into_iter()
                .map(IdeationSessionResponse::from)
                .collect()
        })
        .map_err(|e| e.to_string())
}

/// Archive an ideation session
#[tauri::command]
pub async fn archive_ideation_session(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let session_id = IdeationSessionId::from_string(id);
    state
        .ideation_session_repo
        .update_status(&session_id, IdeationSessionStatus::Archived)
        .await
        .map_err(|e| e.to_string())
}

/// Delete an ideation session
#[tauri::command]
pub async fn delete_ideation_session(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let session_id = IdeationSessionId::from_string(id);
    state
        .ideation_session_repo
        .delete(&session_id)
        .await
        .map_err(|e| e.to_string())
}

/// Update the title of an ideation session
///
/// Sets or clears the session title and emits a real-time event for UI updates.
/// This is used by the session-namer agent for auto-generated titles and
/// by the frontend for manual renames.
#[tauri::command]
pub async fn update_ideation_session_title(
    id: String,
    title: Option<String>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<IdeationSessionResponse, String> {
    let session_id = IdeationSessionId::from_string(id.clone());

    // Update the title in the database
    state
        .ideation_session_repo
        .update_title(&session_id, title.clone())
        .await
        .map_err(|e| e.to_string())?;

    // Get the updated session to return
    let session = state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session not found after update: {}", id))?;

    // Emit event for real-time UI updates
    let _ = app.emit(
        "ideation:session_title_updated",
        serde_json::json!({
            "sessionId": id,
            "title": title,
        }),
    );

    Ok(IdeationSessionResponse::from(session))
}

/// Spawn the session-namer agent to auto-generate a title for the session
///
/// This is a fire-and-forget operation that spawns a background agent.
/// The agent will call the update_session_title MCP tool when complete,
/// which will emit an event for real-time UI updates.
#[tauri::command]
pub async fn spawn_session_namer(
    session_id: String,
    first_message: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    use std::path::PathBuf;
    use std::sync::Arc;

    use crate::domain::agents::{AgentConfig, AgentRole};

    // Build the prompt with session context
    let prompt = format!(
        "Session ID: {}\nContext: {}\n\nGenerate a concise title (exactly 2 words) for this ideation session based on the context, then call the update_session_title tool with the session_id and the generated title.",
        session_id, first_message
    );

    // Get the working directory (project root)
    let working_directory = std::env::current_dir()
        .map(|cwd| cwd.parent().map(|p| p.to_path_buf()).unwrap_or(cwd))
        .unwrap_or_else(|_| PathBuf::from("."));

    let plugin_dir = working_directory.join("ralphx-plugin");

    // Set RALPHX_AGENT_TYPE so MCP server grants access to update_session_title tool
    let mut env = std::collections::HashMap::new();
    env.insert("RALPHX_AGENT_TYPE".to_string(), "session-namer".to_string());

    let config = AgentConfig {
        role: AgentRole::Custom("session-namer".to_string()),
        prompt,
        working_directory,
        plugin_dir: Some(plugin_dir),
        agent: Some("session-namer".to_string()),
        model: None, // Agent file specifies haiku
        max_tokens: None,
        timeout_secs: Some(60), // 60 second timeout for title generation
        env,
    };

    // Clone the agent client for the background task
    let agent_client = Arc::clone(&state.agent_client);

    // Spawn in background (fire-and-forget)
    tokio::spawn(async move {
        match agent_client.spawn_agent(config).await {
            Ok(handle) => {
                // Wait for completion in the background
                if let Err(e) = agent_client.wait_for_completion(&handle).await {
                    tracing::warn!("Session namer agent failed: {}", e);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to spawn session namer agent: {}", e);
            }
        }
    });

    Ok(())
}
