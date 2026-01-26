// Ideation system entities - IdeationSession and related types
// These represent brainstorming sessions that produce task proposals

use chrono::{DateTime, TimeZone, Utc};
use rusqlite::Row;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use super::{ChatConversationId, IdeationSessionId, ProjectId, TaskId, TaskProposalId};

/// Status of an ideation session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdeationSessionStatus {
    /// Session is currently being worked on
    Active,
    /// Session has been archived (completed or paused for later)
    Archived,
    /// All proposals from this session have been applied to Kanban
    Converted,
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
            IdeationSessionStatus::Converted => write!(f, "converted"),
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
            "converted" => Ok(IdeationSessionStatus::Converted),
            _ => Err(ParseIdeationSessionStatusError {
                value: s.to_string(),
            }),
        }
    }
}

/// An ideation session - a brainstorming conversation that produces task proposals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeationSession {
    /// Unique identifier for this session
    pub id: IdeationSessionId,
    /// Project this session belongs to
    pub project_id: ProjectId,
    /// Human-readable title (auto-generated or user-defined)
    pub title: Option<String>,
    /// Current status of the session
    pub status: IdeationSessionStatus,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// When the session was last updated
    pub updated_at: DateTime<Utc>,
    /// When the session was archived (if applicable)
    pub archived_at: Option<DateTime<Utc>>,
    /// When all proposals were converted to tasks (if applicable)
    pub converted_at: Option<DateTime<Utc>>,
}

/// Builder for creating IdeationSession instances
#[derive(Debug, Default)]
pub struct IdeationSessionBuilder {
    id: Option<IdeationSessionId>,
    project_id: Option<ProjectId>,
    title: Option<String>,
    status: Option<IdeationSessionStatus>,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
    archived_at: Option<DateTime<Utc>>,
    converted_at: Option<DateTime<Utc>>,
}

impl IdeationSessionBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the session ID
    pub fn id(mut self, id: IdeationSessionId) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the project ID
    pub fn project_id(mut self, project_id: ProjectId) -> Self {
        self.project_id = Some(project_id);
        self
    }

    /// Set the title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the status
    pub fn status(mut self, status: IdeationSessionStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Set the created_at timestamp
    pub fn created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = Some(created_at);
        self
    }

    /// Set the updated_at timestamp
    pub fn updated_at(mut self, updated_at: DateTime<Utc>) -> Self {
        self.updated_at = Some(updated_at);
        self
    }

    /// Set the archived_at timestamp
    pub fn archived_at(mut self, archived_at: DateTime<Utc>) -> Self {
        self.archived_at = Some(archived_at);
        self
    }

    /// Set the converted_at timestamp
    pub fn converted_at(mut self, converted_at: DateTime<Utc>) -> Self {
        self.converted_at = Some(converted_at);
        self
    }

    /// Build the IdeationSession
    /// Panics if project_id is not set
    pub fn build(self) -> IdeationSession {
        let now = Utc::now();
        IdeationSession {
            id: self.id.unwrap_or_else(IdeationSessionId::new),
            project_id: self.project_id.expect("project_id is required"),
            title: self.title,
            status: self.status.unwrap_or_default(),
            created_at: self.created_at.unwrap_or(now),
            updated_at: self.updated_at.unwrap_or(now),
            archived_at: self.archived_at,
            converted_at: self.converted_at,
        }
    }
}

impl IdeationSession {
    /// Creates a new active session for a project
    pub fn new(project_id: ProjectId) -> Self {
        IdeationSessionBuilder::new()
            .project_id(project_id)
            .build()
    }

    /// Creates a new active session with a title
    pub fn new_with_title(project_id: ProjectId, title: impl Into<String>) -> Self {
        IdeationSessionBuilder::new()
            .project_id(project_id)
            .title(title)
            .build()
    }

    /// Creates a builder for more complex session creation
    pub fn builder() -> IdeationSessionBuilder {
        IdeationSessionBuilder::new()
    }

    /// Returns true if the session is active
    pub fn is_active(&self) -> bool {
        self.status == IdeationSessionStatus::Active
    }

    /// Returns true if the session has been archived
    pub fn is_archived(&self) -> bool {
        self.status == IdeationSessionStatus::Archived
    }

    /// Returns true if all proposals have been converted
    pub fn is_converted(&self) -> bool {
        self.status == IdeationSessionStatus::Converted
    }

    /// Archives the session
    pub fn archive(&mut self) {
        let now = Utc::now();
        self.status = IdeationSessionStatus::Archived;
        self.archived_at = Some(now);
        self.updated_at = now;
    }

    /// Marks the session as converted (all proposals applied)
    pub fn mark_converted(&mut self) {
        let now = Utc::now();
        self.status = IdeationSessionStatus::Converted;
        self.converted_at = Some(now);
        self.updated_at = now;
    }

    /// Updates the updated_at timestamp to now
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    /// Deserialize an IdeationSession from a SQLite row
    /// Expects columns: id, project_id, title, status, created_at, updated_at, archived_at, converted_at
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: IdeationSessionId::from_string(row.get::<_, String>("id")?),
            project_id: ProjectId::from_string(row.get::<_, String>("project_id")?),
            title: row.get("title")?,
            status: row
                .get::<_, String>("status")?
                .parse()
                .unwrap_or(IdeationSessionStatus::Active),
            created_at: Self::parse_datetime(row.get("created_at")?),
            updated_at: Self::parse_datetime(row.get("updated_at")?),
            archived_at: row
                .get::<_, Option<String>>("archived_at")?
                .map(Self::parse_datetime),
            converted_at: row
                .get::<_, Option<String>>("converted_at")?
                .map(Self::parse_datetime),
        })
    }

    /// Parse a datetime string from SQLite into a DateTime<Utc>
    /// Handles both RFC3339 format and SQLite's CURRENT_TIMESTAMP format
    fn parse_datetime(s: String) -> DateTime<Utc> {
        parse_datetime_helper(s)
    }
}

// ============================================================================
// TaskProposal and related types
// ============================================================================

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
pub enum TaskCategory {
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

impl Default for TaskCategory {
    fn default() -> Self {
        Self::Feature
    }
}

impl std::fmt::Display for TaskCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskCategory::Setup => write!(f, "setup"),
            TaskCategory::Feature => write!(f, "feature"),
            TaskCategory::Fix => write!(f, "fix"),
            TaskCategory::Refactor => write!(f, "refactor"),
            TaskCategory::Docs => write!(f, "docs"),
            TaskCategory::Test => write!(f, "test"),
            TaskCategory::Performance => write!(f, "performance"),
            TaskCategory::Security => write!(f, "security"),
            TaskCategory::DevOps => write!(f, "devops"),
            TaskCategory::Research => write!(f, "research"),
            TaskCategory::Design => write!(f, "design"),
            TaskCategory::Chore => write!(f, "chore"),
        }
    }
}

/// Error type for parsing TaskCategory from a string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseTaskCategoryError {
    pub value: String,
}

impl std::fmt::Display for ParseTaskCategoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown task category: '{}'", self.value)
    }
}

impl std::error::Error for ParseTaskCategoryError {}

impl FromStr for TaskCategory {
    type Err = ParseTaskCategoryError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "setup" => Ok(TaskCategory::Setup),
            "feature" => Ok(TaskCategory::Feature),
            "fix" => Ok(TaskCategory::Fix),
            "refactor" => Ok(TaskCategory::Refactor),
            "docs" => Ok(TaskCategory::Docs),
            "test" => Ok(TaskCategory::Test),
            "performance" => Ok(TaskCategory::Performance),
            "security" => Ok(TaskCategory::Security),
            "devops" => Ok(TaskCategory::DevOps),
            "research" => Ok(TaskCategory::Research),
            "design" => Ok(TaskCategory::Design),
            "chore" => Ok(TaskCategory::Chore),
            _ => Err(ParseTaskCategoryError {
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

/// A task proposal generated during an ideation session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProposal {
    /// Unique identifier for this proposal
    pub id: TaskProposalId,
    /// Session this proposal belongs to
    pub session_id: IdeationSessionId,
    /// Short title for the task
    pub title: String,
    /// Detailed description of what needs to be done
    pub description: Option<String>,
    /// Task category
    pub category: TaskCategory,
    /// Implementation steps (JSON array of strings)
    pub steps: Option<String>,
    /// Acceptance criteria (JSON array of strings)
    pub acceptance_criteria: Option<String>,
    /// AI-suggested priority level
    pub suggested_priority: Priority,
    /// Numeric priority score (0-100, higher = more important)
    pub priority_score: i32,
    /// Explanation for why this priority was suggested
    pub priority_reason: Option<String>,
    /// Factors contributing to the priority score
    pub priority_factors: Option<PriorityFactors>,
    /// Estimated complexity
    pub estimated_complexity: Complexity,
    /// User-overridden priority (if different from suggested)
    pub user_priority: Option<Priority>,
    /// Whether the user has modified this proposal
    pub user_modified: bool,
    /// Current status in the workflow
    pub status: ProposalStatus,
    /// Whether this proposal is selected for conversion
    pub selected: bool,
    /// ID of the created task (if converted)
    pub created_task_id: Option<TaskId>,
    /// Sort order within the session
    pub sort_order: i32,
    /// When the proposal was created
    pub created_at: DateTime<Utc>,
    /// When the proposal was last updated
    pub updated_at: DateTime<Utc>,
}

impl TaskProposal {
    /// Creates a new task proposal with required fields
    pub fn new(
        session_id: IdeationSessionId,
        title: impl Into<String>,
        category: TaskCategory,
        suggested_priority: Priority,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: TaskProposalId::new(),
            session_id,
            title: title.into(),
            description: None,
            category,
            steps: None,
            acceptance_criteria: None,
            suggested_priority,
            priority_score: 50,
            priority_reason: None,
            priority_factors: None,
            estimated_complexity: Complexity::default(),
            user_priority: None,
            user_modified: false,
            status: ProposalStatus::default(),
            selected: true,
            created_task_id: None,
            sort_order: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Returns the effective priority (user override or suggested)
    pub fn effective_priority(&self) -> Priority {
        self.user_priority.unwrap_or(self.suggested_priority)
    }

    /// Returns true if the proposal is pending
    pub fn is_pending(&self) -> bool {
        self.status == ProposalStatus::Pending
    }

    /// Returns true if the proposal has been accepted
    pub fn is_accepted(&self) -> bool {
        self.status == ProposalStatus::Accepted
    }

    /// Returns true if the proposal has been converted to a task
    pub fn is_converted(&self) -> bool {
        self.created_task_id.is_some()
    }

    /// Accepts the proposal
    pub fn accept(&mut self) {
        self.status = ProposalStatus::Accepted;
        self.updated_at = Utc::now();
    }

    /// Rejects the proposal
    pub fn reject(&mut self) {
        self.status = ProposalStatus::Rejected;
        self.selected = false;
        self.updated_at = Utc::now();
    }

    /// Sets the user priority override
    pub fn set_user_priority(&mut self, priority: Priority) {
        self.user_priority = Some(priority);
        self.user_modified = true;
        self.status = ProposalStatus::Modified;
        self.updated_at = Utc::now();
    }

    /// Links this proposal to a created task
    pub fn link_to_task(&mut self, task_id: TaskId) {
        self.created_task_id = Some(task_id);
        self.updated_at = Utc::now();
    }

    /// Toggles selection state
    pub fn toggle_selection(&mut self) {
        self.selected = !self.selected;
        self.updated_at = Utc::now();
    }

    /// Updates the updated_at timestamp to now
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    /// Deserialize a TaskProposal from a SQLite row
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let priority_factors_json: Option<String> = row.get("priority_factors")?;
        let priority_factors = priority_factors_json
            .and_then(|json| serde_json::from_str(&json).ok());

        Ok(Self {
            id: TaskProposalId::from_string(row.get::<_, String>("id")?),
            session_id: IdeationSessionId::from_string(row.get::<_, String>("session_id")?),
            title: row.get("title")?,
            description: row.get("description")?,
            category: row
                .get::<_, String>("category")?
                .parse()
                .unwrap_or(TaskCategory::Feature),
            steps: row.get("steps")?,
            acceptance_criteria: row.get("acceptance_criteria")?,
            suggested_priority: row
                .get::<_, String>("suggested_priority")?
                .parse()
                .unwrap_or(Priority::Medium),
            priority_score: row.get("priority_score")?,
            priority_reason: row.get("priority_reason")?,
            priority_factors,
            estimated_complexity: row
                .get::<_, String>("estimated_complexity")?
                .parse()
                .unwrap_or(Complexity::Moderate),
            user_priority: row
                .get::<_, Option<String>>("user_priority")?
                .and_then(|s| s.parse().ok()),
            user_modified: row.get::<_, i32>("user_modified")? != 0,
            status: row
                .get::<_, String>("status")?
                .parse()
                .unwrap_or(ProposalStatus::Pending),
            selected: row.get::<_, i32>("selected")? != 0,
            created_task_id: row
                .get::<_, Option<String>>("created_task_id")?
                .map(TaskId::from_string),
            sort_order: row.get("sort_order")?,
            created_at: parse_datetime_helper(row.get("created_at")?),
            updated_at: parse_datetime_helper(row.get("updated_at")?),
        })
    }
}

/// Helper function to parse datetime strings from SQLite
fn parse_datetime_helper(s: String) -> DateTime<Utc> {
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

// ============================================================================
// PriorityAssessment and detailed factor types
// ============================================================================

/// Factor for dependency analysis - tasks that unblock others get higher priority
/// Max score: 30 points
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DependencyFactor {
    /// Score from 0-30 based on how many tasks this blocks
    pub score: i32,
    /// Number of tasks that depend on this one (blocked by this task)
    pub blocks_count: i32,
    /// Human-readable explanation (e.g., "Blocks 3 other tasks")
    pub reason: String,
}

impl DependencyFactor {
    /// Create a new dependency factor
    pub fn new(score: i32, blocks_count: i32, reason: impl Into<String>) -> Self {
        Self {
            score: score.clamp(0, 30),
            blocks_count,
            reason: reason.into(),
        }
    }

    /// Maximum possible score for this factor
    pub const MAX_SCORE: i32 = 30;

    /// Calculate score based on blocks count
    /// 0 blocks = 0, 1 = 10, 2 = 18, 3 = 24, 4+ = 30
    pub fn calculate(blocks_count: i32) -> Self {
        let score = match blocks_count {
            0 => 0,
            1 => 10,
            2 => 18,
            3 => 24,
            _ => 30,
        };
        let reason = if blocks_count == 0 {
            "Does not block other tasks".to_string()
        } else if blocks_count == 1 {
            "Blocks 1 other task".to_string()
        } else {
            format!("Blocks {} other tasks", blocks_count)
        };
        Self::new(score, blocks_count, reason)
    }
}

/// Factor for critical path analysis - tasks on the longest path get higher priority
/// Max score: 25 points
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CriticalPathFactor {
    /// Score from 0-25 based on critical path position
    pub score: i32,
    /// Whether this task is on the critical path
    pub is_on_critical_path: bool,
    /// Length of the critical path this task is on
    pub path_length: i32,
    /// Human-readable explanation
    pub reason: String,
}

impl CriticalPathFactor {
    /// Create a new critical path factor
    pub fn new(
        score: i32,
        is_on_critical_path: bool,
        path_length: i32,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            score: score.clamp(0, 25),
            is_on_critical_path,
            path_length,
            reason: reason.into(),
        }
    }

    /// Maximum possible score for this factor
    pub const MAX_SCORE: i32 = 25;

    /// Calculate score based on critical path analysis
    pub fn calculate(is_on_critical_path: bool, path_length: i32) -> Self {
        if !is_on_critical_path {
            return Self::new(0, false, 0, "Not on critical path");
        }
        // Score based on path length: longer paths = higher priority
        let score = match path_length {
            1 => 10,
            2 => 15,
            3 => 20,
            _ => 25,
        };
        let reason = format!("On critical path of length {}", path_length);
        Self::new(score, true, path_length, reason)
    }
}

/// Factor for business value analysis - keyword-based importance detection
/// Max score: 20 points
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BusinessValueFactor {
    /// Score from 0-20 based on detected keywords
    pub score: i32,
    /// Keywords detected that indicate importance (e.g., ["MVP", "core", "essential"])
    pub keywords: Vec<String>,
    /// Human-readable explanation
    pub reason: String,
}

impl BusinessValueFactor {
    /// Create a new business value factor
    pub fn new(score: i32, keywords: Vec<String>, reason: impl Into<String>) -> Self {
        Self {
            score: score.clamp(0, 20),
            keywords,
            reason: reason.into(),
        }
    }

    /// Maximum possible score for this factor
    pub const MAX_SCORE: i32 = 20;

    /// Keywords that indicate critical business value
    pub const CRITICAL_KEYWORDS: &'static [&'static str] = &[
        "critical",
        "blocker",
        "blocking",
        "urgent",
        "asap",
        "emergency",
        "must have",
        "must-have",
    ];

    /// Keywords that indicate high business value
    pub const HIGH_KEYWORDS: &'static [&'static str] = &[
        "important",
        "priority",
        "essential",
        "core",
        "mvp",
        "key",
        "crucial",
    ];

