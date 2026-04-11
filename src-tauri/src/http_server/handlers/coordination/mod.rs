use axum::{extract::State, http::StatusCode, Json};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::application::agent_lane_resolution::resolve_agent_spawn_settings;
use crate::application::harness_runtime_registry::resolve_harness_plugin_dir;
use crate::domain::agents::{AgentConfig, AgentRole};
use crate::domain::entities::{ChatContextType, IdeationSessionId};
use crate::http_server::delegation::DelegationJobSnapshot;
use crate::http_server::handlers::session_linking::create_child_session_impl;
use crate::http_server::types::{
    CreateChildSessionRequest, DelegateCancelRequest, DelegateStartRequest, DelegateWaitRequest,
    HttpServerState,
};
use crate::infrastructure::agents::claude::mcp_agent_type;
use crate::infrastructure::agents::harness_agent_catalog::{
    load_canonical_agent_definition, resolve_project_root_from_plugin_dir,
};

type JsonError = (StatusCode, Json<serde_json::Value>);

fn json_error(status: StatusCode, error: impl Into<String>) -> JsonError {
    (
        status,
        Json(serde_json::json!({
            "status": status.as_u16(),
            "error": error.into(),
        })),
    )
}

fn agent_role_for(agent_name: &str, role: &str) -> AgentRole {
    match role {
        "worker" => AgentRole::Worker,
        "reviewer" => AgentRole::Reviewer,
        "qa_prep" | "qa-prep" => AgentRole::QaPrep,
        "qa_refiner" | "qa-refiner" => AgentRole::QaRefiner,
        "qa_tester" | "qa-tester" => AgentRole::QaTester,
        "supervisor" => AgentRole::Supervisor,
        _ => AgentRole::Custom(agent_name.to_string()),
    }
}

async fn resolve_child_session_id(
    state: &HttpServerState,
    req: &DelegateStartRequest,
) -> Result<String, JsonError> {
    if let Some(child_session_id) = &req.child_session_id {
        let child_id = IdeationSessionId::from_string(child_session_id.clone());
        let child = state
            .app_state
            .ideation_session_repo
            .get_by_id(&child_id)
            .await
            .map_err(|error| {
                json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to load child session: {error}"),
                )
            })?
            .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "Child session not found"))?;
        if child.parent_session_id.as_ref().map(|id| id.as_str()) != Some(req.parent_session_id.as_str()) {
            return Err(json_error(
                StatusCode::BAD_REQUEST,
                "Child session does not belong to the provided parent session",
            ));
        }
        return Ok(child_session_id.clone());
    }

    let response = create_child_session_impl(
        state,
        CreateChildSessionRequest {
            parent_session_id: req.parent_session_id.clone(),
            title: req.title.clone(),
            description: None,
            inherit_context: req.inherit_context,
            initial_prompt: None,
            team_mode: Some("solo".to_string()),
            team_config: None,
            purpose: Some("general".to_string()),
            is_external_trigger: false,
            source_task_id: None,
            source_context_type: None,
            source_context_id: None,
            spawn_reason: Some("delegation_bridge".to_string()),
            blocker_fingerprint: None,
        },
    )
    .await?;

    Ok(response.session_id)
}

async fn load_parent_project_working_directory(
    state: &HttpServerState,
    parent_session_id: &str,
) -> Result<(String, PathBuf), JsonError> {
    let parent_id = IdeationSessionId::from_string(parent_session_id.to_string());
    let parent = state
        .app_state
        .ideation_session_repo
        .get_by_id(&parent_id)
        .await
        .map_err(|error| {
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to load parent session: {error}"),
            )
        })?
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "Parent session not found"))?;

    let project = state
        .app_state
        .project_repo
        .get_by_id(&parent.project_id)
        .await
        .map_err(|error| {
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to load parent project: {error}"),
            )
        })?
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "Parent project not found"))?;

    Ok((
        parent.project_id.as_str().to_string(),
        PathBuf::from(project.working_directory),
    ))
}

fn build_delegated_prompt(
    agent_name: &str,
    parent_session_id: &str,
    child_session_id: &str,
    prompt: &str,
) -> String {
    format!(
        "You are running as delegated RalphX specialist `{agent_name}`.\nParent ideation session: `{parent_session_id}`\nChild session: `{child_session_id}`\nOperate through the RalphX MCP tools available to your role and treat the child session as your working context.\n\nDelegated task:\n{prompt}"
    )
}

