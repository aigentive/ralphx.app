//! Type definitions for the ideation system
//! Includes enums, error types, and helper functions

use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Status of an ideation session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdeationSessionStatus {
    /// Session is currently being worked on
    Active,
    /// Session has been archived (completed or paused for later)
    Archived,
    /// All proposals from this session have been accepted and applied to Kanban
    Accepted,
}

impl Default for IdeationSessionStatus {
    fn default() -> Self {
        Self::Active
    }
}

impl std::fmt::Display for IdeationSessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdeationSessionStatus::Active => write!(f, "active"),
            IdeationSessionStatus::Archived => write!(f, "archived"),
            IdeationSessionStatus::Accepted => write!(f, "accepted"),
        }
    }
}

/// Error type for parsing IdeationSessionStatus from a string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseIdeationSessionStatusError {
    pub value: String,
}

impl std::fmt::Display for ParseIdeationSessionStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown ideation session status: '{}'", self.value)
    }
}

impl std::error::Error for ParseIdeationSessionStatusError {}

impl FromStr for IdeationSessionStatus {
    type Err = ParseIdeationSessionStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(IdeationSessionStatus::Active),
            "archived" => Ok(IdeationSessionStatus::Archived),
            "accepted" => Ok(IdeationSessionStatus::Accepted),
            _ => Err(ParseIdeationSessionStatusError {
                value: s.to_string(),
            }),
        }
    }
}

/// Suggested priority level for a task proposal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

impl Default for Priority {
    fn default() -> Self {
        Self::Medium
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::Critical => write!(f, "critical"),
            Priority::High => write!(f, "high"),
            Priority::Medium => write!(f, "medium"),
            Priority::Low => write!(f, "low"),
        }
    }
}

/// Error type for parsing Priority from a string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsePriorityError {
    pub value: String,
}

impl std::fmt::Display for ParsePriorityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown priority: '{}'", self.value)
    }
}

impl std::error::Error for ParsePriorityError {}

impl FromStr for Priority {
    type Err = ParsePriorityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "critical" => Ok(Priority::Critical),
            "high" => Ok(Priority::High),
            "medium" => Ok(Priority::Medium),
            "low" => Ok(Priority::Low),
            _ => Err(ParsePriorityError {
                value: s.to_string(),
            }),
        }
    }
}

/// Estimated complexity of a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Complexity {
    Trivial,
    Simple,
    Moderate,
    Complex,
    VeryComplex,
}

impl Default for Complexity {
    fn default() -> Self {
        Self::Moderate
    }
}

impl std::fmt::Display for Complexity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Complexity::Trivial => write!(f, "trivial"),
            Complexity::Simple => write!(f, "simple"),
            Complexity::Moderate => write!(f, "moderate"),
            Complexity::Complex => write!(f, "complex"),
            Complexity::VeryComplex => write!(f, "very_complex"),
        }
    }
}

/// Error type for parsing Complexity from a string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseComplexityError {
    pub value: String,
}

impl std::fmt::Display for ParseComplexityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown complexity: '{}'", self.value)
    }
}

impl std::error::Error for ParseComplexityError {}

impl FromStr for Complexity {
    type Err = ParseComplexityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "trivial" => Ok(Complexity::Trivial),
            "simple" => Ok(Complexity::Simple),
            "moderate" => Ok(Complexity::Moderate),
            "complex" => Ok(Complexity::Complex),
            "very_complex" => Ok(Complexity::VeryComplex),
            _ => Err(ParseComplexityError {
                value: s.to_string(),
            }),
        }
    }
}

/// Status of a task proposal in the ideation workflow
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    /// Proposal is pending review
    Pending,
    /// Proposal has been accepted and will be converted to a task
    Accepted,
    /// Proposal has been rejected
    Rejected,
    /// Proposal has been modified by the user
    Modified,
}

impl Default for ProposalStatus {
    fn default() -> Self {
        Self::Pending
    }
}

impl std::fmt::Display for ProposalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProposalStatus::Pending => write!(f, "pending"),
            ProposalStatus::Accepted => write!(f, "accepted"),
            ProposalStatus::Rejected => write!(f, "rejected"),
            ProposalStatus::Modified => write!(f, "modified"),
        }
    }
}

