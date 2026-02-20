use async_trait::async_trait;
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::application::permission_state::{PendingPermissionInfo, PermissionDecision};
use crate::domain::repositories::permission_repository::PermissionRepository;
use crate::error::{AppError, AppResult};

pub struct SqlitePermissionRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqlitePermissionRepository {
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
impl PermissionRepository for SqlitePermissionRepository {
    async fn create_pending(&self, info: &PendingPermissionInfo) -> AppResult<()> {
        let conn = self.conn.lock().await;
        let tool_input_json = serde_json::to_string(&info.tool_input)
            .map_err(|e| AppError::Database(e.to_string()))?;

        conn.execute(
            "INSERT INTO pending_permissions (request_id, tool_name, tool_input, context, status)
             VALUES (?1, ?2, ?3, ?4, 'pending')",
            rusqlite::params![
                info.request_id,
                info.tool_name,
                tool_input_json,
                info.context,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn resolve(&self, request_id: &str, decision: &PermissionDecision) -> AppResult<bool> {
        let conn = self.conn.lock().await;
        let rows = conn
            .execute(
                "UPDATE pending_permissions
                 SET status = 'resolved',
                     decision = ?1,
                     decision_message = ?2,
                     resolved_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')
                 WHERE request_id = ?3 AND status = 'pending'",
                rusqlite::params![decision.decision, decision.message, request_id],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows > 0)
    }

    async fn get_pending(&self) -> AppResult<Vec<PendingPermissionInfo>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn
            .prepare(
                "SELECT request_id, tool_name, tool_input, context
                 FROM pending_permissions WHERE status = 'pending'",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            let (request_id, tool_name, tool_input_json, context) =
                row.map_err(|e| AppError::Database(e.to_string()))?;
            let tool_input: serde_json::Value = serde_json::from_str(&tool_input_json)
                .map_err(|e| AppError::Database(e.to_string()))?;
            results.push(PendingPermissionInfo {
                request_id,
                tool_name,
                tool_input,
                context,
            });
        }

        Ok(results)
    }

    async fn get_by_request_id(
        &self,
        request_id: &str,
    ) -> AppResult<Option<PendingPermissionInfo>> {
        let conn = self.conn.lock().await;
        let result = conn.query_row(
            "SELECT request_id, tool_name, tool_input, context
             FROM pending_permissions WHERE request_id = ?1",
            rusqlite::params![request_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        );

        match result {
            Ok((request_id, tool_name, tool_input_json, context)) => {
                let tool_input: serde_json::Value = serde_json::from_str(&tool_input_json)
                    .map_err(|e| AppError::Database(e.to_string()))?;
                Ok(Some(PendingPermissionInfo {
                    request_id,
                    tool_name,
                    tool_input,
                    context,
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
                "UPDATE pending_permissions
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
                "DELETE FROM pending_permissions WHERE request_id = ?1",
                rusqlite::params![request_id],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows > 0)
    }
}

#[cfg(test)]
#[path = "sqlite_permission_repo_tests.rs"]
mod tests;
