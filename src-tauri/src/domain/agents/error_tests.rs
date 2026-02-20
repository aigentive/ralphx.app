use super::*;
use std::io::{Error as IoError, ErrorKind};

#[test]
fn test_not_found_error_displays_correctly() {
    let error = AgentError::NotFound("agent-123".to_string());
    assert_eq!(error.to_string(), "Agent not found: agent-123");
}

#[test]
fn test_spawn_failed_error_displays_correctly() {
    let error = AgentError::SpawnFailed("permission denied".to_string());
    assert_eq!(error.to_string(), "Agent spawn failed: permission denied");
}

#[test]
fn test_spawn_not_allowed_error_displays_correctly() {
    let error = AgentError::SpawnNotAllowed("test env".to_string());
    assert_eq!(error.to_string(), "Agent spawn not allowed: test env");
}

#[test]
fn test_communication_failed_error_displays_correctly() {
    let error = AgentError::CommunicationFailed("connection reset".to_string());
    assert_eq!(
        error.to_string(),
        "Agent communication failed: connection reset"
    );
}

#[test]
fn test_timeout_error_displays_correctly() {
    let error = AgentError::Timeout(5000);
    assert_eq!(error.to_string(), "Agent timeout after 5000ms");
}

#[test]
fn test_cli_not_available_error_displays_correctly() {
    let error = AgentError::CliNotAvailable("claude CLI not found in PATH".to_string());
    assert_eq!(
        error.to_string(),
        "CLI not available: claude CLI not found in PATH"
    );
}

#[test]
fn test_io_error_conversion() {
    let io_error = IoError::new(ErrorKind::NotFound, "file not found");
    let agent_error: AgentError = io_error.into();
    assert!(matches!(agent_error, AgentError::Io(_)));
    assert!(agent_error.to_string().contains("file not found"));
}

#[test]
fn test_io_error_from_impl() {
    let io_error = IoError::new(ErrorKind::PermissionDenied, "access denied");
    let result: AgentResult<()> = Err(io_error.into());
    assert!(result.is_err());
}

#[test]
fn test_agent_result_ok() {
    let result: AgentResult<i32> = Ok(42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}

#[test]
fn test_agent_result_err() {
    let result: AgentResult<i32> = Err(AgentError::NotFound("test".to_string()));
    assert!(result.is_err());
}

#[test]
fn test_error_is_debug() {
    let error = AgentError::NotFound("test".to_string());
    let debug_str = format!("{:?}", error);
    assert!(debug_str.contains("NotFound"));
}

#[test]
fn test_all_error_variants_are_error_trait() {
    fn assert_error<E: std::error::Error>(_: &E) {}

    assert_error(&AgentError::NotFound("test".to_string()));
    assert_error(&AgentError::SpawnFailed("test".to_string()));
    assert_error(&AgentError::SpawnNotAllowed("test".to_string()));
    assert_error(&AgentError::CommunicationFailed("test".to_string()));
    assert_error(&AgentError::Timeout(1000));
    assert_error(&AgentError::CliNotAvailable("test".to_string()));
    assert_error(&AgentError::Io(IoError::new(ErrorKind::Other, "test")));
}
