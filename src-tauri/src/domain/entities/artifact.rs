// Artifact entities for the extensibility system
// Artifacts are typed documents that flow between processes

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use super::types::TaskId;

/// A unique identifier for an Artifact
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArtifactId(pub String);

impl ArtifactId {
    /// Creates a new ArtifactId with a random UUID v4
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Creates an ArtifactId from an existing string
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ArtifactId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ArtifactId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A unique identifier for an ArtifactBucket
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArtifactBucketId(pub String);

impl ArtifactBucketId {
    /// Creates a new ArtifactBucketId with a random UUID v4
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Creates an ArtifactBucketId from an existing string
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ArtifactBucketId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ArtifactBucketId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A unique identifier for a Process
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcessId(pub String);

impl ProcessId {
    /// Creates a new ProcessId with a random UUID v4
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Creates a ProcessId from an existing string
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ProcessId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ProcessId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The type of an artifact - 15 categories of typed documents
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    // Documents
    Prd,
    ResearchDocument,
    DesignDoc,
    Specification,
    // Code
    CodeChange,
    Diff,
    TestResult,
    // Process
    TaskSpec,
    ReviewFeedback,
    Approval,
    Findings,
    Recommendations,
    // Context
    Context,
    PreviousWork,
    ResearchBrief,
    // Logs (bonus types for completeness)
    ActivityLog,
    Alert,
    Intervention,
}

impl ArtifactType {
    /// Returns all artifact types
    pub fn all() -> &'static [ArtifactType] {
        &[
            ArtifactType::Prd,
            ArtifactType::ResearchDocument,
            ArtifactType::DesignDoc,
            ArtifactType::Specification,
            ArtifactType::CodeChange,
            ArtifactType::Diff,
            ArtifactType::TestResult,
            ArtifactType::TaskSpec,
            ArtifactType::ReviewFeedback,
            ArtifactType::Approval,
            ArtifactType::Findings,
            ArtifactType::Recommendations,
            ArtifactType::Context,
            ArtifactType::PreviousWork,
            ArtifactType::ResearchBrief,
            ArtifactType::ActivityLog,
            ArtifactType::Alert,
            ArtifactType::Intervention,
        ]
    }

    /// Returns the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ArtifactType::Prd => "prd",
            ArtifactType::ResearchDocument => "research_document",
            ArtifactType::DesignDoc => "design_doc",
            ArtifactType::Specification => "specification",
            ArtifactType::CodeChange => "code_change",
            ArtifactType::Diff => "diff",
            ArtifactType::TestResult => "test_result",
            ArtifactType::TaskSpec => "task_spec",
            ArtifactType::ReviewFeedback => "review_feedback",
            ArtifactType::Approval => "approval",
            ArtifactType::Findings => "findings",
            ArtifactType::Recommendations => "recommendations",
            ArtifactType::Context => "context",
            ArtifactType::PreviousWork => "previous_work",
            ArtifactType::ResearchBrief => "research_brief",
            ArtifactType::ActivityLog => "activity_log",
            ArtifactType::Alert => "alert",
            ArtifactType::Intervention => "intervention",
        }
    }
}

impl fmt::Display for ArtifactType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error for parsing ArtifactType from string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseArtifactTypeError {
    pub value: String,
}

impl fmt::Display for ParseArtifactTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown artifact type: '{}'", self.value)
    }
}

impl std::error::Error for ParseArtifactTypeError {}

