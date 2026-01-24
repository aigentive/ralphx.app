// SQLite-based TaskQARepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use rusqlite::Connection;

use crate::domain::entities::{TaskId, TaskQA, TaskQAId};
use crate::domain::qa::{AcceptanceCriteria, QAResults, QATestSteps};
use crate::domain::repositories::TaskQARepository;
use crate::error::{AppError, AppResult};

/// SQLite implementation of TaskQARepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteTaskQARepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteTaskQARepository {
    /// Create a new SQLite TaskQA repository with the given connection
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

    /// Parse TaskQA from database row
    fn row_to_task_qa(
        id: String,
        task_id: String,
        acceptance_criteria: Option<String>,
        qa_test_steps: Option<String>,
        prep_agent_id: Option<String>,
        prep_started_at: Option<String>,
        prep_completed_at: Option<String>,
        actual_implementation: Option<String>,
        refined_test_steps: Option<String>,
        refinement_agent_id: Option<String>,
        refinement_completed_at: Option<String>,
        test_results: Option<String>,
        screenshots: Option<String>,
        test_agent_id: Option<String>,
        test_completed_at: Option<String>,
        created_at: String,
    ) -> AppResult<TaskQA> {
        let acceptance_criteria = acceptance_criteria
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(|e| AppError::Database(format!("JSON parse error for acceptance_criteria: {}", e)))?;

        let qa_test_steps = qa_test_steps
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(|e| AppError::Database(format!("JSON parse error for qa_test_steps: {}", e)))?;

        let refined_test_steps = refined_test_steps
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(|e| AppError::Database(format!("JSON parse error for refined_test_steps: {}", e)))?;

        let test_results = test_results
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(|e| AppError::Database(format!("JSON parse error for test_results: {}", e)))?;

        let screenshots: Vec<String> = screenshots
            .map(|s| serde_json::from_str(&s).unwrap_or_default())
            .unwrap_or_default();

        let created_at = Self::parse_datetime(&created_at)
            .unwrap_or_else(Utc::now);

        Ok(TaskQA {
            id: TaskQAId::from_string(id),
            task_id: TaskId::from_string(task_id),
            acceptance_criteria,
            qa_test_steps,
            prep_agent_id,
            prep_started_at: prep_started_at.and_then(|s| Self::parse_datetime(&s)),
            prep_completed_at: prep_completed_at.and_then(|s| Self::parse_datetime(&s)),
            actual_implementation,
            refined_test_steps,
            refinement_agent_id,
            refinement_completed_at: refinement_completed_at.and_then(|s| Self::parse_datetime(&s)),
            test_results,
            screenshots,
            test_agent_id,
            test_completed_at: test_completed_at.and_then(|s| Self::parse_datetime(&s)),
            created_at,
        })
    }
}

