// Memory archive entities - job queue for snapshot generation
//
// This module defines the entities for the memory archive system,
// which generates deterministic markdown snapshots from canonical DB state.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::domain::entities::types::ProjectId;
use crate::error::{AppError, AppResult};

/// Unique identifier for memory archive jobs
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryArchiveJobId(pub String);

impl fmt::Display for MemoryArchiveJobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for MemoryArchiveJobId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for MemoryArchiveJobId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Type of archive snapshot to generate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveJobType {
    /// Per-memory snapshot: .claude/memory-archive/memories/<memory_id>.md
    MemorySnapshot,
    /// Per-rule reconstruction: .claude/memory-archive/rules/<scope_key>/<timestamp>.md
    RuleSnapshot,
    /// Full project rebuild: .claude/memory-archive/projects/<project_id>/<timestamp>.md
    FullRebuild,
}

impl fmt::Display for ArchiveJobType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MemorySnapshot => write!(f, "memory_snapshot"),
            Self::RuleSnapshot => write!(f, "rule_snapshot"),
            Self::FullRebuild => write!(f, "full_rebuild"),
        }
    }
}

impl FromStr for ArchiveJobType {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "memory_snapshot" => Ok(Self::MemorySnapshot),
            "rule_snapshot" => Ok(Self::RuleSnapshot),
            "full_rebuild" => Ok(Self::FullRebuild),
            _ => Err(AppError::Validation(format!(
                "Invalid archive job type: {}",
                s
            ))),
        }
    }
}

/// Status of an archive job
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveJobStatus {
    /// Job is waiting to be processed
    Pending,
    /// Job is currently being processed
    Running,
    /// Job completed successfully
    Done,
    /// Job failed with error
    Failed,
}

impl fmt::Display for ArchiveJobStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Done => write!(f, "done"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

impl FromStr for ArchiveJobStatus {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "done" => Ok(Self::Done),
            "failed" => Ok(Self::Failed),
            _ => Err(AppError::Validation(format!(
                "Invalid archive job status: {}",
                s
            ))),
        }
    }
}

/// Payload for memory snapshot jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshotPayload {
    pub memory_id: String,
}

/// Payload for rule snapshot jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSnapshotPayload {
    pub scope_key: String,
}

/// Payload for full rebuild jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullRebuildPayload {
    pub include_rule_snapshots: bool,
}

/// Archive job payload variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ArchiveJobPayload {
    MemorySnapshot(MemorySnapshotPayload),
    RuleSnapshot(RuleSnapshotPayload),
    FullRebuild(FullRebuildPayload),
}

impl ArchiveJobPayload {
    /// Create a memory snapshot payload
    pub fn memory_snapshot(memory_id: impl Into<String>) -> Self {
        Self::MemorySnapshot(MemorySnapshotPayload {
            memory_id: memory_id.into(),
        })
    }

    /// Create a rule snapshot payload
    pub fn rule_snapshot(scope_key: impl Into<String>) -> Self {
        Self::RuleSnapshot(RuleSnapshotPayload {
            scope_key: scope_key.into(),
        })
    }

    /// Create a full rebuild payload
    pub fn full_rebuild(include_rule_snapshots: bool) -> Self {
        Self::FullRebuild(FullRebuildPayload {
            include_rule_snapshots,
        })
    }

    /// Serialize to JSON string for database storage
    pub fn to_json(&self) -> AppResult<String> {
        serde_json::to_string(self).map_err(|e| AppError::Infrastructure(format!("JSON serialization error: {}", e)))
    }

    /// Deserialize from JSON string
    pub fn from_json(json: &str) -> AppResult<Self> {
        serde_json::from_str(json).map_err(|e| AppError::Infrastructure(format!("JSON deserialization error: {}", e)))
    }
}

