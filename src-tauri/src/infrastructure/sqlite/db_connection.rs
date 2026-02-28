// Shared DbConnection wrapper for executing blocking SQLite operations off tokio worker threads.
//
// All sqlite repo files should use DbConnection::run() for rusqlite calls to prevent
// blocking the tokio async runtime / timer driver.

use std::sync::Arc;

use rusqlite::Connection;
use tokio::sync::Mutex;

use crate::error::{AppError, AppResult};

/// Newtype wrapper around a shared SQLite connection.
///
/// Provides `run()` and `query_optional()` methods that execute blocking rusqlite
/// operations on the tokio blocking thread pool via `spawn_blocking`. This prevents
/// rusqlite calls from blocking tokio worker threads, which would starve the timer
/// driver and make `tokio::time::timeout` unreliable.
///
/// # Usage
///
/// ```rust,ignore
/// let task = self.db.run(move |conn| {
///     conn.query_row(
///         "SELECT id, name FROM tasks WHERE id = ?1",
///         rusqlite::params![id.0],
///         |row| Ok(Task { id: row.get(0)?, name: row.get(1)? }),
///     )?;
///     Ok(task)
/// }).await?;
/// ```
#[derive(Clone)]
pub struct DbConnection {
    conn: Arc<Mutex<Connection>>,
}

impl DbConnection {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Returns the inner Arc for legacy interop during migration.
    pub fn inner(&self) -> &Arc<Mutex<Connection>> {
        &self.conn
    }

    /// Execute a blocking DB operation on the tokio blocking thread pool.
    ///
    /// The closure receives a `&Connection` and must return `AppResult<T>`.
    /// The `?` operator works on rusqlite errors inside the closure thanks to
    /// `impl From<rusqlite::Error> for AppError`.
    ///
    /// JoinError from `spawn_blocking` is mapped to `AppError::Database`.
    pub async fn run<F, T>(&self, f: F) -> AppResult<T>
    where
        F: FnOnce(&Connection) -> AppResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let guard = conn.blocking_lock();
            f(&guard)
        })
        .await
        .map_err(|e| AppError::Database(format!("spawn_blocking join error: {e}")))?
    }

    /// Query that may return zero rows — maps `QueryReturnedNoRows` to `Ok(None)`.
    ///
    /// The closure receives a `&Connection` and should return `Result<T, rusqlite::Error>`.
    /// `QueryReturnedNoRows` is treated as `Ok(None)`, all other errors become
    /// `AppError::Database`.
    pub async fn query_optional<F, T>(&self, f: F) -> AppResult<Option<T>>
    where
        F: FnOnce(&Connection) -> Result<T, rusqlite::Error> + Send + 'static,
        T: Send + 'static,
    {
        self.run(move |conn| match f(conn) {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        })
        .await
    }
}
