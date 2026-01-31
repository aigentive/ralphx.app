// SQLite-based ActivityEventRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access
// Implements cursor-based pagination using (created_at, id) tuples
// Cursor format: "timestamp|id" with pipe separator to avoid ISO 8601 colon conflicts

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use rusqlite::Connection;

use crate::domain::entities::{ActivityEvent, ActivityEventId, IdeationSessionId, TaskId};
use crate::domain::repositories::{
    ActivityEventFilter, ActivityEventPage, ActivityEventRepository,
};
use crate::error::{AppError, AppResult};

/// Maximum allowed limit for pagination
const MAX_LIMIT: u32 = 100;

/// SQLite implementation of ActivityEventRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteActivityEventRepository {
    pub(crate) conn: Arc<Mutex<Connection>>,
}

impl SqliteActivityEventRepository {
    /// Create a new SQLite activity event repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    /// Create from an Arc-wrapped mutex connection (for sharing)
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Parse a cursor string into (timestamp, id) tuple
    /// Cursor format: "timestamp|id" using pipe separator to avoid ISO 8601 colon conflicts
    fn parse_cursor(cursor: &str) -> Option<(String, String)> {
        cursor
            .split_once('|')
            .map(|(ts, id)| (ts.to_string(), id.to_string()))
    }

    /// Format a cursor from an event
    fn format_cursor(event: &ActivityEvent) -> String {
        format!("{}|{}", event.created_at.to_rfc3339(), event.id)
    }

    /// Build filter clause with positional placeholders starting at the given index
    fn build_filter_clause(
        filter: Option<&ActivityEventFilter>,
        start_param_idx: usize,
    ) -> (String, Vec<String>) {
        let mut conditions = Vec::new();
        let mut params = Vec::new();
        let mut idx = start_param_idx;

        if let Some(f) = filter {
            if let Some(event_types) = &f.event_types {
                if !event_types.is_empty() {
                    let placeholders: Vec<String> = event_types
                        .iter()
                        .map(|t| {
                            params.push(t.to_string());
                            let placeholder = format!("?{}", idx);
                            idx += 1;
                            placeholder
                        })
                        .collect();
                    conditions.push(format!("event_type IN ({})", placeholders.join(", ")));
                }
            }
            if let Some(roles) = &f.roles {
                if !roles.is_empty() {
                    let placeholders: Vec<String> = roles
                        .iter()
                        .map(|r| {
                            params.push(r.to_string());
                            let placeholder = format!("?{}", idx);
                            idx += 1;
                            placeholder
                        })
                        .collect();
                    conditions.push(format!("role IN ({})", placeholders.join(", ")));
                }
            }
            if let Some(statuses) = &f.statuses {
                if !statuses.is_empty() {
                    let placeholders: Vec<String> = statuses
                        .iter()
                        .map(|s| {
                            params.push(s.to_string());
                            let placeholder = format!("?{}", idx);
                            idx += 1;
                            placeholder
                        })
                        .collect();
                    conditions.push(format!("internal_status IN ({})", placeholders.join(", ")));
                }
            }
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!(" AND {}", conditions.join(" AND "))
        };

        (where_clause, params)
    }
}

#[async_trait]
impl ActivityEventRepository for SqliteActivityEventRepository {
    async fn save(&self, event: ActivityEvent) -> AppResult<ActivityEvent> {
        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT INTO activity_events (id, task_id, ideation_session_id, internal_status, event_type, role, content, metadata, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                event.id.as_str(),
                event.task_id.as_ref().map(|id| id.as_str()),
                event.ideation_session_id.as_ref().map(|id| id.as_str()),
                event.internal_status.as_ref().map(|s| s.to_string()),
                event.event_type.to_string(),
                event.role.to_string(),
                event.content,
                event.metadata,
                event.created_at.to_rfc3339(),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(event)
    }

    async fn get_by_id(&self, id: &ActivityEventId) -> AppResult<Option<ActivityEvent>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, task_id, ideation_session_id, internal_status, event_type, role, content, metadata, created_at
             FROM activity_events WHERE id = ?1",
            [id.as_str()],
            ActivityEvent::from_row,
        );