    /// Keywords that indicate low business value
    pub const LOW_KEYWORDS: &'static [&'static str] = &[
        "nice to have",
        "nice-to-have",
        "optional",
        "future",
        "later",
        "eventually",
        "if time",
    ];

    /// Calculate score based on keywords found in text
    pub fn calculate(text: &str) -> Self {
        let text_lower = text.to_lowercase();
        let mut detected = Vec::new();

        // Check for critical keywords (high score)
        for &kw in Self::CRITICAL_KEYWORDS {
            if text_lower.contains(kw) {
                detected.push(kw.to_string());
            }
        }
        if !detected.is_empty() {
            return Self::new(
                20,
                detected,
                "Contains critical business value keywords".to_string(),
            );
        }

        // Check for high keywords (medium-high score)
        for &kw in Self::HIGH_KEYWORDS {
            if text_lower.contains(kw) {
                detected.push(kw.to_string());
            }
        }
        if !detected.is_empty() {
            return Self::new(
                15,
                detected,
                "Contains high business value keywords".to_string(),
            );
        }

        // Check for low keywords (low score)
        for &kw in Self::LOW_KEYWORDS {
            if text_lower.contains(kw) {
                detected.push(kw.to_string());
            }
        }
        if !detected.is_empty() {
            return Self::new(
                5,
                detected,
                "Contains low priority keywords".to_string(),
            );
        }

        Self::new(10, vec![], "No business value keywords detected".to_string())
    }
}

/// Factor for complexity analysis - simpler tasks score higher (quick wins first)
/// Max score: 15 points
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ComplexityFactor {
    /// Score from 0-15 (simpler = higher score)
    pub score: i32,
    /// The complexity level
    pub complexity: Complexity,
    /// Human-readable explanation
    pub reason: String,
}

impl ComplexityFactor {
    /// Create a new complexity factor
    pub fn new(score: i32, complexity: Complexity, reason: impl Into<String>) -> Self {
        Self {
            score: score.clamp(0, 15),
            complexity,
            reason: reason.into(),
        }
    }

    /// Maximum possible score for this factor
    pub const MAX_SCORE: i32 = 15;

    /// Calculate score based on complexity (inverse - simpler = higher)
    pub fn calculate(complexity: Complexity) -> Self {
        let (score, reason) = match complexity {
            Complexity::Trivial => (15, "Quick win - trivial task"),
            Complexity::Simple => (12, "Low effort - simple task"),
            Complexity::Moderate => (9, "Moderate complexity"),
            Complexity::Complex => (5, "Complex task - higher effort"),
            Complexity::VeryComplex => (2, "Very complex - significant effort required"),
        };
        Self::new(score, complexity, reason)
    }
}

/// Factor for user hint analysis - explicit urgency signals from user
/// Max score: 10 points
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct UserHintFactor {
    /// Score from 0-10 based on detected hints
    pub score: i32,
    /// Hints detected from user input (e.g., ["urgent", "blocker", "ASAP"])
    pub hints: Vec<String>,
    /// Human-readable explanation
    pub reason: String,
}

impl UserHintFactor {
    /// Create a new user hint factor
    pub fn new(score: i32, hints: Vec<String>, reason: impl Into<String>) -> Self {
        Self {
            score: score.clamp(0, 10),
            hints,
            reason: reason.into(),
        }
    }

    /// Maximum possible score for this factor
    pub const MAX_SCORE: i32 = 10;

    /// Urgency hint keywords
    pub const URGENCY_HINTS: &'static [&'static str] = &[
        "urgent",
        "asap",
        "immediately",
        "now",
        "today",
        "deadline",
        "blocker",
        "blocking",
        "priority",
        "first",
    ];

    /// Calculate score based on hints found in user input
    pub fn calculate(text: &str) -> Self {
        let text_lower = text.to_lowercase();
        let mut detected = Vec::new();

        for &hint in Self::URGENCY_HINTS {
            if text_lower.contains(hint) {
                detected.push(hint.to_string());
            }
        }

        if detected.is_empty() {
            return Self::new(0, vec![], "No urgency hints from user".to_string());
        }

        let score = (detected.len() as i32 * 3).min(10);
        let reason = format!("User indicated urgency: {}", detected.join(", "));
        Self::new(score, detected, reason)
    }
}

/// Container for all priority factors used in priority assessment
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PriorityAssessmentFactors {
    /// Dependency factor (0-30 points)
    pub dependency_factor: DependencyFactor,
    /// Critical path factor (0-25 points)
    pub critical_path_factor: CriticalPathFactor,
    /// Business value factor (0-20 points)
    pub business_value_factor: BusinessValueFactor,
    /// Complexity factor (0-15 points)
    pub complexity_factor: ComplexityFactor,
    /// User hint factor (0-10 points)
    pub user_hint_factor: UserHintFactor,
}

impl PriorityAssessmentFactors {
    /// Maximum possible total score (30 + 25 + 20 + 15 + 10 = 100)
    pub const MAX_TOTAL_SCORE: i32 = 100;

    /// Calculate total score from all factors
    pub fn total_score(&self) -> i32 {
        self.dependency_factor.score
            + self.critical_path_factor.score
            + self.business_value_factor.score
            + self.complexity_factor.score
            + self.user_hint_factor.score
    }
}

/// Complete priority assessment result for a task proposal
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PriorityAssessment {
    /// ID of the proposal this assessment is for
    pub proposal_id: TaskProposalId,
    /// Final suggested priority level
    pub suggested_priority: Priority,
    /// Numeric priority score (0-100)
    pub priority_score: i32,
    /// Human-readable explanation of the priority
    pub priority_reason: String,
    /// Detailed breakdown of all factors
    pub factors: PriorityAssessmentFactors,
}

impl PriorityAssessment {
    /// Create a new priority assessment
    pub fn new(
        proposal_id: TaskProposalId,
        factors: PriorityAssessmentFactors,
    ) -> Self {
        let priority_score = factors.total_score();
        let suggested_priority = Self::score_to_priority(priority_score);
        let priority_reason = Self::generate_reason(&factors, priority_score);

        Self {
            proposal_id,
            suggested_priority,
            priority_score,
            priority_reason,
            factors,
        }
    }

    /// Convert a numeric score (0-100) to a Priority level
    /// 80-100: Critical
    /// 60-79: High
    /// 40-59: Medium
    /// 0-39: Low
    pub fn score_to_priority(score: i32) -> Priority {
        match score {
            80..=100 => Priority::Critical,
            60..=79 => Priority::High,
            40..=59 => Priority::Medium,
            _ => Priority::Low,
        }
    }

    /// Generate a human-readable reason from factors
    fn generate_reason(factors: &PriorityAssessmentFactors, score: i32) -> String {
        let mut reasons = Vec::new();

        if factors.dependency_factor.score > 0 {
            reasons.push(factors.dependency_factor.reason.clone());
        }
        if factors.critical_path_factor.score > 10 {
            reasons.push(factors.critical_path_factor.reason.clone());
        }
        if factors.business_value_factor.score >= 15 {
            reasons.push(factors.business_value_factor.reason.clone());
        }
        if factors.complexity_factor.score >= 12 {
            reasons.push(factors.complexity_factor.reason.clone());
        }
        if factors.user_hint_factor.score > 0 {
            reasons.push(factors.user_hint_factor.reason.clone());
        }

        if reasons.is_empty() {
            format!("Standard priority (score: {})", score)
        } else {
            reasons.join("; ")
        }
    }

    /// Create a default/neutral assessment for a proposal
    pub fn neutral(proposal_id: TaskProposalId) -> Self {
        Self::new(proposal_id, PriorityAssessmentFactors::default())
    }
}

// ============================================================================
// ChatMessage and Related Types
// ============================================================================

use super::ChatMessageId;

/// Role of the message sender in a chat conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    /// Message from the human user
    User,
    /// Message from the Orchestrator AI agent
    Orchestrator,
    /// System message (e.g., session started, context changed)
    System,
    /// Message from the Worker AI agent (task execution output)
    Worker,
}

impl Default for MessageRole {
    fn default() -> Self {
        Self::User
    }
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::User => write!(f, "user"),
            MessageRole::Orchestrator => write!(f, "orchestrator"),
            MessageRole::System => write!(f, "system"),
            MessageRole::Worker => write!(f, "worker"),
        }
    }
}

/// Error type for parsing MessageRole from a string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseMessageRoleError {
    pub value: String,
}

impl std::fmt::Display for ParseMessageRoleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown message role: '{}'", self.value)
    }
}

impl std::error::Error for ParseMessageRoleError {}

impl FromStr for MessageRole {
    type Err = ParseMessageRoleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user" => Ok(MessageRole::User),
            "orchestrator" => Ok(MessageRole::Orchestrator),
            "system" => Ok(MessageRole::System),
            "worker" => Ok(MessageRole::Worker),
            _ => Err(ParseMessageRoleError {
                value: s.to_string(),
            }),
        }
    }
}

/// A chat message in an ideation session or project/task context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Unique identifier for this message
    pub id: ChatMessageId,
    /// Session this message belongs to (for ideation context)
    pub session_id: Option<IdeationSessionId>,
    /// Project this message belongs to (for project context without session)
    pub project_id: Option<ProjectId>,
    /// Task this message is about (for task-specific context)
    pub task_id: Option<TaskId>,
    /// Conversation this message belongs to (for context-aware chat)
    pub conversation_id: Option<ChatConversationId>,
    /// Who sent the message
    pub role: MessageRole,
    /// The message content (supports Markdown)
    pub content: String,
    /// Optional metadata (JSON) for additional context
    pub metadata: Option<String>,
    /// Parent message ID for threading (if applicable)
    pub parent_message_id: Option<ChatMessageId>,
    /// Tool calls made during this message (JSON array)
    /// Stores the tools that Claude called when generating this message
    pub tool_calls: Option<String>,
    /// When the message was created
    pub created_at: DateTime<Utc>,
}

impl ChatMessage {
    /// Create a new user message in an ideation session
    pub fn user_in_session(session_id: IdeationSessionId, content: impl Into<String>) -> Self {
        Self {
            id: ChatMessageId::new(),
            session_id: Some(session_id),
            project_id: None,
            task_id: None,
            conversation_id: None,
            role: MessageRole::User,
            content: content.into(),
            metadata: None,
            parent_message_id: None,
            tool_calls: None,
            created_at: Utc::now(),
        }
    }

    /// Create a new orchestrator message in an ideation session
    pub fn orchestrator_in_session(session_id: IdeationSessionId, content: impl Into<String>) -> Self {
        Self {
            id: ChatMessageId::new(),
            session_id: Some(session_id),
            project_id: None,
            task_id: None,
            conversation_id: None,
            role: MessageRole::Orchestrator,
            content: content.into(),
            metadata: None,
            parent_message_id: None,
            tool_calls: None,
            created_at: Utc::now(),
        }
    }

    /// Create a new system message in an ideation session
    pub fn system_in_session(session_id: IdeationSessionId, content: impl Into<String>) -> Self {
        Self {
            id: ChatMessageId::new(),
            session_id: Some(session_id),
            project_id: None,
            task_id: None,
            conversation_id: None,
            role: MessageRole::System,
            content: content.into(),
            metadata: None,
            parent_message_id: None,
            tool_calls: None,
            created_at: Utc::now(),
        }
    }

    /// Create a new user message in a project context (no session)
    pub fn user_in_project(project_id: ProjectId, content: impl Into<String>) -> Self {
        Self {
            id: ChatMessageId::new(),
            session_id: None,
            project_id: Some(project_id),
            task_id: None,
            conversation_id: None,
            role: MessageRole::User,
            content: content.into(),
            metadata: None,
            parent_message_id: None,
            tool_calls: None,
            created_at: Utc::now(),
        }
    }

