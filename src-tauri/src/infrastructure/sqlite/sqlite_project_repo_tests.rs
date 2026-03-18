use super::*;
use crate::domain::entities::GitMode;
use crate::testing::SqliteTestDb;

fn setup_test_db() -> SqliteTestDb {
    SqliteTestDb::new("sqlite_project_repo_tests")
}

fn create_test_project(name: &str, path: &str) -> Project {
    Project::new(name.to_string(), path.to_string())
}

// ==================== CRUD TESTS ====================

#[tokio::test]
async fn test_create_inserts_project_and_returns_it() {
    let db = setup_test_db();
    let repo = SqliteProjectRepository::from_shared(db.shared_conn());
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
    let db = setup_test_db();
    let repo = SqliteProjectRepository::from_shared(db.shared_conn());
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
    let db = setup_test_db();
    let repo = SqliteProjectRepository::from_shared(db.shared_conn());
    let id = ProjectId::new();

    let result = repo.get_by_id(&id).await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_get_all_returns_all_projects() {
    let db = setup_test_db();
    let repo = SqliteProjectRepository::from_shared(db.shared_conn());

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
    let db = setup_test_db();
    let repo = SqliteProjectRepository::from_shared(db.shared_conn());

    let result = repo.get_all().await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn test_update_modifies_project_fields() {
    let db = setup_test_db();
    let repo = SqliteProjectRepository::from_shared(db.shared_conn());
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
    let db = setup_test_db();
    let repo = SqliteProjectRepository::from_shared(db.shared_conn());
    let project = create_test_project("To Delete", "/delete/me");

    repo.create(project.clone()).await.unwrap();

    let delete_result = repo.delete(&project.id).await;
    assert!(delete_result.is_ok());

    let found = repo.get_by_id(&project.id).await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn test_create_and_retrieve_preserves_all_fields() {
    let db = setup_test_db();
    let repo = SqliteProjectRepository::from_shared(db.shared_conn());

    let mut project = Project::new("Full Project".to_string(), "/full/path".to_string());
    project.git_mode = GitMode::Worktree;
    project.base_branch = Some("main".to_string());

    repo.create(project.clone()).await.unwrap();
    let found = repo.get_by_id(&project.id).await.unwrap().unwrap();

    assert_eq!(found.id, project.id);
    assert_eq!(found.name, project.name);
    assert_eq!(found.working_directory, project.working_directory);
    assert_eq!(found.git_mode, GitMode::Worktree);
    assert_eq!(found.base_branch, Some("main".to_string()));
}

// ==================== GET BY WORKING DIRECTORY TESTS ====================

#[tokio::test]
async fn test_get_by_working_directory_returns_project() {
    let db = setup_test_db();
    let repo = SqliteProjectRepository::from_shared(db.shared_conn());
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
    let db = setup_test_db();
    let repo = SqliteProjectRepository::from_shared(db.shared_conn());
    let project = create_test_project("Test Project", "/test/path");

    repo.create(project).await.unwrap();

    let result = repo.get_by_working_directory("/different/path").await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_get_by_working_directory_finds_correct_project() {
    let db = setup_test_db();
    let repo = SqliteProjectRepository::from_shared(db.shared_conn());

    let project1 = create_test_project("Project 1", "/path/one");
    let project2 = create_test_project("Project 2", "/path/two");

    repo.create(project1.clone()).await.unwrap();
    repo.create(project2.clone()).await.unwrap();

    let found = repo.get_by_working_directory("/path/two").await.unwrap();

    assert!(found.is_some());
    assert_eq!(found.unwrap().id, project2.id);
}
