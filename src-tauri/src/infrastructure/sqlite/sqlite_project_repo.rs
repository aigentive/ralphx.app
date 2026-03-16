// SQLite-based ProjectRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use rusqlite::Connection;

use super::DbConnection;
use crate::domain::entities::{Project, ProjectId};
use crate::domain::repositories::ProjectRepository;
use crate::error::AppResult;

/// SQLite implementation of ProjectRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteProjectRepository {
    db: DbConnection,
}

impl SqliteProjectRepository {
    /// Create a new SQLite project repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            db: DbConnection::new(conn),
        }
    }

    /// Create from an Arc-wrapped mutex connection (for sharing)
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            db: DbConnection::from_shared(conn),
        }
    }
}

#[async_trait]
impl ProjectRepository for SqliteProjectRepository {
    async fn create(&self, project: Project) -> AppResult<Project> {
        self.db
            .run(move |conn| {
                conn.execute(
                    "INSERT INTO projects (id, name, working_directory, git_mode, base_branch, worktree_parent_directory, use_feature_branches, merge_validation_mode, merge_strategy, detected_analysis, custom_analysis, analyzed_at, created_at, updated_at, github_pr_enabled)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
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
                        project.github_pr_enabled as i64,
                    ],
                )?;
                Ok(project)
            })
            .await
    }

    async fn get_by_id(&self, id: &ProjectId) -> AppResult<Option<Project>> {
        let id = id.as_str().to_string();
        self.db
            .query_optional(move |conn| {
                conn.query_row(
                    "SELECT id, name, working_directory, git_mode, base_branch, worktree_parent_directory, use_feature_branches, merge_validation_mode, merge_strategy, detected_analysis, custom_analysis, analyzed_at, created_at, updated_at, github_pr_enabled, archived_at
                     FROM projects WHERE id = ?1",
                    [id.as_str()],
                    |row| Project::from_row(row),
                )
            })
            .await
    }

    async fn get_all(&self) -> AppResult<Vec<Project>> {
        self.db
            .run(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, name, working_directory, git_mode, base_branch, worktree_parent_directory, use_feature_branches, merge_validation_mode, merge_strategy, detected_analysis, custom_analysis, analyzed_at, created_at, updated_at, github_pr_enabled, archived_at
                     FROM projects WHERE archived_at IS NULL ORDER BY name ASC",
                )?;
                let projects = stmt
                    .query_map([], Project::from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(projects)
            })
            .await
    }

    async fn update(&self, project: &Project) -> AppResult<()> {
        let id = project.id.as_str().to_string();
        let name = project.name.clone();
        let working_directory = project.working_directory.clone();
        let git_mode = project.git_mode.to_string();
        let base_branch = project.base_branch.clone();
        let worktree_parent_directory = project.worktree_parent_directory.clone();
        let use_feature_branches = project.use_feature_branches as i64;
        let merge_validation_mode = project.merge_validation_mode.to_string();
        let merge_strategy = project.merge_strategy.to_string();
        let detected_analysis = project.detected_analysis.clone();
        let custom_analysis = project.custom_analysis.clone();
        let analyzed_at = project.analyzed_at.clone();
        let updated_at = project.updated_at.to_rfc3339();
        let github_pr_enabled = project.github_pr_enabled as i64;

        self.db
            .run(move |conn| {
                conn.execute(
                    "UPDATE projects SET name = ?2, working_directory = ?3, git_mode = ?4, base_branch = ?5, worktree_parent_directory = ?6, use_feature_branches = ?7, merge_validation_mode = ?8, merge_strategy = ?9, detected_analysis = ?10, custom_analysis = ?11, analyzed_at = ?12, updated_at = ?13, github_pr_enabled = ?14
                     WHERE id = ?1",
                    rusqlite::params![
                        id,
                        name,
                        working_directory,
                        git_mode,
                        base_branch,
                        worktree_parent_directory,
                        use_feature_branches,
                        merge_validation_mode,
                        merge_strategy,
                        detected_analysis,
                        custom_analysis,
                        analyzed_at,
                        updated_at,
                        github_pr_enabled,
                    ],
                )?;
                Ok(())
            })
            .await
    }

    async fn delete(&self, id: &ProjectId) -> AppResult<()> {
        let id = id.as_str().to_string();
        self.db
            .run(move |conn| {
                conn.execute("DELETE FROM projects WHERE id = ?1", [id.as_str()])?;
                Ok(())
            })
            .await
    }

    async fn get_by_working_directory(&self, path: &str) -> AppResult<Option<Project>> {
        let path = path.to_string();
        self.db
            .query_optional(move |conn| {
                conn.query_row(
                    "SELECT id, name, working_directory, git_mode, base_branch, worktree_parent_directory, use_feature_branches, merge_validation_mode, merge_strategy, detected_analysis, custom_analysis, analyzed_at, created_at, updated_at, github_pr_enabled, archived_at
                     FROM projects WHERE working_directory = ?1",
                    [path.as_str()],
                    |row| Project::from_row(row),
                )
            })
            .await
    }

    async fn archive(&self, id: &ProjectId) -> AppResult<Project> {
        let id = id.as_str().to_string();
        self.db
            .run(move |conn| {
                let now = Utc::now();
                conn.execute(
                    "UPDATE projects SET archived_at = ?2, updated_at = ?3 WHERE id = ?1 AND archived_at IS NULL",
                    rusqlite::params![id.as_str(), now.to_rfc3339(), now.to_rfc3339()],
                )?;
                let project = conn.query_row(
                    "SELECT id, name, working_directory, git_mode, base_branch, worktree_parent_directory, use_feature_branches, merge_validation_mode, merge_strategy, detected_analysis, custom_analysis, analyzed_at, created_at, updated_at, github_pr_enabled, archived_at
                     FROM projects WHERE id = ?1",
                    [id.as_str()],
                    |row| Project::from_row(row),
                )?;
                Ok(project)
            })
            .await
    }
}

#[cfg(test)]
#[path = "sqlite_project_repo_tests.rs"]
mod tests;
