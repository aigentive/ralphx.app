// Agents module - agentic AI client abstraction layer
// This module defines the trait and types for agent clients (Claude, Codex, etc.)
// Implementations live in infrastructure/agents/

pub mod agent_profile;
pub mod agentic_client;
pub mod capabilities;
pub mod error;
pub mod harness;
pub mod types;

// Re-export key types
pub use agent_profile::{
    AgentProfile, AutonomyLevel, BehaviorConfig, ClaudeCodeConfig, ExecutionConfig, IoConfig,
    Model, PermissionMode, ProfileRole,
};
pub use agentic_client::AgenticClient;
pub use capabilities::{ClientCapabilities, ModelInfo};
pub use error::{AgentError, AgentResult};
pub use harness::{
    AgentHarnessKind, AgentLane, AgentLaneSettings, LogicalEffort, ProviderSessionRef,
};
pub use types::{
    AgentConfig, AgentHandle, AgentOutput, AgentResponse, AgentRole, ClientType, ResponseChunk,
};

#[cfg(test)]
#[path = "mod_tests.rs"]
mod dependency_tests;
