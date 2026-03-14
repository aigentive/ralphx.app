// Diagnostic commands — agent health inspection for runtime observability

use serde::Serialize;
use tauri::State;

use crate::application::AppState;

/// Serializable IPR entry for agent health report
#[derive(Debug, Clone, Serialize)]
pub struct IprEntryResponse {
    pub context_type: String,
    pub context_id: String,
}

/// Serializable running agent entry for agent health report
#[derive(Debug, Clone, Serialize)]
pub struct RunningAgentResponse {
    pub context_type: String,
    pub context_id: String,
    pub pid: u32,
    pub conversation_id: String,
    pub agent_run_id: String,
    pub started_at: String,
    pub worktree_path: Option<String>,
    pub last_active_at: Option<String>,
}

/// Full agent health report returned by get_agent_health
#[derive(Debug, Clone, Serialize)]
pub struct AgentHealthReport {
    /// Interactive process registry entries (open stdin handles)
    pub ipr_entries: Vec<IprEntryResponse>,
    /// All agents currently tracked in the running agent registry
    pub running_agents: Vec<RunningAgentResponse>,
}

/// Get agent health — IPR entries + running agents for runtime inspection.
///
/// # Errors
/// Returns an error string if registry access fails.
#[tauri::command]
pub async fn get_agent_health(state: State<'_, AppState>) -> Result<AgentHealthReport, String> {
    let ipr_keys = state.interactive_process_registry.dump_state().await;
    let ipr_entries = ipr_keys
        .into_iter()
        .map(|k| IprEntryResponse {
            context_type: k.context_type,
            context_id: k.context_id,
        })
        .collect();

    let all_agents = state.running_agent_registry.list_all().await;
    let running_agents = all_agents
        .into_iter()
        .map(|(key, info)| RunningAgentResponse {
            context_type: key.context_type,
            context_id: key.context_id,
            pid: info.pid,
            conversation_id: info.conversation_id,
            agent_run_id: info.agent_run_id,
            started_at: info.started_at.to_rfc3339(),
            worktree_path: info.worktree_path,
            last_active_at: info.last_active_at.map(|t| t.to_rfc3339()),
        })
        .collect();

    Ok(AgentHealthReport {
        ipr_entries,
        running_agents,
    })
}
