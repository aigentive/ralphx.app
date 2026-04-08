use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::domain::agents::{standard_harness_registry, AgentHarnessKind};
use crate::infrastructure::agents::claude::find_claude_cli;
use crate::infrastructure::agents::{find_codex_cli, resolve_codex_cli, CodexCliCapabilities};
use which::which;

pub(crate) type HarnessProbeFn = fn() -> HarnessRuntimeProbe;
pub(crate) type ChatHarnessCliResolver = fn(&Path) -> Result<ResolvedChatHarnessCli, String>;

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

#[derive(Clone, Copy)]
pub(crate) struct HarnessRuntimeAdapter {
    pub probe: HarnessProbeFn,
    pub resolve_chat_cli: ChatHarnessCliResolver,
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

pub(crate) fn standard_harness_runtime_adapters() -> HashMap<AgentHarnessKind, HarnessRuntimeAdapter>
{
    standard_harness_registry(|harness| match harness {
        AgentHarnessKind::Claude => HarnessRuntimeAdapter {
            probe: probe_claude_harness,
            resolve_chat_cli: resolve_claude_chat_harness_cli,
        },
        AgentHarnessKind::Codex => HarnessRuntimeAdapter {
            probe: probe_codex_harness,
            resolve_chat_cli: resolve_codex_chat_harness_cli,
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

pub(crate) fn probe_supported_harnesses() -> HashMap<AgentHarnessKind, HarnessRuntimeProbe> {
    standard_harness_runtime_adapters()
        .into_iter()
        .map(|(harness, adapter)| (harness, (adapter.probe)()))
        .collect()
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
