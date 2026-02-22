// Error classification module for chat service
//
// Provides structured error type classification from string-based agent errors,
// enabling specific error handling strategies. Also defines StreamError for typed
// stream processing failures, replacing String-based error returns.

use crate::domain::entities::{ChatContextType, ChatConversationId, InternalStatus};
use crate::error::AppError;
use crate::infrastructure::agents::claude::limits_config;
use serde::{Deserialize, Serialize};

/// Claude CLI error message indicating an expired/invalid session.
/// Source: Claude CLI stderr when resuming with a stale session ID.
pub const STALE_SESSION_ERROR: &str = "No conversation found with session ID";

/// Category of provider/API error for recovery decisions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderErrorCategory {
    /// HTTP 429 or usage limit exceeded
    RateLimit,
    /// HTTP 401/403 or invalid API key
    AuthError,
    /// HTTP 5xx from provider
    ServerError,
    /// Connection refused, DNS failure, network timeout
    NetworkError,
    /// Overloaded API (Claude-specific overloaded_error)
    Overloaded,
}

impl std::fmt::Display for ProviderErrorCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RateLimit => write!(f, "rate_limit"),
            Self::AuthError => write!(f, "auth_error"),
            Self::ServerError => write!(f, "server_error"),
            Self::NetworkError => write!(f, "network_error"),
            Self::Overloaded => write!(f, "overloaded"),
        }
    }
}

/// Metadata stored in task.metadata when paused due to provider error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderErrorMetadata {
    pub category: ProviderErrorCategory,
    pub message: String,
    /// ISO 8601 timestamp when the error's limit resets (parsed from error message)
    pub retry_after: Option<String>,
    /// The task status before pausing (for resuming to correct state)
    pub previous_status: String,
    /// When the task was paused
    pub paused_at: String,
    /// Whether the system should auto-resume this task
    pub auto_resumable: bool,
    /// Number of auto-resume attempts so far
    #[serde(default)]
    pub resume_attempts: u32,
}

impl ProviderErrorMetadata {
    /// Maximum auto-resume attempts before giving up (read from runtime config).
    pub fn max_resume_attempts() -> u32 {
        limits_config().max_resume_attempts as u32
    }

    /// Read provider_error metadata from task metadata JSON string.
    pub fn from_task_metadata(metadata: Option<&str>) -> Option<Self> {
        let json: serde_json::Value = serde_json::from_str(metadata?).ok()?;
        let provider_error = json.get("provider_error")?;
        serde_json::from_value(provider_error.clone()).ok()
    }

    /// Write provider_error metadata into task metadata JSON string.
    pub fn write_to_task_metadata(&self, existing_metadata: Option<&str>) -> String {
        let mut json: serde_json::Value = existing_metadata
            .and_then(|m| serde_json::from_str(m).ok())
            .unwrap_or_else(|| serde_json::json!({}));

        if let Some(obj) = json.as_object_mut() {
            obj.insert(
                "provider_error".to_string(),
                serde_json::to_value(self).unwrap_or_default(),
            );
        }

        json.to_string()
    }

    /// Remove provider_error metadata from task metadata (on successful resume).
    pub fn clear_from_task_metadata(existing_metadata: Option<&str>) -> String {
        let mut json: serde_json::Value = existing_metadata
            .and_then(|m| serde_json::from_str(m).ok())
            .unwrap_or_else(|| serde_json::json!({}));

        if let Some(obj) = json.as_object_mut() {
            obj.remove("provider_error");
        }

        json.to_string()
    }

    /// Check if retry_after time has passed.
    pub fn is_retry_eligible(&self) -> bool {
        if self.resume_attempts >= Self::max_resume_attempts() {
            return false;
        }
        if !self.auto_resumable {
            return false;
        }
        match &self.retry_after {
            Some(retry_after_str) => {
                chrono::DateTime::parse_from_rfc3339(retry_after_str)
                    .map(|dt| chrono::Utc::now() >= dt)
                    .unwrap_or(true) // If can't parse, allow retry
            }
            None => true, // No retry_after means retry immediately
        }
    }
}

/// Unified pause reason metadata stored under `task.metadata.pause_reason`.
///
/// Distinguishes user-initiated pauses from provider-error pauses so the
/// frontend can render appropriate UI and reconciliation can skip user-paused tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PauseReason {
    /// User clicked pause (global or per-task)
    UserInitiated {
        previous_status: String,
        paused_at: String,
        /// "global" for pause_execution, "task" for per-task pause
        scope: String,
    },
    /// Provider/API error caused automatic pause
    ProviderError {
        category: ProviderErrorCategory,
        message: String,
        retry_after: Option<String>,
        previous_status: String,
        paused_at: String,
        auto_resumable: bool,
        #[serde(default)]
        resume_attempts: u32,
    },
}

