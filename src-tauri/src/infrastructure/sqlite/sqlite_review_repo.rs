// SQLite-based ReviewRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use rusqlite::Connection;

use crate::domain::entities::{
    ProjectId, Review, ReviewAction, ReviewActionId, ReviewActionType, ReviewId, ReviewNote,
    ReviewNoteId, ReviewOutcome, ReviewStatus, ReviewerType, TaskId,
};
use crate::domain::repositories::ReviewRepository;
use crate::error::{AppError, AppResult};

/// SQLite implementation of ReviewRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteReviewRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteReviewRepository {
    /// Create a new SQLite Review repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    /// Create from an Arc-wrapped mutex connection (for sharing)
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Helper to parse datetime from SQLite
    fn parse_datetime(s: &str) -> Option<DateTime<Utc>> {
        // Try parsing with common SQLite datetime formats
        chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
            .ok()
            .map(|ndt| Utc.from_utc_datetime(&ndt))
            .or_else(|| {
                chrono::DateTime::parse_from_rfc3339(s)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            })
    }

    /// Helper to format datetime for SQLite
    fn format_datetime(dt: &DateTime<Utc>) -> String {
        dt.format("%Y-%m-%d %H:%M:%S").to_string()
    }

    /// Parse Review from database row
    fn row_to_review(
        id: String,
        project_id: String,
        task_id: String,
        reviewer_type: String,
        status: String,
        notes: Option<String>,
        created_at: String,
        completed_at: Option<String>,
    ) -> AppResult<Review> {
        let reviewer_type = ReviewerType::from_str(&reviewer_type)
            .map_err(|e| AppError::Database(format!("Invalid reviewer_type: {}", e)))?;

        let status = ReviewStatus::from_str(&status)
            .map_err(|e| AppError::Database(format!("Invalid status: {}", e)))?;

        let created_at = Self::parse_datetime(&created_at).unwrap_or_else(Utc::now);

        Ok(Review {
            id: ReviewId::from_string(id),
            project_id: ProjectId::from_string(project_id),
            task_id: TaskId::from_string(task_id),
            reviewer_type,
            status,
            notes,
            created_at,
            completed_at: completed_at.and_then(|s| Self::parse_datetime(&s)),
        })
    }

    /// Parse ReviewAction from database row
    fn row_to_action(
        id: String,
        review_id: String,
        action_type: String,
        target_task_id: Option<String>,
        created_at: String,
    ) -> AppResult<ReviewAction> {
        let action_type = ReviewActionType::from_str(&action_type)
            .map_err(|e| AppError::Database(format!("Invalid action_type: {}", e)))?;

        let created_at = Self::parse_datetime(&created_at).unwrap_or_else(Utc::now);

        Ok(ReviewAction {
            id: ReviewActionId::from_string(id),
            review_id: ReviewId::from_string(review_id),
            action_type,
            target_task_id: target_task_id.map(TaskId::from_string),
            created_at,
        })
    }

    /// Parse ReviewNote from database row
    fn row_to_note(
        id: String,
        task_id: String,
        reviewer: String,
        outcome: String,
        notes: Option<String>,
        created_at: String,
    ) -> AppResult<ReviewNote> {
        let reviewer = ReviewerType::from_str(&reviewer)
            .map_err(|e| AppError::Database(format!("Invalid reviewer: {}", e)))?;

        let outcome = ReviewOutcome::from_str(&outcome)
            .map_err(|e| AppError::Database(format!("Invalid outcome: {}", e)))?;

        let created_at = Self::parse_datetime(&created_at).unwrap_or_else(Utc::now);

        Ok(ReviewNote {
            id: ReviewNoteId::from_string(id),
            task_id: TaskId::from_string(task_id),
            reviewer,
            outcome,
            notes,
            created_at,
        })
    }
}

#[async_trait]
impl ReviewRepository for SqliteReviewRepository {
    // ========================================
    // Review methods
    // ========================================

    async fn create(&self, review: &Review) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT INTO reviews (id, project_id, task_id, reviewer_type, status, notes, created_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            (
                review.id.as_str(),
                review.project_id.as_str(),
                review.task_id.as_str(),
                review.reviewer_type.to_string(),
                review.status.to_string(),
                review.notes.as_ref(),
                Self::format_datetime(&review.created_at),
                review.completed_at.as_ref().map(Self::format_datetime),
            ),
        )
        .map_err(|e| AppError::Database(format!("Failed to create review: {}", e)))?;

