// Agent implementations
// Infrastructure layer implementations of the AgenticClient trait

pub mod claude;
pub mod codex;
pub mod mock;
pub mod spawner;

// Re-export commonly used items
pub use claude::ClaudeCodeClient;
pub use claude::{
    StreamEvent, StreamingSpawnResult, TeammateContext, TeammateSpawnConfig, TeammateSpawnResult,
};
pub use codex::{
    build_codex_exec_args, build_codex_exec_resume_args, build_spawnable_codex_exec_command,
    build_spawnable_codex_resume_command, find_codex_cli, parse_codex_cli_capabilities,
    parse_codex_version, probe_codex_cli, CodexCliCapabilities, CodexExecCliConfig,
};
pub use codex::stream_processor::{
    extract_codex_agent_message, extract_codex_command_execution, extract_codex_error_message,
    extract_codex_tool_call, extract_codex_usage, parse_codex_event_line, CodexCommandExecution,
    CodexItem, CodexItemError, CodexStreamEvent, CodexUsage,
};
pub use mock::{MockAgenticClient, MockCall, MockCallType};
pub use spawner::AgenticClientSpawner;
