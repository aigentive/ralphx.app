// SQLite-based PlanBranchRepository implementation

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use rusqlite::Connection;

use crate::domain::entities::{
    ArtifactId, PlanBranch, PlanBranchId, PlanBranchStatus, ProjectId, TaskId,
};
use crate::domain::repositories::PlanBranchRepository;
use crate::error::{AppError, AppResult};

pub struct SqlitePlanBranchRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqlitePlanBranchRepository {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl PlanBranchRepository for SqlitePlanBranchRepository {
    async fn create(&self, branch: PlanBranch) -> AppResult<PlanBranch> {
        let conn = self.conn.lock().await;
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
    }

    async fn get_by_plan_artifact_id(&self, id: &ArtifactId) -> AppResult<Option<PlanBranch>> {
        let conn = self.conn.lock().await;
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
    }

    async fn get_by_merge_task_id(&self, task_id: &TaskId) -> AppResult<Option<PlanBranch>> {
        let conn = self.conn.lock().await;
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
    }

    async fn get_by_project_id(&self, project_id: &ProjectId) -> AppResult<Vec<PlanBranch>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn
            .prepare("SELECT * FROM plan_branches WHERE project_id = ?1 ORDER BY created_at DESC")
            .map_err(|e| AppError::Database(format!("Failed to prepare query: {}", e)))?;