impl PauseReason {
    /// Metadata key used in task.metadata JSON
    const KEY: &'static str = "pause_reason";

    /// Read pause_reason from task metadata JSON string.
    /// Also checks legacy `provider_error` key for backward compatibility.
    pub fn from_task_metadata(metadata: Option<&str>) -> Option<Self> {
        let json: serde_json::Value = serde_json::from_str(metadata?).ok()?;

        // Try new key first
        if let Some(val) = json.get(Self::KEY) {
            if let Ok(reason) = serde_json::from_value::<Self>(val.clone()) {
                return Some(reason);
            }
        }

        // Backward compat: read old provider_error key and convert
        if let Some(val) = json.get("provider_error") {
            if let Ok(old) = serde_json::from_value::<ProviderErrorMetadata>(val.clone()) {
                return Some(Self::ProviderError {
                    category: old.category,
                    message: old.message,
                    retry_after: old.retry_after,
                    previous_status: old.previous_status,
                    paused_at: old.paused_at,
                    auto_resumable: old.auto_resumable,
                    resume_attempts: old.resume_attempts,
                });
            }
        }

        None
    }

    /// Write pause_reason into task metadata JSON string.
    pub fn write_to_task_metadata(&self, existing_metadata: Option<&str>) -> String {
        let mut json: serde_json::Value = existing_metadata
            .and_then(|m| serde_json::from_str(m).ok())
            .unwrap_or_else(|| serde_json::json!({}));

        if let Some(obj) = json.as_object_mut() {
            obj.insert(
                Self::KEY.to_string(),
                serde_json::to_value(self).unwrap_or_default(),
            );
        }

        json.to_string()
    }

    /// Remove pause_reason (and legacy provider_error) from task metadata.
    pub fn clear_from_task_metadata(existing_metadata: Option<&str>) -> String {
        let mut json: serde_json::Value = existing_metadata
            .and_then(|m| serde_json::from_str(m).ok())
            .unwrap_or_else(|| serde_json::json!({}));

        if let Some(obj) = json.as_object_mut() {
            obj.remove(Self::KEY);
            obj.remove("provider_error"); // clean up legacy key
        }

        json.to_string()
    }

    /// Whether this is a provider error variant.
    pub fn is_provider_error(&self) -> bool {
        matches!(self, Self::ProviderError { .. })
    }

    /// The status the task was in before being paused.
    pub fn previous_status(&self) -> &str {
        match self {
            Self::UserInitiated {
                previous_status, ..
            } => previous_status,
            Self::ProviderError {
                previous_status, ..
            } => previous_status,
        }
    }
}

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
    /// Provider/API error that is potentially recoverable (rate limits, server errors, etc.).
    /// Task should be paused rather than failed, and auto-resumed when conditions improve.
    ProviderError {
        category: ProviderErrorCategory,
        message: String,
        /// ISO 8601 timestamp when the provider limit resets
        retry_after: Option<String>,
    },
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
            Self::ProviderError {
                category, message, ..
            } => write!(f, "Provider error ({}): {}", category, message),
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
    /// ProviderError is retryable after the retry_after period.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Timeout { .. }
                | Self::ParseStall { .. }
                | Self::SessionNotFound { .. }
                | Self::AgentExit { .. }
                | Self::ProviderError { .. }
        )
    }

    /// Whether this error requires clearing the stored Claude session ID.
    ///
    /// SessionNotFound means the session is stale and must be cleared.
    /// Timeout/ParseStall may indicate a stuck session that should be reset.
    /// ProviderError does NOT require session clear — session is still valid.
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
    /// `Paused` for recoverable provider errors (rate limits, server errors, etc.).
    pub fn suggested_task_status(&self) -> Option<InternalStatus> {
        match self {
            Self::Cancelled => Some(InternalStatus::Cancelled),
            Self::ProviderError { .. } => Some(InternalStatus::Paused),
            Self::Timeout { .. }
            | Self::ParseStall { .. }
            | Self::AgentExit { .. }
            | Self::SessionNotFound { .. }
            | Self::ProcessSpawnFailed { .. }
            | Self::NoOutput { .. } => Some(InternalStatus::Failed),
        }
    }

    /// Whether this is a provider/API error that should pause rather than fail.
    pub fn is_provider_error(&self) -> bool {
        matches!(self, Self::ProviderError { .. })
    }

    /// Build ProviderErrorMetadata for storing in task metadata.
    /// Only valid for ProviderError variants.
    pub fn provider_error_metadata(
        &self,
        previous_status: InternalStatus,
    ) -> Option<ProviderErrorMetadata> {
        match self {
            Self::ProviderError {
                category,
                message,
                retry_after,
            } => Some(ProviderErrorMetadata {
                category: category.clone(),
                message: message.clone(),
                retry_after: retry_after.clone(),
                previous_status: previous_status.to_string(),
                paused_at: chrono::Utc::now().to_rfc3339(),
                auto_resumable: true,
                resume_attempts: 0,
            }),
            _ => None,
        }
    }
}

