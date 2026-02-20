// SQLite-based ReviewIssue repository implementation
// Provides CRUD operations for review issues with lifecycle tracking

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use rusqlite::Connection;

use crate::domain::entities::{
    IssueProgressSummary, IssueStatus, ReviewIssueEntity as ReviewIssue, ReviewIssueId, TaskId,
};
use crate::error::{AppError, AppResult};

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
    conn: Arc<Mutex<Connection>>,
}

impl SqliteReviewIssueRepository {
    /// Create a new SQLite review issue repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    /// Create from an Arc-wrapped mutex connection (for sharing)
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
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
        let conn = self.conn.lock().await;

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
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(issue)
    }

    async fn bulk_create(&self, issues: Vec<ReviewIssue>) -> AppResult<Vec<ReviewIssue>> {
        let conn = self.conn.lock().await;

        conn.execute("BEGIN TRANSACTION", [])
            .map_err(|e| AppError::Database(e.to_string()))?;

        for issue in &issues {
            let result = conn.execute(
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
            );

            if let Err(e) = result {
                let _ = conn.execute("ROLLBACK", []);
                return Err(AppError::Database(e.to_string()));
            }
        }

        conn.execute("COMMIT", [])
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(issues)
    }

    async fn get_by_id(&self, id: &ReviewIssueId) -> AppResult<Option<ReviewIssue>> {
        let conn = self.conn.lock().await;

        let query = format!("SELECT {} FROM review_issues WHERE id = ?1", SELECT_COLUMNS);
        let result = conn.query_row(&query, [id.as_str()], |row| ReviewIssue::from_row(row));

        match result {
            Ok(issue) => Ok(Some(issue)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_by_task_id(&self, task_id: &TaskId) -> AppResult<Vec<ReviewIssue>> {
        let conn = self.conn.lock().await;

        let query = format!(
            "SELECT {} FROM review_issues WHERE task_id = ?1 ORDER BY created_at ASC",
            SELECT_COLUMNS
        );

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| AppError::Database(e.to_string()))?;

        let issues = stmt
            .query_map([task_id.as_str()], ReviewIssue::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(issues)
    }

    async fn get_open_by_task_id(&self, task_id: &TaskId) -> AppResult<Vec<ReviewIssue>> {
        let conn = self.conn.lock().await;

        let query = format!(
            "SELECT {} FROM review_issues WHERE task_id = ?1 AND status = 'open' ORDER BY created_at ASC",
            SELECT_COLUMNS
        );

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| AppError::Database(e.to_string()))?;

        let issues = stmt
            .query_map([task_id.as_str()], ReviewIssue::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(issues)
    }

    async fn update_status(
        &self,
        id: &ReviewIssueId,
        status: IssueStatus,
        resolution_notes: Option<String>,
    ) -> AppResult<ReviewIssue> {
        let conn = self.conn.lock().await;

        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "UPDATE review_issues SET status = ?2, resolution_notes = ?3, updated_at = ?4 WHERE id = ?1",
            rusqlite::params![id.as_str(), status.to_db_string(), resolution_notes, now],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Fetch the updated issue
        let query = format!("SELECT {} FROM review_issues WHERE id = ?1", SELECT_COLUMNS);
        let issue = conn
            .query_row(&query, [id.as_str()], |row| ReviewIssue::from_row(row))
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(issue)
    }

    async fn update(&self, issue: &ReviewIssue) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            r#"UPDATE review_issues SET
                review_note_id = ?2, task_id = ?3, step_id = ?4, no_step_reason = ?5,
                title = ?6, description = ?7, severity = ?8, category = ?9,
                file_path = ?10, line_number = ?11, code_snippet = ?12, status = ?13,
                resolution_notes = ?14, addressed_in_attempt = ?15, verified_by_review_id = ?16,
                updated_at = ?17
            WHERE id = ?1"#,
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
                issue.updated_at.to_rfc3339(),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
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
