use super::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_get_default_db_path_returns_path() {
    let path = get_default_db_path();
    assert!(!path.to_str().unwrap().is_empty());
    assert!(path.to_str().unwrap().ends_with(".db"));
}

#[test]
fn test_open_memory_connection_succeeds() {
    let conn = open_memory_connection();
    assert!(conn.is_ok());
}

#[test]
fn test_memory_connection_can_execute_sql() {
    let conn = open_memory_connection().unwrap();
    let result = conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY)", []);
    assert!(result.is_ok());
}

#[test]
fn test_open_connection_creates_file() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    assert!(!db_path.exists());

    let conn = open_connection(&db_path);
    assert!(conn.is_ok());

    // Connection should create the file
    assert!(db_path.exists());
}

#[test]
fn test_open_connection_on_existing_file() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("existing.db");

    // Create file first
    fs::write(&db_path, "").unwrap();
    assert!(db_path.exists());

    let conn = open_connection(&db_path);
    assert!(conn.is_ok());
}

#[test]
fn test_open_connection_error_on_invalid_path() {
    let invalid_path = PathBuf::from("/nonexistent/directory/that/does/not/exist/test.db");
    let result = open_connection(&invalid_path);
    assert!(result.is_err());

    if let Err(AppError::Database(msg)) = result {
        assert!(!msg.is_empty());
    } else {
        panic!("Expected Database error");
    }
}
