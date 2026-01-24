// SQLite infrastructure layer
// Database connection management, migrations, and repository implementations

pub mod connection;
pub mod migrations;
pub mod sqlite_project_repo;
pub mod sqlite_task_repo;
pub mod state_machine_repository;

// Re-export commonly used items
pub use connection::{get_default_db_path, open_connection, open_memory_connection};
pub use migrations::{run_migrations, SCHEMA_VERSION};
pub use sqlite_project_repo::SqliteProjectRepository;
pub use sqlite_task_repo::SqliteTaskRepository;
pub use state_machine_repository::TaskStateMachineRepository;