    /// Create a new user message about a specific task
    pub fn user_about_task(task_id: TaskId, content: impl Into<String>) -> Self {
        Self {
            id: ChatMessageId::new(),
            session_id: None,
            project_id: None,
            task_id: Some(task_id),
            conversation_id: None,
            role: MessageRole::User,
            content: content.into(),
            metadata: None,
            parent_message_id: None,
            tool_calls: None,
            created_at: Utc::now(),
        }
    }

    /// Set metadata on this message
    pub fn with_metadata(mut self, metadata: impl Into<String>) -> Self {
        self.metadata = Some(metadata.into());
        self
    }

    /// Set parent message for threading
    pub fn with_parent(mut self, parent_id: ChatMessageId) -> Self {
        self.parent_message_id = Some(parent_id);
        self
    }

    /// Check if this is a user message
    pub fn is_user(&self) -> bool {
        self.role == MessageRole::User
    }

    /// Check if this is an orchestrator message
    pub fn is_orchestrator(&self) -> bool {
        self.role == MessageRole::Orchestrator
    }

    /// Check if this is a system message
    pub fn is_system(&self) -> bool {
        self.role == MessageRole::System
    }

    /// Create from a rusqlite Row
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let id: String = row.get("id")?;
        let session_id: Option<String> = row.get("session_id")?;
        let project_id: Option<String> = row.get("project_id")?;
        let task_id: Option<String> = row.get("task_id")?;
        let conversation_id: Option<String> = row.get("conversation_id").ok().flatten();
        let role: String = row.get("role")?;
        let content: String = row.get("content")?;
        let metadata: Option<String> = row.get("metadata")?;
        let parent_message_id: Option<String> = row.get("parent_message_id")?;
        let tool_calls: Option<String> = row.get("tool_calls").ok().flatten();
        let created_at_str: String = row.get("created_at")?;

        Ok(Self {
            id: ChatMessageId::from_string(id),
            session_id: session_id.map(IdeationSessionId::from_string),
            project_id: project_id.map(ProjectId::from_string),
            task_id: task_id.map(TaskId::from_string),
            conversation_id: conversation_id.map(ChatConversationId::from_string),
            role: MessageRole::from_str(&role).unwrap_or(MessageRole::User),
            content,
            metadata,
            parent_message_id: parent_message_id.map(ChatMessageId::from_string),
            tool_calls,
            created_at: parse_datetime_helper(created_at_str),
        })
    }
}

// ============================================================================
// DependencyGraph Types
// ============================================================================

/// A node in the dependency graph representing a single proposal
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyGraphNode {
    /// The proposal ID this node represents
    pub proposal_id: TaskProposalId,
    /// Title of the proposal for display
    pub title: String,
    /// Number of dependencies (proposals this depends on)
    pub in_degree: usize,
    /// Number of dependents (proposals that depend on this)
    pub out_degree: usize,
}

impl DependencyGraphNode {
    /// Create a new dependency graph node
    pub fn new(proposal_id: TaskProposalId, title: impl Into<String>) -> Self {
        Self {
            proposal_id,
            title: title.into(),
            in_degree: 0,
            out_degree: 0,
        }
    }

    /// Set the in-degree (dependency count)
    pub fn with_in_degree(mut self, count: usize) -> Self {
        self.in_degree = count;
        self
    }

    /// Set the out-degree (dependent count)
    pub fn with_out_degree(mut self, count: usize) -> Self {
        self.out_degree = count;
        self
    }

    /// Returns true if this node has no dependencies (is a root)
    pub fn is_root(&self) -> bool {
        self.in_degree == 0
    }

    /// Returns true if this node has no dependents (is a leaf)
    pub fn is_leaf(&self) -> bool {
        self.out_degree == 0
    }

    /// Returns true if this node is a blocker (has dependents)
    pub fn is_blocker(&self) -> bool {
        self.out_degree > 0
    }
}

/// An edge in the dependency graph representing a dependency relationship
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyGraphEdge {
    /// The proposal that has a dependency (depends on "to")
    pub from: TaskProposalId,
    /// The proposal that is depended on (is a dependency of "from")
    pub to: TaskProposalId,
}

impl DependencyGraphEdge {
    /// Create a new dependency edge
    /// "from" depends on "to" (from → to means from needs to complete first)
    pub fn new(from: TaskProposalId, to: TaskProposalId) -> Self {
        Self { from, to }
    }
}

/// Complete dependency graph for proposals in a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    /// All nodes in the graph
    pub nodes: Vec<DependencyGraphNode>,
    /// All edges in the graph
    pub edges: Vec<DependencyGraphEdge>,
    /// The critical path (longest path through the graph)
    pub critical_path: Vec<TaskProposalId>,
    /// Whether the graph contains any cycles
    pub has_cycles: bool,
    /// If cycles exist, the proposals involved in each cycle
    pub cycles: Option<Vec<Vec<TaskProposalId>>>,
}

