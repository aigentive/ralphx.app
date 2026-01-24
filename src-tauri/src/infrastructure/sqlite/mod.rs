// SQLite infrastructure layer
// Database connection management, migrations, and repository implementations

pub mod connection;
pub mod migrations;
pub mod sqlite_task_repo;

// Re-export commonly used items
pub use connection::{get_default_db_path, open_connection, open_memory_connection};
pub use migrations::{run_migrations, SCHEMA_VERSION};
pub use sqlite_task_repo::SqliteTaskRepository;
