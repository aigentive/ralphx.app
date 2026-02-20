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

    async fn get_by_request_id(&self, request_id: &str) -> AppResult<Option<PendingQuestionInfo>> {
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
#[path = "sqlite_question_repo_tests.rs"]
mod tests;