        match result {
            Ok(event) => Ok(Some(event)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn list_by_task_id(
        &self,
        task_id: &TaskId,
        cursor: Option<&str>,
        limit: u32,
        filter: Option<&ActivityEventFilter>,
    ) -> AppResult<ActivityEventPage> {
        let conn = self.conn.lock().await;

        // Cap limit at MAX_LIMIT
        let limit = limit.min(MAX_LIMIT);
        // Request one extra to detect has_more
        let fetch_limit = limit + 1;

        let (sql, params): (String, Vec<String>) = if let Some(cursor_str) = cursor {
            if let Some((cursor_ts, cursor_id)) = Self::parse_cursor(cursor_str) {
                // Cursor pagination: get events older than cursor
                // Params: ?1=task_id, ?2=cursor_ts, ?3=cursor_id, ?4=limit, ?5+=filter params
                let (filter_clause, filter_params) = Self::build_filter_clause(filter, 5);
                let sql = format!(
                    "SELECT id, task_id, ideation_session_id, internal_status, event_type, role, content, metadata, created_at
                     FROM activity_events
                     WHERE task_id = ?1 AND (created_at < ?2 OR (created_at = ?2 AND id < ?3)){}
                     ORDER BY created_at DESC, id DESC
                     LIMIT ?4",
                    filter_clause
                );
                let mut params: Vec<String> = vec![
                    task_id.as_str().to_string(),
                    cursor_ts,
                    cursor_id,
                    fetch_limit.to_string(),
                ];
                params.extend(filter_params);
                (sql, params)
            } else {
                // Invalid cursor, treat as first page
                // Params: ?1=task_id, ?2=limit, ?3+=filter params
                let (filter_clause, filter_params) = Self::build_filter_clause(filter, 3);
                let sql = format!(
                    "SELECT id, task_id, ideation_session_id, internal_status, event_type, role, content, metadata, created_at
                     FROM activity_events
                     WHERE task_id = ?1{}
                     ORDER BY created_at DESC, id DESC
                     LIMIT ?2",
                    filter_clause
                );
                let mut params: Vec<String> =
                    vec![task_id.as_str().to_string(), fetch_limit.to_string()];
                params.extend(filter_params);
                (sql, params)
            }
        } else {
            // First page (no cursor)
            // Params: ?1=task_id, ?2=limit, ?3+=filter params
            let (filter_clause, filter_params) = Self::build_filter_clause(filter, 3);
            let sql = format!(
                "SELECT id, task_id, ideation_session_id, internal_status, event_type, role, content, metadata, created_at
                 FROM activity_events
                 WHERE task_id = ?1{}
                 ORDER BY created_at DESC, id DESC
                 LIMIT ?2",
                filter_clause
            );
            let mut params: Vec<String> =
                vec![task_id.as_str().to_string(), fetch_limit.to_string()];
            params.extend(filter_params);
            (sql, params)
        };

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| AppError::Database(e.to_string()))?;

        let params_refs: Vec<&dyn rusqlite::ToSql> = params
            .iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();

        let mut events: Vec<ActivityEvent> = stmt
            .query_map(params_refs.as_slice(), ActivityEvent::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        let has_more = events.len() > limit as usize;
        if has_more {
            events.truncate(limit as usize);
        }

        let next_cursor = if has_more {
            events.last().map(Self::format_cursor)
        } else {
            None
        };

        Ok(ActivityEventPage {
            events,
            cursor: next_cursor,
            has_more,
        })
    }

    async fn list_by_session_id(
        &self,
        session_id: &IdeationSessionId,
        cursor: Option<&str>,
        limit: u32,
        filter: Option<&ActivityEventFilter>,
    ) -> AppResult<ActivityEventPage> {
        let conn = self.conn.lock().await;

        // Cap limit at MAX_LIMIT
        let limit = limit.min(MAX_LIMIT);
        // Request one extra to detect has_more
        let fetch_limit = limit + 1;

        let (sql, params): (String, Vec<String>) = if let Some(cursor_str) = cursor {
            if let Some((cursor_ts, cursor_id)) = Self::parse_cursor(cursor_str) {
                // Cursor pagination: get events older than cursor
                let (filter_clause, filter_params) = Self::build_filter_clause(filter, 5);
                let sql = format!(
                    "SELECT id, task_id, ideation_session_id, internal_status, event_type, role, content, metadata, created_at
                     FROM activity_events
                     WHERE ideation_session_id = ?1 AND (created_at < ?2 OR (created_at = ?2 AND id < ?3)){}
                     ORDER BY created_at DESC, id DESC
                     LIMIT ?4",
                    filter_clause
                );
                let mut params: Vec<String> = vec![
                    session_id.as_str().to_string(),
                    cursor_ts,
                    cursor_id,
                    fetch_limit.to_string(),
                ];
                params.extend(filter_params);
                (sql, params)
            } else {
                // Invalid cursor, treat as first page
                let (filter_clause, filter_params) = Self::build_filter_clause(filter, 3);
                let sql = format!(
                    "SELECT id, task_id, ideation_session_id, internal_status, event_type, role, content, metadata, created_at
                     FROM activity_events
                     WHERE ideation_session_id = ?1{}
                     ORDER BY created_at DESC, id DESC
                     LIMIT ?2",
                    filter_clause
                );
                let mut params: Vec<String> =
                    vec![session_id.as_str().to_string(), fetch_limit.to_string()];
                params.extend(filter_params);
                (sql, params)
            }
        } else {
            // First page (no cursor)
            let (filter_clause, filter_params) = Self::build_filter_clause(filter, 3);
            let sql = format!(
                "SELECT id, task_id, ideation_session_id, internal_status, event_type, role, content, metadata, created_at
                 FROM activity_events
                 WHERE ideation_session_id = ?1{}
                 ORDER BY created_at DESC, id DESC
                 LIMIT ?2",
                filter_clause
            );
            let mut params: Vec<String> =
                vec![session_id.as_str().to_string(), fetch_limit.to_string()];
            params.extend(filter_params);
            (sql, params)
        };

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| AppError::Database(e.to_string()))?;

        let params_refs: Vec<&dyn rusqlite::ToSql> = params
            .iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();

        let mut events: Vec<ActivityEvent> = stmt
            .query_map(params_refs.as_slice(), ActivityEvent::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        let has_more = events.len() > limit as usize;
        if has_more {
            events.truncate(limit as usize);
        }

        let next_cursor = if has_more {
            events.last().map(Self::format_cursor)
        } else {
            None
        };

        Ok(ActivityEventPage {
            events,
            cursor: next_cursor,
            has_more,
        })
    }

    async fn delete_by_task_id(&self, task_id: &TaskId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "DELETE FROM activity_events WHERE task_id = ?1",
            [task_id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete_by_session_id(&self, session_id: &IdeationSessionId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "DELETE FROM activity_events WHERE ideation_session_id = ?1",
            [session_id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn count_by_task_id(
        &self,
        task_id: &TaskId,
        filter: Option<&ActivityEventFilter>,
    ) -> AppResult<u64> {
        let conn = self.conn.lock().await;

        let (filter_clause, filter_params) = Self::build_filter_clause(filter, 2);
        let sql = format!(
            "SELECT COUNT(*) FROM activity_events WHERE task_id = ?1{}",
            filter_clause
        );

        let mut params: Vec<String> = vec![task_id.as_str().to_string()];
        params.extend(filter_params);

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| AppError::Database(e.to_string()))?;

        let params_refs: Vec<&dyn rusqlite::ToSql> = params
            .iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();

        let count: i64 = stmt
            .query_row(params_refs.as_slice(), |row| row.get(0))
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count as u64)
    }

    async fn count_by_session_id(
        &self,
        session_id: &IdeationSessionId,
        filter: Option<&ActivityEventFilter>,
    ) -> AppResult<u64> {
        let conn = self.conn.lock().await;

        let (filter_clause, filter_params) = Self::build_filter_clause(filter, 2);
        let sql = format!(
            "SELECT COUNT(*) FROM activity_events WHERE ideation_session_id = ?1{}",
            filter_clause
        );

        let mut params: Vec<String> = vec![session_id.as_str().to_string()];
        params.extend(filter_params);

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| AppError::Database(e.to_string()))?;

        let params_refs: Vec<&dyn rusqlite::ToSql> = params
            .iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();

        let count: i64 = stmt
            .query_row(params_refs.as_slice(), |row| row.get(0))
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{ActivityEventType, InternalStatus};
    use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

    fn setup_test_db() -> Connection {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Create project, task, and session for foreign key references
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'Test', '/path')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('t1', 'p1', 'feature', 'Task')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, title) VALUES ('s1', 'p1', 'Session')",
            [],
        )
        .unwrap();

        conn
    }

    #[tokio::test]
    async fn test_save_and_get_by_id() {
        let conn = setup_test_db();
        let repo = SqliteActivityEventRepository::new(conn);

        let task_id = TaskId::from_string("t1".to_string());
        let event = ActivityEvent::new_task_event(task_id, ActivityEventType::Thinking, "test")
            .with_status(InternalStatus::Executing);

        let saved = repo.save(event.clone()).await.unwrap();
        assert_eq!(saved.id, event.id);

        let found = repo.get_by_id(&event.id).await.unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id, event.id);
        assert_eq!(found.event_type, ActivityEventType::Thinking);
        assert_eq!(found.internal_status, Some(InternalStatus::Executing));
    }

