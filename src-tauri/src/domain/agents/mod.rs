// Agents module - agentic AI client abstraction layer
// This module defines the trait and types for agent clients (Claude, Codex, etc.)
// Implementations live in infrastructure/agents/

pub mod agent_profile;
pub mod agentic_client;
pub mod capabilities;
pub mod error;
pub mod types;

// Re-export key types
pub use agent_profile::{
    AgentProfile, AutonomyLevel, BehaviorConfig, ClaudeCodeConfig, ExecutionConfig, IoConfig,
    Model, PermissionMode, ProfileRole,
};
pub use agentic_client::AgenticClient;
pub use capabilities::{ClientCapabilities, ModelInfo};
pub use error::{AgentError, AgentResult};
pub use types::{
    AgentConfig, AgentHandle, AgentOutput, AgentResponse, AgentRole, ClientType, ResponseChunk,
};

#[cfg(test)]
#[allow(dead_code)]
mod dependency_tests {
    use async_trait::async_trait;
    use futures::Stream;
    use std::pin::Pin;

    // Test that async_trait is available for the AgenticClient trait
    #[async_trait]
    trait TestTrait: Send + Sync {
        async fn test_method(&self) -> String;
    }

    struct TestImpl;

    #[async_trait]
    impl TestTrait for TestImpl {
        async fn test_method(&self) -> String {
            "test".to_string()
        }
    }

    #[test]
    fn test_async_trait_available() {
        // Verify async_trait compiles for trait definitions
        let _impl = TestImpl;
    }

    #[test]
    fn test_futures_stream_available() {
        // Verify futures Stream trait is available
        fn _takes_stream<S: Stream>(_s: S) {}
    }

    #[test]
    fn test_which_crate_available() {
        // Verify which crate is available for CLI detection
        // Just verify import compiles, don't actually search
        use which::which;
        let _ = which("nonexistent_binary_12345");
    }

    #[test]
    fn test_lazy_static_available() {
        // Verify lazy_static is available for global process tracking
        lazy_static::lazy_static! {
            static ref TEST_VALUE: i32 = 42;
        }
        assert_eq!(*TEST_VALUE, 42);
    }

    #[test]
    fn test_tokio_process_feature() {
        // Verify tokio process feature is available
        use tokio::process::Command;
        let _ = Command::new("echo");
    }

    #[test]
    fn test_pin_box_stream_return_type() {
        // Verify we can use Pin<Box<dyn Stream>> as return type
        use futures::stream;

        fn stream_fn() -> Pin<Box<dyn Stream<Item = i32> + Send>> {
            Box::pin(stream::iter(vec![1, 2, 3]))
        }

        let _ = stream_fn();
    }
}