impl FromStr for ArtifactType {
    type Err = ParseArtifactTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "prd" => Ok(ArtifactType::Prd),
            "research_document" => Ok(ArtifactType::ResearchDocument),
            "design_doc" => Ok(ArtifactType::DesignDoc),
            "specification" => Ok(ArtifactType::Specification),
            "code_change" => Ok(ArtifactType::CodeChange),
            "diff" => Ok(ArtifactType::Diff),
            "test_result" => Ok(ArtifactType::TestResult),
            "task_spec" => Ok(ArtifactType::TaskSpec),
            "review_feedback" => Ok(ArtifactType::ReviewFeedback),
            "approval" => Ok(ArtifactType::Approval),
            "findings" => Ok(ArtifactType::Findings),
            "recommendations" => Ok(ArtifactType::Recommendations),
            "context" => Ok(ArtifactType::Context),
            "previous_work" => Ok(ArtifactType::PreviousWork),
            "research_brief" => Ok(ArtifactType::ResearchBrief),
            "activity_log" => Ok(ArtifactType::ActivityLog),
            "alert" => Ok(ArtifactType::Alert),
            "intervention" => Ok(ArtifactType::Intervention),
            _ => Err(ParseArtifactTypeError {
                value: s.to_string(),
            }),
        }
    }
}

/// The content of an artifact - either inline text or a file path
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ArtifactContent {
    /// Inline text content stored directly in the database
    Inline { text: String },
    /// File content stored at a path
    File { path: String },
}

impl ArtifactContent {
    /// Creates inline content
    pub fn inline(text: impl Into<String>) -> Self {
        ArtifactContent::Inline { text: text.into() }
    }

    /// Creates file content
    pub fn file(path: impl Into<String>) -> Self {
        ArtifactContent::File { path: path.into() }
    }

    /// Returns true if this is inline content
    pub fn is_inline(&self) -> bool {
        matches!(self, ArtifactContent::Inline { .. })
    }

    /// Returns true if this is file content
    pub fn is_file(&self) -> bool {
        matches!(self, ArtifactContent::File { .. })
    }

    /// Returns the content type string for database storage
    pub fn content_type(&self) -> &'static str {
        match self {
            ArtifactContent::Inline { .. } => "inline",
            ArtifactContent::File { .. } => "file",
        }
    }
}

/// Metadata about an artifact
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    /// When the artifact was created
    pub created_at: DateTime<Utc>,
    /// Who created the artifact (agent profile ID or "user")
    pub created_by: String,
    /// Optional associated task ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<TaskId>,
    /// Optional associated process ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_id: Option<ProcessId>,
    /// Version number (starts at 1)
    #[serde(default = "default_version")]
    pub version: u32,
}

fn default_version() -> u32 {
    1
}

impl ArtifactMetadata {
    /// Creates new metadata with the given creator
    pub fn new(created_by: impl Into<String>) -> Self {
        Self {
            created_at: Utc::now(),
            created_by: created_by.into(),
            task_id: None,
            process_id: None,
            version: 1,
        }
    }

    /// Sets the task ID
    pub fn with_task(mut self, task_id: TaskId) -> Self {
        self.task_id = Some(task_id);
        self
    }

    /// Sets the process ID
    pub fn with_process(mut self, process_id: ProcessId) -> Self {
        self.process_id = Some(process_id);
        self
    }

    /// Sets the version number
    pub fn with_version(mut self, version: u32) -> Self {
        self.version = version;
        self
    }
}

/// An artifact - a typed document that flows between processes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Artifact {
    /// Unique identifier
    pub id: ArtifactId,
    /// The type of artifact
    #[serde(rename = "type")]
    pub artifact_type: ArtifactType,
    /// Display name
    pub name: String,
    /// The content (inline or file)
    pub content: ArtifactContent,
    /// Artifact metadata
    pub metadata: ArtifactMetadata,
    /// IDs of artifacts this was derived from
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub derived_from: Vec<ArtifactId>,
    /// Optional bucket ID this artifact belongs to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket_id: Option<ArtifactBucketId>,
}

impl Artifact {
    /// Creates a new artifact with inline content
    pub fn new_inline(
        name: impl Into<String>,
        artifact_type: ArtifactType,
        text: impl Into<String>,
        created_by: impl Into<String>,
    ) -> Self {
        Self {
            id: ArtifactId::new(),
            artifact_type,
            name: name.into(),
            content: ArtifactContent::inline(text),
            metadata: ArtifactMetadata::new(created_by),
            derived_from: vec![],
            bucket_id: None,
        }
    }

