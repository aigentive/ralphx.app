// Infrastructure layer - external implementations
// SQLite, file system, Claude CLI interactions

pub mod memory;
pub mod sqlite;

// Re-export commonly used items
pub use sqlite::{get_default_db_path, open_connection, open_memory_connection, run_migrations};
