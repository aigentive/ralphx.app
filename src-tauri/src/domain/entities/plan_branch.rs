// PlanBranch entity - represents a feature branch created for a plan group
//
// Links a plan artifact to a git feature branch. Tasks within the plan
// merge into this branch instead of main, and a final merge task merges
// the feature branch into main when all work is complete.

use chrono::{DateTime, TimeZone, Utc};
use rusqlite::Row;
use serde::{Deserialize, Serialize};

use super::artifact::ArtifactId;
use super::types::{IdeationSessionId, ProjectId, TaskId};

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
        }
    }

    /// Deserialize a PlanBranch from a SQLite row
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let status_str: String = row.get("status")?;
        let status = PlanBranchStatus::from_db_string(&status_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(e),
            )
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
mod tests {
    use super::*;

    #[test]
    fn plan_branch_id_new_generates_valid_uuid() {
        let id = PlanBranchId::new();
        assert_eq!(id.as_str().len(), 36);
        assert!(uuid::Uuid::parse_str(id.as_str()).is_ok());
    }

    #[test]
    fn plan_branch_id_from_string_preserves_value() {
        let id = PlanBranchId::from_string("pb-custom-id");
        assert_eq!(id.as_str(), "pb-custom-id");
    }

    #[test]
    fn plan_branch_id_display_works() {
        let id = PlanBranchId::from_string("pb-display");
        assert_eq!(format!("{}", id), "pb-display");
    }

    #[test]
    fn plan_branch_id_equality_works() {
        let id1 = PlanBranchId::from_string("pb-abc");
        let id2 = PlanBranchId::from_string("pb-abc");
        let id3 = PlanBranchId::from_string("pb-xyz");
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn plan_branch_id_serializes_to_json() {
        let id = PlanBranchId::from_string("pb-serialize");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"pb-serialize\"");
    }

    #[test]
    fn plan_branch_id_deserializes_from_json() {
        let id: PlanBranchId = serde_json::from_str("\"pb-deser\"").unwrap();
        assert_eq!(id.as_str(), "pb-deser");
    }

    #[test]
    fn plan_branch_status_to_db_string() {
        assert_eq!(PlanBranchStatus::Active.to_db_string(), "active");
        assert_eq!(PlanBranchStatus::Merged.to_db_string(), "merged");
        assert_eq!(PlanBranchStatus::Abandoned.to_db_string(), "abandoned");
    }

    #[test]
    fn plan_branch_status_from_db_string() {
        assert_eq!(
            PlanBranchStatus::from_db_string("active").unwrap(),
            PlanBranchStatus::Active
        );
        assert_eq!(
            PlanBranchStatus::from_db_string("merged").unwrap(),
            PlanBranchStatus::Merged
        );
        assert_eq!(
            PlanBranchStatus::from_db_string("abandoned").unwrap(),
            PlanBranchStatus::Abandoned
        );
    }

    #[test]
    fn plan_branch_status_from_db_string_invalid() {
        let result = PlanBranchStatus::from_db_string("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn plan_branch_status_serializes_to_snake_case() {
        let json = serde_json::to_string(&PlanBranchStatus::Active).unwrap();
        assert_eq!(json, "\"active\"");
    }

    #[test]
    fn plan_branch_status_deserializes_from_snake_case() {
        let status: PlanBranchStatus = serde_json::from_str("\"merged\"").unwrap();
        assert_eq!(status, PlanBranchStatus::Merged);
    }

    #[test]
    fn plan_branch_status_display() {
        assert_eq!(format!("{}", PlanBranchStatus::Active), "active");
        assert_eq!(format!("{}", PlanBranchStatus::Merged), "merged");
        assert_eq!(format!("{}", PlanBranchStatus::Abandoned), "abandoned");
    }

    #[test]
    fn plan_branch_new_creates_with_defaults() {
        let pb = PlanBranch::new(
            ArtifactId::from_string("art-1"),
            IdeationSessionId::from_string("sess-1"),
            ProjectId::from_string("proj-1".to_string()),
            "ralphx/my-app/plan-a1b2c3".to_string(),
            "main".to_string(),
        );

        assert_eq!(pb.plan_artifact_id.as_str(), "art-1");
        assert_eq!(pb.session_id.as_str(), "sess-1");
        assert_eq!(pb.project_id.as_str(), "proj-1");
        assert_eq!(pb.branch_name, "ralphx/my-app/plan-a1b2c3");
        assert_eq!(pb.source_branch, "main");
        assert_eq!(pb.status, PlanBranchStatus::Active);
        assert!(pb.merge_task_id.is_none());
        assert!(pb.merged_at.is_none());
    }

    #[test]
    fn plan_branch_serializes_to_json() {
        let pb = PlanBranch::new(
            ArtifactId::from_string("art-1"),
            IdeationSessionId::from_string("sess-1"),
            ProjectId::from_string("proj-1".to_string()),
            "ralphx/my-app/plan-a1b2c3".to_string(),
            "main".to_string(),
        );
        let json = serde_json::to_string(&pb).unwrap();
        assert!(json.contains("\"status\":\"active\""));
        assert!(json.contains("\"branch_name\":\"ralphx/my-app/plan-a1b2c3\""));
    }

    #[test]
    fn parse_plan_branch_status_error_display() {
        let err = ParsePlanBranchStatusError("bad".to_string());
        assert_eq!(err.to_string(), "unknown plan branch status: 'bad'");
    }
}