    #[tokio::test]
    async fn test_list_by_task_id_empty() {
        let conn = setup_test_db();
        let repo = SqliteActivityEventRepository::new(conn);

        let task_id = TaskId::from_string("t1".to_string());

        let page = repo
            .list_by_task_id(&task_id, None, 50, None)
            .await
            .unwrap();
        assert!(page.events.is_empty());
        assert!(!page.has_more);
        assert!(page.cursor.is_none());
    }

    #[tokio::test]
    async fn test_list_by_task_id_pagination() {
        let conn = setup_test_db();
        let repo = SqliteActivityEventRepository::new(conn);

        let task_id = TaskId::from_string("t1".to_string());

        // Create 5 events with small delays to ensure different timestamps
        for i in 0..5 {
            let event = ActivityEvent::new_task_event(
                task_id.clone(),
                ActivityEventType::Text,
                format!("content {}", i),
            );
            repo.save(event).await.unwrap();
            // Small delay to ensure different timestamps
            tokio::time::sleep(tokio::time::Duration::from_millis(2)).await;
        }

        // First page
        let page1 = repo
            .list_by_task_id(&task_id, None, 3, None)
            .await
            .unwrap();
        assert_eq!(page1.events.len(), 3);
        assert!(page1.has_more);
        assert!(page1.cursor.is_some());

        // Second page
        let page2 = repo
            .list_by_task_id(&task_id, page1.cursor.as_deref(), 3, None)
            .await
            .unwrap();
        assert_eq!(page2.events.len(), 2);
        assert!(!page2.has_more);
    }

