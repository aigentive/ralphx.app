// Unified error handling for RalphX

use serde::Serialize;
use thiserror::Error;

use crate::domain::agents::error::AgentError;

/// Application error type for RalphX
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    #[error("Invalid status transition: {from} → {to}")]
    InvalidTransition { from: String, to: String },

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Agent error: {0}")]
    Agent(String),

    #[error("Claude session expired: {session_id}")]
    StaleSession {
        session_id: String,
        conversation_id: String,
    },

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Infrastructure error: {0}")]
    Infrastructure(String),

    #[error("Git operation error: {0}")]
    GitOperation(String),

    #[error("Execution blocked: {0}")]
    ExecutionBlocked(String),

    #[error("Branch freshness conflict: branches need updating before execution can proceed")]
    BranchFreshnessConflict,
}

impl From<AgentError> for AppError {
    fn from(err: AgentError) -> Self {
        AppError::Agent(err.to_string())
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        AppError::Database(err.to_string())
    }
}

// Make errors serializable for Tauri
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Result type alias for application operations
pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
