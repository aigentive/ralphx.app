// Workflow entities for custom workflow schemas
// These enable custom Kanban columns that map to internal statuses

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use super::status::InternalStatus;

/// A unique identifier for a Workflow
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowId(pub String);

impl WorkflowId {
    /// Creates a new WorkflowId with a random UUID v4
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Creates a WorkflowId from an existing string
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for WorkflowId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for WorkflowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A workflow schema defines the Kanban board layout with custom columns
/// that map to internal statuses for consistent side effects
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowSchema {
    /// Unique identifier
    pub id: WorkflowId,
    /// Display name for the workflow
    pub name: String,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Ordered list of columns in the Kanban board
    pub columns: Vec<WorkflowColumn>,
    /// External sync configuration (future implementation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_sync: Option<ExternalSyncConfig>,
    /// Default agent profile overrides
    #[serde(default)]
    pub defaults: WorkflowDefaults,
    /// Whether this is the default workflow
    #[serde(default)]
    pub is_default: bool,
}

impl WorkflowSchema {
    /// Creates a new workflow schema with the given name and columns
    pub fn new(name: impl Into<String>, columns: Vec<WorkflowColumn>) -> Self {
        Self {
            id: WorkflowId::new(),
            name: name.into(),
            description: None,
            columns,
            external_sync: None,
            defaults: WorkflowDefaults::default(),
            is_default: false,
        }
    }

    /// Sets the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets this as the default workflow
    pub fn as_default(mut self) -> Self {
        self.is_default = true;
        self
    }

    /// Returns the default RalphX workflow
    pub fn default_ralphx() -> Self {
        Self {
            id: WorkflowId::from_string("ralphx-default"),
            name: "RalphX Default".to_string(),
            description: Some("Default RalphX workflow".to_string()),
            columns: vec![
                WorkflowColumn::new("draft", "Draft", InternalStatus::Backlog),
                WorkflowColumn::new("backlog", "Backlog", InternalStatus::Backlog),
                WorkflowColumn::new("todo", "To Do", InternalStatus::Ready),
                WorkflowColumn::new("planned", "Planned", InternalStatus::Ready),
                WorkflowColumn::new("in_progress", "In Progress", InternalStatus::Executing),
                WorkflowColumn::new("in_review", "In Review", InternalStatus::PendingReview),
                WorkflowColumn::new("done", "Done", InternalStatus::Approved),
            ],
            external_sync: None,
            defaults: WorkflowDefaults::default(),
            is_default: true,
        }
    }

    /// Returns the Jira-compatible workflow
    pub fn jira_compatible() -> Self {
        Self {
            id: WorkflowId::from_string("jira-compat"),
            name: "Jira Compatible".to_string(),
            description: Some("Jira-style workflow with familiar columns".to_string()),
            columns: vec![
                WorkflowColumn::new("backlog", "Backlog", InternalStatus::Backlog),
                WorkflowColumn::new("selected", "Selected for Dev", InternalStatus::Ready),
                WorkflowColumn::new("in_progress", "In Progress", InternalStatus::Executing),
                WorkflowColumn::new("in_qa", "In QA", InternalStatus::PendingReview),
                WorkflowColumn::new("done", "Done", InternalStatus::Approved),
            ],
            external_sync: Some(ExternalSyncConfig {
                provider: SyncProvider::Jira,
                mapping: std::collections::HashMap::new(),
                sync: SyncSettings {
                    direction: SyncDirection::Bidirectional,
                    webhook: Some(true),
                },
                conflict_resolution: ConflictResolution::ExternalWins,
            }),
            defaults: WorkflowDefaults::default(),
            is_default: false,
        }
    }
}

/// A column in a workflow's Kanban board
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowColumn {
    /// Unique identifier within the workflow
    pub id: String,
    /// Display name for the column
    pub name: String,
    /// Optional color (hex format, e.g., "#ff6b35")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Optional icon name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// The internal status this column maps to for side effects
    pub maps_to: InternalStatus,
    /// Optional behavior overrides for this column
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behavior: Option<ColumnBehavior>,
}

impl WorkflowColumn {
    /// Creates a new workflow column
    pub fn new(id: impl Into<String>, name: impl Into<String>, maps_to: InternalStatus) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            color: None,
            icon: None,
            maps_to,
            behavior: None,
        }
    }

    /// Sets the column color
    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Sets the column icon
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Sets the column behavior
    pub fn with_behavior(mut self, behavior: ColumnBehavior) -> Self {
        self.behavior = Some(behavior);
        self
    }
}

