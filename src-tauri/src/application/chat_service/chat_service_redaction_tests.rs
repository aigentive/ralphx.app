use crate::application::chat_service::chat_service_errors::StreamError;
use crate::utils::secret_redactor::redact;

// 1. AgentExit with Anthropic key in stderr — to_string() produces redacted output
#[test]
fn agent_exit_stderr_with_anthropic_key_is_redacted() {
    let fake_secret = "sk-ant-api03-ABCDEFGHIJ1234567890abcdef";
    let se = StreamError::AgentExit {
        exit_code: Some(1),
        stderr: fake_secret.to_string(),
    };
    let error_string = se.to_string();
    // Apply redact() as the sinks do
    let redacted = redact(&error_string);
    assert!(
        !redacted.contains(fake_secret),
        "AgentExit stderr secret must be redacted"
    );
    assert!(
        redacted.contains("***REDACTED***"),
        "Redacted string must contain placeholder"
    );
}

// 2. AgentExit with OpenRouter key — same pattern
#[test]
fn agent_exit_stderr_with_openrouter_key_is_redacted() {
    let fake_secret = "sk-or-v1-ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890";
    let se = StreamError::AgentExit {
        exit_code: None,
        stderr: format!("Error calling provider: {}", fake_secret),
    };
    let error_string = se.to_string();
    let redacted = redact(&error_string);
    assert!(!redacted.contains(fake_secret));
    assert!(redacted.contains("***REDACTED***"));
}

// 3. AgentExit with no secret — string passes through unchanged
#[test]
fn agent_exit_stderr_without_secret_passes_through() {
    let se = StreamError::AgentExit {
        exit_code: Some(1),
        stderr: "Process exited unexpectedly".to_string(),
    };
    let error_string = se.to_string();
    let redacted = redact(&error_string);
    assert!(
        !redacted.contains("***REDACTED***"),
        "Non-secret string should not be redacted"
    );
    assert!(redacted.contains("Process exited unexpectedly"));
}

// 4. AgentExit with empty stderr — empty path uses exit code message
#[test]
fn agent_exit_empty_stderr_uses_exit_code() {
    let se = StreamError::AgentExit {
        exit_code: Some(2),
        stderr: String::new(),
    };
    let error_string = se.to_string();
    let redacted = redact(&error_string);
    assert!(
        redacted.contains("non-zero status") || redacted.contains("code=Some(2)"),
        "Empty stderr should produce exit code message, got: {}",
        redacted
    );
    assert!(
        !redacted.contains("***REDACTED***"),
        "No secrets to redact in empty stderr"
    );
}

// 5. error_string from AgentExit in queue path — redact at construction point
#[test]
fn queue_error_string_with_bearer_token_is_redacted() {
    // Simulate the queue path: error_string = redact(&e.to_string())
    let fake_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9ABCDEFGHIJ1234567890";
    // Use Bearer pattern which is actually redacted
    let fake_bearer = format!("Bearer {}", fake_token);
    let raw = format!("HTTP 401: Authorization failed with token {}", fake_bearer);
    let redacted = redact(&raw);
    // The Bearer prefix pattern should be caught
    assert!(!redacted.contains(&fake_bearer));
}

// 6. ProviderErrorMetadata message with API key is redacted
#[test]
fn provider_error_message_with_api_key_is_redacted() {
    let fake_secret = "sk-ant-api03-XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";
    let message = format!("Rate limit exceeded for key {}", fake_secret);
    let redacted = redact(&message);
    assert!(!redacted.contains(fake_secret));
    assert!(redacted.contains("sk-ant-***REDACTED***"));
}
