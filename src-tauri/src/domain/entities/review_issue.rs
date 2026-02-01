// ReviewIssue entity - represents a structured issue from a review
// Provides lifecycle tracking for issue resolution across review cycles
//
// Lifecycle: open → in_progress → addressed → verified (or wontfix)

use chrono::{DateTime, Utc};
use rusqlite::Row;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use super::{ReviewNoteId, TaskId, TaskStepId, ReviewIssueId};

/// Status of a review issue in its lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueStatus {
    /// Issue is open and needs to be addressed
    Open,
    /// Worker is currently working on this issue
    InProgress,
    /// Issue has been addressed by the worker
    Addressed,
    /// Issue has been verified as fixed by a reviewer
    Verified,
    /// Issue was intentionally not fixed (with justification)
    WontFix,
}

impl IssueStatus {
    /// Returns true if this is a terminal status (no further transitions expected)
    pub fn is_terminal(&self) -> bool {
        matches!(self, IssueStatus::Verified | IssueStatus::WontFix)
    }

    /// Returns true if this issue is considered resolved (verified or wontfix)
    pub fn is_resolved(&self) -> bool {
        matches!(self, IssueStatus::Verified | IssueStatus::WontFix | IssueStatus::Addressed)
    }

    /// Converts status to database string representation
    pub fn to_db_string(&self) -> &'static str {
        match self {
            IssueStatus::Open => "open",
            IssueStatus::InProgress => "in_progress",
            IssueStatus::Addressed => "addressed",
            IssueStatus::Verified => "verified",
            IssueStatus::WontFix => "wontfix",
        }
    }

    /// Parses status from database string representation
    pub fn from_db_string(s: &str) -> Result<Self, ParseIssueStatusError> {
        match s {
            "open" => Ok(IssueStatus::Open),
            "in_progress" => Ok(IssueStatus::InProgress),
            "addressed" => Ok(IssueStatus::Addressed),
            "verified" => Ok(IssueStatus::Verified),
            "wontfix" => Ok(IssueStatus::WontFix),
            _ => Err(ParseIssueStatusError(s.to_string())),
        }
    }
}

impl FromStr for IssueStatus {
    type Err = ParseIssueStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_db_string(s)
    }
}

impl std::fmt::Display for IssueStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_db_string())
    }
}

/// Error when parsing an invalid issue status string
#[derive(Debug, Clone, PartialEq)]
pub struct ParseIssueStatusError(pub String);

impl std::fmt::Display for ParseIssueStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid issue status: {}", self.0)
    }
}

impl std::error::Error for ParseIssueStatusError {}

/// Severity of a review issue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    /// Must be fixed before approval
    Critical,
    /// Should be fixed, significant impact
    Major,
    /// Nice to fix, minor impact
    Minor,
    /// Optional improvement suggestion
    Suggestion,
}

impl IssueSeverity {
    /// Returns the priority order (lower = higher priority)
    pub fn priority_order(&self) -> u8 {
        match self {
            IssueSeverity::Critical => 0,
            IssueSeverity::Major => 1,
            IssueSeverity::Minor => 2,
            IssueSeverity::Suggestion => 3,
        }
    }

    /// Converts severity to database string representation
    pub fn to_db_string(&self) -> &'static str {
        match self {
            IssueSeverity::Critical => "critical",
            IssueSeverity::Major => "major",
            IssueSeverity::Minor => "minor",
            IssueSeverity::Suggestion => "suggestion",
        }
    }

    /// Parses severity from database string representation
    pub fn from_db_string(s: &str) -> Result<Self, ParseIssueSeverityError> {
        match s {
            "critical" => Ok(IssueSeverity::Critical),
            "major" => Ok(IssueSeverity::Major),
            "minor" => Ok(IssueSeverity::Minor),
            "suggestion" => Ok(IssueSeverity::Suggestion),
            _ => Err(ParseIssueSeverityError(s.to_string())),
        }
    }
}

impl FromStr for IssueSeverity {
    type Err = ParseIssueSeverityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_db_string(s)
    }
}

impl std::fmt::Display for IssueSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_db_string())
    }
}

