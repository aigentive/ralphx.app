// SQLite-based ReviewIssue repository implementation
// All rusqlite calls go through DbConnection::run() (spawn_blocking + blocking_lock)
// to prevent blocking the tokio async runtime / timer driver.

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use rusqlite::Connection;

use crate::domain::entities::{
    IssueProgressSummary, IssueStatus, ReviewIssueEntity as ReviewIssue, ReviewIssueId, TaskId,
};
use crate::error::AppResult;
use crate::infrastructure::sqlite::DbConnection;

/// Repository trait for ReviewIssue operations
#[async_trait]
pub trait ReviewIssueRepository: Send + Sync {
    /// Create a new review issue
    async fn create(&self, issue: ReviewIssue) -> AppResult<ReviewIssue>;

    /// Create multiple issues in a single transaction
    async fn bulk_create(&self, issues: Vec<ReviewIssue>) -> AppResult<Vec<ReviewIssue>>;

    /// Get an issue by its ID
    async fn get_by_id(&self, id: &ReviewIssueId) -> AppResult<Option<ReviewIssue>>;

    /// Get all issues for a task
    async fn get_by_task_id(&self, task_id: &TaskId) -> AppResult<Vec<ReviewIssue>>;

    /// Get only open issues for a task (status = 'open')
    async fn get_open_by_task_id(&self, task_id: &TaskId) -> AppResult<Vec<ReviewIssue>>;

    /// Update the status of an issue
    async fn update_status(
        &self,
        id: &ReviewIssueId,
        status: IssueStatus,
        resolution_notes: Option<String>,
    ) -> AppResult<ReviewIssue>;

    /// Update full issue (for lifecycle methods)
    async fn update(&self, issue: &ReviewIssue) -> AppResult<()>;

    /// Get progress summary for a task
    async fn get_summary(&self, task_id: &TaskId) -> AppResult<IssueProgressSummary>;
}

/// SQLite implementation of ReviewIssueRepository
pub struct SqliteReviewIssueRepository {
    db: DbConnection,
}

impl SqliteReviewIssueRepository {
    /// Create a new SQLite review issue repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            db: DbConnection::new(conn),
        }
    }

    /// Create from an Arc-wrapped mutex connection (for sharing)
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            db: DbConnection::from_shared(conn),
        }
    }
}