/// Behavior overrides for a workflow column
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ColumnBehavior {
    /// Skip review for tasks in this column
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_review: Option<bool>,
    /// Auto-advance to next column when complete
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_advance: Option<bool>,
    /// Override agent profile for this column
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_profile: Option<String>,
}

impl ColumnBehavior {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_skip_review(mut self, skip: bool) -> Self {
        self.skip_review = Some(skip);
        self
    }

    pub fn with_auto_advance(mut self, advance: bool) -> Self {
        self.auto_advance = Some(advance);
        self
    }

    pub fn with_agent_profile(mut self, profile: impl Into<String>) -> Self {
        self.agent_profile = Some(profile.into());
        self
    }
}

/// Default agent profile configuration for a workflow
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct WorkflowDefaults {
    /// Default worker agent profile
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker_profile: Option<String>,
    /// Default reviewer agent profile
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewer_profile: Option<String>,
}

/// External sync configuration (placeholder for future implementation)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalSyncConfig {
    /// The external provider
    pub provider: SyncProvider,
    /// Status mapping from external to internal
    #[serde(default)]
    pub mapping: std::collections::HashMap<String, ExternalStatusMapping>,
    /// Sync settings
    pub sync: SyncSettings,
    /// How to resolve conflicts
    pub conflict_resolution: ConflictResolution,
}

/// Supported external sync providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncProvider {
    Jira,
    Github,
    Linear,
    Notion,
}

impl fmt::Display for SyncProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncProvider::Jira => write!(f, "jira"),
            SyncProvider::Github => write!(f, "github"),
            SyncProvider::Linear => write!(f, "linear"),
            SyncProvider::Notion => write!(f, "notion"),
        }
    }
}

/// Mapping from an external status to internal status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalStatusMapping {
    /// The external status name
    pub external_status: String,
    /// The internal status to map to
    pub internal_status: InternalStatus,
    /// The workflow column to display in
    pub column_id: String,
}

/// Sync direction settings
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncSettings {
    /// Sync direction
    pub direction: SyncDirection,
    /// Enable webhook for real-time sync
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook: Option<bool>,
}

/// Sync direction options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncDirection {
    Pull,
    Push,
    Bidirectional,
}

impl fmt::Display for SyncDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncDirection::Pull => write!(f, "pull"),
            SyncDirection::Push => write!(f, "push"),
            SyncDirection::Bidirectional => write!(f, "bidirectional"),
        }
    }
}

/// Error for parsing SyncDirection from string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseSyncDirectionError {
    pub value: String,
}

impl fmt::Display for ParseSyncDirectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown sync direction: '{}'", self.value)
    }
}

impl std::error::Error for ParseSyncDirectionError {}

impl FromStr for SyncDirection {
    type Err = ParseSyncDirectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pull" => Ok(SyncDirection::Pull),
            "push" => Ok(SyncDirection::Push),
            "bidirectional" => Ok(SyncDirection::Bidirectional),
            _ => Err(ParseSyncDirectionError {
                value: s.to_string(),
            }),
        }
    }
}

/// Conflict resolution strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolution {
    ExternalWins,
    InternalWins,
    Manual,
}