/// Memory archive job entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryArchiveJob {
    pub id: MemoryArchiveJobId,
    pub project_id: ProjectId,
    pub job_type: ArchiveJobType,
    pub payload: ArchiveJobPayload,
    pub status: ArchiveJobStatus,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl MemoryArchiveJob {
    /// Create a new pending archive job
    pub fn new(
        project_id: ProjectId,
        job_type: ArchiveJobType,
        payload: ArchiveJobPayload,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: MemoryArchiveJobId(uuid::Uuid::new_v4().to_string()),
            project_id,
            job_type,
            payload,
            status: ArchiveJobStatus::Pending,
            error_message: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
        }
    }

    /// Mark job as running
    pub fn start(&mut self) {
        self.status = ArchiveJobStatus::Running;
        self.started_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Mark job as completed successfully
    pub fn complete(&mut self) {
        self.status = ArchiveJobStatus::Done;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Mark job as failed with error message
    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = ArchiveJobStatus::Failed;
        self.error_message = Some(error.into());
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Check if job can be claimed (is pending or failed)
    pub fn can_claim(&self) -> bool {
        matches!(
            self.status,
            ArchiveJobStatus::Pending | ArchiveJobStatus::Failed
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archive_job_type_serialization() {
        assert_eq!(
            ArchiveJobType::MemorySnapshot.to_string(),
            "memory_snapshot"
        );
        assert_eq!(ArchiveJobType::RuleSnapshot.to_string(), "rule_snapshot");
        assert_eq!(ArchiveJobType::FullRebuild.to_string(), "full_rebuild");
    }

    #[test]
    fn test_archive_job_type_parsing() {
        assert_eq!(
            "memory_snapshot".parse::<ArchiveJobType>().unwrap(),
            ArchiveJobType::MemorySnapshot
        );
        assert_eq!(
            "rule_snapshot".parse::<ArchiveJobType>().unwrap(),
            ArchiveJobType::RuleSnapshot
        );
        assert_eq!(
            "full_rebuild".parse::<ArchiveJobType>().unwrap(),
            ArchiveJobType::FullRebuild
        );
        assert!("invalid".parse::<ArchiveJobType>().is_err());
    }

    #[test]
    fn test_archive_job_status_serialization() {
        assert_eq!(ArchiveJobStatus::Pending.to_string(), "pending");
        assert_eq!(ArchiveJobStatus::Running.to_string(), "running");
        assert_eq!(ArchiveJobStatus::Done.to_string(), "done");
        assert_eq!(ArchiveJobStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn test_archive_job_status_parsing() {
        assert_eq!(
            "pending".parse::<ArchiveJobStatus>().unwrap(),
            ArchiveJobStatus::Pending
        );
        assert_eq!(
            "running".parse::<ArchiveJobStatus>().unwrap(),
            ArchiveJobStatus::Running
        );
        assert_eq!(
            "done".parse::<ArchiveJobStatus>().unwrap(),
            ArchiveJobStatus::Done
        );
        assert_eq!(
            "failed".parse::<ArchiveJobStatus>().unwrap(),
            ArchiveJobStatus::Failed
        );
        assert!("invalid".parse::<ArchiveJobStatus>().is_err());
    }

    #[test]
    fn test_memory_archive_job_lifecycle() {
        let project_id = ProjectId::from("test-project");
        let payload = ArchiveJobPayload::memory_snapshot("mem_123");
        let mut job = MemoryArchiveJob::new(project_id, ArchiveJobType::MemorySnapshot, payload);

        // Initially pending
        assert_eq!(job.status, ArchiveJobStatus::Pending);
        assert!(job.can_claim());
        assert!(job.started_at.is_none());
        assert!(job.completed_at.is_none());

        // Start job
        job.start();
        assert_eq!(job.status, ArchiveJobStatus::Running);
        assert!(!job.can_claim());
        assert!(job.started_at.is_some());
        assert!(job.completed_at.is_none());

        // Complete job
        job.complete();
        assert_eq!(job.status, ArchiveJobStatus::Done);
        assert!(!job.can_claim());
        assert!(job.completed_at.is_some());
        assert!(job.error_message.is_none());
    }

    #[test]
    fn test_memory_archive_job_failure() {
        let project_id = ProjectId::from("test-project");
        let payload = ArchiveJobPayload::memory_snapshot("mem_123");
        let mut job = MemoryArchiveJob::new(project_id, ArchiveJobType::MemorySnapshot, payload);

        job.start();
        job.fail("Test error");

        assert_eq!(job.status, ArchiveJobStatus::Failed);
        assert!(job.can_claim()); // Failed jobs can be retried
        assert!(job.completed_at.is_some());
        assert_eq!(job.error_message.as_deref(), Some("Test error"));
    }

    #[test]
    fn test_archive_job_payload_json_roundtrip() {
        let payload = ArchiveJobPayload::memory_snapshot("mem_123");
        let json = payload.to_json().unwrap();
        let parsed = ArchiveJobPayload::from_json(&json).unwrap();

        match parsed {
            ArchiveJobPayload::MemorySnapshot(p) => assert_eq!(p.memory_id, "mem_123"),
            _ => panic!("Wrong payload type"),
        }
    }
}