/// Error when parsing an invalid issue severity string
#[derive(Debug, Clone, PartialEq)]
pub struct ParseIssueSeverityError(pub String);

impl std::fmt::Display for ParseIssueSeverityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid issue severity: {}", self.0)
    }
}

impl std::error::Error for ParseIssueSeverityError {}

/// Category of a review issue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueCategory {
    /// Bug in the implementation
    Bug,
    /// Missing functionality or requirement
    Missing,
    /// Code quality issue (style, maintainability)
    Quality,
    /// Design or architecture issue
    Design,
}

impl IssueCategory {
    /// Converts category to database string representation
    pub fn to_db_string(&self) -> &'static str {
        match self {
            IssueCategory::Bug => "bug",
            IssueCategory::Missing => "missing",
            IssueCategory::Quality => "quality",
            IssueCategory::Design => "design",
        }
    }

    /// Parses category from database string representation
    pub fn from_db_string(s: &str) -> Result<Self, ParseIssueCategoryError> {
        match s {
            "bug" => Ok(IssueCategory::Bug),
            "missing" => Ok(IssueCategory::Missing),
            "quality" => Ok(IssueCategory::Quality),
            "design" => Ok(IssueCategory::Design),
            _ => Err(ParseIssueCategoryError(s.to_string())),
        }
    }
}

impl FromStr for IssueCategory {
    type Err = ParseIssueCategoryError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_db_string(s)
    }
}

impl std::fmt::Display for IssueCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_db_string())
    }
}

/// Error when parsing an invalid issue category string
#[derive(Debug, Clone, PartialEq)]
pub struct ParseIssueCategoryError(pub String);

impl std::fmt::Display for ParseIssueCategoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid issue category: {}", self.0)
    }
}

impl std::error::Error for ParseIssueCategoryError {}

/// A structured issue found during review
///
/// Issues track specific problems identified during code review.
/// Each issue has a lifecycle that mirrors task execution:
/// - Created during review (status: open)
/// - Worker marks as in_progress while working
/// - Worker marks as addressed when done
/// - Next review verifies or reopens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewIssue {
    /// Unique identifier for this issue
    pub id: ReviewIssueId,
    /// The review note that created this issue
    pub review_note_id: ReviewNoteId,
    /// The task this issue belongs to
    pub task_id: TaskId,

    /// Optional link to a specific task step this issue relates to
    pub step_id: Option<TaskStepId>,
    /// Required justification if step_id is None
    /// Explains why the issue doesn't relate to a specific step
    pub no_step_reason: Option<String>,

    /// Short title describing the issue
    pub title: String,
    /// Optional detailed description
    pub description: Option<String>,
    /// Severity of the issue (critical, major, minor, suggestion)
    pub severity: IssueSeverity,
    /// Category of the issue (bug, missing, quality, design)
    pub category: Option<IssueCategory>,

    /// Optional file path where issue was found
    pub file_path: Option<String>,
    /// Optional line number in the file
    pub line_number: Option<i32>,
    /// Optional code snippet showing the issue
    pub code_snippet: Option<String>,

    /// Current status in the issue lifecycle
    pub status: IssueStatus,
    /// Notes about how the issue was resolved
    pub resolution_notes: Option<String>,
    /// Which execution attempt addressed this issue
    pub addressed_in_attempt: Option<i32>,
    /// Which review verified this issue as fixed
    pub verified_by_review_id: Option<ReviewNoteId>,

    /// When the issue was created
    pub created_at: DateTime<Utc>,
    /// When the issue was last updated
    pub updated_at: DateTime<Utc>,
}

