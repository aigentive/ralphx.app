use serde::{Deserialize, Serialize};
use tauri::State;

use crate::application::AppState;
use crate::domain::agents::{
    AgentHarnessKind, AgentModelDefinition, AgentModelRegistrySnapshot, AgentModelSource,
    LogicalEffort,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentModelResponse {
    pub provider: String,
    pub model_id: String,
    pub label: String,
    pub menu_label: String,
    pub description: Option<String>,
    pub supported_efforts: Vec<String>,
    pub default_effort: String,
    pub source: String,
    pub enabled: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertCustomAgentModelInput {
    pub provider: String,
    pub model_id: String,
    pub label: String,
    pub menu_label: Option<String>,
    pub description: Option<String>,
    pub supported_efforts: Vec<String>,
    pub default_effort: String,
    pub enabled: bool,
}

fn parse_provider(value: &str) -> Result<AgentHarnessKind, String> {
    value
        .parse::<AgentHarnessKind>()
        .map_err(|err| format!("Invalid provider: {err}"))
}

fn parse_effort(value: &str) -> Result<LogicalEffort, String> {
    value
        .parse::<LogicalEffort>()
        .map_err(|err| format!("Invalid effort: {err}"))
}

fn parse_efforts(values: &[String]) -> Result<Vec<LogicalEffort>, String> {
    values
        .iter()
        .map(|value| parse_effort(value))
        .collect::<Result<Vec<_>, _>>()
}

fn source_label(source: AgentModelSource) -> &'static str {
    match source {
        AgentModelSource::BuiltIn => "built_in",
        AgentModelSource::Custom => "custom",
    }
}

fn to_response(model: AgentModelDefinition) -> AgentModelResponse {
    AgentModelResponse {
        provider: model.provider.to_string(),
        model_id: model.model_id,
        label: model.label,
        menu_label: model.menu_label,
        description: model.description,
        supported_efforts: model
            .supported_efforts
            .into_iter()
            .map(|effort| effort.to_string())
            .collect(),
        default_effort: model.default_effort.to_string(),
        source: source_label(model.source).to_string(),
        enabled: model.enabled,
        created_at: model.created_at.map(|value| value.to_rfc3339()),
        updated_at: model.updated_at.map(|value| value.to_rfc3339()),
    }
}

fn build_custom_model(input: UpsertCustomAgentModelInput) -> Result<AgentModelDefinition, String> {
    let provider = parse_provider(&input.provider)?;
    let model_id = input.model_id.trim();
    if model_id.is_empty() {
        return Err("Model ID is required".to_string());
    }

    let supported_efforts = parse_efforts(&input.supported_efforts)?;
    if supported_efforts.is_empty() {
        return Err("At least one supported effort is required".to_string());
    }

    let default_effort = parse_effort(&input.default_effort)?;
    if !supported_efforts.contains(&default_effort) {
        return Err("Default effort must be included in supported efforts".to_string());
    }

    let label = if input.label.trim().is_empty() {
        model_id.to_string()
    } else {
        input.label
    };
    let menu_label = input
        .menu_label
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| label.clone());

    Ok(AgentModelDefinition::custom(
        provider,
        model_id,
        label,
        menu_label,
        input.description,
        supported_efforts,
        default_effort,
        input.enabled,
    )
    .normalized())
}

pub(crate) async fn load_agent_model_registry(
    app_state: &AppState,
) -> Result<AgentModelRegistrySnapshot, String> {
    let custom_models = app_state
        .agent_model_registry_repo
        .list_custom_models()
        .await
        .map_err(|error| format!("Failed to fetch custom agent models: {error}"))?;
    Ok(AgentModelRegistrySnapshot::merged(custom_models))
}

#[tauri::command]
pub async fn list_agent_models(
    app_state: State<'_, AppState>,
) -> Result<Vec<AgentModelResponse>, String> {
    let snapshot = load_agent_model_registry(&app_state).await?;
    Ok(snapshot.models.into_iter().map(to_response).collect())
}

#[tauri::command]
pub async fn upsert_custom_agent_model(
    input: UpsertCustomAgentModelInput,
    app_state: State<'_, AppState>,
) -> Result<AgentModelResponse, String> {
    let model = build_custom_model(input)?;
    let saved = app_state
        .agent_model_registry_repo
        .upsert_custom_model(&model)
        .await
        .map_err(|error| format!("Failed to save custom agent model: {error}"))?;
    Ok(to_response(saved))
}

#[tauri::command]
pub async fn delete_custom_agent_model(
    provider: String,
    model_id: String,
    app_state: State<'_, AppState>,
) -> Result<bool, String> {
    let provider = parse_provider(&provider)?;
    let model_id = model_id.trim();
    if model_id.is_empty() {
        return Err("Model ID is required".to_string());
    }

    app_state
        .agent_model_registry_repo
        .delete_custom_model(provider, model_id)
        .await
        .map_err(|error| format!("Failed to delete custom agent model: {error}"))
}