        let branches = stmt
            .query_map(rusqlite::params![project_id.as_str()], |row| {
                PlanBranch::from_row(row)
            })
            .map_err(|e| AppError::Database(format!("Failed to query plan branches: {}", e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(format!("Failed to collect plan branches: {}", e)))?;

        Ok(branches)
    }

    async fn update_status(
        &self,
        id: &PlanBranchId,
        status: PlanBranchStatus,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;
        let rows = conn
            .execute(
                "UPDATE plan_branches SET status = ?1 WHERE id = ?2",
                rusqlite::params![status.to_db_string(), id.as_str()],
            )
            .map_err(|e| AppError::Database(format!("Failed to update plan branch status: {}", e)))?;

        if rows == 0 {
            return Err(AppError::NotFound(format!(
                "Plan branch not found: {}",
                id
            )));
        }
        Ok(())
    }

    async fn set_merge_task_id(&self, id: &PlanBranchId, task_id: &TaskId) -> AppResult<()> {
        let conn = self.conn.lock().await;
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
                id
            )));
        }
        Ok(())
    }

    async fn set_merged(&self, id: &PlanBranchId) -> AppResult<()> {
        let conn = self.conn.lock().await;
        let rows = conn
            .execute(
                "UPDATE plan_branches SET status = 'merged', merged_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now') WHERE id = ?1",
                rusqlite::params![id.as_str()],
            )
            .map_err(|e| AppError::Database(format!("Failed to set plan branch merged: {}", e)))?;

        if rows == 0 {
            return Err(AppError::NotFound(format!(
                "Plan branch not found: {}",
                id
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::IdeationSessionId;
    use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

    fn create_test_branch() -> PlanBranch {
        PlanBranch::new(
            ArtifactId::from_string("art-test-1"),
            IdeationSessionId::from_string("sess-test-1"),
            ProjectId::from_string("proj-test-1".to_string()),
            "ralphx/test-project/plan-abc123".to_string(),
            "main".to_string(),
        )
    }

    async fn setup_repo() -> SqlitePlanBranchRepository {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        SqlitePlanBranchRepository::new(conn)
    }

    #[tokio::test]
    async fn test_create_and_get_by_plan_artifact_id() {
        let repo = setup_repo().await;
        let branch = create_test_branch();
        let artifact_id = branch.plan_artifact_id.clone();

        let created = repo.create(branch).await.unwrap();
        assert_eq!(created.plan_artifact_id, artifact_id);

        let retrieved = repo
            .get_by_plan_artifact_id(&artifact_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.plan_artifact_id, artifact_id);
        assert_eq!(retrieved.branch_name, "ralphx/test-project/plan-abc123");
        assert_eq!(retrieved.source_branch, "main");
        assert_eq!(retrieved.status, PlanBranchStatus::Active);
        assert!(retrieved.merge_task_id.is_none());
        assert!(retrieved.merged_at.is_none());
    }

    #[tokio::test]
    async fn test_get_by_plan_artifact_id_not_found() {
        let repo = setup_repo().await;
        let result = repo
            .get_by_plan_artifact_id(&ArtifactId::from_string("nonexistent"))
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_by_merge_task_id() {
        let repo = setup_repo().await;
        let mut branch = create_test_branch();
        let merge_task_id = TaskId::from_string("merge-task-1".to_string());
        branch.merge_task_id = Some(merge_task_id.clone());

        repo.create(branch).await.unwrap();

        let retrieved = repo
            .get_by_merge_task_id(&merge_task_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            retrieved.merge_task_id.as_ref().unwrap().as_str(),
            "merge-task-1"
        );
    }

    #[tokio::test]
    async fn test_get_by_merge_task_id_not_found() {
        let repo = setup_repo().await;
        let result = repo
            .get_by_merge_task_id(&TaskId::from_string("nonexistent".to_string()))
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_by_project_id() {
        let repo = setup_repo().await;
        let project_id = ProjectId::from_string("proj-multi".to_string());

        let branch1 = PlanBranch::new(
            ArtifactId::from_string("art-1"),
            IdeationSessionId::from_string("sess-1"),
            project_id.clone(),
            "ralphx/proj/plan-1".to_string(),
            "main".to_string(),
        );
        let branch2 = PlanBranch::new(
            ArtifactId::from_string("art-2"),
            IdeationSessionId::from_string("sess-2"),
            project_id.clone(),
            "ralphx/proj/plan-2".to_string(),
            "main".to_string(),
        );

        repo.create(branch1).await.unwrap();
        repo.create(branch2).await.unwrap();

        let branches = repo.get_by_project_id(&project_id).await.unwrap();
        assert_eq!(branches.len(), 2);
    }

    #[tokio::test]
    async fn test_get_by_project_id_empty() {
        let repo = setup_repo().await;
        let branches = repo
            .get_by_project_id(&ProjectId::from_string("empty-proj".to_string()))
            .await
            .unwrap();
        assert!(branches.is_empty());
    }

    #[tokio::test]
    async fn test_update_status() {
        let repo = setup_repo().await;
        let branch = create_test_branch();
        let branch_id = branch.id.clone();
        let artifact_id = branch.plan_artifact_id.clone();

        repo.create(branch).await.unwrap();
        repo.update_status(&branch_id, PlanBranchStatus::Abandoned)
            .await
            .unwrap();

        let retrieved = repo
            .get_by_plan_artifact_id(&artifact_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.status, PlanBranchStatus::Abandoned);
    }

    #[tokio::test]
    async fn test_update_status_not_found() {
        let repo = setup_repo().await;
        let result = repo
            .update_status(
                &PlanBranchId::from_string("nonexistent"),
                PlanBranchStatus::Merged,
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_set_merge_task_id() {
        let repo = setup_repo().await;
        let branch = create_test_branch();
        let branch_id = branch.id.clone();
        let artifact_id = branch.plan_artifact_id.clone();
        let merge_task_id = TaskId::from_string("mt-1".to_string());

        repo.create(branch).await.unwrap();
        repo.set_merge_task_id(&branch_id, &merge_task_id)
            .await
            .unwrap();

        let retrieved = repo
            .get_by_plan_artifact_id(&artifact_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.merge_task_id.unwrap().as_str(), "mt-1");
    }

    #[tokio::test]
    async fn test_set_merge_task_id_not_found() {
        let repo = setup_repo().await;
        let result = repo
            .set_merge_task_id(
                &PlanBranchId::from_string("nonexistent"),
                &TaskId::from_string("mt-1".to_string()),
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_set_merged() {
        let repo = setup_repo().await;
        let branch = create_test_branch();
        let branch_id = branch.id.clone();
        let artifact_id = branch.plan_artifact_id.clone();

        repo.create(branch).await.unwrap();
        repo.set_merged(&branch_id).await.unwrap();

        let retrieved = repo
            .get_by_plan_artifact_id(&artifact_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.status, PlanBranchStatus::Merged);
        assert!(retrieved.merged_at.is_some());
    }

    #[tokio::test]
    async fn test_set_merged_not_found() {
        let repo = setup_repo().await;
        let result = repo
            .set_merged(&PlanBranchId::from_string("nonexistent"))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unique_constraint_on_plan_artifact_id() {
        let repo = setup_repo().await;
        let branch1 = create_test_branch();
        let branch2 = PlanBranch::new(
            branch1.plan_artifact_id.clone(), // same artifact id
            IdeationSessionId::from_string("sess-different"),
            ProjectId::from_string("proj-different".to_string()),
            "ralphx/other/plan-xyz".to_string(),
            "main".to_string(),
        );

        repo.create(branch1).await.unwrap();
        let result = repo.create(branch2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_with_merge_task_id() {
        let repo = setup_repo().await;
        let mut branch = create_test_branch();
        branch.merge_task_id = Some(TaskId::from_string("mt-preset".to_string()));

        let created = repo.create(branch).await.unwrap();
        assert_eq!(created.merge_task_id.unwrap().as_str(), "mt-preset");
    }

    #[tokio::test]
    async fn test_get_by_merge_task_id_after_set() {
        let repo = setup_repo().await;
        let branch = create_test_branch();
        let branch_id = branch.id.clone();
        let merge_task_id = TaskId::from_string("mt-lookup".to_string());

        repo.create(branch).await.unwrap();
        repo.set_merge_task_id(&branch_id, &merge_task_id)
            .await
            .unwrap();

        let retrieved = repo
            .get_by_merge_task_id(&merge_task_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.id, branch_id);
    }
}
