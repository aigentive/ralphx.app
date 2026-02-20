// SQLite-based ProjectRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use rusqlite::Connection;

use crate::domain::entities::{Project, ProjectId};
use crate::domain::repositories::ProjectRepository;
use crate::error::{AppError, AppResult};

/// SQLite implementation of ProjectRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteProjectRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteProjectRepository {
    /// Create a new SQLite project repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    /// Create from an Arc-wrapped mutex connection (for sharing)
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl ProjectRepository for SqliteProjectRepository {
    async fn create(&self, project: Project) -> AppResult<Project> {
        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT INTO projects (id, name, working_directory, git_mode, base_branch, worktree_parent_directory, use_feature_branches, merge_validation_mode, merge_strategy, detected_analysis, custom_analysis, analyzed_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            rusqlite::params![
                project.id.as_str(),
                project.name,
                project.working_directory,
                project.git_mode.to_string(),
                project.base_branch,
                project.worktree_parent_directory,
                project.use_feature_branches as i64,
                project.merge_validation_mode.to_string(),
                project.merge_strategy.to_string(),
                project.detected_analysis,
                project.custom_analysis,
                project.analyzed_at,
                project.created_at.to_rfc3339(),
                project.updated_at.to_rfc3339(),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(project)
    }

    async fn get_by_id(&self, id: &ProjectId) -> AppResult<Option<Project>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, name, working_directory, git_mode, base_branch, worktree_parent_directory, use_feature_branches, merge_validation_mode, merge_strategy, detected_analysis, custom_analysis, analyzed_at, created_at, updated_at
             FROM projects WHERE id = ?1",
            [id.as_str()],
            |row| Project::from_row(row),
        );

        match result {
            Ok(project) => Ok(Some(project)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_all(&self) -> AppResult<Vec<Project>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, working_directory, git_mode, base_branch, worktree_parent_directory, use_feature_branches, merge_validation_mode, merge_strategy, detected_analysis, custom_analysis, analyzed_at, created_at, updated_at
                 FROM projects ORDER BY name ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let projects = stmt
            .query_map([], Project::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(projects)
    }

    async fn update(&self, project: &Project) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "UPDATE projects SET name = ?2, working_directory = ?3, git_mode = ?4, base_branch = ?5, worktree_parent_directory = ?6, use_feature_branches = ?7, merge_validation_mode = ?8, merge_strategy = ?9, detected_analysis = ?10, custom_analysis = ?11, analyzed_at = ?12, updated_at = ?13
             WHERE id = ?1",
            rusqlite::params![
                project.id.as_str(),
                project.name,
                project.working_directory,
                project.git_mode.to_string(),
                project.base_branch,
                project.worktree_parent_directory,
                project.use_feature_branches as i64,
                project.merge_validation_mode.to_string(),
                project.merge_strategy.to_string(),
                project.detected_analysis,
                project.custom_analysis,
                project.analyzed_at,
                project.updated_at.to_rfc3339(),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: &ProjectId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute("DELETE FROM projects WHERE id = ?1", [id.as_str()])
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_by_working_directory(&self, path: &str) -> AppResult<Option<Project>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, name, working_directory, git_mode, base_branch, worktree_parent_directory, use_feature_branches, merge_validation_mode, merge_strategy, detected_analysis, custom_analysis, analyzed_at, created_at, updated_at
             FROM projects WHERE working_directory = ?1",
            [path],
            |row| Project::from_row(row),
        );

        match result {
            Ok(project) => Ok(Some(project)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }
}

#[cfg(test)]
#[path = "sqlite_project_repo_tests.rs"]
mod tests;
