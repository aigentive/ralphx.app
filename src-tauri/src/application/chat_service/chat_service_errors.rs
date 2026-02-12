// Error classification module for chat service
//
// Provides structured error type classification from string-based agent errors,
// enabling specific error handling strategies.

use crate::domain::entities::ChatConversationId;
use crate::error::AppError;

/// Classifies agent error strings into structured AppError types
///
/// # Arguments
/// * `error_message` - The error string from the agent
/// * `conversation_id` - The conversation where the error occurred
/// * `stored_session_id` - Optional stored session ID from conversation
///
/// # Returns
/// * `AppError::StaleSession` - If error indicates stale Claude session
/// * `AppError::Agent` - For all other agent errors
pub fn classify_agent_error(
    error_message: &str,
    conversation_id: &ChatConversationId,
    stored_session_id: Option<&str>,
) -> AppError {
    if error_message.contains("No conversation found with session ID") {
        if let Some(session_id) = stored_session_id {
            return AppError::StaleSession {
                session_id: session_id.to_string(),
                conversation_id: conversation_id.as_str().to_string(),
            };
        }
    }
    AppError::Agent(error_message.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_stale_session_error() {
        let error = "Error: No conversation found with session ID abc123";
        let conv_id = ChatConversationId::new();

        let classified = classify_agent_error(error, &conv_id, Some("abc123"));

        match classified {
            AppError::StaleSession { session_id, .. } => {
                assert_eq!(session_id, "abc123");
            }
            _ => panic!("Expected StaleSession error"),
        }
    }

    #[test]
    fn test_classify_stale_session_without_stored_id() {
        let error = "Error: No conversation found with session ID abc123";
        let conv_id = ChatConversationId::new();

        let classified = classify_agent_error(error, &conv_id, None);

        match classified {
            AppError::Agent(_) => {} // Falls back to Agent error
            _ => panic!("Expected Agent error when no stored session ID"),
        }
    }

    #[test]
    fn test_classify_other_error() {
        let error = "Rate limit exceeded";
        let conv_id = ChatConversationId::new();

        let classified = classify_agent_error(error, &conv_id, Some("abc123"));

        match classified {
            AppError::Agent(msg) => {
                assert_eq!(msg, "Rate limit exceeded");
            }
            _ => panic!("Expected Agent error for non-stale-session errors"),
        }
    }

    /// Regression test: Verify non-stale errors are NOT classified as StaleSession
    /// This ensures the error path in chat_service_send_background.rs will NOT
    /// trigger recovery for common errors like rate limits, network failures, etc.
    #[test]
    fn test_non_stale_errors_do_not_trigger_recovery() {
        let conversation_id = ChatConversationId::new();
        let test_cases = vec![
            "Rate limit exceeded",
            "Network timeout",
            "Command execution failed",
            "Invalid API key",
            "Permission denied",
            "Out of memory",
        ];

        for error_msg in test_cases {
            let result = classify_agent_error(error_msg, &conversation_id, Some("session-123"));

            match result {
                AppError::Agent(_) => {
                    // Expected: non-stale errors should be classified as Agent errors
                    // These will NOT match AppError::StaleSession pattern in error handling
                }
                AppError::StaleSession { .. } => {
                    panic!(
                        "Error '{}' incorrectly classified as StaleSession. \
                         This would trigger unwanted recovery attempts.",
                        error_msg
                    );
                }
                _ => {
                    panic!("Unexpected error type for: {}", error_msg);
                }
            }
        }
    }
}
