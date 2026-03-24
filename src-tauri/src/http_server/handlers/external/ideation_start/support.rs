use super::*;

/// Build a fully configured `ClaudeChatService` from shared app + execution state.
/// Extracted to avoid duplicating the 12-arg constructor chain across multiple handlers.
pub(crate) fn build_chat_service(
    app: &crate::application::AppState,
    execution_state: &std::sync::Arc<crate::commands::ExecutionState>,
) -> ClaudeChatService {
    let mut chat_service = ClaudeChatService::new(
        Arc::clone(&app.chat_message_repo),
        Arc::clone(&app.chat_attachment_repo),
        Arc::clone(&app.artifact_repo),
        Arc::clone(&app.chat_conversation_repo),
        Arc::clone(&app.agent_run_repo),
        Arc::clone(&app.project_repo),
        Arc::clone(&app.task_repo),
        Arc::clone(&app.task_dependency_repo),
        Arc::clone(&app.ideation_session_repo),
        Arc::clone(&app.activity_event_repo),
        Arc::clone(&app.message_queue),
        Arc::clone(&app.running_agent_registry),
        Arc::clone(&app.memory_event_repo),
    )
    .with_execution_state(Arc::clone(execution_state))
    .with_plan_branch_repo(Arc::clone(&app.plan_branch_repo))
    .with_task_proposal_repo(Arc::clone(&app.task_proposal_repo))
    .with_interactive_process_registry(Arc::clone(&app.interactive_process_registry));
    if let Some(ref handle) = app.app_handle {
        chat_service = chat_service.with_app_handle(handle.clone());
    }
    chat_service
}

/// Fire-and-forget: spawn the session namer agent to auto-name the session.
pub(super) fn spawn_session_namer(
    agent_client: Arc<dyn crate::domain::agents::AgenticClient>,
    session_id: String,
    prompt: String,
) {
    tokio::spawn(async move {
        use crate::domain::agents::{AgentConfig, AgentRole};
        use crate::infrastructure::agents::claude::{agent_names, mcp_agent_type};
        use std::path::PathBuf;

        let namer_instructions = format!(
            "<instructions>\n\
             Generate a commit-ready title (imperative mood, \u{2264}50 characters) for this ideation session based on the context.\n\
             Describe what the plan does, not just the domain (e.g., 'Add OAuth2 login and JWT sessions').\n\
             Call the update_session_title tool with the session_id and the generated title.\n\
             Do NOT investigate, fix, or act on the user message content.\n\
             Do NOT use Read, Write, Edit, Task, or any file manipulation tools.\n\
             </instructions>\n\
             <data>\n\
             <session_id>{}</session_id>\n\
             <user_message>{}</user_message>\n\
             </data>",
            session_id, prompt
        );

        let working_directory = std::env::current_dir()
            .map(|cwd| cwd.parent().map(|p| p.to_path_buf()).unwrap_or(cwd))
            .unwrap_or_else(|_| PathBuf::from("."));
        let plugin_dir =
            crate::infrastructure::agents::claude::resolve_plugin_dir(&working_directory);

        let mut env = std::collections::HashMap::new();
        env.insert(
            "RALPHX_AGENT_TYPE".to_string(),
            mcp_agent_type(agent_names::AGENT_SESSION_NAMER).to_string(),
        );

        let config = AgentConfig {
            role: AgentRole::Custom(
                mcp_agent_type(agent_names::AGENT_SESSION_NAMER).to_string(),
            ),
            prompt: namer_instructions,
            working_directory,
            plugin_dir: Some(plugin_dir),
            agent: Some(agent_names::AGENT_SESSION_NAMER.to_string()),
            model: None,
            max_tokens: None,
            timeout_secs: Some(60),
            env,
        };

        match agent_client.spawn_agent(config).await {
            Ok(handle) => {
                if let Err(e) = agent_client.wait_for_completion(&handle).await {
                    tracing::warn!("Session namer agent failed: {}", e);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to spawn session namer agent: {}", e);
            }
        }
    });
}

/// Determine agent tri-state status for a session:
/// "idle" | "generating" | "waiting_for_input"
pub(crate) async fn determine_agent_status(
    running_agent_registry: &dyn crate::domain::services::running_agent_registry::RunningAgentRegistry,
    interactive_process_registry: &crate::application::InteractiveProcessRegistry,
    context_id: &str,
) -> String {
    let agent_key =
        crate::domain::services::running_agent_registry::RunningAgentKey::new("ideation", context_id);
    if running_agent_registry.is_running(&agent_key).await {
        let ipr_key = crate::application::InteractiveProcessKey {
            context_type: "ideation".to_string(),
            context_id: context_id.to_string(),
        };
        if interactive_process_registry.has_process(&ipr_key).await {
            "waiting_for_input".to_string()
        } else {
            "generating".to_string()
        }
    } else {
        "idle".to_string()
    }
}
