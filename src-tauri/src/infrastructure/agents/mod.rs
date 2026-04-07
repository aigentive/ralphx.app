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
    build_codex_exec_args, find_codex_cli, parse_codex_cli_capabilities, parse_codex_version,
    probe_codex_cli, CodexCliCapabilities, CodexExecCliConfig,
};
pub use mock::{MockAgenticClient, MockCall, MockCallType};
pub use spawner::AgenticClientSpawner;