#[async_trait]
impl TaskQARepository for SqliteTaskQARepository {
    async fn create(&self, task_qa: &TaskQA) -> AppResult<()> {
        let conn = self.conn.lock().await;

        let acceptance_criteria_json = task_qa.acceptance_criteria
            .as_ref()
            .map(|c| serde_json::to_string(c))
            .transpose()
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;

        let qa_test_steps_json = task_qa.qa_test_steps
            .as_ref()
            .map(|s| serde_json::to_string(s))
            .transpose()
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;

        let refined_test_steps_json = task_qa.refined_test_steps
            .as_ref()
            .map(|s| serde_json::to_string(s))
            .transpose()
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;

        let test_results_json = task_qa.test_results
            .as_ref()
            .map(|r| serde_json::to_string(r))
            .transpose()
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;

        let screenshots_json = if task_qa.screenshots.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&task_qa.screenshots)
                .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?)
        };

        conn.execute(
            "INSERT INTO task_qa (
                id, task_id,
                acceptance_criteria, qa_test_steps, prep_agent_id, prep_started_at, prep_completed_at,
                actual_implementation, refined_test_steps, refinement_agent_id, refinement_completed_at,
                test_results, screenshots, test_agent_id, test_completed_at,
                created_at
            ) VALUES (
                ?1, ?2,
                ?3, ?4, ?5, ?6, ?7,
                ?8, ?9, ?10, ?11,
                ?12, ?13, ?14, ?15,
                ?16
            )",
            rusqlite::params![
                task_qa.id.as_str(),
                task_qa.task_id.as_str(),
                acceptance_criteria_json,
                qa_test_steps_json,
                task_qa.prep_agent_id,
                task_qa.prep_started_at.map(|dt| Self::format_datetime(&dt)),
                task_qa.prep_completed_at.map(|dt| Self::format_datetime(&dt)),
                task_qa.actual_implementation,
                refined_test_steps_json,
                task_qa.refinement_agent_id,
                task_qa.refinement_completed_at.map(|dt| Self::format_datetime(&dt)),
                test_results_json,
                screenshots_json,
                task_qa.test_agent_id,
                task_qa.test_completed_at.map(|dt| Self::format_datetime(&dt)),
                Self::format_datetime(&task_qa.created_at),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_by_id(&self, id: &TaskQAId) -> AppResult<Option<TaskQA>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT
                id, task_id,
                acceptance_criteria, qa_test_steps, prep_agent_id, prep_started_at, prep_completed_at,
                actual_implementation, refined_test_steps, refinement_agent_id, refinement_completed_at,
                test_results, screenshots, test_agent_id, test_completed_at,
                created_at
            FROM task_qa WHERE id = ?1",
            [id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                    row.get::<_, Option<String>>(13)?,
                    row.get::<_, Option<String>>(14)?,
                    row.get::<_, String>(15)?,
                ))
            },
        );

        match result {
            Ok((id, task_id, ac, qs, pa, ps, pc, ai, rs, ra, rc, tr, ss, ta, tc, ca)) => {
                let task_qa = Self::row_to_task_qa(
                    id, task_id, ac, qs, pa, ps, pc, ai, rs, ra, rc, tr, ss, ta, tc, ca,
                )?;
                Ok(Some(task_qa))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_by_task_id(&self, task_id: &TaskId) -> AppResult<Option<TaskQA>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT
                id, task_id,
                acceptance_criteria, qa_test_steps, prep_agent_id, prep_started_at, prep_completed_at,
                actual_implementation, refined_test_steps, refinement_agent_id, refinement_completed_at,
                test_results, screenshots, test_agent_id, test_completed_at,
                created_at
            FROM task_qa WHERE task_id = ?1",
            [task_id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                    row.get::<_, Option<String>>(13)?,
                    row.get::<_, Option<String>>(14)?,
                    row.get::<_, String>(15)?,
                ))
            },
        );

        match result {
            Ok((id, task_id, ac, qs, pa, ps, pc, ai, rs, ra, rc, tr, ss, ta, tc, ca)) => {
                let task_qa = Self::row_to_task_qa(
                    id, task_id, ac, qs, pa, ps, pc, ai, rs, ra, rc, tr, ss, ta, tc, ca,
                )?;
                Ok(Some(task_qa))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn update_prep(
        &self,
        id: &TaskQAId,
        agent_id: &str,
        criteria: &AcceptanceCriteria,
        steps: &QATestSteps,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;

        let criteria_json = serde_json::to_string(criteria)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;

        let steps_json = serde_json::to_string(steps)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;

        let now = Self::format_datetime(&Utc::now());

        conn.execute(
            "UPDATE task_qa SET
                prep_agent_id = ?1,
                acceptance_criteria = ?2,
                qa_test_steps = ?3,
                prep_completed_at = ?4
            WHERE id = ?5",
            rusqlite::params![
                agent_id,
                criteria_json,
                steps_json,
                now,
                id.as_str(),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_refinement(
        &self,
        id: &TaskQAId,
        agent_id: &str,
        actual_implementation: &str,
        refined_steps: &QATestSteps,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;

        let steps_json = serde_json::to_string(refined_steps)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;

        let now = Self::format_datetime(&Utc::now());

        conn.execute(
            "UPDATE task_qa SET
                refinement_agent_id = ?1,
                actual_implementation = ?2,
                refined_test_steps = ?3,
                refinement_completed_at = ?4
            WHERE id = ?5",
            rusqlite::params![
                agent_id,
                actual_implementation,
                steps_json,
                now,
                id.as_str(),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_results(
        &self,
        id: &TaskQAId,
        agent_id: &str,
        results: &QAResults,
        screenshots: &[String],
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;

        let results_json = serde_json::to_string(results)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;

        let screenshots_json = if screenshots.is_empty() {
            None
        } else {
            Some(serde_json::to_string(screenshots)
                .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?)
        };

        let now = Self::format_datetime(&Utc::now());

        conn.execute(
            "UPDATE task_qa SET
                test_agent_id = ?1,
                test_results = ?2,
                screenshots = ?3,
                test_completed_at = ?4
            WHERE id = ?5",
            rusqlite::params![
                agent_id,
                results_json,
                screenshots_json,
                now,
                id.as_str(),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_pending_prep(&self) -> AppResult<Vec<TaskQA>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare(
            "SELECT
                id, task_id,
                acceptance_criteria, qa_test_steps, prep_agent_id, prep_started_at, prep_completed_at,
                actual_implementation, refined_test_steps, refinement_agent_id, refinement_completed_at,
                test_results, screenshots, test_agent_id, test_completed_at,
                created_at
            FROM task_qa WHERE acceptance_criteria IS NULL"
        ).map_err(|e| AppError::Database(e.to_string()))?;

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, Option<String>>(11)?,
                row.get::<_, Option<String>>(12)?,
                row.get::<_, Option<String>>(13)?,
                row.get::<_, Option<String>>(14)?,
                row.get::<_, String>(15)?,
            ))
        }).map_err(|e| AppError::Database(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            let (id, task_id, ac, qs, pa, ps, pc, ai, rs, ra, rc, tr, ss, ta, tc, ca) =
                row.map_err(|e| AppError::Database(e.to_string()))?;
            let task_qa = Self::row_to_task_qa(
                id, task_id, ac, qs, pa, ps, pc, ai, rs, ra, rc, tr, ss, ta, tc, ca,
            )?;
            results.push(task_qa);
        }

        Ok(results)
    }

    async fn delete(&self, id: &TaskQAId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "DELETE FROM task_qa WHERE id = ?1",
            [id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete_by_task_id(&self, task_id: &TaskId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "DELETE FROM task_qa WHERE task_id = ?1",
            [task_id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn exists_for_task(&self, task_id: &TaskId) -> AppResult<bool> {
        let conn = self.conn.lock().await;

        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM task_qa WHERE task_id = ?1",
            [task_id.as_str()],
            |row| row.get(0),
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::qa::{AcceptanceCriterion, QAStepResult, QATestStep};
    use crate::infrastructure::sqlite::connection::open_memory_connection;
    use crate::infrastructure::sqlite::migrations::run_migrations;

    async fn setup_test_db() -> SqliteTaskQARepository {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert a project and task for foreign key constraint
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        ).unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Test Task')",
            [],
        ).unwrap();

        SqliteTaskQARepository::new(conn)
    }

    #[tokio::test]
    async fn test_create_and_get_by_id() {
        let repo = setup_test_db().await;
        let task_id = TaskId::from_string("task-1".to_string());
        let task_qa = TaskQA::new(task_id);
        let qa_id = task_qa.id.clone();

        repo.create(&task_qa).await.unwrap();

        let retrieved = repo.get_by_id(&qa_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, qa_id);
    }

    #[tokio::test]
    async fn test_get_by_task_id() {
        let repo = setup_test_db().await;
        let task_id = TaskId::from_string("task-1".to_string());
        let task_qa = TaskQA::new(task_id.clone());

        repo.create(&task_qa).await.unwrap();

        let retrieved = repo.get_by_task_id(&task_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().task_id, task_id);
    }

    #[tokio::test]
    async fn test_update_prep() {
        let repo = setup_test_db().await;
        let task_id = TaskId::from_string("task-1".to_string());
        let task_qa = TaskQA::new(task_id);
        let qa_id = task_qa.id.clone();

        repo.create(&task_qa).await.unwrap();

        let criteria = AcceptanceCriteria::from_criteria(vec![
            AcceptanceCriterion::visual("AC1", "Test visual check"),
        ]);
        let steps = QATestSteps::from_steps(vec![
            QATestStep::new("QA1", "AC1", "Test step", vec!["cmd".into()], "Expected"),
        ]);

        repo.update_prep(&qa_id, "agent-1", &criteria, &steps).await.unwrap();

        let retrieved = repo.get_by_id(&qa_id).await.unwrap().unwrap();
        assert!(retrieved.acceptance_criteria.is_some());
        assert!(retrieved.qa_test_steps.is_some());
        assert!(retrieved.prep_completed_at.is_some());
        assert_eq!(retrieved.prep_agent_id, Some("agent-1".into()));
    }

    #[tokio::test]
    async fn test_update_refinement() {
        let repo = setup_test_db().await;
        let task_id = TaskId::from_string("task-1".to_string());
        let task_qa = TaskQA::new(task_id);
        let qa_id = task_qa.id.clone();

        repo.create(&task_qa).await.unwrap();

        let refined_steps = QATestSteps::from_steps(vec![
            QATestStep::new("QA1", "AC1", "Refined step", vec![], "Expected"),
        ]);

        repo.update_refinement(&qa_id, "agent-2", "Added button to header", &refined_steps)
            .await
            .unwrap();

        let retrieved = repo.get_by_id(&qa_id).await.unwrap().unwrap();
        assert!(retrieved.refined_test_steps.is_some());
        assert!(retrieved.actual_implementation.is_some());
        assert!(retrieved.refinement_completed_at.is_some());
        assert_eq!(retrieved.refinement_agent_id, Some("agent-2".into()));
    }

    #[tokio::test]
    async fn test_update_results() {
        let repo = setup_test_db().await;
        let task_id = TaskId::from_string("task-1".to_string());
        let task_qa = TaskQA::new(task_id.clone());
        let qa_id = task_qa.id.clone();

        repo.create(&task_qa).await.unwrap();

        let results = QAResults::from_results(
            task_id.as_str(),
            vec![
                QAStepResult::passed("QA1", Some("ss1.png".into())),
                QAStepResult::passed("QA2", Some("ss2.png".into())),
            ],
        );
        let screenshots = vec!["ss1.png".to_string(), "ss2.png".to_string()];

        repo.update_results(&qa_id, "agent-3", &results, &screenshots)
            .await
            .unwrap();

        let retrieved = repo.get_by_id(&qa_id).await.unwrap().unwrap();
        assert!(retrieved.test_results.is_some());
        assert!(retrieved.test_completed_at.is_some());
        assert_eq!(retrieved.test_agent_id, Some("agent-3".into()));
        assert_eq!(retrieved.screenshots.len(), 2);
    }

    #[tokio::test]
    async fn test_get_pending_prep() {
        let repo = setup_test_db().await;

        // Add another task
        {
            let conn = repo.conn.lock().await;
            conn.execute(
                "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-2', 'proj-1', 'feature', 'Task 2')",
                [],
            ).unwrap();
        }

        // Create two TaskQA records
        let task_id1 = TaskId::from_string("task-1".to_string());
        let task_qa1 = TaskQA::new(task_id1);
        let qa_id1 = task_qa1.id.clone();

        let task_id2 = TaskId::from_string("task-2".to_string());
        let task_qa2 = TaskQA::new(task_id2);

        repo.create(&task_qa1).await.unwrap();
        repo.create(&task_qa2).await.unwrap();

        // Update prep for first one
        let criteria = AcceptanceCriteria::from_criteria(vec![
            AcceptanceCriterion::visual("AC1", "Test"),
        ]);
        let steps = QATestSteps::from_steps(vec![]);
        repo.update_prep(&qa_id1, "agent-1", &criteria, &steps).await.unwrap();

        // Get pending prep - should only return task-2
        let pending = repo.get_pending_prep().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].task_id, TaskId::from_string("task-2".to_string()));
    }

    #[tokio::test]
    async fn test_delete() {
        let repo = setup_test_db().await;
        let task_id = TaskId::from_string("task-1".to_string());
        let task_qa = TaskQA::new(task_id);
        let qa_id = task_qa.id.clone();

        repo.create(&task_qa).await.unwrap();
        repo.delete(&qa_id).await.unwrap();

        let retrieved = repo.get_by_id(&qa_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_delete_by_task_id() {
        let repo = setup_test_db().await;
        let task_id = TaskId::from_string("task-1".to_string());
        let task_qa = TaskQA::new(task_id.clone());

        repo.create(&task_qa).await.unwrap();
        repo.delete_by_task_id(&task_id).await.unwrap();

        let retrieved = repo.get_by_task_id(&task_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_exists_for_task() {
        let repo = setup_test_db().await;
        let task_id = TaskId::from_string("task-1".to_string());

        assert!(!repo.exists_for_task(&task_id).await.unwrap());

        let task_qa = TaskQA::new(task_id.clone());
        repo.create(&task_qa).await.unwrap();

        assert!(repo.exists_for_task(&task_id).await.unwrap());
    }

    #[tokio::test]
    async fn test_json_storage_roundtrip() {
        let repo = setup_test_db().await;
        let task_id = TaskId::from_string("task-1".to_string());
        let task_qa = TaskQA::new(task_id.clone());
        let qa_id = task_qa.id.clone();

        repo.create(&task_qa).await.unwrap();

        // Add complex criteria with multiple types
        let criteria = AcceptanceCriteria::from_criteria(vec![
            AcceptanceCriterion::visual("AC1", "Visual test"),
            AcceptanceCriterion::behavior("AC2", "Behavior test"),
        ]);
        let steps = QATestSteps::from_steps(vec![
            QATestStep::new("QA1", "AC1", "Step 1", vec!["cmd1".into(), "cmd2".into()], "Expected 1"),
            QATestStep::new("QA2", "AC2", "Step 2", vec!["cmd3".into()], "Expected 2"),
        ]);

        repo.update_prep(&qa_id, "agent-1", &criteria, &steps).await.unwrap();

        let retrieved = repo.get_by_id(&qa_id).await.unwrap().unwrap();

        // Verify JSON was stored and retrieved correctly
        let retrieved_criteria = retrieved.acceptance_criteria.unwrap();
        assert_eq!(retrieved_criteria.len(), 2);
        assert_eq!(retrieved_criteria.acceptance_criteria[0].id, "AC1");
        assert_eq!(retrieved_criteria.acceptance_criteria[1].id, "AC2");

        let retrieved_steps = retrieved.qa_test_steps.unwrap();
        assert_eq!(retrieved_steps.len(), 2);
        assert_eq!(retrieved_steps.qa_steps[0].commands.len(), 2);
    }
}
