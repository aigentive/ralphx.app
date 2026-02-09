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
mod tests {
    use super::*;
    use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

    fn setup() -> SqlitePermissionRepository {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        SqlitePermissionRepository::new(conn)
    }

    fn sample_info() -> PendingPermissionInfo {
        PendingPermissionInfo {
            request_id: "perm-1".to_string(),
            tool_name: "Bash".to_string(),
            tool_input: serde_json::json!({"command": "ls -la"}),
            context: Some("List files".to_string()),
        }
    }

    #[tokio::test]
    async fn test_create_and_get_pending() {
        let repo = setup();
        repo.create_pending(&sample_info()).await.unwrap();

        let pending = repo.get_pending().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].request_id, "perm-1");
        assert_eq!(pending[0].tool_name, "Bash");
        assert_eq!(pending[0].tool_input["command"], "ls -la");
        assert_eq!(pending[0].context, Some("List files".to_string()));
    }

    #[tokio::test]
    async fn test_get_by_request_id() {
        let repo = setup();
        repo.create_pending(&sample_info()).await.unwrap();

        let found = repo.get_by_request_id("perm-1").await.unwrap();
        assert!(found.is_some());
        let p = found.unwrap();
        assert_eq!(p.tool_name, "Bash");
        assert_eq!(p.tool_input["command"], "ls -la");

        let not_found = repo.get_by_request_id("nonexistent").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_resolve() {
        let repo = setup();
        repo.create_pending(&sample_info()).await.unwrap();

        let decision = PermissionDecision {
            decision: "allow".to_string(),
            message: Some("Approved".to_string()),
        };
        let resolved = repo.resolve("perm-1", &decision).await.unwrap();
        assert!(resolved);

        // After resolving, no longer in pending
        let pending = repo.get_pending().await.unwrap();
        assert!(pending.is_empty());

        // But still retrievable by id
        let found = repo.get_by_request_id("perm-1").await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_resolve_nonexistent() {
        let repo = setup();
        let decision = PermissionDecision {
            decision: "deny".to_string(),
            message: None,
        };
        let resolved = repo.resolve("nope", &decision).await.unwrap();
        assert!(!resolved);
    }

    #[tokio::test]
    async fn test_expire_all_pending() {
        let repo = setup();

        for i in 0..3 {
            let info = PendingPermissionInfo {
                request_id: format!("perm-{}", i),
                tool_name: "Bash".to_string(),
                tool_input: serde_json::json!({}),
                context: None,
            };
            repo.create_pending(&info).await.unwrap();
        }

        // Resolve one so it's not pending
        let decision = PermissionDecision {
            decision: "allow".to_string(),
            message: None,
        };
        repo.resolve("perm-0", &decision).await.unwrap();

        let expired = repo.expire_all_pending().await.unwrap();
        assert_eq!(expired, 2);

        let pending = repo.get_pending().await.unwrap();
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn test_remove() {
        let repo = setup();
        repo.create_pending(&sample_info()).await.unwrap();

        let removed = repo.remove("perm-1").await.unwrap();
        assert!(removed);

        let found = repo.get_by_request_id("perm-1").await.unwrap();
        assert!(found.is_none());

        let removed_again = repo.remove("perm-1").await.unwrap();
        assert!(!removed_again);
    }

    #[tokio::test]
    async fn test_expire_all_pending_via_permission_state() {
        use crate::application::permission_state::PermissionState;
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        let repo = Arc::new(SqlitePermissionRepository::new(conn));

        // Seed pending permissions (simulating leftover from a previous app run)
        for i in 0..3 {
            let info = PendingPermissionInfo {
                request_id: format!("stale-{}", i),
                tool_name: "Bash".to_string(),
                tool_input: serde_json::json!({}),
                context: None,
            };
            repo.create_pending(&info).await.unwrap();
        }

        // Resolve one so only 2 remain pending
        let decision = PermissionDecision {
            decision: "allow".to_string(),
            message: None,
        };
        repo.resolve("stale-0", &decision).await.unwrap();

        assert_eq!(repo.get_pending().await.unwrap().len(), 2);

        // Simulate startup: create PermissionState with the repo, call expire
        let state = PermissionState::with_repo(repo.clone() as Arc<dyn crate::domain::repositories::permission_repository::PermissionRepository>);
        state.expire_stale_on_startup().await;

        // All pending should be expired
        assert!(repo.get_pending().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_empty_tool_input_round_trip() {
        let repo = setup();
        let info = PendingPermissionInfo {
            request_id: "perm-empty".to_string(),
            tool_name: "Read".to_string(),
            tool_input: serde_json::json!({}),
            context: None,
        };
        repo.create_pending(&info).await.unwrap();

        let found = repo.get_by_request_id("perm-empty").await.unwrap().unwrap();
        assert_eq!(found.tool_input, serde_json::json!({}));
        assert!(found.context.is_none());
    }
}
