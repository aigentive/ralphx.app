// Database connection management for SQLite

use rusqlite::Connection;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use crate::error::{AppError, AppResult};

/// Get the default database path based on Tauri's app data directory
pub fn get_default_db_path() -> PathBuf {
    // For now, use a simple path in the working directory
    // In production, this would use Tauri's app data path
    PathBuf::from("ralphx.db")
}

/// Get the database path inside the app data directory
pub fn get_app_data_db_path(app_handle: &AppHandle) -> AppResult<PathBuf> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Infrastructure(format!("Failed to resolve app data dir: {}", e)))?;

    std::fs::create_dir_all(&app_data_dir)
        .map_err(|e| AppError::Infrastructure(format!("Failed to create app data dir: {}", e)))?;

    Ok(app_data_dir.join("ralphx.db"))
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
#[path = "connection_tests.rs"]
mod tests;
