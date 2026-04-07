use super::*;
use crate::application::session_namer_prompt::build_session_namer_prompt;

/// Build a fully configured `ClaudeChatService` from shared app + execution state.
/// Extracted to avoid duplicating the 12-arg constructor chain across multiple handlers.
pub(crate) fn build_chat_service(
    app: &crate::application::AppState,
    execution_state: &std::sync::Arc<crate::commands::ExecutionState>,
) -> ClaudeChatService {
    app.build_chat_service_with_execution_state(Arc::clone(execution_state))
        .with_interactive_process_registry(Arc::clone(&app.interactive_process_registry))
}

/// Fire-and-forget: spawn the session namer agent to auto-name the session.
pub(super) async fn spawn_session_namer(
    app: &crate::application::AppState,
    project_id: &str,
    session_id: String,
    prompt: String,
) {
    let runtime = app
        .resolve_ideation_background_agent_runtime(Some(project_id))
        .await;
    let agent_client = Arc::clone(&runtime.client);
    tokio::spawn(async move {
        use crate::domain::agents::{AgentConfig, AgentRole};
        use crate::infrastructure::agents::claude::{agent_names, mcp_agent_type};
        use std::path::PathBuf;

        let namer_instructions = build_session_namer_prompt(&format!(
            "<session_id>{}</session_id>\n<user_message>{}</user_message>",
            session_id, prompt
        ));

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
            model: runtime.model,
            harness: runtime.harness,
            logical_effort: runtime.logical_effort,
            approval_policy: runtime.approval_policy,
            sandbox_mode: runtime.sandbox_mode,
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
