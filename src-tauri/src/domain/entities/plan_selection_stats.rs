// Plan selection stats entity for tracking plan selection interactions
// Used for ranking and analytics in plan selector UI

use chrono::{DateTime, Utc};
use rusqlite::Row;

use crate::domain::entities::{IdeationSessionId, ProjectId};
use crate::error::AppError;

/// Tracks user interactions with plan selection (for ranking and analytics)
#[derive(Debug, Clone)]
pub struct PlanSelectionStats {
    /// Project that owns this stats entry
    pub project_id: ProjectId,
    /// Session being tracked
    pub ideation_session_id: IdeationSessionId,
    /// Number of times this plan has been selected
    pub selected_count: u32,
    /// Timestamp of last selection (None if never selected)
    pub last_selected_at: Option<DateTime<Utc>>,
    /// Source of last selection (e.g., "kanban_inline", "graph_inline", "quick_switcher", "ideation")
    pub last_selected_source: Option<String>,
}

impl PlanSelectionStats {
    /// Create a new PlanSelectionStats instance
    pub fn new(project_id: ProjectId, ideation_session_id: IdeationSessionId) -> Self {
        Self {
            project_id,
            ideation_session_id,
            selected_count: 0,
            last_selected_at: None,
            last_selected_source: None,
        }
    }

    /// Parse from SQLite row
    pub fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        let last_selected_at_str: Option<String> = row.get(3)?;
        let last_selected_at = last_selected_at_str
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        Ok(Self {
            project_id: ProjectId::from_string(row.get::<_, String>(0)?),
            ideation_session_id: IdeationSessionId::from_string(row.get::<_, String>(1)?),
            selected_count: row.get::<_, i64>(2)? as u32,
            last_selected_at,
            last_selected_source: row.get(4)?,
        })
    }
}

/// Selection source label for analytics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionSource {
    /// Selected from Kanban inline selector
    KanbanInline,
    /// Selected from Graph inline selector
    GraphInline,
    /// Selected from quick switcher (Cmd+Shift+P)
    QuickSwitcher,
    /// Auto-set when accepting session in Ideation view
    Ideation,
}

impl SelectionSource {
    /// Convert to database string
    pub fn to_db_string(&self) -> &'static str {
        match self {
            SelectionSource::KanbanInline => "kanban_inline",
            SelectionSource::GraphInline => "graph_inline",
            SelectionSource::QuickSwitcher => "quick_switcher",
            SelectionSource::Ideation => "ideation",
        }
    }

    /// Parse from database string
    pub fn from_db_string(s: &str) -> Result<Self, AppError> {
        match s {
            "kanban_inline" => Ok(SelectionSource::KanbanInline),
            "graph_inline" => Ok(SelectionSource::GraphInline),
            "quick_switcher" => Ok(SelectionSource::QuickSwitcher),
            "ideation" => Ok(SelectionSource::Ideation),
            _ => Err(AppError::Validation(format!(
                "Invalid selection source: {}",
                s
            ))),
        }
    }
}