    /// Creates a new artifact with file content
    pub fn new_file(
        name: impl Into<String>,
        artifact_type: ArtifactType,
        path: impl Into<String>,
        created_by: impl Into<String>,
    ) -> Self {
        Self {
            id: ArtifactId::new(),
            artifact_type,
            name: name.into(),
            content: ArtifactContent::file(path),
            metadata: ArtifactMetadata::new(created_by),
            derived_from: vec![],
            bucket_id: None,
        }
    }

    /// Sets the bucket ID
    pub fn with_bucket(mut self, bucket_id: ArtifactBucketId) -> Self {
        self.bucket_id = Some(bucket_id);
        self
    }

    /// Adds a derived-from artifact ID
    pub fn derived_from_artifact(mut self, artifact_id: ArtifactId) -> Self {
        self.derived_from.push(artifact_id);
        self
    }

    /// Sets the task ID in metadata
    pub fn with_task(mut self, task_id: TaskId) -> Self {
        self.metadata.task_id = Some(task_id);
        self
    }

    /// Sets the process ID in metadata
    pub fn with_process(mut self, process_id: ProcessId) -> Self {
        self.metadata.process_id = Some(process_id);
        self
    }
}

/// An artifact bucket - organizes artifacts by purpose with access control
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactBucket {
    /// Unique identifier
    pub id: ArtifactBucketId,
    /// Display name
    pub name: String,
    /// Artifact types accepted in this bucket
    pub accepted_types: Vec<ArtifactType>,
    /// Who can write to this bucket (agent profile IDs, "user", or "system")
    pub writers: Vec<String>,
    /// Who can read from this bucket (agent profile IDs or "all")
    pub readers: Vec<String>,
    /// Whether this is a system bucket (cannot be deleted)
    #[serde(default)]
    pub is_system: bool,
}

impl ArtifactBucket {
    /// Creates a new custom bucket
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: ArtifactBucketId::new(),
            name: name.into(),
            accepted_types: vec![],
            writers: vec![],
            readers: vec!["all".to_string()],
            is_system: false,
        }
    }

    /// Creates a system bucket with a fixed ID
    pub fn system(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: ArtifactBucketId::from_string(id),
            name: name.into(),
            accepted_types: vec![],
            writers: vec![],
            readers: vec!["all".to_string()],
            is_system: true,
        }
    }

    /// Adds an accepted artifact type
    pub fn accepts(mut self, artifact_type: ArtifactType) -> Self {
        if !self.accepted_types.contains(&artifact_type) {
            self.accepted_types.push(artifact_type);
        }
        self
    }

    /// Adds multiple accepted artifact types
    pub fn accepts_all(mut self, types: impl IntoIterator<Item = ArtifactType>) -> Self {
        for t in types {
            if !self.accepted_types.contains(&t) {
                self.accepted_types.push(t);
            }
        }
        self
    }

    /// Adds a writer
    pub fn with_writer(mut self, writer: impl Into<String>) -> Self {
        let w = writer.into();
        if !self.writers.contains(&w) {
            self.writers.push(w);
        }
        self
    }

    /// Adds a reader
    pub fn with_reader(mut self, reader: impl Into<String>) -> Self {
        let r = reader.into();
        if !self.readers.contains(&r) {
            self.readers.push(r);
        }
        self
    }

    /// Checks if a type is accepted in this bucket
    pub fn accepts_type(&self, artifact_type: ArtifactType) -> bool {
        self.accepted_types.is_empty() || self.accepted_types.contains(&artifact_type)
    }

    /// Checks if a writer can write to this bucket
    pub fn can_write(&self, writer: &str) -> bool {
        self.writers.is_empty() || self.writers.iter().any(|w| w == writer || w == "all")
    }

    /// Checks if a reader can read from this bucket
    pub fn can_read(&self, reader: &str) -> bool {
        self.readers.iter().any(|r| r == reader || r == "all")
    }

    /// Returns the 4 system buckets defined in the PRD
    pub fn system_buckets() -> Vec<ArtifactBucket> {
        vec![
            ArtifactBucket::system("research-outputs", "Research Outputs")
                .accepts_all([
                    ArtifactType::ResearchDocument,
                    ArtifactType::Findings,
                    ArtifactType::Recommendations,
                ])
                .with_writer("deep-researcher")
                .with_writer("orchestrator"),
            ArtifactBucket::system("work-context", "Work Context")
                .accepts_all([
                    ArtifactType::Context,
                    ArtifactType::TaskSpec,
                    ArtifactType::PreviousWork,
                ])
                .with_writer("orchestrator")
                .with_writer("system"),
            ArtifactBucket::system("code-changes", "Code Changes")
                .accepts_all([
                    ArtifactType::CodeChange,
                    ArtifactType::Diff,
                    ArtifactType::TestResult,
                ])
                .with_writer("worker"),
            ArtifactBucket::system("prd-library", "PRD Library")
                .accepts_all([
                    ArtifactType::Prd,
                    ArtifactType::Specification,
                    ArtifactType::DesignDoc,
                ])
                .with_writer("orchestrator")
                .with_writer("user"),
        ]
    }
}

