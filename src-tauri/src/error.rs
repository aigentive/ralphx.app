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

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Infrastructure error: {0}")]
    Infrastructure(String),

    #[error("Git operation error: {0}")]
    GitOperation(String),

    #[error("Execution blocked: {0}")]
    ExecutionBlocked(String),
}

impl From<AgentError> for AppError {
    fn from(err: AgentError) -> Self {
        AppError::Agent(err.to_string())
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
mod tests {
    use super::*;

    #[test]
    fn test_database_error_display() {
        let err = AppError::Database("connection failed".to_string());
        assert_eq!(err.to_string(), "Database error: connection failed");
    }

    #[test]
    fn test_task_not_found_error_display() {
        let err = AppError::TaskNotFound("task-123".to_string());
        assert_eq!(err.to_string(), "Task not found: task-123");
    }

    #[test]
    fn test_project_not_found_error_display() {
        let err = AppError::ProjectNotFound("project-456".to_string());
        assert_eq!(err.to_string(), "Project not found: project-456");
    }

    #[test]
    fn test_invalid_transition_error_display() {
        let err = AppError::InvalidTransition {
            from: "backlog".to_string(),
            to: "approved".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Invalid status transition: backlog → approved"
        );
    }

    #[test]
    fn test_validation_error_display() {
        let err = AppError::Validation("title cannot be empty".to_string());
        assert_eq!(err.to_string(), "Validation error: title cannot be empty");
    }

    #[test]
    fn test_database_error_serialization() {
        let err = AppError::Database("db failure".to_string());
        let json = serde_json::to_string(&err)
            .expect("Failed to serialize Database error");
        assert_eq!(json, "\"Database error: db failure\"");
    }

    #[test]
    fn test_task_not_found_error_serialization() {
        let err = AppError::TaskNotFound("abc-123".to_string());
        let json = serde_json::to_string(&err)
            .expect("Failed to serialize TaskNotFound error");
        assert_eq!(json, "\"Task not found: abc-123\"");
    }

    #[test]
    fn test_project_not_found_error_serialization() {
        let err = AppError::ProjectNotFound("proj-789".to_string());
        let json = serde_json::to_string(&err)
            .expect("Failed to serialize ProjectNotFound error");
        assert_eq!(json, "\"Project not found: proj-789\"");
    }

    #[test]
    fn test_invalid_transition_error_serialization() {
        let err = AppError::InvalidTransition {
            from: "ready".to_string(),
            to: "cancelled".to_string(),
        };
        let json = serde_json::to_string(&err)
            .expect("Failed to serialize InvalidTransition error");
        assert_eq!(json, "\"Invalid status transition: ready → cancelled\"");
    }

    #[test]
    fn test_validation_error_serialization() {
        let err = AppError::Validation("invalid input".to_string());
        let json = serde_json::to_string(&err)
            .expect("Failed to serialize Validation error");
        assert_eq!(json, "\"Validation error: invalid input\"");
    }

    #[test]
    fn test_app_result_ok() {
        let result: AppResult<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.expect("Expected Ok value"), 42);
    }

    #[test]
    fn test_app_result_err() {
        let result: AppResult<i32> = Err(AppError::Validation("test".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_error_is_std_error() {
        let err = AppError::Database("test".to_string());
        let _: &dyn std::error::Error = &err;
    }
}
