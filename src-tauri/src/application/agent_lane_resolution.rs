use std::sync::Arc;

use crate::domain::agents::{
    AgentHarnessKind, AgentLane, AgentLaneSettings, LogicalEffort, StoredAgentLaneSettings,
    DEFAULT_AGENT_HARNESS,
};
use crate::domain::entities::ChatContextType;
use crate::domain::repositories::{
    AgentLaneSettingsRepository, IdeationEffortSettingsRepository,
    IdeationModelSettingsRepository,
};
use crate::infrastructure::agents::claude::{
    effort_bucket_for_agent, resolve_effort, resolve_ideation_effort, resolve_ideation_model,
    resolve_ideation_subagent_model_with_source, resolve_model,
    resolve_verifier_subagent_model_with_source,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedAgentSpawnSettings {
    pub configured_harness: Option<AgentHarnessKind>,
    pub effective_harness: AgentHarnessKind,
    pub configured_model: Option<String>,
    pub configured_logical_effort: Option<LogicalEffort>,
    pub configured_approval_policy: Option<String>,
    pub configured_sandbox_mode: Option<String>,
    pub model: String,
    pub logical_effort: Option<LogicalEffort>,
    pub claude_effort: Option<String>,
    pub approval_policy: Option<String>,
    pub sandbox_mode: Option<String>,
    pub configured_subagent_model_cap: Option<String>,
    pub subagent_model_cap: Option<String>,
}

pub(crate) async fn resolve_agent_spawn_settings(
    agent_name: &str,
    project_id: Option<&str>,
    context_type: ChatContextType,
    entity_status: Option<&str>,
    model_override: Option<&str>,
    agent_lane_settings_repo: Option<&Arc<dyn AgentLaneSettingsRepository>>,
    ideation_model_settings_repo: Option<&Arc<dyn IdeationModelSettingsRepository>>,
    ideation_effort_settings_repo: Option<&Arc<dyn IdeationEffortSettingsRepository>>,
) -> ResolvedAgentSpawnSettings {
    let primary_lane = lane_for_context(agent_name, context_type, entity_status);
    let subagent_lane = subagent_lane_for_context(agent_name, context_type);

    if primary_lane.is_none() {
        return ResolvedAgentSpawnSettings {
            configured_harness: None,
            effective_harness: DEFAULT_AGENT_HARNESS,
            configured_model: None,
            configured_logical_effort: None,
            configured_approval_policy: None,
            configured_sandbox_mode: None,
            model: model_override
                .map(str::to_string)
                .unwrap_or_else(|| resolve_model(Some(agent_name))),
            logical_effort: None,
            claude_effort: None,
            approval_policy: None,
            sandbox_mode: None,
            configured_subagent_model_cap: None,
            subagent_model_cap: None,
        };
    }

    let (primary_project_row, primary_global_row) =
        load_lane_rows(agent_lane_settings_repo, project_id, primary_lane).await;
    let configured_primary_settings =
        lane_settings_value(primary_project_row.as_ref(), primary_global_row.as_ref());
    let configured_harness = lane_harness(primary_project_row.as_ref(), primary_global_row.as_ref());
    let effective_harness = configured_harness.unwrap_or(DEFAULT_AGENT_HARNESS);
    let codex_primary_defaults = primary_lane.and_then(|lane| codex_default_lane_settings(
        lane,
        effective_harness,
    ));

    let model = if let Some(model_override) = model_override {
        model_override.to_string()
    } else if let Some(model) = lane_model_value(primary_project_row.as_ref(), primary_global_row.as_ref()) {
        model
    } else if let Some(model) = codex_primary_defaults
        .as_ref()
        .and_then(|settings| settings.model.clone())
    {
        model
    } else {
        resolve_legacy_ideation_model(agent_name, project_id, ideation_model_settings_repo).await
    };

    let logical_effort = if primary_lane.is_some() {
        if let Some(effort) = lane_logical_effort_value(
            primary_project_row.as_ref(),
            primary_global_row.as_ref(),
        ) {
            Some(effort)
        } else if let Some(defaults) = codex_primary_defaults.as_ref() {
            defaults.effort
        } else {
            resolve_legacy_ideation_effort(agent_name, project_id, ideation_effort_settings_repo)
                .await
        }
    } else {
        None
    };

    let (configured_subagent_model_cap, subagent_model_cap) = if let Some(subagent_lane) = subagent_lane {
        let (subagent_project_row, subagent_global_row) =
            load_lane_rows(agent_lane_settings_repo, project_id, Some(subagent_lane)).await;
        let configured_subagent_model_cap = lane_settings_value(
            subagent_project_row.as_ref(),
            subagent_global_row.as_ref(),
        )
        .and_then(|settings| settings.model);

        let subagent_model_cap = if let Some(model) = configured_subagent_model_cap.clone() {
            model
        } else if let Some(model) = codex_default_lane_settings(subagent_lane, effective_harness)
            .and_then(|settings| settings.model)
        {
            model
        } else {
            resolve_legacy_subagent_model_cap(agent_name, project_id, ideation_model_settings_repo)
                .await
        };

        (configured_subagent_model_cap, Some(subagent_model_cap))
    } else {
        (None, None)
    };

    ResolvedAgentSpawnSettings {
        configured_harness,
        effective_harness,
        configured_model: configured_primary_settings
            .as_ref()
            .and_then(|settings| settings.model.clone()),
        configured_logical_effort: configured_primary_settings
            .as_ref()
            .and_then(|settings| settings.effort),
        configured_approval_policy: configured_primary_settings
            .as_ref()
            .and_then(|settings| settings.approval_policy.clone()),
        configured_sandbox_mode: configured_primary_settings
            .as_ref()
            .and_then(|settings| settings.sandbox_mode.clone()),
        model,
        logical_effort,
        claude_effort: logical_effort.map(|effort| effort.to_legacy_claude_effort().to_string()),
        approval_policy: configured_primary_settings
            .as_ref()
            .and_then(|settings| settings.approval_policy.clone())
            .or_else(|| {
                codex_primary_defaults
                    .as_ref()
                    .and_then(|settings| settings.approval_policy.clone())
            }),
        sandbox_mode: configured_primary_settings
            .as_ref()
            .and_then(|settings| settings.sandbox_mode.clone())
            .or_else(|| {
                codex_primary_defaults
                    .as_ref()
                    .and_then(|settings| settings.sandbox_mode.clone())
            }),
        configured_subagent_model_cap,
        subagent_model_cap,
    }
}

fn ideation_lane_for_agent(agent_name: &str) -> Option<AgentLane> {
    let normalized = agent_name.strip_prefix("ralphx:").unwrap_or(agent_name);
    match normalized {
        "orchestrator-ideation"
        | "ideation-team-lead"
        | "ideation-team-member"
        | "orchestrator-ideation-readonly" => Some(AgentLane::IdeationPrimary),
        "plan-verifier" => Some(AgentLane::IdeationVerifier),
        _ => None,
    }
}

fn execution_lane_for_context(
    context_type: ChatContextType,
    entity_status: Option<&str>,
) -> Option<AgentLane> {
    match context_type {
        ChatContextType::TaskExecution => {
            if matches!(entity_status, Some("re_executing")) {
                Some(AgentLane::ExecutionReexecutor)
            } else {
                Some(AgentLane::ExecutionWorker)
            }
        }
        ChatContextType::Review => Some(AgentLane::ExecutionReviewer),
        ChatContextType::Merge => Some(AgentLane::ExecutionMerger),
        ChatContextType::Ideation | ChatContextType::Task | ChatContextType::Project => None,
    }
}

fn lane_for_context(
    agent_name: &str,
    context_type: ChatContextType,
    entity_status: Option<&str>,
) -> Option<AgentLane> {
    match context_type {
        ChatContextType::Ideation => ideation_lane_for_agent(agent_name),
        _ => execution_lane_for_context(context_type, entity_status),
    }
}

fn ideation_subagent_lane_for_agent(agent_name: &str) -> Option<AgentLane> {
    ideation_lane_for_agent(agent_name).map(|lane| match lane {
        AgentLane::IdeationVerifier => AgentLane::IdeationVerifierSubagent,
        AgentLane::IdeationPrimary => AgentLane::IdeationSubagent,
        _ => unreachable!("ideation lane mapper returned a non-ideation lane"),
    })
}

fn subagent_lane_for_context(agent_name: &str, context_type: ChatContextType) -> Option<AgentLane> {
    match context_type {
        ChatContextType::Ideation => ideation_subagent_lane_for_agent(agent_name),
        _ => None,
    }
}

async fn load_lane_rows(
    repo: Option<&Arc<dyn AgentLaneSettingsRepository>>,
    project_id: Option<&str>,
    lane: Option<AgentLane>,
) -> (
    Option<StoredAgentLaneSettings>,
    Option<StoredAgentLaneSettings>,
) {
    let Some(repo) = repo else {
        return (None, None);
    };
    let Some(lane) = lane else {
        return (None, None);
    };

    let project_row = if let Some(project_id) = project_id {
        repo.get_for_project(project_id, lane)
            .await
            .inspect_err(|error| {
                tracing::warn!(
                    %project_id,
                    lane = %lane,
                    %error,
                    "Failed to fetch project-scoped agent lane settings"
                );
            })
            .ok()
            .flatten()
    } else {
        None
    };

    let global_row = repo
        .get_global(lane)
        .await
        .inspect_err(|error| {
            tracing::warn!(
                lane = %lane,
                %error,
                "Failed to fetch global agent lane settings"
            );
        })
        .ok()
        .flatten();

    (project_row, global_row)
}

fn lane_settings_value(
    project_row: Option<&StoredAgentLaneSettings>,
    global_row: Option<&StoredAgentLaneSettings>,
) -> Option<AgentLaneSettings> {
    project_row
        .map(|row| row.settings.clone())
        .or_else(|| global_row.map(|row| row.settings.clone()))
}

fn lane_harness(
    project_row: Option<&StoredAgentLaneSettings>,
    global_row: Option<&StoredAgentLaneSettings>,
) -> Option<AgentHarnessKind> {
    project_row
        .map(|row| row.settings.harness)
        .or_else(|| global_row.map(|row| row.settings.harness))
}

fn lane_model_value(
    project_row: Option<&StoredAgentLaneSettings>,
    global_row: Option<&StoredAgentLaneSettings>,
) -> Option<String> {
    if let Some(row) = project_row {
        if let Some(model) = row.settings.model.clone() {
            return Some(model);
        }
    }

    global_row.and_then(|row| row.settings.model.clone())
}

fn lane_logical_effort_value(
    project_row: Option<&StoredAgentLaneSettings>,
    global_row: Option<&StoredAgentLaneSettings>,
) -> Option<LogicalEffort> {
    if let Some(row) = project_row {
        if let Some(effort) = row.settings.effort {
            return Some(effort);
        }
    }

    global_row.and_then(|row| row.settings.effort)
}

fn codex_default_lane_settings(
    lane: AgentLane,
    harness: AgentHarnessKind,
) -> Option<AgentLaneSettings> {
    if harness != AgentHarnessKind::Codex {
        return None;
    }

    let mut settings = AgentLaneSettings::new(AgentHarnessKind::Codex);
    settings.fallback_harness = Some(AgentHarnessKind::Claude);

    match lane {
        AgentLane::IdeationPrimary => {
            settings.model = Some("gpt-5.4".to_string());
            settings.effort = Some(LogicalEffort::XHigh);
            settings.approval_policy = Some("on-request".to_string());
            settings.sandbox_mode = Some("workspace-write".to_string());
        }
        AgentLane::IdeationVerifier => {
            settings.model = Some("gpt-5.4-mini".to_string());
            settings.effort = Some(LogicalEffort::Medium);
            settings.approval_policy = Some("on-request".to_string());
            settings.sandbox_mode = Some("workspace-write".to_string());
        }
        AgentLane::IdeationSubagent | AgentLane::IdeationVerifierSubagent => {
            settings.model = Some("gpt-5.4-mini".to_string());
            settings.effort = Some(LogicalEffort::Medium);
        }
        AgentLane::ExecutionWorker
        | AgentLane::ExecutionReviewer
        | AgentLane::ExecutionReexecutor
        | AgentLane::ExecutionMerger => {
            settings.model = Some("gpt-5.4".to_string());
            settings.effort = Some(LogicalEffort::XHigh);
            settings.approval_policy = Some("on-request".to_string());
            settings.sandbox_mode = Some("workspace-write".to_string());
        }
    }

    Some(settings)
}

async fn resolve_legacy_ideation_model(
    agent_name: &str,
    project_id: Option<&str>,
    ideation_model_settings_repo: Option<&Arc<dyn IdeationModelSettingsRepository>>,
) -> String {
    if let Some(repo) = ideation_model_settings_repo {
        return resolve_ideation_model(agent_name, project_id, repo.as_ref())
            .await
            .model;
    }

    resolve_model(Some(agent_name))
}

async fn resolve_legacy_ideation_effort(
    agent_name: &str,
    project_id: Option<&str>,
    ideation_effort_settings_repo: Option<&Arc<dyn IdeationEffortSettingsRepository>>,
) -> Option<LogicalEffort> {
    let effort = if effort_bucket_for_agent(agent_name).is_none() {
        resolve_effort(Some(agent_name))
    } else if let Some(repo) = ideation_effort_settings_repo {
        resolve_ideation_effort(agent_name, project_id, repo.as_ref()).await
    } else {
        resolve_effort(Some(agent_name))
    };

    effort.parse::<LogicalEffort>().ok().or_else(|| {
        tracing::warn!(
            agent_name,
            effort,
            "Failed to parse legacy ideation effort into provider-neutral logical effort"
        );
        None
    })
}

async fn resolve_legacy_subagent_model_cap(
    agent_name: &str,
    project_id: Option<&str>,
    ideation_model_settings_repo: Option<&Arc<dyn IdeationModelSettingsRepository>>,
) -> String {
    let Some(repo) = ideation_model_settings_repo else {
        return "haiku".to_string();
    };

    let project_settings = if let Some(project_id) = project_id {
        repo.get_for_project(project_id)
            .await
            .inspect_err(|error| {
                tracing::warn!(
                    %project_id,
                    %error,
                    "Failed to fetch project ideation model settings for legacy fallback"
                );
            })
            .ok()
            .flatten()
    } else {
        None
    };
    let global_settings = repo
        .get_global()
        .await
        .inspect_err(|error| {
            tracing::warn!(
                %error,
                "Failed to fetch global ideation model settings for legacy fallback"
            );
        })
        .ok()
        .flatten();

    if ideation_lane_for_agent(agent_name) == Some(AgentLane::IdeationVerifier) {
        resolve_verifier_subagent_model_with_source(
            project_settings.as_ref().map(|settings| &settings.verifier_subagent_model),
            global_settings.as_ref().map(|settings| &settings.verifier_subagent_model),
        )
        .0
    } else {
        resolve_ideation_subagent_model_with_source(
            project_settings.as_ref().map(|settings| &settings.ideation_subagent_model),
            global_settings.as_ref().map(|settings| &settings.ideation_subagent_model),
        )
        .0
    }
}
