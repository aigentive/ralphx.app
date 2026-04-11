//! Type definitions for the ideation system
//! Includes enums, error types, and helper functions

use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Origin of an ideation session — distinguishes internally created sessions from externally created ones
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionOrigin {
    /// Session created by an internal RalphX user or agent (default)
    Internal,
    /// Session created by an external agent via the External MCP API
    External,
}

impl Default for SessionOrigin {
    fn default() -> Self {
        Self::Internal
    }
}

impl std::fmt::Display for SessionOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionOrigin::Internal => write!(f, "internal"),
            SessionOrigin::External => write!(f, "external"),
        }
    }
}

impl FromStr for SessionOrigin {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "internal" => Ok(SessionOrigin::Internal),
            "external" => Ok(SessionOrigin::External),
            _ => Err(format!("unknown session origin: '{s}'")),
        }
    }
}

/// Purpose of an ideation session — distinguishes general ideation from ralphx-plan-verifier child sessions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionPurpose {
    /// Standard ideation session (default)
    General,
    /// Child session spawned by the ralphx-plan-verifier agent
    Verification,
}

impl Default for SessionPurpose {
    fn default() -> Self {
        Self::General
    }
}

impl std::fmt::Display for SessionPurpose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionPurpose::General => write!(f, "general"),
            SessionPurpose::Verification => write!(f, "verification"),
        }
    }
}

impl FromStr for SessionPurpose {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "general" => Ok(SessionPurpose::General),
            "verification" => Ok(SessionPurpose::Verification),
            _ => Err(format!("unknown session purpose: '{s}'")),
        }
    }
}

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

// ---------------------------------------------------------------------------
// Verification types
// ---------------------------------------------------------------------------

/// Verification status of an ideation session's plan
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    /// Plan has not been verified yet
    Unverified,
    /// Verification loop is currently running
    Reviewing,
    /// Plan passed all verification rounds (0 critical gaps)
    Verified,
    /// Critic found gaps; plan needs revision
    NeedsRevision,
    /// User explicitly skipped verification
    Skipped,
    /// Plan was imported from another project and is pre-verified
    ImportedVerified,
}

impl Default for VerificationStatus {
    fn default() -> Self {
        Self::Unverified
    }
}

impl std::fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerificationStatus::Unverified => write!(f, "unverified"),
            VerificationStatus::Reviewing => write!(f, "reviewing"),
            VerificationStatus::Verified => write!(f, "verified"),
            VerificationStatus::NeedsRevision => write!(f, "needs_revision"),
            VerificationStatus::Skipped => write!(f, "skipped"),
            VerificationStatus::ImportedVerified => write!(f, "imported_verified"),
        }
    }
}

/// Error type for parsing VerificationStatus from a string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseVerificationStatusError {
    pub value: String,
}

impl std::fmt::Display for ParseVerificationStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown verification status: '{}'", self.value)
    }
}

impl std::error::Error for ParseVerificationStatusError {}

impl FromStr for VerificationStatus {
    type Err = ParseVerificationStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "unverified" => Ok(VerificationStatus::Unverified),
            "reviewing" => Ok(VerificationStatus::Reviewing),
            "verified" => Ok(VerificationStatus::Verified),
            "needs_revision" => Ok(VerificationStatus::NeedsRevision),
            "skipped" => Ok(VerificationStatus::Skipped),
            "imported_verified" => Ok(VerificationStatus::ImportedVerified),
            _ => Err(ParseVerificationStatusError {
                value: s.to_string(),
            }),
        }
    }
}

/// A single gap identified by the critic during a verification round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationGap {
    /// Severity: "critical" | "high" | "medium" | "low"
    pub severity: String,
    /// Category for display grouping (e.g., "security", "architecture")
    pub category: String,
    /// Human-readable description of the gap
    pub description: String,
    /// Why this gap matters for the plan's success
    #[serde(default)]
    pub why_it_matters: Option<String>,
    /// Which critic layer identified this gap: "layer1" | "layer2" (NOT included in fingerprint)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Summary of a single verification round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationRound {
    /// Normalized gap fingerprints (one per gap) from the 4-layer pipeline
    #[serde(default)]
    pub fingerprints: Vec<String>,
    /// Aggregate gap score: critical*10 + high*3 + medium*1
    #[serde(default)]
    pub gap_score: u32,
}

/// Persisted metadata for the verification loop stored as JSON in the DB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationMetadata {
    /// Schema version — always 1
    #[serde(default = "verification_metadata_schema_version")]
    pub v: u32,
    /// Current round number (1-based; 0 = not started)
    #[serde(default)]
    pub current_round: u32,
    /// Maximum allowed rounds before hard-cap exit
    #[serde(default)]
    pub max_rounds: u32,
    /// Per-round summaries (most recent appended last)
    #[serde(default)]
    pub rounds: Vec<VerificationRound>,
    /// Gaps from the most recent critic round
    #[serde(default)]
    pub current_gaps: Vec<VerificationGap>,
    /// Why verification converged (set on terminal status)
    #[serde(default)]
    pub convergence_reason: Option<String>,
    /// Round index with the lowest gap_score (for best-version tracking)
    #[serde(default)]
    pub best_round_index: Option<u32>,
    /// Parse failure count in the sliding window (last 5 rounds)
    #[serde(default)]
    pub parse_failures: Vec<u32>,
}