impl DependencyGraph {
    /// Create a new empty dependency graph
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            critical_path: Vec::new(),
            has_cycles: false,
            cycles: None,
        }
    }

    /// Create a dependency graph with nodes and edges
    pub fn with_nodes_and_edges(
        nodes: Vec<DependencyGraphNode>,
        edges: Vec<DependencyGraphEdge>,
    ) -> Self {
        Self {
            nodes,
            edges,
            critical_path: Vec::new(),
            has_cycles: false,
            cycles: None,
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node: DependencyGraphNode) {
        self.nodes.push(node);
    }

    /// Add an edge to the graph
    pub fn add_edge(&mut self, edge: DependencyGraphEdge) {
        self.edges.push(edge);
    }

    /// Set the critical path
    pub fn set_critical_path(&mut self, path: Vec<TaskProposalId>) {
        self.critical_path = path;
    }

    /// Mark the graph as having cycles and record them
    pub fn set_cycles(&mut self, cycles: Vec<Vec<TaskProposalId>>) {
        self.has_cycles = !cycles.is_empty();
        self.cycles = if cycles.is_empty() {
            None
        } else {
            Some(cycles)
        };
    }

    /// Get the number of nodes in the graph
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the number of edges in the graph
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Check if the graph is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Get a node by proposal ID
    pub fn get_node(&self, proposal_id: &TaskProposalId) -> Option<&DependencyGraphNode> {
        self.nodes.iter().find(|n| n.proposal_id == *proposal_id)
    }

    /// Get all edges where the given proposal is the source (depends on others)
    pub fn get_dependencies(&self, proposal_id: &TaskProposalId) -> Vec<&DependencyGraphEdge> {
        self.edges.iter().filter(|e| e.from == *proposal_id).collect()
    }

    /// Get all edges where the given proposal is the target (is depended on)
    pub fn get_dependents(&self, proposal_id: &TaskProposalId) -> Vec<&DependencyGraphEdge> {
        self.edges.iter().filter(|e| e.to == *proposal_id).collect()
    }

    /// Get all root nodes (nodes with no dependencies)
    pub fn get_roots(&self) -> Vec<&DependencyGraphNode> {
        self.nodes.iter().filter(|n| n.is_root()).collect()
    }

    /// Get all leaf nodes (nodes with no dependents)
    pub fn get_leaves(&self) -> Vec<&DependencyGraphNode> {
        self.nodes.iter().filter(|n| n.is_leaf()).collect()
    }

    /// Check if a proposal is on the critical path
    pub fn is_on_critical_path(&self, proposal_id: &TaskProposalId) -> bool {
        self.critical_path.contains(proposal_id)
    }

    /// Get the length of the critical path
    pub fn critical_path_length(&self) -> usize {
        self.critical_path.len()
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== IdeationSessionStatus Tests =====

    #[test]
    fn status_default_is_active() {
        assert_eq!(IdeationSessionStatus::default(), IdeationSessionStatus::Active);
    }

    #[test]
    fn status_display_active() {
        assert_eq!(format!("{}", IdeationSessionStatus::Active), "active");
    }

    #[test]
    fn status_display_archived() {
        assert_eq!(format!("{}", IdeationSessionStatus::Archived), "archived");
    }

    #[test]
    fn status_display_converted() {
        assert_eq!(format!("{}", IdeationSessionStatus::Converted), "converted");
    }

    #[test]
    fn status_serializes_to_snake_case() {
        let active_json = serde_json::to_string(&IdeationSessionStatus::Active).expect("Should serialize");
        let archived_json = serde_json::to_string(&IdeationSessionStatus::Archived).expect("Should serialize");
        let converted_json = serde_json::to_string(&IdeationSessionStatus::Converted).expect("Should serialize");

        assert_eq!(active_json, "\"active\"");
        assert_eq!(archived_json, "\"archived\"");
        assert_eq!(converted_json, "\"converted\"");
    }

    #[test]
    fn status_deserializes_from_snake_case() {
        let active: IdeationSessionStatus = serde_json::from_str("\"active\"").expect("Should deserialize");
        let archived: IdeationSessionStatus = serde_json::from_str("\"archived\"").expect("Should deserialize");
        let converted: IdeationSessionStatus = serde_json::from_str("\"converted\"").expect("Should deserialize");

        assert_eq!(active, IdeationSessionStatus::Active);
        assert_eq!(archived, IdeationSessionStatus::Archived);
        assert_eq!(converted, IdeationSessionStatus::Converted);
    }

    #[test]
    fn status_from_str_active() {
        let status: IdeationSessionStatus = "active".parse().unwrap();
        assert_eq!(status, IdeationSessionStatus::Active);
    }

    #[test]
    fn status_from_str_archived() {
        let status: IdeationSessionStatus = "archived".parse().unwrap();
        assert_eq!(status, IdeationSessionStatus::Archived);
    }

    #[test]
    fn status_from_str_converted() {
        let status: IdeationSessionStatus = "converted".parse().unwrap();
        assert_eq!(status, IdeationSessionStatus::Converted);
    }

    #[test]
    fn status_from_str_invalid() {
        let result: Result<IdeationSessionStatus, _> = "invalid".parse();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.value, "invalid");
    }

    #[test]
    fn status_parse_error_displays_correctly() {
        let err = ParseIdeationSessionStatusError {
            value: "unknown".to_string(),
        };
        assert_eq!(err.to_string(), "unknown ideation session status: 'unknown'");
    }

    #[test]
    fn status_clone_works() {
        let status = IdeationSessionStatus::Archived;
        let cloned = status;
        assert_eq!(status, cloned);
    }

    #[test]
    fn status_equality_works() {
        assert_eq!(IdeationSessionStatus::Active, IdeationSessionStatus::Active);
        assert_eq!(IdeationSessionStatus::Archived, IdeationSessionStatus::Archived);
        assert_eq!(IdeationSessionStatus::Converted, IdeationSessionStatus::Converted);
        assert_ne!(IdeationSessionStatus::Active, IdeationSessionStatus::Archived);
        assert_ne!(IdeationSessionStatus::Active, IdeationSessionStatus::Converted);
        assert_ne!(IdeationSessionStatus::Archived, IdeationSessionStatus::Converted);
    }

    // ===== IdeationSession Creation Tests =====

    #[test]
    fn session_new_creates_with_defaults() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());

        assert_eq!(session.project_id, project_id);
        assert_eq!(session.status, IdeationSessionStatus::Active);
        assert!(session.title.is_none());
        assert!(session.archived_at.is_none());
        assert!(session.converted_at.is_none());
    }

    #[test]
    fn session_new_generates_unique_id() {
        let project_id = ProjectId::new();
        let session1 = IdeationSession::new(project_id.clone());
        let session2 = IdeationSession::new(project_id);

        assert_ne!(session1.id, session2.id);
    }

    #[test]
    fn session_new_sets_timestamps() {
        let before = Utc::now();
        let session = IdeationSession::new(ProjectId::new());
        let after = Utc::now();

        assert!(session.created_at >= before);
        assert!(session.created_at <= after);
        assert!(session.updated_at >= before);
        assert!(session.updated_at <= after);
        assert_eq!(session.created_at, session.updated_at);
    }

    #[test]
    fn session_new_with_title() {
        let session = IdeationSession::new_with_title(ProjectId::new(), "Auth Feature");

        assert_eq!(session.title, Some("Auth Feature".to_string()));
        assert_eq!(session.status, IdeationSessionStatus::Active);
    }

    // ===== Builder Tests =====

    #[test]
    fn builder_creates_session_with_all_fields() {
        let project_id = ProjectId::new();
        let created = Utc::now();

        let session = IdeationSession::builder()
            .project_id(project_id.clone())
            .title("Custom Session")
            .status(IdeationSessionStatus::Active)
            .created_at(created)
            .updated_at(created)
            .build();

        assert_eq!(session.project_id, project_id);
        assert_eq!(session.title, Some("Custom Session".to_string()));
        assert_eq!(session.status, IdeationSessionStatus::Active);
        assert_eq!(session.created_at, created);
    }

    #[test]
    fn builder_uses_defaults_for_optional_fields() {
        let session = IdeationSession::builder()
            .project_id(ProjectId::new())
            .build();

        assert!(session.title.is_none());
        assert_eq!(session.status, IdeationSessionStatus::Active);
        assert!(session.archived_at.is_none());
        assert!(session.converted_at.is_none());
    }

    #[test]
    fn builder_generates_id_if_not_provided() {
        let session = IdeationSession::builder()
            .project_id(ProjectId::new())
            .build();

        assert!(uuid::Uuid::parse_str(session.id.as_str()).is_ok());
    }

    #[test]
    fn builder_uses_provided_id() {
        let id = IdeationSessionId::from_string("custom-id");
        let session = IdeationSession::builder()
            .id(id.clone())
            .project_id(ProjectId::new())
            .build();

        assert_eq!(session.id, id);
    }

    #[test]
    #[should_panic(expected = "project_id is required")]
    fn builder_panics_without_project_id() {
        IdeationSession::builder().build();
    }

    // ===== Session Method Tests =====

    #[test]
    fn session_is_active_returns_true_for_active() {
        let session = IdeationSession::new(ProjectId::new());
        assert!(session.is_active());
    }

    #[test]
    fn session_is_active_returns_false_for_other_statuses() {
        let mut session = IdeationSession::new(ProjectId::new());
        session.archive();
        assert!(!session.is_active());
    }

    #[test]
    fn session_is_archived_returns_true_for_archived() {
        let mut session = IdeationSession::new(ProjectId::new());
        session.archive();
        assert!(session.is_archived());
    }

    #[test]
    fn session_is_converted_returns_true_for_converted() {
        let mut session = IdeationSession::new(ProjectId::new());
        session.mark_converted();
        assert!(session.is_converted());
    }

    #[test]
    fn session_archive_sets_status_and_timestamp() {
        let mut session = IdeationSession::new(ProjectId::new());
        let before = Utc::now();

        session.archive();

        assert_eq!(session.status, IdeationSessionStatus::Archived);
        assert!(session.archived_at.is_some());
        assert!(session.archived_at.unwrap() >= before);
        assert!(session.updated_at >= before);
    }

    #[test]
    fn session_mark_converted_sets_status_and_timestamp() {
        let mut session = IdeationSession::new(ProjectId::new());
        let before = Utc::now();

        session.mark_converted();

        assert_eq!(session.status, IdeationSessionStatus::Converted);
        assert!(session.converted_at.is_some());
        assert!(session.converted_at.unwrap() >= before);
        assert!(session.updated_at >= before);
    }

    #[test]
    fn session_touch_updates_timestamp() {
        let mut session = IdeationSession::new(ProjectId::new());
        let original_updated = session.updated_at;
        let original_created = session.created_at;

        std::thread::sleep(std::time::Duration::from_millis(10));

        session.touch();

        assert_eq!(session.created_at, original_created);
        assert!(session.updated_at > original_updated);
    }

    // ===== Serialization Tests =====

    #[test]
    fn session_serializes_to_json() {
        let session = IdeationSession::new_with_title(ProjectId::new(), "JSON Test");
        let json = serde_json::to_string(&session).expect("Should serialize");

        assert!(json.contains("\"title\":\"JSON Test\""));
        assert!(json.contains("\"status\":\"active\""));
    }

    #[test]
    fn session_deserializes_from_json() {
        let json = r#"{
            "id": "session-123",
            "project_id": "proj-456",
            "title": "Deserialized",
            "status": "archived",
            "created_at": "2026-01-24T12:00:00Z",
            "updated_at": "2026-01-24T13:00:00Z",
            "archived_at": "2026-01-24T13:00:00Z",
            "converted_at": null
        }"#;

        let session: IdeationSession = serde_json::from_str(json).expect("Should deserialize");

        assert_eq!(session.id.as_str(), "session-123");
        assert_eq!(session.project_id.as_str(), "proj-456");
        assert_eq!(session.title, Some("Deserialized".to_string()));
        assert_eq!(session.status, IdeationSessionStatus::Archived);
        assert!(session.archived_at.is_some());
        assert!(session.converted_at.is_none());
    }

    #[test]
    fn session_deserializes_with_null_optionals() {
        let json = r#"{
            "id": "session-min",
            "project_id": "proj-min",
            "title": null,
            "status": "active",
            "created_at": "2026-01-24T12:00:00Z",
            "updated_at": "2026-01-24T12:00:00Z",
            "archived_at": null,
            "converted_at": null
        }"#;

        let session: IdeationSession = serde_json::from_str(json).expect("Should deserialize");

        assert!(session.title.is_none());
        assert!(session.archived_at.is_none());
        assert!(session.converted_at.is_none());
    }

    #[test]
    fn session_roundtrip_serialization() {
        let mut original = IdeationSession::new_with_title(ProjectId::new(), "Roundtrip");
        original.archive();

        let json = serde_json::to_string(&original).expect("Should serialize");
        let restored: IdeationSession = serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(original.id, restored.id);
        assert_eq!(original.project_id, restored.project_id);
        assert_eq!(original.title, restored.title);
        assert_eq!(original.status, restored.status);
    }

    #[test]
    fn session_clone_works() {
        let original = IdeationSession::new_with_title(ProjectId::new(), "Clone Test");
        let cloned = original.clone();

        assert_eq!(original.id, cloned.id);
        assert_eq!(original.project_id, cloned.project_id);
        assert_eq!(original.title, cloned.title);
        assert_eq!(original.status, cloned.status);
    }

    #[test]
    fn session_clone_is_independent() {
        let original = IdeationSession::new(ProjectId::new());
        let mut cloned = original.clone();

        cloned.archive();

        // Original should be unchanged
        assert_eq!(original.status, IdeationSessionStatus::Active);
        assert_eq!(cloned.status, IdeationSessionStatus::Archived);
    }

    // ===== from_row Integration Tests =====

    use chrono::Timelike;
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            r#"CREATE TABLE ideation_sessions (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                title TEXT,
                status TEXT NOT NULL DEFAULT 'active',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                archived_at TEXT,
                converted_at TEXT
            )"#,
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn session_from_row_active() {
        let conn = setup_test_db();
        conn.execute(
            r#"INSERT INTO ideation_sessions (id, project_id, title, status, created_at, updated_at)
               VALUES ('sess-1', 'proj-1', 'Auth Feature', 'active',
               '2026-01-24T10:00:00Z', '2026-01-24T11:00:00Z')"#,
            [],
        )
        .unwrap();

        let session: IdeationSession = conn
            .query_row("SELECT * FROM ideation_sessions WHERE id = 'sess-1'", [], |row| {
                IdeationSession::from_row(row)
            })
            .unwrap();

        assert_eq!(session.id.as_str(), "sess-1");
        assert_eq!(session.project_id.as_str(), "proj-1");
        assert_eq!(session.title, Some("Auth Feature".to_string()));
        assert_eq!(session.status, IdeationSessionStatus::Active);
        assert!(session.archived_at.is_none());
        assert!(session.converted_at.is_none());
    }

    #[test]
    fn session_from_row_archived() {
        let conn = setup_test_db();
        conn.execute(
            r#"INSERT INTO ideation_sessions (id, project_id, title, status, created_at, updated_at, archived_at)
               VALUES ('sess-2', 'proj-1', NULL, 'archived',
               '2026-01-24T08:00:00Z', '2026-01-24T12:00:00Z', '2026-01-24T12:00:00Z')"#,
            [],
        )
        .unwrap();

        let session: IdeationSession = conn
            .query_row("SELECT * FROM ideation_sessions WHERE id = 'sess-2'", [], |row| {
                IdeationSession::from_row(row)
            })
            .unwrap();

        assert_eq!(session.status, IdeationSessionStatus::Archived);
        assert!(session.title.is_none());
        assert!(session.archived_at.is_some());
    }

    #[test]
    fn session_from_row_converted() {
        let conn = setup_test_db();
        conn.execute(
            r#"INSERT INTO ideation_sessions (id, project_id, title, status, created_at, updated_at, converted_at)
               VALUES ('sess-3', 'proj-1', 'Done Session', 'converted',
               '2026-01-24T08:00:00Z', '2026-01-24T14:00:00Z', '2026-01-24T14:00:00Z')"#,
            [],
        )
        .unwrap();

        let session: IdeationSession = conn
            .query_row("SELECT * FROM ideation_sessions WHERE id = 'sess-3'", [], |row| {
                IdeationSession::from_row(row)
            })
            .unwrap();

        assert_eq!(session.status, IdeationSessionStatus::Converted);
        assert!(session.converted_at.is_some());
    }

    #[test]
    fn session_from_row_unknown_status_defaults_to_active() {
        let conn = setup_test_db();
        conn.execute(
            r#"INSERT INTO ideation_sessions (id, project_id, title, status, created_at, updated_at)
               VALUES ('sess-unk', 'proj-1', NULL, 'unknown_status',
               '2026-01-24T08:00:00Z', '2026-01-24T08:00:00Z')"#,
            [],
        )
        .unwrap();

        let session: IdeationSession = conn
            .query_row("SELECT * FROM ideation_sessions WHERE id = 'sess-unk'", [], |row| {
                IdeationSession::from_row(row)
            })
            .unwrap();

        // Unknown status should default to Active
        assert_eq!(session.status, IdeationSessionStatus::Active);
    }

    #[test]
    fn session_from_row_sqlite_datetime_format() {
        let conn = setup_test_db();
        conn.execute(
            r#"INSERT INTO ideation_sessions (id, project_id, title, status, created_at, updated_at)
               VALUES ('sess-sql', 'proj-1', NULL, 'active',
               '2026-01-24 12:30:00', '2026-01-24 14:45:00')"#,
            [],
        )
        .unwrap();

        let session: IdeationSession = conn
            .query_row("SELECT * FROM ideation_sessions WHERE id = 'sess-sql'", [], |row| {
                IdeationSession::from_row(row)
            })
            .unwrap();

        assert_eq!(session.created_at.hour(), 12);
        assert_eq!(session.created_at.minute(), 30);
        assert_eq!(session.updated_at.hour(), 14);
        assert_eq!(session.updated_at.minute(), 45);
    }

    #[test]
    fn session_from_row_with_all_timestamps() {
        let conn = setup_test_db();
        conn.execute(
            r#"INSERT INTO ideation_sessions (id, project_id, title, status, created_at, updated_at, archived_at, converted_at)
               VALUES ('sess-full', 'proj-1', 'Full', 'converted',
               '2026-01-24T08:00:00Z', '2026-01-24T16:00:00Z',
               '2026-01-24T12:00:00Z', '2026-01-24T16:00:00Z')"#,
            [],
        )
        .unwrap();

        let session: IdeationSession = conn
            .query_row("SELECT * FROM ideation_sessions WHERE id = 'sess-full'", [], |row| {
                IdeationSession::from_row(row)
            })
            .unwrap();

        assert!(session.archived_at.is_some());
        assert!(session.converted_at.is_some());
        assert_eq!(session.archived_at.unwrap().hour(), 12);
        assert_eq!(session.converted_at.unwrap().hour(), 16);
    }

    // ==========================================
    // Priority Enum Tests
    // ==========================================

    #[test]
    fn priority_default_is_medium() {
        assert_eq!(Priority::default(), Priority::Medium);
    }

    #[test]
    fn priority_display() {
        assert_eq!(format!("{}", Priority::Critical), "critical");
        assert_eq!(format!("{}", Priority::High), "high");
        assert_eq!(format!("{}", Priority::Medium), "medium");
        assert_eq!(format!("{}", Priority::Low), "low");
    }

    #[test]
    fn priority_from_str() {
        assert_eq!("critical".parse::<Priority>().unwrap(), Priority::Critical);
        assert_eq!("high".parse::<Priority>().unwrap(), Priority::High);
        assert_eq!("medium".parse::<Priority>().unwrap(), Priority::Medium);
        assert_eq!("low".parse::<Priority>().unwrap(), Priority::Low);
    }

    #[test]
    fn priority_from_str_invalid() {
        let result: Result<Priority, _> = "invalid".parse();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().value, "invalid");
    }

    #[test]
    fn priority_serializes() {
        assert_eq!(serde_json::to_string(&Priority::Critical).unwrap(), "\"critical\"");
        assert_eq!(serde_json::to_string(&Priority::Low).unwrap(), "\"low\"");
    }

    #[test]
    fn priority_deserializes() {
        assert_eq!(serde_json::from_str::<Priority>("\"high\"").unwrap(), Priority::High);
    }

    // ==========================================
    // Complexity Enum Tests
    // ==========================================

    #[test]
    fn complexity_default_is_moderate() {
        assert_eq!(Complexity::default(), Complexity::Moderate);
    }

    #[test]
    fn complexity_display() {
        assert_eq!(format!("{}", Complexity::Trivial), "trivial");
        assert_eq!(format!("{}", Complexity::Simple), "simple");
        assert_eq!(format!("{}", Complexity::Moderate), "moderate");
        assert_eq!(format!("{}", Complexity::Complex), "complex");
        assert_eq!(format!("{}", Complexity::VeryComplex), "very_complex");
    }

    #[test]
    fn complexity_from_str() {
        assert_eq!("trivial".parse::<Complexity>().unwrap(), Complexity::Trivial);
        assert_eq!("simple".parse::<Complexity>().unwrap(), Complexity::Simple);
        assert_eq!("moderate".parse::<Complexity>().unwrap(), Complexity::Moderate);
        assert_eq!("complex".parse::<Complexity>().unwrap(), Complexity::Complex);
        assert_eq!("very_complex".parse::<Complexity>().unwrap(), Complexity::VeryComplex);
    }

    #[test]
    fn complexity_from_str_invalid() {
        let result: Result<Complexity, _> = "unknown".parse();
        assert!(result.is_err());
    }

    #[test]
    fn complexity_serializes() {
        assert_eq!(serde_json::to_string(&Complexity::VeryComplex).unwrap(), "\"very_complex\"");
    }

    // ==========================================
    // ProposalStatus Enum Tests
    // ==========================================

    #[test]
    fn proposal_status_default_is_pending() {
        assert_eq!(ProposalStatus::default(), ProposalStatus::Pending);
    }

    #[test]
    fn proposal_status_display() {
        assert_eq!(format!("{}", ProposalStatus::Pending), "pending");
        assert_eq!(format!("{}", ProposalStatus::Accepted), "accepted");
        assert_eq!(format!("{}", ProposalStatus::Rejected), "rejected");
        assert_eq!(format!("{}", ProposalStatus::Modified), "modified");
    }

    #[test]
    fn proposal_status_from_str() {
        assert_eq!("pending".parse::<ProposalStatus>().unwrap(), ProposalStatus::Pending);
        assert_eq!("accepted".parse::<ProposalStatus>().unwrap(), ProposalStatus::Accepted);
        assert_eq!("rejected".parse::<ProposalStatus>().unwrap(), ProposalStatus::Rejected);
        assert_eq!("modified".parse::<ProposalStatus>().unwrap(), ProposalStatus::Modified);
    }

    #[test]
    fn proposal_status_from_str_invalid() {
        let result: Result<ProposalStatus, _> = "invalid".parse();
        assert!(result.is_err());
    }

    // ==========================================
    // TaskCategory Enum Tests
    // ==========================================

    #[test]
    fn task_category_default_is_feature() {
        assert_eq!(TaskCategory::default(), TaskCategory::Feature);
    }

    #[test]
    fn task_category_display() {
        assert_eq!(format!("{}", TaskCategory::Setup), "setup");
        assert_eq!(format!("{}", TaskCategory::Feature), "feature");
        assert_eq!(format!("{}", TaskCategory::Fix), "fix");
        assert_eq!(format!("{}", TaskCategory::Refactor), "refactor");
        assert_eq!(format!("{}", TaskCategory::Docs), "docs");
        assert_eq!(format!("{}", TaskCategory::Test), "test");
        assert_eq!(format!("{}", TaskCategory::Performance), "performance");
        assert_eq!(format!("{}", TaskCategory::Security), "security");
        assert_eq!(format!("{}", TaskCategory::DevOps), "devops");
        assert_eq!(format!("{}", TaskCategory::Research), "research");
        assert_eq!(format!("{}", TaskCategory::Design), "design");
        assert_eq!(format!("{}", TaskCategory::Chore), "chore");
    }

    #[test]
    fn task_category_from_str() {
        assert_eq!("setup".parse::<TaskCategory>().unwrap(), TaskCategory::Setup);
        assert_eq!("feature".parse::<TaskCategory>().unwrap(), TaskCategory::Feature);
        assert_eq!("fix".parse::<TaskCategory>().unwrap(), TaskCategory::Fix);
        assert_eq!("devops".parse::<TaskCategory>().unwrap(), TaskCategory::DevOps);
    }

    #[test]
    fn task_category_from_str_invalid() {
        let result: Result<TaskCategory, _> = "invalid".parse();
        assert!(result.is_err());
    }

    // ==========================================
    // PriorityFactors Tests
    // ==========================================

    #[test]
    fn priority_factors_default() {
        let factors = PriorityFactors::default();
        assert_eq!(factors.dependency, 0);
        assert_eq!(factors.business_value, 0);
        assert_eq!(factors.technical_risk, 0);
        assert_eq!(factors.user_demand, 0);
    }

    #[test]
    fn priority_factors_serializes() {
        let factors = PriorityFactors {
            dependency: 25,
            business_value: 30,
            technical_risk: 10,
            user_demand: 15,
        };
        let json = serde_json::to_string(&factors).unwrap();
        assert!(json.contains("\"dependency\":25"));
        assert!(json.contains("\"business_value\":30"));
    }

    #[test]
    fn priority_factors_deserializes() {
        let json = r#"{"dependency":10,"business_value":20,"technical_risk":5,"user_demand":15}"#;
        let factors: PriorityFactors = serde_json::from_str(json).unwrap();
        assert_eq!(factors.dependency, 10);
        assert_eq!(factors.user_demand, 15);
    }

    #[test]
    fn priority_factors_deserializes_with_missing_fields() {
        let json = r#"{"dependency":10}"#;
        let factors: PriorityFactors = serde_json::from_str(json).unwrap();
        assert_eq!(factors.dependency, 10);
        assert_eq!(factors.business_value, 0); // default
    }

    // ==========================================
    // TaskProposal Creation Tests
    // ==========================================

    #[test]
    fn proposal_new_creates_with_defaults() {
        let session_id = IdeationSessionId::new();
        let proposal = TaskProposal::new(
            session_id.clone(),
            "Add authentication",
            TaskCategory::Feature,
            Priority::High,
        );

        assert_eq!(proposal.session_id, session_id);
        assert_eq!(proposal.title, "Add authentication");
        assert_eq!(proposal.category, TaskCategory::Feature);
        assert_eq!(proposal.suggested_priority, Priority::High);
        assert_eq!(proposal.priority_score, 50);
        assert_eq!(proposal.estimated_complexity, Complexity::Moderate);
        assert_eq!(proposal.status, ProposalStatus::Pending);
        assert!(proposal.selected);
        assert!(!proposal.user_modified);
        assert!(proposal.description.is_none());
        assert!(proposal.created_task_id.is_none());
    }

    #[test]
    fn proposal_new_generates_unique_id() {
        let session_id = IdeationSessionId::new();
        let p1 = TaskProposal::new(session_id.clone(), "Task 1", TaskCategory::Feature, Priority::High);
        let p2 = TaskProposal::new(session_id, "Task 2", TaskCategory::Feature, Priority::Low);

        assert_ne!(p1.id, p2.id);
    }

    #[test]
    fn proposal_effective_priority_returns_suggested_when_no_override() {
        let proposal = TaskProposal::new(
            IdeationSessionId::new(),
            "Test",
            TaskCategory::Feature,
            Priority::High,
        );

        assert_eq!(proposal.effective_priority(), Priority::High);
    }

    #[test]
    fn proposal_effective_priority_returns_user_override() {
        let mut proposal = TaskProposal::new(
            IdeationSessionId::new(),
            "Test",
            TaskCategory::Feature,
            Priority::High,
        );
        proposal.set_user_priority(Priority::Low);

        assert_eq!(proposal.effective_priority(), Priority::Low);
    }

    // ==========================================
    // TaskProposal Method Tests
    // ==========================================

    #[test]
    fn proposal_is_pending() {
        let proposal = TaskProposal::new(
            IdeationSessionId::new(),
            "Test",
            TaskCategory::Feature,
            Priority::Medium,
        );
        assert!(proposal.is_pending());
        assert!(!proposal.is_accepted());
    }

    #[test]
    fn proposal_accept() {
        let mut proposal = TaskProposal::new(
            IdeationSessionId::new(),
            "Test",
            TaskCategory::Feature,
            Priority::Medium,
        );
        proposal.accept();

        assert!(proposal.is_accepted());
        assert!(!proposal.is_pending());
        assert_eq!(proposal.status, ProposalStatus::Accepted);
    }

    #[test]
    fn proposal_reject() {
        let mut proposal = TaskProposal::new(
            IdeationSessionId::new(),
            "Test",
            TaskCategory::Feature,
            Priority::Medium,
        );
        proposal.reject();

        assert_eq!(proposal.status, ProposalStatus::Rejected);
        assert!(!proposal.selected);
    }

    #[test]
    fn proposal_set_user_priority() {
        let mut proposal = TaskProposal::new(
            IdeationSessionId::new(),
            "Test",
            TaskCategory::Feature,
            Priority::Low,
        );
        proposal.set_user_priority(Priority::Critical);

        assert_eq!(proposal.user_priority, Some(Priority::Critical));
        assert!(proposal.user_modified);
        assert_eq!(proposal.status, ProposalStatus::Modified);
    }

    #[test]
    fn proposal_link_to_task() {
        let mut proposal = TaskProposal::new(
            IdeationSessionId::new(),
            "Test",
            TaskCategory::Feature,
            Priority::Medium,
        );
        let task_id = TaskId::new();
        proposal.link_to_task(task_id.clone());

        assert_eq!(proposal.created_task_id, Some(task_id));
        assert!(proposal.is_converted());
    }

    #[test]
    fn proposal_toggle_selection() {
        let mut proposal = TaskProposal::new(
            IdeationSessionId::new(),
            "Test",
            TaskCategory::Feature,
            Priority::Medium,
        );
        assert!(proposal.selected);

        proposal.toggle_selection();
        assert!(!proposal.selected);

        proposal.toggle_selection();
        assert!(proposal.selected);
    }

    #[test]
    fn proposal_touch_updates_timestamp() {
        let mut proposal = TaskProposal::new(
            IdeationSessionId::new(),
            "Test",
            TaskCategory::Feature,
            Priority::Medium,
        );
        let original = proposal.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        proposal.touch();

        assert!(proposal.updated_at > original);
    }

    // ==========================================
    // TaskProposal Serialization Tests
    // ==========================================

    #[test]
    fn proposal_serializes_to_json() {
        let proposal = TaskProposal::new(
            IdeationSessionId::new(),
            "JSON Test",
            TaskCategory::Fix,
            Priority::Critical,
        );
        let json = serde_json::to_string(&proposal).unwrap();

        assert!(json.contains("\"title\":\"JSON Test\""));
        assert!(json.contains("\"category\":\"fix\""));
        assert!(json.contains("\"suggested_priority\":\"critical\""));
    }

    #[test]
    fn proposal_deserializes_from_json() {
        let json = r#"{
            "id": "prop-123",
            "session_id": "sess-456",
            "title": "Deserialized",
            "description": "A test proposal",
            "category": "refactor",
            "steps": null,
            "acceptance_criteria": null,
            "suggested_priority": "high",
            "priority_score": 75,
            "priority_reason": "Important",
            "priority_factors": null,
            "estimated_complexity": "complex",
            "user_priority": "critical",
            "user_modified": true,
            "status": "modified",
            "selected": true,
            "created_task_id": null,
            "sort_order": 5,
            "created_at": "2026-01-24T12:00:00Z",
            "updated_at": "2026-01-24T13:00:00Z"
        }"#;

        let proposal: TaskProposal = serde_json::from_str(json).unwrap();

        assert_eq!(proposal.id.as_str(), "prop-123");
        assert_eq!(proposal.session_id.as_str(), "sess-456");
        assert_eq!(proposal.title, "Deserialized");
        assert_eq!(proposal.category, TaskCategory::Refactor);
        assert_eq!(proposal.suggested_priority, Priority::High);
        assert_eq!(proposal.priority_score, 75);
        assert_eq!(proposal.estimated_complexity, Complexity::Complex);
        assert_eq!(proposal.user_priority, Some(Priority::Critical));
        assert!(proposal.user_modified);
        assert_eq!(proposal.status, ProposalStatus::Modified);
        assert_eq!(proposal.sort_order, 5);
    }

    #[test]
    fn proposal_roundtrip_serialization() {
        let mut original = TaskProposal::new(
            IdeationSessionId::new(),
            "Roundtrip",
            TaskCategory::Security,
            Priority::High,
        );
        original.set_user_priority(Priority::Critical);

        let json = serde_json::to_string(&original).unwrap();
        let restored: TaskProposal = serde_json::from_str(&json).unwrap();

        assert_eq!(original.id, restored.id);
        assert_eq!(original.title, restored.title);
        assert_eq!(original.category, restored.category);
        assert_eq!(original.user_priority, restored.user_priority);
        assert_eq!(original.status, restored.status);
    }

    // ==========================================
    // TaskProposal from_row Integration Tests
    // ==========================================

    fn setup_proposal_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            r#"CREATE TABLE task_proposals (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                category TEXT NOT NULL,
                steps TEXT,
                acceptance_criteria TEXT,
                suggested_priority TEXT NOT NULL,
                priority_score INTEGER NOT NULL DEFAULT 50,
                priority_reason TEXT,
                priority_factors TEXT,
                estimated_complexity TEXT DEFAULT 'moderate',
                user_priority TEXT,
                user_modified INTEGER DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'pending',
                selected INTEGER DEFAULT 1,
                created_task_id TEXT,
                sort_order INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )"#,
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn proposal_from_row_basic() {
        let conn = setup_proposal_test_db();
        conn.execute(
            r#"INSERT INTO task_proposals (id, session_id, title, category, suggested_priority,
               priority_score, estimated_complexity, status, selected, sort_order, created_at, updated_at)
               VALUES ('prop-1', 'sess-1', 'Test Proposal', 'feature', 'high',
               75, 'complex', 'pending', 1, 0, '2026-01-24T10:00:00Z', '2026-01-24T11:00:00Z')"#,
            [],
        )
        .unwrap();

        let proposal: TaskProposal = conn
            .query_row("SELECT * FROM task_proposals WHERE id = 'prop-1'", [], |row| {
                TaskProposal::from_row(row)
            })
            .unwrap();

        assert_eq!(proposal.id.as_str(), "prop-1");
        assert_eq!(proposal.session_id.as_str(), "sess-1");
        assert_eq!(proposal.title, "Test Proposal");
        assert_eq!(proposal.category, TaskCategory::Feature);
        assert_eq!(proposal.suggested_priority, Priority::High);
        assert_eq!(proposal.priority_score, 75);
        assert_eq!(proposal.estimated_complexity, Complexity::Complex);
        assert!(proposal.selected);
        assert!(!proposal.user_modified);
    }

    #[test]
    fn proposal_from_row_with_user_override() {
        let conn = setup_proposal_test_db();
        conn.execute(
            r#"INSERT INTO task_proposals (id, session_id, title, category, suggested_priority,
               priority_score, user_priority, user_modified, status, selected, sort_order, created_at, updated_at)
               VALUES ('prop-2', 'sess-1', 'Modified', 'fix', 'medium',
               50, 'critical', 1, 'modified', 1, 3, '2026-01-24T10:00:00Z', '2026-01-24T12:00:00Z')"#,
            [],
        )
        .unwrap();

        let proposal: TaskProposal = conn
            .query_row("SELECT * FROM task_proposals WHERE id = 'prop-2'", [], |row| {
                TaskProposal::from_row(row)
            })
            .unwrap();

        assert_eq!(proposal.user_priority, Some(Priority::Critical));
        assert!(proposal.user_modified);
        assert_eq!(proposal.status, ProposalStatus::Modified);
        assert_eq!(proposal.sort_order, 3);
    }

    #[test]
    fn proposal_from_row_with_priority_factors() {
        let conn = setup_proposal_test_db();
        conn.execute(
            r#"INSERT INTO task_proposals (id, session_id, title, category, suggested_priority,
               priority_score, priority_factors, status, selected, sort_order, created_at, updated_at)
               VALUES ('prop-3', 'sess-1', 'With Factors', 'feature', 'high',
               80, '{"dependency":25,"business_value":30,"technical_risk":10,"user_demand":15}',
               'pending', 1, 0, '2026-01-24T10:00:00Z', '2026-01-24T10:00:00Z')"#,
            [],
        )
        .unwrap();

        let proposal: TaskProposal = conn
            .query_row("SELECT * FROM task_proposals WHERE id = 'prop-3'", [], |row| {
                TaskProposal::from_row(row)
            })
            .unwrap();

        assert!(proposal.priority_factors.is_some());
        let factors = proposal.priority_factors.unwrap();
        assert_eq!(factors.dependency, 25);
        assert_eq!(factors.business_value, 30);
    }

    #[test]
    fn proposal_from_row_with_created_task() {
        let conn = setup_proposal_test_db();
        conn.execute(
            r#"INSERT INTO task_proposals (id, session_id, title, category, suggested_priority,
               priority_score, status, selected, created_task_id, sort_order, created_at, updated_at)
               VALUES ('prop-4', 'sess-1', 'Converted', 'feature', 'medium',
               50, 'accepted', 1, 'task-abc', 0, '2026-01-24T10:00:00Z', '2026-01-24T14:00:00Z')"#,
            [],
        )
        .unwrap();

        let proposal: TaskProposal = conn
            .query_row("SELECT * FROM task_proposals WHERE id = 'prop-4'", [], |row| {
                TaskProposal::from_row(row)
            })
            .unwrap();

        assert!(proposal.created_task_id.is_some());
        assert_eq!(proposal.created_task_id.as_ref().unwrap().as_str(), "task-abc");
        assert!(proposal.is_converted());
    }

    #[test]
    fn proposal_from_row_unknown_category_defaults_to_feature() {
        let conn = setup_proposal_test_db();
        conn.execute(
            r#"INSERT INTO task_proposals (id, session_id, title, category, suggested_priority,
               priority_score, status, selected, sort_order, created_at, updated_at)
               VALUES ('prop-5', 'sess-1', 'Unknown Cat', 'invalid_category', 'medium',
               50, 'pending', 1, 0, '2026-01-24T10:00:00Z', '2026-01-24T10:00:00Z')"#,
            [],
        )
        .unwrap();

        let proposal: TaskProposal = conn
            .query_row("SELECT * FROM task_proposals WHERE id = 'prop-5'", [], |row| {
                TaskProposal::from_row(row)
            })
            .unwrap();

        assert_eq!(proposal.category, TaskCategory::Feature);
    }

    // ==========================================
    // DependencyFactor Tests
    // ==========================================

    #[test]
    fn dependency_factor_default() {
        let factor = DependencyFactor::default();
        assert_eq!(factor.score, 0);
        assert_eq!(factor.blocks_count, 0);
        assert_eq!(factor.reason, "");
    }

    #[test]
    fn dependency_factor_new() {
        let factor = DependencyFactor::new(25, 3, "Blocks 3 tasks");
        assert_eq!(factor.score, 25);
        assert_eq!(factor.blocks_count, 3);
        assert_eq!(factor.reason, "Blocks 3 tasks");
    }

    #[test]
    fn dependency_factor_new_clamps_score() {
        let factor = DependencyFactor::new(50, 5, "Too high");
        assert_eq!(factor.score, 30); // Max is 30
    }

    #[test]
    fn dependency_factor_calculate_zero_blocks() {
        let factor = DependencyFactor::calculate(0);
        assert_eq!(factor.score, 0);
        assert_eq!(factor.blocks_count, 0);
        assert_eq!(factor.reason, "Does not block other tasks");
    }

    #[test]
    fn dependency_factor_calculate_one_block() {
        let factor = DependencyFactor::calculate(1);
        assert_eq!(factor.score, 10);
        assert_eq!(factor.blocks_count, 1);
        assert_eq!(factor.reason, "Blocks 1 other task");
    }

    #[test]
    fn dependency_factor_calculate_two_blocks() {
        let factor = DependencyFactor::calculate(2);
        assert_eq!(factor.score, 18);
        assert_eq!(factor.blocks_count, 2);
    }

    #[test]
    fn dependency_factor_calculate_three_blocks() {
        let factor = DependencyFactor::calculate(3);
        assert_eq!(factor.score, 24);
    }

    #[test]
    fn dependency_factor_calculate_many_blocks() {
        let factor = DependencyFactor::calculate(10);
        assert_eq!(factor.score, 30); // Max score
        assert_eq!(factor.blocks_count, 10);
    }

    #[test]
    fn dependency_factor_serializes() {
        let factor = DependencyFactor::new(20, 2, "Test");
        let json = serde_json::to_string(&factor).unwrap();
        assert!(json.contains("\"score\":20"));
        assert!(json.contains("\"blocks_count\":2"));
    }

    #[test]
    fn dependency_factor_deserializes() {
        let json = r#"{"score":15,"blocks_count":1,"reason":"Blocks 1 task"}"#;
        let factor: DependencyFactor = serde_json::from_str(json).unwrap();
        assert_eq!(factor.score, 15);
        assert_eq!(factor.blocks_count, 1);
    }

    #[test]
    fn dependency_factor_max_score_constant() {
        assert_eq!(DependencyFactor::MAX_SCORE, 30);
    }

    // ==========================================
    // CriticalPathFactor Tests
    // ==========================================

    #[test]
    fn critical_path_factor_default() {
        let factor = CriticalPathFactor::default();
        assert_eq!(factor.score, 0);
        assert!(!factor.is_on_critical_path);
        assert_eq!(factor.path_length, 0);
    }

    #[test]
    fn critical_path_factor_new() {
        let factor = CriticalPathFactor::new(20, true, 3, "On critical path");
        assert_eq!(factor.score, 20);
        assert!(factor.is_on_critical_path);
        assert_eq!(factor.path_length, 3);
    }

    #[test]
    fn critical_path_factor_new_clamps_score() {
        let factor = CriticalPathFactor::new(50, true, 5, "Too high");
        assert_eq!(factor.score, 25); // Max is 25
    }

    #[test]
    fn critical_path_factor_calculate_not_on_path() {
        let factor = CriticalPathFactor::calculate(false, 0);
        assert_eq!(factor.score, 0);
        assert!(!factor.is_on_critical_path);
    }

    #[test]
    fn critical_path_factor_calculate_path_length_1() {
        let factor = CriticalPathFactor::calculate(true, 1);
        assert_eq!(factor.score, 10);
        assert!(factor.is_on_critical_path);
    }

    #[test]
    fn critical_path_factor_calculate_path_length_2() {
        let factor = CriticalPathFactor::calculate(true, 2);
        assert_eq!(factor.score, 15);
    }

    #[test]
    fn critical_path_factor_calculate_path_length_3() {
        let factor = CriticalPathFactor::calculate(true, 3);
        assert_eq!(factor.score, 20);
    }

    #[test]
    fn critical_path_factor_calculate_long_path() {
        let factor = CriticalPathFactor::calculate(true, 10);
        assert_eq!(factor.score, 25); // Max score
    }

    #[test]
    fn critical_path_factor_serializes() {
        let factor = CriticalPathFactor::new(15, true, 2, "On path");
        let json = serde_json::to_string(&factor).unwrap();
        assert!(json.contains("\"is_on_critical_path\":true"));
        assert!(json.contains("\"path_length\":2"));
    }

    #[test]
    fn critical_path_factor_max_score_constant() {
        assert_eq!(CriticalPathFactor::MAX_SCORE, 25);
    }

    // ==========================================
    // BusinessValueFactor Tests
    // ==========================================

    #[test]
    fn business_value_factor_default() {
        let factor = BusinessValueFactor::default();
        assert_eq!(factor.score, 0);
        assert!(factor.keywords.is_empty());
    }

    #[test]
    fn business_value_factor_new() {
        let factor = BusinessValueFactor::new(15, vec!["mvp".to_string()], "Contains MVP");
        assert_eq!(factor.score, 15);
        assert_eq!(factor.keywords.len(), 1);
    }

    #[test]
    fn business_value_factor_new_clamps_score() {
        let factor = BusinessValueFactor::new(50, vec![], "Too high");
        assert_eq!(factor.score, 20); // Max is 20
    }

    #[test]
    fn business_value_factor_calculate_critical_keywords() {
        let factor = BusinessValueFactor::calculate("This is URGENT and blocking other work");
        assert_eq!(factor.score, 20);
        assert!(factor.keywords.contains(&"urgent".to_string()));
        assert!(factor.keywords.contains(&"blocking".to_string()));
    }

    #[test]
    fn business_value_factor_calculate_high_keywords() {
        let factor = BusinessValueFactor::calculate("This is essential for the MVP");
        assert_eq!(factor.score, 15);
        assert!(factor.keywords.contains(&"essential".to_string()) || factor.keywords.contains(&"mvp".to_string()));
    }

    #[test]
    fn business_value_factor_calculate_low_keywords() {
        let factor = BusinessValueFactor::calculate("Nice to have feature for later");
        assert_eq!(factor.score, 5);
        assert!(factor.keywords.contains(&"nice to have".to_string()) || factor.keywords.contains(&"later".to_string()));
    }

    #[test]
    fn business_value_factor_calculate_no_keywords() {
        let factor = BusinessValueFactor::calculate("Just a regular task description");
        assert_eq!(factor.score, 10);
        assert!(factor.keywords.is_empty());
    }

    #[test]
    fn business_value_factor_serializes() {
        let factor = BusinessValueFactor::new(15, vec!["important".to_string()], "Has important");
        let json = serde_json::to_string(&factor).unwrap();
        assert!(json.contains("\"keywords\":[\"important\"]"));
    }

    #[test]
    fn business_value_factor_max_score_constant() {
        assert_eq!(BusinessValueFactor::MAX_SCORE, 20);
    }

    #[test]
    fn business_value_factor_critical_keywords_exist() {
        assert!(!BusinessValueFactor::CRITICAL_KEYWORDS.is_empty());
        assert!(BusinessValueFactor::CRITICAL_KEYWORDS.contains(&"urgent"));
        assert!(BusinessValueFactor::CRITICAL_KEYWORDS.contains(&"blocker"));
    }

    #[test]
    fn business_value_factor_high_keywords_exist() {
        assert!(!BusinessValueFactor::HIGH_KEYWORDS.is_empty());
        assert!(BusinessValueFactor::HIGH_KEYWORDS.contains(&"important"));
        assert!(BusinessValueFactor::HIGH_KEYWORDS.contains(&"mvp"));
    }

    #[test]
    fn business_value_factor_low_keywords_exist() {
        assert!(!BusinessValueFactor::LOW_KEYWORDS.is_empty());
        assert!(BusinessValueFactor::LOW_KEYWORDS.contains(&"optional"));
        assert!(BusinessValueFactor::LOW_KEYWORDS.contains(&"future"));
    }

    // ==========================================
    // ComplexityFactor Tests
    // ==========================================

    #[test]
    fn complexity_factor_default() {
        let factor = ComplexityFactor::default();
        assert_eq!(factor.score, 0);
        assert_eq!(factor.complexity, Complexity::Moderate);
    }

    #[test]
    fn complexity_factor_new() {
        let factor = ComplexityFactor::new(12, Complexity::Simple, "Simple task");
        assert_eq!(factor.score, 12);
        assert_eq!(factor.complexity, Complexity::Simple);
    }

    #[test]
    fn complexity_factor_new_clamps_score() {
        let factor = ComplexityFactor::new(50, Complexity::Trivial, "Too high");
        assert_eq!(factor.score, 15); // Max is 15
    }

    #[test]
    fn complexity_factor_calculate_trivial() {
        let factor = ComplexityFactor::calculate(Complexity::Trivial);
        assert_eq!(factor.score, 15);
        assert_eq!(factor.complexity, Complexity::Trivial);
        assert!(factor.reason.contains("trivial"));
    }

    #[test]
    fn complexity_factor_calculate_simple() {
        let factor = ComplexityFactor::calculate(Complexity::Simple);
        assert_eq!(factor.score, 12);
    }

    #[test]
    fn complexity_factor_calculate_moderate() {
        let factor = ComplexityFactor::calculate(Complexity::Moderate);
        assert_eq!(factor.score, 9);
    }

    #[test]
    fn complexity_factor_calculate_complex() {
        let factor = ComplexityFactor::calculate(Complexity::Complex);
        assert_eq!(factor.score, 5);
    }

    #[test]
    fn complexity_factor_calculate_very_complex() {
        let factor = ComplexityFactor::calculate(Complexity::VeryComplex);
        assert_eq!(factor.score, 2);
    }

    #[test]
    fn complexity_factor_serializes() {
        let factor = ComplexityFactor::calculate(Complexity::Simple);
        let json = serde_json::to_string(&factor).unwrap();
        assert!(json.contains("\"complexity\":\"simple\""));
    }

    #[test]
    fn complexity_factor_max_score_constant() {
        assert_eq!(ComplexityFactor::MAX_SCORE, 15);
    }

    // ==========================================
    // UserHintFactor Tests
    // ==========================================

    #[test]
    fn user_hint_factor_default() {
        let factor = UserHintFactor::default();
        assert_eq!(factor.score, 0);
        assert!(factor.hints.is_empty());
    }

    #[test]
    fn user_hint_factor_new() {
        let factor = UserHintFactor::new(8, vec!["urgent".to_string()], "User said urgent");
        assert_eq!(factor.score, 8);
        assert_eq!(factor.hints.len(), 1);
    }

    #[test]
    fn user_hint_factor_new_clamps_score() {
        let factor = UserHintFactor::new(50, vec![], "Too high");
        assert_eq!(factor.score, 10); // Max is 10
    }

    #[test]
    fn user_hint_factor_calculate_no_hints() {
        let factor = UserHintFactor::calculate("Just a regular request");
        assert_eq!(factor.score, 0);
        assert!(factor.hints.is_empty());
    }

    #[test]
    fn user_hint_factor_calculate_one_hint() {
        let factor = UserHintFactor::calculate("I need this done ASAP");
        assert_eq!(factor.score, 3);
        assert!(factor.hints.contains(&"asap".to_string()));
    }

    #[test]
    fn user_hint_factor_calculate_multiple_hints() {
        let factor = UserHintFactor::calculate("This is urgent and blocking, do it first");
        assert!(factor.score >= 6);
        assert!(factor.hints.len() >= 2);
    }

    #[test]
    fn user_hint_factor_calculate_max_score() {
        let factor = UserHintFactor::calculate("urgent asap immediately now today deadline blocker");
        assert_eq!(factor.score, 10); // Capped at max
    }

    #[test]
    fn user_hint_factor_serializes() {
        let factor = UserHintFactor::new(6, vec!["urgent".to_string(), "asap".to_string()], "User hints");
        let json = serde_json::to_string(&factor).unwrap();
        assert!(json.contains("\"hints\":["));
    }

    #[test]
    fn user_hint_factor_max_score_constant() {
        assert_eq!(UserHintFactor::MAX_SCORE, 10);
    }

    #[test]
    fn user_hint_factor_urgency_hints_exist() {
        assert!(!UserHintFactor::URGENCY_HINTS.is_empty());
        assert!(UserHintFactor::URGENCY_HINTS.contains(&"urgent"));
        assert!(UserHintFactor::URGENCY_HINTS.contains(&"asap"));
    }

    // ==========================================
    // PriorityAssessmentFactors Tests
    // ==========================================

    #[test]
    fn priority_assessment_factors_default() {
        let factors = PriorityAssessmentFactors::default();
        assert_eq!(factors.dependency_factor.score, 0);
        assert_eq!(factors.critical_path_factor.score, 0);
        assert_eq!(factors.business_value_factor.score, 0);
        assert_eq!(factors.complexity_factor.score, 0);
        assert_eq!(factors.user_hint_factor.score, 0);
    }

    #[test]
    fn priority_assessment_factors_total_score() {
        let factors = PriorityAssessmentFactors {
            dependency_factor: DependencyFactor::new(20, 2, ""),
            critical_path_factor: CriticalPathFactor::new(15, true, 2, ""),
            business_value_factor: BusinessValueFactor::new(10, vec![], ""),
            complexity_factor: ComplexityFactor::new(8, Complexity::Moderate, ""),
            user_hint_factor: UserHintFactor::new(5, vec![], ""),
        };
        assert_eq!(factors.total_score(), 58);
    }

    #[test]
    fn priority_assessment_factors_max_total() {
        assert_eq!(PriorityAssessmentFactors::MAX_TOTAL_SCORE, 100);
    }

    #[test]
    fn priority_assessment_factors_serializes() {
        let factors = PriorityAssessmentFactors::default();
        let json = serde_json::to_string(&factors).unwrap();
        assert!(json.contains("\"dependency_factor\""));
        assert!(json.contains("\"critical_path_factor\""));
        assert!(json.contains("\"business_value_factor\""));
        assert!(json.contains("\"complexity_factor\""));
        assert!(json.contains("\"user_hint_factor\""));
    }

    // ==========================================
    // PriorityAssessment Tests
    // ==========================================

    #[test]
    fn priority_assessment_new() {
        let proposal_id = TaskProposalId::new();
        let factors = PriorityAssessmentFactors {
            dependency_factor: DependencyFactor::calculate(2),
            critical_path_factor: CriticalPathFactor::calculate(true, 3),
            business_value_factor: BusinessValueFactor::calculate("This is essential"),
            complexity_factor: ComplexityFactor::calculate(Complexity::Simple),
            user_hint_factor: UserHintFactor::calculate("urgent"),
        };

        let assessment = PriorityAssessment::new(proposal_id.clone(), factors);

        assert_eq!(assessment.proposal_id, proposal_id);
        assert!(assessment.priority_score > 0);
        assert!(!assessment.priority_reason.is_empty());
    }

    #[test]
    fn priority_assessment_score_to_priority_critical() {
        assert_eq!(PriorityAssessment::score_to_priority(100), Priority::Critical);
        assert_eq!(PriorityAssessment::score_to_priority(85), Priority::Critical);
        assert_eq!(PriorityAssessment::score_to_priority(80), Priority::Critical);
    }

    #[test]
    fn priority_assessment_score_to_priority_high() {
        assert_eq!(PriorityAssessment::score_to_priority(79), Priority::High);
        assert_eq!(PriorityAssessment::score_to_priority(70), Priority::High);
        assert_eq!(PriorityAssessment::score_to_priority(60), Priority::High);
    }

    #[test]
    fn priority_assessment_score_to_priority_medium() {
        assert_eq!(PriorityAssessment::score_to_priority(59), Priority::Medium);
        assert_eq!(PriorityAssessment::score_to_priority(50), Priority::Medium);
        assert_eq!(PriorityAssessment::score_to_priority(40), Priority::Medium);
    }

    #[test]
    fn priority_assessment_score_to_priority_low() {
        assert_eq!(PriorityAssessment::score_to_priority(39), Priority::Low);
        assert_eq!(PriorityAssessment::score_to_priority(20), Priority::Low);
        assert_eq!(PriorityAssessment::score_to_priority(0), Priority::Low);
    }

    #[test]
    fn priority_assessment_neutral() {
        let proposal_id = TaskProposalId::new();
        let assessment = PriorityAssessment::neutral(proposal_id.clone());

        assert_eq!(assessment.proposal_id, proposal_id);
        assert_eq!(assessment.priority_score, 0);
        assert_eq!(assessment.suggested_priority, Priority::Low);
    }

    #[test]
    fn priority_assessment_serializes() {
        let proposal_id = TaskProposalId::new();
        let assessment = PriorityAssessment::neutral(proposal_id);
        let json = serde_json::to_string(&assessment).unwrap();

        assert!(json.contains("\"proposal_id\""));
        assert!(json.contains("\"suggested_priority\""));
        assert!(json.contains("\"priority_score\""));
        assert!(json.contains("\"priority_reason\""));
        assert!(json.contains("\"factors\""));
    }

    #[test]
    fn priority_assessment_deserializes() {
        let json = r#"{
            "proposal_id": "prop-123",
            "suggested_priority": "high",
            "priority_score": 75,
            "priority_reason": "Important task",
            "factors": {
                "dependency_factor": {"score": 20, "blocks_count": 2, "reason": "Blocks 2 tasks"},
                "critical_path_factor": {"score": 15, "is_on_critical_path": true, "path_length": 2, "reason": "On path"},
                "business_value_factor": {"score": 15, "keywords": ["important"], "reason": "High value"},
                "complexity_factor": {"score": 12, "complexity": "simple", "reason": "Simple"},
                "user_hint_factor": {"score": 6, "hints": ["urgent"], "reason": "User urgent"}
            }
        }"#;

        let assessment: PriorityAssessment = serde_json::from_str(json).unwrap();

        assert_eq!(assessment.proposal_id.as_str(), "prop-123");
        assert_eq!(assessment.suggested_priority, Priority::High);
        assert_eq!(assessment.priority_score, 75);
        assert_eq!(assessment.factors.dependency_factor.score, 20);
        assert_eq!(assessment.factors.critical_path_factor.path_length, 2);
    }

    #[test]
    fn priority_assessment_roundtrip_serialization() {
        let proposal_id = TaskProposalId::new();
        let factors = PriorityAssessmentFactors {
            dependency_factor: DependencyFactor::calculate(3),
            critical_path_factor: CriticalPathFactor::calculate(true, 4),
            business_value_factor: BusinessValueFactor::calculate("critical blocker"),
            complexity_factor: ComplexityFactor::calculate(Complexity::Trivial),
            user_hint_factor: UserHintFactor::calculate("urgent asap"),
        };
        let original = PriorityAssessment::new(proposal_id, factors);

        let json = serde_json::to_string(&original).unwrap();
        let restored: PriorityAssessment = serde_json::from_str(&json).unwrap();

        assert_eq!(original.proposal_id, restored.proposal_id);
        assert_eq!(original.suggested_priority, restored.suggested_priority);
        assert_eq!(original.priority_score, restored.priority_score);
        assert_eq!(original.factors.dependency_factor.score, restored.factors.dependency_factor.score);
    }

    #[test]
    fn priority_assessment_high_score_yields_critical_priority() {
        let proposal_id = TaskProposalId::new();
        let factors = PriorityAssessmentFactors {
            dependency_factor: DependencyFactor::new(30, 5, ""),
            critical_path_factor: CriticalPathFactor::new(25, true, 5, ""),
            business_value_factor: BusinessValueFactor::new(20, vec![], ""),
            complexity_factor: ComplexityFactor::new(15, Complexity::Trivial, ""),
            user_hint_factor: UserHintFactor::new(10, vec![], ""),
        };
        let assessment = PriorityAssessment::new(proposal_id, factors);

        assert_eq!(assessment.priority_score, 100);
        assert_eq!(assessment.suggested_priority, Priority::Critical);
    }

    // ==========================================
    // MessageRole Tests
    // ==========================================

    #[test]
    fn message_role_default_is_user() {
        assert_eq!(MessageRole::default(), MessageRole::User);
    }

    #[test]
    fn message_role_display_user() {
        assert_eq!(format!("{}", MessageRole::User), "user");
    }

    #[test]
    fn message_role_display_orchestrator() {
        assert_eq!(format!("{}", MessageRole::Orchestrator), "orchestrator");
    }

    #[test]
    fn message_role_display_system() {
        assert_eq!(format!("{}", MessageRole::System), "system");
    }

    #[test]
    fn message_role_from_str_user() {
        let role: MessageRole = "user".parse().unwrap();
        assert_eq!(role, MessageRole::User);
    }

    #[test]
    fn message_role_from_str_orchestrator() {
        let role: MessageRole = "orchestrator".parse().unwrap();
        assert_eq!(role, MessageRole::Orchestrator);
    }

    #[test]
    fn message_role_from_str_system() {
        let role: MessageRole = "system".parse().unwrap();
        assert_eq!(role, MessageRole::System);
    }

    #[test]
    fn message_role_from_str_invalid() {
        let result: Result<MessageRole, _> = "invalid".parse();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.value, "invalid");
        assert!(err.to_string().contains("unknown message role"));
    }

    #[test]
    fn message_role_serializes_to_json() {
        let json = serde_json::to_string(&MessageRole::Orchestrator).unwrap();
        assert_eq!(json, "\"orchestrator\"");
    }

    #[test]
    fn message_role_deserializes_from_json() {
        let role: MessageRole = serde_json::from_str("\"system\"").unwrap();
        assert_eq!(role, MessageRole::System);
    }

    #[test]
    fn message_role_clone_works() {
        let role = MessageRole::Orchestrator;
        let cloned = role.clone();
        assert_eq!(role, cloned);
    }

    // ==========================================
    // ChatMessage Tests
    // ==========================================

    #[test]
    fn chat_message_user_in_session() {
        let session_id = IdeationSessionId::new();
        let msg = ChatMessage::user_in_session(session_id.clone(), "Hello world");

        assert_eq!(msg.session_id, Some(session_id));
        assert!(msg.project_id.is_none());
        assert!(msg.task_id.is_none());
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content, "Hello world");
        assert!(msg.metadata.is_none());
        assert!(msg.parent_message_id.is_none());
    }

    #[test]
    fn chat_message_orchestrator_in_session() {
        let session_id = IdeationSessionId::new();
        let msg = ChatMessage::orchestrator_in_session(session_id.clone(), "I can help with that");

        assert_eq!(msg.session_id, Some(session_id));
        assert_eq!(msg.role, MessageRole::Orchestrator);
        assert_eq!(msg.content, "I can help with that");
    }

    #[test]
    fn chat_message_system_in_session() {
        let session_id = IdeationSessionId::new();
        let msg = ChatMessage::system_in_session(session_id.clone(), "Session started");

        assert_eq!(msg.role, MessageRole::System);
        assert_eq!(msg.content, "Session started");
    }

    #[test]
    fn chat_message_user_in_project() {
        let project_id = ProjectId::new();
        let msg = ChatMessage::user_in_project(project_id.clone(), "Project question");

        assert!(msg.session_id.is_none());
        assert_eq!(msg.project_id, Some(project_id));
        assert!(msg.task_id.is_none());
        assert_eq!(msg.role, MessageRole::User);
    }

    #[test]
    fn chat_message_user_about_task() {
        let task_id = TaskId::new();
        let msg = ChatMessage::user_about_task(task_id.clone(), "Task question");

        assert!(msg.session_id.is_none());
        assert!(msg.project_id.is_none());
        assert_eq!(msg.task_id, Some(task_id));
        assert_eq!(msg.role, MessageRole::User);
    }

    #[test]
    fn chat_message_with_metadata() {
        let session_id = IdeationSessionId::new();
        let msg = ChatMessage::user_in_session(session_id, "Test")
            .with_metadata(r#"{"key": "value"}"#);

        assert_eq!(msg.metadata, Some(r#"{"key": "value"}"#.to_string()));
    }

    #[test]
    fn chat_message_with_parent() {
        let session_id = IdeationSessionId::new();
        let parent_id = ChatMessageId::new();
        let msg = ChatMessage::user_in_session(session_id, "Reply")
            .with_parent(parent_id.clone());

        assert_eq!(msg.parent_message_id, Some(parent_id));
    }

    #[test]
    fn chat_message_is_user_true() {
        let msg = ChatMessage::user_in_session(IdeationSessionId::new(), "Test");
        assert!(msg.is_user());
        assert!(!msg.is_orchestrator());
        assert!(!msg.is_system());
    }

    #[test]
    fn chat_message_is_orchestrator_true() {
        let msg = ChatMessage::orchestrator_in_session(IdeationSessionId::new(), "Test");
        assert!(!msg.is_user());
        assert!(msg.is_orchestrator());
        assert!(!msg.is_system());
    }

    #[test]
    fn chat_message_is_system_true() {
        let msg = ChatMessage::system_in_session(IdeationSessionId::new(), "Test");
        assert!(!msg.is_user());
        assert!(!msg.is_orchestrator());
        assert!(msg.is_system());
    }

    #[test]
    fn chat_message_serializes() {
        let msg = ChatMessage::user_in_session(IdeationSessionId::new(), "Serialize test");
        let json = serde_json::to_string(&msg).unwrap();

        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Serialize test\""));
    }

    #[test]
    fn chat_message_deserializes() {
        let json = r#"{
            "id": "msg-123",
            "session_id": "sess-456",
            "project_id": null,
            "task_id": null,
            "role": "orchestrator",
            "content": "Hello there",
            "metadata": null,
            "parent_message_id": null,
            "created_at": "2026-01-24T12:00:00Z"
        }"#;

        let msg: ChatMessage = serde_json::from_str(json).unwrap();

        assert_eq!(msg.id.as_str(), "msg-123");
        assert_eq!(msg.session_id.as_ref().unwrap().as_str(), "sess-456");
        assert_eq!(msg.role, MessageRole::Orchestrator);
        assert_eq!(msg.content, "Hello there");
    }

    #[test]
    fn chat_message_roundtrip_serialization() {
        let original = ChatMessage::user_in_session(IdeationSessionId::new(), "Roundtrip")
            .with_metadata("some meta");

        let json = serde_json::to_string(&original).unwrap();
        let restored: ChatMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(original.id, restored.id);
        assert_eq!(original.role, restored.role);
        assert_eq!(original.content, restored.content);
        assert_eq!(original.metadata, restored.metadata);
    }

    #[test]
    fn chat_message_from_row_works() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            r#"CREATE TABLE chat_messages (
                id TEXT PRIMARY KEY,
                session_id TEXT,
                project_id TEXT,
                task_id TEXT,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                metadata TEXT,
                parent_message_id TEXT,
                created_at TEXT NOT NULL
            )"#,
            [],
        ).unwrap();

        conn.execute(
            r#"INSERT INTO chat_messages (id, session_id, project_id, task_id, role, content, metadata, parent_message_id, created_at)
               VALUES ('msg-1', 'sess-1', NULL, NULL, 'user', 'Test message', NULL, NULL, '2026-01-24T10:00:00Z')"#,
            [],
        ).unwrap();

        let msg: ChatMessage = conn
            .query_row("SELECT * FROM chat_messages WHERE id = 'msg-1'", [], |row| {
                ChatMessage::from_row(row)
            })
            .unwrap();

        assert_eq!(msg.id.as_str(), "msg-1");
        assert_eq!(msg.session_id.as_ref().unwrap().as_str(), "sess-1");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content, "Test message");
    }

    #[test]
    fn chat_message_from_row_with_task_context() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            r#"CREATE TABLE chat_messages (
                id TEXT PRIMARY KEY,
                session_id TEXT,
                project_id TEXT,
                task_id TEXT,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                metadata TEXT,
                parent_message_id TEXT,
                created_at TEXT NOT NULL
            )"#,
            [],
        ).unwrap();

        conn.execute(
            r#"INSERT INTO chat_messages (id, session_id, project_id, task_id, role, content, metadata, parent_message_id, created_at)
               VALUES ('msg-2', NULL, 'proj-1', 'task-1', 'orchestrator', 'Task help', '{"foo":"bar"}', NULL, '2026-01-24T11:00:00Z')"#,
            [],
        ).unwrap();

        let msg: ChatMessage = conn
            .query_row("SELECT * FROM chat_messages WHERE id = 'msg-2'", [], |row| {
                ChatMessage::from_row(row)
            })
            .unwrap();

        assert!(msg.session_id.is_none());
        assert_eq!(msg.project_id.as_ref().unwrap().as_str(), "proj-1");
        assert_eq!(msg.task_id.as_ref().unwrap().as_str(), "task-1");
        assert_eq!(msg.role, MessageRole::Orchestrator);
        assert_eq!(msg.metadata, Some(r#"{"foo":"bar"}"#.to_string()));
    }

    // ==========================================
    // DependencyGraphNode Tests
    // ==========================================

    #[test]
    fn dependency_graph_node_new() {
        let node = DependencyGraphNode::new(TaskProposalId::from_string("prop-1"), "Test Task");

        assert_eq!(node.proposal_id.as_str(), "prop-1");
        assert_eq!(node.title, "Test Task");
        assert_eq!(node.in_degree, 0);
        assert_eq!(node.out_degree, 0);
    }

    #[test]
    fn dependency_graph_node_with_degrees() {
        let node = DependencyGraphNode::new(TaskProposalId::new(), "Task")
            .with_in_degree(2)
            .with_out_degree(3);

        assert_eq!(node.in_degree, 2);
        assert_eq!(node.out_degree, 3);
    }

    #[test]
    fn dependency_graph_node_is_root() {
        let root = DependencyGraphNode::new(TaskProposalId::new(), "Root")
            .with_in_degree(0);
        let not_root = DependencyGraphNode::new(TaskProposalId::new(), "Not Root")
            .with_in_degree(1);

        assert!(root.is_root());
        assert!(!not_root.is_root());
    }

    #[test]
    fn dependency_graph_node_is_leaf() {
        let leaf = DependencyGraphNode::new(TaskProposalId::new(), "Leaf")
            .with_out_degree(0);
        let not_leaf = DependencyGraphNode::new(TaskProposalId::new(), "Not Leaf")
            .with_out_degree(1);

        assert!(leaf.is_leaf());
        assert!(!not_leaf.is_leaf());
    }

    #[test]
    fn dependency_graph_node_is_blocker() {
        let blocker = DependencyGraphNode::new(TaskProposalId::new(), "Blocker")
            .with_out_degree(2);
        let not_blocker = DependencyGraphNode::new(TaskProposalId::new(), "Not Blocker")
            .with_out_degree(0);

        assert!(blocker.is_blocker());
        assert!(!not_blocker.is_blocker());
    }

    #[test]
    fn dependency_graph_node_serializes() {
        let node = DependencyGraphNode::new(TaskProposalId::from_string("prop-1"), "Serialize")
            .with_in_degree(1)
            .with_out_degree(2);
        let json = serde_json::to_string(&node).unwrap();

        assert!(json.contains("\"proposal_id\":\"prop-1\""));
        assert!(json.contains("\"title\":\"Serialize\""));
        assert!(json.contains("\"in_degree\":1"));
        assert!(json.contains("\"out_degree\":2"));
    }

    #[test]
    fn dependency_graph_node_deserializes() {
        let json = r#"{
            "proposal_id": "prop-123",
            "title": "Test Node",
            "in_degree": 3,
            "out_degree": 1
        }"#;

        let node: DependencyGraphNode = serde_json::from_str(json).unwrap();

        assert_eq!(node.proposal_id.as_str(), "prop-123");
        assert_eq!(node.title, "Test Node");
        assert_eq!(node.in_degree, 3);
        assert_eq!(node.out_degree, 1);
    }

    #[test]
    fn dependency_graph_node_equality() {
        let id = TaskProposalId::from_string("same-id");
        let node1 = DependencyGraphNode::new(id.clone(), "Node 1").with_in_degree(1);
        let node2 = DependencyGraphNode::new(id.clone(), "Node 1").with_in_degree(1);
        let node3 = DependencyGraphNode::new(id, "Node 1").with_in_degree(2);

        assert_eq!(node1, node2);
        assert_ne!(node1, node3);
    }

    // ==========================================
    // DependencyGraphEdge Tests
    // ==========================================

    #[test]
    fn dependency_graph_edge_new() {
        let from = TaskProposalId::from_string("from-1");
        let to = TaskProposalId::from_string("to-1");
        let edge = DependencyGraphEdge::new(from.clone(), to.clone());

        assert_eq!(edge.from, from);
        assert_eq!(edge.to, to);
    }

    #[test]
    fn dependency_graph_edge_serializes() {
        let edge = DependencyGraphEdge::new(
            TaskProposalId::from_string("prop-a"),
            TaskProposalId::from_string("prop-b"),
        );
        let json = serde_json::to_string(&edge).unwrap();

        assert!(json.contains("\"from\":\"prop-a\""));
        assert!(json.contains("\"to\":\"prop-b\""));
    }

    #[test]
    fn dependency_graph_edge_deserializes() {
        let json = r#"{"from": "edge-from", "to": "edge-to"}"#;
        let edge: DependencyGraphEdge = serde_json::from_str(json).unwrap();

        assert_eq!(edge.from.as_str(), "edge-from");
        assert_eq!(edge.to.as_str(), "edge-to");
    }

    #[test]
    fn dependency_graph_edge_equality() {
        let edge1 = DependencyGraphEdge::new(
            TaskProposalId::from_string("a"),
            TaskProposalId::from_string("b"),
        );
        let edge2 = DependencyGraphEdge::new(
            TaskProposalId::from_string("a"),
            TaskProposalId::from_string("b"),
        );
        let edge3 = DependencyGraphEdge::new(
            TaskProposalId::from_string("a"),
            TaskProposalId::from_string("c"),
        );

        assert_eq!(edge1, edge2);
        assert_ne!(edge1, edge3);
    }

    // ==========================================
    // DependencyGraph Tests
    // ==========================================

    #[test]
    fn dependency_graph_new() {
        let graph = DependencyGraph::new();

        assert!(graph.nodes.is_empty());
        assert!(graph.edges.is_empty());
        assert!(graph.critical_path.is_empty());
        assert!(!graph.has_cycles);
        assert!(graph.cycles.is_none());
    }

    #[test]
    fn dependency_graph_default() {
        let graph: DependencyGraph = Default::default();
        assert!(graph.is_empty());
    }

    #[test]
    fn dependency_graph_with_nodes_and_edges() {
        let nodes = vec![
            DependencyGraphNode::new(TaskProposalId::from_string("n1"), "Node 1"),
            DependencyGraphNode::new(TaskProposalId::from_string("n2"), "Node 2"),
        ];
        let edges = vec![
            DependencyGraphEdge::new(
                TaskProposalId::from_string("n2"),
                TaskProposalId::from_string("n1"),
            ),
        ];

        let graph = DependencyGraph::with_nodes_and_edges(nodes, edges);

        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn dependency_graph_add_node() {
        let mut graph = DependencyGraph::new();
        graph.add_node(DependencyGraphNode::new(TaskProposalId::new(), "Added"));

        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn dependency_graph_add_edge() {
        let mut graph = DependencyGraph::new();
        graph.add_edge(DependencyGraphEdge::new(
            TaskProposalId::from_string("a"),
            TaskProposalId::from_string("b"),
        ));

        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn dependency_graph_set_critical_path() {
        let mut graph = DependencyGraph::new();
        let path = vec![
            TaskProposalId::from_string("step1"),
            TaskProposalId::from_string("step2"),
            TaskProposalId::from_string("step3"),
        ];

        graph.set_critical_path(path.clone());

        assert_eq!(graph.critical_path, path);
        assert_eq!(graph.critical_path_length(), 3);
    }

    #[test]
    fn dependency_graph_set_cycles() {
        let mut graph = DependencyGraph::new();
        let cycles = vec![
            vec![
                TaskProposalId::from_string("a"),
                TaskProposalId::from_string("b"),
                TaskProposalId::from_string("a"),
            ],
        ];

        graph.set_cycles(cycles.clone());

        assert!(graph.has_cycles);
        assert!(graph.cycles.is_some());
        assert_eq!(graph.cycles.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn dependency_graph_set_empty_cycles() {
        let mut graph = DependencyGraph::new();
        graph.set_cycles(vec![]);

        assert!(!graph.has_cycles);
        assert!(graph.cycles.is_none());
    }

    #[test]
    fn dependency_graph_get_node() {
        let id = TaskProposalId::from_string("find-me");
        let mut graph = DependencyGraph::new();
        graph.add_node(DependencyGraphNode::new(id.clone(), "Find Me"));

        let found = graph.get_node(&id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().title, "Find Me");

        let not_found = graph.get_node(&TaskProposalId::from_string("not-there"));
        assert!(not_found.is_none());
    }

    #[test]
    fn dependency_graph_get_dependencies() {
        let id = TaskProposalId::from_string("a");
        let mut graph = DependencyGraph::new();
        graph.add_edge(DependencyGraphEdge::new(id.clone(), TaskProposalId::from_string("b")));
        graph.add_edge(DependencyGraphEdge::new(id.clone(), TaskProposalId::from_string("c")));
        graph.add_edge(DependencyGraphEdge::new(TaskProposalId::from_string("d"), TaskProposalId::from_string("a")));

        let deps = graph.get_dependencies(&id);
        assert_eq!(deps.len(), 2);
    }

    #[test]
    fn dependency_graph_get_dependents() {
        let id = TaskProposalId::from_string("target");
        let mut graph = DependencyGraph::new();
        graph.add_edge(DependencyGraphEdge::new(TaskProposalId::from_string("a"), id.clone()));
        graph.add_edge(DependencyGraphEdge::new(TaskProposalId::from_string("b"), id.clone()));

        let deps = graph.get_dependents(&id);
        assert_eq!(deps.len(), 2);
    }

    #[test]
    fn dependency_graph_get_roots() {
        let mut graph = DependencyGraph::new();
        graph.add_node(DependencyGraphNode::new(TaskProposalId::new(), "Root 1").with_in_degree(0));
        graph.add_node(DependencyGraphNode::new(TaskProposalId::new(), "Root 2").with_in_degree(0));
        graph.add_node(DependencyGraphNode::new(TaskProposalId::new(), "Not Root").with_in_degree(1));

        let roots = graph.get_roots();
        assert_eq!(roots.len(), 2);
    }

    #[test]
    fn dependency_graph_get_leaves() {
        let mut graph = DependencyGraph::new();
        graph.add_node(DependencyGraphNode::new(TaskProposalId::new(), "Leaf 1").with_out_degree(0));
        graph.add_node(DependencyGraphNode::new(TaskProposalId::new(), "Not Leaf").with_out_degree(2));

        let leaves = graph.get_leaves();
        assert_eq!(leaves.len(), 1);
    }

    #[test]
    fn dependency_graph_is_on_critical_path() {
        let on_path = TaskProposalId::from_string("on-path");
        let off_path = TaskProposalId::from_string("off-path");
        let mut graph = DependencyGraph::new();
        graph.set_critical_path(vec![on_path.clone(), TaskProposalId::from_string("other")]);

        assert!(graph.is_on_critical_path(&on_path));
        assert!(!graph.is_on_critical_path(&off_path));
    }

    #[test]
    fn dependency_graph_serializes() {
        let mut graph = DependencyGraph::new();
        graph.add_node(DependencyGraphNode::new(TaskProposalId::from_string("p1"), "Node 1"));
        graph.add_edge(DependencyGraphEdge::new(
            TaskProposalId::from_string("p2"),
            TaskProposalId::from_string("p1"),
        ));

        let json = serde_json::to_string(&graph).unwrap();

        assert!(json.contains("\"nodes\":["));
        assert!(json.contains("\"edges\":["));
        assert!(json.contains("\"has_cycles\":false"));
    }

    #[test]
    fn dependency_graph_deserializes() {
        let json = r#"{
            "nodes": [
                {"proposal_id": "p1", "title": "Node 1", "in_degree": 0, "out_degree": 1}
            ],
            "edges": [
                {"from": "p2", "to": "p1"}
            ],
            "critical_path": ["p1"],
            "has_cycles": false,
            "cycles": null
        }"#;

        let graph: DependencyGraph = serde_json::from_str(json).unwrap();

        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.edge_count(), 1);
        assert_eq!(graph.critical_path_length(), 1);
        assert!(!graph.has_cycles);
    }

    #[test]
    fn dependency_graph_roundtrip_serialization() {
        let mut original = DependencyGraph::new();
        original.add_node(DependencyGraphNode::new(TaskProposalId::from_string("a"), "A"));
        original.add_node(DependencyGraphNode::new(TaskProposalId::from_string("b"), "B"));
        original.add_edge(DependencyGraphEdge::new(
            TaskProposalId::from_string("b"),
            TaskProposalId::from_string("a"),
        ));
        original.set_critical_path(vec![TaskProposalId::from_string("a"), TaskProposalId::from_string("b")]);

        let json = serde_json::to_string(&original).unwrap();
        let restored: DependencyGraph = serde_json::from_str(&json).unwrap();

        assert_eq!(original.node_count(), restored.node_count());
        assert_eq!(original.edge_count(), restored.edge_count());
        assert_eq!(original.critical_path, restored.critical_path);
    }
}
