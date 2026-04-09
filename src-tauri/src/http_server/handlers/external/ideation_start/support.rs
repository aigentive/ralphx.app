use super::*;
use crate::application::harness_runtime_registry::{
    default_repo_root_working_directory, resolve_default_harness_agent_bootstrap,
};
use crate::application::session_namer_prompt::build_session_namer_prompt;

/// Build a fully configured app chat service from shared app + execution state.
/// Extracted to avoid duplicating the 12-arg constructor chain across multiple handlers.
pub(crate) fn build_chat_service(
    app: &crate::application::AppState,
    execution_state: &std::sync::Arc<crate::commands::ExecutionState>,
) -> crate::application::AppChatService {
    app.build_chat_service_with_execution_state(Arc::clone(execution_state))
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
        use crate::infrastructure::agents::claude::agent_names;
        let namer_instructions = build_session_namer_prompt(&format!(
            "<session_id>{}</session_id>\n<user_message>{}</user_message>",
            session_id, prompt
        ));

        let working_directory = default_repo_root_working_directory();
        let bootstrap = resolve_default_harness_agent_bootstrap(
            agent_names::AGENT_SESSION_NAMER,
            working_directory,
        );

        let config = AgentConfig {
            role: AgentRole::Custom(bootstrap.agent_role.clone()),
            prompt: namer_instructions,
            working_directory: bootstrap.working_directory,
            plugin_dir: Some(bootstrap.plugin_dir),
            agent: Some(bootstrap.agent_name),
            model: runtime.model,
            harness: runtime.harness,
            logical_effort: runtime.logical_effort,
            approval_policy: runtime.approval_policy,
            sandbox_mode: runtime.sandbox_mode,
            max_tokens: None,
            timeout_secs: Some(60),
            env: bootstrap.env,
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
