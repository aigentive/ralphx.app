// ReviewIssue entity - represents a structured issue from a review
// Provides lifecycle tracking for issue resolution across review cycles
//
// Lifecycle: open → in_progress → addressed → verified (or wontfix)

use chrono::{DateTime, Utc};
use rusqlite::Row;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use super::{ReviewIssueId, ReviewNoteId, TaskId, TaskStepId};

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
        matches!(
            self,
            IssueStatus::Verified | IssueStatus::WontFix | IssueStatus::Addressed
        )
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

        let severity = IssueSeverity::from_db_string(&severity_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                7,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    e.to_string(),
                )),
            )
        })?;

        let category = category_str
            .map(|s| IssueCategory::from_db_string(&s))
            .transpose()
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    8,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e.to_string(),
                    )),
                )
            })?;

        let status = IssueStatus::from_db_string(&status_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                12,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    e.to_string(),
                )),
            )
        })?;

        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    16,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?
            .with_timezone(&Utc);

        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    17,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?
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
        let open = issues
            .iter()
            .filter(|i| i.status == IssueStatus::Open)
            .count() as u32;
        let in_progress = issues
            .iter()
            .filter(|i| i.status == IssueStatus::InProgress)
            .count() as u32;
        let addressed = issues
            .iter()
            .filter(|i| i.status == IssueStatus::Addressed)
            .count() as u32;
        let verified = issues
            .iter()
            .filter(|i| i.status == IssueStatus::Verified)
            .count() as u32;
        let wontfix = issues
            .iter()
            .filter(|i| i.status == IssueStatus::WontFix)
            .count() as u32;

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
#[path = "review_issue_tests.rs"]
mod tests;
