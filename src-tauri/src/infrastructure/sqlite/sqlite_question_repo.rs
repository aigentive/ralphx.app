use async_trait::async_trait;
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::application::question_state::{PendingQuestionInfo, QuestionAnswer, QuestionOption};
use crate::domain::repositories::question_repository::QuestionRepository;
use crate::error::{AppError, AppResult};

pub struct SqliteQuestionRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteQuestionRepository {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl QuestionRepository for SqliteQuestionRepository {
    async fn create_pending(&self, info: &PendingQuestionInfo) -> AppResult<()> {
        let conn = self.conn.lock().await;
        let options_json =
            serde_json::to_string(&info.options).map_err(|e| AppError::Database(e.to_string()))?;

        conn.execute(
            "INSERT INTO pending_questions (request_id, session_id, question, header, options, multi_select, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending')",
            rusqlite::params![
                info.request_id,
                info.session_id,
                info.question,
                info.header,
                options_json,
                info.multi_select as i64,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn resolve(&self, request_id: &str, answer: &QuestionAnswer) -> AppResult<bool> {
        let conn = self.conn.lock().await;
        let selected_json = serde_json::to_string(&answer.selected_options)
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = conn
            .execute(
                "UPDATE pending_questions
                 SET status = 'resolved',
                     answer_selected_options = ?1,
                     answer_text = ?2,
                     resolved_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')
                 WHERE request_id = ?3 AND status = 'pending'",
                rusqlite::params![selected_json, answer.text, request_id],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows > 0)
    }

    async fn get_pending(&self) -> AppResult<Vec<PendingQuestionInfo>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn
            .prepare(
                "SELECT request_id, session_id, question, header, options, multi_select
                 FROM pending_questions WHERE status = 'pending'",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let options_json: String = row.get(4)?;
                let multi_select_int: i64 = row.get(5)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    options_json,
                    multi_select_int,
                ))
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            let (request_id, session_id, question, header, options_json, multi_select_int) =
                row.map_err(|e| AppError::Database(e.to_string()))?;
            let options: Vec<QuestionOption> = serde_json::from_str(&options_json)
                .map_err(|e| AppError::Database(e.to_string()))?;
            results.push(PendingQuestionInfo {
                request_id,
                session_id,
                question,
                header,
                options,
                multi_select: multi_select_int != 0,
            });
        }

        Ok(results)
    }

    async fn get_by_request_id(
        &self,
        request_id: &str,
    ) -> AppResult<Option<PendingQuestionInfo>> {
        let conn = self.conn.lock().await;
        let result = conn.query_row(
            "SELECT request_id, session_id, question, header, options, multi_select
             FROM pending_questions WHERE request_id = ?1",
            rusqlite::params![request_id],
            |row| {
                let options_json: String = row.get(4)?;
                let multi_select_int: i64 = row.get(5)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    options_json,
                    multi_select_int,
                ))
            },
        );

