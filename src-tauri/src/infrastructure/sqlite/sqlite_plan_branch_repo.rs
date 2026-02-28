// SQLite-based PlanBranchRepository implementation

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use rusqlite::Connection;

use super::DbConnection;
use crate::domain::entities::{
    ArtifactId, IdeationSessionId, PlanBranch, PlanBranchId, PlanBranchStatus, ProjectId, TaskId,
};
use crate::domain::repositories::PlanBranchRepository;
use crate::error::{AppError, AppResult};

pub struct SqlitePlanBranchRepository {
    db: DbConnection,
}

impl SqlitePlanBranchRepository {
    pub fn new(conn: Connection) -> Self {
        Self {
            db: DbConnection::new(conn),
        }
    }

    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            db: DbConnection::from_shared(conn),
        }
    }
}

#[async_trait]
impl PlanBranchRepository for SqlitePlanBranchRepository {
    async fn create(&self, branch: PlanBranch) -> AppResult<PlanBranch> {
        self.db
            .run(move |conn| {
                conn.execute(
                    "INSERT INTO plan_branches (id, plan_artifact_id, session_id, project_id, branch_name, source_branch, status, merge_task_id, created_at, merged_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    rusqlite::params![
                        branch.id.as_str(),
                        branch.plan_artifact_id.as_str(),
                        branch.session_id.as_str(),
                        branch.project_id.as_str(),
                        branch.branch_name,
                        branch.source_branch,
                        branch.status.to_db_string(),
                        branch.merge_task_id.as_ref().map(|t| t.as_str().to_string()),
                        branch.created_at.to_rfc3339(),
                        branch.merged_at.map(|dt| dt.to_rfc3339()),
                    ],
                )
                .map_err(|e| AppError::Database(format!("Failed to create plan branch: {}", e)))?;
                Ok(branch)
            })
            .await
    }

    async fn get_by_plan_artifact_id(&self, id: &ArtifactId) -> AppResult<Option<PlanBranch>> {
        let id = id.as_str().to_string();
        self.db
            .run(move |conn| {
                let mut stmt = conn
                    .prepare("SELECT * FROM plan_branches WHERE plan_artifact_id = ?1")
                    .map_err(|e| AppError::Database(format!("Failed to prepare query: {}", e)))?;
                let result = stmt.query_row(rusqlite::params![id.as_str()], |row| {
                    PlanBranch::from_row(row)
                });
                match result {
                    Ok(branch) => Ok(Some(branch)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(AppError::Database(format!(
                        "Failed to get plan branch by artifact id: {}",
                        e
                    ))),
                }
            })
            .await
    }

    async fn get_by_session_id(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<Option<PlanBranch>> {
        let session_id = session_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let mut stmt = conn
                    .prepare("SELECT * FROM plan_branches WHERE session_id = ?1")
                    .map_err(|e| AppError::Database(format!("Failed to prepare query: {}", e)))?;
                let result = stmt.query_row(rusqlite::params![session_id.as_str()], |row| {
                    PlanBranch::from_row(row)
                });
                match result {
                    Ok(branch) => Ok(Some(branch)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(AppError::Database(format!(
                        "Failed to get plan branch by session id: {}",
                        e
                    ))),
                }
            })
            .await
    }

    async fn get_by_merge_task_id(&self, task_id: &TaskId) -> AppResult<Option<PlanBranch>> {
        let task_id = task_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let mut stmt = conn
                    .prepare("SELECT * FROM plan_branches WHERE merge_task_id = ?1")
                    .map_err(|e| AppError::Database(format!("Failed to prepare query: {}", e)))?;
                let result = stmt.query_row(rusqlite::params![task_id.as_str()], |row| {
                    PlanBranch::from_row(row)
                });
                match result {
                    Ok(branch) => Ok(Some(branch)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(AppError::Database(format!(
                        "Failed to get plan branch by merge task id: {}",
                        e
                    ))),
                }
            })
            .await
    }

    async fn get_by_project_id(&self, project_id: &ProjectId) -> AppResult<Vec<PlanBranch>> {
        let project_id = project_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let mut stmt = conn
                    .prepare(
                        "SELECT * FROM plan_branches WHERE project_id = ?1 ORDER BY created_at DESC",
                    )
                    .map_err(|e| {
                        AppError::Database(format!("Failed to prepare query: {}", e))
                    })?;
                let branches = stmt
                    .query_map(rusqlite::params![project_id.as_str()], |row| {
                        PlanBranch::from_row(row)
                    })
                    .map_err(|e| {
                        AppError::Database(format!("Failed to query plan branches: {}", e))
                    })?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| {
                        AppError::Database(format!("Failed to collect plan branches: {}", e))
                    })?;
                Ok(branches)
            })
            .await
    }

    async fn update_status(&self, id: &PlanBranchId, status: PlanBranchStatus) -> AppResult<()> {
        let id = id.as_str().to_string();
        let id_display = id.clone();
        self.db
            .run(move |conn| {
                let rows = conn
                    .execute(
                        "UPDATE plan_branches SET status = ?1 WHERE id = ?2",
                        rusqlite::params![status.to_db_string(), id.as_str()],
                    )
                    .map_err(|e| {
                        AppError::Database(format!("Failed to update plan branch status: {}", e))
                    })?;
                if rows == 0 {
                    return Err(AppError::NotFound(format!(
                        "Plan branch not found: {}",
                        id_display
                    )));
                }
                Ok(())
            })
            .await
    }

    async fn set_merge_task_id(&self, id: &PlanBranchId, task_id: &TaskId) -> AppResult<()> {
        let id = id.as_str().to_string();
        let task_id = task_id.as_str().to_string();
        let id_display = id.clone();
        self.db
            .run(move |conn| {
                let rows = conn
                    .execute(
                        "UPDATE plan_branches SET merge_task_id = ?1 WHERE id = ?2",
                        rusqlite::params![task_id.as_str(), id.as_str()],
                    )
                    .map_err(|e| {
                        AppError::Database(format!("Failed to set merge task id: {}", e))
                    })?;
                if rows == 0 {
                    return Err(AppError::NotFound(format!(
                        "Plan branch not found: {}",
                        id_display
                    )));
                }
                Ok(())
            })
            .await
    }

    async fn clear_merge_task_id(&self, id: &PlanBranchId) -> AppResult<()> {
        let id = id.as_str().to_string();
        let id_display = id.clone();
        self.db
            .run(move |conn| {
                let rows = conn
                    .execute(
                        "UPDATE plan_branches SET merge_task_id = NULL WHERE id = ?1",
                        rusqlite::params![id.as_str()],
                    )
                    .map_err(|e| {
                        AppError::Database(format!("Failed to clear merge task id: {}", e))
                    })?;
                if rows == 0 {
                    return Err(AppError::NotFound(format!(
                        "Plan branch not found: {}",
                        id_display
                    )));
                }
                Ok(())
            })
            .await
    }

    async fn set_merged(&self, id: &PlanBranchId) -> AppResult<()> {
        let id = id.as_str().to_string();
        let id_display = id.clone();
        self.db
            .run(move |conn| {
                let rows = conn
                    .execute(
                        "UPDATE plan_branches SET status = 'merged', merged_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now') WHERE id = ?1",
                        rusqlite::params![id.as_str()],
                    )
                    .map_err(|e| {
                        AppError::Database(format!("Failed to set plan branch merged: {}", e))
                    })?;
                if rows == 0 {
                    return Err(AppError::NotFound(format!(
                        "Plan branch not found: {}",
                        id_display
                    )));
                }
                Ok(())
            })
            .await
    }

    async fn delete(&self, id: &PlanBranchId) -> AppResult<()> {
        let id = id.as_str().to_string();
        let id_display = id.clone();
        self.db
            .run(move |conn| {
                let rows = conn
                    .execute(
                        "DELETE FROM plan_branches WHERE id = ?1",
                        rusqlite::params![id.as_str()],
                    )
                    .map_err(|e| {
                        AppError::Database(format!("Failed to delete plan branch: {}", e))
                    })?;
                if rows == 0 {
                    return Err(AppError::NotFound(format!(
                        "Plan branch not found: {}",
                        id_display
                    )));
                }
                Ok(())
            })
            .await
    }
}

#[cfg(test)]
#[path = "sqlite_plan_branch_repo_tests.rs"]
mod tests;