/// The type of relation between artifacts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactRelationType {
    /// The from artifact was derived from the to artifact
    DerivedFrom,
    /// The artifacts are related
    RelatedTo,
}

impl ArtifactRelationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ArtifactRelationType::DerivedFrom => "derived_from",
            ArtifactRelationType::RelatedTo => "related_to",
        }
    }
}

impl fmt::Display for ArtifactRelationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error for parsing ArtifactRelationType from string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseArtifactRelationTypeError {
    pub value: String,
}

impl fmt::Display for ParseArtifactRelationTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown artifact relation type: '{}'", self.value)
    }
}

impl std::error::Error for ParseArtifactRelationTypeError {}

impl FromStr for ArtifactRelationType {
    type Err = ParseArtifactRelationTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "derived_from" => Ok(ArtifactRelationType::DerivedFrom),
            "related_to" => Ok(ArtifactRelationType::RelatedTo),
            _ => Err(ParseArtifactRelationTypeError {
                value: s.to_string(),
            }),
        }
    }
}

/// A unique identifier for an ArtifactRelation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArtifactRelationId(pub String);

impl ArtifactRelationId {
    /// Creates a new ArtifactRelationId with a random UUID v4
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Creates an ArtifactRelationId from an existing string
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ArtifactRelationId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ArtifactRelationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A relation between two artifacts
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactRelation {
    /// Unique identifier
    pub id: ArtifactRelationId,
    /// The source artifact ID
    pub from_artifact_id: ArtifactId,
    /// The target artifact ID
    pub to_artifact_id: ArtifactId,
    /// The type of relation
    pub relation_type: ArtifactRelationType,
}

impl ArtifactRelation {
    /// Creates a new artifact relation
    pub fn new(
        from_artifact_id: ArtifactId,
        to_artifact_id: ArtifactId,
        relation_type: ArtifactRelationType,
    ) -> Self {
        Self {
            id: ArtifactRelationId::new(),
            from_artifact_id,
            to_artifact_id,
            relation_type,
        }
    }

    /// Creates a "derived from" relation
    pub fn derived_from(derived: ArtifactId, source: ArtifactId) -> Self {
        Self::new(derived, source, ArtifactRelationType::DerivedFrom)
    }