        match result {
            Ok((request_id, session_id, question, header, options_json, multi_select_int)) => {
                let options: Vec<QuestionOption> = serde_json::from_str(&options_json)
                    .map_err(|e| AppError::Database(e.to_string()))?;
                Ok(Some(PendingQuestionInfo {
                    request_id,
                    session_id,
                    question,
                    header,
                    options,
                    multi_select: multi_select_int != 0,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn expire_all_pending(&self) -> AppResult<u64> {
        let conn = self.conn.lock().await;
        let rows = conn
            .execute(
                "UPDATE pending_questions
                 SET status = 'expired',
                     resolved_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')
                 WHERE status = 'pending'",
                [],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows as u64)
    }

    async fn remove(&self, request_id: &str) -> AppResult<bool> {
        let conn = self.conn.lock().await;
        let rows = conn
            .execute(
                "DELETE FROM pending_questions WHERE request_id = ?1",
                rusqlite::params![request_id],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

    fn setup() -> SqliteQuestionRepository {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        SqliteQuestionRepository::new(conn)
    }

    fn sample_info() -> PendingQuestionInfo {
        PendingQuestionInfo {
            request_id: "req-1".to_string(),
            session_id: "session-1".to_string(),
            question: "Which database?".to_string(),
            header: Some("Database Selection".to_string()),
            options: vec![
                QuestionOption {
                    value: "pg".to_string(),
                    label: "PostgreSQL".to_string(),
                    description: Some("Relational".to_string()),
                },
                QuestionOption {
                    value: "sqlite".to_string(),
                    label: "SQLite".to_string(),
                    description: None,
                },
            ],
            multi_select: false,
        }
    }

    #[tokio::test]
    async fn test_create_and_get_pending() {
        let repo = setup();
        let info = sample_info();

        repo.create_pending(&info).await.unwrap();

        let pending = repo.get_pending().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].request_id, "req-1");
        assert_eq!(pending[0].session_id, "session-1");
        assert_eq!(pending[0].question, "Which database?");
        assert_eq!(pending[0].header, Some("Database Selection".to_string()));
        assert_eq!(pending[0].options.len(), 2);
        assert_eq!(pending[0].options[0].value, "pg");
        assert_eq!(pending[0].options[1].label, "SQLite");
        assert!(!pending[0].multi_select);
    }

    #[tokio::test]
    async fn test_get_by_request_id() {
        let repo = setup();
        repo.create_pending(&sample_info()).await.unwrap();

        let found = repo.get_by_request_id("req-1").await.unwrap();
        assert!(found.is_some());
        let q = found.unwrap();
        assert_eq!(q.question, "Which database?");
        assert_eq!(q.options.len(), 2);

        let not_found = repo.get_by_request_id("nonexistent").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_resolve() {
        let repo = setup();
        repo.create_pending(&sample_info()).await.unwrap();

        let answer = QuestionAnswer {
            selected_options: vec!["pg".to_string()],
            text: None,
        };
        let resolved = repo.resolve("req-1", &answer).await.unwrap();
        assert!(resolved);

        // After resolving, no longer in pending
        let pending = repo.get_pending().await.unwrap();
        assert!(pending.is_empty());

        // But still retrievable by id
        let found = repo.get_by_request_id("req-1").await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_resolve_nonexistent() {
        let repo = setup();
        let answer = QuestionAnswer {
            selected_options: vec![],
            text: None,
        };
        let resolved = repo.resolve("nope", &answer).await.unwrap();
        assert!(!resolved);
    }

    #[tokio::test]
    async fn test_expire_all_pending() {
        let repo = setup();

        for i in 0..3 {
            let info = PendingQuestionInfo {
                request_id: format!("req-{}", i),
                session_id: "session-1".to_string(),
                question: format!("Q{}", i),
                header: None,
                options: vec![],
                multi_select: false,
            };
            repo.create_pending(&info).await.unwrap();
        }

        // Resolve one so it's not pending
        let answer = QuestionAnswer {
            selected_options: vec![],
            text: Some("done".to_string()),
        };
        repo.resolve("req-0", &answer).await.unwrap();

        let expired = repo.expire_all_pending().await.unwrap();
        assert_eq!(expired, 2);

        let pending = repo.get_pending().await.unwrap();
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn test_remove() {
        let repo = setup();
        repo.create_pending(&sample_info()).await.unwrap();

        let removed = repo.remove("req-1").await.unwrap();
        assert!(removed);

        let found = repo.get_by_request_id("req-1").await.unwrap();
        assert!(found.is_none());

        let removed_again = repo.remove("req-1").await.unwrap();
        assert!(!removed_again);
    }

    #[tokio::test]
    async fn test_expire_all_pending_via_question_state() {
        use crate::application::question_state::QuestionState;
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        let repo = Arc::new(SqliteQuestionRepository::new(conn));

        // Seed pending questions (simulating leftover from a previous app run)
        for i in 0..3 {
            let info = PendingQuestionInfo {
                request_id: format!("stale-{}", i),
                session_id: "old-session".to_string(),
                question: format!("Stale Q{}", i),
                header: None,
                options: vec![],
                multi_select: false,
            };
            repo.create_pending(&info).await.unwrap();
        }

        // Resolve one so only 2 remain pending
        let answer = QuestionAnswer {
            selected_options: vec![],
            text: Some("answered".to_string()),
        };
        repo.resolve("stale-0", &answer).await.unwrap();

        assert_eq!(repo.get_pending().await.unwrap().len(), 2);

        // Simulate startup: create QuestionState with the repo, call expire
        let state = QuestionState::with_repo(repo.clone() as Arc<dyn crate::domain::repositories::question_repository::QuestionRepository>);
        state.expire_stale_on_startup().await;

        // All pending should be expired
        assert!(repo.get_pending().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_multi_select_round_trip() {
        let repo = setup();
        let info = PendingQuestionInfo {
            request_id: "req-multi".to_string(),
            session_id: "session-1".to_string(),
            question: "Select all that apply".to_string(),
            header: None,
            options: vec![
                QuestionOption {
                    value: "a".to_string(),
                    label: "A".to_string(),
                    description: None,
                },
                QuestionOption {
                    value: "b".to_string(),
                    label: "B".to_string(),
                    description: None,
                },
            ],
            multi_select: true,
        };
        repo.create_pending(&info).await.unwrap();

        let found = repo.get_by_request_id("req-multi").await.unwrap().unwrap();
        assert!(found.multi_select);
    }
}
