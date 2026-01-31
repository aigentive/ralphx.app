// Tauri commands for Activity Event operations
// Provides paginated access to persistent activity events for tasks and ideation sessions

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::application::AppState;
use crate::domain::entities::{
    ActivityEvent, ActivityEventRole, ActivityEventType, IdeationSessionId, InternalStatus, TaskId,
};
use crate::domain::repositories::ActivityEventFilter;

// ============================================================================
// Response types
// ============================================================================

/// Response wrapper for a single activity event (frontend-friendly)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEventResponse {
    pub id: String,
    pub task_id: Option<String>,
    pub ideation_session_id: Option<String>,
    pub internal_status: Option<String>,
    pub event_type: String,
    pub role: String,
    pub content: String,
    pub metadata: Option<String>,
    pub created_at: String,
}

impl From<ActivityEvent> for ActivityEventResponse {
    fn from(event: ActivityEvent) -> Self {
        Self {
            id: event.id.to_string(),
            task_id: event.task_id.map(|id| id.as_str().to_string()),
            ideation_session_id: event.ideation_session_id.map(|id| id.as_str().to_string()),
            internal_status: event.internal_status.map(|s| s.to_string()),
            event_type: event.event_type.to_string(),
            role: event.role.to_string(),
            content: event.content,
            metadata: event.metadata,
            created_at: event.created_at.to_rfc3339(),
        }
    }
}

/// Paginated response for activity events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEventPageResponse {
    pub events: Vec<ActivityEventResponse>,
    pub cursor: Option<String>,
    pub has_more: bool,
}

// ============================================================================
// Input types
// ============================================================================

/// Filter input for activity event queries
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActivityEventFilterInput {
    /// Filter by event type(s) - ["thinking", "tool_call", "tool_result", "text", "error"]
    pub event_types: Option<Vec<String>>,
    /// Filter by role(s) - ["agent", "system", "user"]
    pub roles: Option<Vec<String>>,
    /// Filter by internal status(es) - ["backlog", "executing", etc.]
    pub statuses: Option<Vec<String>>,
}

impl ActivityEventFilterInput {
    /// Convert frontend filter input to domain filter
    fn to_domain_filter(&self) -> ActivityEventFilter {
        let mut filter = ActivityEventFilter::new();

        if let Some(ref types) = self.event_types {
            let parsed: Vec<ActivityEventType> = types
                .iter()
                .filter_map(|t| t.parse().ok())
                .collect();
            if !parsed.is_empty() {
                filter = filter.with_event_types(parsed);
            }
        }

        if let Some(ref roles) = self.roles {
            let parsed: Vec<ActivityEventRole> = roles
                .iter()
                .filter_map(|r| r.parse().ok())
                .collect();
            if !parsed.is_empty() {
                filter = filter.with_roles(parsed);
            }
        }

        if let Some(ref statuses) = self.statuses {
            let parsed: Vec<InternalStatus> = statuses
                .iter()
                .filter_map(|s| s.parse().ok())
                .collect();
            if !parsed.is_empty() {
                filter = filter.with_statuses(parsed);
            }
        }

        filter
    }
}

// ============================================================================
// Commands
// ============================================================================

