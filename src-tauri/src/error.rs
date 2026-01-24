// Unified error handling for RalphX
// Full implementation with thiserror will be added in subsequent task

use serde::Serialize;
use std::fmt;

/// Application error type for RalphX
#[derive(Debug)]
pub enum AppError {
    /// Generic error for initial scaffolding
    Generic(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Generic(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for AppError {}

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
    fn test_error_display() {
        let err = AppError::Generic("test error".to_string());
        assert_eq!(err.to_string(), "test error");
    }

    #[test]
    fn test_error_serialization() {
        let err = AppError::Generic("serialize me".to_string());
        let json = serde_json::to_string(&err).unwrap();
        assert_eq!(json, "\"serialize me\"");
    }
}
