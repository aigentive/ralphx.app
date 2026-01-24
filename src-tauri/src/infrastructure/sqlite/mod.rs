// SQLite infrastructure layer
// Database connection management and migrations

pub mod connection;
pub mod migrations;

// Re-export commonly used items
pub use connection::{get_default_db_path, open_connection, open_memory_connection};
pub use migrations::{run_migrations, SCHEMA_VERSION};