/// List activity events for a task with cursor-based pagination
///
/// # Arguments
/// * `task_id` - The task ID to get events for
/// * `cursor` - Optional cursor from previous page (format: "timestamp|id")
/// * `limit` - Maximum number of events to return (default: 50, max: 100)
/// * `filter` - Optional filter criteria
///
/// # Returns
/// A page of events ordered by created_at DESC (newest first)
#[tauri::command]
pub async fn list_task_activity_events(
    task_id: String,
    cursor: Option<String>,
    limit: Option<u32>,
    filter: Option<ActivityEventFilterInput>,
    state: State<'_, AppState>,
) -> Result<ActivityEventPageResponse, String> {
    let task_id = TaskId::from_string(task_id);
    let limit = limit.unwrap_or(50).min(100);
    let domain_filter = filter.map(|f| f.to_domain_filter());

    let page = state
        .activity_event_repo
        .list_by_task_id(
            &task_id,
            cursor.as_deref(),
            limit,
            domain_filter.as_ref(),
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(ActivityEventPageResponse {
        events: page.events.into_iter().map(ActivityEventResponse::from).collect(),
        cursor: page.cursor,
        has_more: page.has_more,
    })
}

/// List activity events for an ideation session with cursor-based pagination
///
/// # Arguments
/// * `session_id` - The ideation session ID to get events for
/// * `cursor` - Optional cursor from previous page (format: "timestamp|id")
/// * `limit` - Maximum number of events to return (default: 50, max: 100)
/// * `filter` - Optional filter criteria
///
/// # Returns
/// A page of events ordered by created_at DESC (newest first)
#[tauri::command]
pub async fn list_session_activity_events(
    session_id: String,
    cursor: Option<String>,
    limit: Option<u32>,
    filter: Option<ActivityEventFilterInput>,
    state: State<'_, AppState>,
) -> Result<ActivityEventPageResponse, String> {
    let session_id = IdeationSessionId::from_string(session_id);
    let limit = limit.unwrap_or(50).min(100);
    let domain_filter = filter.map(|f| f.to_domain_filter());

    let page = state
        .activity_event_repo
        .list_by_session_id(
            &session_id,
            cursor.as_deref(),
            limit,
            domain_filter.as_ref(),
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(ActivityEventPageResponse {
        events: page.events.into_iter().map(ActivityEventResponse::from).collect(),
        cursor: page.cursor,
        has_more: page.has_more,
    })
}

/// Count activity events for a task
#[tauri::command]
pub async fn count_task_activity_events(
    task_id: String,
    filter: Option<ActivityEventFilterInput>,
    state: State<'_, AppState>,
) -> Result<u64, String> {
    let task_id = TaskId::from_string(task_id);
    let domain_filter = filter.map(|f| f.to_domain_filter());

    state
        .activity_event_repo
        .count_by_task_id(&task_id, domain_filter.as_ref())
        .await
        .map_err(|e| e.to_string())
}

/// Count activity events for an ideation session
#[tauri::command]
pub async fn count_session_activity_events(
    session_id: String,
    filter: Option<ActivityEventFilterInput>,
    state: State<'_, AppState>,
) -> Result<u64, String> {
    let session_id = IdeationSessionId::from_string(session_id);
    let domain_filter = filter.map(|f| f.to_domain_filter());

    state
        .activity_event_repo
        .count_by_session_id(&session_id, domain_filter.as_ref())
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_event_filter_input_to_domain_empty() {
        let input = ActivityEventFilterInput::default();
        let filter = input.to_domain_filter();
        assert!(filter.is_empty());
    }

    #[test]
    fn activity_event_filter_input_to_domain_with_event_types() {
        let input = ActivityEventFilterInput {
            event_types: Some(vec!["thinking".to_string(), "text".to_string()]),
            roles: None,
            statuses: None,
        };
        let filter = input.to_domain_filter();
        assert!(!filter.is_empty());
        assert!(filter.event_types.is_some());
        assert_eq!(filter.event_types.unwrap().len(), 2);
    }

    #[test]
    fn activity_event_filter_input_to_domain_with_roles() {
        let input = ActivityEventFilterInput {
            event_types: None,
            roles: Some(vec!["agent".to_string()]),
            statuses: None,
        };
        let filter = input.to_domain_filter();
        assert!(!filter.is_empty());
        assert!(filter.roles.is_some());
    }

    #[test]
    fn activity_event_filter_input_to_domain_with_statuses() {
        let input = ActivityEventFilterInput {
            event_types: None,
            roles: None,
            statuses: Some(vec!["executing".to_string()]),
        };
        let filter = input.to_domain_filter();
        assert!(!filter.is_empty());
        assert!(filter.statuses.is_some());
    }

    #[test]
    fn activity_event_filter_input_to_domain_ignores_invalid() {
        let input = ActivityEventFilterInput {
            event_types: Some(vec!["invalid_type".to_string()]),
            roles: Some(vec!["invalid_role".to_string()]),
            statuses: Some(vec!["invalid_status".to_string()]),
        };
        let filter = input.to_domain_filter();
        // Invalid values are filtered out, leaving an empty filter
        assert!(filter.is_empty());
    }
}
