// Client capabilities
// Information about what features an agentic client supports

use super::types::ClientType;

/// Information about a model available to a client
#[derive(Debug, Clone)]
pub struct ModelInfo {
    /// Model identifier (e.g., "claude-sonnet-4-5-20250929")
    pub id: String,
    /// Human-readable model name (e.g., "Claude Sonnet 4.5")
    pub name: String,
    /// Maximum tokens the model can generate
    pub max_tokens: u32,
}

impl ModelInfo {
    /// Create a new model info
    pub fn new(id: impl Into<String>, name: impl Into<String>, max_tokens: u32) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            max_tokens,
        }
    }
}

/// Capabilities of an agentic client
#[derive(Debug, Clone)]
pub struct ClientCapabilities {
    /// Type of the client
    pub client_type: ClientType,
    /// Whether the client can execute shell commands
    pub supports_shell: bool,
    /// Whether the client can access the filesystem
    pub supports_filesystem: bool,
    /// Whether the client supports streaming responses
    pub supports_streaming: bool,
    /// Whether the client supports MCP (Model Context Protocol)
    pub supports_mcp: bool,
    /// Maximum context tokens supported
    pub max_context_tokens: u32,
    /// Available models
    pub models: Vec<ModelInfo>,
}

impl ClientCapabilities {
    /// Create capabilities for Claude Code client
    pub fn claude_code() -> Self {
        Self {
            client_type: ClientType::ClaudeCode,
            supports_shell: true,
            supports_filesystem: true,
            supports_streaming: true,
            supports_mcp: true,
            max_context_tokens: 200_000,
            models: vec![
                ModelInfo::new(
                    "claude-sonnet-4-5-20250929",
                    "Claude Sonnet 4.5",
                    64_000,
                ),
                ModelInfo::new(
                    "claude-opus-4-5-20251101",
                    "Claude Opus 4.5",
                    32_000,
                ),
                ModelInfo::new(
                    "claude-haiku-4-5-20251001",
                    "Claude Haiku 4.5",
                    32_000,
                ),
            ],
        }
    }

    /// Create capabilities for mock client
    pub fn mock() -> Self {
        Self {
            client_type: ClientType::Mock,
            supports_shell: true,
            supports_filesystem: true,
            supports_streaming: true,
            supports_mcp: false,
            max_context_tokens: 200_000,
            models: vec![ModelInfo::new("mock", "Mock Model", 100_000)],
        }
    }

    /// Check if a specific model is available
    pub fn has_model(&self, model_id: &str) -> bool {
        self.models.iter().any(|m| m.id == model_id)
    }

    /// Get the default model (first in list)
    pub fn default_model(&self) -> Option<&ModelInfo> {
        self.models.first()
    }

    /// Get a model by ID
    pub fn get_model(&self, model_id: &str) -> Option<&ModelInfo> {
        self.models.iter().find(|m| m.id == model_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ModelInfo tests
    #[test]
    fn test_model_info_new() {
        let model = ModelInfo::new("claude-sonnet", "Sonnet", 64_000);
        assert_eq!(model.id, "claude-sonnet");
        assert_eq!(model.name, "Sonnet");
        assert_eq!(model.max_tokens, 64_000);
    }

    #[test]
    fn test_model_info_clone() {
        let model = ModelInfo::new("test", "Test", 1000);
        let cloned = model.clone();
        assert_eq!(model.id, cloned.id);
    }

    #[test]
    fn test_model_info_debug() {
        let model = ModelInfo::new("test", "Test", 1000);
        let debug_str = format!("{:?}", model);
        assert!(debug_str.contains("test"));
    }

    // ClientCapabilities tests
    #[test]
    fn test_claude_code_capabilities() {
        let caps = ClientCapabilities::claude_code();
        assert_eq!(caps.client_type, ClientType::ClaudeCode);
        assert!(caps.supports_shell);
        assert!(caps.supports_filesystem);
        assert!(caps.supports_streaming);
        assert!(caps.supports_mcp);
        assert_eq!(caps.max_context_tokens, 200_000);
        assert_eq!(caps.models.len(), 3);
    }

    #[test]
    fn test_mock_capabilities() {
        let caps = ClientCapabilities::mock();
        assert_eq!(caps.client_type, ClientType::Mock);
        assert!(caps.supports_shell);
        assert!(caps.supports_filesystem);
        assert!(caps.supports_streaming);
        assert!(!caps.supports_mcp);
        assert_eq!(caps.models.len(), 1);
    }

    #[test]
    fn test_has_model_true() {
        let caps = ClientCapabilities::claude_code();
        assert!(caps.has_model("claude-sonnet-4-5-20250929"));
        assert!(caps.has_model("claude-opus-4-5-20251101"));
    }

    #[test]
    fn test_has_model_false() {
        let caps = ClientCapabilities::claude_code();
        assert!(!caps.has_model("nonexistent-model"));
    }

    #[test]
    fn test_default_model() {
        let caps = ClientCapabilities::claude_code();
        let default = caps.default_model().unwrap();
        assert_eq!(default.id, "claude-sonnet-4-5-20250929");
    }

    #[test]
    fn test_get_model_found() {
        let caps = ClientCapabilities::claude_code();
        let model = caps.get_model("claude-opus-4-5-20251101").unwrap();
        assert_eq!(model.name, "Claude Opus 4.5");
    }

    #[test]
    fn test_get_model_not_found() {
        let caps = ClientCapabilities::claude_code();
        assert!(caps.get_model("nonexistent").is_none());
    }

    #[test]
    fn test_capabilities_clone() {
        let caps = ClientCapabilities::claude_code();
        let cloned = caps.clone();
        assert_eq!(caps.client_type, cloned.client_type);
        assert_eq!(caps.models.len(), cloned.models.len());
    }

    #[test]
    fn test_claude_models_have_correct_ids() {
        let caps = ClientCapabilities::claude_code();
        let model_ids: Vec<&str> = caps.models.iter().map(|m| m.id.as_str()).collect();
        assert!(model_ids.contains(&"claude-sonnet-4-5-20250929"));
        assert!(model_ids.contains(&"claude-opus-4-5-20251101"));
        assert!(model_ids.contains(&"claude-haiku-4-5-20251001"));
    }

    #[test]
    fn test_mock_model_id() {
        let caps = ClientCapabilities::mock();
        let model = caps.default_model().unwrap();
        assert_eq!(model.id, "mock");
    }
}
