// Ideation system entities - IdeationSession and related types
// These represent brainstorming sessions that produce task proposals

use chrono::{DateTime, TimeZone, Utc};
use rusqlite::Row;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use super::{IdeationSessionId, ProjectId};

/// Status of an ideation session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdeationSessionStatus {
    /// Session is currently being worked on
    Active,
    /// Session has been archived (completed or paused for later)
    Archived,
    /// All proposals from this session have been applied to Kanban
    Converted,
}

impl Default for IdeationSessionStatus {
    fn default() -> Self {
        Self::Active
    }
}

impl std::fmt::Display for IdeationSessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdeationSessionStatus::Active => write!(f, "active"),
            IdeationSessionStatus::Archived => write!(f, "archived"),
            IdeationSessionStatus::Converted => write!(f, "converted"),
        }
    }
}

/// Error type for parsing IdeationSessionStatus from a string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseIdeationSessionStatusError {
    pub value: String,
}

impl std::fmt::Display for ParseIdeationSessionStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown ideation session status: '{}'", self.value)
    }
}

impl std::error::Error for ParseIdeationSessionStatusError {}

impl FromStr for IdeationSessionStatus {
    type Err = ParseIdeationSessionStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(IdeationSessionStatus::Active),
            "archived" => Ok(IdeationSessionStatus::Archived),
            "converted" => Ok(IdeationSessionStatus::Converted),
            _ => Err(ParseIdeationSessionStatusError {
                value: s.to_string(),
            }),
        }
    }
}

/// An ideation session - a brainstorming conversation that produces task proposals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeationSession {
    /// Unique identifier for this session
    pub id: IdeationSessionId,
    /// Project this session belongs to
    pub project_id: ProjectId,
    /// Human-readable title (auto-generated or user-defined)
    pub title: Option<String>,
    /// Current status of the session
    pub status: IdeationSessionStatus,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// When the session was last updated
    pub updated_at: DateTime<Utc>,
    /// When the session was archived (if applicable)
    pub archived_at: Option<DateTime<Utc>>,
    /// When all proposals were converted to tasks (if applicable)
    pub converted_at: Option<DateTime<Utc>>,
}

/// Builder for creating IdeationSession instances
#[derive(Debug, Default)]
pub struct IdeationSessionBuilder {
    id: Option<IdeationSessionId>,
    project_id: Option<ProjectId>,
    title: Option<String>,
    status: Option<IdeationSessionStatus>,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
    archived_at: Option<DateTime<Utc>>,
    converted_at: Option<DateTime<Utc>>,
}

impl IdeationSessionBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the session ID
    pub fn id(mut self, id: IdeationSessionId) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the project ID
    pub fn project_id(mut self, project_id: ProjectId) -> Self {
        self.project_id = Some(project_id);
        self
    }

    /// Set the title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the status
    pub fn status(mut self, status: IdeationSessionStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Set the created_at timestamp
    pub fn created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = Some(created_at);
        self
    }

    /// Set the updated_at timestamp
    pub fn updated_at(mut self, updated_at: DateTime<Utc>) -> Self {
        self.updated_at = Some(updated_at);
        self
    }

    /// Set the archived_at timestamp
    pub fn archived_at(mut self, archived_at: DateTime<Utc>) -> Self {
        self.archived_at = Some(archived_at);
        self
    }

    /// Set the converted_at timestamp
    pub fn converted_at(mut self, converted_at: DateTime<Utc>) -> Self {
        self.converted_at = Some(converted_at);
        self
    }

    /// Build the IdeationSession
    /// Panics if project_id is not set
    pub fn build(self) -> IdeationSession {
        let now = Utc::now();
        IdeationSession {
            id: self.id.unwrap_or_else(IdeationSessionId::new),
            project_id: self.project_id.expect("project_id is required"),
            title: self.title,
            status: self.status.unwrap_or_default(),
            created_at: self.created_at.unwrap_or(now),
            updated_at: self.updated_at.unwrap_or(now),
            archived_at: self.archived_at,
            converted_at: self.converted_at,
        }
    }
}