    /// Creates a "related to" relation
    pub fn related_to(from: ArtifactId, to: ArtifactId) -> Self {
        Self::new(from, to, ArtifactRelationType::RelatedTo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== ArtifactId Tests =====

    #[test]
    fn artifact_id_new_generates_valid_uuid() {
        let id = ArtifactId::new();
        assert_eq!(id.as_str().len(), 36);
        assert!(uuid::Uuid::parse_str(id.as_str()).is_ok());
    }

    #[test]
    fn artifact_id_from_string_preserves_value() {
        let id = ArtifactId::from_string("artifact-123");
        assert_eq!(id.as_str(), "artifact-123");
    }

    #[test]
    fn artifact_id_equality_works() {
        let id1 = ArtifactId::from_string("a1");
        let id2 = ArtifactId::from_string("a1");
        let id3 = ArtifactId::from_string("a2");
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn artifact_id_serializes() {
        let id = ArtifactId::from_string("serialize-test");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"serialize-test\"");
    }

    #[test]
    fn artifact_id_deserializes() {
        let json = "\"deserialize-test\"";
        let id: ArtifactId = serde_json::from_str(json).unwrap();
        assert_eq!(id.as_str(), "deserialize-test");
    }

    // ===== ArtifactBucketId Tests =====

    #[test]
    fn artifact_bucket_id_new_generates_valid_uuid() {
        let id = ArtifactBucketId::new();
        assert_eq!(id.as_str().len(), 36);
        assert!(uuid::Uuid::parse_str(id.as_str()).is_ok());
    }

    #[test]
    fn artifact_bucket_id_from_string_preserves_value() {
        let id = ArtifactBucketId::from_string("bucket-123");
        assert_eq!(id.as_str(), "bucket-123");
    }

    // ===== ProcessId Tests =====

    #[test]
    fn process_id_new_generates_valid_uuid() {
        let id = ProcessId::new();
        assert_eq!(id.as_str().len(), 36);
        assert!(uuid::Uuid::parse_str(id.as_str()).is_ok());
    }

    #[test]
    fn process_id_from_string_preserves_value() {
        let id = ProcessId::from_string("process-123");
        assert_eq!(id.as_str(), "process-123");
    }

    // ===== ArtifactType Tests =====

    #[test]
    fn artifact_type_all_returns_18_types() {
        let all = ArtifactType::all();
        assert_eq!(all.len(), 18);
    }

    #[test]
    fn artifact_type_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&ArtifactType::Prd).unwrap(),
            "\"prd\""
        );
        assert_eq!(
            serde_json::to_string(&ArtifactType::ResearchDocument).unwrap(),
            "\"research_document\""
        );
        assert_eq!(
            serde_json::to_string(&ArtifactType::CodeChange).unwrap(),
            "\"code_change\""
        );
        assert_eq!(
            serde_json::to_string(&ArtifactType::ReviewFeedback).unwrap(),
            "\"review_feedback\""
        );
    }

    #[test]
    fn artifact_type_deserializes() {
        let t: ArtifactType = serde_json::from_str("\"prd\"").unwrap();
        assert_eq!(t, ArtifactType::Prd);
        let t: ArtifactType = serde_json::from_str("\"research_document\"").unwrap();
        assert_eq!(t, ArtifactType::ResearchDocument);
    }

    #[test]
    fn artifact_type_from_str() {
        assert_eq!(ArtifactType::from_str("prd").unwrap(), ArtifactType::Prd);
        assert_eq!(
            ArtifactType::from_str("research_document").unwrap(),
            ArtifactType::ResearchDocument
        );
        assert_eq!(
            ArtifactType::from_str("design_doc").unwrap(),
            ArtifactType::DesignDoc
        );
    }

    #[test]
    fn artifact_type_from_str_error() {
        let err = ArtifactType::from_str("invalid").unwrap_err();
        assert_eq!(err.value, "invalid");
        assert!(err.to_string().contains("invalid"));
    }

    #[test]
    fn artifact_type_display() {
        assert_eq!(ArtifactType::Prd.to_string(), "prd");
        assert_eq!(ArtifactType::ResearchDocument.to_string(), "research_document");
    }

    // ===== ArtifactContent Tests =====

    #[test]
    fn artifact_content_inline_creates_correctly() {
        let content = ArtifactContent::inline("Hello world");
        assert!(content.is_inline());
        assert!(!content.is_file());
        assert_eq!(content.content_type(), "inline");
    }

    #[test]
    fn artifact_content_file_creates_correctly() {
        let content = ArtifactContent::file("/path/to/file.md");
        assert!(content.is_file());
        assert!(!content.is_inline());
        assert_eq!(content.content_type(), "file");
    }

