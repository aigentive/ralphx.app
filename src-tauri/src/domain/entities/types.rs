// Newtype pattern for type-safe IDs
// Prevents mixing up TaskId and ProjectId at compile time

use serde::{Deserialize, Serialize};

/// A unique identifier for a Task
/// Uses newtype pattern to prevent accidentally using a ProjectId where TaskId is expected
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub String);

/// A unique identifier for a Project
/// Uses newtype pattern to prevent accidentally using a TaskId where ProjectId is expected
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectId(pub String);

/// A unique identifier for a TaskQA record
/// Uses newtype pattern to prevent accidentally using TaskId where TaskQAId is expected
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskQAId(pub String);

/// A unique identifier for an IdeationSession
/// Uses newtype pattern to prevent accidentally using other IDs where IdeationSessionId is expected
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IdeationSessionId(pub String);

/// A unique identifier for a TaskProposal
/// Uses newtype pattern to prevent accidentally using other IDs where TaskProposalId is expected
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskProposalId(pub String);

/// A unique identifier for a ChatMessage
/// Uses newtype pattern to prevent accidentally using other IDs where ChatMessageId is expected
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChatMessageId(pub String);

/// A unique identifier for a TaskStep
/// Uses newtype pattern to prevent accidentally using other IDs where TaskStepId is expected
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskStepId(pub String);

/// A unique identifier for a ReviewIssue
/// Uses newtype pattern to prevent accidentally using other IDs where ReviewIssueId is expected
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReviewIssueId(pub String);

/// A unique identifier for a SessionLink
/// Uses newtype pattern to prevent accidentally using other IDs where SessionLinkId is expected
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionLinkId(pub String);

/// A unique identifier for an ExecutionPlan
/// Uses newtype pattern to prevent accidentally using other IDs where ExecutionPlanId is expected
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExecutionPlanId(pub String);

impl TaskId {
    /// Creates a new TaskId with a random UUID v4
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Creates a TaskId from an existing string
    /// Useful for database deserialization
    pub fn from_string(s: String) -> Self {
        Self(s)
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ProjectId {
    /// Creates a new ProjectId with a random UUID v4
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Creates a ProjectId from an existing string
    /// Useful for database deserialization
    pub fn from_string(s: String) -> Self {
        Self(s)
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ProjectId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ProjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TaskQAId {
    /// Creates a new TaskQAId with a random UUID v4
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Creates a TaskQAId from an existing string
    /// Useful for database deserialization
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for TaskQAId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskQAId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl IdeationSessionId {
    /// Creates a new IdeationSessionId with a random UUID v4
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Creates an IdeationSessionId from an existing string
    /// Useful for database deserialization
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for IdeationSessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for IdeationSessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TaskProposalId {
    /// Creates a new TaskProposalId with a random UUID v4
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Creates a TaskProposalId from an existing string
    /// Useful for database deserialization
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for TaskProposalId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskProposalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ChatMessageId {
    /// Creates a new ChatMessageId with a random UUID v4
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Creates a ChatMessageId from an existing string
    /// Useful for database deserialization
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ChatMessageId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ChatMessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TaskStepId {
    /// Creates a new TaskStepId with a random UUID v4
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Creates a TaskStepId from an existing string
    /// Useful for database deserialization
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for TaskStepId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskStepId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ReviewIssueId {
    /// Creates a new ReviewIssueId with a random UUID v4
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Creates a ReviewIssueId from an existing string
    /// Useful for database deserialization
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ReviewIssueId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ReviewIssueId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl SessionLinkId {
    /// Creates a new SessionLinkId with a random UUID v4
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Creates a SessionLinkId from an existing string
    /// Useful for database deserialization
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for SessionLinkId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionLinkId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ExecutionPlanId {
    /// Creates a new ExecutionPlanId with a random UUID v4
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Creates an ExecutionPlanId from an existing string
    /// Useful for database deserialization
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ExecutionPlanId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ExecutionPlanId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;
