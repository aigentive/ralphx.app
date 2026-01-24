// Ideation system entities - IdeationSession and related types
// These represent brainstorming sessions that produce task proposals

use chrono::{DateTime, TimeZone, Utc};
use rusqlite::Row;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use super::{IdeationSessionId, ProjectId, TaskId, TaskProposalId};

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
}