    #[tokio::test]
    async fn test_list_by_session_id() {
        let conn = setup_test_db();
        let repo = SqliteActivityEventRepository::new(conn);

        let session_id = IdeationSessionId::from_string("s1".to_string());

        let event = ActivityEvent::new_session_event(
            session_id.clone(),
            ActivityEventType::ToolCall,
            "tool call",
        );
        repo.save(event).await.unwrap();

        let page = repo
            .list_by_session_id(&session_id, None, 50, None)
            .await
            .unwrap();
        assert_eq!(page.events.len(), 1);
        assert_eq!(page.events[0].event_type, ActivityEventType::ToolCall);
    }

    #[tokio::test]
    async fn test_list_with_filter() {
        let conn = setup_test_db();
        let repo = SqliteActivityEventRepository::new(conn);

        let task_id = TaskId::from_string("t1".to_string());

        // Create different event types
        repo.save(ActivityEvent::new_task_event(
            task_id.clone(),
            ActivityEventType::Thinking,
            "thinking",
        ))
        .await
        .unwrap();
        repo.save(ActivityEvent::new_task_event(
            task_id.clone(),
            ActivityEventType::Text,
            "text",
        ))
        .await
        .unwrap();
        repo.save(ActivityEvent::new_task_event(
            task_id.clone(),
            ActivityEventType::ToolCall,
            "tool",
        ))
        .await
        .unwrap();

        // Filter by event type
        let filter =
            ActivityEventFilter::new().with_event_types(vec![ActivityEventType::Thinking]);
        let page = repo
            .list_by_task_id(&task_id, None, 50, Some(&filter))
            .await
            .unwrap();
        assert_eq!(page.events.len(), 1);
        assert_eq!(page.events[0].event_type, ActivityEventType::Thinking);
    }