impl ReviewIssue {
    /// Creates a new review issue
    pub fn new(
        review_note_id: ReviewNoteId,
        task_id: TaskId,
        title: String,
        severity: IssueSeverity,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: ReviewIssueId::new(),
            review_note_id,
            task_id,
            step_id: None,
            no_step_reason: None,
            title,
            description: None,
            severity,
            category: None,
            file_path: None,
            line_number: None,
            code_snippet: None,
            status: IssueStatus::Open,
            resolution_notes: None,
            addressed_in_attempt: None,
            verified_by_review_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create an issue with a specific ID (for testing or database restoration)
    pub fn with_id(
        id: ReviewIssueId,
        review_note_id: ReviewNoteId,
        task_id: TaskId,
        title: String,
        severity: IssueSeverity,
    ) -> Self {
        let mut issue = Self::new(review_note_id, task_id, title, severity);
        issue.id = id;
        issue
    }

    /// Returns true if the issue is still open
    pub fn is_open(&self) -> bool {
        self.status == IssueStatus::Open
    }

    /// Returns true if the issue needs work
    pub fn needs_work(&self) -> bool {
        matches!(self.status, IssueStatus::Open | IssueStatus::InProgress)
    }

    /// Returns true if the issue has been resolved
    pub fn is_resolved(&self) -> bool {
        self.status.is_resolved()
    }

    /// Returns true if the issue is in a terminal state
    pub fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }

    /// Updates the updated_at timestamp to now
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    /// Mark the issue as being worked on
    pub fn start_work(&mut self) {
        self.status = IssueStatus::InProgress;
        self.touch();
    }

    /// Mark the issue as addressed
    pub fn mark_addressed(&mut self, resolution_notes: Option<String>, attempt_number: i32) {
        self.status = IssueStatus::Addressed;
        self.resolution_notes = resolution_notes;
        self.addressed_in_attempt = Some(attempt_number);
        self.touch();
    }

    /// Mark the issue as verified
    pub fn verify(&mut self, review_note_id: ReviewNoteId) {
        self.status = IssueStatus::Verified;
        self.verified_by_review_id = Some(review_note_id);
        self.touch();
    }

    /// Reopen the issue (was not actually fixed)
    pub fn reopen(&mut self, reason: Option<String>) {
        self.status = IssueStatus::Open;
        if let Some(r) = reason {
            let existing = self.resolution_notes.take().unwrap_or_default();
            self.resolution_notes = Some(format!("{}[Reopened: {}]", existing, r));
        }
        self.verified_by_review_id = None;
        self.touch();
    }

    /// Mark as won't fix
    pub fn wont_fix(&mut self, reason: String) {
        self.status = IssueStatus::WontFix;
        self.resolution_notes = Some(reason);
        self.touch();
    }

    /// Deserializes a ReviewIssue from a SQLite row
    /// Column order: id, review_note_id, task_id, step_id, no_step_reason, title, description,
    ///               severity, category, file_path, line_number, code_snippet, status,
    ///               resolution_notes, addressed_in_attempt, verified_by_review_id, created_at, updated_at
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let id: String = row.get(0)?;
        let review_note_id: String = row.get(1)?;
        let task_id: String = row.get(2)?;
        let step_id: Option<String> = row.get(3)?;
        let no_step_reason: Option<String> = row.get(4)?;
        let title: String = row.get(5)?;
        let description: Option<String> = row.get(6)?;
        let severity_str: String = row.get(7)?;
        let category_str: Option<String> = row.get(8)?;
        let file_path: Option<String> = row.get(9)?;
        let line_number: Option<i32> = row.get(10)?;
        let code_snippet: Option<String> = row.get(11)?;
        let status_str: String = row.get(12)?;
        let resolution_notes: Option<String> = row.get(13)?;
        let addressed_in_attempt: Option<i32> = row.get(14)?;
        let verified_by_review_id_str: Option<String> = row.get(15)?;
        let created_at_str: String = row.get(16)?;
        let updated_at_str: String = row.get(17)?;

        let severity = IssueSeverity::from_db_string(&severity_str)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(7, rusqlite::types::Type::Text, Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))))?;

        let category = category_str
            .map(|s| IssueCategory::from_db_string(&s))
            .transpose()
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(8, rusqlite::types::Type::Text, Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))))?;

        let status = IssueStatus::from_db_string(&status_str)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(12, rusqlite::types::Type::Text, Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))))?;

        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(16, rusqlite::types::Type::Text, Box::new(e)))?
            .with_timezone(&Utc);

        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(17, rusqlite::types::Type::Text, Box::new(e)))?
            .with_timezone(&Utc);

        Ok(Self {
            id: ReviewIssueId::from_string(id),
            review_note_id: ReviewNoteId::from_string(review_note_id),
            task_id: TaskId::from_string(task_id),
            step_id: step_id.map(TaskStepId::from_string),
            no_step_reason,
            title,
            description,
            severity,
            category,
            file_path,
            line_number,
            code_snippet,
            status,
            resolution_notes,
            addressed_in_attempt,
            verified_by_review_id: verified_by_review_id_str.map(ReviewNoteId::from_string),
            created_at,
            updated_at,
        })
    }
}