// SQL for insert - matches column order in from_row
const INSERT_SQL: &str = r#"
    INSERT INTO review_issues (
        id, review_note_id, task_id, step_id, no_step_reason, title, description,
        severity, category, file_path, line_number, code_snippet, status,
        resolution_notes, addressed_in_attempt, verified_by_review_id, created_at, updated_at
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
"#;

// SQL for select - matches column order expected by from_row
const SELECT_COLUMNS: &str = r#"
    id, review_note_id, task_id, step_id, no_step_reason, title, description,
    severity, category, file_path, line_number, code_snippet, status,
    resolution_notes, addressed_in_attempt, verified_by_review_id, created_at, updated_at
"#;

#[async_trait]
impl ReviewIssueRepository for SqliteReviewIssueRepository {
    async fn create(&self, issue: ReviewIssue) -> AppResult<ReviewIssue> {
        self.db
            .run(move |conn| {
                conn.execute(
                    INSERT_SQL,
                    rusqlite::params![
                        issue.id.as_str(),
                        issue.review_note_id.as_str(),
                        issue.task_id.as_str(),
                        issue.step_id.as_ref().map(|id| id.as_str()),
                        issue.no_step_reason,
                        issue.title,
                        issue.description,
                        issue.severity.to_db_string(),
                        issue.category.as_ref().map(|c| c.to_db_string()),
                        issue.file_path,
                        issue.line_number,
                        issue.code_snippet,
                        issue.status.to_db_string(),
                        issue.resolution_notes,
                        issue.addressed_in_attempt,
                        issue.verified_by_review_id.as_ref().map(|id| id.as_str()),
                        issue.created_at.to_rfc3339(),
                        issue.updated_at.to_rfc3339(),
                    ],
                )?;
                Ok(issue)
            })
            .await
    }

    async fn bulk_create(&self, issues: Vec<ReviewIssue>) -> AppResult<Vec<ReviewIssue>> {
        self.db
            .run(move |conn| {
                let tx = conn.unchecked_transaction()?;
                for issue in &issues {
                    tx.execute(
                        INSERT_SQL,
                        rusqlite::params![
                            issue.id.as_str(),
                            issue.review_note_id.as_str(),
                            issue.task_id.as_str(),
                            issue.step_id.as_ref().map(|id| id.as_str()),
                            issue.no_step_reason,
                            issue.title,
                            issue.description,
                            issue.severity.to_db_string(),
                            issue.category.as_ref().map(|c| c.to_db_string()),
                            issue.file_path,
                            issue.line_number,
                            issue.code_snippet,
                            issue.status.to_db_string(),
                            issue.resolution_notes,
                            issue.addressed_in_attempt,
                            issue.verified_by_review_id.as_ref().map(|id| id.as_str()),
                            issue.created_at.to_rfc3339(),
                            issue.updated_at.to_rfc3339(),
                        ],
                    )?;
                }
                tx.commit()?;
                Ok(issues)
            })
            .await
    }

    async fn get_by_id(&self, id: &ReviewIssueId) -> AppResult<Option<ReviewIssue>> {
        let id_str = id.as_str().to_string();
        self.db
            .query_optional(move |conn| {
                let query = format!("SELECT {} FROM review_issues WHERE id = ?1", SELECT_COLUMNS);
                conn.query_row(&query, rusqlite::params![id_str], |row| {
                    ReviewIssue::from_row(row)
                })
            })
            .await
    }

    async fn get_by_task_id(&self, task_id: &TaskId) -> AppResult<Vec<ReviewIssue>> {
        let task_id_str = task_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let query = format!(
                    "SELECT {} FROM review_issues WHERE task_id = ?1 ORDER BY created_at ASC",
                    SELECT_COLUMNS
                );
                let mut stmt = conn.prepare(&query)?;
                let issues = stmt
                    .query_map(rusqlite::params![task_id_str], ReviewIssue::from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(issues)
            })
            .await
    }

    async fn get_open_by_task_id(&self, task_id: &TaskId) -> AppResult<Vec<ReviewIssue>> {
        let task_id_str = task_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let query = format!(
                    "SELECT {} FROM review_issues WHERE task_id = ?1 AND status = 'open' ORDER BY created_at ASC",
                    SELECT_COLUMNS
                );
                let mut stmt = conn.prepare(&query)?;
                let issues = stmt
                    .query_map(rusqlite::params![task_id_str], ReviewIssue::from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(issues)
            })
            .await
    }

    async fn update_status(
        &self,
        id: &ReviewIssueId,
        status: IssueStatus,
        resolution_notes: Option<String>,
    ) -> AppResult<ReviewIssue> {
        let id_str = id.as_str().to_string();
        self.db
            .run(move |conn| {
                let now = chrono::Utc::now().to_rfc3339();
                conn.execute(
                    "UPDATE review_issues SET status = ?2, resolution_notes = ?3, updated_at = ?4 WHERE id = ?1",
                    rusqlite::params![id_str, status.to_db_string(), resolution_notes, now],
                )?;
                let query =
                    format!("SELECT {} FROM review_issues WHERE id = ?1", SELECT_COLUMNS);
                let issue = conn.query_row(&query, rusqlite::params![id_str], |row| {
                    ReviewIssue::from_row(row)
                })?;
                Ok(issue)
            })
            .await
    }

    async fn update(&self, issue: &ReviewIssue) -> AppResult<()> {
        let id_str = issue.id.as_str().to_string();
        let review_note_id_str = issue.review_note_id.as_str().to_string();
        let task_id_str = issue.task_id.as_str().to_string();
        let step_id_str = issue.step_id.as_ref().map(|id| id.as_str().to_string());
        let no_step_reason = issue.no_step_reason.clone();
        let title = issue.title.clone();
        let description = issue.description.clone();
        let severity = issue.severity.to_db_string();
        let category = issue.category.as_ref().map(|c| c.to_db_string());
        let file_path = issue.file_path.clone();
        let line_number = issue.line_number;
        let code_snippet = issue.code_snippet.clone();
        let status = issue.status.to_db_string();
        let resolution_notes = issue.resolution_notes.clone();
        let addressed_in_attempt = issue.addressed_in_attempt;
        let verified_by_review_id_str = issue
            .verified_by_review_id
            .as_ref()
            .map(|id| id.as_str().to_string());
        let updated_at = issue.updated_at.to_rfc3339();

        self.db
            .run(move |conn| {
                conn.execute(
                    r#"UPDATE review_issues SET
                        review_note_id = ?2, task_id = ?3, step_id = ?4, no_step_reason = ?5,
                        title = ?6, description = ?7, severity = ?8, category = ?9,
                        file_path = ?10, line_number = ?11, code_snippet = ?12, status = ?13,
                        resolution_notes = ?14, addressed_in_attempt = ?15, verified_by_review_id = ?16,
                        updated_at = ?17
                    WHERE id = ?1"#,
                    rusqlite::params![
                        id_str,
                        review_note_id_str,
                        task_id_str,
                        step_id_str,
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
                        verified_by_review_id_str,
                        updated_at,
                    ],
                )?;
                Ok(())
            })
            .await
    }

    async fn get_summary(&self, task_id: &TaskId) -> AppResult<IssueProgressSummary> {
        // Fetch all issues for the task and calculate summary
        let issues = self.get_by_task_id(task_id).await?;
        Ok(IssueProgressSummary::from_issues(task_id, &issues))
    }
}

#[cfg(test)]
#[path = "sqlite_review_issue_repo_tests.rs"]
mod tests;
