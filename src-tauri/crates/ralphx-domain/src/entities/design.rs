use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::{ChatConversationId, ChatMessageId, ProjectId};

macro_rules! design_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub String);

        impl $name {
            pub fn new() -> Self {
                Self(uuid::Uuid::new_v4().to_string())
            }

            pub fn from_string(s: impl Into<String>) -> Self {
                Self(s.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

design_id!(DesignSystemId);
design_id!(DesignSystemSourceId);
design_id!(DesignSchemaVersionId);
design_id!(DesignStyleguideItemId);
design_id!(DesignStyleguideFeedbackId);
design_id!(DesignRunId);
design_id!(DesignAssetRefId);
design_id!(DesignExportPackageId);

/// Opaque app-owned storage reference, not a raw filesystem path.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DesignStorageRootRef(pub String);

impl DesignStorageRootRef {
    pub fn from_hash_component(component: impl Into<String>) -> Self {
        Self(component.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesignSystemStatus {
    Draft,
    Analyzing,
    SchemaReady,
    Ready,
    Updating,
    Failed,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesignSourceRole {
    Primary,
    Secondary,
    Reference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesignSourceKind {
    ProjectCheckout,
    Upload,
    Url,
    ManualNote,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesignSchemaVersionStatus {
    Draft,
    Verified,
    Superseded,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesignStyleguideGroup {
    UiKit,
    Type,
    Colors,
    Spacing,
    Components,
    Brand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesignConfidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesignApprovalStatus {
    NeedsReview,
    Approved,
    NeedsWork,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesignFeedbackStatus {
    None,
    Open,
    InProgress,
    Resolved,
    Dismissed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesignRunKind {
    Create,
    Update,
    GenerateScreen,
    GenerateComponent,
    ItemFeedback,
    Audit,
    Export,
    Import,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesignRunStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignSourceRef {
    pub project_id: ProjectId,
    pub path: String,
    pub line: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignSystem {
    pub id: DesignSystemId,
    pub primary_project_id: ProjectId,
    pub name: String,
    pub description: Option<String>,
    pub status: DesignSystemStatus,
    pub current_schema_version_id: Option<DesignSchemaVersionId>,
    pub storage_root_ref: DesignStorageRootRef,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

impl DesignSystem {
    pub fn new(
        primary_project_id: ProjectId,
        name: impl Into<String>,
        storage_root_ref: DesignStorageRootRef,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: DesignSystemId::new(),
            primary_project_id,
            name: name.into(),
            description: None,
            status: DesignSystemStatus::Draft,
            current_schema_version_id: None,
            storage_root_ref,
            created_at: now,
            updated_at: now,
            archived_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignSystemSource {
    pub id: DesignSystemSourceId,
    pub design_system_id: DesignSystemId,
    pub project_id: ProjectId,
    pub role: DesignSourceRole,
    pub selected_paths: Vec<String>,
    pub source_kind: DesignSourceKind,
    pub git_commit: Option<String>,
    pub source_hashes: BTreeMap<String, String>,
    pub last_analyzed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignSchemaVersion {
    pub id: DesignSchemaVersionId,
    pub design_system_id: DesignSystemId,
    pub version: String,
    pub schema_artifact_id: String,
    pub manifest_artifact_id: String,
    pub styleguide_artifact_id: String,
    pub status: DesignSchemaVersionStatus,
    pub created_by_run_id: Option<DesignRunId>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignStyleguideItem {
    pub id: DesignStyleguideItemId,
    pub design_system_id: DesignSystemId,
    pub schema_version_id: DesignSchemaVersionId,
    pub item_id: String,
    pub group: DesignStyleguideGroup,
    pub label: String,
    pub summary: String,
    pub preview_artifact_id: Option<String>,
    pub source_refs: Vec<DesignSourceRef>,
    pub confidence: DesignConfidence,
    pub approval_status: DesignApprovalStatus,
    pub feedback_status: DesignFeedbackStatus,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignStyleguideFeedback {
    pub id: DesignStyleguideFeedbackId,
    pub design_system_id: DesignSystemId,
    pub schema_version_id: DesignSchemaVersionId,
    pub item_id: String,
    pub conversation_id: ChatConversationId,
    pub message_id: Option<ChatMessageId>,
    pub preview_artifact_id: Option<String>,
    pub source_refs: Vec<DesignSourceRef>,
    pub feedback: String,
    pub status: DesignFeedbackStatus,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignRun {
    pub id: DesignRunId,
    pub design_system_id: DesignSystemId,
    pub conversation_id: Option<ChatConversationId>,
    pub kind: DesignRunKind,
    pub status: DesignRunStatus,
    pub input_summary: String,
    pub output_artifact_ids: Vec<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

impl DesignRun {
    pub fn queued(
        design_system_id: DesignSystemId,
        kind: DesignRunKind,
        input_summary: impl Into<String>,
    ) -> Self {
        Self {
            id: DesignRunId::new(),
            design_system_id,
            conversation_id: None,
            kind,
            status: DesignRunStatus::Queued,
            input_summary: input_summary.into(),
            output_artifact_ids: Vec::new(),
            started_at: None,
            completed_at: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignAssetRef {
    pub id: DesignAssetRefId,
    pub design_system_id: DesignSystemId,
    pub source_ref: Option<DesignSourceRef>,
    pub storage_ref: String,
    pub asset_kind: String,
    pub canonical: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignExportPackage {
    pub id: DesignExportPackageId,
    pub design_system_id: DesignSystemId,
    pub schema_version_id: Option<DesignSchemaVersionId>,
    pub manifest_artifact_id: String,
    pub package_storage_ref: String,
    pub redacted: bool,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
#[path = "design_tests.rs"]
mod tests;