/// Summary of issue progress for a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueProgressSummary {
    /// The task this progress summary is for
    pub task_id: String,
    /// Total number of issues
    pub total: u32,
    /// Number of open issues
    pub open: u32,
    /// Number of in-progress issues
    pub in_progress: u32,
    /// Number of addressed issues (awaiting verification)
    pub addressed: u32,
    /// Number of verified issues
    pub verified: u32,
    /// Number of won't fix issues
    pub wontfix: u32,
    /// Percentage resolved (addressed + verified + wontfix) / total * 100
    pub percent_resolved: f32,
    /// Breakdown by severity
    pub by_severity: SeverityBreakdown,
}

/// Count of issues by severity
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SeverityBreakdown {
    pub critical: SeverityCount,
    pub major: SeverityCount,
    pub minor: SeverityCount,
    pub suggestion: SeverityCount,
}

/// Count of total and open issues for a severity level
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SeverityCount {
    pub total: u32,
    pub open: u32,
    pub resolved: u32,
}

impl IssueProgressSummary {
    /// Calculate progress summary from a list of issues
    pub fn from_issues(task_id: &TaskId, issues: &[ReviewIssue]) -> Self {
        let total = issues.len() as u32;
        let open = issues.iter().filter(|i| i.status == IssueStatus::Open).count() as u32;
        let in_progress = issues.iter().filter(|i| i.status == IssueStatus::InProgress).count() as u32;
        let addressed = issues.iter().filter(|i| i.status == IssueStatus::Addressed).count() as u32;
        let verified = issues.iter().filter(|i| i.status == IssueStatus::Verified).count() as u32;
        let wontfix = issues.iter().filter(|i| i.status == IssueStatus::WontFix).count() as u32;

        let resolved = addressed + verified + wontfix;
        let percent_resolved = if total > 0 {
            (resolved as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        let by_severity = Self::calculate_severity_breakdown(issues);

        Self {
            task_id: task_id.as_str().to_string(),
            total,
            open,
            in_progress,
            addressed,
            verified,
            wontfix,
            percent_resolved,
            by_severity,
        }
    }

    fn calculate_severity_breakdown(issues: &[ReviewIssue]) -> SeverityBreakdown {
        let mut breakdown = SeverityBreakdown::default();

        for issue in issues {
            let count = match issue.severity {
                IssueSeverity::Critical => &mut breakdown.critical,
                IssueSeverity::Major => &mut breakdown.major,
                IssueSeverity::Minor => &mut breakdown.minor,
                IssueSeverity::Suggestion => &mut breakdown.suggestion,
            };

            count.total += 1;
            if issue.status == IssueStatus::Open {
                count.open += 1;
            }
            if issue.is_resolved() {
                count.resolved += 1;
            }
        }

        breakdown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_issue() -> ReviewIssue {
        ReviewIssue::new(
            ReviewNoteId::from_string("rn-1"),
            TaskId::from_string("task-1".to_string()),
            "Test issue".to_string(),
            IssueSeverity::Major,
        )
    }

    // ===== IssueStatus Tests =====

    #[test]
    fn issue_status_to_db_string() {
        assert_eq!(IssueStatus::Open.to_db_string(), "open");
        assert_eq!(IssueStatus::InProgress.to_db_string(), "in_progress");
        assert_eq!(IssueStatus::Addressed.to_db_string(), "addressed");
        assert_eq!(IssueStatus::Verified.to_db_string(), "verified");
        assert_eq!(IssueStatus::WontFix.to_db_string(), "wontfix");
    }

    #[test]
    fn issue_status_from_db_string() {
        assert_eq!(IssueStatus::from_db_string("open").unwrap(), IssueStatus::Open);
        assert_eq!(IssueStatus::from_db_string("in_progress").unwrap(), IssueStatus::InProgress);
        assert_eq!(IssueStatus::from_db_string("addressed").unwrap(), IssueStatus::Addressed);
        assert_eq!(IssueStatus::from_db_string("verified").unwrap(), IssueStatus::Verified);
        assert_eq!(IssueStatus::from_db_string("wontfix").unwrap(), IssueStatus::WontFix);
        assert!(IssueStatus::from_db_string("invalid").is_err());
    }

    #[test]
    fn issue_status_is_terminal() {
        assert!(!IssueStatus::Open.is_terminal());
        assert!(!IssueStatus::InProgress.is_terminal());
        assert!(!IssueStatus::Addressed.is_terminal());
        assert!(IssueStatus::Verified.is_terminal());
        assert!(IssueStatus::WontFix.is_terminal());
    }

    #[test]
    fn issue_status_is_resolved() {
        assert!(!IssueStatus::Open.is_resolved());
        assert!(!IssueStatus::InProgress.is_resolved());
        assert!(IssueStatus::Addressed.is_resolved());
        assert!(IssueStatus::Verified.is_resolved());
        assert!(IssueStatus::WontFix.is_resolved());
    }

    #[test]
    fn issue_status_serialization() {
        let status = IssueStatus::InProgress;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"in_progress\"");
        let parsed: IssueStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, parsed);
    }

    // ===== IssueSeverity Tests =====

    #[test]
    fn issue_severity_to_db_string() {
        assert_eq!(IssueSeverity::Critical.to_db_string(), "critical");
        assert_eq!(IssueSeverity::Major.to_db_string(), "major");
        assert_eq!(IssueSeverity::Minor.to_db_string(), "minor");
        assert_eq!(IssueSeverity::Suggestion.to_db_string(), "suggestion");
    }

    #[test]
    fn issue_severity_from_db_string() {
        assert_eq!(IssueSeverity::from_db_string("critical").unwrap(), IssueSeverity::Critical);
        assert_eq!(IssueSeverity::from_db_string("major").unwrap(), IssueSeverity::Major);
        assert_eq!(IssueSeverity::from_db_string("minor").unwrap(), IssueSeverity::Minor);
        assert_eq!(IssueSeverity::from_db_string("suggestion").unwrap(), IssueSeverity::Suggestion);
        assert!(IssueSeverity::from_db_string("invalid").is_err());
    }

    #[test]
    fn issue_severity_priority_order() {
        assert!(IssueSeverity::Critical.priority_order() < IssueSeverity::Major.priority_order());
        assert!(IssueSeverity::Major.priority_order() < IssueSeverity::Minor.priority_order());
        assert!(IssueSeverity::Minor.priority_order() < IssueSeverity::Suggestion.priority_order());
    }

    #[test]
    fn issue_severity_serialization() {
        let severity = IssueSeverity::Critical;
        let json = serde_json::to_string(&severity).unwrap();
        assert_eq!(json, "\"critical\"");
        let parsed: IssueSeverity = serde_json::from_str(&json).unwrap();
        assert_eq!(severity, parsed);
    }

    // ===== IssueCategory Tests =====

    #[test]
    fn issue_category_to_db_string() {
        assert_eq!(IssueCategory::Bug.to_db_string(), "bug");
        assert_eq!(IssueCategory::Missing.to_db_string(), "missing");
        assert_eq!(IssueCategory::Quality.to_db_string(), "quality");
        assert_eq!(IssueCategory::Design.to_db_string(), "design");
    }

    #[test]
    fn issue_category_from_db_string() {
        assert_eq!(IssueCategory::from_db_string("bug").unwrap(), IssueCategory::Bug);
        assert_eq!(IssueCategory::from_db_string("missing").unwrap(), IssueCategory::Missing);
        assert_eq!(IssueCategory::from_db_string("quality").unwrap(), IssueCategory::Quality);
        assert_eq!(IssueCategory::from_db_string("design").unwrap(), IssueCategory::Design);
        assert!(IssueCategory::from_db_string("invalid").is_err());
    }

    #[test]
    fn issue_category_serialization() {
        let category = IssueCategory::Bug;
        let json = serde_json::to_string(&category).unwrap();
        assert_eq!(json, "\"bug\"");
        let parsed: IssueCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(category, parsed);
    }

    // ===== ReviewIssue Tests =====

    #[test]
    fn review_issue_new_creates_with_defaults() {
        let issue = create_test_issue();

        assert_eq!(issue.title, "Test issue");
        assert_eq!(issue.severity, IssueSeverity::Major);
        assert_eq!(issue.status, IssueStatus::Open);
        assert!(issue.step_id.is_none());
        assert!(issue.no_step_reason.is_none());
        assert!(issue.description.is_none());
        assert!(issue.category.is_none());
        assert!(issue.file_path.is_none());
        assert!(issue.line_number.is_none());
        assert!(issue.code_snippet.is_none());
        assert!(issue.resolution_notes.is_none());
        assert!(issue.addressed_in_attempt.is_none());
        assert!(issue.verified_by_review_id.is_none());
    }

    #[test]
    fn review_issue_with_id() {
        let id = ReviewIssueId::from_string("custom-id");
        let issue = ReviewIssue::with_id(
            id.clone(),
            ReviewNoteId::from_string("rn-1"),
            TaskId::from_string("task-1".to_string()),
            "Test".to_string(),
            IssueSeverity::Minor,
        );

        assert_eq!(issue.id, id);
    }

    #[test]
    fn review_issue_is_open() {
        let mut issue = create_test_issue();
        assert!(issue.is_open());

        issue.status = IssueStatus::InProgress;
        assert!(!issue.is_open());
    }

    #[test]
    fn review_issue_needs_work() {
        let mut issue = create_test_issue();
        assert!(issue.needs_work());

        issue.status = IssueStatus::InProgress;
        assert!(issue.needs_work());

        issue.status = IssueStatus::Addressed;
        assert!(!issue.needs_work());
    }

    #[test]
    fn review_issue_lifecycle_start_work() {
        let mut issue = create_test_issue();
        let original_updated = issue.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        issue.start_work();

        assert_eq!(issue.status, IssueStatus::InProgress);
        assert!(issue.updated_at > original_updated);
    }

    #[test]
    fn review_issue_lifecycle_mark_addressed() {
        let mut issue = create_test_issue();
        issue.start_work();

        issue.mark_addressed(Some("Fixed the bug".to_string()), 2);

        assert_eq!(issue.status, IssueStatus::Addressed);
        assert_eq!(issue.resolution_notes, Some("Fixed the bug".to_string()));
        assert_eq!(issue.addressed_in_attempt, Some(2));
    }

    #[test]
    fn review_issue_lifecycle_verify() {
        let mut issue = create_test_issue();
        issue.start_work();
        issue.mark_addressed(Some("Fixed".to_string()), 1);

        let verifying_review = ReviewNoteId::from_string("rn-2");
        issue.verify(verifying_review.clone());

        assert_eq!(issue.status, IssueStatus::Verified);
        assert_eq!(issue.verified_by_review_id, Some(verifying_review));
        assert!(issue.is_terminal());
    }

    #[test]
    fn review_issue_lifecycle_reopen() {
        let mut issue = create_test_issue();
        issue.start_work();
        issue.mark_addressed(Some("Fixed".to_string()), 1);

        issue.reopen(Some("Not actually fixed".to_string()));

        assert_eq!(issue.status, IssueStatus::Open);
        assert!(issue.resolution_notes.as_ref().unwrap().contains("Reopened"));
        assert!(issue.verified_by_review_id.is_none());
    }

    #[test]
    fn review_issue_lifecycle_wont_fix() {
        let mut issue = create_test_issue();

        issue.wont_fix("Not in scope for this task".to_string());

        assert_eq!(issue.status, IssueStatus::WontFix);
        assert_eq!(issue.resolution_notes, Some("Not in scope for this task".to_string()));
        assert!(issue.is_terminal());
    }

    #[test]
    fn review_issue_serialization() {
        let mut issue = create_test_issue();
        issue.description = Some("Detailed description".to_string());
        issue.category = Some(IssueCategory::Bug);
        issue.file_path = Some("src/main.rs".to_string());
        issue.line_number = Some(42);

        let json = serde_json::to_string(&issue).unwrap();
        let parsed: ReviewIssue = serde_json::from_str(&json).unwrap();

        assert_eq!(issue.id, parsed.id);
        assert_eq!(issue.title, parsed.title);
        assert_eq!(issue.severity, parsed.severity);
        assert_eq!(issue.status, parsed.status);
        assert_eq!(issue.description, parsed.description);
        assert_eq!(issue.category, parsed.category);
        assert_eq!(issue.file_path, parsed.file_path);
        assert_eq!(issue.line_number, parsed.line_number);
    }

    // ===== IssueProgressSummary Tests =====

    #[test]
    fn issue_progress_summary_from_empty() {
        let task_id = TaskId::from_string("task-1".to_string());
        let summary = IssueProgressSummary::from_issues(&task_id, &[]);

        assert_eq!(summary.total, 0);
        assert_eq!(summary.open, 0);
        assert_eq!(summary.percent_resolved, 0.0);
    }

    #[test]
    fn issue_progress_summary_calculates_correctly() {
        let task_id = TaskId::from_string("task-1".to_string());
        let review_id = ReviewNoteId::from_string("rn-1");

        let mut issues = vec![
            ReviewIssue::new(review_id.clone(), task_id.clone(), "Issue 1".to_string(), IssueSeverity::Critical),
            ReviewIssue::new(review_id.clone(), task_id.clone(), "Issue 2".to_string(), IssueSeverity::Major),
            ReviewIssue::new(review_id.clone(), task_id.clone(), "Issue 3".to_string(), IssueSeverity::Minor),
            ReviewIssue::new(review_id.clone(), task_id.clone(), "Issue 4".to_string(), IssueSeverity::Suggestion),
        ];

        issues[0].status = IssueStatus::Open;
        issues[1].status = IssueStatus::InProgress;
        issues[2].status = IssueStatus::Addressed;
        issues[3].status = IssueStatus::Verified;

        let summary = IssueProgressSummary::from_issues(&task_id, &issues);

        assert_eq!(summary.total, 4);
        assert_eq!(summary.open, 1);
        assert_eq!(summary.in_progress, 1);
        assert_eq!(summary.addressed, 1);
        assert_eq!(summary.verified, 1);
        assert_eq!(summary.wontfix, 0);
        assert_eq!(summary.percent_resolved, 50.0); // 2 resolved out of 4
    }

    #[test]
    fn issue_progress_summary_severity_breakdown() {
        let task_id = TaskId::from_string("task-1".to_string());
        let review_id = ReviewNoteId::from_string("rn-1");

        let mut issues = vec![
            ReviewIssue::new(review_id.clone(), task_id.clone(), "Critical 1".to_string(), IssueSeverity::Critical),
            ReviewIssue::new(review_id.clone(), task_id.clone(), "Critical 2".to_string(), IssueSeverity::Critical),
            ReviewIssue::new(review_id.clone(), task_id.clone(), "Major 1".to_string(), IssueSeverity::Major),
        ];

        issues[0].status = IssueStatus::Open;
        issues[1].status = IssueStatus::Verified;
        issues[2].status = IssueStatus::Addressed;

        let summary = IssueProgressSummary::from_issues(&task_id, &issues);

        assert_eq!(summary.by_severity.critical.total, 2);
        assert_eq!(summary.by_severity.critical.open, 1);
        assert_eq!(summary.by_severity.critical.resolved, 1);
        assert_eq!(summary.by_severity.major.total, 1);
        assert_eq!(summary.by_severity.major.open, 0);
        assert_eq!(summary.by_severity.major.resolved, 1);
        assert_eq!(summary.by_severity.minor.total, 0);
        assert_eq!(summary.by_severity.suggestion.total, 0);
    }
}