/// Classify an error string from agent stderr/result as a provider error if applicable.
///
/// Detects patterns like:
/// - `429 {"error":{"code":"1308","message":"Usage limit reached..."}}`
/// - `Rate limit exceeded`
/// - `overloaded_error`
/// - `API_TIMEOUT_MS`
/// - HTTP status codes 401, 403, 429, 500, 502, 503, 504
pub fn classify_provider_error(error_text: &str) -> Option<StreamError> {
    let lower = error_text.to_lowercase();

    // 429 rate limit (z.ai style: "429 {"error":{"code":"1308","message":"Usage limit..."}}")
    if lower.contains("429") && (lower.contains("usage limit") || lower.contains("rate limit")) {
        let retry_after = parse_retry_after_from_message(error_text);
        return Some(StreamError::ProviderError {
            category: ProviderErrorCategory::RateLimit,
            message: truncate_error_message(error_text),
            retry_after,
        });
    }

    // Generic rate limit patterns
    if lower.contains("rate limit")
        || lower.contains("rate_limit")
        || lower.contains("too many requests")
    {
        let retry_after = parse_retry_after_from_message(error_text);
        return Some(StreamError::ProviderError {
            category: ProviderErrorCategory::RateLimit,
            message: truncate_error_message(error_text),
            retry_after,
        });
    }

    // Claude overloaded
    if lower.contains("overloaded_error") || lower.contains("overloaded") {
        return Some(StreamError::ProviderError {
            category: ProviderErrorCategory::Overloaded,
            message: truncate_error_message(error_text),
            retry_after: None,
        });
    }

    // Auth errors
    if lower.contains("401") && (lower.contains("unauthorized") || lower.contains("invalid"))
        || lower.contains("403") && lower.contains("forbidden")
        || lower.contains("invalid api key")
        || lower.contains("invalid_api_key")
    {
        return Some(StreamError::ProviderError {
            category: ProviderErrorCategory::AuthError,
            message: truncate_error_message(error_text),
            retry_after: None,
        });
    }

    // Server errors (5xx)
    for code in ["500", "502", "503", "504"] {
        if lower.contains(code)
            && (lower.contains("internal server error")
                || lower.contains("bad gateway")
                || lower.contains("service unavailable")
                || lower.contains("gateway timeout")
                || lower.contains("server error"))
        {
            return Some(StreamError::ProviderError {
                category: ProviderErrorCategory::ServerError,
                message: truncate_error_message(error_text),
                retry_after: None,
            });
        }
    }

    // Network errors
    if lower.contains("connection refused")
        || lower.contains("connection reset")
        || lower.contains("dns resolution failed")
        || lower.contains("network is unreachable")
        || (lower.contains("api_timeout_ms") && lower.contains("try increasing"))
    {
        return Some(StreamError::ProviderError {
            category: ProviderErrorCategory::NetworkError,
            message: truncate_error_message(error_text),
            retry_after: None,
        });
    }

    None
}

/// Parse a retry-after timestamp from error messages.
/// Looks for patterns like "will reset at 2026-02-15 14:15:20"
fn parse_retry_after_from_message(error_text: &str) -> Option<String> {
    // Pattern: "reset at YYYY-MM-DD HH:MM:SS"
    if let Some(idx) = error_text.find("reset at ") {
        let after = &error_text[idx + "reset at ".len()..];
        // Try to grab "YYYY-MM-DD HH:MM:SS" (19 chars)
        if after.len() >= 19 {
            let candidate = &after[..19];
            // Validate it looks like a datetime
            if candidate.chars().nth(4) == Some('-') && candidate.chars().nth(10) == Some(' ') {
                // Convert to RFC3339
                let rfc3339 = format!("{}T{}+00:00", &candidate[..10], &candidate[11..]);
                if chrono::DateTime::parse_from_rfc3339(&rfc3339).is_ok() {
                    return Some(rfc3339);
                }
            }
        }
    }
    None
}

/// Truncate error message to reasonable length for storage.
fn truncate_error_message(msg: &str) -> String {
    if msg.len() > 500 {
        format!("{}...", &msg[..500])
    } else {
        msg.to_string()
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
    if error_message.contains(STALE_SESSION_ERROR) {
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
#[path = "chat_service_errors_tests.rs"]
mod tests;