        Ok(())
    }

    async fn get_by_id(&self, id: &ReviewId) -> AppResult<Option<Review>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, project_id, task_id, reviewer_type, status, notes, created_at, completed_at
             FROM reviews WHERE id = ?1",
            [id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            },
        );

        match result {
            Ok((id, project_id, task_id, reviewer_type, status, notes, created_at, completed_at)) => {
                Ok(Some(Self::row_to_review(
                    id,
                    project_id,
                    task_id,
                    reviewer_type,
                    status,
                    notes,
                    created_at,
                    completed_at,
                )?))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(format!("Failed to get review: {}", e))),
        }
    }

    async fn get_by_task_id(&self, task_id: &TaskId) -> AppResult<Vec<Review>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, task_id, reviewer_type, status, notes, created_at, completed_at
                 FROM reviews WHERE task_id = ?1 ORDER BY created_at DESC",
            )
            .map_err(|e| AppError::Database(format!("Failed to prepare statement: {}", e)))?;

        let rows = stmt
            .query_map([task_id.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            })
            .map_err(|e| AppError::Database(format!("Failed to query reviews: {}", e)))?;

        let mut reviews = Vec::new();
        for row in rows {
            let (id, project_id, task_id, reviewer_type, status, notes, created_at, completed_at) =
                row.map_err(|e| AppError::Database(format!("Failed to read row: {}", e)))?;
            reviews.push(Self::row_to_review(
                id,
                project_id,
                task_id,
                reviewer_type,
                status,
                notes,
                created_at,
                completed_at,
            )?);
        }

        Ok(reviews)
    }

    async fn get_pending(&self, project_id: &ProjectId) -> AppResult<Vec<Review>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, task_id, reviewer_type, status, notes, created_at, completed_at
                 FROM reviews WHERE project_id = ?1 AND status = 'pending' ORDER BY created_at ASC",
            )
            .map_err(|e| AppError::Database(format!("Failed to prepare statement: {}", e)))?;

        let rows = stmt
            .query_map([project_id.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            })
            .map_err(|e| AppError::Database(format!("Failed to query reviews: {}", e)))?;

        let mut reviews = Vec::new();
        for row in rows {
            let (id, project_id, task_id, reviewer_type, status, notes, created_at, completed_at) =
                row.map_err(|e| AppError::Database(format!("Failed to read row: {}", e)))?;
            reviews.push(Self::row_to_review(
                id,
                project_id,
                task_id,
                reviewer_type,
                status,
                notes,
                created_at,
                completed_at,
            )?);
        }

        Ok(reviews)
    }

    async fn update(&self, review: &Review) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "UPDATE reviews SET status = ?1, notes = ?2, completed_at = ?3 WHERE id = ?4",
            (
                review.status.to_string(),
                review.notes.as_ref(),
                review.completed_at.as_ref().map(Self::format_datetime),
                review.id.as_str(),
            ),
        )
        .map_err(|e| AppError::Database(format!("Failed to update review: {}", e)))?;

        Ok(())
    }

    async fn delete(&self, id: &ReviewId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute("DELETE FROM reviews WHERE id = ?1", [id.as_str()])
            .map_err(|e| AppError::Database(format!("Failed to delete review: {}", e)))?;

        Ok(())
    }

    // ========================================
    // ReviewAction methods
    // ========================================

    async fn add_action(&self, action: &ReviewAction) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT INTO review_actions (id, review_id, action_type, target_task_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                action.id.as_str(),
                action.review_id.as_str(),
                action.action_type.to_string(),
                action.target_task_id.as_ref().map(|id| id.as_str()),
                Self::format_datetime(&action.created_at),
            ),
        )
        .map_err(|e| AppError::Database(format!("Failed to create review action: {}", e)))?;

        Ok(())
    }

    async fn get_actions(&self, review_id: &ReviewId) -> AppResult<Vec<ReviewAction>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, review_id, action_type, target_task_id, created_at
                 FROM review_actions WHERE review_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(|e| AppError::Database(format!("Failed to prepare statement: {}", e)))?;

        let rows = stmt
            .query_map([review_id.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(|e| AppError::Database(format!("Failed to query actions: {}", e)))?;

        let mut actions = Vec::new();
        for row in rows {
            let (id, review_id, action_type, target_task_id, created_at) =
                row.map_err(|e| AppError::Database(format!("Failed to read row: {}", e)))?;
            actions.push(Self::row_to_action(
                id,
                review_id,
                action_type,
                target_task_id,
                created_at,
            )?);
        }

        Ok(actions)
    }

    async fn get_action_by_id(&self, id: &ReviewActionId) -> AppResult<Option<ReviewAction>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, review_id, action_type, target_task_id, created_at
             FROM review_actions WHERE id = ?1",
            [id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        );

        match result {
            Ok((id, review_id, action_type, target_task_id, created_at)) => {
                Ok(Some(Self::row_to_action(
                    id,
                    review_id,
                    action_type,
                    target_task_id,
                    created_at,
                )?))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(format!("Failed to get action: {}", e))),
        }
    }

    // ========================================
    // ReviewNote methods
    // ========================================

    async fn add_note(&self, note: &ReviewNote) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT INTO review_notes (id, task_id, reviewer, outcome, notes, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (
                note.id.as_str(),
                note.task_id.as_str(),
                note.reviewer.to_string(),
                note.outcome.to_string(),
                note.notes.as_ref(),
                Self::format_datetime(&note.created_at),
            ),
        )
        .map_err(|e| AppError::Database(format!("Failed to create review note: {}", e)))?;

        Ok(())
    }

    async fn get_notes_by_task_id(&self, task_id: &TaskId) -> AppResult<Vec<ReviewNote>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, task_id, reviewer, outcome, notes, created_at
                 FROM review_notes WHERE task_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(|e| AppError::Database(format!("Failed to prepare statement: {}", e)))?;

        let rows = stmt
            .query_map([task_id.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })
            .map_err(|e| AppError::Database(format!("Failed to query notes: {}", e)))?;

        let mut notes = Vec::new();
        for row in rows {
            let (id, task_id, reviewer, outcome, note_text, created_at) =
                row.map_err(|e| AppError::Database(format!("Failed to read row: {}", e)))?;
            notes.push(Self::row_to_note(
                id, task_id, reviewer, outcome, note_text, created_at,
            )?);
        }

        Ok(notes)
    }

    async fn get_note_by_id(&self, id: &ReviewNoteId) -> AppResult<Option<ReviewNote>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, task_id, reviewer, outcome, notes, created_at
             FROM review_notes WHERE id = ?1",
            [id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        );

        match result {
            Ok((id, task_id, reviewer, outcome, note_text, created_at)) => Ok(Some(
                Self::row_to_note(id, task_id, reviewer, outcome, note_text, created_at)?,
            )),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(format!("Failed to get note: {}", e))),
        }
    }

    // ========================================
    // Query methods
    // ========================================

    async fn get_by_status(
        &self,
        project_id: &ProjectId,
        status: ReviewStatus,
    ) -> AppResult<Vec<Review>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, task_id, reviewer_type, status, notes, created_at, completed_at
                 FROM reviews WHERE project_id = ?1 AND status = ?2 ORDER BY created_at DESC",
            )
            .map_err(|e| AppError::Database(format!("Failed to prepare statement: {}", e)))?;

        let rows = stmt
            .query_map([project_id.as_str(), &status.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            })
            .map_err(|e| AppError::Database(format!("Failed to query reviews: {}", e)))?;

        let mut reviews = Vec::new();
        for row in rows {
            let (id, project_id, task_id, reviewer_type, status, notes, created_at, completed_at) =
                row.map_err(|e| AppError::Database(format!("Failed to read row: {}", e)))?;
            reviews.push(Self::row_to_review(
                id,
                project_id,
                task_id,
                reviewer_type,
                status,
                notes,
                created_at,
                completed_at,
            )?);
        }

        Ok(reviews)
    }

    async fn count_pending(&self, project_id: &ProjectId) -> AppResult<u32> {
        let conn = self.conn.lock().await;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM reviews WHERE project_id = ?1 AND status = 'pending'",
                [project_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(format!("Failed to count reviews: {}", e)))?;

        Ok(count as u32)
    }

    async fn has_pending_review(&self, task_id: &TaskId) -> AppResult<bool> {
        let conn = self.conn.lock().await;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM reviews WHERE task_id = ?1 AND status = 'pending'",
                [task_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(format!("Failed to check pending: {}", e)))?;

        Ok(count > 0)
    }

    async fn count_fix_actions(&self, task_id: &TaskId) -> AppResult<u32> {
        let conn = self.conn.lock().await;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*)
                 FROM review_actions ra
                 INNER JOIN reviews r ON ra.review_id = r.id
                 WHERE r.task_id = ?1 AND ra.action_type = 'created_fix_task'",
                [task_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(format!("Failed to count fix actions: {}", e)))?;

        Ok(count as u32)
    }

    async fn get_fix_actions(&self, task_id: &TaskId) -> AppResult<Vec<ReviewAction>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT ra.id, ra.review_id, ra.action_type, ra.target_task_id, ra.created_at
                 FROM review_actions ra
                 INNER JOIN reviews r ON ra.review_id = r.id
                 WHERE r.task_id = ?1 AND ra.action_type = 'created_fix_task'
                 ORDER BY ra.created_at ASC",
            )
            .map_err(|e| AppError::Database(format!("Failed to prepare statement: {}", e)))?;

        let rows = stmt
            .query_map([task_id.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(|e| AppError::Database(format!("Failed to query fix actions: {}", e)))?;

        let mut actions = Vec::new();
        for row in rows {
            let (id, review_id, action_type, target_task_id, created_at) =
                row.map_err(|e| AppError::Database(format!("Failed to read row: {}", e)))?;
            actions.push(Self::row_to_action(
                id,
                review_id,
                action_type,
                target_task_id,
                created_at,
            )?);
        }

        Ok(actions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::sqlite::connection::open_memory_connection;
    use crate::infrastructure::sqlite::migrations::run_migrations;

    fn setup_test_db() -> Connection {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    fn create_test_project_and_task(conn: &Connection) -> (ProjectId, TaskId) {
        let project_id = ProjectId::new();
        let task_id = TaskId::new();

        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES (?1, 'Test', '/path')",
            [project_id.as_str()],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES (?1, ?2, 'feature', 'Test Task')",
            [task_id.as_str(), project_id.as_str()],
        )
        .unwrap();

        (project_id, task_id)
    }

    #[tokio::test]
    async fn test_create_and_get_review() {
        let conn = setup_test_db();
        let (project_id, task_id) = create_test_project_and_task(&conn);
        let repo = SqliteReviewRepository::new(conn);

        let review = Review::new(project_id, task_id, ReviewerType::Ai);
        let review_id = review.id.clone();

        repo.create(&review).await.unwrap();

        let retrieved = repo.get_by_id(&review_id).await.unwrap();
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, review_id);
        assert_eq!(retrieved.reviewer_type, ReviewerType::Ai);
        assert_eq!(retrieved.status, ReviewStatus::Pending);
    }

    #[tokio::test]
    async fn test_get_by_task_id() {
        let conn = setup_test_db();
        let (project_id, task_id) = create_test_project_and_task(&conn);
        let repo = SqliteReviewRepository::new(conn);

        // Create two reviews for the same task
        let review1 = Review::new(project_id.clone(), task_id.clone(), ReviewerType::Ai);
        let review2 = Review::new(project_id, task_id.clone(), ReviewerType::Human);

        repo.create(&review1).await.unwrap();
        repo.create(&review2).await.unwrap();

        let reviews = repo.get_by_task_id(&task_id).await.unwrap();
        assert_eq!(reviews.len(), 2);
    }

    #[tokio::test]
    async fn test_get_pending() {
        let conn = setup_test_db();
        let (project_id, task_id) = create_test_project_and_task(&conn);
        let repo = SqliteReviewRepository::new(conn);

        let mut review = Review::new(project_id.clone(), task_id, ReviewerType::Ai);
        repo.create(&review).await.unwrap();

        // Initially pending
        let pending = repo.get_pending(&project_id).await.unwrap();
        assert_eq!(pending.len(), 1);

        // Approve and update
        review.approve(Some("Good".to_string()));
        repo.update(&review).await.unwrap();

        // No longer pending
        let pending = repo.get_pending(&project_id).await.unwrap();
        assert_eq!(pending.len(), 0);
    }

    #[tokio::test]
    async fn test_update_review() {
        let conn = setup_test_db();
        let (project_id, task_id) = create_test_project_and_task(&conn);
        let repo = SqliteReviewRepository::new(conn);

        let mut review = Review::new(project_id, task_id, ReviewerType::Ai);
        let review_id = review.id.clone();
        repo.create(&review).await.unwrap();

        review.request_changes("Missing tests".to_string());
        repo.update(&review).await.unwrap();

        let retrieved = repo.get_by_id(&review_id).await.unwrap().unwrap();
        assert_eq!(retrieved.status, ReviewStatus::ChangesRequested);
        assert_eq!(retrieved.notes, Some("Missing tests".to_string()));
        assert!(retrieved.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_delete_review() {
        let conn = setup_test_db();
        let (project_id, task_id) = create_test_project_and_task(&conn);
        let repo = SqliteReviewRepository::new(conn);

        let review = Review::new(project_id, task_id, ReviewerType::Ai);
        let review_id = review.id.clone();

        repo.create(&review).await.unwrap();
        repo.delete(&review_id).await.unwrap();

        let retrieved = repo.get_by_id(&review_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_add_and_get_action() {
        let conn = setup_test_db();
        let (project_id, task_id) = create_test_project_and_task(&conn);

        // Create a fix task ID (no FK constraint needed for review_actions.target_task_id)
        let fix_task_id = TaskId::new();

        let repo = SqliteReviewRepository::new(conn);

        let review = Review::new(project_id, task_id, ReviewerType::Ai);
        let review_id = review.id.clone();
        repo.create(&review).await.unwrap();

        let action = ReviewAction::with_target_task(
            review_id.clone(),
            ReviewActionType::CreatedFixTask,
            fix_task_id,
        );
        let action_id = action.id.clone();

        repo.add_action(&action).await.unwrap();

        let actions = repo.get_actions(&review_id).await.unwrap();
        assert_eq!(actions.len(), 1);
        assert!(actions[0].is_fix_task_action());

        let retrieved = repo.get_action_by_id(&action_id).await.unwrap();
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_add_and_get_note() {
        let conn = setup_test_db();
        let (_project_id, task_id) = create_test_project_and_task(&conn);
        let repo = SqliteReviewRepository::new(conn);

        let note1 = ReviewNote::with_notes(
            task_id.clone(),
            ReviewerType::Ai,
            ReviewOutcome::ChangesRequested,
            "Missing tests".to_string(),
        );
        let note2 = ReviewNote::with_notes(
            task_id.clone(),
            ReviewerType::Ai,
            ReviewOutcome::Approved,
            "Looks good now".to_string(),
        );
        let note1_id = note1.id.clone();

        repo.add_note(&note1).await.unwrap();
        repo.add_note(&note2).await.unwrap();

        let notes = repo.get_notes_by_task_id(&task_id).await.unwrap();
        assert_eq!(notes.len(), 2);
        // Notes should be ordered by created_at
        assert_eq!(notes[0].outcome, ReviewOutcome::ChangesRequested);
        assert_eq!(notes[1].outcome, ReviewOutcome::Approved);

        let retrieved = repo.get_note_by_id(&note1_id).await.unwrap();
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_get_by_status() {
        let conn = setup_test_db();
        let (project_id, task_id) = create_test_project_and_task(&conn);
        let repo = SqliteReviewRepository::new(conn);

        let review1 = Review::new(project_id.clone(), task_id.clone(), ReviewerType::Ai);
        let mut review2 = Review::new(project_id.clone(), task_id, ReviewerType::Ai);

        review2.approve(None);

        repo.create(&review1).await.unwrap();
        repo.create(&review2).await.unwrap();

        let pending = repo
            .get_by_status(&project_id, ReviewStatus::Pending)
            .await
            .unwrap();
        assert_eq!(pending.len(), 1);

        let approved = repo
            .get_by_status(&project_id, ReviewStatus::Approved)
            .await
            .unwrap();
        assert_eq!(approved.len(), 1);
    }

    #[tokio::test]
    async fn test_count_pending() {
        let conn = setup_test_db();
        let (project_id, task_id) = create_test_project_and_task(&conn);
        let repo = SqliteReviewRepository::new(conn);

        let review1 = Review::new(project_id.clone(), task_id.clone(), ReviewerType::Ai);
        let review2 = Review::new(project_id.clone(), task_id.clone(), ReviewerType::Ai);
        let mut review3 = Review::new(project_id.clone(), task_id, ReviewerType::Ai);

        review3.approve(None);

        repo.create(&review1).await.unwrap();
        repo.create(&review2).await.unwrap();
        repo.create(&review3).await.unwrap();

        let count = repo.count_pending(&project_id).await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_has_pending_review() {
        let conn = setup_test_db();
        let (project_id, task_id) = create_test_project_and_task(&conn);
        let repo = SqliteReviewRepository::new(conn);

        // No review yet
        assert!(!repo.has_pending_review(&task_id).await.unwrap());

        // Create pending review
        let review = Review::new(project_id, task_id.clone(), ReviewerType::Ai);
        repo.create(&review).await.unwrap();

        assert!(repo.has_pending_review(&task_id).await.unwrap());
    }

    #[tokio::test]
    async fn test_review_cascade_delete() {
        let conn = setup_test_db();
        let (project_id, task_id) = create_test_project_and_task(&conn);
        let repo = SqliteReviewRepository::new(conn);

        let review = Review::new(project_id, task_id.clone(), ReviewerType::Ai);
        let review_id = review.id.clone();
        repo.create(&review).await.unwrap();

        // Add an action
        let action = ReviewAction::new(review_id.clone(), ReviewActionType::Approved);
        repo.add_action(&action).await.unwrap();

        // Delete the review - action should be cascade deleted
        repo.delete(&review_id).await.unwrap();

        let actions = repo.get_actions(&review_id).await.unwrap();
        assert_eq!(actions.len(), 0);
    }

    #[tokio::test]
    async fn test_count_fix_actions() {
        let conn = setup_test_db();
        let (project_id, task_id) = create_test_project_and_task(&conn);
        let repo = SqliteReviewRepository::new(conn);

        // No fix actions yet
        assert_eq!(repo.count_fix_actions(&task_id).await.unwrap(), 0);

        // Create a review and add fix task action
        let review = Review::new(project_id.clone(), task_id.clone(), ReviewerType::Ai);
        let review_id = review.id.clone();
        repo.create(&review).await.unwrap();

        let fix_task_id = TaskId::new();
        let action = ReviewAction::with_target_task(
            review_id.clone(),
            ReviewActionType::CreatedFixTask,
            fix_task_id,
        );
        repo.add_action(&action).await.unwrap();

        assert_eq!(repo.count_fix_actions(&task_id).await.unwrap(), 1);

        // Add another fix task action
        let fix_task_id_2 = TaskId::new();
        let action2 = ReviewAction::with_target_task(
            review_id.clone(),
            ReviewActionType::CreatedFixTask,
            fix_task_id_2,
        );
        repo.add_action(&action2).await.unwrap();

        assert_eq!(repo.count_fix_actions(&task_id).await.unwrap(), 2);

        // Add a non-fix action (should not be counted)
        let action3 = ReviewAction::new(review_id, ReviewActionType::Approved);
        repo.add_action(&action3).await.unwrap();

        assert_eq!(repo.count_fix_actions(&task_id).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_get_fix_actions() {
        let conn = setup_test_db();
        let (project_id, task_id) = create_test_project_and_task(&conn);
        let repo = SqliteReviewRepository::new(conn);

        // Create a review and add actions
        let review = Review::new(project_id, task_id.clone(), ReviewerType::Ai);
        let review_id = review.id.clone();
        repo.create(&review).await.unwrap();

        let fix_task_id = TaskId::new();
        let action1 = ReviewAction::with_target_task(
            review_id.clone(),
            ReviewActionType::CreatedFixTask,
            fix_task_id.clone(),
        );
        repo.add_action(&action1).await.unwrap();

        // Add a non-fix action (should not be returned)
        let action2 = ReviewAction::new(review_id, ReviewActionType::Approved);
        repo.add_action(&action2).await.unwrap();

        let fix_actions = repo.get_fix_actions(&task_id).await.unwrap();
        assert_eq!(fix_actions.len(), 1);
        assert!(fix_actions[0].is_fix_task_action());
        assert_eq!(fix_actions[0].target_task_id, Some(fix_task_id));
    }

    #[tokio::test]
    async fn test_count_fix_actions_across_multiple_reviews() {
        let conn = setup_test_db();
        let (project_id, task_id) = create_test_project_and_task(&conn);
        let repo = SqliteReviewRepository::new(conn);

        // Create first review with fix action
        let review1 = Review::new(project_id.clone(), task_id.clone(), ReviewerType::Ai);
        repo.create(&review1).await.unwrap();

        let action1 = ReviewAction::with_target_task(
            review1.id.clone(),
            ReviewActionType::CreatedFixTask,
            TaskId::new(),
        );
        repo.add_action(&action1).await.unwrap();

        // Create second review with fix action
        let review2 = Review::new(project_id, task_id.clone(), ReviewerType::Ai);
        repo.create(&review2).await.unwrap();

        let action2 = ReviewAction::with_target_task(
            review2.id.clone(),
            ReviewActionType::CreatedFixTask,
            TaskId::new(),
        );
        repo.add_action(&action2).await.unwrap();

        // Should count both fix actions across reviews
        assert_eq!(repo.count_fix_actions(&task_id).await.unwrap(), 2);
    }
}
