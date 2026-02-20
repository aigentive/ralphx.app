// Agent error types
// Errors specific to agent client operations

use thiserror::Error;

/// Errors that can occur during agent operations
#[derive(Error, Debug)]
pub enum AgentError {
    /// Agent with the given ID was not found
    #[error("Agent not found: {0}")]
    NotFound(String),

    /// Failed to spawn a new agent
    #[error("Agent spawn failed: {0}")]
    SpawnFailed(String),

    /// Agent spawn disallowed by environment policy
    #[error("Agent spawn not allowed: {0}")]
    SpawnNotAllowed(String),

    /// Communication with the agent failed
    #[error("Agent communication failed: {0}")]
    CommunicationFailed(String),

    /// Agent operation timed out
    #[error("Agent timeout after {0}ms")]
    Timeout(u64),

    /// CLI tool is not available
    #[error("CLI not available: {0}")]
    CliNotAvailable(String),

    /// IO error during agent operations
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type alias for agent operations
pub type AgentResult<T> = Result<T, AgentError>;

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
