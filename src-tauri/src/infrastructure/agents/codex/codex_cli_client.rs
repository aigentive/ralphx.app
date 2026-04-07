use async_trait::async_trait;
use futures::Stream;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::Instant;
use tokio::sync::Mutex;

use crate::domain::agents::{
    AgentConfig, AgentError, AgentHandle, AgentOutput, AgentResponse, AgentResult, AgenticClient,
    ClientCapabilities, ClientType, ResponseChunk,
};
use crate::infrastructure::agents::claude::load_agent_system_prompt;

use super::{
    build_codex_mcp_overrides, build_spawnable_codex_exec_command, find_codex_cli, probe_codex_cli,
    CodexExecCliConfig,
};

lazy_static! {
    static ref PROCESSES: Mutex<HashMap<String, (tokio::process::Child, Instant)>> =
        Mutex::new(HashMap::new());
}

pub struct CodexCliClient {
    cli_path: PathBuf,
    capabilities: ClientCapabilities,
}

impl CodexCliClient {
    pub fn new() -> Self {
        let cli_path = find_codex_cli().unwrap_or_else(|| PathBuf::from("codex"));
        Self {
            cli_path,
            capabilities: ClientCapabilities::codex(),
        }
    }

    fn resolve_cli_path(&self) -> AgentResult<PathBuf> {
        if self.cli_path.exists() {
            return Ok(self.cli_path.clone());
        }

        which::which(&self.cli_path).map_err(|_| {
            AgentError::CliNotAvailable(format!("codex CLI not found at {:?}", self.cli_path))
        })
    }

    fn build_prompt(&self, config: &AgentConfig) -> String {
        let Some(plugin_dir) = config.plugin_dir.as_ref() else {
            return config.prompt.clone();
        };
        let Some(agent_name) = config.agent.as_deref() else {
            return config.prompt.clone();
        };
        let Some(system_prompt) = load_agent_system_prompt(plugin_dir, agent_name) else {
            return config.prompt.clone();
        };

        format!(
            "<ralphx_agent_instructions>\n{system_prompt}\n</ralphx_agent_instructions>\n\n{}",
            config.prompt
        )
    }

    fn build_exec_config(&self, config: &AgentConfig, config_overrides: Vec<String>) -> CodexExecCliConfig {
        CodexExecCliConfig {
            model: config.model.clone(),
            reasoning_effort: config.logical_effort,
            approval_policy: config.approval_policy.clone(),
            sandbox_mode: config.sandbox_mode.clone(),
            config_overrides,
            cwd: Some(config.working_directory.clone()),
            add_dirs: Vec::new(),
            skip_git_repo_check: false,
            json_output: true,
            search: false,
        }
    }
}

impl Default for CodexCliClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgenticClient for CodexCliClient {
    async fn spawn_agent(&self, config: AgentConfig) -> AgentResult<AgentHandle> {
        let cli_path = self.resolve_cli_path()?;
        let capabilities =
            probe_codex_cli(&cli_path).map_err(|error| AgentError::CliNotAvailable(error))?;
        if !capabilities.has_core_exec_support() {
            return Err(AgentError::CliNotAvailable(format!(
                "Codex CLI is missing required capability: {}",
                capabilities.missing_core_exec_features().join(", ")
            )));
        }

        let config_overrides = if let (Some(plugin_dir), Some(agent_name)) =
            (config.plugin_dir.as_ref(), config.agent.as_deref())
        {
            build_codex_mcp_overrides(plugin_dir, agent_name, false)
                .map_err(AgentError::SpawnFailed)?
        } else {
            Vec::new()
        };

        let prompt = self.build_prompt(&config);
        let exec_config = self.build_exec_config(&config, config_overrides);
        let mut spawnable = build_spawnable_codex_exec_command(
            &cli_path,
            &prompt,
            &capabilities,
            &exec_config,
        )
        .map_err(AgentError::SpawnFailed)?;

        for (key, value) in &config.env {
            spawnable.env(key, value);
        }

        let start_time = Instant::now();
        let child = spawnable
            .spawn()
            .await
            .map_err(|error| AgentError::SpawnFailed(error.to_string()))?;
        let handle = AgentHandle::new(ClientType::Codex, config.role);

        PROCESSES
            .lock()
            .await
            .insert(handle.id.clone(), (child, start_time));

        Ok(handle)
    }

    async fn stop_agent(&self, handle: &AgentHandle) -> AgentResult<()> {
        let mut processes = PROCESSES.lock().await;
        if let Some((mut child, _)) = processes.remove(&handle.id) {
            child
                .kill()
                .await
                .map_err(|error| AgentError::CommunicationFailed(error.to_string()))?;
        }
        Ok(())
    }

    async fn wait_for_completion(&self, handle: &AgentHandle) -> AgentResult<AgentOutput> {
        let mut processes = PROCESSES.lock().await;
        let (child, start_time) = processes
            .remove(&handle.id)
            .ok_or_else(|| AgentError::NotFound(handle.id.clone()))?;

        let output = child
            .wait_with_output()
            .await
            .map_err(|error| AgentError::CommunicationFailed(error.to_string()))?;

        Ok(AgentOutput {
            success: output.status.success(),
            content: String::from_utf8_lossy(&output.stdout).to_string(),
            exit_code: output.status.code(),
            duration_ms: Some(start_time.elapsed().as_millis() as u64),
        })
    }

    async fn send_prompt(&self, _handle: &AgentHandle, prompt: &str) -> AgentResult<AgentResponse> {
        let handle = self
            .spawn_agent(AgentConfig::worker(prompt).with_harness(crate::domain::agents::AgentHarnessKind::Codex))
            .await?;
        let output = self.wait_for_completion(&handle).await?;
        Ok(AgentResponse {
            content: output.content,
            model: Some("codex".to_string()),
            tokens_used: None,
        })
    }

    fn stream_response(
        &self,
        _handle: &AgentHandle,
        _prompt: &str,
    ) -> Pin<Box<dyn Stream<Item = AgentResult<ResponseChunk>> + Send>> {
        let chunks = vec![
            Ok(ResponseChunk::new(
                "Use codex exec JSONL handling instead of AgenticClient::stream_response",
            )),
            Ok(ResponseChunk::final_chunk("")),
        ];
        Box::pin(futures::stream::iter(chunks))
    }

    fn capabilities(&self) -> &ClientCapabilities {
        &self.capabilities
    }

    async fn is_available(&self) -> AgentResult<bool> {
        let Ok(cli_path) = self.resolve_cli_path() else {
            return Ok(false);
        };

        match probe_codex_cli(&cli_path) {
            Ok(capabilities) => Ok(capabilities.has_core_exec_support()),
            Err(_) => Ok(false),
        }
    }
}
