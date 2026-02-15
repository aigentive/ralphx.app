// Error classification module for chat service
//
// Provides structured error type classification from string-based agent errors,
// enabling specific error handling strategies. Also defines StreamError for typed
// stream processing failures, replacing String-based error returns.

use crate::domain::entities::{ChatContextType, ChatConversationId, InternalStatus};
use crate::error::AppError;

/// Typed error for stream processing failures.
///
/// Replaces `Result<StreamOutcome, String>` with structured variants that enable
/// precise error handling decisions (retryability, session clearing, task transitions).
#[derive(Debug, Clone)]
pub enum StreamError {
    /// No stdout output received within the line-read timeout.
    Timeout {
        context_type: ChatContextType,
        elapsed_secs: u64,
    },
    /// Stdout traffic received but no parseable stream events within the parse-stall timeout.
    ParseStall {
        context_type: ChatContextType,
        elapsed_secs: u64,
        lines_seen: usize,
        lines_parsed: usize,
    },
    /// Agent process exited with non-zero status and no meaningful output.
    AgentExit {
        exit_code: Option<i32>,
        stderr: String,
    },
    /// Session ID referenced in conversation not found on the Claude side.
    SessionNotFound { session_id: String },
    /// Failed to spawn the agent CLI process.
    ProcessSpawnFailed { command: String, error: String },
    /// Agent completed but produced no meaningful output (no text, no tool calls).
    NoOutput { context_type: ChatContextType },
    /// Agent run was cancelled (e.g., user-initiated stop).
    Cancelled,
}

impl std::fmt::Display for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout {
                context_type,
                elapsed_secs,
            } => write!(
                f,
                "Agent timed out: no output for {}s (context={})",
                elapsed_secs, context_type
            ),
            Self::ParseStall {
                context_type,
                elapsed_secs,
                lines_seen,
                lines_parsed,
            } => write!(
                f,
                "Agent stream stalled: {}s without parseable events (context={}, lines_seen={}, lines_parsed={})",
                elapsed_secs, context_type, lines_seen, lines_parsed
            ),
            Self::AgentExit { exit_code, stderr } => {
                if stderr.is_empty() {
                    write!(
                        f,
                        "Agent exited with non-zero status (code={:?})",
                        exit_code
                    )
                } else {
                    write!(f, "Agent failed: {}", stderr.trim())
                }
            }
            Self::SessionNotFound { session_id } => {
                write!(f, "No conversation found with session ID {}", session_id)
            }
            Self::ProcessSpawnFailed { command, error } => {
                write!(f, "Failed to spawn agent ({}): {}", command, error)
            }
            Self::NoOutput { context_type } => {
                write!(
                    f,
                    "Agent completed with no output (context={})",
                    context_type
                )
            }
            Self::Cancelled => write!(f, "Agent run was cancelled"),
        }
    }
}

impl std::error::Error for StreamError {}

