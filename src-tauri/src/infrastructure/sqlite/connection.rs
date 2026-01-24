// Database connection management for SQLite

use rusqlite::Connection;
use std::path::PathBuf;

use crate::error::{AppError, AppResult};

/// Get the default database path based on Tauri's app data directory
pub fn get_default_db_path() -> PathBuf {
    // For now, use a simple path in the working directory
    // In production, this would use Tauri's app data path
    PathBuf::from("ralphx.db")
}

/// Open a database connection at the specified path
/// Creates the database file if it doesn't exist
pub fn open_connection(path: &PathBuf) -> AppResult<Connection> {
    Connection::open(path).map_err(|e| AppError::Database(e.to_string()))
}

/// Open an in-memory database for testing
pub fn open_memory_connection() -> AppResult<Connection> {
    Connection::open_in_memory().map_err(|e| AppError::Database(e.to_string()))
}

#[cfg(test)]
mod tests {
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
}
