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
        let result_events: Vec<ActivityEvent> =
            filtered.into_iter().take(limit as usize).cloned().collect();

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
        let result_events: Vec<ActivityEvent> =
            filtered.into_iter().take(limit as usize).cloned().collect();

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
        let result_events: Vec<ActivityEvent> =
            filtered.into_iter().take(limit as usize).cloned().collect();

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
#[path = "memory_activity_event_repo_tests.rs"]
mod tests;
