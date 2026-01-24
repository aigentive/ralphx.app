// Agent types
// Core types for agent configuration and identification

use std::collections::HashMap;
use std::path::PathBuf;

/// Role of an agent in the RalphX system
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AgentRole {
    /// Worker agent that executes tasks
    Worker,
    /// Reviewer agent that reviews completed work
    Reviewer,
    /// QA prep agent that generates acceptance criteria
    QaPrep,
    /// QA refiner agent that refines QA plan based on implementation
    QaRefiner,
    /// QA tester agent that executes tests
    QaTester,
    /// Supervisor agent that monitors and coordinates
    Supervisor,
    /// Custom agent role with a user-defined name
    Custom(String),
}

impl std::fmt::Display for AgentRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentRole::Worker => write!(f, "worker"),
            AgentRole::Reviewer => write!(f, "reviewer"),
            AgentRole::QaPrep => write!(f, "qa-prep"),
            AgentRole::QaRefiner => write!(f, "qa-refiner"),
            AgentRole::QaTester => write!(f, "qa-tester"),
            AgentRole::Supervisor => write!(f, "supervisor"),
            AgentRole::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// Type of agentic client
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ClientType {
    /// Claude Code CLI client
    ClaudeCode,
    /// OpenAI Codex client (future)
    Codex,
    /// Google Gemini client (future)
    Gemini,
    /// Mock client for testing
    Mock,
    /// Custom client with a user-defined name
    Custom(String),
}

impl std::fmt::Display for ClientType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientType::ClaudeCode => write!(f, "claude-code"),
            ClientType::Codex => write!(f, "codex"),
            ClientType::Gemini => write!(f, "gemini"),
            ClientType::Mock => write!(f, "mock"),
            ClientType::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// Configuration for spawning an agent
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Role of the agent
    pub role: AgentRole,
    /// Initial prompt to send to the agent
    pub prompt: String,
    /// Working directory for the agent
    pub working_directory: PathBuf,
    /// Optional model override (e.g., "claude-sonnet-4-5-20250929")
    pub model: Option<String>,
    /// Optional max tokens for response
    pub max_tokens: Option<u32>,
    /// Optional timeout in seconds
    pub timeout_secs: Option<u64>,
    /// Additional environment variables
    pub env: HashMap<String, String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            role: AgentRole::Worker,
            prompt: String::new(),
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            model: None,
            max_tokens: None,
            timeout_secs: None,
            env: HashMap::new(),
        }
    }
}

impl AgentConfig {
    /// Create a new config for a worker agent
    pub fn worker(prompt: impl Into<String>) -> Self {
        Self {
            role: AgentRole::Worker,
            prompt: prompt.into(),
            ..Default::default()
        }
    }

    /// Create a new config for a reviewer agent
    pub fn reviewer(prompt: impl Into<String>) -> Self {
        Self {
            role: AgentRole::Reviewer,
            prompt: prompt.into(),
            ..Default::default()
        }
    }

    /// Create a new config for a QA prep agent
    pub fn qa_prep(prompt: impl Into<String>) -> Self {
        Self {
            role: AgentRole::QaPrep,
            prompt: prompt.into(),
            ..Default::default()
        }
    }

    /// Set the working directory
    pub fn with_working_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.working_directory = path.into();
        self
    }

    /// Set the model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the timeout
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }

    /// Add an environment variable
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }
}

/// Handle to a spawned agent
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentHandle {
    /// Unique identifier for this agent instance
    pub id: String,
    /// Type of client running this agent
    pub client_type: ClientType,
    /// Role of this agent
    pub role: AgentRole,
}

impl AgentHandle {
    /// Create a new handle with a generated UUID
    pub fn new(client_type: ClientType, role: AgentRole) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            client_type,
            role,
        }
    }

    /// Create a mock handle for testing
    pub fn mock(role: AgentRole) -> Self {
        Self::new(ClientType::Mock, role)
    }

    /// Create a handle with a specific ID (for testing or restoration)
    pub fn with_id(id: impl Into<String>, client_type: ClientType, role: AgentRole) -> Self {
        Self {
            id: id.into(),
            client_type,
            role,
        }
    }
}

