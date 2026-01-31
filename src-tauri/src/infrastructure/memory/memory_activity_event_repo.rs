// In-memory implementation of ActivityEventRepository for testing
// Uses HashMap with RwLock for thread-safe access

use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::RwLock;

use crate::domain::entities::{ActivityEvent, ActivityEventId, IdeationSessionId, TaskId};
use crate::domain::repositories::{
    ActivityEventFilter, ActivityEventPage, ActivityEventRepository,
};
use crate::error::AppResult;

/// Maximum allowed limit for pagination
const MAX_LIMIT: u32 = 100;

/// In-memory implementation of ActivityEventRepository for testing
pub struct MemoryActivityEventRepository {
    events: RwLock<HashMap<ActivityEventId, ActivityEvent>>,
}

impl MemoryActivityEventRepository {
    pub fn new() -> Self {
        Self {
            events: RwLock::new(HashMap::new()),
        }
    }

    /// Check if an event matches the filter criteria (excluding task_id/session_id)
    fn matches_filter(event: &ActivityEvent, filter: Option<&ActivityEventFilter>) -> bool {
        let Some(f) = filter else {
            return true;
        };

        // Check event types filter
        if let Some(types) = &f.event_types {
            if !types.is_empty() && !types.contains(&event.event_type) {
                return false;
            }
        }

        // Check roles filter
        if let Some(roles) = &f.roles {
            if !roles.is_empty() && !roles.contains(&event.role) {
                return false;
            }
        }

        // Check statuses filter
        if let Some(statuses) = &f.statuses {
            if !statuses.is_empty() {
                match &event.internal_status {
                    Some(status) if statuses.contains(status) => {}
                    _ => return false,
                }
            }
        }

        true
    }

    /// Check if an event matches the full filter criteria (including task_id/session_id)
    fn matches_full_filter(event: &ActivityEvent, filter: Option<&ActivityEventFilter>) -> bool {
        let Some(f) = filter else {
            return true;
        };

        // Check task_id filter
        if let Some(filter_task_id) = &f.task_id {
            match &event.task_id {
                Some(event_task_id) if event_task_id == filter_task_id => {}
                _ => return false,
            }
        }

        // Check session_id filter
        if let Some(filter_session_id) = &f.session_id {
            match &event.ideation_session_id {
                Some(event_session_id) if event_session_id == filter_session_id => {}
                _ => return false,
            }
        }

        // Check remaining filters (event_types, roles, statuses)
        Self::matches_filter(event, filter)
    }

    /// Format a cursor from an event (timestamp|id format)
    fn format_cursor(event: &ActivityEvent) -> String {
        format!("{}|{}", event.created_at.to_rfc3339(), event.id)
    }

    /// Parse a cursor string into (timestamp, id) tuple
    fn parse_cursor(cursor: &str) -> Option<(String, String)> {
        cursor
            .split_once('|')
            .map(|(ts, id)| (ts.to_string(), id.to_string()))
    }
}

impl Default for MemoryActivityEventRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ActivityEventRepository for MemoryActivityEventRepository {
    async fn save(&self, event: ActivityEvent) -> AppResult<ActivityEvent> {
        let mut events = self.events.write().await;
        events.insert(event.id.clone(), event.clone());
        Ok(event)
    }

    async fn get_by_id(&self, id: &ActivityEventId) -> AppResult<Option<ActivityEvent>> {
        let events = self.events.read().await;
        Ok(events.get(id).cloned())
    }