fn verification_metadata_schema_version() -> u32 {
    1
}

impl Default for VerificationMetadata {
    fn default() -> Self {
        Self {
            v: verification_metadata_schema_version(),
            current_round: 0,
            max_rounds: 0,
            rounds: Vec::new(),
            current_gaps: Vec::new(),
            convergence_reason: None,
            best_round_index: None,
            parse_failures: Vec::new(),
        }
    }
}

/// Typed errors for verification state machine violations (D17)
#[derive(Debug, thiserror::Error)]
pub enum VerificationError {
    #[error("Plan must be verified before accepting")]
    NotVerified,
    #[error("Plan verification is in progress (round {round}/{max_rounds})")]
    InProgress { round: u32, max_rounds: u32 },
    #[error("Plan has {count} unresolved gaps")]
    HasUnresolvedGaps { count: u32 },
    #[error("Verification was skipped — cannot update from critic")]
    SkippedCannotUpdate,
    #[error("Invalid verification transition: {from} → {to}")]
    InvalidTransition { from: String, to: String },
    #[error("Round {round} exceeds max_rounds ({max})")]
    RoundExceedsMax { round: u32, max: u32 },
    #[error("Verification agent crashed during round {round}")]
    AgentCrashed { round: u32 },

    /// Gate for proposal mutations (create/update/delete).
    /// Distinct from `NotVerified` which gates acceptance.
    #[error("Cannot create proposals: plan verification has not been run. Either run verification (update_plan_verification with status 'reviewing') or skip it (update_plan_verification with status 'skipped', convergence_reason 'user_skipped').")]
    ProposalNotVerified,

    /// Gate for proposal mutations when verification is in progress.
    /// Distinct from `InProgress` which gates acceptance.
    #[error("Cannot {operation} proposals: plan verification is in progress (round {round}/{max_rounds}). Complete the current verification round before modifying proposals.")]
    ProposalReviewInProgress {
        operation: String,
        round: u32,
        max_rounds: u32,
    },

    /// Gate for proposal mutations when plan has unresolved gaps.
    /// Distinct from `HasUnresolvedGaps` which gates acceptance.
    #[error("Cannot {operation} proposals: plan verification found {gap_count} unresolved gap(s). Update the plan to address gaps (update_plan_artifact), then re-run verification.")]
    ProposalHasUnresolvedGaps { operation: String, gap_count: usize },

    /// Gate for proposal creation when verification was skipped.
    /// Skipping verification blocks NEW proposal creation for all sessions (internal and external).
    /// Existing proposals can still be updated/deleted; only Create is blocked.
    #[error("Cannot create proposals: plan verification was skipped. Re-run verification (update_plan_verification with status 'reviewing') to create new proposals.")]
    ProposalSkippedNotAllowed,

    /// Gate for skip operations on external-origin sessions.
    /// External sessions cannot skip plan verification — they must run it to completion.
    #[error("External sessions cannot skip plan verification. Run verification to completion (update_plan_verification with status 'reviewing').")]
    ExternalCannotSkip,
}

// ---------------------------------------------------------------------------

/// Status of user acceptance for the finalize confirmation gate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceStatus {
    /// Proposals have been validated but not yet applied, awaiting user confirmation
    Pending,
    /// User has accepted the proposals, apply_proposals_core will be called
    Accepted,
    /// User has rejected the proposals, acceptance_status will be reset to null
    Rejected,
}

impl Default for AcceptanceStatus {
    fn default() -> Self {
        Self::Pending
    }
}

impl std::fmt::Display for AcceptanceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AcceptanceStatus::Pending => write!(f, "pending"),
            AcceptanceStatus::Accepted => write!(f, "accepted"),
            AcceptanceStatus::Rejected => write!(f, "rejected"),
        }
    }
}

impl FromStr for AcceptanceStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(AcceptanceStatus::Pending),
            "accepted" => Ok(AcceptanceStatus::Accepted),
            "rejected" => Ok(AcceptanceStatus::Rejected),
            _ => Err(format!("unknown acceptance status: '{s}'")),
        }
    }
}

// ---------------------------------------------------------------------------

/// Status of user confirmation for the plan verification gate.
/// Tracks whether the user has acknowledged and confirmed the verified plan
/// before proceeding to proposal creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationConfirmationStatus {
    /// Plan has been verified but user has not yet confirmed
    Pending,
    /// User has accepted the verified plan and confirmed proceeding
    Accepted,
    /// User has rejected the verified plan
    Rejected,
}

impl Default for VerificationConfirmationStatus {
    fn default() -> Self {
        Self::Pending
    }
}

impl std::fmt::Display for VerificationConfirmationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerificationConfirmationStatus::Pending => write!(f, "pending"),
            VerificationConfirmationStatus::Accepted => write!(f, "accepted"),
            VerificationConfirmationStatus::Rejected => write!(f, "rejected"),
        }
    }
}

impl FromStr for VerificationConfirmationStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(VerificationConfirmationStatus::Pending),
            "accepted" => Ok(VerificationConfirmationStatus::Accepted),
            "rejected" => Ok(VerificationConfirmationStatus::Rejected),
            _ => Err(format!("unknown verification confirmation status: '{s}'")),
        }
    }
}

// ---------------------------------------------------------------------------

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