pub(crate) async fn start_delegate_impl(
    state: &HttpServerState,
    req: DelegateStartRequest,
) -> Result<DelegationJobSnapshot, JsonError> {
    let child_session_id = resolve_child_session_id(state, &req).await?;
    let (project_id, working_directory) =
        load_parent_project_working_directory(state, &req.parent_session_id).await?;

    let resolved_spawn = resolve_agent_spawn_settings(
        &req.agent_name,
        Some(project_id.as_str()),
        ChatContextType::Ideation,
        None,
        req.harness,
        req.model.as_deref(),
        Some(&state.app_state.agent_lane_settings_repo),
        Some(&state.app_state.ideation_model_settings_repo),
        Some(&state.app_state.ideation_effort_settings_repo),
    )
    .await;
    let harness = resolved_spawn.effective_harness;
    let plugin_dir = resolve_harness_plugin_dir(harness, &working_directory);
    let project_root = resolve_project_root_from_plugin_dir(&plugin_dir);
    let definition =
        load_canonical_agent_definition(&project_root, &req.agent_name).ok_or_else(|| {
            json_error(
                StatusCode::BAD_REQUEST,
                format!("Unknown canonical agent '{}'", req.agent_name),
            )
        })?;

    let mut env = HashMap::new();
    env.insert(
        "RALPHX_AGENT_TYPE".to_string(),
        mcp_agent_type(&definition.name).to_string(),
    );
    env.insert("RALPHX_CONTEXT_TYPE".to_string(), "ideation".to_string());
    env.insert("RALPHX_CONTEXT_ID".to_string(), child_session_id.clone());
    env.insert("RALPHX_PROJECT_ID".to_string(), project_id);
    env.insert(
        "RALPHX_PARENT_SESSION_ID".to_string(),
        req.parent_session_id.clone(),
    );

    let config = AgentConfig {
        role: agent_role_for(&definition.name, &definition.role),
        prompt: build_delegated_prompt(
            &definition.name,
            &req.parent_session_id,
            &child_session_id,
            &req.prompt,
        ),
        working_directory: working_directory.clone(),
        plugin_dir: Some(plugin_dir),
        agent: Some(definition.name.clone()),
        model: Some(resolved_spawn.model.clone()),
        harness: Some(harness),
        logical_effort: req.logical_effort.or(resolved_spawn.logical_effort),
        approval_policy: req
            .approval_policy
            .clone()
            .or(resolved_spawn.approval_policy.clone()),
        sandbox_mode: req
            .sandbox_mode
            .clone()
            .or(resolved_spawn.sandbox_mode.clone()),
        max_tokens: None,
        timeout_secs: None,
        env,
    };

    let client = state.app_state.resolve_harness_agent_client(harness);
    let handle = client.spawn_agent(config).await.map_err(|error| {
        json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to spawn delegated agent: {error}"),
        )
    })?;

    let job_id = uuid::Uuid::new_v4().to_string();
    let snapshot = state
        .delegation_service
        .register_running(
            job_id.clone(),
            req.parent_session_id.clone(),
            child_session_id,
            definition.name.clone(),
            harness,
            handle.clone(),
        )
        .await;

    let delegation_service = state.delegation_service.clone();
    tokio::spawn(async move {
        match client.wait_for_completion(&handle).await {
            Ok(output) if output.success => {
                delegation_service
                    .mark_completed(&job_id, output.content)
                    .await;
            }
            Ok(output) => {
                let detail = if output.content.trim().is_empty() {
                    format!(
                        "Delegated agent exited unsuccessfully with code {:?}",
                        output.exit_code
                    )
                } else {
                    output.content
                };
                delegation_service.mark_failed(&job_id, detail).await;
            }
            Err(error) => {
                delegation_service
                    .mark_failed(&job_id, error.to_string())
                    .await;
            }
        }
    });

    Ok(snapshot)
}

pub async fn start_delegate(
    State(state): State<HttpServerState>,
    Json(req): Json<DelegateStartRequest>,
) -> Result<Json<DelegationJobSnapshot>, JsonError> {
    Ok(Json(start_delegate_impl(&state, req).await?))
}

pub async fn wait_delegate(
    State(state): State<HttpServerState>,
    Json(req): Json<DelegateWaitRequest>,
) -> Result<Json<DelegationJobSnapshot>, JsonError> {
    let snapshot = state
        .delegation_service
        .snapshot(&req.job_id)
        .await
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "Delegation job not found"))?;
    Ok(Json(snapshot))
}

pub async fn cancel_delegate(
    State(state): State<HttpServerState>,
    Json(req): Json<DelegateCancelRequest>,
) -> Result<Json<DelegationJobSnapshot>, JsonError> {
    let (harness, handle, snapshot) = state
        .delegation_service
        .cancel(&req.job_id)
        .await
        .ok_or_else(|| {
            json_error(
                StatusCode::NOT_FOUND,
                "Delegation job not found or no longer cancellable",
            )
        })?;
    let client = state.app_state.resolve_harness_agent_client(harness);
    client.stop_agent(&handle).await.map_err(|error| {
        json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to stop delegated agent: {error}"),
        )
    })?;
    Ok(Json(snapshot))
}