    #[tokio::test]
    async fn test_delete_by_task_id() {
        let conn = setup_test_db();
        let repo = SqliteActivityEventRepository::new(conn);

        let task_id = TaskId::from_string("t1".to_string());

        repo.save(ActivityEvent::new_task_event(
            task_id.clone(),
            ActivityEventType::Text,
            "test",
        ))
        .await
        .unwrap();
        repo.save(ActivityEvent::new_task_event(
            task_id.clone(),
            ActivityEventType::Text,
            "test2",
        ))
        .await
        .unwrap();

        repo.delete_by_task_id(&task_id).await.unwrap();

        let page = repo
            .list_by_task_id(&task_id, None, 50, None)
            .await
            .unwrap();
        assert!(page.events.is_empty());
    }

    #[tokio::test]
    async fn test_delete_by_session_id() {
        let conn = setup_test_db();
        let repo = SqliteActivityEventRepository::new(conn);

        let session_id = IdeationSessionId::from_string("s1".to_string());

        repo.save(ActivityEvent::new_session_event(
            session_id.clone(),
            ActivityEventType::Text,
            "test",
        ))
        .await
        .unwrap();

        repo.delete_by_session_id(&session_id).await.unwrap();

        let page = repo
            .list_by_session_id(&session_id, None, 50, None)
            .await
            .unwrap();
        assert!(page.events.is_empty());
    }

    #[tokio::test]
    async fn test_count_by_task_id() {
        let conn = setup_test_db();
        let repo = SqliteActivityEventRepository::new(conn);

        let task_id = TaskId::from_string("t1".to_string());

        for _ in 0..3 {
            repo.save(ActivityEvent::new_task_event(
                task_id.clone(),
                ActivityEventType::Text,
                "test",
            ))
            .await
            .unwrap();
        }

        let count = repo.count_by_task_id(&task_id, None).await.unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_count_by_session_id() {
        let conn = setup_test_db();
        let repo = SqliteActivityEventRepository::new(conn);

        let session_id = IdeationSessionId::from_string("s1".to_string());

        for _ in 0..2 {
            repo.save(ActivityEvent::new_session_event(
                session_id.clone(),
                ActivityEventType::Text,
                "test",
            ))
            .await
            .unwrap();
        }

        let count = repo.count_by_session_id(&session_id, None).await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_count_with_filter() {
        let conn = setup_test_db();
        let repo = SqliteActivityEventRepository::new(conn);

        let task_id = TaskId::from_string("t1".to_string());

        repo.save(ActivityEvent::new_task_event(
            task_id.clone(),
            ActivityEventType::Thinking,
            "thinking",
        ))
        .await
        .unwrap();
        repo.save(ActivityEvent::new_task_event(
            task_id.clone(),
            ActivityEventType::Text,
            "text",
        ))
        .await
        .unwrap();

        let filter =
            ActivityEventFilter::new().with_event_types(vec![ActivityEventType::Thinking]);
        let count = repo
            .count_by_task_id(&task_id, Some(&filter))
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_parse_cursor() {
        // Valid cursor
        let cursor = "2026-01-31T10:30:45+00:00|abc123";
        let result = SqliteActivityEventRepository::parse_cursor(cursor);
        assert!(result.is_some());
        let (ts, id) = result.unwrap();
        assert_eq!(ts, "2026-01-31T10:30:45+00:00");
        assert_eq!(id, "abc123");

        // Invalid cursor (no pipe)
        let cursor = "2026-01-31T10:30:45+00:00:abc123";
        let result = SqliteActivityEventRepository::parse_cursor(cursor);
        assert!(result.is_none());
    }

    #[test]
    fn test_format_cursor() {
        let task_id = TaskId::from_string("t1".to_string());
        let event = ActivityEvent::new_task_event(task_id, ActivityEventType::Text, "test");
        let cursor = SqliteActivityEventRepository::format_cursor(&event);

        // Cursor should contain pipe separator
        assert!(cursor.contains('|'));

        // Should be parseable
        let parsed = SqliteActivityEventRepository::parse_cursor(&cursor);
        assert!(parsed.is_some());
        let (ts, id) = parsed.unwrap();
        assert_eq!(id, event.id.as_str());
        assert!(ts.contains("T")); // ISO timestamp
    }
}
