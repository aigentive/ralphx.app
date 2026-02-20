// Memory entry entities - canonical memory storage
//
// This module defines the entities for memory entries, which are the
// canonical source of truth for project memory in the Memory Framework V2.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::domain::entities::types::ProjectId;
use crate::error::{AppError, AppResult};

/// Unique identifier for memory entries
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

impl From<String> for MemoryEntryId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for MemoryEntryId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl MemoryEntryId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Memory bucket taxonomy - exactly three buckets
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
            Self::ArchitecturePatterns => write!(f, "architecture_patterns"),
            Self::ImplementationDiscoveries => write!(f, "implementation_discoveries"),
            Self::OperationalPlaybooks => write!(f, "operational_playbooks"),
        }
    }
}

impl FromStr for MemoryBucket {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "architecture_patterns" => Ok(Self::ArchitecturePatterns),
            "implementation_discoveries" => Ok(Self::ImplementationDiscoveries),
            "operational_playbooks" => Ok(Self::OperationalPlaybooks),
            _ => Err(AppError::Validation(format!(
                "Invalid memory bucket: {}",
                s
            ))),
        }
    }
}

/// Memory entry status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryStatus {
    /// Active and current memory
    Active,
    /// Superseded by newer information
    Obsolete,
    /// Moved to archive
    Archived,
}

impl fmt::Display for MemoryStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Obsolete => write!(f, "obsolete"),
            Self::Archived => write!(f, "archived"),
        }
    }
}

impl FromStr for MemoryStatus {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "obsolete" => Ok(Self::Obsolete),
            "archived" => Ok(Self::Archived),
            _ => Err(AppError::Validation(format!(
                "Invalid memory status: {}",
                s
            ))),
        }
    }
}

/// Memory entry entity - canonical memory storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: MemoryEntryId,
    pub project_id: ProjectId,
    pub bucket: MemoryBucket,
    pub title: String,
    pub summary: String,
    pub details_markdown: String,
    pub scope_paths: Vec<String>,
    pub source_context_type: Option<String>,
    pub source_context_id: Option<String>,
    pub source_conversation_id: Option<String>,
    pub source_rule_file: Option<String>,
    pub quality_score: Option<f64>,
    pub status: MemoryStatus,
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
        project_id: ProjectId,
        bucket: MemoryBucket,
        title: String,
        summary: String,
        details_markdown: String,
        scope_paths: Vec<String>,
        content_hash: String,
    ) -> Self {
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

    /// Mark memory as obsolete
    pub fn mark_obsolete(&mut self) {
        self.status = MemoryStatus::Obsolete;
        self.updated_at = Utc::now();
    }

    /// Mark memory as archived
    pub fn mark_archived(&mut self) {
        self.status = MemoryStatus::Archived;
        self.updated_at = Utc::now();
    }

    /// Serialize scope_paths to JSON for database storage
    pub fn scope_paths_to_json(&self) -> AppResult<String> {
        serde_json::to_string(&self.scope_paths)
            .map_err(|e| AppError::Infrastructure(format!("JSON serialization error: {}", e)))
    }

    /// Deserialize scope_paths from JSON
    pub fn scope_paths_from_json(json: &str) -> AppResult<Vec<String>> {
        serde_json::from_str(json)
            .map_err(|e| AppError::Infrastructure(format!("JSON deserialization error: {}", e)))
    }
}

#[cfg(test)]
#[path = "memory_entry_tests.rs"]
mod tests;
