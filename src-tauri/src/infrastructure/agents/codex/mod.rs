use std::path::{Path, PathBuf};
use std::process::Command;

use crate::domain::agents::LogicalEffort;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexCliCapabilities {
    pub version: Option<String>,
    pub supports_exec_subcommand: bool,
    pub supports_json_output: bool,
    pub supports_model_flag: bool,
    pub supports_config_override: bool,
    pub supports_sandbox_flag: bool,
    pub supports_add_dir: bool,
    pub supports_search_flag: bool,
    pub supports_resume_subcommand: bool,
    pub supports_mcp_subcommand: bool,
}

impl CodexCliCapabilities {
    pub fn has_core_exec_support(&self) -> bool {
        self.missing_core_exec_features().is_empty()
    }

    pub fn missing_core_exec_features(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if !self.supports_exec_subcommand {
            missing.push("exec_subcommand");
        }
        if !self.supports_json_output {
            missing.push("json_output");
        }
        if !self.supports_model_flag {
            missing.push("model_flag");
        }
        if !self.supports_config_override {
            missing.push("config_override");
        }
        if !self.supports_sandbox_flag {
            missing.push("sandbox_flag");
        }
        if !self.supports_add_dir {
            missing.push("add_dir");
        }
        missing
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexExecCliConfig {
    pub model: Option<String>,
    pub reasoning_effort: Option<LogicalEffort>,
    pub approval_policy: Option<String>,
    pub sandbox_mode: Option<String>,
    pub cwd: Option<PathBuf>,
    pub add_dirs: Vec<PathBuf>,
    pub skip_git_repo_check: bool,
    pub json_output: bool,
    pub search: bool,
}

impl Default for CodexExecCliConfig {
    fn default() -> Self {
        Self {
            model: None,
            reasoning_effort: None,
            approval_policy: None,
            sandbox_mode: None,
            cwd: None,
            add_dirs: Vec::new(),
            skip_git_repo_check: false,
            json_output: true,
            search: false,
        }
    }
}

pub fn find_codex_cli() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("CODEX_CLI_PATH") {
        let candidate = PathBuf::from(path);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    if let Ok(path) = which::which("codex") {
        return Some(path);
    }

    for candidate in ["/opt/homebrew/bin/codex", "/usr/local/bin/codex", "/usr/bin/codex"] {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return Some(path);
        }
    }

    None
}

pub fn parse_codex_version(output: &str) -> Option<String> {
    let mut parts = output.split_whitespace();
    let binary = parts.next()?;
    let version = parts.next()?;
    if binary == "codex-cli" {
        Some(version.to_string())
    } else {
        None
    }
}

pub fn parse_codex_cli_capabilities(
    root_help: &str,
    exec_help: &str,
    version_output: Option<&str>,
) -> CodexCliCapabilities {
    CodexCliCapabilities {
        version: version_output.and_then(parse_codex_version),
        supports_exec_subcommand: root_help.contains("exec"),
        supports_json_output: exec_help.contains("--json"),
        supports_model_flag: root_help.contains("--model") && exec_help.contains("--model"),
        supports_config_override: root_help.contains("--config") && exec_help.contains("--config"),
        supports_sandbox_flag: root_help.contains("--sandbox") && exec_help.contains("--sandbox"),
        supports_add_dir: root_help.contains("--add-dir") && exec_help.contains("--add-dir"),
        supports_search_flag: root_help.contains("--search"),
        supports_resume_subcommand: root_help.contains("resume"),
        supports_mcp_subcommand: root_help.contains("mcp"),
    }
}

pub fn probe_codex_cli(cli_path: &Path) -> Result<CodexCliCapabilities, String> {
    let version_output = run_codex_command(cli_path, &["--version"])?;
    let root_help = run_codex_command(cli_path, &["--help"])?;
    let exec_help = run_codex_command(cli_path, &["exec", "--help"])?;
    Ok(parse_codex_cli_capabilities(
        &root_help,
        &exec_help,
        Some(&version_output),
    ))
}

pub fn build_codex_exec_args(
    capabilities: &CodexCliCapabilities,
    config: &CodexExecCliConfig,
) -> Result<Vec<String>, String> {
    if !capabilities.supports_exec_subcommand {
        return Err("Codex CLI does not advertise the exec subcommand".to_string());
    }

    let mut args = vec!["exec".to_string()];

    if config.json_output {
        require_capability(capabilities.supports_json_output, "json_output")?;
        args.push("--json".to_string());
    }

    if let Some(model) = config.model.as_deref() {
        require_capability(capabilities.supports_model_flag, "model_flag")?;
        args.push("-m".to_string());
        args.push(model.to_string());
    }

    if let Some(sandbox_mode) = config.sandbox_mode.as_deref() {
        require_capability(capabilities.supports_sandbox_flag, "sandbox_flag")?;
        args.push("-s".to_string());
        args.push(normalize_cli_token(sandbox_mode));
    }

    if let Some(cwd) = config.cwd.as_ref() {
        args.push("-C".to_string());
        args.push(cwd.to_string_lossy().into_owned());
    }

    for add_dir in &config.add_dirs {
        require_capability(capabilities.supports_add_dir, "add_dir")?;
        args.push("--add-dir".to_string());
        args.push(add_dir.to_string_lossy().into_owned());
    }

    if config.skip_git_repo_check {
        args.push("--skip-git-repo-check".to_string());
    }

    if config.search {
        require_capability(capabilities.supports_search_flag, "search_flag")?;
        args.push("--search".to_string());
    }

    if let Some(reasoning_effort) = config.reasoning_effort {
        require_capability(capabilities.supports_config_override, "config_override")?;
        args.push("-c".to_string());
        args.push(format!(
            "model_reasoning_effort=\"{}\"",
            reasoning_effort
        ));
    }

    if let Some(approval_policy) = config.approval_policy.as_deref() {
        require_capability(capabilities.supports_config_override, "config_override")?;
        args.push("-c".to_string());
        args.push(format!(
            "approval_policy=\"{}\"",
            normalize_cli_token(approval_policy)
        ));
    }

    Ok(args)
}

fn run_codex_command(cli_path: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new(cli_path)
        .args(args)
        .output()
        .map_err(|error| format!("Failed to run {} {:?}: {}", cli_path.display(), args, error))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(format!(
            "Command {} {:?} exited with status {}: {}",
            cli_path.display(),
            args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn require_capability(supported: bool, capability: &str) -> Result<(), String> {
    if supported {
        Ok(())
    } else {
        Err(format!("Codex CLI is missing required capability: {capability}"))
    }
}

fn normalize_cli_token(value: &str) -> String {
    value.trim().replace('_', "-")
}
