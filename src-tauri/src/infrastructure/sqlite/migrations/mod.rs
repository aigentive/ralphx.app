// Database migrations for SQLite
//
// # Migration System Design
//
// ## Adding a new migration
//
// 1. Create a new file: `vN_description.rs` (e.g., `v2_add_user_preferences.rs`)
// 2. Implement a `pub fn migrate(conn: &Connection) -> AppResult<()>` function
// 3. Register it in the MIGRATIONS array below
// 4. Bump SCHEMA_VERSION
//
// ## Guidelines
//
// - Use `IF NOT EXISTS` for CREATE TABLE/INDEX to make migrations idempotent
// - Use helpers::add_column_if_not_exists for ALTER TABLE ADD COLUMN
// - Keep migrations focused - one logical change per migration
// - Test migrations work on both fresh databases and existing ones
//
// ## For existing databases
//
// Existing databases have schema_migrations tracking what version they're at.
// Only migrations newer than their current version will run.

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

pub mod helpers;
mod v1_initial_schema;
mod v2_add_dependency_reason;
mod v3_add_activity_events;
mod v4_add_blocked_reason;
mod v5_add_review_summary_issues;
mod v6_review_issues;
mod v7_session_status_converted_to_accepted;
mod v8_task_git_fields;
mod v9_project_git_fields;
mod v10_execution_settings;
mod v11_per_project_execution_settings;
mod v12_fix_worktree_project_settings;
mod v13_plan_branches;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod v1_initial_schema_tests;
#[cfg(test)]
mod v2_add_dependency_reason_tests;
#[cfg(test)]
mod v3_add_activity_events_tests;
#[cfg(test)]
mod v4_add_blocked_reason_tests;
#[cfg(test)]
mod v6_review_issues_tests;
#[cfg(test)]
mod v7_session_status_converted_to_accepted_tests;
#[cfg(test)]
mod v8_task_git_fields_tests;
#[cfg(test)]
mod v9_project_git_fields_tests;
#[cfg(test)]
mod v10_execution_settings_tests;
#[cfg(test)]
mod v11_per_project_execution_settings_tests;
#[cfg(test)]
mod v12_fix_worktree_project_settings_tests;
#[cfg(test)]
mod v13_plan_branches_tests;

/// Current schema version - bump this when adding a new migration
pub const SCHEMA_VERSION: i32 = 13;

/// Migration function signature
type MigrationFn = fn(&Connection) -> AppResult<()>;

/// Migration definition
struct Migration {
    version: i32,
    name: &'static str,
    migrate: MigrationFn,
}

/// All migrations in order
/// Add new migrations here - they will be run in version order
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "initial_schema",
        migrate: v1_initial_schema::migrate,
    },
    Migration {
        version: 2,
        name: "add_dependency_reason",
        migrate: v2_add_dependency_reason::migrate,
    },
    Migration {
        version: 3,
        name: "add_activity_events",
        migrate: v3_add_activity_events::migrate,
    },
    Migration {
        version: 4,
        name: "add_blocked_reason",
        migrate: v4_add_blocked_reason::migrate,
    },
    Migration {
        version: 5,
        name: "add_review_summary_issues",
        migrate: v5_add_review_summary_issues::migrate,
    },
    Migration {
        version: 6,
        name: "review_issues",
        migrate: v6_review_issues::migrate,
    },
    Migration {
        version: 7,
        name: "session_status_converted_to_accepted",
        migrate: v7_session_status_converted_to_accepted::migrate,
    },
    Migration {
        version: 8,
        name: "task_git_fields",
        migrate: v8_task_git_fields::migrate,
    },
    Migration {
        version: 9,
        name: "project_git_fields",
        migrate: v9_project_git_fields::migrate,
    },
    Migration {
        version: 10,
        name: "execution_settings",
        migrate: v10_execution_settings::migrate,
    },
    Migration {
        version: 11,
        name: "per_project_execution_settings",
        migrate: v11_per_project_execution_settings::migrate,
    },
    Migration {
        version: 12,
        name: "fix_worktree_project_settings",
        migrate: v12_fix_worktree_project_settings::migrate,
    },
    Migration {
        version: 13,
        name: "plan_branches",
        migrate: v13_plan_branches::migrate,
    },
];

/// Run all pending migrations on the database
pub fn run_migrations(conn: &Connection) -> AppResult<()> {
    // Create migrations table if it doesn't exist
    create_migrations_table(conn)?;

    // Get current version
    let current_version = get_schema_version(conn)?;

    // Run migrations sequentially
    for migration in MIGRATIONS {
        if current_version < migration.version {
            tracing::info!(
                "Running migration v{}: {}",
                migration.version,
                migration.name
            );

            (migration.migrate)(conn)?;
            set_schema_version(conn, migration.version)?;

            tracing::info!("Migration v{} complete", migration.version);
        }
    }

    Ok(())
}

/// Create the migrations tracking table
fn create_migrations_table(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
}

/// Get the current schema version
pub fn get_schema_version(conn: &Connection) -> AppResult<i32> {
    let result: Result<i32, _> = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    );

    result.map_err(|e| AppError::Database(e.to_string()))
}

/// Set the schema version after a migration
fn set_schema_version(conn: &Connection, version: i32) -> AppResult<()> {
    conn.execute(
        "INSERT INTO schema_migrations (version) VALUES (?1)",
        [version],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
}