impl StreamError {
    /// Whether this error type is potentially retryable.
    ///
    /// Timeout and ParseStall may succeed on retry (transient stalls).
    /// SessionNotFound is retryable via session recovery.
    /// AgentExit may be retryable depending on the exit code.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Timeout { .. }
                | Self::ParseStall { .. }
                | Self::SessionNotFound { .. }
                | Self::AgentExit { .. }
        )
    }

    /// Whether this error requires clearing the stored Claude session ID.
    ///
    /// SessionNotFound means the session is stale and must be cleared.
    /// Timeout/ParseStall may indicate a stuck session that should be reset.
    pub fn requires_session_clear(&self) -> bool {
        matches!(
            self,
            Self::SessionNotFound { .. } | Self::Timeout { .. } | Self::ParseStall { .. }
        )
    }

    /// The suggested task status to transition to after this error.
    ///
    /// Returns `None` for non-task contexts or when no transition is appropriate.
    /// `Failed` is the default for most errors; `Cancelled` for user-initiated stops.
    pub fn suggested_task_status(&self) -> Option<InternalStatus> {
        match self {
            Self::Cancelled => Some(InternalStatus::Cancelled),
            Self::Timeout { .. }
            | Self::ParseStall { .. }
            | Self::AgentExit { .. }
            | Self::SessionNotFound { .. }
            | Self::ProcessSpawnFailed { .. }
            | Self::NoOutput { .. } => Some(InternalStatus::Failed),
        }
    }
}

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

    // =========================================================================
    // StreamError tests
    // =========================================================================

    #[test]
    fn test_stream_error_is_retryable() {
        let retryable = vec![
            StreamError::Timeout {
                context_type: ChatContextType::TaskExecution,
                elapsed_secs: 600,
            },
            StreamError::ParseStall {
                context_type: ChatContextType::TaskExecution,
                elapsed_secs: 180,
                lines_seen: 100,
                lines_parsed: 0,
            },
            StreamError::SessionNotFound {
                session_id: "abc".to_string(),
            },
            StreamError::AgentExit {
                exit_code: Some(1),
                stderr: "error".to_string(),
            },
        ];

        for err in &retryable {
            assert!(err.is_retryable(), "{} should be retryable", err);
        }

        let not_retryable = vec![
            StreamError::ProcessSpawnFailed {
                command: "claude".to_string(),
                error: "not found".to_string(),
            },
            StreamError::NoOutput {
                context_type: ChatContextType::TaskExecution,
            },
            StreamError::Cancelled,
        ];

        for err in &not_retryable {
            assert!(!err.is_retryable(), "{} should NOT be retryable", err);
        }
    }

    #[test]
    fn test_stream_error_requires_session_clear() {
        let should_clear = vec![
            StreamError::SessionNotFound {
                session_id: "abc".to_string(),
            },
            StreamError::Timeout {
                context_type: ChatContextType::TaskExecution,
                elapsed_secs: 600,
            },
            StreamError::ParseStall {
                context_type: ChatContextType::Review,
                elapsed_secs: 180,
                lines_seen: 50,
                lines_parsed: 0,
            },
        ];

        for err in &should_clear {
            assert!(
                err.requires_session_clear(),
                "{} should require session clear",
                err
            );
        }

        let should_not_clear = vec![
            StreamError::AgentExit {
                exit_code: Some(1),
                stderr: "error".to_string(),
            },
            StreamError::ProcessSpawnFailed {
                command: "claude".to_string(),
                error: "not found".to_string(),
            },
            StreamError::NoOutput {
                context_type: ChatContextType::TaskExecution,
            },
            StreamError::Cancelled,
        ];

        for err in &should_not_clear {
            assert!(
                !err.requires_session_clear(),
                "{} should NOT require session clear",
                err
            );
        }
    }

    #[test]
    fn test_stream_error_suggested_task_status() {
        // Cancelled → Cancelled status
        assert_eq!(
            StreamError::Cancelled.suggested_task_status(),
            Some(InternalStatus::Cancelled)
        );

        // All other errors → Failed status
        let failed_errors = vec![
            StreamError::Timeout {
                context_type: ChatContextType::TaskExecution,
                elapsed_secs: 600,
            },
            StreamError::ParseStall {
                context_type: ChatContextType::TaskExecution,
                elapsed_secs: 180,
                lines_seen: 100,
                lines_parsed: 0,
            },
            StreamError::AgentExit {
                exit_code: Some(1),
                stderr: "error".to_string(),
            },
            StreamError::SessionNotFound {
                session_id: "abc".to_string(),
            },
            StreamError::ProcessSpawnFailed {
                command: "claude".to_string(),
                error: "not found".to_string(),
            },
            StreamError::NoOutput {
                context_type: ChatContextType::TaskExecution,
            },
        ];

        for err in &failed_errors {
            assert_eq!(
                err.suggested_task_status(),
                Some(InternalStatus::Failed),
                "{} should suggest Failed status",
                err
            );
        }
    }

    #[test]
    fn test_stream_error_display() {
        let timeout = StreamError::Timeout {
            context_type: ChatContextType::TaskExecution,
            elapsed_secs: 600,
        };
        assert!(timeout.to_string().contains("600s"));
        assert!(timeout.to_string().contains("task_execution"));

        let parse_stall = StreamError::ParseStall {
            context_type: ChatContextType::Review,
            elapsed_secs: 180,
            lines_seen: 50,
            lines_parsed: 5,
        };
        assert!(parse_stall.to_string().contains("180s"));
        assert!(parse_stall.to_string().contains("lines_seen=50"));

        let agent_exit = StreamError::AgentExit {
            exit_code: Some(1),
            stderr: "  some error  ".to_string(),
        };
        assert!(agent_exit.to_string().contains("some error"));

        let agent_exit_empty = StreamError::AgentExit {
            exit_code: Some(42),
            stderr: "".to_string(),
        };
        assert!(agent_exit_empty.to_string().contains("42"));

        let cancelled = StreamError::Cancelled;
        assert!(cancelled.to_string().contains("cancelled"));
    }
}