/// Error type for parsing ProposalStatus from a string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseProposalStatusError {
    pub value: String,
}

impl std::fmt::Display for ParseProposalStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown proposal status: '{}'", self.value)
    }
}

impl std::error::Error for ParseProposalStatusError {}

impl FromStr for ProposalStatus {
    type Err = ParseProposalStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(ProposalStatus::Pending),
            "accepted" => Ok(ProposalStatus::Accepted),
            "rejected" => Ok(ProposalStatus::Rejected),
            "modified" => Ok(ProposalStatus::Modified),
            _ => Err(ParseProposalStatusError {
                value: s.to_string(),
            }),
        }
    }
}

/// Category of a task proposal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalCategory {
    /// Initial project setup
    Setup,
    /// New feature implementation
    Feature,
    /// Bug fix
    Fix,
    /// Code refactoring
    Refactor,
    /// Documentation
    Docs,
    /// Testing
    Test,
    /// Performance optimization
    Performance,
    /// Security-related
    Security,
    /// DevOps/CI/CD
    DevOps,
    /// Research/investigation
    Research,
    /// Design work
    Design,
    /// Chore/maintenance
    Chore,
}

impl Default for ProposalCategory {
    fn default() -> Self {
        Self::Feature
    }
}

impl std::fmt::Display for ProposalCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProposalCategory::Setup => write!(f, "setup"),
            ProposalCategory::Feature => write!(f, "feature"),
            ProposalCategory::Fix => write!(f, "fix"),
            ProposalCategory::Refactor => write!(f, "refactor"),
            ProposalCategory::Docs => write!(f, "docs"),
            ProposalCategory::Test => write!(f, "test"),
            ProposalCategory::Performance => write!(f, "performance"),
            ProposalCategory::Security => write!(f, "security"),
            ProposalCategory::DevOps => write!(f, "devops"),
            ProposalCategory::Research => write!(f, "research"),
            ProposalCategory::Design => write!(f, "design"),
            ProposalCategory::Chore => write!(f, "chore"),
        }
    }
}

/// Error type for parsing ProposalCategory from a string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseProposalCategoryError {
    pub value: String,
}

impl std::fmt::Display for ParseProposalCategoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown proposal category: '{}'", self.value)
    }
}

impl std::error::Error for ParseProposalCategoryError {}

impl FromStr for ProposalCategory {
    type Err = ParseProposalCategoryError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "setup" => Ok(ProposalCategory::Setup),
            "feature" => Ok(ProposalCategory::Feature),
            "fix" => Ok(ProposalCategory::Fix),
            "refactor" => Ok(ProposalCategory::Refactor),
            "docs" => Ok(ProposalCategory::Docs),
            "test" => Ok(ProposalCategory::Test),
            "performance" => Ok(ProposalCategory::Performance),
            "security" => Ok(ProposalCategory::Security),
            "devops" => Ok(ProposalCategory::DevOps),
            "research" => Ok(ProposalCategory::Research),
            "design" => Ok(ProposalCategory::Design),
            "chore" => Ok(ProposalCategory::Chore),
            _ => Err(ParseProposalCategoryError {
                value: s.to_string(),
            }),
        }
    }
}

/// Priority scoring factors used for automated prioritization
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PriorityFactors {
    /// Score from dependency analysis (blocks other tasks)
    #[serde(default)]
    pub dependency: i32,
    /// Score from business value
    #[serde(default)]
    pub business_value: i32,
    /// Score from technical risk
    #[serde(default)]
    pub technical_risk: i32,
    /// Score from user request frequency
    #[serde(default)]
    pub user_demand: i32,
}

/// Helper function to parse datetime strings from SQLite
pub fn parse_datetime_helper(s: String) -> DateTime<Utc> {
    // Try RFC3339 first (our preferred format)
    if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
        return dt.with_timezone(&Utc);
    }
    // Try SQLite's default datetime format (YYYY-MM-DD HH:MM:SS)
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S") {
        return Utc.from_utc_datetime(&dt);
    }
    // Fallback to now if parsing fails
    Utc::now()
}