/// Output from a completed agent
#[derive(Debug, Clone, Default)]
pub struct AgentOutput {
    /// Whether the agent completed successfully
    pub success: bool,
    /// Content produced by the agent
    pub content: String,
    /// Exit code if applicable
    pub exit_code: Option<i32>,
    /// Duration in milliseconds
    pub duration_ms: Option<u64>,
}

impl AgentOutput {
    /// Create a successful output
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            success: true,
            content: content.into(),
            exit_code: Some(0),
            duration_ms: None,
        }
    }

    /// Create a failed output
    pub fn failed(content: impl Into<String>, exit_code: i32) -> Self {
        Self {
            success: false,
            content: content.into(),
            exit_code: Some(exit_code),
            duration_ms: None,
        }
    }

    /// Set the duration
    pub fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = Some(ms);
        self
    }
}

/// Response from an agent to a prompt
#[derive(Debug, Clone, Default)]
pub struct AgentResponse {
    /// Content of the response
    pub content: String,
    /// Model that generated the response
    pub model: Option<String>,
    /// Number of tokens used
    pub tokens_used: Option<u32>,
}

impl AgentResponse {
    /// Create a new response with content
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            model: None,
            tokens_used: None,
        }
    }

    /// Set the model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set tokens used
    pub fn with_tokens(mut self, tokens: u32) -> Self {
        self.tokens_used = Some(tokens);
        self
    }
}

/// A chunk from a streaming response
#[derive(Debug, Clone)]
pub struct ResponseChunk {
    /// Content of this chunk
    pub content: String,
    /// Whether this is the final chunk
    pub is_final: bool,
}

