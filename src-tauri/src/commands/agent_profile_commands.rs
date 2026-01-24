// Tauri commands for AgentProfile operations
// Thin layer that delegates to AgentProfileRepository

use serde::Serialize;
use tauri::State;

use crate::application::AppState;
use crate::domain::agents::{AgentProfile, ProfileRole};
use crate::domain::repositories::AgentProfileId;

/// Response wrapper for agent profile operations
#[derive(Debug, Serialize)]
pub struct AgentProfileResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub role: String,
    pub claude_code: ClaudeCodeConfigResponse,
    pub execution: ExecutionConfigResponse,
    pub io: IoConfigResponse,
    pub behavior: BehaviorConfigResponse,
}

#[derive(Debug, Serialize)]
pub struct ClaudeCodeConfigResponse {
    pub agent_definition: String,
    pub skills: Vec<String>,
    pub hooks: Option<serde_json::Value>,
    pub mcp_servers: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ExecutionConfigResponse {
    pub model: String,
    pub max_iterations: u32,
    pub timeout_minutes: u32,
    pub permission_mode: String,
}

#[derive(Debug, Serialize)]
pub struct IoConfigResponse {
    pub input_artifact_types: Vec<String>,
    pub output_artifact_types: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct BehaviorConfigResponse {
    pub can_spawn_sub_agents: bool,
    pub auto_commit: bool,
    pub autonomy_level: String,
}

impl From<AgentProfile> for AgentProfileResponse {
    fn from(profile: AgentProfile) -> Self {
        Self {
            id: profile.id,
            name: profile.name,
            description: profile.description,
            role: format!("{:?}", profile.role).to_lowercase(),
            claude_code: ClaudeCodeConfigResponse {
                agent_definition: profile.claude_code.agent_definition,
                skills: profile.claude_code.skills,
                hooks: profile.claude_code.hooks,
                mcp_servers: profile.claude_code.mcp_servers,
            },
            execution: ExecutionConfigResponse {
                model: format!("{:?}", profile.execution.model).to_lowercase(),
                max_iterations: profile.execution.max_iterations,
                timeout_minutes: profile.execution.timeout_minutes,
                permission_mode: format!("{:?}", profile.execution.permission_mode)
                    .to_lowercase()
                    .replace("_", ""),
            },
            io: IoConfigResponse {
                input_artifact_types: profile.io.input_artifact_types,
                output_artifact_types: profile.io.output_artifact_types,
            },
            behavior: BehaviorConfigResponse {
                can_spawn_sub_agents: profile.behavior.can_spawn_sub_agents,
                auto_commit: profile.behavior.auto_commit,
                autonomy_level: format!("{:?}", profile.behavior.autonomy_level)
                    .to_lowercase()
                    .replace("_", "-"),
            },
        }
    }
}

/// List all agent profiles
#[tauri::command]
pub async fn list_agent_profiles(
    state: State<'_, AppState>,
) -> Result<Vec<AgentProfileResponse>, String> {
    state
        .agent_profile_repo
        .get_all()
        .await
        .map(|profiles| profiles.into_iter().map(AgentProfileResponse::from).collect())
        .map_err(|e| e.to_string())
}

/// Get a single agent profile by ID
#[tauri::command]
pub async fn get_agent_profile(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<AgentProfileResponse>, String> {
    let profile_id = AgentProfileId::from_string(&id);
    state
        .agent_profile_repo
        .get_by_id(&profile_id)
        .await
        .map(|opt| opt.map(AgentProfileResponse::from))
        .map_err(|e| e.to_string())
}

/// Get agent profiles by role
#[tauri::command]
pub async fn get_agent_profiles_by_role(
    role: String,
    state: State<'_, AppState>,
) -> Result<Vec<AgentProfileResponse>, String> {
    let profile_role = match role.to_lowercase().as_str() {
        "worker" => ProfileRole::Worker,
        "reviewer" => ProfileRole::Reviewer,
        "supervisor" => ProfileRole::Supervisor,
        "orchestrator" => ProfileRole::Orchestrator,
        "researcher" => ProfileRole::Researcher,
        _ => return Err(format!("Invalid role: {}", role)),
    };

    state
        .agent_profile_repo
        .get_by_role(profile_role)
        .await
        .map(|profiles| profiles.into_iter().map(AgentProfileResponse::from).collect())
        .map_err(|e| e.to_string())
}

/// Get builtin agent profiles
#[tauri::command]
pub async fn get_builtin_agent_profiles(
    state: State<'_, AppState>,
) -> Result<Vec<AgentProfileResponse>, String> {
    state
        .agent_profile_repo
        .get_builtin()
        .await
        .map(|profiles| profiles.into_iter().map(AgentProfileResponse::from).collect())
        .map_err(|e| e.to_string())
}

/// Get custom (user-created) agent profiles
#[tauri::command]
pub async fn get_custom_agent_profiles(
    state: State<'_, AppState>,
) -> Result<Vec<AgentProfileResponse>, String> {
    state
        .agent_profile_repo
        .get_custom()
        .await
        .map(|profiles| profiles.into_iter().map(AgentProfileResponse::from).collect())
        .map_err(|e| e.to_string())
}

/// Seed builtin agent profiles (idempotent)
#[tauri::command]
pub async fn seed_builtin_profiles(state: State<'_, AppState>) -> Result<(), String> {
    state
        .agent_profile_repo
        .seed_builtin_profiles()
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::memory::{
        MemoryAgentProfileRepository, MemoryProjectRepository, MemoryTaskRepository,
    };
    use std::sync::Arc;

    fn setup_test_state() -> AppState {
        AppState::new_test()
    }

    #[tokio::test]
    async fn test_list_agent_profiles_empty() {
        let state = setup_test_state();

        let profiles = state.agent_profile_repo.get_all().await.unwrap();
        assert!(profiles.is_empty());
    }

    #[tokio::test]
    async fn test_seed_and_list_builtin_profiles() {
        let state = setup_test_state();

        state.agent_profile_repo.seed_builtin_profiles().await.unwrap();

        let profiles = state.agent_profile_repo.get_all().await.unwrap();
        assert_eq!(profiles.len(), 5);
    }

    #[tokio::test]
    async fn test_get_agent_profile_by_id() {
        let state = setup_test_state();

        state.agent_profile_repo.seed_builtin_profiles().await.unwrap();

        let profile_id = AgentProfileId::from_string("worker");
        let profile = state
            .agent_profile_repo
            .get_by_id(&profile_id)
            .await
            .unwrap();
        assert!(profile.is_some());
        assert_eq!(profile.unwrap().name, "Worker");
    }

    #[tokio::test]
    async fn test_get_agent_profiles_by_role() {
        let state = setup_test_state();

        state.agent_profile_repo.seed_builtin_profiles().await.unwrap();

        let workers = state
            .agent_profile_repo
            .get_by_role(ProfileRole::Worker)
            .await
            .unwrap();
        assert_eq!(workers.len(), 1);
        assert_eq!(workers[0].role, ProfileRole::Worker);
    }

    #[tokio::test]
    async fn test_get_builtin_profiles() {
        let state = setup_test_state();

        state.agent_profile_repo.seed_builtin_profiles().await.unwrap();

        let builtin = state.agent_profile_repo.get_builtin().await.unwrap();
        assert_eq!(builtin.len(), 5);
    }

    #[tokio::test]
    async fn test_agent_profile_response_serialization() {
        let profile = AgentProfile::worker();
        let response = AgentProfileResponse::from(profile);

        assert_eq!(response.name, "Worker");
        assert_eq!(response.role, "worker");
        assert_eq!(response.execution.model, "sonnet");

        // Verify it serializes to JSON
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"name\":\"Worker\""));
        assert!(json.contains("\"role\":\"worker\""));
    }

    #[tokio::test]
    async fn test_all_builtin_profiles_have_unique_ids() {
        let state = setup_test_state();

        state.agent_profile_repo.seed_builtin_profiles().await.unwrap();

        let profiles = state.agent_profile_repo.get_all().await.unwrap();
        let ids: Vec<_> = profiles.iter().map(|p| &p.id).collect();
        let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(ids.len(), unique_ids.len());
    }
}
