use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::domain::agents::{standard_harness_registry, AgentHarnessKind, DEFAULT_AGENT_HARNESS};
use crate::infrastructure::agents::claude::{
    external_mcp_config, find_claude_cli, node_utils, register_mcp_server, resolve_plugin_dir,
    validate_external_mcp_config, ExternalMcpConfig,
};
use crate::infrastructure::agents::{find_codex_cli, resolve_codex_cli, CodexCliCapabilities};
use which::which;

pub(crate) type HarnessProbeFn = fn() -> HarnessRuntimeProbe;
pub(crate) type ChatHarnessCliResolver = fn(&Path) -> Result<ResolvedChatHarnessCli, String>;
pub(crate) type StartupHarnessIntegrationResolver =
    fn() -> Result<Option<ResolvedHarnessStartupIntegration>, String>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HarnessRuntimeProbe {
    pub binary_path: Option<String>,
    pub binary_found: bool,
    pub probe_succeeded: bool,
    pub available: bool,
    pub missing_core_exec_features: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug)]
pub(crate) enum ResolvedChatHarnessCli {
    Claude {
        cli_path: PathBuf,
    },
    Codex {
        cli_path: PathBuf,
        capabilities: CodexCliCapabilities,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ResolvedHarnessStartupIntegration {
    RegisterConfiguredMcpServer {
        harness: AgentHarnessKind,
        cli_path: PathBuf,
        plugin_dir: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DefaultChatServiceBootstrap {
    pub cli_path: PathBuf,
    pub plugin_dir: PathBuf,
    pub default_working_directory: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct DefaultExternalMcpBootstrap {
    pub config: ExternalMcpConfig,
    pub node_path: PathBuf,
    pub entry_path: PathBuf,
}

impl ResolvedHarnessStartupIntegration {
    pub(crate) fn harness(&self) -> AgentHarnessKind {
        match self {
            Self::RegisterConfiguredMcpServer { harness, .. } => *harness,
        }
    }

    pub(crate) fn description(&self) -> &'static str {
        match self {
            Self::RegisterConfiguredMcpServer { .. } => "configured MCP server registration",
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct HarnessRuntimeAdapter {
    pub probe: HarnessProbeFn,
    pub resolve_chat_cli: ChatHarnessCliResolver,
    pub resolve_startup_integration: StartupHarnessIntegrationResolver,
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
    match resolve_codex_cli() {
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

fn resolve_claude_chat_harness_cli(
    claude_cli_path: &Path,
) -> Result<ResolvedChatHarnessCli, String> {
    if !claude_cli_path.exists() && which(claude_cli_path).is_err() {
        return Err(format!(
            "Claude CLI not found at {}",
            claude_cli_path.display()
        ));
    }

    Ok(ResolvedChatHarnessCli::Claude {
        cli_path: claude_cli_path.to_path_buf(),
    })
}

fn resolve_codex_chat_harness_cli(_: &Path) -> Result<ResolvedChatHarnessCli, String> {
    let resolved_codex = resolve_codex_cli()?;
    Ok(ResolvedChatHarnessCli::Codex {
        cli_path: resolved_codex.path,
        capabilities: resolved_codex.capabilities,
    })
}

fn resolve_claude_startup_integration() -> Result<Option<ResolvedHarnessStartupIntegration>, String>
{
    let cli_path = find_claude_cli().ok_or_else(|| "Claude CLI not found".to_string())?;
    let plugin_dir = crate::infrastructure::agents::claude::find_plugin_dir()
        .ok_or_else(|| "Claude plugin directory not found".to_string())?;
    Ok(Some(
        ResolvedHarnessStartupIntegration::RegisterConfiguredMcpServer {
            harness: AgentHarnessKind::Claude,
            cli_path,
            plugin_dir,
        },
    ))
}

fn resolve_codex_startup_integration() -> Result<Option<ResolvedHarnessStartupIntegration>, String>
{
    Ok(None)
}

pub(crate) fn standard_harness_runtime_adapters() -> HashMap<AgentHarnessKind, HarnessRuntimeAdapter>
{
    standard_harness_registry(|harness| match harness {
        AgentHarnessKind::Claude => HarnessRuntimeAdapter {
            probe: probe_claude_harness,
            resolve_chat_cli: resolve_claude_chat_harness_cli,
            resolve_startup_integration: resolve_claude_startup_integration,
        },
        AgentHarnessKind::Codex => HarnessRuntimeAdapter {
            probe: probe_codex_harness,
            resolve_chat_cli: resolve_codex_chat_harness_cli,
            resolve_startup_integration: resolve_codex_startup_integration,
        },
    })
}

pub(crate) fn standard_harness_probe_registry() -> HashMap<AgentHarnessKind, HarnessProbeFn> {
    standard_harness_runtime_adapters()
        .into_iter()
        .map(|(harness, adapter)| (harness, adapter.probe))
        .collect()
}

pub(crate) fn standard_chat_harness_cli_resolvers(
) -> HashMap<AgentHarnessKind, ChatHarnessCliResolver> {
    standard_harness_runtime_adapters()
        .into_iter()
        .map(|(harness, adapter)| (harness, adapter.resolve_chat_cli))
        .collect()
}

pub(crate) fn probe_harness(harness: AgentHarnessKind) -> HarnessRuntimeProbe {
    let adapters = standard_harness_runtime_adapters();
    adapters
        .get(&harness)
        .map(|adapter| (adapter.probe)())
        .unwrap_or(HarnessRuntimeProbe {
            binary_path: None,
            binary_found: false,
            probe_succeeded: false,
            available: false,
            missing_core_exec_features: Vec::new(),
            error: Some(format!("No harness probe registered for {}", harness)),
        })
}

pub(crate) fn probe_default_harness() -> HarnessRuntimeProbe {
    probe_harness(DEFAULT_AGENT_HARNESS)
}

pub(crate) fn default_harness_runtime_available() -> bool {
    probe_default_harness().available
}

fn default_chat_service_working_directory(cwd: PathBuf) -> PathBuf {
    if cwd.file_name().is_some_and(|name| name == "src-tauri") {
        cwd.parent()
            .map(|parent| parent.to_path_buf())
            .unwrap_or(cwd)
    } else {
        cwd
    }
}

pub(crate) fn resolve_default_chat_service_bootstrap() -> DefaultChatServiceBootstrap {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let default_working_directory = default_chat_service_working_directory(cwd);
    DefaultChatServiceBootstrap {
        cli_path: find_claude_cli().unwrap_or_else(|| PathBuf::from("claude")),
        plugin_dir: resolve_plugin_dir(&default_working_directory),
        default_working_directory,
    }
}

pub(crate) fn resolve_default_external_mcp_bootstrap(
) -> Result<Option<DefaultExternalMcpBootstrap>, String> {
    let config = external_mcp_config().clone();
    if !config.enabled {
        return Ok(None);
    }

    validate_external_mcp_config(&config)?;

    let entry_path = find_claude_external_mcp_entry()
        .ok_or_else(|| "Plugin dir not found, cannot start external MCP".to_string())?;

    if !entry_path.exists() {
        return Err(format!(
            "External MCP entry not found at {} — run `npm run build` in plugins/app/ralphx-external-mcp",
            entry_path.display()
        ));
    }

    Ok(Some(DefaultExternalMcpBootstrap {
        config,
        node_path: node_utils::find_node_binary(),
        entry_path,
    }))
}

fn find_claude_external_mcp_entry() -> Option<PathBuf> {
    crate::infrastructure::agents::claude::find_plugin_dir()
        .map(|plugin_dir| external_mcp_entry_for_plugin_dir(&plugin_dir))
}

fn external_mcp_entry_for_plugin_dir(plugin_dir: &Path) -> PathBuf {
    plugin_dir.join("ralphx-external-mcp/build/index.js")
}

pub(crate) fn probe_supported_harnesses() -> HashMap<AgentHarnessKind, HarnessRuntimeProbe> {
    standard_harness_runtime_adapters()
        .into_iter()
        .map(|(harness, adapter)| (harness, (adapter.probe)()))
        .collect()
}

pub(crate) fn probe_codex_harness_with_capabilities(
) -> (HarnessRuntimeProbe, Option<CodexCliCapabilities>) {
    match resolve_codex_cli() {
        Ok(resolved) => {
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
            (
                HarnessRuntimeProbe {
                    binary_path: Some(resolved.path.to_string_lossy().into_owned()),
                    binary_found: true,
                    probe_succeeded: true,
                    available,
                    missing_core_exec_features,
                    error,
                },
                Some(capabilities),
            )
        }
        Err(error) => {
            let probe = match find_codex_cli() {
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
            };
            (probe, None)
        }
    }
}

pub(crate) fn resolve_chat_harness_cli(
    harness: AgentHarnessKind,
    claude_cli_path: &Path,
) -> Result<ResolvedChatHarnessCli, String> {
    let adapters = standard_harness_runtime_adapters();
    let adapter = adapters
        .get(&harness)
        .copied()
        .ok_or_else(|| format!("No chat harness CLI resolver registered for {}", harness))?;
    (adapter.resolve_chat_cli)(claude_cli_path)
}

pub(crate) fn resolve_startup_harness_integration(
    harness: AgentHarnessKind,
) -> Result<Option<ResolvedHarnessStartupIntegration>, String> {
    let adapters = standard_harness_runtime_adapters();
    let adapter = adapters
        .get(&harness)
        .copied()
        .ok_or_else(|| format!("No startup harness integration registered for {}", harness))?;
    (adapter.resolve_startup_integration)()
}

pub(crate) async fn run_startup_harness_integration(
    integration: ResolvedHarnessStartupIntegration,
) -> Result<(), String> {
    match integration {
        ResolvedHarnessStartupIntegration::RegisterConfiguredMcpServer {
            cli_path,
            plugin_dir,
            ..
        } => register_mcp_server(&cli_path, &plugin_dir).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_startup_harness_integration_returns_none_for_codex() {
        let integration = resolve_startup_harness_integration(AgentHarnessKind::Codex).unwrap();
        assert!(integration.is_none());
    }

    #[test]
    fn startup_integration_description_matches_variant() {
        let integration = ResolvedHarnessStartupIntegration::RegisterConfiguredMcpServer {
            harness: AgentHarnessKind::Claude,
            cli_path: PathBuf::from("claude"),
            plugin_dir: PathBuf::from("plugins/app"),
        };
        assert_eq!(integration.harness(), AgentHarnessKind::Claude);
        assert_eq!(
            integration.description(),
            "configured MCP server registration"
        );
    }

    #[test]
    fn default_chat_service_working_directory_uses_parent_for_src_tauri() {
        let cwd = PathBuf::from("/tmp/example/src-tauri");
        assert_eq!(
            default_chat_service_working_directory(cwd),
            PathBuf::from("/tmp/example")
        );
    }

    #[test]
    fn default_chat_service_working_directory_keeps_non_src_tauri_paths() {
        let cwd = PathBuf::from("/tmp/example");
        assert_eq!(default_chat_service_working_directory(cwd.clone()), cwd);
    }

    #[test]
    fn external_mcp_entry_for_plugin_dir_appends_expected_relative_path() {
        let plugin_dir = PathBuf::from("/tmp/plugins/app");
        assert_eq!(
            external_mcp_entry_for_plugin_dir(&plugin_dir),
            plugin_dir.join("ralphx-external-mcp/build/index.js")
        );
    }
}