impl ResponseChunk {
    /// Create a new non-final chunk
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_final: false,
        }
    }

    /// Create a final chunk
    pub fn final_chunk(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_final: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // AgentRole tests
    #[test]
    fn test_agent_role_worker() {
        let role = AgentRole::Worker;
        assert_eq!(role.to_string(), "worker");
    }

    #[test]
    fn test_agent_role_reviewer() {
        let role = AgentRole::Reviewer;
        assert_eq!(role.to_string(), "reviewer");
    }

    #[test]
    fn test_agent_role_qa_prep() {
        let role = AgentRole::QaPrep;
        assert_eq!(role.to_string(), "qa-prep");
    }

    #[test]
    fn test_agent_role_qa_refiner() {
        let role = AgentRole::QaRefiner;
        assert_eq!(role.to_string(), "qa-refiner");
    }

    #[test]
    fn test_agent_role_qa_tester() {
        let role = AgentRole::QaTester;
        assert_eq!(role.to_string(), "qa-tester");
    }

    #[test]
    fn test_agent_role_supervisor() {
        let role = AgentRole::Supervisor;
        assert_eq!(role.to_string(), "supervisor");
    }

    #[test]
    fn test_agent_role_custom() {
        let role = AgentRole::Custom("my-custom-agent".to_string());
        assert_eq!(role.to_string(), "my-custom-agent");
    }

    #[test]
    fn test_agent_role_equality() {
        assert_eq!(AgentRole::Worker, AgentRole::Worker);
        assert_ne!(AgentRole::Worker, AgentRole::Reviewer);
    }

    #[test]
    fn test_agent_role_clone() {
        let role = AgentRole::Custom("test".to_string());
        let cloned = role.clone();
        assert_eq!(role, cloned);
    }

    #[test]
    fn test_agent_role_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(AgentRole::Worker);
        set.insert(AgentRole::Reviewer);
        assert!(set.contains(&AgentRole::Worker));
        assert!(!set.contains(&AgentRole::Supervisor));
    }

    // ClientType tests
    #[test]
    fn test_client_type_claude_code() {
        let client = ClientType::ClaudeCode;
        assert_eq!(client.to_string(), "claude-code");
    }

    #[test]
    fn test_client_type_codex() {
        let client = ClientType::Codex;
        assert_eq!(client.to_string(), "codex");
    }

    #[test]
    fn test_client_type_gemini() {
        let client = ClientType::Gemini;
        assert_eq!(client.to_string(), "gemini");
    }

    #[test]
    fn test_client_type_mock() {
        let client = ClientType::Mock;
        assert_eq!(client.to_string(), "mock");
    }

    #[test]
    fn test_client_type_custom() {
        let client = ClientType::Custom("my-custom-client".to_string());
        assert_eq!(client.to_string(), "my-custom-client");
    }

    #[test]
    fn test_client_type_equality() {
        assert_eq!(ClientType::Mock, ClientType::Mock);
        assert_ne!(ClientType::Mock, ClientType::ClaudeCode);
    }

    // AgentConfig tests
    #[test]
    fn test_agent_config_default() {
        let config = AgentConfig::default();
        assert_eq!(config.role, AgentRole::Worker);
        assert!(config.prompt.is_empty());
        assert!(config.model.is_none());
        assert!(config.max_tokens.is_none());
        assert!(config.timeout_secs.is_none());
        assert!(config.env.is_empty());
    }

    #[test]
    fn test_agent_config_worker() {
        let config = AgentConfig::worker("Do some work");
        assert_eq!(config.role, AgentRole::Worker);
        assert_eq!(config.prompt, "Do some work");
    }

    #[test]
    fn test_agent_config_reviewer() {
        let config = AgentConfig::reviewer("Review this code");
        assert_eq!(config.role, AgentRole::Reviewer);
        assert_eq!(config.prompt, "Review this code");
    }

    #[test]
    fn test_agent_config_qa_prep() {
        let config = AgentConfig::qa_prep("Prepare QA criteria");
        assert_eq!(config.role, AgentRole::QaPrep);
        assert_eq!(config.prompt, "Prepare QA criteria");
    }

    #[test]
    fn test_agent_config_with_working_dir() {
        let config = AgentConfig::default().with_working_dir("/tmp/work");
        assert_eq!(config.working_directory, PathBuf::from("/tmp/work"));
    }

    #[test]
    fn test_agent_config_with_model() {
        let config = AgentConfig::default().with_model("claude-sonnet-4-5");
        assert_eq!(config.model, Some("claude-sonnet-4-5".to_string()));
    }

    #[test]
    fn test_agent_config_with_timeout() {
        let config = AgentConfig::default().with_timeout(300);
        assert_eq!(config.timeout_secs, Some(300));
    }

    #[test]
    fn test_agent_config_with_env() {
        let config = AgentConfig::default()
            .with_env("API_KEY", "secret")
            .with_env("DEBUG", "true");
        assert_eq!(config.env.get("API_KEY"), Some(&"secret".to_string()));
        assert_eq!(config.env.get("DEBUG"), Some(&"true".to_string()));
    }

    #[test]
    fn test_agent_config_builder_chain() {
        let config = AgentConfig::worker("test")
            .with_working_dir("/tmp")
            .with_model("sonnet")
            .with_timeout(60)
            .with_env("KEY", "value");

        assert_eq!(config.role, AgentRole::Worker);
        assert_eq!(config.prompt, "test");
        assert_eq!(config.working_directory, PathBuf::from("/tmp"));
        assert_eq!(config.model, Some("sonnet".to_string()));
        assert_eq!(config.timeout_secs, Some(60));
        assert_eq!(config.env.get("KEY"), Some(&"value".to_string()));
    }

    // AgentHandle tests
    #[test]
    fn test_agent_handle_new() {
        let handle = AgentHandle::new(ClientType::ClaudeCode, AgentRole::Worker);
        assert_eq!(handle.client_type, ClientType::ClaudeCode);
        assert_eq!(handle.role, AgentRole::Worker);
        assert!(!handle.id.is_empty());
    }

    #[test]
    fn test_agent_handle_mock() {
        let handle = AgentHandle::mock(AgentRole::Reviewer);
        assert_eq!(handle.client_type, ClientType::Mock);
        assert_eq!(handle.role, AgentRole::Reviewer);
    }

    #[test]
    fn test_agent_handle_with_id() {
        let handle = AgentHandle::with_id("custom-id", ClientType::Mock, AgentRole::Worker);
        assert_eq!(handle.id, "custom-id");
    }

    #[test]
    fn test_agent_handle_unique_ids() {
        let h1 = AgentHandle::new(ClientType::Mock, AgentRole::Worker);
        let h2 = AgentHandle::new(ClientType::Mock, AgentRole::Worker);
        assert_ne!(h1.id, h2.id);
    }

    #[test]
    fn test_agent_handle_equality() {
        let h1 = AgentHandle::with_id("same-id", ClientType::Mock, AgentRole::Worker);
        let h2 = AgentHandle::with_id("same-id", ClientType::Mock, AgentRole::Worker);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_agent_handle_hash() {
        use std::collections::HashSet;
        let h1 = AgentHandle::with_id("id-1", ClientType::Mock, AgentRole::Worker);
        let h2 = AgentHandle::with_id("id-2", ClientType::Mock, AgentRole::Worker);
        let mut set = HashSet::new();
        set.insert(h1.clone());
        assert!(set.contains(&h1));
        assert!(!set.contains(&h2));
    }

    // AgentOutput tests
    #[test]
    fn test_agent_output_default() {
        let output = AgentOutput::default();
        assert!(!output.success);
        assert!(output.content.is_empty());
        assert!(output.exit_code.is_none());
        assert!(output.duration_ms.is_none());
    }

    #[test]
    fn test_agent_output_success() {
        let output = AgentOutput::success("Task completed");
        assert!(output.success);
        assert_eq!(output.content, "Task completed");
        assert_eq!(output.exit_code, Some(0));
    }

    #[test]
    fn test_agent_output_failed() {
        let output = AgentOutput::failed("Error occurred", 1);
        assert!(!output.success);
        assert_eq!(output.content, "Error occurred");
        assert_eq!(output.exit_code, Some(1));
    }

    #[test]
    fn test_agent_output_with_duration() {
        let output = AgentOutput::success("test").with_duration(5000);
        assert_eq!(output.duration_ms, Some(5000));
    }

    // AgentResponse tests
    #[test]
    fn test_agent_response_default() {
        let response = AgentResponse::default();
        assert!(response.content.is_empty());
        assert!(response.model.is_none());
        assert!(response.tokens_used.is_none());
    }

    #[test]
    fn test_agent_response_new() {
        let response = AgentResponse::new("Hello, world!");
        assert_eq!(response.content, "Hello, world!");
    }

    #[test]
    fn test_agent_response_with_model() {
        let response = AgentResponse::new("test").with_model("claude-sonnet");
        assert_eq!(response.model, Some("claude-sonnet".to_string()));
    }

    #[test]
    fn test_agent_response_with_tokens() {
        let response = AgentResponse::new("test").with_tokens(150);
        assert_eq!(response.tokens_used, Some(150));
    }

    #[test]
    fn test_agent_response_builder_chain() {
        let response = AgentResponse::new("test")
            .with_model("opus")
            .with_tokens(200);
        assert_eq!(response.content, "test");
        assert_eq!(response.model, Some("opus".to_string()));
        assert_eq!(response.tokens_used, Some(200));
    }

    // ResponseChunk tests
    #[test]
    fn test_response_chunk_new() {
        let chunk = ResponseChunk::new("partial");
        assert_eq!(chunk.content, "partial");
        assert!(!chunk.is_final);
    }

    #[test]
    fn test_response_chunk_final() {
        let chunk = ResponseChunk::final_chunk("done");
        assert_eq!(chunk.content, "done");
        assert!(chunk.is_final);
    }
}
