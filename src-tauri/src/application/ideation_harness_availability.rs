use crate::application::AppState;
use crate::domain::entities::{ChatContextType, IdeationSessionId, TaskId};
use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::agents::{
    AgentHarnessKind, AgentLane, StoredAgentLaneSettings, DEFAULT_AGENT_HARNESS,
};
use crate::domain::repositories::AgentLaneSettingsRepository;
use crate::infrastructure::agents::claude::find_claude_cli;
use crate::infrastructure::agents::find_codex_cli;

type HarnessProbeFn = fn() -> HarnessRuntimeProbe;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedLaneHarnessConfig {
    pub lane: AgentLane,
    pub configured_harness: Option<AgentHarnessKind>,
    pub fallback_harness: Option<AgentHarnessKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HarnessRuntimeProbe {
    pub binary_path: Option<String>,
    pub binary_found: bool,
    pub probe_succeeded: bool,
    pub available: bool,
    pub missing_core_exec_features: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IdeationLaneHarnessAvailability {
    pub lane: AgentLane,
    pub configured_harness: Option<AgentHarnessKind>,
    pub fallback_harness: Option<AgentHarnessKind>,
    pub effective_harness: AgentHarnessKind,
    pub fallback_activated: bool,
    pub binary_path: Option<String>,
    pub binary_found: bool,
    pub probe_succeeded: bool,
    pub available: bool,
    pub missing_core_exec_features: Vec<String>,
    pub error: Option<String>,
}

pub(crate) const IDEATION_LANES: [AgentLane; 4] = [
    AgentLane::IdeationPrimary,
    AgentLane::IdeationSubagent,
    AgentLane::IdeationVerifier,
    AgentLane::IdeationVerifierSubagent,
];

pub(crate) const AGENT_LANES: [AgentLane; 8] = [
    AgentLane::IdeationPrimary,
    AgentLane::IdeationSubagent,
    AgentLane::IdeationVerifier,
    AgentLane::IdeationVerifierSubagent,
    AgentLane::ExecutionWorker,
    AgentLane::ExecutionReviewer,
    AgentLane::ExecutionReexecutor,
    AgentLane::ExecutionMerger,
];

pub(crate) async fn resolve_ideation_lane_harness_availability(
    repo: &Arc<dyn AgentLaneSettingsRepository>,
    project_id: Option<&str>,
    lane: AgentLane,
) -> IdeationLaneHarnessAvailability {
    let config = resolve_lane_harness_config(repo, project_id, lane).await;
    let probes = probe_supported_harnesses();
    build_ideation_lane_harness_availability(config, &probes)
}

pub(crate) async fn resolve_primary_ideation_harness_availability(
    repo: &Arc<dyn AgentLaneSettingsRepository>,
    project_id: Option<&str>,
) -> IdeationLaneHarnessAvailability {
    resolve_ideation_lane_harness_availability(repo, project_id, AgentLane::IdeationPrimary).await
}

pub(crate) async fn ideation_team_mode_supported_for_project(
    repo: &Arc<dyn AgentLaneSettingsRepository>,
    project_id: Option<&str>,
) -> bool {
    resolve_primary_ideation_harness_availability(repo, project_id)
        .await
        .effective_harness
        == AgentHarnessKind::Claude
}

#[cfg(test)]
pub(crate) fn validate_claude_runtime_path(
    availability: &IdeationLaneHarnessAvailability,
    surface_name: &str,
) -> Result<(), String> {
    if !availability.available {
        return Err(availability
            .error
            .clone()
            .unwrap_or_else(|| "Configured ideation harness is not available".to_string()));
    }

    if availability.effective_harness != AgentHarnessKind::Claude {
        return Err(format!(
            "Ideation primary lane resolves to {} but {} still routes through the Claude runtime",
            availability.effective_harness, surface_name
        ));
    }

    Ok(())
}

pub(crate) async fn validate_chat_runtime_for_context(
    state: &AppState,
    context_type: ChatContextType,
    context_id: &str,
    surface_name: &str,
) -> Result<(), String> {
    let Some(lane) = runtime_lane_for_context(context_type) else {
        let probe = probe_harness(DEFAULT_AGENT_HARNESS);
        if probe.available {
            return Ok(());
        }

        return Err(probe.error.unwrap_or_else(|| {
            format!("{surface_name} requires Claude CLI but it is not available")
        }));
    };

    let project_id = project_id_for_context(state, context_type, context_id).await;
    let config =
        resolve_lane_harness_config(&state.agent_lane_settings_repo, project_id.as_deref(), lane)
            .await;
    let probes = probe_supported_harnesses();
    let availability = build_ideation_lane_harness_availability(config, &probes);

    if availability.available {
        Ok(())
    } else {
        Err(availability.error.unwrap_or_else(|| {
            format!("Configured ideation harness is not available for {surface_name}")
        }))
    }
}

pub(crate) async fn team_mode_supported_for_context(
    state: &AppState,
    context_type: ChatContextType,
    context_id: &str,
) -> bool {
    let Some(lane) = runtime_lane_for_context(context_type) else {
        return true;
    };

    let project_id = project_id_for_context(state, context_type, context_id).await;
    let config =
        resolve_lane_harness_config(&state.agent_lane_settings_repo, project_id.as_deref(), lane)
            .await;
    let probes = probe_supported_harnesses();
    let availability = build_ideation_lane_harness_availability(config, &probes);

    availability.effective_harness == AgentHarnessKind::Claude
}

fn runtime_lane_for_context(context_type: ChatContextType) -> Option<AgentLane> {
    match context_type {
        ChatContextType::Ideation => Some(AgentLane::IdeationPrimary),
        ChatContextType::TaskExecution => Some(AgentLane::ExecutionWorker),
        ChatContextType::Review => Some(AgentLane::ExecutionReviewer),
        ChatContextType::Merge => Some(AgentLane::ExecutionMerger),
        ChatContextType::Task | ChatContextType::Project => None,
    }
}

async fn project_id_for_context(
    state: &AppState,
    context_type: ChatContextType,
    context_id: &str,
) -> Option<String> {
    match context_type {
        ChatContextType::Ideation => state
            .ideation_session_repo
            .get_by_id(&IdeationSessionId::from_string(context_id))
            .await
            .ok()
            .flatten()
            .map(|session| session.project_id.as_str().to_string()),
        ChatContextType::TaskExecution | ChatContextType::Review | ChatContextType::Merge => state
            .task_repo
            .get_by_id(&TaskId::from_string(context_id.to_string()))
            .await
            .ok()
            .flatten()
            .map(|task| task.project_id.as_str().to_string()),
        ChatContextType::Task | ChatContextType::Project => None,
    }
}

pub(crate) async fn resolve_lane_harness_config(
    repo: &Arc<dyn AgentLaneSettingsRepository>,
    project_id: Option<&str>,
    lane: AgentLane,
) -> ResolvedLaneHarnessConfig {
    let project_row = if let Some(project_id) = project_id {
        repo.get_for_project(project_id, lane)
            .await
            .inspect_err(|error| {
                tracing::warn!(
                    %project_id,
                    lane = %lane,
                    %error,
                    "Failed to fetch project-scoped agent lane settings for harness availability"
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
                "Failed to fetch global agent lane settings for harness availability"
            );
        })
        .ok()
        .flatten();

    ResolvedLaneHarnessConfig {
        lane,
        configured_harness: lane_harness(project_row.as_ref(), global_row.as_ref()),
        fallback_harness: lane_fallback_harness(project_row.as_ref(), global_row.as_ref()),
    }
}

fn probe_claude_harness() -> HarnessRuntimeProbe {
    let binary_path = find_claude_cli().map(|path| path.to_string_lossy().into_owned());
    let binary_found = binary_path.is_some();
    HarnessRuntimeProbe {
        binary_path,
        binary_found,
        probe_succeeded: binary_found,
        available: binary_found,
        missing_core_exec_features: Vec::new(),
        error: if binary_found {
            None
        } else {
            Some("Claude CLI not found".to_string())
        },
    }
}

fn probe_codex_harness() -> HarnessRuntimeProbe {
    match crate::infrastructure::agents::resolve_codex_cli() {
        Ok(resolved) => {
            let binary_path = Some(resolved.path.to_string_lossy().into_owned());
            let capabilities = resolved.capabilities;
            let missing_core_exec_features = capabilities
                .missing_core_exec_features()
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>();
            let available = missing_core_exec_features.is_empty();
            let error = if available {
                None
            } else {
                Some(format!(
                    "Codex CLI is missing required capability: {}",
                    missing_core_exec_features.join(", ")
                ))
            };
            HarnessRuntimeProbe {
                binary_path,
                binary_found: true,
                probe_succeeded: true,
                available,
                missing_core_exec_features,
                error,
            }
        }
        Err(error) => match find_codex_cli() {
            Some(cli_path) => HarnessRuntimeProbe {
                binary_path: Some(cli_path.to_string_lossy().into_owned()),
                binary_found: true,
                probe_succeeded: false,
                available: false,
                missing_core_exec_features: Vec::new(),
                error: Some(error),
            },
            None => HarnessRuntimeProbe {
                binary_path: None,
                binary_found: false,
                probe_succeeded: false,
                available: false,
                missing_core_exec_features: Vec::new(),
                error: Some(error),
            },
        },
    }
}

pub(crate) fn standard_harness_probe_registry() -> HashMap<AgentHarnessKind, HarnessProbeFn> {
    HashMap::from([
        (DEFAULT_AGENT_HARNESS, probe_claude_harness as HarnessProbeFn),
        (AgentHarnessKind::Codex, probe_codex_harness as HarnessProbeFn),
    ])
}

pub(crate) fn probe_harness(harness: AgentHarnessKind) -> HarnessRuntimeProbe {
    let registry = standard_harness_probe_registry();
    registry
        .get(&harness)
        .or_else(|| registry.get(&DEFAULT_AGENT_HARNESS))
        .map(|probe| probe())
        .unwrap_or(HarnessRuntimeProbe {
            binary_path: None,
            binary_found: false,
            probe_succeeded: false,
            available: false,
            missing_core_exec_features: Vec::new(),
            error: Some(format!("No harness probe registered for {}", harness)),
        })
}

pub(crate) fn probe_supported_harnesses() -> HashMap<AgentHarnessKind, HarnessRuntimeProbe> {
    standard_harness_probe_registry()
        .into_iter()
        .map(|(harness, probe)| (harness, probe()))
        .collect()
}

fn probe_for_harness<'a>(
    probes: &'a HashMap<AgentHarnessKind, HarnessRuntimeProbe>,
    harness: AgentHarnessKind,
) -> &'a HarnessRuntimeProbe {
    probes
        .get(&harness)
        .unwrap_or_else(|| {
            probes
                .get(&DEFAULT_AGENT_HARNESS)
                .expect("default harness probe available")
        })
}

pub(crate) fn build_ideation_lane_harness_availability(
    config: ResolvedLaneHarnessConfig,
    probes: &HashMap<AgentHarnessKind, HarnessRuntimeProbe>,
) -> IdeationLaneHarnessAvailability {
    let configured_harness = config
        .configured_harness
        .unwrap_or(DEFAULT_AGENT_HARNESS);
    let configured_probe = probe_for_harness(probes, configured_harness);

    if configured_probe.available {
        return IdeationLaneHarnessAvailability {
            lane: config.lane,
            configured_harness: config.configured_harness,
            fallback_harness: config.fallback_harness,
            effective_harness: configured_harness,
            fallback_activated: false,
            binary_path: configured_probe.binary_path.clone(),
            binary_found: configured_probe.binary_found,
            probe_succeeded: configured_probe.probe_succeeded,
            available: true,
            missing_core_exec_features: configured_probe.missing_core_exec_features.clone(),
            error: configured_probe.error.clone(),
        };
    }

    if let Some(fallback_harness) = config.fallback_harness {
        let fallback_probe = probe_for_harness(probes, fallback_harness);
        if fallback_probe.available {
            return IdeationLaneHarnessAvailability {
                lane: config.lane,
                configured_harness: config.configured_harness,
                fallback_harness: config.fallback_harness,
                effective_harness: fallback_harness,
                fallback_activated: true,
                binary_path: fallback_probe.binary_path.clone(),
                binary_found: configured_probe.binary_found,
                probe_succeeded: configured_probe.probe_succeeded,
                available: true,
                missing_core_exec_features: configured_probe.missing_core_exec_features.clone(),
                error: configured_probe.error.clone(),
            };
        }
    }

    IdeationLaneHarnessAvailability {
        lane: config.lane,
        configured_harness: config.configured_harness,
        fallback_harness: config.fallback_harness,
        effective_harness: configured_harness,
        fallback_activated: false,
        binary_path: configured_probe.binary_path.clone(),
        binary_found: configured_probe.binary_found,
        probe_succeeded: configured_probe.probe_succeeded,
        available: false,
        missing_core_exec_features: configured_probe.missing_core_exec_features.clone(),
        error: configured_probe.error.clone(),
    }
}

fn lane_harness(
    project_row: Option<&StoredAgentLaneSettings>,
    global_row: Option<&StoredAgentLaneSettings>,
) -> Option<AgentHarnessKind> {
    project_row
        .map(|row| row.settings.harness)
        .or_else(|| global_row.map(|row| row.settings.harness))
}

fn lane_fallback_harness(
    project_row: Option<&StoredAgentLaneSettings>,
    global_row: Option<&StoredAgentLaneSettings>,
) -> Option<AgentHarnessKind> {
    project_row
        .and_then(|row| row.settings.fallback_harness)
        .or_else(|| global_row.and_then(|row| row.settings.fallback_harness))
}
