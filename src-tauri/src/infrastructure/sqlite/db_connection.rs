// Shared DbConnection wrapper for executing blocking SQLite operations off tokio worker threads.
//
// All sqlite repo files should use DbConnection::run() for rusqlite calls to prevent
// blocking the tokio async runtime / timer driver.

#[cfg(test)]
#[path = "db_connection_tests.rs"]
mod tests;

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
            #[cfg(debug_assertions)]
            let lock_start = std::time::Instant::now();

            let guard = conn.blocking_lock();

            #[cfg(debug_assertions)]
            let lock_acquired = std::time::Instant::now();

            let result = f(&guard);

            #[cfg(debug_assertions)]
            {
                let lock_wait_ms = lock_acquired.duration_since(lock_start).as_millis();
                let lock_hold_ms = lock_acquired.elapsed().as_millis();
                if lock_wait_ms > 100 {
                    tracing::warn!(
                        target: "ralphx::db",
                        lock_wait_ms,
                        lock_hold_ms,
                        method = "run",
                        "DB lock contention: lock wait exceeded 100ms"
                    );
                } else {
                    tracing::debug!(
                        target: "ralphx::db",
                        lock_wait_ms,
                        lock_hold_ms,
                        method = "run",
                    );
                }
            }

            result
        })
        .await
        .map_err(|e| AppError::Database(format!("spawn_blocking join error: {e}")))?
    }

    /// Run a closure inside a SQLite transaction (BEGIN/COMMIT/ROLLBACK).
    ///
    /// Acquires the same `tokio::sync::Mutex` as `db.run()`. MUST NOT be called
    /// from within a `db.run()` closure — the tokio Mutex is non-reentrant and will
    /// deadlock immediately (caught in any test exercising the nested path).
    ///
    /// Events should be emitted AFTER this returns, outside the lock.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Database` on BEGIN/COMMIT failure or if the closure errors
    /// (which triggers automatic ROLLBACK).
    pub async fn run_transaction<F, T>(&self, f: F) -> AppResult<T>
    where
        F: FnOnce(&Connection) -> AppResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            #[cfg(debug_assertions)]
            let lock_start = std::time::Instant::now();

            let guard = conn.blocking_lock();

            #[cfg(debug_assertions)]
            let lock_acquired = std::time::Instant::now();

            guard
                .execute_batch("BEGIN")
                .map_err(|e| AppError::Database(format!("BEGIN failed: {e}")))?;
            let result = match f(&guard) {
                Ok(result) => {
                    guard
                        .execute_batch("COMMIT")
                        .map_err(|e| AppError::Database(format!("COMMIT failed: {e}")))?;
                    Ok(result)
                }
                Err(e) => {
                    let _ = guard.execute_batch("ROLLBACK");
                    Err(e)
                }
            };

            #[cfg(debug_assertions)]
            {
                let lock_wait_ms = lock_acquired.duration_since(lock_start).as_millis();
                let lock_hold_ms = lock_acquired.elapsed().as_millis();
                if lock_wait_ms > 100 {
                    tracing::warn!(
                        target: "ralphx::db",
                        lock_wait_ms,
                        lock_hold_ms,
                        method = "run_transaction",
                        "DB lock contention: lock wait exceeded 100ms"
                    );
                } else {
                    tracing::debug!(
                        target: "ralphx::db",
                        lock_wait_ms,
                        lock_hold_ms,
                        method = "run_transaction",
                    );
                }
            }

            result
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