    async fn list_by_task_id(
        &self,
        task_id: &TaskId,
        cursor: Option<&str>,
        limit: u32,
        filter: Option<&ActivityEventFilter>,
    ) -> AppResult<ActivityEventPage> {
        let events = self.events.read().await;

        // Cap limit at MAX_LIMIT
        let limit = limit.min(MAX_LIMIT);

        // Filter events by task_id and apply filter
        let mut filtered: Vec<&ActivityEvent> = events
            .values()
            .filter(|e| e.task_id.as_ref() == Some(task_id))
            .filter(|e| Self::matches_filter(e, filter))
            .collect();

        // Sort by created_at DESC, id DESC (newest first)
        filtered.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| b.id.as_str().cmp(a.id.as_str()))
        });

        // Apply cursor if provided
        if let Some(cursor_str) = cursor {
            if let Some((cursor_ts, cursor_id)) = Self::parse_cursor(cursor_str) {
                // Find position after cursor
                if let Some(pos) = filtered.iter().position(|e| {
                    let e_ts = e.created_at.to_rfc3339();
                    e_ts < cursor_ts || (e_ts == cursor_ts && e.id.as_str() < cursor_id.as_str())
                }) {
                    filtered = filtered.into_iter().skip(pos).collect();
                } else {
                    // Cursor is beyond all events
                    filtered.clear();
                }
            }
        }

        // Take limit + 1 to detect has_more
        let has_more = filtered.len() > limit as usize;
        let result_events: Vec<ActivityEvent> = filtered
            .into_iter()
            .take(limit as usize)
            .cloned()
            .collect();

        let next_cursor = if has_more {
            result_events.last().map(Self::format_cursor)
        } else {
            None
        };

        Ok(ActivityEventPage {
            events: result_events,
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
        let events = self.events.read().await;

        // Cap limit at MAX_LIMIT
        let limit = limit.min(MAX_LIMIT);

        // Filter events by session_id and apply filter
        let mut filtered: Vec<&ActivityEvent> = events
            .values()
            .filter(|e| e.ideation_session_id.as_ref() == Some(session_id))
            .filter(|e| Self::matches_filter(e, filter))
            .collect();

        // Sort by created_at DESC, id DESC (newest first)
        filtered.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| b.id.as_str().cmp(a.id.as_str()))
        });

        // Apply cursor if provided
        if let Some(cursor_str) = cursor {
            if let Some((cursor_ts, cursor_id)) = Self::parse_cursor(cursor_str) {
                // Find position after cursor
                if let Some(pos) = filtered.iter().position(|e| {
                    let e_ts = e.created_at.to_rfc3339();
                    e_ts < cursor_ts || (e_ts == cursor_ts && e.id.as_str() < cursor_id.as_str())
                }) {
                    filtered = filtered.into_iter().skip(pos).collect();
                } else {
                    // Cursor is beyond all events
                    filtered.clear();
                }
            }
        }

        // Take limit + 1 to detect has_more
        let has_more = filtered.len() > limit as usize;
        let result_events: Vec<ActivityEvent> = filtered
            .into_iter()
            .take(limit as usize)
            .cloned()
            .collect();

        let next_cursor = if has_more {
            result_events.last().map(Self::format_cursor)
        } else {
            None
        };

        Ok(ActivityEventPage {
            events: result_events,
            cursor: next_cursor,
            has_more,
        })
    }

    async fn delete_by_task_id(&self, task_id: &TaskId) -> AppResult<()> {
        let mut events = self.events.write().await;
        events.retain(|_, e| e.task_id.as_ref() != Some(task_id));
        Ok(())
    }

    async fn delete_by_session_id(&self, session_id: &IdeationSessionId) -> AppResult<()> {
        let mut events = self.events.write().await;
        events.retain(|_, e| e.ideation_session_id.as_ref() != Some(session_id));
        Ok(())
    }

    async fn count_by_task_id(
        &self,
        task_id: &TaskId,
        filter: Option<&ActivityEventFilter>,
    ) -> AppResult<u64> {
        let events = self.events.read().await;
        let count = events
            .values()
            .filter(|e| e.task_id.as_ref() == Some(task_id))
            .filter(|e| Self::matches_filter(e, filter))
            .count();
        Ok(count as u64)
    }

    async fn count_by_session_id(
        &self,
        session_id: &IdeationSessionId,
        filter: Option<&ActivityEventFilter>,
    ) -> AppResult<u64> {
        let events = self.events.read().await;
        let count = events
            .values()
            .filter(|e| e.ideation_session_id.as_ref() == Some(session_id))
            .filter(|e| Self::matches_filter(e, filter))
            .count();
        Ok(count as u64)
    }

    async fn list_all(
        &self,
        cursor: Option<&str>,
        limit: u32,
        filter: Option<&ActivityEventFilter>,
    ) -> AppResult<ActivityEventPage> {
        let events = self.events.read().await;

        // Cap limit at MAX_LIMIT
        let limit = limit.min(MAX_LIMIT);

        // Filter all events by the full filter criteria
        let mut filtered: Vec<&ActivityEvent> = events
            .values()
            .filter(|e| Self::matches_full_filter(e, filter))
            .collect();

        // Sort by created_at DESC, id DESC (newest first)
        filtered.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| b.id.as_str().cmp(a.id.as_str()))
        });

        // Apply cursor if provided
        if let Some(cursor_str) = cursor {
            if let Some((cursor_ts, cursor_id)) = Self::parse_cursor(cursor_str) {
                // Find position after cursor
                if let Some(pos) = filtered.iter().position(|e| {
                    let e_ts = e.created_at.to_rfc3339();
                    e_ts < cursor_ts || (e_ts == cursor_ts && e.id.as_str() < cursor_id.as_str())
                }) {
                    filtered = filtered.into_iter().skip(pos).collect();
                } else {
                    // Cursor is beyond all events
                    filtered.clear();
                }
            }
        }

        // Take limit + 1 to detect has_more
        let has_more = filtered.len() > limit as usize;
        let result_events: Vec<ActivityEvent> = filtered
            .into_iter()
            .take(limit as usize)
            .cloned()
            .collect();

        let next_cursor = if has_more {
            result_events.last().map(Self::format_cursor)
        } else {
            None
        };

        Ok(ActivityEventPage {
            events: result_events,
            cursor: next_cursor,
            has_more,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{ActivityEventType, InternalStatus};

    #[tokio::test]
    async fn test_save_and_get_by_id() {
        let repo = MemoryActivityEventRepository::new();
        let task_id = TaskId::new();
        let event = ActivityEvent::new_task_event(task_id, ActivityEventType::Text, "test");
        let event_id = event.id.clone();

        repo.save(event.clone()).await.unwrap();

        let found = repo.get_by_id(&event_id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, event_id);
    }

    #[tokio::test]
    async fn test_list_by_task_id_empty() {
        let repo = MemoryActivityEventRepository::new();
        let task_id = TaskId::new();

        let page = repo
            .list_by_task_id(&task_id, None, 50, None)
            .await
            .unwrap();
        assert!(page.events.is_empty());
        assert!(!page.has_more);
    }

    #[tokio::test]
    async fn test_list_by_task_id_with_events() {
        let repo = MemoryActivityEventRepository::new();
        let task_id = TaskId::new();

        // Create a few events
        for i in 0..3 {
            let event = ActivityEvent::new_task_event(
                task_id.clone(),
                ActivityEventType::Text,
                format!("content {}", i),
            );
            repo.save(event).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(2)).await;
        }

        let page = repo
            .list_by_task_id(&task_id, None, 50, None)
            .await
            .unwrap();
        assert_eq!(page.events.len(), 3);
        assert!(!page.has_more);
    }

    #[tokio::test]
    async fn test_list_by_task_id_pagination() {
        let repo = MemoryActivityEventRepository::new();
        let task_id = TaskId::new();

        // Create 5 events
        for i in 0..5 {
            let event = ActivityEvent::new_task_event(
                task_id.clone(),
                ActivityEventType::Text,
                format!("content {}", i),
            );
            repo.save(event).await.unwrap();
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
        let repo = MemoryActivityEventRepository::new();
        let session_id = IdeationSessionId::new();

        let event =
            ActivityEvent::new_session_event(session_id.clone(), ActivityEventType::ToolCall, "x");
        repo.save(event).await.unwrap();

        let page = repo
            .list_by_session_id(&session_id, None, 50, None)
            .await
            .unwrap();
        assert_eq!(page.events.len(), 1);
    }

    #[tokio::test]
    async fn test_list_with_filter() {
        let repo = MemoryActivityEventRepository::new();
        let task_id = TaskId::new();

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
        let page = repo
            .list_by_task_id(&task_id, None, 50, Some(&filter))
            .await
            .unwrap();
        assert_eq!(page.events.len(), 1);
        assert_eq!(page.events[0].event_type, ActivityEventType::Thinking);
    }

    #[tokio::test]
    async fn test_delete_by_task_id() {
        let repo = MemoryActivityEventRepository::new();
        let task_id = TaskId::new();

        repo.save(ActivityEvent::new_task_event(
            task_id.clone(),
            ActivityEventType::Text,
            "test",
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
        let repo = MemoryActivityEventRepository::new();
        let session_id = IdeationSessionId::new();

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
        let repo = MemoryActivityEventRepository::new();
        let task_id = TaskId::new();

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
        let repo = MemoryActivityEventRepository::new();
        let session_id = IdeationSessionId::new();

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
        let repo = MemoryActivityEventRepository::new();
        let task_id = TaskId::new();

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

    #[tokio::test]
    async fn test_filter_by_status() {
        let repo = MemoryActivityEventRepository::new();
        let task_id = TaskId::new();

        repo.save(
            ActivityEvent::new_task_event(task_id.clone(), ActivityEventType::Text, "executing")
                .with_status(InternalStatus::Executing),
        )
        .await
        .unwrap();
        repo.save(
            ActivityEvent::new_task_event(task_id.clone(), ActivityEventType::Text, "ready")
                .with_status(InternalStatus::Ready),
        )
        .await
        .unwrap();

        let filter = ActivityEventFilter::new().with_statuses(vec![InternalStatus::Executing]);
        let page = repo
            .list_by_task_id(&task_id, None, 50, Some(&filter))
            .await
            .unwrap();
        assert_eq!(page.events.len(), 1);
        assert_eq!(
            page.events[0].internal_status,
            Some(InternalStatus::Executing)
        );
    }

    #[tokio::test]
    async fn test_list_all_returns_all_events() {
        let repo = MemoryActivityEventRepository::new();
        let task_id = TaskId::new();
        let session_id = IdeationSessionId::new();

        // Create events for both task and session
        repo.save(ActivityEvent::new_task_event(
            task_id.clone(),
            ActivityEventType::Text,
            "task event 1",
        ))
        .await
        .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(2)).await;
        repo.save(ActivityEvent::new_session_event(
            session_id.clone(),
            ActivityEventType::Text,
            "session event 1",
        ))
        .await
        .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(2)).await;
        repo.save(ActivityEvent::new_task_event(
            task_id.clone(),
            ActivityEventType::ToolCall,
            "task event 2",
        ))
        .await
        .unwrap();

        // list_all should return all 3 events
        let page = repo.list_all(None, 50, None).await.unwrap();
        assert_eq!(page.events.len(), 3);
        assert!(!page.has_more);
    }

    #[tokio::test]
    async fn test_list_all_with_task_id_filter() {
        let repo = MemoryActivityEventRepository::new();
        let task_id = TaskId::new();
        let session_id = IdeationSessionId::new();

        // Create events for both task and session
        repo.save(ActivityEvent::new_task_event(
            task_id.clone(),
            ActivityEventType::Text,
            "task event",
        ))
        .await
        .unwrap();
        repo.save(ActivityEvent::new_session_event(
            session_id.clone(),
            ActivityEventType::Text,
            "session event",
        ))
        .await
        .unwrap();

        // Filter by task_id
        let filter = ActivityEventFilter::new().with_task_id(task_id.clone());
        let page = repo.list_all(None, 50, Some(&filter)).await.unwrap();
        assert_eq!(page.events.len(), 1);
        assert_eq!(page.events[0].task_id, Some(task_id));
    }

    #[tokio::test]
    async fn test_list_all_with_session_id_filter() {
        let repo = MemoryActivityEventRepository::new();
        let task_id = TaskId::new();
        let session_id = IdeationSessionId::new();

        // Create events for both task and session
        repo.save(ActivityEvent::new_task_event(
            task_id.clone(),
            ActivityEventType::Text,
            "task event",
        ))
        .await
        .unwrap();
        repo.save(ActivityEvent::new_session_event(
            session_id.clone(),
            ActivityEventType::Text,
            "session event",
        ))
        .await
        .unwrap();

        // Filter by session_id
        let filter = ActivityEventFilter::new().with_session_id(session_id.clone());
        let page = repo.list_all(None, 50, Some(&filter)).await.unwrap();
        assert_eq!(page.events.len(), 1);
        assert_eq!(page.events[0].ideation_session_id, Some(session_id));
    }

    #[tokio::test]
    async fn test_list_all_pagination() {
        let repo = MemoryActivityEventRepository::new();
        let task_id = TaskId::new();

        // Create 5 events with delays
        for i in 0..5 {
            let event = ActivityEvent::new_task_event(
                task_id.clone(),
                ActivityEventType::Text,
                format!("content {}", i),
            );
            repo.save(event).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(2)).await;
        }

        // First page
        let page1 = repo.list_all(None, 3, None).await.unwrap();
        assert_eq!(page1.events.len(), 3);
        assert!(page1.has_more);
        assert!(page1.cursor.is_some());

        // Second page
        let page2 = repo
            .list_all(page1.cursor.as_deref(), 3, None)
            .await
            .unwrap();
        assert_eq!(page2.events.len(), 2);
        assert!(!page2.has_more);
    }
}
