// Memory entry entity for Memory Framework V2

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

use super::ProcessId;

/// Unique identifier for a memory entry
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryEntryId(pub String);

impl MemoryEntryId {
    /// Creates a new MemoryEntryId with a random UUID v4
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Creates a MemoryEntryId from an existing string
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for MemoryEntryId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for MemoryEntryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Memory bucket taxonomy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryBucket {
    /// Subsystem relationships, state-machine behavior, invariant rules, complex data flows
    ArchitecturePatterns,
    /// Non-obvious code-level findings, framework quirks, migration gotchas
    ImplementationDiscoveries,
    /// Reproducible operational procedures, diagnostics, recovery tactics
    OperationalPlaybooks,
}

impl fmt::Display for MemoryBucket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryBucket::ArchitecturePatterns => write!(f, "architecture_patterns"),
            MemoryBucket::ImplementationDiscoveries => write!(f, "implementation_discoveries"),
            MemoryBucket::OperationalPlaybooks => write!(f, "operational_playbooks"),
        }
    }
}

impl std::str::FromStr for MemoryBucket {
    type Err = ParseMemoryBucketError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "architecture_patterns" => Ok(MemoryBucket::ArchitecturePatterns),
            "implementation_discoveries" => Ok(MemoryBucket::ImplementationDiscoveries),
            "operational_playbooks" => Ok(MemoryBucket::OperationalPlaybooks),
            _ => Err(ParseMemoryBucketError(s.to_string())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParseMemoryBucketError(String);

impl fmt::Display for ParseMemoryBucketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid memory bucket: {}", self.0)
    }
}

impl std::error::Error for ParseMemoryBucketError {}

/// Memory entry status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryStatus {
    /// Active memory entry
    Active,
    /// Marked as obsolete (soft delete)
    Obsolete,
    /// Archived to filesystem
    Archived,
}

impl fmt::Display for MemoryStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryStatus::Active => write!(f, "active"),
            MemoryStatus::Obsolete => write!(f, "obsolete"),
            MemoryStatus::Archived => write!(f, "archived"),
        }
    }
}

impl std::str::FromStr for MemoryStatus {
    type Err = ParseMemoryStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(MemoryStatus::Active),
            "obsolete" => Ok(MemoryStatus::Obsolete),
            "archived" => Ok(MemoryStatus::Archived),
            _ => Err(ParseMemoryStatusError(s.to_string())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParseMemoryStatusError(String);

impl fmt::Display for ParseMemoryStatusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid memory status: {}", self.0)
    }
}

impl std::error::Error for ParseMemoryStatusError {}

/// Canonical memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: MemoryEntryId,
    pub project_id: ProcessId,
    pub bucket: MemoryBucket,
    pub title: String,
    pub summary: String,
    pub details_markdown: String,
    /// Glob patterns for path scoping (e.g., ["src/domain/**", "src-tauri/src/application/**"])
    pub scope_paths: Vec<String>,
    /// Context type (e.g., "task_execution", "ideation", "review")
    pub source_context_type: Option<String>,
    /// Context ID (e.g., task_id, session_id)
    pub source_context_id: Option<String>,
    /// Conversation ID for traceability
    pub source_conversation_id: Option<String>,
    /// Rule file path if ingested from a rule file
    pub source_rule_file: Option<String>,
    /// Quality score (0.0-1.0)
    pub quality_score: Option<f64>,
    pub status: MemoryStatus,
    /// Content hash for deduplication (SHA-256 of title + summary + details_markdown)
    pub content_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl MemoryEntry {
    /// Compute content hash from memory content
    pub fn compute_content_hash(title: &str, summary: &str, details: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(title.as_bytes());
        hasher.update(summary.as_bytes());
        hasher.update(details.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Create a new memory entry
    pub fn new(
        project_id: ProcessId,
        bucket: MemoryBucket,
        title: String,
        summary: String,
        details_markdown: String,
        scope_paths: Vec<String>,
    ) -> Self {
        let content_hash = Self::compute_content_hash(&title, &summary, &details_markdown);
        let now = Utc::now();

        Self {
            id: MemoryEntryId::new(),
            project_id,
            bucket,
            title,
            summary,
            details_markdown,
            scope_paths,
            source_context_type: None,
            source_context_id: None,
            source_conversation_id: None,
            source_rule_file: None,
            quality_score: None,
            status: MemoryStatus::Active,
            content_hash,
            created_at: now,
            updated_at: now,
        }
    }
}
