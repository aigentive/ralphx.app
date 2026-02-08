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
            "INSERT INTO projects (id, name, working_directory, git_mode, worktree_path, worktree_branch, base_branch, worktree_parent_directory, use_feature_branches, merge_validation_mode, detected_analysis, custom_analysis, analyzed_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            rusqlite::params![
                project.id.as_str(),
                project.name,
                project.working_directory,
                project.git_mode.to_string(),
                project.worktree_path,
                project.worktree_branch,
                project.base_branch,
                project.worktree_parent_directory,
                project.use_feature_branches as i64,
                project.merge_validation_mode.to_string(),
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
            "SELECT id, name, working_directory, git_mode, worktree_path, worktree_branch, base_branch, worktree_parent_directory, use_feature_branches, merge_validation_mode, detected_analysis, custom_analysis, analyzed_at, created_at, updated_at
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
                "SELECT id, name, working_directory, git_mode, worktree_path, worktree_branch, base_branch, worktree_parent_directory, use_feature_branches, merge_validation_mode, detected_analysis, custom_analysis, analyzed_at, created_at, updated_at
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
            "UPDATE projects SET name = ?2, working_directory = ?3, git_mode = ?4, worktree_path = ?5, worktree_branch = ?6, base_branch = ?7, worktree_parent_directory = ?8, use_feature_branches = ?9, merge_validation_mode = ?10, detected_analysis = ?11, custom_analysis = ?12, analyzed_at = ?13, updated_at = ?14
             WHERE id = ?1",
            rusqlite::params![
                project.id.as_str(),
                project.name,
                project.working_directory,
                project.git_mode.to_string(),
                project.worktree_path,
                project.worktree_branch,
                project.base_branch,
                project.worktree_parent_directory,
                project.use_feature_branches as i64,
                project.merge_validation_mode.to_string(),
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
            "SELECT id, name, working_directory, git_mode, worktree_path, worktree_branch, base_branch, worktree_parent_directory, use_feature_branches, merge_validation_mode, detected_analysis, custom_analysis, analyzed_at, created_at, updated_at
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
mod tests {
    use super::*;
    use crate::domain::entities::GitMode;
    use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

    fn setup_test_db() -> Connection {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    fn create_test_project(name: &str, path: &str) -> Project {
        Project::new(name.to_string(), path.to_string())
    }

    // ==================== CRUD TESTS ====================

    #[tokio::test]
    async fn test_create_inserts_project_and_returns_it() {
        let conn = setup_test_db();
        let repo = SqliteProjectRepository::new(conn);
        let project = create_test_project("Test Project", "/test/path");

        let result = repo.create(project.clone()).await;

        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.id, project.id);
        assert_eq!(created.name, "Test Project");
        assert_eq!(created.working_directory, "/test/path");
    }

    #[tokio::test]
    async fn test_get_by_id_retrieves_project_correctly() {
        let conn = setup_test_db();
        let repo = SqliteProjectRepository::new(conn);
        let project = create_test_project("Test Project", "/test/path");

        repo.create(project.clone()).await.unwrap();
        let result = repo.get_by_id(&project.id).await;

        assert!(result.is_ok());
        let found = result.unwrap();
        assert!(found.is_some());
        let found_project = found.unwrap();
        assert_eq!(found_project.id, project.id);
        assert_eq!(found_project.name, "Test Project");
        assert_eq!(found_project.working_directory, "/test/path");
    }

    #[tokio::test]
    async fn test_get_by_id_returns_none_for_nonexistent() {
        let conn = setup_test_db();
        let repo = SqliteProjectRepository::new(conn);
        let id = ProjectId::new();

        let result = repo.get_by_id(&id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_all_returns_all_projects() {
        let conn = setup_test_db();
        let repo = SqliteProjectRepository::new(conn);

        let project1 = create_test_project("Alpha Project", "/alpha");
        let project2 = create_test_project("Beta Project", "/beta");
        let project3 = create_test_project("Gamma Project", "/gamma");

        repo.create(project3).await.unwrap();
        repo.create(project1).await.unwrap();
        repo.create(project2).await.unwrap();

        let result = repo.get_all().await;

        assert!(result.is_ok());
        let projects = result.unwrap();
        assert_eq!(projects.len(), 3);
        // Should be sorted by name
        assert_eq!(projects[0].name, "Alpha Project");
        assert_eq!(projects[1].name, "Beta Project");
        assert_eq!(projects[2].name, "Gamma Project");
    }

    #[tokio::test]
    async fn test_get_all_returns_empty_for_no_projects() {
        let conn = setup_test_db();
        let repo = SqliteProjectRepository::new(conn);

        let result = repo.get_all().await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_update_modifies_project_fields() {
        let conn = setup_test_db();
        let repo = SqliteProjectRepository::new(conn);
        let mut project = create_test_project("Original Name", "/original/path");

        repo.create(project.clone()).await.unwrap();

        project.name = "Updated Name".to_string();
        project.working_directory = "/updated/path".to_string();
        project.git_mode = GitMode::Worktree;
        project.base_branch = Some("develop".to_string());

        let update_result = repo.update(&project).await;
        assert!(update_result.is_ok());

        let found = repo.get_by_id(&project.id).await.unwrap().unwrap();
        assert_eq!(found.name, "Updated Name");
        assert_eq!(found.working_directory, "/updated/path");
        assert_eq!(found.git_mode, GitMode::Worktree);
        assert_eq!(found.base_branch, Some("develop".to_string()));
    }

    #[tokio::test]
    async fn test_delete_removes_project_from_database() {
        let conn = setup_test_db();
        let repo = SqliteProjectRepository::new(conn);
        let project = create_test_project("To Delete", "/delete/me");

        repo.create(project.clone()).await.unwrap();

        let delete_result = repo.delete(&project.id).await;
        assert!(delete_result.is_ok());

        let found = repo.get_by_id(&project.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_create_and_retrieve_preserves_all_fields() {
        let conn = setup_test_db();
        let repo = SqliteProjectRepository::new(conn);

        let project = Project::new_with_worktree(
            "Full Project".to_string(),
            "/full/path".to_string(),
            "/worktree/path".to_string(),
            "feature-branch".to_string(),
            Some("main".to_string()),
        );

        repo.create(project.clone()).await.unwrap();
        let found = repo.get_by_id(&project.id).await.unwrap().unwrap();

        assert_eq!(found.id, project.id);
        assert_eq!(found.name, project.name);
        assert_eq!(found.working_directory, project.working_directory);
        assert_eq!(found.git_mode, GitMode::Worktree);
        assert_eq!(found.worktree_path, Some("/worktree/path".to_string()));
        assert_eq!(found.worktree_branch, Some("feature-branch".to_string()));
        assert_eq!(found.base_branch, Some("main".to_string()));
    }

    // ==================== GET BY WORKING DIRECTORY TESTS ====================

    #[tokio::test]
    async fn test_get_by_working_directory_returns_project() {
        let conn = setup_test_db();
        let repo = SqliteProjectRepository::new(conn);
        let project = create_test_project("Test Project", "/test/path");

        repo.create(project.clone()).await.unwrap();

        let result = repo.get_by_working_directory("/test/path").await;

        assert!(result.is_ok());
        let found = result.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, project.id);
    }

    #[tokio::test]
    async fn test_get_by_working_directory_returns_none_for_nonexistent() {
        let conn = setup_test_db();
        let repo = SqliteProjectRepository::new(conn);
        let project = create_test_project("Test Project", "/test/path");

        repo.create(project).await.unwrap();

        let result = repo.get_by_working_directory("/different/path").await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_by_working_directory_finds_correct_project() {
        let conn = setup_test_db();
        let repo = SqliteProjectRepository::new(conn);

        let project1 = create_test_project("Project 1", "/path/one");
        let project2 = create_test_project("Project 2", "/path/two");

        repo.create(project1.clone()).await.unwrap();
        repo.create(project2.clone()).await.unwrap();

        let found = repo.get_by_working_directory("/path/two").await.unwrap();

        assert!(found.is_some());
        assert_eq!(found.unwrap().id, project2.id);
    }
}