impl fmt::Display for ConflictResolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConflictResolution::ExternalWins => write!(f, "external_wins"),
            ConflictResolution::InternalWins => write!(f, "internal_wins"),
            ConflictResolution::Manual => write!(f, "manual"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== WorkflowId Tests =====

    #[test]
    fn workflow_id_new_generates_valid_uuid() {
        let id = WorkflowId::new();
        assert_eq!(id.as_str().len(), 36);
        assert!(uuid::Uuid::parse_str(id.as_str()).is_ok());
    }

    #[test]
    fn workflow_id_from_string_preserves_value() {
        let id = WorkflowId::from_string("wf-custom");
        assert_eq!(id.as_str(), "wf-custom");
    }

    #[test]
    fn workflow_id_equality_works() {
        let id1 = WorkflowId::from_string("wf-1");
        let id2 = WorkflowId::from_string("wf-1");
        let id3 = WorkflowId::from_string("wf-2");
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn workflow_id_serializes() {
        let id = WorkflowId::from_string("wf-serialize");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"wf-serialize\"");
    }

    #[test]
    fn workflow_id_deserializes() {
        let json = "\"wf-deserialize\"";
        let id: WorkflowId = serde_json::from_str(json).unwrap();
        assert_eq!(id.as_str(), "wf-deserialize");
    }

    // ===== WorkflowSchema Tests =====

    #[test]
    fn workflow_schema_new_creates_with_id() {
        let schema = WorkflowSchema::new(
            "Test Workflow",
            vec![WorkflowColumn::new("col1", "Column 1", InternalStatus::Backlog)],
        );
        assert!(!schema.id.as_str().is_empty());
        assert_eq!(schema.name, "Test Workflow");
        assert_eq!(schema.columns.len(), 1);
        assert!(!schema.is_default);
    }

    #[test]
    fn workflow_schema_with_description() {
        let schema = WorkflowSchema::new("Test", vec![])
            .with_description("A test workflow");
        assert_eq!(schema.description, Some("A test workflow".to_string()));
    }

    #[test]
    fn workflow_schema_as_default() {
        let schema = WorkflowSchema::new("Test", vec![]).as_default();
        assert!(schema.is_default);
    }

    #[test]
    fn workflow_schema_serializes_roundtrip() {
        let schema = WorkflowSchema::new(
            "Roundtrip",
            vec![
                WorkflowColumn::new("a", "A", InternalStatus::Backlog),
                WorkflowColumn::new("b", "B", InternalStatus::Ready),
            ],
        );
        let json = serde_json::to_string(&schema).unwrap();
        let parsed: WorkflowSchema = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, schema.name);
        assert_eq!(parsed.columns.len(), 2);
    }

    #[test]
    fn workflow_schema_skips_null_optional_fields() {
        let schema = WorkflowSchema::new("Minimal", vec![]);
        let json = serde_json::to_string(&schema).unwrap();
        assert!(!json.contains("external_sync"));
        assert!(!json.contains("description"));
    }

    // ===== Default Workflows Tests =====

    #[test]
    fn default_ralphx_workflow_has_7_columns() {
        let workflow = WorkflowSchema::default_ralphx();
        assert_eq!(workflow.id.as_str(), "ralphx-default");
        assert_eq!(workflow.name, "RalphX Default");
        assert_eq!(workflow.columns.len(), 7);
        assert!(workflow.is_default);
    }

    #[test]
    fn default_ralphx_workflow_column_mappings() {
        let workflow = WorkflowSchema::default_ralphx();
        let column_mappings: Vec<_> = workflow
            .columns
            .iter()
            .map(|c| (&c.id, c.maps_to))
            .collect();

        assert!(column_mappings.iter().any(|(id, status)| *id == "draft" && *status == InternalStatus::Backlog));
        assert!(column_mappings.iter().any(|(id, status)| *id == "todo" && *status == InternalStatus::Ready));
        assert!(column_mappings.iter().any(|(id, status)| *id == "in_progress" && *status == InternalStatus::Executing));
        assert!(column_mappings.iter().any(|(id, status)| *id == "done" && *status == InternalStatus::Approved));
    }

    #[test]
    fn jira_compatible_workflow_has_5_columns() {
        let workflow = WorkflowSchema::jira_compatible();
        assert_eq!(workflow.id.as_str(), "jira-compat");
        assert_eq!(workflow.name, "Jira Compatible");
        assert_eq!(workflow.columns.len(), 5);
        assert!(!workflow.is_default);
    }

    #[test]
    fn jira_compatible_has_external_sync() {
        let workflow = WorkflowSchema::jira_compatible();
        let sync = workflow.external_sync.as_ref().unwrap();
        assert_eq!(sync.provider, SyncProvider::Jira);
        assert_eq!(sync.sync.direction, SyncDirection::Bidirectional);
        assert_eq!(sync.conflict_resolution, ConflictResolution::ExternalWins);
    }

    // ===== WorkflowColumn Tests =====

    #[test]
    fn workflow_column_new_creates_minimal() {
        let col = WorkflowColumn::new("col-id", "Column Name", InternalStatus::Ready);
        assert_eq!(col.id, "col-id");
        assert_eq!(col.name, "Column Name");
        assert_eq!(col.maps_to, InternalStatus::Ready);
        assert!(col.color.is_none());
        assert!(col.icon.is_none());
        assert!(col.behavior.is_none());
    }

    #[test]
    fn workflow_column_with_color() {
        let col = WorkflowColumn::new("col", "Col", InternalStatus::Backlog)
            .with_color("#ff6b35");
        assert_eq!(col.color, Some("#ff6b35".to_string()));
    }

    #[test]
    fn workflow_column_with_icon() {
        let col = WorkflowColumn::new("col", "Col", InternalStatus::Backlog)
            .with_icon("check-circle");
        assert_eq!(col.icon, Some("check-circle".to_string()));
    }

    #[test]
    fn workflow_column_with_behavior() {
        let behavior = ColumnBehavior::new()
            .with_skip_review(true)
            .with_agent_profile("worker-fast");
        let col = WorkflowColumn::new("col", "Col", InternalStatus::Executing)
            .with_behavior(behavior);

        let b = col.behavior.unwrap();
        assert_eq!(b.skip_review, Some(true));
        assert_eq!(b.agent_profile, Some("worker-fast".to_string()));
    }

    #[test]
    fn workflow_column_serializes() {
        let col = WorkflowColumn::new("test", "Test", InternalStatus::Blocked)
            .with_color("#aabbcc");
        let json = serde_json::to_string(&col).unwrap();
        assert!(json.contains("\"id\":\"test\""));
        assert!(json.contains("\"maps_to\":\"blocked\""));
        assert!(json.contains("\"color\":\"#aabbcc\""));
    }

    #[test]
    fn workflow_column_deserializes() {
        let json = r#"{"id":"col1","name":"Column 1","maps_to":"ready"}"#;
        let col: WorkflowColumn = serde_json::from_str(json).unwrap();
        assert_eq!(col.id, "col1");
        assert_eq!(col.name, "Column 1");
        assert_eq!(col.maps_to, InternalStatus::Ready);
    }

    // ===== ColumnBehavior Tests =====

    #[test]
    fn column_behavior_default_is_empty() {
        let b = ColumnBehavior::default();
        assert!(b.skip_review.is_none());
        assert!(b.auto_advance.is_none());
        assert!(b.agent_profile.is_none());
    }

    #[test]
    fn column_behavior_builder_chain() {
        let b = ColumnBehavior::new()
            .with_skip_review(false)
            .with_auto_advance(true)
            .with_agent_profile("reviewer");

        assert_eq!(b.skip_review, Some(false));
        assert_eq!(b.auto_advance, Some(true));
        assert_eq!(b.agent_profile, Some("reviewer".to_string()));
    }

    // ===== SyncProvider Tests =====

    #[test]
    fn sync_provider_serializes_snake_case() {
        assert_eq!(serde_json::to_string(&SyncProvider::Jira).unwrap(), "\"jira\"");
        assert_eq!(serde_json::to_string(&SyncProvider::Github).unwrap(), "\"github\"");
        assert_eq!(serde_json::to_string(&SyncProvider::Linear).unwrap(), "\"linear\"");
        assert_eq!(serde_json::to_string(&SyncProvider::Notion).unwrap(), "\"notion\"");
    }

    #[test]
    fn sync_provider_deserializes() {
        let p: SyncProvider = serde_json::from_str("\"jira\"").unwrap();
        assert_eq!(p, SyncProvider::Jira);
    }

    #[test]
    fn sync_provider_display() {
        assert_eq!(SyncProvider::Jira.to_string(), "jira");
        assert_eq!(SyncProvider::Github.to_string(), "github");
    }

    // ===== SyncDirection Tests =====

    #[test]
    fn sync_direction_serializes() {
        assert_eq!(serde_json::to_string(&SyncDirection::Pull).unwrap(), "\"pull\"");
        assert_eq!(serde_json::to_string(&SyncDirection::Push).unwrap(), "\"push\"");
        assert_eq!(serde_json::to_string(&SyncDirection::Bidirectional).unwrap(), "\"bidirectional\"");
    }

    #[test]
    fn sync_direction_from_str() {
        assert_eq!(SyncDirection::from_str("pull").unwrap(), SyncDirection::Pull);
        assert_eq!(SyncDirection::from_str("push").unwrap(), SyncDirection::Push);
        assert_eq!(SyncDirection::from_str("bidirectional").unwrap(), SyncDirection::Bidirectional);
    }

    #[test]
    fn sync_direction_from_str_error() {
        let err = SyncDirection::from_str("invalid").unwrap_err();
        assert_eq!(err.value, "invalid");
        assert!(err.to_string().contains("invalid"));
    }

    // ===== ConflictResolution Tests =====

    #[test]
    fn conflict_resolution_serializes() {
        assert_eq!(serde_json::to_string(&ConflictResolution::ExternalWins).unwrap(), "\"external_wins\"");
        assert_eq!(serde_json::to_string(&ConflictResolution::InternalWins).unwrap(), "\"internal_wins\"");
        assert_eq!(serde_json::to_string(&ConflictResolution::Manual).unwrap(), "\"manual\"");
    }

    #[test]
    fn conflict_resolution_display() {
        assert_eq!(ConflictResolution::ExternalWins.to_string(), "external_wins");
        assert_eq!(ConflictResolution::InternalWins.to_string(), "internal_wins");
        assert_eq!(ConflictResolution::Manual.to_string(), "manual");
    }

    // ===== ExternalSyncConfig Tests =====

    #[test]
    fn external_sync_config_serializes() {
        let config = ExternalSyncConfig {
            provider: SyncProvider::Github,
            mapping: std::collections::HashMap::new(),
            sync: SyncSettings {
                direction: SyncDirection::Pull,
                webhook: None,
            },
            conflict_resolution: ConflictResolution::Manual,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"provider\":\"github\""));
        assert!(json.contains("\"direction\":\"pull\""));
        assert!(json.contains("\"conflict_resolution\":\"manual\""));
    }

    // ===== WorkflowDefaults Tests =====

    #[test]
    fn workflow_defaults_is_empty_by_default() {
        let d = WorkflowDefaults::default();
        assert!(d.worker_profile.is_none());
        assert!(d.reviewer_profile.is_none());
    }

    #[test]
    fn workflow_defaults_serializes() {
        let d = WorkflowDefaults {
            worker_profile: Some("fast-worker".to_string()),
            reviewer_profile: None,
        };
        let json = serde_json::to_string(&d).unwrap();
        assert!(json.contains("\"worker_profile\":\"fast-worker\""));
    }
}