impl IdeationSession {
    /// Creates a new active session for a project
    pub fn new(project_id: ProjectId) -> Self {
        IdeationSessionBuilder::new()
            .project_id(project_id)
            .build()
    }

    /// Creates a new active session with a title
    pub fn new_with_title(project_id: ProjectId, title: impl Into<String>) -> Self {
        IdeationSessionBuilder::new()
            .project_id(project_id)
            .title(title)
            .build()
    }

    /// Creates a builder for more complex session creation
    pub fn builder() -> IdeationSessionBuilder {
        IdeationSessionBuilder::new()
    }

    /// Returns true if the session is active
    pub fn is_active(&self) -> bool {
        self.status == IdeationSessionStatus::Active
    }

    /// Returns true if the session has been archived
    pub fn is_archived(&self) -> bool {
        self.status == IdeationSessionStatus::Archived
    }

    /// Returns true if all proposals have been converted
    pub fn is_converted(&self) -> bool {
        self.status == IdeationSessionStatus::Converted
    }

    /// Archives the session
    pub fn archive(&mut self) {
        let now = Utc::now();
        self.status = IdeationSessionStatus::Archived;
        self.archived_at = Some(now);
        self.updated_at = now;
    }

    /// Marks the session as converted (all proposals applied)
    pub fn mark_converted(&mut self) {
        let now = Utc::now();
        self.status = IdeationSessionStatus::Converted;
        self.converted_at = Some(now);
        self.updated_at = now;
    }

