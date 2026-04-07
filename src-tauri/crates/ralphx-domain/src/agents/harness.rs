use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::types::ClientType;

/// Provider-neutral harness kind used by RalphX orchestration.
///
/// This is intentionally narrower than `ClientType`: only first-class harnesses
/// that RalphX actively routes user-facing work through should appear here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentHarnessKind {
    Claude,
    Codex,
}

impl fmt::Display for AgentHarnessKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Claude => write!(f, "claude"),
            Self::Codex => write!(f, "codex"),
        }
    }
}

impl FromStr for AgentHarnessKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "claude" => Ok(Self::Claude),
            "codex" => Ok(Self::Codex),
            other => Err(format!(
                "Invalid agent harness '{}'. Valid values: claude, codex",
                other
            )),
        }
    }
}

impl From<AgentHarnessKind> for ClientType {
    fn from(value: AgentHarnessKind) -> Self {
        match value {
            AgentHarnessKind::Claude => ClientType::ClaudeCode,
            AgentHarnessKind::Codex => ClientType::Codex,
        }
    }
}

impl TryFrom<ClientType> for AgentHarnessKind {
    type Error = String;

    fn try_from(value: ClientType) -> Result<Self, Self::Error> {
        match value {
            ClientType::ClaudeCode => Ok(Self::Claude),
            ClientType::Codex => Ok(Self::Codex),
            other => Err(format!(
                "Client type '{}' does not map to a first-class agent harness",
                other
            )),
        }
    }
}

/// Provider-neutral reasoning effort surfaced in RalphX lane settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogicalEffort {
    Low,
    Medium,
    High,
    XHigh,
}

impl fmt::Display for LogicalEffort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
            Self::XHigh => write!(f, "xhigh"),
        }
    }
}

impl FromStr for LogicalEffort {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "xhigh" => Ok(Self::XHigh),
            other => Err(format!(
                "Invalid logical effort '{}'. Valid values: low, medium, high, xhigh",
                other
            )),
        }
    }
}

impl LogicalEffort {
    /// Claude's current "max" bucket is the closest legacy equivalent to `xhigh`.
    pub fn to_legacy_claude_effort(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::XHigh => "max",
        }
    }
}

/// Provider-neutral lane key for harness/model/effort routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentLane {
    IdeationPrimary,
    IdeationVerifier,
    IdeationSubagent,
    IdeationVerifierSubagent,
    ExecutionWorker,
    ExecutionReviewer,
    ExecutionReexecutor,
    ExecutionMerger,
}

impl fmt::Display for AgentLane {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IdeationPrimary => write!(f, "ideation_primary"),
            Self::IdeationVerifier => write!(f, "ideation_verifier"),
            Self::IdeationSubagent => write!(f, "ideation_subagent"),
            Self::IdeationVerifierSubagent => write!(f, "ideation_verifier_subagent"),
            Self::ExecutionWorker => write!(f, "execution_worker"),
            Self::ExecutionReviewer => write!(f, "execution_reviewer"),
            Self::ExecutionReexecutor => write!(f, "execution_reexecutor"),
            Self::ExecutionMerger => write!(f, "execution_merger"),
        }
    }
}

impl FromStr for AgentLane {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ideation_primary" => Ok(Self::IdeationPrimary),
            "ideation_verifier" => Ok(Self::IdeationVerifier),
            "ideation_subagent" => Ok(Self::IdeationSubagent),
            "ideation_verifier_subagent" => Ok(Self::IdeationVerifierSubagent),
            "execution_worker" => Ok(Self::ExecutionWorker),
            "execution_reviewer" => Ok(Self::ExecutionReviewer),
            "execution_reexecutor" => Ok(Self::ExecutionReexecutor),
            "execution_merger" => Ok(Self::ExecutionMerger),
            other => Err(format!("Invalid agent lane '{}'", other)),
        }
    }
}

/// Minimal provider-neutral session handle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSessionRef {
    pub harness: AgentHarnessKind,
    pub provider_session_id: String,
}

/// Stored lane settings shape used by the upcoming multi-harness config layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentLaneSettings {
    pub harness: AgentHarnessKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<LogicalEffort>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_policy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sandbox_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_harness: Option<AgentHarnessKind>,
}

impl AgentLaneSettings {
    pub fn new(harness: AgentHarnessKind) -> Self {
        Self {
            harness,
            model: None,
            effort: None,
            approval_policy: None,
            sandbox_mode: None,
            fallback_harness: None,
        }
    }
}

#[cfg(test)]
#[path = "harness_tests.rs"]
mod tests;
