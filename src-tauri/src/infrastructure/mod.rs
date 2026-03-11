// Infrastructure layer - external implementations
// SQLite, file system, Claude CLI interactions

pub mod agents;
pub mod memory;
pub mod sqlite;
pub mod supervisor;
pub mod external_mcp_supervisor;

// Re-export commonly used items
pub use agents::{
    AgenticClientSpawner, ClaudeCodeClient, MockAgenticClient, MockCall, MockCallType,
};
pub use sqlite::{get_default_db_path, open_connection, open_memory_connection, run_migrations};
pub use supervisor::{EventBus, EventSubscriber};
pub use external_mcp_supervisor::{ExternalMcpHandle, ExternalMcpSupervisor};

#[cfg(test)]
mod external_mcp_supervisor_tests;
