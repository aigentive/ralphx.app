use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::Connection;
use tempfile::{Builder, TempDir};
use tokio::sync::Mutex;

use crate::application::AppState;
use crate::infrastructure::sqlite::{open_connection, run_migrations};

pub struct SqliteTestDb {
    _temp_dir: TempDir,
    path: PathBuf,
    shared_conn: Arc<Mutex<Connection>>,
}

impl SqliteTestDb {
    pub fn new(name: &str) -> Self {
        let sanitized_name = sanitize_name(name);
        let temp_dir = Builder::new()
            .prefix(&format!("ralphx-{sanitized_name}-"))
            .tempdir()
            .expect("Failed to create temp dir for SQLite test DB");
        let path = temp_dir.path().join("test.db");
        let conn = open_connection(&path).expect("Failed to open SQLite test DB");
        run_migrations(&conn).expect("Failed to run migrations for SQLite test DB");

        Self {
            _temp_dir: temp_dir,
            path,
            shared_conn: Arc::new(Mutex::new(conn)),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn shared_conn(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.shared_conn)
    }

    pub fn with_connection<T>(&self, f: impl FnOnce(&Connection) -> T) -> T {
        let guard = self
            .shared_conn
            .try_lock()
            .expect("SQLite test DB unexpectedly contended during setup");
        f(&guard)
    }
}

pub struct SqliteStateFixture {
    _db: SqliteTestDb,
    state: AppState,
}

impl SqliteStateFixture {
    pub fn new(name: &str, configure: impl FnOnce(&SqliteTestDb, &mut AppState)) -> Self {
        let db = SqliteTestDb::new(name);
        let mut state = AppState::new_test();
        configure(&db, &mut state);
        Self { _db: db, state }
    }

    pub fn db(&self) -> &SqliteTestDb {
        &self._db
    }
}

impl Deref for SqliteStateFixture {
    type Target = AppState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

fn sanitize_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect();
    sanitized.trim_matches('-').to_string()
}