    #[test]
    fn artifact_content_inline_serializes() {
        let content = ArtifactContent::inline("Test content");
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"inline\""));
        assert!(json.contains("\"text\":\"Test content\""));
    }

    #[test]
    fn artifact_content_file_serializes() {
        let content = ArtifactContent::file("/path/to/file");
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"file\""));
        assert!(json.contains("\"path\":\"/path/to/file\""));
    }

    #[test]
    fn artifact_content_deserializes_inline() {
        let json = r#"{"type":"inline","text":"Hello"}"#;
        let content: ArtifactContent = serde_json::from_str(json).unwrap();
        assert!(content.is_inline());
        if let ArtifactContent::Inline { text } = content {
            assert_eq!(text, "Hello");
        } else {
            panic!("Expected inline content");
        }
    }

    #[test]
    fn artifact_content_deserializes_file() {
        let json = r#"{"type":"file","path":"/test/path"}"#;
        let content: ArtifactContent = serde_json::from_str(json).unwrap();
        assert!(content.is_file());
        if let ArtifactContent::File { path } = content {
            assert_eq!(path, "/test/path");
        } else {
            panic!("Expected file content");
        }
    }

    // ===== ArtifactMetadata Tests =====

    #[test]
    fn artifact_metadata_new_sets_defaults() {
        let meta = ArtifactMetadata::new("user");
        assert_eq!(meta.created_by, "user");
        assert_eq!(meta.version, 1);
        assert!(meta.task_id.is_none());
        assert!(meta.process_id.is_none());
    }

    #[test]
    fn artifact_metadata_with_task() {
        let task_id = TaskId::from_string("task-1".to_string());
        let meta = ArtifactMetadata::new("user").with_task(task_id.clone());
        assert_eq!(meta.task_id, Some(task_id));
    }

    #[test]
    fn artifact_metadata_with_process() {
        let process_id = ProcessId::from_string("process-1");
        let meta = ArtifactMetadata::new("user").with_process(process_id.clone());
        assert_eq!(meta.process_id, Some(process_id));
    }

    #[test]
    fn artifact_metadata_with_version() {
        let meta = ArtifactMetadata::new("user").with_version(5);
        assert_eq!(meta.version, 5);
    }

    // ===== Artifact Tests =====

    #[test]
    fn artifact_new_inline_creates_correctly() {
        let artifact = Artifact::new_inline("Test PRD", ArtifactType::Prd, "PRD content", "user");
        assert_eq!(artifact.name, "Test PRD");
        assert_eq!(artifact.artifact_type, ArtifactType::Prd);
        assert!(artifact.content.is_inline());
        assert_eq!(artifact.metadata.created_by, "user");
        assert!(artifact.derived_from.is_empty());
        assert!(artifact.bucket_id.is_none());
    }

    #[test]
    fn artifact_new_file_creates_correctly() {
        let artifact =
            Artifact::new_file("Design Doc", ArtifactType::DesignDoc, "/docs/design.md", "system");
        assert_eq!(artifact.name, "Design Doc");
        assert_eq!(artifact.artifact_type, ArtifactType::DesignDoc);
        assert!(artifact.content.is_file());
        assert_eq!(artifact.metadata.created_by, "system");
    }

    #[test]
    fn artifact_with_bucket() {
        let bucket_id = ArtifactBucketId::from_string("bucket-1");
        let artifact = Artifact::new_inline("Test", ArtifactType::Prd, "Content", "user")
            .with_bucket(bucket_id.clone());
        assert_eq!(artifact.bucket_id, Some(bucket_id));
    }

    #[test]
    fn artifact_derived_from_artifact() {
        let parent_id = ArtifactId::from_string("parent-1");
        let artifact = Artifact::new_inline("Child", ArtifactType::Findings, "Content", "agent")
            .derived_from_artifact(parent_id.clone());
        assert_eq!(artifact.derived_from.len(), 1);
        assert_eq!(artifact.derived_from[0], parent_id);
    }

    #[test]
    fn artifact_with_task() {
        let task_id = TaskId::from_string("task-1".to_string());
        let artifact = Artifact::new_inline("Test", ArtifactType::CodeChange, "Code", "worker")
            .with_task(task_id.clone());
        assert_eq!(artifact.metadata.task_id, Some(task_id));
    }

    #[test]
    fn artifact_serializes_roundtrip() {
        let artifact = Artifact::new_inline("Test", ArtifactType::Prd, "Content", "user")
            .with_bucket(ArtifactBucketId::from_string("bucket-1"))
            .derived_from_artifact(ArtifactId::from_string("parent-1"));
        let json = serde_json::to_string(&artifact).unwrap();
        let parsed: Artifact = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, artifact.name);
        assert_eq!(parsed.artifact_type, artifact.artifact_type);
        assert_eq!(parsed.bucket_id, artifact.bucket_id);
        assert_eq!(parsed.derived_from.len(), 1);
    }

    // ===== ArtifactBucket Tests =====

    #[test]
    fn artifact_bucket_new_creates_custom() {
        let bucket = ArtifactBucket::new("My Bucket");
        assert_eq!(bucket.name, "My Bucket");
        assert!(!bucket.is_system);
        assert!(bucket.accepted_types.is_empty());
    }

    #[test]
    fn artifact_bucket_system_creates_with_id() {
        let bucket = ArtifactBucket::system("research-outputs", "Research Outputs");
        assert_eq!(bucket.id.as_str(), "research-outputs");
        assert_eq!(bucket.name, "Research Outputs");
        assert!(bucket.is_system);
    }

    #[test]
    fn artifact_bucket_accepts_type() {
        let bucket = ArtifactBucket::new("Test")
            .accepts(ArtifactType::Prd)
            .accepts(ArtifactType::DesignDoc);
        assert!(bucket.accepts_type(ArtifactType::Prd));
        assert!(bucket.accepts_type(ArtifactType::DesignDoc));
        assert!(!bucket.accepts_type(ArtifactType::CodeChange));
    }

    #[test]
    fn artifact_bucket_accepts_all_types_when_empty() {
        let bucket = ArtifactBucket::new("Test");
        assert!(bucket.accepts_type(ArtifactType::Prd));
        assert!(bucket.accepts_type(ArtifactType::CodeChange));
    }

    #[test]
    fn artifact_bucket_with_writer() {
        let bucket = ArtifactBucket::new("Test")
            .with_writer("worker")
            .with_writer("reviewer");
        assert!(bucket.can_write("worker"));
        assert!(bucket.can_write("reviewer"));
        assert!(!bucket.can_write("user"));
    }

    #[test]
    fn artifact_bucket_allows_all_writers_when_empty() {
        let bucket = ArtifactBucket::new("Test");
        assert!(bucket.can_write("anyone"));
    }

    #[test]
    fn artifact_bucket_with_reader() {
        let bucket = ArtifactBucket::new("Test").with_reader("worker");
        assert!(bucket.can_read("worker"));
        assert!(bucket.can_read("all")); // "all" is added by default
    }

    #[test]
    fn artifact_bucket_system_buckets_returns_4() {
        let buckets = ArtifactBucket::system_buckets();
        assert_eq!(buckets.len(), 4);

        let ids: Vec<&str> = buckets.iter().map(|b| b.id.as_str()).collect();
        assert!(ids.contains(&"research-outputs"));
        assert!(ids.contains(&"work-context"));
        assert!(ids.contains(&"code-changes"));
        assert!(ids.contains(&"prd-library"));
    }

    #[test]
    fn artifact_bucket_research_outputs_has_correct_types() {
        let buckets = ArtifactBucket::system_buckets();
        let research = buckets.iter().find(|b| b.id.as_str() == "research-outputs").unwrap();
        assert!(research.accepts_type(ArtifactType::ResearchDocument));
        assert!(research.accepts_type(ArtifactType::Findings));
        assert!(research.accepts_type(ArtifactType::Recommendations));
        assert!(!research.accepts_type(ArtifactType::CodeChange));
    }

    #[test]
    fn artifact_bucket_serializes() {
        let bucket = ArtifactBucket::new("Test").accepts(ArtifactType::Prd);
        let json = serde_json::to_string(&bucket).unwrap();
        assert!(json.contains("\"name\":\"Test\""));
        assert!(json.contains("\"prd\""));
    }

    // ===== ArtifactRelationType Tests =====

    #[test]
    fn artifact_relation_type_serializes() {
        assert_eq!(
            serde_json::to_string(&ArtifactRelationType::DerivedFrom).unwrap(),
            "\"derived_from\""
        );
        assert_eq!(
            serde_json::to_string(&ArtifactRelationType::RelatedTo).unwrap(),
            "\"related_to\""
        );
    }

    #[test]
    fn artifact_relation_type_deserializes() {
        let t: ArtifactRelationType = serde_json::from_str("\"derived_from\"").unwrap();
        assert_eq!(t, ArtifactRelationType::DerivedFrom);
    }

    #[test]
    fn artifact_relation_type_from_str() {
        assert_eq!(
            ArtifactRelationType::from_str("derived_from").unwrap(),
            ArtifactRelationType::DerivedFrom
        );
        assert_eq!(
            ArtifactRelationType::from_str("related_to").unwrap(),
            ArtifactRelationType::RelatedTo
        );
    }

    #[test]
    fn artifact_relation_type_from_str_error() {
        let err = ArtifactRelationType::from_str("invalid").unwrap_err();
        assert_eq!(err.value, "invalid");
    }

    #[test]
    fn artifact_relation_type_display() {
        assert_eq!(ArtifactRelationType::DerivedFrom.to_string(), "derived_from");
        assert_eq!(ArtifactRelationType::RelatedTo.to_string(), "related_to");
    }

    // ===== ArtifactRelationId Tests =====

    #[test]
    fn artifact_relation_id_new_generates_valid_uuid() {
        let id = ArtifactRelationId::new();
        assert_eq!(id.as_str().len(), 36);
        assert!(uuid::Uuid::parse_str(id.as_str()).is_ok());
    }

    #[test]
    fn artifact_relation_id_from_string_preserves_value() {
        let id = ArtifactRelationId::from_string("rel-123");
        assert_eq!(id.as_str(), "rel-123");
    }

    // ===== ArtifactRelation Tests =====

    #[test]
    fn artifact_relation_new_creates_correctly() {
        let from = ArtifactId::from_string("from-1");
        let to = ArtifactId::from_string("to-1");
        let rel = ArtifactRelation::new(from.clone(), to.clone(), ArtifactRelationType::DerivedFrom);
        assert_eq!(rel.from_artifact_id, from);
        assert_eq!(rel.to_artifact_id, to);
        assert_eq!(rel.relation_type, ArtifactRelationType::DerivedFrom);
    }

    #[test]
    fn artifact_relation_derived_from_helper() {
        let derived = ArtifactId::from_string("derived");
        let source = ArtifactId::from_string("source");
        let rel = ArtifactRelation::derived_from(derived.clone(), source.clone());
        assert_eq!(rel.from_artifact_id, derived);
        assert_eq!(rel.to_artifact_id, source);
        assert_eq!(rel.relation_type, ArtifactRelationType::DerivedFrom);
    }

    #[test]
    fn artifact_relation_related_to_helper() {
        let a = ArtifactId::from_string("a");
        let b = ArtifactId::from_string("b");
        let rel = ArtifactRelation::related_to(a.clone(), b.clone());
        assert_eq!(rel.from_artifact_id, a);
        assert_eq!(rel.to_artifact_id, b);
        assert_eq!(rel.relation_type, ArtifactRelationType::RelatedTo);
    }

    #[test]
    fn artifact_relation_serializes() {
        let rel = ArtifactRelation::derived_from(
            ArtifactId::from_string("from"),
            ArtifactId::from_string("to"),
        );
        let json = serde_json::to_string(&rel).unwrap();
        assert!(json.contains("\"from_artifact_id\":\"from\""));
        assert!(json.contains("\"to_artifact_id\":\"to\""));
        assert!(json.contains("\"relation_type\":\"derived_from\""));
    }
}
