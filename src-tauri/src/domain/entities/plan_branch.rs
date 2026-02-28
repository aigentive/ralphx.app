// PlanBranch entity - represents a feature branch created for a plan group
//
// Links a plan artifact to a git feature branch. Tasks within the plan
// merge into this branch instead of main, and a final merge task merges
// the feature branch into main when all work is complete.

use chrono::{DateTime, TimeZone, Utc};
use rusqlite::Row;
use serde::{Deserialize, Serialize};

use super::artifact::ArtifactId;
use super::types::{ExecutionPlanId, IdeationSessionId, ProjectId, TaskId};

/// A unique identifier for a PlanBranch
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlanBranchId(pub String);

impl PlanBranchId {
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

impl Default for PlanBranchId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PlanBranchId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Status of a plan feature branch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanBranchStatus {
    Active,
    Merged,
    Abandoned,
}

impl PlanBranchStatus {
    pub fn to_db_string(&self) -> &'static str {
        match self {
            PlanBranchStatus::Active => "active",
            PlanBranchStatus::Merged => "merged",
            PlanBranchStatus::Abandoned => "abandoned",
        }
    }

    pub fn from_db_string(s: &str) -> Result<Self, ParsePlanBranchStatusError> {
        match s {
            "active" => Ok(PlanBranchStatus::Active),
            "merged" => Ok(PlanBranchStatus::Merged),
            "abandoned" => Ok(PlanBranchStatus::Abandoned),
            _ => Err(ParsePlanBranchStatusError(s.to_string())),
        }
    }
}

impl std::fmt::Display for PlanBranchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_db_string())
    }
}

/// Error when parsing PlanBranchStatus from a string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsePlanBranchStatusError(pub String);

impl std::fmt::Display for ParsePlanBranchStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown plan branch status: '{}'", self.0)
    }
}

impl std::error::Error for ParsePlanBranchStatusError {}

/// A feature branch created for a plan group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanBranch {
    pub id: PlanBranchId,
    pub plan_artifact_id: ArtifactId,
    pub session_id: IdeationSessionId,
    pub project_id: ProjectId,
    pub branch_name: String,
    pub source_branch: String,
    pub status: PlanBranchStatus,
    pub merge_task_id: Option<TaskId>,
    pub created_at: DateTime<Utc>,
    pub merged_at: Option<DateTime<Utc>>,
    /// Execution plan this branch belongs to (set at proposal-apply time)
    /// Used to look up the branch for a specific execution attempt
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_plan_id: Option<ExecutionPlanId>,
}

impl PlanBranch {
    pub fn new(
        plan_artifact_id: ArtifactId,
        session_id: IdeationSessionId,
        project_id: ProjectId,
        branch_name: String,
        source_branch: String,
    ) -> Self {
        Self {
            id: PlanBranchId::new(),
            plan_artifact_id,
            session_id,
            project_id,
            branch_name,
            source_branch,
            status: PlanBranchStatus::Active,
            merge_task_id: None,
            created_at: Utc::now(),
            merged_at: None,
            execution_plan_id: None,
        }
    }

    /// Deserialize a PlanBranch from a SQLite row
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let status_str: String = row.get("status")?;
        let status = PlanBranchStatus::from_db_string(&status_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?;

        let merged_at: Option<DateTime<Utc>> = row
            .get::<_, Option<String>>("merged_at")?
            .map(|s| Self::parse_datetime(s));

        Ok(Self {
            id: PlanBranchId::from_string(row.get::<_, String>("id")?),
            plan_artifact_id: ArtifactId::from_string(row.get::<_, String>("plan_artifact_id")?),
            session_id: IdeationSessionId::from_string(row.get::<_, String>("session_id")?),
            project_id: ProjectId::from_string(row.get::<_, String>("project_id")?),
            branch_name: row.get("branch_name")?,
            source_branch: row.get("source_branch")?,
            status,
            merge_task_id: row
                .get::<_, Option<String>>("merge_task_id")?
                .map(TaskId::from_string),
            created_at: Self::parse_datetime(row.get::<_, String>("created_at")?),
            merged_at,
            execution_plan_id: row
                .get::<_, Option<String>>("execution_plan_id")?
                .map(ExecutionPlanId::from_string),
        })
    }

    fn parse_datetime(s: String) -> DateTime<Utc> {
        if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
            return dt.with_timezone(&Utc);
        }
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S") {
            return Utc.from_utc_datetime(&dt);
        }
        Utc::now()
    }
}

#[cfg(test)]
#[path = "plan_branch_tests.rs"]
mod tests;
