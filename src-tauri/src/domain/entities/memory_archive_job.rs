// Memory archive job entity for background snapshot generation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::fmt;

use super::ProcessId;

/// Unique identifier for a memory archive job
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryArchiveJobId(pub String);

impl MemoryArchiveJobId {
    /// Creates a new MemoryArchiveJobId with a random UUID v4
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Creates a MemoryArchiveJobId from an existing string
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for MemoryArchiveJobId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for MemoryArchiveJobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Type of archive job
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryArchiveJobType {
    /// Snapshot a single memory entry
    MemorySnapshot,
    /// Snapshot a full rule reconstruction
    RuleSnapshot,
    /// Full rebuild of all archives
    FullRebuild,
}

impl fmt::Display for MemoryArchiveJobType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryArchiveJobType::MemorySnapshot => write!(f, "memory_snapshot"),
            MemoryArchiveJobType::RuleSnapshot => write!(f, "rule_snapshot"),
            MemoryArchiveJobType::FullRebuild => write!(f, "full_rebuild"),
        }
    }
}

impl std::str::FromStr for MemoryArchiveJobType {
    type Err = ParseMemoryArchiveJobTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "memory_snapshot" => Ok(MemoryArchiveJobType::MemorySnapshot),
            "rule_snapshot" => Ok(MemoryArchiveJobType::RuleSnapshot),
            "full_rebuild" => Ok(MemoryArchiveJobType::FullRebuild),
            _ => Err(ParseMemoryArchiveJobTypeError(s.to_string())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParseMemoryArchiveJobTypeError(String);

impl fmt::Display for ParseMemoryArchiveJobTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid memory archive job type: {}", self.0)
    }
}

impl std::error::Error for ParseMemoryArchiveJobTypeError {}

/// Memory archive job status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryArchiveJobStatus {
    Pending,
    Running,
    Done,
    Failed,
}

impl fmt::Display for MemoryArchiveJobStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryArchiveJobStatus::Pending => write!(f, "pending"),
            MemoryArchiveJobStatus::Running => write!(f, "running"),
            MemoryArchiveJobStatus::Done => write!(f, "done"),
            MemoryArchiveJobStatus::Failed => write!(f, "failed"),
        }
    }
}

impl std::str::FromStr for MemoryArchiveJobStatus {
    type Err = ParseMemoryArchiveJobStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(MemoryArchiveJobStatus::Pending),
            "running" => Ok(MemoryArchiveJobStatus::Running),
            "done" => Ok(MemoryArchiveJobStatus::Done),
            "failed" => Ok(MemoryArchiveJobStatus::Failed),
            _ => Err(ParseMemoryArchiveJobStatusError(s.to_string())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParseMemoryArchiveJobStatusError(String);

impl fmt::Display for ParseMemoryArchiveJobStatusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid memory archive job status: {}", self.0)
    }
}

impl std::error::Error for ParseMemoryArchiveJobStatusError {}

/// Memory archive job for background snapshot generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryArchiveJob {
    pub id: MemoryArchiveJobId,
    pub project_id: ProcessId,
    pub job_type: MemoryArchiveJobType,
    pub payload: JsonValue,
    pub status: MemoryArchiveJobStatus,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl MemoryArchiveJob {
    /// Create a new pending archive job
    pub fn new(
        project_id: ProcessId,
        job_type: MemoryArchiveJobType,
        payload: JsonValue,
    ) -> Self {
        let now = Utc::now();

        Self {
            id: MemoryArchiveJobId::new(),
            project_id,
            job_type,
            payload,
            status: MemoryArchiveJobStatus::Pending,
            error_message: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
        }
    }
}