    /// Updates the updated_at timestamp to now
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    /// Deserialize an IdeationSession from a SQLite row
    /// Expects columns: id, project_id, title, status, created_at, updated_at, archived_at, converted_at
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: IdeationSessionId::from_string(row.get::<_, String>("id")?),
            project_id: ProjectId::from_string(row.get::<_, String>("project_id")?),
            title: row.get("title")?,
            status: row
                .get::<_, String>("status")?
                .parse()
                .unwrap_or(IdeationSessionStatus::Active),
            created_at: Self::parse_datetime(row.get("created_at")?),
            updated_at: Self::parse_datetime(row.get("updated_at")?),
            archived_at: row
                .get::<_, Option<String>>("archived_at")?
                .map(Self::parse_datetime),
            converted_at: row
                .get::<_, Option<String>>("converted_at")?
                .map(Self::parse_datetime),
        })
    }

    /// Parse a datetime string from SQLite into a DateTime<Utc>
    /// Handles both RFC3339 format and SQLite's CURRENT_TIMESTAMP format
    fn parse_datetime(s: String) -> DateTime<Utc> {
        // Try RFC3339 first (our preferred format)
        if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
            return dt.with_timezone(&Utc);
        }
        // Try SQLite's default datetime format (YYYY-MM-DD HH:MM:SS)
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S") {
            return Utc.from_utc_datetime(&dt);
        }
        // Fallback to now if parsing fails
        Utc::now()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== IdeationSessionStatus Tests =====

    #[test]
    fn status_default_is_active() {
        assert_eq!(IdeationSessionStatus::default(), IdeationSessionStatus::Active);
    }

    #[test]
    fn status_display_active() {
        assert_eq!(format!("{}", IdeationSessionStatus::Active), "active");
    }

    #[test]
    fn status_display_archived() {
        assert_eq!(format!("{}", IdeationSessionStatus::Archived), "archived");
    }

    #[test]
    fn status_display_converted() {
        assert_eq!(format!("{}", IdeationSessionStatus::Converted), "converted");
    }

    #[test]
    fn status_serializes_to_snake_case() {
        let active_json = serde_json::to_string(&IdeationSessionStatus::Active).expect("Should serialize");
        let archived_json = serde_json::to_string(&IdeationSessionStatus::Archived).expect("Should serialize");
        let converted_json = serde_json::to_string(&IdeationSessionStatus::Converted).expect("Should serialize");

        assert_eq!(active_json, "\"active\"");
        assert_eq!(archived_json, "\"archived\"");
        assert_eq!(converted_json, "\"converted\"");
    }

    #[test]
    fn status_deserializes_from_snake_case() {
        let active: IdeationSessionStatus = serde_json::from_str("\"active\"").expect("Should deserialize");
        let archived: IdeationSessionStatus = serde_json::from_str("\"archived\"").expect("Should deserialize");
        let converted: IdeationSessionStatus = serde_json::from_str("\"converted\"").expect("Should deserialize");

        assert_eq!(active, IdeationSessionStatus::Active);
        assert_eq!(archived, IdeationSessionStatus::Archived);
        assert_eq!(converted, IdeationSessionStatus::Converted);
    }

    #[test]
    fn status_from_str_active() {
        let status: IdeationSessionStatus = "active".parse().unwrap();
        assert_eq!(status, IdeationSessionStatus::Active);
    }

    #[test]
    fn status_from_str_archived() {
        let status: IdeationSessionStatus = "archived".parse().unwrap();
        assert_eq!(status, IdeationSessionStatus::Archived);
    }

    #[test]
    fn status_from_str_converted() {
        let status: IdeationSessionStatus = "converted".parse().unwrap();
        assert_eq!(status, IdeationSessionStatus::Converted);
    }

    #[test]
    fn status_from_str_invalid() {
        let result: Result<IdeationSessionStatus, _> = "invalid".parse();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.value, "invalid");
    }

    #[test]
    fn status_parse_error_displays_correctly() {
        let err = ParseIdeationSessionStatusError {
            value: "unknown".to_string(),
        };
        assert_eq!(err.to_string(), "unknown ideation session status: 'unknown'");
    }

    #[test]
    fn status_clone_works() {
        let status = IdeationSessionStatus::Archived;
        let cloned = status;
        assert_eq!(status, cloned);
    }

    #[test]
    fn status_equality_works() {
        assert_eq!(IdeationSessionStatus::Active, IdeationSessionStatus::Active);
        assert_eq!(IdeationSessionStatus::Archived, IdeationSessionStatus::Archived);
        assert_eq!(IdeationSessionStatus::Converted, IdeationSessionStatus::Converted);
        assert_ne!(IdeationSessionStatus::Active, IdeationSessionStatus::Archived);
        assert_ne!(IdeationSessionStatus::Active, IdeationSessionStatus::Converted);
        assert_ne!(IdeationSessionStatus::Archived, IdeationSessionStatus::Converted);
    }

    // ===== IdeationSession Creation Tests =====

    #[test]
    fn session_new_creates_with_defaults() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());

        assert_eq!(session.project_id, project_id);
        assert_eq!(session.status, IdeationSessionStatus::Active);
        assert!(session.title.is_none());
        assert!(session.archived_at.is_none());
        assert!(session.converted_at.is_none());
    }

    #[test]
    fn session_new_generates_unique_id() {
        let project_id = ProjectId::new();
        let session1 = IdeationSession::new(project_id.clone());
        let session2 = IdeationSession::new(project_id);

        assert_ne!(session1.id, session2.id);
    }

    #[test]
    fn session_new_sets_timestamps() {
        let before = Utc::now();
        let session = IdeationSession::new(ProjectId::new());
        let after = Utc::now();

        assert!(session.created_at >= before);
        assert!(session.created_at <= after);
        assert!(session.updated_at >= before);
        assert!(session.updated_at <= after);
        assert_eq!(session.created_at, session.updated_at);
    }

    #[test]
    fn session_new_with_title() {
        let session = IdeationSession::new_with_title(ProjectId::new(), "Auth Feature");

        assert_eq!(session.title, Some("Auth Feature".to_string()));
        assert_eq!(session.status, IdeationSessionStatus::Active);
    }

    // ===== Builder Tests =====

    #[test]
    fn builder_creates_session_with_all_fields() {
        let project_id = ProjectId::new();
        let created = Utc::now();

        let session = IdeationSession::builder()
            .project_id(project_id.clone())
            .title("Custom Session")
            .status(IdeationSessionStatus::Active)
            .created_at(created)
            .updated_at(created)
            .build();

        assert_eq!(session.project_id, project_id);
        assert_eq!(session.title, Some("Custom Session".to_string()));
        assert_eq!(session.status, IdeationSessionStatus::Active);
        assert_eq!(session.created_at, created);
    }

    #[test]
    fn builder_uses_defaults_for_optional_fields() {
        let session = IdeationSession::builder()
            .project_id(ProjectId::new())
            .build();

        assert!(session.title.is_none());
        assert_eq!(session.status, IdeationSessionStatus::Active);
        assert!(session.archived_at.is_none());
        assert!(session.converted_at.is_none());
    }

    #[test]
    fn builder_generates_id_if_not_provided() {
        let session = IdeationSession::builder()
            .project_id(ProjectId::new())
            .build();

        assert!(uuid::Uuid::parse_str(session.id.as_str()).is_ok());
    }

    #[test]
    fn builder_uses_provided_id() {
        let id = IdeationSessionId::from_string("custom-id");
        let session = IdeationSession::builder()
            .id(id.clone())
            .project_id(ProjectId::new())
            .build();

        assert_eq!(session.id, id);
    }

    #[test]
    #[should_panic(expected = "project_id is required")]
    fn builder_panics_without_project_id() {
        IdeationSession::builder().build();
    }

    // ===== Session Method Tests =====

    #[test]
    fn session_is_active_returns_true_for_active() {
        let session = IdeationSession::new(ProjectId::new());
        assert!(session.is_active());
    }

    #[test]
    fn session_is_active_returns_false_for_other_statuses() {
        let mut session = IdeationSession::new(ProjectId::new());
        session.archive();
        assert!(!session.is_active());
    }

    #[test]
    fn session_is_archived_returns_true_for_archived() {
        let mut session = IdeationSession::new(ProjectId::new());
        session.archive();
        assert!(session.is_archived());
    }

    #[test]
    fn session_is_converted_returns_true_for_converted() {
        let mut session = IdeationSession::new(ProjectId::new());
        session.mark_converted();
        assert!(session.is_converted());
    }

    #[test]
    fn session_archive_sets_status_and_timestamp() {
        let mut session = IdeationSession::new(ProjectId::new());
        let before = Utc::now();

        session.archive();

        assert_eq!(session.status, IdeationSessionStatus::Archived);
        assert!(session.archived_at.is_some());
        assert!(session.archived_at.unwrap() >= before);
        assert!(session.updated_at >= before);
    }

    #[test]
    fn session_mark_converted_sets_status_and_timestamp() {
        let mut session = IdeationSession::new(ProjectId::new());
        let before = Utc::now();

        session.mark_converted();

        assert_eq!(session.status, IdeationSessionStatus::Converted);
        assert!(session.converted_at.is_some());
        assert!(session.converted_at.unwrap() >= before);
        assert!(session.updated_at >= before);
    }

    #[test]
    fn session_touch_updates_timestamp() {
        let mut session = IdeationSession::new(ProjectId::new());
        let original_updated = session.updated_at;
        let original_created = session.created_at;

        std::thread::sleep(std::time::Duration::from_millis(10));

        session.touch();

        assert_eq!(session.created_at, original_created);
        assert!(session.updated_at > original_updated);
    }

    // ===== Serialization Tests =====

    #[test]
    fn session_serializes_to_json() {
        let session = IdeationSession::new_with_title(ProjectId::new(), "JSON Test");
        let json = serde_json::to_string(&session).expect("Should serialize");

        assert!(json.contains("\"title\":\"JSON Test\""));
        assert!(json.contains("\"status\":\"active\""));
    }

    #[test]
    fn session_deserializes_from_json() {
        let json = r#"{
            "id": "session-123",
            "project_id": "proj-456",
            "title": "Deserialized",
            "status": "archived",
            "created_at": "2026-01-24T12:00:00Z",
            "updated_at": "2026-01-24T13:00:00Z",
            "archived_at": "2026-01-24T13:00:00Z",
            "converted_at": null
        }"#;

        let session: IdeationSession = serde_json::from_str(json).expect("Should deserialize");

        assert_eq!(session.id.as_str(), "session-123");
        assert_eq!(session.project_id.as_str(), "proj-456");
        assert_eq!(session.title, Some("Deserialized".to_string()));
        assert_eq!(session.status, IdeationSessionStatus::Archived);
        assert!(session.archived_at.is_some());
        assert!(session.converted_at.is_none());
    }

    #[test]
    fn session_deserializes_with_null_optionals() {
        let json = r#"{
            "id": "session-min",
            "project_id": "proj-min",
            "title": null,
            "status": "active",
            "created_at": "2026-01-24T12:00:00Z",
            "updated_at": "2026-01-24T12:00:00Z",
            "archived_at": null,
            "converted_at": null
        }"#;

        let session: IdeationSession = serde_json::from_str(json).expect("Should deserialize");

        assert!(session.title.is_none());
        assert!(session.archived_at.is_none());
        assert!(session.converted_at.is_none());
    }

    #[test]
    fn session_roundtrip_serialization() {
        let mut original = IdeationSession::new_with_title(ProjectId::new(), "Roundtrip");
        original.archive();

        let json = serde_json::to_string(&original).expect("Should serialize");
        let restored: IdeationSession = serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(original.id, restored.id);
        assert_eq!(original.project_id, restored.project_id);
        assert_eq!(original.title, restored.title);
        assert_eq!(original.status, restored.status);
    }

    #[test]
    fn session_clone_works() {
        let original = IdeationSession::new_with_title(ProjectId::new(), "Clone Test");
        let cloned = original.clone();

        assert_eq!(original.id, cloned.id);
        assert_eq!(original.project_id, cloned.project_id);
        assert_eq!(original.title, cloned.title);
        assert_eq!(original.status, cloned.status);
    }

    #[test]
    fn session_clone_is_independent() {
        let original = IdeationSession::new(ProjectId::new());
        let mut cloned = original.clone();

        cloned.archive();

        // Original should be unchanged
        assert_eq!(original.status, IdeationSessionStatus::Active);
        assert_eq!(cloned.status, IdeationSessionStatus::Archived);
    }

    // ===== from_row Integration Tests =====

    use chrono::Timelike;
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            r#"CREATE TABLE ideation_sessions (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                title TEXT,
                status TEXT NOT NULL DEFAULT 'active',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                archived_at TEXT,
                converted_at TEXT
            )"#,
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn session_from_row_active() {
        let conn = setup_test_db();
        conn.execute(
            r#"INSERT INTO ideation_sessions (id, project_id, title, status, created_at, updated_at)
               VALUES ('sess-1', 'proj-1', 'Auth Feature', 'active',
               '2026-01-24T10:00:00Z', '2026-01-24T11:00:00Z')"#,
            [],
        )
        .unwrap();

        let session: IdeationSession = conn
            .query_row("SELECT * FROM ideation_sessions WHERE id = 'sess-1'", [], |row| {
                IdeationSession::from_row(row)
            })
            .unwrap();

        assert_eq!(session.id.as_str(), "sess-1");
        assert_eq!(session.project_id.as_str(), "proj-1");
        assert_eq!(session.title, Some("Auth Feature".to_string()));
        assert_eq!(session.status, IdeationSessionStatus::Active);
        assert!(session.archived_at.is_none());
        assert!(session.converted_at.is_none());
    }

    #[test]
    fn session_from_row_archived() {
        let conn = setup_test_db();
        conn.execute(
            r#"INSERT INTO ideation_sessions (id, project_id, title, status, created_at, updated_at, archived_at)
               VALUES ('sess-2', 'proj-1', NULL, 'archived',
               '2026-01-24T08:00:00Z', '2026-01-24T12:00:00Z', '2026-01-24T12:00:00Z')"#,
            [],
        )
        .unwrap();

        let session: IdeationSession = conn
            .query_row("SELECT * FROM ideation_sessions WHERE id = 'sess-2'", [], |row| {
                IdeationSession::from_row(row)
            })
            .unwrap();

        assert_eq!(session.status, IdeationSessionStatus::Archived);
        assert!(session.title.is_none());
        assert!(session.archived_at.is_some());
    }

    #[test]
    fn session_from_row_converted() {
        let conn = setup_test_db();
        conn.execute(
            r#"INSERT INTO ideation_sessions (id, project_id, title, status, created_at, updated_at, converted_at)
               VALUES ('sess-3', 'proj-1', 'Done Session', 'converted',
               '2026-01-24T08:00:00Z', '2026-01-24T14:00:00Z', '2026-01-24T14:00:00Z')"#,
            [],
        )
        .unwrap();

        let session: IdeationSession = conn
            .query_row("SELECT * FROM ideation_sessions WHERE id = 'sess-3'", [], |row| {
                IdeationSession::from_row(row)
            })
            .unwrap();

        assert_eq!(session.status, IdeationSessionStatus::Converted);
        assert!(session.converted_at.is_some());
    }

    #[test]
    fn session_from_row_unknown_status_defaults_to_active() {
        let conn = setup_test_db();
        conn.execute(
            r#"INSERT INTO ideation_sessions (id, project_id, title, status, created_at, updated_at)
               VALUES ('sess-unk', 'proj-1', NULL, 'unknown_status',
               '2026-01-24T08:00:00Z', '2026-01-24T08:00:00Z')"#,
            [],
        )
        .unwrap();

        let session: IdeationSession = conn
            .query_row("SELECT * FROM ideation_sessions WHERE id = 'sess-unk'", [], |row| {
                IdeationSession::from_row(row)
            })
            .unwrap();

        // Unknown status should default to Active
        assert_eq!(session.status, IdeationSessionStatus::Active);
    }

    #[test]
    fn session_from_row_sqlite_datetime_format() {
        let conn = setup_test_db();
        conn.execute(
            r#"INSERT INTO ideation_sessions (id, project_id, title, status, created_at, updated_at)
               VALUES ('sess-sql', 'proj-1', NULL, 'active',
               '2026-01-24 12:30:00', '2026-01-24 14:45:00')"#,
            [],
        )
        .unwrap();

        let session: IdeationSession = conn
            .query_row("SELECT * FROM ideation_sessions WHERE id = 'sess-sql'", [], |row| {
                IdeationSession::from_row(row)
            })
            .unwrap();

        assert_eq!(session.created_at.hour(), 12);
        assert_eq!(session.created_at.minute(), 30);
        assert_eq!(session.updated_at.hour(), 14);
        assert_eq!(session.updated_at.minute(), 45);
    }

    #[test]
    fn session_from_row_with_all_timestamps() {
        let conn = setup_test_db();
        conn.execute(
            r#"INSERT INTO ideation_sessions (id, project_id, title, status, created_at, updated_at, archived_at, converted_at)
               VALUES ('sess-full', 'proj-1', 'Full', 'converted',
               '2026-01-24T08:00:00Z', '2026-01-24T16:00:00Z',
               '2026-01-24T12:00:00Z', '2026-01-24T16:00:00Z')"#,
            [],
        )
        .unwrap();

        let session: IdeationSession = conn
            .query_row("SELECT * FROM ideation_sessions WHERE id = 'sess-full'", [], |row| {
                IdeationSession::from_row(row)
            })
            .unwrap();

        assert!(session.archived_at.is_some());
        assert!(session.converted_at.is_some());
        assert_eq!(session.archived_at.unwrap().hour(), 12);
        assert_eq!(session.converted_at.unwrap().hour(), 16);
    }
}
